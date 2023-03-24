#![deny(clippy::all)]
#![forbid(unsafe_code)]

mod prelude {
    pub const GAME_TICK_IN_MS : u128 = 250;
}

use core_render::render::GameRender;
use core_server::prelude::*;
use prelude::*;

use std::rc::Rc;

use log::error;
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;
use winit::event::KeyboardInput;

use std::path::PathBuf;

pub use std::time::*;

fn main() {

    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Trace).expect("error initializing logger");

        wasm_bindgen_futures::spawn_local(run());
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();

        pollster::block_on(run());
    }
}

async fn run() {

    let width     : usize = 1024;
    let height    : usize = 608;

    let event_loop = EventLoop::new();
    let window = {
        let size = LogicalSize::new(width as f64, height as f64);
        WindowBuilder::new()
            .with_title("Eldiron Web")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .expect("WindowBuilder error")
    };

    let window = Rc::new(window);

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        use winit::platform::web::WindowExtWebSys;

        // Retrieve current width and height dimensions of browser client window
        let get_window_size = || {
            let client_window = web_sys::window().unwrap();
            LogicalSize::new(
                client_window.inner_width().unwrap().as_f64().unwrap(),
                client_window.inner_height().unwrap().as_f64().unwrap(),
            )
        };

        let window = Rc::clone(&window);

        // Initialize winit window with current dimensions of browser client
        window.set_inner_size(get_window_size());

        let client_window = web_sys::window().unwrap();

        // Attach winit canvas to body element
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");

        // Listen for resize event on browser client. Adjust winit window dimensions
        // on event trigger
        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let size = get_window_size();
            window.set_inner_size(size)
        }) as Box<dyn FnMut(_)>);
        client_window
            .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    let mut input = WinitInputHelper::new();
    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture =
            SurfaceTexture::new(window_size.width, window_size.height, window.as_ref());
        Pixels::new_async(width as u32, height as u32, surface_texture)
            .await
            .expect("Pixels error")
    };

    // Init server
    let game_data = GameData::load_from_path(PathBuf::new());

    let mut server = core_server::server::Server::new();
    server.collect_data(&game_data);
    _ = server.start(None);
    let player_uuid = server.create_player_instance();

    let mut game_rect = (0, 0, 0, 0);

    // Init renderer
    let mut render = GameRender::new(PathBuf::new(), player_uuid);

    let mut anim_counter : usize = 0;
    let mut timer : u128 = 0;
    let mut game_tick_timer : u128 = 0;

    let curr_time = 0;

    let mut messages : Vec<Message> = vec![];

    event_loop.run(move |event, _, control_flow| {
        use winit::event::{ElementState, VirtualKeyCode};

        let mut key_string = "";

        if let Event::RedrawRequested(_) = event {

            for message in &messages {
                match message {
                    Message::PlayerUpdate(_uuid, update) => {
                        render.draw(anim_counter, Some(&update));
                    },
                    _ => {}
                }
            }

            // Draw current screen

            let mut cx : usize = 0;
            let mut cy : usize = 0;

            let frame = pixels.get_frame_mut();

            if render.width < width {
                cx = (width - render.width) / 2;
            }

            if render.height < height {
                cy = (height - render.height) / 2;
            }

            game_rect = (cx, cy, render.width, render.height);

            fn copy_slice(dest: &mut [u8], source: &[u8], rect: &(usize, usize, usize, usize), dest_stride: usize) {
                for y in 0..rect.3 {
                    let d = rect.0 * 4 + (y + rect.1) * dest_stride * 4;
                    let s = y * rect.2 * 4;
                    dest[d..d + rect.2 * 4].copy_from_slice(&source[s..s + rect.2 * 4]);
                }
            }

            copy_slice(frame, &mut render.frame, &game_rect, width);

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
                        let rc = render.key_down(char.to_string(), player_uuid);
                        for cmd in rc.0 {
                            server.execute_packed_player_action(player_uuid, cmd);
                        }
                    }
                },

                WindowEvent::ModifiersChanged(state) => match state {
                    _ => {
                        // if curr_screen.modifier_changed(state.shift(), state.ctrl(), state.alt(), state.logo(), &mut asset) {
                        //     window.request_redraw();
                        // }
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

                    VirtualKeyCode::Up => {
                        key_string = "up";
                    },
                    VirtualKeyCode::Right => {
                        key_string = "right";
                    },
                    VirtualKeyCode::Down => {
                        key_string = "down";
                    },
                    VirtualKeyCode::Left => {
                        key_string = "left";
                    },
                    _ => (),
                },
                _ => (),
            },

            Event::DeviceEvent { event, .. } => match event {
                // DeviceEvent::Text { codepoint } => {
                //     println!("text: ({})", codepoint);
                // }
                // DeviceEvent::MouseWheel { delta } => match delta {
                //     winit::event::MouseScrollDelta::LineDelta(x, y) => {
                //         println!("mouse wheel Line Delta: ({},{})", x, y);
                //     }
                //     winit::event::MouseScrollDelta::PixelDelta(p) => {
                //         println!("mouse wheel Pixel Delta: ({},{})", p.x, p.y);
                //         if curr_screen.mouse_wheel((p.x as isize, p.y as isize), &mut asset) {
                //             window.request_redraw();
                //             mouse_wheel_ongoing = true;
                //         }

                //         if p.x == 0.0 && p.y == 0.0 {
                //             mouse_wheel_ongoing = false;
                //         }
                //     }
                // },
                _ => (),
            },
            _ => (),
        }

        // Perform key action
        if key_string.is_empty() == false {
            let rc = render.key_down(key_string.to_owned(), player_uuid);
            for cmd in rc.0 {
                server.execute_packed_player_action(player_uuid, cmd);
            }
        }

        // Handle input events
        if input.update(&event) {
            // Close events
            if /*input.key_pressed(VirtualKeyCode::Escape) ||*/ input.quit() {
                *control_flow = ControlFlow::Exit;
                _ = server.shutdown();
                return;
            }

            if input.mouse_pressed(0) {
                let coords =  input.mouse().unwrap();
                let pixel_pos: (usize, usize) = pixels.window_pos_to_pixel(coords)
                   .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                if contains_pos_for(pixel_pos, game_rect) {
                    let rc = render.mouse_down((pixel_pos.0 - game_rect.0, pixel_pos.1 - game_rect.1), player_uuid);
                    for cmd in rc.0 {
                        server.execute_packed_player_action(player_uuid, cmd);
                    }
                }
            }

            if input.mouse_released(0) {
                //let coords =  input.mouse().unwrap();
                //let pixel_pos: (usize, usize) = pixels.window_pos_to_pixel(coords)
                //    .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                // if curr_screen.mouse_up((pixel_pos.0, pixel_pos.1), &mut asset) {
                //     window.request_redraw();
                // }
            }

            if input.mouse_held(0) {
                let diff =  input.mouse_diff();
                if diff.0 != 0.0 || diff.1 != 0.0 {
                    //let coords =  input.mouse().unwrap();
                    //let pixel_pos: (usize, usize) = pixels.window_pos_to_pixel(coords)
                    //    .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                    // if curr_screen.mouse_dragged((pixel_pos.0, pixel_pos.1), &mut asset) {
                    //     window.request_redraw();
                    // }
                }
            } else {
                let diff =  input.mouse_diff();
                if diff.0 != 0.0 || diff.1 != 0.0 {
                    //let coords =  input.mouse().unwrap();
                    //let pixel_pos: (usize, usize) = pixels.window_pos_to_pixel(coords)
                    //    .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                    // if curr_screen.mouse_hover((pixel_pos.0, pixel_pos.1), &mut asset) {
                    //     window.request_redraw();
                    // }
                }
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                _ = pixels.resize_surface(size.width, size.height);
            }

            #[cfg(target_arch = "wasm32")]
            {
                curr_time = web_sys::window().unwrap().performance().unwrap().now() as u128;
            }

            // Game tick ?
            if curr_time > game_tick_timer + GAME_TICK_IN_MS {
                messages = server.tick();
                game_tick_timer = curr_time;
                anim_counter = anim_counter.wrapping_add(1);
            } else {

                // If not, lets see if we need to redraw for the target fps
                // 4 is the target fps here, for now hardcoded

                let tick_in_ms =  (1000.0 / 4 as f32) as u128;

                if curr_time > timer + tick_in_ms {
                    window.request_redraw();
                    timer = curr_time;
                } else {
                    let t = (timer + tick_in_ms - curr_time) as u64;
                    if t > 10 {
                        //std::thread::sleep(Duration::from_millis(10));
                    }
                }
            }
        }
    });
}

/// Returns true if the given rect contains the given position
pub fn contains_pos_for(pos: (usize, usize), rect: (usize, usize, usize, usize)) -> bool {
    if pos.0 >= rect.0 && pos.0 < rect.0 + rect.2 && pos.1 >= rect.1 && pos.1 < rect.1 + rect.3 {
        true
    } else {
        false
    }
}

