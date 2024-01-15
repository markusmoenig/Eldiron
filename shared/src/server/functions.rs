use crate::prelude::*;
use crate::server::{REGIONS, RNG, TILES, UPDATES};
use theframework::prelude::*;

pub fn add_compiler_functions(compiler: &mut TheCompiler) {
    // RandWalk
    compiler.add_external_call(
        "RandWalk".to_string(),
        |stack, data, sandbox| {
            let region_id = sandbox.id;
            if let Some(region) = REGIONS.read().unwrap().get(&region_id) {
                if let Some(object) = sandbox.get_self_mut() {
                    if let Some(TheValue::Position(p)) = object.get_mut(&"position".into()) {
                        let mut x = p.x;
                        let mut y = p.y;

                        let dir = RNG.lock().unwrap().gen_range(0..=4);

                        if dir == 0 {
                            x += 1.0;
                        } else if dir == 1 {
                            x -= 1.0;
                        } else if dir == 2 {
                            y += 1.0;
                        } else if dir == 3 {
                            y -= 1.0;
                        }

                        if region.can_move_to(vec3f(x, y, p.z), &TILES.read().unwrap()) {
                            let old_position = *p;

                            *p = vec3f(x, y, p.z);

                            if let Some(update) = UPDATES.write().unwrap().get_mut(&region_id) {
                                if let Some(cu) = update.characters.get_mut(&object.id) {
                                    cu.position = vec2f(x, y);
                                    cu.moving = Some((old_position.xy(), cu.position));
                                    cu.move_delta = 0.0;
                                }
                            }

                            if sandbox.debug_mode {
                                sandbox
                                    .set_debug_value(data.location, (None, TheValue::Bool(true)));
                            }
                            if !data.sub_functions.is_empty() {
                                _ = data.sub_functions[0].execute(sandbox).pop();
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
                if !data.sub_functions.is_empty() {
                    _ = data.sub_functions[0].execute(sandbox).pop();
                }
                stack.push(TheValue::Bool(true));
                TheCodeNodeCallResult::Continue
            }
        },
        vec![TheValue::Int(0), TheValue::Int(0)],
    );
}
