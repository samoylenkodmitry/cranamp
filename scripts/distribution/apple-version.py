#!/usr/bin/env python3
"""Write the release version into an assembled Apple app bundle."""
import os
import pathlib
import plistlib
import sys

from version import release_version


def stamp(path, version, build):
    if not build.isdecimal() or int(build) < 1:
        raise ValueError("CRANAMP_BUILD_NUMBER must be a positive integer")
    path = pathlib.Path(path)
    data = plistlib.loads(path.read_bytes())
    data["CFBundleShortVersionString"] = version
    data["CFBundleVersion"] = build
    path.write_bytes(plistlib.dumps(data))


if __name__ == "__main__":
    stamp(sys.argv[1], release_version(), os.environ.get("CRANAMP_BUILD_NUMBER", "1"))
