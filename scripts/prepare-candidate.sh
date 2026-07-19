#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
usage() {
  cat >&2 <<'EOF'
Usage: scripts/prepare-candidate.sh SOURCE_VAULT [SNAPSHOT_PARENT]

Creates and verifies a timestamped writable Candidate snapshot. The default
snapshot parent is ~/.local/share/flynt/candidates. Set FLYNT_CANDIDATE_RETAIN
to control per-vault retention (default: 5).
EOF
  exit 2
}

[[ $# -ge 1 && $# -le 2 ]] || usage
SOURCE_INPUT="$1"
DEST_PARENT="${2:-$HOME/.local/share/flynt/candidates}"
RETAIN="${FLYNT_CANDIDATE_RETAIN:-5}"

exec python3 "$ROOT/scripts/candidate-snapshot.py" create \
  "$SOURCE_INPUT" "$DEST_PARENT" --retain "$RETAIN"
