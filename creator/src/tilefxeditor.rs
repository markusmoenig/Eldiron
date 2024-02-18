use crate::prelude::*;

pub struct TileFXEditor {}

#[allow(clippy::new_without_default)]
impl TileFXEditor {
    pub fn new() -> Self {
        Self {}
    }

    /// Build the tile fx UI
    pub fn build(&self, ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        // Toolbar
        let mut toolbar_canvas = TheCanvas::default();
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.limiter_mut().set_max_height(30);

        let mut time_slider = TheTimeSlider::new(TheId::named("TileFX Timeline"));
        time_slider.set_status_text("The tile color correction timeline.");

        toolbar_hlayout.add_widget(Box::new(time_slider));

        toolbar_hlayout.set_margin(vec4i(10, 1, 5, 3));
        toolbar_hlayout.set_mode(TheHLayoutMode::SizeBased);
        toolbar_canvas.set_layout(toolbar_hlayout);

        canvas.set_top(toolbar_canvas);

        // Left FX List

        let mut list_canvas = TheCanvas::default();
        let mut list_layout = TheListLayout::new(TheId::named("TileFX List"));

        let mut item = TheListItem::new(TheId::named("TileFX Color Correction"));
        item.set_text(str!("Color Correction"));
        list_layout.add_item(item, ctx);

        let mut item = TheListItem::new(TheId::named("TileFX Light Emitter"));
        item.set_text(str!("Light Emitter"));
        list_layout.add_item(item, ctx);

        list_layout.limiter_mut().set_max_width(130);
        list_layout.select_first_item(ctx);
        list_canvas.set_layout(list_layout);

        canvas.set_left(list_canvas);

        canvas
    }

    pub fn handle_event(
        &mut self,
        _event: &TheEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
    ) -> bool {
        let redraw = false;

        // match event {}
        //
        redraw
    }
}
