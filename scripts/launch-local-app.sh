#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROJECT_INPUT="${1:-$ROOT/fixtures/demo-vault}"
if [[ "$PROJECT_INPUT" = /* ]]; then
  PROJECT="$PROJECT_INPUT"
else
  PROJECT="$ROOT/$PROJECT_INPUT"
fi
PROJECT="$(cd "$PROJECT" && pwd)"
APP="$ROOT/target/dx/flynt/debug/macos/Flynt.app"

cd "$ROOT"
dx build --macos -p flynt-app --bin flynt
cp crates/flynt-app/assets/icon.icns "$APP/Contents/Resources/icon.icns"
cp crates/flynt-app/assets/icon.icns "$APP/Contents/Resources/AppIcon.icns"
osascript -e 'tell application "Flynt" to quit' >/dev/null 2>&1 || true
pkill -x flynt >/dev/null 2>&1 || true
open -n "$APP" --args --project "$PROJECT"

echo "Launched $APP with project $PROJECT"
