#!/usr/bin/env python3
"""Generate the Bunkr Downloader logo (icon + banner) with PIL."""
from PIL import Image, ImageDraw, ImageFont

S = 1024  # supersample, then downscale for smooth edges
W = H = S

top = (16, 14, 38)
mid = (44, 22, 84)
bot = (92, 40, 140)

base = Image.new("RGB", (W, H))
d = ImageDraw.Draw(base)
for y in range(H):
    t = y / (H - 1)
    if t < 0.5:
        u = t / 0.5
        c = tuple(int(top[i] + (mid[i] - top[i]) * u) for i in range(3))
    else:
        u = (t - 0.5) / 0.5
        c = tuple(int(mid[i] + (bot[i] - mid[i]) * u) for i in range(3))
    d.line([(0, y), (W, y)], fill=c)

cx, cy = W / 2, H / 2
R = 400

# soft radial glow behind the glyph
glow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
dg = ImageDraw.Draw(glow)
for i in range(60, 0, -1):
    r = R * i / 60
    a = int(3.0 * (60 - i))
    dg.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(150, 110, 255, a))
base = Image.alpha_composite(base.convert("RGBA"), glow).convert("RGB")
d = ImageDraw.Draw(base)

# ring
d.ellipse([cx - R, cy - R, cx + R, cy + R], outline=(255, 255, 255), width=22)
d.ellipse([cx - R + 42, cy - R + 42, cx + R - 42, cy + R - 42], outline=(140, 120, 220), width=6)

ACCENT = (255, 175, 55)
ACCENT_D = (230, 140, 30)

# arrow shaft
shaft_w = 132
shaft_top = cy - 190
shaft_bot = cy + 18
x0 = cx - shaft_w / 2
d.rounded_rectangle([x0, shaft_top, x0 + shaft_w, shaft_bot], radius=26, fill=ACCENT)

# arrow head
head_w = 190
head_top = shaft_bot - 4
head_tip = cy + 196
d.polygon([(cx - head_w, head_top), (cx + head_w, head_top), (cx, head_tip)], fill=ACCENT)

# tray / base line inside the ring
tw = 210
ty = cy + 236
th = 40
d.rounded_rectangle([cx - tw, ty, cx + tw, ty + th], radius=th / 2, fill=(255, 255, 255))

ICON = base.resize((512, 512), Image.LANCZOS)
ICON.save("bunkr-icon.png")

# ---------------------------------------------------------------- banner
BW, BH = 1600, 420
BAN = Image.new("RGB", (BW, BH))
db = ImageDraw.Draw(BAN)
for y in range(BH):
    t = y / (BH - 1)
    c = tuple(int(top[i] + (bot[i] - top[i]) * t) for i in range(3))
    db.line([(0, y), (BW, y)], fill=c)

# accent strip on the left edge
ACCENTS = [(255, 175, 55), (255, 120, 90), (150, 120, 255)]
for i, col in enumerate(ACCENTS):
    db.rectangle([0, BH - (i + 1) * 14, 18, BH - i * 14], fill=col)

# circular icon mask so the square background doesn't show
size = 330
icon_small = ICON.resize((size, size), Image.LANCZOS).convert("RGBA")
mask = Image.new("L", (size, size), 0)
ImageDraw.Draw(mask).ellipse([0, 0, size - 1, size - 1], fill=255)
BAN.paste(icon_small, (70, 45), mask)


def load_font(size, bold=True):
    names = ["arialbd.ttf", "segoeuib.ttf", "calibrib.ttf"] if bold else ["arial.ttf", "segoeui.ttf"]
    for n in names:
        try:
            return ImageFont.truetype(rf"C:\Windows\Fonts\{n}", size)
        except Exception:
            continue
    return ImageFont.load_default(size)


db.text((450, 105), "Bunkr Downloader", font=load_font(96), fill=(255, 255, 255))
db.text((456, 235), "Baixe álbuns e arquivos do Bunkr em alta velocidade", font=load_font(38, False), fill=(225, 215, 255))

BAN.save("bunkr-banner.png")
print("wrote bunkr-icon.png + bunkr-banner.png")
