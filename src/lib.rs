#![deny(unsafe_code)]

#[cfg(target_os = "android")]
mod android_bridge;
pub mod audio;
mod fonts;
mod sync;
pub mod winamp;

use cranpose::AppLauncher;

const TITLE: &str = "Cranamp";

pub fn create_desktop_app() -> AppLauncher {
    AppLauncher::new()
        .with_title(TITLE)
        .with_size(1, 1)
        .with_fonts(fonts::APP_FONTS)
}

pub fn create_surface_app() -> AppLauncher {
    AppLauncher::new()
        .with_title(TITLE)
        .with_size(900, 700)
        .with_fonts(fonts::APP_FONTS)
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn create_web_app() -> AppLauncher {
    AppLauncher::new()
        .with_title(TITLE)
        .with_size(275, 493)
        .with_fonts(fonts::APP_FONTS)
}

#[cfg(target_os = "android")]
pub fn create_android_app() -> AppLauncher {
    winamp::set_android_floating_overlay_enabled(false);
    AppLauncher::new()
        .with_title(TITLE)
        .with_fonts(fonts::APP_FONTS)
}

/// iOS entry point. winit starts `UIApplicationMain`, so this is called
/// directly from the `cranamp-ios` binary's `main`. The player renders into a
/// single fullscreen surface, inset by the system safe area.
#[cfg(all(feature = "ios", feature = "renderer-wgpu", target_os = "ios"))]
pub fn ios_entry_point() {
    AppLauncher::new()
        .with_title(TITLE)
        .with_fonts(fonts::APP_FONTS)
        .run(ios_root);
}

#[cfg(all(feature = "ios", feature = "renderer-wgpu", target_os = "ios"))]
#[cranpose::composable]
fn ios_root() {
    let insets = cranpose::local_safe_area_insets().current();
    cranpose::Box(
        cranpose::Modifier::empty().fill_max_size().padding_each(
            insets.left,
            insets.top,
            insets.right,
            insets.bottom,
        ),
        cranpose::BoxSpec::default(),
        winamp::WinampSurfaceApp,
    );
}

#[cfg(target_os = "android")]
#[allow(unsafe_code)]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    if let Err(error) = android_bridge::init(&app) {
        log::error!("failed to initialize Cranamp Android bridge: {error}");
    }
    create_android_app().run(app, winamp::WinampAndroidApp);
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
