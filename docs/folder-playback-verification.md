# Folder playback verification

Checked on 2026-10-29 against the original network music library. Cranamp keeps
file references and opens the selected track through Cranpose media services.
Folder enumeration uses Cranpose's content stream and blocking executor. No
audio import or audio cache is created by Cranamp. Remote providers control their
own downloads and caches.

The first discovered file contains unsupported AC-3 audio. Folder autoplay now
skips a failed candidate and starts the next discovered track, even when the
next batch or the rest of the tree is still pending. Adding a folder to a stopped
playlist selects the first new track. Later batches preserve playback and Stop.
New picker requests invalidate an older scan, and scan errors release the picker.
Adding, restoring, or importing a playlist no longer triggers background duration
reads for the whole playlist. Durations come from saved metadata or playback.

| Platform | Result |
| --- | --- |
| macOS | Selected the original mounted folder in the native GUI. The first valid E-Mantra track played with an active spectrum about 1.6 seconds after Open. The final scan took about 77 seconds and found 2,092 audio entries among 2,754 files. |
| Linux | Ran the public folder stream and real media backend against the original directory on `samarch`. First candidate at 11 ms, next candidate at 3.97 seconds after the unsupported codec. Decoded sample peak 0.0599, state Playing, clock 1.90 seconds. Enumeration remained incomplete. |
| Android | Galaxy Tab S7 provider test reached Playing and an advancing clock through the existing remote folder grant. Cranamp audio cache growth was zero. A separate provider regression verified recovery from a transient null cursor and errors for persistent null/loading responses. |
| Web | Chrome selected the original mounted directory through `showDirectoryPicker`. Playback advanced while the library populated; 2,077 audio entries among 2,732 files. Tracks use Blob URLs. Browser fallback directory inputs depend on the browser completing its own enumeration before it returns a FileList. |
| iOS | Installed and launched the updated build on the paired iPhone. The picker uses `UTTypeFolder` with `asCopy: false`. The system disables the installed Tailscale and ES File Explorer providers for folder selection. The same remote folder test is blocked at that provider boundary. |
| Windows | The configured Windows host refused SSH on port 22. Live folder playback remains unverified. |

Native and web extension filters differ, so their discovered counts differ.
Build checks do not replace the device results above.

## Regression checks

- `cargo test --all-targets --features audio-audit`: 423 library tests and 54 integration tests passed.
- `cargo clippy --all-targets --features audio-audit -- -D warnings` passed.
- Cranpose web folder tests verify a first entry before a pending nested directory,
  empty directory traversal, and cancellation/errors without reading file bytes.
- `test/desktop_playback_probe.rs` accepts `--folder PATH` and checks decoded
  samples through the real media backend before completing the folder stream.
- `test/android/run_provider_folder.py` runs playback through an existing folder
  grant, measures Cranamp's audio cache, and restores the saved preferences.
- `test/android/FolderProviderFailureTest.java` exercises provider failures
  without creating or reading audio files.
- `test/ios/CranampUITests/PickerProviders.swift` drives Add Folder, retains a
  provider screenshot, and requires at least two seconds of playback progress.
