#!/usr/bin/env python3
"""Generate RustFox application icons (PNG / ICNS / ICO) with pure stdlib.

Usage:
    python3 scripts/make_icon.py [out_dir]

Outputs (default out_dir = assets/icons):
    rustfox-16.png  rustfox-32.png  rustfox-64.png
    rustfox-128.png rustfox-256.png rustfox-512.png rustfox-1024.png
    rustfox.ico                     (Windows, 256x256 PNG-in-ICO)
    rustfox.icns                    (macOS, via iconutil when available)
"""
import math
import os
import struct
import subprocess
import sys
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "assets", "icons")

SS = 2  # supersampling factor

def edge_w(size):
    """Smooth-edge width in normalized [0,1] icon space (~2.5 final px)."""
    return 2.5 / SS / size

# ---------- shape helpers (signed distance functions) ----------

def sd_round_rect(px, py, cx, cy, half, r):
    dx = abs(px - cx) - half + r
    dy = abs(py - cy) - half + r
    ox = max(dx, 0.0)
    oy = max(dy, 0.0)
    return math.hypot(ox, oy) + min(max(dx, dy), 0.0) - r

def sd_circle(px, py, cx, cy, r):
    return math.hypot(px - cx, py - cy) - r

def sd_tri(px, py, a, b, c):
    """Signed distance to triangle (a, b, c); negative inside."""
    def cross(o, p, q):
        return (p[0] - o[0]) * (q[1] - o[1]) - (p[1] - o[1]) * (q[0] - o[0])
    eq = (cross(a, b, (px, py)), cross(b, c, (px, py)), cross(c, a, (px, py)))
    inside = (eq[0] >= 0 and eq[1] >= 0 and eq[2] >= 0) or (eq[0] <= 0 and eq[1] <= 0 and eq[2] <= 0)
    d = min(
        dot_to_seg(px, py, a, b),
        dot_to_seg(px, py, b, c),
        dot_to_seg(px, py, c, a),
    )
    return -d if inside else d

def dot_to_seg(px, py, a, b):
    abx, aby = b[0] - a[0], b[1] - a[1]
    apx, apy = px - a[0], py - a[1]
    t = max(0.0, min(1.0, (apx * abx + apy * aby) / (abx * abx + aby * aby)))
    return math.hypot(px - (a[0] + t * abx), py - (a[1] + t * aby))

def smooth(d, w=1.2):
    """1 inside, 0 outside, smooth edge over ~w px."""
    return 1.0 - min(1.0, max(0.0, (d + w / 2.0) / w))

def mix(a, b, t):
    return tuple(a[i] + (b[i] - a[i]) * t for i in range(3))

# ---------- palette ----------
ORANGE_A = (255, 158, 61)
ORANGE_B = (235, 84, 35)
DARK = (150, 40, 12)
WHITE = (255, 255, 255)

