use super::prelude::*;
use crate::server::{CHARACTERS, REGIONS, TILES, UPDATES}; //, _INTERACTIONS, ITEMS, KEY_DOWN};
use regex::Regex;
use theframework::prelude::*;

/// Executes the given command for the given player / client. Returns true if the player was changed.
pub fn execute(
    _client_id: &Uuid,
    cmd: &str,
    player: &mut ActivePlayer,
    sandbox: &mut TheCodeSandbox,
) -> bool {
    let mut rc = false;
    if cmd.starts_with("move") {
        if let Ok(reg) = Regex::new(r"^move\s*(-?\d+(\.\d+)?),\s*(-?\d+(\.\d+)?)$") {
            if let Some(caps) = reg.captures(cmd) {
                let x = caps.get(1).unwrap().as_str().parse::<f32>().unwrap();
                let y = caps.get(3).unwrap().as_str().parse::<f32>().unwrap();
                //println!("move x, y {} {}", x, y);
                //return true;
                rc = move_cmd(vec2f(x, y), player, sandbox);
            }
        }
    }
    rc
}

pub fn move_cmd(by: Vec2f, player: &mut ActivePlayer, sandbox: &mut TheCodeSandbox) -> bool {
    if let Some(region) = REGIONS.read().unwrap().get(&player.region_id) {
        let mut self_instance_id = Uuid::nil();
        // let mut self_package_id = Uuid::nil();

        let mut target_instance_id = None;

        if let Some(object) = sandbox.objects.get_mut(&player.id) {
            self_instance_id = object.id;
            // self_package_id = object.package_id;

            // Set the facing direction to the direction we are moving to
            if let Some(TheValue::Direction(dir)) = object.get_mut(&"facing".into()) {
                *dir = vec3f(by.x, 0.0, by.y);
            }

            if let Some(TheValue::Position(p)) = object.get_mut(&"position".into()) {
                let x = p.x + by.x;
                let z = p.z + by.y;

                if let Some(update) = UPDATES.write().unwrap().get_mut(&player.region_id) {
                    if region.can_move_to(vec2i(x as i32, z as i32), &TILES.read().unwrap(), update)
                    {
                        let mut can_move = true;
                        for c in update.characters.values() {
                            if c.position.x == x && c.position.y == z {
                                can_move = false;
                                target_instance_id = Some(c.id);
                            }
                        }

                        if can_move {
                            let old_position = *p;
                            *p = vec3f(x, p.y, z);

                            if let Some(cu) = update.characters.get_mut(&object.id) {
                                cu.position = vec2f(x, z);
                                cu.moving = Some((old_position.xz(), cu.position));

                                cu.facing = by;
                                cu.move_delta = 0.0;
                            }
                        }
                    }
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
            if let Some(target_character) = CHARACTERS.write().unwrap().get_mut(&target_package_id)
            {
                if let Some(on_contact) = target_character.get_function_mut(&"onContact".into()) {
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
    false
}
