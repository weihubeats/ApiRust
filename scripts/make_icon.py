#!/usr/bin/env python3
"""Generate RustFox application icons (PNG / ICNS / ICO) with pure stdlib.

Clay-style mascot icon: cyberpunk fox with glowing API-node headphones,
macaron palette (rust orange x deep blue), soft studio lighting.

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

def sd_ellipse(px, py, cx, cy, rx, ry):
    """Signed distance to ellipse (approx, scaled by min axis)."""
    h = math.hypot((px - cx) / rx, (py - cy) / ry) - 1.0
    return h * min(rx, ry)

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
    t = max(0.0, min(1.0, t))
    return tuple(a[i] + (b[i] - a[i]) * t for i in range(3))

def clamp01(v):
    return max(0.0, min(1.0, v))

def over(ccur, acur, cnew, anew):
    """Composite cnew/anew over ccur/acur (straight alpha)."""
    if anew <= 0.0:
        return ccur, acur
    na = acur + anew * (1.0 - acur)
    if na <= 0.0:
        return cnew, anew
    return tuple((ccur[i] * acur * (1.0 - anew) + cnew[i] * anew) / na for i in range(3)), na

def _f(c):
    return tuple(v / 255.0 for v in c)

# ---------- palette ----------
# macaron rust orange x deep blue, clay shades
BG_TOP = _f((126, 146, 255)) # pastel periwinkle blue
BG_BOT = _f((255, 156, 98))  # macaron rust orange
HEAD_L = _f((255, 178, 120))
HEAD_B = _f((231, 84, 38))
HEAD_D = _f((198, 62, 26))   # bottom shade
EAR_L = _f((242, 100, 44))
EAR_B = _f((212, 56, 22))
MZ_L = _f((255, 247, 232))   # cream muzzle
MZ_B = _f((255, 227, 196))
MZ_D = _f((246, 202, 160))
DEEP = _f((33, 40, 94))         # deep blue (eyes, nose, pads)
BAND_L = _f((70, 92, 206))
BAND_B = _f((38, 52, 126))
CUP_L = _f((76, 99, 214))
CUP_B = _f((43, 58, 147))
PAD = _f((34, 43, 99))
CYAN = _f((87, 233, 255))    # glowing node
WHITE = _f((255, 255, 255))
BLUSH = _f((255, 128, 128))

# headphone band ring (drawn as upper arc of a circle)
RING_CX, RING_CY, RING_R, RING_T = 0.5, 0.48, 0.34, 0.06

def on_ring_arc(ang):
    """Upper arc: near-horizontal sides allowed (head hides the rest)."""
    a = abs(ang)
    return a <= 0.61 or (2.53 <= a <= 3.75)

def render(size):
    """Render RGBA buffer for a square icon of `size` pixels."""
    w = size * SS
    buf = bytearray(w * w * 4)
    w_edge = edge_w(size)

    def put(x, y, r, g, b, a):
        w = size * SS
        i = (int(round(y * w)) * w + int(round(x * w))) * 4
        buf[i] = int(round(max(0.0, min(1.0, r)) * 255)); buf[i + 1] = int(round(max(0.0, min(1.0, g)) * 255))
        buf[i + 2] = int(round(max(0.0, min(1.0, b)) * 255)); buf[i + 3] = int(round(max(0.0, min(1.0, a)) * 255))

    for yy in range(w):
        y = yy / SS / size
        for xx in range(w):
            x = xx / SS / size

            a0 = 0.0
            c0 = (0.0, 0.0, 0.0)

            # ---- squircle background: macaron vertical gradient + soft light ----
            d_rect = sd_round_rect(x, y, 0.5, 0.5, 0.485, 0.165)
            if d_rect >= w_edge:
                continue
            bg = mix(BG_TOP, BG_BOT, clamp01(y * 1.25))
            # top-left studio sheen, bottom-right soft deepening
            sheen = (1.0 - clamp01(math.hypot(x - 0.30, y - 0.22) / 0.46))
            bg = mix(bg, WHITE, sheen * sheen * 0.12)
            deep = (1.0 - clamp01(math.hypot(x - 0.80, y - 0.88) / 0.48))
            bg = mix(bg, _f((52, 66, 150)), deep * deep * 0.14)
            c0, a0 = over(c0, a0, bg, smooth(d_rect, w_edge))

            # ---- soft contact shadow under fox ----
            d_sh = sd_ellipse(x, y, 0.5, 0.865, 0.33, 0.055)
            if d_sh < w_edge:
                sa = smooth(d_sh, w_edge) * 0.30
                c0, a0 = over(c0, a0, _f((24, 30, 82)), sa)

            # ---- ears (behind band + head) ----
            l_ear = sd_tri(x, y, (0.285, 0.315), (0.445, 0.27), (0.345, 0.035))
            r_ear = sd_tri(x, y, (0.715, 0.315), (0.555, 0.27), (0.655, 0.035))
            l_in = sd_tri(x, y, (0.325, 0.27), (0.41, 0.24), (0.355, 0.09))
            r_in = sd_tri(x, y, (0.675, 0.27), (0.59, 0.24), (0.645, 0.09))
            ea = clamp01((y - 0.02) / 0.30)
            ear_c = mix(EAR_L, EAR_B, ea)
            in_c = mix(DEEP, _f((52, 66, 158)), ea)
            for d_ear, d_in, cc in ((l_ear, l_in, ear_c), (r_ear, r_in, ear_c)):
                if d_ear < w_edge:
                    c0, a0 = over(c0, a0, cc, smooth(d_ear, w_edge))
                    if d_in < w_edge:
                        c0, a0 = over(c0, a0, in_c, smooth(d_in, w_edge))

            # ---- headphone band (clay arc, drawn over ears, under head) ----
            hb = math.hypot(x - RING_CX, y - RING_CY)
            d_band = abs(hb - RING_R) - RING_T / 2.0
            ang = math.atan2(y - RING_CY, x - RING_CX)
            if d_band < w_edge and on_ring_arc(ang):
                bc = mix(BAND_L, BAND_B, clamp01((y - 0.06) / 0.40))
                bsp = (1.0 - clamp01(math.hypot(x - 0.44, y - 0.20) / 0.12))
                bc = mix(bc, WHITE, bsp * bsp * 0.22)
                c0, a0 = over(c0, a0, bc, smooth(d_band, w_edge))

            # ---- head: clay blob with gradient + shading + highlight ----
            d_head = sd_circle(x, y, 0.5, 0.585, 0.30)
            if d_head < w_edge:
                hc = mix(HEAD_L, HEAD_B, clamp01((y - 0.28) / 0.62))
                hc = mix(hc, HEAD_D, clamp01((y - 0.72) / 0.17) * 0.45)
                hsp = (1.0 - clamp01(math.hypot(x - 0.37, y - 0.44) / 0.15))
                hc = mix(hc, WHITE, hsp * hsp * 0.16)
                hsp2 = (1.0 - clamp01(math.hypot(x - 0.63, y - 0.30) / 0.055))
                hc = mix(hc, WHITE, hsp2 * hsp2 * 0.10)
                c0, a0 = over(c0, a0, hc, smooth(d_head, w_edge))

            # ---- muzzle ----
            d_mz = sd_ellipse(x, y, 0.5, 0.675, 0.158, 0.128)
            if d_mz < w_edge:
                mc = mix(MZ_L, MZ_B, clamp01((y - 0.55) / 0.26))
                mc = mix(mc, MZ_D, clamp01((y - 0.73) / 0.14) * 0.5)
                msp = (1.0 - clamp01(math.hypot(x - 0.44, y - 0.62) / 0.07))
                mc = mix(mc, WHITE, msp * msp * 0.14)
                c0, a0 = over(c0, a0, mc, smooth(d_mz, w_edge))

            # ---- face details ----
            d_eyeL = sd_ellipse(x, y, 0.43, 0.585, 0.040, 0.052)
            d_eyeR = sd_ellipse(x, y, 0.57, 0.585, 0.040, 0.052)
            for d_eye in (d_eyeL, d_eyeR):
                if d_eye < w_edge:
                    c0, a0 = over(c0, a0, DEEP, smooth(d_eye, w_edge))
            d_gl1 = sd_circle(x, y, 0.417, 0.572, 0.012)
            d_gl2 = sd_circle(x, y, 0.557, 0.572, 0.012)
            for d_gl in (d_gl1, d_gl2):
                if d_gl < w_edge:
                    c0, a0 = over(c0, a0, WHITE, smooth(d_gl, w_edge) * 0.95)
            d_g2 = sd_circle(x, y, 0.441, 0.596, 0.0045)
            if d_g2 < w_edge:
                c0, a0 = over(c0, a0, WHITE, smooth(d_g2, w_edge) * 0.8)
            d_ns = sd_ellipse(x, y, 0.5, 0.695, 0.030, 0.023)
            if d_ns < w_edge:
                c0, a0 = over(c0, a0, DEEP, smooth(d_ns, w_edge))
            d_nh = sd_circle(x, y, 0.492, 0.688, 0.0065)
            if d_nh < w_edge:
                c0, a0 = over(c0, a0, WHITE, smooth(d_nh, w_edge) * 0.85)
            d_mth = sd_ellipse(x, y, 0.5, 0.722, 0.016, 0.0085)
            if d_mth < w_edge:
                c0, a0 = over(c0, a0, _f((44, 48, 96)), smooth(d_mth, w_edge) * 0.55)
            d_bl1 = sd_ellipse(x, y, 0.365, 0.655, 0.034, 0.021)
            d_bl2 = sd_ellipse(x, y, 0.635, 0.655, 0.034, 0.021)
            for d_bl in (d_bl1, d_bl2):
                if d_bl < w_edge:
                    c0, a0 = over(c0, a0, BLUSH, smooth(d_bl, w_edge) * 0.42)

            # ---- ear-cups (over head sides) ----
            for ccx in (0.185, 0.815):
                d_cp = sd_circle(x, y, ccx, 0.53, 0.075)
                if d_cp < w_edge:
                    cp = mix(CUP_L, CUP_B, clamp01((y - 0.44) / 0.22))
                    csp = (1.0 - clamp01(math.hypot(x - (ccx - 0.022), y - 0.49) / 0.055))
                    cp = mix(cp, WHITE, csp * csp * 0.18)
                    c0, a0 = over(c0, a0, cp, smooth(d_cp, w_edge))
                d_pd = sd_circle(x, y, ccx, 0.535, 0.048)
                if d_pd < w_edge:
                    c0, a0 = over(c0, a0, PAD, smooth(d_pd, w_edge))
                d_led = sd_circle(x, y, ccx, 0.465, 0.011)
                if d_led < w_edge:
                    c0, a0 = over(c0, a0, CYAN, smooth(d_led, w_edge) * 0.95)

            # ---- glowing API nodes on the band ----
            for nx, ny, nr, gw in ((0.5, 0.14, 0.032, 0.085), (0.24, 0.27, 0.026, 0.065), (0.76, 0.27, 0.026, 0.065)):
                dd = math.hypot(x - nx, y - ny)
                if dd < nr + gw:
                    ga = (1.0 - clamp01((dd - nr) / gw)) * 0.55
                    c0, a0 = over(c0, a0, CYAN, ga)
                if dd < nr:
                    c0, a0 = over(c0, a0, CYAN, smooth(dd - nr, w_edge))
                if dd < nr * 0.45:
                    c0, a0 = over(c0, a0, WHITE, smooth(dd - nr * 0.45, w_edge) * 0.9)
            # signal ping ring on apex node
            d_ping = abs(math.hypot(x - 0.5, y - 0.14) - 0.055)
            if d_ping < 0.006:
                c0, a0 = over(c0, a0, CYAN, (1.0 - d_ping / 0.006) * 0.35)

            r, g, b = c0
            put(x, y, r, g, b, a0)

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
        # explicit pairs so every slot exists (dict would drop 16@2x)
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
