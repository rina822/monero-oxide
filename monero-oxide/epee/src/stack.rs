//! 非ヒープ割当ての代替スタック（`Vec` の代替）。EPEE デコーダの状態管理に使用します。
//!
//! EPEE のオブジェクト深さが最大 100 に限定されている特性を利用して、深さに依存する
//! 固定配列で状態を管理します。これによりヒープ割当てを回避し、非常に小さいメモリ
//! フットプリント（kibibyte 未満）で動作させられます。
//!
//! このモジュールはライブラリ内部実装向けであり、パニックを発生させないよう慎重に
//! 書かれています。

/*
  Introducing `NonZero` bumped us from a MSRV of Rust 1.60 to Rust 1.79. We have no incentive to
  support Rust 1.60 here, yet it would be trivial to expand the supported versions of Rust.
*/
use core::num::NonZero;

use crate::{EpeeError, Type, TypeOrEntry};

// https://github.com/monero-project/monero/blob/8d4c625713e3419573dfcc7119c8848f47cabbaa/
//   contrib/epee/include/storages/portable_storage_from_bin.h#L42
const EPEE_LIB_MAX_OBJECT_DEPTH: usize = 100;
/*
  Explicitly set a larger depth in case we have slight differences in counting the current depth.

  The goal of this library is to decode at _least_ the set of objects EPEE itself will handle.
  While we may be slightly more tolerant, this is accepted over incompatibility given the lack of a
  strict specification for EPEE.

  Additionally, decoding a larget set of objects will not be considered an incompatibility so long
  as encodings (unsupported at the time of writing this comment) will always be handled by EPEE,
  ensuring _mutual_ compatibility.
*/
const MAX_OBJECT_DEPTH: usize = EPEE_LIB_MAX_OBJECT_DEPTH + 2;

/// An array of `TypeOrEntry`, using `u4` for each value.
struct PackedTypes([u8; MAX_OBJECT_DEPTH.div_ceil(2)]);
impl PackedTypes {
  fn get(&self, i: usize) -> TypeOrEntry {
    let mut entry = self.0[i / 2];
    entry >>= (i & 1) * 4;
    entry &= 0b1111;
    match entry {
      0 => TypeOrEntry::Entry,
      1 => TypeOrEntry::Type(Type::Int64),
      2 => TypeOrEntry::Type(Type::Int32),
      3 => TypeOrEntry::Type(Type::Int16),
      4 => TypeOrEntry::Type(Type::Int8),
      5 => TypeOrEntry::Type(Type::Uint64),
      6 => TypeOrEntry::Type(Type::Uint32),
      7 => TypeOrEntry::Type(Type::Uint16),
      8 => TypeOrEntry::Type(Type::Uint8),
      9 => TypeOrEntry::Type(Type::Double),
      10 => TypeOrEntry::Type(Type::String),
      11 => TypeOrEntry::Type(Type::Bool),
      12 => TypeOrEntry::Type(Type::Object),
      13 ..= 15 => panic!("`PackedTypes` was written to with a non-existent `TypeOrEntry`"),
      _ => unreachable!("masked by 0b1111"),
    }
  }

  fn set(&mut self, i: usize, kind: TypeOrEntry) {
    let four_bits = match kind {
      TypeOrEntry::Entry => 0,
      TypeOrEntry::Type(kind) => kind as u8,
    };
    let shift = (i & 1) * 4;
    // Clear the existing value in this slot
    self.0[i / 2] &= 0b11110000 >> shift;
    // Set the new value
    self.0[i / 2] |= four_bits << shift;
  }
}

/// A non-allocating `Vec`.
///
/// This has a maximum depth premised on the bound for an EPEE's object depth.
pub(crate) struct Stack {
  /*
    We represent items to decode as `(TypeOrEntry, usize)` so that if told to decode a vector with
    one billion entries, we don't have to allocate
    `vec![TypeOrEntry::Entry(Type::*), 1_000_000_000]` to keep track of the state. Instead, the
    size of the state is solely a function of depth, not width.

    The following two arrays are separate as Rust would pad `(TypeOrEntry, usize)` to 16 bytes,
    when it only requires 9 bytes to represent. Additionally, as there's less than 2**4 possible
    states for a `TypeOrEntry`, we represent it with a `u4` (via `PackedTypes`).
  */
  /// The type of the item being read.
  types: PackedTypes,
  /// The amount remaining for the item being read.
  amounts: [NonZero<usize>; MAX_OBJECT_DEPTH],

