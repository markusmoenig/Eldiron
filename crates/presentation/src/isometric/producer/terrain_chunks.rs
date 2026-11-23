//! Retained world-space bins for the normal isometric terrain pass. Each pixel
//! visits one bin and evaluates its triangles in the reference painter order.
use super::{SurfaceMaterials, terrain_materials::*};
use bytemuck::{Pod, Zeroable};
use renderkit::{GpuContext, Result, WorkingBufferDescriptor, WorkingBufferLease, wgpu};
use rustc_hash::{FxHashMap, FxHasher};
use rusterix::{BBox, Map, chunkbuilder::terrain_surface::TerrainSurface};
use scenevm::{
    isometric::IsometricCanvas,
    prelude::{Vec2, Vec3},
};
use std::{
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};
use theframework_core::Uuid;

const BINS_PER_UNIT: f32 = 4.;
const BIN_MARGIN: f32 = 1. / 256.;
const MAX_TRIANGLES: usize = 320_000;
const MAX_BINS: usize = 1_000_000;
const MAX_INDICES: usize = 5_000_000;

type Word = [u32; 4];
type Triangle = [Word; 5];

impl super::IsometricSceneRenderer {
    pub(super) fn visible_terrain_chunks(
        &mut self,
        key: super::TerrainKey,
        clip: BBox,
        terrain: &TerrainSurface,
        input: &super::IsometricSceneInput<'_>,
    ) -> Result<Option<Arc<GpuTerrainChunks>>> {
        const WINDOW_STEP: f32 = 8.;
        let bounds = BBox::new(
            (clip.min / WINDOW_STEP).map(f32::floor) * WINDOW_STEP,
            (clip.max / WINDOW_STEP).map(f32::ceil) * WINDOW_STEP,
        );
        let coarse = terrain
            .stride_for_triangle_budget(bounds, MAX_TRIANGLES)
            .is_some_and(|stride| stride > 1);
        if let Some((prior, cached_bounds, chunks)) = &self.terrain_window
            && *prior == key
            && (*cached_bounds == bounds
                || chunks.as_ref().is_some_and(|chunks| !chunks.coarse || coarse)
                    && cached_bounds.contains(clip.min)
                    && cached_bounds.contains(clip.max))
        {
            return Ok(chunks.clone());
        }
        let policy = SurfaceMaterials::for_terrain(input.map, input.assets);
        let mut chunks =
            GpuTerrainChunks::prepare_in_bounds(&self.context, &policy, terrain, bounds, coarse)?;
        if chunks.is_none() && !coarse && terrain.subdivisions > 1 {
            chunks =
                GpuTerrainChunks::prepare_in_bounds(&self.context, &policy, terrain, bounds, true)?;
        }
        self.terrain_window = Some((key, bounds, chunks.clone()));
        Ok(chunks)
    }
}

/// One immutable terrain publication for the authoring view, overview and Play.
/// Cameras, lighting, outputs and render receipts remain local to each view.
#[derive(Clone, Default)]
pub struct IsometricTerrainCache(Arc<Mutex<SharedTerrain>>);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TerrainConsumer {
    Editor,
    Overview,
    Play,
}

#[derive(Default)]
struct SharedTerrain {
    scope: Option<(Uuid, Uuid, u64)>,
    entry: Option<SharedEntry>,
}

struct SharedEntry {
    map: Map,
    materials: u64,
    chunks: Option<Arc<GpuTerrainChunks>>,
}

impl IsometricTerrainCache {
    /// Bind the publication to the current document, map and GPU generation.
    pub fn set_scope(&self, scope: Option<(Uuid, Uuid, u64)>) {
        let mut cache = self.0.lock().unwrap();
        if cache.scope != scope {
            cache.scope = scope;
            cache.entry = None;
        }
    }

