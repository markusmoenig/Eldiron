use crate::prelude::*;
use shared::prelude::*;

pub struct MaterialEditor {
    material_start_index: i32,
}

#[allow(clippy::new_without_default)]
impl MaterialEditor {
    pub fn new() -> Self {
        Self {
            material_start_index: 0,
        }
    }

    pub fn build(&mut self) -> TheCanvas {
        let mut center = TheCanvas::new();

        let render_view = TheRenderView::new(TheId::named("MaterialView"));
        center.set_widget(render_view);

        // Toolbar
        let mut top_toolbar = TheCanvas::new();
        top_toolbar.set_widget(TheTraybar::new(TheId::empty()));

        let mut toolbar_hlayout = TheHLayout::new(TheId::named("Material Tool Params"));
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(10, 4, 5, 4));

        for i in 0..20 {
            let mut icon = TheIconView::new(TheId::named(&format!("Material Icon #{}", i)));
            // ground_icon.set_text(Some("FLOOR".to_string()));
            // ground_icon.set_text_size(10.0);
            // ground_icon.set_text_color([200, 200, 200, 255]);
            icon.limiter_mut().set_max_size(vec2i(20, 20));

            if i == 0 {
                icon.set_border_color(Some(WHITE));
            } else {
                icon.set_border_color(Some(BLACK));
            }

            toolbar_hlayout.add_widget(Box::new(icon));
        }

        top_toolbar.set_layout(toolbar_hlayout);
        center.set_top(top_toolbar);

        let mut material_node_canvas = TheCanvas::new();
        let node_view = TheNodeCanvasView::new(TheId::named("Map NodeCanvas"));
        material_node_canvas.set_widget(node_view);

        center.set_center(material_node_canvas);

        center
    }

    #[allow(clippy::too_many_arguments)]
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &mut Project,
        _client: &mut Client,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = false;
        #[allow(clippy::single_match)]
        match event {
            TheEvent::Custom(id, _) => {
                if id.name == "Update Material Previews" {
                    for i in 0..20 {
                        if let Some(icon_view) = ui.get_icon_view(&format!("Material Icon #{}", i))
                        {
                            let index = self.material_start_index + i;
                            if let Some((_, material)) = project.materials.get_index(index as usize)
                            {
                                icon_view.set_rgba_tile(TheRGBATile::buffer(
                                    material.get_preview().scaled(20, 20),
                                ));
                            }
                        }
                    }
                }
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) => {
                if id.name.starts_with("Material Icon #") {
                    let index_str = id.name.replace("Material Icon #", "");
                    if let Ok(index) = index_str.parse::<i32>() {
                        let index = index + self.material_start_index;
                        if let Some((_, material)) = project.materials.get_index_mut(index as usize)
                        {
                            let node_canvas = material.to_canvas(&project.palette);
                            ui.set_node_canvas("Map NodeCanvas", node_canvas);
                        }
                    }
                }
            }
            // TheEvent::StateChanged(id, TheWidgetState::Selected) => {
            //     if id.name == "Material Item" {
            //         let material_id = id.uuid;
            //         server_ctx.curr_material_object = Some(material_id);
            //         if let Some(material) = project.materials.get_mut(&material_id) {
            //             let node_canvas = material.to_canvas(&project.palette);
            //             ui.set_node_canvas("MaterialFX NodeCanvas", node_canvas);
            //         }
            //     }
            // }
            _ => {}
        }

        redraw
    }
}
