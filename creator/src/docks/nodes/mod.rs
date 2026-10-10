mod catalog;
mod entity;
mod list;
mod live;
mod overlay;
mod rules;
mod tiles;
mod workspace;
use crate::editor::RUSTERIX;
use crate::prelude::*;
pub use list::{
    branch_list_canvas, has_catalog, node_available, node_list_canvas, sync_branch_list,
    sync_node_list,
};
mod conversation;
use conversation::{Click as ConversationClick, ConversationEditor, Field as ConversationField};
use overlay::{TextOverlay, TextTarget};
use rusterix::server::nodes::Conversation;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use theframework::thegraph::*;

/// Editing rules does not initialize a renderer in headless tooling.
pub(crate) fn sync_live_rules(project: &Project) -> Result<(), String> {
    if let Some(runtime) = std::sync::LazyLock::get(&RUSTERIX) {
        shared::rusterix_utils::sync_live_rules(&mut runtime.write().unwrap(), project)?;
    } else {
        project.rules_source()?;
    }
    Ok(())
}

const VIEW: &str = "Behavior Nodes View";
/// Offset applied to a pasted graph so it is visible next to its original.
const PASTE_OFFSET: f32 = 40.;
const CLEAR_NODES: &str = "Node Graph Clear";
const COPY_BRANCH: &str = "Node Graph Copy Branch";
const PASTE_BRANCH: &str = "Node Graph Paste Branch";
const BRANCH_REMOVE: &str = "Node Graph Branch Remove";
const BRANCH_TOGGLE: &str = "Node Graph Branch Toggle";
const BRANCH_TIDY: &str = "Node Graph Branch Tidy";
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
    tiles: tiles::TileControls,
    owner: Option<String>,
    histories: HashMap<String, History>,
    committed: GraphDocument,
    dirty: bool,
    live: live::LiveEvents,
    load_error: Option<String>,
    popup: Option<(GraphPicker, Option<Uuid>)>,
    choice_target: Option<TextTarget>,
    /// Graph copied with the toolbar, ready to paste into another graph.
    clipboard: Option<GraphDocument>,
    /// Multi-line editor open over the graph for one long value.
    text_overlay: Option<TextOverlay>,
    /// Last click, for detecting a double click that expands a text value.
    last_click: Option<(Instant, [f32; 2])>,
    /// Open conversation editor for a Talk node.
    conversation: Option<ConversationEditor>,
    /// Which conversation field the text overlay is editing.
    conversation_field: Option<ConversationField>,
    /// Which branch the canvas shows. Branches are separate, so exactly one is
    /// shown; `None` only while a graph has no trigger at all.
    active_branch: Option<GraphId>,
    branch_graphs: HashMap<GraphId, GraphDocument>,
    branch_order: Vec<GraphId>,
    /// The branch list has to be rebuilt after the graph changed.
    branches_dirty: bool,
    initial_layouts: HashMap<(String, GraphId), (GraphDocument, GraphDocument)>,
    rules_error: Option<String>,
    branch_filter: String,
    particle_preview_started: Instant,
    particle_preview_updated: Option<Instant>,
}
/// Appends a copied graph to a document. Node, row, port and connection ids are
/// regenerated, connections follow their ports, and the pasted copy is nudged so
/// it does not land exactly on its original.
fn paste_graph(document: &mut GraphDocument, clipboard: &GraphDocument) -> HashMap<Uuid, Uuid> {
    let mut moved: HashMap<Uuid, Uuid> = HashMap::new();
    for source in &clipboard.nodes {
        let mut node = source.clone();
        let id = Uuid::new_v4();
        moved.insert(node.id, id);
        node.id = id;
        for row in &mut node.rows {
            let row_id = Uuid::new_v4();
            moved.insert(row.id, row_id);
            row.id = row_id;
            if let GraphControlValue::List { row_ids, .. } = &mut row.value {
                for item in row_ids {
                    let fresh = Uuid::new_v4();
                    moved.insert(*item, fresh);
                    *item = fresh;
                }
            }
        }
        for port in &mut node.ports {
            let port_id = Uuid::new_v4();
            moved.insert(port.id, port_id);
            port.id = port_id;
            port.row = port.row.and_then(|id| moved.get(&id).copied());
            port.list_item = port.list_item.and_then(|id| moved.get(&id).copied());
            if let Some(item) = port.list_item {
                port.key = Some(format!("field:{item}"));
            }
        }
        node.position[0] += PASTE_OFFSET;
        node.position[1] += PASTE_OFFSET;
        document.nodes.push(node);
    }
    for node in document
        .nodes
        .iter_mut()
        .filter(|n| moved.values().any(|id| *id == n.id))
    {
        node.branch = node.branch.and_then(|owner| moved.get(&owner).copied());
    }
    for source in &clipboard.connections {
        document.connections.push(GraphConnection {
            id: Uuid::new_v4(),
            from: moved.get(&source.from).copied().unwrap_or(source.from),
            to: moved.get(&source.to).copied().unwrap_or(source.to),
        });
    }
    moved
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
        if n.definition
            .as_deref()
            .is_some_and(|id| id.starts_with("entity"))
        {
            if let GraphControlValue::Choice { options, selected } = &r.value {
                if options.get(*selected).is_some_and(|s| s.is_empty()) {
                    return Some(fl!("entity_inherit"));
                }
            }
        }
        if r.key.as_deref() == Some("event") {
            return self
                .0
                .selected_event(n)
                .and_then(|id| self.0.event(id))
                .map(|e| format!("{} ...", e.label));
        }
        // A conversation row shows its size, so a folded Talk node still reads.
        if r.key.as_deref() == Some("conversation")
            && let GraphControlValue::Custom { data, .. } = &r.value
            && let Ok(conversation) = serde_json::from_value::<Conversation>(data.clone())
        {
            return Some(conversation.describe());
        }
        None
    }
}
fn node_editor() -> GraphEditor {
    let mut editor = GraphEditor::default();
    editor.viewport.zoom_at(editor.viewport.pan, 0.9);
    editor
}
struct NodeConnectionPolicy;
impl GraphConnectionPolicy for NodeConnectionPolicy {
    fn validate(&self, doc: &GraphDocument, from: GraphId, to: GraphId) -> Result<(), String> {
        let (source, _) = doc.port(from).ok_or_else(|| fl!("node_guard_connection"))?;
        let (_, target) = doc.port(to).ok_or_else(|| fl!("node_guard_connection"))?;
        if doc
            .port(from)
            .is_some_and(|(_, port)| port.kind == "rules-data")
        {
            return shared::rulesets::graph::validate_connection(doc, from, to);
        }
        let is_guard = matches!(
            source.definition.as_deref(),
            Some("quest_guard" | "item_guard" | "player_attribute_guard")
        );
        let is_guard_input = target
            .key
            .as_deref()
            .is_some_and(|key| key.starts_with("when:"));
        if doc
            .port(from)
            .is_some_and(|(_, p)| p.kind == "entity-config")
        {
            return Ok(());
        }
        if is_guard != is_guard_input {
            return Err(fl!("node_guard_connection"));
        }
        Ok(())
    }
}

