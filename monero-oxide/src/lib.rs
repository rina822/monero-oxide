#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

// ライブラリの公開 API として外部クレートを再公開するトップレベルモジュール。
pub use monero_io as io;
pub use monero_ed25519 as ed25519;
pub use monero_primitives as primitives;

/// Merkle 木機能
pub mod merkle;

/// リング署名関連機能
pub mod ring_signatures;

/// RingCT 関連機能
pub mod ringct;

/// トランザクション関連機能
pub mod transaction;
/// ブロック関連機能
pub mod block;

#[cfg(test)]
mod tests;

/// デフォルトの出力ロックウィンドウ（ブロック数）。
///
/// リオーガニゼーション（チェーン分岐）による影響を避けるために、過去 n ブロックより
/// 新しい出力をデコイに選ばない等の合意的制約をサポートする値です。
pub const DEFAULT_LOCK_WINDOW: usize = 10;

/// コインベース出力の最小ロック期間（ブロック数）。
pub const COINBASE_LOCK_WINDOW: usize = 60;

/// ブロックのターゲット時間（秒）。
pub const BLOCK_TIME: usize = 120;
