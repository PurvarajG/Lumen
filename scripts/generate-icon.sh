#!/bin/sh
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PROJECT_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
ASSETS_DIR="$PROJECT_ROOT/crates/lumen-app/assets"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

SWIFT_SRC="$TMP_DIR/render_icon.swift"
PNG_1024="$TMP_DIR/icon_1024.png"
ICONSET="$TMP_DIR/AppIcon.iconset"
mkdir -p "$ICONSET"

cat << 'EOF' > "$SWIFT_SRC"
import AppKit

let size = NSSize(width: 1024, height: 1024)
let image = NSImage(size: size)
image.lockFocus()

guard let ctx = NSGraphicsContext.current?.cgContext else {
    exit(1)
}

ctx.clear(CGRect(x: 0, y: 0, width: 1024, height: 1024))

let iconRect = CGRect(x: 100, y: 100, width: 824, height: 824)
let squircle = CGPath(roundedRect: iconRect, cornerWidth: 185, cornerHeight: 185, transform: nil)

// Drop shadow
ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: -16), blur: 24, color: CGColor(red: 0, green: 0, blue: 0, alpha: 0.5))
ctx.addPath(squircle)
ctx.setFillColor(CGColor(red: 0.07, green: 0.07, blue: 0.09, alpha: 1.0))
ctx.fillPath()
ctx.restoreGState()

// Clip squircle
ctx.saveGState()
ctx.addPath(squircle)
ctx.clip()

// Dark graphite background gradient
let colorSpace = CGColorSpaceCreateDeviceRGB()
let bgColors = [
    CGColor(red: 0.14, green: 0.15, blue: 0.17, alpha: 1.0), // #23262B
    CGColor(red: 0.07, green: 0.07, blue: 0.09, alpha: 1.0)  // #121316
] as CFArray
if let bgGradient = CGGradient(colorsSpace: colorSpace, colors: bgColors, locations: [0.0, 1.0]) {
    ctx.drawLinearGradient(bgGradient, start: CGPoint(x: 512, y: 924), end: CGPoint(x: 512, y: 100), options: [])
}

// Inner stroke
ctx.setLineWidth(3.0)
ctx.setStrokeColor(CGColor(red: 0.24, green: 0.26, blue: 0.30, alpha: 0.8))
ctx.addPath(squircle)
ctx.strokePath()

// Soft flame ambient glow
let glowColors = [
    CGColor(red: 0.95, green: 0.64, blue: 0.23, alpha: 0.35),
    CGColor(red: 0.95, green: 0.64, blue: 0.23, alpha: 0.12),
    CGColor(red: 0.95, green: 0.64, blue: 0.23, alpha: 0.0)
] as CFArray
if let glowGradient = CGGradient(colorsSpace: colorSpace, colors: glowColors, locations: [0.0, 0.5, 1.0]) {
    ctx.drawRadialGradient(glowGradient, startCenter: CGPoint(x: 512, y: 514), startRadius: 10, endCenter: CGPoint(x: 512, y: 514), endRadius: 300, options: [])
}

// Lamp / diya bowl base
let lampPath = CGMutablePath()
lampPath.move(to: CGPoint(x: 330, y: 390))
lampPath.addCurve(to: CGPoint(x: 694, y: 390), control1: CGPoint(x: 400, y: 314), control2: CGPoint(x: 624, y: 314))
lampPath.addCurve(to: CGPoint(x: 512, y: 348), control1: CGPoint(x: 674, y: 368), control2: CGPoint(x: 620, y: 348))
lampPath.addCurve(to: CGPoint(x: 330, y: 390), control1: CGPoint(x: 404, y: 348), control2: CGPoint(x: 350, y: 368))
lampPath.closeSubpath()

let lampColors = [
    CGColor(red: 0.29, green: 0.31, blue: 0.35, alpha: 1.0), // #4A4E58
    CGColor(red: 0.12, green: 0.13, blue: 0.15, alpha: 1.0)  // #1F2126
] as CFArray
if let lampGradient = CGGradient(colorsSpace: colorSpace, colors: lampColors, locations: [0.0, 1.0]) {
    ctx.saveGState()
    ctx.addPath(lampPath)
    ctx.clip()
    ctx.drawLinearGradient(lampGradient, start: CGPoint(x: 512, y: 390), end: CGPoint(x: 512, y: 314), options: [])
    ctx.restoreGState()
}
ctx.setLineWidth(2.0)
ctx.setStrokeColor(CGColor(red: 0.36, green: 0.38, blue: 0.42, alpha: 1.0))
ctx.addPath(lampPath)
ctx.strokePath()

