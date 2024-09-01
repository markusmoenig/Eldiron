use crate::prelude::*;
use shared::prelude::*;

pub struct ModelEditor {}

#[allow(clippy::new_without_default)]
impl ModelEditor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn init_ui(
        &mut self,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
    ) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut model_editor = TheRGBALayout::new(TheId::named("Model Editor"));
        if let Some(rgba_view) = model_editor.rgba_view_mut().as_rgba_view() {
            rgba_view.set_mode(TheRGBAViewMode::TilePicker);
            rgba_view.set_dont_show_grid(true);

            //rgba_view.set_grid_color([255, 255, 255, 5]);
            //rgba_view.set_hover_color(Some([255, 255, 255, 100]));
            rgba_view.set_grid(Some(1));

            let mut buffer = TheRGBABuffer::new(TheDim::sized(400, 400));
            buffer.fill(WHITE);
            rgba_view.set_buffer(buffer);

            // Side Panel
            let mut side_panel = TheCanvas::new();
            let mut vlayout = TheVLayout::new(TheId::named("Editor Icon Layout"));
            vlayout.set_background_color(Some(TheThemeColors::ListLayoutBackground));
            vlayout.limiter_mut().set_max_width(90);
            vlayout.set_margin(vec4i(0, 10, 0, 5));

            // vlayout.add_widget(Box::new(ground_icon));
            // vlayout.add_widget(Box::new(wall_icon));
            // vlayout.add_widget(Box::new(ceiling_icon));
            // //vlayout.add_widget(Box::new(cc_icon));

            // let mut spacer = TheIconView::new(TheId::empty());
            // spacer.limiter_mut().set_max_height(2);
            // vlayout.add_widget(Box::new(spacer));

            let mut text = TheText::new(TheId::named("Cursor Position"));
            text.set_text("()".to_string());
            text.set_text_color([200, 200, 200, 255]);
            vlayout.add_widget(Box::new(text));

            let mut text = TheText::new(TheId::named("Cursor Height"));
            text.set_text("H: -".to_string());
            text.set_text_color([200, 200, 200, 255]);
            vlayout.add_widget(Box::new(text));

            side_panel.set_layout(vlayout);
            center.set_left(side_panel);
        }

        center.set_layout(model_editor);

        // Toolbar
        let mut top_toolbar = TheCanvas::new();
        top_toolbar.set_widget(TheTraybar::new(TheId::empty()));

        let mut toolbar_hlayout = TheHLayout::new(TheId::named("Terrain Tool Params"));
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(10, 4, 5, 4));

        top_toolbar.set_layout(toolbar_hlayout);
        center.set_top(top_toolbar);

        center
    }

    pub fn build_node_ui(&mut self) -> TheCanvas {
        let mut center = TheCanvas::new();

        // Toolbar
        let mut top_toolbar = TheCanvas::new();
        top_toolbar.set_widget(TheTraybar::new(TheId::empty()));

        let mut toolbar_hlayout = TheHLayout::new(TheId::named("Model Bottom Tools"));
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(10, 4, 5, 4));

        top_toolbar.set_layout(toolbar_hlayout);
        center.set_top(top_toolbar);

        let node_view = TheNodeCanvasView::new(TheId::named("MaterialFX NodeCanvas"));
        center.set_widget(node_view);

        center
    }

    pub fn activated(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &ServerContext,
    ) {
        let mut width = 400;
        let mut height = 400;

        if let Some(geo_obj_id) = server_ctx.curr_geo_object {
            if let Some(region) = project.get_region(&server_ctx.curr_region) {
                if let Some(geo_obj) = region.geometry.get(&geo_obj_id) {
                    width = (geo_obj.get_length() * 400.0) as usize;
                    height = (geo_obj.get_height() * 400.0) as usize;
                }
            }
        }

        if let Some(geo_obj_id) = server_ctx.curr_geo_object {
            if let Some(region) = project.get_region(&server_ctx.curr_region) {
                if let Some(ftctx) = region.compiled_geometry.get(&geo_obj_id) {
                    if let Some(editor) = ui.get_rgba_layout("Model Editor") {
                        if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                            let mut buffer =
                                TheRGBABuffer::new(TheDim::sized(width as i32, height as i32));
                            ftctx.render(width, height, buffer.pixels_mut());
                            rgba_view.set_buffer(buffer);
                            ctx.ui.relayout = true;
                            ctx.ui.redraw_all = true;
                        }
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &mut Project,
        _client: &mut Server,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = false;
        match event {
            TheEvent::TileEditorHoverChanged(id, coord) => {
                if id.name == "Model Editor View" {
                    if let Some(geo_obj) = server_ctx.curr_geo_object {
                        if let Some(region) = project.get_region(&server_ctx.curr_region) {
                            if let Some(ftctx) = region.compiled_geometry.get(&geo_obj) {
                                let meta = ftctx.meta_data_at(coord.x, coord.y, 400, 400);
                                println!("{:?}", meta);
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        //redraw
        false
    }
}
