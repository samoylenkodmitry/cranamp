# Cranamp

Cranamp is a Cranpose-powered Winamp-style audio player written in Rust. The first cut extracts the Cranpose Winamp skin renderer into a standalone app and adds real playlist state, native file/folder selection, and Rodio playback on desktop targets.

Web widget: https://samoylenkodmitry.github.io/cranamp/

## Platform Shape

- Desktop: standalone borderless Winamp windows using Cranpose native peer windows.
- Android: Cranpose surface entry point packaged as a resizable activity. The
  app uses one stacked Winamp surface, Android document pickers for
  file/folder/playlist import and export, and Rodio playback through copied
  app-private media files. The release APK does not request
  `SYSTEM_ALERT_WINDOW`; freeform Activity mode is optional desktop/tablet
  windowing, not the true always-on-top overlay path.
- iOS: single fullscreen Winamp surface on Cranpose's winit-based UIKit backend
  (`CAMetalLayer`, `CADisplayLink`, touch input), inset by the system safe
  area; no Xcode project, the pure-Rust binary owns `UIApplicationMain`.
- WebAssembly: embeddable canvas widget built with `wasm-pack`; GitHub Pages
  deploys the widget from `dist/`. Chromium browsers can open an experimental
  Document Picture-in-Picture window containing the live Cranamp canvas.

## Current Controls

- Eject opens audio files.
- The top-left options control opens an audio folder on desktop.
- Previous, play, pause, stop, next, repeat, playlist, equalizer, volume, balance, and position controls update Cranamp state and the audio backend where supported.
- If no user playlist is loaded at startup, Cranamp loads the demo MP3 playlist from the distributable `demo-music/` folder when it is present.

## Build

```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo run --release
```

## Web

```bash
cargo install wasm-pack
./build-web.sh
```

Open `dist/index.html` through a local static server or use the published GitHub Pages build:
https://samoylenkodmitry.github.io/cranamp/

## Android Freeform

Freeform Activity mode is a fallback and debug-friendly desktop/tablet UX. It
does not provide a chat-head style always-on-top mini-player. The true Android
floating-player path would need a `TYPE_APPLICATION_OVERLAY` service surface,
but Cranamp intentionally omits that permission from release builds until the
surface implementation exists. Cranpose support for rendering into that kind of
service-owned Android surface is tracked upstream in
`samoylenkodmitry/Cranpose#232`.

Use a freeform-capable emulator profile, such as the SDK's `13.5in Freeform`
device, then enable Android's developer freeform flags before launching:

```bash
adb shell settings put global development_settings_enabled 1
adb shell settings put global enable_freeform_support 1
adb shell settings put global force_resizable_activities 1
adb shell am start --windowingMode 5 --activity-task-on-home -n com.cranamp.app/dev.cranpose.android.CranposeActivity
```

Phone-shaped Pixel AVDs can still force fullscreen even when the app manifest is
resizeable.

See `docs/FLOATING_SURFACES.md` for the Android overlay, Android freeform, and
browser Document Picture-in-Picture split.

## Releases

Tags matching `v*` publish GitHub Release assets for Linux, macOS, Windows, Android, iOS, and the WebAssembly widget bundle. Desktop and web archives include demo MP3 files as separate assets rather than embedding them in the executable or WASM binary. The Android APK is debug-signed for sideload testing.

The macOS `.app` is signed with a Developer ID certificate and notarized, so a downloaded build opens on a double click. That needs five repository secrets, which `scripts/export_macos_signing_secrets.sh` sets in one go. Without them the release job falls back to an ad-hoc signature, and then Gatekeeper blocks the first launch after a download (double-clicking appears to do nothing, which is the block, not a crash): approve it once with right-click `Cranamp.app` → **Open** → **Open**, or `xattr -dr com.apple.quarantine Cranamp.app`. The iOS `.ipa` is a device build for sideloading; the separate `…-ios-simulator.app.zip` installs on a Simulator via `xcrun simctl install booted Cranamp.app`.

That `.ipa` carries no signature — CI holds no development identity and does
not know which iPhones are yours — so putting it on a device is a local step:

```
./platform/ios/regen-profile.sh                       # once per device, and when the profile expires
./platform/ios/install-ipa.sh cranamp-<version>-ios.ipa
```

`regen-profile.sh` builds a stub app through Xcode's automatic signing, which
registers the attached iPhone with your team and writes a provisioning profile;
`install-ipa.sh` embeds that profile in the `.app`, re-signs it with your
`Apple Development` identity and installs it over `devicectl`. Both need an
Apple ID with the team added in Xcode → Settings → Accounts.

## Unsafe Policy

Application code denies `unsafe` with crate-level and Cargo lints and contains no unsafe blocks, including on Android: the exported `android_main` entry symbol is written by the Cranpose framework's `android_main!` macro, not by application code, so nothing here carries an `unsafe_code` exception. Third-party dependencies may use unsafe internally where their platform integrations require it.
