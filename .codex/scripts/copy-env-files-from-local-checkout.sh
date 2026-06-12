#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$repo_root" ]]; then
  exit 0
fi
repo_root="$(cd "$repo_root" && pwd -P)"

codex_home="${CODEX_HOME:-$HOME/.codex}"
codex_home="${codex_home%/}"
if [[ -d "$codex_home" ]]; then
  codex_home="$(cd "$codex_home" && pwd -P)"
fi
codex_worktrees_dir="$codex_home/worktrees"

case "$repo_root" in
  "$codex_worktrees_dir"/*) ;;
  *) exit 0 ;;
esac

common_git_dir="$(cd "$(git rev-parse --git-common-dir)" && pwd -P)"
source_root="$(cd "$common_git_dir/.." && pwd -P)"

if [[ "$source_root" == "$repo_root" || ! -d "$source_root" ]]; then
  exit 0
fi

while IFS= read -r -d '' source_file; do
  relative_path="${source_file#"$source_root"/}"
  target_file="$repo_root/$relative_path"

  if [[ -e "$target_file" || -L "$target_file" ]]; then
    continue
  fi

  mkdir -p "$(dirname "$target_file")"
  install -m 600 "$source_file" "$target_file"
  echo "Copied $relative_path"
done < <(
  find "$source_root" \
    \( -type d \( \
      -name .git -o \
      -name node_modules -o \
      -name .venv -o \
      -name dist -o \
      -name build -o \
      -name .next -o \
      -name target -o \
      -name .wrangler \
    \) -prune \) -o \
    \( -type f \( -name .env -o -name .dev.vars \) -print0 \)
)
