mod catalog;
mod list;
mod live;
use crate::prelude::*;
pub use list::{has_catalog, node_available, node_list_canvas, sync_node_list};
use std::collections::HashMap;
use theframework::thegraph::*;

const VIEW: &str = "Behavior Nodes View";
#[derive(Default)]
struct History {
    undo: Vec<GraphDocument>,
    redo: Vec<GraphDocument>,
}

/// Creator adapter. Graph drawing, input, schemas and storage remain independent.
pub struct NodesDock {
    doc: GraphDocument,
    editor: GraphEditor,
    definitions: GraphDefinitions,
    resources: GraphRasterResources,
    owner: Option<String>,
    histories: HashMap<String, History>,
    committed: GraphDocument,
    dirty: bool,
    live: live::LiveEvents,
    load_error: Option<String>,
    popup: Option<(GraphPicker, Option<Uuid>)>,
}
struct AuthoringContext<'a>(&'a GraphDefinitions);
impl GraphContext for AuthoringContext<'_> {
    fn observe(&self, _: &GraphNode) -> GraphObservation {
        GraphObservation {
            text: fl!("node_authoring"),
            ..Default::default()
        }
    }
    fn node_title(&self, n: &GraphNode) -> Option<String> {
        self.0
            .selected_event(n)
            .and_then(|id| self.0.event(id))
            .map(|e| fl!("node_on_event", event = e.label.clone()))
    }
    fn row_label(&self, n: &GraphNode, r: &GraphRow) -> Option<String> {
        if r.key.as_deref() == Some("event") {
            return self
                .0
                .selected_event(n)
                .and_then(|id| self.0.event(id))
                .map(|e| format!("{} ...", e.label));
        }
        None
    }
}
fn node_editor() -> GraphEditor {
    let mut editor = GraphEditor::default();
    editor.viewport.zoom_at(editor.viewport.pan, 0.9);
    editor
}

