use crate::{Cell, CellItem, DebugModule, GridCtx, Routine, cell::CellRole};
use indexmap::*;
use theframework::prelude::*;

const BLOCKS: [&str; 3] = ["Event", "Var = ..", "If .. == .."];
const VALUES: [&str; 5] = ["Boolean", "Float", "Integer", "String", "Variable"];
const OPERATORS: [&str; 4] = ["Arithmetic", "Assignment", "Comparison", "Else"];
const USER_EVENTS: [&str; 2] = ["key_down", "key_up"];
const FUNCTIONS: [&str; 31] = [
    "action",
    "add_item",
    "block_events",
    "close_in",
    "deal_damage",
    "drop",
    "drop_items",
    "entities_in_radius",
    "equip",
    "get_attr",
    "get_attr_of",
    "goto",
    "id",
    "intent",
    "inventory_items",
    "inventory_items_of",
    "message",
    "notify_in",
    "offer_inventory",
    "random",
    "random_walk",
    "random_walk_in_sector",
    "set_attr",
    "set_emit_light",
    "set_player_camera",
    "set_proximity_tracking",
    "set_tile",
    "take",
    "teleport",
    "toggle_attr",
    "took_damage",
];

const SHADER_BLOCKS: [&str; 3] = ["Event", "Color = ..", "If .. == .."];
const SHADER_VALUES: [&str; 4] = ["Boolean", "Palette Color", "Value", "Variable"];
const SHADER_FUNCTIONS: [&str; 31] = [
    "abs",
    "atan",
    "atan2",
    "ceil",
    "clamp",
    "cos",
    "cross",
    "degrees",
    "dot",
    "exp",
    "floor",
    "fract",
    "length",
    "log",
    "max",
    "min",
    "mix",
    "mod",
    "normalize",
    "pow",
    "radians",
    "rand",
    "rotate2d",
    "sign",
    "sin",
    "smoothstep",
    "sample",
    "sample_normal",
    "sqrt",
    "step",
    "tan",
];

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone, Copy)]
pub enum ModuleType {
    Unknown,
    #[default]
    CharacterInstance,
    ItemInstance,
    CharacterTemplate,
    ItemTemplate,
    Shader,
}

impl ModuleType {
    pub fn is_instance(&self) -> bool {
        match self {
            ModuleType::CharacterInstance | ModuleType::ItemInstance => true,
            _ => false,
        }
    }

