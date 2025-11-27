// リング署名（旧 Cryptonote の署名形式）に関する実装。
// RingCT により非推奨となったが、プロトコル互換性のため実装は残す。
use std_shims::{
  io::{self, *},
  vec::Vec,
};

use zeroize::Zeroize;

use crate::{io::*, ed25519::*};

/// 内部的な署名要素（c, s）。テスト時は pub、通常は非公開フィールド。
#[derive(Clone, PartialEq, Eq, Debug, Zeroize)]
pub(crate) struct Signature {
  #[cfg(test)]
  pub(crate) c: Scalar,
  #[cfg(test)]
  pub(crate) s: Scalar,
  #[cfg(not(test))]
  c: Scalar,
  #[cfg(not(test))]
  s: Scalar,
}

impl Signature {
  fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
    self.c.write(w)?;
    self.s.write(w)?;
    Ok(())
  }

  fn read<R: Read>(r: &mut R) -> io::Result<Signature> {
    Ok(Signature { c: Scalar::read(r)?, s: Scalar::read(r)? })
  }
}

/// リング署名本体。複数の `Signature` を保持する。
#[derive(Clone, PartialEq, Eq, Debug, Zeroize)]
pub struct RingSignature {
  #[cfg(test)]
  pub(crate) sigs: Vec<Signature>,
  #[cfg(not(test))]
  sigs: Vec<Signature>,
}

impl RingSignature {
  /// RingSignature を書き込む。
  pub fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
    for sig in &self.sigs {
      sig.write(w)?;
    }
    Ok(())
  }

  /// RingSignature を読み取る。
  pub fn read<R: Read>(members: usize, r: &mut R) -> io::Result<RingSignature> {
    Ok(RingSignature { sigs: read_raw_vec(Signature::read, members, r)? })
  }

  /// リング署名の検証。
  ///
  /// 注意: Monero の Fiat-Shamir トランスクリプト形式に従っており、`msg_hash` に何が
  /// 含まれているべきかの前提があるため、誤用は非常に危険です。
  pub fn verify(
    &self,
    msg_hash: &[u8; 32],
    ring: &[CompressedPoint],
    key_image: &CompressedPoint,
  ) -> bool {
    if ring.len() != self.sigs.len() {
      return false;
    }

    let Some(key_image) = key_image.decompress() else {
      return false;
    };
    let Some(key_image) = key_image.key_image() else {
      return false;
    };

    let mut buf = Vec::with_capacity(32 + (2 * 32 * ring.len()));
    buf.extend_from_slice(msg_hash);

    let mut sum = curve25519_dalek::Scalar::ZERO;
    for (ring_member, sig) in ring.iter().zip(&self.sigs) {
      /*
        ここでは各リングメンバーについて双対 Schnorr 署名相当の検証値を生成し、
        最終的に c 値の合計がハッシュと一致するかを確認する。
      */

      let Some(decomp_ring_member) = ring_member.decompress() else {
        return false;
      };

      #[allow(non_snake_case)]
      let Li = curve25519_dalek::EdwardsPoint::vartime_double_scalar_mul_basepoint(
        &sig.c.into(),
        &decomp_ring_member.into(),
        &sig.s.into(),
      );
      buf.extend_from_slice(Li.compress().as_bytes());
      #[allow(non_snake_case)]
      let Ri = (sig.s.into() * Point::biased_hash(ring_member.to_bytes()).into()) +
        (sig.c.into() * key_image);
      buf.extend_from_slice(Ri.compress().as_bytes());

      sum += sig.c.into();
    }
    Scalar::from(sum) == Scalar::hash(buf)
  }
}
