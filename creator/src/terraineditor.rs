use shared::prelude::*;

use crate::prelude::*;

pub struct TerrainEditor {}

#[allow(clippy::new_without_default)]
impl TerrainEditor {
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

        let render_view = TheRenderView::new(TheId::named("TerrainView"));
        center.set_widget(render_view);

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

    #[allow(clippy::too_many_arguments)]
    pub fn handle_event(
        &mut self,
        _event: &TheEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
        _client: &mut Client,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        //let redraw = false;
        // match event {
        //     _ => {}
        // }

        //redraw
        false
    }
}
