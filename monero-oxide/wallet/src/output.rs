// 標準ライブラリ互換プレイサブルな項目をインポートします（`no_std` 対応の shim）。
use std_shims::{
  vec,
  vec::Vec,
  io::{self, Read, Write},
};

// メモリ消去用のトレイトをインポート（秘密情報のゼロ化に使用）。
use zeroize::{Zeroize, ZeroizeOnDrop};
// 定数時間比較のための型をインポート
use subtle::{Choice, ConstantTimeEq};

// カレントクレートから必要な型をインポート
use crate::{
  // IO ヘルパー（read_u64 等）をまとめてインポート
  io::*,
  // ed25519 関連の基本型
  ed25519::{Scalar, CompressedPoint, Point, Commitment},
  // トランザクションのタイムロック型
  transaction::Timelock,
  // サブアドレスインデックス型
  address::SubaddressIndex,
  // extra 関連の定数・型
  extra::{MAX_ARBITRARY_DATA_SIZE, MAX_EXTRA_SIZE_BY_RELAY_RULE, PaymentId},
};

// --- AbsoluteId: トランザクションハッシュ + トランザクション内出力インデックスで表される絶対出力ID ---
/// 絶対出力 ID: トランザクションハッシュとトランザクション内の出力インデックスで定義される構造体
/// 複数の出力が同じ出力鍵を共有し得るため、これは出力鍵そのものではない点に注意
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub(crate) struct AbsoluteId {
  // トランザクションハッシュ（32 バイト）
  pub(crate) transaction: [u8; 32],
  // トランザクション内の出力インデックス
  pub(crate) index_in_transaction: u64,
}

// Debug 実装: バイナリを人間可読な hex 表現にして表示
impl core::fmt::Debug for AbsoluteId {
  fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
    fmt
      .debug_struct("AbsoluteId")
      .field("transaction", &hex::encode(self.transaction))
      .field("index_in_transaction", &self.index_in_transaction)
      .finish()
  }
}

impl AbsoluteId {
  // 定数時間比較を行う内部ヘルパー（タイミング攻撃を緩和）
  fn ct_eq(&self, other: &Self) -> Choice {
    self.transaction.ct_eq(&other.transaction) &
      self.index_in_transaction.ct_eq(&other.index_in_transaction)
  }

  // シリアライズ: リトルエンディアンでインデックスを書き込む
  fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
    w.write_all(&self.transaction)?;
    w.write_all(&self.index_in_transaction.to_le_bytes())
  }

  // デシリアライズ: read_bytes/read_u64 を使用して復元
  fn read<R: Read>(r: &mut R) -> io::Result<AbsoluteId> {
    Ok(AbsoluteId { transaction: read_bytes(r)?, index_in_transaction: read_u64(r)? })
  }
}

// --- RelativeId: ブロックチェーン上の連続的なインデックス ---
/// 相対出力 ID: ブロックチェーン上のインデックスで定義される
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub(crate) struct RelativeId {
  // ブロックチェーン上のインデックス
  pub(crate) index_on_blockchain: u64,
}

impl core::fmt::Debug for RelativeId {
  fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
    fmt.debug_struct("RelativeId").field("index_on_blockchain", &self.index_on_blockchain).finish()
  }
}

impl RelativeId {
  // 定数時間比較
  fn ct_eq(&self, other: &Self) -> Choice {
    self.index_on_blockchain.ct_eq(&other.index_on_blockchain)
  }

  // シリアライズ（リトルエンディアン）
  fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
    w.write_all(&self.index_on_blockchain.to_le_bytes())
  }

  // デシリアライズ
  fn read<R: Read>(r: &mut R) -> io::Result<Self> {
    Ok(RelativeId { index_on_blockchain: read_u64(r)? })
  }
}

// --- OutputData: 出力を消費するために必要なデータ ---
/// 出力の中身: 出力鍵、鍵オフセット、コミットメントを保持する
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub(crate) struct OutputData {
  // 出力の公開鍵（Point）
  pub(crate) key: Point,
  // 出力鍵に対するオフセット（スカラー）
  pub(crate) key_offset: Scalar,
  // 出力で使用されたコミットメント
  pub(crate) commitment: Commitment,
}

impl core::fmt::Debug for OutputData {
  fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
    // key は圧縮して hex 表示、commitment はそのまま Debug 表示
    fmt
      .debug_struct("OutputData")
      .field("key", &hex::encode(self.key.compress().to_bytes()))
      .field("commitment", &self.commitment)
      .finish_non_exhaustive()
  }
}

