//! Framework-level end-to-end tests, driven through cranpose's real
//! hit-dispatch pipeline (no device/adb): synthesized pointer clicks on the
//! actual composables, asserting on what a real click dispatch produces
//! rather than on the underlying state functions directly.
//!
//! Uses a `HitGraphRenderer` that builds a real hit graph from the layout tree
//! (via the published `cranpose-render-common`) so `pointer_pressed`/
//! `pointer_released` dispatch to live click handlers.

#![cfg(not(target_arch = "wasm32"))]

use cranpose_app_shell::AppShell;
use cranpose_core::location_key;
use cranpose_foundation::{Modifiers, PointerEvent};
use cranpose_render_common::graph::ProjectiveTransform;
use cranpose_render_common::graph_scene::{ClickAction, HitGeometry, Scene};
use cranpose_render_common::hit_graph::{collect_hits_from_graph, HitGraphSink};
use cranpose_render_common::{RenderScene, Renderer};
use cranpose_ui::{LayoutTree, Size};
use cranpose_ui_graphics::{Point, RoundedCornerShape};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct HitGraphRenderer {
    scene: Scene,
}

struct SceneHitSink<'a> {
    scene: &'a mut Scene,
}

impl HitGraphSink for SceneHitSink<'_> {
    fn push_hit(
        &mut self,
        node_id: cranpose_core::NodeId,
        capture_path: &[cranpose_core::NodeId],
        geometry: HitGeometry,
        shape: Option<RoundedCornerShape>,
        click_actions: &[Rc<dyn Fn(Point)>],
        pointer_inputs: &[Rc<dyn Fn(PointerEvent)>],
    ) {
        self.scene.push_hit(
            node_id,
            capture_path.to_vec(),
            geometry,
            shape,
            click_actions
                .iter()
                .cloned()
                .map(ClickAction::WithPoint)
                .collect(),
            pointer_inputs.to_vec(),
        );
    }
}

impl Renderer for HitGraphRenderer {
    type Scene = Scene;
    type Error = ();

    fn scene(&self) -> &Self::Scene {
        &self.scene
    }

    fn scene_mut(&mut self) -> &mut Self::Scene {
        &mut self.scene
    }

    fn rebuild_scene(
        &mut self,
        layout_tree: &LayoutTree,
        _viewport: Size,
    ) -> Result<(), Self::Error> {
        self.scene.clear();
        let graph = cranpose_render_common::scene_builder::build_graph_from_layout_tree(
            layout_tree.root(),
            1.0,
        );
        let mut sink = SceneHitSink {
            scene: &mut self.scene,
        };
        collect_hits_from_graph(
            &graph.root,
            ProjectiveTransform::identity(),
            &mut sink,
            None,
        );
        self.scene.replace_graph(graph);
        Ok(())
    }

    fn rebuild_scene_from_applier(
        &mut self,
        applier: &mut cranpose_core::MemoryApplier,
        root: cranpose_core::NodeId,
        _viewport: Size,
    ) -> Result<(), Self::Error> {
        self.scene.clear();
        if let Some(graph) =
            cranpose_render_common::scene_builder::build_graph_from_applier(applier, root, 1.0)
        {
            let mut sink = SceneHitSink {
                scene: &mut self.scene,
            };
            collect_hits_from_graph(
                &graph.root,
                ProjectiveTransform::identity(),
                &mut sink,
                None,
            );
            self.scene.replace_graph(graph);
        }
        Ok(())
    }
}

fn pump(shell: &mut AppShell<HitGraphRenderer>) {
    for _ in 0..80 {
        if !(shell.needs_redraw() || shell.has_active_animations()) {
            break;
        }
        shell.update();
    }
}

fn collect_texts(node: &cranpose_ui::LayoutBox, out: &mut Vec<String>) {
    if let Some(text) = node.node_data.modifier_slices().text_content() {
        out.push(text.to_string());
    }
    for child in &node.children {
        collect_texts(child, out);
    }
}

fn visible_texts(shell: &mut AppShell<HitGraphRenderer>) -> Vec<String> {
    shell.with_layout_tree(|tree| {
        let mut texts = Vec::new();
        if let Some(tree) = tree {
            collect_texts(tree.root(), &mut texts);
        }
        texts
    })
}

fn contains(texts: &[String], needle: &str) -> bool {
    texts.iter().any(|text| text.contains(needle))
}

