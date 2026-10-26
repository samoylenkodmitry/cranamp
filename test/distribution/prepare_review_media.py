#!/usr/bin/env python3
"""Create a quiet synthetic audio file for a store review recording."""
import argparse
import math
from pathlib import Path
import struct
import wave


def generate(path):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    rate = 16000
    # One second, with a short fade at each end. Repeat it for two minutes so
    # the reviewer can inspect background controls without racing the track.
    frames = bytearray()
    for index in range(rate):
        edge = min(index / (rate * 0.02), (rate - 1 - index) / (rate * 0.02), 1)
        sample = int(800 * max(0, edge) * math.sin(2 * math.pi * 440 * index / rate))
        frames.extend(struct.pack("<h", sample))
    with wave.open(str(path), "wb") as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(rate)
        for _ in range(120):
            output.writeframesraw(frames)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    generate(parser.parse_args().output)
