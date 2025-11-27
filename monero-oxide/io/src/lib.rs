#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

use core::fmt::Debug;
#[allow(unused_imports)]
use std_shims::prelude::*;
use std_shims::io::{self, Read, Write};

mod varint;
pub use varint::*;

/// 1 バイトを書き込む補助関数。
///
/// 汎用的なシリアライザ内でのビルディングブロックとして使用される。
pub fn write_byte<W: Write>(byte: &u8, w: &mut W) -> io::Result<()> {
  w.write_all(&[*byte])
}

/// 長さプレフィックス無しで要素列を書き込む。
pub fn write_raw_vec<T, W: Write, F: FnMut(&T, &mut W) -> io::Result<()>>(
  mut f: F,
  values: &[T],
  w: &mut W,
) -> io::Result<()> {
  for value in values {
    f(value, w)?;
  }
  Ok(())
}

/// 要素列を書き込み、先頭に VarInt で長さを付ける。
pub fn write_vec<T, W: Write, F: FnMut(&T, &mut W) -> io::Result<()>>(
  f: F,
  values: &[T],
  w: &mut W,
) -> io::Result<()> {
  VarInt::write(&values.len(), w)?;
  write_raw_vec(f, values, w)
}

/// 固定長のバイト数を読み取るユーティリティ。
pub fn read_bytes<R: Read, const N: usize>(r: &mut R) -> io::Result<[u8; N]> {
  let mut res = [0; N];
  r.read_exact(&mut res)?;
  Ok(res)
}

/// 1 バイトを読み取る。
pub fn read_byte<R: Read>(r: &mut R) -> io::Result<u8> {
  Ok(read_bytes::<_, 1>(r)?[0])
}

/// リトルエンディアンで `u16` を読み取る。
pub fn read_u16<R: Read>(r: &mut R) -> io::Result<u16> {
  read_bytes(r).map(u16::from_le_bytes)
}

/// リトルエンディアンで `u32` を読み取る。
pub fn read_u32<R: Read>(r: &mut R) -> io::Result<u32> {
  read_bytes(r).map(u32::from_le_bytes)
}

/// リトルエンディアンで `u64` を読み取る。
pub fn read_u64<R: Read>(r: &mut R) -> io::Result<u64> {
  read_bytes(r).map(u64::from_le_bytes)
}

/// 長さが既知の要素列を読み取る（長さプレフィックス無し）。
pub fn read_raw_vec<R: Read, T, F: FnMut(&mut R) -> io::Result<T>>(
  mut f: F,
  len: usize,
  r: &mut R,
) -> io::Result<Vec<T>> {
  let mut res = vec![];
  for _ in 0 .. len {
    res.push(f(r)?);
  }
  Ok(res)
}

/// 固定長の配列を読み取るユーティリティ（`Vec` を読み取ってから変換する）。
pub fn read_array<R: Read, T: Debug, F: FnMut(&mut R) -> io::Result<T>, const N: usize>(
  f: F,
  r: &mut R,
) -> io::Result<[T; N]> {
  read_raw_vec(f, N, r).map(|vec| {
    vec.try_into().expect(
      "read vector of specific length yet couldn't transform to an array of the same length",
    )
  })
}

/// VarInt（長さ付き）を読み取り、必要に応じて上限チェックを行ってベクタを構築する。
pub fn read_vec<R: Read, T, F: FnMut(&mut R) -> io::Result<T>>(
  f: F,
  length_bound: Option<usize>,
  r: &mut R,
) -> io::Result<Vec<T>> {
  let declared_length: usize = VarInt::read(r)?;
  if let Some(length_bound) = length_bound {
    if declared_length > length_bound {
      Err(io::Error::other("vector exceeds bound on length"))?;
    }
  }
  read_raw_vec(f, declared_length, r)
}
