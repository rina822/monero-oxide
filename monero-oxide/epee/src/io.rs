//! IO 周りの低レイヤプリミティブ（EPEE デコード用）
//!
//! このモジュールはバイト列から値を読み取るためのトレイトとユーティリティを提供します。

use core::marker::PhantomData;

use crate::EpeeError;

/// `&[u8]` のように振る舞うオブジェクトの抽象。
///
/// 具象型がスライスやカーソルなどどのような形であれ、必要な読み出し操作を提供すれば
/// `BytesLike` を実装できます。`ExternallyTrackedLength` は外部で長さを追跡する必要がある型
/// （例えばスライスなら `()`、別のコンテナなら `usize`）を表します。
#[allow(clippy::len_without_is_empty)]
pub trait BytesLike<'encoding>: Sized {
  /// 読み取り時に外部で追跡される長さの型（通常は `()` か `usize`）
  type ExternallyTrackedLength: Sized + Copy;

  /// このバイト列の長さを返す。外部追跡長さを渡す場合はそれを使って計算する。
  fn len(&self, len: Self::ExternallyTrackedLength) -> usize;

  /// 固定長のバイト列を読み取り、残りのコンテナとともに返す（成功時は `(len, slice)`）。
  fn read_bytes(
    &mut self,
    bytes: usize,
  ) -> Result<(Self::ExternallyTrackedLength, Self), EpeeError>;

  /// 固定長のバイト列を与えられたスライスにコピーする。汎用実装は非効率なため
  /// 呼び出し側に実装を要求する。
  fn read_into_slice(&mut self, slice: &mut [u8]) -> Result<(), EpeeError>;

  /// 1 バイト読み取る便利メソッド。
  #[inline(always)]
  fn read_byte(&mut self) -> Result<u8, EpeeError> {
    let mut buf = [0; 1];
    self.read_into_slice(&mut buf)?;
    Ok(buf[0])
  }

  /// コンテナを N バイト進める（読み飛ばす）便利メソッド。
  #[inline(always)]
  fn advance<const N: usize>(&mut self) -> Result<(), EpeeError> {
    self.read_bytes(N).map(|_| ())
  }
}

/// `&[u8]` に対する `BytesLike` 実装。
impl<'encoding> BytesLike<'encoding> for &'encoding [u8] {
  type ExternallyTrackedLength = ();

  #[inline(always)]
  fn len(&self, (): ()) -> usize {
    <[u8]>::len(self)
  }

  #[inline(always)]
  fn read_bytes(
    &mut self,
    bytes: usize,
  ) -> Result<(Self::ExternallyTrackedLength, Self), EpeeError> {
    if self.len() < bytes {
      // 足りなければ Short エラーを返す
      Err(EpeeError::Short(bytes))?;
    }
    let res = &self[.. bytes];
    *self = &self[bytes ..];
    Ok(((), res))
  }

  #[inline(always)]
  fn read_into_slice(&mut self, slice: &mut [u8]) -> Result<(), EpeeError> {
    /*
      スライス自体であれば部分スライスを返すことでコピーを避けられるが、汎用性を保つため
      ここではコピー実装を使う。呼び出し側では通常最大 8 バイトしか読まない。
    */
    slice.copy_from_slice(self.read_bytes(slice.len())?.1);
    Ok(())
  }
}

/// EPEE の定義に従って VarInt を読み取る。
///
/// ここでは正規化（最短表現）を要求せず、より大きなエンコードも許容する実装になっている。
// 参照: monero の実装
pub(crate) fn read_varint<'encoding>(
  reader: &mut impl BytesLike<'encoding>,
) -> Result<u64, EpeeError> {
  let vi_start = reader.read_byte()?;

  // 下位 2 ビットでその後に来るバイト数を表す（0 => 1, 1 => 2, 2 => 4, 3 => 8）
  let len = match vi_start & 0b11 {
    0 => 1,
    1 => 2,
    2 => 4,
    3 => 8,
    _ => unreachable!(),
  };

  let mut vi = u64::from(vi_start);
  for i in 1 .. len {
    vi |= u64::from(reader.read_byte()?) << (i * 8);
  }
  // 2 ビット右にシフトして実際の値を得る
  vi >>= 2;

  Ok(vi)
}

/// 長さ情報を持つバイト列のラッパー。
///
/// EPEE の仕様では文字列やキーの長さが別途エンコードされるため、長さ情報を外部で追跡する
/// 必要があるケースに対してこの型が使われる。
pub struct String<'encoding, B: BytesLike<'encoding>> {
  pub(crate) len: B::ExternallyTrackedLength,
  pub(crate) bytes: B,
  pub(crate) _encoding: PhantomData<&'encoding ()>,
}

impl<'encoding, B: BytesLike<'encoding>> String<'encoding, B> {
  /// 空文字列かどうか
  #[inline(always)]
  pub fn is_empty(&self) -> bool {
    self.bytes.len(self.len) == 0
  }

  /// 現在の長さ（バイト数）
  #[inline(always)]
  pub fn len(&self) -> usize {
    self.bytes.len(self.len)
  }

  /// 内部のバイトコンテナを取り出す
  #[inline(always)]
  pub fn consume(self) -> B {
    self.bytes
  }
}

/// EPEE の文字列（length-prefixed）を読み取るユーティリティ。
#[inline(always)]
pub(crate) fn read_str<'encoding, B: BytesLike<'encoding>>(
  reader: &mut B,
) -> Result<String<'encoding, B>, EpeeError> {
  /*
    VarInt が usize::MAX を超える場合は扱えないため、その判定に失敗したら Short エラー
    を返す。
  */
  let len = usize::try_from(read_varint(reader)?).map_err(|_| EpeeError::Short(usize::MAX))?;
  let (len, bytes) = reader.read_bytes(len)?;
  Ok(String { len, bytes, _encoding: PhantomData })
}
