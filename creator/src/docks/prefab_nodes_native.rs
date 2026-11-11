//! Native views retain CPU source ownership independently of the visible pane.
use super::*;
use shared::prefab_graph::{
    PrefabGraphWorkspace, PrefabSourceApply, PrefabSourceError, PrefabSourceKey,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
pub(super) type NativePrefabState = Arc<Mutex<GraphViewState>>;
struct SourceView {
    model: NativePrefabState,
    revision: u64,
}
#[derive(Default)]
pub(super) struct NativePrefabWorkspace {
    cpu: PrefabGraphWorkspace,
    views: HashMap<PrefabSourceKey, SourceView>,
}
fn owner(key: PrefabSourceKey) -> String {
    format!("prefab/{}/{}", key.asset, key.part)
}
fn exists(project: &Project, key: PrefabSourceKey) -> bool {
    project
        .block_props
        .get(&key.asset)
        .is_some_and(|asset| asset.find_part(key.part).is_some())
}
impl PrefabNodePane {
    fn active_native_key(&self) -> Option<PrefabSourceKey> {
        Some(PrefabSourceKey {
            asset: self.asset?,
            part: self.part?,
        })
    }
    fn checkpoint_native_source(&mut self, key: PrefabSourceKey, finish: bool) {
        if let Some(view) = self.native_workspace.views.get(&key) {
            let mut model = view.model.lock().unwrap();
            if finish {
                model.edit(|document, editor| editor.cancel_gesture(document));
            }
            model.checkpoint_text();
            if finish {
                model.finish_text(true);
                model.publish_edits();
            }
        }
    }
    pub(super) fn load_native_source(
        &mut self,
        project: &Project,
        asset: Uuid,
        part: Option<Uuid>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> bool {
        let key = part.map(|part| PrefabSourceKey { asset, part });
        let previous = self.active_native_key();
        if previous != key
            && let Some(previous) = previous
        {
            self.checkpoint_native_source(previous, true);
        }
        let Some(key) = key else {
            self.asset = Some(asset);
            self.part = None;
            self.doc = GraphDocument::default();
            self.committed = self.doc.clone();
            self.store_error = None;
            self.editor = GraphEditor::default();
            self.native = Some(Arc::new(Mutex::new(GraphViewState::new(
                "prefab/empty",
                self.doc.clone(),
            ))));
            return true;
        };
        let preserve = self.native_workspace.views.get(&key).is_some_and(|view| {
            let model = view.model.lock().unwrap();
            model.pending_commits().next().is_some()
                || self.native_workspace.cpu.source(key).is_some_and(|source| {
                    matches!(
                        source.diagnostic(),
                        Some(PrefabSourceError::SourceChanged(_))
                    ) || (model.editor().text_focus().is_some()
                        && model.document() != source.document())
                })
        });
        let (document, committed, error, revision) =
            match self.native_workspace.cpu.load(project, key, preserve) {
                Ok(source) => (
                    source.document().clone(),
                    source.committed().clone(),
                    source.diagnostic().map(ToString::to_string),
                    source.revision(),
                ),
                Err(error) => {
                    ctx.ui
                        .send(TheEvent::SetStatusText(TheId::empty(), error.to_string()));
                    return true;
                }
            };
        let recreate = self
            .native_workspace
            .views
            .get(&key)
            .is_none_or(|view| view.revision != revision);
        self.asset = Some(asset);
        self.part = Some(key.part);
        self.doc = document;
        self.committed = committed;
        self.store_error = error;
        if recreate {
            let previous_viewport = self
                .native_workspace
                .views
                .get(&key)
                .map(|view| view.model.lock().unwrap().editor().viewport);
            if let Some(old) = self.native_workspace.views.remove(&key) {
                old.model
                    .lock()
                    .unwrap()
                    .replace_document("retired-source", GraphDocument::default());
            }
            self.editor = GraphEditor::default();
            if let Some(viewport) = previous_viewport {
                self.editor.viewport = viewport;
            }
            if self.opened.insert((asset, key.part)) {
                self.tidy();
                self.native_workspace
                    .cpu
                    .set_working_document(key, self.doc.clone())
                    .unwrap();
            }
            let mut model = GraphViewState::new(owner(key), self.doc.clone());
            let viewport = self.editor.viewport;
            let error = self.store_error.clone();
            model.edit(|_, editor| {
                editor.viewport = viewport;
                editor.error = error;
            });
            self.native_workspace
                .cpu
                .bind(key, model.binding())
                .unwrap();
            self.native_workspace.views.insert(
                key,
                SourceView {
                    model: Arc::new(Mutex::new(model)),
                    revision,
                },
            );
        }
        self.native = Some(self.native_workspace.views[&key].model.clone());
        if recreate || previous != Some(key) {
            self.fit(ui);
        }
        true
    }
    fn sync_native_mirror(&mut self) {
        if let Some(key) = self.active_native_key()
            && let Some(source) = self.native_workspace.cpu.source(key)
        {
            self.doc = source.document().clone();
            self.committed = source.committed().clone();
            self.store_error = source.diagnostic().map(ToString::to_string);
        }
        for (key, view) in &self.native_workspace.views {
            let error = self
                .native_workspace
                .cpu
                .source(*key)
                .and_then(|source| source.diagnostic())
                .map(ToString::to_string);
            let mut model = view.model.lock().unwrap();
            if model.editor().error != error {
                model.edit(|_, editor| editor.error = error);
            }
        }
    }
    fn apply_native_effect(&mut self, outcome: PrefabSourceApply, ctx: &mut TheContext) -> bool {
        if let Some(change) = outcome.change {
            crate::editor::UNDOMANAGER
                .write()
                .unwrap()
                .add_undo(ProjectUndoAtom::PrefabGraphEdit(Box::new(change)), ctx);
            ctx.ui.send(TheEvent::Custom(
                TheId::named(super::super::blocks::BLOCKS_DOCK_SYNC_EVENT),
                TheValue::Empty,
            ));
            true
        } else {
            false
        }
    }
    fn native_error(&self, project: &Project) -> Option<String> {
        self.native_workspace
            .cpu
            .diagnostics()
            .next()
            .map(|(key, error)| {
                let asset = project.block_props.get(&key.asset);
                let name = asset.map(|asset| asset.name.as_str()).unwrap_or("Prefab");
                let part = asset
                    .and_then(|asset| asset.find_part(key.part))
                    .map(|part| part.name.as_str())
                    .unwrap_or("Part");
                format!("{name} / {part}: {error}")
            })
    }
    fn collect_native_through(
        &mut self,
        sequence: u64,
        project: &mut Project,
        ctx: &mut TheContext,
    ) -> Result<bool, String> {
        let missing: Vec<_> = self
            .native_workspace
            .views
            .keys()
            .copied()
            .filter(|key| !exists(project, *key))
            .collect();
        for key in missing {
            if let Some(view) = self.native_workspace.views.remove(&key) {
                view.model
                    .lock()
                    .unwrap()
                    .replace_document("deleted-source", GraphDocument::default());
            }
            self.native_workspace.cpu.remove(key);
        }
        let mut commits = Vec::new();
        for view in self.native_workspace.views.values() {
            let model = view.model.lock().unwrap();
            commits.extend(
                model
                    .pending_through(u64::MAX)
                    .into_iter()
                    .filter(|commit| commit.sequence <= sequence)
                    .map(|commit| (view.model.clone(), commit)),
            );
        }
        commits.sort_by_key(|(_, commit)| commit.sequence);
        let mut changed = false;
        let mut dispatch_error = None;
        for (model, commit) in commits {
            if self
                .native_workspace
                .cpu
                .binding_source(commit.binding)
                .is_none()
            {
                continue;
            }
            match self.native_workspace.cpu.apply_commit(project, &commit) {
                Ok(outcome) => {
                    changed |= self.apply_native_effect(outcome, ctx);
                    if !model
                        .lock()
                        .unwrap()
                        .acknowledge(commit.binding, commit.serial)
                    {
                        dispatch_error =
                            Some("Prefab source changed while acknowledging an edit".into());
                    }
                }
                Err(error) => {
                    dispatch_error = Some(error.to_string());
                }
            }
        }
        if changed {
            crate::block_props::sync_existing_prefab_presentations(project);
        }
        self.sync_native_mirror();
        if let Some(error) = dispatch_error.or_else(|| self.native_error(project)) {
            ctx.ui
                .send(TheEvent::SetStatusText(TheId::empty(), error.clone()));
            Err(error)
        } else {
            Ok(changed)
        }
    }
    pub(in crate::docks) fn commit_native_pending(
        &mut self,
        project: &mut Project,
        ctx: &mut TheContext,
    ) -> Result<bool, String> {
        let keys: Vec<_> = self.native_workspace.views.keys().copied().collect();
        for key in keys {
            self.checkpoint_native_source(key, false);
        }
        self.collect_native_through(u64::MAX, project, ctx)
    }
    pub(super) fn store_native_document(
        &mut self,
        project: &mut Project,
        ctx: &mut TheContext,
    ) -> bool {
        let Some(key) = self
            .active_native_key()
            .filter(|key| self.native_workspace.cpu.source(*key).is_some())
        else {
            return false;
        };
        match self
            .native_workspace
            .cpu
            .commit_document(project, key, self.doc.clone())
        {
            Ok(outcome) => {
                if self.apply_native_effect(outcome, ctx) {
                    crate::block_props::sync_existing_prefab_presentations(project);
                }
            }
            Err(error) => {
                ctx.ui
                    .send(TheEvent::SetStatusText(TheId::empty(), error.to_string()));
            }
        }
        self.sync_native_mirror();
        if let Some(error) = &self.store_error {
            ctx.ui
                .send(TheEvent::SetStatusText(TheId::empty(), error.clone()));
        }
        true
    }
    fn rebind_native_source(&mut self, key: PrefabSourceKey) {
        let Some(source) = self.native_workspace.cpu.source(key) else {
            return;
        };
        let document = source.document().clone();
        let error = source.diagnostic().map(ToString::to_string);
        let Some(view) = self.native_workspace.views.get(&key) else {
            return;
        };
        let mut model = view.model.lock().unwrap();
        let old = model.binding();
        let viewport = model.editor().viewport;
        let selected = model.editor().selected;
        model.replace_document(owner(key), document);
        model.edit(|_, editor| {
            editor.viewport = viewport;
            editor.selected = selected;
            editor.error = error;
        });
        self.native_workspace.cpu.unbind(old);
        self.native_workspace
            .cpu
            .bind(key, model.binding())
            .unwrap();
    }
    pub(super) fn replace_native_document(&mut self) {
        if let Some(key) = self.active_native_key() {
            self.rebind_native_source(key);
        }
    }
    pub(super) fn render_native(&mut self, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
        if !ctx.ui.command_paint {
            return false;
        }
        let Some(widget) = ui.get_widget(VIEW) else {
            return true;
        };
        let state = self
            .native
            .get_or_insert_with(|| {
                Arc::new(Mutex::new(GraphViewState::new(
                    "prefab/empty",
                    GraphDocument::default(),
                )))
            })
            .clone();
        let native_widget = widget.as_any().is::<TheGraphView>();
        let bound = widget
            .as_any()
            .downcast_mut::<TheGraphView>()
            .is_some_and(|view| Arc::ptr_eq(&view.model(), &state));
        if !bound {
            let previous = widget.id().clone();
            let id = if native_widget {
                TheId::named(VIEW)
            } else {
                previous.clone()
            };
            if ctx.ui.has_focus(&previous) {
                ctx.ui.clear_focus();
            }
            if ctx.ui.keyboard_focus.as_ref() == Some(&previous) {
                ctx.ui.keyboard_focus = None;
            }
            let dim = *widget.dim();
            let mut view = TheGraphView::with_state(id, state);
            view.set_context(Context);
            view.set_controls(Controls);
            view.set_theme(Some(GraphTheme::default()));
            view.set_auto_focus(true);
            view.set_dim(dim, ctx);
            *widget = Box::new(view);
            self.resources = None;
            ctx.ui.redraw_all = true;
        }
        true
    }
    pub(super) fn native_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
    ) -> Option<bool> {
        match event {
            TheEvent::WidgetResized(id, _)
                if id.name == VIEW
                    && self.native.is_some()
                    && ui.get_widget(VIEW).is_none_or(|widget| widget.id() != id) =>
            {
                Some(false)
            }
            TheEvent::GraphCommitted(id, commit) if id.name == VIEW => {
                self.native_workspace.cpu.binding_source(commit.binding)?;
                let _ = self.collect_native_through(commit.sequence, project, ctx);
                Some(true)
            }
            TheEvent::GraphDrop(id, binding, point, drop) if id.name == VIEW => {
                let Some(key) = self.native_workspace.cpu.binding_source(*binding) else {
                    return Some(false);
                };
                let Some(definition) = drop
                    .id
                    .name
                    .strip_prefix("Node Catalog/")
                    .filter(|key| node_available(key))
                else {
                    return Some(false);
                };
                self.checkpoint_native_source(key, true);
                let _ = self.collect_native_through(u64::MAX, project, ctx);
                let Some(source) = self.native_workspace.cpu.source(key) else {
                    return Some(false);
                };
                let mut document = source.document().clone();
                document.nodes.push(
                    prefab_graph::definitions()
                        .node(definition)
                        .unwrap()
                        .instantiate(*point),
                );
                if let Ok(outcome) = self
                    .native_workspace
                    .cpu
                    .commit_document(project, key, document)
                    && self.apply_native_effect(outcome, ctx)
                {
                    crate::block_props::sync_existing_prefab_presentations(project);
                }
                self.sync_native_mirror();
                self.rebind_native_source(key);
                Some(true)
            }
            TheEvent::RenderViewDrop(id, _, _) if id.name == VIEW && self.native.is_some() => {
                Some(false)
            }
            TheEvent::GraphCommand(id, _, _) if id.name == VIEW => Some(false),
            _ => None,
        }
    }
    pub(super) fn prepare_native_domain_event(
        &mut self,
        event: &TheEvent,
        ctx: &mut TheContext,
        project: &mut Project,
    ) -> Result<bool, String> {
        if self.native.is_none() {
            return Ok(false);
        }
        let domain = match event {
            TheEvent::NewListItemSelected(id, _) => id.name == "Prefab Branch",
            TheEvent::StateChanged(id, TheWidgetState::Clicked) => id.name == TIDY,
            TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Delete)) => {
                ctx.ui.focus.as_ref().is_some_and(|id| id.name == VIEW)
            }
            _ => false,
        };
        if !domain {
            return Ok(false);
        }
        let _ = self.commit_native_pending(project, ctx);
        if matches!(event, TheEvent::NewListItemSelected(id, _) if id.name == "Prefab Branch")
            && let Some(error) = &self.store_error
        {
            return Err(error.clone());
        }
        let mut model = self.native.as_ref().unwrap().lock().unwrap();
        model.edit(|document, editor| editor.cancel_gesture(document));
        self.doc = model.document().clone();
        self.editor.viewport = model.editor().viewport;
        self.editor.selected = model.editor().selected;
        self.editor.selected_connection = model.editor().selected_connection;
        Ok(true)
    }
    pub(in crate::docks) fn has_native_pending(&self) -> bool {
        self.native_workspace.cpu.diagnostics().next().is_some()
            || self.native_workspace.views.iter().any(|(key, view)| {
                let model = view.model.lock().unwrap();
                model.pending_commits().next().is_some()
                    || self
                        .native_workspace
                        .cpu
                        .source(*key)
                        .is_some_and(|source| {
                            model.editor().text_focus().is_some()
                                && model.document() != source.document()
                        })
            })
    }
    pub(in crate::docks) fn native_source_asset(&self, binding: u64) -> Option<Uuid> {
        self.native_workspace
            .cpu
            .binding_source(binding)
            .map(|key| key.asset)
    }
    pub(in crate::docks) fn reset_native_project(&mut self) {
        for view in self.native_workspace.views.values() {
            view.model
                .lock()
                .unwrap()
                .replace_document("closed-project", GraphDocument::default());
        }
        if let Some(model) = self.native.take() {
            model
                .lock()
                .unwrap()
                .replace_document("closed-project", GraphDocument::default());
        }
        self.native_workspace = NativePrefabWorkspace::default();
        self.asset = None;
        self.part = None;
        self.doc = GraphDocument::default();
        self.committed = self.doc.clone();
        self.editor = GraphEditor::default();
        self.opened.clear();
        self.resources = None;
        self.store_error = None;
    }
}

