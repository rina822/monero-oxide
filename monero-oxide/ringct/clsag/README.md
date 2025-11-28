# Monero CLSAG

# Monero CLSAG

Monero プロトコルで定義される CLSAG（可鎖接可能リング署名）。
また、[FROSTLASS](
  https://github.com/monero-oxide/monero-oxide/tree/main/audits/FROSTLASS
)（FROST にインスパイアされた識別可能なアボート機能を持つしきい値マルチシグアルゴリズム）の実装も含まれています。

このライブラリは、デフォルトで有効な `std` 機能を無効にすることで、no-std 環境下で利用可能です。
disabled.

### Cargo Features

- `std` (on by default): Enables `std` (and with it, more efficient internal
  implementations).
- `compile-time-generators` (on by default): Derives (expansions of) generators
  at compile-time so they don't need to be derived at runtime. This is
  recommended if program size doesn't need to be kept minimal.
- `multisig`: Provides a FROST-inspired threshold multisignature algorithm for
  use. This functionality is not covered by SemVer, except along minor
  versions.
