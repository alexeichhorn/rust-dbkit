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
baseline_file="$state_dir/$safe_session_id.baseline"

(
  cd "$repo_root"
  {
    git diff --name-only
    git diff --cached --name-only
    git ls-files --others --exclude-standard
  } | sed '/^$/d' | LC_ALL=C sort -u
) > "$baseline_file"
