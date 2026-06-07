#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT"

mkdir -p audits target/sync-audit

require_clean_except_gitignore() {
  local report=$1
  python3 - "$report" <<'PY'
import json, sys
path = sys.argv[1]
data = json.load(open(path))
allowed_added = {".gitignore"}
added = set(data["added"])
modified = data["modified"]
removed = data["removed"]
if not added.issubset(allowed_added) or modified or removed:
    print(f"FAILED {path}")
    print("added", data["added"])
    print("modified", modified)
    print("removed", removed)
    sys.exit(1)
print(f"ok {path}: added={data['added']} modified=[] removed=[]")
PY
}

require_no_changes() {
  local report=$1
  python3 - "$report" <<'PY'
import json, sys
path = sys.argv[1]
data = json.load(open(path))
if data["added"] or data["modified"] or data["removed"]:
    print(f"FAILED {path}")
    print("added", data["added"])
    print("modified", data["modified"])
    print("removed", data["removed"])
    sys.exit(1)
print(f"ok {path}: no changes")
PY
}

seed_git_repo() {
  local dir=$1
  rm -rf "$dir"
  mkdir -p "$dir"
  git -C "$dir" init -q
  git -C "$dir" config user.name Test
  git -C "$dir" config user.email test@example.com
}

# Existing Git folder: first open may add managed .gitignore only.
EXISTING="$ROOT/target/sync-audit/local-existing-folder"
seed_git_repo "$EXISTING"
printf '# Existing Folder\n' > "$EXISTING/README.md"
git -C "$EXISTING" add README.md
git -C "$EXISTING" commit -q -m "test: seed existing folder"
EXISTING_REPORT="$ROOT/audits/local-existing-folder-open-idle.json"
scripts/flynt-open-idle-audit.py --duration 18 --json-output "$EXISTING_REPORT" "$EXISTING" -- cargo run -p flynt-app
require_clean_except_gitignore "$EXISTING_REPORT"

# Repeated open including local state should be fully clean after .gitignore already exists.
EXISTING_STATE_REPORT="$ROOT/audits/local-existing-folder-open-idle-with-state.json"
scripts/flynt-open-idle-audit.py --duration 12 --include-local-state --json-output "$EXISTING_STATE_REPORT" "$EXISTING" -- cargo run -p flynt-app
require_no_changes "$EXISTING_STATE_REPORT"

# Drawing wrapper/view fixture. This is still a timed open; manually open the drawing if needed.
DRAWING="$ROOT/target/sync-audit/local-drawing-noop"
seed_git_repo "$DRAWING"
mkdir -p "$DRAWING/drawings"
cat > "$DRAWING/drawings/Test.excalidraw" <<'EOF'
{
  "type": "excalidraw",
  "version": 2,
  "source": "flynt-sync-validation",
  "elements": [],
  "appState": { "viewBackgroundColor": "#06080e" },
  "files": {}
}
EOF
cat > "$DRAWING/drawings/Test.md" <<'EOF'
+++
title = "Test Drawing"
tags = ["drawing"]
+++

![[drawings/Test.excalidraw]]
EOF
git -C "$DRAWING" add drawings/Test.excalidraw drawings/Test.md
git -C "$DRAWING" commit -q -m "test: seed drawing no-op"
DRAWING_REPORT="$ROOT/audits/local-drawing-noop-open-idle.json"
scripts/flynt-open-idle-audit.py --duration 18 --json-output "$DRAWING_REPORT" "$DRAWING" -- cargo run -p flynt-app
require_clean_except_gitignore "$DRAWING_REPORT"

# Non-Git iCloud-like folder should have zero idle-open mutations.
NONGIT="$ROOT/target/sync-audit/local-non-git-folder"
rm -rf "$NONGIT"
mkdir -p "$NONGIT"
printf '# Non Git Folder\n' > "$NONGIT/README.md"
NONGIT_REPORT="$ROOT/audits/local-non-git-folder-open-idle.json"
scripts/flynt-open-idle-audit.py --duration 12 --include-local-state --json-output "$NONGIT_REPORT" "$NONGIT" -- cargo run -p flynt-app
require_no_changes "$NONGIT_REPORT"

echo "sync validation local: passed"
