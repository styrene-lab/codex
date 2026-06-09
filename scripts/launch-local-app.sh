#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROJECT="${1:-$ROOT/fixtures/demo-vault}"
APP="$ROOT/target/dx/flynt/debug/macos/Flynt.app"

cd "$ROOT"
dx build --macos -p flynt-app --bin flynt
cp crates/flynt-app/assets/icon.icns "$APP/Contents/Resources/icon.icns"
cp crates/flynt-app/assets/icon.icns "$APP/Contents/Resources/AppIcon.icns"
open -n "$APP" --args --project "$PROJECT"

echo "Launched $APP with project $PROJECT"
