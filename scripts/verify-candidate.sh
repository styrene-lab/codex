#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SNAPSHOT="${1:-}"
[[ -n "$SNAPSHOT" ]] || { echo "Usage: scripts/verify-candidate.sh SNAPSHOT" >&2; exit 2; }

python3 "$ROOT/scripts/candidate-snapshot.py" verify "$SNAPSHOT"
