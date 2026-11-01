import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("play_upload", ROOT / "scripts/distribution/play-upload.py")
play_upload = importlib.util.module_from_spec(spec)
spec.loader.exec_module(play_upload)


class PlayUploadTests(unittest.TestCase):
    def test_the_release_notes_come_from_the_reviewed_listing(self):
        version = play_upload.release_version()
        notes = play_upload.release_notes(version)
        self.assertTrue(notes)
        self.assertLessEqual(len(notes), play_upload.NOTES_LIMIT)

    def test_a_full_release_reaches_everyone(self):
        release = play_upload.track_release(10093, "0.1.93", "Notes")
        self.assertEqual(release["status"], "completed")
        self.assertEqual(release["versionCodes"], ["10093"])
        self.assertEqual(release["name"], "10093 (0.1.93)")
        self.assertNotIn("userFraction", release)

    def test_a_staged_release_names_its_fraction(self):
        release = play_upload.track_release(10093, "0.1.93", "Notes", 0.2)
        self.assertEqual((release["status"], release["userFraction"]), ("inProgress", 0.2))
        with self.assertRaises(SystemExit):
            play_upload.track_release(10093, "0.1.93", "Notes", 1.5)


if __name__ == "__main__":
    unittest.main()