// Outer saffron flame
let flamePath = CGMutablePath()
flamePath.move(to: CGPoint(x: 512, y: 730))
flamePath.addCurve(to: CGPoint(x: 640, y: 470), control1: CGPoint(x: 575, y: 640), control2: CGPoint(x: 640, y: 560))
flamePath.addCurve(to: CGPoint(x: 512, y: 370), control1: CGPoint(x: 640, y: 400), control2: CGPoint(x: 580, y: 370))
flamePath.addCurve(to: CGPoint(x: 384, y: 470), control1: CGPoint(x: 444, y: 370), control2: CGPoint(x: 384, y: 400))
flamePath.addCurve(to: CGPoint(x: 512, y: 730), control1: CGPoint(x: 384, y: 560), control2: CGPoint(x: 449, y: 640))
flamePath.closeSubpath()

let flameColors = [
    CGColor(red: 1.0, green: 0.76, blue: 0.28, alpha: 1.0),  // #FFC247
    CGColor(red: 0.95, green: 0.64, blue: 0.23, alpha: 1.0),  // #F2A33A
    CGColor(red: 0.85, green: 0.32, blue: 0.08, alpha: 1.0)   // #D95214
] as CFArray
if let flameGradient = CGGradient(colorsSpace: colorSpace, colors: flameColors, locations: [0.0, 0.5, 1.0]) {
    ctx.saveGState()
    ctx.addPath(flamePath)
    ctx.clip()
    ctx.drawLinearGradient(flameGradient, start: CGPoint(x: 512, y: 730), end: CGPoint(x: 512, y: 370), options: [])
    ctx.restoreGState()
}

// Inner core of the flame
let corePath = CGMutablePath()
corePath.move(to: CGPoint(x: 512, y: 610))
corePath.addCurve(to: CGPoint(x: 570, y: 460), control1: CGPoint(x: 545, y: 560), control2: CGPoint(x: 570, y: 510))
corePath.addCurve(to: CGPoint(x: 512, y: 395), control1: CGPoint(x: 570, y: 420), control2: CGPoint(x: 545, y: 395))
corePath.addCurve(to: CGPoint(x: 454, y: 460), control1: CGPoint(x: 479, y: 395), control2: CGPoint(x: 454, y: 420))
corePath.addCurve(to: CGPoint(x: 512, y: 610), control1: CGPoint(x: 454, y: 510), control2: CGPoint(x: 479, y: 560))
corePath.closeSubpath()

let coreColors = [
    CGColor(red: 1.0, green: 0.99, blue: 0.96, alpha: 0.95),  // #FFFDF5
    CGColor(red: 1.0, green: 0.90, blue: 0.60, alpha: 0.90),  // #FFE699
    CGColor(red: 1.0, green: 0.70, blue: 0.28, alpha: 0.85)   // #FFB347
] as CFArray
if let coreGradient = CGGradient(colorsSpace: colorSpace, colors: coreColors, locations: [0.0, 0.5, 1.0]) {
    ctx.saveGState()
    ctx.addPath(corePath)
    ctx.clip()
    ctx.drawLinearGradient(coreGradient, start: CGPoint(x: 512, y: 610), end: CGPoint(x: 512, y: 395), options: [])
    ctx.restoreGState()
}

ctx.restoreGState() // unclip squircle

image.unlockFocus()

guard let tiffData = image.tiffRepresentation,
      let rep = NSBitmapImageRep(data: tiffData),
      let pngData = rep.representation(using: .png, properties: [:]) else {
    exit(1)
}

try! pngData.write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
EOF

swift "$SWIFT_SRC" "$PNG_1024"

# Generate multi-resolution icons via sips
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

# Compile to AppIcon.icns via iconutil
iconutil -c icns "$ICONSET" -o "$ASSETS_DIR/AppIcon.icns"
printf 'Generated %s/AppIcon.icns\n' "$ASSETS_DIR"
