#!/usr/bin/env python3
"""Gera app.ico, header.bmp e welcome.bmp para o instalador NSIS."""
import os
from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, "..", "assets", "bunkr-icon.png")

icon = Image.open(SRC).convert("RGBA")

# ---- ICO com varios tamanhos ----
icon.save(os.path.join(HERE, "app.ico"), sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])

top, mid, bot = (16, 14, 38), (44, 22, 84), (92, 40, 140)


def gradient(w, h):
    im = Image.new("RGB", (w, h))
    d = ImageDraw.Draw(im)
    for y in range(h):
        t = y / max(1, h - 1)
        if t < 0.5:
            u = t / 0.5
            c = tuple(int(top[i] + (mid[i] - top[i]) * u) for i in range(3))
        else:
            u = (t - 0.5) / 0.5
            c = tuple(int(mid[i] + (bot[i] - mid[i]) * u) for i in range(3))
        d.line([(0, y), (w, y)], fill=c)
    return im


# ---- header (150x57) ----
hw, hh = 150, 57
header = gradient(hw, hh)
ic = icon.resize((46, 46), Image.LANCZOS)
mask = Image.new("L", ic.size, 0)
ImageDraw.Draw(mask).ellipse([0, 0, ic.size[0] - 1, ic.size[1] - 1], fill=255)
header.paste(ic, (6, 5), mask)
header.save(os.path.join(HERE, "header.bmp"))

# ---- welcome (164x314) ----
ww, wh = 164, 314
welcome = gradient(ww, wh)
big = icon.resize((140, 140), Image.LANCZOS)
m2 = Image.new("L", big.size, 0)
ImageDraw.Draw(m2).ellipse([0, 0, big.size[0] - 1, big.size[1] - 1], fill=255)
welcome.paste(big, ((ww - 140) // 2, 80), m2)
welcome.save(os.path.join(HERE, "welcome.bmp"))

print("wrote app.ico, header.bmp, welcome.bmp")
