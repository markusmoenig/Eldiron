//! Adapt portable MZ effects to Eldiron's existing particle simulation.
use crate::{ParticleEmissionShape, ParticleEmitter};
use denrim_materials::geometry::effects::{ParticleEffect, ParticlePreset};
use theframework::prelude::ThePalette;
use vek::Vec3;
#[derive(Clone, Debug)]
pub struct Source {
    pub id: String,
    pub definition: ParticleEffect,
    pub position: Vec3<f32>,
    pub scale: f32,
}
pub fn emitter(
    effect: &ParticleEffect,
    palette: &ThePalette,
    origin: Vec3<f32>,
    direction: Vec3<f32>,
    scale: f32,
) -> ParticleEmitter {
    let colors = effect.palette.map(|index| {
        palette
            .colors
            .get(index as usize)
            .and_then(Option::as_ref)
            .map(|c| c.to_u8_array())
            .unwrap_or([0, 0, 0, 0])
    });
    let mut e = ParticleEmitter::new(origin, direction);
    e.color = colors[1];
    e.color_ramp = Some(colors);
    e.color_variation = 0;
    e.rate = effect.rate;
    e.radius_range = (effect.size * 0.6 * scale, effect.size * scale);
    e.lifetime_range = (effect.lifetime * 0.6, effect.lifetime);
    e.speed_range = (effect.speed * 0.6 * scale, effect.speed * scale);
    e.turbulence = effect.turbulence * scale;
    e.spawn_area = [effect.size * 0.5 * scale; 3];
    e.emission_shape = ParticleEmissionShape::Box;
    e.spread = 0.3;
    e.flame_base = effect.preset == ParticlePreset::Flame && effect.rate > 0.;
    e.size_curve = match effect.preset {
        ParticlePreset::Flame => [0.75, 1., 0.55, 0.05],
        ParticlePreset::Smoke => [0.4, 0.8, 1.2, 1.6],
        ParticlePreset::Sparks => [1., 0.8, 0.4, 0.],
    };
    e.opacity_curve = [1., 1., 0.7, 0.];
    if effect.preset == ParticlePreset::Sparks {
        e.spread = 0.8;
        e.gravity = -direction * 1.4 * scale;
    }
    e
}

/// The modelling viewport also uses the engine simulator, rendering its particles as quads.
#[derive(Default)]
pub struct Preview {
    emitters: std::collections::BTreeMap<String, ParticleEmitter>,
    last: Option<instant::Instant>,
}
impl Preview {
    pub fn mesh(
        &mut self,
        mesh: &denrim_materials::geometry::Mesh,
        palette: &ThePalette,
        yaw: f32,
        pitch: f32,
    ) -> denrim_materials::geometry::Mesh {
        use denrim_materials::geometry::{Part, Surface, Vertex};
        let now = instant::Instant::now();
        let dt = self
            .last
            .replace(now)
            .map_or(1. / 30., |t| (now - t).as_secs_f32().min(0.1));
        let mut result = mesh.clone();
        let mut active = std::collections::BTreeSet::new();
        let (sy, cy) = yaw.to_radians().sin_cos();
        let (sp, cp) = pitch.to_radians().sin_cos();
        let right = [cy, 0., sy];
        let up = [-sy * sp, cp, cy * sp];
        for source in &mesh.effects {
            let e = &source.definition;
            if e.rate <= 0. {
                continue;
            }
            active.insert(source.id.clone());
            let origin = Vec3::from(e.position);
            let fresh = emitter(e, palette, origin, Vec3::unit_y(), 1.);
            let sim = self.emitters.entry(source.id.clone()).or_insert_with(|| {
                let mut sim = fresh.clone();
                for _ in 0..8 {
                    sim.update(1. / 60.);
                }
                sim
            });
            sim.sync_settings_from(&fresh);
            sim.origin = origin;
            sim.direction = Vec3::unit_y();
            sim.update(dt);
            let mut particles = sim
                .particles
                .iter()
                .map(|p| (p.pos, p.radius, p.color))
                .collect::<Vec<_>>();
            if sim.flame_base {
                particles.push((
                    origin + Vec3::unit_y() * e.size * 0.3,
                    e.size,
                    sim.color_ramp.unwrap()[0],
                ));
            }
            for (i, (position, radius, color)) in particles.into_iter().enumerate() {
                let palette_index = palette
                    .colors
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| c.as_ref().map(|c| (i, c.to_u8_array())))
                    .min_by_key(|(_, c)| {
                        (0..3)
                            .map(|k| (c[k] as i32 - color[k] as i32).pow(2))
                            .sum::<i32>()
                    })
                    .map_or(0, |(i, _)| i as u16);
                // A small billboard disc, using palette colors rather than opaque squares.
                let mut vertices = vec![];
                for j in 0..12 {
                    let a = j as f32 * std::f32::consts::TAU / 12.;
                    let b = (j + 1) as f32 * std::f32::consts::TAU / 12.;
                    for [x, y] in [
                        [0., 0.],
                        [a.cos(), a.sin() * 1.25],
                        [b.cos(), b.sin() * 1.25],
                    ] {
                        vertices.push(Vertex {
                            position: std::array::from_fn(|k| {
                                position[k] + (right[k] * x + up[k] * y) * radius
                            }),
                            normal: [0., 1., 0.],
                            uv: [0.; 2],
                            skin: None,
                        });
                    }
                }
                result.parts.push(Part {
                    id: format!("@particle/{}/{i}", source.id),
                    label: "Particle".into(),
                    vertices,
                    surface: Some(Surface {
                        palette: palette_index,
                        roughness: 1.,
                        metallic: 0.,
                    }),
                    texture: None,
                });
            }
        }
        self.emitters.retain(|id, _| active.contains(id));
        result
    }
}

