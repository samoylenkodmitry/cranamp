use super::*;
use cranpose_app_shell::AppShell;
use cranpose_ui::{SemanticsNode, SemanticsRole};

#[path = "../../common/mod.rs"]
mod common;

const TITLES: [&str; 4] = [
    "07. E-Mantra - Distant Signals",
    "07. E-Mantra – Distant Signals (Artifact303 Remix)",
    "34. E-Mantra – Soul Searcher",
    "Музыка: 再生中",
];

fn text_bounds(node: &SemanticsNode, title: &str) -> Option<Rect> {
    if node.hidden {
        return None;
    }
    if node.description.as_deref() == Some(title)
        || matches!(&node.role, SemanticsRole::Text { value } if value.as_str() == title)
    {
        return Some(node.bounds);
    }
    node.children
        .iter()
        .find_map(|child| text_bounds(child, title))
}

#[test]
fn bitmap_and_unicode_playlist_titles_stay_in_the_same_row_grid() {
    for scale in [1.0, 1.25, 2.0] {
        let mut shell = AppShell::new(
            common::HitGraphRenderer::default(),
            cranpose_core::location_key(file!(), line!(), column!()),
            move || {
                Box(
                    Modifier::empty().size_points(600.0, 140.0),
                    BoxSpec::default(),
                    move || {
                        for (row, title) in TITLES.iter().enumerate() {
                            PlaylistWinampText(
                                (*title).into(),
                                4.0,
                                10.0 + row as f32 * 11.0,
                                250.0,
                                11.0,
                                scale,
                                [255; 4],
                            );
                        }
                    },
                );
            },
        );
        shell.set_buffer_size(600, 140);
        shell.set_viewport(600.0, 140.0);
        shell.set_semantics_enabled(true);
        common::pump(&mut shell);
        let tree = shell.semantics_tree().expect("playlist text semantics");
        for (row, title) in TITLES.iter().enumerate() {
            let bounds = text_bounds(tree.root(), title).expect("visible title");
            let top = ((10.0 + row as f32 * 11.0) * scale).round();
            assert!(
                (bounds.y - top).abs() < 0.01,
                "{title:?} at scale {scale}: top {} should be {top}",
                bounds.y
            );
            assert!((bounds.height - (11.0 * scale).round()).abs() < 0.01);
        }
    }
}
