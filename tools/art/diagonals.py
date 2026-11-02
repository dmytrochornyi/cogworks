#!/usr/bin/env python3
"""Paint the robot's four diagonal views from its front, side and back art.

    python3 tools/art/diagonals.py

The robot's body is a sphere, so a 3/4 view is the same sphere turned 45
degrees: each pixel of the turned sphere is looked up on the painted front,
side or back view that sees that spot most squarely, and the three are
blended across the seams. The arms, antenna and thruster ring sit outside the
sphere and are carried over from the front or back art, slid toward the
middle (the far arm behind the body, the near arm in front of it).

Reads assets/art/player_0.png, player_side.png (facing left) and player_back.png,
and writes assets/art/player_front_right.png and player_back_right.png. The
left-facing views are these, mirrored at draw time.

Needs Pillow and numpy.
"""

import os

import numpy as np
from PIL import Image

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ART = os.path.join(REPO, "assets", "art")

SIZE = 64
SS = 4  # supersampling
# The body sphere in the 64x64 sprites (measured from the art).
CX, CY, R = 32.0, 37.5, 16.5
ARM_SLIDE = 5.0  # px the arms move toward the middle in a 3/4 view
SEAM = 6.0  # blend sharpness between views


def load(name):
    im = Image.open(os.path.join(ART, f"{name}.png")).convert("RGBA")
    im = im.resize((SIZE * SS, SIZE * SS), Image.BICUBIC)
    a = np.asarray(im).astype(np.float32) / 255.0
    # Premultiply so blending and resampling don't bleed the background.
    a[..., :3] *= a[..., 3:4]
    return a


def sample(img, x, y):
    """Bilinear lookup of premultiplied RGBA at sprite-pixel coords (arrays)."""
    h, w = img.shape[:2]
    fx = np.clip(x * SS - 0.5, 0, w - 1.001)
    fy = np.clip(y * SS - 0.5, 0, h - 1.001)
    x0, y0 = fx.astype(int), fy.astype(int)
    tx, ty = (fx - x0)[..., None], (fy - y0)[..., None]
    c00, c10 = img[y0, x0], img[y0, x0 + 1]
    c01, c11 = img[y0 + 1, x0], img[y0 + 1, x0 + 1]
    return (c00 * (1 - tx) + c10 * tx) * (1 - ty) + (c01 * (1 - tx) + c11 * tx) * ty


def over(dst, src):
    return src + dst * (1 - src[..., 3:4])


def turned(theta_deg, front, side, back, arms_from, near_is_left):
    n = SIZE * SS
    ys, xs = np.mgrid[0:n, 0:n].astype(np.float32)
    px, py = (xs + 0.5) / SS, (ys + 0.5) / SS
    nx, ny = (px - CX) / R, (py - CY) / R
    inside = nx * nx + ny * ny < 1.0
    nz = np.sqrt(np.clip(1.0 - nx * nx - ny * ny, 0.0, 1.0))

    # Screen point -> robot-local (L = robot's lateral axis, F = its front).
    t = np.radians(theta_deg)
    # Robot turned by t about the vertical axis: F = (sin t, cos t), L = (cos t, -sin t) in (x, z).
    f_loc = nx * np.sin(t) + nz * np.cos(t)
    l_loc = nx * np.cos(t) - nz * np.sin(t)

    # Where that local point appears on each painted view (x on screen, depth).
    #   front: x = L, z = F       back: x = -L, z = -F
    #   side (facing left): x = -F, z = |L| (left/right sides are alike)
    views = [
        (front, l_loc, np.maximum(f_loc, 0)),
        (back, -l_loc, np.maximum(-f_loc, 0)),
        (side, -f_loc, np.abs(l_loc)),
    ]
    acc = np.zeros((n, n, 4), np.float32)
    wsum = np.zeros((n, n, 1), np.float32)
    for img, sx, depth in views:
        w = (depth ** SEAM)[..., None]
        acc += sample(img, CX + sx * R, py) * w
        wsum += w
    body = np.where(inside[..., None], acc / np.maximum(wsum, 1e-6), 0.0)

    # Dark rim like the painted outline, a touch inside the silhouette.
    rr = np.sqrt(nx * nx + ny * ny)
    rim = np.clip((rr - 0.93) / 0.05, 0, 1) * inside
    ink = np.array([0.20, 0.11, 0.06, 1.0], np.float32)
    body = body * (1 - rim[..., None] * 0.85) + ink * rim[..., None] * 0.85

    # Parts outside the sphere: antenna, thruster ring and the two arms.
    src = arms_from
    outside = ~inside
    rest = np.where(outside[..., None], sample(src, px, py), 0.0)

    def arm_layer(side_sign):
        """One arm, slid toward the middle: the left arm (-1) moves right, the right (+1) left."""
        sx = px + side_sign * ARM_SLIDE
        layer = sample(src, sx, py)
        # Keep only pixels that were outside the sphere in the source and on that side.
        snx, sny = (sx - CX) / R, (py - CY) / R
        was_out = snx * snx + sny * sny >= 1.0
        on_side = (sx < CX - R * 0.55) if side_sign < 0 else (sx > CX + R * 0.55)
        return np.where((was_out & on_side)[..., None], layer, 0.0)

    left_arm = arm_layer(-1)
    right_arm = arm_layer(+1)
    centre = np.where((outside & (np.abs(px - CX) <= R * 0.55))[..., None], rest, 0.0)
    near, far = (left_arm, right_arm) if near_is_left else (right_arm, left_arm)

    out = np.zeros((n, n, 4), np.float32)
    out = over(out, far)
    out = over(out, centre)
    out = over(out, body)
    out = over(out, near)

    a = out[..., 3:4]
    rgb = np.where(a > 1e-4, out[..., :3] / np.maximum(a, 1e-4), 0)
    img = np.concatenate([rgb, a], -1)
    im = Image.fromarray((np.clip(img, 0, 1) * 255).astype(np.uint8), "RGBA")
    return im.resize((SIZE, SIZE), Image.LANCZOS)


def main():
    front, side, back = load("player_0"), load("player_side"), load("player_back")
    # Facing the viewer and to the right: the robot's left arm (screen-left) swings near.
    turned(45, front, side, back, front, near_is_left=True).save(os.path.join(ART, "player_front_right.png"))
    # Facing away and to the right: the screen-right arm of the back view swings near.
    turned(135, front, side, back, back, near_is_left=False).save(os.path.join(ART, "player_back_right.png"))
    print("wrote player_front_right.png, player_back_right.png")


if __name__ == "__main__":
    main()
