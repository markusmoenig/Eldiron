use crate::{GridCtx, Group, Routine};
use indexmap::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Module {
    pub name: String,
    pub routines: IndexMap<String, Routine>,
    grid_ctx: GridCtx,
}

impl Module {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            grid_ctx: GridCtx::new(),
            ..Default::default()
        }
    }

    /// Add a routine.
    pub fn add_routine(&mut self, routine: Routine) {
        self.grid_ctx.selected_routine = Some(routine.id);
        self.grid_ctx.selected_cell = None;
        self.routines.insert(routine.name.clone(), routine);
    }

    /// Read out the colors out of the style.
    pub fn get_colors(&mut self, ui: &mut TheUI) {
        self.grid_ctx.background_color = ui.style.theme().color(DefaultWidgetBackground).clone();
        self.grid_ctx.dark_background_color =
            ui.style.theme().color(DefaultWidgetDarkBackground).clone();
        self.grid_ctx.selection_color = ui.style.theme().color(DefaultSelection).clone();
        self.grid_ctx.text_color = ui.style.theme().color(TextEditTextColor).clone();
    }

    pub fn build_canvas(&self) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let render_view = TheRenderView::new(TheId::named("ModuleView"));

        let mut context_menu = TheContextMenu::named(str!("Context"));
        context_menu.add(TheContextMenuItem::new(
            str!("Assignment"),
            TheId::named("CGFAssignment"),
        ));
        // context_menu.add(TheContextMenuItem::new(str!("Set"), TheId::named("Setter")));
        // file_menu.add_separator();

        //render_view.set_context_menu(Some(context_menu));
        canvas.set_widget(render_view);

        canvas
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
            TheEvent::ContextMenuSelected(id, item) => {
                if id.name == "ModuleView" {
                    if let Some(group) = Group::from_str(&item.name) {
                        if let Some(cell) = self.grid_ctx.selected_cell.clone() {
                            println!("1");
                            for r in self.routines.values_mut() {
                                if Some(r.id) == self.grid_ctx.selected_routine {
                                    r.add_group_at(group, cell);
                                    r.draw(ctx, &self.grid_ctx);
                                    break;
                                }
                            }
                        }
                    }
                    if let Some(renderview) = ui.get_render_view("ModuleView") {
                        self.draw(renderview.render_buffer_mut());
                        redraw = true;
                    }
                }
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
}
