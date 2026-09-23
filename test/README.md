# Tests

All test code and fixtures live here.

- `cargo test --all-targets` runs Rust unit and integration suites. Integration targets are registered in `Cargo.toml`; private unit suites in `unit/` use external `#[path]` modules.
- `node --test test/web_host.mjs` checks the web host.
- `python3 test/state_holder_gate.py` checks remembered state holders.
- `test/macos/` contains the native docking and playlist-resize checks.
- `test/linux/check_xwayland_session.sh <out-dir>` starts the player with a Wayland session's environment on an X display and checks that it runs on XWayland with a panel icon and a draggable title bar.
- `test/windows-shell.sh <cranamp.exe> <out-dir> [ssh-host]` runs `test/windows-shell.ps1` on a signed-in Windows desktop over SSH: the executable must be a GUI program with the Cranamp icon and version resource, open no terminal, and give every window the icon in its title bar and taskbar.
- `python3 scripts/dev/render_app_icon.py` redraws the app icon for every platform; `--sheet <png>` puts every rendering side by side for review.
- `test/ios/run-uitests.sh <device-udid>` runs the Xcode UI suite in `ios/`. Results are written under `target/`.
- With Skin Studio running, `node test/skin-studio/midnight_snack.mjs` checks project/export GPU equality, exact spectrum/editor pixels, keyed preamp and all 28 EQ track frames, and continuous border/footer pixels at all 29 playlist tile phases. It also captures control states, slider endpoints and five playlist sizes.

Keep implementation files free of test bodies and test-only helpers. The `test_location` integration target checks the boundary.
