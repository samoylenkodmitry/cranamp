#!/usr/bin/env python3
"""Append native compiler notices that Cargo dependency metadata does not include."""

import argparse
import html
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DXC_SOURCE = "https://github.com/hexops/DirectXShaderCompiler/tree/4190bb0c90d374c6b4d0b0f2c7b45b604eda24b6"
MACH_SOURCE = "https://github.com/DouglasDwyer/mach-dxcompiler/tree/2026.09.16%2B48d5a66.1"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target")
    parser.add_argument("notices", type=Path)
    args = parser.parse_args()
    if args.target != "x86_64-pc-windows-msvc":
        return
    sections = []
    for name, source in [
        ("mach-dxcompiler-LICENSE-MIT.txt", MACH_SOURCE),
        ("DirectXShaderCompiler-LICENSE.txt", DXC_SOURCE),
        ("DirectXShaderCompiler-ThirdPartyNotices.txt", DXC_SOURCE),
    ]:
        text = (ROOT / "docs/third-party" / name).read_text(encoding="utf-8")
        sections.append(
            f'<section><h2>{name}</h2><p>Windows bundled shader compiler: '
            f'<a href="{source}">exact source revision</a>.</p>'
            f"<pre>{html.escape(text)}</pre></section>"
        )
    notices = args.notices.read_text(encoding="utf-8")
    if notices.count("</html>") != 1:
        raise ValueError("Expected one closing HTML tag in generated notices")
    args.notices.write_text(
        notices.replace("</html>", "\n".join(sections) + "</html>"),
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
