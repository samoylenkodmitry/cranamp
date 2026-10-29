# Folder playback verification

Checked on 2026-10-29 against the original network music library. Cranamp keeps
file references and opens the selected track through Cranpose media services.
Folder enumeration uses Cranpose's content stream and blocking executor. No
audio import or audio cache is created by Cranamp. Remote providers control their
own downloads and caches.

The first discovered file contains AC-3 audio. It now decodes directly from the
original MKV. Folder autoplay also skips a failed candidate and starts the next
discovered track while further batches remain pending. Adding a folder to a stopped
playlist selects the first new track. Later batches preserve playback and Stop.
New picker requests invalidate an older scan, and scan errors release the picker.
Adding, restoring, or importing a playlist no longer triggers background duration
reads for the whole playlist. Durations come from saved metadata or playback.

| Platform | Result |
| --- | --- |
| macOS | The new WebDAV probe yielded the original AC-3 MKV in 66 ms and played it with decoded samples and an advancing clock. Streaming and seeking to 120 seconds passed. Previously selected the original mounted folder in the native GUI. The first valid E-Mantra track played with an active spectrum about 1.6 seconds after Open. The final scan took about 77 seconds and found 2,092 audio entries among 2,754 files. |
| Linux | Ran the public folder stream and real media backend against the original directory on `samarch`. First candidate at 11 ms, next candidate at 3.97 seconds after the unsupported codec. Decoded sample peak 0.0599, state Playing, clock 1.90 seconds. Enumeration remained incomplete. |
| Android | Galaxy Tab S7 provider test reached Playing and an advancing clock through the existing remote folder grant. Cranamp audio cache growth was zero. A separate provider regression verified recovery from a transient null cursor and errors for persistent null/loading responses. |
| Web | Chrome selected the original mounted directory through `showDirectoryPicker`. Playback advanced while the library populated; 2,077 audio entries among 2,732 files. Tracks use Blob URLs. Browser fallback directory inputs depend on the browser completing its own enumeration before it returns a FileList. |
| iOS | Installed the fixed build on the paired iPhone. Add Network opened the original folder through Taildrive WebDAV. The original AC-3 MKV reached two seconds of playback 2.49 seconds after Open; discovery finished afterward with 2,096 audio entries among 2,754 files. Next played the original E-Mantra MP3. Audio file sizes and modification dates in the app container were unchanged, with zero audio byte growth. |
| Windows | After the host booted, the real media probe played its existing demo folder. First candidate at 40 ms, decoded sample peak 0.4645, state Playing, clock 1.79 seconds, exit code 0. The original library remains untested on Windows: Windows and its Linux file server share one machine through dual boot, so the network source is offline during the Windows run. |

The iOS target previously enabled the shared media backend without enabling any
file readers or codecs. Its `ios` feature now enables `cranpose/media`, as the
other native app targets do. The native picker still uses `UTTypeFolder` with
`asCopy: false`; installed providers that disable directory selection can be
accessed through Add Network when they expose WebDAV.

Native and web extension filters differ, so their discovered counts differ.
Build checks do not replace the device results above.

## Regression checks

- `cargo test --all-targets --features audio-audit`: 423 library tests and 54 integration tests passed.
- `cargo clippy --all-targets --features audio-audit -- -D warnings` passed.
- Cranpose web folder tests verify a first entry before a pending nested directory,
  empty directory traversal, and cancellation/errors without reading file bytes.
- `test/desktop_playback_probe.rs` accepts `--folder PATH` or `--webdav URL` and checks decoded
  samples through the real media backend before completing the folder stream.
- `test/android/run_provider_folder.py` runs playback through an existing folder
  grant, measures Cranamp's audio cache, and restores the saved preferences.
- `test/android/FolderProviderFailureTest.java` exercises provider failures
  without creating or reading audio files.
- `test/ios/CranampUITests/PickerProviders.swift` drives Add Network, types the
  folder URL, and requires two seconds of playback progress. It also tests Next.
  Set `TEST_RUNNER_CRANAMP_TEST_WEBDAV_URL` to an accessible original folder URL.
  The separate native provider test retains the provider screenshot.
- `test/windows-folder-playback.ps1` runs the desktop probe in the signed-in
  Windows session through a temporary scheduled task. It opens an existing folder,
  checks decoded samples and process completion, and removes its task afterward.
  Use `-Schedule -Executable PATH -Folder PATH -OutputDirectory NEW_PATH` over SSH.
