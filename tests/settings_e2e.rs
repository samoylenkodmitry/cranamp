//! Framework-level end-to-end test for the Settings window, driven through
//! cranpose's real hit-dispatch pipeline (no device/adb). It renders the actual
//! `WinampSurfaceApp`, synthesizes a pointer click on the logo, and asserts the
//! Settings panel opens — exercising the same `ClickTarget` → handler path the
//! user taps on a device.
//!
//! Uses a `HitGraphRenderer` that builds a real hit graph from the layout tree
//! (via the published `cranpose-render-common`) so `pointer_pressed`/
//! `pointer_released` dispatch to live click handlers.

#![cfg(not(target_arch = "wasm32"))]

use cranpose_app_shell::AppShell;
use cranpose_core::location_key;
use cranpose_foundation::PointerEvent;
use cranpose_render_common::graph::ProjectiveTransform;
use cranpose_render_common::graph_scene::{ClickAction, HitGeometry, Scene};
use cranpose_render_common::hit_graph::{collect_hits_from_graph, HitGraphSink};
use cranpose_render_common::{RenderScene, Renderer};
use cranpose_ui::{LayoutTree, Size};
use cranpose_ui_graphics::{Point, RoundedCornerShape};
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
