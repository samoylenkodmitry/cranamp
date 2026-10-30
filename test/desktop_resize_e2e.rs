mod common;
use common::HitGraphRenderer;
use cranpose_app_shell::{AppShell, RootId};
use cranpose_core::location_key;
use cranpose_ui::Size;

fn sync(shell: &mut AppShell<HitGraphRenderer>, root: RootId) -> Size {
    for _ in 0..6 {
        shell.update();
    }
    // Without an OS driver the native descriptor keeps its initial size.
    // Check the real panel bounds produced by the public player composition.
    shell
        .surface(root)
        .unwrap()
        .scene()
        .hits
        .iter()
        .filter(|hit| hit.rect.x.abs() < 0.001 && hit.rect.height > 40.)
        .fold(Size::ZERO, |size, hit| {
            Size::new(
                size.width.max(hit.rect.x + hit.rect.width),
                size.height.max(hit.rect.y + hit.rect.height),
            )
        })
}

fn assert_panel_widths(shell: &mut AppShell<HitGraphRenderer>, root: RootId, size: Size) {
    let main_height = size.width * 116. / 275.;
    for y in [0., main_height, main_height * 2.] {
        assert!(
            shell.surface(root).unwrap().scene().hits.iter().any(|hit| {
                (hit.rect.y - y).abs() < 1.
                    && (hit.rect.width - size.width).abs() < 1.
                    && hit.rect.height > 80.
            }),
            "the panel at {y} fills the shared width {}",
            size.width
        );
    }
}

fn drag(shell: &mut AppShell<HitGraphRenderer>, root: RootId, dx: f32, dy: f32) -> Size {
    let size = sync(shell, root);
    let x = size.width - 3.;
    let y = size.height - 3.;
    shell.surface(root).unwrap().set_cursor(x, y);
    assert!(
        shell.surface(root).unwrap().pointer_pressed(),
        "the playlist corner handles the press"
    );
    sync(shell, root);
    for step in 1..=4 {
        shell
            .surface(root)
            .unwrap()
            .set_cursor(x + dx * step as f32 / 4., y + dy * step as f32 / 4.);
        sync(shell, root);
    }
    shell.surface(root).unwrap().pointer_released();
    sync(shell, root)
}

#[test]
fn native_stack_grows_and_shrinks_from_the_playlist_corner_at_fractional_dpi() {
    cranpose_services::set_application_id(&format!(
        "com.cranamp.resize-test-{}",
        std::process::id()
    ))
    .unwrap();
    let config = cranpose_services::application_directories()
        .unwrap()
        .config
        .join("windows.conf");
    let _ = std::fs::remove_file(&config);
    let density = 130. / 96.;
    let mut shell = AppShell::new_with_size_and_density(
        HitGraphRenderer::default(),
        location_key(file!(), line!(), column!()),
        cranamp::winamp::WinampStandaloneApp,
        (1, 1),
        (1., 1.),
        density,
    );
    for _ in 0..8 {
        shell.update();
    }
    let entry = shell.window_roots().remove(0);
    let initial = entry.descriptor.layout_size();
    let root = RootId::Window(entry.node);
    shell.add_window_surface(
        entry.node,
        HitGraphRenderer::default(),
        (initial.width.ceil() as u32, initial.height.ceil() as u32),
        (initial.width, initial.height),
    );
    assert!((initial.width * density - 412.5).abs() < 0.01);
    let initial = sync(&mut shell, root);
    let smaller = drag(&mut shell, root, -90., -140.);
    assert_panel_widths(&mut shell, root, smaller);
    assert!(
        (smaller.width - initial.width + 90.).abs() < 1.,
        "{initial:?} -> {smaller:?}"
    );
    assert!((smaller.height - initial.height + 140.).abs() < 1.);
    let restored = drag(&mut shell, root, 90., 140.);
    assert!(
        (restored.width - initial.width).abs() < 1.,
        "{smaller:?} -> {restored:?}"
    );
    assert!((restored.height - initial.height).abs() < 1.);
    assert_panel_widths(&mut shell, root, restored);
    let shorter = drag(&mut shell, root, 0., -80.);
    assert!((shorter.width - initial.width).abs() < 1.);
    assert!((shorter.height - initial.height + 80.).abs() < 1.);
    assert_panel_widths(&mut shell, root, shorter);
    drag(&mut shell, root, 0., 80.);
    let _ = std::fs::remove_file(config);
}
