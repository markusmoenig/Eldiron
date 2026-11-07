//! Opt-in, in-memory gameplay fixture. Never changes a saved project.
//! Movement and attacks go through the region's normal UserAction pipeline;
//! positioning/equipment/HP are fixture setup, not simulated attack results.
use rusterix::server::{
    message::RegionMessage,
    region::{RegionInstance, get_regionctx},
};
use rusterix::{EntityAction, Rusterix, Value};
use std::sync::{Arc, Mutex};
use vek::{Vec2, Vec3};

const PHASE_FRAMES: usize = 240;
const PHASES: [&str; 4] = ["movement", "sword", "spear", "bow"];
pub(super) const DEFAULT_FRAMES: usize = PHASE_FRAMES * PHASES.len();

pub(super) struct StressRun {
    region: u32,
    instance: Arc<Mutex<RegionInstance>>,
    player: u32,
    opponent: u32,
    movers: Vec<(u32, Vec3<f32>)>,
    origin: Vec3<f32>,
    last_positions: Vec<Vec3<f32>>,
    travelled: Vec<f32>,
    attacks: [i32; 4],
    damage: [f32; 4],
    samples: [Vec<f64>; 4],
    baseline_serial: i32,
    baseline_health: f32,
    health_attr: String,
    walk_frames: usize,
    attack_frames: [usize; 4],
}

impl StressRun {
    pub(super) fn new(runtime: &Rusterix) -> Self {
        let (region, player) = runtime
            .server
            .entities
            .iter()
            .find_map(|(r, es)| es.iter().find(|e| e.is_player()).map(|e| (*r, e.id)))
            .expect("Stress run needs a live player");
        let instance = runtime
            .server
            .instances
            .iter()
            .find(|i| i.lock().unwrap().id == region)
            .unwrap()
            .clone();
        let handle = get_regionctx(region).unwrap();
        let mut ctx = handle.lock().unwrap();
        let origin = ctx.find_entity(player).unwrap().position;
        let opponent = ctx
            .map
            .entities
            .iter()
            .find(|e| e.id != player && e.attributes.get_str("race") == Some("Orc"))
            .expect("Stress scene needs a ruleset orc")
            .id;
        // Scripted inputs replace NPC decision making in this controlled fixture.
        // Collision, movement, rules, cooldowns, damage and rendering stay active.
        ctx.node_behaviors = None;
        ctx.entity_proximity_alerts.clear();
        let health_attr = ctx.health_attr.clone();
        let is_2d = matches!(
            runtime.client.active_game_widget_camera_mode(),
            Some(rusterix::PlayerCamera::D2Grid | rusterix::PlayerCamera::D2)
        );
        let max_health_attr = ctx.max_health_attr.clone();
        for e in &mut ctx.map.entities {
            e.set_attribute(&health_attr, Value::Int(100_000));
            e.set_attribute(&max_health_attr, Value::Int(100_000));
            e.action = EntityAction::Off;
        }
        let mut movers = ctx
            .map
            .entities
            .iter()
            .filter(|e| e.id != player)
            .take(3)
            .enumerate()
            .map(|(i, e)| {
                (
                    e.id,
                    origin
                        + if is_2d {
                            // Harbor starts on a small boat. Keep the other walkers on
                            // the land above it, rather than placing them in blocked water.
                            Vec3::new(-2. + i as f32 * 2., 0., -4.)
                        } else {
                            Vec3::new(2. + i as f32, 0., 1.)
                        },
                )
            })
            .collect::<Vec<_>>();
        movers.push((player, origin));
        println!(
            "Creator stress fixture region={region} player={player} opponent={opponent} movers={} origin={origin:?} normal_tick_clock=true",
            movers.len()
        );
        Self {
            region,
            instance,
            player,
            opponent,
            last_positions: movers.iter().map(|(_, p)| *p).collect(),
            travelled: vec![0.; movers.len()],
            movers,
            origin,
            attacks: [0; 4],
            damage: [0.; 4],
            samples: Default::default(),
            baseline_serial: 0,
            baseline_health: 0.,
            health_attr,
            walk_frames: 0,
            attack_frames: [0; 4],
        }
    }

    fn send(&self, id: u32, action: EntityAction) {
        self.instance
            .lock()
            .unwrap()
            .to_sender
            .send(RegionMessage::UserAction(id, action))
            .unwrap();
    }

