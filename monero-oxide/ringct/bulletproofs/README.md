# Monero Bulletproofs(+)

# Monero Bulletproofs(+)

Bulletproofs(+) 範囲証明（Monero プロトコルで定義される）。

このライブラリは、デフォルトで有効な `std` 機能を無効にすることで、no-std 環境下で利用可能です。

### Cargo 機能

- `std` (デフォルトで有効): `std` を有効にし、より効率的な内部実装を提供します。
  implementations).
- `compile-time-generators` (on by default): Derives the generators at
  compile-time so they don't need to be derived at runtime. This is recommended
  if program size doesn't need to be kept minimal.
