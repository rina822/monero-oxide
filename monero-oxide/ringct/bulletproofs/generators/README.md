# Monero Bulletproofs(+) Generators

# Monero Bulletproofs(+) ジェネレータ

Monero が Bulletproofs(+) をインスタンス化するために使用するジェネレータ。これはセマンティックバージョニングの保証や公開 API なしの内部クレートです。

このライブラリは、デフォルトで有効な `std` 機能を無効にすることで、no-std 環境下で利用可能です。

### Cargo 機能

- `std` (デフォルトで有効): `std` を有効にし、より効率的な内部実装を提供します。
