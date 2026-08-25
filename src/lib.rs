#![deny(unsafe_code)]

pub mod audio;
mod fonts;
mod sync;
pub mod winamp;

use cranpose::AppLauncher;

const TITLE: &str = "Cranamp";

/// The id every framework-owned storage path is scoped by. It matches the
/// Android `applicationId`, so saved state follows the application rather than
/// the name of whichever binary launched it.
const APPLICATION_ID: &str = "com.cranamp.app";

/// What every target's launcher starts from.
///
/// Nothing is registered here: every shell installs its own platform's media
/// stack - `cranpose/media` for the in-process decoder that desktop and Android
/// share, and the iOS and web hosts for theirs.
fn launcher() -> AppLauncher {
    AppLauncher::new()
        .with_title(TITLE)
        .with_application_id(APPLICATION_ID)
        .with_fonts(fonts::APP_FONTS)
}

pub fn create_desktop_app() -> AppLauncher {
    launcher().with_size(1, 1)
}

pub fn create_surface_app() -> AppLauncher {
    launcher().with_size(900, 700)
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn create_web_app() -> AppLauncher {
    launcher().with_size(275, 493)
}

#[cfg(target_os = "android")]
pub fn create_android_app() -> AppLauncher {
    winamp::set_android_floating_overlay_enabled(false);
    launcher()
}

/// iOS entry point. winit starts `UIApplicationMain`, so this is called
/// directly from the `cranamp-ios` binary's `main`. The player renders into a
/// single fullscreen surface, inset by the system safe area.
#[cfg(all(feature = "ios", feature = "renderer-wgpu", target_os = "ios"))]
pub fn ios_entry_point() {
    launcher().run(ios_root);
}

#[cfg(all(feature = "ios", feature = "renderer-wgpu", target_os = "ios"))]
#[cranpose::composable]
fn ios_root() {
    cranpose::Box(
        cranpose::Modifier::empty()
            .fill_max_size()
            .safe_area_padding(),
        cranpose::BoxSpec::default(),
        winamp::WinampSurfaceApp,
    );
}

// The exported `android_main` symbol `NativeActivity` looks up. The framework
// writes it, so this crate keeps its `deny(unsafe_code)` and never names
// `android_activity` for a parameter type.
cranpose::android_main! {
    launcher: create_android_app(),
    content: winamp::WinampAndroidApp,
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
use wasm_bindgen::prelude::*;

#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[wasm_bindgen(start)]
pub fn web_init() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
    console_error_panic_hook::set_once();
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[wasm_bindgen]
pub async fn run_app() -> Result<(), JsValue> {
    create_web_app()
        .run_web("cranamp-canvas", winamp::WinampWidgetApp)
        .await
}
