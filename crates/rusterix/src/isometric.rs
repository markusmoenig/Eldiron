//! Native 2D isometric sprite projection. Collision stays in the map's X/Z plane.
pub mod elevation;
pub mod lighting;
pub mod paint;
pub mod surface;
mod terrain;
use crate::{Assets, EntityAction, Map, Texture, Tile, Value};
use serde::{Deserialize, Serialize};
use theframework::prelude::*;

/// Texture frames are direction-major. Animated directions contain walk frames
/// followed by an optional idle frame. Anchors are measured from the top left.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct SpriteLayout {
    #[serde(default)]
    pub bitmap:bool,
    #[serde(default)]
    pub prelit:bool,
    pub anchor: [i32; 2],
    pub pixels_per_unit: f32,
    pub ground: bool,
    pub directions: usize,
    pub frames_per_direction: usize,
    pub walk_frames: usize,
    pub frames_per_second: f32,
    /// Optional derived isometric frames. Flat material textures stay available
    /// for ordinary 2D, 3D surfaces, and MZ's material compiler.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frames: Vec<Texture>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub source_signature: u64,
    /// Unlit per-pixel geometry and material data, in exactly the sprite frame order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub surfaces: Vec<surface::SurfaceFrame>,
    /// Derived from a unit MZ top at local Y=0. Continuous source pages can
    /// replace its repeated baked texture while its bank/side art stays intact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_material: Option<Uuid>,
    /// Derived local X/Z bounds per static view; optional rectangular placement footprints.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub placement_bounds: Vec<[[f32; 2]; 2]>,
    /// Recipe-authored simple X/Z polygon per static view; preferred over bounds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub placement_contours: Vec<Vec<[f32; 2]>>,
    /// Invisible recipe sockets, rotated into each static sprite direction.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub placement_connectors: Vec<Vec<denrim_materials::geometry::connectors::Connector>>,
    /// Compound recipe regions and holes per direction; preferred over legacy contours.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub placement_shapes: Vec<denrim_materials::geometry::footprint::Shape<[f32; 2]>>,
}
fn is_zero(value: &u64) -> bool {
    *value == 0
}
impl SpriteLayout {
    pub fn frame(&self, facing: Vec2<f32>, walking: bool, seconds: f32) -> usize {
        let n = self.directions.clamp(1, 8);
        let direction = ((facing.x.atan2(facing.y) / std::f32::consts::TAU * n as f32).round()
            as i32)
            .rem_euclid(n as i32) as usize;
        let count = self.frames_per_direction.max(1);
        let walk = self.walk_frames.min(count);
        let frame = if walk == 0 && count > 1 && seconds.is_finite() {
            ((seconds.max(0.) * self.frames_per_second.max(0.)) as usize) % count
        } else if walking && walk > 0 && seconds.is_finite() {
            ((seconds.max(0.) * self.frames_per_second.max(0.)) as usize) % walk
        } else {
            walk.min(count - 1)
        };
        direction * count + frame
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Projection {
    pub origin: Vec2<f32>,
    /// Pixels along the horizontal half-width of one ground diamond.
    pub unit: f32,
}

/// Pick the visible terrain surface, independently of edit elevation and props.
/// In an unsculpted map only authored ground is paintable.
pub fn terrain_point(map: &Map, p: Projection, pixel: Vec2<f32>) -> Option<Vec3<f32>> {
    let base = p.ground(pixel);
    if let Some(surface) = crate::chunkbuilder::terrain_surface::TerrainSurface::cached(map) {
        return surface.raycast(
            vek::Vec3::new(base.x, 0., base.y),
            vek::Vec3::new(-1.2247449, 1., 1.2247449),
            surface.max_height,
        );
    }
    flat_terrain_contains(map, base).then_some(Vec3::new(base.x, 0., base.y))
}

pub fn flat_terrain_contains(map: &Map, at: Vec2<f32>) -> bool {
    map.sectors.iter().any(|s| {
        s.properties.get_float_default("floor_height", 0.).abs() < 0.001
            && s.layer.is_none_or(|layer| layer == 0)
            && s.properties.get_default_source().is_some()
            && s.is_inside(map, at)
    })
}
impl Projection {
    pub fn at_height(mut self, height: f32) -> Self {
        self.origin.y -= height * self.unit * 1.2247449;
        self
    }
    pub fn editor(map: &Map, size: Vec2<f32>) -> Self {
        Self {
            origin: size * 0.5 + Vec2::new(map.offset.x, -map.offset.y),
            unit: map.grid_size,
        }
    }
    pub fn screen(&self, ground: Vec2<f32>, height: f32) -> Vec2<f32> {
        self.origin
            + Vec2::new(
                (ground.x + ground.y) * self.unit,
                (ground.y - ground.x) * self.unit * 0.5 - height * self.unit * 1.2247449,
            )
    }
    pub fn ground(&self, screen: Vec2<f32>) -> Vec2<f32> {
        let p = (screen - self.origin) / self.unit.max(0.001);
        Vec2::new(p.x * 0.5 - p.y, p.x * 0.5 + p.y)
    }
}

/// Convert a flat material frame to the same foot-anchored diamond used by MZ geometry.
pub fn ground_sprite(texture: &Texture) -> Texture {
    let mut pixels = vec![0; 64 * 32 * 4];
    let projection = Projection {
        origin: Vec2::new(32., 16.),
        unit: 32.,
    };
    for y in 0..32 {
        for x in 0..64 {
            let world = projection.ground(Vec2::new(x as f32 + 0.5, y as f32 + 0.5));
            if world.x >= -0.5 && world.x < 0.5 && world.y >= -0.5 && world.y < 0.5 {
                let color = texture.sample(
                    world.x + 0.5,
                    world.y + 0.5,
                    crate::SampleMode::Nearest,
                    crate::RepeatMode::ClampXY,
                );
                let i = (y * 64 + x) * 4;
                pixels[i..i + 4].copy_from_slice(&color);
            }
        }
    }
    Texture::new(pixels, 64, 32)
}

#[derive(Clone, Copy)]
pub struct EditorGuides {
    pub grid: bool,
    pub hover: Option<Vec2<f32>>,
}

/// Keyboard axes follow the iso grid: forward +X, right +Z.
/// Click destinations and StepTo actions already contain world coordinates.
pub fn movement(action: &EntityAction) -> EntityAction {
    match action {
        EntityAction::Forward => EntityAction::Right,
        EntityAction::Backward => EntityAction::Left,
        EntityAction::Left => EntityAction::Forward,
        EntityAction::Right => EntityAction::Backward,
        EntityAction::ForwardLeft => EntityAction::BackwardRight,
        EntityAction::ForwardRight => EntityAction::ForwardRight,
        EntityAction::BackwardLeft => EntityAction::BackwardLeft,
        EntityAction::BackwardRight => EntityAction::ForwardLeft,
        other => other.clone(),
    }
}

pub fn grid_step(action: &EntityAction) -> Option<Vec2<f32>> {
    let (x, z) = match action {
        EntityAction::Forward => (1., 0.),
        EntityAction::Backward => (-1., 0.),
        EntityAction::Left => (0., -1.),
        EntityAction::Right => (0., 1.),
        EntityAction::ForwardLeft => (1., -1.),
        EntityAction::ForwardRight => (1., 1.),
        EntityAction::BackwardLeft => (-1., -1.),
        EntityAction::BackwardRight => (-1., 1.),
        _ => return None,
    };
    Some(Vec2::new(x, z))
}

#[derive(Clone)]
pub struct AvatarSprite {
    pub pixels: std::sync::Arc<Vec<u8>>,
    pub surface: Option<std::sync::Arc<surface::SurfaceFrame>>,
}

#[derive(Default)]
pub struct GroundCache {
    key: Option<u64>,
    pixels: Vec<u8>,
    sprite_owners: Vec<u32>,
    terrain_key: Option<(Uuid, u32)>,
    terrain_model: Option<std::sync::Arc<crate::chunkbuilder::terrain_surface::TerrainSurface>>,
    terrain_pixels: Vec<[f32; 5]>,
    surface_key: Option<u64>,
    surfaces: std::collections::HashMap<(u32, Uuid, usize), CachedSurface>,
    shadow_geometry: Option<(u64, std::sync::Arc<scenevm::isometric::ShadowGeometry>)>,
    terrain_shadow: Option<((Uuid, u32), std::sync::Arc<scenevm::isometric::ShadowGeometry>)>,
}

struct CachedSurface {
    image: Texture,
    offset: Vec2<f32>,
}

struct Draw<'a> {
    tile: Option<&'a Tile>,
    avatar: Option<&'a AvatarSprite>,
    stamp: Option<&'a paint::TerrainStamp>,
    scale: f32,
    pos: Vec2<f32>,
    height: f32,
    frame: usize,
    opacity: f32,
    tie: u64,
    support: Option<&'a crate::Sector>,
    radius: f32,
    floor: Option<&'a crate::Sector>,
}

/// Pixel ownership only resolves a floor painting over a sprite it supports.
/// Higher terrain and foreground props keep their normal painter occlusion.
struct ContactPass<'a> {
    owners: &'a mut [u32],
    owner: u32,
    protected: &'a [bool],
    /// Union of the affected sprites, in screen pixels. Distant scenery can
    /// use the ordinary blitter without reading or writing pixel ownership.
    rect: [i32; 4],
    terrain: Option<terrain::Occlusion<'a>>,
}

struct SpriteImage<'a> {
    data: &'a [u8],
    width: usize,
    height: usize,
    anchor: Vec2<f32>,
    unit: f32,
    surface: Option<&'a surface::SurfaceFrame>,
    bitmap:bool,
    prelit:bool,
    ground:bool,
}

impl<'a> Draw<'a> {
    fn image(&self) -> Option<SpriteImage<'a>> {
        if let Some(stamp) = self.stamp {
            return Some(SpriteImage {
                data: &stamp.sprite.texture.data,
                width: stamp.sprite.texture.width,
                height: stamp.sprite.texture.height,
                anchor: Vec2::from(stamp.sprite.anchor),
                unit: paint::PIXELS_PER_UNIT,
                surface: None,
                bitmap:false,prelit:false,ground:false,
            });
        }
        if let Some(avatar) = self.avatar {
            return Some(SpriteImage {
                data: &avatar.pixels,
                width: 128,
                height: 128,
                anchor: Vec2::new(64., 100.),
                unit: 32.,
                surface: avatar.surface.as_deref(),
                bitmap:false,prelit:false,ground:false,
            });
        }
        let tile = self.tile?;
        let (textures, anchor, unit) = if let Some(layout) = &tile.isometric {
            (
                if layout.frames.is_empty() {
                    &tile.textures
                } else {
                    &layout.frames
                },
                Vec2::new(layout.anchor[0] as f32, layout.anchor[1] as f32),
                layout.pixels_per_unit,
            )
        } else {
            let image = tile.textures.first()?;
            (
                &tile.textures,
                Vec2::new(image.width as f32 * 0.5, image.height as f32),
                image.height as f32 / 2.,
            )
        };
        let texture = textures.get(self.frame)?;
        Some(SpriteImage {
            data: &texture.data,
            width: texture.width,
            height: texture.height,
            anchor,
            unit,
            surface: tile.isometric.as_ref().and_then(|s|s.surfaces.get(self.frame)),
            bitmap:tile.isometric.as_ref().is_some_and(|s|s.bitmap),
            prelit:tile.isometric.as_ref().is_some_and(|s|s.prelit),
            ground:tile.isometric.as_ref().is_some_and(|s|s.ground),
        })
    }

    fn image_placement(&self, image: &SpriteImage<'_>, projection: Projection) -> (Vec2<f32>, f32) {
        let scale = projection.unit / image.unit.max(0.001) * self.scale.max(0.01);
        (
            projection.screen(self.pos, self.height) - image.anchor * scale,
            scale,
        )
    }
}

/// Draws native map tile sectors, item sprites, and directional character sprites.
/// Each connecting part is its own tile. Props and characters share one painter order.
pub fn draw(
    map: &Map,
    assets: &Assets,
    buffer: &mut TheRGBABuffer,
    projection: Projection,
    seconds: f32,
) {
    draw_scene(map, assets, buffer, projection, seconds, None);
}

pub fn draw_scene(
    map: &Map,
    assets: &Assets,
    buffer: &mut TheRGBABuffer,
    projection: Projection,
    seconds: f32,
    guides: Option<EditorGuides>,
) {
    draw_scene_with_avatars(
        map,
        assets,
        buffer,
        projection,
        seconds,
        guides,
        &Default::default(),
    );
}