impl OutputData {
  // 定数時間比較: key, key_offset, commitment をすべて比較
  pub(crate) fn ct_eq(&self, other: &Self) -> Choice {
    self.key.ct_eq(&other.key) &
      self.key_offset.ct_eq(&other.key_offset) &
      self.commitment.ct_eq(&other.commitment)
  }

  // key を返す（コピー可能な Point）
  pub(crate) fn key(&self) -> Point {
    self.key
  }

  // key_offset を返す
  pub(crate) fn key_offset(&self) -> Scalar {
    self.key_offset
  }

  // commitment への参照を返す
  pub(crate) fn commitment(&self) -> &Commitment {
    &self.commitment
  }

  // シリアライズ: key の圧縮バイト列、key_offset、commitment を順に書き込む
  pub(crate) fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
    w.write_all(&self.key.compress().to_bytes())?;
    self.key_offset.write(w)?;
    self.commitment.write(w)
  }

  // デシリアライズ: 圧縮点から復元、失敗した場合はエラー
  pub(crate) fn read<R: Read>(r: &mut R) -> io::Result<OutputData> {
    Ok(OutputData {
      key: CompressedPoint::read(r)?
        .decompress()
        .ok_or_else(|| io::Error::other("output data included an invalid key"))?,
      key_offset: Scalar::read(r)?,
      commitment: Commitment::read(r)?,
    })
  }
}

// --- Metadata: 出力に付随するメタデータ ---
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub(crate) struct Metadata {
  // 追加のタイムロック（トランザクションレベルの標準タイムロックとは別）
  pub(crate) additional_timelock: Timelock,
  // スキャンで判定されたサブアドレス（存在すれば Some）
  pub(crate) subaddress: Option<SubaddressIndex>,
  // 任意で付与される支払い ID
  pub(crate) payment_id: Option<PaymentId>,
  // 任意データ（extra の nonce 部分などの集まり）
  pub(crate) arbitrary_data: Vec<Vec<u8>>,
}

impl core::fmt::Debug for Metadata {
  fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
    fmt
      .debug_struct("Metadata")
      .field("additional_timelock", &self.additional_timelock)
      .field("subaddress", &self.subaddress)
      .field("payment_id", &self.payment_id)
      .field("arbitrary_data", &self.arbitrary_data.iter().map(hex::encode).collect::<Vec<_>>())
      .finish()
  }
}

impl Metadata {
  // 単純比較（Debug 用ではなく実際の等価性を判断するためのヘルパー）
  fn eq(&self, other: &Self) -> bool {
    (self.additional_timelock == other.additional_timelock) &&
      (self.subaddress == other.subaddress) &&
      (self.payment_id == other.payment_id) &&
      (self.arbitrary_data == other.arbitrary_data)
  }

  // シリアライズ: 各フィールドを順に書き出す
  fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
    self.additional_timelock.write(w)?;

    // サブアドレスがある場合はフラグ 1 とアカウント/アドレスを書き込む
    if let Some(subaddress) = self.subaddress {
      w.write_all(&[1])?;
      w.write_all(&subaddress.account().to_le_bytes())?;
      w.write_all(&subaddress.address().to_le_bytes())?;
    } else {
      // 無ければ 0
      w.write_all(&[0])?;
    }

    // payment_id の有無フラグとデータ
    if let Some(payment_id) = self.payment_id {
      w.write_all(&[1])?;
      payment_id.write(w)?;
    } else {
      w.write_all(&[0])?;
    }

    // arbitrary_data の配列長を VarInt で書く
    VarInt::write(&self.arbitrary_data.len(), w)?;
    for part in &self.arbitrary_data {
      // arbitrary data の各チャンクは u8 長で前置される（最大サイズチェック）
      const _ASSERT_MAX_ARBITRARY_DATA_SIZE_FITS_WITHIN_U8: [();
        (u8::MAX as usize) - MAX_ARBITRARY_DATA_SIZE] = [(); _];
      w.write_all(&[
        u8::try_from(part.len()).expect("piece of arbitrary data exceeded max length of u8::MAX")
      ])?;
      w.write_all(part)?;
    }
    Ok(())
  }

  // デシリアライズ: 順に読み出し、各種検査を行う
  fn read<R: Read>(r: &mut R) -> io::Result<Metadata> {
    let additional_timelock = Timelock::read(r)?;

    let subaddress = match read_byte(r)? {
      0 => None,
      1 => Some(
        SubaddressIndex::new(read_u32(r)?, read_u32(r)?)
          .ok_or_else(|| io::Error::other("invalid subaddress in metadata"))?,
      ),
      _ => Err(io::Error::other("invalid subaddress is_some boolean in metadata"))?,
    };

    Ok(Metadata {
      additional_timelock,
      subaddress,
      // payment_id はフラグが 1 なら読んで Some に、そうでなければ None
      payment_id: if read_byte(r)? == 1 { PaymentId::read(r).ok() } else { None },
      arbitrary_data: {
        // チャンク数を VarInt で読み込み、総サイズが許容量を超えないか検査する
        let chunks = <usize as VarInt>::read(r)?;
        if chunks > MAX_EXTRA_SIZE_BY_RELAY_RULE {
          Err(io::Error::other(
            "amount of arbitrary data chunks exceeded amount possible under policy",
          ))?;
        }

        let mut data = vec![];
        let mut total_len = 0usize;
        for _ in 0 .. chunks {
          let len = read_byte(r)?;
          let chunk = read_raw_vec(read_byte, usize::from(len), r)?;
          total_len = total_len.saturating_add(chunk.len());
          if total_len > MAX_EXTRA_SIZE_BY_RELAY_RULE {
            Err(io::Error::other("amount of arbitrary data exceeded amount allowed by policy"))?;
          }
          data.push(chunk);
        }
        data
      },
    })
  }
}

