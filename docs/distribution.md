# Distribution

Cranamp is a music player in Rust with WSZ support and its own Skin Studio. The desktop editor exposes MCP tools for agents to draw skins.

## Registered apps

Store status was verified on October 26, 2026:

| Store | App record | Identity | Status |
| --- | --- | --- | --- |
| Google Play | [Cranamp dashboard](https://play.google.com/console/u/4/developers/5537543190610439587/app/4972717649659015464/app-dashboard) | `com.cranamp.app` | 0.1.88 (10088) published with a 100% production rollout |
| Apple App Store | [Cranamp distribution](https://appstoreconnect.apple.com/apps/6818451356/distribution) | `io.cranamp.app`; app ID `6818451356`; team `YP97W9MQC8` | macOS 0.1.89 (76) and iOS 0.1.88 (6) waiting for review |
| Microsoft Store | [Cranamp overview](https://partner.microsoft.com/en-US/dashboard/products/9N0XTC9PGM8X/overview) | Store ID `9N0XTC9PGM8X` | 0.1.89.0 published; submission 4 is live |
| Arch User Repository | [Source](https://aur.archlinux.org/packages/cranamp), [binary](https://aur.archlinux.org/packages/cranamp-bin) | `cranamp`, `cranamp-bin` | Both updated to 0.1.89-1 on October 26 |

GitHub repository variables contain the registered Apple bundle ID and the exact Microsoft manifest values:

- `CRANAMP_APPLE_BUNDLE_ID=io.cranamp.app`
- `CRANAMP_MSIX_IDENTITY=DmitriiSamoilenko.Cranamp`
- `CRANAMP_MSIX_PUBLISHER=CN=B3D99093-F004-43B3-80D7-6F628B17CAA6`
- `CRANAMP_MSIX_PUBLISHER_DISPLAY_NAME=DmitriiSamoilenko`

All four store submissions are configured to publish automatically after approval. France is excluded from the initial release. Store submission and approval do not establish legal clearance. The initial signed binaries were built by [Store packages run 37015856145](https://github.com/samoylenkodmitry/cranamp/actions/runs/37015856145). Replacement Apple build 5 comes from commit `c8d1eb7` in [run 37149395670](https://github.com/samoylenkodmitry/cranamp/actions/runs/37149395670); both Apple jobs passed. The 0.1.88 packages passed all five platform jobs in [run 40230129037](https://github.com/samoylenkodmitry/cranamp/actions/runs/40230129037). Apple refused the first Mac upload because build 2 is below the previously uploaded build 5. Replacement build 6 passed all platform jobs in [run 40233042738](https://github.com/samoylenkodmitry/cranamp/actions/runs/40233042738). Both Apple uploads succeeded in [run 40235178431](https://github.com/samoylenkodmitry/cranamp/actions/runs/40235178431). The [processing check](https://github.com/samoylenkodmitry/cranamp/actions/runs/40235854871) reports both 0.1.88 (6) builds as valid. macOS 0.1.88 (6) was submitted on October 26 with four new screenshots and the updated description. Its [review submission](https://appstoreconnect.apple.com/apps/6818451356/distribution/reviewsubmissions/details/0b54f95d-bf3c-4dcf-9811-12ebd90ad874) is waiting for review. The tagged tree passed all five store package jobs in [run 40265520149](https://github.com/samoylenkodmitry/cranamp/actions/runs/40265520149).

Apple rejected Mac build 2 under guideline 2.4.5 because automated analysis could not identify the functionality requiring incoming network access. Build 5 adds the visible Start/Stop/Test connection controls described below. The reviewer received updated instructions and a successful connection screenshot. The replacement iOS build excludes the MCP server entirely. Both build 5 uploads passed Apple processing and were submitted on October 3. The October 25 check found no active rejection or reviewer request on the pending iOS submission. The build remained valid. The wait alone did not identify a policy violation. On October 26, the old iOS submission was removed so its version and build could be replaced. The version, description and review notes were saved as 0.1.88. Build 6 was selected and saved. Its encryption declaration is complete, and 14 new iPhone and iPad screenshots were uploaded. The iOS version was submitted on October 26 after Chrome reconnected. Its [review submission](https://appstoreconnect.apple.com/apps/6818451356/distribution/reviewsubmissions/details/5cd87702-3c68-4ca7-87b5-b6b21e62de62) is waiting for review and will release automatically after approval. The old submission now has Developer Rejected status because we removed it.

## Channels and artifacts

The existing **Release** workflow builds direct downloads. The new **Store packages** workflow runs for `v*` tags and manual dispatches. Its `store` Cargo feature removes updater UI and the APK installation capability, and starts without an automatic demo playlist. No channel adds the former radio preset. All channels use Catamp for fallback artwork.

| Destination | Workflow artifact | Format |
| --- | --- | --- |
| Google Play | `google-play-bundle` | AAB |
| iOS / TestFlight | `apple-ios-store` | XCArchive; signed runs also export an IPA |
| Mac App Store | `apple-macos-store` | Universal arm64/x86_64 XCArchive and PKG |
| Microsoft Store | `microsoft-store-msix` | x64 MSIX |
| Linux | `linux-repository-packages` | DEB, RPM, tarball, source archive, AUR source/binary PKGBUILDs and `.SRCINFO` |

Tag builds produce validation artifacts without distribution credentials. Manually enable **signed** to require Apple and Google Play credentials. Signed runs fail if those credentials are missing. Windows packages remain unsigned because Microsoft signs Store submissions. Without Partner Center variables, validation runs use `Cranamp.LocalValidation`, which cannot be submitted.

Packages are retained as workflow artifacts. The `apple_status_only` input reads current Apple versions, recent build processing states and review status without building. The `apple_upload_run` input uploads signed Apple packages from a successful Store packages run. It does not submit a version for review or change its listing. Tags must match Cargo.toml. Set `build_number` above the highest previously uploaded Mac build number, including earlier marketing versions. Leave `build_number` empty to use the increasing Store packages workflow run number. An explicit override must still exceed the highest uploaded Mac build number. The previous Apple release uses build `5`; the replacement uses `6`.

## 0.1.88 listing preparation

The reviewed [screenshot gallery](../store/releases/0.1.88/screenshots/index.html) contains 23 real app captures across six Apple and Windows size groups. The [manifest](../store/releases/0.1.88/screenshots.json) records their dimensions, hashes and capture sources. The [listing text](../store/releases/0.1.88/listing-en-US.json) contains each platform description and review notes. Android screenshot groups remain pending.

The `apple_listing_platform` workflow input (`ios` or `macos`) stages the matching files for fastlane 2.241.1. Supply the already uploaded `build_number`. By default it only uploads listing assets; `apple_submit_for_review` also submits the selected build and keeps automatic release after approval. The job preserves pricing, countries and privacy declarations. It does not cancel pending review submissions.

The current App Store Connect key permits binary uploads and status reads, but Apple refused listing updates for insufficient permissions in [run 40236251185](https://github.com/samoylenkodmitry/cranamp/actions/runs/40236251185) and [iOS run 40270015005](https://github.com/samoylenkodmitry/cranamp/actions/runs/40270015005). The macOS listing and review submission were completed through the browser. Microsoft submission 2 was submitted with the validated 0.1.88.0 package, five new screenshots, updated text and certification notes. Partner Center confirmed submission 2 is live on October 26. Native computer control stopped responding during Android screenshot preparation. Google Play accepted version 10088 (0.1.88) with no loss of supported devices. The full rollout and new short and full descriptions were submitted together; Publishing overview now shows no unpublished changes and the release is live at 100%. Managed publishing is off, so approval publishes the release. The existing Android gallery remains in use. Play reports optional deobfuscation and native debug-symbol warnings; this package does not use R8 obfuscation. iOS metadata, build 6 and screenshots were submitted successfully after Chrome reconnected.

[PR 122](https://github.com/samoylenkodmitry/cranamp/pull/122) merged as `5aeae7f4b788cee06b9f7b4495cdcadbad8f6b9c`. Tag `v0.1.88` points to that merge. [Release run 40265520207](https://github.com/samoylenkodmitry/cranamp/actions/runs/40265520207) passed every release job and published [0.1.88](https://github.com/samoylenkodmitry/cranamp/releases/tag/v0.1.88) on October 26. The merge also passed [CI](https://github.com/samoylenkodmitry/cranamp/actions/runs/40265455456), and [Pages](https://github.com/samoylenkodmitry/cranamp/actions/runs/40265455499) deployed the web app.

## Desktop MCP follow-up

Version 0.1.89 includes the redesigned desktop MCP panel. The [listing text](../store/releases/0.1.89/listing-en-US.json) and [screenshot gallery](../store/releases/0.1.89/screenshots/index.html) include six macOS images and seven Windows images. Mobile retains its 0.1.88 store rollout; MCP remains excluded from Android and iOS.

The actual running player appears in both desktop galleries. Apple and Microsoft reviewer notes identify the menu button at the lower-right of the top player window, then **Open Skin Studio**. iOS retains its playback screenshot; Google Play retains its actual player image. The capture launcher uses an isolated application profile and generated Cranamp demo tracks, so no user media enters the gallery.

The [signed build](https://github.com/samoylenkodmitry/cranamp/actions/runs/40283101250) passed all five platforms. Mac build 76 was uploaded and submitted with six screenshots in [submission 02c1ca97](https://appstoreconnect.apple.com/apps/6818451356/distribution/reviewsubmissions/details/02c1ca97-8a36-4d47-a877-380637be1371). Microsoft submission 4 is live; submission 4 adds the running player screenshot and precise reviewer entry instructions.

[GitHub Release 0.1.89](https://github.com/samoylenkodmitry/cranamp/releases/tag/v0.1.89) was published from green main commit `6d1f1c46eca5c6a4f33f721592d1da7885eb1b8a`. The [release workflow](https://github.com/samoylenkodmitry/cranamp/actions/runs/40285725283) and [Pages deployment](https://github.com/samoylenkodmitry/cranamp/actions/runs/40285476925) passed. The release also contains Linux repository packages and the source archive used by AUR. Both AUR recipes passed clean Arch builds; the binary installed with no missing shared libraries. Published AUR commits are `32486c7a8589d8693f44dab171cc4a4b5f4ca498` for `cranamp` and `7b81cd4ab9d0a3f41f098ef1f708a49e6a362763` for `cranamp-bin`.

## Android 0.1.90 follow-up

[PR 127](https://github.com/samoylenkodmitry/cranamp/pull/127) pins the merged Cranpose Android window and cursor fixes at `e7b999ebdcc3cca05b1136675546f228f3fec0fc`. All application checks and five store package builds passed. The public Android window and cursor instrumentation tests passed on Galaxy Tab S7. Studio entry and gallery capture passed on the tablet and phone emulator.

The [gallery](../store/releases/0.1.90/screenshots/index.html) contains six phone images and four images for each Android tablet and desktop group. Each group starts with the actual player. The [manifest](../store/releases/0.1.90/screenshots.json) records dimensions and hashes. The [listing](../store/releases/0.1.90/listing-en-US.json) gives the exact player-to-Studio path. Google Play gallery changes are saved pending the signed 0.1.90 bundle. Tag `v0.1.90` points to green main commit `44221505d6c3abca87b2add74055a5ae4037bbbb`; [Pages deployment](https://github.com/samoylenkodmitry/cranamp/actions/runs/40310070526) passed. The 0.1.90 candidate is installed on Galaxy Tab S7, where all four window and cursor tests passed. Apple reviews remain in place; Microsoft submission 4 is live.

## Google Play

Set repository secrets:

- `CRANAMP_PLAY_UPLOAD_KEYSTORE_BASE64`
- `CRANAMP_PLAY_UPLOAD_KEYSTORE_PASSWORD`
- `CRANAMP_PLAY_UPLOAD_KEY_ALIAS`
- `CRANAMP_PLAY_UPLOAD_KEY_PASSWORD`

Use the upload key registered with Play App Signing. The application ID is `com.cranamp.app`. Establish signing continuity before distributing direct APKs and Play packages under this ID: their app-signing certificates must match for cross-channel updates.

Until a separate Play upload key is configured, CI uses the existing `CRANAMP_RELEASE_KEYSTORE_*` / `CRANAMP_RELEASE_KEY_*` secrets. Configure all four Play secrets together when rotating the upload key. Private key material stays inside the runner.

```sh
cd platform/android
./gradlew --no-daemon -PcranampStore=true :app:bundleRelease
```

The store build removes the installer permission and update receiver, and CI checks the final manifest after Cranpose adds capability declarations. Unsigned AABs are for validation only. Before production submission, complete publisher verification, privacy-policy URL, Data Safety, content rating and applicable testing requirements.

## Apple

Set repository secrets:

- `CRANAMP_APPLE_STORE_P12_BASE64`: distribution certificates and private keys, including the Mac Installer Distribution identity.
- `CRANAMP_APPLE_STORE_P12_PASSWORD`
- `CRANAMP_IOS_STORE_PROFILE_BASE64`: iOS App Store distribution profile.
- `CRANAMP_MAC_STORE_PROFILE_BASE64`: Mac App Store distribution profile.

Set repository variables:

- `CRANAMP_APPLE_BUNDLE_ID`: defaults to `io.cranamp.app`; must match the profiles and App Store Connect.
- `CRANAMP_APPLE_STORE_SIGN_IDENTITY`: Apple Distribution identity name or SHA-1 usable for both platforms.
- `CRANAMP_MAC_INSTALLER_IDENTITY`: Mac Installer Distribution identity name.

Credentials are imported into a temporary keychain and removed at job completion. Developer ID certificates for direct Mac downloads do not replace Mac App Store credentials.

Direct Mac releases require the `CRANAMP_MACOS_DEVID_P12_BASE64` and `CRANAMP_MACOS_DEVID_P12_PASSWORD` secrets plus the three `CRANAMP_ASC_API_*` notarization secrets. The Release workflow checks the local certificate chain and signs a probe before compiling. Signing, notarization, ticket stapling and Gatekeeper assessment must all succeed; it does not fall back to an ad-hoc signature.

Direct Mac builds and signing run on a clean GitHub-hosted macOS runner. Run **Release** manually to verify this signing path. That run downloads the current Mac release, signs and notarizes a temporary copy, and saves Apple's receipt as a CI artifact. It does not publish or replace release assets. Tag pushes still build and publish the full release. The signing job uses a temporary keychain and removes its own credentials and search-list entry when it finishes.

The store signing secrets and both Cranamp provisioning profiles were configured
on October 2, 2026. The encrypted PKCS12 backup contains only the Apple Distribution
and Mac Installer Distribution identities. Signing jobs receive private material through
GitHub encrypted secrets. The local iOS 0.1.86 build 1 passed Apple's validation
and uploaded successfully. Device testing and App Review are separate steps.

The Mac package uses App Sandbox with user-selected file read/write, app-scoped bookmarks, network client access for streams, and network server access for the desktop MCP endpoint. Test saved-file reopening, folder sync, playback and MCP in the signed sandbox. Building a package does not validate those runtime behaviors.

In desktop Skin Studio, open **Agent connection → Start server**. The server is off by default. **Test connection** reads the server identity and available tools without changing the document. The panel also copies connection details and stops the server. It listens only on `127.0.0.1:18765` and closes with Studio. Local programs can access the active document and files within the app's permissions while it runs. iOS, Android, and web keep Skin Studio's editing features but do not compile the MCP network server or its connection controls.

Studio launched from the desktop player closes when the player exits; a Studio launched separately from the command line remains independent. Selected music stays in its original location. Android keeps document URIs; Apple platforms keep file bookmarks. Provider streams use a bounded memory buffer. Cranamp does not create audio import copies or apply the former 2 GiB copy limit. Skin/project archives are limited to 64 MiB on disk, 128 MiB expanded, 16 MiB per entry, and 64 MiB of decoded images.

iOS uses the shared audio decoder for local music and direct audio streams, including EQ, balance, and sample analysis. HLS uses AVPlayer and reports EQ, balance, and analysis as unavailable. The app declares the audio background mode and keeps iOS media-session controls. Simulator and automated checks do not verify physical-device interruptions, Bluetooth routing, background behavior, or thermal performance.

An unsigned iOS XCArchive cannot be installed on a device or uploaded as a finished submission. Signed runs export an IPA through `xcodebuild -exportArchive`; Mac uses `productbuild`. Upload with Transporter or App Store Connect tooling. Complete privacy labels and review the final binary for required-reason APIs, SDK privacy manifests and export compliance. Do not invent privacy declarations just to pass upload checks.

## Microsoft Store

Reserve the app in Partner Center. Set these repository variables to the exact registered values:

- `CRANAMP_MSIX_IDENTITY`: Package/Identity/Name.
- `CRANAMP_MSIX_PUBLISHER`: full Package/Identity/Publisher, including `CN=`.
- `CRANAMP_MSIX_PUBLISHER_DISPLAY_NAME`: publisher display name.

The desktop package declares `runFullTrust` and uses Cargo's version plus `.0`. Upload the MSIX in Partner Center and complete the listing, ratings and privacy disclosures. Microsoft applies Store signing. Local sideload testing requires a trusted development signature.

Windows x64 builds bundle DXC because the legacy FXC compiler failed to compile the renderer's shape shader. The generated package notices include DXC's native LLVM/Microsoft and Mach licenses in addition to Cargo dependency notices. CI starts the desktop MCP server through the visible Studio controls, captures the actual GPU output, and rejects blank or undersized screenshots.

## Linux and AUR

`yay` installs recipes from the AUR. The AUR stores PKGBUILDs, not application binaries. The `cranamp` recipe builds the release source with Cargo's locked dependencies. The [cranamp-bin package](https://aur.archlinux.org/packages/cranamp-bin) installs the tested x86_64 store tarball and is available with `yay -S cranamp-bin`. CI generates both recipes and their `.SRCINFO` files using `makepkg`.

The existing AUR maintainer account is `faceless33`. Use its registered maintainer SSH key. The source recipe disables debug symbols to keep Rust release builds within available memory and disables Arch's C/C++ LTO so the native TLS library links with Rust. Rust release LTO remains enabled. The source recipe removes inherited C optimization flags so AWS-LC can compile its entropy code with the required `-O0`. The remaining C flags are preserved. Both 0.1.88 recipes built in a fresh Arch container and passed `namcap` without errors. The warnings name GPU and window libraries that the app loads at runtime. The binary package also installed successfully and had no missing linked libraries. Runtime dependencies include the libraries loaded dynamically by winit and wgpu, which ELF dependency scanning does not detect.

For a tagged release:

1. Attach `cranamp-VERSION-source.tar.gz` from the workflow artifact to the matching GitHub release. PKGBUILD already contains its checksum.
2. Test the recipe in a clean Arch build environment (`extra-x86_64-build`) and run `namcap`.
3. Use the `faceless33` maintainer account and its registered SSH key.
4. Push only PKGBUILD and `.SRCINFO` to the matching `cranamp.git` or `cranamp-bin.git` repository at `ssh://aur@aur.archlinux.org/`.
5. Users install with `yay -S cranamp-bin`, or `yay -S cranamp` for a source build once that recipe is published.

Publish the exact source archive referenced by the recipe’s checksum before pushing to the AUR. The source must match the tagged release tree; a verified artifact from the same tree can be reused. Source archives use the checked-out Git commit, so local uncommitted edits are not included.

DEB and RPM files can be attached to a GitHub release for direct installation. They do not create apt or dnf repositories. The x86_64 binaries are built on Ubuntu 22.04; verify installation and runtime dependencies on each advertised distribution. Official Debian/Fedora repositories require their own maintainers and packaging review. Flathub is a separate packaging target.

## Before submission

- Review generated THIRD-PARTY.html, the font notices, and MPL source availability. Cargo notices cover Rust dependencies; separately audit additional platform libraries.
- Keep provenance and permission records for bundled artwork. Packaging does not establish legal clearance.
- Native editions, including iOS, include `symphonia-codec-aac` through `cranpose-media`. Codec patent coverage for the intended countries remains a separate legal assessment; free distribution and open-source copyright licenses do not establish that coverage. [Via LA’s AAC program](https://www.via-la.com/licensing-programs/aac/) describes licensing for end-user encoder/decoder products, but does not determine which claims apply to this specific AAC implementation. The store privacy forms are complete for the audited build.
- Test the exact signed package on each OS, including file access and sandbox behavior.
- Supply product screenshots, support and privacy URLs, age ratings and publisher identity in the consoles.

References: [Android bundles](https://developer.android.com/build/building-cmdline), [Apple distribution](https://developer.apple.com/documentation/xcode/creating-distribution-signed-code-for-the-mac/), [Microsoft MSIX submissions](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/create-app-submission), [AUR](https://wiki.archlinux.org/title/Arch_User_Repository).

## Listing assets and privacy

`store/listing-en-US.json` contains shared listing text and platform-specific additions. Mobile and desktop listings include Skin Studio. The desktop listing also explains agent connections. The landing page, privacy policy and support page are served at [cranamp.dmitrysamoylenko.in](https://cranamp.dmitrysamoylenko.in/) directly from `site/` in the VPS checkout; see [website deployment](website.md). Settings and store metadata use this domain. `site/privacy/` and `site/support/` are also copied into the Pages web-player build.

The Apple privacy manifest declares file metadata access inside the app container (`C617.1`) and for user-selected files (`3B52.1`), plus elapsed-time measurement for playback and UI timers (`35F9.1`). The iOS release binary imports `stat`, `fstat`, `fstatat`, and `mach_absolute_time`. Preferences use Cranpose's file-backed store rather than UserDefaults. These declarations follow [Apple's required-reason API documentation](https://developer.apple.com/documentation/bundleresources/describing-use-of-required-reason-api). Re-audit the binary when dependencies change.
