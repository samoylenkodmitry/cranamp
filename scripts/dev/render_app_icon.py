#!/usr/bin/env python3
"""Draws the Cranamp icon and writes every platform's copy of it.

    python3 scripts/dev/render_app_icon.py
    python3 scripts/dev/render_app_icon.py --sheet <png>   # every output side by side

The icon is the Catamp cat wearing the player's display as glasses, a
spectrum in each lens, on the player's slate-teal plate. One drawing is
placed the way each platform expects: a rounded plate with room for the
Windows, Linux and web renderings, Apple's 824-point body with a shadow for
macOS, a full-bleed square that iOS masks itself, and a transparent layer
inside the 66dp safe zone for Android's adaptive icon. Small sizes drop the
whiskers, stripes and mouth, which only blur at that size.

Needs Pillow. Every output is committed, so a build never runs this.
"""

import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[2]
ICON_DIR = ROOT / "assets" / "icon"
IOS_DIR = ROOT / "platform" / "ios" / "Assets.xcassets" / "AppIcon.appiconset"
ANDROID_RES = ROOT / "platform" / "android" / "app" / "src" / "main" / "res"

PLATE_TOP = (47, 94, 112)
PLATE_BOTTOM = (13, 32, 41)
BEVEL = (164, 222, 234)
FUR = (221, 233, 237)
FUR_SHADE = (184, 203, 210)
STRIPE = (116, 142, 153)
EAR_INNER = (212, 146, 163)
LENS = (7, 20, 26)
FRAME = (116, 204, 224)
SPECTRUM_LOW = (84, 214, 170)
SPECTRUM_HIGH = (134, 240, 255)
NOSE = (205, 118, 140)
MOUTH = (96, 118, 128)

SUPERSAMPLE = 4
DETAIL_FROM = 48
WINDOWS_SIZES = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256]
ANDROID_FOREGROUND = {"mdpi": 108, "hdpi": 162, "xhdpi": 216, "xxhdpi": 324, "xxxhdpi": 432}


def mix(a, b, t):
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))


def gradient(size, top, bottom):
    column = Image.new("RGBA", (1, size))
    for y in range(size):
        column.putpixel((0, y), mix(top, bottom, y / max(size - 1, 1)) + (255,))
    return column.resize((size, size))


def rounded_mask(size, box, radius):
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle(box, radius=radius, fill=255)
    return mask


def plate(canvas, box, radius):
    size = canvas.size[0]
    width = box[2] - box[0]
    fill = gradient(size, PLATE_TOP, PLATE_BOTTOM)
    canvas.paste(fill, (0, 0), rounded_mask(size, box, radius))
    edge = max(1, round(width * 0.018))
    rim = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    ImageDraw.Draw(rim).rounded_rectangle(box, radius=radius, outline=BEVEL + (150,), width=edge)
    fade = gradient(size, (255, 255, 255), (0, 0, 0)).convert("L")
    rim.putalpha(Image.composite(rim.getchannel("A"), Image.new("L", (size, size), 0), fade))
    canvas.alpha_composite(rim)


def cat(canvas, box, detail):
    x0, y0, x1, y1 = box
    side = x1 - x0
    draw = ImageDraw.Draw(canvas)

    def p(x, y):
        return (x0 + x * side, y0 + y * side)

    def r(x, y, w, h):
        return [p(x, y), p(x + w, y + h)]

    def stroke(width):
        return max(1, round(width * side))

    for sign in (-1, 1):
        def ear(dx, dy):
            return p(0.5 + sign * dx, dy)

        draw.polygon([ear(0.31, 0.44), ear(0.25, 0.12), ear(0.07, 0.33)], fill=FUR_SHADE)
        draw.polygon([ear(0.27, 0.38), ear(0.235, 0.19), ear(0.12, 0.33)], fill=EAR_INNER)

    draw.ellipse(r(0.15, 0.27, 0.70, 0.58), fill=FUR_SHADE)
    draw.ellipse(r(0.155, 0.265, 0.69, 0.545), fill=FUR)

    if detail:
        for dx, top, height in ((-0.075, 0.31, 0.09), (0.0, 0.295, 0.11), (0.075, 0.31, 0.09)):
            draw.rounded_rectangle(
                r(0.5 + dx - 0.017, top, 0.034, height), radius=0.017 * side, fill=STRIPE
            )

    lens_top, lens_height = 0.455, 0.165
    lenses = [(0.215, 0.255), (0.53, 0.255)]
    draw.rounded_rectangle(r(0.46, 0.505, 0.08, 0.04), radius=0.01 * side, fill=FRAME)
    for index, (left, width) in enumerate(lenses):
        frame = r(left, lens_top, width, lens_height)
        draw.rounded_rectangle(frame, radius=0.045 * side, fill=LENS, outline=FRAME, width=stroke(0.016))
        heights = [(0.45, 0.85, 0.65, 0.35), (0.75, 0.5, 0.9, 0.55)][index]
        pad, gap = 0.035, 0.014
        bar_width = (width - 2 * pad - gap * (len(heights) - 1)) / len(heights)
        floor = lens_top + lens_height - 0.03
        reach = lens_height - 0.06
        for column, level in enumerate(heights):
            left_edge = left + pad + column * (bar_width + gap)
            color = mix(SPECTRUM_LOW, SPECTRUM_HIGH, level)
            draw.rectangle(r(left_edge, floor - reach * level, bar_width, reach * level), fill=color)

    draw.polygon([p(0.465, 0.665), p(0.535, 0.665), p(0.5, 0.705)], fill=NOSE)
    if detail:
        mouth = stroke(0.012)
        draw.arc(r(0.43, 0.67, 0.07, 0.07), 20, 160, fill=MOUTH, width=mouth)
        draw.arc(r(0.50, 0.67, 0.07, 0.07), 20, 160, fill=MOUTH, width=mouth)
        whiskers = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
        lines = ImageDraw.Draw(whiskers)
        for sign in (-1, 1):
            for start, end in ((0.66, 0.62), (0.70, 0.71), (0.74, 0.80)):
                lines.line(
                    [p(0.5 + sign * 0.3, start), p(0.5 + sign * 0.47, end)],
                    fill=FUR + (190,),
                    width=stroke(0.011),
                )
        canvas.alpha_composite(whiskers)