pub fn draw_scene_with_avatars(
    map: &Map,
    assets: &Assets,
    buffer: &mut TheRGBABuffer,
    projection: Projection,
    seconds: f32,
    guides: Option<EditorGuides>,
    avatars: &std::collections::HashMap<usize, AvatarSprite>,
) {
    draw_scene_cached(
        map, assets, buffer, projection, seconds, guides, avatars, None,
    );
}

pub fn draw_scene_cached(
    map: &Map,
    assets: &Assets,
    buffer: &mut TheRGBABuffer,
    projection: Projection,
    seconds: f32,
    guides: Option<EditorGuides>,
    avatars: &std::collections::HashMap<usize, AvatarSprite>,
    cache: Option<&mut GroundCache>,
) {
    draw_scene_lit_cached(
        map, assets, buffer, projection, seconds, guides, avatars, cache, None,
    );
}

pub fn draw_scene_lit_cached(
    map: &Map,
    assets: &Assets,
    buffer: &mut TheRGBABuffer,
    projection: Projection,
    seconds: f32,
    guides: Option<EditorGuides>,
    avatars: &std::collections::HashMap<usize, AvatarSprite>,
    cache: Option<&mut GroundCache>,
    lighting: Option<&lighting::Lighting>,
) {
    draw_scene_impl(
        map, assets, buffer, projection, seconds, guides, avatars, cache, lighting, None,
    );
}
#[cfg(feature = "graphics")]
pub fn draw_scene_gpu_cached(
    map: &Map,
    assets: &Assets,
    buffer: &mut TheRGBABuffer,
    projection: Projection,
    seconds: f32,
    guides: Option<EditorGuides>,
    avatars: &std::collections::HashMap<usize, AvatarSprite>,
    cache: Option<&mut GroundCache>,
    lighting: &lighting::Lighting,
    scene: &mut scenevm::SceneVM,
    particles: &[scenevm::isometric::Particle],
) -> Result<(), String> {
    let mut result = Ok(());
    draw_scene_impl(
        map,
        assets,
        buffer,
        projection,
        seconds,
        guides,
        avatars,
        cache,
        Some(lighting),
        Some(&mut |frame, buffer, p| {
            result = lighting::gpu_draw(scene, frame, buffer, p, particles);
        }),
    );
    result
}

