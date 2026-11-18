use super::*;
use crate::server::regionctx::SimulationMode;

fn region(mode: SimulationMode, camera: PlayerCamera) -> (RegionInstance, Arc<Mutex<RegionCtx>>) {
    let mut ctx = RegionCtx::default();
    ctx.ticks_per_minute = 10;
    ctx.simulation_mode = mode;
    ctx.turn_timeout_ms = 100;
    let mut player = Entity::default();
    player.id = 1;
    player.position = Vec3::new(0.5, 0., 2.5);
    player.orientation = Vec2::new(0., -1.);
    player.set_attribute("player", Value::Bool(true));
    player.set_attribute("player_camera", Value::PlayerCamera(camera));
    ctx.map.entities.push(player);
    let context = Arc::new(Mutex::new(ctx));
    let instance = RegionInstance::with_context(1, context.clone());
    context
        .lock()
        .unwrap()
        .from_sender
        .set(instance.from_sender.clone())
        .unwrap();
    (instance, context)
}
fn action(instance: &mut RegionInstance, action: EntityAction) {
    instance
        .to_sender
        .send(RegionMessage::UserAction(1, action))
        .unwrap();
    instance.redraw_tick();
}

#[test]
fn polls_and_command_floods_cannot_move_or_advance_timers() {
    let (mut instance, ctx) = region(SimulationMode::Realtime, PlayerCamera::D3FirstPMouse);
    let start = ctx.lock().unwrap().map.entities[0].position;
    ctx.lock().unwrap().map.entities[0].set_attribute("avatar_attack_left", Value::Float(1.));
    for _ in 0..1000 {
        action(&mut instance, EntityAction::Forward);
    }
    assert_eq!(ctx.lock().unwrap().map.entities[0].position, start);
    assert_eq!(
        ctx.lock().unwrap().map.entities[0]
            .attributes
            .get_float("avatar_attack_left"),
        Some(1.)
    );
    for _ in 0..50 {
        instance.movement_tick();
    }
    let actor = ctx.lock().unwrap().map.entities[0].clone();
    assert!((actor.position.z - (start.z - 4.)).abs() < 1e-4);
    assert!(actor.attributes.get_float_default("avatar_attack_left", 0.) < 1e-5);
    assert_eq!(instance.simulation_time, Duration::from_secs(1));
    ctx.lock().unwrap().paused = true;
    for _ in 0..50 {
        instance.movement_tick();
    }
    assert_eq!(ctx.lock().unwrap().map.entities[0].position, actor.position);
    assert_eq!(instance.simulation_time, Duration::from_secs(1));
}

#[test]
fn grid_release_finishes_one_step_without_repeating_or_polling_it_forward() {
    let (mut instance, ctx) = region(SimulationMode::TurnBased, PlayerCamera::D3FirstPGrid);
    for _ in 0..100 {
        action(&mut instance, EntityAction::Forward);
    }
    instance.movement_tick();
    action(&mut instance, EntityAction::Off);
    let after_release = ctx.lock().unwrap().map.entities[0].position;
    for _ in 0..100 {
        instance.redraw_tick();
    }
    assert_eq!(ctx.lock().unwrap().map.entities[0].position, after_release);
    for _ in 0..100 {
        instance.movement_tick();
    }
    let actor = ctx.lock().unwrap().map.entities[0].clone();
    assert!(
        (actor.position.z - 1.5).abs() < 1e-4,
        "{:?}",
        actor.position
    );
    assert_eq!(actor.action, EntityAction::Off);
    assert_eq!(
        instance.pending_system_steps, 1,
        "one turn, not one turn per repeated request"
    );
    instance.system_tick();
    instance.system_tick();
    assert_eq!(ctx.lock().unwrap().ticks, 1);
}

#[test]
fn hybrid_timeout_uses_only_host_time_and_turn_backlog_is_bounded() {
    let (mut instance, ctx) = region(SimulationMode::Hybrid, PlayerCamera::D3FirstPMouse);
    for _ in 0..100 {
        instance.redraw_tick();
        instance.system_tick();
    }
    assert_eq!(ctx.lock().unwrap().ticks, 0);
    for _ in 0..5 {
        instance.movement_tick();
    }
    instance.system_tick();
    assert_eq!(ctx.lock().unwrap().ticks, 1);
    for _ in 0..100 {
        action(&mut instance, EntityAction::Forward);
        instance.movement_tick();
    }
    assert!(instance.pending_system_steps <= 1);
    assert!(instance.pending_redraw_steps <= 1);
}
