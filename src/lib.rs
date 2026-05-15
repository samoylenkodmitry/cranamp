#![deny(unsafe_code)]

#[cfg(target_os = "android")]
mod android_bridge;
pub mod audio;
mod fonts;
pub mod winamp;

#[cfg(target_os = "android")]
use cranpose::AndroidOverlayWindowOptions;
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
pub fn create_android_app(can_draw_overlays: bool) -> AppLauncher {
    winamp::set_android_floating_overlay_enabled(can_draw_overlays);

    let launcher = AppLauncher::new()
        .with_title(TITLE)
        .with_fonts(fonts::APP_FONTS);

    if can_draw_overlays {
        launcher
            .with_size(
                winamp::ANDROID_OVERLAY_WIDTH,
                winamp::ANDROID_OVERLAY_HEIGHT,
            )
            .with_android_overlay_window(
                AndroidOverlayWindowOptions::new(
                    winamp::ANDROID_OVERLAY_WIDTH,
                    winamp::ANDROID_OVERLAY_HEIGHT,
                )
                .with_position(
                    winamp::ANDROID_OVERLAY_INITIAL_X,
                    winamp::ANDROID_OVERLAY_INITIAL_Y,
                ),
            )
    } else {
        launcher
    }
}

#[cfg(target_os = "ios")]
#[allow(unsafe_code)]
#[no_mangle]
pub extern "C" fn ios_main() {
    create_surface_app().run(winamp::WinampFullscreenApp);
}

#[cfg(target_os = "android")]
#[allow(unsafe_code)]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    if let Err(error) = android_bridge::init(&app) {
        log::error!("failed to initialize Cranamp Android bridge: {error}");
    }
    let can_draw_overlays = android_bridge::can_draw_overlays();
    if !can_draw_overlays {
        let _ = android_bridge::request_overlay_permission();
    }
    create_android_app(can_draw_overlays).run(app, winamp::WinampAndroidApp);
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
