//! Application-owned view sessions around the shared CPU authoring workspace.
//! No renderer, window, Project clone or raster/font cache is created here.
use super::native_view::NativeNodePanels;
use super::*;
use shared::graph_authoring::{
    NodeGraphKind, NodeGraphWorkspace, NodeSourceChange, NodeSourceError, NodeSourceKey,
};
use std::sync::{Arc, Mutex};
use theframework::text::TextSystem;

pub(super) struct NativeNodeSourceView {
    pub model: Arc<Mutex<GraphViewState>>,
    pub panels: Arc<Mutex<NativeNodePanels>>,
    pub id: TheId,
    pub context: ProjectContext,
    pub shown: Option<GraphId>,
    revision: u64,
}
#[derive(Default)]
pub(super) struct NativeNodesWorkspace {
    pub cpu: NodeGraphWorkspace,
    pub views: HashMap<NodeSourceKey, NativeNodeSourceView>,
    pub active: Option<NodeSourceKey>,
    selected: HashMap<String, Option<GraphId>>,
    changes: Vec<NodeSourceChange>,
}
pub(super) fn source_definitions(project: &Project, key: &NodeSourceKey) -> GraphDefinitions {
    match key.kind() {
        NodeGraphKind::Configuration => entity::definitions(project),
        NodeGraphKind::Rules => shared::rulesets::graph::definitions(),
        NodeGraphKind::Behavior => catalog::definitions_for_project(project),
    }
}
pub(super) fn project_source_graph(
    document: &mut GraphDocument,
    baseline: &GraphDocument,
    project: &Project,
    key: &NodeSourceKey,
    definitions: &GraphDefinitions,
) -> Option<String> {
    let validation = if key.kind() == NodeGraphKind::Configuration {
        configuration::project_graph(document, baseline, &key.owner, definitions, project).error
    } else {
        None
    };
    catalog::hydrate_lookout_distances(document, project);
    catalog::sync_fields(document, definitions);
    if key.kind() == NodeGraphKind::Configuration {
        assign_branch_owners(document, definitions, None);
    } else {
        for node in &mut document.nodes {
            node.branch = key.branch;
        }
    }
    validation
}
fn model_owner(key: &NodeSourceKey) -> String {
    format!(
        "nodes/{}/{}",
        key.owner,
        key.branch
            .map(|id| id.to_string())
            .unwrap_or_else(|| "document".into())
    )
}
impl NativeNodesWorkspace {
    fn pending(&self, key: &NodeSourceKey) -> bool {
        self.views.get(key).is_some_and(|view| {
            let panels = view.panels.lock().unwrap();
            let model = view.model.lock().unwrap();
            panels.has_changes()
                || model.pending_commits().next().is_some()
                || self.cpu.source(key).is_some_and(|source| {
                    source.error().is_some()
                        || (model.editor().text_focus().is_some()
                            && model.document() != source.document())
                })
        })
    }
    pub fn has_pending(&self) -> bool {
        self.cpu.diagnostics().next().is_some() || self.views.keys().any(|key| self.pending(key))
    }
    fn suspend(&mut self, key: &NodeSourceKey, fonts: &mut TextSystem) -> Result<(), String> {
        let Some(view) = self.views.get(key) else {
            return Ok(());
        };
        let mut panels = view.panels.lock().unwrap();
        let mut model = view.model.lock().unwrap();
        model.edit(|document, editor| editor.cancel_gesture(document));
        let result = panels
            .checkpoint(&mut model)
            .map_err(|error| error.to_string());
        model.finish_text(true);
        model.publish_edits();
        panels.suspend(fonts);
        result.map(|_| ())
    }
    /// Activation is read-only to Project. An old owner's completed edits remain
    /// queued even when its notification has not reached the application yet.
    pub fn activate(
        &mut self,
        project: &Project,
        context: ProjectContext,
        branch: Option<GraphId>,
        fonts: &mut TextSystem,
    ) -> Result<Option<NodeSourceKey>, String> {
        let Some(owner) = NodesDock::owner(context) else {
            if let Some(previous) = self.active.clone() {
                let _ = self.suspend(&previous, fonts);
            }
            self.active = None;
            return Ok(None);
        };
        let prototype = NodeSourceKey {
            owner: owner.clone(),
            branch: None,
        };
        if self
            .active
            .as_ref()
            .is_some_and(|previous| previous.owner != owner)
        {
            let previous = self.active.take().unwrap();
            let _ = self.suspend(&previous, fonts);
        }
        let definitions = source_definitions(project, &prototype);
        let graphs = self
            .cpu
            .graphs(project, &owner, &definitions)
            .map_err(|error| error.to_string())?;
        if prototype.kind() != NodeGraphKind::Configuration
            && branch.is_some_and(|root| !graphs.branches.contains_key(&root))
        {
            return Err(NodeSourceError::MissingBranch.to_string());
        }
        let chosen = if prototype.kind() == NodeGraphKind::Configuration {
            None
        } else {
            branch
                .or_else(|| self.selected.get(&owner).copied().flatten())
                .filter(|root| graphs.branches.contains_key(root))
                .or_else(|| graphs.order.first().copied())
        };
        let key = NodeSourceKey {
            owner: owner.clone(),
            branch: chosen,
        };
        if self.active.as_ref() != Some(&key)
            && let Some(previous) = self.active.clone()
        {
            let _ = self.suspend(&previous, fonts);
        }
        let preserve = self.pending(&key);
        let source = self
            .cpu
            .load(project, key.clone(), &definitions, preserve)
            .map_err(|error| error.to_string())?;
        let revision = source.revision();
        self.cpu
            .project_source(project, &key, |doc, base, project| {
                project_source_graph(doc, base, project, &key, &definitions)
            })
            .map_err(|error| error.to_string())?;
        let source = self.cpu.source(&key).unwrap();
        let document = source.document().clone();
        let recreate = self
            .views
            .get(&key)
            .is_none_or(|view| view.revision != revision);
        if recreate {
            if let Some(old) = self.views.remove(&key) {
                old.panels.lock().unwrap().revoke();
                old.model
                    .lock()
                    .unwrap()
                    .replace_document("retired-source", GraphDocument::default());
            }
            let mut model = GraphViewState::new(model_owner(&key), document);
            let shown = if key.kind() == NodeGraphKind::Configuration {
                graph_branches(model.document(), &definitions)
                    .branches
                    .first()
                    .map(|branch| branch.root)
            } else {
                key.branch
            };
            let visible = if key.kind() == NodeGraphKind::Configuration {
                let branches = graph_branches(model.document(), &definitions);
                shown.and_then(|root| branches.branch(root).map(|branch| branch.nodes.clone()))
            } else {
                None
            };
            model.edit(|_, editor| {
                *editor = node_editor();
                editor.set_visible(visible);
            });
            self.cpu
                .bind(&key, model.binding())
                .map_err(|error| error.to_string())?;
            self.views.insert(
                key.clone(),
                NativeNodeSourceView {
                    model: Arc::new(Mutex::new(model)),
                    panels: Arc::new(Mutex::new(NativeNodePanels::default())),
                    id: TheId::named(VIEW),
                    context,
                    shown,
                    revision,
                },
            );
        } else {
            let view = &self.views[&key];
            let mut model = view.model.lock().unwrap();
            let baseline = self.cpu.source(&key).unwrap().document().clone();
            model.edit(|doc, _| {
                project_source_graph(doc, &baseline, project, &key, &definitions);
            });
        }
        if key.kind() == NodeGraphKind::Configuration
            && let Some(root) = branch
        {
            self.show_configuration_branch(&key, root, &definitions, fonts)?;
        }
        self.active = Some(key.clone());
        self.selected.insert(owner, chosen);
        self.refresh_errors();
        Ok(Some(key))
    }
    fn show_configuration_branch(
        &mut self,
        key: &NodeSourceKey,
        root: GraphId,
        definitions: &GraphDefinitions,
        fonts: &mut TextSystem,
    ) -> Result<(), String> {
        let view = &self.views[key];
        if view.shown == Some(root) {
            return Ok(());
        }
        let visible = graph_branches(view.model.lock().unwrap().document(), definitions)
            .branch(root)
            .map(|branch| branch.nodes.clone())
            .ok_or_else(|| NodeSourceError::MissingBranch.to_string())?;
        self.suspend(key, fonts)?;
        let view = self.views.get_mut(key).unwrap();
        view.panels.lock().unwrap().revoke();
        view.model
            .lock()
            .unwrap()
            .edit(|_, editor| editor.set_visible(Some(visible)));
        view.shown = Some(root);
        // Source binding/queue stay owned by the whole configuration document;
        // stale resize/drop events keep the retired widget's input identity.
        view.id = TheId::named(VIEW);
        Ok(())
    }
    fn refresh_errors(&mut self) {
        for (key, view) in &self.views {
            let error = self.cpu.source(key).and_then(|source| {
                source
                    .error()
                    .map(ToString::to_string)
                    .or_else(|| source.validation().map(str::to_owned))
            });
            let mut model = view.model.lock().unwrap();
            if model.editor().error != error {
                model.edit(|_, editor| editor.error = error);
            }
        }
    }
    pub fn collect_through(
        &mut self,
        project: &mut Project,
        sequence: u64,
    ) -> Result<bool, String> {
        let missing: Vec<_> = self
            .views
            .keys()
            .filter(|key| !shared::graph_authoring::node_owner_exists(project, &key.owner))
            .cloned()
            .collect();
        for key in missing {
            if let Some(view) = self.views.remove(&key) {
                view.panels.lock().unwrap().revoke();
                view.model
                    .lock()
                    .unwrap()
                    .replace_document("deleted-source", GraphDocument::default());
            }
            self.cpu.remove(&key);
            if self.active.as_ref() == Some(&key) {
                self.active = None;
            }
        }
        let mut packets = vec![];
        for view in self.views.values() {
            let model = view.model.lock().unwrap();
            packets.extend(
                model
                    .pending_through(u64::MAX)
                    .into_iter()
                    .filter(|packet| packet.sequence <= sequence)
                    .map(|packet| (view.model.clone(), packet)),
            );
        }
        packets.sort_by_key(|(_, packet)| packet.sequence);
        let mut changed = false;
        let mut error = None;
        for (model, packet) in packets {
            let Some(key) = self.cpu.binding_source(packet.binding).cloned() else {
                continue;
            };
            let definitions = source_definitions(project, &key);
            match self
                .cpu
                .apply_commit(project, &packet, &definitions, |doc, base, project| {
                    project_source_graph(doc, base, project, &key, &definitions)
                }) {
                Ok(outcome) => {
                    if let Some(change) = outcome.change {
                        self.changes.push(change);
                        changed = true;
                    }
                    if !model
                        .lock()
                        .unwrap()
                        .acknowledge(packet.binding, packet.serial)
                    {
                        error = Some("Node source changed while acknowledging an edit".into());
                    }
                    if let Some(diagnostic) = outcome.error {
                        error = Some(diagnostic.to_string());
                    }
                    let baseline = self.cpu.source(&key).unwrap().document().clone();
                    model.lock().unwrap().edit(|doc, _| {
                        project_source_graph(doc, &baseline, project, &key, &definitions);
                    });
                }
                Err(diagnostic) => error = Some(diagnostic.to_string()),
            }
        }
        self.refresh_errors();
        if let Some(error) = error.or_else(|| {
            self.cpu
                .diagnostics()
                .next()
                .map(|(_, error)| error.to_string())
        }) {
            Err(error)
        } else {
            Ok(changed)
        }
    }
    pub fn checkpoint_all(&mut self, project: &mut Project) -> Result<bool, String> {
        let mut error = None;
        for view in self.views.values() {
            let mut panels = view.panels.lock().unwrap();
            let mut model = view.model.lock().unwrap();
            if let Err(diagnostic) = panels.checkpoint(&mut model) {
                error = Some(diagnostic.to_string());
            }
        }
        let result = self.collect_through(project, u64::MAX);
        if let Some(error) = error {
            Err(error)
        } else {
            result
        }
    }
    pub fn take_changes(&mut self) -> Vec<NodeSourceChange> {
        std::mem::take(&mut self.changes)
    }
    /// Read-only activation can schedule retained prefixes after the UI owner
    /// switch without borrowing Project mutably or invoking a reentrant hook.
    pub fn notifications(&self) -> Vec<(TheId, Arc<GraphCommit>)> {
        self.views
            .values()
            .filter_map(|view| {
                view.model
                    .lock()
                    .unwrap()
                    .pending_through(u64::MAX)
                    .last()
                    .cloned()
                    .map(|packet| (view.id.clone(), packet))
            })
            .collect()
    }
    pub fn reset(&mut self) {
        for view in self.views.values() {
            view.panels.lock().unwrap().revoke();
            view.model
                .lock()
                .unwrap()
                .replace_document("closed-project", GraphDocument::default());
        }
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::super::native_view::NativeNodesView;
    use super::*;
    use theframework::{
        paint::{ImageHandle, PaintList},
        platform::*,
        text_edit::EditorCommand,
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
            panic!("native source view must not rasterize a graph panel");
        }
    }
    struct Fixture {
        registry: NativeNodesWorkspace,
        project: Project,
        ui: TheUI,
        ctx: TheContext,
        character: Uuid,
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
                let mut event = definitions.instantiate("event", [30., 40.]).unwrap();
                let mut say = definitions.instantiate("say", [420., 40.]).unwrap();
                event.branch = Some(event.id);
                say.branch = Some(event.id);
                let doc = GraphDocument {
                    nodes: vec![event, say],
                    ..Default::default()
                };
                project
                    .node_graphs
                    .insert(owner, serde_json::json!({"version":1,"branches":[doc]}));
            }
            let mut ctx = TheContext::new(1000, 700, 1.5);
            ctx.ui.command_paint = true;
            let mut ui = TheUI::new();
            ui.init(&mut ctx);
            ui.set_theme(Box::new(TheBlackBlueTheme::new()), &mut ctx);
            let mut side = TheCanvas::new();
            let mut spacer = TheSpacer::new(TheId::named("side"));
            spacer.limiter_mut().set_max_width(80);
            side.set_widget(spacer);
            ui.canvas.set_left(side);
            let mut f = Self {
                registry: NativeNodesWorkspace::default(),
                project,
                ui,
                ctx,
                character: id,
            };
            f.show(ProjectContext::WorldCode);
            f
        }
        fn show(&mut self, context: ProjectContext) {
            let key = self
                .registry
                .activate(&self.project, context, None, &mut self.ctx.text)
                .unwrap()
                .unwrap();
            let view = &self.registry.views[&key];
            let mut center = TheCanvas::new();
            center.set_widget(NativeNodesView::with_state(
                view.id.clone(),
                view.model.clone(),
                view.panels.clone(),
            ));
            self.ctx.ui.clear_focus();
            self.ui.canvas.set_center(center);
            self.paint();
        }
        fn paint(&mut self) -> PaintList {
            self.ui
                .paint_commands(&mut self.ctx, &mut NoImages)
                .unwrap()
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
        fn key(&mut self, physical: PhysicalKey) {
            self.send(InputEvent::Key {
                physical,
                logical: LogicalKey::Other,
                pressed: true,
                repeat: false,
            });
        }
        fn command(&mut self, command: EditorCommand) {
            self.ui
                .get_widget(VIEW)
                .unwrap()
                .editor_command(command, &mut self.ctx);
            self.ui.process_events(&mut self.ctx);
        }
        fn type_text(&mut self, text: &str, submit: bool) -> TextInputTarget {
            let dim = *self.ui.get_widget(VIEW).unwrap().dim();
            self.click([
                (dim.x + dim.width - 10) as f32,
                (dim.y + dim.height - 10) as f32,
            ]);
            let model = self.registry.views[self.registry.active.as_ref().unwrap()]
                .model
                .clone();
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
                .to_screen([rect.origin[0] + 15., rect.origin[1] + 10.]);
            drop(model);
            self.click([p[0] + dim.x as f32, p[1] + dim.y as f32]);
            self.command(EditorCommand::SelectAll);
            let target = self
                .ui
                .get_widget(VIEW)
                .unwrap()
                .editor_input_state(&mut self.ctx)
                .unwrap()
                .target;
            assert!(self.send(InputEvent::Text(text.into())).consumed);
            if submit {
                self.key(PhysicalKey::Enter);
            }
            target
        }
        fn saved(&self, owner: &str) -> String {
            let docs: Vec<GraphDocument> =
                serde_json::from_value(self.project.node_graphs[owner]["branches"].clone())
                    .unwrap();
            let node = docs
                .iter()
                .flat_map(|doc| &doc.nodes)
                .find(|node| node.definition.as_deref() == Some("say"))
                .unwrap();
            node.rows
                .iter()
                .find_map(|row| match &row.value {
                    GraphControlValue::Text(text) => Some(text.clone()),
                    _ => None,
                })
                .unwrap()
        }
    }
    #[test]
    fn native_node_sources_retain_queued_and_unsubmitted_text_across_owner_switches() {
        for submit in [false, true] {
            let mut f = Fixture::new();
            let world = f.registry.active.clone().unwrap();
            let model = f.registry.views[&world].model.clone();
            let target = f.type_text("World 東京", submit);
            if !submit {
                f.send(InputEvent::CompositionFor {
                    target,
                    text: "仮".into(),
                    cursor: None,
                });
            }
            f.show(ProjectContext::CharacterCode(f.character));
            assert_eq!(f.saved("behavior/world"), "");
            assert!(model.lock().unwrap().pending_commits().next().is_some());
            f.type_text("Character e\u{301}", false);
            assert!(f.registry.has_pending());
            assert!(f.registry.checkpoint_all(&mut f.project).unwrap());
            assert_eq!(f.saved("behavior/world"), "World 東京");
            assert_eq!(
                f.saved(&format!("behavior/character/{}", f.character)),
                "Character e\u{301}"
            );
            let changes = f.registry.take_changes();
            assert_eq!(changes.len(), 2);
            assert_eq!(changes[0].key.owner, "behavior/world");
            assert!(
                !f.send(InputEvent::TextFor {
                    target,
                    text: "stale".into()
                })
                .consumed
            );
            assert!(!f.registry.checkpoint_all(&mut f.project).unwrap());
            assert!(f.registry.take_changes().is_empty());
            f.show(ProjectContext::WorldCode);
            assert!(Arc::ptr_eq(&f.registry.views[&world].model, &model));
            assert_eq!(f.saved("behavior/world"), "World 東京");
            if std::env::var_os("ELDIRON_NATIVE_NODE_SOURCE_ISOLATED").is_some() {
                assert!(std::sync::LazyLock::get(&crate::editor::RUSTERIX).is_none());
                assert!(std::sync::LazyLock::get(&crate::editor::SCENEMANAGER).is_none());
                assert!(std::sync::LazyLock::get(&crate::editor::EDITCAMERA).is_none());
            }
        }
    }
    #[test]
    fn native_node_sources_consume_interleaved_prefixes_without_overwriting_a_newer_text_draft() {
        let mut f = Fixture::new();
        let a = f.registry.active.clone().unwrap();
        f.type_text("A1", true);
        let first = f.registry.views[&a]
            .model
            .lock()
            .unwrap()
            .pending_through(u64::MAX)[0]
            .clone();
        f.show(ProjectContext::CharacterCode(f.character));
        f.type_text("B1", true);
        f.show(ProjectContext::WorldCode);
        f.type_text("A2", true);
        let target = f.type_text("A3 private", false);
        assert!(
            f.registry
                .collect_through(&mut f.project, first.sequence)
                .unwrap()
        );
        assert_eq!(f.saved("behavior/world"), "A1");
        assert_eq!(
            f.ui.get_widget(VIEW)
                .unwrap()
                .editor_input_state(&mut f.ctx)
                .unwrap()
                .target,
            target
        );
        assert!(f.registry.checkpoint_all(&mut f.project).unwrap());
        assert_eq!(f.saved("behavior/world"), "A3 private");
        assert_eq!(
            f.saved(&format!("behavior/character/{}", f.character)),
            "B1"
        );
        let changes = f.registry.take_changes();
        assert_eq!(changes.len(), 4);
        assert_eq!(
            changes
                .iter()
                .map(|change| change.key.owner.as_str())
                .collect::<Vec<_>>(),
            vec![
                "behavior/world",
                format!("behavior/character/{}", f.character).as_str(),
                "behavior/world",
                "behavior/world"
            ]
        );
        assert!(
            !f.registry
                .collect_through(&mut f.project, first.sequence)
                .unwrap()
        );
        assert!(f.registry.take_changes().is_empty());
    }
    #[test]
    fn native_node_sources_reset_revokes_every_model_queue_and_old_text_target() {
        let mut f = Fixture::new();
        f.type_text("A pending", true);
        let model = f.registry.views[f.registry.active.as_ref().unwrap()]
            .model
            .clone();
        let old = model.lock().unwrap().binding();
        f.show(ProjectContext::CharacterCode(f.character));
        let target = f.type_text("B private", false);
        let exact = f.project.node_graphs.clone();
        f.registry.reset();
        assert_ne!(model.lock().unwrap().binding(), old);
        assert!(model.lock().unwrap().pending_commits().next().is_none());
        assert!(
            !f.send(InputEvent::TextFor {
                target,
                text: "stale".into()
            })
            .consumed
        );
        assert!(!f.registry.checkpoint_all(&mut f.project).unwrap());
        assert_eq!(f.project.node_graphs, exact);
        f.show(ProjectContext::WorldCode);
        assert_eq!(f.saved("behavior/world"), "");
    }
    #[test]
    fn native_node_sources_report_conflicts_after_adopting_independent_valid_source_changes() {
        let mut f = Fixture::new();
        f.type_text("Private world", true);
        let world = f.registry.active.clone().unwrap();
        let defs = source_definitions(&f.project, &world);
        let mut graphs = shared::graph_authoring::NodeGraphDocuments::from_value(
            &f.project.node_graphs[&world.owner],
            world.kind(),
            &defs,
        )
        .unwrap();
        let doc = graphs.branches.get_mut(&world.branch.unwrap()).unwrap();
        let node = doc
            .nodes
            .iter_mut()
            .find(|node| node.definition.as_deref() == Some("say"))
            .unwrap();
        node.rows
            .iter_mut()
            .find(|row| matches!(row.value, GraphControlValue::Text(_)))
            .unwrap()
            .value = GraphControlValue::Text("External world".into());
        f.project
            .node_graphs
            .insert(world.owner.clone(), graphs.to_value());
        f.show(ProjectContext::CharacterCode(f.character));
        f.type_text("Valid character", true);
        assert!(f.registry.checkpoint_all(&mut f.project).is_err());
        assert_eq!(f.saved("behavior/world"), "External world");
        assert_eq!(
            f.saved(&format!("behavior/character/{}", f.character)),
            "Valid character"
        );
        let changes = f.registry.take_changes();
        assert_eq!(changes.len(), 1);
        assert_ne!(changes[0].key.owner, world.owner);
        assert!(f.registry.has_pending());
        assert!(
            f.registry.views[&world]
                .model
                .lock()
                .unwrap()
                .document()
                .nodes
                .iter()
                .any(|node| node
                    .rows
                    .iter()
                    .any(|row| row.value == GraphControlValue::Text("Private world".into())))
        );
    }
    #[test]
    fn native_node_sources_configuration_branch_selection_keeps_one_owner_and_queued_field_edits() {
        let mut f = Fixture::new();
        f.project.characters.get_mut(&f.character).unwrap().data =
            "[attributes]\nrace=\"Human\"\nclass=\"Warrior\"\n[input]\nw=\"control.forward\""
                .into();
        let context = ProjectContext::CharacterData(f.character);
        let key = f
            .registry
            .activate(&f.project, context, None, &mut f.ctx.text)
            .unwrap()
            .unwrap();
        assert_eq!(key.branch, None);
        let definitions = source_definitions(&f.project, &key);
        let view = &f.registry.views[&key];
        let model = view.model.clone();
        let old_id = view.id.clone();
        assert_eq!(view.context, context);
        let mut guard = model.lock().unwrap();
        let branches = graph_branches(guard.document(), &definitions);
        assert!(branches.branches.len() >= 2);
        let other = branches
            .branches
            .iter()
            .find(|branch| Some(branch.root) != view.shown)
            .unwrap()
            .root;
        let root = guard
            .document()
            .nodes
            .iter()
            .find(|node| node.definition.as_deref() == Some("entity"))
            .unwrap();
        let row = root
            .rows
            .iter()
            .find(|row| row.key.as_deref() == Some("race"))
            .unwrap();
        let (node, row, before, binding) = (root.id, row.id, row.value.clone(), guard.binding());
        guard
            .commit_control(
                binding,
                node,
                row,
                &before,
                GraphControlValue::Text("Orc".into()),
            )
            .unwrap()
            .unwrap();
        drop(guard);
        let exact = f.project.node_graphs.clone();
        let selected = f
            .registry
            .activate(&f.project, context, Some(other), &mut f.ctx.text)
            .unwrap()
            .unwrap();
        assert_eq!(selected, key);
        assert_eq!(f.project.node_graphs, exact);
        let view = &f.registry.views[&key];
        assert!(Arc::ptr_eq(&view.model, &model));
        assert_ne!(view.id, old_id);
        assert_eq!(view.shown, Some(other));
        assert_eq!(view.model.lock().unwrap().binding(), binding);
        assert!(view.model.lock().unwrap().editor().node_visible(other));
        let notices = f.registry.notifications();
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].1.binding, binding);
        assert!(f.registry.checkpoint_all(&mut f.project).unwrap());
        assert!(f.project.characters[&f.character].data.contains("Orc"));
        assert_eq!(f.registry.take_changes().len(), 1);
    }
}
