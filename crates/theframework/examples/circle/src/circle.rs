use theframework::prelude::*;

pub struct Circle {}

impl TheTrait for Circle {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {}
    }

    /// Draw a circle in the middle of the window
    fn draw(&mut self, pixels: &mut [u8], ctx: &mut TheContext) {
        ctx.draw.rect(
            pixels,
            &(0, 0, ctx.width, ctx.height),
            ctx.width,
            &[0, 0, 0, 255],
        );
        ctx.draw.circle(
            pixels,
            &(ctx.width / 2 - 100, ctx.height / 2 - 100, 200, 200),
            ctx.width,
            &[255, 255, 255, 255],
            100.0,
        )
    }

    /// Touch down event
    fn touch_down(&mut self, _x: f32, _y: f32, _ctx: &mut TheContext) -> bool {
        false
    }

    /// Touch up event
    fn touch_up(&mut self, _x: f32, _y: f32, _ctx: &mut TheContext) -> bool {
        false
    }

    /// Query if the widget needs a redraw
    fn update(&mut self, _ctx: &mut TheContext) -> bool {
        false
    }
}
