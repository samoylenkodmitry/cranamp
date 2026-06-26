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
