#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
Usage: scripts/prepare-candidate.sh SOURCE_VAULT [SNAPSHOT_PARENT]

Creates a timestamped, writable candidate snapshot without modifying SOURCE_VAULT.
The default destination parent is ~/.local/share/flynt/candidates.
EOF
  exit 2
}

[[ $# -ge 1 && $# -le 2 ]] || usage
SOURCE_INPUT="$1"
DEST_PARENT="${2:-$HOME/.local/share/flynt/candidates}"
[[ -d "$SOURCE_INPUT" ]] || { echo "Source vault does not exist: $SOURCE_INPUT" >&2; exit 2; }
SOURCE="$(cd "$SOURCE_INPUT" && pwd)"
mkdir -p "$DEST_PARENT"
DEST_PARENT="$(cd "$DEST_PARENT" && pwd)"

case "$DEST_PARENT/" in
  "$SOURCE/"*) echo "Snapshot destination must not be inside the source vault." >&2; exit 2 ;;
esac

NAME="$(basename "$SOURCE")"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
DEST="$DEST_PARENT/${NAME}-candidate-$STAMP"
if [[ -e "$DEST" ]]; then
  SUFFIX=1
  while [[ -e "${DEST}-${SUFFIX}" ]]; do
    SUFFIX=$((SUFFIX + 1))
  done
  DEST="${DEST}-${SUFFIX}"
fi

mkdir "$DEST"
# rsync preserves hidden project state. Exclude Git worktree metadata so
# Candidate cannot accidentally push commits from the canonical vault.
rsync -a --exclude='.git/' "$SOURCE/" "$DEST/"
cat > "$DEST/.flynt-candidate-source.json" <<EOF
{
  "source": $(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$SOURCE"),
  "created_at": "${STAMP}",
  "writable_snapshot": true
}
EOF

printf '%s\n' "$DEST"
