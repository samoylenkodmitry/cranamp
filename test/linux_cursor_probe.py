"""Read the X11 cursor metadata after a manual hover; never move the pointer."""
import ctypes
import hashlib
import json


class CursorImage(ctypes.Structure):
    _fields_ = [
        ("x", ctypes.c_short), ("y", ctypes.c_short),
        ("width", ctypes.c_ushort), ("height", ctypes.c_ushort),
        ("xhot", ctypes.c_ushort), ("yhot", ctypes.c_ushort),
        ("serial", ctypes.c_ulong),
        ("pixels", ctypes.POINTER(ctypes.c_ulong)),
        ("atom", ctypes.c_ulong), ("name", ctypes.c_char_p),
    ]


xlib = ctypes.CDLL("libX11.so.6")
fixes = ctypes.CDLL("libXfixes.so.3")
xlib.XOpenDisplay.argtypes = [ctypes.c_char_p]
xlib.XOpenDisplay.restype = ctypes.c_void_p
xlib.XCloseDisplay.argtypes = [ctypes.c_void_p]
xlib.XFree.argtypes = [ctypes.c_void_p]
fixes.XFixesGetCursorImage.argtypes = [ctypes.c_void_p]
fixes.XFixesGetCursorImage.restype = ctypes.POINTER(CursorImage)
display = xlib.XOpenDisplay(None)
if not display:
    raise SystemExit("Cannot open the X11 display")
cursor = fixes.XFixesGetCursorImage(display)
if not cursor:
    xlib.XCloseDisplay(display)
    raise SystemExit("XFixes did not return a cursor")
try:
    image = cursor.contents
    pixels = b"".join((int(image.pixels[i]) & 0xffffffff).to_bytes(4, "little")
                      for i in range(image.width * image.height))
    print(json.dumps({"position": [image.x, image.y], "serial": image.serial,
                      "width": image.width, "height": image.height,
                      "hotspot": [image.xhot, image.yhot],
                      "sha256": hashlib.sha256(pixels).hexdigest()}))
finally:
    xlib.XFree(cursor)
    xlib.XCloseDisplay(display)
