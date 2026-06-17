# Contributing to Nagi

## ブランチ・マージ方針

`main` の履歴は常に直線（linear history）に保つ。マージコミットは作らない。

### なぜ

- 履歴が一本道になり、変更の順序と各コミットの差分を追いやすい。
- `git bisect` や `git log` が直感的に働き、問題の混入箇所を特定しやすい。
- リバート・チェリーピックの単位が明確になる。

### どうやって

- PR のマージは **Squash and merge** または **Rebase and merge** のみを使う。リポジトリ設定で merge commit は無効化してある。
- `main` には「直線履歴必須（require linear history）」の ruleset を適用しており、マージコミットを含む更新はサーバー側で拒否される。
- ローカルでも `git pull` / `git merge` が意図せずマージコミットを作らないよう、このリポジトリには次を設定済み。別のマシンで clone した場合は同じ設定を入れること。

  ```sh
  git config merge.ff only
  git config pull.ff only
  ```

## Issue / PR / コミットの対応規約

1 つの Issue → 1 つの PR → 1 つのコミットを一対一で対応させ、タイトルを一貫させる。CI がこれを検証し、外れた PR はマージできない。

### ルール

- **1 PR = 1 commit**: PR は必ず 1 コミットにまとめる（複数になったら squash する）。
- **コミット件名 = PR タイトル**: コミットの件名は PR タイトルで始め、末尾はピリオドにする。
  - 例: PR タイトル `[iOS] 設定画面を追加` → コミット件名 `[iOS] 設定画面を追加.`
- **カテゴリ接頭辞**: Issue・PR のタイトルは `[カテゴリ]` で始める。許可カテゴリは [hack/prefix.yaml](hack/prefix.yaml) を唯一の出所として管理する（`iOS` / `Rust` / `Android` / `CI/CD` / `Docs` / `Infra` / `Chore`。複合は `[iOS/Rust]`）。`Chore` はツール・設定・雑務（`.gitignore`、formatter、依存更新など）。
- **Issue との紐付け**: PR 本文で `Closes #<番号>` 等の closing キーワードを使い、対応 Issue を必ず参照する（マージで Issue が閉じる）。

### なぜ

- Issue・PR・コミットのタイトルが一致することで、`git log` だけで「どの Issue のどの対応か」を一目で追える。
- 1 PR = 1 commit により、`main` の各コミットがレビュー済みの変更単位と一致し、revert やリリースノート生成が単純になる。
- closing キーワードで Issue が自動的に閉じ、課題管理と実装履歴の乖離を防ぐ。

### 検証の仕組み

- [.github/workflows/validate_pr.yaml](.github/workflows/validate_pr.yaml) … 上記ルールを PR ごとに検証する（[hack/validate_pr.sh](hack/validate_pr.sh)）。ローカルでも、open な PR があるブランチ上で `sh hack/validate_pr.sh` を実行して事前確認できる。
- [.github/workflows/validate_issue_title.yaml](.github/workflows/validate_issue_title.yaml) … Issue タイトルの接頭辞を検証する（[hack/validate_issue_title.sh](hack/validate_issue_title.sh)）。
