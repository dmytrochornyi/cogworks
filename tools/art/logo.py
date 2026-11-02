#!/usr/bin/env python3
"""Draw the Cogworks logo and window icons.

    python3 tools/art/logo.py

The logo is the game's window plaque blown up: an ink-outlined copper frame
with corner rivets around a moss-green face, a brass cog emblem, and the name
in cream M PLUS Rounded Bold with a thick ink outline.

Writes:
  docs/logo.png             1600x520, for the README
  assets/art/logo.png       640x208, the title-screen sprite
  assets/icon/icon_{16,32,64}.png   the cog alone, for the window icon

Needs Pillow and numpy.
"""

import math
import os

import numpy as np

from PIL import Image, ImageDraw, ImageFilter, ImageFont

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
FONT = os.path.join(REPO, "assets", "fonts", "MPLUSRounded1c-Bold.ttf")


def hexc(v, a=255):
    return ((v >> 16) & 255, (v >> 8) & 255, v & 255, a)


INK = hexc(0x2A170E)
COPPER = hexc(0xB8744C)
COPPER_LO = hexc(0x8A5233)
COPPER_HI = hexc(0xD99163)
MOSS = hexc(0x56763A)
MOSS_HI = hexc(0x8FB260)
MOSS_LO = hexc(0x3A5426)
CREAM = hexc(0xFBEFD2)
RIVET = hexc(0xE8C478)
BRASS = hexc(0xC99A42)
BRASS_HI = hexc(0xF0CE7A)
BRASS_LO = hexc(0x8C6526)


def gear_points(cx, cy, r_out, r_in, teeth, phase=0.0):
    pts = []
    for k in range(teeth):
        a0 = phase + k * 2 * math.pi / teeth
        step = 2 * math.pi / teeth
        # Each tooth: root, rise, flat top, fall (a slightly tapered tooth).
        for f, r in ((0.0, r_in), (0.14, r_out), (0.40, r_out), (0.54, r_in)):
            a = a0 + f * step
            pts.append((cx + math.cos(a) * r, cy + math.sin(a) * r))
    return pts


def draw_cog(d, cx, cy, r, ink_w):
    """A brass cog with an ink outline, a lit upper half and a cream hub."""
    teeth = 10
    outer = gear_points(cx, cy, r, r * 0.80, teeth, -math.pi / 2)
    d.polygon(gear_points(cx, cy + ink_w * 0.6, r + ink_w, r * 0.80 + ink_w, teeth, -math.pi / 2), fill=INK)
    d.polygon(outer, fill=BRASS_LO)
    # Lit body, nudged up so the dark lower rim shows like a bevel.
    d.polygon(gear_points(cx, cy - r * 0.05, r * 0.95, r * 0.76, teeth, -math.pi / 2), fill=BRASS)
    d.ellipse([cx - r * 0.62, cy - r * 0.70, cx + r * 0.62, cy + r * 0.50], fill=BRASS_HI)
    d.ellipse([cx - r * 0.60, cy - r * 0.56, cx + r * 0.60, cy + r * 0.64], fill=BRASS)
    # Hub ring and cream centre
    hr = r * 0.42
    d.ellipse([cx - hr - ink_w, cy - hr - ink_w, cx + hr + ink_w, cy + hr + ink_w], fill=INK)
    d.ellipse([cx - hr, cy - hr, cx + hr, cy + hr], fill=COPPER)
    d.ellipse([cx - hr, cy - hr, cx + hr, cy + hr * 0.7], fill=COPPER_HI)
    d.ellipse([cx - hr * 0.92, cy - hr * 0.8, cx + hr * 0.92, cy + hr * 0.92], fill=COPPER)
    cr = r * 0.2
    d.ellipse([cx - cr - ink_w * 0.8, cy - cr - ink_w * 0.8, cx + cr + ink_w * 0.8, cy + cr + ink_w * 0.8], fill=INK)
    d.ellipse([cx - cr, cy - cr, cx + cr, cy + cr], fill=CREAM)
    # Four bolts around the hub
    for k in range(4):
        a = math.pi / 4 + k * math.pi / 2
        bx, by = cx + math.cos(a) * r * 0.6, cy + math.sin(a) * r * 0.6
        br = r * 0.07
        d.ellipse([bx - br - ink_w * 0.5, by - br - ink_w * 0.5, bx + br + ink_w * 0.5, by + br + ink_w * 0.5], fill=INK)
        d.ellipse([bx - br, by - br, bx + br, by + br], fill=RIVET)


