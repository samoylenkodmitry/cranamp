import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/distribution"))
sys.path.insert(0, str(Path(__file__).parent))
from version import release_version
from check_android_manifest import verify


class PackagingTests(unittest.TestCase):
    def check_manifest(self, entry):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "AndroidManifest.xml"
            path.write_text('<manifest xmlns:android="http://schemas.android.com/apk/res/android">' + entry + '</manifest>')
            verify(path)

    def test_store_rejects_apk_permission_and_receiver(self):
        for entry in ['<uses-permission android:name="android.permission.REQUEST_INSTALL_PACKAGES"/>',
                      '<application><receiver android:name="dev.cranpose.android.CranposeAppUpdate"/></application>']:
            with self.subTest(entry=entry), self.assertRaises(ValueError):
                self.check_manifest(entry)

    def test_store_allows_media_and_network_permissions(self):
        self.check_manifest('<uses-permission android:name="android.permission.INTERNET"/><uses-permission android:name="android.permission.FOREGROUND_SERVICE_MEDIA_PLAYBACK"/>')

    def test_release_tag_must_match_the_application_version(self):
        with patch.dict(os.environ, {"GITHUB_REF": "refs/tags/v999.999.999"}):
            with self.assertRaises(SystemExit):
                release_version()
        with patch.dict(os.environ, {"GITHUB_REF": "refs/heads/main"}):
            version = release_version()
        with patch.dict(os.environ, {"GITHUB_REF": "refs/tags/v" + version}):
            self.assertEqual(release_version(), version)


if __name__ == "__main__":
    unittest.main()
