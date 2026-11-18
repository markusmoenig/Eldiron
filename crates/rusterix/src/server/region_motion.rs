//! Opt-in frame-driven actors. Existing local/script movement remains intact.
use super::*;
use crate::movement::{
    ControllerState, MotionParameters, MotionSnapshot, MovementCommand, MovementWorld,
    motion_revision, step_controller,
};

pub(super) struct FrameDriver {
    pub generation: u64,
    pub processed: u64,
    pub controller: ControllerState,
    pub revision: u32,
    pub anchor: Vec3<f32>,
}
impl RegionInstance {
    fn frame_parameters(&self, ctx: &RegionCtx, actor: &Entity) -> MotionParameters {
        MotionParameters {
            camera: match actor.attributes.get("player_camera") {
                Some(Value::PlayerCamera(camera)) => *camera,
                _ => PlayerCamera::D2,
            },
            units_per_second: self.movement_units_per_sec,
            turn_degrees_per_second: get_config_i32_default(
                ctx,
                "game",
                "turn_speed_deg_per_sec",
                120,
            )
            .max(1) as f32,
            entity_block_mode: self.entity_block_mode,
            collision_mode: self.collision_mode,
            turn_period: if matches!(
                ctx.simulation_mode,
                crate::server::regionctx::SimulationMode::Realtime
            ) {
                0.
            } else {
                Self::non_realtime_turn_dt(ctx)
            },
        }
    }
    pub fn set_frame_motion(&mut self, actor: u32, enabled: bool) -> bool {
        let context = self.context.clone();
        with_regionctx(context, |ctx| {
            let Some(entity) = ctx.map.entities.iter_mut().find(|e| e.id == actor) else {
                return false;
            };
            if !enabled || !self.frame_motion.contains_key(&actor) {
                entity.action = EntityAction::Off;
            }
            entity.set_attribute("__frame_motion", Value::Bool(enabled));
            if enabled {
                self.frame_motion.insert(
                    actor,
                    FrameDriver {
                        generation: 0,
                        processed: 0,
                        controller: Default::default(),
                        revision: motion_revision(entity),
                        anchor: entity.position,
                    },
                );
            } else {
                self.frame_motion.remove(&actor);
            }
            true
        })
        .unwrap_or(false)
    }
    pub fn frame_motion_snapshot(&self, actor: u32) -> Option<MotionSnapshot> {
        with_regionctx(self.context.clone(), |ctx| {
            let entity = ctx.map.entities.iter().find(|e| e.id == actor)?;
            let driver = self.frame_motion.get(&actor)?;
            let discontinuity =
                driver.revision != motion_revision(entity) || driver.anchor != entity.position;
            Some(MotionSnapshot {
                generation: driver.generation,
                processed_sequence: driver.processed,
                revision: motion_revision(entity),
                controller: if discontinuity {
                    Default::default()
                } else {
                    driver.controller.clone()
                },
                parameters: self.frame_parameters(ctx, entity),
                predictable: entity.get_mode() != "dead"
                    && matches!(entity.action, EntityAction::Off)
                    && !crate::furniture::is_resting(entity),
            })
        })
        .flatten()
    }
    pub fn apply_movement_frame(&mut self, actor: u32, command: MovementCommand) -> bool {
        let context = self.context.clone();
        let mut turn = false;
        let applied = with_regionctx(context, |ctx| {
            if ctx.paused || !command.valid() {
                return false;
            }
            let Some(index) = ctx.map.entities.iter().position(|e| e.id == actor) else {
                return false;
            };
            let mut entity = ctx.map.entities[index].clone();
            let parameters = self.frame_parameters(ctx, &entity);
            let Some(driver) = self.frame_motion.get_mut(&actor) else {
                return false;
            };
            if driver.generation != command.generation
                || driver.revision != motion_revision(&entity)
                || driver.anchor != entity.position
            {
                driver.controller = Default::default();
                driver.generation = command.generation;
                driver.processed = 0;
            }
            if command.sequence <= driver.processed {
                return false;
            }
            // A locomotion frame can cancel an authored walk, but an idle frame
            // must not prevent authoritative scripted movement from completing.
            if command.intent != crate::movement::MovementIntent::Stop {
                crate::furniture::stand(&mut entity);
                entity.action = EntityAction::Off;
            }
            if matches!(entity.action, EntityAction::Off) {
                let step = step_controller(
                    &MovementWorld {
                        diagnostics: ctx.diagnostics.as_ref(),
                        map: &ctx.map,
                        mapmini: &ctx.mapmini,
                        collision_world: &ctx.collision_world,
                        assets: &ctx.assets,
                        mode: self.collision_mode,
                    },
                    &mut entity,
                    &mut driver.controller,
                    parameters,
                    command,
                );
                entity.set_attribute("__frame_moved", Value::Bool(step.moved));
                dispatch_motion_contacts(ctx, actor, &step.contacts);
                turn = step.turn_started;
            }
            driver.processed = command.sequence;
            driver.revision = motion_revision(&entity);
            driver.anchor = entity.position;
            ctx.map.entities[index] = entity;
            ctx.check_player_for_section_change_id(actor);
            true
        })
        .unwrap_or(false);
        if turn {
            self.note_simulation_step_request();
        }
        applied
    }
}