def logo(w, h):
    s = 3  # supersample
    W, H = w * s, h * s
    im = Image.new("RGBA", (W, H), (0, 0, 0, 0))

    # Soft drop shadow under the plaque
    sh = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    ImageDraw.Draw(sh).rounded_rectangle([W * 0.025, H * 0.07, W * 0.975, H * 0.94], radius=H * 0.17, fill=(30, 15, 8, 110))
    sh = sh.filter(ImageFilter.GaussianBlur(H * 0.03))
    im.alpha_composite(sh)

    d = ImageDraw.Draw(im)
    x0, y0, x1, y1 = W * 0.012, H * 0.03, W * 0.975, H * 0.92
    rad = H * 0.17
    ink = H * 0.022
    band = H * 0.07
    # Ink outline, copper band (lit top edge), ink line, moss face
    d.rounded_rectangle([x0, y0, x1, y1], radius=rad, fill=INK)
    d.rounded_rectangle([x0 + ink, y0 + ink, x1 - ink, y1 - ink], radius=rad - ink, fill=COPPER_LO)
    d.rounded_rectangle([x0 + ink, y0 + ink, x1 - ink, y1 - ink * 2.2], radius=rad - ink, fill=COPPER)
    d.rounded_rectangle([x0 + rad * 0.5, y0 + ink * 1.4, x1 - rad * 0.5, y0 + ink * 2.4], radius=ink, fill=COPPER_HI)
    fx0, fy0, fx1, fy1 = x0 + band, y0 + band, x1 - band, y1 - band
    d.rounded_rectangle([fx0 - ink * 0.8, fy0 - ink * 0.8, fx1 + ink * 0.8, fy1 + ink * 0.8], radius=rad - band + ink, fill=INK)
    d.rounded_rectangle([fx0, fy0, fx1, fy1], radius=rad - band, fill=MOSS_LO)
    d.rounded_rectangle([fx0, fy0, fx1, fy1 - ink * 1.6], radius=rad - band, fill=MOSS)
    d.rounded_rectangle([fx0 + rad * 0.4, fy0 + ink * 0.7, fx1 - rad * 0.4, fy0 + ink * 1.6], radius=ink, fill=MOSS_HI)
    # Corner rivets on the copper band
    k = band * 0.5 + ink * 0.5
    for cx, cy in ((x0 + k + rad * 0.25, y0 + k + rad * 0.25), (x1 - k - rad * 0.25, y0 + k + rad * 0.25),
                   (x0 + k + rad * 0.25, y1 - k - rad * 0.25), (x1 - k - rad * 0.25, y1 - k - rad * 0.25)):
        r = band * 0.26
        d.ellipse([cx - r - ink * 0.6, cy - r - ink * 0.6, cx + r + ink * 0.6, cy + r + ink * 0.6], fill=INK)
        d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=RIVET)
        d.ellipse([cx - r * 0.5, cy - r * 0.7, cx + r * 0.1, cy - r * 0.1], fill=CREAM)

    # Cog emblem, overlapping the left edge of the face
    face_h = fy1 - fy0
    cog_r = face_h * 0.47
    cog_cx, cog_cy = fx0 + face_h * 0.55, (fy0 + fy1) / 2
    draw_cog(d, cog_cx, cog_cy, cog_r, ink * 1.1)

    # Name: ink outline, a dark moss drop, then cream
    text = "Cogworks"
    tx = cog_cx + cog_r + face_h * 0.16
    stroke = int(ink * 1.6)
    # Largest size that fits between the cog and the right edge of the face.
    room = fx1 - tx - face_h * 0.22
    size = int(face_h * 0.66)
    while True:
        font = ImageFont.truetype(FONT, size)
        bbox = d.textbbox((0, 0), text, font=font, stroke_width=stroke)
        if bbox[2] - bbox[0] <= room or size < 10:
            break
        size -= 2
    tx += (room - (bbox[2] - bbox[0])) / 2 - bbox[0]
    th = bbox[3] - bbox[1]
    ty = (fy0 + fy1) / 2 - th / 2 - bbox[1] - face_h * 0.02
    d.text((tx, ty + ink * 1.4), text, font=font, fill=INK, stroke_width=stroke, stroke_fill=INK)
    d.text((tx, ty), text, font=font, fill=CREAM, stroke_width=stroke, stroke_fill=INK)
    # A soft warm sheen on the upper half of the letters
    glyphs = Image.new("L", (W, H), 0)
    ImageDraw.Draw(glyphs).text((tx, ty), text, font=font, fill=255)
    sheen = Image.new("L", (W, H), 0)
    ImageDraw.Draw(sheen).rectangle([0, 0, W, ty + bbox[1] + th * 0.42], fill=70)
    mask = Image.fromarray(np.minimum(np.asarray(glyphs), np.asarray(sheen)))
    im.paste(Image.new("RGBA", (W, H), (255, 255, 255, 255)), (0, 0), mask)

    return im.resize((w, h), Image.LANCZOS)


def icon(n):
    s = 8
    im = Image.new("RGBA", (n * s, n * s), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    c = n * s / 2
    draw_cog(d, c, c, n * s * 0.44, n * s * 0.045)
    return im.resize((n, n), Image.LANCZOS)


def main():
    os.makedirs(os.path.join(REPO, "assets", "icon"), exist_ok=True)
    big = logo(1600, 520)
    big.save(os.path.join(REPO, "docs", "logo.png"))
    logo(640, 208).save(os.path.join(REPO, "assets", "art", "logo.png"))
    for n in (16, 32, 64):
        icon(n).save(os.path.join(REPO, "assets", "icon", f"icon_{n}.png"))
    print("wrote docs/logo.png, assets/art/logo.png, assets/icon/icon_*.png")


if __name__ == "__main__":
    main()
