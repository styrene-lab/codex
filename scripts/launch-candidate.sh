#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE_VAULT="${1:-}"
[[ -n "$SOURCE_VAULT" ]] || { echo "Usage: scripts/launch-candidate.sh SOURCE_VAULT [SNAPSHOT_PARENT]" >&2; exit 2; }
SNAPSHOT_PARENT="${2:-$HOME/.local/share/flynt/candidates}"
PROJECT="$($ROOT/scripts/prepare-candidate.sh "$SOURCE_VAULT" "$SNAPSHOT_PARENT")"
"$ROOT/scripts/candidate-smoke.sh" "$PROJECT"
BASE_APP="$ROOT/target/dx/flynt/release/macos/Flynt.app"
CANDIDATE_APP="$ROOT/target/dx/flynt/release/macos/Flynt Candidate.app"
PROFILE="${FLYNT_CANDIDATE_LAUNCHER_PROFILE:-$HOME/Library/Application Support/io.styrene.flynt.candidate/launcher-profile.json}"

cd "$ROOT"
FLYNT_BUILD_IDENTITY=candidate dx build --release --macos -p flynt-app --bin flynt
[[ -d "$BASE_APP/Contents/Resources" ]] || { echo "Candidate bundle was not produced: $BASE_APP" >&2; exit 1; }
rm -rf "$CANDIDATE_APP"
mv "$BASE_APP" "$CANDIDATE_APP"
cp crates/flynt-app/assets/icon.icns "$CANDIDATE_APP/Contents/Resources/icon.icns"
cp crates/flynt-app/assets/icon.icns "$CANDIDATE_APP/Contents/Resources/AppIcon.icns"
/usr/libexec/PlistBuddy -c 'Set :CFBundleIdentifier io.styrene.flynt.candidate' "$CANDIDATE_APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleName Flynt Candidate' "$CANDIDATE_APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleDisplayName Flynt Candidate' "$CANDIDATE_APP/Contents/Info.plist" 2>/dev/null || \
  /usr/libexec/PlistBuddy -c 'Add :CFBundleDisplayName string Flynt Candidate' "$CANDIDATE_APP/Contents/Info.plist"
osascript -e 'tell application id "io.styrene.flynt.candidate" to quit' >/dev/null 2>&1 || true
open -n "$CANDIDATE_APP" --env "FLYNT_LAUNCHER_PROFILE=$PROFILE" --args --project "$PROJECT"

echo "Launched Candidate against snapshot $PROJECT"
