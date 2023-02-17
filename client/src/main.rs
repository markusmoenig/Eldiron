use message_io::node::{self, NodeEvent};
use message_io::network::{NetEvent, Transport};

mod prelude {
    pub const GAME_TICK_IN_MS : u128 = 250;
}

use core_render::render::GameRender;
use core_shared::prelude::*;
use prelude::*;

use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;
use winit::event::KeyboardInput;

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use uuid::Uuid;
use crossbeam_channel::{ unbounded };

/// Gets the current time in milliseconds
fn get_time() -> u128 {
    let stop = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
        stop.as_millis()
}

fn main() -> Result<(), Error> {

    env_logger::init();

    let (sender, receiver) = unbounded();
    let (cmd_sender, cmd_receiver) = unbounded();

    let (handler, listener) = node::split::<()>();
    handler.signals().send_with_timer((), Duration::from_millis(10));

    if let Some((server, _)) = handler.network().connect(Transport::FramedTcp, "127.0.0.1:3042").ok() {
        let _task = listener.for_each_async(move |event| match event {
            NodeEvent::Network(net_event) => match net_event {
                NetEvent::Connected(_endpoint, _ok) => {
                    let cmd = ServerCmd::LoginAnonymous;
                    if let Some(json) = cmd.to_json() {
                        handler.network().send(server, json.as_bytes());
                    }
                },
                NetEvent::Message(_endpoint, data) => {
                    let cmd_string = String::from_utf8_lossy(data);
                    let cmd : ServerCmd = serde_json::from_str(&cmd_string).ok()
                        .unwrap_or(ServerCmd::NoOp);

                    match cmd {
                        ServerCmd::GameUpdate(update) => {
                            //println!("Received: {:?}", update);
                            sender.send(update).unwrap();
                        },
                        _ => {
                        }
                    }
                },
                _ => {

                }
            },
            NodeEvent::Signal(_) => {
                let t : Option<String> = cmd_receiver.try_recv().ok();
                if t.is_some() {
                    let cmd = ServerCmd::GameCmd(t.unwrap());
                    if let Some(to_send) = cmd.to_json() {
                        handler.network().send(server, to_send.as_bytes());
                    }
                }
                handler.signals().send_with_timer((), Duration::from_millis(10));
            }
        });

        let width     : usize = 1024;
        let height    : usize = 608;

        let mut game_rect = (0, 0, 0, 0);

        let player_uuid = Uuid::new_v4();
        // Init renderer
        let mut render = GameRender::new(PathBuf::from(".."), player_uuid);

        /*
        if let Some(screen_size) = settings.get("screen_size") {
            match screen_size.value {
                PropertyValue::IntX(value) => {
                    width = value[0] as usize;
                    height = value[1] as usize;
                },
                _ => {}
            }
        }*/
        //render.process_game_settings(settings);

        // Open the Window
        let event_loop = EventLoop::new();
        let mut input = WinitInputHelper::new();
        let window = {
            let size = LogicalSize::new(width as f64, height as f64);

            WindowBuilder::new()
                .with_title("Eldiron Client")
                .with_inner_size(size)
                .with_min_inner_size(size)

                .build(&event_loop)
                .unwrap()
        };

        let mut pixels = {
            let window_size = window.inner_size();
            let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
            Pixels::new(width as u32, height as u32, surface_texture)?
        };

        let mut anim_counter : usize = 0;
        let mut timer : u128 = 0;
        let mut game_tick_timer : u128 = 0;

        event_loop.run(move |event, _, control_flow| {
            use winit::event::{ElementState, VirtualKeyCode};

            let mut key_string = "";
            let str_c;

            if let Event::RedrawRequested(_) = event {

                let curr_time = get_time();

                // Game tick ?
                if curr_time > game_tick_timer + GAME_TICK_IN_MS {
                    // let start = get_time();

                    // let stop = get_time();
                    // println!("tick time {:?}", stop - start);
                    //window.request_redraw();
                    game_tick_timer = curr_time;
                    anim_counter = anim_counter.wrapping_add(1);
                }

                if let Some(update) = receiver.recv().ok() {
                    render.player_id = update.id;
                    render.draw(anim_counter, Some(&update));
                } else {
                    render.draw(anim_counter, None);
                }

                //if let Some(update) = &received_update {
                //if let Some(update) = received_update.take() {

                //}
                        //render.draw(anim_counter, Some(&received_update.as_ref().unwrap()));

                //} else {
                //    render.draw(anim_counter, None);
                //}

                //received_update = None;

                // Draw the frame

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
                            str_c = char.to_string();
                            key_string = str_c.as_str();
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
                let rc = render.key_down(key_string.to_owned(), render.player_id);
                for cmd in rc.0 {
                    // todo server.execute_packed_player_action(player_uuid, cmd);
                    cmd_sender.send(cmd).unwrap();
                }
            }

            // Handle input events
            if input.update(&event) {
                // Close events
                if /*input.key_pressed(VirtualKeyCode::Escape) ||*/ input.quit() {
                    *control_flow = ControlFlow::Exit;
                    // todo _ = server.shutdown();
                    return;
                }

                if input.mouse_pressed(0) {
                    let coords =  input.mouse().unwrap();
                    let pixel_pos: (usize, usize) = pixels.window_pos_to_pixel(coords)
                    .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                    if contains_pos_for(pixel_pos, game_rect) {
                        let rc = render.mouse_down((pixel_pos.0 - game_rect.0, pixel_pos.1 - game_rect.1), player_uuid);
                        for cmd in rc.0 {
                            // todo server.execute_packed_player_action(player_uuid, cmd);
                            cmd_sender.send(cmd).unwrap();
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
                    let _rc = pixels.resize_surface(size.width, size.height);
                    // let scale = window.scale_factor() as u32;
                    // pixels.resize_buffer(size.width / scale, size.height / scale);
                    //curr_screen.resize(size.width as usize / scale as usize, size.height as usize / scale as usize);
                    //width = size.width as usize / scale as usize;
                    //height = size.height as usize / scale as usize;
                    //render.width =  size.width as usize / scale as usize;
                    //render.height =  size.height as usize / scale as usize;
                    //window.request_redraw();
                }

                let curr_time = get_time();

                // // Game tick ?
                // if curr_time > game_tick_timer + GAME_TICK_IN_MS {
                //     // let start = get_time();
                //     game.tick();
                //     // let stop = get_time();
                //     // println!("tick time {:?}", stop - start);
                //     window.request_redraw();
                //     game_tick_timer = curr_time;
                //     anim_counter = anim_counter.wrapping_add(1);
                // } else
                {

                    // If not, lets see if we need to redraw for the target fps
                    // 4 is the target fps here, for now hardcoded

                    let tick_in_ms =  (1000.0 / 4 as f32) as u128;

                    if curr_time > timer + tick_in_ms {
                        //curr_screen.update();
                        window.request_redraw();
                        timer = curr_time;
                    } else {
                        let t = (timer + tick_in_ms - curr_time) as u64;
                        if t > 10 {
                            std::thread::sleep(Duration::from_millis(10));
                        }
                    }
                }
            }
        });
    }
    handler.stop();
    Ok(())
}

/// Returns true if the given rect contains the given position
pub fn contains_pos_for(pos: (usize, usize), rect: (usize, usize, usize, usize)) -> bool {
    if pos.0 >= rect.0 && pos.0 < rect.0 + rect.2 && pos.1 >= rect.1 && pos.1 < rect.1 + rect.3 {
        true
    } else {
        false
    }
}