    pub(super) fn prepare(
        &self,
        gpu: &GpuContext,
        map: &Map,
        policy: &SurfaceMaterials<'_>,
        model: &TerrainSurface,
        consumer: TerrainConsumer,
    ) -> Result<Option<Arc<GpuTerrainChunks>>> {
        let mut cache = self.0.lock().unwrap();
        if !cache.scope.is_some_and(|(_, id, generation)| {
            id == map.id && generation == gpu.generation()
        }) {
            drop(cache);
            return if consumer != TerrainConsumer::Overview {
                GpuTerrainChunks::prepare(gpu, policy, model)
            } else {
                Ok(None)
            };
        }
        let materials = policy.source_fingerprint();
        if let Some(entry) = &cache.entry
            && entry.materials == materials
            && (entry.map.same_overview_source(map)
                // Runtime sectors can carry gameplay data that the terrain
                // pass never reads. Verify the actual facets in that case.
                || consumer == TerrainConsumer::Play
                    && entry.chunks.as_ref().is_some_and(|chunks| chunks.matches_geometry(model)))
        {
            #[cfg(not(target_arch = "wasm32"))]
            if entry.chunks.is_some() && std::env::var_os("ELDIRON_PROFILE_ISOMETRIC").is_some() {
                let view = match consumer {
                    TerrainConsumer::Editor => "editor",
                    TerrainConsumer::Overview => "overview",
                    TerrainConsumer::Play => "Play",
                };
                eprintln!("Shared isometric terrain reused by {view}");
            }
            return Ok(entry.chunks.clone());
        }
        match consumer {
            TerrainConsumer::Overview => return Ok(None),
            // A runtime terrain change belongs to this session. Keep the
            // authored publication available to the editor and minimap.
            TerrainConsumer::Play => {
                drop(cache);
                return GpuTerrainChunks::prepare(gpu, policy, model);
            }
            TerrainConsumer::Editor => {}
        }
        cache.entry = None;
        let chunks = GpuTerrainChunks::prepare(gpu, policy, model)?;
        cache.entry = Some(SharedEntry {
            map: map.clone(),
            materials,
            chunks: chunks.clone(),
        });
        Ok(chunks)
    }
}

pub(super) struct GpuTerrainChunks {
    geometry: WorkingBufferLease,
    materials: [WorkingBufferLease; 2],
    subdivisions: f32,
    pub(super) coarse: bool,
    model_bounds: BBox,
    geometry_source: ([u32; 6], u64),
}

fn geometry_layout(terrain: &TerrainSurface) -> [u32; 6] {
    [
        terrain.bounds.min.x.to_bits(),
        terrain.bounds.min.y.to_bits(),
        terrain.bounds.max.x.to_bits(),
        terrain.bounds.max.y.to_bits(),
        terrain.max_height.to_bits(),
        terrain.subdivisions,
    ]
}

fn hash_triangle(triangle: [Vec3<f32>; 3], hash: &mut FxHasher) {
    triangle.map(|vertex| vertex.into_array().map(f32::to_bits)).hash(hash);
}

impl GpuTerrainChunks {
    fn matches_geometry(&self, terrain: &TerrainSurface) -> bool {
        if geometry_layout(terrain) != self.geometry_source.0 {
            return false;
        }
        let mut hash = FxHasher::default();
        terrain.triangles(terrain.bounds, |triangle| hash_triangle(triangle, &mut hash));
        hash.finish() == self.geometry_source.1
    }

