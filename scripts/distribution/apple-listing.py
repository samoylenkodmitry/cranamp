"""Stage the reviewed release text and native screenshots for fastlane deliver."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import tomllib


def stage(platform, destination):
    root = Path(__file__).resolve().parents[2]
    version = tomllib.loads((root / "Cargo.toml").read_text())["package"]["version"]
    release = root / "store/releases" / version
    listing = json.loads((release / "listing-en-US.json").read_text())
    manifest = json.loads((release / "screenshots.json").read_text())
    assert listing["version"] == manifest["version"] == version
    families = {"macos"} if platform == "macos" else {"ios-small", "ios-6.9", "ipad-11", "ipad-13"}
    galleries = [g for g in manifest["galleries"] if g["family"] in families]
    assert {g["family"] for g in galleries} == families
    for gallery in galleries:
        assert gallery["status"] == "ready"
        assert 1 <= len(gallery["images"]) <= 10
        for image in gallery["images"]:
            path = release / image["file"]
            assert hashlib.sha256(path.read_bytes()).hexdigest() == image["sha256"], path

    metadata = destination / "metadata"
    localized = metadata / listing["locale"]
    screenshots = destination / "screenshots" / listing["locale"]
    review = metadata / "review_information"
    for path in (localized, screenshots, review):
        path.mkdir(parents=True, exist_ok=False)
    text = listing["apple_" + platform]
    for key, value in {
        "description": text["description"],
        "keywords": text["keywords"],
        "release_notes": text["whats_new"],
        "support_url": listing["links"]["support_url"],
        "marketing_url": listing["links"]["website"],
    }.items():
        (localized / (key + ".txt")).write_text(value)
    (metadata / "copyright.txt").write_text(listing["links"]["copyright"])
    (review / "notes.txt").write_text(text["review_notes"])
    for gallery in galleries:
        for image in gallery["images"]:
            source = release / image["file"]
            shutil.copyfile(source, screenshots / (gallery["family"] + "-" + source.name))
    (destination / "version.txt").write_text(version)
    print(f"Staged {platform} {version}: {len(list(screenshots.iterdir()))} screenshots")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("platform", choices=["ios", "macos"])
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    stage(args.platform, args.destination)