fn draw_scene_impl(
    map: &Map,
    assets: &Assets,
    buffer: &mut TheRGBABuffer,
    projection: Projection,
    seconds: f32,
    guides: Option<EditorGuides>,
    avatars: &std::collections::HashMap<usize, AvatarSprite>,
    mut cache: Option<&mut GroundCache>,
    lighting: Option<&lighting::Lighting>,
    mut gpu: Option<&mut dyn FnMut(&lighting::Frame<'_>, &mut TheRGBABuffer, Projection)>,
) {
    let terrain_model = if let Some(c) = cache.as_deref_mut() {
        let key = (map.id, map.changed);
        if c.terrain_key != Some(key) {
            c.terrain_key = Some(key);
            c.terrain_model = crate::chunkbuilder::terrain_surface::TerrainSurface::from_map(map)
                .map(std::sync::Arc::new);
        }
        c.terrain_model.clone()
    } else {
        crate::chunkbuilder::terrain_surface::TerrainSurface::from_map(map).map(std::sync::Arc::new)
    };
    let mut casters = Vec::new();
    let mut mesh_instances = Vec::new();
    let mut ground = Vec::new();
    let mut upright = Vec::new();
    let mut floors = Vec::new();
    let columns: Vec<_> = map
        .sectors
        .iter()
        .filter(|s| s.properties.get_bool_default("iso_walkable", false))
        .map(|s| {
            (
                s.bounding_box(map),
                s.properties.get_float_default("floor_height", 0.),
            )
        })
        .collect();
    let player = map
        .entities
        .iter()
        .find(|e| e.is_player())
        .map(|e| e.get_pos_xz())
        .or_else(|| {
            map.entities.iter().find_map(|entity| {
                let shown = crate::server::data::character_for_display(entity, assets);
                shown.is_player().then_some(shown.get_pos_xz())
            })
        });
    let inside = player.is_some_and(|p| {
        map.sectors
            .iter()
            .any(|s| s.properties.get_bool_default("iso_interior", false) && s.is_inside(map, p))
    });
    for sector in &map.sectors {
        // Classic isometric interiors remove the roof and near walls entirely.
        // Keeping translucent roof sprites here obscures furniture and actors.
        let hidden = inside && sector.properties.get_bool_default("iso_cutaway", false);
        let Some(source) = sector.properties.get_default_source() else {
            continue;
        };
        let Some(tile) = source
            .render_tile_id(assets)
            .and_then(|id| assets.tile_index(&id))
            .and_then(|i| assets.tile_list.get(i as usize))
        else {
            continue;
        };
        let Some(layout) = &tile.isometric else {
            floors.push((sector, tile));
            continue;
        };
        if layout.ground && !layout.bitmap && (!layout.frames.is_empty() || tile.uv_pages.is_some()) {
            let height = sector.properties.get_float_default("floor_height", 0.);
            if height.abs() < 0.001 {
                floors.push((sector, tile));
            } else if let Some(pos) = sector.center(map) {
                upright.push(Draw {
                    tile: Some(tile),
                    avatar: None,
                    stamp: None,
                    scale: 1.,
                    pos,
                    height,
                    frame: 0,
                    opacity: 1.,
                    tie: sector.id as u64,
                    support: Some(sector),
                    radius: 0.,
                    floor: Some(sector),
                });
            }
            continue;
        }
        let Some(center) = sector.center(map) else {
            continue;
        };
        let pos = sector
            .properties
            .get_vec2("iso_anchor_offset")
            .map(|offset| center + Vec2::from(offset))
            .or_else(|| sector.properties.get_vec2("iso_anchor").map(Vec2::from))
            .unwrap_or(center);
        let entry = Draw {
            tile: Some(tile),
            avatar: None,
            stamp: None,
            scale: 1.,
            pos,
            height: {
                let h = sector.properties.get_float_default("floor_height", 0.);
                if h.abs() < 0.001
                    && !sector.properties.get_bool_default("iso_walkable", false)
                    && sector.properties.get_int("iso_snap_target").is_none()
                {
                    terrain_model
                        .as_ref()
                        .and_then(|t| t.height_at(pos))
                        .unwrap_or(h)
                } else {
                    h
                }
            },
            frame: {
                let yaw = sector
                    .properties
                    .get_float_default("iso_yaw", 0.)
                    .to_radians();
                layout.frame(Vec2::new(yaw.sin(), yaw.cos()), true, seconds)
            },
            opacity: 1.,
            tie: sector.id as u64,
            support: if layout.bitmap && layout.ground {Some(sector)}else if !layout.ground && !tile.blocking && !columns.is_empty() {
                let bounds = sector.bounding_box(map);
                let height = sector.properties.get_float_default("floor_height", 0.);
                // Fill blocks share a walkable column, but ordinary upright
                // props at the same cell must remain occluders.
                let footprint_top =
                    (bounds.min.y - pos.y - (bounds.max.x - pos.x)) * 0.5 * layout.pixels_per_unit;
                let is_top = sector.properties.get_bool_default("iso_walkable", false);
                let below_anchor = layout.anchor[1] as f32 <= -footprint_top + 4.;
                ((is_top || below_anchor)
                    && columns.iter().any(|(b, h)| {
                        *h + 0.001 >= height
                            && (b.min - bounds.min).magnitude_squared() < 0.00001
                            && (b.max - bounds.max).magnitude_squared() < 0.00001
                    }))
                .then_some(sector)
            } else {
                None
            },
            radius: 0.,
            floor: None,
        };
        if let Some(mesh) = layout.surfaces.get(entry.frame).and_then(|f| f.shadow_mesh.as_ref()) {
            mesh_instances.push((mesh, Vec3::new(entry.pos.x, entry.height, entry.pos.y), entry.scale));
        }
        if let Some(grid) = layout
            .surfaces
            .get(entry.frame)
            .and_then(|f| f.shadow.as_ref())
        {
            casters.push(lighting::Caster {
                exact: layout.surfaces.get(entry.frame).is_some_and(|s| s.shadow_mesh.is_some()),
                grid,
                position: Vec3::new(entry.pos.x, entry.height, entry.pos.y),
                scale: entry.scale,
            });
        }
        if hidden {
            continue;
        }
        if layout.ground && !layout.bitmap && entry.height.abs() < 0.001 {
            ground.push(entry)
        } else {
            upright.push(entry)
        }
        if sector.properties.get_bool_default("iso_walkable", false) {
            if let Some(top) = layout
                .top_material
                .and_then(|id| assets.tile_index(&id))
                .and_then(|i| assets.tile_list.get(i as usize))
                .filter(|t| t.uv_pages.is_some())
            {
                upright.push(Draw {
                    tile: Some(top),
                    avatar: None,
                    stamp: None,
                    scale: 1.,
                    pos,
                    height: sector.properties.get_float_default("floor_height", 0.),
                    frame: 0,
                    opacity: 1.,
                    tie: sector.id as u64,
                    support: Some(sector),
                    radius: 0.,
                    floor: Some(sector),
                });
            }
        }
    }
    for item in &map.items {
        if !item.attributes.get_bool_default("visible", true) {
            continue;
        }
        let Some(Value::Source(source)) = item.attributes.get("source") else {
            continue;
        };
        let Some(tile) = source
            .render_tile_id(assets)
            .and_then(|id| assets.tile_index(&id))
            .and_then(|i| assets.tile_list.get(i as usize))
        else {
            continue;
        };
        let Some(layout) = &tile.isometric else {
            continue;
        };
        upright.push(Draw {
            tile: Some(tile),
            avatar: None,
            stamp: None,
            scale: item.attributes.get_float_default("size_2d", 1.),
            pos: item.get_pos_xz(),
            height: item.position.y,
            frame: layout.frame(Vec2::new(0., 1.), true, seconds),
            opacity: 1.,
            tie: (1 << 32) + item.id as u64,
            support: None,
            radius: item.attributes.get_float_default("radius", 0.15).max(0.),
            floor: None,
        });
    }
    for (index, original) in map.entities.iter().enumerate() {
        let display;
        let entity = if original.attributes.get("source").is_none() {
            display = crate::server::data::character_for_display(original, assets);
            &display
        } else {
            original
        };
        if let Some(avatar) = avatars.get(&index) {
            upright.push(Draw {
                tile: None,
                avatar: Some(avatar),
                stamp: None,
                scale: entity.attributes.get_float_default("size_2d", 1.),
                pos: entity.get_pos_xz(),
                height: entity.position.y,
                frame: 0,
                opacity: 1.,
                tie: (2 << 32) + index as u64,
                support: None,
                radius: entity.attributes.get_float_default("radius", 0.22).max(0.),
                floor: None,
            });
            continue;
        }
        if !entity.attributes.get_bool_default("visible", true) {
            continue;
        }
        let Some(Value::Source(source)) = entity.attributes.get("source") else {
            continue;
        };
        let Some(tile) = source
            .render_tile_id(assets)
            .and_then(|id| assets.tile_index(&id))
            .and_then(|i| assets.tile_list.get(i as usize))
        else {
            continue;
        };
        let walking = entity
            .attributes
            .get_str("avatar_animation")
            .is_some_and(|a| a.eq_ignore_ascii_case("walk"));
        upright.push(Draw {
            tile: Some(tile),
            avatar: None,
            stamp: None,
            scale: entity.attributes.get_float_default("size_2d", 1.),
            pos: entity.get_pos_xz(),
            height: entity.position.y,
            frame: tile.isometric.as_ref().map_or_else(
                || tile.frame_index((seconds / 0.25) as usize),
                |layout| layout.frame(entity.orientation, walking, seconds),
            ),
            opacity: 1.,
            tie: (2 << 32) + entity.id as u64,
            support: None,
            radius: entity.attributes.get_float_default("radius", 0.22).max(0.),
            floor: None,
        });
    }
    if let Some(paint) = assets.terrain_paint.get(&map.id) {
        for stamp in &paint.stamps {
            let pos = Vec2::from(stamp.position);
            let height = if let Some(terrain) = &terrain_model {
                let Some(height) = terrain.height_at(pos) else {
                    continue;
                };
                height
            } else {
                if !flat_terrain_contains(map, pos) {
                    continue;
                }
                0.
            };
            upright.push(Draw {
                tile: None,
                avatar: None,
                stamp: Some(stamp),
                scale: 1.,
                pos,
                height,
                frame: 0,
                opacity: 1.,
                tie: (3 << 32) + stamp.order,
                support: None,
                radius: 0.,
                floor: None,
            });
        }
    }
    for entry in &upright {
        if entry.tie < (1u64 << 32) {
            continue;
        }
        if let Some(mesh) = entry.tile
            .and_then(|t| t.isometric.as_ref())
            .and_then(|s| s.surfaces.get(entry.frame))
            .and_then(|s| s.shadow_mesh.as_ref())
            .or_else(|| entry.avatar.and_then(|a| a.surface.as_ref()).and_then(|s| s.shadow_mesh.as_ref()))
        {
            mesh_instances.push((mesh, Vec3::new(entry.pos.x, entry.height, entry.pos.y), entry.scale));
        }
        if let Some(grid) = entry
            .tile
            .and_then(|t| t.isometric.as_ref())
            .and_then(|s| s.surfaces.get(entry.frame))
            .and_then(|s| s.shadow.as_ref())
        {
            casters.push(lighting::Caster {
                exact: entry.tile.and_then(|t| t.isometric.as_ref()).and_then(|s| s.surfaces.get(entry.frame)).is_some_and(|s| s.shadow_mesh.is_some()),
                grid,
                position: Vec3::new(entry.pos.x, entry.height, entry.pos.y),
                scale: entry.scale,
            });
        }
    }
    let mut live = lighting.map(|lighting| {
        if gpu.is_some() {
            lighting::Frame::gpu(
                lighting,
                casters,
                buffer.dim().width as usize * buffer.dim().height as usize,
            )
        } else {
            lighting::Frame::new(lighting, casters)
        }
    });
    if gpu.is_some() && let Some(frame) = &mut live {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (map.id, map.changed, assets.tiles_revision).hash(&mut hasher);
        for (mesh, pos, scale) in &mesh_instances {
            (mesh.vertices.as_ptr() as usize, mesh.vertices.len(), pos.x.to_bits(), pos.y.to_bits(), pos.z.to_bits(), scale.to_bits()).hash(&mut hasher);
        }
        let key = hasher.finish();
        let cached = cache.as_ref().and_then(|c| c.shadow_geometry.as_ref()).filter(|c| c.0 == key).map(|c| c.1.clone());
        let geometry = cached.unwrap_or_else(|| {
            let mut geometry = scenevm::isometric::ShadowGeometry { revision: key, ..Default::default() };
            if let Some(terrain) = &terrain_model {
                let terrain_key = (map.id, map.changed);
                let saved = cache.as_ref().and_then(|c| c.terrain_shadow.as_ref()).filter(|c| c.0 == terrain_key).map(|c| c.1.clone());
                let triangles = saved.unwrap_or_else(|| {
                    let mut triangles = scenevm::isometric::ShadowGeometry::default();
                    terrain.triangles(terrain.bounds, |t| triangles.triangle(t.map(|p| [p.x, p.y, p.z])));
                    let triangles = std::sync::Arc::new(triangles);
                    if let Some(c) = cache.as_deref_mut() { c.terrain_shadow = Some((terrain_key, triangles.clone())); }
                    triangles
                });
                geometry.append(&triangles, [0.; 3], 1.);
            }
            for (mesh, pos, scale) in mesh_instances {
                geometry.append(mesh, [pos.x, pos.y, pos.z], scale);
            }
            let geometry = std::sync::Arc::new(geometry);
            if let Some(c) = cache.as_deref_mut() { c.shadow_geometry = Some((key, geometry.clone())); }
            geometry
        });
        if !geometry.vertices.is_empty() { frame.geometry = Some(geometry); }
    }
    let order = |a: &Draw<'_>, b: &Draw<'_>| {
        (a.pos.y - a.pos.x + a.height * 0.8164966)
            .total_cmp(&(b.pos.y - b.pos.x + b.height * 0.8164966))
            .then(a.tie.cmp(&b.tie))
    };
    ground.sort_by(order);
    upright.sort_by(order);
    let animated_floor = floors.iter().any(|(_, t)| {
        t.uv_pages
            .as_ref()
            .map_or(t.textures.len(), |pages| pages.animation_frames as usize)
            > 1
    });
    let sprite_frames = ground.iter().map(|entry| entry.frame).collect::<Vec<_>>();
    // Cache a padded viewport in 256-pixel camera blocks. Walking/panning can
    // scroll the cached ground by row copies instead of rerasterizing every tile.
    // Preserve the subpixel phase so cached and uncached sampling stay identical.
    let padding = 256;
    let base = projection
        .origin
        .map(|v| (v / padding as f32).floor() * padding as f32);
    let phase = projection.origin.map(|v| v - v.floor());
    let cached_projection = Projection {
        origin: base + padding as f32 + phase,
        unit: projection.unit,
    };
    let cache_width = buffer.dim().width + padding;
    let cache_height = buffer.dim().height + padding;
    let key = crate::procedural_avatar::fingerprint(&(
        map.id,
        map.changed,
        assets.tiles_revision,
        assets.terrain_paint.get(&map.id).map(|p| p.ground_key),
        cached_projection.origin.x.to_bits(),
        cached_projection.origin.y.to_bits(),
        projection.unit.to_bits(),
        cache_width,
        cache_height,
        &sprite_frames,
        if animated_floor {
            (seconds.max(0.) / 0.25) as usize
        } else {
            0
        },
    ));
    let render_ground = |target: &mut TheRGBABuffer, p| {
        target.fill([17, 34, 29, 0]);
        if let Some(terrain) = &terrain_model {
            return terrain::render(map, assets, terrain, target, p, seconds);
        }
        for (sector, tile) in floors {
            raster_floor(map, sector, tile, assets, target, p, seconds, None);
        }
        for entry in ground {
            draw_entry(entry, target, p);
        }
        target
            .pixels_mut()
            .chunks_exact_mut(4)
            .map(|pixel| {
                let hit = if pixel[3] > 0 {
                    [0., 0., 0., 1., 0.]
                } else {
                    [f32::NEG_INFINITY, 0., 0., 1., 0.]
                };
                pixel[3] = 255;
                hit
            })
            .collect()
    };
    let mut fresh_terrain = Vec::new();
    let mut terrain_offset = Vec2::<usize>::zero();
    if let Some(cached) = cache.as_deref_mut() {
        if cached.key != Some(key) {
            let mut surface = TheRGBABuffer::new(TheDim::sized(cache_width, cache_height));
            cached.terrain_pixels = render_ground(&mut surface, cached_projection);
            cached.pixels = surface.pixels().to_vec();
            cached.key = Some(key);
        }
        let offset =
            (cached_projection.origin - projection.origin).map(|v| v.round().max(0.) as usize);
        terrain_offset = offset;
        let width = buffer.dim().width as usize;
        let height = buffer.dim().height as usize;
        for y in 0..height {
            let source = ((y + offset.y) * cache_width as usize + offset.x) * 4;
            buffer.pixels_mut()[y * width * 4..(y + 1) * width * 4]
                .copy_from_slice(&cached.pixels[source..source + width * 4]);
        }
    } else {
        fresh_terrain = render_ground(buffer, projection);
    }
    if let Some(live) = &live {
        let (geometry, width) = if let Some(c) = cache.as_deref() {
            (&c.terrain_pixels, cache_width as usize)
        } else {
            (&fresh_terrain, buffer.dim().width as usize)
        };
        lighting::ground(live, buffer, projection, geometry, width, terrain_offset);
    }
    if let Some(guides) = guides.filter(|_| gpu.is_none()) {
        draw_guides(
            map,
            buffer,
            projection.at_height(map.properties.get_float_default("iso_edit_height", 0.)),
            guides,
        );
    }
    let cache_surfaces = cache.is_some();
    let mut transient = GroundCache {
        terrain_pixels: fresh_terrain,
        ..Default::default()
    };
    let state = cache.as_deref_mut().unwrap_or(&mut transient);
    let surface_key = crate::procedural_avatar::fingerprint(&(
        map.id,
        map.changed,
        assets.tiles_revision,
        projection.unit.to_bits(),
        projection.origin.x.fract().to_bits(),
        projection.origin.y.fract().to_bits(),
    ));
    if state.surface_key != Some(surface_key) {
        state.surfaces.clear();
        state.surface_key = Some(surface_key);
    }
    let (surfaces, owners) = (&mut state.surfaces, &mut state.sprite_owners);
    let objects: Vec<_> = upright.iter().filter(|e| e.support.is_none()).collect();
    let protections: Vec<Vec<bool>> = upright
        .iter()
        .map(|floor| {
            let Some(surface) = floor.support else {
                return Vec::new();
            };
            let bounds = surface.bounding_box(map);
            let mut protected = Vec::with_capacity(objects.len() + 1);
            protected.push(false);
            for object in &objects {
                let nearest = object.pos.clamped(bounds.min, bounds.max);
                protected.push(
                    // A floor already behind the object cannot erase it. Keep
                    // the ordinary fast path unless a later floor overlaps.
                    order(object, floor).is_lt()
                        && object.height + 0.001 >= floor.height
                        && (nearest - object.pos).magnitude_squared()
                            <= object.radius * object.radius + 0.000001
                        && surface.is_inside(map, nearest + (bounds.center() - nearest) * 0.0001),
                );
            }
            protected
        })
        .collect();
    let mut contact_rect = [i32::MAX, i32::MAX, i32::MIN, i32::MIN];
    for (index, object) in objects.iter().enumerate() {
        if protections
            .iter()
            .any(|mask| mask.get(index + 1) == Some(&true))
        {
            if let Some(image) = object.image() {
                let (at, scale) = object.image_placement(&image, projection);
                let x = at.x.round() as i32;
                let y = at.y.round() as i32;
                contact_rect[0] = contact_rect[0].min(x);
                contact_rect[1] = contact_rect[1].min(y);
                contact_rect[2] = contact_rect[2]
                    .max(x.saturating_add((image.width as f32 * scale).round() as i32));
                contact_rect[3] = contact_rect[3]
                    .max(y.saturating_add((image.height as f32 * scale).round() as i32));
            }
        }
    }
    if (contact_rect[0] < contact_rect[2] && contact_rect[1] < contact_rect[3])
        || terrain_model.is_some()
    {
        owners.resize(
            buffer.dim().width as usize * buffer.dim().height as usize,
            0,
        );
        owners.fill(0);
        let mut owner = 0;
        for (entry, protected) in upright.into_iter().zip(&protections) {
            let id = if entry.support.is_none() {
                owner += 1;
                owner
            } else {
                0
            };
            let terrain = (terrain_model.is_some()
                && !state.terrain_pixels.is_empty()
                && entry.support.is_none()
                && entry.floor.is_none())
            .then(|| terrain::Occlusion {
                pixels: &state.terrain_pixels,
                width: if cache_surfaces {
                    cache_width as usize
                } else {
                    buffer.dim().width as usize
                },
                offset: terrain_offset,
                foot_y: projection.screen(entry.pos, entry.height).y,
                foot_height: entry.height,
                foot_depth: entry.pos.y - entry.pos.x + entry.height * 0.8164966,
                unit: projection.unit,
            });
            draw_scene_entry(
                entry,
                buffer,
                projection,
                map,
                assets,
                seconds,
                Some(ContactPass {
                    owners,
                    owner: id,
                    protected,
                    rect: contact_rect,
                    terrain,
                }),
                cache_surfaces.then_some(&mut *surfaces),
                live.as_ref(),
            );
        }
    } else {
        for entry in upright {
            draw_scene_entry(
                entry,
                buffer,
                projection,
                map,
                assets,
                seconds,
                None,
                cache_surfaces.then_some(&mut *surfaces),
                live.as_ref(),
            );
        }
    }
    if let (Some(draw), Some(frame)) = (&mut gpu, &live) {
        draw(frame, buffer, projection);
        if let Some(guides) = guides {
            draw_guides(
                map,
                buffer,
                projection.at_height(map.properties.get_float_default("iso_edit_height", 0.)),
                guides,
            );
        }
    }
}

fn draw_scene_entry(
    entry: Draw<'_>,
    buffer: &mut TheRGBABuffer,
    projection: Projection,
    map: &Map,
    assets: &Assets,
    seconds: f32,
    contacts: Option<ContactPass<'_>>,
    surfaces: Option<&mut std::collections::HashMap<(u32, Uuid, usize), CachedSurface>>,
    lighting: Option<&lighting::Frame<'_>>,
) {
    if let (Some(sector), Some(tile)) = (entry.floor, entry.tile) {
        if let Some(surfaces) = surfaces {
            let frame = (seconds.max(0.) / 0.25) as usize
                % tile
                    .uv_pages
                    .as_ref()
                    .map_or(tile.textures.len(), |p| p.animation_frames as usize)
                    .max(1);
            let key = (sector.id, tile.id, frame);
            if !surfaces.contains_key(&key) {
                if let Some((vertices, _)) = sector.generate_geometry(map) {
                    let height = sector.properties.get_float_default("floor_height", 0.);
                    let points: Vec<_> = vertices
                        .iter()
                        .map(|v| projection.screen(Vec2::from(*v), height))
                        .collect();
                    let min = points
                        .iter()
                        .fold(Vec2::broadcast(f32::INFINITY), |a, p| {
                            Vec2::new(a.x.min(p.x), a.y.min(p.y))
                        })
                        .map(f32::floor);
                    let max = points
                        .iter()
                        .fold(Vec2::broadcast(f32::NEG_INFINITY), |a, p| {
                            Vec2::new(a.x.max(p.x), a.y.max(p.y))
                        })
                        .map(f32::ceil);
                    let size = (max - min).map(|v| v as i32);
                    if size.x > 0 && size.y > 0 && size.x <= 1024 && size.y <= 1024 {
                        let mut image = TheRGBABuffer::new(TheDim::sized(size.x, size.y));
                        let local = Projection {
                            origin: projection.origin - min,
                            unit: projection.unit,
                        };
                        raster_floor_pixels(
                            map, sector, tile, assets, &mut image, local, seconds, None, false,
                            None,
                        );
                        if surfaces.len() > 512 {
                            surfaces.clear();
                        }
                        surfaces.insert(
                            key,
                            CachedSurface {
                                image: Texture::new(
                                    image.pixels().to_vec(),
                                    size.x as usize,
                                    size.y as usize,
                                ),
                                offset: min - projection.origin,
                            },
                        );
                    }
                }
            }
            if let Some(surface) = surfaces.get(&key) {
                blit_image(
                    buffer,
                    surface.image.width,
                    surface.image.height,
                    &surface.image.data,
                    projection.origin + surface.offset,
                    1.,
                    entry.opacity,
                    contacts,
                    lighting.map(|frame| SpriteLighting {
                        frame,
                        projection,
                        surface: None,
                        base_height: entry.height,
                        scale: 1.,
                        bitmap:false,prelit:false,ground:true,anchor:Vec2::zero(),unit:projection.unit,
                    }),
                );
                return;
            }
        }
        raster_floor_pixels(
            map, sector, tile, assets, buffer, projection, seconds, contacts, true, lighting,
        );
    } else {
        draw_entry_lit(entry, buffer, projection, contacts, lighting);
    }
}

fn draw_entry(entry: Draw<'_>, buffer: &mut TheRGBABuffer, projection: Projection) {
    draw_entry_contacts(entry, buffer, projection, None);
}
fn draw_entry_contacts(
    entry: Draw<'_>,
    buffer: &mut TheRGBABuffer,
    projection: Projection,
    contacts: Option<ContactPass<'_>>,
) {
    draw_entry_lit(entry, buffer, projection, contacts, None);
}
fn draw_entry_lit(
    entry: Draw<'_>,
    buffer: &mut TheRGBABuffer,
    projection: Projection,
    contacts: Option<ContactPass<'_>>,
    lighting: Option<&lighting::Frame<'_>>,
) {
    let Some(image) = entry.image() else {
        return;
    };
    let (at, scale) = entry.image_placement(&image, projection);
    blit_image(
        buffer,
        image.width,
        image.height,
        image.data,
        at,
        scale,
        entry.opacity,
        contacts,
        lighting.map(|frame| SpriteLighting {
            frame,
            projection,
            surface: image.surface,
            base_height: entry.height,
            scale: entry.scale,
            bitmap:image.bitmap,prelit:image.prelit,ground:image.ground,anchor:image.anchor,unit:image.unit,
        }),
    );
}

pub fn draw_tile_preview(
    buffer: &mut TheRGBABuffer,
    tile: &Tile,
    p: Projection,
    position: Vec2<f32>,
    yaw: f32,
) {
    let Some(layout) = &tile.isometric else {
        return;
    };
    let yaw = yaw.to_radians();
    draw_entry(
        Draw {
            tile: Some(tile),
            avatar: None,
            stamp: None,
            scale: 1.,
            pos: position,
            height: 0.,
            support: None,
            radius: 0.,
            floor: None,
            frame: layout.frame(Vec2::new(yaw.sin(), yaw.cos()), false, 0.),
            opacity: 0.65,
            tie: 0,
        },
        buffer,
        p,
    );
}

fn raster_floor(
    map: &Map,
    sector: &crate::Sector,
    tile: &Tile,
    assets: &Assets,
    buffer: &mut TheRGBABuffer,
    p: Projection,
    seconds: f32,
    contacts: Option<ContactPass<'_>>,
) {
    raster_floor_pixels(
        map, sector, tile, assets, buffer, p, seconds, contacts, true, None,
    );
}

fn raster_floor_pixels(
    map: &Map,
    sector: &crate::Sector,
    tile: &Tile,
    assets: &Assets,
    buffer: &mut TheRGBABuffer,
    p: Projection,
    seconds: f32,
    mut contacts: Option<ContactPass<'_>>,
    composite: bool,
    lighting: Option<&lighting::Frame<'_>>,
) {
    let mut shading = lighting
        .filter(|f| f.canvas.is_none())
        .map(lighting::Shading::new);
    let Some((points, triangles)) = sector.generate_geometry(map) else {
        return;
    };
    let width = buffer.dim().width.max(0) as usize;
    let height = buffer.dim().height.max(0) as usize;
    let floor = sector.properties.get_float_default("floor_height", 0.);
    let projected = points
        .iter()
        .map(|v| p.screen(Vec2::from(*v), floor))
        .collect::<Vec<_>>();
    let bbox = sector.bounding_box(map);
    let repeat = sector.properties.get_int_default("tile_mode", 1) != 0;
    let frame = (seconds.max(0.) / 0.25) as usize;
    // Keep the same world-cell overrides used by the top-down and 3D painters.
    // A sector-local override takes precedence over the terrain's ground layer.
    let resolve = |source: &crate::PixelSource| {
        source
            .render_tile_id(assets)
            .and_then(|id| assets.tile_index(&id))
            .and_then(|i| assets.tile_list.get(i as usize))
    };
    let mut overrides = std::collections::HashMap::new();
    let mut blends = std::collections::HashMap::new();
    for properties in [
        (floor.abs() < 0.001).then_some(&map.properties),
        Some(&sector.properties),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(Value::TileOverrides(tiles)) = properties.get("tiles") {
            overrides.extend(
                tiles
                    .iter()
                    .filter_map(|(cell, source)| resolve(source).map(|tile| (*cell, tile))),
            );
        }
        if let Some(Value::BlendOverrides(tiles)) = properties.get("blend_tiles") {
            blends.extend(tiles.iter().filter_map(|(cell, (preset, source))| {
                resolve(source).map(|tile| (*cell, (preset.weights(), tile)))
            }));
        }
    }
    let sample = |tile: &Tile, uv: Vec2<f32>| {
        tile.sample_2d(
            uv.into_array(),
            frame,
            crate::SampleMode::Nearest,
            if repeat {
                crate::RepeatMode::RepeatXY
            } else {
                crate::RepeatMode::ClampXY
            },
        )
    };
    let edge = |a: Vec2<f32>, b: Vec2<f32>, q: Vec2<f32>| {
        (b.x - a.x) * (q.y - a.y) - (b.y - a.y) * (q.x - a.x)
    };
    for (a, b, c) in triangles {
        let vertices = [projected[a], projected[b], projected[c]];
        let area = edge(vertices[0], vertices[1], vertices[2]);
        if area.abs() < 0.0001 {
            continue;
        }
        let edges: [_; 3] = std::array::from_fn(|i| {
            let (a, b) = if area > 0. {
                (vertices[i], vertices[(i + 1) % 3])
            } else {
                (vertices[(i + 1) % 3], vertices[i])
            };
            let owns = b.y < a.y || (b.y == a.y && b.x > a.x);
            (a, b, owns)
        });
        let min = vertices
            .iter()
            .fold(Vec2::new(f32::INFINITY, f32::INFINITY), |a, b| {
                Vec2::new(a.x.min(b.x), a.y.min(b.y))
            });
        let max = vertices
            .iter()
            .fold(Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY), |a, b| {
                Vec2::new(a.x.max(b.x), a.y.max(b.y))
            });
        for y in (min.y.floor() as i32).max(0)..(max.y.ceil() as i32).min(height as i32) {
            for x in (min.x.floor() as i32).max(0)..(max.x.ceil() as i32).min(width as i32) {
                let q = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                if edges.iter().any(|(a, b, owns)| {
                    let e = edge(*a, *b, q);
                    e < -0.00001 || (e.abs() <= 0.00001 && !owns)
                }) {
                    continue;
                }
                let world = p.ground(q + Vec2::new(0., floor * p.unit * 1.2247449));
                let uv = if repeat {
                    world
                } else {
                    (world - bbox.min) / (bbox.max - bbox.min).map(|v| v.max(0.001))
                };
                let cell = (world.x.floor() as i32, world.y.floor() as i32);
                let mut color = sample(overrides.get(&cell).copied().unwrap_or(tile), uv);
                if let Some((weights, secondary)) = blends.get(&cell) {
                    let (u, v) = (world.x.rem_euclid(1.), world.y.rem_euclid(1.));
                    let left = weights[0] * (1. - v) + weights[3] * v;
                    let right = weights[1] * (1. - v) + weights[2] * v;
                    color = mix_floor_colors(
                        color,
                        sample(secondary, uv),
                        (left * (1. - u) + right * u).clamp(0., 1.),
                    );
                }
                if floor.abs() < 0.001 {
                    if let Some(paint) = assets.terrain_paint.get(&map.id) {
                        color = paint.composite(color, [world.x, world.y]);
                    }
                }
                if color[3] > 0 {
                    if let Some(pass) = contacts.as_mut() {
                        let at = y as usize * width + x as usize;
                        if pass
                            .protected
                            .get(pass.owners[at] as usize)
                            .copied()
                            .unwrap_or(false)
                        {
                            continue;
                        }
                        pass.owners[at] = pass.owner;
                    }
                }
                if let Some(canvas) = lighting.and_then(|f| f.canvas.as_ref()) {
                    canvas.lock().unwrap()[y as usize * width + x as usize] =
                        surface::packed(Vec3::unit_y(), floor, 0.95, 0., 0.);
                }
                if let Some(shading) = &mut shading {
                    color = shading.color(
                        color,
                        Vec3::new(world.x, floor, world.y),
                        surface::Sample {
                            normal: Vec3::unit_y(),
                            height: floor,
                            roughness: 0.95,
                            metallic: 0.,
                            emission: 0.,
                        },
                    );
                }
                let at = (y as usize * width + x as usize) * 4;
                if composite {
                    blend(buffer.pixels_mut(), at, color);
                } else {
                    buffer.pixels_mut()[at..at + 4].copy_from_slice(&color);
                }
            }
        }
    }
}

