#!/usr/bin/env python3
"""One version for store manifests and packages; tags must agree with Cargo."""
import os
import pathlib
import re
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[2]

def release_version():
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise SystemExit("Store builds require a numeric MAJOR.MINOR.PATCH version")
    tag = os.environ.get("GITHUB_REF", "")
    if tag.startswith("refs/tags/") and tag != "refs/tags/v" + version:
        raise SystemExit(f"Release tag {tag} does not match Cargo version {version}")
    return version

if __name__ == "__main__":
    print(release_version())