impl NodesDock {
    fn owner(pc: ProjectContext) -> Option<String> {
        Some(match pc {
            ProjectContext::CharacterCode(id) | ProjectContext::Character(id) => {
                format!("behavior/character/{id}")
            }
            ProjectContext::RegionCharacterInstance(region, id) => {
                format!("behavior/region/{region}/character/{id}")
            }
            ProjectContext::ItemCode(id) | ProjectContext::Item(id) => {
                format!("behavior/item/{id}")
            }
            ProjectContext::RegionItemInstance(region, id) => {
                format!("behavior/region/{region}/item/{id}")
            }
            ProjectContext::RegionCode(id) => format!("behavior/region/{id}"),
            ProjectContext::WorldCode => "behavior/world".into(),
            _ => return None,
        })
    }
    fn render(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        let Some(view) = ui.get_render_view(VIEW) else {
            return;
        };
        let d = *view.dim();
        if d.width <= 0 || d.height <= 0 {
            return;
        }
        view.set_text_input(self.editor.text_focus().is_some() || self.popup.is_some());
        let buffer = view.render_buffer_mut();
        if buffer.dim().width != d.width || buffer.dim().height != d.height {
            *buffer = TheRGBABuffer::new(TheDim::new(0, 0, d.width, d.height));
        }
        buffer.set_render_scale(ctx.ui_render_scale);
        let (width, height, density) = (
            buffer.pixel_width(),
            buffer.pixel_height(),
            buffer.render_scale(),
        );
        let mut painter = RasterGraphPainter::new(
            buffer.pixels_mut(),
            width,
            height,
            density,
            &mut self.resources,
            &(),
        );
        self.editor.paint(
            &self.doc,
            &live::LiveContext {
                definitions: &self.definitions,
                live: &self.live,
            },
            &BasicGraphControls,
            &mut painter,
            [d.width as f32, d.height as f32],
            &GraphTheme::default(),
        );
        if self.doc.nodes.is_empty() || self.load_error.is_some() {
            let empty = if self.owner.is_some() {
                fl!("node_empty")
            } else {
                fl!("node_select_behavior")
            };
            let label = self.load_error.as_deref().unwrap_or(&empty);
            painter.text(
                GraphRect {
                    origin: [24., 32.],
                    size: [d.width as f32 - 48., 40.],
                },
                label,
                14.,
                [210, 210, 210, 255],
            );
        }
        if let Some((picker, _)) = &mut self.popup {
            picker.fit([d.width as f32, d.height as f32]);
            picker.paint(&mut painter);
        }
        ctx.ui.redraw_all = true;
    }
    fn store(&mut self, project: &mut Project, checkpoint: bool) {
        let Some(owner) = &self.owner else {
            return;
        };
        if self.load_error.is_some() || self.editor.text_focus().is_some() {
            return;
        }
        if !project.node_graphs.contains_key(owner)
            && self.doc == self.committed
            && self.doc.nodes.is_empty()
        {
            return;
        }
        if checkpoint && self.doc != self.committed {
            let h = self.histories.entry(owner.clone()).or_default();
            h.undo.push(self.committed.clone());
            h.redo.clear();
            self.committed = self.doc.clone();
        }
        let value = serde_json::to_value(&self.doc).unwrap();
        if project.node_graphs.get(owner) != Some(&value) {
            project.node_graphs.insert(owner.clone(), value.clone());
            rusterix::server::publish_node_graph(owner.clone(), value);
            self.dirty = true;
        }
    }
    fn finish(&mut self, project: &mut Project) {
        self.editor.finish_text(&mut self.doc, false);
        self.editor.take_edits();
        self.store(project, true);
    }
    fn pick(&mut self, item: GraphPickerItem, target: Option<Uuid>) {
        let position = self
            .editor
            .selected
            .and_then(|id| self.doc.nodes.iter().find(|n| n.id == id))
            .map(|n| [n.position[0] + n.width + 100., n.position[1]])
            .unwrap_or_else(|| self.editor.viewport.to_graph([60., 60.]));
        if let Some(id) = target {
            if let Some(n) = self.doc.nodes.iter_mut().find(|n| n.id == id) {
                if let Some(r) = n
                    .rows
                    .iter_mut()
                    .find(|r| r.key.as_deref() == Some("event"))
                {
                    r.value = GraphControlValue::Custom {
                        kind: "event".into(),
                        data: item.id.into(),
                    };
                }
            }
        } else if let Ok(n) = self.definitions.instantiate(&item.id, position) {
            self.editor.selected = Some(n.id);
            self.doc.nodes.push(n);
        }
        catalog::sync_fields(&mut self.doc, &self.definitions);
    }
}
impl Dock for NodesDock {
    fn new() -> Self {
        let font = fontdue::Font::from_bytes(
            theframework::Embedded::get("fonts/Roboto-Bold.ttf")
                .unwrap()
                .data
                .as_ref(),
            fontdue::FontSettings::default(),
        )
        .unwrap();
        Self {
            doc: GraphDocument::default(),
            committed: GraphDocument::default(),
            editor: node_editor(),
            definitions: catalog::definitions(),
            resources: GraphRasterResources::new(font),
            owner: None,
            histories: HashMap::new(),
            dirty: false,
            live: Default::default(),
            load_error: None,
            popup: None,
        }
    }
    fn setup(&mut self, _: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();
        let mut view = TheRenderView::new(TheId::named(VIEW));
        // A graph canvas needs equal sensitivity on both axes, independent of
        // the dock's aspect ratio. The shared widget keeps its existing defaults.
        view.set_scroll_behavior(-0.6, false);
        view.limiter_mut()
            .set_max_size(Vec2::new(i32::MAX, i32::MAX));
        canvas.set_widget(view);

        canvas
    }
    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server: &mut ServerContext,
    ) {
        let owner = Self::owner(server.pc);
        self.definitions = catalog::definitions_for_project(project);
        self.editor.finish_text(&mut self.doc, false);
        self.editor.take_edits();
        self.popup = None;
        // Edits are persisted on input; keep the pending undo group on reactivation.
        if let Some(key) = &self.owner {
            if self.doc != self.committed {
                let h = self.histories.entry(key.clone()).or_default();
                h.undo.push(self.committed.clone());
                h.redo.clear();
            }
        }
        if self.owner != owner {
            self.owner = owner;
            self.live = Default::default();
            self.editor = node_editor();
            self.popup = None;
        }
        self.load_error = None;
        self.doc = if let Some(value) = self.owner.as_ref().and_then(|k| project.node_graphs.get(k))
        {
            match serde_json::from_value::<GraphDocument>(value.clone()) {
                Ok(doc) if doc.version == 1 => doc,
                _ => {
                    self.load_error = Some(fl!("node_unsupported"));
                    GraphDocument::default()
                }
            }
        } else {
            GraphDocument::default()
        };
        catalog::sync_fields(&mut self.doc, &self.definitions);
        self.committed = self.doc.clone();
        self.render(ui, ctx);
    }
    fn poll_background(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut ServerContext,
    ) -> bool {
        if self.refresh_live(project, server) {
            self.render(ui, ctx);
            true
        } else {
            false
        }
    }
    fn supports_actions(&self) -> bool {
        false
    }
    fn supports_undo(&self) -> bool {
        true
    }
    fn has_changes(&self) -> bool {
        self.dirty
    }
    fn mark_saved(&mut self) {
        self.dirty = false;
    }
    fn reset_for_project_switch(&mut self) {
        self.histories.clear();
        self.live = Default::default();
        self.owner = None;
        self.doc = GraphDocument::default();
        self.committed = self.doc.clone();
        self.editor = node_editor();
        self.popup = None;
        self.dirty = false;
    }
    fn set_undo_state_to_ui(&self, ctx: &mut TheContext) {
        let h = self.owner.as_ref().and_then(|k| self.histories.get(k));
        if h.is_some_and(|h| !h.undo.is_empty()) {
            ctx.ui.set_enabled("Undo");
        } else {
            ctx.ui.set_disabled("Undo");
        }
        if h.is_some_and(|h| !h.redo.is_empty()) {
            ctx.ui.set_enabled("Redo");
        } else {
            ctx.ui.set_disabled("Redo");
        }
    }
    fn undo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _: &mut ServerContext,
    ) {
        self.finish(project);
        if let Some(h) = self.owner.as_ref().and_then(|k| self.histories.get_mut(k)) {
            if let Some(doc) = h.undo.pop() {
                h.redo.push(self.doc.clone());
                self.doc = doc;
                self.committed = self.doc.clone();
                self.editor = node_editor();
                self.store(project, false);
            }
        }
        self.set_undo_state_to_ui(ctx);
        self.render(ui, ctx);
    }
    fn redo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _: &mut ServerContext,
    ) {
        self.finish(project);
        if let Some(h) = self.owner.as_ref().and_then(|k| self.histories.get_mut(k)) {
            if let Some(doc) = h.redo.pop() {
                h.undo.push(self.doc.clone());
                self.doc = doc;
                self.committed = self.doc.clone();
                self.editor = node_editor();
                self.store(project, false);
            }
        }
        self.set_undo_state_to_ui(ctx);
        self.render(ui, ctx);
    }
    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut ServerContext,
    ) -> bool {
        if let TheEvent::WidgetResized(id, _) = event {
            if id.name == VIEW {
                self.render(ui, ctx);
                return true;
            }
        }
        if self.owner.is_none() || self.load_error.is_some() {
            return false;
        }
        let focused = ctx.ui.focus.as_ref().is_some_and(|id| id.name == VIEW);
        let mut checkpoint = false;
        match event {
            TheEvent::RenderViewDrop(id, point, drop) if id.name == VIEW => {
                let Some(key) = drop.id.name.strip_prefix("Node Catalog/") else {
                    return false;
                };
                if key != drop.data || !node_available(server.pc, key) {
                    return false;
                }
                self.finish(project);
                let position = self
                    .editor
                    .viewport
                    .to_graph([point.x as f32, point.y as f32]);
                if let Ok(mut node) = self.definitions.instantiate(key, position) {
                    if key == "event"
                        && !matches!(
                            server.pc,
                            ProjectContext::Character(_)
                                | ProjectContext::CharacterCode(_)
                                | ProjectContext::RegionCharacterInstance(_, _)
                        )
                    {
                        node.rows[0].value = GraphControlValue::Custom {
                            kind: "event".into(),
                            data: "startup".into(),
                        };
                    }
                    self.editor.selected = Some(node.id);
                    self.doc.nodes.push(node);
                    catalog::sync_fields(&mut self.doc, &self.definitions);
                    checkpoint = true;
                }
                if let Some(view) = ui.get_render_view(VIEW) {
                    ctx.ui.set_focus(view.id());
                }
            }
            TheEvent::RenderViewHoverChanged(id, point) if id.name == VIEW => {
                self.editor.cursor = [point.x as f32, point.y as f32];
                if let Some((picker, _)) = &mut self.popup {
                    let changed = picker.hover(Some(self.editor.cursor));
                    if changed {
                        self.render(ui, ctx);
                    }
                    return changed;
                }
                let p = self.editor.viewport.to_graph(self.editor.cursor);
                let help = self
                    .doc
                    .nodes
                    .iter()
                    .rev()
                    .find(|n| n.rect().contains(p))
                    .and_then(|n| n.definition.as_deref())
                    .map(list::node_help)
                    .unwrap_or_default();
                ctx.ui.send(TheEvent::SetStatusText(id.clone(), help));
                return false;
            }
            TheEvent::RenderViewLostHover(id) if id.name == VIEW => {
                if let Some((picker, _)) = &mut self.popup {
                    let changed = picker.hover(None);
                    if changed {
                        self.render(ui, ctx);
                    }
                    return changed;
                }
                ctx.ui
                    .send(TheEvent::SetStatusText(id.clone(), String::new()));
                return false;
            }
            TheEvent::RenderViewClicked(id, p) if id.name == VIEW => {
                let point = [p.x as f32, p.y as f32];
                if let Some((picker, target)) = self.popup.take() {
                    if let Some(item) = picker.pick(point) {
                        self.pick(
                            GraphPickerItem {
                                id: item,
                                label: String::new(),
                            },
                            target,
                        );
                        checkpoint = true;
                    } else if picker.contains(point) {
                        self.popup = Some((picker, target));
                    }
                } else {
                    let graph_point = self.editor.viewport.to_graph(point);
                    let event_node = self
                        .doc
                        .nodes
                        .iter()
                        .rev()
                        .find(|n| {
                            n.definition.as_deref() == Some("event")
                                && n.row_rect(0).contains(graph_point)
                        })
                        .map(|n| n.id);
                    if let Some(id) = event_node {
                        self.finish(project);
                        let row = self
                            .doc
                            .nodes
                            .iter()
                            .find(|n| n.id == id)
                            .unwrap()
                            .row_rect(0);
                        let anchor = GraphRect {
                            origin: self.editor.viewport.to_screen(row.origin),
                            size: [
                                row.size[0] * self.editor.viewport.zoom(),
                                row.size[1] * self.editor.viewport.zoom(),
                            ],
                        };
                        let dim = *ui.get_render_view(VIEW).unwrap().dim();
                        self.popup = Some((
                            GraphPicker::compact(
                                anchor,
                                [dim.width as f32, dim.height as f32],
                                self.definitions
                                    .events()
                                    .map(|e| GraphPickerItem {
                                        id: e.id.clone(),
                                        label: e.label.clone(),
                                    })
                                    .collect(),
                            ),
                            Some(id),
                        ));
                        if let Some((picker, _)) = &mut self.popup {
                            picker.search_label = fl!("node_search");
                            picker.empty_label = fl!("node_no_results");
                        }
                    } else {
                        // Return commits text; clicking away cancels the draft.
                        self.editor.finish_text(&mut self.doc, false);
                        self.editor
                            .pointer_down(&mut self.doc, point, &BasicGraphControls);
                    }
                }
            }
            TheEvent::RenderViewDragged(id, p) if id.name == VIEW && self.popup.is_none() => self
                .editor
                .pointer_move(&mut self.doc, [p.x as f32, p.y as f32], &BasicGraphControls),
            TheEvent::RenderViewUp(id, p) if id.name == VIEW && self.popup.is_none() => {
                self.editor.pointer_up(
                    &mut self.doc,
                    [p.x as f32, p.y as f32],
                    &AllowGraphConnections,
                );
                checkpoint = self.editor.text_focus().is_none();
            }
            TheEvent::RenderViewScrollBy(id, d) | TheEvent::RenderViewPreciseScrollBy(id, d)
                if id.name == VIEW =>
            {
                if let Some((p, _)) = &mut self.popup {
                    if matches!(event, TheEvent::RenderViewPreciseScrollBy(_, _)) {
                        p.scroll_pixels(d.y as f32);
                    } else {
                        p.scroll_by(d.y.signum());
                    }
                } else if ui.ctrl || ui.logo {
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
            TheEvent::KeyDown(TheValue::Char(c)) if focused => {
                if let Some((p, _)) = &mut self.popup {
                    p.type_char(*c);
                } else if (ui.ctrl || ui.logo) && c.eq_ignore_ascii_case(&'a') {
                    self.editor
                        .text_input(&mut self.doc, GraphTextInput::SelectAll);
                } else if !ui.ctrl && !ui.logo {
                    self.editor
                        .text_input(&mut self.doc, GraphTextInput::Insert(c.to_string()));
                }
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(key)) if focused => {
                if let Some((mut p, target)) = self.popup.take() {
                    match key {
                        TheKeyCode::Escape => {}
                        TheKeyCode::Return => {
                            let items = p.filtered();
                            let selected = items
                                .iter()
                                .find(|item| p.hovered.as_deref() == Some(item.id.as_str()))
                                .copied()
                                .or_else(|| items.get(p.scroll).copied());
                            if let Some(item) = selected {
                                self.pick(item.clone(), target);
                                checkpoint = true;
                            }
                        }
                        TheKeyCode::Delete => {
                            p.backspace();
                            self.popup = Some((p, target));
                        }
                        _ => self.popup = Some((p, target)),
                    }
                } else if *key == TheKeyCode::Delete && self.editor.text_focus().is_none() {
                    self.finish(project);
                    if self.editor.selected_connection.is_some() {
                        self.editor.delete_connection(&mut self.doc);
                    } else if let Some(id) = self.editor.selected.take() {
                        let ports: Vec<_> = self
                            .doc
                            .nodes
                            .iter()
                            .filter(|n| n.id == id)
                            .flat_map(|n| n.ports.iter().map(|p| p.id))
                            .collect();
                        self.doc.nodes.retain(|n| n.id != id);
                        self.doc
                            .connections
                            .retain(|c| !ports.contains(&c.from) && !ports.contains(&c.to));
                    }
                    checkpoint = true;
                } else {
                    let input = match key {
                        TheKeyCode::Delete => GraphTextInput::Backspace,
                        TheKeyCode::Return => GraphTextInput::Commit,
                        TheKeyCode::Escape => GraphTextInput::Cancel,
                        TheKeyCode::Left => GraphTextInput::Left { extend: ui.shift },
                        TheKeyCode::Right => GraphTextInput::Right { extend: ui.shift },
                        _ => return false,
                    };
                    self.editor.text_input(&mut self.doc, input);
                    checkpoint = self.editor.text_focus().is_none();
                }
            }
            _ => return false,
        }
        if !self.editor.take_edits().is_empty() {
            checkpoint = true;
        }
        self.store(project, checkpoint);
        self.set_undo_state_to_ui(ctx);
        self.render(ui, ctx);
        true
    }
}

#[cfg(test)]
mod tests;
