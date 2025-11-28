# Monero Ed25519

# Monero Ed25519

Monero プロトコル内で使用される Ed25519 機能。

このライブラリは主に、私たちの API が特定の楕円曲線実装に縛られないようにすることで、
SemVer を破さずにそれをアップグレード・置き換えられるようにすることを目的としています。

このライブラリは、デフォルトで有効な `std` 機能を無効にすることで、no-std 環境下で利用可能です。

### Cargo Features

- `std` (on by default): Enables `std` (and with it, more efficient internal
  implementations).
