use crate::{Cell, GridCtx, Routine, cell::CellRole};
use indexmap::*;
use rusterix::Debug;
use theframework::prelude::*;

const VALUES: [&str; 5] = ["Boolean", "Float", "Integer", "String", "Variable"];
const OPERATORS: [&str; 3] = ["Arithmetic", "Assignment", "Comparison"];
const FUNCTIONS: [&str; 4] = ["add_item", "get_attr", "random_walk_in_sector", "set_attr"];

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
pub enum ModuleType {
    CharacterInstance,
    ItemInstance,
    #[default]
    CharacterTemplate,
    ItemTemplate,
}

impl ModuleType {
    pub fn is_instance(&self) -> bool {
        match self {
            ModuleType::CharacterInstance | ModuleType::ItemInstance => true,
            _ => false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Module {
    pub module_type: ModuleType,
    pub name: String,
    pub routines: IndexMap<Uuid, Routine>,
    grid_ctx: GridCtx,

    filter_text: String,
}

impl Module {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            grid_ctx: GridCtx::new(),
            ..Default::default()
        }
    }

    /// Sets the module type
    pub fn set_module_type(&mut self, module_type: ModuleType) {
        self.module_type = module_type;
        self.update_routines();
    }

    /// Checks if the given event exists
    pub fn contains(&self, event: &str) -> bool {
        for r in self.routines.values() {
            if r.name == event {
                return true;
            }
        }
        false
    }

    /// Add/ Update the routines of the module
    pub fn update_routines(&mut self) {
        if self.module_type.is_instance() {
            if !self.contains("instantiation") {
                let routine = Routine::new("instantiation".into(), "instance specifics".into());
                self.routines.insert(routine.id, routine);
            }
        } else if !self.contains("startup") {
            let routine = Routine::new("startup".into(), "send on creation".into());
            self.routines.insert(routine.id, routine);
        }
    }

    /// Read out the colors out of the style.
    pub fn get_colors(&mut self, ui: &mut TheUI) {
        self.grid_ctx.background_color = ui.style.theme().color(DefaultWidgetBackground).clone();
        self.grid_ctx.normal_color = ui.style.theme().color(CodeGridNormal).clone();
        self.grid_ctx.dark_color = ui.style.theme().color(CodeGridDark).clone();
        self.grid_ctx.selection_color = ui.style.theme().color(CodeGridSelected).clone();
        self.grid_ctx.text_color = ui.style.theme().color(CodeGridText).clone();
        self.grid_ctx.highlight_text_color = ui.style.theme().color(TextEditTextColor).clone();
        self.grid_ctx.error_color = ui.style.theme().color(Red).clone();
    }

    pub fn build_canvas(&self, ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        // Left code list

        let mut list_canvas: TheCanvas = TheCanvas::new();

        let mut list_toolbar_canvas = TheCanvas::new();

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_margin(Vec4::new(5, 2, 5, 2));
        toolbar_hlayout.set_background_color(None);

        // let mut filter_text = TheText::new(TheId::empty());
        // filter_text.set_text("Filter".to_string());
        // toolbar_hlayout.add_widget(Box::new(filter_text));

        let mut filter_edit = TheTextLineEdit::new(TheId::named("Code Editor Filter Edit"));
        filter_edit.set_text("".to_string());
        filter_edit.limiter_mut().set_max_size(Vec2::new(140, 22)); // 95
        filter_edit.set_font_size(12.5);
        filter_edit.set_embedded(true);
        filter_edit.set_status_text("Show content containing the given text.");
        filter_edit.set_continuous(true);
        filter_edit.set_info_text(Some("Filter".into()));
        toolbar_hlayout.add_widget(Box::new(filter_edit));
        list_toolbar_canvas.set_layout(toolbar_hlayout);
        list_toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        list_canvas.set_top(list_toolbar_canvas);

        let mut code_layout = TheListLayout::new(TheId::named("Code Editor Code List"));
        code_layout.limiter_mut().set_max_width(180);

        self.build_item_list(&mut code_layout, ctx);
        // code_layout.select_first_item(ctx);

        list_canvas.set_layout(code_layout);
        canvas.set_left(list_canvas);

        // --

        let render_view = TheRenderView::new(TheId::named("ModuleView"));

        // let mut context_menu = TheContextMenu::named(str!("Context"));
        // context_menu.add(TheContextMenuItem::new(
        //     str!("Assignment"),
        //     TheId::named("CGFAssignment"),
        // ));
        // context_menu.add(TheContextMenuItem::new(str!("Set"), TheId::named("Setter")));
        // file_menu.add_separator();
        //render_view.set_context_menu(Some(context_menu));

        canvas.set_widget(render_view);

        canvas
    }

    pub fn build_item_list(&self, list: &mut dyn TheListLayoutTrait, ctx: &mut TheContext) {
        list.clear();

        if self.filter_text.is_empty() || "event".contains(&self.filter_text) {
            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Event".into());
            item.set_associated_layout(list.id().clone());
            item.set_background_color(TheColor::from(CellRole::Event.to_color()));
            list.add_item(item, ctx);
        }

        let color = CellRole::Value.to_color();
        for item_name in VALUES {
            if self.filter_text.is_empty() || item_name.to_lowercase().contains(&self.filter_text) {
                let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
                item.set_text(item_name.to_string());
                item.set_associated_layout(list.id().clone());
                item.set_background_color(TheColor::from(color));
                list.add_item(item, ctx);
            }
        }

        let color = CellRole::Operator.to_color();
        for item_name in OPERATORS {
            if self.filter_text.is_empty() || item_name.to_lowercase().contains(&self.filter_text) {
                let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
                item.set_text(item_name.to_string());
                item.set_associated_layout(list.id().clone());
                item.set_background_color(TheColor::from(color));
                list.add_item(item, ctx);
            }
        }

        let color = CellRole::Function.to_color();
        for item_name in FUNCTIONS {
            if self.filter_text.is_empty() || item_name.to_lowercase().contains(&self.filter_text) {
                let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
                item.set_text(item_name.to_string());
                item.set_associated_layout(list.id().clone());
                item.set_background_color(TheColor::from(color));
                if let Some(cell) = Cell::from_str(item_name) {
                    item.set_status_text(&cell.status());
                }
                list.add_item(item, ctx);
            }
        }
    }

    pub fn draw(&mut self, buffer: &mut TheRGBABuffer) {
        buffer.fill(self.grid_ctx.background_color);

        let mut y: i32 = self.grid_ctx.offset_y;
        for r in self.routines.values_mut() {
            if y < buffer.dim().height {
                buffer.copy_into(0, y as i32, &r.buffer);
                r.module_offset = y as u32;
                y += r.buffer.dim().height;

                r.visible = true;
            } else {
                r.visible = false;
            }
        }
    }

    pub fn redraw(&mut self, ui: &mut TheUI, ctx: &TheContext) {
        self.get_colors(ui);
        if let Some(renderview) = ui.get_render_view("ModuleView") {
            *renderview.render_buffer_mut() = TheRGBABuffer::new(TheDim::new(
                0,
                0,
                renderview.dim().width,
                renderview.dim().height,
            ));
            for r in self.routines.values_mut() {
                r.set_screen_width(renderview.dim().width as u32, ctx, &self.grid_ctx);
                r.draw(ctx, &self.grid_ctx, 0, None);
            }
            self.draw(renderview.render_buffer_mut());
        }
    }

    pub fn redraw_debug(&mut self, ui: &mut TheUI, ctx: &TheContext, id: u32, debug: &Debug) {
        self.get_colors(ui);
        if let Some(renderview) = ui.get_render_view("ModuleView") {
            *renderview.render_buffer_mut() = TheRGBABuffer::new(TheDim::new(
                0,
                0,
                renderview.dim().width,
                renderview.dim().height,
            ));
            for r in self.routines.values_mut() {
                r.set_screen_width(renderview.dim().width as u32, ctx, &self.grid_ctx);
                r.draw(ctx, &self.grid_ctx, id, Some(debug));
            }
            self.draw(renderview.render_buffer_mut());
        }
    }

    /// Returns the selected routine
    pub fn get_selected_routine_mut(&mut self) -> Option<&mut Routine> {
        for r in self.routines.values_mut() {
            if Some(r.id) == self.grid_ctx.selected_routine {
                return Some(r);
            }
        }
        None
    }

    /// Handle events
    pub fn handle_event(&mut self, event: &TheEvent, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
        let mut redraw: bool = false;

        match event {
            TheEvent::WidgetResized(id, dim) => {
                if id.name == "ModuleView" {
                    // Set the screen widths in case something changed and the routines need a redraw.
                    for r in self.routines.values_mut() {
                        r.set_screen_width(dim.width as u32, ctx, &self.grid_ctx);
                    }

                    if let Some(renderview) = ui.get_render_view("ModuleView") {
                        *renderview.render_buffer_mut() =
                            TheRGBABuffer::new(TheDim::new(0, 0, dim.width, dim.height));
                        self.draw(renderview.render_buffer_mut());
                    }

                    redraw = true;
                }
            }
            TheEvent::RenderViewScrollBy(id, coord) => {
                if id.name == "ModuleView" {
                    if let Some(renderview) = ui.get_render_view("ModuleView") {
                        let view_port_height = renderview.dim().height;
                        let total_height = self.height();

                        self.grid_ctx.offset_y -= coord.y;
                        // Clamp offset_y so content stays within the visible area
                        let vp_h_i32 = view_port_height as i32;
                        let total_h_i32 = total_height as i32;
                        if total_h_i32 <= vp_h_i32 {
                            // Content fits entirely; lock to top
                            self.grid_ctx.offset_y = 0;
                        } else {
                            // Allowed range: [-(total - viewport), 0]
                            let min_offset = vp_h_i32 - total_h_i32; // negative value
                            let max_offset = 0;
                            if self.grid_ctx.offset_y < min_offset {
                                self.grid_ctx.offset_y = min_offset;
                            }
                            if self.grid_ctx.offset_y > max_offset {
                                self.grid_ctx.offset_y = max_offset;
                            }
                        }
                        self.draw(renderview.render_buffer_mut());
                    }
                }
            }
            TheEvent::KeyCodeDown(key) => {
                if let Some(focus) = &ctx.ui.focus {
                    if focus.name == "ModuleView" {
                        if let Some(key_code) = key.to_key_code() {
                            if key_code == TheKeyCode::Return {
                                if let Some(sel) = self.grid_ctx.current_cell.clone() {
                                    if let Some(routine) = self.get_selected_routine_mut() {
                                        routine.grid.return_at(sel.1);
                                        self.grid_ctx.current_cell = Some((sel.0, sel.1 + 1));
                                        self.redraw(ui, ctx);
                                    }
                                }
                            } else if key_code == TheKeyCode::Delete {
                                if let Some(sel) = self.grid_ctx.current_cell.clone() {
                                    if let Some(routine) = self.get_selected_routine_mut() {
                                        routine.grid.delete_at(sel.1);
                                        if sel.1 > 0 {
                                            self.grid_ctx.current_cell = Some((sel.0, sel.1 - 1));
                                        } else {
                                            self.grid_ctx.current_cell = Some((sel.0, 0));
                                        }
                                        self.redraw(ui, ctx);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Code Editor Filter Edit" {
                    self.filter_text = if let Some(widget) = ui
                        .canvas
                        .get_widget(Some(&"Code Editor Filter Edit".to_string()), None)
                    {
                        widget.value().to_string().unwrap_or_default()
                    } else {
                        "".to_string()
                    };
                    if let Some(list) = ui.get_list_layout("Code Editor Code List") {
                        self.build_item_list(list, ctx);
                    }
                } else if id.name == "cgfxEventName" {
                    if let Some(text) = value.to_string() {
                        let mut exists = false;

                        for r in self.routines.values_mut() {
                            if r.name == text {
                                exists = true;
                                break;
                            }
                        }

                        if !exists {
                            if let Some(r) = self.get_selected_routine_mut() {
                                r.name = text;
                                self.redraw(ui, ctx);
                            }
                        }
                    }
                } else if id.name.starts_with("cgfx") {
                    let prev = self.to_json();
                    for r in self.routines.values_mut() {
                        if Some(r.id) == self.grid_ctx.selected_routine {
                            if let Some(coord) = self.grid_ctx.current_cell {
                                if let Some(item) = r.grid.grid.get_mut(&coord) {
                                    item.apply_value(&id.name, value);
                                    r.draw(ctx, &self.grid_ctx, 0, None);
                                }
                            }
                        }
                    }
                    if let Some(renderview) = ui.get_render_view("ModuleView") {
                        self.draw(renderview.render_buffer_mut());
                    }
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("ModuleChanged"),
                        TheValue::Empty,
                    ));
                    ctx.ui.send(TheEvent::CustomUndo(
                        TheId::named("ModuleUndo"),
                        prev,
                        self.to_json(),
                    ));
                }
            }
            TheEvent::DragStarted(id, text, offset) => {
                if id.name == "Code Editor Code List Item" {
                    // if let Some(atom) = Some(self.create_atom(text.as_str(), id.uuid)) {
                    let mut drop = TheDrop::new(TheId::named("Code Editor Atom"));
                    // drop.set_data(atom.to_json());
                    drop.set_title(text.clone());
                    drop.set_offset(*offset);
                    ui.style.create_drop_image(&mut drop, ctx);
                    ctx.ui.set_drop(drop);
                    // }
                }
            }
            TheEvent::Drop(coord, drop) => {
                // println!("{}, {}", coord, drop.title);
                let mut handled = false;

                let prev = self.to_json();

                if drop.title == "Event" {
                    if self.module_type.is_instance() {
                        return false;
                    }

                    let routine = Routine::new("custom".into(), "".into());

                    self.grid_ctx.selected_routine = Some(routine.id);
                    self.grid_ctx.current_cell = None;

                    self.routines.insert(routine.id, routine);
                    self.redraw(ui, ctx);

                    self.show_event_settings(ui, ctx);
                } else {
                    for r in self.routines.values_mut() {
                        if r.visible {
                            handled = r.drop_at(
                                Vec2::new(coord.x as u32, coord.y as u32 - r.module_offset),
                                ui,
                                ctx,
                                &mut self.grid_ctx,
                                drop,
                            );
                            if handled {
                                break;
                            }
                        }
                    }
                }

                if handled {
                    self.redraw(ui, ctx);
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("ModuleChanged"),
                        TheValue::Empty,
                    ));

                    ctx.ui.send(TheEvent::CustomUndo(
                        TheId::named("ModuleUndo"),
                        prev,
                        self.to_json(),
                    ));

                    redraw = true;
                }
            }
            TheEvent::ContextMenuSelected(_id, _item) => {
                // if id.name == "ModuleView" {
                //     if let Some(group) = Group::from_str(&item.name) {
                //         if let Some(cell) = self.grid_ctx.current_cell.clone() {
                //             println!("1");
                //             for r in self.routines.values_mut() {
                //                 if Some(r.id) == self.grid_ctx.selected_routine {
                //                     r.draw(ctx, &self.grid_ctx);
                //                     break;
                //                 }
                //             }
                //         }
                //     }
                //     if let Some(renderview) = ui.get_render_view("ModuleView") {
                //         self.draw(renderview.render_buffer_mut());
                //         redraw = true;
                //     }
                // }
            }
            TheEvent::RenderViewContext(id, coord) => {
                if id.name == "ModuleView" {
                    for r in self.routines.values_mut() {
                        if r.visible {
                            if let Some(menu) = r.context_at(
                                Vec2::new(coord.x as u32, coord.y as u32 - r.module_offset),
                                ctx,
                                &mut self.grid_ctx,
                            ) {
                                r.draw(ctx, &mut self.grid_ctx, 0, None);
                                if let Some(renderview) = ui.get_render_view("ModuleView") {
                                    self.draw(renderview.render_buffer_mut());
                                    redraw = true;
                                }
                                ctx.ui
                                    .send(TheEvent::ShowContextMenu(id.clone(), *coord, menu));
                                break;
                            }
                        }
                    }
                }
            }
            TheEvent::RenderViewClicked(id, coord) => {
                if id.name == "ModuleView" {
                    let mut handled = false;
                    for r in self.routines.values_mut() {
                        if r.visible {
                            handled = r.click_at(
                                Vec2::new(coord.x as u32, coord.y as u32 - r.module_offset),
                                ui,
                                ctx,
                                &mut self.grid_ctx,
                            );
                            if handled {
                                if self.grid_ctx.current_cell == None {
                                    self.show_event_settings(ui, ctx);
                                }
                                break;
                            }
                        }
                    }
                    if handled {
                        self.redraw(ui, ctx);
                        // if let Some(renderview) = ui.get_render_view("ModuleView") {
                        //     self.draw(renderview.render_buffer_mut());
                        //     redraw = true;
                        // }
                    }
                }
            }
            _ => {}
        }

        redraw
    }

    /// Show the current settings
    pub fn show_settings(&mut self, _ui: &mut TheUI, _ctx: &mut TheContext) {
        return;
        /*
        let mut handled = false;
        for r in self.routines.values() {
            if Some(r.id) == self.grid_ctx.selected_routine {
                if let Some(pos) = self.grid_ctx.current_cell {
                    if let Some(item) = r.grid.grid.get(&pos) {
                        let nodeui: TheNodeUI = item.create_settings();
                        if let Some(layout) = ui.get_text_layout("Node Settings") {
                            nodeui.apply_to_text_layout(layout);
                            ctx.ui.relayout = true;

                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Show Node Settings"),
                                TheValue::Text(format!("{} Settings", item.cell.description())),
                            ));

                            handled = true;
                        }
                    }
                }
            }
        }

        if !handled {
            self.show_event_settings(ui, ctx);
        }*/
    }

    /// Show the settings for the current event.
    fn show_event_settings(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        let mut name = "".into();
        if let Some(r) = self.get_selected_routine_mut() {
            name = r.name.clone();
        }

        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::Text(
            "cgfxEventName".into(),
            "Event Name".into(),
            "Set the event name.".into(),
            name.clone(),
            None,
            false,
        );
        nodeui.add_item(item);

        if let Some(layout) = ui.get_text_layout("Node Settings") {
            nodeui.apply_to_text_layout(layout);
            ctx.ui.relayout = true;

            ctx.ui.send(TheEvent::Custom(
                TheId::named("Show Node Settings"),
                TheValue::Text(format!("\"{}\" Settings", name)),
            ));
        }
    }

    /// Returns the total height
    fn height(&self) -> u32 {
        let mut height = 0;
        for r in self.routines.values() {
            height += r.buffer.dim().height as u32;
        }
        height
    }

    /// Build the module into Python source
    pub fn build(&self, debug: bool) -> String {
        let mut out = String::new();

        if self.module_type == ModuleType::CharacterTemplate
            || self.module_type == ModuleType::ItemTemplate
        {
            out += &format!("class {}:\n", self.name);
            out += "    def event(self, event, value):\n";

            for r in self.routines.values() {
                r.build(&mut out, 8, debug);
            }
        } else {
            out += "def setup():\n";

            for r in self.routines.values() {
                r.build(&mut out, 4, debug);
            }
        }
        out
    }

    /// Load a module from a JSON string.
    pub fn from_json(json: &str) -> Self {
        let module: Module = serde_json::from_str(json).unwrap_or_default();
        module
    }

    /// Convert the module to a JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}