#[test]
fn clicking_the_logo_opens_and_closes_the_settings_window() {
    let root_key = location_key(file!(), line!(), column!());
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        root_key,
        cranamp::winamp::WinampSurfaceApp,
    );
    shell.set_buffer_size(500, 700);
    shell.set_viewport(500.0, 700.0);
    pump(&mut shell);

    // Settings is closed on launch. "SYNC" is a section heading unique to the
    // open panel, so it is an unambiguous open/closed marker (the player status
    // line can itself read "Settings"/"Settings Closed").
    let before = visible_texts(&mut shell);
    assert!(
        !contains(&before, "SYNC"),
        "settings panel should be closed on launch; visible={before:?}"
    );

    // The logo (MAIN_SKIN_CHOOSER_HIT_AREA = 249,79,26,33) lives inside the
    // inline MainWindow, which is offset by its default inline position (26,22).
    // Click its center.
    let logo_x = 26.0 + 249.0 + 13.0;
    let logo_y = 22.0 + 79.0 + 16.0;
    shell.set_cursor(logo_x, logo_y);
    let pressed = shell.pointer_pressed();
    pump(&mut shell);
    let released = shell.pointer_released();
    pump(&mut shell);
    assert!(
        pressed && released,
        "pointer down/up should hit the logo target at ({logo_x},{logo_y})"
    );

    // The modern Settings panel and its sections are now present.
    let after = visible_texts(&mut shell);
    assert!(
        contains(&after, "Settings"),
        "settings panel header should appear after logo click; visible={after:?}"
    );
    assert!(
        contains(&after, "SKINS"),
        "skins section should appear; visible={after:?}"
    );
    assert!(
        contains(&after, "Bundled"),
        "bundled skin row should be listed; visible={after:?}"
    );
    assert!(
        contains(&after, "SYNC"),
        "sync section should appear; visible={after:?}"
    );
    assert!(
        contains(&after, "UPDATES"),
        "updates section should appear; visible={after:?}"
    );

    // Tapping outside the centered panel hits the dim backdrop, which dismisses
    // the modal. (500x700 surface; the 300x500 panel is centered, so the corner
    // at (20,20) is safely on the backdrop.)
    shell.set_cursor(20.0, 20.0);
    shell.pointer_pressed();
    pump(&mut shell);
    shell.pointer_released();
    pump(&mut shell);

    let after_close = visible_texts(&mut shell);
    assert!(
        !contains(&after_close, "SYNC"),
        "settings panel should close after tapping the backdrop; visible={after_close:?}"
    );
}

/// Regression test for the bug in PLAN.md: playlist shift/ctrl-click
/// multi-select used to read keyboard modifiers via a raw `x11rb` connection.
/// `x11rb::connect` fails on every non-X11 desktop, so on macOS and Windows the
/// click silently saw "no modifiers held" no matter what was actually pressed.
///
/// Modifiers now travel on `PointerEvent` itself -- stamped by
/// `AppShell::set_modifiers`, the same per-shell state every desktop backend's
/// event loop already feeds from its native `ModifiersChanged`/DOM event (see
/// cranpose PR #452) -- so this drives a real click through the real
/// `pointer_pressed`/`pointer_released` dispatch pipeline (no X11, no platform
/// keyboard query of any kind) and asserts `PlaylistRowClickTarget` reports
/// exactly what was set.
#[test]
fn playlist_row_click_reports_shift_and_ctrl_from_the_pointer_event() {
    let captured: Rc<RefCell<Vec<Modifiers>>> = Rc::new(RefCell::new(Vec::new()));
    let captured_for_app = Rc::clone(&captured);

    let root_key = location_key(file!(), line!(), column!());
    let mut shell = AppShell::new(HitGraphRenderer::default(), root_key, move || {
        let captured = Rc::clone(&captured_for_app);
        cranamp::winamp::PlaylistRowClickTarget(0.0, 0.0, 100.0, 20.0, 1.0, 0, move |modifiers| {
            captured.borrow_mut().push(modifiers);
        });
    });
    shell.set_buffer_size(100, 100);
    shell.set_viewport(100.0, 100.0);
    pump(&mut shell);

    // Before any platform ever calls `set_modifiers`, a click must read as
    // "nothing held" -- not silently drop the multi-select gesture.
    shell.set_cursor(10.0, 10.0);
    assert!(shell.pointer_pressed(), "press should hit the row target");
    pump(&mut shell);
    assert!(
        shell.pointer_released(),
        "release should complete the click"
    );
    pump(&mut shell);

    // A shift-held click must reach the handler with shift set.
    shell.set_modifiers(Modifiers {
        shift: true,
        ..Modifiers::NONE
    });
    shell.set_cursor(10.0, 10.0);
    assert!(shell.pointer_pressed());
    pump(&mut shell);
    assert!(shell.pointer_released());
    pump(&mut shell);

    // A ctrl-held click must reach the handler with ctrl set, and the earlier
    // shift must not leak into it.
    shell.set_modifiers(Modifiers {
        ctrl: true,
        ..Modifiers::NONE
    });
    shell.set_cursor(10.0, 10.0);
    assert!(shell.pointer_pressed());
    pump(&mut shell);
    assert!(shell.pointer_released());
    pump(&mut shell);

    let events = captured.borrow();
    assert_eq!(events.len(), 3, "expected exactly three clicks: {events:?}");
    assert_eq!(
        events[0],
        Modifiers::NONE,
        "click before set_modifiers was ever called must report no modifiers"
    );
    assert!(
        events[1].shift && !events[1].ctrl,
        "shift-held click: {:?}",
        events[1]
    );
    assert!(
        events[2].ctrl && !events[2].shift,
        "ctrl-held click: {:?}",
        events[2]
    );
}
