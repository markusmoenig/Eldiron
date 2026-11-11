//! Native prefab graph binding. The pane applies ordered edits to its authoring
//! document/project undo; the widget owns transient controller/input state only.
use super::*;
use std::sync::{Arc, Mutex};
pub(super) type NativePrefabState = Arc<Mutex<GraphViewState>>;

impl PrefabNodePane {
    fn native_owner(&self) -> String {
        format!("prefab/{:?}/{:?}", self.asset, self.part)
    }
    pub(super) fn replace_native_document(&mut self) {
        if let Some(state) = &self.native {
            let mut state = state.lock().unwrap();
            state.replace_document(self.native_owner(), self.doc.clone());
            let viewport = self.editor.viewport;
            let selected = self.editor.selected;
            let error = self.store_error.clone();
            state.edit(|_, editor| {
                editor.viewport = viewport;
                editor.selected = selected;
                editor.error = error;
            });
        }
    }
    pub(super) fn render_native(&mut self, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
        if !ctx.ui.command_paint {
            return false;
        }
        let Some(widget) = ui.get_widget(VIEW) else {
            return true;
        };
        let state = if let Some(state) = &self.native {
            state.clone()
        } else {
            let mut state = GraphViewState::new(self.native_owner(), self.doc.clone());
            let viewport = self.editor.viewport;
            let selected = self.editor.selected;
            state.edit(|_, editor| {
                editor.viewport = viewport;
                editor.selected = selected;
            });
            let state = Arc::new(Mutex::new(state));
            self.native = Some(state.clone());
            state
        };
        let bound = widget
            .as_any()
            .downcast_mut::<TheGraphView>()
            .is_some_and(|view| Arc::ptr_eq(&view.model(), &state));
        if !bound {
            let id = widget.id().clone();
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
    fn native_target_exists(&self, project: &Project) -> bool {
        self.asset.zip(self.part).is_some_and(|(asset, part)| {
            project
                .block_props
                .get(&asset)
                .is_some_and(|asset| asset.find_part(part).is_some())
        })
    }
    fn collect_native_through(
        &mut self,
        serial: u64,
        project: &mut Project,
        ctx: &mut TheContext,
    ) -> Result<bool, String> {
        let Some(state) = self.native.clone() else {
            return Ok(false);
        };
        if !self.native_target_exists(project) {
            return Ok(false);
        }
        let (binding, commits) = {
            let state = state.lock().unwrap();
            if state.owner() != self.native_owner() {
                return Ok(false);
            }
            (state.binding(), state.pending_through(serial))
        };
        let mut changed = false;
        for commit in commits {
            let before = self.committed.clone();
            for edit in &commit.edits {
                edit.apply(&mut self.doc, true);
            }
            self.store(project, ctx);
            changed |= before != self.committed;
            // The pane has adopted the authoring draft even if compilation
            // rejected it. A later correction can progress past that draft.
            if !state.lock().unwrap().acknowledge(binding, commit.serial) {
                return Err("Prefab graph changed document while collecting its edits".into());
            }
        }
        let error = self.store_error.clone();
        state
            .lock()
            .unwrap()
            .edit(|_, editor| editor.error = error.clone());
        error.map_or(Ok(changed), Err)
    }
    /// Synchronous document barrier for Save/Play/owner transitions. This neither
    /// finishes the inline session nor writes preedit into the serialized project.
    pub(in crate::docks) fn commit_native_pending(
        &mut self,
        project: &mut Project,
        ctx: &mut TheContext,
    ) -> Result<bool, String> {
        let Some(state) = &self.native else {
            return Ok(false);
        };
        let serial = {
            let mut state = state.lock().unwrap();
            state.checkpoint_text();
            state.last_serial()
        };
        self.collect_native_through(serial, project, ctx)
    }
    pub(super) fn native_event(
        &mut self,
        event: &TheEvent,
        _: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
    ) -> Option<bool> {
        match event {
            TheEvent::GraphCommitted(id, commit) if id.name == VIEW => {
                let current = self
                    .native
                    .as_ref()
                    .is_some_and(|state| state.lock().unwrap().binding() == commit.binding);
                if !current {
                    return Some(false);
                }
                let _ = self.collect_native_through(commit.serial, project, ctx);
                Some(true)
            }
            TheEvent::GraphDrop(id, binding, point, drop) if id.name == VIEW => {
                let current = self
                    .native
                    .as_ref()
                    .is_some_and(|state| state.lock().unwrap().binding() == *binding);
                if !current || !self.native_target_exists(project) {
                    return Some(false);
                }
                let Some(key) = drop
                    .id
                    .name
                    .strip_prefix("Node Catalog/")
                    .filter(|key| node_available(key))
                else {
                    return Some(false);
                };
                let _ = self.commit_native_pending(project, ctx);
                {
                    let mut state = self.native.as_ref().unwrap().lock().unwrap();
                    state.edit(|document, editor| editor.cancel_gesture(document));
                    self.editor.viewport = state.editor().viewport;
                    self.editor.selected = state.editor().selected;
                    self.editor.selected_connection = state.editor().selected_connection;
                }
                // The application owns the new-node transaction. The point is
                // already in graph space; do not use a newer viewport camera.
                self.doc.nodes.push(
                    prefab_graph::definitions()
                        .node(key)
                        .unwrap()
                        .instantiate(*point),
                );
                self.store(project, ctx);
                self.replace_native_document();
                Some(true)
            }
            TheEvent::RenderViewDrop(id, _, _) if id.name == VIEW && self.native.is_some() => {
                Some(false) // native hosts must provide the addressed drop contract
            }
            // Diagram clipboard/undo commands are owned by the application's
            // capabilities. Prefab's advertised inline text commands are local;
            // its document undo continues through Creator's project undo owner.
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
        let applied = self.commit_native_pending(project, ctx);
        if matches!(event, TheEvent::NewListItemSelected(id, _) if id.name == "Prefab Branch") {
            applied?;
        }
        let state = self.native.as_ref().unwrap();
        let mut state = state.lock().unwrap();
        state.edit(|document, editor| editor.cancel_gesture(document));
        self.doc = state.document().clone();
        self.editor.viewport = state.editor().viewport;
        self.editor.selected = state.editor().selected;
        self.editor.selected_connection = state.editor().selected_connection;
        Ok(true)
    }
    pub(in crate::docks) fn has_native_pending(&self) -> bool {
        self.store_error.is_some()
            || self.native.as_ref().is_some_and(|state| {
                let state = state.lock().unwrap();
                state.pending_commits().next().is_some()
                    || (state.editor().text_focus().is_some() && state.document() != &self.doc)
            })
    }
    pub(in crate::docks) fn native_source_asset(&self, binding: u64) -> Option<Uuid> {
        self.native
            .as_ref()
            .filter(|state| state.lock().unwrap().binding() == binding)
            .and(self.asset)
    }
    pub(in crate::docks) fn reset_native_project(&mut self) {
        if let Some(state) = self.native.take() {
            state
                .lock()
                .unwrap()
                .replace_document("", GraphDocument::default());
        }
        self.asset = None;
        self.part = None;
        self.doc = GraphDocument::default();
        self.committed = GraphDocument::default();
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
