#![deny(clippy::all)]
#![forbid(unsafe_code)]
#![windows_subsystem = "windows"]

use creator_lib::prelude::*;

use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{Event, DeviceEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;
use winit::event::KeyboardInput;

use std::time::Duration;
use std::ffi::CString;

use directories::{ UserDirs };

fn main() -> Result<(), Error> {

    let mut width     : usize = 1300;
    let mut height    : usize = 700;

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

    let mut anim_counter : usize = 0;
    let mut timer : u128 = 0;
    let mut game_tick_timer : u128 = 0;

    let mut mouse_wheel_ongoing = false;

    // Init the editor

    let resource_path = CString::new("").unwrap();
    let mut project_path = CString::new("").unwrap();

    if let Some(user_dirs) = UserDirs::new() {
        if let Some(dir) = user_dirs.document_dir() {

            let eldiron_path = dir.join("Eldiron");
            project_path = CString::new(eldiron_path.to_str().unwrap()).unwrap();

            // Check or create "Eldiron" directory
            if fs::metadata(eldiron_path.clone()).is_ok() == false {
                // have to create dir
                let rc = fs::create_dir(eldiron_path.clone());

                if rc.is_err() {
                    return Ok(());
                }
            }
        }
    }

    creator_lib::rust_init(resource_path.as_ptr() as *const i8, project_path.as_ptr() as *const i8);

    // Draw first frame
    let frame = pixels.frame_mut();
    creator_lib::rust_draw(frame.as_mut_ptr(), width as u32, height as u32, anim_counter);
    if pixels
        .render()
        .map_err(|e| error!("pixels.render() failed: {}", e))
        .is_err()
    {
        return Ok(());
    }

    // Loop
    event_loop.run(move |event, _, control_flow| {
        use winit::event::{ElementState, VirtualKeyCode};

        if let Event::RedrawRequested(_) = event {
            // let start = get_time();
            let frame = pixels.frame_mut();
            creator_lib::rust_draw(frame.as_mut_ptr(), width as u32, height as u32, anim_counter);
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

                WindowEvent::DroppedFile(path ) => match path {
                    _ => {
                        let path = CString::new(path.to_str().unwrap()).unwrap();
                        creator_lib::rust_dropped_file(path.as_ptr() as *const i8);
                        window.request_redraw();
                    }
                },

                WindowEvent::ReceivedCharacter(char ) => match char {
                    _ => {
                        let key = CString::new(char.to_string()).unwrap();
                        if creator_lib::rust_key_down(key.as_ptr() as *const i8) {
                            window.request_redraw();
                        }
                    }
                },

                WindowEvent::ModifiersChanged(state) => match state {
                    _ => {
                        if creator_lib::rust_key_modifier_changed(state.shift(), state.ctrl(), state.alt(), state.logo()) {
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
                        if creator_lib::rust_special_key_down(KEY_DELETE) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Back => {
                        if creator_lib::rust_special_key_down(KEY_DELETE) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Up => {
                        if creator_lib::rust_special_key_down(KEY_UP) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Right => {
                        if creator_lib::rust_special_key_down(KEY_RIGHT) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Down => {
                        if creator_lib::rust_special_key_down(KEY_DOWN) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Left => {
                        if creator_lib::rust_special_key_down(KEY_LEFT) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Space => {
                        if creator_lib::rust_special_key_down(KEY_SPACE) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Tab => {
                        if creator_lib::rust_special_key_down(KEY_TAB) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Return => {
                        if creator_lib::rust_special_key_down(KEY_RETURN) {
                            window.request_redraw();
                        }
                    },
                    VirtualKeyCode::Escape => {
                        if creator_lib::rust_special_key_down(KEY_ESCAPE) {
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
                        if creator_lib::rust_touch_wheel(*x * 100.0, *y * 100.0) {
                            window.request_redraw();
                            mouse_wheel_ongoing = true;
                        }

                        if *x == 0.0 && *y == 0.0 {
                            mouse_wheel_ongoing = false;
                        }
                    }
                    winit::event::MouseScrollDelta::PixelDelta(p) => {
                        //println!("mouse wheel Pixel Delta: ({},{})", p.x, p.y);
                        if creator_lib::rust_touch_wheel(p.x as f32, p.y as f32) {
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
            if /*input.key_pressed(VirtualKeyCode::Escape) ||*/ input.close_requested() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            if input.mouse_pressed(0) {
                let coords =  input.mouse().unwrap();
                let pixel_pos: (usize, usize) = pixels.window_pos_to_pixel(coords)
                    .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                if creator_lib::rust_touch_down(pixel_pos.0 as f32, pixel_pos.1 as f32) {
                    window.request_redraw();
                }
            }

            if input.mouse_released(0) {
                let coords =  input.mouse().unwrap();
                let pixel_pos: (usize, usize) = pixels.window_pos_to_pixel(coords)
                    .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                if creator_lib::rust_touch_up(pixel_pos.0 as f32, pixel_pos.1 as f32) {
                    window.request_redraw();
                }
            }

            if input.mouse_held(0) {
                let diff =  input.mouse_diff();
                if diff.0 != 0.0 || diff.1 != 0.0 {
                    let coords =  input.mouse().unwrap();
                    let pixel_pos: (usize, usize) = pixels.window_pos_to_pixel(coords)
                        .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                    if creator_lib::rust_touch_dragged(pixel_pos.0 as f32, pixel_pos.1 as f32) {
                        window.request_redraw();
                    }
                }
            } else {
                let diff =  input.mouse_diff();
                if diff.0 != 0.0 || diff.1 != 0.0 {
                    let coords =  input.mouse().unwrap();
                    let pixel_pos: (usize, usize) = pixels.window_pos_to_pixel(coords)
                        .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                    if creator_lib::rust_hover(pixel_pos.0 as f32, pixel_pos.1 as f32) {
                        window.request_redraw();
                    }
                }
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                let _rc = pixels.resize_surface(size.width, size.height);
                let scale = window.scale_factor() as u32;
                let _rc = pixels.resize_buffer(size.width / scale, size.height / scale);
                // editor.resize(size.width as usize / scale as usize, size.height as usize / scale as usize);
                width = size.width as usize / scale as usize;
                height = size.height as usize / scale as usize;
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
                let fps = creator_lib::rust_target_fps() as f32;//if mouse_wheel_ongoing { 60.0 } else { curr_screen.get_target_fps() as f32 };
                //println!("{}", fps);
                let tick_in_ms =  (1000.0 / fps) as u128;

                if curr_time > timer + tick_in_ms {
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