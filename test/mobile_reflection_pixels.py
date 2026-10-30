"""Check a GPU capture of the player standing on a horizontal glass plane.

Requires Pillow. Example with 440x956 and CRANAMP_REVIEW_PHONE_INSETS=1:
  python test/mobile_reflection_pixels.py capture.png 'assets/skins/Catamp Silverplay.wsz' --contact 748.5 --player-top 59 --bottom-inset 34

Contact is the pixel center of the first floor row in the captured frame.
"""
import argparse
import io
import math
import zipfile
from PIL import Image


def sample(frame, x, y):
    x = min(max(x, 0), frame.width - 1)
    y = min(max(y, 0), frame.height - 1)
    col, row = math.floor(x), math.floor(y)
    fx, fy = x - col, y - row
    pixels = [frame.getpixel((min(col + dx, frame.width - 1), min(row + dy, frame.height - 1)))
              for dx, dy in [(0, 0), (1, 0), (0, 1), (1, 1)]]
    return [pixels[0][c] * (1-fx) * (1-fy) + pixels[1][c] * fx * (1-fy)
            + pixels[2][c] * (1-fx) * fy + pixels[3][c] * fx * fy for c in range(3)]


def check(capture, archive, contact, player_top, bottom_inset, left, width):
    with zipfile.ZipFile(archive) as skin:
        name = next(name for name in skin.namelist() if name.lower() == "main.bmp")
        corner = Image.open(io.BytesIO(skin.read(name))).convert("RGB").getpixel((0, 0))
    frame = Image.open(capture).convert("RGB")
    assert frame.getpixel((0, frame.height - 1)) == corner, "Player background differs from the skin corner"
    width = width or frame.width
    source_height = math.floor((contact - player_top) / 2 + 0.5)
    depth = min(frame.height - bottom_inset - contact, 128, source_height / 2)
    errors, flat_errors, unskewed_errors = [], [], []
    # Inspect the exposed plane above navigation. Compare actual GPU pixels to
    # the half-height mirror, including skew, perspective and the fade gradient.
    exposed_end = min(contact + 88, frame.height - bottom_inset - 86)
    for y in range(math.ceil(contact + 2), math.floor(exposed_end)):
        distance = y + 0.5 - contact
        t = distance / depth
        stretch = 1 + 0.24 * t
        source_fraction = t * 1.24 / stretch
        opacity = 0.42 * (1 - t) ** 2
        sheen = 0.035 * math.exp(-distance / 5)
        for x in range(math.ceil(left + 8), min(frame.width - 8, math.floor(left + width))):
            mirrored_x = (x + 0.5 - left - width * (0.5 + 0.18 * t)) / stretch + width / 2
            actual = frame.getpixel((x, y))
            if 0 <= mirrored_x <= width:
                reflected = sample(frame, left + mirrored_x - 0.5, contact - source_height * source_fraction - 0.5)
                expected = [(corner[c] * (1-opacity) + reflected[c] * opacity) * (1-sheen) + 255 * sheen for c in range(3)]
            else:
                expected = corner
            unskewed = sample(frame, x, contact - source_height * t - 0.5)
            for c in range(3):
                errors.append(abs(actual[c] - expected[c]))
                flat_errors.append(abs(actual[c] - corner[c]))
                unskewed_errors.append(abs(actual[c] - ((corner[c] * (1-opacity) + unskewed[c] * opacity) * (1-sheen) + 255 * sheen)))
    mean = lambda values: sum(values) / len(values)
    error, flat_error, unskewed_error = map(mean, (errors, flat_errors, unskewed_errors))
    print(f"Half-player source {source_height:g}px; floor depth {depth:g}px; projection error {error:.3f}/255; flat error {flat_error:.3f}/255; unskewed error {unskewed_error:.3f}/255")
    assert error < 3, f"Floor projection pixel error: {error:.3f} levels"
    assert error < flat_error * 0.35, "Reflection has no visible player detail"
    assert error < unskewed_error * 0.5, "Reflection does not show the floor perspective and skew"


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("capture")
    parser.add_argument("skin")
    parser.add_argument("--contact", type=float, required=True)
    parser.add_argument("--player-top", type=float, default=0)
    parser.add_argument("--bottom-inset", type=float, default=0)
    parser.add_argument("--left", type=float, default=0)
    parser.add_argument("--width", type=float)
    args = parser.parse_args()
    check(args.capture, args.skin, args.contact, args.player_top, args.bottom_inset, args.left, args.width)
