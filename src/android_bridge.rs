#![allow(unsafe_code)]

use std::path::PathBuf;
use std::sync::OnceLock;

use android_activity::AndroidApp;
use jni::errors::Result as JniResult;
use jni::objects::{JObject, JString};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::strings::JNIString;
use jni::sys::jobject;
use jni::vm::JavaVM;
use jni::{jni_sig, jni_str, JValue};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AndroidBridgeResult {
    PlaylistImport { text: String },
    PlaylistExport { target: String },
    Cancelled { operation: &'static str },
    Error(String),
}

/// Progress of an in-app self-update, parsed from `update_status` written by the
/// Java side (see `CranampActivity.cranampCheckUpdate`/`cranampInstallUpdate`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateStatus {
    Checking,
    UpToDate,
    Available { version: String, url: String },
    Downloading { pct: u8 },
    Installing,
    Error(String),
}

struct AndroidBridge {
    vm: JavaVM,
    activity: Global<JObject<'static>>,
    bridge_dir: PathBuf,
}

static BRIDGE: OnceLock<AndroidBridge> = OnceLock::new();

pub fn init(app: &AndroidApp) -> Result<(), String> {
    if BRIDGE.get().is_some() {
        return Ok(());
    }

    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
    let (activity, bridge_dir) = vm
        .attach_current_thread(|env| -> JniResult<(Global<JObject<'static>>, String)> {
            let activity_obj = unsafe { JObject::from_raw(env, app.activity_as_ptr() as jobject) };
            let activity = env.new_global_ref(&activity_obj)?;
            let bridge_dir_obj = env
                .call_method(
                    activity.as_obj(),
                    jni_str!("cranampBridgeDirectory"),
                    jni_sig!("()Ljava/lang/String;"),
                    &[],
                )
                .and_then(|value| value.l())?;
            let bridge_dir = JString::cast_local(env, bridge_dir_obj)?.try_to_string(env)?;
            Ok((activity, bridge_dir))
        })
        .map_err(|error| format!("failed to initialize Android bridge: {error}"))?;

    let bridge = AndroidBridge {
        vm,
        activity,
        bridge_dir: PathBuf::from(bridge_dir),
    };
    let _ = BRIDGE.set(bridge);
    Ok(())
}

pub fn request_playlist_import() -> Result<(), String> {
    call_android_picker("cranampImportPlaylist", "()V", &[])
}

pub fn request_playlist_export(text: &str) -> Result<(), String> {
    let Some(bridge) = BRIDGE.get() else {
        return Err("Android activity bridge is not initialized".to_string());
    };
    bridge
        .vm
        .attach_current_thread(|env| -> JniResult<()> {
            let text = env.new_string(text)?;
            let text_obj: &JObject<'_> = text.as_ref();
            env.call_method(
                bridge.activity.as_obj(),
                jni_str!("cranampExportPlaylist"),
                jni_sig!("(Ljava/lang/String;)V"),
                &[JValue::Object(text_obj)],
            )?;
            Ok(())
        })
        .map_err(|error| format!("failed to launch Android playlist export: {error}"))?;
    Ok(())
}

/// Asks the Java side to query GitHub for the latest release and compare it to
/// `current_version`. The result is written to `update_status` and surfaced via
/// [`read_update_status`].
pub fn request_check_update(current_version: &str) -> Result<(), String> {
    call_android_string_arg("cranampCheckUpdate", current_version)
        .map_err(|error| format!("failed to start update check: {error}"))
}

/// Asks the Java side to download the APK at `url` and launch the in-place
/// installer. Download/install progress is written to `update_status`.
pub fn request_install_update(url: &str) -> Result<(), String> {
    call_android_string_arg("cranampInstallUpdate", url)
        .map_err(|error| format!("failed to start update install: {error}"))
}

/// Reads (and consumes) the latest `update_status` line written by the Java
/// update worker. Returns `None` when no new status is pending.
pub fn read_update_status() -> Option<UpdateStatus> {
    let bridge = BRIDGE.get()?;
    let text = take_file_to_string(&bridge.bridge_dir.join("update_status"))?;

    let mut state = None;
    let mut version = String::new();
    let mut url = String::new();
    let mut pct: u8 = 0;
    let mut message = String::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "state" => state = Some(value.to_string()),
            "version" => version = value.to_string(),
            "url" => url = value.to_string(),
            "pct" => pct = value.parse().unwrap_or(0),
            "message" => message = value.to_string(),
            _ => {}
        }
    }

    match state.as_deref()? {
        "checking" => Some(UpdateStatus::Checking),
        "uptodate" => Some(UpdateStatus::UpToDate),
        "available" => Some(UpdateStatus::Available { version, url }),
        "downloading" => Some(UpdateStatus::Downloading { pct }),
        "installing" => Some(UpdateStatus::Installing),
        "error" => Some(UpdateStatus::Error(if message.is_empty() {
            "update failed".to_string()
        } else {
            message
        })),
        _ => None,
    }
}

