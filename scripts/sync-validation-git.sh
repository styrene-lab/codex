#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
WORK="$ROOT/target/sync-audit/git-workflow"
REMOTE="$WORK/remote.git"
A="$WORK/git-a"
B="$WORK/git-b"
FF_LOG="$WORK/ff-only.log"

rm -rf "$WORK"
mkdir -p "$WORK"
git init --bare -q "$REMOTE"

git clone -q "$REMOTE" "$A"
git -C "$A" config user.name Test
git -C "$A" config user.email test@example.com
printf '# Git Sync Fixture\n' > "$A/README.md"
git -C "$A" add README.md
git -C "$A" commit -q -m "test: seed git sync fixture"
git -C "$A" branch -M main
git -C "$A" push -q -u origin main

git clone -q "$REMOTE" "$B"
git -C "$B" config user.name Test
git -C "$B" config user.email test@example.com

git_status_clean() {
  local repo=$1
  local status
  status=$(git -C "$repo" status --porcelain=v1)
  if [[ -n "$status" ]]; then
    echo "repo not clean: $repo" >&2
    echo "$status" >&2
    exit 1
  fi
}

# A -> B fast-forward.
printf 'from A\n' > "$A/A.md"
git -C "$A" add A.md
git -C "$A" commit -q -m "test: add from A"
git -C "$A" push -q origin main
git -C "$B" pull -q --ff-only origin main
[[ "$(cat "$B/A.md")" == "from A" ]]
git_status_clean "$B"

# B -> A fast-forward.
printf 'from B\n' > "$B/B.md"
git -C "$B" add B.md
git -C "$B" commit -q -m "test: add from B"
git -C "$B" push -q origin main
git -C "$A" pull -q --ff-only origin main
[[ "$(cat "$A/B.md")" == "from B" ]]
git_status_clean "$A"

# Divergence characterization using plain git: local-only commit + remote-only commit must not be ff-able.
printf 'local divergence\n' > "$A/Local.md"
git -C "$A" add Local.md
git -C "$A" commit -q -m "test: local divergence"
printf 'remote divergence\n' > "$B/Remote.md"
git -C "$B" add Remote.md
git -C "$B" commit -q -m "test: remote divergence"
git -C "$B" push -q origin main
if git -C "$A" pull --ff-only origin main >"$FF_LOG" 2>&1; then
  echo "expected divergent pull --ff-only to fail" >&2
  exit 1
fi
[[ ! -e "$A/Remote.md" ]]
git_status_clean "$A"

echo "sync validation git: passed"
