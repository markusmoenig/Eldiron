//! Complete ordered source-scene production on one injected device. Each view
//! owns its canvas/output; all CPU work describes geometry and immutable assets.
use super::{
    GpuMaterialSource, GpuTerrainPaintSource, IsometricMaterialSources, IsometricPaintSources,
    IsometricSourceVersion, IsometricSources, IsometricSurfaceRenderer, IsometricWorldTargets,
    SurfaceKind, SurfaceTriangle,
};
use renderkit::{GpuContext, RenderError, Result};
use rusterix::{
    Assets, BBox, Map, Sector, Tile,
    isometric::{
        AvatarSprite, Projection, SceneCache,
        lighting::{GpuLightingParameters, Lighting},
        materials::{MaterialSelection, SurfaceMaterials},
        scene::{ContactPolicy, ShadowView, Sprite},
    },
};
use scenevm::{
    isometric::{
        GpuSpriteSource, IsometricBlitter, IsometricCanvas, IsometricRenderer, Particle,
        SpriteBlit, SpriteContacts, SpriteEnvironment, SpriteTerrainClip,
    },
    render_output::SceneOutput,
};
use std::{collections::HashMap, sync::Arc};
mod frame_sources;
mod ground_cache;
mod terrain_chunks;
mod terrain_materials;
mod terrain_overview;
mod terrain_sources;
#[cfg(test)]
mod unlit_tests;
pub(crate) use frame_sources::IsometricFrameSources;
pub use ground_cache::GroundCacheUsage;
pub use terrain_chunks::IsometricTerrainCache;
pub(crate) use terrain_chunks::TerrainConsumer;

const DETAIL_TRIANGLE_BUDGET: usize = 96_000;

type TerrainKey = (theframework_core::Uuid, u32, IsometricSourceVersion);
type TerrainChunks = Option<Arc<terrain_chunks::GpuTerrainChunks>>;

