#![deny(clippy::all)]
#![forbid(unsafe_code)]

mod editor;
mod widget;

mod prelude {
    pub const GAME_TICK_IN_MS : u128 = 250;

    pub use serde::{Deserialize, Serialize};

    pub use core_server::prelude::*;
    pub use core_shared::prelude::*;
    pub use core_render::prelude::*;

    pub use crate::draw2d::Draw2D;
    pub use crate::widget::*;
    pub use crate::editor::*;
    pub use crate::context::*;
    pub use crate::atom::*;
    pub use crate::tilemapwidget::*;
    pub use crate::tileselector::*;
    pub use crate::characterselector::*;
    pub use crate::lootselector::*;

    pub use crate::editor::dialog::*;
    pub use crate::editor::codeeditorwidget::*;
    pub use crate::editor::toolbar::*;
    pub use crate::editor::controlbar::*;
    pub use crate::editor::node::*;
    pub use crate::editor::tilemapwidget::*;
    pub use crate::editor::regionoptions::*;
    pub use crate::editor::regionwidget::*;
    pub use crate::editor::traits::*;
    pub use crate::editor::nodegraph::*;
    pub use crate::editor::behavioroptions::*;
    pub use crate::editor::behavior_overview_options::*;
    pub use crate::editor::systemsoptions::*;
    pub use crate::editor::systems_overview_options::*;
    pub use crate::editor::itemsoptions::*;
    pub use crate::editor::items_overview_options::*;
    pub use crate::editor::region_overview_options::*;
    pub use crate::editor::gameoptions::*;
    pub use crate::dialog_position::*;
    pub use crate::screeneditor_options::*;
    pub use crate::tilemapoptions::*;
    pub use crate::statusbar::*;
    pub use crate::node_preview::*;
    pub use crate::assets_overview_options::*;

    pub use code_editor::prelude::*;
}

use prelude::*;

use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{Event, DeviceEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;
use winit::event::KeyboardInput;

use std::time::Duration;

fn main() -> Result<(), Error> {

    let width     : usize = 1240;
    let height    : usize = 700;

    env_logger::init();

    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {

        if cfg!(target_os = "macos") {
            let size = LogicalSize::new(width as f64, height as f64);
            WindowBuilder::new()
            .with_title("Eldiron")
            .with_inner_size(size)
            .with_min_inner_size(size)

            .build(&event_loop)
            .unwrap()
        } else {
            let size = PhysicalSize::new(width as f64, height as f64);
            WindowBuilder::new()
            .with_title("Eldiron")
            .with_inner_size(size)
            .with_min_inner_size(size)

            .build(&event_loop)
            .unwrap()
        }
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(width as u32, height as u32, surface_texture)?
    };

    let mut asset = Asset::new();

    let mut editor = Editor::new(&mut asset, width, height);

    let mut anim_counter : usize = 0;
    let mut timer : u128 = 0;
    let mut game_tick_timer : u128 = 0;

    let mut mouse_wheel_ongoing = false;

    event_loop.run(move |event, _, control_flow| {
        use winit::event::{ElementState, VirtualKeyCode};

        if let Event::RedrawRequested(_) = event {
            // let start = get_time();
            editor.draw(pixels.get_frame_mut(), anim_counter, &mut asset);
            // println!("Time: {}", get_time() - start);
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
                        if editor.key_down(Some(*char), None, &mut asset) {
                            window.request_redraw();
                        }
                    }
                },

                WindowEvent::ModifiersChanged(state) => match state {
                    _ => {
                        if editor.modifier_changed(state.shift(), state.ctrl(), state.alt(), state.logo(), &mut asset) {
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
                        if editor.key_down(None, Some(WidgetKey::Delete), &mut asset) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Back => {
                        if editor.key_down(None, Some(WidgetKey::Delete), &mut asset) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Up => {
                        if editor.key_down(None, Some(WidgetKey::Up), &mut asset) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Right => {
                        if editor.key_down(None, Some(WidgetKey::Right), &mut asset) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Down => {
                        if editor.key_down(None, Some(WidgetKey::Down), &mut asset) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Left => {
                        if editor.key_down(None, Some(WidgetKey::Left), &mut asset) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Space => {
                        if editor.key_down(None, Some(WidgetKey::Space), &mut asset) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Tab => {
                        if editor.key_down(None, Some(WidgetKey::Tab), &mut asset) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Return => {
                        if editor.key_down(None, Some(WidgetKey::Return), &mut asset) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Escape => {
                        if editor.key_down(None, Some(WidgetKey::Escape), &mut asset) {
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
                        //println!("mouse wheel Line Delta: ({},{})", x, y);
                        if editor.mouse_wheel(((*x * 100.0) as isize,(*y * 100.0) as isize), &mut asset) {
                            window.request_redraw();
                            mouse_wheel_ongoing = true;
                        }

                        if *x == 0.0 && *y == 0.0 {
                            mouse_wheel_ongoing = false;
                        }
                    }
                    winit::event::MouseScrollDelta::PixelDelta(p) => {
                        //println!("mouse wheel Pixel Delta: ({},{})", p.x, p.y);
                        if editor.mouse_wheel((p.x as isize, p.y as isize), &mut asset) {
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

                if editor.mouse_down((pixel_pos.0, pixel_pos.1), &mut asset) {
                    window.request_redraw();
                }
            }

            if input.mouse_released(0) {
                let coords =  input.mouse().unwrap();
                let pixel_pos: (usize, usize) = pixels.window_pos_to_pixel(coords)
                    .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                if editor.mouse_up((pixel_pos.0, pixel_pos.1), &mut asset) {
                    window.request_redraw();
                }
            }

            if input.mouse_held(0) {
                let diff =  input.mouse_diff();
                if diff.0 != 0.0 || diff.1 != 0.0 {
                    let coords =  input.mouse().unwrap();
                    let pixel_pos: (usize, usize) = pixels.window_pos_to_pixel(coords)
                        .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                    if editor.mouse_dragged((pixel_pos.0, pixel_pos.1), &mut asset) {
                        window.request_redraw();
                    }
                }
            } else {
                let diff =  input.mouse_diff();
                if diff.0 != 0.0 || diff.1 != 0.0 {
                    let coords =  input.mouse().unwrap();
                    let pixel_pos: (usize, usize) = pixels.window_pos_to_pixel(coords)
                        .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                    if editor.mouse_hover((pixel_pos.0, pixel_pos.1), &mut asset) {
                        window.request_redraw();
                    }
                }
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                let _rc = pixels.resize_surface(size.width, size.height);
                let scale = window.scale_factor() as u32;
                let _rc = pixels.resize_buffer(size.width / scale, size.height / scale);
                editor.resize(size.width as usize / scale as usize, size.height as usize / scale as usize);
                //width = size.width as usize / scale as usize;
                //height = size.height as usize / scale as usize;
                window.request_redraw();
            }

            let curr_time = get_time();

            // Game tick ?
            if curr_time > game_tick_timer + GAME_TICK_IN_MS {
                window.request_redraw();
                game_tick_timer = curr_time;
                anim_counter = anim_counter.wrapping_add(1);
            } else {

                // If not, lets see if we need to redraw for the target fps
                let fps = editor.get_target_fps() as f32;//if mouse_wheel_ongoing { 60.0 } else { curr_screen.get_target_fps() as f32 };
                //println!("{}", fps);
                let tick_in_ms =  (1000.0 / fps) as u128;

                if curr_time > timer + tick_in_ms {
                    editor.update();
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

// Get the the current time in ms

fn get_time() -> u128 {
    let stop = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards");
        stop.as_millis()
}