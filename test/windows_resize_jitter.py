#!/usr/bin/env python3
"""Measures how much the player shakes while test/windows-resize-jitter.ps1
stretches the playlist.

Only the playlist grows, so the main window and the equalizer above it, the
top ``--still`` rows, should stay exactly as they were before the drag. Each
frame is compared with ``frame-000.png`` over that band, leaving out the
main window's display, whose clock and title scroll on their own. The script
prints the share of changed pixels per frame and fails when any frame changes
more than ``--tolerance`` of them.

    python3 test/windows_resize_jitter.py /path/to/cranamp-jitter
"""

import argparse
import pathlib
import sys

from PIL import Image, ImageChops

# The main window's display: time, visualizer and scrolling title.
DISPLAY = (9, 20, 275, 60)


def changed_share(before: Image.Image, after: Image.Image, still: int) -> float:
    band = (0, 0, before.width, still)
    difference = ImageChops.difference(before.crop(band), after.crop(band)).convert("L")
    pixels = difference.load()
    counted = changed = 0
    for y in range(still):
        for x in range(before.width):
            if DISPLAY[0] <= x < DISPLAY[2] and DISPLAY[1] <= y < DISPLAY[3]:
                continue
            counted += 1
            if pixels[x, y] > 24:
                changed += 1
    return changed / max(counted, 1)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("frames", type=pathlib.Path)
    parser.add_argument("--still", type=int, default=232)
    parser.add_argument("--tolerance", type=float, default=0.01)
    args = parser.parse_args()
    frames = sorted(args.frames.glob("frame-*.png"))
    baseline = Image.open(args.frames / "frame-000.png").convert("RGB")
    worst = 0.0
    shaken = 0
    for frame in frames:
        share = changed_share(baseline, Image.open(frame).convert("RGB"), args.still)
        worst = max(worst, share)
        shaken += share > args.tolerance
        print(f"{frame.name}: {share:.2%} of the still band changed")
    print(f"{shaken} of {len(frames)} frames shook; worst {worst:.2%}")
    return 1 if shaken else 0


if __name__ == "__main__":
    sys.exit(main())
