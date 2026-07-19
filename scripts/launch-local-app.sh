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
DEV_APP="$ROOT/target/dx/flynt/debug/macos/Flynt Dev.app"
DEV_BUNDLE_ID="io.styrene.flynt.dev"
DEV_PROFILE="${FLYNT_DEV_LAUNCHER_PROFILE:-$HOME/Library/Application Support/io.styrene.flynt.dev/launcher-profile.json}"

cd "$ROOT"
if ! command -v dx >/dev/null 2>&1; then
  echo "Dioxus CLI (dx) is required for an operator-facing macOS launch." >&2
  echo "Install it with: cargo install dioxus-cli --version 0.8.0-alpha.0 --locked" >&2
  exit 127
fi
DX_VERSION="$(dx --version 2>/dev/null || true)"
if [[ "$DX_VERSION" != dioxus\ 0.8.0-alpha.0* ]]; then
  echo "Flynt requires Dioxus CLI 0.8.0-alpha.0; found: ${DX_VERSION:-unknown}" >&2
  echo "Install it with: cargo install dioxus-cli --version 0.8.0-alpha.0 --locked --force" >&2
  exit 2
fi
FLYNT_BUILD_IDENTITY=dev dx build --macos -p flynt-app --bin flynt
if [[ ! -d "$APP/Contents/Resources" ]]; then
  echo "Dioxus build completed without producing the expected app bundle: $APP" >&2
  exit 1
fi
rm -rf "$DEV_APP"
mv "$APP" "$DEV_APP"
cp crates/flynt-app/assets/icon.icns "$DEV_APP/Contents/Resources/icon.icns"
cp crates/flynt-app/assets/icon.icns "$DEV_APP/Contents/Resources/AppIcon.icns"
/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier $DEV_BUNDLE_ID" "$DEV_APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleName Flynt Dev" "$DEV_APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleDisplayName Flynt Dev" "$DEV_APP/Contents/Info.plist" 2>/dev/null || \
  /usr/libexec/PlistBuddy -c "Add :CFBundleDisplayName string Flynt Dev" "$DEV_APP/Contents/Info.plist"

# Target only the Dev bundle. Never terminate the installed Stable app by process name.
osascript -e 'tell application id "io.styrene.flynt.dev" to quit' >/dev/null 2>&1 || true
open -n "$DEV_APP" --env "FLYNT_LAUNCHER_PROFILE=$DEV_PROFILE" --args --project "$PROJECT"

echo "Launched $DEV_APP with isolated profile $DEV_PROFILE and project $PROJECT"
