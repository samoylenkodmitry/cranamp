# Cranamp 0.1.92 release validation

The user resumed publication after the store-review audit. This record describes
the final 0.1.92 packages and the checks completed before submission.
Store approval and public release are separate states; see the release receipts
below for each destination.

## Source and artifacts

The app uses Cranpose revision `03efd19e916c62caa0b2ad3ee8ae233251e02175`,
retained by tag `cranamp-v0.1.92-2`. It adds persistent Android read access for
individually selected documents to the previously tested release revision.
Framework integration is reviewed in Cranpose PR 1899. The earlier streaming
and Apple TLS changes have merged through PRs 1885 and 1897.

Cranamp PR 130 merged as `4124f121aa1a0d8117a972497cecd6a776ef5228`, tagged
`v0.1.92`. Its tree matches the checked PR head
`0aee672963c58b6cbcdbe4ded660c4136c14a5a1`. All eight app and store CI checks
passed. Cranpose PR 1899 merged as
`a0e684e598e2dab5e90fefd6ff3cda091359990e` after its required checks passed.

Signed packages came from [Store packages run 40392578935](https://github.com/samoylenkodmitry/cranamp/actions/runs/40392578935),
source `31845153d331cd2653a627e0b3d2cdb3d101ddfc`. The only later changes were
the Android cancellation test and this validation document; product code and
assets are identical. Apple build 102 processed as VALID for both platforms,
with no Missing Compliance state. Android uses version code 10092.

Linux repository packages came from the final PR head through
[run 40393950117](https://github.com/samoylenkodmitry/cranamp/actions/runs/40393950117).
Their source and binary archive hashes match the tested AUR recipes.

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
  already authorized RoundSync folder. Playback began in 4.329 seconds.
  Audio-cache size remained 0 KiB before
  and after the test. The runner restored the original app preferences.
- Galaxy Tab WindowLayoutTest passed both tests in 2.26 seconds.
- Galaxy Tab StudioCancelReviewTest passed in 103.324 seconds. It cancelled
  the real provider picker and rendered all 130 Studio control cases. The test
  now handles Back navigating through provider folders before cancellation.
- The final AAB-derived package passed background playback, notification
  pause/resume and Stop checks on the API 37 / 16 KB emulator. Stop removed
  its foreground media service and media session. The crash buffer was empty.
- The final production package retained a persistable read grant after selecting
  the sample through the system picker. Playback worked after force-stop and
  restart. The 75-second reviewer recording includes actual device audio.
  Its audio gain was increased by 24 dB; frames and timing were unchanged.
  The [public recording](https://cranamp.dmitrysamoylenko.in/review/0.1.92/)
  loads without authentication.
- The normal store-feature debug APK was restored on the Galaxy Tab after
  the review-test APK. The test runner restored the user preferences.

The app opens original audio through bounded reads. It does not copy a selected
folder or transcode whole tracks. A storage provider may maintain its own cache.
The provider must remain reachable; seek behavior depends on its capabilities.

## Other validation

The earlier candidate passed 471 app tests, nine packaging tests and the release
workflow test. Its Apple dependency graphs use system TLS. The framework HTTPS
test accepted a trusted server and rejected a self-signed certificate.

Both final source and binary AUR recipes built on the native Arch host with two
Cargo jobs under the shared host-capacity lock. The source build used an isolated
build directory. The binary had no unresolved shared-library dependencies.
The source recipe declares aarch64 support, but this run tested x86_64 only.

Android and Windows galleries now contain actual rendered 0.1.92 captures.
The first image in every store gallery shows the player. Screenshot provenance
and SHA-256 values remain in screenshots.json, including older captures that
have not been replaced. Desktop galleries include the MCP connection controls.
Mobile listings and binaries exclude MCP. Store builds exclude self-update UI
and installer permission. The final AAB-derived APK has no updater class or
installer permission and is not debuggable.

Local logs and recordings are under `/tmp/cranamp-release-0.1.92/`. Device
preferences and provider URIs remain private and must not be uploaded.

## Privacy, encryption and review evidence

The final privacy review followed the actual data path. There is no analytics SDK
or developer account service. Optional sync writes to the selected local or
system-provider folder; it does not integrate a cloud SDK. External folder
providers and desktop agents are separate applications. Their access is described
before enabling sync or the agent connection. Direct stream hosts receive the
user-requested URL and IP address. HTTP remains possible, so no universal
transit-encryption claim is made. Apple and Google retain their no-data-collected
declarations on this basis. A future built-in cloud transport would require a
new assessment. Microsoft retains its personal-information declaration.

The live support and privacy pages were deployed from the merged source. The
review sample is served as audio/wav. The video page permits same-origin media
and was checked in Chrome: readyState 4, duration 75.070833 seconds, no media error.

Apple uses system TLS on iOS and macOS; final build 102 has no missing encryption
compliance prompt. The iOS history showed a long wait followed by our withdrawal,
not a reviewer guideline rejection. Its earlier delay remains unexplained.
No reviewer correspondence or support inquiry was sent.

## Release receipts

- iOS 0.1.92 (102): Waiting for Review, submitted October 26, 2026. Submission
  `0b14c350-d1d3-4398-8f8f-13f00543b2bc`.
- macOS 0.1.92 (102): Waiting for Review, submitted October 26, 2026. Submission
  `98114419-9c0f-4bcd-ace6-b6c89d0c4d00`.
- Microsoft: 0.1.92.0 package validated; submission 5
  (`1152921505702235504`) is In certification. Publication follows approval.
- Google Play: final package 10092, all galleries and the foreground-service
  declaration are saved. No declarations need attention, and automatic package
  checks passed. The final Send changes action closes its confirmation dialog
  without creating a submission receipt. A fresh tab, native controls and a
  package-only attempt had the same result. No error was displayed. The six
  changes remain ready for submission; the user was asked to try the final
  action in the retained console tab. Android is not claimed as submitted.
- [GitHub 0.1.92](https://github.com/samoylenkodmitry/cranamp/releases/tag/v0.1.92):
  published October 26 at 21:02:24 UTC. All release jobs passed, including
  Mac signing/notarization and iOS attachment. Fourteen assets are public.
- [AUR cranamp](https://aur.archlinux.org/packages/cranamp) and
  [cranamp-bin](https://aur.archlinux.org/packages/cranamp-bin): both public
  package pages show 0.1.92-1. Source commit `34c657c`, binary commit `9ad8bcc`.
  The public source and binary archives return HTTP 200 and their SHA-256
  values match the tested recipes. AUR's RPC cache briefly retained 0.1.89.
- [Pages](https://samoylenkodmitry.github.io/cranamp/): run 40397750618 deployed
  the release source successfully. Chrome rendered the player; no console
  errors were recorded. Main CI run 40397750601 also passed.

Apple release settings publish automatically after approval. Each store gallery
starts with the player. Google has 6 phone, 4 small-tablet, 4 large-tablet and
4 desktop captures; Windows and Mac each have 6. Apple iPhone and iPad galleries
retain the supported size groups recorded in screenshots.json.

No iOS-device, IPv6-only-network or manual Windows-runtime acceptance test is
claimed for these final packages. Windows packaging and CI rendering checks
passed. Store review may still identify additional issues.
