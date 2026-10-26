# Cranamp 0.1.92 release validation

The user resumed publication after the store-review audit. Release preparation
is authorized. At the time of this record, the four store submissions remain
drafts. No 0.1.92 public release is claimed by this document.

## Source and artifacts

The app uses Cranpose revision `03efd19e916c62caa0b2ad3ee8ae233251e02175`,
retained by tag `cranamp-v0.1.92-2`. It adds persistent Android read access for
individually selected documents to the previously tested release revision.
Framework integration is reviewed in Cranpose PR 1899. The earlier streaming
and Apple TLS changes have merged through PRs 1885 and 1897.

The previous signed candidate, source `f9b050280caff3bfd7195baa541b653e900d3451`,
passed Store packages run 40382240309. Apple build 101 processed as VALID for
iOS and macOS and no longer displayed Missing Compliance. The file-grant fix
requires replacement Android artifacts. The final build will use Apple number
102 and Android version code 10092. Upload and submission must be verified
against those artifacts, not inferred from a successful build.

## Android file access and playback

- A real Downloads selection of the generated `Cranamp playback check.wav`
  retained a read grant on the API 37 emulator. The public ContentResolver API
  could read the original file after force-stop and a new instrumentation process.
- `FileAccessPersistenceTest#pickingAudioRetainsReadAccess` passed in 9.146 seconds.
  `selectedFileIsReadableAfterProcessRestart` passed in a separate process.
  The picker helper invokes the accessible clickable parent; an earlier touch
  version raced the system picker transition and was replaced.
- The patched store-feature debug build was installed beside the production app
  on Galaxy Tab S7 (SM-T970). ProviderFolderPlaybackTest passed against the
  already authorized RoundSync folder. Audio-cache size remained 0 KiB before
  and after the test. The runner restored the original app preferences.
- Galaxy Tab WindowLayoutTest passed both tests in 2.26 seconds.
- The previous exact AAB-derived candidate passed background playback, notification
  pause/resume and Stop checks. Stop removed its foreground media service.
  Final AAB evidence must be recorded again after the file-grant change.

The app opens original audio through bounded reads. It does not copy a selected
folder or transcode whole tracks. A storage provider may maintain its own cache.
The provider must remain reachable; seek behavior depends on its capabilities.

## Other validation

The earlier candidate passed 471 app tests, nine packaging tests and the release
workflow test. Its Apple dependency graphs use system TLS. The framework HTTPS
test accepted a trusted server and rejected a self-signed certificate.

Both source and binary AUR recipes built on the native Arch host with two Cargo
jobs. The binary had no unresolved shared-library dependencies. Final recipe
hashes must come from the published final assets.

Android and Windows galleries now contain actual rendered 0.1.92 captures.
The first image in every store gallery shows the player. Screenshot provenance
and SHA-256 values remain in screenshots.json, including older captures that
have not been replaced. Desktop galleries include the MCP connection controls.
Mobile listings and binaries exclude MCP. Store builds exclude self-update UI
and installer permission. The final Android package must pass these checks again.

Local logs and recordings are under `/tmp/cranamp-release-0.1.92/`. Device
preferences and provider URIs remain private and must not be uploaded.
