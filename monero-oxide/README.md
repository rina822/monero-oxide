# monero-oxide

# monero-oxide

モダン Monero トランザクションライブラリ。Monero プロトコルへの最新の Rust フレンドリーなビューを提供します。

このライブラリは、デフォルトで有効な `std` 機能を無効にすることで、no-std 環境下で利用可能です。

ライブラリの推奨使用方法は、リリースビルドでさえ `overflow-checks = true` で行うことです。

### Cargo Features

- `std` (on by default): Enables `std` (and with it, more efficient internal
  implementations).
- `compile-time-generators` (on by default): Derives the generators at
  compile-time so they don't need to be derived at runtime. This is recommended
  if program size doesn't need to be kept minimal.
- `multisig`: Enables the `multisig` feature for all dependencies.