def render(size):
    """Render RGBA buffer for a square icon of `size` pixels."""
    w = size * SS
    buf = bytearray(w * w * 4)
    # design coordinates in [0,1] space
    w_edge = edge_w(size)

    def put(x, y, col):
        w = size * SS
        i = (int(round(y * w)) * w + int(round(x * w))) * 4
        buf[i: i + 4] = col

    for yy in range(w):
        y = yy / SS / size
        for xx in range(w):
            x = xx / SS / size
            bg = mix(ORANGE_A, ORANGE_B, y)
            d_rect = sd_round_rect(x, y, 0.5, 0.5, 0.485, 0.16)
            if d_rect >= w_edge:
                put(xx, yy, bytes((0, 0, 0, 0)))
                continue
            alpha = smooth(d_rect, w_edge)
            col = bg
            # white fox face
            d_head = sd_circle(x, y, 0.5, 0.63, 0.31)
            col = mix(col, WHITE, smooth(d_head, w_edge))
            # ears (dark, with white inner ear)
            l_ear = sd_tri(x, y, (0.255, 0.64), (0.40, 0.60), (0.33, 0.20))
            r_ear = sd_tri(x, y, (0.745, 0.64), (0.60, 0.60), (0.67, 0.20))
            l_in = sd_tri(x, y, (0.32, 0.575), (0.375, 0.565), (0.345, 0.28))
            r_in = sd_tri(x, y, (0.68, 0.575), (0.625, 0.565), (0.655, 0.28))
            col = mix(col, DARK, min(smooth(l_ear, w_edge), 1.0))
            col = mix(col, DARK, min(smooth(r_ear, w_edge), 1.0))
            col = mix(col, WHITE, min(smooth(l_in, w_edge), 1.0))
            col = mix(col, WHITE, min(smooth(r_in, w_edge), 1.0))
            # eyes
            col = mix(col, DARK, min(smooth(sd_circle(x, y, 0.44, 0.565, 0.032), w_edge), 1.0))
            col = mix(col, DARK, min(smooth(sd_circle(x, y, 0.56, 0.565, 0.032), w_edge), 1.0))
            # nose
            d_nose = sd_tri(x, y, (0.46, 0.655), (0.54, 0.655), (0.50, 0.705))
            col = mix(col, DARK, min(smooth(d_nose, w_edge), 1.0))
            r, g, b = (int(round(c * alpha)) for c in col)
            put(x, y, bytes((r, g, b, int(round(255 * alpha)))))
    # downsample with box filter
    out = bytearray(size * size * 4)
    for oy in range(size):
        for ox in range(size):
            acc = [0, 0, 0, 0]
            for dy in range(SS):
                for dx in range(SS):
                    i = ((oy * SS + dy) * w + ox * SS + dx) * 4
                    acc[0] += buf[i]
                    acc[1] += buf[i + 1]
                    acc[2] += buf[i + 2]
                    acc[3] += buf[i + 3]
            n = SS * SS
            out[(oy * size + ox) * 4: (oy * size + ox) * 4 + 4] = bytes(
                (acc[0] // n, acc[1] // n, acc[2] // n, acc[3] // n)
            )
    return bytes(out)

def write_png(path, size, rgba):
    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    raw = bytearray()
    for y in range(size):
        raw.append(0)  # filter type: None
        raw += rgba[y * size * 4: (y + 1) * size * 4]
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(png)
    print("wrote", path)

def write_ico(path, png_data):
    # single-entry PNG-compressed ICO (256x256)
    entry = struct.pack("<BBBBHHII", 0, 0, 0, 0, 1, 32, len(png_data), 22)
    header = struct.pack("<HHH", 0, 1, 1)
    with open(path, "wb") as f:
        f.write(header + entry + png_data)
    print("wrote", path)

def main():
    os.makedirs(OUT, exist_ok=True)
    for size in (16, 32, 64, 128, 256, 512, 1024):
        write_png(os.path.join(OUT, f"rustfox-{size}.png"), size, render(size))
    with open(os.path.join(OUT, "rustfox-256.png"), "rb") as f:
        write_ico(os.path.join(OUT, "rustfox.ico"), f.read())
    if sys.platform == "darwin":
        setdir = os.path.join(OUT, "rustfox.iconset")
        os.makedirs(setdir, exist_ok=True)
        mapping = {
            16: "icon_16x16.png",
            32: "icon_16x16@2x.png",
            32: "icon_32x32.png",
            64: "icon_32x32@2x.png",
            128: "icon_128x128.png",
            256: "icon_128x128@2x.png",
            256: "icon_256x256.png",
            512: "icon_256x256@2x.png",
            512: "icon_512x512.png",
            1024: "icon_512x512@2x.png",
        }
        for size, name in mapping.items():
            with open(os.path.join(OUT, f"rustfox-{size}.png"), "rb") as f, open(os.path.join(setdir, name), "wb") as g:
                g.write(f.read())
        subprocess.run(
            ["iconutil", "-c", "icns", setdir, "-o", os.path.join(OUT, "rustfox.icns")],
            check=True,
        )
        print("wrote", os.path.join(OUT, "rustfox.icns"))
    else:
        print("skip icns (not on macOS; generate on a Mac with iconutil or in CI)")

if __name__ == "__main__":
    main()
