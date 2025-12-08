# Monero CLSAG

# Monero CLSAG

Monero プロトコルで定義される CLSAG（可鎖接可能リング署名）。
また、[FROSTLASS](
  https://github.com/monero-oxide/monero-oxide/tree/main/audits/FROSTLASS
)（FROST にインスパイアされた識別可能なアボート機能を持つしきい値マルチシグアルゴリズム）の実装も含まれています。

このライブラリは、デフォルトで有効な `std` 機能を無効にすることで、no-std 環境下で利用可能です。
disabled.

### Cargo 機能

- `std` (デフォルト有効): `std` を有効にし、より効率的な内部実装を使用します。
- `compile-time-generators` (デフォルト有効): ジェネレータをコンパイル時に導出し、実行時の導出を不要にします。プログラムサイズを最小限に保つ必要がない場合に推奨されます。
- `multisig`: FROST インスパイアの閾値マルチシグアルゴリズムを提供します。この機能は SemVer でカバーされていません（マイナーバージョンに沿って）。
