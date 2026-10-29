"""Repair duplicate bundled clock glyphs through Studio's shared native pen.

Run against an isolated Studio MCP session. Outputs stay in the supplied review
directory until the editor and GPU review is complete. No bitmap writer is used.
"""
import argparse
import os
from collections import Counter
import json
from pathlib import Path
import struct
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[2]
SKINS = ["Silverplay", "Catnip", "Midnight Snack", "Moon Garden", "Moonlit"]
SEGMENTS = ["abcdef", "bc", "abdeg", "abcdg", "bcfg", "acdfg", "acdefg", "abc", "abcdefg", "abcdfg"]


def call(name, arguments):
    request = urllib.request.Request(
        os.environ["CRANAMP_MCP_URL"],
        json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                    "params": {"name": name, "arguments": arguments}}).encode(),
        headers={"Content-Type": "application/json"},
    )
    result = json.load(urllib.request.urlopen(request, timeout=30))
    if "error" in result or result.get("result", {}).get("isError"):
        raise RuntimeError(result)
    return result


def cells(data):
    offset = struct.unpack_from("<I", data, 10)[0]
    width, height = struct.unpack_from("<ii", data, 18)
    bits = struct.unpack_from("<H", data, 28)[0]
    assert bits in (24, 32) and abs(height) == 13
    stride = ((width * bits + 31) // 32) * 4
    def pixel(x, y):
        row = height - 1 - y if height > 0 else y
        at = offset + row * stride + x * bits // 8
        return "#" + data[at:at + 3][::-1].hex()
    return [[[pixel(d * 9 + x, y) for x in range(9)] for y in range(13)] for d in range(10)]


def segment_at(x, y):
    if 2 <= x <= 6 and y in (1, 6, 12):
        return {1: "a", 6: "g", 12: "d"}[y]
    if x in (1, 7) and 2 <= y <= 5:
        return "f" if x == 1 else "b"
    if x in (1, 7) and 7 <= y <= 11:
        return "e" if x == 1 else "c"
    return ""


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    output = parser.parse_args().output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    for name in SKINS:
        source = ROOT / "assets/skins" / f"Catamp {name}.wsz"
        project = source.with_suffix(".cstudio")
        with zipfile.ZipFile(source) as archive:
            entries = {n: archive.read(n) for n in archive.namelist()}
        glyphs = cells(entries["numbers.bmp"])
        background, ink = [item[0] for item in Counter(p for row in glyphs[0] for p in row).most_common(2)]
        call("studio_open", {"path": str(project if project.exists() else source), "discard": True})
        call("studio_project", {"action": "save", "path": str(output / f"{name}-before.cstudio")})
        call("studio_study", {"rect": [44, 22, 59, 21], "states": True,
                               "path": str(output / f"{name}-before.png")})
        for digit, cell in enumerate(glyphs):
            call("studio_canvas", {"surface": "skin", "digit": digit,
                                    "states": "current", "drawer": "none"})
            colors = []
            rows = []
            for y, row in enumerate(cell):
                encoded = ""
                for x, old in enumerate(row):
                    color = background if old == ink else old
                    segment = segment_at(x, y)
                    if segment and segment in SEGMENTS[digit]:
                        color = ink
                    if color not in colors:
                        colors.append(color)
                    encoded += chr(65 + colors.index(color))
                rows.append(encoded)
            result = call("studio_draw", {
                "origin": "main.digit0", "states": "current",
                "label": f"Correct clock digit {digit}",
                "operations": [{"op": "stamp", "x": 0, "y": 0, "rows": rows,
                                "palette": {chr(65 + i): c for i, c in enumerate(colors)}}],
            })
            (output / f"{name}-digit-{digit}.json").write_text(json.dumps(result, indent=2))
        call("studio_atlas", {"sheet": "numbers.bmp", "magnify": 8,
                              "path": str(output / f"{name}-digits.png")})
        call("studio_canvas", {"surface": "skin", "digit": 3})
        call("studio_study", {"rect": [44, 22, 59, 21], "states": True,
                               "path": str(output / f"{name}-after.png")})
        target = output / source.name
        call("studio_export", {"path": str(target)})
        with zipfile.ZipFile(target) as archive:
            changed = [n for n, b in entries.items() if archive.read(n) != b]
            assert set(archive.namelist()) == set(entries)
        assert changed == ["numbers.bmp"], (name, changed)
        print(f"{name}: only numbers.bmp changed", flush=True)


if __name__ == "__main__":
    main()
