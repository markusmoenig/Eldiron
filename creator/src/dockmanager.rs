use crate::editor::TOOLLIST;
use crate::prelude::*;
use rusterix::prelude::EldrinDebugModule;

#[derive(Clone, Copy, PartialEq)]
pub enum DockManagerState {
    Minimized,
    Maximized,
    Editor,
}

pub struct DockManager {
    pub state: DockManagerState,

    pub docks: IndexMap<String, Box<dyn Dock>>,

    pub editor_canvases: IndexMap<String, usize>,
    pub editor_docks: IndexMap<String, Box<dyn Dock>>,

    pub dock: String,
    pub index: usize,
    pub editor_index: Option<usize>,
    normal_split_ratio: f32,

    pub supports_undo: bool,
    pub auto_text_play_prev_dock: Option<String>,
    pub auto_text_play_active: bool,
    prefab_return_context: Option<(ProjectContext, EditorViewMode, Uuid)>,
    prefab_return_orbit: Option<(Vec3<f32>, f32)>,
    prefab_return_preview_post: Option<bool>,
}

impl Default for DockManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DockManager {
    pub fn edit_maximize_accelerator() -> TheAccelerator {
        TheAccelerator::new(TheAcceleratorKey::CTRLCMD, '[')
    }

    pub fn restore_accelerator() -> TheAccelerator {
        TheAccelerator::new(TheAcceleratorKey::CTRLCMD, ']')
    }

    /// Builds an action panel backed by the global action model.
    ///
    /// The panel is intentionally independent from the dock canvas so hosts can
    /// place it where it best fits their layout (the Creator sidebar uses it).
    pub(crate) fn action_panel(list_id: &str) -> TheCanvas {
        let mut action_canvas = TheCanvas::new();

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);

        let mut text = TheText::new(TheId::named("Action Text"));
        text.set_text(fl!("dock_auto"));
        text.set_text_size(12.0);

        let mut action_auto_button = TheCheckButton::new(TheId::named("Action Auto"));
        action_auto_button.set_status_text(&fl!("status_dock_action_auto"));
        action_auto_button.set_value(TheValue::Bool(false));

        let mut action_apply_button = TheTraybarButton::new(TheId::named("Action Apply"));
        action_apply_button.set_text(fl!("apply"));
        action_apply_button.set_status_text(&fl!("status_dock_action_apply"));

        toolbar_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        toolbar_hlayout.set_padding(3);
        toolbar_hlayout.add_widget(Box::new(text));
        toolbar_hlayout.add_widget(Box::new(action_auto_button));
        toolbar_hlayout.add_widget(Box::new(action_apply_button));
        toolbar_hlayout.set_reverse_index(Some(1));
        toolbar_canvas.set_layout(toolbar_hlayout);

