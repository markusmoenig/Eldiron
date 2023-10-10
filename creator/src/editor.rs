
use theframework::prelude::*;

pub struct Editor {
    circle_id           : u32,
}

pub trait CustomEditor {
    //fn process_cmds(&mut self);
    //fn to_world(&self, pos: Vec2f) -> Vec2f;
}

impl CustomEditor for Editor {

}

impl TheTrait for Editor {
    fn new() -> Self where Self: Sized {
    Self {
            circle_id   : 0,
        }
    }

    fn init(&mut self, ctx: &mut TheContext) {
    }

    /// Draw a circle in the middle of the window
    fn draw(&mut self, pixels: &mut [u8], ctx: &mut TheContext) {
        //ctx.renderer.draw(pixels, ctx.width, ctx.height);
        // ctx.draw.rounded_rect(pixels, &(100, 100, 100, 100), ctx.width, &[128, 128, 128, 255], &(1.0, 1.0, 1.0, 1.0));
        println!("{:?}", ctx.width);
        ctx.draw.rect(pixels, &(100, 100, 100, 100), ctx.width, &[128, 128, 128, 255])

    }

    /// If the touch event is inside the circle, set the circle state to Selected
    fn touch_down(&mut self, x: f32, y: f32, ctx: &mut TheContext) -> bool {
        false
    }

    /// Set the circle state to Selected.
    fn touch_up(&mut self, _x: f32, _y: f32, ctx: &mut TheContext) -> bool {
        false
    }

    /// Query if the renderer needs an update (tramsition animation ongoing etc.)
    fn needs_update(&mut self, ctx: &mut TheContext) -> bool {
        false
    }
}