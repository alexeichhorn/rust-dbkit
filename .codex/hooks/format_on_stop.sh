#!/usr/bin/env bash

set -euo pipefail

input="$(cat)"
session_id="$(printf '%s' "$input" | jq -r '.session_id // empty')"
cwd="$(printf '%s' "$input" | jq -r '.cwd // empty')"

if [[ -z "$session_id" || -z "$cwd" ]]; then
  exit 0
fi

repo_root="$(git -C "$cwd" rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$repo_root" ]]; then
  exit 0
fi

state_dir="$repo_root/.codex/hooks/state"
mkdir -p "$state_dir"

safe_session_id="$(printf '%s' "$session_id" | tr -c 'A-Za-z0-9._-' '_')"
current_file="$state_dir/$safe_session_id.current"
rust_list="$state_dir/$safe_session_id.rust"

(
  cd "$repo_root"
  {
    git diff --name-only
    git diff --cached --name-only
    git ls-files --others --exclude-standard
  } | sed '/^$/d' | LC_ALL=C sort -u
) > "$current_file"

if [[ ! -s "$current_file" ]]; then
  exit 0
fi

emit_block() {
  local reason="$1"
  jq -n --arg reason "$reason" '{decision: "block", reason: $reason}'
}

compact_output() {
  printf '%s' "$1" | tr '\n' ' ' | sed 's/[[:space:]]\+/ /g' | cut -c1-500
}

hash_files_from_list() {
  local list_file="$1"

  while IFS= read -r rel_path; do
    [[ -n "$rel_path" ]] || continue
    if [[ -f "$repo_root/$rel_path" ]]; then
      shasum "$repo_root/$rel_path"
    fi
  done < "$list_file" | LC_ALL=C sort
}

: > "$rust_list"

while IFS= read -r rel_path; do
  [[ -n "$rel_path" ]] || continue

  case "$rel_path" in
    target/*|*/target/*)
      ;;
    *.rs)
      if [[ -f "$repo_root/$rel_path" ]]; then
        printf '%s\n' "$rel_path" >> "$rust_list"
      fi
      ;;
  esac
done < "$current_file"

if [[ ! -s "$rust_list" ]]; then
  exit 0
fi

LC_ALL=C sort -u "$rust_list" -o "$rust_list"

if ! command -v rustfmt >/dev/null 2>&1; then
  emit_block "Auto-formatting failed before finishing. rustfmt is required to format changed Rust files."
  exit 0
fi

before_hashes="$(hash_files_from_list "$rust_list")"

rust_args=()
while IFS= read -r rel_path; do
  [[ -n "$rel_path" ]] || continue
  rust_args+=("$rel_path")
done < "$rust_list"

if ! formatter_output="$(cd "$repo_root" && rustfmt --edition 2021 "${rust_args[@]}" 2>&1)"; then
  emit_block "Auto-formatting failed before finishing. Fix or report this rustfmt error: $(compact_output "$formatter_output")"
  exit 0
fi

after_hashes="$(hash_files_from_list "$rust_list")"
if [[ "$before_hashes" != "$after_hashes" ]]; then
  changed_files="$(paste -sd ', ' "$rust_list")"
  emit_block "Auto-formatting changed Rust files: $changed_files. Review the updated diff, then finish the turn."
  exit 0
fi

exit 0
