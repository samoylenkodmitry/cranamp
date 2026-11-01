#!/usr/bin/env python3
"""Upload a signed Android App Bundle to a Google Play track and send it for review.

The release notes come from the reviewed listing of the Cargo version. The
service account key is read from PLAY_SERVICE_ACCOUNT_JSON.
"""
import json
import os
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/distribution"))
from version import release_version

PACKAGE = "com.cranamp.app"
LANGUAGE = "en-US"
NOTES_LIMIT = 500


def release_notes(version):
    listing = json.loads((ROOT / "store/releases" / version / "listing-en-US.json").read_text())
    assert listing["version"] == version
    notes = listing["google_play"]["release_notes"].strip()
    if len(notes) > NOTES_LIMIT:
        raise SystemExit(f"Google Play release notes exceed {NOTES_LIMIT} characters")
    return notes


def track_release(version_code, version, notes, rollout=None):
    release = {
        "name": f"{version_code} ({version})",
        "versionCodes": [str(version_code)],
        "status": "completed" if rollout is None else "inProgress",
        "releaseNotes": [{"language": LANGUAGE, "text": notes}],
    }
    if rollout is not None:
        if not 0 < rollout < 1:
            raise SystemExit("A staged rollout fraction lies between 0 and 1")
        release["userFraction"] = rollout
    return release


def main():
    from google.oauth2 import service_account
    from googleapiclient.discovery import build
    from googleapiclient.http import MediaFileUpload

    bundle, track = sys.argv[1], os.environ.get("PLAY_TRACK", "production")
    rollout = float(os.environ["PLAY_ROLLOUT"]) if os.environ.get("PLAY_ROLLOUT") else None
    version = release_version()
    notes = release_notes(version)
    credentials = service_account.Credentials.from_service_account_info(
        json.loads(os.environ["PLAY_SERVICE_ACCOUNT_JSON"]),
        scopes=["https://www.googleapis.com/auth/androidpublisher"],
    )
    edits = build("androidpublisher", "v3", credentials=credentials, cache_discovery=False).edits()
    edit = edits.insert(packageName=PACKAGE, body={}).execute()["id"]
    media = MediaFileUpload(bundle, mimetype="application/octet-stream", resumable=True)
    version_code = edits.bundles().upload(packageName=PACKAGE, editId=edit, media_body=media).execute()["versionCode"]
    edits.tracks().update(
        packageName=PACKAGE,
        editId=edit,
        track=track,
        body={"track": track, "releases": [track_release(version_code, version, notes, rollout)]},
    ).execute()
    edits.commit(packageName=PACKAGE, editId=edit).execute()
    print(f"Sent {version} ({version_code}) to the {track} track for review")


if __name__ == "__main__":
    main()
