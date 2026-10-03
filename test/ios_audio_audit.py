"""Run the dedicated audio audit binary on an already booted iOS simulator."""
import argparse
import json
import plistlib
from pathlib import Path
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = "io.cranamp.audioaudit"


def command(*args):
    return subprocess.check_output(args, text=True).strip()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("device")
    args = parser.parse_args()
    binary = ROOT / "target/aarch64-apple-ios-sim/debug/cranamp-audio-audit"
    output = ROOT / "store-artifacts/submission/apple-review-audit"
    app = output / "AudioAudit.app"
    app.mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, app / "AudioAudit")
    info = plistlib.loads((ROOT / "platform/ios/Info.plist").read_bytes())
    info.update(CFBundleIdentifier=BUNDLE, CFBundleExecutable="AudioAudit",
                CFBundleName="Cranamp Audio Audit", CFBundleDisplayName="Cranamp Audio Audit")
    (app / "Info.plist").write_bytes(plistlib.dumps(info))
    command("codesign", "--force", "--sign", "-", str(app))
    command("xcrun", "simctl", "install", args.device, str(app))
    container = Path(command("xcrun", "simctl", "get_app_container", args.device, BUNDLE, "data"))
    for stale in container.rglob("audio-audit.json"):
        stale.unlink()
    print(command("xcrun", "simctl", "launch", "--terminate-running-process", args.device, BUNDLE), flush=True)
    deadline = time.monotonic() + 55
    while time.monotonic() < deadline:
        for report in container.rglob("audio-audit.json"):
            try:
                result = json.loads(report.read_text())
            except json.JSONDecodeError:
                continue
            if result.get("passed") or result.get("error"):
                shutil.copy2(report, output / "ios-audio-audit.json")
                print(json.dumps({key: value for key, value in result.items() if key != "observations"}))
                if result.get("error"):
                    raise SystemExit(1)
                return
        time.sleep(1)
    raise SystemExit("Audio audit did not finish within 55 seconds")


if __name__ == "__main__":
    main()