    pub(super) fn prepare<'a>(
        gpu: &GpuContext,
        policy: &SurfaceMaterials<'a>,
        terrain: &TerrainSurface,
    ) -> Result<Option<Arc<Self>>> {
        Self::prepare_in_bounds(gpu, policy, terrain, terrain.bounds, false)
    }

    fn prepare_in_bounds<'a>(
        gpu: &GpuContext,
        policy: &SurfaceMaterials<'a>,
        terrain: &TerrainSurface,
        clip: BBox,
        coarse: bool,
    ) -> Result<Option<Arc<Self>>> {
        let subdivisions = terrain.subdivisions as f32;
        let bounds = BBox::new(
            (terrain.bounds.min.map2(clip.min, f32::max) * subdivisions).floor() / subdivisions,
            (terrain.bounds.max.map2(clip.max, f32::min) * subdivisions).ceil() / subdivisions,
        );
        let mesh_resolution = if coarse { 1. } else { subdivisions };
        let extent = (bounds.max * mesh_resolution).ceil() - (bounds.min * mesh_resolution).floor();
        if !extent.into_array().iter().all(|v| v.is_finite() && *v > 0.)
            || extent.x as f64 * extent.y as f64 * 2. > MAX_TRIANGLES as f64
            || bounds
                .min
                .map(f32::abs)
                .map2(bounds.max.map(f32::abs), f32::max)
                .into_array()
                .into_iter()
                .any(|value| value > 4096.)
            || !terrain.max_height.is_finite()
            || terrain.max_height > 4096.
        {
            return Ok(None);
        }
        let mut addresses = FxHashMap::default();
        let mut cell_sources = FxHashMap::default();
        let mut materials = Vec::new();
        let mut triangles = Vec::<Triangle>::new();
        let mut ranges = Vec::new();
        let mut min = Vec2::broadcast(i32::MAX);
        let mut max = Vec2::broadcast(i32::MIN);
        let mut unsupported = false;
        let mut geometry_hash = FxHasher::default();
        let bin_scale = BINS_PER_UNIT / (subdivisions / mesh_resolution);
        let emit_triangle = |vertices: [Vec3<f32>; 3]| {
            if unsupported
                || vertices
                    .iter()
                    .any(|vertex| !vertex.y.is_finite() || vertex.y.abs() > 4096.)
            {
                unsupported = true;
                return;
            }
            hash_triangle(vertices, &mut geometry_hash);
            let center = (vertices[0] + vertices[1] + vertices[2]) / 3.;
            let cell = [center.x.floor() as i32, center.z.floor() as i32];
            let Some((primary, secondary, blend)) =
                *cell_sources.entry(cell).or_insert_with(|| {
                    let selection = policy.cell((cell[0], cell[1]))?;
                    let mut material = |tile: &'a rusterix::Tile| {
                        let id = *addresses.entry(tile.id).or_insert_with(|| {
                            let id = materials.len();
                            materials.push(TerrainMaterial::new(tile));
                            id
                        });
                        materials[id].include_cell(cell);
                        id as u32
                    };
                    let primary = material(selection.primary);
                    let (secondary, blend) = selection
                        .secondary
                        .map_or((NO_MATERIAL, [0; 4]), |(weights, tile)| {
                            (material(tile), weights.map(f32::to_bits))
                        });
                    Some((primary, secondary, blend))
                })
            else {
                return;
            };
            let grid_cell = [
                vertices.iter().map(|v| v.x).fold(f32::INFINITY, f32::min),
                vertices.iter().map(|v| v.z).fold(f32::INFINITY, f32::min),
            ]
            .map(|v| (v * subdivisions).round() as i32 as u32);
            let normal = super::super::surface_renderer::terrain_normal(vertices);
            let mut triangle = [[0; 4]; 5];
            for (i, vertex) in vertices.iter().enumerate() {
                triangle[i][..3].copy_from_slice(&vertex.into_array().map(f32::to_bits));
                triangle[i][3] = normal[i].to_bits();
            }
            triangle[3] = blend;
            triangle[4] = [grid_cell[0], grid_cell[1], primary, secondary];
            triangles.push(triangle);

            let unit_projection =
                |v: Vec3<f32>| Vec2::new(v.x + v.z, (v.z - v.x) * 0.5 - v.y * 1.2247449);
            let projected = vertices.map(unit_projection);
            let low = projected[0]
                .map2(projected[1], f32::min)
                .map2(projected[2], f32::min);
            let high = projected[0]
                .map2(projected[1], f32::max)
                .map2(projected[2], f32::max);
            let low = ((low - BIN_MARGIN) * bin_scale).floor().as_::<i32>();
            let high = ((high + BIN_MARGIN) * bin_scale).floor().as_::<i32>();
            min = min.map2(low, i32::min);
            max = max.map2(high, i32::max);
            ranges.push((low, high));
        };
        if coarse {
            terrain.material_cell_triangles(bounds, emit_triangle);
        } else {
            terrain.triangles(bounds, emit_triangle);
        }
        if unsupported || triangles.is_empty() {
            return Ok(None);
        }
        let Some((mut packet, index_base)) = index_bins(&ranges, min, max) else {
            return Ok(None);
        };
        let triangle_base = packet.len();
        let limit = u64::from(gpu.device.limits().max_storage_buffer_binding_size)
            .min(gpu.device.limits().max_buffer_size);
        if (triangle_base + triangles.len() * 5) as u64 * 16 > limit {
            return Ok(None);
        }
        let Some(atlas) = TerrainAtlas::prepare(&materials, limit) else {
            if cfg!(not(target_arch = "wasm32"))
                && std::env::var_os("ELDIRON_PROFILE_ISOMETRIC").is_some()
            {
                eprintln!("Retained isometric terrain material pages exceed admission limits");
            }
            return Ok(None);
        };
        for triangle in &mut triangles {
            let meta = &mut triangle[4];
            meta[2] = atlas.addresses[meta[2] as usize];
            if meta[3] != NO_MATERIAL {
                meta[3] = atlas.addresses[meta[3] as usize];
            }
        }
        packet[1] = [index_base as u32, triangle_base as u32, 0, 0];
        packet.extend(triangles.into_iter().flatten());

        let upload = |words: Vec<Word>| -> Result<WorkingBufferLease> {
            let bytes = bytemuck::cast_slice(&words);
            let buffer =
                gpu.acquire_working_buffer(WorkingBufferDescriptor::storage(bytes.len() as u64))?;
            gpu.queue.write_buffer(buffer.buffer(), 0, bytes);
            gpu.record_upload(bytes.len() as u64);
            Ok(buffer)
        };
        let [first, second] = atlas.banks;
        if cfg!(not(target_arch = "wasm32"))
            && std::env::var_os("ELDIRON_PROFILE_ISOMETRIC").is_some()
        {
            eprintln!(
                "Retained isometric terrain uploaded {} geometry and {} material bytes",
                packet.len() * 16,
                (first.len() + second.len()) * 16
            );
        }
        Ok(Some(Arc::new(Self {
            geometry: upload(packet)?,
            materials: [upload(first)?, upload(second)?],
            subdivisions,
            coarse,
            model_bounds: bounds,
            geometry_source: (geometry_layout(terrain), geometry_hash.finish()),
        })))
    }

    fn grid_bounds(&self, clip: BBox) -> [i32; 4] {
        let low = self.model_bounds.min.map2(clip.min, f32::max);
        let low = if self.coarse { low.floor() } else { low };
        let min = (low * self.subdivisions)
            .floor()
            .as_::<i32>();
        let max = (self.model_bounds.max.map2(clip.max, f32::min) * self.subdivisions)
            .ceil()
            .as_::<i32>();
        [min.x, min.y, max.x, max.y]
    }

    fn bins_per_unit(&self) -> f32 {
        if self.coarse {
            BINS_PER_UNIT / self.subdivisions
        } else {
            BINS_PER_UNIT
        }
    }
}

