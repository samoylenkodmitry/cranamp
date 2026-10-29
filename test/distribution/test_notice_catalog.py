from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts/distribution"))
from notice_catalog import catalog


class NoticeCatalogTests(unittest.TestCase):
    def test_preserves_license_text_and_source_links(self):
        notices = catalog('''<html><section><h2>MIT &amp; Apache</h2>
            <ul><li><a href="https://example.test/source">crate 1.2</a></li></ul>
            <pre>Copyright &lt;author&gt;\n\nPermission &amp; warranty.</pre></section>
            <section><h2>Native library</h2><pre>Second notice</pre></section></html>''')
        self.assertEqual(len(notices), 2)
        self.assertEqual(notices[0]["title"], "crate 1.2")
        self.assertTrue(notices[0]["text"].startswith("MIT & Apache\n\n"))
        self.assertIn("https://example.test/source crate 1.2", notices[0]["text"])
        self.assertIn("Copyright <author>\n\nPermission & warranty.", notices[0]["text"])
        self.assertEqual(notices[1]["text"], "Second notice")

    def test_rejects_missing_or_incomplete_notices(self):
        for html in ("<html></html>", "<section><h2>MIT</h2>",
                     "<section><h2>MIT</h2></section>"):
            with self.subTest(html=html), self.assertRaises(ValueError):
                catalog(html)


if __name__ == "__main__":
    unittest.main()
