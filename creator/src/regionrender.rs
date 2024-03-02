use crate::prelude::*;

pub struct RegionRender {}

#[allow(clippy::new_without_default)]
impl RegionRender {
    pub fn new() -> Self {
        Self {}
    }

    /// Build the UI
    pub fn build(&self) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        // Toolbar
        let mut toolbar_canvas = TheCanvas::default();
        let traybar_widget = TheTraybar::new(TheId::empty());
        toolbar_canvas.set_widget(traybar_widget);
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);

        toolbar_hlayout.set_margin(vec4i(10, 1, 5, 1));
        toolbar_hlayout.set_padding(3);

        let mut filter_text = TheText::new(TheId::empty());
        filter_text.set_text("Filter".to_string());
        toolbar_hlayout.add_widget(Box::new(filter_text));

        //

        canvas.set_top(toolbar_canvas);

        canvas
    }

    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
    ) -> bool {
        let mut redraw = false;

        //match event {}

        redraw
    }
}
