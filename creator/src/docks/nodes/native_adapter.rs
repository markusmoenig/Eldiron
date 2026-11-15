//! Native NodesDock host adapter. This mounts retained widgets and applies CPU
//! source effects; it does not draw a graph or read back a displayed image.
#[path = "native_clipboard.rs"]
mod clipboard;
pub(super) type PendingClipboard = clipboard::Pending;
use super::*;
use native_view::{NativeNodePanels, NativeNodesView};
use shared::graph_authoring::{NodeGraphKind, NodeSourceKey};
use std::sync::{Arc, Mutex};
use theframework::text_edit::EditorCommand;
pub(super) type BlankView = (
    TheId,
    Arc<Mutex<GraphViewState>>,
    Arc<Mutex<NativeNodePanels>>,
);

impl NodesDock {
    pub(super) fn native_particles_visible(&self, ui: &mut TheUI) -> bool {
        if !self.is_rules() {
            return false;
        }
        let mut size = None;
        ui.canvas.visit_visible_widgets(&mut |widget| {
            if widget.id().name == VIEW && widget.dim().width > 0 && widget.dim().height > 0 {
                size = Some([widget.dim().width as f32, widget.dim().height as f32]);
            }
        });
        let Some(size) = size else {
            return false;
        };
        let Some(view) = self
            .native
            .active
            .as_ref()
            .and_then(|key| self.native.views.get(key))
        else {
            return false;
        };
        let model = view.model.lock().unwrap();
        let metrics = model.document().metrics();
        model.document().nodes.iter().any(|node| {
            if node.definition.as_deref() != Some("rules_fx")
                || node.disabled
                || !model.editor().node_visible(node.id)
            {
                return false;
            }
            let rect = node.rect(&metrics);
            let origin = model.editor().viewport.to_screen(rect.origin);
            let zoom = model.editor().viewport.zoom();
            origin[0] < size[0]
                && origin[1] < size[1]
                && origin[0] + rect.size[0] * zoom > 0.
                && origin[1] + rect.size[1] * zoom > 0.
        })
    }
    fn native_status(&self, ctx: &mut TheContext, result: Result<bool, String>) -> bool {
        match result {
            Ok(changed) => changed,
            Err(error) => {
                ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                false
            }
        }
    }
    fn native_effects(
        &mut self,
        project: &Project,
        ctx: &mut TheContext,
        reference: &CreatorReference,
    ) -> bool {
        let changes = self.native.take_changes();
        let changed = !changes.is_empty();
        let mut owners = std::collections::HashSet::new();
        for change in changes {
            if change.key.kind() == NodeGraphKind::Behavior
                && owners.insert(change.key.owner.clone())
                && let Some(value) = project.node_graphs.get(&change.key.owner)
            {
                if let Some(runtime) = reference.existing_mut() {
                    runtime
                        .server
                        .publish_node_graph(change.key.owner, value.clone());
                }
            }
        }
        if changed {
            self.dirty = true;
            self.branches_dirty = true;
        }
        self.set_undo_state_to_ui(ctx);
        changed
    }
    fn sync_native(&mut self, project: &Project) {
        let Some(key) = self.native.active.clone() else {
            self.owner = None;
            self.doc = GraphDocument::default();
            self.committed = self.doc.clone();
            self.active_branch = None;
            self.branch_order.clear();
            self.branch_graphs.clear();
            return;
        };
        let Some(view) = self.native.views.get(&key) else {
            return;
        };
        let model = view.model.lock().unwrap();
        self.doc = model.document().clone();
        self.editor.viewport = model.editor().viewport;
        self.editor.selected = model.editor().selected;
        self.editor.selected_connection = model.editor().selected_connection;
        self.active_branch = view.shown;
        drop(model);
        self.committed = self.native.cpu.source(&key).unwrap().document().clone();
        self.owner = Some(key.owner.clone());
        self.definitions = native_workspace::source_definitions(project, &key);
        if let Ok(graphs) = self
            .native
            .cpu
            .graphs(project, &key.owner, &self.definitions)
        {
            self.branch_order = graphs.order;
            self.branch_graphs = graphs.branches;
        }
        self.branches_dirty = true;
    }
    fn prepare_native_layout(&mut self) {
        let Some(key) = self.native.active.clone() else {
            return;
        };
        let view = &self.native.views[&key];
        let token = (view.session, view.shown);
        if !self.native_prepared.insert(token) {
            return;
        }
        // Activation prepares a private layout only. It is not a Project edit.
        let before = self.doc.clone();
        self.tidy_branch_once();
        if self.doc != before {
            let document = self.doc.clone();
            let _ = self.native.cpu.set_working_document(&key, document.clone());
            self.native.views[&key]
                .model
                .lock()
                .unwrap()
                .edit(|doc, _| *doc = document);
            self.committed = self.doc.clone();
        }
        self.native_fit_pending = true;
    }
    pub(super) fn blank_native_view(&mut self) -> BlankView {
        self.native_blank
            .get_or_insert_with(|| {
                (
                    TheId::named(VIEW),
                    Arc::new(Mutex::new(GraphViewState::new(
                        "unbound-nodes",
                        GraphDocument::default(),
                    ))),
                    Arc::new(Mutex::new(NativeNodePanels::default())),
                )
            })
            .clone()
    }
    pub(super) fn render_native(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.update_particle_preview();
        let (id, model, panels) = if let Some(view) = self
            .native
            .active
            .as_ref()
            .and_then(|key| self.native.views.get(key))
        {
            (view.id.clone(), view.model.clone(), view.panels.clone())
        } else {
            self.blank_native_view()
        };
        let allow_empty = self.is_configuration();
        let Some(widget) = ui.get_widget(VIEW) else {
            return;
        };
        let bound = widget.id() == &id
            && widget
                .as_any()
                .downcast_mut::<NativeNodesView>()
                .is_some_and(|view| Arc::ptr_eq(&view.model(), &model));
        if !bound {
            let old = widget.id().clone();
            let dim = *widget.dim();
            if ctx.ui.has_focus(&old) {
                ctx.ui.clear_focus();
            }
            if ctx.ui.keyboard_focus.as_ref() == Some(&old) {
                ctx.ui.keyboard_focus = None;
            }
            let mut view = NativeNodesView::with_state(id, model.clone(), panels);
            view.set_dim(dim, ctx);
            *widget = Box::new(view);
            self.resources = None;
        }
        if let Some(view) = widget.as_any().downcast_mut::<NativeNodesView>() {
            view.set_assets(self.tiles.clone());
            view.set_snapshot(
                Arc::new(self.definitions.clone()),
                Arc::new(self.live.clone()),
                allow_empty,
            );
            view.set_needs_redraw(true);
        }
        if self.native_fit_pending && widget.dim().width > 0 && widget.dim().height > 0 {
            let size = [widget.dim().width as f32, widget.dim().height as f32];
            let mut model = model.lock().unwrap();
            model.edit(|doc, editor| {
                let nodes = doc
                    .nodes
                    .iter()
                    .filter(|node| editor.node_visible(node.id))
                    .map(|node| node.id)
                    .collect();
                editor.viewport.fit_to_nodes(doc, size, &nodes);
            });
            self.native_fit_pending = false;
        }
        let scope = self
            .native
            .active
            .as_ref()
            .and_then(|key| self.native.views.get(key))
            .map(|view| (view.session, view.shown));
        if self.native_toolbar_scope != scope {
            if let Some(layout) = ui.get_hlayout("Node Graph Actions") {
                Self::populate_toolbar(layout, false);
                ctx.ui.invalidate_layout();
            }
            self.native_toolbar_scope = scope;
        }
        if self.branches_dirty {
            self.sync_branches(ui, ctx);
        }
        ctx.ui.invalidate_paint();
    }
    pub(super) fn activate_native(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        context: ProjectContext,
        branch: Option<GraphId>,
    ) {
        self.native_enabled = true;
        self.native_clipboard = None;
        sync_node_list(ui, ctx, context);
        let previous = self.native.active.as_ref().map(|key| key.owner.clone());
        self.load_error = None;
        match self
            .native
            .activate(project, context, branch, &mut ctx.text)
        {
            Ok(_) => {}
            Err(error) => {
                self.load_error = Some(error.clone());
                ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
            }
        }
        if previous != self.native.active.as_ref().map(|key| key.owner.clone()) {
            self.live = Default::default();
            self.branch_filter.clear();
            ui.set_widget_value("Node Branch Search", ctx, TheValue::Text(String::new()));
        }
        self.tiles.refresh(project);
        self.sync_native(project);
        self.prepare_native_layout();
        for (id, packet) in self.native.notifications() {
            ctx.ui.send(TheEvent::GraphCommitted(id, packet));
        }
        self.set_undo_state_to_ui(ctx);
        self.render_native(ui, ctx);
    }
    pub(super) fn commit_native(
        &mut self,
        ctx: &mut TheContext,
        project: &mut Project,
        reference: &CreatorReference,
    ) -> Result<bool, String> {
        let result = self.native.checkpoint_all(project);
        let changed = self.native_effects(project, ctx, reference);
        self.sync_native(project);
        result.map(|collected| changed || collected)
    }
    pub(super) fn history_native(
        &mut self,
        forward: bool,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        reference: &CreatorReference,
    ) {
        if let Some(key) = self.native.active.clone() {
            let result = if forward {
                self.native.redo_source(project, &key, &mut ctx.text)
            } else {
                self.native.undo_source(project, &key, &mut ctx.text)
            };
            self.native_status(ctx, result);
            self.native_effects(project, ctx, reference);
            self.sync_native(project);
            self.render_native(ui, ctx);
        }
    }
    pub(super) fn native_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        reference: &CreatorReference,
    ) -> bool {
        if let Some(handled) = self.native_clipboard_event(event, ui, ctx, project, reference) {
            return handled;
        }
        match event {
            TheEvent::GraphCommitted(id, packet) if id.name == VIEW => {
                let Some(key) = self.native.cpu.binding_source(packet.binding).cloned() else {
                    return false;
                };
                if self
                    .native
                    .views
                    .get(&key)
                    .is_none_or(|view| &view.id != id)
                {
                    return false;
                }
                let result = self.native.collect_through(project, packet.sequence);
                self.native_status(ctx, result);
                self.native_effects(project, ctx, reference);
                self.sync_native(project);
                self.render_native(ui, ctx);
                return true;
            }
            TheEvent::GraphDrop(id, binding, point, drop) if id.name == VIEW => {
                let Some(key) = self.native.cpu.binding_source(*binding).cloned() else {
                    return false;
                };
                if self
                    .native
                    .views
                    .get(&key)
                    .is_none_or(|view| &view.id != id)
                {
                    return false;
                }
                if drop.id.name == "Item Item" {
                    let result = self.native.drop_equipment_item(
                        project,
                        &key,
                        drop.id.references,
                        *point,
                        &mut ctx.text,
                    );
                    if let Err(error) = &result {
                        self.native.command_error(&key, Some(error.clone()));
                    }
                    let handled = matches!(&result, Ok(Some(_)) | Err(_));
                    self.native_status(ctx, result.map(|changed| changed.unwrap_or(false)));
                    self.native_effects(project, ctx, reference);
                    self.sync_native(project);
                    self.render_native(ui, ctx);
                    return handled;
                }
                let Some(definition) = drop
                    .id
                    .name
                    .strip_prefix("Node Catalog/")
                    .filter(|key| *key == drop.data)
                else {
                    return false;
                };
                let result =
                    self.native
                        .add_catalog_node(project, &key, definition, *point, &mut ctx.text);
                self.native_status(ctx, result);
                self.native_effects(project, ctx, reference);
                self.sync_native(project);
                self.prepare_native_layout();
                self.render_native(ui, ctx);
                return true;
            }
            TheEvent::GraphInteractionStarted(id, binding) if id.name == VIEW => {
                let Some(key) = self.native.cpu.binding_source(*binding) else {
                    return false;
                };
                if self.native.views.get(key).is_none_or(|view| &view.id != id) {
                    return false;
                }
                let result = self.native.collect_through(project, u64::MAX);
                self.native_status(ctx, result);
                self.native_effects(project, ctx, reference);
                self.sync_native(project);
                return false;
            }
            TheEvent::GraphCommand(id, binding, command) if id.name == VIEW => {
                let Some(key) = self.native.cpu.binding_source(*binding).cloned() else {
                    return false;
                };
                if self
                    .native
                    .views
                    .get(&key)
                    .is_none_or(|view| &view.id != id)
                {
                    return false;
                }
                let result = match command {
                    EditorCommand::Undo => self.native.undo_source(project, &key, &mut ctx.text),
                    EditorCommand::Redo => self.native.redo_source(project, &key, &mut ctx.text),
                    _ => return false,
                };
                self.native_status(ctx, result);
                self.native_effects(project, ctx, reference);
                self.sync_native(project);
                self.render_native(ui, ctx);
                return true;
            }
            TheEvent::WidgetResized(id, _) if id.name == VIEW => {
                if ui.get_widget(VIEW).is_none_or(|widget| widget.id() != id) {
                    return false;
                }
                self.render_native(ui, ctx);
                return true;
            }
            TheEvent::Custom(id, _) if id.name == "Creator Theme Changed" => {
                self.render_native(ui, ctx);
                return true;
            }
            TheEvent::ValueChanged(id, TheValue::Text(text)) if id.name == "Node Branch Search" => {
                self.branch_filter = text.clone();
                self.sync_branches(ui, ctx);
                return true;
            }
            _ => {}
        }
        let Some(key) = self.native.active.clone() else {
            return false;
        };
        if let TheEvent::StateChanged(id, state) = event
            && matches!(state, TheWidgetState::Clicked | TheWidgetState::Selected)
        {
            if let Some(root) = id
                .name
                .strip_prefix("Branch/")
                .and_then(|id| GraphId::parse_str(id).ok())
            {
                let result = self.commit_native(ctx, project, reference);
                self.native_status(ctx, result);
                let context = self.native.views[&key].context;
                self.activate_native(ui, ctx, project, context, Some(root));
                self.native_fit_pending = true;
                self.render_native(ui, ctx);
                return true;
            }
            if *state == TheWidgetState::Clicked {
                // Toolbar widgets are rebuilt for each activation. A queued
                // callback from the retired toolbar cannot target this owner.
                if matches!(
                    id.name.as_str(),
                    CLEAR_NODES
                        | BRANCH_TOGGLE
                        | BRANCH_TIDY
                        | BRANCH_REMOVE
                        | COPY_BRANCH
                        | PASTE_BRANCH
                ) && ui
                    .get_widget(&id.name)
                    .is_none_or(|widget| widget.id() != id)
                {
                    return false;
                }
                let result = match id.name.as_str() {
                    CLEAR_NODES => {
                        self.native
                            .edit_source(project, &key, &mut ctx.text, |doc, _| {
                                doc.nodes.clear();
                                doc.connections.clear();
                                Ok(key.clone())
                            })
                    }
                    BRANCH_TOGGLE => {
                        self.native
                            .edit_source(project, &key, &mut ctx.text, |doc, _| {
                                if key.kind() != NodeGraphKind::Configuration
                                    && let Some(root) = key.branch
                                    && let Some(node) =
                                        doc.nodes.iter_mut().find(|node| node.id == root)
                                {
                                    node.disabled = !node.disabled;
                                }
                                Ok(key.clone())
                            })
                    }
                    BRANCH_TIDY => {
                        let shown = self.native.views[&key].shown;
                        self.native
                            .edit_source(project, &key, &mut ctx.text, |doc, defs| {
                                let nodes = if key.kind() == NodeGraphKind::Configuration {
                                    shown.and_then(|root| {
                                        graph_branches(doc, defs)
                                            .branch(root)
                                            .map(|branch| branch.nodes.clone())
                                    })
                                } else {
                                    Some(doc.nodes.iter().map(|node| node.id).collect())
                                };
                                if let Some(nodes) = nodes {
                                    layout_branch(doc, &nodes);
                                }
                                Ok(key.clone())
                            })
                    }
                    BRANCH_REMOVE => {
                        let shown = self.native.views[&key].shown;
                        self.native
                            .edit_source(project, &key, &mut ctx.text, |doc, defs| {
                                if key.kind() != NodeGraphKind::Configuration {
                                    doc.nodes.clear();
                                    doc.connections.clear();
                                } else if let Some(root) = shown {
                                    let branches = graph_branches(doc, defs);
                                    let shared = branches.shared_nodes();
                                    if let Some(branch) = branches.branch(root) {
                                        let nodes: std::collections::HashSet<_> = branch
                                            .nodes
                                            .iter()
                                            .filter(|node| !shared.contains(node))
                                            .copied()
                                            .collect();
                                        remove_native_nodes(doc, &nodes);
                                    }
                                }
                                Ok(key.clone())
                            })
                    }
                    COPY_BRANCH => {
                        match self.native.branch_document(project, &key, &mut ctx.text) {
                            Ok(document) => {
                                self.clipboard = Some(document);
                                Ok(false)
                            }
                            Err(error) => Err(error),
                        }
                    }
                    PASTE_BRANCH => {
                        if let Some(document) = self.clipboard.clone() {
                            self.native.paste_branch_document(
                                project,
                                &key,
                                &document,
                                &mut ctx.text,
                            )
                        } else {
                            Ok(false)
                        }
                    }
                    _ => return false,
                };
                self.native_status(ctx, result);
                self.native_effects(project, ctx, reference);
                self.sync_native(project);
                self.prepare_native_layout();
                self.render_native(ui, ctx);
                return true;
            }
        }
        if matches!(
            event,
            TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Delete))
        ) && self
            .native
            .views
            .get(&key)
            .is_some_and(|view| ctx.ui.has_focus(&view.id))
        {
            if self.native.views[&key].panels.lock().unwrap().is_open() {
                return false;
            }
            let model = self.native.views[&key].model.lock().unwrap();
            if model.editor().text_focus().is_some() {
                return false;
            }
            let selected = model.editor().selected;
            let connection = model.editor().selected_connection;
            drop(model);
            let result = self
                .native
                .edit_source(project, &key, &mut ctx.text, |doc, defs| {
                    if let Some(wire) = connection {
                        doc.connections.retain(|connection| connection.id != wire);
                    } else if let Some(node) = selected {
                        remove_native_nodes(doc, &[node].into_iter().collect());
                    }
                    let to = if key.kind() == NodeGraphKind::Behavior
                        && !doc.nodes.is_empty()
                        && !doc.nodes.iter().any(|node| is_branch_trigger(node, defs))
                    {
                        NodeSourceKey {
                            owner: key.owner.clone(),
                            branch: None,
                        }
                    } else {
                        key.clone()
                    };
                    Ok(to)
                });
            self.native_status(ctx, result);
            self.native_effects(project, ctx, reference);
            self.sync_native(project);
            self.render_native(ui, ctx);
            return true;
        }
        false
    }
}
fn remove_native_nodes(document: &mut GraphDocument, nodes: &std::collections::HashSet<GraphId>) {
    let ports: std::collections::HashSet<_> = document
        .nodes
        .iter()
        .filter(|node| nodes.contains(&node.id))
        .flat_map(|node| node.ports.iter().map(|port| port.id))
        .collect();
    document.nodes.retain(|node| !nodes.contains(&node.id));
    document
        .connections
        .retain(|wire| !ports.contains(&wire.from) && !ports.contains(&wire.to));
}

