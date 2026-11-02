#!/usr/bin/env python3
"""Paint the ore-field textures and the copper pipe pieces.

    python3 tools/art/fields.py

Ore fields: one seamless 256x256 texture per ore (a 4x4-tile period, like the
grass), made by scattering the painted ore sprites (assets/art/ore_<ore>_<n>.png)
over a packed-earth floor with wrap-around, so the field continues across
tiles. The alpha channel is not transparency but rock height: 1 on rocks, lower
on the earth between them. The ore shader uses it to let rocks poke out past
the field's edge, so the outline is rocky rather than a clean curve.

Pipes: pipe_seg (32x24, one arm from the tile centre to its edge, pointing +x,
ending in a flange) and pipe_hub (64x64, a round coupling for joints, bends
and pipe ends), in the copper of the painted machines.

Writes into assets/art/. Needs Pillow and numpy.
"""

import os
import random

import numpy as np
from PIL import Image

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ART = os.path.join(REPO, "assets", "art")

INK = np.array([42, 23, 14], np.float32) / 255.0

# Earth under each ore, and how its rocks are scattered: (floor rgb, rocks, size range px)
ORES = {
    "iron": ((0.36, 0.34, 0.33), 120, (30, 50)),
    "copper": ((0.42, 0.29, 0.20), 120, (30, 50)),
    "coal": ((0.22, 0.20, 0.19), 125, (30, 50)),
    "stone": ((0.52, 0.46, 0.38), 115, (30, 52)),
    "quartz": ((0.55, 0.46, 0.45), 95, (30, 50)),
}
SHEET = 256


def value_noise(n, cells, rng):
    """Smooth, seamless (periodic) value noise in 0..1, n x n."""
    g = rng.random((cells, cells)).astype(np.float32)
    xs = np.arange(n) / n * cells
    i0 = np.floor(xs).astype(int)
    t = xs - i0
    t = t * t * (3 - 2 * t)
    i1 = (i0 + 1) % cells
    a = g[np.ix_(i0, i0)] * (1 - t)[None, :] + g[np.ix_(i0, i1)] * t[None, :]
    b = g[np.ix_(i1, i0)] * (1 - t)[None, :] + g[np.ix_(i1, i1)] * t[None, :]
    return a * (1 - t)[:, None] + b * t[:, None]


def paste_wrapped(dst, src, x, y):
    """Alpha-composite RGBA float `src` onto periodic `dst` at (x, y), wrapping."""
    h, w = src.shape[:2]
    n = dst.shape[0]
    for oy in (-n, 0, n):
        for ox in (-n, 0, n):
            x0, y0 = x + ox, y + oy
            xa, ya = max(x0, 0), max(y0, 0)
            xb, yb = min(x0 + w, n), min(y0 + h, n)
            if xa >= xb or ya >= yb:
                continue
            s = src[ya - y0:yb - y0, xa - x0:xb - x0]
            a = s[..., 3:4]
            d = dst[ya:yb, xa:xb]
            d[..., :3] = s[..., :3] * a + d[..., :3] * (1 - a)
            d[..., 3:4] = np.maximum(d[..., 3:4], a)