/// Each pair in the directory points into a compact rectangle-independent
/// triangle list. Filling in source order preserves depth and alpha ties.
fn index_bins(
    ranges: &[(Vec2<i32>, Vec2<i32>)],
    min: Vec2<i32>,
    max: Vec2<i32>,
) -> Option<(Vec<Word>, usize)> {
    let size = (max - min + 1).as_::<usize>();
    let count = size.x.checked_mul(size.y)?;
    if count > MAX_BINS {
        return None;
    }
    let mut lengths = vec![0u32; count];
    let bins = |(low, high): (Vec2<i32>, Vec2<i32>)| {
        let low = (low - min).as_::<usize>();
        let high = (high - min).as_::<usize>();
        (low.y..=high.y).flat_map(move |row| row * size.x + low.x..=row * size.x + high.x)
    };
    let mut total = 0;
    for &range in ranges {
        for bin in bins(range) {
            lengths[bin] += 1;
            total += 1;
            if total > MAX_INDICES {
                return None;
            }
        }
    }
    let index_base = 2 + count.div_ceil(2);
    let mut packet = vec![[0u32; 4]; index_base + total.div_ceil(4)];
    packet[0] = [min.x as u32, min.y as u32, size.x as u32, size.y as u32];
    let mut positions = Vec::with_capacity(count);
    let mut start = 0u32;
    for (bin, length) in lengths.into_iter().enumerate() {
        let offset = bin % 2 * 2;
        packet[2 + bin / 2][offset..offset + 2].copy_from_slice(&[start, length]);
        positions.push(start as usize);
        start += length;
    }
    for (triangle, range) in ranges.iter().copied().enumerate() {
        for bin in bins(range) {
            let position = &mut positions[bin];
            packet[index_base + *position / 4][*position % 4] = triangle as u32;
            *position += 1;
        }
    }
    Some((packet, index_base))
}