/// Blend complete material colors in linear light, as the other cameras do.
fn mix_floor_colors(a: [u8; 4], b: [u8; 4], amount: f32) -> [u8; 4] {
    let linear = |v: u8| {
        let x = v as f32 / 255.;
        if x <= 0.04045 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    };
    std::array::from_fn(|i| {
        if i == 3 {
            return (a[i] as f32 * (1. - amount) + b[i] as f32 * amount).round() as u8;
        }
        let x = linear(a[i]) * (1. - amount) + linear(b[i]) * amount;
        let srgb = if x <= 0.0031308 {
            x * 12.92
        } else {
            1.055 * x.powf(1. / 2.4) - 0.055
        };
        (srgb * 255.).round() as u8
    })
}

fn blend(pixels: &mut [u8], at: usize, color: [u8; 4]) {
    let alpha = color[3] as u32;
    if alpha == 0 || at + 4 > pixels.len() {
        return;
    }
    for k in 0..3 {
        pixels[at + k] =
            ((color[k] as u32 * alpha + pixels[at + k] as u32 * (255 - alpha) + 127) / 255) as u8;
    }
    pixels[at + 3] = 255;
}

/// Editor guides are projected ground geometry, never silhouette outlines on the art.
fn draw_guides(map: &Map, buffer: &mut TheRGBABuffer, p: Projection, guides: EditorGuides) {
    let surface = if map
        .properties
        .get_float_default("iso_edit_height", 0.)
        .abs()
        < 0.001
    {
        crate::chunkbuilder::terrain_surface::TerrainSurface::cached(map)
    } else {
        None
    };
    let screen = |q: Vec2<f32>| {
        p.screen(
            q,
            surface.as_ref().and_then(|t| t.height_at(q)).unwrap_or(0.),
        )
    };
    if guides.grid {
        let w = buffer.dim().width as f32;
        let h = buffer.dim().height as f32;
        let corners = [
            Vec2::zero(),
            Vec2::new(w, 0.),
            Vec2::new(w, h),
            Vec2::new(0., h),
        ]
        .map(|q| p.ground(q));
        let min = corners
            .iter()
            .fold(Vec2::new(f32::INFINITY, f32::INFINITY), |a, b| {
                Vec2::new(a.x.min(b.x), a.y.min(b.y))
            })
            .map(|v| v.floor());
        let max = corners
            .iter()
            .fold(Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY), |a, b| {
                Vec2::new(a.x.max(b.x), a.y.max(b.y))
            })
            .map(|v| v.ceil());
        let subdivisions = map.subdivisions.clamp(1., 8.);
        for axis in 0..2 {
            let start = (min[axis] * subdivisions) as i32;
            let end = (max[axis] * subdivisions) as i32;
            for i in start..=end.min(start + 2048) {
                let v = i as f32 / subdivisions;
                let color = if v.fract() == 0. {
                    [165, 193, 174, 50]
                } else {
                    [145, 174, 155, 20]
                };
                let (a, b) = if axis == 0 {
                    (Vec2::new(v, min.y), Vec2::new(v, max.y))
                } else {
                    (Vec2::new(min.x, v), Vec2::new(max.x, v))
                };
                let steps = if surface.is_some() {
                    ((b - a).magnitude() * 4.).ceil().min(2048.) as usize
                } else {
                    1
                };
                let mut previous = screen(a);
                for k in 1..=steps.max(1) {
                    let next = screen(a + (b - a) * (k as f32 / steps.max(1) as f32));
                    line(buffer, previous, next, color);
                    previous = next;
                }
            }
        }
    }
    for id in &map.selected_sectors {
        if let Some(sector) = map.find_sector(*id) {
            for id in &sector.linedefs {
                if let Some(l) = map.find_linedef(*id) {
                    if let (Some(a), Some(b)) =
                        (map.get_vertex(l.start_vertex), map.get_vertex(l.end_vertex))
                    {
                        line(buffer, screen(a), screen(b), [232, 201, 132, 220]);
                    }
                }
            }
        }
    }
    for id in &map.selected_linedefs {
        if let Some(l) = map.find_linedef(*id) {
            if let (Some(a), Some(b)) =
                (map.get_vertex(l.start_vertex), map.get_vertex(l.end_vertex))
            {
                line(buffer, screen(a), screen(b), [232, 201, 132, 255]);
            }
        }
    }
    for v in map
        .vertices
        .iter()
        .filter(|v| v.properties.get_bool_default("terrain_control", false))
    {
        let q = screen(v.as_vec2());
        line(
            buffer,
            q - Vec2::new(4., 0.),
            q + Vec2::new(4., 0.),
            [215, 190, 120, 220],
        );
        line(
            buffer,
            q - Vec2::new(0., 4.),
            q + Vec2::new(0., 4.),
            [215, 190, 120, 220],
        );
    }
    for id in &map.selected_vertices {
        if let Some(v) = map.get_vertex(*id) {
            let q = screen(v);
            line(
                buffer,
                q - Vec2::new(3., 0.),
                q + Vec2::new(3., 0.),
                [245, 219, 150, 255],
            );
            line(
                buffer,
                q - Vec2::new(0., 3.),
                q + Vec2::new(0., 3.),
                [245, 219, 150, 255],
            );
        }
    }
    if let Some(hover) = guides.hover {
        let a = hover.map(|v| v.floor());
        terrain_rectangle_guide(buffer, map, p, a, a + 1., [196, 211, 179, 150]);
    }
}

