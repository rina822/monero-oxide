use core::marker::PhantomData;

use crate::{EpeeError, Stack, io::*};

/// The EPEE-defined type of the field being read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Type {
  /// An `i64`.
  Int64 = 1,
  /// An `i32`.
  Int32 = 2,
  /// An `i16`.
  Int16 = 3,
  /// An `i8`.
  Int8 = 4,
  /// A `u64`.
  Uint64 = 5,
  /// A `u32`.
  Uint32 = 6,
  /// A `u16`.
  Uint16 = 7,
  /// A `u8`.
  Uint8 = 8,
  /// A `f64`.
  Double = 9,
  /// A length-prefixed collection of bytes.
  String = 10,
  /// A `bool`.
  Bool = 11,
  /// An object.
  Object = 12,
  /*
    Unused and unsupported. See
    https://github.com/monero-project/monero/pull/10138 for more info.
  */
  // Array = 13,
}

/// A bitflag for if the field is actually an array.
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum Array {
  /// A unit type.
  Unit = 0,
  /// An array.
  Array = 1 << 7,
}

/*
  An internal marker used to distinguish if we're reading an EPEE-defined field OR if we're reading
  an entry within an section (object). This lets us collapse the definition of a section to an
  array of entries, simplifying decoding.
*/
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeOrEntry {
  // An epee-defined type
  Type(Type),
  use core::marker::PhantomData;

  use crate::{EpeeError, Stack, io::*};

  /// EPEE が定義するフィールド型を表す列挙型。
  #[derive(Clone, Copy, PartialEq, Eq, Debug)]
  pub enum Type {
    /// i64
    Int64 = 1,
    /// i32
    Int32 = 2,
    /// i16
    Int16 = 3,
    /// i8
    Int8 = 4,
    /// u64
    Uint64 = 5,
    /// u32
    Uint32 = 6,
    /// u16
    Uint16 = 7,
    /// u8
    Uint8 = 8,
    /// f64
    Double = 9,
    /// 長さ付きバイト列（string）
    String = 10,
    /// bool
    Bool = 11,
    /// オブジェクト（セクション）
    Object = 12,
  }

  /// 配列かどうかを示すフラグ（上位ビットを使用）。
  #[derive(Clone, Copy, Debug)]
  #[repr(u8)]
  pub enum Array {
    /// 単位型
    Unit = 0,
    /// 配列フラグ（1 << 7）
    Array = 1 << 7,
  }

  /*
    内部的には、Type（プリミティブ型）と Entry（オブジェクト内のエントリ）を区別する
    マーカーを用いてデコードを行う。これによりオブジェクトをエントリの配列として扱える。
  */
  #[derive(Clone, Copy, PartialEq, Eq)]
  pub(crate) enum TypeOrEntry {
    // EPEE 定義の型
    Type(Type),
    // エントリ（キー, 型, 値）
    Entry,
  }

  impl Type {
    /// 型仕様を読み取り、(Type, length) を返す。長さは配列の場合にのみ VarInt で読み取られる。
    pub fn read<'encoding>(reader: &mut impl BytesLike<'encoding>) -> Result<(Self, u64), EpeeError> {
      let kind = reader.read_byte()?;

      // 配列ビットがセットされているか判定
      let array = kind & (Array::Array as u8);
      // 配列ビットをクリアして実際の型を抽出
      let kind = kind & (!(Array::Array as u8));

      let kind = match kind {
        1 => Type::Int64,
        2 => Type::Int32,
        3 => Type::Int16,
        4 => Type::Int8,
        5 => Type::Uint64,
        6 => Type::Uint32,
        7 => Type::Uint16,
        8 => Type::Uint8,
        9 => Type::Double,
        10 => Type::String,
        11 => Type::Bool,
        12 => Type::Object,
        _ => Err(EpeeError::UnrecognizedType)?,
      };

      // 配列であれば VarInt で長さを読み取る。配列でなければ長さは 1 と扱う。
      let len = if array != 0 { read_varint(reader)? } else { 1 };

      Ok((kind, len))
    }
  }

  /// エントリのキー（文字列）を読み取る。先頭バイトはキー長。
  fn read_key<'encoding, B: BytesLike<'encoding>>(
    reader: &mut B,
  ) -> Result<String<'encoding, B>, EpeeError> {
    let len = usize::from(reader.read_byte()?);
    if len == 0 {
      Err(EpeeError::EmptyKey)?;
    }
    let (len, bytes) = reader.read_bytes(len)?;
    Ok(String { len, bytes, _encoding: PhantomData })
  }

  /// デコーダの単一ステップの結果を表す。
  pub(crate) enum SingleStepResult<'encoding, B: BytesLike<'encoding>> {
    /// オブジェクトを開始し、フィールド数を返す
    Object { fields: usize },
    /// エントリ（キー, 型, 長さ）を返す
    Entry { key: String<'encoding, B>, kind: Type, len: usize },
    /// 単位（データのスキップなど）
    Unit,
  }

  impl Stack {
    /// デコードアルゴリズムの単一ステップを実行する。
    ///
    /// エントリを読み取った場合は `Some(SingleStepResult::Entry)` などを返す。スタックが空なら `Ok(None)`。
    pub(crate) fn single_step<'encoding, B: BytesLike<'encoding>>(
      &mut self,
      encoding: &mut B,
    ) -> Result<Option<SingleStepResult<'encoding, B>>, EpeeError> {
      let Some(kind) = self.pop() else {
        return Ok(None);
      };
      match kind {
        TypeOrEntry::Type(Type::Int64) => {
          encoding.advance::<{ core::mem::size_of::<i64>() }>()?;
        }
        TypeOrEntry::Type(Type::Int32) => {
          encoding.advance::<{ core::mem::size_of::<i32>() }>()?;
        }
        TypeOrEntry::Type(Type::Int16) => {
          encoding.advance::<{ core::mem::size_of::<i16>() }>()?;
        }
        TypeOrEntry::Type(Type::Int8) => {
          encoding.advance::<{ core::mem::size_of::<i8>() }>()?;
        }
        TypeOrEntry::Type(Type::Uint64) => {
          encoding.advance::<{ core::mem::size_of::<u64>() }>()?;
        }
        TypeOrEntry::Type(Type::Uint32) => {
          encoding.advance::<{ core::mem::size_of::<u32>() }>()?;
        }
        TypeOrEntry::Type(Type::Uint16) => {
          encoding.advance::<{ core::mem::size_of::<u16>() }>()?;
        }
        TypeOrEntry::Type(Type::Uint8) => {
          encoding.advance::<{ core::mem::size_of::<u8>() }>()?;
        }
        TypeOrEntry::Type(Type::Double) => {
          encoding.advance::<{ core::mem::size_of::<f64>() }>()?;
        }
        TypeOrEntry::Type(Type::String) => {
          // 文字列は length-prefix を読み飛ばす
          read_str(encoding)?;
        }
        TypeOrEntry::Type(Type::Bool) => {
          encoding.advance::<{ core::mem::size_of::<bool>() }>()?;
        }
        TypeOrEntry::Type(Type::Object) => {
          // オブジェクト開始: フィールド数を読み、スタックに Entry を push
          let fields = read_varint(encoding)?;
          // フィールド数が usize に収まらない場合は Short として扱う
          let fields = usize::try_from(fields).map_err(|_| EpeeError::Short(usize::MAX))?;
          self.push(TypeOrEntry::Entry, fields)?;
          return Ok(Some(SingleStepResult::Object { fields }));
        }
        TypeOrEntry::Entry => {
          // オブジェクト内の次のエントリを読み取る: キー、型、長さ
          let key = read_key(encoding)?;
          let (kind, len) = Type::read(encoding)?;
          let len = usize::try_from(len).map_err(|_| EpeeError::Short(usize::MAX))?;
          /* Lines 178-180 omitted */
        }
      }
      Ok(Some(SingleStepResult::Unit))
    }

    /// 次のアイテム全体が読み終わるまでステップを進めるユーティリティ。
    ///
    /// スタックが空なら `Ok(None)` を返す。
    pub(crate) fn step<'encoding, B: BytesLike<'encoding>>(
      &mut self,
      encoding: &mut B,
    ) -> Result<Option<()>, EpeeError> {
      let Some((_kind, len)) = self.peek() else { return Ok(None) };

      let current_stack_depth = self.depth();
      // 次のアイテム内まで読み進めるための停止深度を計算
      let stop_at_stack_depth = if len.get() > 1 {
        current_stack_depth
      } else {
        // 単一要素なら現在の深さ - 1 まで読み切る
        current_stack_depth - 1
      };

      while {
        self.single_step(encoding)?;
        self.depth() != stop_at_stack_depth
      } {}

      Ok(Some(()))
    }
  }
