#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROJECT="${1:-}"
[[ -n "$PROJECT" ]] || { echo "Usage: scripts/candidate-smoke.sh SNAPSHOT" >&2; exit 2; }
[[ -d "$PROJECT" ]] || { echo "Candidate snapshot does not exist: $PROJECT" >&2; exit 2; }
PROJECT="$(cd "$PROJECT" && pwd)"

python3 "$ROOT/scripts/candidate-snapshot.py" verify "$PROJECT"
[[ ! -e "$PROJECT/.git" ]] || { echo "Candidate snapshot unexpectedly contains .git" >&2; exit 1; }
[[ -d "$PROJECT/.flynt" ]] || { echo "Candidate snapshot has no .flynt project metadata" >&2; exit 1; }
[[ -f "$PROJECT/.flynt/config.toml" ]] || { echo "Candidate snapshot has no .flynt/config.toml" >&2; exit 1; }

# Opening through flynt-store exercises config parsing, index/database startup,
# migrations, and a full reindex without launching or disturbing any GUI app.
FLYNT_CANDIDATE_SMOKE_PROJECT="$PROJECT" cargo test -p flynt-store \
  --test candidate_snapshot_smoke candidate_snapshot_opens_and_reindexes -- --exact --nocapture

echo "Candidate smoke validation passed: $PROJECT"
