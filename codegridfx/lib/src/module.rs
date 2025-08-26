use crate::{GridCtx, Routine};
use indexmap::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
pub enum ModuleType {
    CharacterInstance,
    ItemInstance,
    #[default]
    CharacterTemplate,
    ItemTemplate,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Module {
    pub module_type: ModuleType,
    pub name: String,
    pub routines: IndexMap<String, Routine>,
    grid_ctx: GridCtx,

    filter_text: String,

    values: Vec<String>,
    operators: Vec<String>,
    functions: Vec<String>,
}

impl Module {
    pub fn new(name: &str) -> Self {
        let values = vec![
            "Boolean".into(),
            "Float".into(),
            "Integer".into(),
            "String".into(),
            "Variable".into(),
        ];
        let operators = vec!["Assignment".into()];
        let functions = vec!["get_attr".into(), "set_attr".into()];
        Self {
            name: name.into(),
            grid_ctx: GridCtx::new(),
            values,
            operators,
            functions,
            ..Default::default()
        }
    }

    /// Sets the module type
    pub fn set_module_type(&mut self, module_type: ModuleType) {
        self.module_type = module_type;
        self.update_routines();
    }

    /// Add/ Update the routines of the module
    pub fn update_routines(&mut self) {
        if !self.routines.contains_key("startup") {
            let routine = Routine::new("startup".into(), "called on creation".into());
            self.routines.insert(routine.name.clone(), routine);
        }
    }

    /// Read out the colors out of the style.
    pub fn get_colors(&mut self, ui: &mut TheUI) {
        self.grid_ctx.background_color = ui.style.theme().color(DefaultWidgetBackground).clone();
        self.grid_ctx.normal_color = ui.style.theme().color(CodeGridNormal).clone();
        self.grid_ctx.dark_color = ui.style.theme().color(CodeGridDark).clone();
        self.grid_ctx.selection_color = ui.style.theme().color(CodeGridSelected).clone();
        self.grid_ctx.text_color = ui.style.theme().color(CodeGridText).clone();
        self.grid_ctx.highlight_text_color = [170, 170, 170, 255];
        // self.grid_ctx.highlight_text_color = ui.style.theme().color(TextEditTextColor).clone();
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

        let mut filter_text = TheText::new(TheId::empty());
        filter_text.set_text("Filter".to_string());

        toolbar_hlayout.add_widget(Box::new(filter_text));
        let mut filter_edit = TheTextLineEdit::new(TheId::named("Code Editor Filter Edit"));
        filter_edit.set_text("".to_string());
        filter_edit.limiter_mut().set_max_size(Vec2::new(95, 18));
        filter_edit.set_font_size(12.5);
        filter_edit.set_embedded(true);
        filter_edit.set_status_text("Show content containing the given text.");
        filter_edit.set_continuous(true);
        toolbar_hlayout.add_widget(Box::new(filter_edit));
        list_toolbar_canvas.set_layout(toolbar_hlayout);
        list_toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        list_canvas.set_top(list_toolbar_canvas);

        let mut code_layout = TheListLayout::new(TheId::named("Code Editor Code List"));
        code_layout.limiter_mut().set_max_width(150);
        // self.get_code_list_items(0, &mut code_layout, ctx);

        self.build_item_list(&mut code_layout, ctx);
        code_layout.select_first_item(ctx);

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

        for item_name in &self.values {
            if self.filter_text.is_empty() || item_name.to_lowercase().contains(&self.filter_text) {
                let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
                item.set_text(item_name.clone());
                item.set_associated_layout(list.id().clone());
                item.set_background_color(TheColor::from([200, 230, 201, 255]));
                list.add_item(item, ctx);
            }
        }

        for item_name in &self.operators {
            if self.filter_text.is_empty() || item_name.to_lowercase().contains(&self.filter_text) {
                let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
                item.set_text(item_name.clone());
                item.set_associated_layout(list.id().clone());
                item.set_background_color(TheColor::from([255, 249, 196, 255]));
                list.add_item(item, ctx);
            }
        }

        for item_name in &self.functions {
            if self.filter_text.is_empty() || item_name.to_lowercase().contains(&self.filter_text) {
                let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
                item.set_text(item_name.clone());
                item.set_associated_layout(list.id().clone());
                item.set_background_color(TheColor::from([187, 222, 251, 255]));
                list.add_item(item, ctx);
            }
        }
    }

    pub fn draw(&mut self, buffer: &mut TheRGBABuffer) {
        buffer.fill(self.grid_ctx.background_color);

        let mut y: u32 = 0;
        for r in self.routines.values_mut() {
            if y < buffer.dim().height as u32 {
                buffer.copy_into(0, y as i32, &r.buffer);
                r.module_offset = y;
                y += r.buffer.dim().height as u32;

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
                r.draw(ctx, &self.grid_ctx);
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
                } else if id.name.starts_with("cgfx") {
                    for r in self.routines.values_mut() {
                        if Some(r.id) == self.grid_ctx.selected_routine {
                            if let Some(coord) = self.grid_ctx.current_cell {
                                if let Some(item) = r.grid.grid.get_mut(&coord) {
                                    item.apply_value(&id.name, value);
                                    r.draw(ctx, &self.grid_ctx);
                                }
                            }
                        }
                    }
                    if let Some(renderview) = ui.get_render_view("ModuleView") {
                        self.draw(renderview.render_buffer_mut());
                    }
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
                if handled {
                    if let Some(renderview) = ui.get_render_view("ModuleView") {
                        self.draw(renderview.render_buffer_mut());

                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Module Changed"),
                            TheValue::Empty,
                        ));

                        redraw = true;
                    }
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
                                r.draw(ctx, &mut self.grid_ctx);
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
                                break;
                            }
                        }
                    }
                    if handled {
                        if let Some(renderview) = ui.get_render_view("ModuleView") {
                            self.draw(renderview.render_buffer_mut());
                            redraw = true;
                        }
                    }
                }
            }
            _ => {}
        }

        redraw
    }

    /// Build the module into Python source
    pub fn build(&self) -> String {
        let mut out = String::new();

        if self.module_type == ModuleType::CharacterTemplate
            || self.module_type == ModuleType::ItemTemplate
        {
            out += &format!("class {}:\n", self.name);
            out += "    def event(self, event, value):\n";

            for r in self.routines.values() {
                r.build(&mut out, 8);
            }
        }
        out
    }
}
