# Distribution

Cranamp is a music player in Rust with WSZ support and its own Skin Studio. The desktop editor exposes MCP tools for agents to draw skins.

## Registered apps

Apple status was verified on October 3, 2026; Google Play and Microsoft status below was last verified on October 2:

| Store | App record | Identity | Status |
| --- | --- | --- | --- |
| Google Play | [Cranamp dashboard](https://play.google.com/console/u/4/developers/5537543190610439587/app/4972717649659015464/app-dashboard) | `com.cranamp.app` | 0.1.86 (10086) submitted; changes in review |
| Apple App Store | [Cranamp distribution](https://appstoreconnect.apple.com/apps/6818451356/distribution) | `io.cranamp.app`; app ID `6818451356`; team `YP97W9MQC8` | Replacement iOS and macOS 0.1.86 (5) submitted; waiting for review |
| Microsoft Store | [Cranamp overview](https://partner.microsoft.com/en-US/dashboard/products/9N0XTC9PGM8X/overview) | Store ID `9N0XTC9PGM8X` | 0.1.86.0 submitted; in certification |

GitHub repository variables contain the registered Apple bundle ID and the exact Microsoft manifest values:

- `CRANAMP_APPLE_BUNDLE_ID=io.cranamp.app`
- `CRANAMP_MSIX_IDENTITY=DmitriiSamoilenko.Cranamp`
- `CRANAMP_MSIX_PUBLISHER=CN=B3D99093-F004-43B3-80D7-6F628B17CAA6`
- `CRANAMP_MSIX_PUBLISHER_DISPLAY_NAME=DmitriiSamoilenko`

All four store submissions are configured to publish automatically after approval. France is excluded from the initial release. Store submission and approval do not establish legal clearance. The initial signed binaries were built by [Store packages run 37015856145](https://github.com/samoylenkodmitry/cranamp/actions/runs/37015856145). Replacement Apple build 5 comes from commit `c8d1eb7` in [run 37149395670](https://github.com/samoylenkodmitry/cranamp/actions/runs/37149395670); both Apple jobs passed. Later commits correct tests and Windows screenshot automation without changing the submitted app code.

Apple rejected Mac build 2 under guideline 2.4.5 because automated analysis could not identify the functionality requiring incoming network access. Build 5 adds the visible Start/Stop/Test connection controls described below. The reviewer received updated instructions and a successful connection screenshot. The replacement iOS build excludes the MCP server entirely. Both build 5 uploads passed Apple processing and were submitted on October 3.

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

Packages are retained as workflow artifacts. This workflow does not submit store listings or publish to repositories. Tags must match Cargo.toml. Set the manual `build_number` input to a new Apple build number for each upload of the same version. Its default is `1`; the current Apple submission uses build `5`.

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
and Mac Installer Distribution identities. Private material stays in Vault and
GitHub encrypted secrets. The local iOS 0.1.86 build 1 passed Apple's validation
and uploaded successfully. Device testing and App Review are separate steps.

The Mac package uses App Sandbox with user-selected file read/write, app-scoped bookmarks, network client access for streams, and network server access for the desktop MCP endpoint. Test saved-file reopening, folder sync, playback and MCP in the signed sandbox. Building a package does not validate those runtime behaviors.

In desktop Skin Studio, open **Agent connection → Start server**. The server is off by default. **Test connection** reads the server identity and available tools without changing the document. The panel also copies connection details and stops the server. It listens only on `127.0.0.1:18765` and closes with Studio. Local programs can access the active document and files within the app's permissions while it runs. iOS, Android, and web keep Skin Studio's editing features but do not compile the MCP network server or its connection controls.

Studio launched from the desktop player closes when the player exits; a Studio launched separately from the command line remains independent. Imported music is copied into durable app storage on iOS, Android, and sandboxed macOS. Imports have unique names, so selecting two identically named songs does not replace either song. Audio copies are limited to 2 GiB per file. Skin/project archives are limited to 64 MiB on disk, 128 MiB expanded, 16 MiB per entry, and 64 MiB of decoded images.

iOS uses the shared audio decoder for local music and direct audio streams, including EQ, balance, and sample analysis. HLS uses AVPlayer and reports EQ, balance, and analysis as unavailable. The app declares the audio background mode and keeps iOS media-session controls. Simulator and automated checks do not verify physical-device interruptions, Bluetooth routing, background behavior, or thermal performance.

An unsigned iOS XCArchive cannot be installed on a device or uploaded as a finished submission. Signed runs export an IPA through `xcodebuild -exportArchive`; Mac uses `productbuild`. Upload with Transporter or App Store Connect tooling. Complete privacy labels and review the final binary for required-reason APIs, SDK privacy manifests and export compliance. Do not invent privacy declarations just to pass upload checks.

## Microsoft Store

Reserve the app in Partner Center. Set these repository variables to the exact registered values:

- `CRANAMP_MSIX_IDENTITY`: Package/Identity/Name.
- `CRANAMP_MSIX_PUBLISHER`: full Package/Identity/Publisher, including `CN=`.
- `CRANAMP_MSIX_PUBLISHER_DISPLAY_NAME`: publisher display name.

The desktop package declares `runFullTrust` and uses Cargo's version plus `.0`. Upload the MSIX in Partner Center and complete the listing, ratings and privacy disclosures. Microsoft applies Store signing. Local sideload testing requires a trusted development signature.

## Linux and AUR

`yay` installs recipes from the AUR. The AUR stores PKGBUILDs, not application binaries. The `cranamp` recipe builds the release source with Cargo's locked dependencies. The [cranamp-bin package](https://aur.archlinux.org/packages/cranamp-bin) installs the tested x86_64 store tarball and is available with `yay -S cranamp-bin`. CI generates both recipes and their `.SRCINFO` files using `makepkg`.

The existing AUR maintainer account is `faceless33`. Its Cranamp SSH key is registered; the private key is stored in the owner’s mounted Vault. The source recipe disables debug symbols to keep Rust release builds within available memory and disables Arch's C/C++ LTO so the native TLS library links with Rust. Rust release LTO remains enabled. Runtime dependencies include the libraries loaded dynamically by winit and wgpu, which ELF dependency scanning does not detect.

For a tagged release:

1. Attach `cranamp-VERSION-source.tar.gz` from the workflow artifact to the matching GitHub release. PKGBUILD already contains its checksum.
2. Test the recipe in a clean Arch build environment (`extra-x86_64-build`) and run `namcap`.
3. Use the registered `faceless33` account and its Vault SSH key.
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
