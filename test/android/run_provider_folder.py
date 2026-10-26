"""Exercise an already granted provider folder and record audio cache growth."""
import argparse
import json
from pathlib import Path
import shlex
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--adb", default="adb")
    parser.add_argument("--device", required=True)
    parser.add_argument("--uri-file", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    prefix = [args.adb, "-s", args.device, "shell"]
    package = "com.cranamp.app.debug"

    def shell(*parts, input_text=None):
        try:
            return subprocess.run(prefix + [shlex.join(parts)], input=input_text,
                                  capture_output=True, text=True, timeout=75)
        except subprocess.TimeoutExpired as error:
            output = error.stdout or b""
            if isinstance(output, bytes):
                output = output.decode(errors="replace")
            return subprocess.CompletedProcess([], 124, output, "Device command timed out.\n")

    def audio_cache_kib():
        result = shell("run-as", package, "du", "-sk",
                       f"files/{package}/imported-audio",
                       f"cache/{package}/cranpose-media-spool")
        return sum(int(line.split()[0]) for line in result.stdout.splitlines()
                   if line.split() and line.split()[0].isdigit())

    shell("am", "force-stop", package).check_returncode()
    preferences = f"files/config/{package}/preferences"
    original = shell("run-as", package, "cat", preferences)
    original.check_returncode()
    backup = args.output / "preferences.before.txt"
    backup.touch(mode=0o600, exist_ok=False)
    backup.write_text(original.stdout)
    empty_playlist = "".join(line for line in original.stdout.splitlines(keepends=True)
                             if not line.startswith("cranamp.player="))
    before = audio_cache_kib()
    try:
        shell("run-as", package, "tee", preferences,
              input_text=empty_playlist).check_returncode()
        result = shell("am", "instrument", "-w", "-r", "-e", "class",
                       "com.cranamp.app.ProviderFolderPlaybackTest", "-e", "folderUri",
                       args.uri_file.read_text().strip(),
                       package + ".test/androidx.test.runner.AndroidJUnitRunner")
    finally:
        shell("am", "force-stop", package).check_returncode()
        shell("run-as", package, "tee", preferences,
              input_text=original.stdout).check_returncode()
        restored = shell("run-as", package, "cat", preferences)
        restored.check_returncode()
        assert restored.stdout == original.stdout, "Device preferences were not restored"
    log = result.stdout + result.stderr
    (args.output / "instrumentation.txt").write_text(log)
    after = audio_cache_kib()
    receipt = {"audio_cache_kib_before": before, "audio_cache_kib_after": after,
               "audio_cache_growth_kib": after - before,
               "playback_test_passed": "OK (1 test)" in log}
    (args.output / "storage.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt))
    if result.returncode or not receipt["playback_test_passed"] or after != before:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
