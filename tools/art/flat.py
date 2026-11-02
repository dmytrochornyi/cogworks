#!/usr/bin/env python3
"""Paint the flat, top-down machinery that rotates with its direction: the
belt splitter, the tunnel entrance and the arm's turntable base.

    python3 tools/art/flat.py        (run tools/art/belts.py first)

They lie on the ground like belts, so they are drawn straight from above with
no front face or painted light direction, and reuse the belt's own frames so
the conveyors match exactly. Copper plates follow the logo palette: dark ink
outline, copper fill lit along the edge facing the viewer, brass rivets.
Drawn at 4x and downsampled for smooth edges. Needs Pillow.
"""

import os

from PIL import Image, ImageDraw

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ART = os.path.join(REPO, "assets", "art")
SS = 4  # supersampling factor

INK = (42, 23, 14, 255)
COPPER = (196, 116, 47, 255)
COPPER_HI = (242, 181, 106, 255)
COPPER_LO = (126, 67, 32, 255)
IRON = (92, 84, 80, 255)
IRON_HI = (138, 128, 120, 255)
MOUTH = (28, 18, 14, 255)
CREAM = (246, 234, 210, 255)
RIVET = (232, 196, 120, 255)


def canvas(w, h):
    return Image.new("RGBA", (w * SS, h * SS), (0, 0, 0, 0))


def box(d, x, y, w, h, r, fill, outline=INK, ow=2.2):
    """Rounded box in sprite pixels with an ink outline drawn outside it."""
    s = SS
    d.rounded_rectangle([(x - ow) * s, (y - ow) * s, (x + w + ow) * s, (y + h + ow) * s], (r + ow) * s, fill=outline)
    d.rounded_rectangle([x * s, y * s, (x + w) * s, (y + h) * s], r * s, fill=fill)


def plate(d, x, y, w, h, r=5.0, base=COPPER, hi=COPPER_HI, lo=COPPER_LO):
    """A riveted metal plate: rim shading on all sides (no light direction)."""
    box(d, x, y, w, h, r, lo)
    d.rounded_rectangle([(x + 1.5) * SS, (y + 1.5) * SS, (x + w - 1.5) * SS, (y + h - 1.5) * SS], (r - 1) * SS, fill=base)
    d.rounded_rectangle([(x + 3) * SS, (y + 3) * SS, (x + w - 3) * SS, (y + h - 3) * SS], max(1, r - 2) * SS, outline=hi, width=SS)
    for cx, cy in [(x + 4.5, y + 4.5), (x + w - 4.5, y + 4.5), (x + 4.5, y + h - 4.5), (x + w - 4.5, y + h - 4.5)]:
        dot(d, cx, cy, 1.6, RIVET)


def dot(d, cx, cy, r, fill, ow=0.8):
    s = SS
    d.ellipse([(cx - r - ow) * s, (cy - r - ow) * s, (cx + r + ow) * s, (cy + r + ow) * s], fill=INK)
    d.ellipse([(cx - r) * s, (cy - r) * s, (cx + r) * s, (cy + r) * s], fill=fill)


def chevron(d, cx, cy, size, up=True, fill=CREAM):
    """A small arrow showing the direction of travel."""
    s, k = SS, size
    pts = [(cx - k, cy + k * 0.5), (cx, cy - k * 0.5), (cx + k, cy + k * 0.5)] if up else \
          [(cx - k * 0.5, cy - k), (cx + k * 0.5, cy), (cx - k * 0.5, cy + k)]
    d.line([(px * s, py * s) for px, py in pts], fill=INK, width=int(3.6 * s), joint="curve")
    d.line([(px * s, py * s) for px, py in pts], fill=fill, width=int(1.8 * s), joint="curve")


def finish(img, name):
    w, h = img.size
    img.resize((w // SS, h // SS), Image.LANCZOS).save(os.path.join(ART, f"{name}.png"))
    print(name)


def belt_frame(rotate=False):
    b = Image.open(os.path.join(ART, "belt_0.png")).convert("RGBA")
    b = b.resize((64 * SS, 64 * SS), Image.LANCZOS)
    return b.transpose(Image.ROTATE_90) if rotate else b


def splitter():
    """Two tiles wide, facing north: two belts run up through a copper
    housing that spans both lanes."""
    img = canvas(128, 64)
    belt = belt_frame(rotate=True)
    img.alpha_composite(belt, (0, 0))
    img.alpha_composite(belt, (64 * SS, 0))
    d = ImageDraw.Draw(img)
    plate(d, 5, 17, 118, 30, r=7)
    # Divider between the two halves and a dark slot each belt passes through.
    box(d, 60, 20, 8, 24, 3, COPPER_LO)
    for x in (12, 76):
        box(d, x, 27, 40, 10, 4, MOUTH, ow=1.6)
        chevron(d, x + 20, 32, 4.0, up=True)
    finish(img, "splitter")


def tunnel():
    """Facing east: the belt runs in from the left under a copper hood whose
    dark mouth opens toward it."""
    img = canvas(64, 64)
    belt = belt_frame()
    img.alpha_composite(belt.crop((0, 0, 40 * SS, 64 * SS)), (0, 0))
    d = ImageDraw.Draw(img)
    plate(d, 26, 3, 35, 58, r=8)
    box(d, 24, 13, 11, 38, 5, MOUTH, ow=1.8)
    for y in (22, 32, 42):
        chevron(d, 46, y, 3.2, up=False)
    finish(img, "tunnel")


def arm_base():
    """A round copper turntable bolted to a square iron plate, seen from above."""
    img = canvas(64, 64)
    d = ImageDraw.Draw(img)
    box(d, 7, 7, 50, 50, 8, IRON)
    d.rounded_rectangle([10 * SS, 10 * SS, 54 * SS, 54 * SS], 6 * SS, outline=IRON_HI, width=SS)
    for cx, cy in [(12, 12), (52, 12), (12, 52), (52, 52)]:
        dot(d, cx, cy, 1.8, (170, 160, 150, 255))
    c, r = 32, 19
    d.ellipse([(c - r - 2.2) * SS, (c - r - 2.2) * SS, (c + r + 2.2) * SS, (c + r + 2.2) * SS], fill=INK)
    d.ellipse([(c - r) * SS, (c - r) * SS, (c + r) * SS, (c + r) * SS], fill=COPPER_LO)
    d.ellipse([(c - r + 2) * SS, (c - r + 2) * SS, (c + r - 2) * SS, (c + r - 2) * SS], fill=COPPER)
    d.ellipse([(c - r + 4) * SS, (c - r + 4) * SS, (c + r - 4) * SS, (c + r - 4) * SS], outline=COPPER_HI, width=SS)
    import math
    for i in range(8):
        a = i * math.pi / 4
        dot(d, c + math.cos(a) * 14.5, c + math.sin(a) * 14.5, 1.4, RIVET)
    dot(d, c, c, 6.5, MOUTH, ow=1.6)
    finish(img, "arm_base")


if __name__ == "__main__":
    splitter()
    tunnel()
    arm_base()