/// Draw the brush/selection perimeter on the same facets as ground picking.
/// `p` already includes the active construction plane offset, if any.
pub fn terrain_rectangle_guide(
    buffer: &mut TheRGBABuffer,
    map: &Map,
    p: Projection,
    a: Vec2<f32>,
    b: Vec2<f32>,
    color: [u8; 4],
) {
    let surface = (map
        .properties
        .get_float_default("iso_edit_height", 0.)
        .abs()
        < 0.001)
        .then(|| crate::chunkbuilder::terrain_surface::TerrainSurface::cached(map))
        .flatten();
    let Some(surface) = surface else {
        rectangle_guide(buffer, p, a, b, color);
        return;
    };
    let corners = [a, Vec2::new(b.x, a.y), b, Vec2::new(a.x, b.y)];
    let screen = |q| p.screen(q, surface.height_at(q).unwrap_or(0.));
    for i in 0..4 {
        let a = corners[i];
        let b = corners[(i + 1) % 4];
        let steps = ((b - a).magnitude() * surface.subdivisions as f32)
            .ceil()
            .clamp(1., 2048.) as usize;
        let mut prev = screen(a);
        for k in 1..=steps {
            let next = screen(a + (b - a) * (k as f32 / steps as f32));
            line(buffer, prev, next, color);
            prev = next;
        }
    }
}

pub fn rectangle_guide(
    buffer: &mut TheRGBABuffer,
    p: Projection,
    a: Vec2<f32>,
    b: Vec2<f32>,
    color: [u8; 4],
) {
    let corners = [a, Vec2::new(b.x, a.y), b, Vec2::new(a.x, b.y)];
    for i in 0..4 {
        line(
            buffer,
            p.screen(corners[i], 0.),
            p.screen(corners[(i + 1) % 4], 0.),
            color,
        );
    }
}

pub fn line(buffer: &mut TheRGBABuffer, a: Vec2<f32>, b: Vec2<f32>, color: [u8; 4]) {
    let width = buffer.dim().width.max(0) as usize;
    let height = buffer.dim().height.max(0) as usize;
    let delta = b - a;
    let count = delta.x.abs().max(delta.y.abs()).ceil().min(8192.) as usize;
    if count == 0 {
        return;
    }
    for i in 0..=count {
        let q = a + delta * (i as f32 / count as f32);
        let x = q.x.round() as i32;
        let y = q.y.round() as i32;
        if x >= 0 && y >= 0 && x < width as i32 && y < height as i32 {
            blend(
                buffer.pixels_mut(),
                (y as usize * width + x as usize) * 4,
                color,
            );
        }
    }
}

