//! The prefab authoring pane uses the same canvas, branch list and interaction model as behavior.
use crate::prelude::*;
use shared::prefab_graph::{self, PrefabGraphs};
use std::collections::HashSet;
use theframework::thegraph::*;

pub const VIEW: &str = "Prefab Graph View";
const LIST: &str = "Prefab Graph Branches";
const TIDY: &str = "Prefab Graph Tidy";

pub fn node_available(key: &str) -> bool {
    prefab_graph::definitions()
        .node(key)
        .is_some_and(|n| !n.starts_branch)
}
pub fn sync_node_list(ui: &mut TheUI, ctx: &mut TheContext) {
    let Some(list) = ui.get_list_layout("Node Catalog List") else {
        return;
    };
    list.clear();
    for node in prefab_graph::definitions()
        .nodes()
        .filter(|n| !n.starts_branch)
    {
        let mut item = TheListItem::new(TheId::named(&format!("Node Catalog/{}", node.id)));
        item.set_text(title(&node.id));
        item.set_status_text(&fl!("prefab_nodes_catalog_help"));
        item.set_background_color(TheColor::from_u8_array([
            node.color[0] / 2,
            node.color[1] / 2,
            node.color[2] / 2,
            255,
        ]));
        item.set_text_color([235, 235, 235, 255]);
        list.add_item(item, ctx);
    }
}
fn title(key: &str) -> String {
    match key {
        "prefab_part" => fl!("prefab_node_part"),
        "prefab_geometry" => fl!("prefab_node_geometry"),
        "prefab_transform" => fl!("prefab_node_transform"),
        "prefab_door" => fl!("prefab_node_door"),
        _ => key.into(),
    }
}
struct Controls;
impl GraphControls for Controls {
    fn label(&self, value: &GraphControlValue) -> String {
        if let GraphControlValue::Choice { options, selected } = value {
            match options.get(*selected).map(String::as_str) {
                Some("Swing") => return fl!("prefab_editor_door_motion_swing"),
                Some("Slide") => return fl!("prefab_editor_door_motion_slide"),
                _ => {}
            }
        }
        BasicGraphControls.label(value)
    }
    fn interact(
        &self,
        value: &GraphControlValue,
        input: GraphControlInput,
    ) -> Option<GraphControlValue> {
        BasicGraphControls.interact(value, input)
    }
}
struct Context;
impl GraphContext for Context {
    fn observe(&self, _: &GraphNode) -> GraphObservation {
        GraphObservation::default()
    }
    fn node_title(&self, n: &GraphNode) -> Option<String> {
        n.definition.as_deref().map(title)
    }
    fn port_label(&self, node: &GraphNode, port: &GraphPort) -> Option<String> {
        if node.definition.as_deref() == Some("prefab_part")
            && port.direction == PortDirection::Output
        {
            Some(fl!("prefab_node_geometry"))
        } else {
            None
        }
    }
    fn parameter_label(&self, _: &GraphNode, row: &GraphRow) -> Option<String> {
        Some(match row.key.as_deref()? {
            "name" => fl!("prefab_editor_part_name"),
            "pivot_x" => fl!("prefab_node_pivot_x"),
            "pivot_y" => fl!("prefab_node_pivot_y"),
            "pivot_z" => fl!("prefab_node_pivot_z"),
            "motion" => fl!("prefab_editor_door_motion"),
            "angle_degrees" => fl!("prefab_editor_door_angle"),
            "slide_distance" => fl!("prefab_editor_door_slide_distance"),
            "interaction_range" => fl!("prefab_editor_door_usage_distance"),
            _ => row.label.clone(),
        })
    }
}
pub struct PrefabNodePane {
    asset: Option<Uuid>,
    pub part: Option<Uuid>,
    doc: GraphDocument,
    committed: GraphDocument,
    editor: GraphEditor,
    resources: GraphRasterResources,
    opened: HashSet<(Uuid, Uuid)>,
}
impl PrefabNodePane {
    pub fn new() -> Self {
        let font = fontdue::Font::from_bytes(
            theframework::Embedded::get("fonts/Roboto-Bold.ttf")
                .unwrap()
                .data
                .as_ref(),
            fontdue::FontSettings::default(),
        )
        .unwrap();
        Self {
            asset: None,
            part: None,
            doc: GraphDocument::default(),
            committed: GraphDocument::default(),
            editor: GraphEditor::default(),
            resources: GraphRasterResources::new(font),
            opened: HashSet::new(),
        }
    }
    pub fn canvas(toolbar: TheCanvas) -> TheCanvas {
        let mut canvas = TheCanvas::new();
        let mut branches = TheCanvas::new();
        let mut list = TheListLayout::new(TheId::named(LIST));
        list.limiter_mut().set_min_width(190);
        list.limiter_mut().set_max_width(190);
        branches.set_layout(list);
        let mut view = TheCanvas::new();
        let mut render = TheRenderView::new(TheId::named(VIEW));
        render.set_auto_focus(true);
        view.set_widget(render);
        canvas.set_left(branches);
        canvas.set_center(view);
        canvas.set_top(toolbar);
        canvas
    }
    fn graphs(project: &Project, id: Uuid) -> Option<PrefabGraphs> {
        let asset = project.block_props.get(&id)?;
        let mut graphs = project
            .prefab_graphs
            .get(&id)
            .cloned()
            .unwrap_or_else(|| PrefabGraphs::import(asset));
        graphs.reconcile(asset);
        Some(graphs)
    }
    pub fn load(
        &mut self,
        project: &Project,
        asset: Uuid,
        part: Option<Uuid>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        let Some(graphs) = Self::graphs(project, asset) else {
            return;
        };
        let part = part
            .filter(|id| graphs.branches.iter().any(|b| b.part_id == *id))
            .or_else(|| graphs.branches.first().map(|b| b.part_id));
        let doc = graphs
            .branches
            .iter()
            .find(|b| Some(b.part_id) == part)
            .map(|b| b.graph.clone())
            .unwrap_or_default();
        if self.asset != Some(asset) || self.part != part || self.committed != doc {
            self.asset = Some(asset);
            self.part = part;
            self.doc = doc;
            self.committed = self.doc.clone();
            self.editor = GraphEditor::default();
            if part.is_some_and(|part| self.opened.insert((asset, part))) {
                self.tidy();
            }
            self.fit(ui);
        }
        if let Some(list) = ui.get_list_layout(LIST) {
            list.clear();
            for branch in graphs.branches {
                let mut item =
                    TheListItem::new(TheId::named_with_id("Prefab Branch", branch.part_id));
                let name = project
                    .block_props
                    .get(&asset)
                    .and_then(|a| a.find_part(branch.part_id))
                    .map(|p| p.name.clone())
                    .unwrap_or_default();
                item.set_text(name);
                item.set_status_text(&fl!("prefab_nodes_branch_help"));
                item.set_background_color(TheColor::from_u8_array(
                    if Some(branch.part_id) == part {
                        [64, 96, 120, 255]
                    } else {
                        [40, 42, 44, 255]
                    },
                ));
                list.add_item(item, ctx);
            }
        }
        if let Some(list) = ui.get_list_layout(LIST) {
            if let Some(asset) = project.block_props.get(&asset) {
                for surface in &asset.support_surfaces {
                    let mut item = TheListItem::new(TheId::named_with_id(
                        "Prefab Editor Support Surface",
                        surface.id,
                    ));
                    item.set_text(surface.name.clone());
                    item.set_status_text(&fl!("status_prefab_editor_edit_support_surface"));
                    list.add_item(item, ctx);
                }
            }
        }
        sync_node_list(ui, ctx);
        self.render(ui, ctx);
    }
    fn tidy(&mut self) {
        let nodes = self.doc.nodes.iter().map(|n| n.id).collect();
        layout_branch(&mut self.doc, &nodes);
    }
    fn fit(&mut self, ui: &mut TheUI) {
        if let Some(view) = ui.get_render_view(VIEW) {
            let d = *view.dim();
            let nodes = self.doc.nodes.iter().map(|n| n.id).collect();
            self.editor
                .viewport
                .fit_to_nodes(&self.doc, [d.width as f32, d.height as f32], &nodes);
        }
    }
    fn render(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        let Some(view) = ui.get_render_view(VIEW) else {
            return;
        };
        let d = *view.dim();
        if d.width <= 0 || d.height <= 0 {
            return;
        }
        view.set_text_input(self.editor.text_focus().is_some());
        let buffer = view.render_buffer_mut();
        if buffer.dim().width != d.width || buffer.dim().height != d.height {
            *buffer = TheRGBABuffer::new(TheDim::new(0, 0, d.width, d.height));
        }
        buffer.set_render_scale(ctx.ui_render_scale);
        let (w, h, s) = (
            buffer.pixel_width(),
            buffer.pixel_height(),
            buffer.render_scale(),
        );
        let mut painter =
            RasterGraphPainter::new(buffer.pixels_mut(), w, h, s, &mut self.resources, &());
        self.editor.paint(
            &self.doc,
            &Context,
            &Controls,
            &mut painter,
            [d.width as f32, d.height as f32],
            &GraphTheme::default(),
        );
        ctx.ui.redraw_all = true;
    }
    fn store(&mut self, project: &mut Project, ctx: &mut TheContext) {
        if self.doc == self.committed {
            return;
        }
        let (Some(asset_id), Some(part)) = (self.asset, self.part) else {
            return;
        };
        let Some(mut graphs) = Self::graphs(project, asset_id) else {
            return;
        };
        let Some(branch) = graphs.branches.iter_mut().find(|b| b.part_id == part) else {
            return;
        };
        branch.graph = self.doc.clone();
        let Some(base) = project.block_props.get(&asset_id) else {
            return;
        };
        match graphs.compile(base) {
            Ok(asset) => {
                for component in &asset.components {
                    if component.kind == "Door"
                        && !graphs
                            .door_components
                            .iter()
                            .any(|seed| seed.id == component.id)
                    {
                        graphs.door_components.push(component.clone());
                    }
                }
                for target in &asset.interaction_targets {
                    if target
                        .component_id
                        .is_some_and(|id| graphs.door_components.iter().any(|seed| seed.id == id))
                        && !graphs.door_targets.iter().any(|seed| seed.id == target.id)
                    {
                        graphs.door_targets.push(target.clone());
                    }
                }
                let before = project.clone();
                project.prefab_graphs.insert(asset_id, graphs);
                project.block_props.insert(asset_id, asset);
                super::prefabs_editor::PrefabsEditorDock::push_project_undo(before, project, ctx);
                super::prefabs_editor::PrefabsEditorDock::sync_prefab_runtime(project);
                self.committed = self.doc.clone();
                ctx.ui.send(TheEvent::Custom(
                    TheId::named(super::blocks::BLOCKS_DOCK_SYNC_EVENT),
                    TheValue::Empty,
                ));
            }
            Err(message) => {
                ctx.ui
                    .send(TheEvent::SetStatusText(TheId::empty(), message));
            }
        }
    }
    pub fn handle(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
    ) -> bool {
        match event {
            TheEvent::WidgetResized(id, _) if id.name == VIEW => {
                self.fit(ui);
            }
            TheEvent::NewListItemSelected(id, _) if id.name == "Prefab Branch" => {
                self.editor.finish_text(&mut self.doc, true);
                self.store(project, ctx);
                if let Some(asset) = self.asset {
                    self.load(project, asset, Some(id.uuid), ui, ctx);
                }
                return true;
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) if id.name == TIDY => {
                self.tidy();
                self.fit(ui);
                self.store(project, ctx);
            }
            TheEvent::RenderViewDrop(id, point, drop) if id.name == VIEW => {
                let Some(key) = drop.id.name.strip_prefix("Node Catalog/") else {
                    return false;
                };
                if !node_available(key) {
                    return false;
                }
                self.editor.finish_text(&mut self.doc, true);
                let node = prefab_graph::definitions().node(key).unwrap().instantiate(
                    self.editor
                        .viewport
                        .to_graph([point.x as f32, point.y as f32]),
                );
                self.doc.nodes.push(node);
                self.store(project, ctx);
            }
            TheEvent::RenderViewClicked(id, p) if id.name == VIEW => {
                if let Some(view) = ui.get_render_view(VIEW) {
                    ctx.ui.set_focus(view.id());
                }
                if ui.shift {
                    self.editor
                        .begin_cut(&mut self.doc, [p.x as f32, p.y as f32]);
                } else {
                    self.editor
                        .pointer_down(&mut self.doc, [p.x as f32, p.y as f32], &Controls);
                }
            }
            TheEvent::RenderViewDragged(id, p) if id.name == VIEW => {
                self.editor
                    .pointer_move(&mut self.doc, [p.x as f32, p.y as f32], &Controls)
            }
            TheEvent::RenderViewUp(id, p) if id.name == VIEW => {
                self.editor.pointer_up(
                    &mut self.doc,
                    [p.x as f32, p.y as f32],
                    &AllowGraphConnections,
                );
                self.store(project, ctx);
            }
            TheEvent::RenderViewScrollBy(id, d) | TheEvent::RenderViewPreciseScrollBy(id, d)
                if id.name == VIEW =>
            {
                if ui.ctrl || ui.logo {
                    self.editor
                        .viewport
                        .zoom_at(self.editor.cursor, (d.y as f32 * 0.01).exp());
                } else {
                    self.editor.viewport.pan[0] -= d.x as f32;
                    self.editor.viewport.pan[1] -= d.y as f32;
                }
            }
            TheEvent::RenderViewZoomBy(id, d) if id.name == VIEW => self
                .editor
                .viewport
                .zoom_at(self.editor.cursor, (1. + *d).max(0.1)),
            TheEvent::RenderViewHoverChanged(id, p) if id.name == VIEW => {
                self.editor.cursor = [p.x as f32, p.y as f32];
            }
            TheEvent::KeyDown(TheValue::Char(ch))
                if ctx.ui.focus.as_ref().is_some_and(|id| id.name == VIEW) =>
            {
                self.editor
                    .text_input(&mut self.doc, GraphTextInput::Insert(ch.to_string()));
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(key))
                if ctx.ui.focus.as_ref().is_some_and(|id| id.name == VIEW) =>
            {
                match key {
                    TheKeyCode::Return => {
                        self.editor.finish_text(&mut self.doc, true);
                        self.store(project, ctx);
                    }
                    TheKeyCode::Escape => {
                        self.editor.finish_text(&mut self.doc, false);
                    }
                    TheKeyCode::Delete if self.editor.text_focus().is_some() => {
                        self.editor
                            .text_input(&mut self.doc, GraphTextInput::Backspace);
                    }
                    TheKeyCode::Delete => {
                        self.editor.delete_connection(&mut self.doc);
                        if let Some(selected) = self
                            .editor
                            .selected
                            .take()
                            .filter(|id| Some(*id) != self.part)
                        {
                            self.doc.nodes.retain(|n| n.id != selected);
                        }
                        let ports: HashSet<_> = self
                            .doc
                            .nodes
                            .iter()
                            .flat_map(|n| n.ports.iter().map(|p| p.id))
                            .collect();
                        self.doc
                            .connections
                            .retain(|c| ports.contains(&c.from) && ports.contains(&c.to));
                        self.store(project, ctx);
                    }
                    _ => {}
                }
            }
            _ => return false,
        }
        self.render(ui, ctx);
        true
    }
}