#[cfg(test)]
mod tests {
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
            panic!("native prefab graph must not rasterize its panel")
        }
    }
    fn key(physical: PhysicalKey) -> InputEvent {
        InputEvent::Key {
            physical,
            logical: LogicalKey::Other,
            pressed: true,
            repeat: false,
        }
    }
    struct Fixture {
        pane: PrefabNodePane,
        ui: TheUI,
        ctx: TheContext,
        project: Project,
        asset: Uuid,
        first: Uuid,
        second: Uuid,
        events: std::sync::mpsc::Receiver<TheEvent>,
        undo_signals: usize,
    }
    impl Fixture {
        fn new() -> Self {
            let mut project = Project::default();
            let mut asset = rusterix::BlockPropAsset::new("Native fixture");
            asset
                .parts
                .push(rusterix::BlockPropPart::new_authored("First", Vec::new()));
            asset
                .parts
                .push(rusterix::BlockPropPart::new_authored("Second", Vec::new()));
            let (id, first, second) = (asset.id, asset.parts[0].id, asset.parts[1].id);
            project
                .prefab_graphs
                .insert(id, PrefabGraphs::import(&asset));
            project.block_props.insert(id, asset);
            let mut ctx = TheContext::new(960, 600, 1.5);
            ctx.ui.command_paint = true;
            let mut ui = TheUI::new();
            ui.init(&mut ctx);
            ui.set_theme(Box::new(TheBlackBlueTheme::new()), &mut ctx);
            let events = ui.add_state_listener("native-prefab-test".into());
            let mut toolbar = TheCanvas::new();
            let mut spacer = TheSpacer::new(TheId::named("toolbar"));
            spacer.limiter_mut().set_max_height(24);
            toolbar.set_widget(spacer);
            ui.canvas.set_center(PrefabNodePane::canvas(toolbar));
            ui.prepare_paint_layout(&mut ctx);
            let mut pane = PrefabNodePane::new();
            pane.load(&project, id, Some(first), &mut ui, &mut ctx);
            let mut f = Self {
                pane,
                ui,
                ctx,
                project,
                asset: id,
                first,
                second,
                events,
                undo_signals: 0,
            };
            f.pump();
            f.paint();
            assert!(f.ui.get_widget(VIEW).unwrap().as_any().is::<TheGraphView>());
            assert!(f.pane.resources.is_none());
            f
        }
        fn model(&self) -> NativePrefabState {
            self.pane.native.as_ref().unwrap().clone()
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
        fn pump(&mut self) {
            for _ in 0..32 {
                self.ui.process_events(&mut self.ctx);
                let events: Vec<_> = self.events.try_iter().collect();
                if events.is_empty() {
                    return;
                }
                for event in events {
                    if matches!(&event, TheEvent::SetState(name, _) if name == "Undo") {
                        self.undo_signals += 1;
                    }
                    self.pane
                        .handle(&event, &mut self.ui, &mut self.ctx, &mut self.project);
                }
            }
            panic!("native prefab events must settle");
        }
        fn click(&mut self, definition: &str, row_key: &str) {
            let model = self.model();
            let model = model.lock().unwrap();
            let node = model
                .document()
                .nodes
                .iter()
                .find(|node| node.definition.as_deref() == Some(definition))
                .unwrap();
            let index = node
                .rows
                .iter()
                .position(|row| row.key.as_deref() == Some(row_key))
                .unwrap();
            let rect = node.row_rect(index, &model.document().metrics());
            let point = model
                .editor()
                .viewport
                .to_screen([rect.origin[0] + 12., rect.origin[1] + 10.]);
            drop(model);
            let dim = *self.ui.get_widget(VIEW).unwrap().dim();
            let position = [dim.x as f32 + point[0], dim.y as f32 + point[1]];
            self.send(InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: true,
                position,
            });
            self.send(InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: false,
                position,
            });
        }
        fn command(&mut self, command: EditorCommand) {
            self.ui
                .get_widget(VIEW)
                .unwrap()
                .editor_command(command, &mut self.ctx);
            self.ui.process_events(&mut self.ctx);
        }
        fn state(&mut self) -> TextInputState {
            self.ui
                .get_widget(VIEW)
                .unwrap()
                .editor_input_state(&mut self.ctx)
                .unwrap()
        }
        fn first_name(&self) -> &str {
            &self.project.block_props[&self.asset]
                .find_part(self.first)
                .unwrap()
                .name
        }
        fn commit(&mut self) -> Result<bool, String> {
            self.pane
                .commit_native_pending(&mut self.project, &mut self.ctx)
        }
    }

    #[test]
    fn native_prefab_edits_compile_serialize_once_and_do_not_construct_world_renderers() {
        let isolated = std::env::var_os("ELDIRON_NATIVE_PREFAB_ISOLATED").is_some();
        if isolated {
            assert!(std::sync::LazyLock::get(&crate::editor::RUSTERIX).is_none());
            assert!(std::sync::LazyLock::get(&crate::editor::SCENEMANAGER).is_none());
        }
        let mut f = Fixture::new();
        let first = f.first;
        let second = f.second;
        f.click("prefab_part", "name");
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("e\u{301}日本".into()));
        f.send(key(PhysicalKey::Enter));
        f.pump();
        assert_eq!(f.first_name(), "e\u{301}日本");
        assert_eq!(
            f.project.block_props[&f.asset]
                .find_part(second)
                .unwrap()
                .name,
            "Second"
        );
        assert_eq!(
            f.undo_signals, 1,
            "one native transaction enables one project undo entry"
        );
        assert!(!f.commit().unwrap());
        f.pump();
        assert_eq!(f.undo_signals, 1);
        let bytes = shared::project_io::encode_project(&f.project).unwrap();
        let loaded = shared::project_io::decode_project(&bytes).unwrap();
        assert_eq!(
            loaded.block_props[&f.asset].find_part(first).unwrap().name,
            "e\u{301}日本"
        );
        assert_eq!(
            loaded.prefab_graphs[&f.asset],
            f.project.prefab_graphs[&f.asset]
        );
        assert!(f.pane.resources.is_none());
        assert!(f.ui.canvas.buffer.pixels().is_empty());
        assert!(!f.pane.has_native_pending());
        if isolated {
            assert!(std::sync::LazyLock::get(&crate::editor::RUSTERIX).is_none());
            assert!(std::sync::LazyLock::get(&crate::editor::SCENEMANAGER).is_none());
        }
    }

    #[test]
    fn native_prefab_save_barrier_keeps_ime_and_ignores_an_earlier_queued_commit() {
        let mut f = Fixture::new();
        f.click("prefab_part", "name");
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("First commit".into()));
        f.send(key(PhysicalKey::Enter));
        // Do not dispatch that wake-up to the pane until after Save.
        f.click("prefab_part", "name");
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("Saved draft".into()));
        let target = f.state().target;
        f.send(InputEvent::CompositionFor {
            target,
            text: "仮".into(),
            cursor: None,
        });
        assert!(f.pane.has_native_pending());
        assert!(f.commit().unwrap());
        assert_eq!(f.first_name(), "Saved draft");
        assert_eq!(f.state().target, target);
        f.send(key(PhysicalKey::Enter));
        assert!(f.model().lock().unwrap().editor().text_focus().is_some());
        f.pump();
        assert_eq!(f.undo_signals, 2);
        assert_eq!(f.first_name(), "Saved draft");
        f.command(EditorCommand::Undo);
        assert!(f.commit().unwrap());
        f.pump();
        assert_eq!(f.first_name(), "First commit");
        assert_eq!(f.undo_signals, 3);
        assert!(!f.pane.has_native_pending());
    }

    #[test]
    fn native_prefab_branch_and_project_replacement_reject_old_transactions_and_text_callbacks() {
        let mut f = Fixture::new();
        f.click("prefab_part", "name");
        f.command(EditorCommand::SelectAll);
        let target = f.state().target;
        f.send(InputEvent::Text("Saved first".into()));
        f.send(key(PhysicalKey::Enter));
        let old = f.model().lock().unwrap().pending_through(u64::MAX)[0].clone();
        f.pane.handle(
            &TheEvent::NewListItemSelected(
                TheId::named_with_id("Prefab Branch", f.second),
                TheId::named(LIST),
            ),
            &mut f.ui,
            &mut f.ctx,
            &mut f.project,
        );
        assert_eq!(f.pane.part, Some(f.second));
        assert_eq!(f.first_name(), "Saved first");
        assert_ne!(f.model().lock().unwrap().binding(), old.binding);
        f.pump();
        f.pane.handle(
            &TheEvent::GraphCommitted(TheId::named(VIEW), old.clone()),
            &mut f.ui,
            &mut f.ctx,
            &mut f.project,
        );
        assert_eq!(
            f.project.block_props[&f.asset]
                .find_part(f.second)
                .unwrap()
                .name,
            "Second"
        );
        f.click("prefab_part", "name");
        f.send(InputEvent::TextFor {
            target,
            text: "stale".into(),
        });
        f.pump();
        assert_eq!(
            f.project.block_props[&f.asset]
                .find_part(f.second)
                .unwrap()
                .name,
            "Second"
        );
        f.pane.reset_native_project();
        f.pane
            .load(&f.project, f.asset, Some(f.first), &mut f.ui, &mut f.ctx);
        f.pane.handle(
            &TheEvent::GraphCommitted(TheId::named(VIEW), old),
            &mut f.ui,
            &mut f.ctx,
            &mut f.project,
        );
        assert_eq!(f.first_name(), "Saved first");
        assert!(f.pane.resources.is_none());
    }

    #[test]
    fn native_prefab_validation_preserves_the_draft_until_correction_and_never_updates_compiled_geometry()
     {
        let mut f = Fixture::new();
        let pivot = f.project.block_props[&f.asset]
            .find_part(f.first)
            .unwrap()
            .pivot;
        f.click("prefab_transform", "pivot_x");
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("not a number".into()));
        let target = f.state().target;
        assert!(f.commit().is_err());
        assert_eq!(
            f.project.block_props[&f.asset]
                .find_part(f.first)
                .unwrap()
                .pivot,
            pivot
        );
        assert!(f.pane.has_native_pending());
        assert_eq!(f.state().target, target);
        f.pane.handle(
            &TheEvent::NewListItemSelected(
                TheId::named_with_id("Prefab Branch", f.second),
                TheId::named(LIST),
            ),
            &mut f.ui,
            &mut f.ctx,
            &mut f.project,
        );
        assert_eq!(
            f.pane.part,
            Some(f.first),
            "invalid source must not be discarded by a branch transition"
        );
        // Re-enter the same retained source after the branch selector has focus.
        f.click("prefab_transform", "pivot_x");
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("1.25".into()));
        assert!(f.commit().unwrap());
        f.pump();
        assert_eq!(
            f.project.block_props[&f.asset]
                .find_part(f.first)
                .unwrap()
                .pivot[0],
            1.25
        );
        assert!(!f.pane.has_native_pending());
        assert!(f.pane.store_error.is_none());
        assert!(f.pane.resources.is_none());
    }
    #[test]
    fn native_prefab_source_switch_keeps_completed_and_unsubmitted_edits_owned_by_their_source() {
        for other_asset in [false, true] {
            for submitted in [false, true] {
                let mut f = Fixture::new();
                f.click("prefab_part", "name");
                f.command(EditorCommand::SelectAll);
                f.send(InputEvent::Text("Before switch".into()));
                let old_target = f.state().target;
                if submitted {
                    f.send(key(PhysicalKey::Enter));
                } else {
                    f.send(InputEvent::CompositionFor {
                        target: old_target,
                        text: "未確定".into(),
                        cursor: None,
                    });
                }
                let (asset, part, name) = if other_asset {
                    let mut asset = rusterix::BlockPropAsset::new("Another asset");
                    asset.parts.push(rusterix::BlockPropPart::new_authored(
                        "Other source",
                        Vec::new(),
                    ));
                    let (id, part) = (asset.id, asset.parts[0].id);
                    f.project
                        .prefab_graphs
                        .insert(id, PrefabGraphs::import(&asset));
                    f.project.block_props.insert(id, asset);
                    (id, part, "Other source")
                } else {
                    (f.asset, f.second, "Second")
                };
                // This is the same read-only reload used by the parent inspector
                // after legacy tree/asset activation, bypassing the branch button.
                // Keep the previous GraphCommitted wake-up queued until afterward.
                f.pane
                    .load(&f.project, asset, Some(part), &mut f.ui, &mut f.ctx);
                f.pump();
                f.commit().unwrap();
                f.pump();
                assert_eq!(
                    f.first_name(),
                    "Before switch",
                    "asset switch {other_asset}, submitted {submitted}"
                );
                assert_eq!(
                    f.project.block_props[&asset].find_part(part).unwrap().name,
                    name
                );
                assert_eq!(f.undo_signals, 1);
                f.click("prefab_part", "name");
                f.send(InputEvent::TextFor {
                    target: old_target,
                    text: "stale".into(),
                });
                f.send(InputEvent::CompositionFor {
                    target: old_target,
                    text: "stale".into(),
                    cursor: None,
                });
                assert!(!f.commit().unwrap());
                f.pump();
                assert_eq!(f.first_name(), "Before switch");
                assert_eq!(
                    f.project.block_props[&asset].find_part(part).unwrap().name,
                    name
                );
                assert_eq!(
                    f.undo_signals, 1,
                    "late wake-ups/IME must not create another transaction"
                );
            }
        }
    }

    #[test]
    fn retained_prefab_sources_consume_a_later_notification_in_global_order_and_do_not_replay_old_wakeups()
     {
        let mut f = Fixture::new();
        f.click("prefab_part", "name");
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("A1".into()));
        f.send(key(PhysicalKey::Enter));
        f.pane
            .load(&f.project, f.asset, Some(f.second), &mut f.ui, &mut f.ctx);
        f.click("prefab_part", "name");
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("B1".into()));
        f.send(key(PhysicalKey::Enter));
        f.pane
            .load(&f.project, f.asset, Some(f.first), &mut f.ui, &mut f.ctx);
        f.click("prefab_part", "name");
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("A2".into()));
        f.send(key(PhysicalKey::Enter));
        let latest = f
            .model()
            .lock()
            .unwrap()
            .pending_through(u64::MAX)
            .last()
            .unwrap()
            .clone();
        // Deliberately handle A2's wake-up before dispatching A1 or B1.
        f.pane.handle(
            &TheEvent::GraphCommitted(TheId::named(VIEW), latest),
            &mut f.ui,
            &mut f.ctx,
            &mut f.project,
        );
        f.pump();
        assert_eq!(f.first_name(), "A2");
        assert_eq!(
            f.project.block_props[&f.asset]
                .find_part(f.second)
                .unwrap()
                .name,
            "B1"
        );
        assert_eq!(f.undo_signals, 3);
        assert!(!f.commit().unwrap());
        f.pump();
        assert_eq!(f.undo_signals, 3);
        assert!(!f.pane.has_native_pending());
    }

    #[test]
    fn retained_invalid_prefab_source_does_not_block_other_source_edits_and_can_be_revisited() {
        let mut f = Fixture::new();
        f.click("prefab_transform", "pivot_x");
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("invalid".into()));
        assert!(f.commit().is_err());
        f.pump();
        f.pane
            .load(&f.project, f.asset, Some(f.second), &mut f.ui, &mut f.ctx);
        f.click("prefab_part", "name");
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("Valid second".into()));
        f.send(key(PhysicalKey::Enter));
        f.pump();
        assert_eq!(
            f.project.block_props[&f.asset]
                .find_part(f.second)
                .unwrap()
                .name,
            "Valid second"
        );
        assert_eq!(f.undo_signals, 1);
        assert!(f.pane.has_native_pending());
        // A hidden source error must not trap navigation from the valid branch.
        f.pane.handle(
            &TheEvent::NewListItemSelected(
                TheId::named_with_id("Prefab Branch", f.first),
                TheId::named(LIST),
            ),
            &mut f.ui,
            &mut f.ctx,
            &mut f.project,
        );
        assert_eq!(f.pane.part, Some(f.first));
        assert!(f.pane.store_error.is_some());
        f.click("prefab_transform", "pivot_x");
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("1.5".into()));
        assert!(f.commit().unwrap());
        f.pump();
        assert_eq!(
            f.project.block_props[&f.asset]
                .find_part(f.first)
                .unwrap()
                .pivot[0],
            1.5
        );
        assert_eq!(
            f.project.block_props[&f.asset]
                .find_part(f.second)
                .unwrap()
                .name,
            "Valid second"
        );
        assert_eq!(f.undo_signals, 2);
        assert!(!f.pane.has_native_pending());
    }

    #[test]
    fn native_prefab_save_leaves_an_unfinished_node_drag_with_the_view_until_release() {
        let mut f = Fixture::new();
        let dim = *f.ui.get_widget(VIEW).unwrap().dim();
        let model = f.model();
        let model = model.lock().unwrap();
        let node = &model.document().nodes[0];
        let id = node.id;
        let point = model
            .editor()
            .viewport
            .to_screen([node.position[0] + 20., node.position[1] + 12.]);
        drop(model);
        let position = [dim.x as f32 + point[0], dim.y as f32 + point[1]];
        f.send(InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: true,
            position,
        });
        let moved = [position[0] + 60., position[1] + 20.];
        f.send(InputEvent::PointerMoved { position: moved });
        let working_position = f
            .model()
            .lock()
            .unwrap()
            .document()
            .nodes
            .iter()
            .find(|node| node.id == id)
            .unwrap()
            .position;
        assert!(f.model().lock().unwrap().editor().is_gesturing());
        assert!(!f.commit().unwrap());
        assert!(f.model().lock().unwrap().editor().is_gesturing());
        assert_eq!(
            f.model()
                .lock()
                .unwrap()
                .document()
                .nodes
                .iter()
                .find(|node| node.id == id)
                .unwrap()
                .position,
            working_position
        );
        f.send(InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: false,
            position: moved,
        });
        f.pump();
        assert!(!f.model().lock().unwrap().editor().is_gesturing());
        let stored = &f.project.prefab_graphs[&f.asset]
            .branches
            .iter()
            .find(|branch| branch.part_id == f.first)
            .unwrap()
            .graph;
        assert_eq!(
            stored
                .nodes
                .iter()
                .find(|node| node.id == id)
                .unwrap()
                .position,
            working_position
        );
        assert_eq!(f.undo_signals, 1);
    }

    #[test]
    fn native_prefab_imports_keep_source_ids_without_persisting_until_an_actual_edit() {
        let mut f = Fixture::new();
        f.pane.reset_native_project();
        f.project.prefab_graphs.shift_remove(&f.asset);
        f.pane
            .load(&f.project, f.asset, Some(f.first), &mut f.ui, &mut f.ctx);
        let before = f.model();
        let binding = before.lock().unwrap().binding();
        let document = before.lock().unwrap().document().clone();
        for _ in 0..3 {
            f.pane
                .load(&f.project, f.asset, Some(f.first), &mut f.ui, &mut f.ctx);
            assert!(Arc::ptr_eq(&before, &f.model()));
            assert_eq!(f.model().lock().unwrap().binding(), binding);
            assert_eq!(f.model().lock().unwrap().document(), &document);
        }
        assert!(!f.project.prefab_graphs.contains_key(&f.asset));
        f.click("prefab_part", "name");
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("Imported edit".into()));
        f.send(key(PhysicalKey::Enter));
        f.pump();
        assert_eq!(f.first_name(), "Imported edit");
        assert_eq!(f.model().lock().unwrap().binding(), binding);
        assert_eq!(f.undo_signals, 1);
    }

    #[test]
    fn native_prefab_project_reset_revokes_all_retained_source_bindings_and_queued_edits() {
        let mut f = Fixture::new();
        f.click("prefab_part", "name");
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("Old A".into()));
        f.send(key(PhysicalKey::Enter));
        let first = f.model();
        let first_binding = first.lock().unwrap().binding();
        f.pane
            .load(&f.project, f.asset, Some(f.second), &mut f.ui, &mut f.ctx);
        f.click("prefab_part", "name");
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("Old B".into()));
        f.send(key(PhysicalKey::Enter));
        let second = f.model();
        let second_binding = second.lock().unwrap().binding();
        f.pane.reset_native_project();
        assert_ne!(first.lock().unwrap().binding(), first_binding);
        assert_ne!(second.lock().unwrap().binding(), second_binding);
        assert!(first.lock().unwrap().pending_commits().next().is_none());
        assert!(second.lock().unwrap().pending_commits().next().is_none());
        f.pane
            .load(&f.project, f.asset, Some(f.first), &mut f.ui, &mut f.ctx);
        f.pump();
        assert!(!f.commit().unwrap());
        f.pump();
        assert_eq!(f.first_name(), "First");
        assert_eq!(
            f.project.block_props[&f.asset]
                .find_part(f.second)
                .unwrap()
                .name,
            "Second"
        );
        assert_eq!(f.undo_signals, 0);
    }

    #[test]
    fn native_prefab_external_undo_rebinds_the_source_and_clears_the_obsolete_error() {
        let mut f = Fixture::new();
        let before = f.project.clone();
        f.click("prefab_part", "name");
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("Committed rename".into()));
        assert!(f.commit().unwrap());
        f.click("prefab_transform", "pivot_x");
        f.command(EditorCommand::SelectAll);
        f.send(InputEvent::Text("invalid".into()));
        assert!(f.commit().is_err());
        let old_target = f.state().target;
        let old_binding = f.model().lock().unwrap().binding();
        f.project = before;
        f.pane
            .load(&f.project, f.asset, Some(f.first), &mut f.ui, &mut f.ctx);
        assert!(f.pane.store_error.is_none());
        assert!(!f.pane.has_native_pending());
        assert_ne!(f.model().lock().unwrap().binding(), old_binding);
        f.send(InputEvent::TextFor {
            target: old_target,
            text: "stale".into(),
        });
        f.pump();
        assert_eq!(f.first_name(), "First");
        assert_eq!(f.project.block_props[&f.asset].parts[0].pivot[0], 0.);
    }

    #[test]
    fn native_prefab_drop_is_addressed_and_keeps_its_input_camera_coordinates() {
        let mut f = Fixture::new();
        let before = f.pane.doc.nodes.len();
        let model = f.model();
        let (binding, expected) = {
            let state = model.lock().unwrap();
            (
                state.binding(),
                state.editor().viewport.to_graph([160., 220.]),
            )
        };
        let drop = TheDrop::new(TheId::named("Node Catalog/prefab_door"));
        f.ui.get_widget(VIEW).unwrap().on_event(
            &TheEvent::Drop(Vec2::new(160, 220), drop.clone()),
            &mut f.ctx,
        );
        model
            .lock()
            .unwrap()
            .edit(|_, editor| editor.viewport.pan = [999., 777.]);
        f.pump();
        assert_eq!(f.pane.doc.nodes.len(), before + 1);
        assert_eq!(f.pane.doc.nodes.last().unwrap().position, expected);
        assert_eq!(f.undo_signals, 1);
        assert_eq!(
            f.model().lock().unwrap().editor().viewport.pan,
            [999., 777.]
        );
        assert_ne!(f.model().lock().unwrap().binding(), binding);
        f.pane.handle(
            &TheEvent::GraphDrop(TheId::named(VIEW), binding, [0.; 2], drop),
            &mut f.ui,
            &mut f.ctx,
            &mut f.project,
        );
        assert_eq!(f.pane.doc.nodes.len(), before + 1);
        assert!(f.pane.resources.is_none());
    }
    #[cfg(feature = "native-render")]
    #[test]
    #[ignore = "requires a native GPU"]
    fn native_prefab_pane_paints_on_shared_gpu_without_a_cpu_graph_cache() {
        use renderkit::*;
        use theframework::render_wgpu::{PaintRenderer, UiImages};
        let mut f = Fixture::new();
        f.click("prefab_part", "name");
        f.command(EditorCommand::SelectAll);
        let target = f.state().target;
        f.send(InputEvent::CompositionFor {
            target,
            text: "日本".into(),
            cursor: Some((0, 6)),
        });
        let mut captures = Vec::new();
        for _ in 0..2 {
            let gpu = pollster::block_on(GpuContext::headless()).unwrap();
            let mut images = UiImages::new(gpu.clone());
            let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
            let output = GpuImage::new(&gpu, ImageDescriptor::rgba(1440, 900)).unwrap();
            let list = f.ui.paint_commands(&mut f.ctx, &mut images).unwrap();
            images.finish_frame(&list).unwrap();
            assert_eq!(
                list.image_handles().count(),
                0,
                "no CPU graph framebuffer upload"
            );
            renderer
                .render(
                    &list,
                    &images.registry,
                    &output.view,
                    output.descriptor.format,
                    (1440, 900),
                    1.5,
                    [0.02, 0.02, 0.02, 1.],
                )
                .unwrap();
            let uploaded = gpu.transfers().uploaded_bytes;
            let rasterized = renderer.glyphs.rasterizations;
            assert!(rasterized > 0);
            let repeated = f.ui.paint_commands(&mut f.ctx, &mut images).unwrap();
            images.finish_frame(&repeated).unwrap();
            renderer
                .render(
                    &repeated,
                    &images.registry,
                    &output.view,
                    output.descriptor.format,
                    (1440, 900),
                    1.5,
                    [0.02, 0.02, 0.02, 1.],
                )
                .unwrap();
            assert_eq!(gpu.transfers().uploaded_bytes, uploaded);
            assert_eq!(renderer.glyphs.rasterizations, rasterized);
            assert_eq!(gpu.transfers().readback_bytes, 0);
            assert_eq!(f.state().target, target);
            assert!(f.pane.resources.is_none());
            assert!(f.ui.canvas.buffer.pixels().is_empty());
            let mut ticket = ReadbackTicket::capture(&gpu, output).unwrap();
            gpu.device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(std::time::Duration::from_secs(30)),
                })
                .unwrap();
            captures.push(ticket.try_finish(&gpu).unwrap().unwrap().pixels().to_vec());
        }
        assert_eq!(captures[0], captures[1]);
        let file = std::fs::File::create("/private/tmp/eldiron-native-prefab-graph.png").unwrap();
        let mut encoder = png::Encoder::new(file, 1440, 900);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&captures[0])
            .unwrap();
        f.send(InputEvent::TextFor {
            target,
            text: "日本".into(),
        });
        f.send(key(PhysicalKey::Enter));
        f.pump();
        assert_eq!(f.first_name(), "日本");
        assert_eq!(f.undo_signals, 1);
    }
}
