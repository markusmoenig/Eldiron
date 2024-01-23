use crate::prelude::*;
use crate::server::{REGIONS, RNG, TILES, UPDATES, KEY_DOWN};
use theframework::prelude::*;

pub fn add_compiler_functions(compiler: &mut TheCompiler) {
    //
    compiler.add_external_call(
        "KeyDown".to_string(),
        |stack, _data, _sandbox| {
            if let Some(key) = KEY_DOWN.read().unwrap().clone() {
                stack.push(TheValue::Text(key));
            } else {
                stack.push(TheValue::Empty);
            }
            TheCodeNodeCallResult::Continue
        },
        vec![],
    );
    // RandWalk
    compiler.add_external_call(
        "RandWalk".to_string(),
        |stack, data, sandbox| {
            let region_id = sandbox.id;
            if let Some(region) = REGIONS.read().unwrap().get(&region_id) {
                if let Some(object) = sandbox.get_self_mut() {
                    if let Some(TheValue::Position(p)) = object.get_mut(&"position".into()) {
                        let mut x = p.x;
                        let mut z = p.z;

                        let dir = RNG.lock().unwrap().gen_range(0..4);

                        if dir == 0 {
                            x += 1.0;
                        } else if dir == 1 {
                            x -= 1.0;
                        } else if dir == 2 {
                            z += 1.0;
                        } else if dir == 3 {
                            z -= 1.0;
                        }

                        if region.can_move_to(vec3f(x, p.y, z), &TILES.read().unwrap()) {
                            let old_position = *p;

                            *p = vec3f(x, p.y, z);

                            if let Some(update) = UPDATES.write().unwrap().get_mut(&region_id) {
                                if let Some(cu) = update.characters.get_mut(&object.id) {
                                    cu.position = vec2f(x, z);
                                    cu.moving = Some((old_position.xz(), cu.position));
                                    cu.move_delta = 0.0;
                                }
                            }

                            if sandbox.debug_mode {
                                sandbox
                                    .set_debug_value(data.location, (None, TheValue::Bool(true)));
                            }
                            stack.push(TheValue::Bool(true));
                        } else if sandbox.debug_mode {
                            sandbox.set_debug_value(data.location, (None, TheValue::Bool(false)));
                            stack.push(TheValue::Bool(false));
                        }
                    }
                }
            }
            TheCodeNodeCallResult::Continue
        },
        vec![],
    );

    // Move
    compiler.add_external_call(
        "Move".to_string(),
        |stack, data, sandbox| {
            let region_id = sandbox.id;

            let mut by = vec2f(0.0, 0.0);
            if let Some(v) = stack.pop() {
                if let Some(f2) = v.to_vec2f() {
                    by = f2;
                }
            }

            if let Some(region) = REGIONS.read().unwrap().get(&region_id) {
                if let Some(object) = sandbox.get_self_mut() {
                    if let Some(TheValue::Position(p)) = object.get_mut(&"position".into()) {
                        let x = p.x + by.x;
                        let z = p.z + by.y;

                        if region.can_move_to(vec3f(x, p.y, z), &TILES.read().unwrap()) {
                            let old_position = *p;

                            *p = vec3f(x, p.y, z);

                            if let Some(update) = UPDATES.write().unwrap().get_mut(&region_id) {
                                if let Some(cu) = update.characters.get_mut(&object.id) {
                                    cu.position = vec2f(x, z);
                                    cu.moving = Some((old_position.xz(), cu.position));
                                    cu.move_delta = 0.0;
                                }
                            }

                            if sandbox.debug_mode {
                                sandbox
                                    .set_debug_value(data.location, (None, TheValue::Bool(true)));
                            }
                            stack.push(TheValue::Bool(true));
                        } else if sandbox.debug_mode {
                            sandbox.set_debug_value(data.location, (None, TheValue::Bool(false)));
                            stack.push(TheValue::Bool(false));
                        }
                    }
                }
            }
            TheCodeNodeCallResult::Continue
        },
        vec![TheValue::Float2(vec2f(0.0, 0.0))],
    );

    // Pulse
    compiler.add_external_call(
        "Pulse".to_string(),
        |stack: &mut Vec<TheValue>, data: &mut TheCodeNodeData, sandbox: &mut TheCodeSandbox| {
            let count = data.values[0].to_i32().unwrap();
            let mut max_value = data.values[1].to_i32().unwrap();

            let stack_v = stack.pop();

            // If the max value is zero, this is the first call, compute it.
            if let Some(v) = &stack_v {
                if max_value == 0 {
                    if let Some(int) = v.to_i32() {
                        max_value = int;
                    }
                }
            }

            if count < max_value {
                data.values[0] = TheValue::Int(count + 1);
                if sandbox.debug_mode {
                    sandbox.set_debug_value(
                        data.location,
                        (
                            Some(TheValue::Text(format!("{} / {}", count, max_value))),
                            TheValue::Bool(false),
                        ),
                    );
                }
                stack.push(TheValue::Bool(false));
                TheCodeNodeCallResult::Continue
            } else {
                if sandbox.debug_mode {
                    sandbox.set_debug_value(
                        data.location,
                        (
                            Some(TheValue::Text(format!("{} / {}", count, max_value))),
                            TheValue::Bool(true),
                        ),
                    );
                }
                data.values[0] = TheValue::Int(0);
                if let Some(stack_v) = stack_v {
                    if let Some(int) = stack_v.to_i32() {
                        data.values[1] = TheValue::Int(int);
                    }
                }
                stack.push(TheValue::Bool(true));
                TheCodeNodeCallResult::Continue
            }
        },
        vec![TheValue::Int(0), TheValue::Int(0)],
    );

    // Debug
    compiler.add_external_call(
        "Debug".to_string(),
        |stack: &mut Vec<TheValue>, data: &mut TheCodeNodeData, sandbox: &mut TheCodeSandbox| {

            if let Some(v) = stack.pop() {
                sandbox.set_debug_value(
                    data.location,
                    (
                        None,
                        v,
                    ),
                );
            }

            TheCodeNodeCallResult::Continue
        },
        vec![TheValue::Int(0), TheValue::Int(0)],
    );
}
