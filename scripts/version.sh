#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
git_root="$(git -C "$project_root" rev-parse --show-toplevel 2>/dev/null || true)"

if [[ "$git_root" != "$project_root" ]]; then
  printf '%s-nogit\n' "$(date -u +%Y%m%d)"
  exit 0
fi

tag="$(git -C "$project_root" describe --tags --exact-match HEAD 2>/dev/null || true)"
if [[ "$tag" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  printf '%s\n' "$tag"
  exit 0
fi

commit_date="$(git -C "$project_root" show -s --format=%cs HEAD | tr -d '-')"
short_commit="$(git -C "$project_root" rev-parse --short=8 HEAD)"
printf '%s-%s\n' "$commit_date" "$short_commit"

