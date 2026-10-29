//! What Cranamp asks of a device. The Android manifest and the Apple usage
//! descriptions are written from this one list. A Windows build also carries
//! the icon Explorer, the Start menu and shortcuts show, and the name Task
//! Manager and the volume mixer list it under.

use std::{env, fs, path::PathBuf};

use cranpose_capabilities::{declare, Use};

fn main() {
    embed_license_catalog();
    let mut capabilities = vec![Use::media(), Use::network()];
    if env::var_os("CARGO_FEATURE_STORE").is_none() {
        capabilities.push(Use::update());
    }
    declare(&capabilities).emit();
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_windows_resources();
    }
}

fn embed_license_catalog() {
    println!("cargo::rerun-if-env-changed=CRANAMP_NOTICE_CATALOG");
    let catalog = if let Some(path) = env::var_os("CRANAMP_NOTICE_CATALOG") {
        let path = PathBuf::from(path);
        println!("cargo::rerun-if-changed={}", path.display());
        fs::read_to_string(path).expect("read generated release license catalog")
    } else {
        // Local builds retain the app and font licenses. Packaging supplies the
        // dependency catalog for the exact target and feature set it builds.
        "[]".into()
    };
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("cargo build output"));
    fs::write(out.join("notices.json"), catalog).expect("embed license catalog");
}

fn embed_windows_resources() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo names the manifest dir"));
    let icon = root.join("assets").join("icon").join("cranamp.ico");
    println!("cargo::rerun-if-changed={}", icon.display());

    let version = env::var("CARGO_PKG_VERSION").expect("cargo names the package version");
    let numeric = version
        .split(['.', '-', '+'])
        .map(|part| part.parse::<u16>().unwrap_or(0))
        .chain(std::iter::repeat(0))
        .take(4)
        .map(|part| part.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let icon = icon.display().to_string().replace('\\', "/");
    let script = format!(
        r#"1 ICON "{icon}"

1 VERSIONINFO
FILEVERSION {numeric}
PRODUCTVERSION {numeric}
FILEOS 0x40004
FILETYPE 0x1
{{
    BLOCK "StringFileInfo"
    {{
        BLOCK "040904B0"
        {{
            VALUE "FileDescription", "Cranamp"
            VALUE "FileVersion", "{version}"
            VALUE "InternalName", "cranamp"
            VALUE "OriginalFilename", "cranamp.exe"
            VALUE "ProductName", "Cranamp"
            VALUE "ProductVersion", "{version}"
        }}
    }}
    BLOCK "VarFileInfo"
    {{
        VALUE "Translation", 0x409, 1200
    }}
}}
"#
    );
    let out =
        PathBuf::from(env::var("OUT_DIR").expect("cargo names the out dir")).join("cranamp.rc");
    fs::write(&out, script).expect("write the Windows resource script");
    if let Err(error) =
        embed_resource::compile_for(&out, ["cranamp"], embed_resource::NONE).manifest_required()
    {
        panic!("the Windows icon and version resource did not build: {error}");
    }
}
