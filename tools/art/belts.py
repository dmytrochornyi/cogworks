#!/usr/bin/env python3
"""Build the animated belt sprites from one AI-painted conveyor texture.

    python3 tools/art/belts.py RAW.png

RAW is a long straight horizontal conveyor on a flat light background: a belt
surface with cross ridges between two copper rails. This writes

- assets/art/belt_0..7.png        straight belt facing east, 64x64
- assets/art/belt_curve_0..7.png  belt entering from the bottom edge and
                                  leaving through the right edge, 64x64

The game advances one frame per 2 px of belt travel (BELT_SPEED 1.6 tiles/s at
51.2 frames/s), so the moving surface is resampled to exactly 4 ridges per
64 px tile (16 px period) and frame f samples it shifted by 2f px. The rails
stay put and are resampled to a whole number of bolts per tile, so tiles join
without seams. The curve bends the same texture around the tile's bottom-right
corner with 64 px of travel along the quarter circle, so its ridges meet the
straight belts on both edges.

Needs Pillow and numpy; reuses tools/art/process.py for the cut-out.
"""

import math
import os
import shutil
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import process as P  # noqa: E402

REPO = P.REPO
TILE = 64
# Vertical layout of the straight belt, matching the old procedural sprite so
# items on the two lanes (at y = 32 +/- 12.8) still sit on the belt.
TOP, BOTTOM = 4, 60
FRAMES = 8
PERIOD = 16  # px between ridges after resampling: 8 frames x 2 px