pub fn take_results() -> Vec<AndroidBridgeResult> {
    let Some(bridge) = BRIDGE.get() else {
        return Vec::new();
    };

    let mut results = Vec::new();
    collect_playlist_import_result(&bridge.bridge_dir, &mut results);
    collect_playlist_export_result(&bridge.bridge_dir, &mut results);
    results
}

pub fn config_dir() -> Option<PathBuf> {
    BRIDGE.get().map(|bridge| bridge.bridge_dir.join("config"))
}

/// App-private directory for temporary streaming caches. Lives under the same
/// writable bridge directory as the config, so the audio engine can spool a
/// `content://` stream to disk for seeking without touching shared storage.
pub fn stream_cache_dir() -> Option<PathBuf> {
    BRIDGE
        .get()
        .map(|bridge| bridge.bridge_dir.join("stream-cache"))
}

/// Calls a `void method(String)` on the activity with a single string argument.
fn call_android_string_arg(method: &str, value: &str) -> Result<(), String> {
    let Some(bridge) = BRIDGE.get() else {
        return Err("Android activity bridge is not initialized".to_string());
    };
    let method = JNIString::new(method);
    bridge
        .vm
        .attach_current_thread(|env| -> JniResult<()> {
            let value = env.new_string(value)?;
            let value_obj: &JObject<'_> = value.as_ref();
            env.call_method(
                bridge.activity.as_obj(),
                method.as_ref(),
                jni_sig!("(Ljava/lang/String;)V"),
                &[JValue::Object(value_obj)],
            )?;
            Ok(())
        })
        .map_err(|error| format!("{error}"))?;
    Ok(())
}

fn call_android_picker(method: &str, signature: &str, args: &[JValue<'_>]) -> Result<(), String> {
    let Some(bridge) = BRIDGE.get() else {
        return Err("Android activity bridge is not initialized".to_string());
    };
    let method = JNIString::new(method);
    let signature = RuntimeMethodSignature::from_str(signature)
        .map_err(|error| format!("invalid Android picker signature: {error}"))?;
    bridge
        .vm
        .attach_current_thread(|env| -> JniResult<()> {
            env.call_method(
                bridge.activity.as_obj(),
                method.as_ref(),
                signature.method_signature(),
                args,
            )?;
            Ok(())
        })
        .map_err(|error| format!("failed to launch Android picker: {error}"))?;
    Ok(())
}

fn collect_playlist_import_result(
    bridge_dir: &std::path::Path,
    results: &mut Vec<AndroidBridgeResult>,
) {
    let import_file = bridge_dir.join("playlist_import.m3u");
    if let Some(text) = take_file_to_string(&import_file) {
        results.push(AndroidBridgeResult::PlaylistImport { text });
    }
    collect_cancel_error(bridge_dir, "playlist_import", "Playlist Import", results);
}

fn collect_playlist_export_result(
    bridge_dir: &std::path::Path,
    results: &mut Vec<AndroidBridgeResult>,
) {
    let ok_file = bridge_dir.join("playlist_export.ok");
    if let Some(target) = take_file_to_string(&ok_file) {
        results.push(AndroidBridgeResult::PlaylistExport {
            target: target.trim().to_string(),
        });
    }
    collect_cancel_error(bridge_dir, "playlist_export", "Playlist Export", results);
}

fn collect_cancel_error(
    bridge_dir: &std::path::Path,
    name: &'static str,
    operation: &'static str,
    results: &mut Vec<AndroidBridgeResult>,
) {
    let cancel_file = bridge_dir.join(format!("{name}.cancel"));
    if cancel_file.is_file() {
        let _ = std::fs::remove_file(cancel_file);
        results.push(AndroidBridgeResult::Cancelled { operation });
    }

    let error_file = bridge_dir.join(format!("{name}.error"));
    if let Some(error) = take_file_to_string(&error_file) {
        let error = error.trim();
        results.push(AndroidBridgeResult::Error(if error.is_empty() {
            format!("{operation} failed")
        } else {
            error.to_string()
        }));
    }
}

fn take_file_to_string(path: &std::path::Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    let _ = std::fs::remove_file(path);
    Some(text)
}
