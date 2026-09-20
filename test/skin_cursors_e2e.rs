//! Classic skin cursors, from the `.wsz` on disk to the pointer icon the shell
//! hands the window.
//!
//! The player is composed for real here — the Skin Studio's presentation mode
//! draws the actual main window from a loaded skin — so the pointer icon that
//! comes back is the one a hovering mouse would get.

#![cfg(not(target_arch = "wasm32"))]

mod common;

use common::{pump, HitGraphRenderer};
use cranpose_app_shell::AppShell;
use cranpose_core::location_key;
use cranpose_ui::PointerIcon;

/// Points inside the main window, in the surface coordinates
/// `WinampSurfaceApp` draws it at.
const MAIN_WINDOW_ORIGIN: (f32, f32) = (26.0, 22.0);

fn surface_app() -> AppShell<HitGraphRenderer> {
    let root_key = location_key(file!(), line!(), column!());
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        root_key,
        cranamp::winamp::WinampSurfaceApp,
    );
    shell.set_buffer_size(500, 700);
    shell.set_viewport(500.0, 700.0);
    pump(&mut shell);
    shell
}

fn hover(shell: &mut AppShell<HitGraphRenderer>, (x, y): (f32, f32)) {
    shell.set_cursor(MAIN_WINDOW_ORIGIN.0 + x, MAIN_WINDOW_ORIGIN.1 + y);
    pump(shell);
}

#[test]
fn the_bundled_skin_asks_for_its_own_pointer_over_its_own_regions() {
    let mut shell = surface_app();
    let _ = shell.take_pointer_icon_change();

    hover(&mut shell, (137.0, 100.0));
    let body = shell
        .take_pointer_icon_change()
        .expect("the window body asks for the skin's own arrow");
    assert!(
        matches!(body, PointerIcon::Custom(_)),
        "the bundled skin draws its own pointer rather than naming a system one: {body:?}"
    );

    hover(&mut shell, (120.0, 76.0));
    let seek = shell
        .take_pointer_icon_change()
        .expect("the seek bar asks for a pointer of its own");
    assert_ne!(
        seek, body,
        "the seek bar and the window body must not share a pointer"
    );

    hover(&mut shell, (137.0, 7.0));
    let title = shell
        .take_pointer_icon_change()
        .expect("the title bar asks for a pointer of its own");
    assert_ne!(
        title, seek,
        "the title bar and the seek bar must not share a pointer"
    );
}

/// A 2x2 cursor of one flat colour with its hotspot at (1, 0): an icon
/// directory holding one bottom-up 32-bit DIB and its transparency mask.
///
/// `blue` picks the colour so two regions can carry visibly different cursors,
/// which is what lets a test tell one region's answer from another's.
fn sample_cursor(blue: bool) -> Vec<u8> {
    let mut dib = Vec::new();
    dib.extend_from_slice(&40u32.to_le_bytes());
    dib.extend_from_slice(&2u32.to_le_bytes());
    dib.extend_from_slice(&4u32.to_le_bytes());
    dib.extend_from_slice(&1u16.to_le_bytes());
    dib.extend_from_slice(&32u16.to_le_bytes());
    for _ in 0..5 {
        dib.extend_from_slice(&0u32.to_le_bytes());
    }
    dib.extend_from_slice(&0u32.to_le_bytes());
    let pixel: [u8; 4] = if blue {
        [255, 0, 0, 255]
    } else {
        [0, 0, 255, 255]
    };
    for _ in 0..4 {
        dib.extend_from_slice(&pixel);
    }
    dib.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]);

    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&[2, 2, 0, 0]);
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(dib.len() as u32).to_le_bytes());
    out.extend_from_slice(&22u32.to_le_bytes());
    out.extend_from_slice(&dib);
    out
}

