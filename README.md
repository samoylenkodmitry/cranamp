# Cranamp

Cranamp is a Cranpose-powered Winamp-style audio player written in Rust. The first cut extracts the Cranpose Winamp skin renderer into a standalone app and adds real playlist state, native file/folder selection, and Rodio playback on desktop targets.

## Platform Shape

- Desktop: standalone borderless Winamp windows using Cranpose native peer windows.
- Android/iOS: fullscreen Cranpose surface entry points are wired through the library crate.
- WebAssembly: embeddable canvas widget built with `wasm-pack`; GitHub Pages deploys the widget from `dist/`.

## Current Controls

- Eject opens audio files.
- The top-left options control opens an audio folder on desktop.
- Previous, play, pause, stop, next, repeat, playlist, equalizer, volume, balance, and position controls update Cranamp state and the audio backend where supported.

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

Open `dist/index.html` through a local static server or let GitHub Pages publish it.

## Unsafe Policy

Application code forbids `unsafe` with crate-level and Cargo lints. Third-party dependencies may use unsafe internally where their platform integrations require it.
