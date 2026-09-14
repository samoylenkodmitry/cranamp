use cranpose_app_shell::AppShell;
use cranpose_foundation::PointerEvent;
use cranpose_render_common::graph::ProjectiveTransform;
use cranpose_render_common::graph_scene::{ClickAction, HitGeometry, Scene};
use cranpose_render_common::hit_graph::{collect_hits_from_graph, HitGraphSink};
use cranpose_render_common::{RenderScene, Renderer};
use cranpose_ui::{LayoutTree, Size};
use cranpose_ui_graphics::{Point, RoundedCornerShape};
use std::rc::Rc;
#[derive(Default)]
pub struct HitGraphRenderer {
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
            capture_path,
            geometry,
            shape,
            click_actions.iter().cloned().map(ClickAction::WithPoint),
            pointer_inputs,
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

pub fn pump(shell: &mut AppShell<HitGraphRenderer>) {
    for _ in 0..80 {
        if !(shell.needs_redraw() || shell.has_active_animations()) {
            break;
        }
        shell.update();
    }
}

// Native pixel labels are images with text descriptions. Read the semantic
// tree so the same checks cover those labels and Unicode/system text.
pub fn visible_texts(shell: &mut AppShell<HitGraphRenderer>) -> Vec<String> {
    fn collect(node: &cranpose_ui::SemanticsNode, out: &mut Vec<String>) {
        if let cranpose_ui::SemanticsRole::Text { value } = &node.role {
            out.push(value.clone());
        } else if let Some(description) = &node.description {
            out.push(description.clone());
        }
        for child in &node.children {
            collect(child, out);
        }
    }
    shell.set_semantics_enabled(true);
    let mut texts = Vec::new();
    if let Some(tree) = shell.semantics_tree() {
        collect(tree.root(), &mut texts);
    }
    texts
}
