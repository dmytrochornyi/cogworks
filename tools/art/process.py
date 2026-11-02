#!/usr/bin/env python3
"""Turn raw AI-generated images into game sprites.

    python3 tools/art/process.py RAW.png NAME [NAME ...] [--fill 0.94] [--anchor center|bottom]

- One NAME: the whole image is one sprite.
- Several NAMEs: RAW is a sheet; its objects are found, read in rows (top to
  bottom, left to right) and matched to the NAMEs in that order.

For each sprite it removes the flat light background (flood fill from the
border, stopped by the art's dark outline), trims, and fits the art into the
size listed in src/sprite_sizes.txt, centred or standing on the bottom edge.
The untouched source is kept as assets/art/raw/<first NAME>.png and the sprite
written to assets/art/NAME.png; `cargo run --release --bin gen_assets` then
uses it instead of the procedurally painted sprite.

Needs Pillow and numpy.
"""

import argparse
import os
import shutil
import sys

import numpy as np
from PIL import Image, ImageFilter

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def sprite_sizes():
    out = {}
    with open(os.path.join(REPO, "src", "sprite_sizes.txt")) as f:
        for line in f:
            parts = line.split()
            if len(parts) == 3:
                out[parts[0]] = (int(parts[1]), int(parts[2]))
    return out


def dilate(mask, r=1):
    out = mask.copy()
    for dy in range(-r, r + 1):
        for dx in range(-r, r + 1):
            out |= np.roll(np.roll(mask, dy, 0), dx, 1)
    return out


def background_mask(rgb, tol):
    """Pixels connected to the border that are close to the border's colour."""
    h, w, _ = rgb.shape
    border = np.concatenate([rgb[0], rgb[-1], rgb[:, 0], rgb[:, -1]]).astype(float)
    bg = np.median(border, axis=0)
    close = np.linalg.norm(rgb.astype(float) - bg, axis=2) < tol
    seed = np.zeros((h, w), bool)
    seed[0, :] = seed[-1, :] = seed[:, 0] = seed[:, -1] = True
    filled = seed & close
    while True:
        grown = dilate(filled) & close
        if (grown == filled).all():
            return filled
        filled = grown


def components(mask, min_area):
    """Bounding boxes (x0, y0, x1, y1) of connected regions, small specks dropped."""
    h, w = mask.shape
    seen = np.zeros_like(mask)
    boxes = []
    for y0, x0 in zip(*np.nonzero(mask)):
        if seen[y0, x0]:
            continue
        stack = [(y0, x0)]
        seen[y0, x0] = True
        ys, xs = [], []
        while stack:
            y, x = stack.pop()
            ys.append(y)
            xs.append(x)
            for ny, nx in ((y + 1, x), (y - 1, x), (y, x + 1), (y, x - 1)):
                if 0 <= ny < h and 0 <= nx < w and mask[ny, nx] and not seen[ny, nx]:
                    seen[ny, nx] = True
                    stack.append((ny, nx))
        if len(ys) >= min_area:
            boxes.append((min(xs), min(ys), max(xs) + 1, max(ys) + 1))
    return boxes


def reading_order(boxes):
    """Sort boxes into rows by vertical centre, then left to right."""
    boxes = sorted(boxes, key=lambda b: (b[1] + b[3]) / 2)
    rows, row = [], []
    for b in boxes:
        cy = (b[1] + b[3]) / 2
        if row and cy - (row[0][1] + row[0][3]) / 2 > (row[0][3] - row[0][1]) * 0.5:
            rows.append(row)
            row = []
        row.append(b)
    if row:
        rows.append(row)
    return [b for r in rows for b in sorted(r, key=lambda b: b[0])]


def fit(img, size, fill, anchor):
    w, h = size
    bw, bh = max(1, round(w * fill)), max(1, round(h * fill))
    scale = min(bw / img.width, bh / img.height)
    img = img.resize((max(1, round(img.width * scale)), max(1, round(img.height * scale))), Image.LANCZOS)
    out = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    x = (w - img.width) // 2
    y = (h - img.height) // 2 if anchor == "center" else h - img.height - (h - bh) // 2
    out.paste(img, (x, y), img)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("raw")
    ap.add_argument("names", nargs="+")
    ap.add_argument("--fill", type=float, default=0.94, help="fraction of the sprite box the art may fill")
    ap.add_argument("--anchor", choices=["center", "bottom"], default="center")
    ap.add_argument("--join", type=int, default=3, help="px gap bridged when grouping a sheet's objects")
    ap.add_argument("--tol", type=float, default=40.0, help="background colour tolerance (0-441)")
    a = ap.parse_args()

    table = sprite_sizes()
    for n in a.names:
        if n not in table:
            sys.exit(f"unknown sprite {n!r}; see src/sprite_sizes.txt")

    raw_dir = os.path.join(REPO, "assets", "art", "raw")
    os.makedirs(raw_dir, exist_ok=True)
    raw_copy = os.path.join(raw_dir, f"{a.names[0]}.png")
    if os.path.abspath(a.raw) != os.path.abspath(raw_copy):
        shutil.copyfile(a.raw, raw_copy)

    rgb = np.asarray(Image.open(raw_copy).convert("RGB"))
    fg = ~background_mask(rgb, a.tol)
    # Pull the cut edge in by a pixel and feather it, so no light fringe survives.
    fg_soft = ~dilate(~fg, 1)
    alpha = Image.fromarray((fg_soft * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(0.6))
    rgba = Image.fromarray(rgb).convert("RGBA")
    rgba.putalpha(alpha)

    if len(a.names) == 1:
        ys, xs = np.nonzero(fg)
        boxes = [(xs.min(), ys.min(), xs.max() + 1, ys.max() + 1)]
    else:
        # Join each object's loose bits (sparkles, shadows) before finding objects.
        boxes = components(dilate(fg, a.join) if a.join else fg, min_area=fg.size // 400)
        boxes = reading_order(boxes)
        if len(boxes) != len(a.names):
            sys.exit(f"found {len(boxes)} objects but {len(a.names)} names were given")

    for name, (x0, y0, x1, y1) in zip(a.names, boxes):
        crop = rgba.crop((x0, y0, x1, y1))
        out = fit(crop, table[name], a.fill, a.anchor)
        path = os.path.join(REPO, "assets", "art", f"{name}.png")
        out.save(path)
        print(f"{name}: {table[name][0]}x{table[name][1]} <- {x1 - x0}x{y1 - y0} px")


if __name__ == "__main__":
    main()
