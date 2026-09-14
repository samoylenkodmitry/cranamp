# Screenshot sources

- `android.png`: v0.1.61 APK running in the CranampTest35 Android emulator, captured with `adb exec-out screencap -p`.
- `ios.png`: v0.1.61 simulator app running on Codex iPhone, captured with `simctl io booted screenshot`.
- `desktop.png`: actual native Cranamp GPU player render from Skin Studio's player preview (Catamp Silverplay 22), at exact 2× pixels.
- `studio-layers.png`, `studio-tools.png`: actual desktop Studio GPU captures with the painting-layer panel and drawing-tool panel open.
- `devices.html`: decorative device frames around the unaltered captures; open through a static server and capture the `.board` rectangle at 1280×672 to regenerate `devices.png`. CSS uses proportional sizes and pixelated image sampling.
