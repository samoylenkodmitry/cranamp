// Shared by every end-to-end test; each one uses the part it needs.
#![allow(dead_code)]
use cranpose_app_shell::AppShell;
use cranpose_render_common::graph::ProjectiveTransform;
use cranpose_render_common::graph_scene::Scene;
use cranpose_render_common::hit_graph::collect_hits_from_graph;
use cranpose_render_common::{RenderScene, Renderer};
use cranpose_ui::{LayoutTree, Size};
#[derive(Default)]
pub struct HitGraphRenderer {
    scene: Scene,
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
        collect_hits_from_graph(
            &graph.root,
            ProjectiveTransform::identity(),
            &mut self.scene,
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
            collect_hits_from_graph(
                &graph.root,
                ProjectiveTransform::identity(),
                &mut self.scene,
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
pub fn visible_texts(shell: &mut AppShell<HitGraphRenderer>) -> Vec<String> {
    fn collect(node: &cranpose_ui::SemanticsNode, out: &mut Vec<String>) {
        if let cranpose_ui::SemanticsRole::Text { value } = &node.role {
            out.push(value.as_str().to_owned());
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
