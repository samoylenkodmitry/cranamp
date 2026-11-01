# Cranamp 0.1.93 release validation

This record lists the 0.1.93 packages and the checks completed before each
submission. Store approval and public release are separate states.

## Source

Cranamp builds on Cranpose 0.18.0 and its forks from crates.io only.
`test/distribution/test_dependency_sources.py` fails on any Git dependency or Git
patch.

Tag `v0.1.93` points to `fcbaf7bf19e2b09abe16f530909ec0f7b080f860`. The
[release run 41311650489](https://github.com/samoylenkodmitry/cranamp/actions/runs/41311650489)
published the GitHub release. That workflow pins `nightly-2026-10-31`, because
`nightly-2026-11-01` fails to compile zbus.

The store builds 154 come from main `ce679cf`, after the tag. They add three
merged changes:

- [PR 137](https://github.com/samoylenkodmitry/cranamp/pull/137): Add music can
  browse Taildrive folders and open the shown folder.
- [PR 138](https://github.com/samoylenkodmitry/cranamp/pull/138): a link opened
  by address plays at once, and a stream address with no file name takes the
  server name as its title.
- [PR 139](https://github.com/samoylenkodmitry/cranamp/pull/139): the browser
  uses Tailscale's own name, Taildrive, and the store notes describe it.

## Packages

| Destination | Package | Source | State |
| --- | --- | --- | --- |
| Mac App Store | 0.1.93 (153) | [run 41307669493](https://github.com/samoylenkodmitry/cranamp/actions/runs/41307669493), `a411c3a` | Submitted for review |
| App Store, iOS | 0.1.93 (154) | [run 41322865802](https://github.com/samoylenkodmitry/cranamp/actions/runs/41322865802), `ce679cf` | Submitted for review |
| Google Play | AAB, version code 10093 | run 41322865802 | Ready for upload in Play Console |
| Microsoft Store | `cranamp-0.1.93-x64.msix` | run 41322865802 | Ready for upload in Partner Center |
| AUR | `cranamp` and `cranamp-bin` 0.1.93-1 | run 41311650662, `fcbaf7b` | Published |

[Upload run 41326014180](https://github.com/samoylenkodmitry/cranamp/actions/runs/41326014180)
sent iOS and macOS build 154 to App Store Connect. Both builds processed as
VALID. The CI App Store Connect key cannot change listings, so the iOS listing
and submission ran locally through fastlane 2.241.1 with an App Manager key.

The AAB signature verifies with `jarsigner`. The macOS build 153 under review
does not contain PRs 137 to 139.

## Device checks

- Mac: the user approved the 0.1.93 app. `cranamp-playback-probe --url
  'https://naxi128ssl.streaming.rs:9152/;'` decoded the stream within 2 seconds.
- Android: the user approved the 0.1.93 app on the Pixel 9 Pro and Samsung
  devices before PRs 137 to 139, and approved shipping the later build.
- iPhone: the user approved the app after the Taildrive browser. With
  `testTaildriveBrowseFolder`, the browser opened the original music folder.
  Discovery found 2,096 audio entries among 2,754 files in 6 seconds.
  `testAudioUrl` played the stream link while another track was playing.

## AUR

The source archive and the x86_64 store tarball from run 41311650662 were
attached to the `v0.1.93` release. Their hashes match both recipes.
`cranamp-bin` built with `makepkg` on samarch. `cranamp` built in a clean
`archlinux:latest` container. Neither binary has a missing shared library. The
AUR commits are `a528683` for `cranamp` and `ce5a4ba` for `cranamp-bin`.
