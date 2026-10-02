#!/usr/bin/env python3
"""Build Apple store archives; distribution signing is explicit and fails closed."""
import argparse
import datetime
import os
import pathlib
import plistlib
import shutil
import subprocess
import tempfile
from version import ROOT, release_version


def run(*args, **kwargs):
    return subprocess.run([str(a) for a in args], check=True, **kwargs)


def output(*args):
    return subprocess.check_output([str(a) for a in args], text=True).strip()


def write_plist(path, value):
    path.write_bytes(plistlib.dumps(value))


def required(name):
    value = os.environ.get(name)
    if not value:
        raise SystemExit(f"Missing {name}; signed store builds never fall back to development signing")
    return value


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("platform", choices=["ios", "macos"])
    args = parser.parse_args()
    os.chdir(ROOT)
    version = release_version()
    signed = os.environ.get("CRANAMP_STORE_SIGNED") == "true"
    bundle_id = os.environ.get("CRANAMP_APPLE_BUNDLE_ID", "io.cranamp.app")
    build_number = os.environ.get("CRANAMP_BUILD_NUMBER", "1")
    if not build_number.isdecimal() or int(build_number) < 1:
        raise SystemExit("CRANAMP_BUILD_NUMBER must be a positive integer")
    kind = args.platform
    work = ROOT / "target/distribution" / kind
    archive = work / "Cranamp.xcarchive"
    if work.exists():
        shutil.rmtree(work)
    app = archive / "Products/Applications/Cranamp.app"
    app.parent.mkdir(parents=True)
    out = ROOT / "store-artifacts"
    out.mkdir(exist_ok=True)
    if kind == "ios":
        env = dict(os.environ, PROFILE="release", CRANAMP_IOS_FEATURES="ios,store", CRANAMP_UNSIGNED="true")
        run("bash", "platform/ios/build-app.sh", "aarch64-apple-ios", env=env)
        shutil.copytree(ROOT / "target/aarch64-apple-ios/release/Cranamp.app", app)
        plist_path = app / "Info.plist"
        resources = app
        binary = app / "Cranamp"
        sdk = "iphoneos"
    else:
        for target in ("aarch64-apple-darwin", "x86_64-apple-darwin"):
            run("cargo", "build", "--locked", "--release", "--features", "store", "--target", target)
        (app / "Contents/MacOS").mkdir(parents=True)
        resources = app / "Contents/Resources"
        resources.mkdir()
        binary = app / "Contents/MacOS/Cranamp"
        run("lipo", "-create", ROOT / "target/aarch64-apple-darwin/release/cranamp",
            ROOT / "target/x86_64-apple-darwin/release/cranamp", "-output", binary)
        plist_path = app / "Contents/Info.plist"
        shutil.copyfile(ROOT / "platform/macos/Info.plist", plist_path)
        shutil.copyfile(ROOT / "assets/icon/Cranamp.icns", resources / "Cranamp.icns")
        sdk = "macosx"
    for name, source in [("LICENSE", ROOT / "LICENSE"), ("LiberationSans-OFL-1.1.txt", ROOT / "docs/third-party/LiberationSans-OFL-1.1.txt"), ("THIRD-PARTY.html", ROOT / "target/distribution/THIRD-PARTY.html"), ("PrivacyInfo.xcprivacy", ROOT / "platform/apple/PrivacyInfo.xcprivacy")]:
        shutil.copyfile(source, resources / name)
    plist = plistlib.loads(plist_path.read_bytes())
    xcode_lines = output("xcodebuild", "-version").splitlines()
    xcode_parts = xcode_lines[0].split()[1].split(".")
    xcode_number = f"{int(xcode_parts[0]):02}{int(xcode_parts[1]):01}{int(xcode_parts[2]) if len(xcode_parts) > 2 else 0:01}"
    sdk_version = output("xcrun", "--sdk", sdk, "--show-sdk-version")
    plist.update(CFBundleIdentifier=bundle_id, CFBundleShortVersionString=version,
                 CFBundleVersion=build_number, CFBundleSupportedPlatforms=["iPhoneOS" if kind == "ios" else "MacOSX"],
                 DTPlatformName=sdk, DTPlatformVersion=sdk_version, DTSDKName=sdk + sdk_version,
                 DTSDKBuild=output("xcrun", "--sdk", sdk, "--show-sdk-build-version"),
                 DTPlatformBuild=output("xcrun", "--sdk", sdk, "--show-sdk-build-version"),
                 DTXcode=xcode_number, DTXcodeBuild=xcode_lines[1].split()[-1],
                 DTCompiler="com.apple.compilers.llvm.clang.1_0", BuildMachineOSBuild=output("sw_vers", "-buildVersion"))
    if kind == "macos":
        plist["LSApplicationCategoryType"] = "public.app-category.music"
    write_plist(plist_path, plist)
    entitlements = {}
    team = ""
    profile_name = ""
    if signed:
        profile_path = pathlib.Path(required("CRANAMP_PROVISIONING_PROFILE"))
        profile = plistlib.loads(subprocess.check_output(["security", "cms", "-D", "-i", str(profile_path)]))
        entitlements = profile["Entitlements"]
        team = profile["TeamIdentifier"][0]
        profile_name = profile["Name"]
        if profile.get("ProvisionedDevices") or profile.get("ProvisionsAllDevices") or entitlements.get("get-task-allow"):
            raise SystemExit("Use an App Store distribution profile, not a development, ad hoc or enterprise profile")
        if profile["ExpirationDate"] <= datetime.datetime.now(datetime.timezone.utc).replace(tzinfo=None):
            raise SystemExit("Provisioning profile has expired")
        app_identifier = entitlements.get("application-identifier", entitlements.get("com.apple.application-identifier", ""))
        if app_identifier != profile["ApplicationIdentifierPrefix"][0] + "." + bundle_id:
            raise SystemExit("Provisioning profile must explicitly match the team and bundle ID")
        destination = app / "embedded.mobileprovision" if kind == "ios" else app / "Contents/embedded.provisionprofile"
        shutil.copyfile(profile_path, destination)
    if kind == "macos":
        entitlements.update({"com.apple.security.app-sandbox": True,
                             "com.apple.security.files.user-selected.read-write": True,
                             "com.apple.security.files.bookmarks.app-scope": True,
                             "com.apple.security.network.client": True,
                             "com.apple.security.network.server": True})
    entitlements_path = work / "entitlements.plist"
    write_plist(entitlements_path, entitlements)
    sign_args = ["codesign", "--force", "--sign", required("CRANAMP_APPLE_SIGN_IDENTITY") if signed else "-",
                 "--entitlements", entitlements_path]
    if signed:
        sign_args += ["--keychain", required("CRANAMP_SIGN_KEYCHAIN"), "--timestamp"]
    run(*sign_args, app)
    run("codesign", "--verify", "--strict", app)
    properties = {"ApplicationPath": "Applications/Cranamp.app", "CFBundleIdentifier": bundle_id,
                  "CFBundleShortVersionString": version, "CFBundleVersion": build_number,
                  "Architectures": ["arm64"] if kind == "ios" else ["arm64", "x86_64"],
                  "SigningIdentity": os.environ.get("CRANAMP_APPLE_SIGN_IDENTITY", "-"), "Team": team}
    write_plist(archive / "Info.plist", {"ArchiveVersion": 2, "Name": "Cranamp", "SchemeName": "Cranamp",
                "CreationDate": datetime.datetime.now(datetime.timezone.utc).replace(tzinfo=None), "ApplicationProperties": properties})
    run("ditto", "-c", "-k", "--keepParent", archive, out / f"cranamp-{version}-{kind}.xcarchive.zip")
    if kind == "ios" and signed:
        options = work / "ExportOptions.plist"
        write_plist(options, {"method": "app-store-connect", "destination": "export", "teamID": team,
                    "signingStyle": "manual", "signingCertificate": required("CRANAMP_APPLE_SIGN_IDENTITY"),
                    "provisioningProfiles": {bundle_id: profile_name}, "manageAppVersionAndBuildNumber": False})
        export = work / "export"
        run("xcodebuild", "-exportArchive", "-archivePath", archive, "-exportOptionsPlist", options, "-exportPath", export)
        ipas = list(export.glob("*.ipa"))
        if len(ipas) != 1:
            raise SystemExit("Expected one exported App Store IPA")
        shutil.copyfile(ipas[0], out / f"cranamp-{version}-ios-app-store.ipa")
    elif kind == "macos":
        pkg_args = ["productbuild", "--component", app, "/Applications"]
        if signed:
            pkg_args += ["--sign", required("CRANAMP_MAC_INSTALLER_IDENTITY"), "--keychain", required("CRANAMP_SIGN_KEYCHAIN")]
        run(*pkg_args, out / f"cranamp-{version}-macos-{'app-store' if signed else 'unsigned'}.pkg")
    if not signed:
        print("Validation artifacts only: distribution signing is required before store submission.")

if __name__ == "__main__":
    main()