pub struct IsometricSceneInput<'a> {
    pub map: &'a Map,
    pub assets: &'a Assets,
    pub avatars: &'a HashMap<usize, AvatarSprite>,
    pub seconds: f32,
    pub projection: Projection,
    pub lighting: &'a Lighting,
    pub particles: &'a [Particle],
    pub source_version: IsometricSourceVersion,
}
/// Immutable frame metadata and GPU source leases. Preparing a newer frame
/// never changes this frame's source identity or its painter/contact policy.
pub struct IsometricPreparedFrame {
    size: (u32, u32),
    projection: Projection,
    generation: u64,
    terrain: bool,
    terrain_draw: Option<terrain_chunks::TerrainDraw>,
    ground: Vec<Command>,
    upright: Vec<(usize, Command)>,
    contacts: ContactPolicy,
    lighting: GpuLightingParameters,
    particles: Vec<Particle>,
    geometry: Option<Arc<scenevm::isometric::ShadowGeometry>>,
    targets: Arc<IsometricWorldTargets>,
}
impl IsometricPreparedFrame {
    pub fn targets(&self) -> Arc<IsometricWorldTargets> {
        self.targets.clone()
    }
    pub fn size(&self) -> (u32, u32) {
        self.size
    }
    pub fn projection(&self) -> Projection {
        self.projection
    }
}
/// The application keeps one producer per independent view. No destination CPU
/// storage, window, runtime tick or implicit device initialization belongs here.
pub struct IsometricSceneRenderer {
    context: Arc<GpuContext>,
    scene: SceneCache,
    elevation: Option<(
        theframework_core::Uuid,
        u32,
        bool,
        rusterix::isometric::elevation::ElevationMap,
    )>,
    canvas: Option<IsometricCanvas>,
    output: IsometricRenderer,
    blitter: IsometricBlitter,
    surfaces: IsometricSurfaceRenderer,
    sprites: IsometricSources,
    materials: IsometricMaterialSources,
    overview_materials: IsometricMaterialSources,
    paints: IsometricPaintSources,
    ground_cache: ground_cache::GroundCache,
    terrain_chunks: Option<(TerrainKey, TerrainChunks)>,
    terrain_window: Option<(TerrainKey, BBox, TerrainChunks)>,
    terrain_rasterizer: Option<terrain_chunks::TerrainRasterizer>,
}
impl IsometricSceneRenderer {
    pub fn set_player_view(&mut self, player: rusterix::party::PlayerView) {
        self.scene.player_view = player;
    }
    /// Asset overviews preserve authored source colors, without live lighting,
    /// emission or shadows. Coverage, contacts and terrain still run on the GPU.
    pub fn prepare_unlit(
        &mut self,
        size: (u32, u32),
        input: &IsometricSceneInput<'_>,
    ) -> Result<IsometricPreparedFrame> {
        self.prepare_impl(size, input, true, false, None)
    }
    pub fn new(context: Arc<GpuContext>) -> Result<Self> {
        Ok(Self {
            output: IsometricRenderer::new(context.clone())?,
            blitter: IsometricBlitter::new(context.clone())?,
            surfaces: IsometricSurfaceRenderer::new(context.clone())?,
            sprites: IsometricSources::new(context.clone(), 64 * 1024 * 1024, 2048),
            materials: IsometricMaterialSources::new(context.clone(), 32 * 1024 * 1024, 1024),
            overview_materials: IsometricMaterialSources::new(context.clone(), 512 * 1024, 1024),
            paints: IsometricPaintSources::new(context.clone(), 32 * 1024 * 1024, 256),
            ground_cache: Default::default(),
            terrain_chunks: None,
            terrain_window: None,
            terrain_rasterizer: None,
            context,
            scene: SceneCache::default(),
            elevation: None,
            canvas: None,
        })
    }
    pub fn has_cpu_storage(&self) -> bool {
        self.output.has_cpu_storage()
    }
    pub fn has_readback_storage(&self) -> bool {
        self.output.has_readback_storage()
    }
    /// A view may show an ephemeral projection with the authored map identity
    /// and revision. Revoke its geometry/elevation without discarding immutable
    /// source textures or the device/target lifetime.
    pub(crate) fn invalidate_geometry(&mut self) {
        self.scene = SceneCache::default();
        self.elevation = None;
        self.ground_cache.clear();
        self.terrain_chunks = None;
        self.terrain_window = None;
    }
    pub fn ground_cache_usage(&self) -> GroundCacheUsage {
        self.ground_cache.usage()
    }
    pub fn source_builds(&self) -> [u64; 3] {
        [
            self.sprites.builds(),
            self.materials.builds(),
            self.paints.builds(),
        ]
    }
    pub fn render(
        &mut self,
        size: (u32, u32),
        input: &IsometricSceneInput<'_>,
        revision: u64,
    ) -> Result<SceneOutput> {
        let frame = self.prepare(size, input)?;
        self.render_prepared(&frame, revision)
    }
    pub fn prepare(
        &mut self,
        size: (u32, u32),
        input: &IsometricSceneInput<'_>,
    ) -> Result<IsometricPreparedFrame> {
        self.prepare_impl(size, input, false, true, None)
    }
    pub(crate) fn prepare_view(
        &mut self,
        size: (u32, u32),
        input: &IsometricSceneInput<'_>,
        consumer: TerrainConsumer,
        shared_terrain: Option<&IsometricTerrainCache>,
    ) -> Result<IsometricPreparedFrame> {
        let unlit = consumer == TerrainConsumer::Overview;
        self.prepare_impl(size, input, unlit, !unlit, Some((consumer, shared_terrain)))
    }
    fn prepare_impl(
        &mut self,
        size: (u32, u32),
        input: &IsometricSceneInput<'_>,
        mut unlit: bool,
        retain_terrain: bool,
        view: Option<(TerrainConsumer, Option<&IsometricTerrainCache>)>,
    ) -> Result<IsometricPreparedFrame> {
        let shared_terrain = view.and_then(|(consumer, cache)| cache.map(|cache| (cache, consumer)));
        let p = input.projection;
        let limits = self.context.device.limits();
        let pixels = u64::from(size.0) * u64::from(size.1);
        if size.0 == 0
            || size.1 == 0
            || size.0 > limits.max_texture_dimension_2d
            || size.1 > limits.max_texture_dimension_2d
            || pixels * 8 > u64::from(limits.max_storage_buffer_binding_size)
            || pixels * 8 > limits.max_buffer_size
        {
            return Err(RenderError("invalid isometric scene size".into()));
        }
        if self.context.is_lost()
            || !p.unit.is_finite()
            || p.unit <= 0.
            || !p.origin.x.is_finite()
            || !p.origin.y.is_finite()
        {
            return Err(RenderError(
                "invalid isometric scene projection or device".into(),
            ));
        }
        let plan = self
            .scene
            .prepare(input.map, input.assets, input.seconds, input.avatars);
        let elevation_key = (
            input.map.id,
            input.map.changed,
            input.map.camera == rusterix::MapCamera::TwoDIso,
        );
        if self
            .elevation
            .as_ref()
            .is_none_or(|e| (e.0, e.1, e.2) != elevation_key)
        {
            let mut elevation = rusterix::isometric::elevation::ElevationMap::from_map(input.map);
            elevation.terrain = plan.terrain.clone();
            self.elevation = Some((elevation_key.0, elevation_key.1, elevation_key.2, elevation));
        }
        let targets = IsometricWorldTargets::capture(
            input,
            size,
            plan.ground.iter().chain(&plan.upright),
            &mut self.sprites,
            self.elevation.as_ref().unwrap().3.clone(),
            view.is_some_and(|(consumer, _)| consumer == TerrainConsumer::Editor),
        )?;
        let terrain = plan.terrain.is_some();
        let mut ground = Vec::new();
        let mut terrain_draw = None;
        let mut overview = false;
        let mut swatches = HashMap::new();
        let use_chunks = (retain_terrain || shared_terrain.is_some())
            && terrain
            && !input.assets.terrain_paint.contains_key(&input.map.id);
        if !use_chunks {
            self.terrain_chunks = None;
            self.terrain_window = None;
        }
        if let Some(model) = &plan.terrain {
            let visible = p.visible_bounds(size, [0., model.max_height]);
            let wide = model.stride_for_triangle_budget(visible, DETAIL_TRIANGLE_BUDGET)
                .is_some_and(|stride| stride > 1);
            if use_chunks {
                let key = (input.map.id, input.map.changed, input.source_version);
                let chunks = if let Some((_, chunks)) =
                    self.terrain_chunks.as_ref().filter(|(previous, _)| *previous == key)
                {
                    chunks.clone()
                } else {
                    self.terrain_chunks = None;
                    let policy = SurfaceMaterials::for_terrain(input.map, input.assets);
                    let chunks = if let Some((cache, consumer)) = shared_terrain {
                        cache.prepare(&self.context, input.map, &policy, model, consumer)?
                    } else {
                        terrain_chunks::GpuTerrainChunks::prepare(&self.context, &policy, model)?
                    };
                    if retain_terrain {
                        self.terrain_chunks = Some((key, chunks.clone()));
                    }
                    chunks
                };
                // The retained GPU path has its own geometry and material
                // limits. The per-triangle fallback budget should not force
                // a normal editing view into the flat-color overview.
                let chunks = if chunks.is_none() && retain_terrain {
                    self.visible_terrain_chunks(key, visible, model, input)?
                } else {
                    chunks
                };
                if let Some(chunks) = chunks {
                    terrain_draw = Some(terrain_chunks::TerrainDraw {
                        chunks,
                        clip: visible,
                        phase: plan.material_frame,
                    });
                    self.ground_cache.clear();
                }
            }
            if terrain_draw.is_none() && wide {
                overview = true;
                unlit = true;
                ground.push(Command::Surface(self.prepare_terrain_overview(
                    input, &plan, model, visible, size, &mut swatches,
                )?));
            } else if terrain_draw.is_none() {
                let policy = SurfaceMaterials::for_terrain(input.map, input.assets);
                let mut triangles = Vec::new();
                model.triangles(visible, |t| triangles.push(t));
                let mut surfaces = Vec::new();
                let mut sources = terrain_sources::CellSources::default();
                for t in triangles {
                    let center = (t[0] + t[1] + t[2]) / 3.;
                    let cell = [center.x.floor() as i32, center.z.floor() as i32];
                    if let Some(surface) = sources.prepare(
                        self, input, &policy, plan.material_frame, cell,
                        t.map(|v| v.into_array()),
                    )? {
                        surfaces.push(surface);
                    }
                }
                ground.push(Command::Surface(surfaces));
            } else if terrain_draw.as_ref().is_some_and(|draw| draw.chunks.coarse) {
                // Keep the overview's authored roads and water visible over
                // the textured mesh. Layer zero belongs to the terrain itself.
                let mut floors: Vec<_> = plan
                    .floors
                    .iter()
                    .copied()
                    .filter(|(sector, _)| sector.layer.is_some_and(|layer| layer > 0))
                    .collect();
                floors.sort_by_key(|(sector, _)| sector.layer);
                let mut overlays = Vec::new();
                for (sector, tile) in floors {
                    overlays.extend(self.prepare_overview_floor(
                        input,
                        sector,
                        tile,
                        size,
                        &mut swatches,
                    )?);
                }
                if !overlays.is_empty() {
                    ground.push(Command::Surface(overlays));
                }
            }
        } else {
            for (sector, tile) in &plan.floors {
                ground.push(Command::Surface(self.prepare_floor(
                    input,
                    sector,
                    tile,
                    plan.material_frame,
                    size,
                )?));
            }
            for sprite in &plan.ground {
                if let Some(sprite) = self.prepare_sprite(sprite, input, size, true)? {
                    ground.push(Command::Sprite(sprite));
                }
            }
        }
        let contacts = ContactPolicy::new(input.map, &plan.upright);
        let mut upright = Vec::new();
        for (i, sprite) in plan.upright.iter().enumerate() {
            let command = if let (Some(sector), Some(tile)) = (sprite.floor, sprite.tile) {
                Command::Surface(if overview {
                    self.prepare_overview_floor(input, sector, tile, size, &mut swatches)?
                } else {
                    self.prepare_floor(input, sector, tile, plan.material_frame, size)?
                })
            } else if let Some(sprite) = self.prepare_sprite(sprite, input, size, false)? {
                Command::Sprite(sprite)
            } else {
                continue;
            };
            upright.push((i, command));
        }
        let geometry = if unlit {
            None
        } else {
            self.scene.shadow_geometry_in_view(
                input.map,
                input.assets,
                &plan,
                ShadowView {
                    projection: p,
                    size,
                    lighting: input.lighting,
                    max_storage_bytes: limits.max_buffer_size
                        .min(u64::from(limits.max_storage_buffer_binding_size)) as usize,
                },
            )
        };
        let particles = if overview { &[] } else { input.particles };
        let lighting = if unlit {
            // The unlit shader returns source RGBA before reading any lights,
            // shadow geometry or columns. Particles still use the projection.
            GpuLightingParameters {
                uniforms: scenevm::isometric::Uniforms {
                    projection: [p.origin.x, p.origin.y, p.unit, size.0 as f32],
                    counts: [size.0, size.1, 0, 0],
                    shadow: [0., 0., particles.len() as f32, -1.],
                    ..Default::default()
                },
                lights: Vec::new(),
                casters: Vec::new(),
                columns: Vec::new(),
            }
        } else {
            input.lighting.gpu_parameters(
                p,
                size,
                &plan.casters,
                geometry.is_some(),
                particles.len(),
            )
        };
        Ok(IsometricPreparedFrame {
            targets,
            size,
            projection: p,
            generation: self.context.generation(),
            terrain,
            terrain_draw,
            ground,
            upright,
            contacts,
            lighting,
            particles: particles.to_vec(),
            geometry,
        })
    }
    pub fn render_prepared(
        &mut self,
        frame: &IsometricPreparedFrame,
        revision: u64,
    ) -> Result<SceneOutput> {
        if self.context.is_lost() || frame.generation != self.context.generation() {
            return Err(RenderError(
                "isometric frame belongs to a lost or different GPU domain".into(),
            ));
        }
        if self.canvas.as_ref().is_none_or(|c| c.size() != frame.size) {
            self.canvas = Some(IsometricCanvas::new(
                &self.context,
                frame.size.0,
                frame.size.1,
            )?);
        }
        let p = frame.projection;
        let terrain = frame.terrain;
        let ground = &frame.ground;
        let upright = &frame.upright;
        let contacts = &frame.contacts;
        let lighting = &frame.lighting;
        let geometry = &frame.geometry;
        let canvas = self.canvas.as_ref().unwrap();
        if let Some(terrain) = &frame.terrain_draw {
            self.blitter.render(canvas, [17, 34, 29, 0], &[])?;
            if self.terrain_rasterizer.is_none() {
                self.terrain_rasterizer = Some(terrain_chunks::TerrainRasterizer::new(&self.context)?);
            }
            self.terrain_rasterizer.as_ref().unwrap().draw(&self.context, canvas, p, terrain);
            Self::execute(
                &mut self.blitter,
                &mut self.surfaces,
                canvas,
                p,
                ground.iter().map(|command| (command, SpriteContacts::default())),
                false,
            )?;
        } else if !self.ground_cache.restore(&self.context, canvas, frame)? {
            self.blitter.render(
                canvas,
                if terrain {
                    [17, 34, 29, 0]
                } else {
                    [17, 34, 29, 255]
                },
                &[],
            )?;
            Self::execute(
                &mut self.blitter,
                &mut self.surfaces,
                canvas,
                p,
                ground
                    .iter()
                    .map(|command| (command, SpriteContacts::default())),
                false,
            )?;
            self.ground_cache.store(&self.context, canvas, frame)?;
        }
        Self::execute(
            &mut self.blitter,
            &mut self.surfaces,
            canvas,
            p,
            upright.iter().map(|(i, command)| {
                (
                    command,
                    SpriteContacts {
                        owner: contacts.owner_by_instance[*i],
                        protected: &contacts.protected[*i],
                    },
                )
            }),
            terrain,
        )?;
        self.output.render(
            canvas,
            &lighting.uniforms,
            &lighting.lights,
            &lighting.casters,
            &lighting.columns,
            &frame.particles,
            geometry.as_deref(),
            revision,
        )
    }
    fn execute<'a>(
        blitter: &mut IsometricBlitter,
        surfaces: &mut IsometricSurfaceRenderer,
        canvas: &IsometricCanvas,
        projection: Projection,
        commands: impl IntoIterator<Item = (&'a Command, SpriteContacts<'a>)>,
        terrain: bool,
    ) -> Result<()> {
        let mut sprites = Vec::new();
        let mut environments = Vec::new();
        for (command, contacts) in commands {
            match command {
                Command::Surface(prepared) => {
                    // Keep painter order across floors. Consecutive sprites
                    // can share a submission and still execute in order.
                    if !sprites.is_empty() {
                        blitter.append_with_environment(canvas, &sprites, &environments)?;
                        sprites.clear();
                        environments.clear();
                    }
                    let draws = prepared
                        .iter()
                        .map(|t| SurfaceTriangle {
                            vertices: t.vertices,
                            source: &t.source,
                            secondary: t.secondary.as_ref().map(|(s, w)| (s.as_ref(), *w)),
                            paint: t.paint.as_deref(),
                            kind: t.kind,
                            repeat: t.repeat,
                            uv_bounds: t.bounds,
                            clip_cell: t.cell,
                            contacts,
                        })
                        .collect::<Vec<_>>();
                    surfaces.append(canvas, projection, &draws)?;
                }
                Command::Sprite(s) => {
                    sprites.push(SpriteBlit {
                        source: &s.source,
                        origin: s.origin,
                        pixel_scale: s.pixel_scale,
                        base_height: s.base_height,
                        world_scale: s.world_scale,
                        opacity: s.opacity,
                        bitmap: s.bitmap,
                        prelit: s.prelit,
                        ground: s.ground,
                        anchor_y: s.anchor_y,
                        pixels_per_unit: s.unit,
                    });
                    environments.push(SpriteEnvironment {
                        contacts,
                        terrain: terrain.then_some(s.terrain).flatten(),
                        flat_surface: s.flat,
                    });
                }
            }
        }
        if !sprites.is_empty() {
            blitter.append_with_environment(canvas, &sprites, &environments)?;
        }
        Ok(())
    }
    fn prepare_sprite(
        &mut self,
        sprite: &Sprite<'_>,
        input: &IsometricSceneInput<'_>,
        viewport: (u32, u32),
        flat: bool,
    ) -> Result<Option<PreparedSprite>> {
        let Some(image) = sprite.image() else {
            return Ok(None);
        };
        let (at, scale) = sprite.image_placement(&image, input.projection);
        let origin = at.into_array().map(|v| v.round() as i32);
        let extent = [image.width, image.height].map(|v| (v as f32 * scale).round() as i32);
        let viewport = [viewport.0 as i32, viewport.1 as i32];
        if (0..2).any(|axis| {
            extent[axis] <= 0
                || origin[axis] >= viewport[axis]
                || origin[axis].saturating_add(extent[axis]) <= 0
        }) {
            return Ok(None);
        }
        let Some(source) = self.sprites.prepare(sprite, input.source_version)? else {
            return Ok(None);
        };
        Ok(Some(PreparedSprite {
            source,
            origin: at.into_array(),
            pixel_scale: scale,
            base_height: sprite.height,
            world_scale: sprite.scale,
            opacity: sprite.opacity,
            bitmap: image.bitmap,
            prelit: image.prelit,
            ground: image.ground,
            anchor_y: image.anchor.y,
            unit: image.unit,
            flat,
            terrain: sprite
                .terrain_contact(input.projection)
                .map(|c| SpriteTerrainClip {
                    foot_y: c.foot_y,
                    foot_height: c.foot_height,
                    foot_depth: c.foot_depth,
                    unit: c.unit,
                }),
        }))
    }
    fn prepare_surface(
        &mut self,
        input: &IsometricSceneInput<'_>,
        selection: MaterialSelection<'_>,
        frame: usize,
        cell: [i32; 2],
        vertices: [[f32; 3]; 3],
        kind: SurfaceKind,
        repeat: bool,
        bounds: [f32; 4],
        clip: bool,
        painted: bool,
    ) -> Result<PreparedSurface> {
        let source = self
            .materials
            .prepare(selection.primary, frame, input.source_version)?;
        let secondary = selection
            .secondary
            .map(|(weights, tile)| {
                self.materials
                    .prepare(tile, frame, input.source_version)
                    .map(|s| (s, weights))
            })
            .transpose()?;
        let paint = if painted {
            input
                .assets
                .terrain_paint
                .get(&input.map.id)
                .map(|paint| {
                    self.paints
                        .prepare(paint, input.map.id, cell, input.source_version)
                })
                .transpose()?
                .flatten()
        } else {
            None
        };
        Ok(PreparedSurface {
            vertices,
            source,
            secondary,
            paint,
            kind,
            repeat,
            bounds,
            cell: clip.then_some(cell),
        })
    }
    fn prepare_floor(
        &mut self,
        input: &IsometricSceneInput<'_>,
        sector: &Sector,
        tile: &Tile,
        frame: usize,
        size: (u32, u32),
    ) -> Result<Vec<PreparedSurface>> {
        let Some((points, indices)) = sector.generate_geometry(input.map) else {
            return Ok(Vec::new());
        };
        let height = sector.properties.get_float_default("floor_height", 0.);
        let bounds = sector.bounding_box(input.map);
        let visible = input.projection.visible_bounds(size, [height; 2]);
        let min = bounds
            .min
            .map2(visible.min, f32::max)
            .map(|v| v.floor() as i32);
        let max = bounds
            .max
            .map2(visible.max, f32::min)
            .map(|v| v.ceil() as i32);
        let repeat = sector.properties.get_int_default("tile_mode", 1) != 0;
        let policy = SurfaceMaterials::for_floor(input.map, sector, tile, input.assets);
        let mut prepared = Vec::new();
        for z in min.y..max.y {
            for x in min.x..max.x {
                let Some(selection) = policy.cell((x, z)) else {
                    continue;
                };
                for &(a, b, c) in &indices {
                    prepared.push(self.prepare_surface(
                        input,
                        selection,
                        frame,
                        [x, z],
                        [a, b, c].map(|i| [points[i][0], height, points[i][1]]),
                        SurfaceKind::Floor,
                        repeat,
                        [bounds.min.x, bounds.min.y, bounds.max.x, bounds.max.y],
                        true,
                        height.abs() < 0.001,
                    )?);
                }
            }
        }
        Ok(prepared)
    }
}
enum Command {
    Surface(Vec<PreparedSurface>),
    Sprite(PreparedSprite),
}
#[derive(Clone)]
struct PreparedSurface {
    vertices: [[f32; 3]; 3],
    source: Arc<GpuMaterialSource>,
    secondary: Option<(Arc<GpuMaterialSource>, [f32; 4])>,
    paint: Option<Arc<GpuTerrainPaintSource>>,
    kind: SurfaceKind,
    repeat: bool,
    bounds: [f32; 4],
    cell: Option<[i32; 2]>,
}
struct PreparedSprite {
    source: Arc<GpuSpriteSource>,
    origin: [f32; 2],
    pixel_scale: f32,
    base_height: f32,
    world_scale: f32,
    opacity: f32,
    bitmap: bool,
    prelit: bool,
    ground: bool,
    anchor_y: f32,
    unit: f32,
    flat: bool,
    terrain: Option<SpriteTerrainClip>,
}
