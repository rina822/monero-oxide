# Monero Bulletproofs(+)

# Monero Bulletproofs(+)

Bulletproofs(+) 範囲証明（Monero プロトコルで定義される）。

このライブラリは、デフォルトで有効な `std` 機能を無効にすることで、no-std 環境下で利用可能です。

### Cargo 機能

- `std` (デフォルト有効): `std` を有効にし、より効率的な内部実装を使用します。
- `compile-time-generators` (デフォルト有効): ジェネレータをコンパイル時に導出し、実行時の導出を不要にします。プログラムサイズを最小限に保つ必要がない場合に推奨されます。
