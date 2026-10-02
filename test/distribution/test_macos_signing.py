import base64
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[2] / "scripts/release/macos-signing.sh"


class MacSigningTests(unittest.TestCase):
    def run_sign(self, *, signing_exit=0, notarization="Accepted", api_key=True):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            commands = root / "commands"
            commands.mkdir()
            log = root / "calls"
            state = root / "state"
            state.mkdir()
            for name in ["security", "codesign", "ditto", "xcrun", "spctl"]:
                body = '#!/bin/sh\nprintf "%s\\n" "' + name + ' $*" >> "$TEST_CALLS"\n'
                if name == "codesign":
                    body += f"exit {signing_exit}\n"
                elif name == "xcrun":
                    body += 'if [ "$1" = notarytool ]; then\n'
                    body += f"  printf '%s\\n' '{{\"id\":\"test\",\"status\":\"{notarization}\"}}'\nfi\n"
                command = commands / name
                command.write_text(body)
                command.chmod(0o755)
            env = os.environ | {
                "PATH": str(commands) + os.pathsep + os.environ["PATH"],
                "TEST_CALLS": str(log),
                "CRANAMP_SIGN_STATE": str(state),
                "CRANAMP_SIGN_KEYCHAIN": str(state / "signing.keychain-db"),
                "CRANAMP_SIGN_KEYCHAIN_PASS": "test-password",
                "CRANAMP_SIGN_IDENTITY": "test-identity",
                "ASC_P8_BASE64": base64.b64encode(b"test key").decode() if api_key else "",
                "ASC_KEY_ID": "test-key-id",
                "ASC_ISSUER_ID": "test-issuer",
            }
            result = subprocess.run(["bash", str(SCRIPT), "sign", str(root / "Cranamp.app")],
                                    env=env, capture_output=True, text=True)
            self.assertFalse((state / "asc.p8").exists(), "Notarization key must be removed")
            return result, log.read_text() if log.exists() else ""

    def test_signing_failure_does_not_notarize_or_fall_back(self):
        result, calls = self.run_sign(signing_exit=1)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(calls.count("codesign "), 1)
        self.assertNotIn("xcrun ", calls)
        self.assertNotIn("--sign -", calls)

    def test_rejected_notarization_does_not_staple(self):
        result, calls = self.run_sign(notarization="Invalid")
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("stapler", calls)
        self.assertNotIn("spctl", calls)

    def test_success_requires_stapling_and_gatekeeper(self):
        result, calls = self.run_sign()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("xcrun stapler staple", calls)
        self.assertIn("xcrun stapler validate", calls)
        self.assertIn("spctl --assess", calls)

    def test_missing_notarization_key_fails_before_signing(self):
        result, calls = self.run_sign(api_key=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(calls, "")


if __name__ == "__main__":
    unittest.main()
