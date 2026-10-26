"""Capture the actual desktop MCP UI with the native GPU review executable."""
import html
import json
import os
from pathlib import Path
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[2]
BINARY = ROOT / "target/debug/cranamp-studio-review"
OUTPUT = ROOT / "target/mcp-ui-review"
SIZES = [(320, 780), (520, 820), (900, 900), (1280, 900), (1440, 900), (1920, 1080), (1440, 420)]


def main():
    OUTPUT.mkdir(parents=True, exist_ok=True)
    cases = json.loads(subprocess.check_output([BINARY, "--list"], text=True))
    cards = []
    for width, height in SIZES:
        for state in ["agent-connection", "agent-running"]:
            name = f"{width}x{height}-{state}.png"
            path = OUTPUT / name
            env = dict(
                os.environ,
                CRANAMP_REVIEW_CAPTURE=str(path),
                CRANAMP_REVIEW_CAPTURE_ONCE="1",
                CRANAMP_REVIEW_CAPTURE_RGB="1",
            )
            with (OUTPUT / "capture.log").open("a") as log:
                subprocess.run(
                    [BINARY, str(width), str(height), str(cases.index(state))],
                    env=env, stdout=log, stderr=log, check=True, timeout=90,
                )
            header = path.read_bytes()[:26]
            actual = struct.unpack(">II", header[16:24])
            assert actual == (width, height), (name, actual)
            assert header[25] == 2, "Listing captures must use RGB without alpha"
            print(name, flush=True)
            caption = html.escape(f"{width} x {height}: {state}")
            cards.append(f'<figure><a href="{name}"><img src="{name}" alt="{caption}"></a><figcaption>{caption}</figcaption></figure>')
    (OUTPUT / "index.html").write_text('<!doctype html><meta charset="utf-8"><title>Desktop MCP layout review</title><style>body{background:#0b1018;color:#e8f6ff;font:14px system-ui;margin:24px}main{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:20px}figure{margin:0}img{width:100%;height:440px;object-fit:contain;background:#111a26}figcaption{padding:12px}</style><h1>Desktop MCP layout review</h1><p>Native GPU captures. Small widths are desktop windows. Mobile and web builds exclude MCP.</p><main>' + ''.join(cards) + '</main>')


if __name__ == "__main__":
    main()
