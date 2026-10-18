#!/bin/sh
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PROJECT_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
PATH="/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:$PATH"
export PATH
cd "$PROJECT_ROOT"

PROFILE=debug
WITH_BOOKS=1
for arg in "$@"; do
    case "$arg" in
        --release) PROFILE=release ;;
        --without-books) WITH_BOOKS=0 ;;
        *) echo "usage: scripts/package-macos.sh [--release] [--without-books]" >&2; exit 2 ;;
    esac
done

# Extract version dynamically from root Cargo.toml workspace.package
VERSION=$(sed -n '/^\[workspace.package\]/,/^\[/p' "$PROJECT_ROOT/Cargo.toml" | grep -E '^version[[:space:]]*=' | head -n1 | sed -E 's/version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/')
if [ -z "$VERSION" ]; then
    echo "Could not extract workspace version from Cargo.toml" >&2
    exit 1
fi

TARGET_DIR="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}"
if [ "$PROFILE" = release ]; then
    cargo build -p lumen-app --release
else
    cargo build -p lumen-app
fi

BIN="$TARGET_DIR/$PROFILE/lumen"
if [ ! -f "$BIN" ]; then
    echo "Built binary not found at $BIN" >&2
    exit 1
fi

DIST_DIR="$PROJECT_ROOT/.local/dist"
APP="$DIST_DIR/Lumen Presenter.app"
CONTENTS="$APP/Contents"
MACOS="$CONTENTS/MacOS"
RESOURCES="$CONTENTS/Resources"
LICENSES="$RESOURCES/Licenses"

rm -rf "$APP"
mkdir -p "$MACOS" "$RESOURCES" "$LICENSES"

cp "$BIN" "$MACOS/lumen"
chmod +x "$MACOS/lumen"

# Copy bundled licenses into Resources/Licenses/
cp "$PROJECT_ROOT/crates/lumen-text/fonts/Poppins-OFL.txt" "$LICENSES/Poppins-OFL.txt"
cp "$PROJECT_ROOT/crates/lumen-text/fonts/NotoSansGujarati-OFL.txt" "$LICENSES/NotoSansGujarati-OFL.txt"
cp "$PROJECT_ROOT/crates/lumen-app/assets/fonts/PHOSPHOR-LICENSE.txt" "$LICENSES/Phosphor-MIT.txt"
cp "$PROJECT_ROOT/LICENSE" "$LICENSES/Lumen-MIT.txt"
cp "$PROJECT_ROOT/crates/lumen-text/fonts/Poppins-OFL.txt" "$RESOURCES/Poppins-OFL.txt"

# Books fetched into .local/books by `cargo run --release -p lumen-library --example fetch_books -- .local/books`
# (never committed). A release needs all of them; --without-books makes a quick dev package.
if [ "$WITH_BOOKS" = 1 ]; then
    BOOKS_SRC="$PROJECT_ROOT/.local/books"
    BOOK_IDS=$(sed -n 's/^ *id: "\([a-z-]*\)",$/\1/p' "$PROJECT_ROOT/crates/lumen-library/src/model.rs")
    # ALLOW_MISSING_BOOKS="id id" lets a known-broken book (see docs/STATUS.md) stay out of a release.
    for id in $BOOK_IDS; do
        if [ ! -f "$BOOKS_SRC/$id/book.json" ]; then
            case " ${ALLOW_MISSING_BOOKS-} " in
                *" $id "*) echo "NOTE: book '$id' is not included (allowed by ALLOW_MISSING_BOOKS)" >&2 ;;
                *) echo "Missing book '$id' in $BOOKS_SRC. Run the fetch_books step in docs/RELEASE.md, set ALLOW_MISSING_BOOKS=\"$id\", or pass --without-books." >&2; exit 1 ;;
            esac
        fi
    done
    for id in $BOOK_IDS; do
        [ -f "$BOOKS_SRC/$id/book.json" ] || continue
        mkdir -p "$RESOURCES/Books/$id"
        cp "$BOOKS_SRC/$id/book.json" "$RESOURCES/Books/$id/book.json"
    done
    cp "$PROJECT_ROOT/crates/lumen-app/assets/Anirdesh-books.txt" "$LICENSES/Anirdesh-books.txt"
fi

# Ensure app icon is present
ICON_SRC="$PROJECT_ROOT/crates/lumen-app/assets/AppIcon.icns"
if [ ! -f "$ICON_SRC" ]; then
    "$PROJECT_ROOT/scripts/generate-icon.sh"
fi
cp "$ICON_SRC" "$RESOURCES/AppIcon.icns"

cat > "$CONTENTS/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>lumen</string>
  <key>CFBundleIdentifier</key><string>org.lumen.presenter</string>
  <key>CFBundleName</key><string>Lumen Presenter</string>
  <key>CFBundleDisplayName</key><string>Lumen Presenter</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSAppleEventsUsageDescription</key><string>Lumen controls PowerPoint or Keynote slideshows when you play a deck.</string>
  <key>NSPrincipalClass</key><string>NSApplication</string>
</dict>
</plist>
PLIST

# Assert that ChitraUni is not bundled inside the app (the book texts are, with permission)
if find "$APP" -iname "*chitra*" | grep -q .; then
    echo "ERROR: ChitraUni font or reference found inside app bundle!" >&2
    exit 1
fi

# Ad-hoc code signing
if command -v codesign >/dev/null 2>&1; then
    codesign --force --deep --sign - "$APP"
    codesign --verify --deep --strict "$APP"
fi
printf 'Created and verified %s (%s build, v%s)\n' "$APP" "$PROFILE" "$VERSION"

# Build compressed DMG
DMG_NAME="Lumen-Presenter-${VERSION}-macos-arm64.dmg"
DMG_PATH="$DIST_DIR/$DMG_NAME"
DMG_STAGE=$(mktemp -d)
trap 'rm -rf "$DMG_STAGE"' EXIT

cp -R "$APP" "$DMG_STAGE/"
ln -s /Applications "$DMG_STAGE/Applications"

rm -f "$DMG_PATH"
hdiutil create -volname "Lumen Presenter" -srcfolder "$DMG_STAGE" -ov -format UDZO "$DMG_PATH" >/dev/null
hdiutil verify "$DMG_PATH" >/dev/null

printf 'Created and verified DMG: %s\n' "$DMG_PATH"

# External runtime requirements
cat << 'REQ'

External runtime requirements:
  - FFmpeg / ffprobe: required for video backgrounds and thumbnail extraction (Homebrew: brew install ffmpeg)
  - ChitraUni font: optional commercial font in Font Book for Gujarati; Noto Sans Gujarati is bundled as fallback
  - Network: not required; the scripture books are included in the app
REQ