/// Writes the bundled skin plus the named cursor files to a temporary `.wsz`.
/// Each entry names the file and whether its cursor is the blue one.
fn skin_with_cursors(name: &str, cursors: &[(&str, bool)]) -> std::path::PathBuf {
    use std::io::{Read, Write};

    let source = include_bytes!("../assets/winamp.wsz");
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(source.as_slice()))
        .expect("bundled skin is a zip");
    let mut bytes = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut bytes);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).expect("readable entry");
            let entry_name = entry.name().to_string();
            let mut data = Vec::new();
            entry.read_to_end(&mut data).expect("readable bytes");
            writer.start_file(entry_name, options).expect("writable");
            writer.write_all(&data).expect("written");
        }
        for (cursor, blue) in cursors {
            writer
                .start_file(*cursor, options)
                .expect("writable cursor");
            writer
                .write_all(&sample_cursor(*blue))
                .expect("written cursor");
        }
        writer.finish().expect("zip finishes");
    }
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, bytes.into_inner()).expect("test skin is written");
    path
}

/// The Skin Studio in presentation mode, which composes the real player from
/// the document's skin.
fn studio_presenting(path: &std::path::Path) -> AppShell<HitGraphRenderer> {
    let document = cranamp::winamp::studio::open_document(Some(
        path.to_str().expect("a utf-8 temporary path"),
    ))
    .expect("the test skin opens");
    document.lock().expect("document lock").view.presentation = true;

    let root_key = location_key(file!(), line!(), column!());
    let mut shell = AppShell::new(HitGraphRenderer::default(), root_key, move || {
        cranamp::winamp::studio::SkinStudio(document.clone(), None)
    });
    shell.set_buffer_size(1160, 850);
    shell.set_viewport(1160.0, 850.0);
    pump(&mut shell);
    shell
}

#[test]
fn a_skin_cursor_reaches_the_pointer_over_the_region_it_names() {
    let path = skin_with_cursors(
        "cranamp-cursor-test.wsz",
        &[("NORMAL.CUR", false), ("VOLBAL.CUR", true)],
    );
    let mut shell = studio_presenting(&path);
    let [origin_x, origin_y, zoom] =
        cranamp::winamp::studio::player_scene().expect("the preview reports where it drew");
    let _ = shell.take_pointer_icon_change();

    let mut hover_at = |x: f32, y: f32| {
        shell.set_cursor(origin_x + x * zoom, origin_y + y * zoom);
        pump(&mut shell);
        shell.take_pointer_icon_change()
    };

    let body = hover_at(137.0, 100.0).expect("the window body carries NORMAL.CUR");
    let PointerIcon::Custom(window_cursor) = body.clone() else {
        panic!("a skin cursor is a custom pointer icon, not a system one");
    };
    assert_eq!(window_cursor.image().width(), 2);
    assert_eq!(
        window_cursor.hotspot_x(),
        1,
        "the hotspot survives the trip"
    );

    assert_eq!(
        hover_at(120.0, 76.0),
        None,
        "the seek bar names no cursor of its own, so the window's answer stands"
    );

    let volume = hover_at(130.0, 62.0).expect("the volume slider carries VOLBAL.CUR");
    assert_ne!(
        volume, body,
        "the slider's own cursor replaces the window's"
    );

    assert_eq!(
        hover_at(137.0, 100.0),
        Some(body),
        "leaving the slider puts the window's cursor back"
    );

    std::fs::remove_file(path).ok();
}

#[test]
fn a_region_without_a_cursor_leaves_the_pointer_to_the_platform() {
    let path = skin_with_cursors("cranamp-cursor-volume-only.wsz", &[("VOLBAL.CUR", true)]);
    let mut shell = studio_presenting(&path);
    let [origin_x, origin_y, zoom] =
        cranamp::winamp::studio::player_scene().expect("the preview reports where it drew");
    let _ = shell.take_pointer_icon_change();

    let mut hover_at = |x: f32, y: f32| {
        shell.set_cursor(origin_x + x * zoom, origin_y + y * zoom);
        pump(&mut shell);
        shell.take_pointer_icon_change()
    };

    assert!(
        matches!(hover_at(130.0, 62.0), Some(PointerIcon::Custom(_))),
        "the volume slider carries the one cursor this skin ships"
    );
    assert_eq!(
        hover_at(137.0, 100.0),
        Some(PointerIcon::DEFAULT),
        "the window names no cursor, so the platform arrow comes back"
    );

    std::fs::remove_file(path).ok();
}