def ore_field(ore, seed):
    floor, count, (lo, hi) = ORES[ore]
    rng = np.random.default_rng(seed)
    pyr = random.Random(seed)
    n = SHEET
    noise = value_noise(n, 8, rng) * 0.6 + value_noise(n, 32, rng) * 0.4
    img = np.zeros((n, n, 4), np.float32)
    base = np.array(floor, np.float32)
    img[..., :3] = base[None, None, :] * (0.82 + 0.3 * noise[..., None])
    # Speckle: grit and pebbles in the earth.
    grit = rng.random((n, n)) < 0.05
    img[grit, :3] *= 0.75
    img[..., 3] = 0.35 + 0.25 * noise

    sprites = [np.asarray(Image.open(os.path.join(ART, f"ore_{ore}_{k}.png")).convert("RGBA")).astype(np.float32) / 255.0 for k in range(3)]
    shadow_layer = np.zeros((n, n, 4), np.float32)
    stamps = []
    for _ in range(count):
        s = pyr.randint(lo, hi)
        x, y = pyr.randrange(n), pyr.randrange(n)
        stamps.append((y, x, s, pyr.randrange(3), pyr.random() < 0.5))
    stamps.sort()  # back to front
    for y, x, s, k, flip in stamps:
        im = Image.fromarray((sprites[k] * 255).astype(np.uint8), "RGBA").resize((s, s), Image.LANCZOS)
        if flip:
            im = im.transpose(Image.FLIP_LEFT_RIGHT)
        a = np.asarray(im).astype(np.float32) / 255.0
        # A soft contact shadow, nudged down-right, then the rock.
        sh = np.zeros_like(a)
        sh[..., 3] = a[..., 3] * 0.35
        paste_wrapped(shadow_layer, sh, x - s // 2 + 2, y - s // 2 + 3)
        paste_wrapped(img, np.concatenate([a[..., :3], a[..., 3:4]], -1), x - s // 2, y - s // 2)
    # Shadows go under the rocks: darken the earth where shadow fell and no rock is.
    rock = np.clip((img[..., 3] - 0.6) / 0.4, 0, 1)
    img[..., :3] *= (1 - shadow_layer[..., 3:4] * (1 - rock[..., None]))
    img[..., 3] = np.clip(img[..., 3], 0, 1)
    out = Image.fromarray((np.clip(img, 0, 1) * 255).astype(np.uint8), "RGBA")
    out.save(os.path.join(ART, f"orefield_{ore}.png"))


# ---------------------------------------------------------------- pipes
COPPER_HI = np.array([0.96, 0.72, 0.45])
COPPER = np.array([0.80, 0.47, 0.24])
COPPER_LO = np.array([0.45, 0.22, 0.11])
BRASS = np.array([0.86, 0.66, 0.30])
SS = 4


def tube_shade(t):
    """Copper colour across a tube, t = -1 (top edge) .. 1 (bottom edge)."""
    # Lit from the top: highlight a third of the way down, shadow at the bottom.
    hl = np.exp(-((t + 0.35) / 0.28) ** 2)
    base = COPPER * (1 - 0.45 * np.clip(t, 0, 1) ** 1.5)[..., None]
    return base * (1 - hl[..., None]) + COPPER_HI * hl[..., None] * 0.9 + base * hl[..., None] * 0.1


def finish(rgb, alpha, w, h):
    img = np.concatenate([np.clip(rgb, 0, 1), np.clip(alpha, 0, 1)[..., None]], -1)
    im = Image.fromarray((img * 255).astype(np.uint8), "RGBA")
    return im.resize((w, h), Image.LANCZOS)


def pipe_seg():
    w, h = 32 * SS, 24 * SS
    ys, xs = np.mgrid[0:h, 0:w].astype(np.float32)
    px, py = (xs + 0.5) / SS, (ys + 0.5) / SS
    cy = 12.0
    tube_r = 7.0          # tube half-height
    fl_r, fl_x = 9.5, 26.0  # flange half-height and start x
    ink = 1.6
    rgb = np.zeros((h, w, 3), np.float32)
    alpha = np.zeros((h, w), np.float32)

    # Tube body (from the centre, x=0, to the flange)
    t = (py - cy) / tube_r
    inside = np.abs(py - cy) <= tube_r
    body = tube_shade(np.clip(t, -1, 1))
    edge = np.abs(py - cy) > tube_r - ink
    rgb = np.where(inside[..., None], np.where(edge[..., None], INK, body), rgb)
    alpha = np.where(inside, 1.0, alpha)

    # Flange: a wider brass-copper collar at the tile edge with a bolt.
    fl = (np.abs(py - cy) <= fl_r) & (px >= fl_x) & (px <= 32.0)
    tf = (py - cy) / fl_r
    collar = tube_shade(np.clip(tf, -1, 1)) * 0.55 + BRASS * 0.45 * (1 - 0.4 * np.clip(tf, 0, 1))[..., None]
    fl_edge = (np.abs(py - cy) > fl_r - ink) | (px < fl_x + ink)
    rgb = np.where(fl[..., None], np.where(fl_edge[..., None], INK, collar), rgb)
    alpha = np.where(fl, 1.0, alpha)
    # Seam groove down the middle of the collar
    groove = fl & (np.abs(px - (fl_x + 3.2)) < 0.5) & (np.abs(py - cy) < fl_r - ink)
    rgb = np.where(groove[..., None], COPPER_LO, rgb)
    # Bolt heads top and bottom of the collar
    for by in (cy - fl_r + 2.6, cy + fl_r - 2.6):
        d = np.hypot(px - (fl_x + 3.2), py - by)
        rgb = np.where((d < 1.4)[..., None], BRASS * 1.1, rgb)
        rgb = np.where(((d >= 1.4) & (d < 1.9))[..., None], INK, rgb)
    return finish(rgb, alpha, 32, 24)


def pipe_hub():
    n = 64 * SS
    ys, xs = np.mgrid[0:n, 0:n].astype(np.float32)
    px, py = (xs + 0.5) / SS, (ys + 0.5) / SS
    c = 32.0
    r = 12.5
    ink = 1.8
    d = np.hypot(px - c, py - c)
    rgb = np.zeros((n, n, 3), np.float32)
    alpha = np.zeros((n, n), np.float32)
    # Soft drop shadow
    ds = np.hypot(px - c - 1.5, py - c - 2.5)
    sh = np.clip((r + 2.0 - ds) / 3.0, 0, 1) * 0.35
    alpha = np.maximum(alpha, sh)
    # Body: a dome, lit from the top-left
    inside = d <= r
    nx, ny = (px - c) / r, (py - c) / r
    nz = np.sqrt(np.clip(1 - nx * nx - ny * ny, 0, 1))
    light = np.clip(-0.45 * nx - 0.6 * ny + 0.65 * nz, 0, 1)
    dome = COPPER_LO + (COPPER - COPPER_LO) * np.clip(light * 1.4, 0, 1)[..., None]
    spec = np.clip((light - 0.82) / 0.18, 0, 1) ** 2
    dome = dome * (1 - spec[..., None]) + COPPER_HI * spec[..., None]
    rgb = np.where(inside[..., None], dome, rgb)
    alpha = np.where(inside, 1.0, alpha)
    # Ink rim and an inner brass ring
    rgb = np.where((inside & (d > r - ink))[..., None], INK, rgb)
    ring = inside & (np.abs(d - r * 0.58) < 0.9)
    rgb = np.where(ring[..., None], BRASS * (0.7 + 0.4 * light)[..., None], rgb)
    # Four bolts
    for k in range(4):
        a = np.pi / 4 + k * np.pi / 2
        bx, by = c + np.cos(a) * r * 0.78, c + np.sin(a) * r * 0.78
        db = np.hypot(px - bx, py - by)
        rgb = np.where((db < 1.3)[..., None], BRASS * 1.15, rgb)
        rgb = np.where(((db >= 1.3) & (db < 1.8))[..., None], INK, rgb)
    sh_only = ~inside
    rgb = np.where(sh_only[..., None], np.zeros(3), rgb)
    return finish(rgb, alpha, 64, 64)


def main():
    for k, ore in enumerate(ORES):
        ore_field(ore, 1000 + k)
    pipe_seg().save(os.path.join(ART, "pipe_seg.png"))
    pipe_hub().save(os.path.join(ART, "pipe_hub.png"))
    print("wrote orefield_*.png, pipe_seg.png, pipe_hub.png")


if __name__ == "__main__":
    main()