        action_canvas.set_layout(TheListLayout::new(TheId::named(list_id)));
        action_canvas.set_top(toolbar_canvas);
        action_canvas
    }

    pub fn new() -> Self {
        let mut docks = IndexMap::default();

        let dock: Box<dyn Dock> = Box::new(crate::docks::tiles::TilesDock::new());
        docks.insert("Tiles".into(), dock);

        let dock: Box<dyn Dock> = Box::new(crate::docks::blocks::BlocksDock::new());
        docks.insert("Prefabs".into(), dock);

        let dock: Box<dyn Dock> = Box::new(crate::docks::builder::BuilderDock::new());
        docks.insert("Builder".into(), dock);

        let dock: Box<dyn Dock> = Box::new(crate::docks::palette::PaletteDock::new());
        docks.insert("Palette".into(), dock);

        let dock: Box<dyn Dock> = Box::new(crate::docks::iso_paint::IsoPaintDock::new());
        docks.insert("3D Paint".into(), dock);

        let dock: Box<dyn Dock> = Box::new(crate::docks::authoring::AuthoringDock::new());
        docks.insert("Authoring".into(), dock);

        let dock: Box<dyn Dock> = Box::new(crate::docks::code::CodeDock::new());
        docks.insert("Code".into(), dock);

        let dock: Box<dyn Dock> = Box::new(crate::docks::data::DataDock::new());
        docks.insert("Data".into(), dock);

        let dock: Box<dyn Dock> = Box::new(crate::docks::tilemap::TilemapDock::new());
        docks.insert("Tilemap".into(), dock);

        let dock: Box<dyn Dock> = Box::new(crate::docks::text_play::TextPlayDock::new());
        docks.insert("Text Play".into(), dock);

        Self {
            state: DockManagerState::Minimized,
            docks,
            editor_canvases: IndexMap::default(),
            editor_docks: IndexMap::default(),
            dock: "".into(),
            index: 0,
            editor_index: None,
            normal_split_ratio: crate::DEFAULT_VLAYOUT_RATIO,
            supports_undo: false,
            auto_text_play_prev_dock: None,
            auto_text_play_active: false,
            prefab_return_context: None,
            prefab_return_orbit: None,
            prefab_return_preview_post: None,
        }
    }

    /// Builds only the dock area. Action panels are mounted by their host.
    pub fn init_docks(&mut self, ctx: &mut TheContext) -> TheCanvas {
        let mut dock_canvas = TheCanvas::new();
        let mut dock_stack = TheStackLayout::new(TheId::named("Dock Stack"));

        for dock in &mut self.docks.values_mut() {
            let canvas = dock.setup(ctx);
            dock_stack.add_canvas(canvas);
        }

        dock_canvas.set_layout(dock_stack);
        dock_canvas
    }

    pub fn remember_normal_split(&mut self, ui: &mut TheUI) {
        if let Some(layout) = ui.get_sharedvlayout("Shared VLayout")
            && layout.get_mode() == TheSharedVLayoutMode::Shared
        {
            self.normal_split_ratio = layout.get_shared_ratio();
        }
    }

    fn restore_normal_split(&self, ui: &mut TheUI) {
        if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
            layout.set_shared_ratio(self.normal_split_ratio);
            layout.set_mode(TheSharedVLayoutMode::Shared);
        }
    }

    pub fn sync_size_controls(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        if self.state == DockManagerState::Minimized {
            ui.set_disabled("Dock Restore", ctx);
            if self.dock.is_empty() {
                ui.set_disabled("Dock Edit Maximize", ctx);
            } else {
                ui.set_enabled("Dock Edit Maximize", ctx);
            }
        } else {
            ui.set_enabled("Dock Restore", ctx);
            ui.set_disabled("Dock Edit Maximize", ctx);
        }
    }

    pub fn set_dock(
        &mut self,
        dock: String,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        if dock != self.dock {
            self.minimize(ui, ctx, project, server_ctx);

            if let Some(index) = self.docks.get_index_of(&dock) {
                self.index = index;
                self.dock = dock;

                if let Some(stack) = ui.get_stack_layout("Dock Stack") {
                    stack.set_index(index);
                }

                self.editor_index = self.editor_canvases.get(&self.dock).copied();
            } else {
                eprint!("Dock \"{}\" not found!", self.dock);
                return;
            }

            if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                let state = self.docks[self.index].default_state();
                if state == DockDefaultState::Minimized {
                    self.state = DockManagerState::Minimized;
                    layout.set_shared_ratio(self.normal_split_ratio);
                    layout.set_mode(TheSharedVLayoutMode::Shared);
                } else {
                    self.state = DockManagerState::Maximized;
                    layout.set_mode(TheSharedVLayoutMode::Bottom);
                }
            }
        }
        self.docks[self.index].activate(ui, ctx, project, server_ctx);
        self.set_supports_undo(self.docks[self.index].supports_undo(), ctx);
        if self.supports_undo {
            self.docks[self.index].set_undo_state_to_ui(ctx);
        }
        self.sync_size_controls(ui, ctx);
    }

    pub fn import(
        &mut self,
        content: String,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some((_, dock)) = self.docks.get_index_mut(self.index) {
            dock.import(content.clone(), ui, ctx, project, server_ctx);

            if let Some(editor_dock) = self.editor_docks.get_mut(&self.dock) {
                editor_dock.import(content, ui, ctx, project, server_ctx);
            }
        }
    }

    pub fn export(&self) -> Option<String> {
        if let Some((_, dock)) = self.docks.get_index(self.index) {
            dock.export()
        } else {
            None
        }
    }

    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        if let Some((_, dock)) = self.docks.get_index_mut(self.index) {
            redraw = dock.handle_event(event, ui, ctx, project, server_ctx);

            if let Some(editor_dock) = self.editor_docks.get_mut(&self.dock) {
                if editor_dock.handle_event(event, ui, ctx, project, server_ctx) {
                    redraw = true;
                }
            }
        }
        if self.dock != "Recipes"
            && matches!(
                event,
                TheEvent::Custom(id, _)
                    if id.name == "Render Procedural Recipe Preview"
                        || id.name == crate::docks::recipes::RECIPE_SOURCE_CHANGED
            )
            && let Some(recipe_editor) = self.editor_docks.get_mut("Recipes")
            && recipe_editor.handle_event(event, ui, ctx, project, server_ctx)
        {
            redraw = true;
        }
        redraw
    }

    /// Poll dock-owned workers without relying on their wake-up event being
    /// routed through whichever dock happens to be visible at that moment.
    pub fn poll_background(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        self.editor_docks
            .get_mut("Recipes")
            .is_some_and(|dock| dock.poll_background(ui, ctx, project, server_ctx))
    }

    /// Returns the state of the dock manager.
    pub fn get_state(&self) -> DockManagerState {
        self.state
    }

    /// Add the dock editors to the stack and maps.
    pub fn add_editors_to_stack(&mut self, stack: &mut TheStackLayout, ctx: &mut TheContext) {
        let mut tiles_editor: Box<dyn Dock> =
            Box::new(crate::docks::tiles_editor::TilesEditorDock::new());
        let tiles_editor_canvas = tiles_editor.setup(ctx);
        let index = stack.add_canvas(tiles_editor_canvas);
        self.editor_canvases.insert("Tiles".to_string(), index);
        self.editor_docks.insert("Tiles".to_string(), tiles_editor);

        let mut recipe_editor: Box<dyn Dock> =
            Box::new(crate::docks::recipes::RecipeEditorDock::new());
        let recipe_editor_canvas = recipe_editor.setup(ctx);
        let index = stack.add_canvas(recipe_editor_canvas);
        self.editor_canvases.insert("Recipes".to_string(), index);
        self.editor_docks
            .insert("Recipes".to_string(), recipe_editor);

        let mut builder_editor: Box<dyn Dock> =
            Box::new(crate::docks::builder_editor::BuilderEditorDock::new());
        let builder_editor_canvas = builder_editor.setup(ctx);
        let index = stack.add_canvas(builder_editor_canvas);
        self.editor_canvases.insert("Builder".to_string(), index);
        self.editor_docks
            .insert("Builder".to_string(), builder_editor);

        let mut data_editor: Box<dyn Dock> =
            Box::new(crate::docks::data_editor::DataEditorDock::new());
        let data_editor_canvas = data_editor.setup(ctx);
        let index = stack.add_canvas(data_editor_canvas);
        self.editor_canvases.insert("Data".to_string(), index);
        self.editor_docks.insert("Data".to_string(), data_editor);

        let mut prefabs_editor: Box<dyn Dock> =
            Box::new(crate::docks::prefabs_editor::PrefabsEditorDock::new());
        let prefabs_editor_canvas = prefabs_editor.setup(ctx);
        let index = stack.add_canvas(prefabs_editor_canvas);
        self.editor_canvases.insert("Prefabs".to_string(), index);
        self.editor_docks
            .insert("Prefabs".to_string(), prefabs_editor);
    }

    /// Shows the editor of the current dock if available, otherwise maximizes the dock.
    pub fn edit_maximize(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        self.remember_normal_split(ui);
        if self.dock == "Prefabs" {
            let mut asset_id = server_ctx.curr_block_asset_id;
            if let Some(id) = asset_id
                && !project.block_props.contains_key(&id)
                && let Some(built_in) = crate::blocks::block_asset(id)
            {
                let editable = crate::blocks::editable_prefab_from_block_asset(built_in);
                let editable_id = editable.id;
                let editable_name = editable.name.clone();
                project.block_props.insert(editable_id, editable);
                server_ctx.curr_block_asset_id = Some(editable_id);
                server_ctx.curr_block_asset_name = Some(editable_name.clone());
                asset_id = Some(editable_id);
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    format!("Created editable Prefab '{editable_name}' from the built-in asset"),
                ));
            }
            if let Some(id) = asset_id
                && !project.block_props.contains_key(&id)
                && let Some(bundled) = crate::blocks::bundled_effect_prefab(id)
            {
                project.block_props.insert(id, bundled.clone());
            }
            let Some(asset_id) = asset_id.filter(|id| project.block_props.contains_key(id)) else {
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    "Select a Prefab to edit".to_string(),
                ));
                return;
            };
            let prefab_changed =
                crate::blocks::upgrade_legacy_effect_prefab_geometry(project, asset_id)
                    | crate::blocks::ensure_prefab_default_surfaces(project, asset_id);
            if prefab_changed {
                crate::undo::project_helper::refresh_palette_runtime(project);
            }
            if let Err(message) = crate::block_props::begin_prefab_editor(project, asset_id) {
                ctx.ui
                    .send(TheEvent::SetStatusText(TheId::empty(), message));
                return;
            }
            self.prefab_return_context = Some((
                server_ctx.pc,
                server_ctx.editor_view_mode,
                server_ctx.curr_region,
            ));
            self.prefab_return_preview_post = Some({
                let mut rusterix = crate::editor::RUSTERIX.write().unwrap();
                let enabled = rusterix.editor_preview_post_enabled;
                rusterix.editor_preview_post_enabled = false;
                enabled
            });
            server_ctx.pc = ProjectContext::Prefab(asset_id);
            server_ctx.editor_view_mode = EditorViewMode::Orbit;
            server_ctx.geometry_edit_mode = GeometryEditMode::Geometry;

            if let Some((center, distance)) =
                crate::block_props::prefab_editor_camera_frame(project)
            {
                let mut camera = crate::editor::EDITCAMERA.write().unwrap();
                self.prefab_return_orbit =
                    Some((camera.orbit_camera.center, camera.orbit_camera.distance));
                camera.orbit_camera.center = center;
                camera.orbit_camera.distance = distance;
            }
            {
                let camera = crate::editor::EDITCAMERA
                    .read()
                    .unwrap()
                    .orbit_camera
                    .clone();
                let mut rusterix = crate::editor::RUSTERIX.write().unwrap();
                rusterix.scene_handler.vm.set_active_vm(0);
                rusterix.scene_handler.clear_runtime_scene();
                rusterix.client.scene.d3_overlay.clear();
                rusterix.scene_handler.clear_overlay();
                rusterix
                    .scene_handler
                    .vm
                    .execute(scenevm::Atom::ClearRaster3DPaintOverlay);
                rusterix
                    .scene_handler
                    .vm
                    .execute(scenevm::Atom::ClearPaintBillboards);
                rusterix
                    .scene_handler
                    .vm
                    .execute(scenevm::Atom::ClearOrganicBillboards);
                rusterix
                    .scene_handler
                    .vm
                    .execute(scenevm::Atom::ClearAvatarBillboardData);
                rusterix.client.set_camera_d3(Box::new(camera));
            }
            if let Some(group) = ui
                .get_widget("Editor View Switch")
                .and_then(|widget| widget.as_group_button())
            {
                group.set_index(EditorViewMode::Orbit.to_index());
            }
            crate::utils::editor_scene_full_rebuild(project, server_ctx);
            let initial_tool = project
                .block_props
                .get(&asset_id)
                .is_some_and(|asset| {
                    !asset.particle_effects.is_empty() || !asset.light_effects.is_empty()
                })
                .then_some("tool.effects")
                .unwrap_or("tool.geometry");
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Set Tool"),
                TheValue::Text(initial_tool.to_string()),
            ));
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                fl!(
                    "status_prefab_editor_open",
                    name = server_ctx.curr_block_asset_name.as_deref().unwrap_or("")
                ),
            ));
        }
        let use_editor_canvas = if self.dock == "Data" {
            matches!(server_ctx.pc, ProjectContext::CharacterPreviewRigging(_))
        } else {
            self.editor_index.is_some()
        };

        if use_editor_canvas {
            let Some(editor_index) = self.editor_index else {
                return;
            };
            if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                layout.set_mode(TheSharedVLayoutMode::Top);
            }
            ctx.ui.relayout = true;
            ctx.ui.redraw_all = true;
            if let Some(stack) = ui.get_stack_layout("Editor Stack") {
                stack.set_index(editor_index);
                self.state = DockManagerState::Editor;

                let mut supports_undo = None;
                if let Some(editor_dock) = self.editor_docks.get_mut(&self.dock) {
                    editor_dock.activate(ui, ctx, project, server_ctx);
                    supports_undo = Some(editor_dock.supports_undo());
                    if let Some(supports_undo) = supports_undo
                        && supports_undo
                    {
                        editor_dock.set_undo_state_to_ui(ctx);
                    }

                    // Switch to editor tools if the dock provides them
                    if let Some(tools) = editor_dock.editor_tools() {
                        TOOLLIST.write().unwrap().set_editor_tools(tools, ui, ctx);
                    }
                }

                if self.dock == "Prefabs" {
                    TOOLLIST
                        .write()
                        .unwrap()
                        .set_prefab_tools(ui, ctx, server_ctx);
                }

                if let Some(supports_undo) = supports_undo {
                    self.set_supports_undo(supports_undo, ctx);
                }
            }
        } else if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
            layout.set_mode(TheSharedVLayoutMode::Bottom);
            self.state = DockManagerState::Maximized;
        }
        self.sync_size_controls(ui, ctx);
    }

    /// Open a dock that exists only as a full-screen editor. Recipes use this
    /// path deliberately: their tree already provides the catalog, so a second
    /// regular list dock must never be constructed or exposed.
    pub fn set_editor_dock(
        &mut self,
        dock: String,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        let Some(editor_index) = self.editor_canvases.get(&dock).copied() else {
            eprintln!("Editor dock \"{dock}\" not found!");
            return;
        };

        if self.state == DockManagerState::Editor && self.dock != dock {
            self.minimize(ui, ctx, project, server_ctx);
        } else if self.state != DockManagerState::Editor {
            self.minimize(ui, ctx, project, server_ctx);
        }

        self.dock = dock;
        self.editor_index = Some(editor_index);
        if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
            layout.set_mode(TheSharedVLayoutMode::Top);
        }
        if let Some(stack) = ui.get_stack_layout("Editor Stack") {
            stack.set_index(editor_index);
        }
        self.state = DockManagerState::Editor;
        ctx.ui.relayout = true;
        ctx.ui.redraw_all = true;

        let mut supports_undo = false;
        if let Some(editor_dock) = self.editor_docks.get_mut(&self.dock) {
            editor_dock.activate(ui, ctx, project, server_ctx);
            supports_undo = editor_dock.supports_undo();
            if supports_undo {
                editor_dock.set_undo_state_to_ui(ctx);
            }
            if let Some(tools) = editor_dock.editor_tools() {
                TOOLLIST.write().unwrap().set_editor_tools(tools, ui, ctx);
            }
        }
        self.set_supports_undo(supports_undo, ctx);
        self.sync_size_controls(ui, ctx);
    }

    fn minimize_inner(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
        restore_game_tools: bool,
    ) {
        if self.state != DockManagerState::Minimized {
            // Switch back to game tools when minimizing from editor mode
            if self.state == DockManagerState::Editor {
                if let Some(editor_dock) = self.editor_docks.get_mut(&self.dock) {
                    editor_dock.minimized(ui, ctx);
                }
                if restore_game_tools {
                    TOOLLIST.write().unwrap().set_game_tools(ui, ctx);
                    if self.dock == "Prefabs" {
                        server_ctx.palette_tool_active = false;
                    }
                }
                if let Some(stack) = ui.get_stack_layout("Editor Stack") {
                    stack.set_index(0);
                }
                self.restore_normal_split(ui);
                ctx.ui.relayout = true;
                ctx.ui.redraw_all = true;
                self.state = DockManagerState::Minimized;

                // Editor-only docks have no corresponding lower canvas. Put
                // the manager back on the regular dock whose stack index was
                // preserved when the editor opened.
                if !self.docks.contains_key(&self.dock)
                    && let Some((regular_dock, _)) = self.docks.get_index(self.index)
                {
                    self.dock = regular_dock.clone();
                    self.editor_index = self.editor_canvases.get(&self.dock).copied();
                }
            } else {
                self.restore_normal_split(ui);
                self.state = DockManagerState::Minimized;
            }

            self.set_supports_undo(self.docks[self.index].supports_undo(), ctx);
        }
        self.sync_size_controls(ui, ctx);

        // Restore an isolated Prefab workspace whenever one is pending. Do not
        // key this off the currently selected dock: tool and sidebar events can
        // change the dock before the minimize event reaches this manager, which
        // used to leave `server_ctx.pc` pointing at the hidden PrefabView.
        if let Some((pc, view_mode, region_id)) = self.prefab_return_context.take() {
            if let Some((center, distance)) = self.prefab_return_orbit.take() {
                let mut camera = crate::editor::EDITCAMERA.write().unwrap();
                camera.orbit_camera.center = center;
                camera.orbit_camera.distance = distance;
            }
            server_ctx.pc = if pc.is_prefab() {
                // A Prefab context is editor-only and can never be a useful
                // return target for the regular viewport. This also recovers a
                // workspace left behind by an interrupted earlier close.
                ProjectContext::Region(region_id)
            } else {
                pc
            };
            server_ctx.editor_view_mode = view_mode;
            server_ctx.curr_region = region_id;

            {
                let mut rusterix = crate::editor::RUSTERIX.write().unwrap();
                if let Some(enabled) = self.prefab_return_preview_post.take() {
                    rusterix.editor_preview_post_enabled = enabled;
                }
                rusterix.scene_handler.vm.set_active_vm(0);
                rusterix.scene_handler.clear_runtime_scene();
                rusterix.scene_handler.build_index.clear();
                rusterix.client.scene.d3_overlay.clear();
                rusterix.scene_handler.clear_overlay();
                rusterix.scene_handler.set_overlay();
            }

            if let Some(group) = ui
                .get_widget("Editor View Switch")
                .and_then(|widget| widget.as_group_button())
            {
                group.set_index(view_mode.to_index());
            }
            crate::utils::editor_scene_full_rebuild(project, server_ctx);
            // Prefab editing reuses the regular geometry and paint tools. Their
            // selection and preview are editor-local state and must not leak
            // back into the region workspace when the isolated editor closes.
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Set Tool"),
                TheValue::Text("tool.blocks".to_string()),
            ));
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Update Geometry Overlay 3D"),
                TheValue::Empty,
            ));
            ctx.ui.redraw_all = true;
        }
    }

    /// Shows the editor of the current dock if available, otherwise maximizes the dock.
    pub fn minimize(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        self.minimize_inner(ui, ctx, project, server_ctx, true);
    }

    /// Minimize during game-tool switching, while the tool list is already locked by the caller.
    pub fn minimize_for_tool_switch(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        self.minimize_inner(ui, ctx, project, server_ctx, false);
    }

    /// Returns true if the current dock (either the editor dock or the normal dock) supports undo.
    pub fn current_dock_supports_undo(&self) -> bool {
        self.supports_undo
    }

    /// Sets the undo support.
    fn set_supports_undo(&mut self, supports_undo: bool, ctx: &mut TheContext) {
        if !supports_undo {
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Set Project Undo State"),
                TheValue::Empty,
            ));
        }
        self.supports_undo = supports_undo;
    }

    pub fn undo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if self.state == DockManagerState::Editor {
            if let Some(editor_dock) = self.editor_docks.get_mut(&self.dock) {
                editor_dock.undo(ui, ctx, project, server_ctx);
            }
        } else {
            self.docks[self.index].undo(ui, ctx, project, server_ctx);
        }
    }

    pub fn redo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if self.state == DockManagerState::Editor {
            if let Some(editor_dock) = self.editor_docks.get_mut(&self.dock) {
                editor_dock.redo(ui, ctx, project, server_ctx);
            }
        } else {
            self.docks[self.index].redo(ui, ctx, project, server_ctx);
        }
    }

    /// Returns true if the current (visible) dock needs animated minimap updates.
    pub fn current_dock_supports_minimap_animation(&self) -> bool {
        match self.state {
            DockManagerState::Editor => self
                .editor_docks
                .get(&self.dock)
                .map(|d| d.supports_minimap_animation())
                .unwrap_or(false),
            _ => self
                .docks
                .get_index(self.index)
                .map(|(_, d)| d.supports_minimap_animation())
                .unwrap_or(false),
        }
    }

    /// Get the currently active dock (editor dock if in editor mode, otherwise the current dock)
    pub fn get_active_dock(&self) -> Option<&dyn Dock> {
        if self.state == DockManagerState::Editor {
            self.editor_docks.get(&self.dock).map(|d| d.as_ref())
        } else {
            Some(self.docks[self.index].as_ref())
        }
    }

    /// Check if any dock has unsaved changes in its undo stack
    pub fn has_dock_changes(&self) -> bool {
        // Check all regular docks
        for dock in self.docks.values() {
            if dock.has_changes() {
                return true;
            }
        }

        // Check all editor docks
        for dock in self.editor_docks.values() {
            if dock.has_changes() {
                return true;
            }
        }

        false
    }

    /// Mark all dock-local undo states as saved.
    pub fn mark_saved(&mut self) {
        for dock in self.docks.values_mut() {
            dock.mark_saved();
        }
        for dock in self.editor_docks.values_mut() {
            dock.mark_saved();
        }
    }

    /// Clear state that belongs to the project which is being deactivated.
    pub fn reset_for_project_switch(&mut self) {
        for dock in self.docks.values_mut() {
            dock.reset_for_project_switch();
        }
        for dock in self.editor_docks.values_mut() {
            dock.reset_for_project_switch();
        }
        self.auto_text_play_prev_dock = None;
        self.auto_text_play_active = false;
        self.supports_undo = false;
        self.prefab_return_context = None;
        self.prefab_return_orbit = None;
        self.prefab_return_preview_post = None;
    }

    pub fn apply_eldrin_debug_data(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &ServerContext,
        debug: &EldrinDebugModule,
    ) {
        if self.state == DockManagerState::Editor {
            if let Some(editor_dock) = self.editor_docks.get_mut(&self.dock) {
                editor_dock.apply_eldrin_debug_data(ui, ctx, project, server_ctx, debug);
            }
        } else if let Some((_, dock)) = self.docks.get_index_mut(self.index) {
            dock.apply_eldrin_debug_data(ui, ctx, project, server_ctx, debug);
        }
    }

    pub fn sync_text_play_dock(
        &mut self,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
        is_running: bool,
    ) {
        let toollist = TOOLLIST.read().unwrap();
        let current_tool_name = toollist.game_tools[toollist.curr_game_tool]
            .id()
            .name
            .clone();
        let in_game_tool_mode = server_ctx.game_mode || current_tool_name == "Game Tool";
        let should_show = is_running
            && server_ctx.text_game_mode
            && !server_ctx.game_mode
            && !toollist.editor_mode
            && current_tool_name != "Game Tool"
            && server_ctx.get_map_context() == MapContext::Region;
        drop(toollist);

        if should_show {
            if !self.auto_text_play_active {
                self.auto_text_play_prev_dock = if !self.dock.is_empty() && self.dock != "Text Play"
                {
                    Some(self.dock.clone())
                } else {
                    None
                };
                self.auto_text_play_active = true;
            }

            if self.dock != "Text Play" {
                self.set_dock("Text Play".into(), _ui, ctx, project, server_ctx);
            }
        } else if self.auto_text_play_active {
            if in_game_tool_mode {
                self.auto_text_play_active = false;
                self.auto_text_play_prev_dock = None;
                return;
            }

            let restore_dock = self.auto_text_play_prev_dock.take();
            self.auto_text_play_active = false;

            if self.dock == "Text Play" {
                if let Some(dock) = restore_dock {
                    self.set_dock(dock, _ui, ctx, project, server_ctx);
                } else {
                    self.minimize(_ui, ctx, project, server_ctx);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipes_are_not_registered_as_a_regular_dock() {
        let manager = DockManager::new();
        assert!(!manager.docks.contains_key("Recipes"));
    }

    #[test]
    fn prefabs_dock_exposes_the_shared_action_list() {
        let manager = DockManager::new();
        assert!(manager.docks["Prefabs"].supports_actions());
    }
}
