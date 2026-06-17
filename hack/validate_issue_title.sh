#!/bin/sh
# Issue タイトルが許可カテゴリ接頭辞で始まることを検証する（hack/prefix.yaml）。
# 詳細は CONTRIBUTING.md。
#   CI    : ISSUE_TITLE を env で渡す。
#   ローカル: ISSUE_TITLE='[iOS] ...' sh hack/validate_issue_title.sh
set -eu

dir=$(CDPATH= cd "$(dirname "$0")" && pwd)

error() { printf '\033[1;31mError: %s\033[0m\n' "$1" >&2; }

title="${ISSUE_TITLE:?ISSUE_TITLE が必要}"

cats=$(grep -E '^[[:space:]]*-[[:space:]]+' "$dir/prefix.yaml" | sed -E 's/^[[:space:]]*-[[:space:]]+//' | paste -sd '|' -)
if ! printf '%s' "$title" | grep -Eq "^\\[(${cats})(/(${cats}))*\\][[:space:]]"; then
  error "Issue タイトルはカテゴリ接頭辞で始める。許可: [${cats}]
  例: [iOS] 設定画面を追加"
  exit 1
fi

printf 'Issue title OK: %s\n' "$title"