    pub fn is_shader(&self) -> bool {
        match self {
            ModuleType::Shader => true,
            _ => false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Module {
    #[serde(default)]
    pub id: Uuid,
    pub module_type: ModuleType,
    pub name: String,
    pub routines: IndexMap<Uuid, Routine>,
    #[serde(skip)]
    grid_ctx: GridCtx,
    #[serde(default)]
    pub player: bool,

    #[serde(default)]
    pub view_name: String,

    filter_text: String,
}

impl Module {
    /// Replace persisted module data while keeping runtime-only UI state
    /// (selection, scroll, view binding, filter text) intact.
    pub fn replace_preserving_runtime(&mut self, next: &Module) {
        let selected_routine_name = self
            .grid_ctx
            .selected_routine
            .and_then(|id| self.routines.get(&id))
            .map(|r| r.name.clone());
        let grid_ctx = self.grid_ctx.clone();
        let view_name = self.view_name.clone();
        let filter_text = self.filter_text.clone();

        *self = next.clone();

        self.grid_ctx = grid_ctx;
        self.view_name = view_name;
        self.filter_text = filter_text;

        // Reselect routine by name if UUID changed across undo/redo snapshots.
        if let Some(selected_id) = self.grid_ctx.selected_routine
            && !self.routines.contains_key(&selected_id)
        {
            self.grid_ctx.selected_routine = selected_routine_name.as_ref().and_then(|name| {
                self.routines
                    .iter()
                    .find(|(_, routine)| routine.name == *name)
                    .map(|(id, _)| *id)
            });
        }
    }

    fn toolbar_layout_id(&self) -> String {
        format!("{} Toolbar", self.view_name)
    }

    fn add_toolbar_filter_controls(&self, hlayout: &mut dyn TheHLayoutTrait) {
        let mut filter_text = TheText::new(TheId::empty());
        filter_text.set_text("Filter".to_string());
        hlayout.add_widget(Box::new(filter_text));

        let mut filter_edit = TheTextLineEdit::new(TheId::named("Code Editor Filter Edit"));
        filter_edit.set_text(self.filter_text.clone());
        filter_edit.limiter_mut().set_max_size(Vec2::new(120, 18));
        filter_edit.set_font_size(12.5);
        filter_edit.set_frameless(true);
        filter_edit.set_status_text("Show content containing the given text.");
        filter_edit.set_continuous(true);
        hlayout.add_widget(Box::new(filter_edit));
    }

    fn apply_toolbar_settings(&self, ui: &mut TheUI, ctx: &mut TheContext, nodeui: &TheNodeUI) {
        if let Some(layout) = ui.get_layout(&self.toolbar_layout_id())
            && let Some(hlayout) = layout.as_hlayout()
        {
            hlayout.clear();
            self.add_toolbar_filter_controls(hlayout);
            hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));

            for (_, item) in nodeui.list_items() {
                match item {
                    TheNodeUIItem::Text(id, name, status, value, default_value, continous) => {
                        if !name.is_empty() {
                            let mut label = TheText::new(TheId::empty());
                            label.set_text(name.clone());
                            hlayout.add_widget(Box::new(label));
                        }
                        let mut edit = TheTextLineEdit::new(TheId::named(id));
                        edit.set_text(value.clone());
                        edit.set_continuous(*continous);
                        edit.set_status_text(status);
                        edit.set_info_text(default_value.clone());
                        edit.set_font_size(12.5);
                        edit.set_frameless(true);
                        edit.limiter_mut().set_max_size(Vec2::new(280, 18));
                        hlayout.add_widget(Box::new(edit));
                    }
                    TheNodeUIItem::Selector(id, name, status, values, value) => {
                        if !name.is_empty() {
                            let mut label = TheText::new(TheId::empty());
                            label.set_text(name.clone());
                            hlayout.add_widget(Box::new(label));
                        }
                        let mut dropdown = TheDropdownMenu::new(TheId::named(id));
                        for option in values {
                            dropdown.add_option(option.clone());
                        }
                        dropdown.set_selected_index(*value);
                        dropdown.set_status_text(status);
                        dropdown.limiter_mut().set_max_size(Vec2::new(165, 18));
                        hlayout.add_widget(Box::new(dropdown));
                    }
                    TheNodeUIItem::FloatEditSlider(id, name, status, value, range, continous) => {
                        if !name.is_empty() {
                            let mut label = TheText::new(TheId::empty());
                            label.set_text(name.clone());
                            hlayout.add_widget(Box::new(label));
                        }
                        let mut edit = TheTextLineEdit::new(TheId::named(id));
                        edit.set_value(TheValue::Float(*value));
                        if *range.start() != 0.0 || *range.end() != 0.0 {
                            edit.set_range(TheValue::RangeF32(range.clone()));
                        }
                        edit.set_continuous(*continous);
                        edit.set_status_text(status);
                        edit.set_font_size(12.5);
                        edit.set_frameless(true);
                        edit.limiter_mut().set_max_size(Vec2::new(120, 18));
                        hlayout.add_widget(Box::new(edit));
                    }
                    TheNodeUIItem::IntEditSlider(id, name, status, value, range, continous) => {
                        if !name.is_empty() {
                            let mut label = TheText::new(TheId::empty());
                            label.set_text(name.clone());
                            hlayout.add_widget(Box::new(label));
                        }
                        let mut edit = TheTextLineEdit::new(TheId::named(id));
                        edit.set_value(TheValue::Int(*value));
                        if *range.start() != 0 || *range.end() != 0 {
                            edit.set_range(TheValue::RangeI32(range.clone()));
                        }
                        edit.set_continuous(*continous);
                        edit.set_status_text(status);
                        edit.set_font_size(12.5);
                        edit.set_frameless(true);
                        edit.limiter_mut().set_max_size(Vec2::new(100, 18));
                        hlayout.add_widget(Box::new(edit));
                    }
                    TheNodeUIItem::PaletteSlider(id, name, status, value, palette, continous) => {
                        if !name.is_empty() {
                            let mut label = TheText::new(TheId::empty());
                            label.set_text(name.clone());
                            hlayout.add_widget(Box::new(label));
                        }
                        let mut edit = TheTextLineEdit::new(TheId::named(id));
                        edit.set_value(TheValue::Int(*value));
                        edit.set_range(TheValue::RangeI32(0..=255));
                        edit.set_continuous(*continous);
                        edit.set_status_text(status);
                        edit.set_palette(palette.clone());
                        edit.set_font_size(12.5);
                        edit.set_frameless(true);
                        edit.limiter_mut().set_max_size(Vec2::new(100, 18));
                        hlayout.add_widget(Box::new(edit));
                    }
                    TheNodeUIItem::Checkbox(id, name, status, value) => {
                        if !name.is_empty() {
                            let mut label = TheText::new(TheId::empty());
                            label.set_text(name.clone());
                            hlayout.add_widget(Box::new(label));
                        }
                        let mut cb = TheCheckButton::new(TheId::named(id));
                        cb.set_value(TheValue::Bool(*value));
                        cb.set_status_text(status);
                        hlayout.add_widget(Box::new(cb));
                    }
                    _ => {}
                }
            }
            hlayout.relayout(ctx);
        }
    }

    /// Reset toolbar to filter-only controls (drops any stale value widgets).
    pub fn clear_toolbar_settings(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(layout) = ui.get_layout(&self.toolbar_layout_id())
            && let Some(hlayout) = layout.as_hlayout()
        {
            hlayout.clear();
            self.add_toolbar_filter_controls(hlayout);
            hlayout.relayout(ctx);
        }
    }

    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            grid_ctx: GridCtx::new(),
            ..Default::default()
        }
    }

    pub fn as_type(t: ModuleType) -> Self {
        Self {
            id: Uuid::new_v4(),
            module_type: t,
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
        println!("{:?}", self.module_type);
        if self.module_type.is_shader() {
            if !self.contains("shader") {
                let routine = Routine::new("shader".into());
                self.routines.insert(routine.id, routine);
            }
        } else if self.module_type.is_instance() {
            if !self.contains("instantiation") {
                let routine = Routine::new("instantiation".into());
                self.routines.insert(routine.id, routine);
            }
        } else if self.module_type != ModuleType::Unknown {
            if !self.contains("startup") {
                let routine = Routine::new("startup".into());
                self.routines.insert(routine.id, routine);
            }
            if self.module_type == ModuleType::CharacterTemplate {
                for event in USER_EVENTS {
                    // Search for the user_event id
                    let user_event_id = self
                        .routines
                        .iter()
                        .find(|(_, r)| r.name == event)
                        .map(|(id, _)| *id);
                    if !self.player {
                        // If not a player, make sure to delete the "user_event" routine if it exists
                        if let Some(id) = user_event_id {
                            self.routines.shift_remove(&id);
                        }
                    } else if user_event_id.is_none() {
                        // If a player and there is no user_event routine, add one
                        let mut routine = Routine::new(event.into());
                        routine.folded = true;
                        self.routines.insert(routine.id, routine);
                    }
                }
            }
        }
    }

    /// Read out the colors out of the style.
    // pub fn get_colors(&mut self, ui: &mut TheUI) {
    //     self.grid_ctx.background_color = ui.style.theme().color(DefaultWidgetBackground).clone();
    //     self.grid_ctx.normal_color = ui.style.theme().color(CodeGridNormal).clone();
    //     self.grid_ctx.dark_color = ui.style.theme().color(CodeGridDark).clone();
    //     self.grid_ctx.selection_color = ui.style.theme().color(CodeGridSelected).clone();
    //     self.grid_ctx.text_color = ui.style.theme().color(CodeGridText).clone();
    //     self.grid_ctx.highlight_text_color = ui.style.theme().color(TextEditTextColor).clone();
    //     self.grid_ctx.error_color = ui.style.theme().color(Red).clone();
    // }

    pub fn build_canvas(&mut self, ctx: &mut TheContext, name: &str) -> TheCanvas {
        self.view_name = name.to_string();

        let mut canvas = TheCanvas::new();

        // Left code list

        let mut list_canvas: TheCanvas = TheCanvas::new();

        let mut code_layout = TheListLayout::new(TheId::named("Code Editor Code List"));
        code_layout.limiter_mut().set_max_width(180);

        self.build_item_list(&mut code_layout, ctx);
        // code_layout.select_first_item(ctx);

        list_canvas.set_layout(code_layout);
        canvas.set_left(list_canvas);

        // --

        let render_view = TheRenderView::new(TheId::named(name));

        // let mut context_menu = TheContextMenu::named(str!("Context"));
        // context_menu.add(TheContextMenuItem::new(
        //     str!("Assignment"),
        //     TheId::named("CGFAssignment"),
        // ));
        // context_menu.add(TheContextMenuItem::new(str!("Set"), TheId::named("Setter")));
        // file_menu.add_separator();
        //render_view.set_context_menu(Some(context_menu));

        let mut module_toolbar_canvas = TheCanvas::default();
        module_toolbar_canvas.limiter_mut().set_max_height(28);
        module_toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));

        let mut module_toolbar_hlayout = TheHLayout::new(TheId::named(&self.toolbar_layout_id()));
        module_toolbar_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        module_toolbar_hlayout.set_padding(3);
        module_toolbar_hlayout.set_background_color(None);
        self.add_toolbar_filter_controls(&mut module_toolbar_hlayout);
        module_toolbar_canvas.set_layout(module_toolbar_hlayout);

        canvas.set_top(module_toolbar_canvas);
        canvas.set_widget(render_view);

        canvas
    }

    pub fn build_item_list(&self, list: &mut dyn TheListLayoutTrait, ctx: &mut TheContext) {
        list.clear();

        let color = CellRole::Event.to_color();

        if self.module_type.is_shader() {
            for item_name in SHADER_BLOCKS {
                if self.filter_text.is_empty()
                    || item_name.to_lowercase().contains(&self.filter_text)
                {
                    let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
                    item.set_text(item_name.to_string());
                    item.set_associated_layout(list.id().clone());
                    item.set_background_color(TheColor::from(color));
                    list.add_item(item, ctx);
                }
            }
        } else {
            for item_name in BLOCKS {
                if self.filter_text.is_empty()
                    || item_name.to_lowercase().contains(&self.filter_text)
                {
                    let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
                    item.set_text(item_name.to_string());
                    item.set_associated_layout(list.id().clone());
                    item.set_background_color(TheColor::from(color));
                    list.add_item(item, ctx);
                }
            }
        }

        if self.module_type.is_shader() {
            let color = CellRole::Value.to_color();
            for item_name in SHADER_VALUES {
                if self.filter_text.is_empty()
                    || item_name.to_lowercase().contains(&self.filter_text)
                {
                    let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
                    item.set_text(item_name.to_string());
                    item.set_associated_layout(list.id().clone());
                    item.set_background_color(TheColor::from(color));
                    list.add_item(item, ctx);
                }
            }
        } else {
            let color = CellRole::Value.to_color();
            for item_name in VALUES {
                if self.filter_text.is_empty()
                    || item_name.to_lowercase().contains(&self.filter_text)
                {
                    let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
                    item.set_text(item_name.to_string());
                    item.set_associated_layout(list.id().clone());
                    item.set_background_color(TheColor::from(color));
                    list.add_item(item, ctx);
                }
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

        if self.module_type.is_shader() {
            let color = CellRole::Function.to_color();
            for item_name in SHADER_FUNCTIONS {
                if self.filter_text.is_empty()
                    || item_name.to_lowercase().contains(&self.filter_text)
                {
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
        } else if !self.module_type.is_shader() {
            let color = CellRole::Function.to_color();
            for item_name in FUNCTIONS {
                if self.filter_text.is_empty()
                    || item_name.to_lowercase().contains(&self.filter_text)
                {
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
    }

    pub fn draw(&mut self, buffer: &mut TheRGBABuffer) {
        buffer.fill([116, 116, 116, 255]);

        let mut y: i32 = self.grid_ctx.offset_y;
        for r in self.routines.values_mut() {
            if y < buffer.dim().height {
                buffer.copy_into(self.grid_ctx.offset_x, y as i32, &r.buffer);
                // Store content-space offset (without scroll) for hit testing
                r.module_offset = y - self.grid_ctx.offset_y;
                y += r.buffer.dim().height;

                r.visible = true;
            } else {
                r.visible = false;
            }
        }
    }

    pub fn redraw(&mut self, ui: &mut TheUI, ctx: &TheContext) {
        if let Some(renderview) = ui.get_render_view(&self.get_view_name()) {
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

    pub fn redraw_debug(&mut self, ui: &mut TheUI, ctx: &TheContext, id: u32, debug: &DebugModule) {
        if let Some(renderview) = ui.get_render_view(&self.get_view_name()) {
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

    /// Copy a library module into the routine t the given position.
    pub fn insert_module(&mut self, module: &Module, coord: Vec2<i32>) -> bool {
        let header_height = 35;

        // Translate click coordinate into content space (accounts for scrolling)
        let content = Vec2::new(
            coord.x - self.grid_ctx.offset_x,
            coord.y - self.grid_ctx.offset_y,
        );

        for r in self.routines.values_mut() {
            if r.visible {
                let loc_y = content.y - r.module_offset;
                if loc_y < 0 {
                    continue;
                }
                let loc = Vec2::new(content.x.max(0) as u32, loc_y as u32);
                // TODO: Check for body hit too
                if loc.y < header_height {
                    if let Some(shader) = module.routines.get_index(0) {
                        r.id = *shader.0;
                        r.grid = shader.1.grid.clone();
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Handle events
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        palette: &ThePalette,
    ) -> bool {
        let mut redraw: bool = false;

        match event {
            TheEvent::WidgetResized(id, dim) => {
                if id.name == self.get_view_name() {
                    // Set the screen widths in case something changed and the routines need a redraw.
                    for r in self.routines.values_mut() {
                        r.set_screen_width(dim.width as u32, ctx, &self.grid_ctx);
                    }

                    if let Some(renderview) = ui.get_render_view(&self.get_view_name()) {
                        *renderview.render_buffer_mut() =
                            TheRGBABuffer::new(TheDim::new(0, 0, dim.width, dim.height));
                        self.draw(renderview.render_buffer_mut());
                    }

                    redraw = true;
                }
            }
            TheEvent::RenderViewScrollBy(id, coord) => {
                if id.name == self.get_view_name() {
                    if let Some(renderview) = ui.get_render_view(&self.get_view_name()) {
                        let view_port_height = renderview.dim().height;
                        let view_port_width = renderview.dim().width;
                        let total_height = self.height();
                        let total_width = self.width();

                        self.grid_ctx.offset_y -= coord.y;
                        self.grid_ctx.offset_x -= coord.x;
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

                        // Clamp offset_x similarly
                        let vp_w_i32 = view_port_width as i32;
                        let total_w_i32 = total_width as i32;
                        if total_w_i32 <= vp_w_i32 {
                            self.grid_ctx.offset_x = 0;
                        } else {
                            let min_offset = vp_w_i32 - total_w_i32;
                            let max_offset = 0;
                            if self.grid_ctx.offset_x < min_offset {
                                self.grid_ctx.offset_x = min_offset;
                            }
                            if self.grid_ctx.offset_x > max_offset {
                                self.grid_ctx.offset_x = max_offset;
                            }
                        }
                        self.draw(renderview.render_buffer_mut());
                    }
                }
            }
            TheEvent::KeyCodeDown(key) => {
                if ui.focus_widget_supports_text_input(ctx) {
                    return redraw;
                }
                if let Some(focus) = &ctx.ui.focus {
                    if focus.name == self.get_view_name() {
                        let prev = self.to_json();
                        if let Some(key_code) = key.to_key_code() {
                            if key_code == TheKeyCode::Return {
                                if let Some(sel) = self.grid_ctx.current_cell.clone() {
                                    if let Some(routine) = self.get_selected_routine_mut() {
                                        if ui.shift {
                                            routine.grid.return_sibling_at(sel.1);
                                        } else {
                                            routine.grid.return_at(sel.1);
                                        }
                                        self.grid_ctx.current_cell = Some((sel.0, sel.1 + 1));
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
                                    }
                                } else {
                                    // Return on header
                                    if let Some(routine) = self.get_selected_routine_mut() {
                                        routine.grid.shift_rows_down_from(0, 1);
                                        routine.grid.insert((0, 0), CellItem::new(Cell::Empty));
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
                                } else if let Some(r) = self.grid_ctx.selected_routine {
                                    self.routines.shift_remove(&r);
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
                        if self.grid_ctx.selected_routine.is_none() {
                            if let Some((id, _)) = self.routines.first() {
                                self.grid_ctx.selected_routine = Some(*id);
                            }
                        }

                        if let Some(selected_id) = self.grid_ctx.selected_routine {
                            // Only block duplicates from other routines; allow keeping current name.
                            let exists_elsewhere = self
                                .routines
                                .iter()
                                .any(|(id, routine)| *id != selected_id && routine.name == text);

                            if !exists_elsewhere {
                                let prev = self.to_json();
                                let mut changed = false;
                                if let Some(routine) = self.routines.get_mut(&selected_id) {
                                    if routine.name != text {
                                        routine.name = text;
                                        changed = true;
                                    }
                                }

                                if changed {
                                    self.redraw(ui, ctx);
                                    self.show_event_settings(ui, ctx);
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
                        }
                    }
                }
                if id.name == "cgfxPixelization" {
                    let prev = self.to_json();
                    if let Some(value) = value.to_i32() {
                        if let Some(event) = self.get_selected_routine_mut() {
                            event.pixelization = value;

                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("ModuleChanged"),
                                TheValue::Bool(true),
                            ));
                            ctx.ui.send(TheEvent::CustomUndo(
                                TheId::named("ModuleUndo"),
                                prev,
                                self.to_json(),
                            ));
                        }
                    }
                } else if id.name == "cgfxColorSteps" {
                    let prev = self.to_json();
                    if let Some(value) = value.to_i32() {
                        if let Some(event) = self.get_selected_routine_mut() {
                            event.color_steps = value;

                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("ModuleChanged"),
                                TheValue::Bool(true),
                            ));
                            ctx.ui.send(TheEvent::CustomUndo(
                                TheId::named("ModuleUndo"),
                                prev,
                                self.to_json(),
                            ));
                        }
                    }
                } else if id.name == "cgfxRotation" {
                    let prev = self.to_json();
                    if let Some(value) = value.to_f32() {
                        if let Some(event) = self.get_selected_routine_mut() {
                            event.rotation = value;

                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("ModuleChanged"),
                                TheValue::Bool(true),
                            ));
                            ctx.ui.send(TheEvent::CustomUndo(
                                TheId::named("ModuleUndo"),
                                prev,
                                self.to_json(),
                            ));
                        }
                    }
                } else if id.name == "cgfxScale" {
                    let prev = self.to_json();
                    if let Some(value) = value.to_f32() {
                        if let Some(event) = self.get_selected_routine_mut() {
                            event.scale = value;

                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("ModuleChanged"),
                                TheValue::Bool(true),
                            ));
                            ctx.ui.send(TheEvent::CustomUndo(
                                TheId::named("ModuleUndo"),
                                prev,
                                self.to_json(),
                            ));
                        }
                    }
                } else if id.name.starts_with("cgfx") {
                    let prev = self.to_json();
                    let mut needs_update = true;
                    for r in self.routines.values_mut() {
                        if Some(r.id) == self.grid_ctx.selected_routine {
                            if let Some(coord) = self.grid_ctx.current_cell {
                                if let Some(item) = r.grid.grid.get_mut(&coord) {
                                    needs_update =
                                        item.apply_value(&id.name, value, self.module_type);
                                    r.draw(ctx, &self.grid_ctx, 0, None);
                                }
                            }
                        }
                    }
                    if let Some(renderview) = ui.get_render_view(&self.get_view_name()) {
                        self.draw(renderview.render_buffer_mut());
                    }
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("ModuleChanged"),
                        TheValue::Bool(needs_update),
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
                let mut handled = false;
                let prev = self.to_json();
                let mut settings: Option<TheNodeUI> = None;
                let content_x = coord.x as i32 - self.grid_ctx.offset_x;
                let content_y = coord.y as i32 - self.grid_ctx.offset_y;

                if drop.title == "Event" {
                    if self.module_type.is_instance() {
                        return false;
                    }

                    let routine = Routine::new("custom".into());

                    self.grid_ctx.selected_routine = Some(routine.id);
                    self.grid_ctx.current_cell = None;

                    // Make sure to always insert before potential user events
                    let mut insert_before = None;
                    for (index, r) in self.routines.values().enumerate() {
                        if r.name == USER_EVENTS[0] {
                            insert_before = Some(index);
                            break;
                        }
                    }

                    if let Some(insert_before) = insert_before {
                        self.routines
                            .insert_before(insert_before, routine.id, routine);
                    } else {
                        self.routines.insert(routine.id, routine);
                    }

                    self.redraw(ui, ctx);

                    self.show_event_settings(ui, ctx);
                } else {
                    for r in self.routines.values_mut() {
                        if r.visible {
                            let local_y = content_y - r.module_offset;
                            if local_y < 0 {
                                continue;
                            }
                            handled = r.drop_at(
                                Vec2::new(content_x.max(0) as u32, local_y as u32),
                                ctx,
                                &mut self.grid_ctx,
                                drop,
                                self.module_type,
                                palette,
                                &mut settings,
                            );
                            if handled {
                                break;
                            }
                        }
                    }
                }

                if handled {
                    if let Some(settings) = &settings {
                        self.apply_toolbar_settings(ui, ctx, settings);
                    } else if self.grid_ctx.current_cell.is_none() {
                        self.show_event_settings(ui, ctx);
                    }
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
                if id.name == self.get_view_name() {
                    let content_x = coord.x as i32 - self.grid_ctx.offset_x;
                    let content_y = coord.y as i32 - self.grid_ctx.offset_y;
                    for r in self.routines.values_mut() {
                        if r.visible {
                            let local_y = content_y - r.module_offset;
                            if local_y < 0 {
                                continue;
                            }
                            let loc = Vec2::new(content_x.max(0) as u32, local_y as u32);
                            if let Some(menu) = r.context_at(loc, ctx, &mut self.grid_ctx) {
                                r.draw(ctx, &mut self.grid_ctx, 0, None);
                                if let Some(renderview) = ui.get_render_view(&self.get_view_name())
                                {
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
                if id.name == self.get_view_name() {
                    let mut settings: Option<TheNodeUI> = None;
                    let content_x = coord.x as i32 - self.grid_ctx.offset_x;
                    let content_y = coord.y as i32 - self.grid_ctx.offset_y;
                    let mut handled = false;
                    for r in self.routines.values_mut() {
                        if r.visible {
                            let local_y = content_y - r.module_offset;
                            if local_y < 0 {
                                continue;
                            }
                            handled = r.click_at(
                                Vec2::new(content_x.max(0) as u32, local_y as u32),
                                ctx,
                                &mut self.grid_ctx,
                                self.module_type,
                                palette,
                                &mut settings,
                            );
                            if handled {
                                if self.grid_ctx.current_cell == None {
                                    self.show_event_settings(ui, ctx);
                                } else if let Some(settings) = &settings {
                                    self.apply_toolbar_settings(ui, ctx, settings);
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

    /// Refresh the current toolbar settings from the selected cell or event.
    pub fn show_settings(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        let palette = ThePalette::default();
        let mut handled = false;

        for r in self.routines.values_mut() {
            if Some(r.id) == self.grid_ctx.selected_routine {
                if let Some(pos) = self.grid_ctx.current_cell {
                    if let Some(item) = r.grid.grid.get(&pos) {
                        let nodeui = item.create_settings(&palette, self.module_type);
                        self.apply_toolbar_settings(ui, ctx, &nodeui);
                        handled = true;
                    }
                }
                break;
            }
        }

        if !handled {
            self.show_event_settings(ui, ctx);
        }
    }

    /// Show the settings for the current event.
    fn show_event_settings(&mut self, ui: &mut TheUI, _ctx: &mut TheContext) {
        let mut name = "".into();
        let mut pixelization = 0;
        let mut color_steps = 0;
        let mut rotation = 0.0;
        let mut scale = 1.0;
        if let Some(r) = self.get_selected_routine_mut() {
            name = r.name.clone();
            pixelization = r.pixelization;
            rotation = r.rotation;
            scale = r.scale;
            color_steps = r.color_steps;
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

        if self.module_type.is_shader() {
            let item = TheNodeUIItem::Text(
                "cgfxEventName".into(),
                "Event Name".into(),
                "Set the event name.".into(),
                name.clone(),
                None,
                false,
            );
            nodeui.add_item(item);

            let item = TheNodeUIItem::FloatEditSlider(
                "cgfxScale".into(),
                "Scale".into(),
                "Set the scale for this shader. Scales the UV component.".into(),
                scale,
                0.1..=4.0,
                false,
            );
            nodeui.add_item(item);

            let item = TheNodeUIItem::FloatEditSlider(
                "cgfxRotation".into(),
                "Rotatation".into(),
                "Set the rotation for this shader. Rotates the UV component.".into(),
                rotation,
                0.0..=360.0,
                false,
            );
            nodeui.add_item(item);

            let item = TheNodeUIItem::IntEditSlider(
                "cgfxPixelization".into(),
                "Pixelization".into(),
                "Set the pixelization for this shader. A value of 0 disables pixelization.".into(),
                pixelization,
                0..=256,
                false,
            );
            nodeui.add_item(item);

            let item = TheNodeUIItem::IntEditSlider(
                "cgfxColorSteps".into(),
                "Color Steps".into(),
                "Set the color steps for this shader. A value of 0 disables color stepping.".into(),
                color_steps,
                0..=256,
                false,
            );
            nodeui.add_item(item);
        }

        self.apply_toolbar_settings(ui, _ctx, &nodeui);
    }

    /// Returns the total height
    fn height(&self) -> u32 {
        let mut height = 0;
        for r in self.routines.values() {
            height += r.buffer.dim().height as u32;
        }
        height
    }

    /// Returns the maximum width among routines
    fn width(&self) -> u32 {
        self.routines
            .values()
            .map(|r| r.buffer.dim().width as u32)
            .max()
            .unwrap_or(0)
    }

    /// Set the backround for a shader
    pub fn set_shader_background(
        &mut self,
        buffer: TheRGBABuffer,
        ui: &mut TheUI,
        ctx: &TheContext,
    ) {
        if self.module_type == ModuleType::Shader {
            if let Some((_, routine)) = self.routines.first_mut() {
                routine.shader_background = buffer;
                self.redraw(ui, ctx);
            }
        }
    }

    /// Build shader code
    pub fn build_shader(&self) -> String {
        let mut out = String::new();

        if self.module_type.is_shader() {
            for r in self.routines.values() {
                if r.name == "shader" {
                    r.build_shader(&mut out, 0);
                    break;
                }
            }
        }

        // if !out.is_empty() {
        //     println!("{}", out);
        // }

        out
    }

    /// Build shader code: ceiling
    pub fn build_custom_shader(&self, name: &str) -> String {
        let mut out = String::new();

        if self.module_type.is_shader() {
            for r in self.routines.values() {
                if r.name == name {
                    r.build_shader(&mut out, 0);
                    break;
                }
            }
        }

        out
    }

    /// Build the module into script source
    pub fn build(&self, debug: bool) -> String {
        let mut out = String::new();

        if self.module_type == ModuleType::CharacterTemplate
            || self.module_type == ModuleType::ItemTemplate
        {
            out += "fn event(event, value) {\n";

            let mut contains_user_events = false;

            // Build non user_events first
            for r in self.routines.values() {
                if !USER_EVENTS.contains(&r.name.as_str()) {
                    r.build_source(&mut out, 4, debug);
                } else {
                    contains_user_events = true;
                }
            }
            out += "}\n\n";

            if contains_user_events {
                out += "fn user_event(event, value) {\n";
                // Build user_event (if any)
                for r in self.routines.values() {
                    if USER_EVENTS.contains(&r.name.as_str()) {
                        r.build_source(&mut out, 4, debug);
                    }
                }
                out += "}\n";
            }
        } else {
            out += "fn setup() {\n";

            for r in self.routines.values() {
                r.build_source(&mut out, 4, debug);
            }
        }
        out
    }

    /// Returns the view name for the module type
    pub fn get_view_name(&self) -> String {
        return self.view_name.clone();
        // if self.module_type == ModuleType::Shader {
        //     return "ShadeModuleView";
        // }
        // "CodeModuleView"
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

use std::cmp::PartialEq;

impl PartialEq for Module {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}
