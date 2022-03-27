#![deny(clippy::all)]
#![forbid(unsafe_code)]

mod game;
mod editor;
mod widget;

mod prelude {
    pub const GAME_TICK_IN_MS : u128 = 250;
}

use prelude::*;

use crate::game::*;
use crate::widget::*;
use crate::editor::*;

use server::asset::*;

use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, DeviceEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;
use winit::event::KeyboardInput;

use std::time::Duration;

fn main() -> Result<(), Error> {

    const WIDTH     : usize = 1240;//60 * 16;
    const HEIGHT    : usize = 700;//40 * 16;

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
        Pixels::new(WIDTH as u32, HEIGHT as u32, surface_texture)?
    };

    let mut asset = Asset::new();

    let game : Box<dyn ScreenWidget> = Box::new(Game::new(&asset, WIDTH, HEIGHT));
    let editor : Box<dyn ScreenWidget> = Box::new(Editor::new(&asset, WIDTH, HEIGHT));

    let mut curr_screen = editor;

    let mut anim_counter : usize = 0;
    let mut timer : u128 = 0;
    let mut game_tick_timer : u128 = 0;

    let mut mouse_wheel_ongoing = false;

    event_loop.run(move |event, _, control_flow| {
        use winit::event::{ElementState, VirtualKeyCode};

        if let Event::RedrawRequested(_) = event {
            curr_screen.draw(pixels.get_frame(), anim_counter, &mut asset);
            if pixels
                .render()
                .map_err(|e| error!("pixels.render() failed: {}", e))
                .is_err()
            {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        match &event {

            Event::WindowEvent { event, .. } => match event {
                WindowEvent::ReceivedCharacter(char ) => match char {
                    _ => {
                        if curr_screen.key_down(Some(*char), None, &mut asset) {
                            window.request_redraw();
                        }
                    }
                },

                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(virtual_code),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => match virtual_code {
                    VirtualKeyCode::Delete => {
                        if curr_screen.key_down(None, Some(WidgetKey::Delete), &mut asset) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Back => {
                        if curr_screen.key_down(None, Some(WidgetKey::Delete), &mut asset) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Up => {
                        if curr_screen.key_down(None, Some(WidgetKey::Up), &mut asset) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Right => {
                        if curr_screen.key_down(None, Some(WidgetKey::Right), &mut asset) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Down => {
                        if curr_screen.key_down(None, Some(WidgetKey::Down), &mut asset) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Left => {
                        if curr_screen.key_down(None, Some(WidgetKey::Left), &mut asset) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Space => {
                        if curr_screen.key_down(None, Some(WidgetKey::Space), &mut asset) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Return => {
                        if curr_screen.key_down(None, Some(WidgetKey::Return), &mut asset) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Escape => {
                        if curr_screen.key_down(None, Some(WidgetKey::Escape), &mut asset) {
                            window.request_redraw();
                        }
                    }
                    _ => (),
                },
                _ => (),
            },

            Event::DeviceEvent { event, .. } => match event {
                // DeviceEvent::Text { codepoint } => {
                //     println!("text: ({})", codepoint);
                // }
                DeviceEvent::MouseWheel { delta } => match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        println!("mouse wheel Line Delta: ({},{})", x, y);
                    }
                    winit::event::MouseScrollDelta::PixelDelta(p) => {
                        //println!("mouse wheel Pixel Delta: ({},{})", p.x, p.y);
                        if curr_screen.mouse_wheel((p.x as isize, p.y as isize), &mut asset) {
                            window.request_redraw();
                            mouse_wheel_ongoing = true;
                        }

                        if p.x == 0.0 && p.y == 0.0 {
                            mouse_wheel_ongoing = false;
                        }
                    }
                },
                _ => (),
            },
            _ => (),
        }

        /*
        let text = input.text();

        if text.is_empty() == false {
            for t in text {
                println!("{:?}", t);
            }
        }*/

        // Handle input events
        if input.update(&event) {
            // Close events
            if /*input.key_pressed(VirtualKeyCode::Escape) ||*/ input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            if input.mouse_pressed(0) {
                let coords =  input.mouse().unwrap();
                let pixel_pos: (usize, usize) = pixels.window_pos_to_pixel(coords)
                    .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                if curr_screen.mouse_down((pixel_pos.0, pixel_pos.1), &mut asset) {
                    window.request_redraw();
                }
            }

            if input.mouse_released(0) {
                let coords =  input.mouse().unwrap();
                let pixel_pos: (usize, usize) = pixels.window_pos_to_pixel(coords)
                    .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                if curr_screen.mouse_up((pixel_pos.0, pixel_pos.1), &mut asset) {
                    window.request_redraw();
                }
            }

            if input.mouse_held(0) {
                let diff =  input.mouse_diff();
                if diff.0 != 0.0 || diff.1 != 0.0 {
                    let coords =  input.mouse().unwrap();
                    let pixel_pos: (usize, usize) = pixels.window_pos_to_pixel(coords)
                        .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                    if curr_screen.mouse_dragged((pixel_pos.0, pixel_pos.1), &mut asset) {
                        window.request_redraw();
                    }
                }
            } else {
                let diff =  input.mouse_diff();
                if diff.0 != 0.0 || diff.1 != 0.0 {
                    let coords =  input.mouse().unwrap();
                    let pixel_pos: (usize, usize) = pixels.window_pos_to_pixel(coords)
                        .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                    if curr_screen.mouse_hover((pixel_pos.0, pixel_pos.1), &mut asset) {
                        window.request_redraw();
                    }
                }
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                pixels.resize_surface(size.width, size.height);
                let scale = window.scale_factor() as u32;
                pixels.resize_buffer(size.width / scale, size.height / scale);
                curr_screen.resize(size.width as usize / scale as usize, size.height as usize / scale as usize);
                window.request_redraw();
            }

            let curr_time = game.get_time();

            // Game tick ?
            if curr_time > game_tick_timer + GAME_TICK_IN_MS {
                curr_screen.update();
                window.request_redraw();
                game_tick_timer = curr_time;
                anim_counter = anim_counter.wrapping_add(1);
            } else {

                // If not, lets see if we need to redraw for the target fps
                let tick_in_ms =  (1000.0 / curr_screen.get_target_fps() as f32) as u128;

                if curr_time > timer + tick_in_ms {
                    curr_screen.update();
                    window.request_redraw();
                    timer = curr_time;
                } else
                if mouse_wheel_ongoing == false {
                    let t = (timer + tick_in_ms - curr_time) as u64;
                    if t > 10 {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                }
            }
        }
    });
}
