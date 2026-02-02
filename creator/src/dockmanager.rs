use crate::editor::TOOLLIST;
use crate::prelude::*;

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

    pub supports_undo: bool,
}

impl Default for DockManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DockManager {
    pub fn new() -> Self {
        let mut docks = IndexMap::default();

        let dock: Box<dyn Dock> = Box::new(crate::docks::tiles::TilesDock::new());
        docks.insert("Tiles".into(), dock);

        let dock: Box<dyn Dock> = Box::new(crate::docks::visual_code::VisualCodeDock::new());
        docks.insert("Visual Code".into(), dock);

        let dock: Box<dyn Dock> = Box::new(crate::docks::code::CodeDock::new());
        docks.insert("Code".into(), dock);

        let dock: Box<dyn Dock> = Box::new(crate::docks::data::DataDock::new());
        docks.insert("Data".into(), dock);

        let dock: Box<dyn Dock> = Box::new(crate::docks::log::LogDock::new());
        docks.insert("Log".into(), dock);

        let dock: Box<dyn Dock> = Box::new(crate::docks::tilemap::TilemapDock::new());
        docks.insert("Tilemap".into(), dock);

        Self {
            state: DockManagerState::Minimized,
            docks,
            editor_canvases: IndexMap::default(),
            editor_docks: IndexMap::default(),
            dock: "".into(),
            index: 0,
            editor_index: None,
            supports_undo: false,
        }
    }

    pub fn init(&mut self, ctx: &mut TheContext) -> TheCanvas {
        let mut canvas: TheCanvas = TheCanvas::new();

        let mut shared_layout = TheSharedHLayout::new(TheId::named("Dock Shared Layout"));
        shared_layout.set_shared_ratio(1.0 - 0.27);
        shared_layout.set_mode(TheSharedHLayoutMode::Shared);

        // Main Stack

        let mut dock_canvas = TheCanvas::new();
        let mut dock_stack = TheStackLayout::new(TheId::named("Dock Stack"));

        for dock in &mut self.docks.values_mut() {
            let canvas = dock.setup(ctx);
            dock_stack.add_canvas(canvas);
        }

        dock_canvas.set_layout(dock_stack);
        shared_layout.add_canvas(dock_canvas);

        // Action Canvas
        let mut action_canvas: TheCanvas = TheCanvas::new();

        let mut toolbar_canvas = TheCanvas::default();
        let traybar_widget = TheTraybar::new(TheId::empty());
        toolbar_canvas.set_widget(traybar_widget);
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);

        let mut text = TheText::new(TheId::named("Action Text"));
        text.set_text(fl!("dock_auto"));
        text.set_text_size(12.0);

        let mut action_auto_button = TheCheckButton::new(TheId::named("Action Auto"));
        action_auto_button.set_status_text(&fl!("status_dock_action_auto"));
        action_auto_button.set_value(TheValue::Bool(true));

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

        let action_list_layout = TheListLayout::new(TheId::named("Action List"));
        action_canvas.set_layout(action_list_layout);
        action_canvas.set_top(toolbar_canvas);

        // ---

        shared_layout.add_canvas(action_canvas);

        canvas.set_layout(shared_layout);

        canvas
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
            self.minimize(ui, ctx);

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

            // Turn actions off / on
            if let Some(layout) = ui.get_sharedhlayout("Dock Shared Layout") {
                if self.docks[self.index].supports_actions() {
                    layout.set_mode(TheSharedHLayoutMode::Shared);
                } else {
                    layout.set_mode(TheSharedHLayoutMode::Left);
                }
            }

            if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                let state = self.docks[self.index].default_state();
                if state == DockDefaultState::Minimized {
                    self.state = DockManagerState::Minimized;
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
        redraw
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
    }

    /// Shows the editor of the current dock if available, otherwise maximizes the dock.
    pub fn edit_maximize(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(editor_index) = self.editor_index {
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

                if let Some(supports_undo) = supports_undo {
                    self.set_supports_undo(supports_undo, ctx);
                }
            }
        } else if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
            layout.set_mode(TheSharedVLayoutMode::Bottom);
            self.state = DockManagerState::Maximized;
        }
    }

    /// Shows the editor of the current dock if available, otherwise maximizes the dock.
    pub fn minimize(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        if self.state != DockManagerState::Minimized {
            // Switch back to game tools when minimizing from editor mode
            if self.state == DockManagerState::Editor {
                if let Some(editor_dock) = self.editor_docks.get_mut(&self.dock) {
                    editor_dock.minimized(ui, ctx);
                }
                TOOLLIST.write().unwrap().set_game_tools(ui, ctx);
            }

            if let Some(_editor_index) = self.editor_index {
                if let Some(stack) = ui.get_stack_layout("Editor Stack") {
                    stack.set_index(0);
                    self.state = DockManagerState::Minimized;
                }
            } else if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                layout.set_mode(TheSharedVLayoutMode::Shared);
                self.state = DockManagerState::Minimized;
            }

            self.set_supports_undo(self.docks[self.index].supports_undo(), ctx);
        }
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
}
