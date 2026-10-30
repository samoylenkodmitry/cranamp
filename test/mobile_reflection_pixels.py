"""Check an unscaled mobile GPU capture from cranamp-studio-review --mobile.

Requires Pillow. Example for a 393x780 capture with all panels visible:
  python test/mobile_reflection_pixels.py capture.png 'assets/skins/Catamp Silverplay.wsz' --contact 668
"""
import argparse
import io
import math
import zipfile
from PIL import Image


def check(capture, archive, contact):
    with zipfile.ZipFile(archive) as skin:
        name = next(name for name in skin.namelist() if name.lower() == "main.bmp")
        corner = Image.open(io.BytesIO(skin.read(name))).convert("RGB").getpixel((0, 0))
    frame = Image.open(capture).convert("RGB")
    assert frame.getpixel((0, frame.height - 1)) == corner, "Player background differs from the skin corner"
    errors = []
    flat_errors = []
    # The exposed glass above the tabs must follow the mirrored live pixels.
    # Check a whole strip, including the footer controls, rather than one matching pixel.
    for y in range(int(contact + 2), int(contact + 26)):
        distance = y + 0.5 - contact
        opacity = 0.34 * (1 - distance / 104) ** 2
        sheen = 0.035 * math.exp(-distance / 5)
        source_y = contact - distance / 0.65 - 0.5
        row = math.floor(source_y)
        fraction = source_y - row
        for x in range(8, frame.width - 8):
            above, below = frame.getpixel((x, row)), frame.getpixel((x, row + 1))
            actual = frame.getpixel((x, y))
            for channel in range(3):
                source = above[channel] * (1 - fraction) + below[channel] * fraction
                expected = (corner[channel] * (1 - opacity) + source * opacity) * (1 - sheen) + 255 * sheen
                errors.append(abs(actual[channel] - expected))
                flat_errors.append(abs(actual[channel] - corner[channel]))
    error = sum(errors) / len(errors)
    flat_error = sum(flat_errors) / len(flat_errors)
    assert error < 3, f"Reflection differs from mirrored player pixels: {error:.3f} levels"
    assert error < flat_error * 0.25, "Reflection has no visible player detail"
    print(f"Corner RGB {corner}; mirrored pixel error {error:.3f}/255; plain fill error {flat_error:.3f}/255")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("capture")
    parser.add_argument("skin")
    parser.add_argument("--contact", type=float, required=True)
    args = parser.parse_args()
    check(args.capture, args.skin, args.contact)
