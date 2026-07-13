#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
NAME="Flynt-QBF-${VERSION}-macos"
STAGE="$ROOT/dist/$NAME"
ZIP="$ROOT/dist/$NAME.zip"
APP_SOURCE="${FLYNT_APP_SOURCE:-$ROOT/dist/Flynt.app}"
PROJECT_SOURCE="$ROOT/fixtures/demo-vault"
MAX_APP_BYTES="${FLYNT_MAX_COWORKER_APP_BYTES:-83886080}"
MAX_ZIP_BYTES="${FLYNT_MAX_COWORKER_ZIP_BYTES:-31457280}"

app_size_bytes() {
  du -sk "$1" | awk '{print $1 * 1024}'
}

reject_duplicate_hashed_assets() {
  local assets="$1/Contents/Resources/assets"
  [[ -d "$assets" ]] || { echo "Missing generated asset directory: $assets" >&2; exit 1; }
  local duplicates
  duplicates="$(find "$assets" -maxdepth 1 -type f -print | \
    sed -E 's#^.*/##; s/-dxh[0-9a-f]+(\.[^.]+)$/\1/' | \
    sort | uniq -d)"
  if [[ -n "$duplicates" ]]; then
    echo "App bundle contains duplicate generated assets (stale Dioxus output):" >&2
    printf '  %s\n' "$duplicates" >&2
    exit 1
  fi
}

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Coworker demo packaging currently supports macOS only." >&2
  exit 1
fi
if [[ "$(uname -m)" != "arm64" ]]; then
  echo "This package is for Apple-silicon Macs; build host is $(uname -m)." >&2
  exit 1
fi

if [[ "${1:-}" != "--use-existing-app" ]]; then
  (cd "$ROOT" && just sign)
fi

[[ -d "$APP_SOURCE" ]] || { echo "Missing app bundle: $APP_SOURCE" >&2; exit 1; }
python3 "$ROOT/scripts/check-demo-vault.py"

APP_VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP_SOURCE/Contents/Info.plist")"
[[ "$APP_VERSION" == "$VERSION" ]] || {
  echo "App version $APP_VERSION does not match workspace version $VERSION" >&2
  exit 1
}
codesign --verify --deep --strict --verbose=2 "$APP_SOURCE"

APP_BYTES="$(app_size_bytes "$APP_SOURCE")"
if (( APP_BYTES > MAX_APP_BYTES )); then
  echo "App bundle is $APP_BYTES bytes; limit is $MAX_APP_BYTES. Coworker packages require a clean release build." >&2
  exit 1
fi
reject_duplicate_hashed_assets "$APP_SOURCE"

rm -rf "$STAGE" "$ZIP"
mkdir -p "$STAGE"
ditto --norsrc --noextattr "$APP_SOURCE" "$STAGE/Flynt.app"
# Copy the versioned fixture, then remove state that must not cross machines.
ditto --norsrc --noextattr "$PROJECT_SOURCE" "$STAGE/Quick Brown Fox"
rm -rf \
  "$STAGE/Quick Brown Fox/.git" \
  "$STAGE/Quick Brown Fox/.flynt/local" \
  "$STAGE/Quick Brown Fox/.flynt/runtime"
find "$STAGE/Quick Brown Fox" -name .DS_Store -delete

cat > "$STAGE/Open Flynt Demo.command" <<'LAUNCHER'
#!/bin/bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
open "$HERE/Flynt.app" --args --project "$HERE/Quick Brown Fox"
LAUNCHER
chmod +x "$STAGE/Open Flynt Demo.command"

cat > "$STAGE/README.txt" <<README
FLYNT QUICK BROWN FOX DEMO

Requirements: Apple-silicon Mac (M1 or newer), macOS 13 or newer.

1. Move this entire folder to Documents. Keep its contents together.
2. Double-click “Open Flynt Demo.command”.
3. If macOS asks whether to open Flynt, choose Open.

The launcher opens the included “Quick Brown Fox” project directly. Changes you
make are saved inside that project folder. The demo does not require Node.js,
Rust, Homebrew, or a terminal setup.

Build: Flynt $VERSION ($(git -C "$ROOT" rev-parse --short HEAD))
README

# Preserve the app bundle while omitting Finder AppleDouble (__MACOSX) debris.
COPYFILE_DISABLE=1 ditto -c -k --keepParent --norsrc --noextattr "$STAGE" "$ZIP"
ZIP_BYTES="$(python3 -c 'import os, sys; print(os.path.getsize(sys.argv[1]))' "$ZIP")"
if (( ZIP_BYTES > MAX_ZIP_BYTES )); then
  echo "Coworker ZIP is $ZIP_BYTES bytes; limit is $MAX_ZIP_BYTES." >&2
  exit 1
fi
python3 "$ROOT/scripts/check-coworker-package.py" "$ZIP"
shasum -a 256 "$ZIP" | tee "$ZIP.sha256"
echo "Created $ZIP"
