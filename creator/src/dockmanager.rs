use crate::prelude::*;

pub struct DockManager {
    pub docks: IndexMap<String, Box<dyn Dock>>,

    pub dock: String,
    pub index: usize,
}

impl Default for DockManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DockManager {
    pub fn new() -> Self {
        let mut docks = IndexMap::default();

        let dock: Box<dyn Dock> = Box::new(TilesDock::new());
        docks.insert("Tiles".into(), dock);

        Self {
            docks,
            dock: "".into(),
            index: 0,
        }
    }

    pub fn init(&mut self) -> TheCanvas {
        let mut canvas: TheCanvas = TheCanvas::new();

        let mut shared_layout = TheSharedHLayout::new(TheId::named("Dock Shared Layout"));
        shared_layout.set_shared_ratio(1.0 - 0.27);
        shared_layout.set_mode(TheSharedHLayoutMode::Shared);

        // Main Stack

        let mut dock_canvas = TheCanvas::new();
        let mut dock_stack = TheStackLayout::new(TheId::named("Dock Stack"));

        for dock in &mut self.docks.values_mut() {
            let canvas = dock.setup();
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
        text.set_text("Action List".to_string());
        text.set_text_size(12.0);

        let mut action_apply_button = TheTraybarButton::new(TheId::named("Action Apply"));
        action_apply_button.set_text("Apply".to_string());
        action_apply_button.set_status_text("Apply the current action.");

        toolbar_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        toolbar_hlayout.set_padding(3);
        toolbar_hlayout.add_widget(Box::new(text));
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
            if let Some(index) = self.docks.get_index_of(&dock) {
                self.index = index;
                self.dock = dock;

                self.docks[index].activate(ui, ctx, project, server_ctx);
            } else {
                eprint!("Dock \"{}\" not found!", self.dock);
            }
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
        }
        redraw
    }
}