// --- WalletOutput: スキャン済み出力と関連メタデータ ---
/// スキャンされた出力とそれに紐づく全データを表す構造体
/// この構造体はブロックチェーンの特定の状態に結びつくため、再編成が起きたら破棄する必要がある
#[derive(Clone, Debug, Zeroize, ZeroizeOnDrop)]
pub struct WalletOutput {
  /// 絶対 ID（トランザクションハッシュ + トランザクション内インデックス）
  pub(crate) absolute_id: AbsoluteId,
  /// ブロックチェーン上の相対 ID（連続インデックス）
  pub(crate) relative_id: RelativeId,
  /// 出力データ本体
  pub(crate) data: OutputData,
  /// 支払い処理に必要なメタデータ
  pub(crate) metadata: Metadata,
}

impl PartialEq for WalletOutput {
  /// 完全な等価性: ID やデータをすべて比較する。定数時間比較で秘密情報の漏洩を防ぐ。
  fn eq(&self, other: &Self) -> bool {
    bool::from(
      self.absolute_id.ct_eq(&other.absolute_id) &
        self.relative_id.ct_eq(&other.relative_id) &
        self.data.ct_eq(&other.data),
    ) & self.metadata.eq(&other.metadata)
  }
}
impl Eq for WalletOutput {}

impl WalletOutput {
  /// トランザクションハッシュを返す
  pub fn transaction(&self) -> [u8; 32] {
    self.absolute_id.transaction
  }

  /// トランザクション内インデックスを返す
  pub fn index_in_transaction(&self) -> u64 {
    self.absolute_id.index_in_transaction
  }

  /// ブロックチェーン上のインデックスを返す
  pub fn index_on_blockchain(&self) -> u64 {
    self.relative_id.index_on_blockchain
  }

  /// 出力鍵を返す
  pub fn key(&self) -> Point {
    self.data.key()
  }

  /// 鍵オフセットを返す
  pub fn key_offset(&self) -> Scalar {
    self.data.key_offset()
  }

  /// コミットメントを返す
  pub fn commitment(&self) -> &Commitment {
    self.data.commitment()
  }

  /// 追加のタイムロック（標準 10 ブロック以外の追加分）を返す
  pub fn additional_timelock(&self) -> Timelock {
    self.metadata.additional_timelock
  }

  /// サブアドレス情報を返す（存在する場合）
  pub fn subaddress(&self) -> Option<SubaddressIndex> {
    self.metadata.subaddress
  }

  /// 支払い ID を返す（デコード済みの PaymentId）
  pub fn payment_id(&self) -> Option<PaymentId> {
    self.metadata.payment_id
  }

  /// トランザクションの extra に含まれる任意データ部分を返す
  pub fn arbitrary_data(&self) -> &[Vec<u8>] {
    &self.metadata.arbitrary_data
  }

  /// シリアライズ（書き込み）を行うユーティリティ
  pub fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
    self.absolute_id.write(w)?;
    self.relative_id.write(w)?;
    self.data.write(w)?;
    self.metadata.write(w)
  }

  /// シリアライズして Vec<u8> を返すヘルパー
  pub fn serialize(&self) -> Vec<u8> {
    let mut serialized = Vec::with_capacity(128);
    self.write(&mut serialized).expect("write failed but <Vec as io::Write> doesn't fail");
    serialized
  }

  /// デシリアライズ: 各フィールドを順に読み出して WalletOutput を復元
  pub fn read<R: Read>(r: &mut R) -> io::Result<WalletOutput> {
    Ok(WalletOutput {
      absolute_id: AbsoluteId::read(r)?,
      relative_id: RelativeId::read(r)?,
      data: OutputData::read(r)?,
      metadata: Metadata::read(r)?,
    })
  }
}