impl NodesDock {
    fn owner(pc: ProjectContext) -> Option<String> {
        Some(match pc {
            ProjectContext::GameRules => shared::rulesets::graph::OWNER.into(),
            ProjectContext::CharacterData(id) => shared::entity_graph::character_key(id),
            ProjectContext::ItemData(id) => shared::entity_graph::item_key(id),
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
            ProjectContext::RegionArea(region, area) => {
                format!("behavior/region/{region}/area/{area}")
            }
            ProjectContext::RegionCode(id) => format!("behavior/region/{id}"),
            ProjectContext::WorldCode => "behavior/world".into(),
            _ => return None,
        })
    }
    fn render(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.apply_branch_filter();
        self.update_particle_preview();
        let Some(view) = ui.get_render_view(VIEW) else {
            return;
        };
        let d = *view.dim();
        if d.width <= 0 || d.height <= 0 {
            return;
        }
        view.set_text_input(
            self.editor.text_focus().is_some()
                || self.popup.is_some()
                || self.text_overlay.is_some(),
        );
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
        let configuration = self.is_configuration();
        let rules_error = self.rules_error.clone();
        let mut painter = RasterGraphPainter::new(
            buffer.pixels_mut(),
            width,
            height,
            density,
            &mut self.resources,
            &self.tiles,
        );
        self.editor.paint(
            &self.doc,
            &live::LiveContext {
                definitions: &self.definitions,
                live: &self.live,
            },
            &self.tiles,
            &mut painter,
            [d.width as f32, d.height as f32],
            &GraphTheme::default(),
        );
        if self.doc.nodes.is_empty() || self.load_error.is_some() {
            let empty = if configuration {
                fl!("entity_empty")
            } else if self.owner.is_some() {
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
        if let Some(error) = rules_error {
            painter.text(
                GraphRect {
                    origin: [16., 8.],
                    size: [d.width as f32 - 32., 24.],
                },
                &error.lines().next().unwrap_or("Invalid rules").to_string(),
                12.,
                [255, 150, 120, 255],
            );
        }
        if let Some((picker, _)) = &mut self.popup {
            picker.fit([d.width as f32, d.height as f32]);
            picker.paint(&mut painter);
        }
        if let Some(editor) = &mut self.conversation {
            editor.paint(&mut painter, &GraphTheme::default());
        }
        if let Some(overlay) = &mut self.text_overlay {
            let view = [d.width as f32, d.height as f32];
            overlay.rect = match self.conversation.as_ref().zip(self.conversation_field) {
                Some((editor, field)) => editor.editor_rect(field, view),
                None => GraphRect {
                    origin: [d.width as f32 * 0.15, d.height as f32 * 0.12],
                    size: [d.width as f32 * 0.7, d.height as f32 * 0.76],
                },
            };
            overlay.paint(&mut painter, &GraphTheme::default());
            painter.text(
                GraphRect {
                    origin: [
                        overlay.rect.origin[0] + 14.,
                        overlay.rect.origin[1] + overlay.rect.size[1] - 22.,
                    ],
                    size: [overlay.rect.size[0] - 28., 18.],
                },
                &fl!("node_text_overlay_hint"),
                12.,
                [163, 169, 166, 255],
            );
        }
        if self.branches_dirty {
            self.sync_branches(ui, ctx);
        }
        ctx.ui.redraw_all = true;
    }
    fn store(&mut self, project: &mut Project, checkpoint: bool) {
        let Some(owner) = self.owner.clone() else {
            return;
        };
        if self.load_error.is_some() || self.editor.text_focus().is_some() {
            return;
        }
        if !project.node_graphs.contains_key(&owner)
            && !self.is_rules()
            && self.doc == self.committed
            && self.doc.nodes.is_empty()
        {
            return;
        }
        self.refresh_configuration(project);
        if checkpoint {
            catalog::sync_fields(&mut self.doc, &self.definitions);
        }
        if checkpoint && self.doc != self.committed {
            let key = self.history_key().unwrap();
            let h = self.histories.entry(key).or_default();
            h.undo.push(self.committed.clone());
            h.redo.clear();
            self.committed = self.doc.clone();
        }
        if self.is_rules() {
            let graph = shared::rulesets::graph::RulesGraph {
                version: 1,
                branches: self.branch_documents(),
            };
            if project.rules.current != graph {
                project.rules.current = graph;
                self.dirty = true;
                self.branches_dirty = true;
                self.rules_error = sync_live_rules(project).err();
                self.tiles.refresh(project);
            }
            return;
        }
        let value = self.branch_value();
        if project.node_graphs.get(&owner) != Some(&value) {
            project.node_graphs.insert(owner.clone(), value.clone());
            if owner.starts_with(shared::entity_graph::PREFIX) {
                let _ = shared::entity_graph::update_owner(project, &owner, &self.doc);
            } else {
                rusterix::server::publish_node_graph(owner.clone(), value);
            }
            self.dirty = true;
            self.branches_dirty = true;
        }
    }

    fn finish(&mut self, project: &mut Project) {
        self.editor.finish_text(&mut self.doc, false);
        self.editor.take_edits();
        // Another action while an editor is open commits its draft.
        self.close_text_overlay(true);
        if let Some(editor) = self.conversation.take() {
            self.write_conversation(editor);
        }
        self.conversation_field = None;
        self.store(project, true);
    }
    /// Read a text value the overlay targets, whether a row or a list cell.
    fn value_at(doc: &GraphDocument, target: &TextTarget) -> Option<String> {
        let row = doc
            .nodes
            .iter()
            .find(|n| n.id == target.node)?
            .rows
            .iter()
            .find(|r| r.id == target.row)?;
        match (&row.value, target.cell) {
            (GraphControlValue::Text(value), None) => Some(value.clone()),
            (GraphControlValue::List { rows, .. }, Some((r, c))) => {
                match rows.get(r).and_then(|cells| cells.get(c)) {
                    Some(GraphControlValue::Text(value)) => Some(value.clone()),
                    _ => None,
                }
            }
            _ => None,
        }
    }
    fn write_value(doc: &mut GraphDocument, target: &TextTarget, text: String) {
        let Some(row) = doc
            .nodes
            .iter_mut()
            .find(|n| n.id == target.node)
            .and_then(|n| n.rows.iter_mut().find(|r| r.id == target.row))
        else {
            return;
        };
        match (&mut row.value, target.cell) {
            (GraphControlValue::Text(value), None) => *value = text,
            (GraphControlValue::List { rows, .. }, Some((r, c))) => {
                if let Some(GraphControlValue::Text(value)) =
                    rows.get_mut(r).and_then(|cells| cells.get_mut(c))
                {
                    *value = text;
                }
            }
            _ => {}
        }
    }
    /// Open the multi-line editor for the text value under a graph point.
    fn open_text_overlay(&mut self, graph_point: [f32; 2], view: [f32; 2]) -> bool {
        // The editor's own hit testing already knows rows from list cells.
        self.editor.finish_text(&mut self.doc, false);
        let screen = self.editor.viewport.to_screen(graph_point);
        self.editor
            .pointer_down(&mut self.doc, screen, &BasicGraphControls);
        let Some(focus) = self.editor.text_focus().cloned() else {
            return false;
        };
        let target = TextTarget {
            node: focus.node,
            row: focus.row,
            cell: focus.cell,
        };
        let Some(value) = Self::value_at(&self.doc, &target) else {
            self.editor.finish_text(&mut self.doc, false);
            return false;
        };
        self.editor.finish_text(&mut self.doc, false);
        let rect = GraphRect {
            origin: [view[0] * 0.15, view[1] * 0.12],
            size: [view[0] * 0.7, view[1] * 0.76],
        };
        self.text_overlay = Some(TextOverlay::new(target, value, rect));
        true
    }
    /// Close the overlay. Returns true when the document changed.
    fn close_text_overlay(&mut self, commit: bool) -> bool {
        let Some(overlay) = self.text_overlay.take() else {
            return false;
        };
        let Some(field) = self.conversation_field.take() else {
            if commit && overlay.changed() {
                Self::write_value(&mut self.doc, &overlay.target, overlay.text().to_string());
                catalog::sync_fields(&mut self.doc, &self.definitions);
                return true;
            }
            return false;
        };
        // A conversation field belongs to the panel, not to a node row.
        if commit
            && overlay.changed()
            && let Some(editor) = self.conversation.as_mut()
        {
            editor.set_text(field, overlay.text().to_string());
            return true;
        }
        false
    }
    /// Human label for a branch root, such as `On Event: claim_sigil`.
    fn branch_label(&self, root: GraphId) -> String {
        let graph = if self.active_branch == Some(root) {
            &self.doc
        } else {
            self.branch_graphs.get(&root).unwrap_or(&self.doc)
        };
        let Some(node) = graph.nodes.iter().find(|n| n.id == root) else {
            return fl!("node_branch_empty");
        };
        if self.is_rules() {
            return node
                .rows
                .iter()
                .find(|r| r.key.as_deref() == Some("path"))
                .and_then(|r| {
                    if let GraphControlValue::Text(path) = &r.value {
                        Some(path.trim_start_matches('/').replace('/', " / "))
                    } else {
                        None
                    }
                })
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "New definition".into());
        }
        let kind = node
            .definition
            .as_deref()
            .and_then(|id| self.definitions.node(id))
            .map(|definition| definition.title.clone())
            .unwrap_or_default();
        for key in ["event", "area"] {
            let Some(row) = node.rows.iter().find(|r| r.key.as_deref() == Some(key)) else {
                continue;
            };
            let value = match &row.value {
                GraphControlValue::Custom { data, .. } => {
                    data.as_str().map(str::to_string).unwrap_or_default()
                }
                GraphControlValue::Text(text) if !text.trim().is_empty() => text.clone(),
                _ => continue,
            };
            if value.trim().is_empty() {
                continue;
            }
            let value = if key == "event" {
                self.definitions
                    .events()
                    .find(|event| event.id == value)
                    .map(|event| event.label.clone())
                    .unwrap_or(value)
            } else {
                value
            };
            return format!("{kind}: {value}");
        }
        kind
    }
    /// Rows for the branch list: `(item id, label, active)`. Branches are
    /// separate, so there is no combined entry.
    fn branch_items(&self) -> Vec<(String, String, bool)> {
        if !self.is_configuration() {
            let mut roots = self.branch_order.clone();
            if let Some(root) = self.active_branch {
                if !roots.contains(&root) {
                    roots.push(root);
                }
            }
            return roots
                .into_iter()
                .filter(|root| {
                    if self.active_branch == Some(*root) {
                        !self.doc.nodes.is_empty()
                    } else {
                        self.branch_graphs
                            .get(root)
                            .is_some_and(|g| !g.nodes.is_empty())
                    }
                })
                .map(|root| {
                    let graph = if self.active_branch == Some(root) {
                        Some(&self.doc)
                    } else {
                        self.branch_graphs.get(&root)
                    };
                    let disabled =
                        graph.is_some_and(|g| g.nodes.iter().any(|n| n.id == root && n.disabled));
                    let label = self.branch_label(root);
                    (
                        format!("Branch/{root}"),
                        if disabled {
                            format!("{label} ({})", fl!("node_branch_disabled"))
                        } else {
                            label
                        },
                        self.active_branch == Some(root),
                    )
                })
                .collect();
        }
        let branches = graph_branches(&self.doc, &self.definitions);
        branches
            .branches
            .iter()
            .map(|branch| {
                (
                    format!("Branch/{}", branch.root),
                    if self
                        .doc
                        .nodes
                        .iter()
                        .any(|n| n.id == branch.root && n.disabled)
                    {
                        format!(
                            "{} ({})",
                            self.branch_label(branch.root),
                            fl!("node_branch_disabled")
                        )
                    } else {
                        self.branch_label(branch.root)
                    },
                    self.active_branch == Some(branch.root),
                )
            })
            .collect()
    }
    /// The branch to show and its node set: the branch plus any unwired nodes,
    /// which stay reachable so they can be wired into it. Falls back to the first
    /// branch when the active one is gone, and to everything when a graph has no
    /// trigger yet.
    fn shown_branch(&self) -> (Option<GraphId>, Option<std::collections::HashSet<GraphId>>) {
        if !self.is_configuration() {
            return (
                self.active_branch,
                Some(self.doc.nodes.iter().map(|n| n.id).collect()),
            );
        }
        let branches = graph_branches(&self.doc, &self.definitions);
        if branches.branches.is_empty() {
            return (None, None);
        }
        let root = self
            .active_branch
            .filter(|root| branches.branch(*root).is_some())
            .or_else(|| branches.branches.first().map(|branch| branch.root));
        let nodes = root
            .and_then(|root| branches.branch(root))
            .map(|branch| branch.nodes.clone());
        (root, nodes)
    }
    /// Show one branch, and nothing else.
    fn set_branch(&mut self, root: Option<GraphId>) {
        if !self.is_configuration() {
            if let Some(root) = root {
                self.open_branch_document(root);
            }
        }
        self.active_branch = root;
        if self.is_rules() {
            // Normalize presentation before taking an undo baseline. Otherwise
            // replaying an imported branch can look like a new edit and erase redo.
            catalog::sync_fields(&mut self.doc, &self.definitions);
        }
        self.apply_branch_filter();
        self.tidy_branch_once();
        self.committed = self.doc.clone();
    }
    fn apply_branch_filter(&mut self) {
        if !self.is_configuration() {
            if self.active_branch.is_none() {
                self.active_branch = self
                    .doc
                    .nodes
                    .iter()
                    .find(|n| is_branch_trigger(n, &self.definitions))
                    .map(|n| n.id);
            }
            for node in &mut self.doc.nodes {
                node.branch = self.active_branch;
            }
            self.editor
                .set_visible(Some(self.doc.nodes.iter().map(|n| n.id).collect()));
            return;
        }
        assign_branch_owners(&mut self.doc, &self.definitions, self.active_branch);
        let (root, nodes) = self.shown_branch();
        self.active_branch = root;
        self.editor.set_visible(nodes);
    }
    /// Centre the shown branch in the canvas.
    fn fit_branch(&mut self, ui: &mut TheUI) {
        let (_, Some(nodes)) = self.shown_branch() else {
            return;
        };
        let Some(view) = ui.get_render_view(VIEW) else {
            return;
        };
        let d = *view.dim();
        self.editor
            .viewport
            .fit_to_nodes(&self.doc, [d.width as f32, d.height as f32], &nodes);
    }
    /// Arrange each branch once, retaining that initial view across owner reloads.
    fn tidy_branch_once(&mut self) {
        let (Some(owner), Some(root)) = (self.owner.clone(), self.active_branch) else {
            return;
        };
        let key = (owner, root);
        if let Some((original, arranged)) = self.initial_layouts.get(&key) {
            if self.doc == *original {
                self.doc = arranged.clone();
            }
        } else {
            let original = self.doc.clone();
            self.tidy_branch();
            self.initial_layouts
                .insert(key, (original, self.doc.clone()));
        }
    }
    /// Lay the shown branch out left to right. Returns true when it moved.
    fn tidy_branch(&mut self) -> bool {
        let Some(nodes) = self.shown_branch().1 else {
            return false;
        };
        layout_branch(&mut self.doc, &nodes)
    }
    fn sync_branches(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.branches_dirty = false;
        let items = self.branch_items();
        let items: Vec<_> = items
            .into_iter()
            .filter(|(_, label, _)| {
                label
                    .to_lowercase()
                    .contains(&self.branch_filter.to_lowercase())
            })
            .collect();
        sync_branch_list(ui, ctx, &items);
        for id in rules::BUTTONS {
            ui.set_widget_disabled_state(id, ctx, !self.is_rules());
        }
        let root = self
            .active_branch
            .and_then(|id| self.doc.nodes.iter().find(|n| n.id == id));
        let unavailable = self.is_configuration() || root.is_none();
        if let Some(button) = ui.get_widget(BRANCH_TOGGLE) {
            button.set_disabled(unavailable);
            button.set_value(TheValue::Text(if root.is_some_and(|n| n.disabled) {
                fl!("node_branch_enable")
            } else {
                fl!("node_branch_disable")
            }));
        }
    }
    /// A node's conversation: the document row it carries.
    fn conversation_of(node: &GraphNode) -> Option<Conversation> {
        node.rows
            .iter()
            .find(|row| row.key.as_deref() == Some("conversation"))
            .and_then(|row| serde_json::to_value(&row.value).ok())
            .and_then(|value| Conversation::from_control(&value))
    }
    /// Open the conversation of the Talk node under a canvas point.
    fn open_conversation(&mut self, point: [f32; 2], view: [f32; 2]) -> bool {
        let graph_point = self.editor.viewport.to_graph(point);
        let metrics = self.doc.metrics();
        let Some(node) = self.doc.nodes.iter().rev().find(|node| {
            node.definition.as_deref() == Some("talk") && node.rect(&metrics).contains(graph_point)
        }) else {
            return false;
        };
        let Some(conversation) = Self::conversation_of(node) else {
            return false;
        };
        let node_id = node.id;
        self.conversation = Some(ConversationEditor::new(node_id, conversation, view));
        true
    }
    /// Edits land in the node as one conversation document, which is what the
    /// runtime prefers; the starter tables then step aside.
    fn write_conversation(&mut self, editor: ConversationEditor) -> bool {
        if !editor.changed() {
            return false;
        }
        let Ok(data) = serde_json::to_value(editor.conversation()) else {
            return false;
        };
        let Some(node) = self
            .doc
            .nodes
            .iter_mut()
            .find(|node| node.id == editor.node)
        else {
            return false;
        };
        let mut row = GraphRow::new(
            &fl!("node_talk_conversation"),
            GraphControlValue::Custom {
                kind: "conversation".into(),
                data,
            },
        );
        row.key = Some("conversation".into());
        node.rows = vec![row];
        true
    }
    fn close_conversation(&mut self, project: &mut Project, commit: bool) {
        self.close_text_overlay(commit);
        self.conversation_field = None;
        let Some(editor) = self.conversation.take() else {
            return;
        };
        if commit && self.write_conversation(editor) {
            self.finish(project);
        }
    }
    /// Start editing one conversation field with the shared text overlay.
    fn open_conversation_field(
        &mut self,
        field: ConversationField,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        let view = ui
            .get_render_view(VIEW)
            .map(|view| {
                let d = *view.dim();
                [d.width as f32, d.height as f32]
            })
            .unwrap_or([900., 600.]);
        let Some((value, rect)) = self.conversation.as_ref().and_then(|editor| {
            editor
                .text(field)
                .map(|text| (text, editor.editor_rect(field, view)))
        }) else {
            return;
        };
        let target = TextTarget {
            node: GraphId::nil(),
            row: GraphId::nil(),
            cell: None,
        };
        self.conversation_field = Some(field);
        self.text_overlay = Some(TextOverlay::new(target, value, rect));
        if let Some(view) = ui.get_render_view(VIEW) {
            ctx.ui.set_focus(view.id());
        }
    }
    /// The shown branch's own nodes, without the unwired strays the canvas also
    /// shows. Branches are the unit of reuse.
    fn shown_branch_only(&self) -> Option<(GraphId, std::collections::HashSet<GraphId>)> {
        let branches = graph_branches(&self.doc, &self.definitions);
        let root = self
            .active_branch
            .filter(|root| branches.branch(*root).is_some())
            .or_else(|| branches.branches.first().map(|branch| branch.root))?;
        let branch = branches.branch(root)?;
        Some((root, branch.nodes.clone()))
    }
    /// A document holding just the shown branch: its nodes and the connections
    /// between them. That is what Copy Branch puts on the clipboard, so pasting
    /// brings over a branch and nothing else.
    fn branch_document(&self) -> Option<GraphDocument> {
        let (_, nodes) = self.shown_branch_only()?;
        let mut document = GraphDocument {
            version: self.doc.version,
            nodes: self
                .doc
                .nodes
                .iter()
                .filter(|node| nodes.contains(&node.id))
                .cloned()
                .collect(),
            connections: vec![],
        };
        let ports: std::collections::HashSet<GraphId> = document
            .nodes
            .iter()
            .flat_map(|node| node.ports.iter().map(|port| port.id))
            .collect();
        document.connections = self
            .doc
            .connections
            .iter()
            .filter(|connection| ports.contains(&connection.from) && ports.contains(&connection.to))
            .cloned()
            .collect();
        Some(document)
    }
    /// Put the shown branch on the clipboard.
    fn copy_branch(&mut self, project: &mut Project) -> bool {
        self.finish(project);
        let Some(document) = self.branch_document() else {
            return false;
        };
        self.clipboard = Some(document);
        true
    }
    /// Add the copied branch to this graph as a fresh branch: new ids, nudged
    /// clear of the original, and shown once it lands.
    fn paste_branch(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
    ) -> bool {
        let Some(clipboard) = self.clipboard.clone() else {
            return false;
        };
        if clipboard.nodes.iter().any(|n| {
            n.definition.as_deref().is_some_and(|id| {
                if self.is_rules() {
                    !id.starts_with("rules_") && !id.starts_with("particle_")
                } else {
                    id.starts_with("rules_")
                        || id.starts_with("particle_")
                        || id.starts_with("entity") != self.is_configuration()
                }
            })
        }) {
            return false;
        }
        self.finish(project);
        let clipboard = clipboard;

        if clipboard
            .nodes
            .iter()
            .any(|n| is_branch_trigger(n, &self.definitions))
        {
            self.begin_branch();
        }
        let moved = paste_graph(&mut self.doc, &clipboard);
        // The copied trigger heads the pasted branch, so focus what just landed.
        let root = clipboard
            .nodes
            .iter()
            .find(|node| is_branch_trigger(node, &self.definitions))
            .and_then(|node| moved.get(&node.id).copied());
        self.active_branch = root;
        self.branches_dirty = true;
        self.apply_branch_filter();
        self.tidy_branch_once();
        self.finish(project);
        self.set_undo_state_to_ui(ctx);
        self.sync_branches(ui, ctx);
        self.fit_branch(ui);
        self.render(ui, ctx);
        true
    }
    /// Delete the selected branch's own nodes, leaving shared ones in place.
    fn remove_branch(&mut self) -> bool {
        let Some(root) = self.active_branch else {
            return false;
        };
        let branches = graph_branches(&self.doc, &self.definitions);
        let Some(branch) = branches.branch(root) else {
            return false;
        };
        let shared = branches.shared_nodes();
        let remove: std::collections::HashSet<GraphId> = branch
            .nodes
            .iter()
            .copied()
            .filter(|id| !shared.contains(id))
            .collect();
        if remove.is_empty() {
            return false;
        }
        let ports: std::collections::HashSet<GraphId> = self
            .doc
            .nodes
            .iter()
            .filter(|node| remove.contains(&node.id))
            .flat_map(|node| node.ports.iter().map(|port| port.id))
            .collect();
        self.doc.nodes.retain(|node| !remove.contains(&node.id));
        self.doc
            .connections
            .retain(|c| !ports.contains(&c.from) && !ports.contains(&c.to));
        if self.is_configuration() {
            self.active_branch = None;
        }
        self.branches_dirty = true;
        self.apply_branch_filter();
        true
    }
    fn set_clipboard(text: &str) {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text.to_string());
        }
    }
    fn get_clipboard() -> Option<String> {
        arboard::Clipboard::new()
            .ok()
            .and_then(|mut clipboard| clipboard.get_text().ok())
    }
    fn pick(&mut self, item: GraphPickerItem, target: Option<Uuid>) {
        let position = self
            .editor
            .selected
            .and_then(|id| self.doc.nodes.iter().find(|n| n.id == id))
            .map(|n| [n.position[0] + n.width + 100., n.position[1]])
            .unwrap_or_else(|| self.editor.viewport.to_graph([60., 60.]));
        if let Some(choice) = self.choice_target.take() {
            if let Some(row) = self
                .doc
                .nodes
                .iter_mut()
                .find(|n| n.id == choice.node)
                .and_then(|n| n.rows.iter_mut().find(|r| r.id == choice.row))
            {
                let mut value = if let Some((r, c)) = choice.cell {
                    if let GraphControlValue::List { rows, .. } = &mut row.value {
                        rows.get_mut(r).and_then(|r| r.get_mut(c))
                    } else {
                        None
                    }
                } else {
                    Some(&mut row.value)
                };
                if let Some(GraphControlValue::Custom { kind, data }) = value.as_deref_mut() {
                    if matches!(kind.as_str(), "tile" | "rules_icon") {
                        *data = item.id.clone().into();
                    }
                }
                if let Some(GraphControlValue::Choice { options, selected }) = value {
                    if let Ok(index) = item.id.parse::<usize>() {
                        if index < options.len() {
                            *selected = index;
                        }
                    }
                }
            }
            if self.is_rules() {
                rules::apply_icon_selection(&mut self.doc, &choice, &item.id);
            }
        } else if let Some(id) = target {
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
        } else if let Ok(mut n) = self.definitions.instantiate(&item.id, position) {
            if is_branch_trigger(&n, &self.definitions) {
                self.begin_branch();
            }
            n.branch = if is_branch_trigger(&n, &self.definitions) {
                Some(n.id)
            } else {
                self.active_branch
            };
            self.editor.selected = Some(n.id);
            self.doc.nodes.push(n);
            // A new trigger is a new branch, so the list has to catch up.
            self.branches_dirty = true;
        }
        catalog::sync_fields(&mut self.doc, &self.definitions);
        // Adding a trigger starts a branch and focuses it. Anything else joins
        // the shown branch as an unwired node, so refresh the filter either way;
        // otherwise the node the author just dropped would be invisible.
        if let Some(id) = self.editor.selected
            && self
                .doc
                .nodes
                .iter()
                .any(|node| node.id == id && is_branch_trigger(node, &self.definitions))
        {
            self.active_branch = Some(id);
        }
        self.apply_branch_filter();
    }
    fn graph_toolbar() -> TheCanvas {
        let mut canvas = TheCanvas::new();
        canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut layout = TheHLayout::new(TheId::named("Node Graph Actions"));
        layout.set_background_color(None);
        layout.set_margin(Vec4::new(6, 2, 6, 2));
        layout.set_padding(5);
        Self::populate_toolbar(&mut layout, false);
        canvas.set_layout(layout);
        canvas
    }
    fn populate_toolbar(layout: &mut dyn TheHLayoutTrait, rules: bool) {
        layout.clear();
        for (id, text, status) in [
            (
                BRANCH_TOGGLE,
                fl!("node_branch_disable"),
                fl!("status_node_branch_toggle"),
            ),
            (CLEAR_NODES, fl!("node_clear"), fl!("status_node_clear")),
            (
                COPY_BRANCH,
                fl!("node_copy_branch"),
                fl!("status_node_copy_branch"),
            ),
            (
                PASTE_BRANCH,
                fl!("node_paste_branch"),
                fl!("status_node_paste_branch"),
            ),
            (
                BRANCH_REMOVE,
                fl!("node_branch_remove"),
                fl!("status_node_branch_remove"),
            ),
            (
                BRANCH_TIDY,
                fl!("node_branch_tidy"),
                fl!("status_node_branch_tidy"),
            ),
        ] {
            if rules && matches!(id, CLEAR_NODES | COPY_BRANCH | PASTE_BRANCH) {
                continue;
            }
            let mut button = TheTraybarButton::new(TheId::named(id));
            button.set_text(text);
            button.set_status_text(&status);
            button.set_fixed_size(false);
            layout.add_widget(Box::new(button));
        }
        if rules {
            rules::add_toolbar(layout);
        }
    }
    /// Replaces the graph with an empty one.
    fn clear_nodes(&mut self, ui: &mut TheUI, ctx: &mut TheContext, project: &mut Project) {
        self.finish(project);
        self.doc.nodes.clear();
        self.doc.connections.clear();
        if self.is_configuration() {
            self.active_branch = None;
        }
        self.branches_dirty = true;
        self.apply_branch_filter();
        self.finish(project);
        self.set_undo_state_to_ui(ctx);
        self.render(ui, ctx);
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
            tiles: Default::default(),
            owner: None,
            histories: HashMap::new(),
            dirty: false,
            live: Default::default(),
            load_error: None,
            popup: None,
            choice_target: None,
            clipboard: None,
            text_overlay: None,
            conversation: None,
            conversation_field: None,
            last_click: None,
            active_branch: None,
            branch_graphs: HashMap::new(),
            branch_order: Vec::new(),
            branches_dirty: true,
            initial_layouts: HashMap::new(),
            rules_error: None,
            branch_filter: String::new(),
            particle_preview_started: Instant::now(),
            particle_preview_updated: None,
        }
    }
    fn setup(&mut self, _: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();
        canvas.set_top(Self::graph_toolbar());
        canvas.set_left(branch_list_canvas());
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
        sync_node_list(ui, ctx, server.pc);
        if let Some(layout) = ui.get_hlayout("Node Graph Actions") {
            Self::populate_toolbar(layout, server.pc == ProjectContext::GameRules);
            ctx.ui.relayout = true;
        }
        self.tiles.refresh(project);
        let owner = Self::owner(server.pc);
        self.definitions = if server.pc == ProjectContext::GameRules {
            shared::rulesets::graph::definitions()
        } else if entity::is_configuration(server.pc) {
            entity::definitions(project)
        } else {
            catalog::definitions_for_project(project)
        };
        self.editor.finish_text(&mut self.doc, false);
        self.editor.take_edits();
        self.popup = None;
        self.choice_target = None;
        self.text_overlay = None;
        self.last_click = None;
        // Edits are persisted on input; keep the pending undo group on reactivation.
        if let Some(key) = self.history_key() {
            if self.doc != self.committed {
                let h = self.histories.entry(key).or_default();
                h.undo.push(self.committed.clone());
                h.redo.clear();
            }
        }
        if self.owner != owner {
            self.branch_graphs.clear();
            self.branch_order.clear();
            self.branch_filter.clear();
            ui.set_widget_value("Node Branch Search", ctx, TheValue::Text(String::new()));
            self.active_branch = None;
            self.owner = owner;
            self.live = Default::default();
            self.editor = node_editor();
            self.popup = None;
            self.choice_target = None;
        }
        self.load_error = None;
        self.rules_error = if self.is_rules() {
            project.rules_source().err()
        } else {
            None
        };
        let saved = if self.is_rules() {
            Some(serde_json::to_value(&project.rules.current).unwrap())
        } else {
            self.owner
                .as_ref()
                .and_then(|k| project.node_graphs.get(k))
                .cloned()
        };
        self.doc = if let Some(value) = saved {
            let loaded = if self.is_configuration() {
                serde_json::from_value::<GraphDocument>(value).map_err(|e| e.to_string())
            } else {
                self.load_branch_documents(&value)
            };
            match loaded {
                Ok(doc) if doc.version == 1 => doc,
                _ => {
                    self.load_error = Some(fl!("node_unsupported"));
                    GraphDocument::default()
                }
            }
        } else if self.is_configuration() {
            let rules = project
                .rules_source()
                .unwrap_or_default()
                .parse::<shared::entity_graph::RulesTable>()
                .unwrap_or_default();
            let data = self
                .owner
                .as_ref()
                .and_then(|key| shared::entity_graph::owner_data(project, key))
                .unwrap_or("");
            let character_template = self
                .owner
                .as_ref()
                .and_then(|key| key.strip_prefix("entity/character/"))
                .and_then(|id| Uuid::parse_str(id).ok())
                .is_some_and(|id| project.characters.contains_key(&id));
            let item_template = self
                .owner
                .as_ref()
                .and_then(|key| key.strip_prefix("entity/item/"))
                .and_then(|id| Uuid::parse_str(id).ok())
                .is_some_and(|id| project.items.contains_key(&id));
            (if character_template {
                shared::entity_graph::import_character(data, &rules)
            } else if item_template {
                shared::entity_graph::import_item(data, &rules)
            } else {
                shared::entity_graph::import(data, &rules)
            })
            .unwrap_or_default()
        } else {
            GraphDocument::default()
        };
        if self.is_configuration() {
            shared::entity_graph::normalize(&mut self.doc);
        }
        self.refresh_configuration(project);
        catalog::hydrate_lookout_distances(&mut self.doc, project);
        catalog::sync_fields(&mut self.doc, &self.definitions);
        // First opening arranges the branch; later edits keep their authored layout.
        if self.is_configuration() {
            self.active_branch = None;
        }
        self.branches_dirty = true;
        self.apply_branch_filter();
        self.tidy_branch_once();
        self.committed = self.doc.clone();
        self.fit_branch(ui);
        self.render(ui, ctx);
    }
    fn poll_background(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut ServerContext,
    ) -> bool {
        if self.is_rules() {
            if self
                .doc
                .nodes
                .iter()
                .any(|n| n.definition.as_deref() == Some("rules_fx") && !n.disabled)
                && self
                    .particle_preview_updated
                    .is_none_or(|last| last.elapsed() >= Duration::from_millis(67))
            {
                self.render(ui, ctx);
                return true;
            }
            return false;
        }
        if self.is_configuration() {
            return false;
        }
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
        self.rules_error = None;
        self.branch_filter.clear();
        self.histories.clear();
        self.branch_graphs.clear();
        self.branch_order.clear();
        self.live = Default::default();
        self.owner = None;
        if self.is_configuration() {
            shared::entity_graph::normalize(&mut self.doc);
        }
        self.doc = GraphDocument::default();
        self.committed = self.doc.clone();
        self.editor = node_editor();
        self.popup = None;
        self.choice_target = None;
        self.text_overlay = None;
        self.conversation = None;
        self.conversation_field = None;
        self.last_click = None;
        self.active_branch = None;
        self.branches_dirty = true;
        self.dirty = false;
    }
    fn set_undo_state_to_ui(&self, ctx: &mut TheContext) {
        let h = self
            .history_key()
            .as_ref()
            .and_then(|k| self.histories.get(k));
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
        let key = self.history_key();
        if let Some(h) = key.as_ref().and_then(|k| self.histories.get_mut(k)) {
            if let Some(doc) = h.undo.pop() {
                h.redo.push(self.doc.clone());
                self.doc = doc;
                self.committed = self.doc.clone();
                let viewport = self.editor.viewport.clone();
                self.editor = node_editor();
                self.editor.viewport = viewport;
                self.apply_branch_filter();
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
        let key = self.history_key();
        if let Some(h) = key.as_ref().and_then(|k| self.histories.get_mut(k)) {
            if let Some(doc) = h.redo.pop() {
                h.undo.push(self.doc.clone());
                self.doc = doc;
                self.committed = self.doc.clone();
                let viewport = self.editor.viewport.clone();
                self.editor = node_editor();
                self.editor.viewport = viewport;
                self.apply_branch_filter();
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
        if let TheEvent::ValueChanged(id, TheValue::Text(text)) = event {
            if id.name == "Node Branch Search" {
                self.branch_filter = text.clone();
                self.sync_branches(ui, ctx);
                return true;
            }
        }
        if self.handle_rules_event(event, ui, ctx, project, server) {
            return true;
        }
        if let TheEvent::WidgetResized(id, _) = event {
            if id.name == VIEW {
                self.render(ui, ctx);
                return true;
            }
        }
        if let TheEvent::StateChanged(id, state) = event
            && matches!(state, TheWidgetState::Clicked | TheWidgetState::Selected)
        {
            match id.name.as_str() {
                CLEAR_NODES if *state == TheWidgetState::Clicked => {
                    self.clear_nodes(ui, ctx, project);
                    return true;
                }
                COPY_BRANCH if *state == TheWidgetState::Clicked => {
                    self.copy_branch(project);
                    return true;
                }
                PASTE_BRANCH if *state == TheWidgetState::Clicked => {
                    return self.paste_branch(ui, ctx, project);
                }
                BRANCH_TOGGLE if *state == TheWidgetState::Clicked => {
                    if !self.is_configuration() {
                        self.finish(project);
                        if let Some(node) = self
                            .active_branch
                            .and_then(|id| self.doc.nodes.iter_mut().find(|n| n.id == id))
                        {
                            node.disabled = !node.disabled;
                            self.store(project, true);
                            self.set_undo_state_to_ui(ctx);
                        }
                    }
                    self.sync_branches(ui, ctx);
                    self.render(ui, ctx);
                    return true;
                }
                BRANCH_REMOVE if *state == TheWidgetState::Clicked => {
                    if self.remove_branch() {
                        self.finish(project);
                    }
                    self.sync_branches(ui, ctx);
                    self.fit_branch(ui);
                    self.render(ui, ctx);
                    return true;
                }
                BRANCH_TIDY if *state == TheWidgetState::Clicked => {
                    if self.tidy_branch() {
                        self.store(project, true);
                        self.set_undo_state_to_ui(ctx);
                    }
                    self.fit_branch(ui);
                    self.render(ui, ctx);
                    return true;
                }
                _ => {}
            }
            if let Some(key) = id.name.strip_prefix("Branch/")
                && let Ok(root) = Uuid::parse_str(key)
            {
                self.finish(project);
                // Each branch owns a document and its own history.
                self.set_branch(Some(root));
                self.set_undo_state_to_ui(ctx);
                self.sync_branches(ui, ctx);
                self.fit_branch(ui);
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
                    let trigger = is_branch_trigger(&node, &self.definitions);
                    if trigger {
                        self.begin_branch();
                    }
                    node.branch = if trigger {
                        Some(node.id)
                    } else {
                        self.active_branch
                    };
                    if trigger {
                        self.active_branch = Some(node.id);
                    }
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
                if self.text_overlay.is_some() {
                    return false;
                }
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
                    .find(|n| n.rect(&self.doc.metrics()).contains(p))
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
            TheEvent::RenderViewClicked(id, p)
                if id.name == VIEW
                    && ui.shift
                    && self.popup.is_none()
                    && self.text_overlay.is_none()
                    && self.conversation.is_none() =>
            {
                self.finish(project);
                self.editor
                    .begin_cut(&mut self.doc, [p.x as f32, p.y as f32]);
            }
            TheEvent::RenderViewClicked(id, p) if id.name == VIEW => {
                let point = [p.x as f32, p.y as f32];
                let double_click = self.last_click.is_some_and(|(at, last)| {
                    at.elapsed() < Duration::from_millis(400)
                        && (last[0] - point[0]).abs() < 6.
                        && (last[1] - point[1]).abs() < 6.
                });
                self.last_click = Some((Instant::now(), point));
                let overlay_open = self.text_overlay.is_some();
                let overlay_contains = self
                    .text_overlay
                    .as_ref()
                    .is_some_and(|overlay| overlay.contains(point));
                if overlay_open {
                    // Click inside places the caret; a click outside commits.
                    if overlay_contains {
                        let font = &self.resources.font;
                        let extend = ui.shift;
                        if let Some(overlay) = self.text_overlay.as_mut() {
                            overlay.caret_at(point, font, extend);
                        }
                    } else {
                        checkpoint = self.close_text_overlay(true);
                        // Editing a conversation field keeps the panel open.
                        if self.conversation.is_some() {
                            return true;
                        }
                    }
                } else if self.conversation.is_some() {
                    let click = self
                        .conversation
                        .as_mut()
                        .map(|editor| editor.pointer_down(point))
                        .unwrap_or(ConversationClick::Outside);
                    match click {
                        ConversationClick::Outside => self.close_conversation(project, true),
                        ConversationClick::Edit(field) => {
                            self.open_conversation_field(field, ui, ctx)
                        }
                        ConversationClick::Handled => {}
                    }
                    return true;
                } else if double_click && {
                    let graph_point = self.editor.viewport.to_graph(point);
                    let dim = ui
                        .get_render_view(VIEW)
                        .map(|view| {
                            let d = *view.dim();
                            [d.width as f32, d.height as f32]
                        })
                        .unwrap_or([800., 600.]);
                    // A Talk node opens its conversation; anything else opens
                    // the text row or cell that was double-clicked.
                    self.open_conversation(point, dim) || self.open_text_overlay(graph_point, dim)
                } {
                    // The overlay handled the click. Keep keyboard focus on the
                    // canvas, so Escape reaches the panel.
                    if let Some(view) = ui.get_render_view(VIEW) {
                        ctx.ui.set_focus(view.id());
                    }
                } else if let Some((picker, target)) = self.popup.take() {
                    if let Some(item) = picker.pick(point) {
                        self.pick(
                            GraphPickerItem {
                                id: item,
                                label: String::new(),
                            },
                            target,
                        );
                        checkpoint = true;
                        // Adding a branch focuses it, so bring it into view.
                        self.sync_branches(ui, ctx);
                        self.fit_branch(ui);
                    } else if picker.contains(point) {
                        self.popup = Some((picker, target));
                    } else {
                        self.choice_target = None;
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
                                && n.row_rect(0, &self.doc.metrics()).contains(graph_point)
                        })
                        .map(|n| n.id);
                    if self.open_tile_picker(point, ui, project)
                        || self.open_configuration_choice(point, ui)
                    {
                        self.editor.finish_text(&mut self.doc, false);
                    } else if let Some(id) = event_node {
                        self.choice_target = None;
                        self.finish(project);
                        let row = self
                            .doc
                            .nodes
                            .iter()
                            .find(|n| n.id == id)
                            .unwrap()
                            .row_rect(0, &self.doc.metrics());
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
            TheEvent::RenderViewDragged(id, p) if id.name == VIEW && self.popup.is_none() => {
                if self.text_overlay.is_some() {
                    // Dragging inside the overlay extends the selection.
                    let point = [p.x as f32, p.y as f32];
                    let font = &self.resources.font;
                    if let Some(overlay) = self.text_overlay.as_mut() {
                        overlay.caret_at(point, font, true);
                    }
                } else {
                    self.editor.pointer_move(
                        &mut self.doc,
                        [p.x as f32, p.y as f32],
                        &BasicGraphControls,
                    );
                }
            }
            TheEvent::RenderViewUp(id, p)
                if id.name == VIEW && self.popup.is_none() && self.text_overlay.is_none() =>
            {
                self.editor.pointer_up(
                    &mut self.doc,
                    [p.x as f32, p.y as f32],
                    &NodeConnectionPolicy,
                );
                checkpoint = self.editor.text_focus().is_none();
            }
            TheEvent::RenderViewScrollBy(id, d) | TheEvent::RenderViewPreciseScrollBy(id, d)
                if id.name == VIEW =>
            {
                if let Some(overlay) = &mut self.text_overlay {
                    overlay.scroll_by(d.y as f32);
                } else if let Some(editor) = &mut self.conversation {
                    // The panel scrolls its own columns; the graph stays put.
                    editor.scroll_at(self.editor.cursor, d.y as f32);
                } else if let Some((p, _)) = &mut self.popup {
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
                if let Some(overlay) = &mut self.text_overlay {
                    if ui.ctrl || ui.logo {
                        match c.to_ascii_lowercase() {
                            'a' => overlay.input(GraphTextInput::SelectAll),
                            'c' => Self::set_clipboard(&overlay.copy()),
                            'x' => Self::set_clipboard(&overlay.cut()),
                            'v' => {
                                if let Some(text) = Self::get_clipboard() {
                                    overlay.paste(&text);
                                }
                            }
                            _ => {}
                        }
                    } else {
                        overlay.input(GraphTextInput::Insert(c.to_string()));
                    }
                } else if let Some((p, _)) = &mut self.popup {
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
                if self.text_overlay.is_some() {
                    match key {
                        TheKeyCode::Escape => {
                            checkpoint = self.close_text_overlay(false);
                        }
                        TheKeyCode::Return if ui.ctrl || ui.logo => {
                            checkpoint = self.close_text_overlay(true);
                        }
                        TheKeyCode::Return => {
                            if let Some(overlay) = &mut self.text_overlay {
                                overlay.input(GraphTextInput::Insert("\n".into()));
                            }
                        }
                        TheKeyCode::Delete => {
                            if let Some(overlay) = &mut self.text_overlay {
                                overlay.input(GraphTextInput::Backspace);
                            }
                        }
                        TheKeyCode::Left => {
                            if let Some(overlay) = &mut self.text_overlay {
                                overlay.input(GraphTextInput::Left { extend: ui.shift });
                            }
                        }
                        TheKeyCode::Right => {
                            if let Some(overlay) = &mut self.text_overlay {
                                overlay.input(GraphTextInput::Right { extend: ui.shift });
                            }
                        }
                        TheKeyCode::Home => {
                            if let Some(overlay) = &mut self.text_overlay {
                                overlay.input(GraphTextInput::Home { extend: ui.shift });
                            }
                        }
                        TheKeyCode::End => {
                            if let Some(overlay) = &mut self.text_overlay {
                                overlay.input(GraphTextInput::End { extend: ui.shift });
                            }
                        }
                        TheKeyCode::Up | TheKeyCode::Down => {
                            let delta = if *key == TheKeyCode::Up { -1 } else { 1 };
                            let font = &self.resources.font;
                            if let Some(overlay) = self.text_overlay.as_mut() {
                                overlay.move_vertical(delta, font);
                            }
                        }
                        _ => {}
                    }
                } else if self.conversation.is_some() && matches!(key, TheKeyCode::Escape) {
                    // Escape closes the conversation panel, like any other overlay.
                    self.close_conversation(project, true);
                } else if let Some((mut p, target)) = self.popup.take() {
                    match key {
                        TheKeyCode::Escape => {
                            self.choice_target = None;
                        }
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