def period(signal, lo, hi):
    """Dominant repeat length of a 1-D signal, searched between lo and hi."""
    s = signal - signal.mean()
    best, best_lag = -1e18, lo
    for lag in range(lo, min(hi, len(s) // 2)):
        c = float(np.dot(s[:-lag], s[lag:])) / (len(s) - lag)
        if c > best:
            best, best_lag = c, lag
    return best_lag


def periodic_strip(rgba, rows, src_period, out_len, n_periods):
    """Cut n whole periods from the middle of `rgba` (only `rows`) and resample
    them to out_len px, so the strip tiles seamlessly."""
    h, w = rgba.shape[:2]
    span = src_period * n_periods
    x0 = (w - span) // 2
    strip = rgba[rows[0]:rows[1], x0:x0 + span]
    img = Image.fromarray(strip)
    return np.asarray(img.resize((out_len, rows[1] - rows[0]), Image.LANCZOS))


def main():
    raw = sys.argv[1]
    os.makedirs(os.path.join(REPO, "assets", "art", "raw"), exist_ok=True)
    raw_copy = os.path.join(REPO, "assets", "art", "raw", "belt_0.png")
    if os.path.abspath(raw) != os.path.abspath(raw_copy):
        shutil.copyfile(raw, raw_copy)

    rgb = np.asarray(Image.open(raw_copy).convert("RGB"))
    fg = ~P.background_mask(rgb, 40.0)
    rows = np.nonzero(fg.mean(axis=1) > 0.6)[0]
    y0, y1 = rows.min(), rows.max() + 1
    cols = np.nonzero(fg[y0:y1].mean(axis=0) > 0.9)[0]
    x0, x1 = cols.min() + 4, cols.max() - 3
    belt = rgb[y0:y1, x0:x1].astype(np.int32)
    h = belt.shape[0]

    # Rail rows are the coppery ones: clearly more red than blue.
    warm = (belt[..., 0] - belt[..., 2]).mean(axis=1) > 45
    mid = h // 2
    top_rail_end = next((y for y in range(mid, -1, -1) if warm[y]), h // 6) + 1
    bot_rail_start = next((y for y in range(mid, h) if warm[y]), h - h // 6)

    rgba = np.dstack([belt, np.full(belt.shape[:2], 255)]).astype(np.uint8)
    surf_rows = (top_rail_end, bot_rail_start)
    surf_lum = belt[surf_rows[0]:surf_rows[1]].mean(axis=(0, 2))
    p_surf = period(surf_lum, 3, 40)
    rail_lum = belt[:top_rail_end].mean(axis=(0, 2))
    p_rail = period(rail_lum, 6, 80)

    # Scale factor from source px to tile px, set by the belt's height.
    scale = (BOTTOM - TOP) / h
    n_surf = TILE // PERIOD
    n_rail = max(1, round(TILE / (p_rail * scale)))
    surf = periodic_strip(rgba, surf_rows, p_surf, PERIOD * n_surf, n_surf)
    top = periodic_strip(rgba, (0, top_rail_end), p_rail, TILE, n_rail)
    bot = periodic_strip(rgba, (bot_rail_start, h), p_rail, TILE, n_rail)

    def resized(a, rows_px):
        return np.asarray(Image.fromarray(a).resize((TILE, rows_px), Image.LANCZOS))

    # Both rails use the top one (the bottom is its mirror image) so the belt
    # has no painted light direction and reads the same at every rotation.
    top_px = max(4, round(top_rail_end * scale))
    bot_px = top_px
    surf_px = (BOTTOM - TOP) - top_px - bot_px
    surf = resized(surf, surf_px)
    # Lift the ridges so the belt's motion reads at a glance: stretch contrast
    # along the travel direction only (column means), keeping the colour.
    surf = surf.astype(np.float64)
    col = surf[..., :3].mean(axis=(0, 2))
    gain = 1.0 + 1.6 * (col - col.mean()) / max(1.0, col.std() * 3.0)
    surf[..., :3] *= np.clip(gain, 0.55, 1.6)[None, :, None]
    surf = np.clip(surf, 0, 255).astype(np.uint8)
    top = resized(top, top_px)
    bot = np.flipud(top).copy()

    def straight(frame):
        out = np.zeros((TILE, TILE, 4), np.uint8)
        shift = 2 * frame
        out[TOP:TOP + top_px] = top
        out[TOP + top_px:TOP + top_px + surf_px] = np.roll(surf, shift, axis=1)
        out[BOTTOM - bot_px:BOTTOM] = bot
        return out

    # Curve: tile pixel at distance d from the bottom-right corner and angle a
    # (pi at the bottom edge, 1.5 pi at the right edge) samples the straight
    # tile at y = d, x = (a - pi) * 128 / pi, i.e. 64 px of travel per corner.
    yy, xx = np.mgrid[0:TILE, 0:TILE].astype(np.float64) + 0.5
    qx, qy = xx - TILE, yy - TILE
    d = np.hypot(qx, qy)
    a = np.mod(np.arctan2(qy, qx), 2 * math.pi)
    sx = (a - math.pi) * (2 * TILE / math.pi)
    sy = d

    def bilinear(src, x, y):
        x = np.mod(x, TILE)
        y = np.clip(y, 0, TILE - 1.001)
        x0 = np.floor(x).astype(int) % TILE
        y0 = np.floor(y).astype(int)
        fx, fy = (x - np.floor(x))[..., None], (y - y0)[..., None]
        x1, y1 = (x0 + 1) % TILE, np.minimum(y0 + 1, TILE - 1)
        s = src.astype(np.float64)
        top_ = s[y0, x0] * (1 - fx) + s[y0, x1] * fx
        bot_ = s[y1, x0] * (1 - fx) + s[y1, x1] * fx
        return top_ * (1 - fy) + bot_ * fy

    art = os.path.join(REPO, "assets", "art")
    for f in range(FRAMES):
        s = straight(f)
        Image.fromarray(s).save(os.path.join(art, f"belt_{f}.png"))
        c = bilinear(s, sx, sy)
        c[(d < TOP) | (d > BOTTOM), 3] = 0
        Image.fromarray(np.clip(c, 0, 255).astype(np.uint8)).save(os.path.join(art, f"belt_curve_{f}.png"))
    print(f"belt: surface period {p_surf}px, rail period {p_rail}px, "
          f"rails {top_px}/{bot_px}px, surface {surf_px}px, {n_rail} bolts per tile")


if __name__ == "__main__":
    main()
