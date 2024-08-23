use shared::prelude::*;

use crate::prelude::*;

pub struct MaterialEditor {}

#[allow(clippy::new_without_default)]
impl MaterialEditor {
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

        let render_view = TheRenderView::new(TheId::named("MaterialView"));
        center.set_widget(render_view);

        // Toolbar
        let mut top_toolbar = TheCanvas::new();
        top_toolbar.set_widget(TheTraybar::new(TheId::empty()));

        let mut toolbar_hlayout = TheHLayout::new(TheId::named("Material Tool Params"));
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(10, 4, 5, 4));

        top_toolbar.set_layout(toolbar_hlayout);
        center.set_top(top_toolbar);

        let mut material_node_canvas = TheCanvas::new();
        let node_view = TheNodeCanvasView::new(TheId::named("MaterialFX NodeCanvas"));
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
        server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = false;
        #[allow(clippy::single_match)]
        match event {
            TheEvent::StateChanged(id, TheWidgetState::Selected) => {
                if id.name == "Material Item" {
                    let material_id = id.uuid;
                    server_ctx.curr_material_object = Some(material_id);
                    if let Some(material) = project.materials.get_mut(&material_id) {
                        let node_canvas = material.to_canvas(&project.palette);
                        ui.set_node_canvas("MaterialFX NodeCanvas", node_canvas);
                    }
                }
                //println!("id.name {}", id.name);
            }
            _ => {}
        }

        redraw
    }
}
