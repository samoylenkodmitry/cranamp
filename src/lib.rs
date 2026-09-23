#![deny(unsafe_code)]
#![recursion_limit = "256"]
mod app_icon;
pub mod audio;
mod fonts;
mod sync;
pub mod winamp;
use cranpose::{AppLauncher, CustomCursorSize};
cranpose::app_capabilities!();
const TITLE: &str = "Cranamp";
const APPLICATION_ID: &str = "com.cranamp.app";
fn launcher() -> AppLauncher {
    AppLauncher::new()
        .with_capabilities(&CAPABILITIES)
        .with_title(TITLE)
        .with_application_id(APPLICATION_ID)
        .with_fonts(fonts::APP_FONTS)
}
fn desktop_launcher() -> AppLauncher {
    // A skin's cursors are pixel art drawn at an exact size; enlarged with the
    // system pointer they turn into smeared blocks.
    let launcher = launcher().with_custom_cursor_size(CustomCursorSize::AsDrawn);
    match app_icon::window_icon() {
        Ok(icon) => launcher.with_window_icon(icon),
        Err(error) => {
            log::warn!("Cranamp's window icon did not decode: {error:#}");
            launcher
        }
    }
}
pub fn create_desktop_app() -> AppLauncher {
    desktop_launcher().with_size(1, 1)
}
pub fn create_surface_app() -> AppLauncher {
    desktop_launcher().with_size(900, 700)
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
        winamp::WinampStackedApp,
    );
}
cranpose::android_main! {
    launcher: create_android_app(),
    content: winamp::WinampStackedApp,
}
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use wasm_bindgen::prelude::*;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[wasm_bindgen(start)]
pub fn web_init() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
    console_error_panic_hook::set_once();
}
/// Called by the page when it moves the player's canvas into a floating
/// window of its own, and again when the canvas comes back to the page.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[wasm_bindgen]
pub fn set_floating(floating: bool) {
    winamp::set_floating_surface(floating);
}
/// The width and height a floating window opens at to hold the player as it
/// is now.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[wasm_bindgen]
pub fn floating_size() -> Vec<f32> {
    let size = winamp::floating_open_size();
    vec![size.width, size.height]
}
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[wasm_bindgen]
pub async fn run_app() -> Result<(), JsValue> {
    create_web_app()
        .run_web("cranamp-canvas", winamp::WinampWidgetApp)
        .await
}
