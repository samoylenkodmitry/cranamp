"""Read release states without exporting Apple credentials or reviewer contacts."""

import base64
import json
import os
import time
import urllib.error
import urllib.parse
import urllib.request

import jwt


def main():
    now = int(time.time())
    token = jwt.encode(
        {"iss": os.environ["ASC_ISSUER_ID"], "iat": now, "exp": now + 600,
         "aud": "appstoreconnect-v1"},
        base64.b64decode(os.environ["ASC_P8_BASE64"]),
        algorithm="ES256", headers={"kid": os.environ["ASC_KEY_ID"], "typ": "JWT"},
    )

    def get(path, params):
        url = "https://api.appstoreconnect.apple.com/v1/" + path
        request = urllib.request.Request(
            url + "?" + urllib.parse.urlencode(params),
            headers={"Authorization": "Bearer " + token},
        )
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                return json.load(response)
        except urllib.error.HTTPError as error:
            # Never print request headers or credential material.
            details = json.loads(error.read())
            for item in details.get("errors", []):
                print(item.get("code"), item.get("title"), item.get("detail"))
            raise SystemExit(f"Apple status request failed: HTTP {error.code}") from None

    app = "6818451356"
    versions = get(f"apps/{app}/appStoreVersions", {
        "limit": 50, "include": "build",
        "fields[appStoreVersions]": "platform,versionString,appStoreState,appVersionState,releaseType,createdDate,build",
        "fields[builds]": "version,processingState,uploadedDate",
    })
    reviews = get(f"apps/{app}/reviewSubmissions", {
        "limit": 50, "include": "items,appStoreVersionForReview",
        "fields[reviewSubmissions]": "platform,submittedDate,state,items,appStoreVersionForReview",
        "fields[reviewSubmissionItems]": "state,appStoreVersion",
        "fields[appStoreVersions]": "platform,versionString,appVersionState",
    })
    # These field selections contain release status only, with no contact data.
    print(json.dumps({"versions": versions, "reviews": reviews}, indent=2))


if __name__ == "__main__":
    main()
