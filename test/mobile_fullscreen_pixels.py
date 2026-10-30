"""Check actual GPU captures of the mobile shell with simulated phone insets.

Use cranamp-studio-review --mobile 440 956 --touch-preview with
CRANAMP_REVIEW_PHONE_INSETS=1 and CRANAMP_REVIEW_THEME=light/dark.
Name captures {player,library,settings,studio}-{light,dark}.png.
Requires Pillow; pass the capture directory and the active skin archive.
"""
import argparse
import io
from pathlib import Path
import zipfile

from PIL import Image


def check(directory, skin):
    with zipfile.ZipFile(skin) as archive:
        main = next(name for name in archive.namelist() if name.lower() == "main.bmp")
        corner = Image.open(io.BytesIO(archive.read(main))).convert("RGB").getpixel((0, 0))
    for theme, background in [("light", (242, 242, 247)), ("dark", (10, 10, 12))]:
        for screen in ["player", "library", "settings", "studio"]:
            capture = Path(directory) / f"{screen}-{theme}.png"
            image = Image.open(capture).convert("RGB")
            width, height = image.size
            expected = corner if screen == "player" else background
            for point in [(4, 4), (width // 2, 20), (width - 5, 30),
                          (4, height - 10), (width // 2, height - 3), (width - 5, height - 10)]:
                actual = image.getpixel(point)
                assert max(abs(a - b) for a, b in zip(actual, expected)) <= 1, (
                    f"{capture.name} at {point}: {actual}, expected {expected}"
                )
            print(f"{capture.name}: status and home areas use RGB {expected}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory")
    parser.add_argument("skin")
    args = parser.parse_args()
    check(args.directory, args.skin)
