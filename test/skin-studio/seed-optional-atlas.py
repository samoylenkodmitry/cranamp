"""Add an optional canvas using an existing blank Studio bitmap.

This only repackages Studio's own base bytes. All artwork is subsequently drawn
with studio_draw. No image is decoded, generated, or changed by this helper.
"""
import io
import sys
import zipfile
from pathlib import Path

source, destination = map(Path, sys.argv[1:3])
with zipfile.ZipFile(source) as project:
    files = {name: project.read(name) for name in project.namelist()}
with zipfile.ZipFile(io.BytesIO(files["base.wsz"])) as base:
    entries = {name: base.read(name) for name in base.namelist()}
if "eq_ex.bmp" in entries:
    raise SystemExit("The optional atlas already exists")
entries["eq_ex.bmp"] = entries["main.bmp"]
buffer = io.BytesIO()
with zipfile.ZipFile(buffer, "w", zipfile.ZIP_DEFLATED) as base:
    for name, data in entries.items():
        base.writestr(name, data)
files["base.wsz"] = buffer.getvalue()
files["base/eq_ex.bmp.png"] = files["base/main.bmp.png"]
with zipfile.ZipFile(destination, "w", zipfile.ZIP_DEFLATED) as project:
    for name, data in files.items():
        project.writestr(name, data)
