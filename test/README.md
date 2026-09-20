# Tests

All test code and fixtures live here.

- `cargo test --all-targets` runs Rust unit and integration suites. Integration targets are registered in `Cargo.toml`; private unit suites in `unit/` use external `#[path]` modules.
- `node --test test/web_host.mjs` checks the web host.
- `python3 test/state_holder_gate.py` checks remembered state holders.
- `test/macos/` contains the native docking and playlist-resize checks.
- `test/ios/run-uitests.sh <device-udid>` runs the Xcode UI suite in `ios/`. Results are written under `target/`.

Keep implementation files free of test bodies and test-only helpers. The `test_location` integration target checks the boundary.
