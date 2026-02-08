use crate::prelude::*;
use crate::server::{CHARACTERS, INTERACTIONS, ITEMS, KEY_DOWN, REGIONS, TILES, UPDATES};
use theframework::prelude::*;

use super::WallFX;

pub fn add_compiler_functions(compiler: &mut TheCompiler) {
    // KeyDown
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

    // Region
    compiler.add_external_call(
        "Region".to_string(),
        |stack, _data, sandbox| {
            let region_id = sandbox.id;

            let mut property: i32 = 0;
            if let Some(v) = stack.pop() {
                if let Some(f) = v.to_i32() {
                    property = f;
                }
            }

            if let Some(region) = REGIONS.read().unwrap().get(&region_id) {
                if property == 0 {
                    stack.push(TheValue::Text(region.property_1.clone()));
                } else if property == 1 {
                    stack.push(TheValue::Text(region.property_2.clone()));
                } else if property == 2 {
                    stack.push(TheValue::Text(region.property_3.to_string()));
                } else if property == 3 {
                    stack.push(TheValue::Text(region.property_4.to_string()));
                } else {
                    stack.push(TheValue::Empty);
                }
            }
            TheCodeNodeCallResult::Continue
        },
        vec![TheValue::Float2(Vec2::new(0.0, 0.0))],
    );
    /*
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
    );*/

    // Move
    compiler.add_external_call(
        "Move".to_string(),
        |stack, data, sandbox| {
            let region_id = sandbox.id;

            let mut by: Vec2<f32> = Vec2::new(0.0, 0.0);
            if let Some(v) = stack.pop() {
                if let Some(f2) = v.to_vec2f() {
                    by = f2;
                }
            }

            if let Some(region) = REGIONS.read().unwrap().get(&region_id) {
                let mut self_instance_id = Uuid::nil();
                // let mut self_package_id = Uuid::nil();

                let mut target_instance_id = None;

                if let Some(object) = sandbox.get_self_mut() {
                    self_instance_id = object.id;
                    // self_package_id = object.package_id;

                    // Set the facing direction to the direction we are moving to
                    if let Some(TheValue::Direction(dir)) = object.get_mut(&"facing".into()) {
                        *dir = Vec3::new(by.x, 0.0, by.y);
                    }

                    if let Some(TheValue::Position(p)) = object.get_mut(&"position".into()) {
                        let x = p.x + by.x;
                        let z = p.z + by.y;

                        if let Some(update) = UPDATES.write().unwrap().get_mut(&region_id) {
                            if region.can_move_to(
                                Vec2::new(x as i32, z as i32),
                                &TILES.read().unwrap(),
                                update,
                            ) {
                                let mut can_move = true;
                                for c in update.characters.values() {
                                    if c.position.x == x && c.position.y == z {
                                        can_move = false;
                                        target_instance_id = Some(c.id);
                                    }
                                }

                                if can_move {
                                    let old_position = *p;
                                    *p = Vec3::new(x, p.y, z);

                                    if let Some(cu) = update.characters.get_mut(&object.id) {
                                        cu.position = Vec2::new(x, z);
                                        cu.moving = Some((
                                            Vec2::new(old_position.x, old_position.z),
                                            cu.position,
                                        ));

                                        cu.facing = by;
                                        cu.move_delta = 0.0;
                                    }
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

                // We bumped into another character. Get the package id of the other character
                // and call the onContact function of the other / target character.
                if let Some(target_instance_id) = target_instance_id {
                    let mut target_package_id = Uuid::nil();
                    if let Some(target_object) = sandbox.objects.get(&target_instance_id) {
                        target_package_id = target_object.package_id;
                    }

                    //
                    if let Some(target_character) =
                        CHARACTERS.write().unwrap().get_mut(&target_package_id)
                    {
                        if let Some(on_contact) =
                            target_character.get_function_mut(&"onContact".into())
                        {
                            let prev_aliases = sandbox.aliases.clone();
                            sandbox
                                .aliases
                                .insert("self".to_string(), target_instance_id);
                            sandbox
                                .aliases
                                .insert("target".to_string(), self_instance_id);
                            on_contact.execute(sandbox);
                            sandbox.aliases = prev_aliases;
                        }
                    }
                }
            }
            TheCodeNodeCallResult::Continue
        },
        vec![TheValue::Float2(Vec2::new(0.0, 0.0))],
    );

    // Walk
    compiler.add_external_call(
        "Walk".to_string(),
        |stack, data, sandbox| {
            let region_id = sandbox.id;

            let mut distance: f32 = 1.0;
            if let Some(v) = stack.pop() {
                if let Some(f) = v.to_f32() {
                    distance = f;
                }
            }

            let mut direction: i32 = 0;
            if let Some(v) = stack.pop() {
                if let Some(f) = v.to_i32() {
                    direction = f;
                }
            }

            if let Some(region) = REGIONS.read().unwrap().get(&region_id) {
                let mut self_instance_id = Uuid::nil();
                // let mut self_package_id = Uuid::nil();

                let mut target_instance_id = None;

                if let Some(object) = sandbox.get_self_mut() {
                    self_instance_id = object.id;
                    // self_package_id = object.package_id;

                    let mut facing = Vec3::new(0.0, 0.0, 0.0);
                    // Get the facing direction.
                    if let Some(TheValue::Direction(dir)) = object.get_mut(&"facing".into()) {
                        facing = *dir;
                    }

                    let mut by = Vec2::new(0.0, 0.0);
                    if direction == 0 {
                        by = Vec2::new(facing.x, facing.z) * distance;
                    } else if direction == 1 {
                        by = Vec2::new(facing.x, facing.z) * -distance;
                    } else if direction == 2 {
                        by = Vec2::new(-facing.z, facing.x);
                    } else if direction == 3 {
                        by = Vec2::new(facing.z, -facing.x);
                    }

                    if let Some(TheValue::Position(p)) = object.get_mut(&"position".into()) {
                        let x = p.x + by.x;
                        let z = p.z + by.y;

                        if let Some(update) = UPDATES.write().unwrap().get_mut(&region_id) {
                            if region.can_move_to(
                                Vec2::new(x as i32, z as i32),
                                &TILES.read().unwrap(),
                                update,
                            ) {
                                let mut can_move = true;
                                for c in update.characters.values() {
                                    if c.position.x == x && c.position.y == z {
                                        can_move = false;
                                        target_instance_id = Some(c.id);
                                    }
                                }

                                if can_move {
                                    let old_position = *p;
                                    *p = Vec3::new(x, p.y, z);

                                    if let Some(cu) = update.characters.get_mut(&object.id) {
                                        cu.position = Vec2::new(x, z);
                                        cu.moving = Some((
                                            Vec2::new(old_position.x, old_position.z),
                                            cu.position,
                                        ));
                                    }
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

                // We bumped into another character. Get the package id of the other character
                // and call the onContact function of the other / target character.
                if let Some(target_instance_id) = target_instance_id {
                    let mut target_package_id = Uuid::nil();
                    if let Some(target_object) = sandbox.objects.get(&target_instance_id) {
                        target_package_id = target_object.package_id;
                    }

                    //
                    if let Some(target_character) =
                        CHARACTERS.write().unwrap().get_mut(&target_package_id)
                    {
                        if let Some(on_contact) =
                            target_character.get_function_mut(&"onContact".into())
                        {
                            let prev_aliases = sandbox.aliases.clone();
                            sandbox
                                .aliases
                                .insert("self".to_string(), target_instance_id);
                            sandbox
                                .aliases
                                .insert("target".to_string(), self_instance_id);
                            on_contact.execute(sandbox);
                            sandbox.aliases = prev_aliases;
                        }
                    }
                }
            }
            TheCodeNodeCallResult::Continue
        },
        vec![TheValue::Float2(Vec2::new(0.0, 0.0))],
    );

    // Rotate
    compiler.add_external_call(
        "Rotate".to_string(),
        |_stack, _data, _sandbox| {
            // let region_id = sandbox.id;

            // let mut angle: f32 = 0.0;
            // if let Some(v) = stack.pop() {
            //     if let Some(f) = v.to_f32() {
            //         angle = f;
            //     }
            // }

            /*
            if let Some(object) = sandbox.get_self_mut() {
                if let Some(TheValue::Direction(dir)) = object.get_mut(&"facing".into()) {
                    let new_dir_2d = rotate_2d(vec2f(dir.x, dir.z), angle.to_radians());

                    if let Some(update) = UPDATES.write().unwrap().get_mut(&region_id) {
                        let old_direction = *dir;
                        *dir = vec3f(new_dir_2d.x, dir.y, new_dir_2d.y);

                        if let Some(cu) = update.characters.get_mut(&object.id) {
                            cu.facing = new_dir_2d;
                            cu.facing_anim = Some((old_direction.xz(), cu.facing));

                            //     cu.facing = by;
                            cu.move_delta = 0.0;
                        }
                    }
                }
            }*/
            TheCodeNodeCallResult::Continue
        },
        vec![TheValue::Float2(Vec2::new(0.0, 0.0))],
    );

    // Create
    compiler.add_external_call(
        "Create".to_string(),
        |stack, _data, _sandbox| {
            let mut item_name: Option<String> = None;
            if let Some(v) = stack.pop() {
                if let Some(name) = v.to_string() {
                    item_name = Some(name);
                }
            }

            let mut item_id: Option<Uuid> = None;
            let mut items = ITEMS.write().unwrap();

            if let Some(item_name) = item_name {
                for item in items.values() {
                    if item.name == item_name {
                        item_id = Some(item.id);
                        break;
                    }
                }
            }

            if let Some(item_id) = item_id {
                if let Some(package) = items.get_mut(&item_id) {
                    if let Some(init) = package.get_function_mut(&"init".to_string()) {
                        let mut sandbox = TheCodeSandbox::new();
                        let mut object = TheCodeObject::new();
                        let id = Uuid::new_v4();
                        object.id = id;
                        object.package_id = item_id;
                        object.set(str!("_type"), TheValue::Text(str!("Item")));
                        sandbox.add_object(object);
                        sandbox.aliases.insert("self".to_string(), id);
                        init.execute(&mut sandbox);
                        if let Some(object) = sandbox.objects.get(&id) {
                            stack.push(TheValue::CodeObject(object.clone()));
                        }
                    }
                }
            }

            TheCodeNodeCallResult::Continue
        },
        vec![TheValue::CodeObject(TheCodeObject::default())],
    );

    // InArea
    compiler.add_external_call(
        "InArea".to_string(),
        |stack, data, sandbox| {
            let region_id = sandbox.id;

            let mut count = 0;
            if let Some(region) = REGIONS.read().unwrap().get(&region_id) {
                if let Some(area_object) = sandbox.get_self_area_mut() {
                    let id = area_object.id;
                    for object in sandbox.objects.values() {
                        if let Some(TheValue::Position(p)) = object.get(&"position".into()) {
                            if let Some(area) = region.areas.get(&id) {
                                if area.contains(&(p.x as i32, p.z as i32)) {
                                    count += 1;
                                }
                            }
                        }
                    }
                }
            }
            stack.push(TheValue::Int(count));
            if sandbox.debug_mode {
                sandbox.set_debug_value(data.location, (None, TheValue::Int(count)));
            }
            TheCodeNodeCallResult::Continue
        },
        vec![],
    );

    // Tell
    compiler.add_external_call(
        "Tell".to_string(),
        |stack, _data, sandbox| {
            let text = stack.pop().unwrap_or(TheValue::Text(str!("")));

            let mut self_instance_id = Uuid::nil();
            let mut self_name = "".to_string();

            if let Some(object) = sandbox.get_self_mut() {
                self_instance_id = object.id;
                self_name = object
                    .get(&str!("name"))
                    .unwrap_or(&TheValue::Text(str!("")))
                    .describe();
            }

            if let Some(object) = sandbox.get_target_mut() {
                let tell =
                    Interaction::tell(self_instance_id, self_name, object.id, text.describe());
                INTERACTIONS.write().unwrap().push(tell);
            }

            TheCodeNodeCallResult::Continue
        },
        vec![TheValue::Text("".to_string())],
    );

    // WallFX
    compiler.add_external_call(
        "WallFX".to_string(),
        |stack, data, sandbox| {
            let region_id = sandbox.id;

            let mut position = (0, 0);
            let mut effect = "Normal".to_string();

            if let Some(v) = stack.pop() {
                effect = v.describe();
            }

            if let Some(TheValue::Position(v)) = stack.pop() {
                position = (v.x as i32, v.z as i32);
            }

            //println!("WallFX: {} {}", effect, position);

            let fx = WallFX::from_string(&effect);
            if let Some(update) = UPDATES.write().unwrap().get_mut(&region_id) {
                if let Some(wallfx) = update.wallfx.get_mut(&position) {
                    if wallfx.fx != fx {
                        wallfx.prev_fx = wallfx.fx.clone();
                        wallfx.fx = fx;
                        wallfx.at_tick = update.server_tick;
                    }
                } else {
                    update.wallfx.insert(
                        position,
                        WallFxUpdate {
                            at_tick: update.server_tick,
                            fx,
                            prev_fx: WallFX::Normal,
                        },
                    );
                }
            }

            if sandbox.debug_mode {
                sandbox.set_debug_executed(data.location);
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
                        data.values[1] = TheValue::Int(int);
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
                sandbox.add_debug_message(v.describe());
                sandbox.set_debug_value(data.location, (None, v));
            }

            TheCodeNodeCallResult::Continue
        },
        vec![TheValue::Int(0), TheValue::Int(0)],
    );
}
