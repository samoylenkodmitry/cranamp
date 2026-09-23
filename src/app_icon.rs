//! The picture Cranamp's desktop windows carry in the title bar, taskbar and
//! task switcher. `scripts/dev/render_app_icon.py` draws it with the rest of
//! the icon set.

use anyhow::Result;
use cranpose::ImageBitmap;

const WINDOW_ICON_PNG: &[u8] = include_bytes!("../assets/icon/cranamp-window.png");

pub(crate) fn window_icon() -> Result<ImageBitmap> {
    let rgba =
        image::load_from_memory_with_format(WINDOW_ICON_PNG, image::ImageFormat::Png)?.into_rgba8();
    Ok(ImageBitmap::from_rgba8(
        rgba.width(),
        rgba.height(),
        rgba.into_raw(),
    )?)
}

#[cfg(test)]
#[path = "../test/unit/app_icon/tests.rs"]
mod tests;
