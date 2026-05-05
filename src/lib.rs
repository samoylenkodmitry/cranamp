#![forbid(unsafe_code)]

pub mod audio;
mod fonts;
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

#[cfg(target_os = "ios")]
#[no_mangle]
pub extern "C" fn ios_main() {
    create_surface_app().run(winamp::WinampFullscreenApp);
}

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    create_surface_app().run(app, winamp::WinampFullscreenApp);
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
    create_surface_app()
        .run_web("cranamp-canvas", winamp::WinampWidgetApp)
        .await
}
