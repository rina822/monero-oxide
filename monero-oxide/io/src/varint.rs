//! Monero の VarInt 実装（可変長整数）。
//!
//! 小さい数値を効率的にエンコードするための形式で、Monero の C++ 実装に対応しています。
//!
//! 参照: https://github.com/monero-project/monero/blob/8e9ab9677f90492bca3c7555a246f2a8677bd570/src/common/varint.h

#[allow(unused_imports)]
use std_shims::prelude::*;
use std_shims::io::{self, Read, Write};

use crate::{read_byte, write_byte};

const VARINT_CONTINUATION_FLAG: u8 = 0b1000_0000;
const VARINT_VALUE_MASK: u8 = !VARINT_CONTINUATION_FLAG;

mod sealed {
  /// A seal to prevent implementing `VarInt` on foreign types.
  pub trait Sealed {
    /// Lossless, guaranteed conversion into a `u64`.
    ///
    /// This is due to internally implementing encoding for `u64` alone and `usize` not implementing
    /// `From<u64>`.
    // This is placed here so it's not within our public API commitment.
    fn into_u64(self) -> u64;
  }
}

/// 与えられたビット幅から VarInt の最大エンコード長を計算する（コンパイル時に評価される）。
#[allow(clippy::cast_possible_truncation)]
const fn upper_bound(bits: u32) -> usize {
  // 256 ビット（u256 相当）を超えない想定
  assert!(bits <= 256, "defining a number exceeding u256 as a VarInt");
  // 7 ビットずつエンコードするため ceil(bits / 7)
  ((bits + (7 - 1)) / 7) as usize
}

/// VarInt として読み書きできる数値向けトレイト（sealed）。
///
/// 原則プリミティブ型のみで実装される想定で、誤った型の実装を防ぐためにシールドされている。
pub trait VarInt: TryFrom<u64> + Copy + sealed::Sealed {
  /// エンコード時の最小バイト数
  const LOWER_BOUND: usize;

  /// エンコード時の最大バイト数（上限）
  const UPPER_BOUND: usize;

  /// この値を VarInt としてエンコードしたときのバイト長を返す。
  fn varint_len(self) -> usize {
    let varint_u64 = self.into_u64();
    usize::try_from(u64::BITS - varint_u64.leading_zeros()).expect("64 > usize::MAX?").div_ceil(7)
  }

  /// 正格な（canonical）VarInt を読み取る。
  fn read<R: Read>(r: &mut R) -> io::Result<Self> {
    let mut bits = 0;
    let mut res = 0;
    while {
      let b = read_byte(r)?;
      // 余分な 0 バイト（先行ゼロ）は許容しない（非正規表現）
      if (bits != 0) && (b == 0) {
        Err(io::Error::other("non-canonical varint"))?;
      }

      // 実装側で VarInt を付与する型のサイズを用いてオーバーフローを検出
      #[allow(non_snake_case)]
      let U_BITS = core::mem::size_of::<Self>() * 8;
      if ((bits + 7) >= U_BITS) && (b >= (1 << (U_BITS - bits))) {
        Err(io::Error::other("varint overflow"))?;
      }

      res += u64::from(b & VARINT_VALUE_MASK) << bits;
      bits += 7;
      (b & VARINT_CONTINUATION_FLAG) == VARINT_CONTINUATION_FLAG
    } {}
    res.try_into().map_err(|_| io::Error::other("VarInt does not fit into integer type"))
  }

  /// VarInt をエンコードして書き込む。
  ///
  /// `self` を取らず参照を受ける API にしているのは、呼び出し側で明示的に `VarInt::write` を使う
  /// 意図を示したいため。
  fn write<W: Write>(varint: &Self, w: &mut W) -> io::Result<()> {
    let mut varint: u64 = varint.into_u64();

    // 少なくとも 1 バイトは出力する必要があるため擬似 do-while ループを用いる
    while {
      // 次の 7 ビットを取り出す
      let mut b = u8::try_from(varint & u64::from(VARINT_VALUE_MASK))
        .expect("& 0b0111_1111 left more than 8 bits set");
      varint >>= 7;

      // さらに残りがある場合は継続ビットを立てる
      if varint != 0 {
        b |= VARINT_CONTINUATION_FLAG;
      }

      // バイトを書き込む
      write_byte(&b, w)?;

      // 数値を最後までエンコードするまでループ
      varint != 0
    } {}

    Ok(())
  }
}

impl sealed::Sealed for u8 {
  fn into_u64(self) -> u64 {
    self.into()
  }
}
impl VarInt for u8 {
  const LOWER_BOUND: usize = 1;
  const UPPER_BOUND: usize = upper_bound(Self::BITS);
}

impl sealed::Sealed for u32 {
  fn into_u64(self) -> u64 {
    self.into()
  }
}
impl VarInt for u32 {
  const LOWER_BOUND: usize = 1;
  const UPPER_BOUND: usize = upper_bound(Self::BITS);
}

impl sealed::Sealed for u64 {
  fn into_u64(self) -> u64 {
    self
  }
}
impl VarInt for u64 {
  const LOWER_BOUND: usize = 1;
  const UPPER_BOUND: usize = upper_bound(Self::BITS);
}

impl sealed::Sealed for usize {
  fn into_u64(self) -> u64 {
    // Ensure the falling conversion is infallible
    const _NO_128_BIT_PLATFORMS: [(); (u64::BITS - usize::BITS) as usize] =
      [(); (u64::BITS - usize::BITS) as usize];

    self.try_into().expect("compiling on platform with <64-bit usize yet value didn't fit in u64")
  }
}
impl VarInt for usize {
  const LOWER_BOUND: usize = 1;
  const UPPER_BOUND: usize = upper_bound(Self::BITS);
}
