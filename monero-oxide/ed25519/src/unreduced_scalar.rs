use core::cmp::Ordering;
use std_shims::{
  sync::LazyLock,
  io::{self, *},
};

use subtle::{Choice, ConstantTimeEq};
use zeroize::Zeroize;

use monero_io::*;

use crate::Scalar;

/// 非縮約（unreduced）スカラー。
///
/// 補足: 現代の Monero ではほとんどのスカラーが縮約（reduced）されますが、レガシーな箇所では
/// そうでないことがありました。本構造体は変換を遅延させ、必要に応じて非標準の還元を行います。
///
/// 英語原文: An unreduced scalar. This struct delays scalar conversions and offers the non-standard reduction.
#[derive(Clone, Copy, Eq, Debug, Zeroize)]
pub struct UnreducedScalar([u8; 32]);

impl ConstantTimeEq for UnreducedScalar {
  fn ct_eq(&self, other: &Self) -> Choice {
    self.0.ct_eq(&other.0)
  }
}
impl PartialEq for UnreducedScalar {
  /// This defers to `ConstantTimeEq::ct_eq`.
  fn eq(&self, other: &Self) -> bool {
    bool::from(self.ct_eq(other))
  }
}

impl UnreducedScalar {
  /// `UnreducedScalar` を読み込みます（バイナリ）。
  pub fn read<R: Read>(r: &mut R) -> io::Result<UnreducedScalar> {
    Ok(UnreducedScalar(read_bytes(r)?))
  }

  /// `UnreducedScalar` を書き出します（バイナリ）。
  pub fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
    w.write_all(&self.0)
  }

  fn as_bits(&self) -> [u8; 256] {
    let mut bits = [0; 256];
    for (i, bit) in bits.iter_mut().enumerate() {
      // Using `Choice` here takes advantage of `subtle`'s internal `black_box` function, necessary
      // as our MSRV doesn't allow us to use `core::hint::black_box`
      *bit = Choice::from(1 & (self.0[i / 8] >> (i % 8))).unwrap_u8();
    }

    bits
  }

  // 幅 5 の非隣接表現（NAF: non-adjacent form）を計算します。
  //
  // 補足: これは Monero の `slide` 関数と同等であり、Monero と一致させるために特定条件下で
  // 正しくない出力を返す場合があります。
  //
  // 注意: 定数時間ではなく、公開データでのみ使用してください。
  fn non_adjacent_form(&self) -> [i8; 256] {
    let bits = self.as_bits();
    let mut naf = [0i8; 256];
    for (b, bit) in bits.into_iter().enumerate() {
      naf[b] = i8::try_from(bit).expect("bit didn't fit within an i8");
    }

    for i in 0 .. 256 {
      if naf[i] != 0 {
        // if the bit is a one, work our way up through the window
        // combining the bits with this bit.
        for b in 1 .. 6 {
          if (i + b) >= 256 {
            // if we are at the length of the array then break out
            // the loop.
            break;
          }
          // potential_carry - the value of the bit at i+b compared to the bit at i
          let potential_carry = naf[i + b] << b;

          if potential_carry != 0 {
            if (naf[i] + potential_carry) <= 15 {
              // if our current "bit" plus the potential carry is less than 16
              // add it to our current "bit" and set the potential carry bit to 0.
              naf[i] += potential_carry;
              naf[i + b] = 0;
            } else if (naf[i] - potential_carry) >= -15 {
              // else if our current "bit" minus the potential carry is more than -16
              // take it away from our current "bit".
              // we then work our way up through the bits setting ones to zero, when
              // we hit the first zero we change it to one then stop, this is to factor
              // in the minus.
              naf[i] -= potential_carry;
              #[allow(clippy::needless_range_loop)]
              for k in (i + b) .. 256 {
                if naf[k] == 0 {
                  naf[k] = 1;
                  break;
                }
                naf[k] = 0;
              }
            } else {
              break;
            }
          }
        }
      }
    }

    naf
  }

  /// ref10 の `slide` 関数によって誤って解釈されたスカラーを復元します（Monero 参照実装に合わせるため）。
  ///
  /// 補足: Borromean 範囲証明において、Monero は使用されたスカラーが縮約されているかを確認していなかったため、
  /// シリアライズされた一部スカラーが別の値として扱われました。本関数はそれらを復元し、証明の検証に必要です。
  ///
  /// 参考: <https://github.com/monero-project/monero/issues/8438>
  ///
  /// 注意: 定数時間ではなく、公開データでのみ使用してください。
  pub fn ref10_slide_scalar_vartime(&self) -> Scalar {
    use curve25519_dalek::Scalar as DScalar;

    /// Precomputed scalars used to recover an incorrectly reduced scalar
    static PRECOMPUTED_SCALARS: LazyLock<[DScalar; 8]> = LazyLock::new(|| {
      let mut precomputed_scalars = [DScalar::ONE; 8];
      for (i, scalar) in precomputed_scalars.iter_mut().enumerate().skip(1) {
        *scalar = DScalar::from(
          u64::try_from((i * 2) + 1).expect("enumerating more than u64::MAX / 2 items"),
        );
      }
      precomputed_scalars
    });

    if self.0[31] & 128 == 0 {
      // Computing the w-NAF of a number can only give an output with 1 more bit than
      // the number, so even if the number isn't reduced, the `slide` function will be
      // correct when the last bit isn't set.
      return Scalar::from(DScalar::from_bytes_mod_order(self.0));
    }

    let mut recovered = DScalar::ZERO;
    for &numb in self.non_adjacent_form().iter().rev() {
      recovered += recovered;
      match numb.cmp(&0) {
        Ordering::Greater => {
          recovered += PRECOMPUTED_SCALARS[usize::try_from(numb).expect("positive i8 -> usize") / 2]
        }
        Ordering::Less => {
          recovered -=
            PRECOMPUTED_SCALARS[usize::try_from(-numb).expect("negated negative i8 -> usize") / 2]
        }
        Ordering::Equal => (),
      }
    }
    Scalar::from(recovered)
  }
}
