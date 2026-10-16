#!/bin/sh
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PROJECT_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
ASSETS_DIR="$PROJECT_ROOT/crates/lumen-app/assets"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

SWIFT_SRC="$TMP_DIR/render_assets.swift"
PNG_1024="$TMP_DIR/icon_1024.png"
ICONSET="$TMP_DIR/AppIcon.iconset"
mkdir -p "$ICONSET"

cat << 'EOF' > "$SWIFT_SRC"
import AppKit

func render(_ sourcePath: String, size: Int, destination: String) throws {
    guard let image = NSImage(contentsOfFile: sourcePath) else {
        throw NSError(domain: "LumenAssets", code: 1)
    }
    let dimensions = NSSize(width: size, height: size)
    image.size = dimensions
    guard let bitmap = NSBitmapImageRep(
        bitmapDataPlanes: nil,
        pixelsWide: size,
        pixelsHigh: size,
        bitsPerSample: 8,
        samplesPerPixel: 4,
        hasAlpha: true,
        isPlanar: false,
        colorSpaceName: .deviceRGB,
        bytesPerRow: 0,
        bitsPerPixel: 0
    ) else {
        throw NSError(domain: "LumenAssets", code: 2)
    }
    bitmap.size = dimensions
    guard let graphics = NSGraphicsContext(bitmapImageRep: bitmap) else {
        throw NSError(domain: "LumenAssets", code: 3)
    }
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = graphics
    graphics.imageInterpolation = .high
    graphics.cgContext.clear(CGRect(origin: .zero, size: dimensions))
    image.draw(in: CGRect(origin: .zero, size: dimensions))
    graphics.flushGraphics()
    NSGraphicsContext.restoreGraphicsState()
    guard let png = bitmap.representation(using: .png, properties: [:]) else {
        throw NSError(domain: "LumenAssets", code: 4)
    }
    try png.write(to: URL(fileURLWithPath: destination))
}

let assets = CommandLine.arguments[1]
try render("\(assets)/icon.svg", size: 1024, destination: CommandLine.arguments[2])
try render("\(assets)/lumen-mark.svg", size: 28, destination: "\(assets)/lumen-mark-28.png")
try render("\(assets)/lumen-mark.svg", size: 56, destination: "\(assets)/lumen-mark-56.png")
EOF

swift "$SWIFT_SRC" "$ASSETS_DIR" "$PNG_1024"

sips -z 16 16 "$PNG_1024" --out "$ICONSET/icon_16x16.png" >/dev/null
sips -z 32 32 "$PNG_1024" --out "$ICONSET/icon_16x16@2x.png" >/dev/null
sips -z 32 32 "$PNG_1024" --out "$ICONSET/icon_32x32.png" >/dev/null
sips -z 64 64 "$PNG_1024" --out "$ICONSET/icon_32x32@2x.png" >/dev/null
sips -z 128 128 "$PNG_1024" --out "$ICONSET/icon_128x128.png" >/dev/null
sips -z 256 256 "$PNG_1024" --out "$ICONSET/icon_128x128@2x.png" >/dev/null
sips -z 256 256 "$PNG_1024" --out "$ICONSET/icon_256x256.png" >/dev/null
sips -z 512 512 "$PNG_1024" --out "$ICONSET/icon_256x256@2x.png" >/dev/null
sips -z 512 512 "$PNG_1024" --out "$ICONSET/icon_512x512.png" >/dev/null
cp "$PNG_1024" "$ICONSET/icon_512x512@2x.png"

iconutil -c icns "$ICONSET" -o "$ASSETS_DIR/AppIcon.icns"
printf 'Generated %s/AppIcon.icns and Lumen mark PNGs\n' "$ASSETS_DIR"
