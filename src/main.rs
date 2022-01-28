#![deny(clippy::all)]
#![forbid(unsafe_code)]

mod game;
mod editor;
mod widget;
mod asset;

mod prelude {
    pub const TICK_IN_MS            : u128 = 200;

    pub const WIDTH                 : u32 = 60 * 16;
    pub const HEIGHT                : u32 = 40 * 16;

    pub const UI_ELEMENT_HEIGHT     : u32 = 24;
    pub const UI_ELEMENT_MARGIN     : u32 = 4;
}

use prelude::*;

use crate::game::*;
use crate::widget::*;
use crate::editor::*;

use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use std::time::Duration;

fn main() -> Result<(), Error> {
    env_logger::init();

    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        
        WindowBuilder::new()
            .with_title("Eldiron")
            .with_inner_size(size)
            .with_min_inner_size(size)

            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };

    let game : Box<dyn ScreenWidget> = Box::new(Game::new());
    let editor : Box<dyn ScreenWidget> = Box::new(Editor::new());

    let mut curr_screen = editor;

    let mut timer : u128 = 0;

    event_loop.run(move |event, _, control_flow| {

        if let Event::RedrawRequested(_) = event {
            curr_screen.draw(pixels.get_frame());
            if pixels
                .render()
                .map_err(|e| error!("pixels.render() failed: {}", e))
                .is_err()
            {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            if input.mouse_pressed(0) {                
                let coords =  input.mouse().unwrap();
                let pixel_pos: (usize, usize) = pixels.window_pos_to_pixel(coords)
                    .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                if curr_screen.mouse_down((pixel_pos.0 as u32, pixel_pos.1 as u32)) {
                    window.request_redraw();
                }
            }

            if input.mouse_released(0) {                
                let coords =  input.mouse().unwrap();
                let pixel_pos: (usize, usize) = pixels.window_pos_to_pixel(coords)
                    .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                curr_screen.mouse_up((pixel_pos.0 as u32, pixel_pos.1 as u32));
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                pixels.resize_surface(size.width, size.height);
                window.request_redraw();
            }

            let curr_time = game.get_time();

            if curr_time > timer + TICK_IN_MS {
                curr_screen.update();
                window.request_redraw();
                timer = curr_time;
            } else {
                let t = (timer + TICK_IN_MS - curr_time) as u64;
                std::thread::sleep(Duration::from_millis(t / 2)); 
            }
        }
    });
}