    pub(super) fn before_frame(&mut self, frame: usize) {
        let phase = (frame / PHASE_FRAMES) % 4;
        let offset = frame % PHASE_FRAMES;
        if offset == 0 {
            let handle = get_regionctx(self.region).unwrap();
            let mut ctx = handle.lock().unwrap();
            if phase == 0 {
                for (id, position) in &self.movers {
                    let e = ctx.get_entity_mut(*id).unwrap();
                    e.set_position(*position);
                    e.action = EntityAction::Off;
                }
                self.last_positions = self.movers.iter().map(|(_, p)| *p).collect();
            } else {
                let weapon = ["", "training_sword", "training_spear", "hunting_bow"][phase];
                for (id, position) in [
                    (self.player, self.origin),
                    (self.opponent, self.origin + Vec3::new(0., 0., 1.)),
                ] {
                    let item = ctx.create_item(weapon.into()).expect("Ruleset weapon");
                    let ammo = ctx
                        .create_item("wooden_arrows".into())
                        .expect("Ruleset ammunition");
                    let e = ctx.get_entity_mut(id).unwrap();
                    e.set_position(position);
                    e.set_orientation(Vec2::new(0., if id == self.player { 1. } else { -1. }));
                    e.action = EntityAction::Off;
                    e.equipped.shift_remove("off_hand");
                    e.equipped.shift_remove("shield");
                    e.equipped.insert("main_hand".into(), item);
                    e.add_item(ammo).unwrap();
                    e.mark_all_dirty();
                }
            }
            self.baseline_serial = [self.player, self.opponent]
                .iter()
                .map(|id| {
                    ctx.find_entity(*id)
                        .unwrap()
                        .attributes
                        .get_int_default("avatar_attack_serial", 0)
                })
                .sum();
            self.baseline_health = [self.player, self.opponent]
                .iter()
                .map(|id| {
                    ctx.find_entity(*id)
                        .unwrap()
                        .attributes
                        .get(&self.health_attr)
                        .and_then(Value::to_f32)
                        .unwrap()
                })
                .sum();
            println!("Creator stress phase frame={frame} phase={}", PHASES[phase]);
        }
        if phase == 0 && offset % 60 == 0 {
            // Traverse and reverse through the real collision/movement path.
            for (id, p) in &self.movers {
                let z = if offset % 120 == 0 { 1. } else { 0. };
                self.send(*id, EntityAction::Goto(Vec2::new(p.x, p.z + z), 1.));
            }
        } else if phase != 0 && offset % 15 == 0 {
            for (actor, target) in [(self.player, self.opponent), (self.opponent, self.player)] {
                self.send(
                    actor,
                    EntityAction::EntityClicked(target, 1., Some("action:basic_attack".into())),
                );
            }
        }
    }

    pub(super) fn after_frame(&mut self, frame: usize, ms: f64) {
        let phase = (frame / PHASE_FRAMES) % 4;
        if phase != 0 && frame % PHASE_FRAMES == 30 {
            let runtime = super::RUSTERIX.read().unwrap();
            for id in [self.player, self.opponent] {
                let entity = runtime.server.entities[&self.region]
                    .iter()
                    .find(|e| e.id == id)
                    .unwrap();
                let model =
                    rusterix::procedural_avatar::Runtime::resolve(entity, &runtime.assets).unwrap();
                assert!(
                    model.equipment_is_prepared(entity, &runtime.assets),
                    "Equipment preparation did not finish within one second for {}",
                    PHASES[phase]
                );
            }
        }
        let handle = get_regionctx(self.region).unwrap();
        let ctx = handle.lock().unwrap();
        if phase == 0 {
            for (i, (id, _)) in self.movers.iter().enumerate() {
                let p = ctx.find_entity(*id).unwrap().position;
                let delta = p - self.last_positions[i];
                self.travelled[i] += Vec2::new(delta.x, delta.z).magnitude();
                self.last_positions[i] = p;
            }
            if self.movers.iter().any(|(id, _)| {
                ctx.find_entity(*id)
                    .unwrap()
                    .attributes
                    .get_str("avatar_animation")
                    == Some("Walk")
            }) {
                self.walk_frames += 1;
            }
        }
        if [self.player, self.opponent].iter().any(|id| {
            ctx.find_entity(*id)
                .unwrap()
                .attributes
                .get_str("avatar_animation")
                == Some("Attack")
        }) {
            self.attack_frames[phase] += 1;
        }
        let serial: i32 = [self.player, self.opponent]
            .iter()
            .map(|id| {
                ctx.find_entity(*id)
                    .unwrap()
                    .attributes
                    .get_int_default("avatar_attack_serial", 0)
            })
            .sum();
        let health: f32 = [self.player, self.opponent]
            .iter()
            .map(|id| {
                ctx.find_entity(*id)
                    .unwrap()
                    .attributes
                    .get(&self.health_attr)
                    .and_then(Value::to_f32)
                    .unwrap()
            })
            .sum();
        self.attacks[phase] += (serial - self.baseline_serial).max(0);
        self.damage[phase] += (self.baseline_health - health).max(0.);
        self.baseline_serial = serial;
        self.baseline_health = health;
        if frame >= 20 {
            self.samples[phase].push(ms);
        }
    }

    pub(super) fn finish(mut self) {
        assert!(
            self.travelled.iter().any(|d| *d > 2.),
            "Movement fixture did not move: {:?}",
            self.travelled
        );
        assert!(*self.travelled.last().unwrap() > 0.5, "Player did not move");
        assert!(self.walk_frames > 0, "No walking animation observed");
        assert!(
            self.travelled.iter().filter(|d| **d > 0.5).count() >= 3,
            "Crowd movement fixture is blocked: {:?}",
            self.travelled
        );
        println!(
            "Creator stress movement travelled={:?} walk_frames={}",
            self.travelled, self.walk_frames
        );
        for (phase, name) in PHASES.iter().enumerate() {
            let samples = &mut self.samples[phase];
            if samples.is_empty() {
                continue;
            }
            if phase != 0 {
                assert!(
                    self.attacks[phase] >= 3,
                    "{name}: no real attacks: {}",
                    self.attacks[phase]
                );
                assert!(self.damage[phase] > 0., "{name}: no real damage");
                assert!(
                    self.attack_frames[phase] > 0,
                    "{name}: no attack animation observed"
                );
            }
            samples.sort_by(f64::total_cmp);
            println!(
                "Creator stress result phase={name} frames={} mean={:.3}ms p50={:.3}ms p95={:.3}ms max={:.3}ms over_budget={} attacks={} damage={:.0} attack_frames={}",
                samples.len(),
                samples.iter().sum::<f64>() / samples.len() as f64,
                samples[samples.len() / 2],
                samples[samples.len() * 95 / 100],
                samples.last().unwrap(),
                samples.iter().filter(|ms| **ms > 1000. / 30.).count(),
                self.attacks[phase],
                self.damage[phase],
                self.attack_frames[phase]
            );
        }
    }
}
