#!/usr/bin/env python3
"""Rebuild the handoff assets; writes only inside this design package.

Requires Python fontTools, rsvg-convert; macOS sips and iconutil for ICNS.
All wordmark glyphs are outlined from the repository's bundled OFL Poppins.
"""
from pathlib import Path
import json
import shutil
import subprocess
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen

PACKAGE = Path(__file__).resolve().parents[1]
REPO = PACKAGE.parents[2]
ASSETS = PACKAGE / "assets"
FONTS = REPO / "crates/lumen-app/assets/fonts"
ASSETS.mkdir(exist_ok=True)
MARK = "M31 16 Q20 13 20 27 L20 222 Q20 236 34 236 L245 236 Q251 236 245 232 L80 181 L80 38 Q80 29 71 27 Z"
BEAM = "M93 115 L211 66 L211 200 L93 171 Z"

def mark(color="#F2A33A"):
    return f'<g fill="{color}"><path d="{MARK}"/><path d="{BEAM}"/></g>'

def outlined(text, x, baseline, size, color, weight="SemiBold", tracking=0):
    font = TTFont(FONTS / f"Poppins-{weight}.ttf")
    glyphs = font.getGlyphSet()
    cmap = font.getBestCmap()
    scale = size / font["head"].unitsPerEm
    out = []
    for ch in text:
        name = cmap[ord(ch)]
        pen = SVGPathPen(glyphs)
        glyphs[name].draw(pen)
        out.append(f'<path transform="translate({x:.5f} {baseline}) scale({scale:.7f} {-scale:.7f})" d="{pen.getCommands()}"/>')
        x += font["hmtx"][name][0] * scale + tracking
    return f'<g fill="{color}">' + ''.join(out) + '</g>'

def svg(filename, width, height, content, title):
    data = f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-label="{title}"><title>{title}</title>{content}</svg>\n'
    (ASSETS / filename).write_text(data)

for name, color in [("saffron", "#F2A33A"), ("light", "#ECE8E1"), ("dark", "#121316")]:
    svg(f"lumen-mark-{name}.svg", 256, 256, mark(color), "Lumen Presenter mark")
    for size in [16, 24, 28, 32, 64, 128, 256, 512, 1024]:
        subprocess.run(["rsvg-convert", "-w", str(size), "-h", str(size), "-o", str(ASSETS / f"lumen-mark-{name}-{size}.png"), str(ASSETS / f"lumen-mark-{name}.svg")], check=True)

for variant, fg, logo in [("on-dark", "#ECE8E1", "#F2A33A"), ("on-light", "#121316", "#F2A33A"), ("mono-light", "#ECE8E1", "#ECE8E1"), ("mono-dark", "#121316", "#121316")]:
    content = f'<g transform="scale({28/256})">{mark(logo)}</g>'
    content += outlined("Lumen", 36, 21, 18, fg)
    content += outlined("PRESENTER", 110, 19, 11, fg, "Medium", 0.45)
    svg(f"lumen-header-{variant}.svg", 188, 28, content, "Lumen Presenter")
    for scale in [1, 2, 3]:
        subprocess.run(["rsvg-convert", "-z", str(scale), "-o", str(ASSETS / f"lumen-header-{variant}@{scale}x.png"), str(ASSETS / f"lumen-header-{variant}.svg")], check=True)

icon = '<rect x="100" y="100" width="824" height="824" rx="185" fill="#1A1C20" stroke="#2E3238" stroke-width="3"/>'
icon += f'<g transform="translate(252 252) scale(2.03125)">{mark()}</g>'
svg("lumen-app-icon.svg", 1024, 1024, icon, "Lumen Presenter macOS app icon")
subprocess.run(["rsvg-convert", "-o", str(ASSETS / "lumen-app-icon-1024.png"), str(ASSETS / "lumen-app-icon.svg")], check=True)
iconset = ASSETS / "Lumen.iconset"
iconset.mkdir(exist_ok=True)
if shutil.which("sips") and shutil.which("iconutil"):
    for points in [16, 32, 128, 256, 512]:
        for scale in [1, 2]:
            px = points * scale
            suffix = "@2x" if scale == 2 else ""
            dest = iconset / f"icon_{points}x{points}{suffix}.png"
            subprocess.run(["sips", "-z", str(px), str(px), str(ASSETS / "lumen-app-icon-1024.png"), "--out", str(dest)], check=True, stdout=subprocess.DEVNULL)
    subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(ASSETS / "Lumen-AppIcon.icns")], check=True)
shutil.copy2(FONTS / "OFL.txt", PACKAGE / "POPPINS-OFL.txt")
(ASSETS / "fonts").mkdir(exist_ok=True)
for weight in ["Regular", "Medium", "SemiBold"]:
    shutil.copy2(FONTS / f"Poppins-{weight}.ttf", ASSETS / "fonts" / f"Poppins-{weight}.ttf")
print(json.dumps({"asset_files": len(list(ASSETS.rglob('*'))), "package": str(PACKAGE)}))
