#!/usr/bin/env python3
"""Generate RustFox icons from rustfox-source.png.

Crops to square, generates all PNG sizes, ICO, iconset, and icns.

Usage:
    python3 scripts/make_icon.py [out_dir]
"""
import os
import subprocess
import sys

try:
    from PIL import Image
except ImportError:
    Image = None

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "assets", "icons")
SRC = os.path.join(OUT, "rustfox-source.png")

# How much of the source to crop (fraction from center)
CROP_RATIO = 0.85


def crop_to_square():
    img = Image.open(SRC).convert("RGBA")
    w, h = img.size
    size = int(min(w, h) * CROP_RATIO)
    left = (w - size) // 2
    top = (h - size) // 2
    return img.crop((left, top, left + size, top + size))


def write_png(square, path, size):
    img = square.resize((size, size), Image.LANCZOS)
    img.save(path)
    print("wrote", path)


def write_ico(square, path):
    img = square.resize((256, 256), Image.LANCZOS)
    img.save(path, format="ICO", sizes=[(256, 256)])
    print("wrote", path)


def main():
    os.makedirs(OUT, exist_ok=True)
    if Image is None:
        print("Pillow required: pip3 install pillow", file=sys.stderr)
        sys.exit(1)
    if not os.path.exists(SRC):
        print(f"Source not found: {SRC}", file=sys.stderr)
        sys.exit(1)

    square = crop_to_square()
    print(f"cropped to {square.size}")

    for size in (16, 32, 64, 128, 256, 512, 1024):
        write_png(square, os.path.join(OUT, f"rustfox-{size}.png"), size)

    write_ico(square, os.path.join(OUT, "rustfox.ico"))

    if sys.platform == "darwin":
        setdir = os.path.join(OUT, "rustfox.iconset")
        os.makedirs(setdir, exist_ok=True)
        mapping = [
            (16, "icon_16x16.png"),
            (32, "icon_16x16@2x.png"),
            (32, "icon_32x32.png"),
            (64, "icon_32x32@2x.png"),
            (128, "icon_128x128.png"),
            (256, "icon_128x128@2x.png"),
            (256, "icon_256x256.png"),
            (512, "icon_256x256@2x.png"),
            (512, "icon_512x512.png"),
            (1024, "icon_512x512@2x.png"),
        ]
        for size, name in mapping:
            with open(os.path.join(OUT, f"rustfox-{size}.png"), "rb") as f, \
                 open(os.path.join(setdir, name), "wb") as g:
                g.write(f.read())
        subprocess.run(
            ["iconutil", "-c", "icns", setdir, "-o", os.path.join(OUT, "rustfox.icns")],
            check=True,
        )
        print("wrote", os.path.join(OUT, "rustfox.icns"))
    else:
        print("skip icns (not on macOS)")


if __name__ == "__main__":
    main()