struct SpriteLighting<'a, 'b> {
    frame: &'a lighting::Frame<'b>,
    projection: Projection,
    surface: Option<&'a surface::SurfaceFrame>,
    base_height: f32,
    scale: f32,
    bitmap:bool,prelit:bool,ground:bool,anchor:Vec2<f32>,unit:f32,
}
fn blit_image(
    buffer: &mut TheRGBABuffer,
    image_width: usize,
    image_height: usize,
    data: &[u8],
    at: Vec2<f32>,
    scale: f32,
    opacity: f32,
    contacts: Option<ContactPass<'_>>,
    lighting: Option<SpriteLighting<'_, '_>>,
) {
    if !scale.is_finite() || scale <= 0. || !at.x.is_finite() || !at.y.is_finite() {
        return;
    }
    let width = buffer.dim().width.max(0) as usize;
    let height = buffer.dim().height.max(0) as usize;
    let x0 = at.x.round() as i32;
    let y0 = at.y.round() as i32;
    let x1 = x0.saturating_add((image_width as f32 * scale).round() as i32);
    let y1 = y0.saturating_add((image_height as f32 * scale).round() as i32);
    let mut contacts = contacts.filter(|pass| {
        pass.terrain.is_some()
            || (x0 < pass.rect[2] && y0 < pass.rect[3] && x1 > pass.rect[0] && y1 > pass.rect[1])
    });
    let pixels = buffer.pixels_mut();
    let mut shading = lighting
        .as_ref()
        .filter(|l| l.frame.canvas.is_none())
        .map(|l| lighting::Shading::new(l.frame));
    // Upscaling repeats source texels. Shade each source surface once per
    // draw, keeping illumination fixed to its geometry rather than screen pixels.
    let mut lit_pixels = lighting
        .as_ref()
        .filter(|l| l.frame.canvas.is_none())
        .map(|_| vec![[0u8; 4]; image_width * image_height]);
    let mut canvas = lighting
        .as_ref()
        .and_then(|l| l.frame.canvas.as_ref())
        .map(|c| c.lock().unwrap());
    for y in y0.max(0)..y1.min(height as i32) {
        for x in x0.max(0)..x1.min(width as i32) {
            let sx = (((x - x0) as f32 / scale) as usize).min(image_width.saturating_sub(1));
            let sy = (((y - y0) as f32 / scale) as usize).min(image_height.saturating_sub(1));
            let src = (sy * image_width + sx) * 4;
            if src + 4 > data.len() {
                continue;
            }
            let alpha = (data[src + 3] as f32 * opacity).round() as u32;
            if alpha == 0 {
                continue;
            }
            let dst = (y as usize * width + x as usize) * 4;
            if let Some(pass) = contacts.as_mut() {
                let at = dst / 4;
                if pass
                    .terrain
                    .as_ref()
                    .is_some_and(|t| t.hides(x as usize, y as usize))
                {
                    continue;
                }
                if pass
                    .protected
                    .get(pass.owners[at] as usize)
                    .copied()
                    .unwrap_or(false)
                {
                    continue;
                }
                pass.owners[at] = pass.owner;
            }
            if let (Some(canvas),Some(l))=(&mut canvas,&lighting) {
                if l.bitmap {
                    let h=l.base_height+if l.ground{0.}else{(l.anchor.y-sy as f32-0.5)/l.unit*l.scale*0.61237246};
                    let mut packed=surface::packed(Vec3::unit_y(),h,0.95,0.,0.);
                    packed[1]|=if l.prelit{1}else{2};canvas[dst/4]=packed;
                }else {
                    canvas[dst/4]=l.surface.and_then(|s|s.packed(sy*image_width+sx,l.base_height,l.scale)).unwrap_or_else(||surface::packed(Vec3::unit_y(),l.base_height,0.95,0.,0.));
                }
            }
            let mut color: [u8; 4] = data[src..src + 4].try_into().unwrap();
            if let (Some(l), Some(shading), Some(lit)) = (&lighting, &mut shading, &mut lit_pixels)
                && !(l.bitmap && l.prelit)
            {
                let cached = &mut lit[sy * image_width + sx];
                if cached[3] == 0 {
                    let sample = l
                        .surface
                        .and_then(|s| s.sample(sy * image_width + sx))
                        .unwrap_or(surface::Sample {
                            normal: Vec3::unit_y(),
                            height: 0.,
                            roughness: 0.95,
                            metallic: 0.,
                            emission: 0.,
                        });
                    let h = l.base_height + sample.height * l.scale;
                    let pixel = Vec2::new(
                        x0 as f32 + (sx as f32 + 0.5) * scale,
                        y0 as f32 + (sy as f32 + 0.5) * scale + h * l.projection.unit * 1.2247449,
                    );
                    let world = l.projection.ground(pixel);
                    *cached = shading.color(color, Vec3::new(world.x, h, world.y), sample);
                }
                color = *cached;
            }
            if alpha == 255 {
                pixels[dst..dst + 4].copy_from_slice(&color);
                continue;
            }
            for k in 0..3 {
                pixels[dst + k] = ((color[k] as u32 * alpha
                    + pixels[dst + k] as u32 * (255 - alpha)
                    + 127)
                    / 255) as u8;
            }
            pixels[dst + 3] = 255;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shadow_scene_reuses_triangles_and_updates_after_terrain_sculpting() {
        let mut map=Map::default();map.camera=crate::MapCamera::TwoDIso;
        map.vertices=vec![crate::Vertex::new(0,-4.,-4.),crate::Vertex::new(1,4.,4.)];
        map.properties.set("terrain_enabled",Value::Bool(true));
        map.properties.set("terrain_edge_falloff",Value::Float(0.));
        map.properties.set("terrain_subdivisions",Value::Int(2));
        let ground=material_floor([50,65,35,255]);
        map.properties.set("default_terrain_tile",Value::Source(crate::PixelSource::TileId(ground.id)));
        let mut prop=contact_actor();
        let mut surface=surface::SurfaceFrame::new(prop.textures[0].width*prop.textures[0].height);
        let mut mesh=scenevm::isometric::ShadowGeometry::default();
        mesh.triangle([[0.,0.,0.],[0.,2.,0.],[0.8,0.,0.]]);
        surface.shadow_mesh=Some(mesh);
        prop.isometric.as_mut().unwrap().surfaces=vec![surface];
        add_contact_surface(&mut map,prop.id,0.,0.,0.,false);
        let mut bitmap=Tile::from_texture(Texture::new(vec![255;4*4*4],4,4));
        bitmap.isometric=Some(SpriteLayout{bitmap:true,prelit:true,anchor:[2,4],pixels_per_unit:2.,directions:1,frames_per_direction:1,..Default::default()});
        add_contact_surface(&mut map,bitmap.id,2.,2.,0.,false);
        let mut assets=Assets::default();assets.set_tiles([(ground.id,ground),(prop.id,prop),(bitmap.id,bitmap)].into_iter().collect());
        let lighting=lighting::Lighting::new(&crate::RenderSettings::default(),std::iter::empty(),0);
        let mut cache=GroundCache::default();
        let mut buffer=TheRGBABuffer::new(TheDim::sized(256,192));
        let projection=Projection{origin:Vec2::new(128.,96.),unit:24.};
        let mut snapshots=Vec::new();
        for pass in 0..3 {
            if pass==2 {
                assert!(crate::chunkbuilder::terrain_sculpt::TerrainSculpt::dab(&mut map,Vec2::new(-1.,-1.),2.,1.,crate::chunkbuilder::terrain_sculpt::SculptMode::Raise));
            }
            draw_scene_impl(&map,&assets,&mut buffer,projection,0.,None,&Default::default(),Some(&mut cache),Some(&lighting),Some(&mut |frame,_,_| snapshots.push(frame.geometry.as_ref().unwrap().clone())));
        }
        assert!(std::sync::Arc::ptr_eq(&snapshots[0],&snapshots[1]),"camera redraw reuses source triangles");
        assert_ne!(snapshots[0].revision,snapshots[2].revision,"sculpt invalidates shadow cache");
        assert_ne!(snapshots[0].vertices,snapshots[2].vertices,"sculpted terrain actually changes the GPU geometry");
        let terrain=crate::chunkbuilder::terrain_surface::TerrainSurface::cached(&map).unwrap();
        let mut terrain_vertices=0;terrain.triangles(terrain.bounds,|_|terrain_vertices+=3);
        assert_eq!(snapshots[2].vertices.len(),terrain_vertices+3,"exactly the terrain and MZ triangle; bitmap gains no invented caster");
    }
    #[test]
    fn generic_iso_animation_loops_without_changing_character_idle_selection() {
        let mut layout=SpriteLayout{directions:1,frames_per_direction:2,frames_per_second:4.,..Default::default()};
        assert_eq!(layout.frame(Vec2::unit_y(),false,0.),0);assert_eq!(layout.frame(Vec2::unit_y(),false,0.25),1);assert_eq!(layout.frame(Vec2::unit_y(),false,0.5),0);
        layout.walk_frames=1;assert_eq!(layout.frame(Vec2::unit_y(),false,0.),1);assert_eq!(layout.frame(Vec2::unit_y(),false,0.25),1);
    }
    #[test]
    fn bitmap_ground_keeps_native_diamond_padding_and_authored_shading() {
        let mut data=vec![0;64*128*4];
        for y in 96..128 {for x in 0..64 {if (x as f32+0.5-32.).abs()/32. + (y as f32+0.5-112.).abs()/16. < 1. {
            data[(y*64+x)*4..(y*64+x)*4+4].copy_from_slice(&[77,113,158,255]);
        }}}
        let mut tile=Tile::from_texture(Texture::new(data,64,128));
        tile.isometric=Some(SpriteLayout{bitmap:true,prelit:true,ground:true,anchor:[32,112],pixels_per_unit:32.,directions:1,frames_per_direction:1,..Default::default()});
        let mut map=Map::default();map.camera=crate::MapCamera::TwoDIso;
        for x in 0..2 {for z in 0..2 {add_contact_surface(&mut map,tile.id,x as f32,z as f32,0.,true);}}
        let mut assets=Assets::default();assets.set_tiles([(tile.id,tile)].into_iter().collect());
        let projection=Projection{origin:Vec2::new(48.,64.),unit:32.};
        let mut buffer=TheRGBABuffer::new(TheDim::sized(224,128));
        let mut settings=crate::RenderSettings::default();settings.sun_enabled=false;settings.ambient_strength=0.;
        let lighting=lighting::Lighting::new(&settings,std::iter::empty(),0);
        draw_scene_lit_cached(&map,&assets,&mut buffer,projection,0.,None,&Default::default(),None,Some(&lighting));
        for x in 0..2 {for z in 0..2 {let p=projection.screen(Vec2::new(x as f32+0.5,z as f32+0.5),0.);
            let at=(p.y as usize*224+p.x as usize)*4;
            assert_eq!(&buffer.pixels()[at..at+4],&[77,113,158,255],"Source diamond must not be projected twice or darkened");
        }}
        let count=buffer.pixels().chunks_exact(4).filter(|p|p[0]==77).count();assert_eq!(count,4096,"Four native 64x32 diamonds retain their coverage");
    }
    #[test]
    fn live_lighting_reuses_unlit_ground_cache_when_sun_and_lamps_change() {
        let floor = material_floor([160, 140, 100, 255]);
        let mut map = Map::default();
        add_contact_surface(&mut map, floor.id, 0., 0., 0., false);
        let mut assets = Assets::default();
        assets.set_tiles([(floor.id, floor)].into_iter().collect());
        let p = Projection {
            origin: Vec2::new(8., 32.),
            unit: 24.,
        };
        let mut image = TheRGBABuffer::new(TheDim::sized(72, 64));
        let mut cache = GroundCache::default();
        let mut settings = crate::RenderSettings::default();
        settings.sun_direction = [0., -1., 0.];
        settings.sun_color = [1.; 3];
        settings.ambient_strength = 0.;
        let sun = lighting::Lighting::new(&settings, std::iter::empty(), 0);
        draw_scene_lit_cached(
            &map,
            &assets,
            &mut image,
            p,
            0.,
            None,
            &Default::default(),
            Some(&mut cache),
            Some(&sun),
        );
        let day = image.pixels().to_vec();
        let cached_pixels = cache.pixels.clone();
        let cached_key = cache.key;
        settings.sun_enabled = false;
        let dark = lighting::Lighting::new(&settings, std::iter::empty(), 0);
        draw_scene_lit_cached(
            &map,
            &assets,
            &mut image,
            p,
            0.,
            None,
            &Default::default(),
            Some(&mut cache),
            Some(&dark),
        );
        assert_ne!(day, image.pixels());
        assert_eq!(cache.key, cached_key);
        assert_eq!(cache.pixels, cached_pixels);
        let lamp = scenevm::Light::new_pointlight(Vec3::new(0.5, 1., 0.5)).with_intensity(2.);
        let local = lighting::Lighting::new(&settings, std::iter::once(&lamp), 0);
        let night = image.pixels().to_vec();
        draw_scene_lit_cached(
            &map,
            &assets,
            &mut image,
            p,
            0.,
            None,
            &Default::default(),
            Some(&mut cache),
            Some(&local),
        );
        assert_ne!(night, image.pixels());
        assert_eq!(cache.pixels, cached_pixels);
        let mut fresh = image.clone();
        draw_scene_lit_cached(
            &map,
            &assets,
            &mut fresh,
            p,
            0.,
            None,
            &Default::default(),
            None,
            Some(&local),
        );
        assert_eq!(fresh.pixels(), image.pixels());
    }
    #[test]
    fn organic_stamps_follow_sculpt_and_share_character_depth() {
        use crate::chunkbuilder::{
            terrain_sculpt::{SculptMode, TerrainSculpt},
            terrain_surface::TerrainSurface,
        };
        let mut map = Map::default();
        map.camera = crate::MapCamera::TwoDIso;
        map.vertices = vec![
            crate::Vertex::new(0, -4., -4.),
            crate::Vertex::new(1, 4., 4.),
        ];
        map.properties.set("terrain_enabled", Value::Bool(true));
        map.properties.set("terrain_subdivisions", Value::Int(4));
        let floor = material_floor([30, 60, 40, 255]);
        map.properties.set(
            "default_terrain_tile",
            Value::Source(crate::PixelSource::TileId(floor.id)),
        );
        let actor = contact_actor();
        let actor_id = actor.id;
        let mut assets = Assets::default();
        assets.set_tiles([(floor.id, floor), (actor_id, actor)].into_iter().collect());
        let pink = [230, 30, 170, 255];
        assets.terrain_paint.insert(
            map.id,
            std::sync::Arc::new(paint::TerrainPaint {
                key: 1,
                ground_key: 0,
                chunks: Default::default(),
                stamps: vec![paint::TerrainStamp {
                    id: Uuid::new_v4(),
                    order: 1,
                    position: [0., 0.],
                    raster_key: 1,
                    sprite: std::sync::Arc::new(paint::StampSprite {
                        texture: Texture::new(pink.repeat(6 * 14), 6, 14),
                        anchor: [3., 14.],
                    }),
                }],
            }),
        );
        let p = Projection {
            origin: Vec2::new(160., 170.),
            unit: 32.,
        };
        let mut cache = GroundCache::default();
        let mut buffer = TheRGBABuffer::new(TheDim::sized(320, 260));
        let mut previous = 170.;
        for mode in [SculptMode::Raise, SculptMode::Smooth, SculptMode::Lower] {
            TerrainSculpt::dab(&mut map, Vec2::zero(), 2., 0.8, mode);
            let h = TerrainSurface::from_map(&map)
                .unwrap()
                .height_at(Vec2::zero())
                .unwrap();
            let foot = p.screen(Vec2::zero(), h);
            draw_scene_cached(
                &map,
                &assets,
                &mut buffer,
                p,
                0.,
                None,
                &Default::default(),
                Some(&mut cache),
            );
            let at = ((foot.y.round() as usize - 7) * 320 + 160) * 4;
            assert_eq!(&buffer.pixels()[at..at + 4], &pink);
            let mut fresh = TheRGBABuffer::new(TheDim::sized(320, 260));
            draw(&map, &assets, &mut fresh, p, 0.);
            assert_eq!(buffer.pixels(), fresh.pixels());
            if matches!(mode, SculptMode::Raise) {
                assert!(foot.y < previous - 20.);
            }
            if matches!(mode, SculptMode::Lower) {
                assert!(foot.y > previous + 20.);
            }
            previous = foot.y;
        }
        for (delta, expected) in [(0.1, BOOTS), (-0.1, pink)] {
            let pos = Vec2::new(-delta, delta);
            let h = TerrainSurface::from_map(&map)
                .unwrap()
                .height_at(pos)
                .unwrap();
            let mut entity = crate::Entity::new();
            entity.position = Vec3::new(pos.x, h, pos.y);
            entity.attributes.set(
                "source",
                Value::Source(crate::PixelSource::TileId(actor_id)),
            );
            map.entities = vec![entity];
            draw_scene_cached(
                &map,
                &assets,
                &mut buffer,
                p,
                0.,
                None,
                &Default::default(),
                Some(&mut cache),
            );
            let h = TerrainSurface::from_map(&map)
                .unwrap()
                .height_at(Vec2::zero())
                .unwrap();
            let at = ((p.screen(Vec2::zero(), h).y.round() as usize - 7) * 320 + 160) * 4;
            assert_eq!(
                &buffer.pixels()[at..at + 4],
                &expected,
                "stamp depth must follow the world, not creation order"
            );
        }
    }
    #[test]
    fn smooth_terrain_keeps_sprite_feet_and_projects_painted_materials_with_cached_depth() {
        let mut map = Map::default();
        map.camera = crate::MapCamera::TwoDIso;
        map.properties.set("terrain_enabled", Value::Bool(true));
        map.properties.set("terrain_subdivisions", Value::Int(4));
        map.properties.set("terrain_edge_falloff", Value::Float(0.));
        map.vertices = vec![
            crate::Vertex::new(0, -4., -4.),
            crate::Vertex::new(1, 4., 4.),
        ];
        let mut control = crate::Vertex::new(2, 0.5, 0.5);
        control.z = 1.2;
        control.properties.set("terrain_control", Value::Bool(true));
        control.properties.set("smoothness", Value::Float(1.));
        map.vertices.push(control);
        let blue = material_floor([20, 30, 160, 255]);
        let red = material_floor([180, 20, 10, 255]);
        let actor = contact_actor();
        let actor_id = actor.id;
        map.properties.set(
            "default_terrain_tile",
            Value::Source(crate::PixelSource::TileId(blue.id)),
        );
        map.properties.set(
            "tiles",
            Value::TileOverrides(
                [((0, 0), crate::PixelSource::TileId(red.id))]
                    .into_iter()
                    .collect(),
            ),
        );
        let mut assets = Assets::default();
        assets.set_tiles(
            [(blue.id, blue), (red.id, red), (actor.id, actor)]
                .into_iter()
                .collect(),
        );
        let terrain = crate::chunkbuilder::terrain_surface::TerrainSurface::from_map(&map).unwrap();
        let p = Projection {
            origin: Vec2::new(128., 150.),
            unit: 32.,
        };
        let mut image = TheRGBABuffer::new(TheDim::sized(256, 256));
        draw(&map, &assets, &mut image, p, 0.);
        for (point, red) in [(Vec2::new(0.5, 0.5), true), (Vec2::new(-0.5, 0.5), false)] {
            let at = p
                .screen(point, terrain.height_at(point).unwrap())
                .map(|v| v as usize);
            let c = &image.pixels()[(at.y * 256 + at.x) * 4..(at.y * 256 + at.x) * 4 + 3];
            assert_eq!(
                c[0] > c[2],
                red,
                "Material failed to follow the hill at {point:?}: {c:?}"
            );
        }
        let mut hero = crate::Entity::new();
        hero.set_attribute(
            "source",
            Value::Source(crate::PixelSource::TileId(actor_id)),
        );
        map.entities.push(hero);
        let mut cache = GroundCache::default();
        let mut cached = image.clone();
        for x in [-1., 0.5, 1.5] {
            let point = Vec2::new(x, 0.5);
            map.entities[0].position = Vec3::new(x, terrain.height_at(point).unwrap(), 0.5);
            draw_scene_cached(
                &map,
                &assets,
                &mut cached,
                p,
                0.,
                None,
                &Default::default(),
                Some(&mut cache),
            );
            draw(&map, &assets, &mut image, p, 0.);
            assert!(
                cached.pixels() == image.pixels(),
                "Terrain depth cache diverged"
            );
            assert_eq!(
                boot_pixels(&cached),
                6 * 22,
                "Supporting slope erased the character at {x}"
            );
        }
    }

    fn material_floor(color: [u8; 4]) -> Tile {
        let mut tile = Tile::from_texture(Texture::new(color.repeat(16), 4, 4));
        tile.isometric = Some(SpriteLayout {
            ground: true,
            anchor: [32, 16],
            pixels_per_unit: 32.,
            directions: 1,
            frames_per_direction: 1,
            frames: vec![ground_sprite(&tile.textures[0])],
            ..Default::default()
        });
        tile
    }

    #[test]
    fn iso_floors_apply_world_paint_overrides_and_soft_edge_blends_with_cache_invalidation() {
        let blue = material_floor([0, 0, 255, 255]);
        let green = material_floor([0, 255, 0, 255]);
        let red = material_floor([255, 0, 0, 255]);
        let mut map = Map::default();
        add_contact_surface(&mut map, blue.id, 0., 0., 0., false);
        let (base, paint, blend) = (blue.id, green.id, red.id);
        let mut assets = Assets::default();
        assets.set_tiles(
            [(base, blue), (paint, green), (blend, red)]
                .into_iter()
                .collect(),
        );
        let p = Projection {
            origin: Vec2::new(32., 64.),
            unit: 32.,
        };
        let mut image = TheRGBABuffer::new(TheDim::sized(128, 128));
        let mut fresh = image.clone();
        let mut cache = GroundCache::default();
        let pixel = |image: &TheRGBABuffer, u: f32| {
            let at = p.screen(Vec2::new(u, 0.5), 0.).map(|v| v.floor() as usize);
            image.pixels()[(at.y * 128 + at.x) * 4..(at.y * 128 + at.x) * 4 + 4].to_vec()
        };
        for state in 0..3 {
            if state == 1 {
                map.properties.set(
                    "tiles",
                    Value::TileOverrides(
                        [((0, 0), crate::PixelSource::TileId(paint))]
                            .into_iter()
                            .collect(),
                    ),
                );
                map.sectors[0].properties.set(
                    "blend_tiles",
                    Value::BlendOverrides(
                        [(
                            (0, 0),
                            (
                                crate::VertexBlendPreset::Left,
                                crate::PixelSource::TileId(blend),
                            ),
                        )]
                        .into_iter()
                        .collect(),
                    ),
                );
            }
            if state == 2 {
                map.sectors[0].properties.remove("blend_tiles");
                map.sectors[0].properties.set(
                    "tiles",
                    Value::TileOverrides(
                        [((0, 0), crate::PixelSource::TileId(base))]
                            .into_iter()
                            .collect(),
                    ),
                );
            }
            map.changed += 1;
            draw_scene_cached(
                &map,
                &assets,
                &mut image,
                p,
                0.,
                None,
                &Default::default(),
                Some(&mut cache),
            );
            draw(&map, &assets, &mut fresh, p, 0.);
            assert_eq!(image.pixels(), fresh.pixels());
            if state == 1 {
                let left = pixel(&image, 0.125);
                let right = pixel(&image, 0.875);
                assert!(left[0] > 220 && left[1] < 120, "left edge: {left:?}");
                assert!(right[1] > 220 && right[0] < 120, "right edge: {right:?}");
            } else {
                assert_eq!(pixel(&image, 0.5), vec![0, 0, 255, 255]);
            }
        }
    }

    #[test]
    fn iso_composed_materials_sample_distinct_world_pages_across_negative_coordinates() {
        let colors = [
            [180, 20, 10, 255],
            [20, 180, 10, 255],
            [20, 10, 180, 255],
            [180, 180, 10, 255],
        ];
        let mut tile = material_floor(colors[0]);
        tile.textures = colors
            .iter()
            .map(|c| Texture::new(c.repeat(16), 4, 4))
            .collect();
        tile.uv_pages = Some(crate::TileUvPages {
            origin: [-1, -1],
            size: [2, 2],
            units_per_page: 1,
            axes: [[0.; 3]; 2],
            animation_frames: 1,
            repeat_period: [0.; 2],
        });
        let mut map = Map::default();
        for z in -1..1 {
            for x in -1..1 {
                add_contact_surface(&mut map, tile.id, x as f32, z as f32, 0., false);
            }
        }
        let mut assets = Assets::default();
        assets.set_tiles([(tile.id, tile)].into_iter().collect());
        let p = Projection {
            origin: Vec2::new(64., 64.),
            unit: 32.,
        };
        let mut image = TheRGBABuffer::new(TheDim::sized(128, 128));
        draw(&map, &assets, &mut image, p, 0.);
        for (index, pos) in [
            Vec2::new(-0.5, -0.5),
            Vec2::new(0.5, -0.5),
            Vec2::new(-0.5, 0.5),
            Vec2::new(0.5, 0.5),
        ]
        .into_iter()
        .enumerate()
        {
            let at = p.screen(pos, 0.).map(|v| v as usize);
            assert_eq!(
                &image.pixels()[(at.y * 128 + at.x) * 4..(at.y * 128 + at.x) * 4 + 4],
                &colors[index],
                "world page {index}"
            );
        }
    }

    #[test]
    fn cached_raised_materials_match_fresh_render_after_panning_and_surface_edits() {
        let mut tile = material_floor([100, 60, 30, 128]);
        let id = tile.id;
        for (i, pixel) in tile.textures[0].data.chunks_exact_mut(4).enumerate() {
            if i % 3 == 0 {
                pixel.copy_from_slice(&[30, 80, 60, 255]);
            }
        }
        let mut map = Map::default();
        add_contact_surface(&mut map, id, 0., 0., 0.75, true);
        let mut assets = Assets::default();
        assets.set_tiles([(id, tile)].into_iter().collect());
        let mut cached = TheRGBABuffer::new(TheDim::sized(160, 160));
        let mut fresh = cached.clone();
        let mut cache = GroundCache::default();
        let mut p = Projection {
            origin: Vec2::new(48., 90.),
            unit: 40.,
        };
        for step in 0..8 {
            match step {
                1 => p.origin.x += 7.,
                2 => {
                    let mut tiles = assets.tiles.clone();
                    tiles.get_mut(&id).unwrap().textures[0].data[0] = 180;
                    assets.set_tiles(tiles);
                }
                3 => {
                    map.vertices[0].x -= 0.25;
                    map.changed += 1;
                }
                4 => {
                    map.sectors[0]
                        .properties
                        .set("floor_height", Value::Float(0.5));
                    map.changed += 1;
                }
                5 => {
                    p.unit = 47.5;
                    p.origin.x += 0.25;
                }
                6 => p.origin.y += 9.,
                _ => {}
            }
            draw_scene_cached(
                &map,
                &assets,
                &mut cached,
                p,
                0.,
                None,
                &Default::default(),
                Some(&mut cache),
            );
            draw(&map, &assets, &mut fresh, p, 0.);
            assert!(
                cached.pixels() == fresh.pixels(),
                "Cached raised surface diverged at edit {step}"
            );
        }
    }

    const BOOTS: [u8; 4] = [20, 180, 10, 255];

    fn contact_actor() -> Tile {
        let mut pixels = vec![0; 128 * 128 * 4];
        for y in 80..102 {
            for x in 61..67 {
                pixels[(y * 128 + x) * 4..(y * 128 + x) * 4 + 4].copy_from_slice(&BOOTS);
            }
        }
        let mut tile = Tile::from_texture(Texture::new(pixels, 128, 128));
        tile.isometric = Some(SpriteLayout {
            anchor: [64, 100],
            pixels_per_unit: 32.,
            directions: 1,
            frames_per_direction: 1,
            ..Default::default()
        });
        tile
    }

    fn contact_block() -> Tile {
        // A top-anchored diamond with a vertical front, like the MZ step bake.
        let diamond = ground_sprite(&Texture::new([180, 20, 10, 255].repeat(16), 4, 4));
        let mut pixels = vec![0; 64 * 42 * 4];
        for y in 0..32 {
            for x in 0..64 {
                let src = (y * 64 + x) * 4;
                if diamond.data[src + 3] != 0 {
                    for depth in 0..=10 {
                        let dst = ((y + depth) * 64 + x) * 4;
                        pixels[dst..dst + 4].copy_from_slice(&diamond.data[src..src + 4]);
                    }
                }
            }
        }
        let mut tile = Tile::from_texture(Texture::new(pixels, 64, 42));
        tile.isometric = Some(SpriteLayout {
            anchor: [32, 16],
            pixels_per_unit: 32.,
            directions: 1,
            frames_per_direction: 1,
            ..Default::default()
        });
        tile
    }

    fn add_contact_surface(map: &mut Map, tile: Uuid, x: f32, z: f32, height: f32, top: bool) {
        let vertices = [[x, z], [x + 1., z], [x + 1., z + 1.], [x, z + 1.]].map(|p| {
            // Stacked sectors have independent rings at the same X/Z.
            let id = map
                .vertices
                .iter()
                .map(|v| v.id)
                .max()
                .map_or(0, |id| id + 1);
            map.vertices.push(crate::Vertex::new(id, p[0], p[1]));
            id
        });
        for i in 0..4 {
            map.create_linedef_manual(vertices[i], vertices[(i + 1) % 4]);
        }
        let id = map.close_polygon_manual().unwrap();
        let sector = map.find_sector_mut(id).unwrap();
        sector
            .properties
            .set("source", Value::Source(crate::PixelSource::TileId(tile)));
        sector.properties.set("floor_height", Value::Float(height));
        sector.properties.set("iso_walkable", Value::Bool(top));
    }

    fn boot_pixels(buffer: &TheRGBABuffer) -> usize {
        buffer
            .pixels()
            .chunks_exact(4)
            .filter(|pixel| *pixel == BOOTS)
            .count()
    }

    #[test]
    fn supporting_steps_keep_feet_visible_across_interiors_seams_and_cached_frames() {
        let actor = contact_actor();
        let block = contact_block();
        let mut map = Map::default();
        for x in 0..2 {
            for z in 0..2 {
                for height in [0.5, 0.75, 1.] {
                    add_contact_surface(
                        &mut map,
                        block.id,
                        x as f32,
                        z as f32,
                        height,
                        height == 1.,
                    );
                }
            }
        }
        let mut hero = crate::Entity::new();
        hero.set_attribute(
            "source",
            Value::Source(crate::PixelSource::TileId(actor.id)),
        );
        map.entities.push(hero);
        let mut avatars = std::collections::HashMap::new();
        let live = AvatarSprite {
            surface: None,
            pixels: std::sync::Arc::new(actor.textures[0].data.clone()),
        };
        let mut assets = Assets::default();
        let mut top = material_floor([20, 30, 160, 255]);
        top.uv_pages = Some(crate::TileUvPages {
            origin: [0, 0],
            size: [1, 1],
            units_per_page: 2,
            axes: [[0.; 3]; 2],
            animation_frames: 1,
            repeat_period: [0.; 2],
        });
        let (block_id, top_id) = (block.id, top.id);
        assets.set_tiles(
            [(block.id, block), (actor.id, actor), (top.id, top)]
                .into_iter()
                .collect(),
        );
        let mut cache = GroundCache::default();
        let mut cached = TheRGBABuffer::new(TheDim::sized(256, 256));
        let mut fresh = cached.clone();
        for (live_avatar, continuous) in
            [(false, false), (true, false), (false, true), (true, true)]
        {
            let mut tiles = assets.tiles.clone();
            tiles
                .get_mut(&block_id)
                .unwrap()
                .isometric
                .as_mut()
                .unwrap()
                .top_material = continuous.then_some(top_id);
            assets.set_tiles(tiles);
            avatars.clear();
            if live_avatar {
                avatars.insert(
                    0,
                    AvatarSprite {
                        surface: None,
                        pixels: live.pixels.clone(),
                    },
                );
            }
            // Both halves of each tile, the shared edge, and a frame away from
            // any support. Reuse the owner scratch through all of these frames.
            for (x, z) in [
                (0.2, 0.2),
                (0.8, 0.2),
                (0.2, 0.8),
                (0.8, 0.8),
                (0.95, 0.95),
                (1.05, 1.05),
                (1.8, 1.2),
                (1.2, 1.8),
                (3., 3.),
                (0.8, 0.2),
            ] {
                let height = if x > 2. { 0. } else { 1. };
                map.entities[0].position = Vec3::new(x, height, z);
                let mut projection = Projection {
                    origin: Vec2::zero(),
                    unit: 32.,
                };
                projection.origin =
                    Vec2::new(128., 160.) - projection.screen(Vec2::new(x, z), height);
                draw_scene_cached(
                    &map,
                    &assets,
                    &mut cached,
                    projection,
                    0.,
                    None,
                    &avatars,
                    Some(&mut cache),
                );
                draw_scene_with_avatars(&map, &assets, &mut fresh, projection, 0., None, &avatars);
                assert_eq!(
                    cached.pixels(),
                    fresh.pixels(),
                    "cached contacts at {x},{z}"
                );
                assert_eq!(
                    boot_pixels(&fresh),
                    6 * 22,
                    "missing feet at {x},{z}, live={live_avatar}"
                );
                if continuous && x < 2. {
                    assert!(
                        fresh
                            .pixels()
                            .chunks_exact(4)
                            .any(|p| p == [20, 30, 160, 255]),
                        "Continuous cap was not rendered"
                    );
                }
            }
        }
    }

    #[test]
    fn supporting_floor_preserves_foreground_props_and_higher_terrain_occlusion() {
        let actor = contact_actor();
        let block = contact_block();
        let mut prop = contact_actor();
        for color in prop.textures[0].data.chunks_exact_mut(4) {
            if color[3] != 0 {
                color.copy_from_slice(&[10, 20, 180, 255]);
            }
        }
        let mut map = Map::default();
        let mut hero = crate::Entity::new();
        hero.position = Vec3::new(0.8, 1., 0.2); // Back half: supporting floor sorts later.
        hero.set_attribute(
            "source",
            Value::Source(crate::PixelSource::TileId(actor.id)),
        );
        map.entities.push(hero);
        let mut item = crate::Item::new();
        item.position = Vec3::new(0.8, 1., 0.25);
        item.attributes
            .set("source", Value::Source(crate::PixelSource::TileId(prop.id)));
        map.items.push(item);
        let mut assets = Assets::default();
        let block_id = block.id;
        assets.set_tiles(
            [(actor.id, actor), (block.id, block), (prop.id, prop)]
                .into_iter()
                .collect(),
        );
        let projection = Projection {
            origin: Vec2::new(64., 144.),
            unit: 32.,
        };
        let mut baseline = TheRGBABuffer::new(TheDim::sized(192, 192));
        draw(&map, &assets, &mut baseline, projection, 0.);
        assert!(
            boot_pixels(&baseline) < 6 * 22,
            "prop must hide part of the actor"
        );
        add_contact_surface(&mut map, block_id, 0., 0., 1., true);
        let mut actual = baseline.clone();
        draw(&map, &assets, &mut actual, projection, 0.);
        for (before, after) in baseline
            .pixels()
            .chunks_exact(4)
            .zip(actual.pixels().chunks_exact(4))
        {
            if before == BOOTS || before == [10, 20, 180, 255] {
                assert_eq!(
                    before, after,
                    "support painted over character or foreground prop"
                );
            }
        }
        map.items.clear();
        // A higher top is a real occluder, even when its footprint contains the actor.
        add_contact_surface(&mut map, block_id, 0., 0., 1.25, true);
        draw(&map, &assets, &mut actual, projection, 0.);
        assert!(
            boot_pixels(&actual) < 6 * 22,
            "higher terrain must still cover the actor"
        );
    }

    #[test]
    fn right_aligned_axes_match_ground_tiles() {
        let p = Projection {
            origin: Vec2::new(100., 100.),
            unit: 32.,
        };
        let x = p.screen(Vec2::new(1., 0.), 0.);
        let z = p.screen(Vec2::new(0., 1.), 0.);
        assert_eq!(x, Vec2::new(132., 84.));
        assert_eq!(z, Vec2::new(132., 116.));
        let tile = ground_sprite(&Texture::new([80, 120, 30, 255].repeat(16), 4, 4));
        assert_eq!((tile.width, tile.height), (64, 32));
        assert_eq!(tile.data[3], 0);
        assert_eq!(tile.data[(16 * 64 + 32) * 4 + 3], 255);
        assert!(tile.data.chunks_exact(4).all(|c| c[3] == 0 || c[3] == 255));
    }
    #[test]
    fn picking_roundtrips_at_zoom_and_offset() {
        for unit in [16., 32., 67.] {
            let p = Projection {
                origin: Vec2::new(412., 175.),
                unit,
            };
            for x in -5..12 {
                for z in -5..12 {
                    let a = Vec2::new(x as f32 + 0.3, z as f32 + 0.7);
                    assert!((p.ground(p.screen(a, 0.)) - a).magnitude() < 0.0001);
                }
            }
        }
    }
    #[test]
    fn animation_selects_all_directions_and_idle_separately() {
        let l = SpriteLayout {
            anchor: [64, 100],
            pixels_per_unit: 32.,
            ground: false,
            directions: 8,
            frames_per_direction: 17,
            walk_frames: 16,
            frames_per_second: 16.,
            frames: Vec::new(),
            source_signature: 0,
            bitmap:false,prelit:false,
            surfaces: Vec::new(),
            top_material: None,
            placement_bounds: Vec::new(),
            placement_contours: Vec::new(),
            placement_shapes: Vec::new(),
            placement_connectors: Vec::new(),
        };
        for i in 0..8 {
            let angle = i as f32 * std::f32::consts::FRAC_PI_4;
            let facing = Vec2::new(angle.sin(), angle.cos());
            assert_eq!(l.frame(facing, true, 0.25), i * 17 + 4);
            assert_eq!(l.frame(facing, false, 0.25), i * 17 + 16);
        }
    }
    #[test]
    fn keyboard_directions_follow_isometric_grid() {
        assert_eq!(movement(&EntityAction::Forward), EntityAction::Right);
        assert_eq!(movement(&EntityAction::Right), EntityAction::Backward);
        assert_eq!(grid_step(&EntityAction::Forward), Some(Vec2::new(1., 0.)));
        assert_eq!(grid_step(&EntityAction::Right), Some(Vec2::new(0., 1.)));
        assert_eq!(
            grid_step(&EntityAction::ForwardLeft),
            Some(Vec2::new(1., -1.))
        );
        assert_eq!(
            movement(&EntityAction::StepTo(
                Vec2::one(),
                1.,
                Vec2::one(),
                Vec2::zero(),
                Vec2::one()
            )),
            EntityAction::StepTo(Vec2::one(), 1., Vec2::one(), Vec2::zero(), Vec2::one())
        );
    }

    #[test]
    fn isometric_ground_cache_matches_fresh_render_and_invalidates_edits() {
        let mut tile = Tile::from_texture(ground_sprite(&Texture::new(
            [90, 120, 40, 255].repeat(16),
            4,
            4,
        )));
        tile.isometric = Some(SpriteLayout {
            ground: true,
            anchor: [32, 16],
            pixels_per_unit: 32.,
            directions: 1,
            frames_per_direction: 1,
            ..Default::default()
        });
        let id = tile.id;
        let mut assets = Assets::default();
        assets.set_tiles([(id, tile)].into_iter().collect());
        let mut map = Map::default();
        let vertices =
            [[0., 0.], [1., 0.], [1., 1.], [0., 1.]].map(|p| map.add_vertex_at(p[0], p[1]));
        for i in 0..4 {
            map.create_linedef_manual(vertices[i], vertices[(i + 1) % 4]);
        }
        let sector = map.close_polygon_manual().unwrap();
        map.find_sector_mut(sector)
            .unwrap()
            .properties
            .set("source", Value::Source(crate::PixelSource::TileId(id)));
        let mut cached = TheRGBABuffer::new(TheDim::sized(128, 128));
        let mut fresh = cached.clone();
        let mut cache = GroundCache::default();
        let avatars = Default::default();
        let mut projection = Projection {
            origin: Vec2::new(32., 64.),
            unit: 32.,
        };
        for step in 0..8 {
            match step {
                2 => {
                    projection.origin.x += 7.;
                }
                3 => {
                    map.vertices[0].x -= 1.;
                    map.changed += 1;
                }
                4 => {
                    let mut tiles = assets.tiles.clone();
                    tiles.get_mut(&id).unwrap().textures[0].data.fill(255);
                    assets.set_tiles(tiles);
                }
                5 => {
                    let mut tiles = assets.tiles.clone();
                    let tile = tiles.get_mut(&id).unwrap();
                    tile.textures.push(ground_sprite(&Texture::new(
                        [200, 20, 50, 255].repeat(16),
                        4,
                        4,
                    )));
                    let layout = tile.isometric.as_mut().unwrap();
                    layout.frames_per_direction = 2;
                    layout.walk_frames = 2;
                    layout.frames_per_second = 4.;
                    assets.set_tiles(tiles);
                }
                7 => {
                    projection.origin.x += 0.25;
                }
                _ => {}
            }
            draw_scene_cached(
                &map,
                &assets,
                &mut cached,
                projection,
                step as f32 * 0.125,
                None,
                &avatars,
                Some(&mut cache),
            );
            draw(&map, &assets, &mut fresh, projection, step as f32 * 0.125);
            assert_eq!(
                cached.pixels(),
                fresh.pixels(),
                "cache diverged at edit {step}"
            );
        }
    }

    #[test]
    fn characters_and_props_share_painter_order() {
        let make = |color: [u8; 4]| {
            let mut tile = Tile::from_texture(Texture::new(color.repeat(8 * 12), 8, 12));
            tile.isometric = Some(SpriteLayout {
                anchor: [0, 0],
                pixels_per_unit: 32.,
                ground: false,
                directions: 1,
                frames_per_direction: 1,
                walk_frames: 0,
                frames_per_second: 1.,
                frames: Vec::new(),
                source_signature: 0,
            bitmap:false,prelit:false,
                surfaces: Vec::new(),
                top_material: None,
                placement_bounds: Vec::new(),
                placement_contours: Vec::new(),
                placement_shapes: Vec::new(),
                placement_connectors: Vec::new(),
            });
            tile
        };
        let red = make([180, 20, 10, 255]);
        let green = make([20, 180, 10, 255]);
        let mut assets = Assets::new();
        let mut tiles = indexmap::IndexMap::default();
        let prop_id = red.id;
        let actor_id = green.id;
        tiles.insert(red.id, red);
        tiles.insert(green.id, green);
        assets.set_tiles(tiles);
        let mut map = Map::default();
        let mut item = crate::Item::new();
        item.position = Vec3::zero();
        item.attributes
            .set("source", Value::Source(crate::PixelSource::TileId(prop_id)));
        map.items.push(item);
        let mut hero = crate::Entity::new();
        hero.attributes.set(
            "source",
            Value::Source(crate::PixelSource::TileId(actor_id)),
        );
        map.entities.push(hero);
        let projection = Projection {
            origin: Vec2::new(32., 32.),
            unit: 32.,
        };
        let mut buffer = TheRGBABuffer::new(TheDim::sized(64, 64));
        for (pos, expected) in [(0.125, [20, 180, 10, 255]), (-0.125, [180, 20, 10, 255])] {
            map.entities[0].position = Vec3::new(-pos, 0., pos);
            draw(&map, &assets, &mut buffer, projection, 0.);
            assert_eq!(
                &buffer.pixels()[(36 * 64 + 34) * 4..(36 * 64 + 34) * 4 + 4],
                &expected
            );
        }
    }
}
