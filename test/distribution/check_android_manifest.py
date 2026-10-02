#!/usr/bin/env python3
"""Fail the store build if its final Android manifest allows APK installation."""
import pathlib
import sys
import xml.etree.ElementTree as ET

ANDROID = "{http://schemas.android.com/apk/res/android}"

def verify(path):
    root = ET.parse(path).getroot()
    for node in root.iter():
        name = node.get(ANDROID + "name", "")
        if name == "android.permission.REQUEST_INSTALL_PACKAGES" or name.endswith("CranposeAppUpdate"):
            raise ValueError(f"Store manifest still contains updater component: {name}")

if __name__ == "__main__":
    verify(pathlib.Path(sys.argv[1]))