  /// The current depth of the stack.
  ///
  /// This is analogous to the length of a `Vec`, yet we use the term `depth` to distinguish how it
  /// tracks the depth of an object, not the amount of items present (which would be a function of
  /// depth and width, as noted above).
  depth: u8,
}

/*
  Because every time we decode an object, we allocate this fixed-size item on the stack (avoiding
  requiring dynamic allocation on the heap), ensure it's tiny and not an issue to allocate.
*/
#[cfg(test)]
const _ASSERT_KIBIBYTE_STACK: [(); 1024 - core::mem::size_of::<Stack>()] =
  [(); 1024 - core::mem::size_of::<Stack>()];

impl Stack {
    /// ルートオブジェクト用の新しいスタックを作成する。
    ///
    /// 内部で固定長配列をゼロ初期化し、`amounts` は `NonZero::MIN` を使って非ゼロ化している。
    #[inline(always)]
    pub(crate) fn root_object() -> Self {
      /*
        配列をゼロ初期化する。

        `amounts` は非ゼロである必要があるため `NonZero::MIN` を用いる設計にしている。
        生ポインタや `unsafe` を使う代替設計も考えられるが、ここでは安全実装を優先している。
      */
    let mut types = PackedTypes([0; MAX_OBJECT_DEPTH.div_ceil(2)]);
    let mut amounts = [NonZero::<usize>::MIN; MAX_OBJECT_DEPTH];

    types.set(0, TypeOrEntry::Type(Type::Object));
    amounts[0] = NonZero::<usize>::MIN; // 1

    Self { types, amounts, depth: 1 }
  }

  /// 現在のスタック深さを返す（`Vec::len` 相当）。
  #[inline(always)]
  pub(crate) fn depth(&self) -> usize {
    usize::from(self.depth)
  }

  /// スタックの先頭（現在処理中の項目）を覗く。存在しない場合は `None` を返す。
  #[inline(always)]
  pub(crate) fn peek(&self) -> Option<(TypeOrEntry, NonZero<usize>)> {
    let i = self.depth().checked_sub(1)?;
    Some((self.types.get(i), self.amounts[i]))
  }

  /// スタックから次の項目をポップする。
  ///
  /// `amounts` をデクリメントし、0 になれば深さを一段下げる（そのスロットは次に上書きされる）。
  pub(crate) fn pop(&mut self) -> Option<TypeOrEntry> {
    let i = self.depth().checked_sub(1)?;

    let kind = self.types.get(i);

    // `amounts[i]` は非ゼロなので安全にデクリメント可能
    let amount = self.amounts[i].get() - 1;
    if let Some(amount) = NonZero::new(amount) {
      self.amounts[i] = amount;
    } else {
      // 残数がなくなれば深さを減らす
      self.depth -= 1;
    }

    Some(kind)
  }

  /// スタックに項目をプッシュする。
  ///
  /// `amount` はそのレベル内に残る要素数を表す（0 の場合は即座に何もしない）。
  pub(crate) fn push(&mut self, kind: TypeOrEntry, amount: usize) -> Result<(), EpeeError> {
    // 深さの上限を超えるとエラー
    if self.depth() == MAX_OBJECT_DEPTH {
      Err(EpeeError::DepthLimitExceeded)?;
    }

    let Some(amount) = NonZero::new(amount) else {
      // デコードする要素が無ければ早期リターン
      return Ok(());
    };

    // 深さチェック済みなので安全に書き込む
    self.types.set(self.depth(), kind);
    self.amounts[self.depth()] = amount;
    self.depth += 1;

    Ok(())
  }
}
