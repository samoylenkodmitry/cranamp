# Distribution

Cranamp is a music player in Rust with WSZ support and its own Skin Studio. The desktop editor exposes MCP tools for agents to draw skins.

## Channels and artifacts

The existing **Release** workflow builds direct downloads. The new **Store packages** workflow runs for `v*` tags and manual dispatches. Its `store` Cargo feature removes updater UI and the APK installation capability, and starts without an automatic demo playlist. No channel adds the former radio preset. All channels use Catamp for fallback artwork.

| Destination | Workflow artifact | Format |
| --- | --- | --- |
| Google Play | `google-play-bundle` | AAB |
| iOS / TestFlight | `apple-ios-store` | XCArchive; signed runs also export an IPA |
| Mac App Store | `apple-macos-store` | Universal arm64/x86_64 XCArchive and PKG |
| Microsoft Store | `microsoft-store-msix` | x64 MSIX |
| Linux | `linux-repository-packages` | DEB, RPM, tarball, source archive, AUR PKGBUILD and `.SRCINFO` |

Tag builds produce validation artifacts without distribution credentials. Manually enable **signed** to require Apple and Google Play credentials. Signed runs fail if those credentials are missing. Windows packages remain unsigned because Microsoft signs Store submissions. Without Partner Center variables, validation runs use `Cranamp.LocalValidation`, which cannot be submitted.

Packages are retained as workflow artifacts. This workflow does not submit store listings or publish to repositories. Tags must match Cargo.toml. Apple build numbers use the workflow run number; start a new run for each new upload.

## Google Play

Set repository secrets:

- `CRANAMP_PLAY_UPLOAD_KEYSTORE_BASE64`
- `CRANAMP_PLAY_UPLOAD_KEYSTORE_PASSWORD`
- `CRANAMP_PLAY_UPLOAD_KEY_ALIAS`
- `CRANAMP_PLAY_UPLOAD_KEY_PASSWORD`

Use the upload key registered with Play App Signing. The application ID is `com.cranamp.app`. Establish signing continuity before distributing direct APKs and Play packages under this ID: their app-signing certificates must match for cross-channel updates.

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

The Mac package uses App Sandbox with user-selected file read/write, app-scoped bookmarks, network client access for streams, and network server access for the desktop MCP endpoint. Test saved-file reopening, folder sync, playback and MCP in the signed sandbox. Building a package does not validate those runtime behaviors.

An unsigned iOS XCArchive cannot be installed on a device or uploaded as a finished submission. Signed runs export an IPA through `xcodebuild -exportArchive`; Mac uses `productbuild`. Upload with Transporter or App Store Connect tooling. Complete privacy labels and review the final binary for required-reason APIs, SDK privacy manifests and export compliance. Do not invent privacy declarations just to pass upload checks.

## Microsoft Store

Reserve the app in Partner Center. Set these repository variables to the exact registered values:

- `CRANAMP_MSIX_IDENTITY`: Package/Identity/Name.
- `CRANAMP_MSIX_PUBLISHER`: full Package/Identity/Publisher, including `CN=`.
- `CRANAMP_MSIX_PUBLISHER_DISPLAY_NAME`: publisher display name.

The desktop package declares `runFullTrust` and uses Cargo's version plus `.0`. Upload the MSIX in Partner Center and complete the listing, ratings and privacy disclosures. Microsoft applies Store signing. Local sideload testing requires a trusted development signature.

## Linux and AUR

`yay` installs recipes from the AUR. The AUR stores PKGBUILDs, not application binaries. The generated recipe builds the release source with Cargo's locked dependencies. CI generates `.SRCINFO` using `makepkg`.

For a tagged release:

1. Attach `cranamp-VERSION-source.tar.gz` from the workflow artifact to the matching GitHub release. PKGBUILD already contains its checksum.
2. Test the recipe in a clean Arch build environment (`extra-x86_64-build`) and run `namcap`.
3. Register an AUR account and SSH key; create or take ownership of the `cranamp` package.
4. Push only PKGBUILD and `.SRCINFO` to `ssh://aur@aur.archlinux.org/cranamp.git`.
5. Users can then install with `yay -S cranamp`.

Do not submit a recipe from a non-tag run: its URL expects a published release. Source archives use the checked-out Git commit, so local uncommitted edits are not included.

DEB and RPM files can be attached to a GitHub release for direct installation. They do not create apt or dnf repositories. The x86_64 binaries are built on Ubuntu 22.04; verify installation and runtime dependencies on each advertised distribution. Official Debian/Fedora repositories require their own maintainers and packaging review. Flathub is a separate packaging target.

## Before submission

- Review generated THIRD-PARTY.html, the font notices, and MPL source availability. Cargo notices cover Rust dependencies; separately audit additional platform libraries.
- Keep provenance and permission records for bundled artwork. Packaging does not establish legal clearance.
- Complete the Android AAC patent assessment and privacy review identified in the initial investigation.
- Test the exact signed package on each OS, including file access and sandbox behavior.
- Supply product screenshots, support and privacy URLs, age ratings and publisher identity in the consoles.

References: [Android bundles](https://developer.android.com/build/building-cmdline), [Apple distribution](https://developer.apple.com/documentation/xcode/creating-distribution-signed-code-for-the-mac/), [Microsoft MSIX submissions](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/create-app-submission), [AUR](https://wiki.archlinux.org/title/Arch_User_Repository).