/// Standalone appearance previews share the same emitter and simulation as map effects.
#[cfg(feature = "graphics")]
#[derive(Default)]
pub struct Player {
    emitters: std::collections::BTreeMap<String, ParticleEmitter>,
    last: Option<instant::Instant>,
}
#[cfg(feature = "graphics")]
impl Player {
    pub fn render(
        &mut self,
        vm: &mut scenevm::SceneVM,
        sources: &[Source],
        palette: &ThePalette,
        two_d: bool,
        right: Vec3<f32>,
        up: Vec3<f32>,
        uploaded: &mut rustc_hash::FxHashSet<theframework::prelude::Uuid>,
    ) {
        use scenevm::{Atom, DynamicObject, GeoId, Light};
        let tile = theframework::prelude::Uuid::from_u128(0x333e7ce7_f15b_425d_b16d_aa3f34c6ab71);
        if uploaded.insert(tile) {
            let mut pixels = vec![0; 8 * 8 * 4];
            for y in 0..8 {
                for x in 0..8 {
                    if (x as f32 - 3.5).hypot(y as f32 - 3.5) < 3.6 {
                        pixels[(y * 8 + x) * 4..(y * 8 + x) * 4 + 4].copy_from_slice(&[255; 4]);
                    }
                }
            }
            vm.execute(Atom::AddTile {
                id: tile,
                width: 8,
                height: 8,
                frames: vec![pixels],
                material_frames: None,
            });
        }
        let now = instant::Instant::now();
        let dt = self
            .last
            .replace(now)
            .map_or(1. / 30., |t| (now - t).as_secs_f32().min(0.1));
        let mut active = std::collections::BTreeSet::new();
        for source in sources {
            if source.definition.rate <= 0. && source.definition.light <= 0. {
                continue;
            }
            active.insert(source.id.clone());
            let direction = if two_d {
                -Vec3::unit_z()
            } else {
                Vec3::unit_y()
            };
            let fresh = emitter(
                &source.definition,
                palette,
                source.position,
                direction,
                source.scale,
            );
            let sim = self.emitters.entry(source.id.clone()).or_insert_with(|| {
                let mut sim = fresh.clone();
                for _ in 0..8 {
                    sim.update(1. / 60.);
                }
                sim
            });
            sim.sync_settings_from(&fresh);
            sim.origin = source.position;
            sim.direction = direction;
            sim.update(dt);
            let key = super::fingerprint(&source.id) as u32;
            let mut particles = sim
                .particles
                .iter()
                .map(|p| {
                    (
                        p.pos,
                        p.radius,
                        p.color,
                        p.lifetime / source.definition.lifetime,
                        1.0,
                    )
                })
                .collect::<Vec<_>>();
            if sim.flame_base && source.definition.rate > 0. {
                particles.push((
                    source.position + direction * source.definition.size * source.scale * 0.5,
                    source.definition.size * source.scale,
                    sim.color_ramp.unwrap()[0],
                    1.,
                    1.8,
                ));
            }
            for (i, (position, radius, color, alpha, aspect)) in particles.into_iter().enumerate() {
                let id = GeoId::Unknown(0xD100_0000 ^ key.wrapping_add(i as u32));
                let size = radius * 2.;
                let object = if two_d {
                    DynamicObject::particle_tile_2d(
                        id,
                        tile,
                        vek::Vec2::new(position.x, position.z),
                        size,
                        size * aspect,
                    )
                    .with_layer(5)
                } else {
                    DynamicObject::particle_tile(id, tile, position, right, up, size, size * aspect)
                };
                vm.execute(Atom::AddDynamic {
                    object: object
                        .with_tint(
                            Vec3::new(
                                color[0] as f32 / 255.,
                                color[1] as f32 / 255.,
                                color[2] as f32 / 255.,
                            )
                            .map(|v| v.powf(2.2)),
                        )
                        .with_opacity(alpha.clamp(0., 1.)),
                });
            }
            if source.definition.light > 0. {
                let color = fresh.color_ramp.unwrap()[0];
                vm.execute(Atom::AddLight {
                    id: GeoId::Unknown(0xD200_0000 ^ key),
                    light: Light::new_pointlight(source.position)
                        .with_color(
                            Vec3::new(
                                color[0] as f32 / 255.,
                                color[1] as f32 / 255.,
                                color[2] as f32 / 255.,
                            )
                            .map(|v| if two_d { v } else { v.powf(2.2) }),
                        )
                        .with_intensity(source.definition.light)
                        .with_end_distance(source.definition.range)
                        .with_emitting(true),
                });
            }
        }
        self.emitters.retain(|id, _| active.contains(id));
    }
}

pub fn preview_bounds(
    mesh: &denrim_materials::geometry::Mesh,
    mut bounds: ([f32; 3], [f32; 3]),
) -> ([f32; 3], [f32; 3]) {
    for effect in &mesh.effects {
        let e = &effect.definition;
        for k in 0..3 {
            bounds.0[k] = bounds.0[k].min(e.position[k] - e.size * 2.);
            bounds.1[k] = bounds.1[k].max(e.position[k] + e.size * 2.);
        }
        bounds.1[1] = bounds.1[1].max(e.position[1] + e.speed * e.lifetime + e.size * 2.);
    }
    bounds
}
