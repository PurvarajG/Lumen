#!/bin/sh
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PROJECT_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
PATH="/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:$PATH"
export PATH
cd "$PROJECT_ROOT"

PROFILE=debug
if [ "${1-}" = "--release" ]; then PROFILE=release; shift; fi
if [ "$#" -ne 0 ]; then echo "usage: scripts/package-macos.sh [--release]" >&2; exit 2; fi
if [ "$PROFILE" = release ]; then cargo build -p lumen-app --release; else cargo build -p lumen-app; fi
BIN="$PROJECT_ROOT/target/$PROFILE/lumen"
APP="$PROJECT_ROOT/.local/dist/Lumen Presenter.app"
CONTENTS="$APP/Contents"
MACOS="$CONTENTS/MacOS"
RESOURCES="$CONTENTS/Resources"
mkdir -p "$MACOS" "$RESOURCES"
rm -f "$MACOS/lumen-bin"
cp "$BIN" "$MACOS/lumen"
chmod +x "$MACOS/lumen"
cp "$PROJECT_ROOT/crates/lumen-app/assets/fonts/OFL.txt" "$RESOURCES/Poppins-OFL.txt"
cat > "$CONTENTS/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>lumen</string>
  <key>CFBundleIdentifier</key><string>org.lumen.presenter</string>
  <key>CFBundleName</key><string>Lumen Presenter</string>
  <key>CFBundleDisplayName</key><string>Lumen Presenter</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>0.1.0</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSPrincipalClass</key><string>NSApplication</string>
</dict>
</plist>
PLIST
if command -v codesign >/dev/null 2>&1; then codesign --force --deep --sign - "$APP"; fi
printf 'Created %s (%s build)\n' "$APP" "$PROFILE"
