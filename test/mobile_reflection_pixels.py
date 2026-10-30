"""Check a GPU capture of the player standing on a horizontal glass plane.

Requires Pillow. Example with 440x956 and CRANAMP_REVIEW_PHONE_INSETS=1:
  python test/mobile_reflection_pixels.py capture.png 'assets/skins/Catamp Silverplay.wsz' --contact 840.5 --player-top 59 --bottom-inset 34

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
    source_height = (contact - player_top) / 3
    depth = frame.height - contact
    errors, flat_errors, rightward_errors, half_errors = [], [], [], []
    safe_area_detail = 0
    # The navbar covers most of the floor. Inspect its exposed outer edges all
    # the way through the home-indicator inset. This avoids sampling the glass
    # buttons and their shadows as if they were part of the reflected player.
    edge_columns = [0, 1, frame.width - 2, frame.width - 1]
    for y in range(math.ceil(contact + 2), frame.height):
        distance = y + 0.5 - contact
        t = distance / depth
        stretch = 1 + 0.24 * t
        source_fraction = t * 1.24 / stretch
        opacity = 0.42 * (1 - t) ** 2
        sheen = 0.035 * math.exp(-distance / 5)
        for x in edge_columns:
            actual = frame.getpixel((x, y))
            for height, skew, result in [(source_height, -0.18, errors),
                                         (source_height, 0.18, rightward_errors),
                                         ((contact - player_top) / 2, -0.18, half_errors)]:
                mirrored_x = (x + 0.5 - left - width * (0.5 + skew * t)) / stretch + width / 2
                if 0 <= mirrored_x <= width:
                    reflected = sample(frame, left + mirrored_x - 0.5, contact - height * source_fraction - 0.5)
                    expected = [(corner[c] * (1-opacity) + reflected[c] * opacity) * (1-sheen) + 255 * sheen for c in range(3)]
                else:
                    expected = corner
                result.extend(abs(actual[c] - expected[c]) for c in range(3))
            flat_errors.extend(abs(actual[c] - corner[c]) for c in range(3))
            if y >= frame.height - bottom_inset and max(abs(actual[c] - corner[c]) for c in range(3)) > 1:
                safe_area_detail += 1
    mean = lambda values: sum(values) / len(values)
    error, flat_error, rightward_error, half_error = map(mean, (errors, flat_errors, rightward_errors, half_errors))
    print(f"Third-player source {source_height:g}px; floor depth {depth:g}px; projection error {error:.3f}/255; flat error {flat_error:.3f}/255; rightward error {rightward_error:.3f}/255; half-player error {half_error:.3f}/255")
    assert error < 3, f"Floor projection pixel error: {error:.3f} levels"
    assert error < flat_error * 0.35, "Reflection has no visible player detail"
    assert error < rightward_error * 0.5, "Reflection does not skew left"
    assert error < half_error * 0.8, "Reflection source does not match the lower third"
    if bottom_inset:
        assert safe_area_detail > 0, "Reflection stops before the bottom safe area"


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
