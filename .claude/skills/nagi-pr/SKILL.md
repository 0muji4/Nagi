---
name: nagi-pr
description: >-
  Nagi リポジトリで、今ローカルにある変更を規約準拠の Pull Request にまとめる。
  ユーザーが変更を push / ship / land / PR したいとき必ず使う。例:
  「今ローカルにある修正を push したい」「push して」「PR にして」「この変更まとめて」
  「ローカルの変更を PR にして」「make a PR for my changes」「これ ready なので出したい」。
  git status / diff を見て論点ごとに 1 PR に分割し、hack/prefix.yaml から [カテゴリ] 接頭辞を選び、
  規約・用語の不整合を点検し、Rust 変更なら事前検証し、実行コマンド列を生成してユーザーに渡す。
  Claude は git/gh の状態変更を実行せず、コマンドを渡すだけ。「push」「PR」だけの短い依頼でも発動する。
---

# Nagi: ローカル変更を規約準拠 PR にする

## 大原則: あなたは「準備」、ユーザーが「実行」

Claude は変更を調査し、ドキュメント/コードの不整合を編集で直し、**正確なコマンド列を出力する**。
commit・push・branch 作成・PR/Issue 作成・merge といった **git/gh の状態変更はユーザー自身が実行する**。
これらを Claude が勝手に実行してはいけない（このリポジトリの確立された運用方針）。
`git status` / `diff` / `log` などの読み取り確認は行ってよい。

## 規約（出所）

詳細は [CONTRIBUTING.md](../../../CONTRIBUTING.md) と [hack/prefix.yaml](../../../hack/prefix.yaml)。CI が全 PR で強制する。生成するコマンドは必ず次を満たすこと:

- **直線履歴**: マージコミット禁止。マージは必ず `--rebase`。
- **1 PR = 1 commit**: 複数論点なら PR を分ける。1 PR の中は 1 コミット。
- **コミット件名 = PR タイトル + 末尾ピリオド**。`TITLE` 変数で完全一致させ、手打ちのズレを防ぐ。
- **`[カテゴリ]` 接頭辞**: Issue・PR・コミットのタイトルに付ける。許可カテゴリは `hack/prefix.yaml` が唯一の出所。
- **`Closes #<番号>`**: PR 本文で対応 Issue を必ず参照する。
- **マージは `--rebase`（`--squash` 不可）**: squash は GitHub が件名を再生成して末尾ピリオドを落とし、規約と不整合になる。rebase なら検証済みの 1 コミットがそのまま main に載る。

## 手順

### 1. 現状を調べる（読み取り）

```sh
git rev-parse --abbrev-ref HEAD
git status --short
git diff --stat && git diff            # 変更の中身を把握
git rev-list --left-right --count origin/main...main
```

未追跡ディレクトリは `git add -n <dir>` で「実際に何が追加されるか」を dry-run 確認する（`.gitignore` の除外を反映するため）。

### 2. 論点ごとに分割する（1 PR = 1 論点・単一カテゴリ）

変更が複数の無関係な関心事にまたがるなら、**PR を分ける**。関連する変更（例: 1 つの用語統一が doc とコメントにまたがる）は **1 PR にまとめる**——分けると 1 つの変更が複数 PR になり、かえって規約の趣旨に反する。
複数 PR になる場合は順序を明示し、各 PR をマージしてから次へ進む流れで出す。

### 3. カテゴリを選ぶ

`hack/prefix.yaml` を読んで現在の許可カテゴリから選ぶ（ハードコードしない。ドリフトするため）。
現状: `iOS` / `Rust` / `Android` / `CI/CD` / `Docs` / `Infra` / `Chore`。複合は `[iOS/Rust]`。
迷ったら境界の使い分け: `CI/CD`=パイプライン、`Infra`=クラウド/デプロイ、`Chore`=ツール・設定・雑務（`.gitignore`・formatter・依存更新）。

### 4. 不整合を点検する

push 前に、変更が既存の規約・文書と矛盾しないか確認する。よくある例:

- **用語ドリフト**: 過去に統一した語の言い換えが新規ファイルに混入していないか。`grep -rn '<旧語>' <変更パス>` で確認。
- **CONTRIBUTING.md / README の開発フローとの矛盾**。

見つけたら **Claude がファイル編集で直す**（git 操作ではない）。直したファイルも同じ PR に含める。

### 5. 事前検証（Rust 変更のとき）

`rust/**` を触る PR は Rust CI が走る。赤を避けるため、push 前にローカルで同一検証を勧める:

```sh
cd rust && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
```

### 6. コマンド列を生成して渡す

論点ごとに次のテンプレートを埋めて出す。`TITLE` は接頭辞付き・ピリオドなし。Issue 番号は自動取得。`<files>` はその論点のファイルだけ（`git add -A` や `.` は使わない。`.gitignore` 除外も尊重）。

```sh
cd /Users/motoshi.suzuki/go/src/github.com/0muji4/Nagi

TITLE='[カテゴリ] 変更の要約'
ISSUE_URL=$(gh issue create --title "$TITLE" --body '<Issue 本文>')
ISSUE_NUM=$(basename "$ISSUE_URL"); echo "issue=#$ISSUE_NUM"

git switch -c <category>/<slug>
git add <files>
git status --short                      # 対象ファイルだけ staged か確認

git commit -m "${TITLE}." -m '<本文・任意>'
git push -u origin <category>/<slug>

gh pr create --base main --head <category>/<slug> --title "$TITLE" --body "Closes #${ISSUE_NUM}"
gh pr checks --watch
```

CI が緑になったら（別ブロックで渡す。緑確認後に実行させる）:

```sh
gh pr merge --rebase --delete-branch
git switch main && git pull
```

## ブートストラップの注意

`hack/prefix.yaml` に**新カテゴリを追加する**変更のとき、その Issue 作成時点では新カテゴリがまだ main に無く、issue タイトル検証 workflow（main を参照）が赤くなる。その PR は**既存カテゴリ**（例 `[CI/CD]`）で出し、**最初にマージ**する。以降の PR で新カテゴリが使える。

## ブランチ名

`<category-lowercase>/<slug>`（例 `docs/expand-readme`）。Issue 番号を入れる運用なら `<category>/<issue#>-<slug>`。
CI 強制はしていないが、マージ済み PR 記録に残るので一貫させる。
