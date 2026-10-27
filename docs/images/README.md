# Screenshot sources

The main README uses actual player and Studio captures from
[`store/releases/0.1.92/screenshots.json`](../../store/releases/0.1.92/screenshots.json).
That manifest records dimensions, hashes and capture provenance. The files below
are older device illustrations, retained as source material.

- `android.png`: v0.1.61 APK running in the CranampTest35 Android emulator, captured with `adb exec-out screencap -p`.
- `ios.png`: v0.1.61 simulator app running on Codex iPhone, captured with `simctl io booted screenshot`.
- `desktop.png`: screenshot added by the repository owner in commit `6e62c71`, preserved from https://github.com/user-attachments/assets/ffb4ea04-ed0a-4300-ae2e-6c030ad2e6de (564×996 pixels).
- `devices.html`: decorative device frames around the unaltered captures; open through a static server and capture the `.board` rectangle at 1360×800 to regenerate `devices.png`. CSS uses proportional sizes and pixelated image sampling.

Before publishing a capture, check its visible text and metadata for local paths,
private filenames, account details and device identifiers. Use generated demo
audio and a project with a neutral name. Keep the source version and image hash
in the release manifest.
