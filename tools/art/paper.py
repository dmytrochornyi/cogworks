#!/usr/bin/env python3
"""Paint the seamless parchment texture tiled over UI panels.

    python3 tools/art/paper.py

Writes assets/art/paper.png (128x128): a neutral grey-brown mottle and fine
fibres around mid-grey, drawn with a low alpha over the panel's cream face so
the panel colour stays in charge. Built from noise shaped in frequency space,
which is periodic by construction, so it tiles without seams. Needs numpy+Pillow.
"""
import os

import numpy as np
from PIL import Image

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
N = 128
rng = np.random.default_rng(7)

def periodic_noise(power):
    """Random noise shaped in frequency space (1/f^power). The inverse FFT of
    any spectrum is periodic, so the result tiles perfectly and looks organic."""
    f = np.fft.fftfreq(N)
    r = np.sqrt(f[None, :] ** 2 + f[:, None] ** 2)
    r[0, 0] = 1.0
    spec = np.fft.fft2(rng.normal(size=(N, N))) / r**power
    spec[0, 0] = 0
    n = np.real(np.fft.ifft2(spec))
    return n / np.abs(n).max()

mottle = periodic_noise(1.6)
fibre = periodic_noise(0.4)
v = 0.5 + 0.34 * mottle + 0.10 * fibre
v = np.clip(v, 0, 1)
rgb = np.stack([v * 0.78 + 0.1, v * 0.66 + 0.1, v * 0.52 + 0.1], axis=-1)
alpha = np.full((N, N), 1.0)
img = np.dstack([rgb, alpha])
Image.fromarray((img * 255).astype(np.uint8)).save(os.path.join(REPO, "assets", "art", "paper.png"))
print("paper")
