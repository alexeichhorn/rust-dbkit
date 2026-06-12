#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$repo_root" ]]; then
  exit 0
fi

codex_home="${CODEX_HOME:-$HOME/.codex}"
codex_worktrees="$codex_home/worktrees"

case "$repo_root" in
  "$codex_worktrees"/*) ;;
  *) exit 0 ;;
esac

source_root=""
while IFS= read -r line; do
  case "$line" in
    worktree\ *)
      candidate="${line#worktree }"
      if [[ "$candidate" == "$repo_root" ]]; then
        continue
      fi
      case "$candidate" in
        "$codex_worktrees"/*) continue ;;
      esac
      if [[ -d "$candidate" ]]; then
        source_root="$candidate"
        break
      fi
      ;;
  esac
done < <(git -C "$repo_root" worktree list --porcelain)

if [[ -z "$source_root" ]]; then
  exit 0
fi

while IFS= read -r -d '' source_file; do
  relative_path="${source_file#"$source_root"/}"
  target_file="$repo_root/$relative_path"

  if [[ -e "$target_file" ]]; then
    continue
  fi

  mkdir -p "$(dirname "$target_file")"
  cp "$source_file" "$target_file"
  chmod 600 "$target_file"
  printf 'Copied %s\n' "$relative_path"
done < <(
  find "$source_root" \
    \( -name .git -o -name node_modules -o -name .venv -o -name dist -o -name build -o -name .next -o -name target -o -name .wrangler \) -prune \
    -o \( -name .env -o -name .dev.vars \) -type f -print0
)