pub(super) struct TerrainDraw {
    pub(super) chunks: Arc<GpuTerrainChunks>,
    pub(super) clip: BBox,
    pub(super) phase: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniform {
    projection: [f32; 4],
    canvas: [u32; 4],
    grid_bounds: [i32; 4],
}

pub(super) struct TerrainRasterizer {
    pipeline: wgpu::ComputePipeline,
    uniform: WorkingBufferLease,
}

impl TerrainRasterizer {
    pub(super) fn new(gpu: &GpuContext) -> Result<Self> {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("retained-isometric-terrain"),
                source: wgpu::ShaderSource::Wgsl(
                    concat!(
                        include_str!("terrain_chunks.wgsl"),
                        include_str!("../surface_math.wgsl"),
                        include_str!("../material_sampling.wgsl"),
                    )
                    .into(),
                ),
            });
        let pipeline = gpu
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("retained-isometric-terrain"),
                layout: None,
                module: &shader,
                entry_point: Some("raster"),
                compilation_options: Default::default(),
                cache: None,
            });
        let uniform = gpu.acquire_working_buffer(WorkingBufferDescriptor {
            bytes: std::mem::size_of::<Uniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })?;
        Ok(Self { pipeline, uniform })
    }

    pub(super) fn draw(
        &self,
        gpu: &GpuContext,
        canvas: &IsometricCanvas,
        projection: rusterix::isometric::Projection,
        draw: &TerrainDraw,
    ) {
        let (width, height) = canvas.size();
        let uniform = Uniform {
            projection: [
                projection.origin.x,
                projection.origin.y,
                projection.unit,
                draw.chunks.bins_per_unit(),
            ],
            canvas: [width, height, draw.phase as u32, 0],
            grid_bounds: draw.chunks.grid_bounds(draw.clip),
        };
        let bytes = bytemuck::bytes_of(&uniform);
        gpu.queue.write_buffer(self.uniform.buffer(), 0, bytes);
        gpu.record_upload(bytes.len() as u64);
        let buffers = [
            self.uniform.buffer(),
            canvas.albedo(),
            canvas.surfaces(),
            canvas.owners(),
            canvas.terrain(),
            draw.chunks.geometry.buffer(),
            draw.chunks.materials[0].buffer(),
            draw.chunks.materials[1].buffer(),
        ];
        let entries = buffers
            .iter()
            .enumerate()
            .map(|(index, buffer)| wgpu::BindGroupEntry {
                binding: index as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect::<Vec<_>>();
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("retained-isometric-terrain"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let mut encoder = renderkit::OrderedCompute::new(gpu, "retained-isometric-terrain");
        {
            let mut pass = encoder.begin_pass();
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }
        encoder.finish();
    }
}
