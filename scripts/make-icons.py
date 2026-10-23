#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy==2.3.4", "pillow==11.3.0"]
# ///
"""Makes every app and site icon from the logo master, packaging/logo/bible-logo-source.png.

    uv run scripts/make-icons.py
    (cd site && sh scripts/og.sh)   # then the share image, which shows site/static/icon.png

The master is a screenshot of the logo's rounded square on a dark background, about 179 px
across. Each icon is that square alone with transparent corners, resampled with Lanczos straight
from the screenshot to its own size. Small icons never pass through a big upscaled copy. For a
larger original, change MASTER and the square's measurements below.
"""

from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter

ROOT = Path(__file__).resolve().parent.parent
MASTER = ROOT / "packaging/logo/bible-logo-source.png"

# The square in the master, in pixels: its left and top edges, side and corner radius. Measured
# from the edge pixels' coverage; the corners fit circular arcs to within 0.3 px.
LEFT, TOP, SIDE, RADIUS = 77.34, 38.31, 179.28, 39.9


def rounded_square_distance(xs, ys, cx, cy, half, r):
    """Signed distance from each point to a rounded square's edge, negative inside."""
    qx = np.abs(xs - cx) - (half - r)
    qy = np.abs(ys - cy) - (half - r)
    outside = np.hypot(np.maximum(qx, 0), np.maximum(qy, 0))
    return outside + np.minimum(np.maximum(qx, qy), 0) - r


def extend_edges(master):
    """The master with every pixel outside the square, or blended with the background at its edge,
    replaced by the nearest colours inside. Resampling then never pulls the dark background into
    the edge, and the square's corners can be filled where an icon must be opaque."""
    rgb = np.asarray(master.convert("RGB"), dtype=np.float64)
    h, w, _ = rgb.shape
    ys, xs = np.mgrid[0:h, 0:w] + 0.5
    d = rounded_square_distance(xs, ys, LEFT + SIDE / 2, TOP + SIDE / 2, SIDE / 2, RADIUS)
    known = d <= -1.0

    def shift(a, dy, dx):
        out = np.zeros_like(a)
        out[max(dy, 0) : h + min(dy, 0), max(dx, 0) : w + min(dx, 0)] = a[
            max(-dy, 0) : h + min(-dy, 0), max(-dx, 0) : w + min(-dx, 0)
        ]
        return out

    # Grow the known area a pixel at a time, each new pixel the average of its known neighbours.
    # Small icons sample far past the square, so this fills the whole master.
    while not known.all():
        total = np.zeros_like(rgb)
        count = np.zeros((h, w))
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                if dx or dy:
                    k = shift(known, dy, dx)
                    total += shift(rgb, dy, dx) * k[..., None]
                    count += k
        new = ~known & (count > 0)
        rgb[new] = total[new] / count[new][:, None]
        known |= new
    return Image.fromarray(np.clip(rgb.round(), 0, 255).astype(np.uint8))


def square(extended, n, corners=True):
    """The logo's square at n x n px, transparent outside its rounded corners. With corners=False
    it is opaque and the corners are filled in, for iOS, which rounds them itself."""
    box = (LEFT, TOP, LEFT + SIDE, TOP + SIDE)
    out = extended.resize((n, n), Image.LANCZOS, box=box)
    scale = n / SIDE
    if scale > 1.5:
        # Upscaling softens the edges; a light unsharp mask, scaled to the enlargement, restores some.
        out = out.filter(ImageFilter.UnsharpMask(radius=0.35 * scale, percent=40, threshold=0))
    alpha = np.full((n, n), 255, np.uint8)
    if corners:
        ys, xs = np.mgrid[0:n, 0:n] + 0.5
        d = rounded_square_distance(xs, ys, n / 2, n / 2, n / 2, RADIUS * scale)
        alpha = (np.clip(0.5 - d, 0, 1) * 255).round().astype(np.uint8)
    return Image.fromarray(np.dstack([np.asarray(out), alpha]))


def on_canvas(extended, canvas, side, offset):
    """The square at side px on a transparent canvas, with a margin like the macOS icon grid's."""
    out = Image.new("RGBA", (canvas, canvas), (0, 0, 0, 0))
    out.paste(square(extended, side), (offset, offset))
    return out


def save(image, path):
    path = ROOT / path
    image.save(path, optimize=True)
    print(f"{path.relative_to(ROOT)}  {image.width}x{image.height}")


def main():
    extended = extend_edges(Image.open(MASTER))

    # scripts/bundle-macos.sh makes the macOS .icns from this. Apple's grid puts an 824 px square
    # in 1024, which leaves room for the Dock's shadow.
    save(on_canvas(extended, 1024, 824, 100), "crates/app/assets/icon.png")
    # The window icon on Windows and Linux, and the Dock icon of an unbundled build.
    save(on_canvas(extended, 256, 208, 24), "crates/app/assets/icon-256.png")
    # The logo the app draws on its welcome screen and in Settings > About (onboarding.rs).
    save(square(extended, 256), "crates/app/assets/logo.png")

    # iOS rounds the corners itself and wants an opaque square.
    save(square(extended, 1024, corners=False).convert("RGB"), "apps/ios/Bible/Assets.xcassets/AppIcon.appiconset/AppIcon.png")

    # The Windows .exe's icon, which crates/app/build.rs embeds, and the installer's. Each size has
    # its own square and margin, so the small ones land on whole pixels.
    ico = {16: (14, 1), 20: (18, 1), 24: (20, 2), 32: (26, 3), 40: (34, 3), 48: (40, 4), 64: (52, 6), 96: (78, 9), 128: (104, 12), 256: (208, 24)}
    images = [on_canvas(extended, n, side, offset) for n, (side, offset) in ico.items()]
    path = ROOT / "packaging/windows/bible-rs.ico"
    images[-1].save(path, format="ICO", sizes=[(n, n) for n in ico], append_images=images[:-1])
    print(f"{path.relative_to(ROOT)}  {', '.join(map(str, ico))}")

    # The site's favicon, the logo in its nav, footer and hero, the reader's logo, and the
    # home-screen icon, which iOS rounds like an app icon. The Mac app's Ark page copies the favicon.
    icon = on_canvas(extended, 256, 206, 25)
    for path in ("site/static/icon.png", "site/static/favicon.png", "crates/app/web/favicon.png"):
        save(icon, path)
    save(square(extended, 256), "site/static/logo.png")
    save(square(extended, 180, corners=False).convert("RGB"), "site/static/apple-touch-icon.png")


if __name__ == "__main__":
    main()