#[cfg(test)]
mod tests {
    use super::*;
    use theframework::{
        paint::{ImageHandle, PaintList},
        platform::*,
        text_edit::EditorClipboard,
        theui::paint::{UiImageResources, UiPaintError},
    };
    struct NoImages;
    impl UiImageResources for NoImages {
        fn image(
            &mut self,
            _: &str,
            _: &TheRGBABuffer,
            _: bool,
        ) -> Result<ImageHandle, UiPaintError> {
            panic!("native node dock cannot rasterize its graph or chrome");
        }
    }
    struct Fixture {
        dock: NodesDock,
        ui: TheUI,
        ctx: TheContext,
        project: Project,
        server: CreatorContext,
        character: GraphId,
        events: std::sync::mpsc::Receiver<TheEvent>,
    }
    impl Fixture {
        fn new() -> Self {
            let mut project = Project::default();
            let character = shared::character::Character::new();
            let id = character.id;
            project.characters.insert(id, character);
            let definitions = catalog::definitions();
            for owner in [
                "behavior/world".to_owned(),
                format!("behavior/character/{id}"),
            ] {
                let root = definitions.instantiate("event", [20., 30.]).unwrap();
                let mut say = definitions.instantiate("say", [450., 30.]).unwrap();
                say.branch = Some(root.id);
                project.node_graphs.insert(owner,serde_json::json!({"version":1,"branches":[GraphDocument {nodes:vec![root,say],..Default::default()}]}));
            }
            let mut ctx = TheContext::new(1100, 760, 1.5);
            ctx.ui.command_paint = true;
            let mut ui = TheUI::new();
            ui.init(&mut ctx);
            ui.set_theme(Box::new(TheBlackBlueTheme::new()), &mut ctx);
            let events = ui.add_state_listener("native-nodes-dock".into());
            let mut dock = NodesDock::new();
            ui.canvas = dock.setup(&mut ctx);
            assert!(ui.get_render_view(VIEW).is_none());
            assert!(
                ui.get_widget(VIEW)
                    .unwrap()
                    .as_any()
                    .is::<NativeNodesView>()
            );
            ui.prepare_paint_layout(&mut ctx);
            let mut server = CreatorContext::default();
            server.shared.pc = ProjectContext::WorldCode;
            let before = serde_json::to_value(&project).unwrap();
            dock.activate(&mut ui, &mut ctx, &project, &mut server);
            let mut f = Self {
                dock,
                ui,
                ctx,
                project,
                server,
                character: id,
                events,
            };
            f.pump();
            f.paint();
            assert_eq!(serde_json::to_value(&f.project).unwrap(), before);
            assert!(f.dock.resources.is_none());
            f
        }
        fn paint(&mut self) -> PaintList {
            self.ui
                .paint_commands(&mut self.ctx, &mut NoImages)
                .unwrap()
        }
        fn pump(&mut self) {
            for _ in 0..64 {
                self.ui.process_events(&mut self.ctx);
                let events: Vec<_> = self.events.try_iter().collect();
                if events.is_empty() {
                    return;
                }
                for event in events {
                    self.dock.handle_event(
                        &event,
                        &mut self.ui,
                        &mut self.ctx,
                        &mut self.project,
                        &mut self.server,
                    );
                }
            }
            panic!("native node dock events must settle");
        }
        fn show(&mut self, context: ProjectContext) {
            self.server.shared.pc = context;
            self.dock
                .activate(&mut self.ui, &mut self.ctx, &self.project, &mut self.server);
        }
        fn model(&self) -> Arc<Mutex<GraphViewState>> {
            self.dock.native.views[self.dock.native.active.as_ref().unwrap()]
                .model
                .clone()
        }
        fn diagram(&mut self, selected: Option<GraphId>) {
            let id = self.ui.get_widget(VIEW).unwrap().id().clone();
            self.ctx.ui.set_focus(&id);
            self.ui
                .get_widget(VIEW)
                .unwrap()
                .editor_set_active(true, &mut self.ctx);
            self.model().lock().unwrap().edit(|_, editor| {
                editor.selected = selected;
                editor.selected_connection = None;
            });
        }
        fn clipboard_request(&mut self, command: EditorCommand) -> EditorClipboard {
            self.ui
                .get_widget(VIEW)
                .unwrap()
                .editor_command(command, &mut self.ctx);
            self.pump();
            let operation = self.ctx.ui.editor_clipboard.pop_front().unwrap();
            assert!(self.ctx.ui.editor_clipboard.is_empty());
            operation
        }
        fn send(&mut self, event: InputEvent) -> InputResponse {
            self.ui.prepare_paint_layout(&mut self.ctx);
            self.ui.dispatch_input(&event, &mut self.ctx)
        }
        fn click(&mut self, position: [f32; 2]) {
            for pressed in [true, false] {
                self.send(InputEvent::PointerButton {
                    button: PointerButton::Primary,
                    pressed,
                    position,
                });
            }
        }
        fn text(&mut self, text: &str) -> TextInputTarget {
            let dim = *self.ui.get_widget(VIEW).unwrap().dim();
            self.click([
                (dim.x + dim.width - 10) as f32,
                (dim.y + dim.height - 10) as f32,
            ]);
            let model = self.model();
            let model = model.lock().unwrap();
            let node = model
                .document()
                .nodes
                .iter()
                .find(|node| node.definition.as_deref() == Some("say"))
                .unwrap();
            let index = node
                .rows
                .iter()
                .position(|row| matches!(row.value, GraphControlValue::Text(_)))
                .unwrap();
            let rect = node.row_rect(index, &model.document().metrics());
            let p = model
                .editor()
                .viewport
                .to_screen([rect.origin[0] + 10., rect.origin[1] + 10.]);
            drop(model);
            self.click([dim.x as f32 + p[0], dim.y as f32 + p[1]]);
            self.ui
                .get_widget(VIEW)
                .unwrap()
                .editor_command(EditorCommand::SelectAll, &mut self.ctx);
            let target = self
                .ui
                .get_widget(VIEW)
                .unwrap()
                .editor_input_state(&mut self.ctx)
                .unwrap()
                .target;
            assert!(self.send(InputEvent::Text(text.into())).consumed);
            target
        }
        fn saved(&self, owner: &str) -> String {
            let definitions = catalog::definitions();
            let graphs = shared::graph_authoring::NodeGraphDocuments::from_value(
                &self.project.node_graphs[owner],
                NodeGraphKind::Behavior,
                &definitions,
            )
            .unwrap();
            graphs
                .documents()
                .iter()
                .flat_map(|doc| &doc.nodes)
                .find(|node| node.definition.as_deref() == Some("say"))
                .unwrap()
                .rows
                .iter()
                .find_map(|row| {
                    if let GraphControlValue::Text(text) = &row.value {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
                .unwrap()
        }
        fn drop_node(&mut self, definition: &str, point: Point) {
            let key = self.dock.native.active.as_ref().unwrap();
            let view = &self.dock.native.views[key];
            let mut drop = TheDrop::new(TheId::named(&format!("Node Catalog/{definition}")));
            drop.data = definition.into();
            let event = TheEvent::GraphDrop(
                view.id.clone(),
                view.model.lock().unwrap().binding(),
                point,
                drop,
            );
            self.dock.handle_event(
                &event,
                &mut self.ui,
                &mut self.ctx,
                &mut self.project,
                &mut self.server,
            );
        }
    }
    #[test]
    fn native_nodes_dock_mounts_retained_widgets_and_saves_hidden_owner_text_without_a_graph_bitmap()
     {
        let isolated = std::env::var_os("ELDIRON_NATIVE_NODE_DOCK_ISOLATED").is_some();
        if isolated {
            // No global reference owner exists; data-only APIs receive no service.
        }

        let mut f = Fixture::new();
        assert!(!f.server.jobs.scene_initialized());
        assert!(!f.server.reference.initialized());
        assert!(
            f.ui.get_widget(VIEW)
                .unwrap()
                .as_any()
                .is::<NativeNodesView>()
        );
        assert!(f.ui.get_render_view(VIEW).is_none());
        let target = f.text("World 東京");
        f.send(InputEvent::CompositionFor {
            target,
            text: "日本".into(),
            cursor: Some((0, 6)),
        });
        f.show(ProjectContext::CharacterCode(f.character));
        f.pump();
        assert_eq!(f.saved("behavior/world"), "World 東京");
        f.text("Character e\u{301}");
        assert!(
            f.dock
                .commit_pending_edits(&mut f.ctx, &mut f.project, &f.server.jobs, &f.server.reference, &f.server.history)
                .unwrap()
        );
        assert_eq!(
            f.saved(&format!("behavior/character/{}", f.character)),
            "Character e\u{301}"
        );
        assert_eq!(f.saved("behavior/world"), "World 東京");
        f.paint();
        assert!(f.ui.canvas.buffer.pixels().is_empty());
        assert!(f.dock.resources.is_none());
        assert!(
            !f.send(InputEvent::TextFor {
                target,
                text: "stale".into()
            })
            .consumed
        );
        f.show(ProjectContext::WorldCode);
        f.pump();
        f.dock
            .undo(&mut f.ui, &mut f.ctx, &mut f.project, &mut f.server);
        assert_ne!(f.saved("behavior/world"), "World 東京");
        assert_eq!(
            f.saved(&format!("behavior/character/{}", f.character)),
            "Character e\u{301}"
        );
        if isolated {
            assert_no_world_owners(&f.server.jobs);
        }
    }
    #[test]
    fn native_nodes_dock_drop_new_branch_toolbar_copy_paste_and_hidden_undo_keep_their_source() {
        let mut f = Fixture::new();
        let original = f.dock.native.active.clone().unwrap();
        let retired_toolbar = f.ui.get_widget(CLEAR_NODES).unwrap().id().clone();
        f.drop_node("time_range", [123.25, 234.75]);
        let model = f.model();
        assert!(
            model
                .lock()
                .unwrap()
                .document()
                .nodes
                .iter()
                .any(|node| node.definition.as_deref() == Some("time_range")
                    && node.position == [123.25, 234.75])
        );
        f.drop_node("event", [250.5, 180.25]);
        let branch = f.dock.native.active.clone().unwrap();
        assert_ne!(branch, original);
        let before = serde_json::to_value(&f.project).unwrap();
        assert!(!f.dock.handle_event(
            &TheEvent::StateChanged(retired_toolbar, TheWidgetState::Clicked),
            &mut f.ui,
            &mut f.ctx,
            &mut f.project,
            &mut f.server
        ));
        assert_eq!(serde_json::to_value(&f.project).unwrap(), before);
        assert_eq!(
            f.dock
                .native
                .cpu
                .graphs(&f.project, &branch.owner, &catalog::definitions())
                .unwrap()
                .order
                .len(),
            2
        );
        for action in [COPY_BRANCH, PASTE_BRANCH] {
            let id = f.ui.get_widget(action).unwrap().id().clone();
            f.dock.handle_event(
                &TheEvent::StateChanged(id, TheWidgetState::Clicked),
                &mut f.ui,
                &mut f.ctx,
                &mut f.project,
                &mut f.server,
            );
        }
        let pasted = f.dock.native.active.clone().unwrap();
        assert_ne!(pasted, branch);
        assert_eq!(
            f.dock
                .native
                .cpu
                .graphs(&f.project, &branch.owner, &catalog::definitions())
                .unwrap()
                .order
                .len(),
            3
        );
        let view = &f.dock.native.views[&pasted];
        let event = TheEvent::GraphCommand(
            view.id.clone(),
            view.model.lock().unwrap().binding(),
            EditorCommand::Undo,
        );
        f.show(ProjectContext::CharacterCode(f.character));
        let visible = f.dock.native.active.clone();
        f.dock
            .handle_event(&event, &mut f.ui, &mut f.ctx, &mut f.project, &mut f.server);
        assert_eq!(f.dock.native.active, visible);
        assert_eq!(
            f.dock
                .native
                .cpu
                .graphs(&f.project, &branch.owner, &catalog::definitions())
                .unwrap()
                .order
                .len(),
            2
        );
        f.pump();
        f.paint();
    }
    #[test]
    fn native_nodes_dock_rejects_retired_toolbar_resize_and_project_callbacks() {
        let mut f = Fixture::new();
        let toolbar = f.ui.get_widget(CLEAR_NODES).unwrap().id().clone();
        let key = f.dock.native.active.clone().unwrap();
        let id = f.dock.native.views[&key].id.clone();
        let old = f.model();
        let binding = old.lock().unwrap().binding();
        f.show(ProjectContext::CharacterCode(f.character));
        let visible = f.dock.native.active.clone();
        let before = serde_json::to_value(&f.project).unwrap();
        assert!(!f.dock.handle_event(
            &TheEvent::StateChanged(toolbar, TheWidgetState::Clicked),
            &mut f.ui,
            &mut f.ctx,
            &mut f.project,
            &mut f.server
        ));
        assert!(!f.dock.handle_event(
            &TheEvent::WidgetResized(id.clone(), TheDim::new(0, 0, 333, 222)),
            &mut f.ui,
            &mut f.ctx,
            &mut f.project,
            &mut f.server
        ));
        assert_eq!(f.dock.native.active, visible);
        assert_eq!(serde_json::to_value(&f.project).unwrap(), before);
        f.dock.reset_for_project_switch();
        assert!(old.lock().unwrap().document().nodes.is_empty());
        assert!(f.dock.native.cpu.binding_source(binding).is_none());
        f.ctx.ui.command_paint = true;
        f.show(ProjectContext::WorldCode);
        let before = serde_json::to_value(&f.project).unwrap();
        assert!(!f.dock.handle_event(
            &TheEvent::GraphCommitted(id, Arc::new(GraphCommit::new(binding, 1, vec![]))),
            &mut f.ui,
            &mut f.ctx,
            &mut f.project,
            &mut f.server
        ));
        assert_eq!(serde_json::to_value(&f.project).unwrap(), before);
    }
    #[test]
    fn native_nodes_dock_equipment_drop_is_reversible_source_owned_and_keeps_rejection_feedback() {
        let mut f = Fixture::new();
        f.project.characters.get_mut(&f.character).unwrap().data =
            "[attributes]\nrace='Human'\nclass='Warrior'\nstart_equipped_items=[]\n".into();
        let mut shirt = shared::item::Item::new();
        shirt.name = "Native linen".into();
        shirt.data =
            "[attributes]\nruleset_id='linen_shirt'\nslot='torso'\ncategory='cloth'\n".into();
        let item = shirt.id;
        f.project.items.insert(item, shirt);
        let mut other = shared::item::Item::new();
        other.name = "Other cloth".into();
        other.data = "[attributes]\nslot='torso'\ncategory='cloth'\n".into();
        let other_id = other.id;
        f.project.items.insert(other_id, other);
        let mut junk = shared::item::Item::new();
        junk.name = "Not equipment".into();
        junk.data = "[attributes]\ncategory='junk'\n".into();
        let junk_id = junk.id;
        f.project.items.insert(junk_id, junk);
        f.show(ProjectContext::CharacterData(f.character));
        f.pump();
        let key = f.dock.native.active.clone().unwrap();
        let point = {
            let model = f.model();
            let model = model.lock().unwrap();
            let node = model
                .document()
                .nodes
                .iter()
                .find(|node| node.definition.as_deref() == Some("entity_equipment"))
                .unwrap();
            [
                node.position[0] + node.width * 0.5 + 0.125,
                node.position[1] + 12.25,
            ]
        };
        let event = |f: &Fixture, item| {
            let view = &f.dock.native.views[&key];
            TheEvent::GraphDrop(
                view.id.clone(),
                view.model.lock().unwrap().binding(),
                point,
                TheDrop::new(TheId::named_with_reference("Item Item", item)),
            )
        };
        let names = |f: &Fixture| {
            f.project.characters[&f.character]
                .data
                .parse::<toml::Table>()
                .unwrap()["attributes"]["start_equipped_items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap().to_owned())
                .collect::<Vec<_>>()
        };
        for _ in 0..2 {
            let event = event(&f, item);
            assert!(f.dock.handle_event(
                &event,
                &mut f.ui,
                &mut f.ctx,
                &mut f.project,
                &mut f.server
            ));
        }
        assert_eq!(names(&f), vec![item.to_string()]);
        f.dock
            .undo(&mut f.ui, &mut f.ctx, &mut f.project, &mut f.server);
        assert!(names(&f).is_empty());
        f.dock
            .redo(&mut f.ui, &mut f.ctx, &mut f.project, &mut f.server);
        assert_eq!(names(&f), vec![item.to_string()]);
        f.dock.mark_saved();
        let before = serde_json::to_value(&f.project).unwrap();
        let invalid = event(&f, junk_id);
        assert!(f.dock.handle_event(
            &invalid,
            &mut f.ui,
            &mut f.ctx,
            &mut f.project,
            &mut f.server
        ));
        assert_eq!(serde_json::to_value(&f.project).unwrap(), before);
        let feedback = f.model().lock().unwrap().editor().error.clone();
        assert!(feedback.is_some());
        assert!(!f.dock.has_changes());
        assert!(
            !f.dock
                .commit_pending_edits(&mut f.ctx, &mut f.project, &f.server.jobs, &f.server.reference, &f.server.history)
                .unwrap()
        );
        assert_eq!(f.model().lock().unwrap().editor().error, feedback);
        assert_eq!(serde_json::to_value(&f.project).unwrap(), before);
        let hidden = event(&f, other_id);
        f.show(ProjectContext::WorldCode);
        let visible = f.dock.native.active.clone();
        assert!(f.dock.handle_event(
            &hidden,
            &mut f.ui,
            &mut f.ctx,
            &mut f.project,
            &mut f.server
        ));
        assert_eq!(f.dock.native.active, visible);
        assert_eq!(names(&f), vec![other_id.to_string()]);
        let mut loaded: Project =
            serde_json::from_value(serde_json::to_value(&f.project).unwrap()).unwrap();
        shared::entity_graph::synchronize(&mut loaded).unwrap();
        assert_eq!(
            loaded.characters[&f.character]
                .data
                .parse::<toml::Table>()
                .unwrap()["attributes"]["start_equipped_items"][0]
                .as_str(),
            Some(other_id.to_string().as_str())
        );
        f.paint();
    }
    fn assert_no_world_owners(jobs: &CreatorJobs) {
        // No global reference owner exists; data-only APIs receive no service.
        assert!(!jobs.scene_initialized());

        // Authoring cameras are CPU values owned by each Creator context.
    }
    #[test]
    fn native_nodes_dock_keyboard_undo_keeps_the_captured_source_after_owner_activation() {
        let mut f = Fixture::new();
        let key = f.dock.native.active.clone().unwrap();
        f.text("Keyboard-owned edit");
        let dim = *f.ui.get_widget(VIEW).unwrap().dim();
        f.click([
            (dim.x + dim.width - 8) as f32,
            (dim.y + dim.height - 8) as f32,
        ]);
        f.pump();
        assert_eq!(f.saved(&key.owner), "Keyboard-owned edit");
        assert!(f.ui.get_widget(VIEW).unwrap().supports_undo_redo());
        f.send(InputEvent::Modifiers(Modifiers {
            super_key: true,
            ..Default::default()
        }));
        assert!(
            f.send(InputEvent::Key {
                physical: PhysicalKey::Letter(b'Z'),
                logical: LogicalKey::Character("z".into()),
                pressed: true,
                repeat: false
            })
            .consumed
        );
        f.show(ProjectContext::CharacterCode(f.character));
        let visible = f.dock.native.active.clone();
        f.pump();
        assert_eq!(f.dock.native.active, visible);
        assert_ne!(f.saved(&key.owner), "Keyboard-owned edit");
        assert!(f.dock.native.can_redo(&key));
        f.paint();
    }
    #[test]
    fn native_nodes_dock_pastes_legacy_behavior_nodes_without_weakening_catalog_availability() {
        let mut f = Fixture::new();
        let source = f.model().lock().unwrap().document().clone();
        assert!(!node_available(ProjectContext::WorldCode, "say"));
        for action in [COPY_BRANCH, PASTE_BRANCH] {
            let id = f.ui.get_widget(action).unwrap().id().clone();
            assert!(f.dock.handle_event(
                &TheEvent::StateChanged(id, TheWidgetState::Clicked),
                &mut f.ui,
                &mut f.ctx,
                &mut f.project,
                &mut f.server
            ));
        }
        let copy = f.model().lock().unwrap().document().clone();
        assert!(
            copy.nodes
                .iter()
                .any(|node| node.definition.as_deref() == Some("say"))
        );
        let source_ids: std::collections::HashSet<_> =
            source.nodes.iter().map(|node| node.id).collect();
        assert!(copy.nodes.iter().all(|node| !source_ids.contains(&node.id)));
        let before = serde_json::to_value(&f.project).unwrap();
        f.drop_node("say", [500.25, 200.125]);
        assert_eq!(serde_json::to_value(&f.project).unwrap(), before);
    }
    #[test]
    fn native_nodes_clipboard_keyboard_copy_keeps_input_time_selection_without_project_changes() {
        let mut f = Fixture::new();
        let document = f.model().lock().unwrap().document().clone();
        let say = document
            .nodes
            .iter()
            .find(|n| n.definition.as_deref() == Some("say"))
            .unwrap();
        f.diagram(Some(say.id));
        let before = serde_json::to_value(&f.project).unwrap();
        f.send(InputEvent::Modifiers(Modifiers {
            super_key: true,
            ..Default::default()
        }));
        assert!(
            f.send(InputEvent::Key {
                physical: PhysicalKey::Letter(b'C'),
                logical: LogicalKey::Character("c".into()),
                pressed: true,
                repeat: false,
            })
            .consumed
        );
        // Application dispatch happens after selection has moved elsewhere.
        f.model()
            .lock()
            .unwrap()
            .edit(|_, editor| editor.selected = Some(document.nodes[0].id));
        f.pump();
        let Some(EditorClipboard::Write { text, cut: None }) =
            f.ctx.ui.editor_clipboard.pop_front()
        else {
            panic!("copy must request a host write")
        };
        let copy = GraphClipboard::decode(&text, "eldiron.behavior", 8 * 1024 * 1024).unwrap();
        assert_eq!(copy.document.nodes, vec![say.clone()]);
        assert_eq!(serde_json::to_value(&f.project).unwrap(), before);
        assert!(
            !f.dock
                .native
                .can_undo(f.dock.native.active.as_ref().unwrap())
        );
    }
    #[test]
    fn native_nodes_clipboard_cut_requires_successful_write_and_deletes_once_with_one_undo() {
        let mut f = Fixture::new();
        let document = f.model().lock().unwrap().document().clone();
        let say = document
            .nodes
            .iter()
            .find(|n| n.definition.as_deref() == Some("say"))
            .unwrap()
            .id;
        f.diagram(Some(say));
        let before = serde_json::to_value(&f.project).unwrap();
        let EditorClipboard::Write {
            cut: Some(failed), ..
        } = f.clipboard_request(EditorCommand::Cut)
        else {
            panic!("cut must request an acknowledged write")
        };
        assert_eq!(serde_json::to_value(&f.project).unwrap(), before);
        assert!(
            f.send(InputEvent::Clipboard {
                request: failed,
                result: ClipboardResult::Written(false)
            })
            .consumed
        );
        f.pump();
        assert_eq!(serde_json::to_value(&f.project).unwrap(), before);
        let EditorClipboard::Write {
            cut: Some(request), ..
        } = f.clipboard_request(EditorCommand::Cut)
        else {
            panic!()
        };
        assert_ne!(request, failed);
        assert!(
            f.send(InputEvent::Clipboard {
                request,
                result: ClipboardResult::Written(true)
            })
            .consumed
        );
        f.pump();
        assert_eq!(f.model().lock().unwrap().document().nodes.len(), 1);
        let after = serde_json::to_value(&f.project).unwrap();
        assert!(
            !f.send(InputEvent::Clipboard {
                request,
                result: ClipboardResult::Written(true)
            })
            .consumed
        );
        f.pump();
        assert_eq!(serde_json::to_value(&f.project).unwrap(), after);
        f.dock
            .undo(&mut f.ui, &mut f.ctx, &mut f.project, &mut f.server);
        assert_eq!(f.model().lock().unwrap().document(), &document);
        assert!(
            !f.dock
                .native
                .can_undo(f.dock.native.active.as_ref().unwrap())
        );
        f.dock
            .redo(&mut f.ui, &mut f.ctx, &mut f.project, &mut f.server);
        assert!(
            !f.model()
                .lock()
                .unwrap()
                .document()
                .nodes
                .iter()
                .any(|n| n.id == say)
        );
    }
    #[test]
    fn native_nodes_clipboard_paste_uses_frozen_graph_anchor_fresh_ids_and_source_history() {
        let mut f = Fixture::new();
        let document = f.model().lock().unwrap().document().clone();
        let say = document
            .nodes
            .iter()
            .find(|n| n.definition.as_deref() == Some("say"))
            .unwrap()
            .clone();
        let text = GraphClipboard::new(
            "eldiron.behavior",
            GraphDocument {
                nodes: vec![say.clone()],
                ..Default::default()
            },
        )
        .encode(100_000)
        .unwrap();
        f.diagram(None);
        let anchor = [222.125, 180.25];
        f.model()
            .lock()
            .unwrap()
            .edit(|_, editor| editor.cursor = editor.viewport.to_screen(anchor));
        let EditorClipboard::Read { request } = f.clipboard_request(EditorCommand::Paste) else {
            panic!("paste must request a host read")
        };
        f.model().lock().unwrap().edit(|_, editor| {
            editor.viewport.pan = [600., -400.];
            editor.viewport.zoom_at([100., 100.], 1.75);
        });
        assert!(
            f.send(InputEvent::Clipboard {
                request,
                result: ClipboardResult::Text(Some(text))
            })
            .consumed
        );
        f.pump();
        let model = f.model();
        let model = model.lock().unwrap();
        let pasted = model
            .document()
            .nodes
            .iter()
            .find(|n| !document.nodes.iter().any(|old| old.id == n.id))
            .unwrap();
        assert_ne!(pasted.id, say.id);
        for axis in 0..2 {
            assert!((pasted.position[axis] - anchor[axis]).abs() < 0.001);
        }
        assert_eq!(pasted.rows[0].value, say.rows[0].value);
        assert_eq!(model.editor().selected, Some(pasted.id));
        drop(model);
        f.dock
            .undo(&mut f.ui, &mut f.ctx, &mut f.project, &mut f.server);
        assert_eq!(f.model().lock().unwrap().document(), &document);
        assert!(
            !f.dock
                .native
                .can_undo(f.dock.native.active.as_ref().unwrap())
        );
    }
    #[test]
    fn native_nodes_clipboard_rejects_invalid_payloads_and_stale_owner_or_selection() {
        let mut f = Fixture::new();
        let document = f.model().lock().unwrap().document().clone();
        let say = document
            .nodes
            .iter()
            .find(|n| n.definition.as_deref() == Some("say"))
            .unwrap()
            .id;
        f.diagram(Some(say));
        let before = serde_json::to_value(&f.project).unwrap();
        for text in [
            None,
            Some("ordinary text".into()),
            Some(
                GraphClipboard::new("eldiron.configuration", document.clone())
                    .encode(100_000)
                    .unwrap(),
            ),
            Some(" ".repeat(8 * 1024 * 1024 + 1)),
        ] {
            let EditorClipboard::Read { request } = f.clipboard_request(EditorCommand::Paste)
            else {
                panic!()
            };
            f.send(InputEvent::Clipboard {
                request,
                result: ClipboardResult::Text(text),
            });
            f.pump();
            assert_eq!(serde_json::to_value(&f.project).unwrap(), before);
        }
        let EditorClipboard::Write {
            cut: Some(request), ..
        } = f.clipboard_request(EditorCommand::Cut)
        else {
            panic!()
        };
        f.diagram(None);
        f.send(InputEvent::Clipboard {
            request,
            result: ClipboardResult::Written(true),
        });
        f.pump();
        assert_eq!(serde_json::to_value(&f.project).unwrap(), before);
        f.diagram(Some(say));
        let EditorClipboard::Write {
            cut: Some(request), ..
        } = f.clipboard_request(EditorCommand::Cut)
        else {
            panic!()
        };
        f.show(ProjectContext::CharacterCode(f.character));
        f.pump();
        assert!(
            !f.send(InputEvent::Clipboard {
                request,
                result: ClipboardResult::Written(true)
            })
            .consumed
        );
        f.pump();
        assert_eq!(serde_json::to_value(&f.project).unwrap(), before);
    }
    #[test]
    fn native_nodes_clipboard_text_and_diagram_requests_cannot_complete_each_other() {
        let mut f = Fixture::new();
        f.text("Kept text");
        f.ui.paste(&mut f.ctx);
        let Some(EditorClipboard::Read {
            request: text_request,
        }) = f.ctx.ui.editor_clipboard.pop_front()
        else {
            panic!()
        };
        let dim = *f.ui.get_widget(VIEW).unwrap().dim();
        f.click([
            (dim.x + dim.width - 8) as f32,
            (dim.y + dim.height - 8) as f32,
        ]);
        f.pump();
        f.diagram(None);
        let EditorClipboard::Read { request } = f.clipboard_request(EditorCommand::Paste) else {
            panic!()
        };
        assert_ne!(request, text_request);
        let before = serde_json::to_value(&f.project).unwrap();
        assert!(
            !f.send(InputEvent::Clipboard {
                request: text_request,
                result: ClipboardResult::Text(Some("late text".into()))
            })
            .consumed
        );
        f.pump();
        assert_eq!(serde_json::to_value(&f.project).unwrap(), before);
        assert!(f.dock.native_clipboard.is_some());
        assert!(
            f.send(InputEvent::Clipboard {
                request,
                result: ClipboardResult::Text(None)
            })
            .consumed
        );
        f.pump();
        assert!(f.dock.native_clipboard.is_none());
        assert_eq!(f.saved("behavior/world"), "Kept text");
    }
    #[cfg(feature = "native-render")]
    #[test]
    #[ignore = "requires a native GPU"]
    fn native_nodes_dock_gpu_composes_toolbar_branches_and_ime_without_a_raster_graph() {
        use renderkit::*;
        use theframework::render_wgpu::{PaintRenderer, UiImages};
        let mut f = Fixture::new();
        f.show(ProjectContext::CharacterCode(f.character));
        f.pump();
        let target = f.text("Native 東京 and e\u{301}");
        f.send(InputEvent::CompositionFor {
            target,
            text: "日本".into(),
            cursor: Some((0, 6)),
        });
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let mut images = UiImages::new(gpu.clone());
        let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
        let output = GpuImage::new(&gpu, ImageDescriptor::rgba(1650, 1140)).unwrap();
        let list = f.ui.paint_commands(&mut f.ctx, &mut images).unwrap();
        assert_eq!(list.image_handles().count(), 0);
        assert!(f.ui.canvas.buffer.pixels().is_empty());
        assert!(f.dock.resources.is_none());
        images.finish_frame(&list).unwrap();
        renderer
            .render(
                &list,
                &images.registry,
                &output.view,
                output.descriptor.format,
                (1650, 1140),
                1.5,
                [0.02, 0.02, 0.02, 1.],
            )
            .unwrap();
        assert!(renderer.glyphs.rasterizations > 0);
        assert_eq!(gpu.transfers().readback_bytes, 0);
        let mut ticket = ReadbackTicket::capture(&gpu, output).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(30)),
            })
            .unwrap();
        let capture = ticket.try_finish(&gpu).unwrap().unwrap();
        let file = std::fs::File::create("/private/tmp/eldiron-native-nodes-dock.png").unwrap();
        let mut encoder = png::Encoder::new(file, 1650, 1140);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(capture.pixels())
            .unwrap();
        let state =
            f.ui.get_widget(VIEW)
                .unwrap()
                .editor_input_state(&mut f.ctx)
                .unwrap();
        assert_eq!(state.target, target);
        assert!(f.model().lock().unwrap().editor().text_focus().is_some());
    }
    #[test]
    fn native_nodes_dock_scroll_preserves_equal_axis_policy_in_a_rectangular_viewport() {
        let mut f = Fixture::new();
        let dim = *f.ui.get_widget(VIEW).unwrap().dim();
        assert_ne!(dim.width, dim.height);
        f.send(InputEvent::PointerMoved {
            position: [dim.x as f32 + 8., dim.y as f32 + 8.],
        });
        let before = serde_json::to_value(&f.project).unwrap();
        let model = f.model();
        let pan = model.lock().unwrap().editor().viewport.pan;
        f.send(InputEvent::Scroll {
            delta: [7.5, -12.25],
            precise: true,
        });
        let after = model.lock().unwrap().editor().viewport.pan;
        assert!(
            (after[0] - pan[0] - 4.5).abs() < 0.0001,
            "horizontal sensitivity: {pan:?} -> {after:?}"
        );
        assert!(
            (after[1] - pan[1] + 7.35).abs() < 0.0001,
            "vertical sensitivity: {pan:?} -> {after:?}"
        );
        assert!(model.lock().unwrap().pending_commits().next().is_none());
        assert_eq!(serde_json::to_value(&f.project).unwrap(), before);
        f.paint();
    }
    #[test]
    fn native_nodes_dock_pinch_preserves_the_graph_point_beneath_the_pointer() {
        let mut f = Fixture::new();
        let dim = *f.ui.get_widget(VIEW).unwrap().dim();
        let anchor = [80., 80.];
        f.send(InputEvent::PointerMoved {
            position: [dim.x as f32 + anchor[0], dim.y as f32 + anchor[1]],
        });
        let model = f.model();
        let before = model.lock().unwrap().editor().viewport;
        let point = before.to_graph(anchor);
        let source = serde_json::to_value(&f.project).unwrap();
        f.send(InputEvent::Pinch(0.15));
        let after = model.lock().unwrap().editor().viewport;
        assert!(
            (after.zoom() / before.zoom() - 1.15).abs() < 0.0001,
            "pinch must reach the native graph"
        );
        let actual = after.to_graph(anchor);
        assert!((actual[0] - point[0]).abs() < 0.0001 && (actual[1] - point[1]).abs() < 0.0001);
        assert!(model.lock().unwrap().pending_commits().next().is_none());
        assert_eq!(serde_json::to_value(&f.project).unwrap(), source);
    }
}
