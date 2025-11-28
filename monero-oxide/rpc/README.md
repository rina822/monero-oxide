# Monero RPC

# Monero RPC クライアント

Monero デーモンへの RPC 接続用トレイト。monero-oxide を中心に構築されています。

このライブラリは、デフォルトで有効な `std` 機能を無効にすることで、no-std 環境下で利用可能です。

### Cargo 機能

- `std` (デフォルトで有効): `std` を有効にし、より効率的な内部実装を提供します。
