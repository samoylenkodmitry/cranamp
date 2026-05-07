# Cranamp

Cranamp is a Cranpose-powered Winamp-style audio player written in Rust. The first cut extracts the Cranpose Winamp skin renderer into a standalone app and adds real playlist state, native file/folder selection, and Rodio playback on desktop targets.

Web widget: https://samoylenkodmitry.github.io/cranamp/

## Platform Shape

- Desktop: standalone borderless Winamp windows using Cranpose native peer windows.
- Android: Cranpose surface entry point packaged as a resizable activity. The
  app uses one stacked Winamp surface, Android document pickers for
  file/folder/playlist import and export, and Rodio playback through copied
  app-private media files.
- iOS: fullscreen Cranpose surface entry point is wired through the library crate.
- WebAssembly: embeddable canvas widget built with `wasm-pack`; GitHub Pages deploys the widget from `dist/`.

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

Use a freeform-capable emulator profile, such as the SDK's `13.5in Freeform`
device, then enable Android's developer freeform flags before launching:

```bash
adb shell settings put global development_settings_enabled 1
adb shell settings put global enable_freeform_support 1
adb shell settings put global force_resizable_activities 1
adb shell am start --windowingMode 5 --activity-task-on-home -n com.cranamp.app/.CranampActivity
```

Phone-shaped Pixel AVDs can still force fullscreen even when the app manifest is
resizeable.

## Releases

Tags matching `v*` publish GitHub Release assets for Linux, macOS, Windows, Android, iOS libraries, and the WebAssembly widget bundle. Desktop and web archives include demo MP3 files as separate assets rather than embedding them in the executable or WASM binary. The Android APK is debug-signed for sideload testing; iOS release output is a static library package until signed Xcode archive/export packaging is added.

## Unsafe Policy

Application code denies `unsafe` with crate-level and Cargo lints and contains no unsafe blocks. The Android/iOS loader entry symbols narrowly allow Rust's `unsafe_code` lint around `#[no_mangle]`, because those platforms require stable exported entry-point names. Third-party dependencies may use unsafe internally where their platform integrations require it.
