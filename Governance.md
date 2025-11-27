# monero-oxide Governance

# monero-oxide のガバナンス

monero-oxide プロジェクトは、二名の管理者 `kayabaNerve`（`@kayabaNerve:monero.social`）と `boog900`（`@boog900:monero.social`）によって運営されます。両者が意見を異にする場合、`j-berman` が決定のタイブレーカーを務めます。タイブレーカーは、管理者の代行が必要な場合にも務めることができますが、管理者が長期間にわたって合意できないか、いずれかの管理者が実質的に不在の場合に限るべきです。

行動規範、貢献ガイド、セキュリティポリシー、およびガバナンス規則（総称して「ガバナンス文書」）は常に順守されるものとします。

プロジェクトに対する全ての変更（ガバナンスの変更を含む）は、プルリクエストを通じて行われます。

## 議論の場

monero-oxide に関する議論は、monero-oxide 専用チャネルが作成されるまで `#cuprate:monero.social` で行われます。

## ガバナンスの変更

ガバナンス文書の変更は両管理者の合意を必要とします。行動規範およびガバナンス規則の変更は、コミュニティによる公開コメント期間の後にのみマージされるべきです。

## 機能追加

新機能の追加は両管理者の合意を必要とします。新機能を実装する PR は同様の審査を受けます。

## バグ修正とドキュメント

バグ修正およびドキュメントの変更は、単一の管理者による PR レビューで十分です。

## リリース

リリースの実行は両管理者の合意が必要であり、[Rust の SemVer 定義](https://doc.rust-lang.org/cargo/reference/semver.html) に従うものとします。
メジャーバージョンのリリースは 1 年のサポート目標を持つ必要があります。以下の場合のみ例外が許されます:
- Monero のハードフォーク（または同等の重大な変更）がそれを要求する場合
- 新しいメジャーバージョンでのみ適切に解決できる重大な問題が発見された場合

The Code of Conduct, Contributing guide, Security policy, and Governance rules
(collectively the "governance documents") are to be followed at all times.

All changes to the project, including its governance, are to be done via pull
requests.

## Place of Discussion

monero-oxide discussions are to be held in `#cuprate:monero.social` pending
the creation of a channel for monero-oxide itself.

## Governance Changes

Changes to the governance documents are to be agreed upon by both
administrators. Changes to the Code of Conduct and Governance rules must only
be merged after an open comment period by the community.

## Feature Additions

New features are to be agreed upon by both administrators. The PR implementing
them is to be reviewed in the same fashion.

## Bug Fixes and Documentation

Bug fixes and documentation only require a single administrator to perform the
PR review.

## Releases

Cut releases must be agreed upon by both administrators and follow
[Rust's definition of SemVer](https://doc.rust-lang.org/cargo/reference/semver.html).
Any major version release must have one-year support target. The only allowed
exceptions to this rule are:
- If a Monero hard-fork (or other such significant change) mandates it
- If a critical issue is found which can only be properly resolved with a new
  major version