def render(size, layout):
    big = size * SUPERSAMPLE
    canvas = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    detail = size >= DETAIL_FROM
    if layout == "plate":
        inset = round(big * 0.03)
        body = (inset, inset, big - inset, big - inset)
        plate(canvas, body, radius=big * 0.2)
        mark = round(big * 0.09)
        cat(canvas, (mark, mark, big - mark, big - mark), detail)
    elif layout == "macos":
        inset = round(big * 100 / 1024)
        body = (inset, inset, big - inset, big - inset)
        shadow = Image.new("RGBA", (big, big), (0, 0, 0, 0))
        offset = round(big * 12 / 1024)
        ImageDraw.Draw(shadow).rounded_rectangle(
            (inset, inset + offset, big - inset, big - inset + offset),
            radius=big * 185 / 1024,
            fill=(0, 0, 0, 110),
        )
        canvas.alpha_composite(shadow.filter(ImageFilter.GaussianBlur(big * 14 / 1024)))
        plate(canvas, body, radius=big * 185 / 1024)
        mark = inset + round(big * 0.06)
        cat(canvas, (mark, mark, big - mark, big - mark), detail)
    elif layout == "full-bleed":
        canvas.paste(gradient(big, PLATE_TOP, PLATE_BOTTOM), (0, 0))
        mark = round(big * 0.07)
        cat(canvas, (mark, mark, big - mark, big - mark), detail)
    elif layout == "adaptive-foreground":
        mark = round(big * 0.2)
        cat(canvas, (mark, mark, big - mark, big - mark), detail)
    else:
        raise ValueError(layout)
    return canvas.resize((size, size), Image.LANCZOS)


def write(image, path):
    path.parent.mkdir(parents=True, exist_ok=True)
    image.save(path, optimize=True)
    print(path.relative_to(ROOT))


def contact_sheet(path):
    ico = Image.open(ICON_DIR / "cranamp.ico")
    sizes = sorted(ico.info["sizes"])
    renderings = []
    for size in sizes:
        ico.size = size
        renderings.append(ico.convert("RGBA").copy())
    previews = [
        Image.open(ICON_DIR / "Cranamp.icns"),
        Image.open(IOS_DIR / "AppIcon-1024.png"),
        Image.open(ANDROID_RES / "mipmap-xxxhdpi" / "ic_launcher_foreground.png"),
    ]
    width = sum(image.width + 12 for image in renderings) + 3 * 268 + 12
    sheet = Image.new("RGBA", (width, 292), (236, 239, 242, 255))
    x = 12
    for image in renderings:
        sheet.alpha_composite(image, (x, 280 - image.height))
        x += image.width + 12
    for preview in previews:
        sheet.alpha_composite(preview.convert("RGBA").resize((256, 256), Image.LANCZOS), (x, 24))
        x += 268
    sheet.save(path)
    print(path)


def main():
    if len(sys.argv) == 3 and sys.argv[1] == "--sheet":
        contact_sheet(Path(sys.argv[2]))
        return
    write(render(1024, "plate"), ICON_DIR / "cranamp.png")
    write(render(128, "plate"), ICON_DIR / "cranamp-window.png")

    windows = [render(size, "plate") for size in WINDOWS_SIZES]
    ico = ICON_DIR / "cranamp.ico"
    windows[-1].save(ico, sizes=[(s, s) for s in WINDOWS_SIZES], append_images=windows[:-1])
    print(ico.relative_to(ROOT))

    icns = ICON_DIR / "Cranamp.icns"
    render(1024, "macos").save(icns)
    print(icns.relative_to(ROOT))

    write(render(64, "plate"), ICON_DIR / "favicon.png")
    write(render(192, "plate"), ICON_DIR / "icon-192.png")
    write(render(512, "plate"), ICON_DIR / "icon-512.png")
    write(render(512, "full-bleed").convert("RGB"), ICON_DIR / "icon-maskable-512.png")
    write(render(180, "full-bleed").convert("RGB"), ICON_DIR / "apple-touch-icon.png")

    write(render(1024, "full-bleed").convert("RGB"), IOS_DIR / "AppIcon-1024.png")

    for density, size in ANDROID_FOREGROUND.items():
        write(render(size, "adaptive-foreground"), ANDROID_RES / f"mipmap-{density}" / "ic_launcher_foreground.png")


if __name__ == "__main__":
    main()
