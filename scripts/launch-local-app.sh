#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROJECT_INPUT="${1:-$ROOT/fixtures/demo-vault}"
if [[ "$PROJECT_INPUT" = /* ]]; then
  PROJECT="$PROJECT_INPUT"
else
  PROJECT="$ROOT/$PROJECT_INPUT"
fi
if [[ ! -d "$PROJECT" ]]; then
  echo "Flynt project directory does not exist: $PROJECT" >&2
  exit 2
fi
PROJECT="$(cd "$PROJECT" && pwd)"
APP="$ROOT/target/dx/flynt/debug/macos/Flynt.app"

cd "$ROOT"
if ! command -v dx >/dev/null 2>&1; then
  echo "Dioxus CLI (dx) is required for an operator-facing macOS launch." >&2
  echo "Install it with: cargo install dioxus-cli --version 0.7.9 --locked" >&2
  exit 127
fi
dx build --macos -p flynt-app --bin flynt
if [[ ! -d "$APP/Contents/Resources" ]]; then
  echo "Dioxus build completed without producing the expected app bundle: $APP" >&2
  exit 1
fi
cp crates/flynt-app/assets/icon.icns "$APP/Contents/Resources/icon.icns"
cp crates/flynt-app/assets/icon.icns "$APP/Contents/Resources/AppIcon.icns"
osascript -e 'tell application "Flynt" to quit' >/dev/null 2>&1 || true
pkill -x flynt >/dev/null 2>&1 || true
open -n "$APP" --args --project "$PROJECT"

echo "Launched $APP with project $PROJECT"
