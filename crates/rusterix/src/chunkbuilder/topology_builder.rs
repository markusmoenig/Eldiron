use crate::chunkbuilder::action::{
    ConnectionMode, ControlPoint, MeshTopology, SectorMeshDescriptor,
};
use crate::chunkbuilder::d3chunkbuilder::{D3ChunkBuilder, DEFAULT_TILE_ID};
use crate::chunkbuilder::surface_mesh_builder::SurfaceMeshBuilder;
use crate::collision_world::ChunkCollision;
use crate::{
    Assets, BBox, Batch3D, Chunk, ChunkBuilder, GeometrySource, Map, MapTopology, PixelSource,
    RepeatMode, Value,
};
use buildergraph::{
    BuilderCutMask, BuilderCutMode, BuilderCutShape, BuilderDetailPlacement, BuilderDocument,
    BuilderHost, BuilderMasonryPattern, BuilderSurfaceDetail,
};
use rustc_hash::{FxHashMap, FxHashSet};
use scenevm::GeoId;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use uuid::Uuid;
use vek::{Vec2, Vec3};

/// Topology-backed 3D chunk builder entry point.
///
/// This is intentionally separate from `D3ChunkBuilder`. The old builder stays
/// in the tree as the known-good reference while sector/surface/terrain
/// generation is moved to a retained topology scene incrementally.
#[derive(Clone)]
pub struct TopologyBuilder {
    reference_builder: D3ChunkBuilder,
    cached_scene: Option<(Uuid, u32, u64, TopologyScene)>,
}

#[derive(Clone, Debug)]
pub struct TopologySector {
    pub id: u32,
    pub bbox: BBox,
    pub surfaces: FxHashSet<Uuid>,
}

#[derive(Clone, Debug)]
pub struct TopologyScene {
    pub topology: MapTopology,
    pub sectors: FxHashMap<u32, TopologySector>,
    pub builder_cuts: Vec<TopologyBuilderCut>,
    pub builder_details: Vec<TopologyBuilderDetail>,
    pub builder_wall_details: Vec<TopologyBuilderWallDetail>,
}

#[derive(Clone, Debug)]
pub struct TopologyBuilderCut {
    pub sector_id: u32,
    pub surface_id: Option<Uuid>,
    pub host_min_uv: Vec2<f32>,
    pub host_size_uv: Vec2<f32>,
    pub mask: BuilderCutMask,
}

#[derive(Clone, Debug)]
pub struct TopologyBuilderDetail {
    pub sector_id: u32,
    pub surface_id: Option<Uuid>,
    pub host_min_uv: Vec2<f32>,
    pub detail: BuilderSurfaceDetail,
}

#[derive(Clone, Debug)]
pub struct TopologyBuilderWallDetail {
    pub linedef_id: u32,
    pub length: f32,
    pub wall_height: f32,
    pub wall_width: f32,
    pub origin: Vec3<f32>,
    pub along: Vec3<f32>,
    pub up: Vec3<f32>,
    pub outward: Vec3<f32>,
    pub detail: BuilderSurfaceDetail,
}

#[derive(Clone, Copy, Debug)]
struct TopologyBuilderLinedefMount {
    outward: Vec3<f32>,
    up: Vec3<f32>,
    wall_height: f32,
    wall_width: f32,
    host_origin: Vec3<f32>,
}

impl TopologyBuilderWallDetail {
    fn wall_detail_radius(&self) -> f32 {
        match &self.detail {
            BuilderSurfaceDetail::Rect {
                min, max, offset, ..
            } => (max.x - min.x).abs().max((max.y - min.y).abs()) + offset.abs(),
            BuilderSurfaceDetail::Column {
                radius,
                offset,
                base_height,
                cap_height,
                ..
            } => radius * 2.0 + offset.abs() + base_height.max(*cap_height).max(0.0),
            BuilderSurfaceDetail::Masonry {
                min, max, offset, ..
            } => (max.x - min.x).abs().max((max.y - min.y).abs()) + offset.abs(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TopologyResolvedBuilderCut {
    pub sector_id: u32,
    pub surface_id: Option<Uuid>,
    pub mode: BuilderCutMode,
    pub offset: f32,
    pub inset: f32,
    pub shape: BuilderCutShape,
    pub loop_uv: Vec<Vec2<f32>>,
}

#[derive(Clone, Debug)]
pub struct TopologySurfacePatch {
    pub sector_id: u32,
    pub surface_id: Uuid,
    pub descriptor: SectorMeshDescriptor,
    pub material_slot: Option<String>,
    pub tile_alias: Option<String>,
}

impl TopologyBuilderCut {
    pub fn resolved(&self) -> Option<TopologyResolvedBuilderCut> {
        match &self.mask {
            BuilderCutMask::Rect {
                min,
                max,
                mode,
                offset,
                inset,
                shape,
            } => {
                if max.x <= min.x || max.y <= min.y {
                    return None;
                }
                let min_uv = self.host_min_uv + *min;
                let max_uv = self.host_min_uv + *max;
                Some(TopologyResolvedBuilderCut {
                    sector_id: self.sector_id,
                    surface_id: self.surface_id,
                    mode: *mode,
                    offset: *offset,
                    inset: *inset,
                    shape: *shape,
                    loop_uv: vec![
                        Vec2::new(min_uv.x, min_uv.y),
                        Vec2::new(max_uv.x, min_uv.y),
                        Vec2::new(max_uv.x, max_uv.y),
                        Vec2::new(min_uv.x, max_uv.y),
                    ],
                })
            }
            BuilderCutMask::Loop {
                points,
                mode,
                offset,
                inset,
                shape,
            } => {
                if points.len() < 3 {
                    return None;
                }
                Some(TopologyResolvedBuilderCut {
                    sector_id: self.sector_id,
                    surface_id: self.surface_id,
                    mode: *mode,
                    offset: *offset,
                    inset: *inset,
                    shape: *shape,
                    loop_uv: points
                        .iter()
                        .map(|point| self.host_min_uv + *point)
                        .collect(),
                })
            }
        }
    }
}

fn inset_loop(points: &[Vec2<f32>], inset: f32) -> Vec<Vec2<f32>> {
    if inset <= 0.0 || points.len() < 3 {
        return points.to_vec();
    }

    let mut center = Vec2::zero();
    for point in points {
        center += *point;
    }
    center /= points.len() as f32;

    points
        .iter()
        .map(|point| {
            let delta = *point - center;
            let len = delta.magnitude();
            if len <= inset || len <= 0.0001 {
                center
            } else {
                center + delta * ((len - inset) / len)
            }
        })
        .collect()
}

fn rounded_column_loop(
    center: Vec2<f32>,
    height: f32,
    radius: f32,
    segments: usize,
) -> Vec<Vec2<f32>> {
    let segments = segments.max(4);
    let bottom = center.y + radius;
    let top = center.y + height - radius;
    if top <= bottom {
        return regular_polygon(center + Vec2::new(0.0, height * 0.5), radius, segments * 2);
    }

    let mut points = Vec::with_capacity(segments * 2 + 2);
    for index in 0..=segments {
        let t = index as f32 / segments as f32;
        let angle = std::f32::consts::PI * (1.0 - t);
        points.push(Vec2::new(
            center.x + angle.cos() * radius,
            top + angle.sin() * radius,
        ));
    }
    for index in 0..=segments {
        let t = index as f32 / segments as f32;
        let angle = std::f32::consts::PI * (2.0 - t);
        points.push(Vec2::new(
            center.x + angle.cos() * radius,
            bottom + angle.sin() * radius,
        ));
    }
    points
}

fn regular_polygon(center: Vec2<f32>, radius: f32, segments: usize) -> Vec<Vec2<f32>> {
    let segments = segments.max(3);
    (0..segments)
        .map(|index| {
            let angle = std::f32::consts::TAU * index as f32 / segments as f32;
            Vec2::new(
                center.x + angle.cos() * radius,
                center.y + angle.sin() * radius,
            )
        })
        .collect()
}

impl TopologyScene {
    const TERRAIN_CHUNK_PAD: f32 = 12.0;

    pub fn build(map: &Map) -> Self {
        let topology = MapTopology::build(map);
        let mut sectors = FxHashMap::default();

        for sector in &map.sectors {
            let bbox = sector.bounding_box(map);
            if !bbox.min.x.is_finite()
                || !bbox.min.y.is_finite()
                || !bbox.max.x.is_finite()
                || !bbox.max.y.is_finite()
            {
                continue;
            }

            sectors.insert(
                sector.id,
                TopologySector {
                    id: sector.id,
                    bbox,
                    surfaces: topology
                        .sector_to_surfaces
                        .get(&sector.id)
                        .cloned()
                        .unwrap_or_default(),
                },
            );
        }

        let builder_cuts = Self::collect_builder_cuts(map);
        let builder_details = Self::collect_builder_details(map);
        let builder_wall_details = Self::collect_builder_wall_details(map);

        Self {
            topology,
            sectors,
            builder_cuts,
            builder_details,
            builder_wall_details,
        }
    }

    fn collect_builder_cuts(map: &Map) -> Vec<TopologyBuilderCut> {
        let mut out = Vec::new();

        for sector in &map.sectors {
            let builder_graph_data = sector
                .properties
                .get_str_default("builder_graph_data", String::new());
            if builder_graph_data.trim().is_empty() {
                continue;
            }

            let Ok(document) = BuilderDocument::from_text(&builder_graph_data) else {
                eprintln!(
                    "[BuilderGraphDebug][collect] sector={} failed to parse builder graph",
                    sector.id
                );
                continue;
            };

            let (host, surface_id, host_min_uv, host_size_uv) =
                Self::builder_host_for_sector_surface(map, sector.id).unwrap_or_else(|| {
                    let bbox = sector.bounding_box(map);
                    let size = Vec2::new(
                        (bbox.max.x - bbox.min.x).abs().max(0.01),
                        (bbox.max.y - bbox.min.y).abs().max(0.01),
                    );
                    (
                        BuilderHost::preview_floor(size.x, size.y),
                        None,
                        Vec2::zero(),
                        size,
                    )
                });

            let Ok(assembly) = document.evaluate_with_host(&host) else {
                eprintln!(
                    "[BuilderGraphDebug][collect] sector={} graph='{}' failed to evaluate with {} host",
                    sector.id,
                    document.name(),
                    host.kind_name()
                );
                continue;
            };

            eprintln!(
                "[BuilderGraphDebug][collect] sector={} graph='{}' host={} surface={:?} host_min=({}, {}) host_size=({}, {}) cuts={} primitives={} warnings={}",
                sector.id,
                document.name(),
                host.kind_name(),
                surface_id,
                host_min_uv.x,
                host_min_uv.y,
                host_size_uv.x,
                host_size_uv.y,
                assembly.cuts.len(),
                assembly.primitives.len(),
                assembly.warnings.len()
            );
            for warning in &assembly.warnings {
                eprintln!(
                    "[BuilderGraphDebug][collect] sector={} warning[{}]: {}",
                    sector.id, warning.code, warning.message
                );
            }

            for mask in assembly.cuts {
                eprintln!(
                    "[BuilderGraphDebug][collect] sector={} cut={:?}",
                    sector.id, mask
                );
                out.push(TopologyBuilderCut {
                    sector_id: sector.id,
                    surface_id,
                    host_min_uv,
                    host_size_uv,
                    mask,
                });
            }
        }

        out
    }

    fn collect_builder_details(map: &Map) -> Vec<TopologyBuilderDetail> {
        let mut out = Vec::new();

        for sector in &map.sectors {
            let builder_graph_data = sector
                .properties
                .get_str_default("builder_graph_data", String::new());
            if builder_graph_data.trim().is_empty() {
                continue;
            }

            let Ok(document) = BuilderDocument::from_text(&builder_graph_data) else {
                continue;
            };

            let (host, surface_id, host_min_uv, _host_size_uv) =
                Self::builder_host_for_sector_surface(map, sector.id).unwrap_or_else(|| {
                    let bbox = sector.bounding_box(map);
                    let size = Vec2::new(
                        (bbox.max.x - bbox.min.x).abs().max(0.01),
                        (bbox.max.y - bbox.min.y).abs().max(0.01),
                    );
                    (
                        BuilderHost::preview_floor(size.x, size.y),
                        None,
                        Vec2::zero(),
                        size,
                    )
                });

            let Ok(assembly) = document.evaluate_with_host(&host) else {
                continue;
            };

            for detail in assembly.surface_details {
                eprintln!(
                    "[BuilderGraphDebug][collect] sector={} detail={:?}",
                    sector.id, detail
                );
                out.push(TopologyBuilderDetail {
                    sector_id: sector.id,
                    surface_id,
                    host_min_uv,
                    detail,
                });
            }
        }

        out
    }

    fn collect_builder_wall_details(map: &Map) -> Vec<TopologyBuilderWallDetail> {
        let mut out = Vec::new();

        for linedef in &map.linedefs {
            let builder_graph_data = linedef
                .properties
                .get_str_default("builder_graph_data", String::new());
            if builder_graph_data.trim().is_empty() {
                continue;
            }

            let Ok(document) = BuilderDocument::from_text(&builder_graph_data) else {
                continue;
            };

            let Some(v0) = map.get_vertex_3d(linedef.start_vertex) else {
                continue;
            };
            let Some(v1) = map.get_vertex_3d(linedef.end_vertex) else {
                continue;
            };

            let mut along = Vec3::new(v1.x - v0.x, 0.0, v1.z - v0.z);
            let length = along.magnitude();
            if length <= 0.001 {
                continue;
            }
            along /= length;

            let along = Self::builder_linedef_along(along, &linedef.properties);
            let midpoint = (v0 + v1) * 0.5;
            let mount = Self::builder_linedef_mount(midpoint, along, &linedef.properties);
            let host =
                BuilderHost::preview_linedef(length, mount.wall_height, mount.wall_width.max(0.01));

            let Ok(assembly) = document.evaluate_with_host(&host) else {
                continue;
            };

            for detail in assembly.surface_details {
                eprintln!(
                    "[BuilderGraphDebug][collect] linedef={} wall detail={:?}",
                    linedef.id, detail
                );
                out.push(TopologyBuilderWallDetail {
                    linedef_id: linedef.id,
                    length,
                    wall_height: mount.wall_height,
                    wall_width: mount.wall_width,
                    origin: mount.host_origin,
                    along,
                    up: mount.up,
                    outward: mount.outward,
                    detail,
                });
            }
        }

        out
    }

    pub fn builder_cuts_for_sector(
        &self,
        sector_id: u32,
    ) -> impl Iterator<Item = &TopologyBuilderCut> {
        self.builder_cuts
            .iter()
            .filter(move |cut| cut.sector_id == sector_id)
    }

    pub fn resolved_builder_cuts_for_sector(
        &self,
        sector_id: u32,
    ) -> Vec<TopologyResolvedBuilderCut> {
        self.builder_cuts_for_sector(sector_id)
            .filter_map(TopologyBuilderCut::resolved)
            .collect()
    }

    pub fn surface_patch_descriptor(
        &self,
        map: &Map,
        sector_id: u32,
        surface_id: Uuid,
    ) -> Option<TopologySurfacePatch> {
        let surface = map.surfaces.get(&surface_id)?;
        if surface.sector_id != sector_id {
            return None;
        }

        let outer_uv = surface.sector_loop_uv(map)?;
        if outer_uv.len() < 3 {
            return None;
        }

        let outer = outer_uv
            .into_iter()
            .map(|uv| ControlPoint { uv, extrusion: 0.0 })
            .collect::<Vec<_>>();

        let mut holes = self
            .resolved_builder_cuts_for_sector(sector_id)
            .into_iter()
            .filter(|cut| cut.surface_id == Some(surface_id))
            .filter(|cut| matches!(cut.mode, BuilderCutMode::Cut | BuilderCutMode::Replace))
            .filter(|cut| cut.loop_uv.len() >= 3)
            .map(|cut| {
                cut.loop_uv
                    .into_iter()
                    .map(|uv| ControlPoint { uv, extrusion: 0.0 })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        holes.extend(
            self.freestanding_detail_footprint_holes(sector_id, surface_id)
                .into_iter(),
        );

        Some(TopologySurfacePatch {
            sector_id,
            surface_id,
            descriptor: SectorMeshDescriptor {
                is_hole: false,
                cap: Some(MeshTopology::FilledRegion { outer, holes }),
                sides: None,
                connection: ConnectionMode::Hard,
            },
            material_slot: None,
            tile_alias: None,
        })
    }

    fn freestanding_detail_footprint_holes(
        &self,
        sector_id: u32,
        surface_id: Uuid,
    ) -> Vec<Vec<ControlPoint>> {
        self.builder_details
            .iter()
            .filter(|detail| detail.sector_id == sector_id)
            .filter(|detail| detail.surface_id == Some(surface_id))
            .filter_map(|detail| Self::freestanding_detail_footprint_loop(detail))
            .filter(|points| points.len() >= 3)
            .map(|points| {
                points
                    .into_iter()
                    .map(|uv| ControlPoint { uv, extrusion: 0.0 })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn freestanding_detail_footprint_loop(
        detail: &TopologyBuilderDetail,
    ) -> Option<Vec<Vec2<f32>>> {
        let BuilderSurfaceDetail::Column {
            center,
            radius,
            base_height,
            cap_height,
            segments,
            placement,
            cut_footprint,
            ..
        } = &detail.detail
        else {
            return None;
        };
        if *placement != BuilderDetailPlacement::Freestanding || !*cut_footprint || *radius <= 0.0 {
            return None;
        }

        let center = detail.host_min_uv + *center;
        let has_square_base = *base_height > 0.0 || *cap_height > 0.0;
        if has_square_base {
            let half = radius * 1.55;
            Some(vec![
                Vec2::new(center.x - half, center.y - half),
                Vec2::new(center.x + half, center.y - half),
                Vec2::new(center.x + half, center.y + half),
                Vec2::new(center.x - half, center.y + half),
            ])
        } else {
            Some(regular_polygon(
                center,
                *radius * 1.08,
                (*segments as usize).clamp(8, 32),
            ))
        }
    }

    pub fn replacement_patch_descriptors(
        &self,
        sector_id: u32,
        surface_id: Uuid,
    ) -> Vec<TopologySurfacePatch> {
        self.resolved_builder_cuts_for_sector(sector_id)
            .into_iter()
            .filter(|cut| cut.surface_id == Some(surface_id))
            .filter(|cut| cut.mode == BuilderCutMode::Replace)
            .filter(|cut| cut.loop_uv.len() >= 3)
            .map(|cut| {
                let outer_uv = cut.loop_uv;
                let inner_uv = inset_loop(&outer_uv, cut.inset);
                let cap_uv = if cut.shape == BuilderCutShape::Border {
                    outer_uv.clone()
                } else {
                    inner_uv.clone()
                };
                let base_loop = cap_uv
                    .iter()
                    .copied()
                    .map(|uv| ControlPoint { uv, extrusion: 0.0 })
                    .collect::<Vec<_>>();
                let offset_loop = cap_uv
                    .iter()
                    .copied()
                    .map(|uv| ControlPoint {
                        uv,
                        extrusion: cut.offset,
                    })
                    .collect::<Vec<_>>();
                let sides = if cut.offset.abs() > 0.001 {
                    Some(MeshTopology::QuadStrip {
                        loop_a: base_loop,
                        loop_b: offset_loop.clone(),
                    })
                } else {
                    None
                };
                let holes = if cut.shape == BuilderCutShape::Border
                    && cut.inset > 0.0
                    && inner_uv.len() >= 3
                {
                    vec![
                        inner_uv
                            .into_iter()
                            .map(|uv| ControlPoint {
                                uv,
                                extrusion: cut.offset,
                            })
                            .collect::<Vec<_>>(),
                    ]
                } else {
                    Vec::new()
                };

                TopologySurfacePatch {
                    sector_id,
                    surface_id,
                    descriptor: SectorMeshDescriptor {
                        is_hole: false,
                        cap: Some(MeshTopology::FilledRegion {
                            outer: offset_loop,
                            holes,
                        }),
                        sides,
                        connection: ConnectionMode::Hard,
                    },
                    material_slot: None,
                    tile_alias: None,
                }
            })
            .collect()
    }

    pub fn detail_patch_descriptors(
        &self,
        sector_id: u32,
        surface_id: Uuid,
    ) -> Vec<TopologySurfacePatch> {
        let mut out = Vec::new();

        for detail in self
            .builder_details
            .iter()
            .filter(|detail| detail.sector_id == sector_id)
            .filter(|detail| detail.surface_id == Some(surface_id))
        {
            match &detail.detail {
                BuilderSurfaceDetail::Rect {
                    min,
                    max,
                    offset,
                    inset,
                    shape,
                    material_slot,
                    tile_alias,
                } => {
                    if let Some(patch) = Self::rect_detail_patch(
                        sector_id,
                        surface_id,
                        detail.host_min_uv + *min,
                        detail.host_min_uv + *max,
                        *offset,
                        *inset,
                        *shape,
                        material_slot.clone(),
                        tile_alias.clone(),
                    ) {
                        out.push(patch);
                    }
                }
                BuilderSurfaceDetail::Column {
                    center,
                    height,
                    radius,
                    offset,
                    base_height,
                    cap_height,
                    segments,
                    placement,
                    cut_footprint: _,
                    material_slot,
                    tile_alias,
                } => {
                    if *height <= 0.0 || *radius <= 0.0 {
                        continue;
                    }
                    if *placement == BuilderDetailPlacement::Freestanding {
                        continue;
                    }
                    let center = detail.host_min_uv + *center;
                    let base_width = radius * 1.55;
                    let shaft_loop =
                        rounded_column_loop(center, *height, *radius, *segments as usize);
                    if let Some(patch) = Self::loop_detail_patch(
                        sector_id,
                        surface_id,
                        shaft_loop,
                        *offset,
                        material_slot.clone(),
                        tile_alias.clone(),
                    ) {
                        out.push(patch);
                    }

                    if *base_height > 0.0 {
                        let base_min = Vec2::new(center.x - base_width, center.y);
                        let base_max = Vec2::new(center.x + base_width, center.y + base_height);
                        if let Some(patch) = Self::rect_detail_patch(
                            sector_id,
                            surface_id,
                            base_min,
                            base_max,
                            *offset,
                            0.0,
                            BuilderCutShape::Fill,
                            material_slot.clone(),
                            tile_alias.clone(),
                        ) {
                            out.push(patch);
                        }
                    }

                    if *cap_height > 0.0 {
                        let cap_min =
                            Vec2::new(center.x - base_width, center.y + height - cap_height);
                        let cap_max = Vec2::new(center.x + base_width, center.y + height);
                        if let Some(patch) = Self::rect_detail_patch(
                            sector_id,
                            surface_id,
                            cap_min,
                            cap_max,
                            *offset,
                            0.0,
                            BuilderCutShape::Fill,
                            material_slot.clone(),
                            tile_alias.clone(),
                        ) {
                            out.push(patch);
                        }
                    }
                }
                BuilderSurfaceDetail::Masonry {
                    min,
                    max,
                    block,
                    mortar,
                    offset,
                    pattern,
                    material_slot,
                    tile_alias,
                } => {
                    let local_min = Vec2::new(min.x.min(max.x), min.y.min(max.y));
                    let local_max = Vec2::new(min.x.max(max.x), min.y.max(max.y));
                    for (block_min, block_max) in
                        masonry_block_rects(local_min, local_max, *block, *mortar, *pattern)
                    {
                        if let Some(patch) = Self::rect_detail_patch(
                            sector_id,
                            surface_id,
                            detail.host_min_uv + block_min,
                            detail.host_min_uv + block_max,
                            *offset,
                            0.0,
                            BuilderCutShape::Fill,
                            material_slot.clone(),
                            tile_alias.clone(),
                        ) {
                            out.push(patch);
                        }
                    }
                }
            }
        }

        out
    }

    fn rect_detail_patch(
        sector_id: u32,
        surface_id: Uuid,
        min: Vec2<f32>,
        max: Vec2<f32>,
        offset: f32,
        inset: f32,
        shape: BuilderCutShape,
        material_slot: Option<String>,
        tile_alias: Option<String>,
    ) -> Option<TopologySurfacePatch> {
        if max.x <= min.x || max.y <= min.y {
            return None;
        }
        let outer_uv = vec![
            Vec2::new(min.x, min.y),
            Vec2::new(max.x, min.y),
            Vec2::new(max.x, max.y),
            Vec2::new(min.x, max.y),
        ];
        Self::polygon_detail_patch(
            sector_id,
            surface_id,
            outer_uv,
            offset,
            inset,
            shape,
            material_slot,
            tile_alias,
        )
    }

    fn loop_detail_patch(
        sector_id: u32,
        surface_id: Uuid,
        outer_uv: Vec<Vec2<f32>>,
        offset: f32,
        material_slot: Option<String>,
        tile_alias: Option<String>,
    ) -> Option<TopologySurfacePatch> {
        Self::polygon_detail_patch(
            sector_id,
            surface_id,
            outer_uv,
            offset,
            0.0,
            BuilderCutShape::Fill,
            material_slot,
            tile_alias,
        )
    }

    fn polygon_detail_patch(
        sector_id: u32,
        surface_id: Uuid,
        outer_uv: Vec<Vec2<f32>>,
        offset: f32,
        inset: f32,
        shape: BuilderCutShape,
        material_slot: Option<String>,
        tile_alias: Option<String>,
    ) -> Option<TopologySurfacePatch> {
        if outer_uv.len() < 3 {
            return None;
        }
        let inner_uv = inset_loop(&outer_uv, inset);
        let cap_uv = if shape == BuilderCutShape::Border {
            outer_uv.clone()
        } else {
            inner_uv.clone()
        };
        let base_loop = cap_uv
            .iter()
            .copied()
            .map(|uv| ControlPoint { uv, extrusion: 0.0 })
            .collect::<Vec<_>>();
        let offset_loop = cap_uv
            .iter()
            .copied()
            .map(|uv| ControlPoint {
                uv,
                extrusion: offset,
            })
            .collect::<Vec<_>>();
        let sides = if offset.abs() > 0.001 {
            Some(MeshTopology::QuadStrip {
                loop_a: base_loop,
                loop_b: offset_loop.clone(),
            })
        } else {
            None
        };
        let holes = if shape == BuilderCutShape::Border && inset > 0.0 && inner_uv.len() >= 3 {
            vec![
                inner_uv
                    .into_iter()
                    .map(|uv| ControlPoint {
                        uv,
                        extrusion: offset,
                    })
                    .collect::<Vec<_>>(),
            ]
        } else {
            Vec::new()
        };

        Some(TopologySurfacePatch {
            sector_id,
            surface_id,
            descriptor: SectorMeshDescriptor {
                is_hole: false,
                cap: Some(MeshTopology::FilledRegion {
                    outer: offset_loop,
                    holes,
                }),
                sides,
                connection: ConnectionMode::Hard,
            },
            material_slot,
            tile_alias,
        })
    }

    fn builder_host_for_sector_surface(
        map: &Map,
        sector_id: u32,
    ) -> Option<(BuilderHost, Option<Uuid>, Vec2<f32>, Vec2<f32>)> {
        let mut best: Option<(&crate::Surface, Vec2<f32>, Vec2<f32>)> = None;

        for surface in map.surfaces.values() {
            if surface.sector_id != sector_id {
                continue;
            }
            let Some(loop_uv) = surface.sector_loop_uv(map) else {
                continue;
            };
            if loop_uv.len() < 3 {
                continue;
            }

            let mut min_uv = loop_uv[0];
            let mut max_uv = loop_uv[0];
            for uv in &loop_uv {
                min_uv.x = min_uv.x.min(uv.x);
                min_uv.y = min_uv.y.min(uv.y);
                max_uv.x = max_uv.x.max(uv.x);
                max_uv.y = max_uv.y.max(uv.y);
            }

            let replace = best
                .as_ref()
                .map(|(best_surface, _, _)| surface.plane.origin.y > best_surface.plane.origin.y)
                .unwrap_or(true);
            if replace {
                best = Some((surface, min_uv, max_uv));
            }
        }

        best.map(|(surface, min_uv, max_uv)| {
            let width = (max_uv.x - min_uv.x).abs().max(0.01);
            let depth = (max_uv.y - min_uv.y).abs().max(0.01);
            (
                BuilderHost::preview_floor(width, depth),
                Some(surface.id),
                min_uv,
                Vec2::new(width, depth),
            )
        })
    }

    fn builder_linedef_outward(along: Vec3<f32>, props: &crate::ValueContainer) -> Vec3<f32> {
        let explicit = Vec3::new(
            props.get_float_default("builder_graph_outward_x", 0.0),
            props.get_float_default("builder_graph_outward_y", 0.0),
            props.get_float_default("builder_graph_outward_z", 0.0),
        );
        if let Some(outward) = explicit.try_normalized() {
            return outward;
        }

        let mut outward = Vec3::new(-along.z, 0.0, along.x);
        let side = props.get_float_default("builder_graph_wall_side", 0.0);
        if side < 0.0 {
            outward = -outward;
        }
        outward
    }

    fn builder_linedef_along(
        fallback_along: Vec3<f32>,
        props: &crate::ValueContainer,
    ) -> Vec3<f32> {
        let explicit = Vec3::new(
            props.get_float_default("host_along_x", 0.0),
            props.get_float_default("host_along_y", 0.0),
            props.get_float_default("host_along_z", 0.0),
        );
        explicit.try_normalized().unwrap_or(fallback_along)
    }

    fn builder_linedef_mount(
        origin: Vec3<f32>,
        along: Vec3<f32>,
        props: &crate::ValueContainer,
    ) -> TopologyBuilderLinedefMount {
        let outward = Vec3::new(
            props.get_float_default("host_outward_x", 0.0),
            props.get_float_default("host_outward_y", 0.0),
            props.get_float_default("host_outward_z", 0.0),
        )
        .try_normalized()
        .unwrap_or_else(|| Self::builder_linedef_outward(along, props));
        let up = Vec3::new(0.0, 1.0, 0.0);
        let wall_height = props.get_float_default("wall_height", 2.0).max(0.01);
        let wall_width = props.get_float_default("wall_width", 0.0).max(0.0);
        let wall_epsilon = props
            .get_float_default("profile_wall_epsilon", 0.001)
            .max(0.0);
        let surface_origin = match (
            props.get_float("builder_graph_surface_origin_x"),
            props.get_float("builder_graph_surface_origin_y"),
            props.get_float("builder_graph_surface_origin_z"),
        ) {
            (Some(x), Some(y), Some(z)) => Some(Vec3::new(x, y, z)),
            _ => None,
        };
        let face_origin = if let Some(face_offset) = props.get_float("builder_graph_face_offset") {
            origin + outward * face_offset.max(wall_epsilon)
        } else if let Some(surface_origin) = surface_origin {
            surface_origin
        } else {
            origin + outward * (wall_width * 0.5 + wall_epsilon)
        };

        TopologyBuilderLinedefMount {
            outward,
            up,
            wall_height,
            wall_width,
            host_origin: face_origin - up * (wall_height * 0.5) + outward * wall_epsilon,
        }
    }

    pub fn sector_owners_for_chunk(&self, chunk_bbox: &BBox) -> FxHashSet<GeoId> {
        self.sectors
            .values()
            .filter(|sector| {
                sector.bbox.intersects(chunk_bbox) && chunk_bbox.contains(sector.bbox.center())
            })
            .map(|sector| GeoId::Sector(sector.id))
            .collect()
    }

    pub fn owners_for_chunk(&self, map: &Map, chunk_bbox: &BBox) -> FxHashSet<GeoId> {
        let mut owners = self.sector_owners_for_chunk(chunk_bbox);
        owners.extend(
            self.feature_linedef_ids_for_chunk(map, chunk_bbox)
                .into_iter()
                .map(GeoId::Linedef),
        );

        if map.properties.get_bool_default("terrain_enabled", false) {
            owners.extend(Self::terrain_tile_owners_for_chunk(chunk_bbox));
        }

        owners
    }

    pub fn filtered_map_for_chunk(&self, map: &Map, chunk_bbox: &BBox) -> Map {
        let mut sector_ids = self
            .sector_owners_for_chunk(chunk_bbox)
            .into_iter()
            .filter_map(|owner| match owner {
                GeoId::Sector(id) => Some(id),
                _ => None,
            })
            .collect::<FxHashSet<_>>();
        let terrain_enabled = map.properties.get_bool_default("terrain_enabled", false);
        if terrain_enabled {
            sector_ids.extend(self.terrain_sector_ids_for_chunk(map, chunk_bbox));
        }

        let mut surface_ids = FxHashSet::default();
        for sector_id in &sector_ids {
            if let Some(sector) = self.sectors.get(sector_id) {
                surface_ids.extend(sector.surfaces.iter().copied());
            }
        }

        let mut linedef_ids = FxHashSet::default();
        for sector in &map.sectors {
            if sector_ids.contains(&sector.id) {
                linedef_ids.extend(sector.linedefs.iter().copied());
            }
        }
        if terrain_enabled {
            linedef_ids.extend(self.terrain_linedef_ids_for_chunk(map, chunk_bbox));
        }
        linedef_ids.extend(self.feature_linedef_ids_for_chunk(map, chunk_bbox));

        let mut vertex_ids = FxHashSet::default();
        for linedef in &map.linedefs {
            if linedef_ids.contains(&linedef.id) {
                vertex_ids.insert(linedef.start_vertex);
                vertex_ids.insert(linedef.end_vertex);
            }
        }
        if terrain_enabled {
            vertex_ids.extend(self.terrain_vertex_ids_for_chunk(map, chunk_bbox));
            vertex_ids.extend(map.vertices.iter().map(|vertex| vertex.id));
        }
        vertex_ids.extend(self.feature_vertex_ids_for_chunk(map, chunk_bbox));

        let mut filtered = map.clone();
        filtered
            .sectors
            .retain(|sector| sector_ids.contains(&sector.id));
        filtered
            .linedefs
            .retain(|linedef| linedef_ids.contains(&linedef.id));
        filtered
            .vertices
            .retain(|vertex| vertex_ids.contains(&vertex.id));
        filtered.surfaces.retain(|surface_id, surface| {
            surface_ids.contains(surface_id) || sector_ids.contains(&surface.sector_id)
        });
        filtered
    }

    fn expanded_chunk_bbox(chunk_bbox: &BBox) -> BBox {
        let mut bbox = *chunk_bbox;
        bbox.expand(Vec2::broadcast(Self::TERRAIN_CHUNK_PAD * 2.0));
        bbox
    }

    fn terrain_tile_owners_for_chunk(chunk_bbox: &BBox) -> FxHashSet<GeoId> {
        let min_x = chunk_bbox.min.x.floor() as i32;
        let min_z = chunk_bbox.min.y.floor() as i32;
        let max_x = chunk_bbox.max.x.ceil() as i32;
        let max_z = chunk_bbox.max.y.ceil() as i32;
        let mut owners = FxHashSet::default();
        for z in min_z..max_z.max(min_z + 1) {
            for x in min_x..max_x.max(min_x + 1) {
                owners.insert(GeoId::Terrain(x, z));
            }
        }
        owners
    }

    fn terrain_sector_ids_for_chunk(&self, map: &Map, chunk_bbox: &BBox) -> FxHashSet<u32> {
        let query_bbox = Self::expanded_chunk_bbox(chunk_bbox);
        let mut sector_ids = FxHashSet::default();

        for sector in &map.sectors {
            let terrain_mode = sector.properties.get_int_default("terrain_mode", 0);
            let builder_replace_surface = self.builder_cuts.iter().any(|cut| {
                cut.sector_id == sector.id
                    && matches!(
                        cut.mask,
                        BuilderCutMask::Rect {
                            mode: BuilderCutMode::Replace,
                            ..
                        } | BuilderCutMask::Loop {
                            mode: BuilderCutMode::Replace,
                            ..
                        }
                    )
            });

            let cutout_related = sector.properties.get_bool_default("cutout_handle", false)
                || sector.properties.contains("linked_cutout_handle");

            if terrain_mode == 0 && !builder_replace_surface && !cutout_related {
                continue;
            }

            let mut bbox = sector.bounding_box(map);
            if terrain_mode == 2 {
                let influence = sector
                    .properties
                    .get_float_default("ridge_plateau_width", 0.0)
                    .max(0.0)
                    + sector
                        .properties
                        .get_float_default("ridge_falloff_distance", 0.0)
                        .max(0.0)
                    + sector
                        .properties
                        .get_float_default("terrain_tile_falloff", 1.0)
                        .max(0.0);
                bbox.expand(Vec2::broadcast(influence * 2.0));
            }

            if bbox.intersects(&query_bbox) {
                sector_ids.insert(sector.id);
            }
        }

        sector_ids
    }

    fn terrain_linedef_ids_for_chunk(&self, map: &Map, chunk_bbox: &BBox) -> FxHashSet<u32> {
        let mut query_bbox = Self::expanded_chunk_bbox(chunk_bbox);
        query_bbox.expand(Vec2::broadcast(100.0));
        let mut linedef_ids = FxHashSet::default();

        for linedef in &map.linedefs {
            if !linedef.properties.contains("terrain_source")
                && !linedef.properties.get_bool_default("terrain_smooth", false)
            {
                continue;
            }
            let width = linedef
                .properties
                .get_float_default("terrain_width", 2.0)
                .max(0.0);
            let falloff = linedef
                .properties
                .get_float_default("terrain_tile_falloff", 1.0)
                .max(0.0);
            let organic = linedef
                .properties
                .get_float_default("terrain_road_organic", 0.0)
                .clamp(0.0, 1.0);
            let mut bbox = linedef.bounding_box(map);
            bbox.expand(Vec2::broadcast(
                (width + falloff + organic * 6.0 + 4.0) * 2.0,
            ));
            if bbox.intersects(&query_bbox) {
                linedef_ids.insert(linedef.id);
            }
        }

        linedef_ids
    }

    fn terrain_vertex_ids_for_chunk(&self, map: &Map, chunk_bbox: &BBox) -> FxHashSet<u32> {
        let mut query_bbox = Self::expanded_chunk_bbox(chunk_bbox);
        query_bbox.expand(Vec2::broadcast(100.0));
        let mut vertex_ids = FxHashSet::default();

        for vertex in &map.vertices {
            if !vertex.properties.get_bool_default("terrain_control", false)
                && !vertex.properties.contains("terrain_source")
            {
                continue;
            }
            let smoothness = vertex
                .properties
                .get_float_default("smoothness", 1.0)
                .max(0.0);
            let falloff = vertex
                .properties
                .get_float_default("terrain_tile_falloff", 1.0)
                .max(0.0);
            let radius = smoothness * 2.0 + falloff + Self::TERRAIN_CHUNK_PAD;
            let bbox = BBox::new(
                Vec2::new(vertex.x - radius, vertex.y - radius),
                Vec2::new(vertex.x + radius, vertex.y + radius),
            );
            if bbox.intersects(&query_bbox) {
                vertex_ids.insert(vertex.id);
            }
        }

        vertex_ids
    }

    fn feature_linedef_ids_for_chunk(&self, map: &Map, chunk_bbox: &BBox) -> FxHashSet<u32> {
        let mut linedef_ids = FxHashSet::default();

        for linedef in &map.linedefs {
            let feature = linedef
                .properties
                .get_str_default("linedef_feature", "None".to_string());
            let has_linedef_feature = feature == "Palisade" || feature == "Fence";
            let has_builder_feature = !linedef
                .properties
                .get_str_default("builder_graph_data", String::new())
                .trim()
                .is_empty();
            if !has_linedef_feature && !has_builder_feature {
                continue;
            }

            let Some(v0) = map.get_vertex_3d(linedef.start_vertex) else {
                continue;
            };
            let Some(v1) = map.get_vertex_3d(linedef.end_vertex) else {
                continue;
            };
            let line_mid = Vec2::new((v0.x + v1.x) * 0.5, (v0.z + v1.z) * 0.5);
            if chunk_bbox.contains(line_mid) || linedef.bounding_box(map).intersects(chunk_bbox) {
                linedef_ids.insert(linedef.id);
            }
        }

        linedef_ids
    }

    fn feature_vertex_ids_for_chunk(&self, map: &Map, chunk_bbox: &BBox) -> FxHashSet<u32> {
        let mut vertex_ids = FxHashSet::default();

        for vertex in &map.vertices {
            if vertex
                .properties
                .get_str_default("builder_graph_data", String::new())
                .trim()
                .is_empty()
            {
                continue;
            }
            let pos = Vec2::new(vertex.x, vertex.y);
            if chunk_bbox.contains(pos) {
                vertex_ids.insert(vertex.id);
            }
        }

        vertex_ids
    }

    fn builder_replace_surface_sector_ids(&self, chunk_bbox: &BBox) -> FxHashSet<u32> {
        self.sectors
            .values()
            .filter(|topology_sector| topology_sector.bbox.intersects(chunk_bbox))
            .filter(|topology_sector| {
                let has_replace_cut = self
                    .builder_cuts
                    .iter()
                    .filter(|cut| cut.sector_id == topology_sector.id)
                    .any(|cut| {
                        matches!(
                            cut.mask,
                            BuilderCutMask::Rect {
                                mode: BuilderCutMode::Replace,
                                ..
                            } | BuilderCutMask::Loop {
                                mode: BuilderCutMode::Replace,
                                ..
                            }
                        )
                    });
                let has_footprint_cut = self
                    .builder_details
                    .iter()
                    .filter(|detail| detail.sector_id == topology_sector.id)
                    .any(|detail| Self::freestanding_detail_footprint_loop(detail).is_some());
                has_replace_cut || has_footprint_cut
            })
            .map(|topology_sector| topology_sector.id)
            .collect()
    }

    fn builder_detail_sector_ids(&self, chunk_bbox: &BBox) -> FxHashSet<u32> {
        self.sectors
            .values()
            .filter(|topology_sector| topology_sector.bbox.intersects(chunk_bbox))
            .filter(|topology_sector| {
                self.builder_details
                    .iter()
                    .any(|detail| detail.sector_id == topology_sector.id)
            })
            .map(|topology_sector| topology_sector.id)
            .collect()
    }

    fn builder_wall_detail_linedef_ids(&self, map: &Map, chunk_bbox: &BBox) -> FxHashSet<u32> {
        let detail_linedefs = self
            .builder_wall_details
            .iter()
            .map(|detail| detail.linedef_id)
            .collect::<FxHashSet<_>>();
        if detail_linedefs.is_empty() {
            return FxHashSet::default();
        }

        map.linedefs
            .iter()
            .filter(|linedef| detail_linedefs.contains(&linedef.id))
            .filter(|linedef| {
                let pad = self
                    .builder_wall_details
                    .iter()
                    .filter(|detail| detail.linedef_id == linedef.id)
                    .map(|detail| detail.wall_detail_radius())
                    .fold(0.5_f32, f32::max);
                let mut bbox = linedef.bounding_box(map);
                bbox.expand(Vec2::broadcast(pad + 0.25));
                bbox.intersects(chunk_bbox) || chunk_bbox.contains(bbox.center())
            })
            .map(|linedef| linedef.id)
            .collect()
    }

    fn suppress_replace_surface_sectors(
        &self,
        filtered: &mut Map,
        replace_sector_ids: &FxHashSet<u32>,
    ) {
        if replace_sector_ids.is_empty() {
            return;
        }

        filtered
            .sectors
            .retain(|sector| !replace_sector_ids.contains(&sector.id));
        filtered
            .surfaces
            .retain(|_, surface| !replace_sector_ids.contains(&surface.sector_id));
    }
}

impl TopologyBuilder {
    pub fn build_topology(map: &Map) -> MapTopology {
        MapTopology::build(map)
    }

    fn scene_for_map(&mut self, map: &Map) -> &TopologyScene {
        let builder_signature = Self::builder_scene_signature(map);
        let needs_rebuild =
            self.cached_scene
                .as_ref()
                .is_none_or(|(map_id, changed, signature, _)| {
                    *map_id != map.id || *changed != map.changed || *signature != builder_signature
                });

        if needs_rebuild {
            self.cached_scene = Some((
                map.id,
                map.changed,
                builder_signature,
                TopologyScene::build(map),
            ));
        }

        &self.cached_scene.as_ref().expect("topology scene cache").3
    }

    fn builder_scene_signature(map: &Map) -> u64 {
        let mut hasher = DefaultHasher::new();
        for sector in &map.sectors {
            let data = sector
                .properties
                .get_str_default("builder_graph_data", String::new());
            if data.trim().is_empty() {
                continue;
            }
            sector.id.hash(&mut hasher);
            data.hash(&mut hasher);
            sector
                .properties
                .get_str_default("builder_graph_target", String::new())
                .hash(&mut hasher);
            sector
                .properties
                .get_str_default("builder_surface_mode", String::new())
                .hash(&mut hasher);
            sector
                .properties
                .get_bool_default("builder_hide_host", false)
                .hash(&mut hasher);
        }
        for linedef in &map.linedefs {
            let data = linedef
                .properties
                .get_str_default("builder_graph_data", String::new());
            if data.trim().is_empty() {
                continue;
            }
            linedef.id.hash(&mut hasher);
            linedef.start_vertex.hash(&mut hasher);
            linedef.end_vertex.hash(&mut hasher);
            data.hash(&mut hasher);
            linedef
                .properties
                .get_str_default("builder_graph_target", String::new())
                .hash(&mut hasher);
            for key in [
                "builder_graph_wall_side",
                "builder_graph_outward_x",
                "builder_graph_outward_y",
                "builder_graph_outward_z",
                "builder_graph_surface_origin_x",
                "builder_graph_surface_origin_y",
                "builder_graph_surface_origin_z",
                "builder_graph_face_offset",
                "wall_height",
                "wall_width",
            ] {
                linedef
                    .properties
                    .get_float_default(key, 0.0)
                    .to_bits()
                    .hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    fn emit_builder_replace_surface_patches(
        scene: &TopologyScene,
        map: &Map,
        assets: &Assets,
        chunk: &mut Chunk,
        vmchunk: &mut scenevm::Chunk,
        replace_sector_ids: &FxHashSet<u32>,
    ) {
        if !replace_sector_ids.is_empty() {
            eprintln!(
                "[BuilderGraphDebug][emit] chunk=({}, {}) replace sectors={:?}",
                chunk.origin.x, chunk.origin.y, replace_sector_ids
            );
        }
        for sector_id in replace_sector_ids {
            let Some(topology_sector) = scene.sectors.get(sector_id) else {
                eprintln!(
                    "[BuilderGraphDebug][emit] sector={} missing topology sector",
                    sector_id
                );
                continue;
            };
            let Some(sector) = map.find_sector(*sector_id) else {
                eprintln!(
                    "[BuilderGraphDebug][emit] sector={} missing map sector",
                    sector_id
                );
                continue;
            };
            let tile_id = Self::sector_surface_tile_id(sector, assets);

            for surface_id in &topology_sector.surfaces {
                let Some(surface) = map.surfaces.get(surface_id) else {
                    eprintln!(
                        "[BuilderGraphDebug][emit] sector={} surface={} missing",
                        sector_id, surface_id
                    );
                    continue;
                };
                let Some(patch) = scene.surface_patch_descriptor(map, *sector_id, *surface_id)
                else {
                    eprintln!(
                        "[BuilderGraphDebug][emit] sector={} surface={} patch descriptor failed",
                        sector_id, surface_id
                    );
                    continue;
                };
                if !Self::patch_has_builder_holes(&patch) {
                    eprintln!(
                        "[BuilderGraphDebug][emit] sector={} surface={} patch has no holes",
                        sector_id, surface_id
                    );
                    continue;
                }

                let mesh_builder = SurfaceMeshBuilder::new(surface);
                let meshes = mesh_builder.build(&patch.descriptor);
                eprintln!(
                    "[BuilderGraphDebug][emit] sector={} surface={} tile={} meshes={}",
                    sector_id,
                    surface_id,
                    tile_id,
                    meshes.len()
                );
                for mesh in meshes {
                    if mesh.indices.is_empty() {
                        eprintln!(
                            "[BuilderGraphDebug][emit] sector={} surface={} skipped empty mesh vertices={} indices=0",
                            sector_id,
                            surface_id,
                            mesh.vertices.len()
                        );
                        continue;
                    }

                    let mut batch = Batch3D::new(
                        mesh.vertices.clone(),
                        mesh.indices.clone(),
                        mesh.uvs.clone(),
                    );
                    batch.repeat_mode = RepeatMode::RepeatXY;
                    batch.geometry_source = GeometrySource::Sector(*sector_id);
                    if let Some(texture_index) = assets.tile_index(&tile_id) {
                        batch.source = PixelSource::StaticTileIndex(texture_index);
                    }
                    chunk.batches3d.push(batch);

                    vmchunk.add_poly_3d(
                        GeoId::Sector(*sector_id),
                        tile_id,
                        mesh.vertices,
                        mesh.uvs,
                        mesh.indices,
                        0,
                        true,
                    );
                    eprintln!(
                        "[BuilderGraphDebug][emit] sector={} surface={} emitted vertices={} triangles={}",
                        sector_id,
                        surface_id,
                        chunk
                            .batches3d
                            .last()
                            .map(|b| b.vertices.len())
                            .unwrap_or(0),
                        chunk.batches3d.last().map(|b| b.indices.len()).unwrap_or(0)
                    );
                }

                for replacement in scene.replacement_patch_descriptors(*sector_id, *surface_id) {
                    let meshes = mesh_builder.build(&replacement.descriptor);
                    eprintln!(
                        "[BuilderGraphDebug][emit] sector={} surface={} replacement meshes={}",
                        sector_id,
                        surface_id,
                        meshes.len()
                    );
                    for mesh in meshes {
                        if mesh.indices.is_empty() {
                            eprintln!(
                                "[BuilderGraphDebug][emit] sector={} surface={} skipped empty replacement mesh vertices={} indices=0",
                                sector_id,
                                surface_id,
                                mesh.vertices.len()
                            );
                            continue;
                        }

                        let mut batch = Batch3D::new(
                            mesh.vertices.clone(),
                            mesh.indices.clone(),
                            mesh.uvs.clone(),
                        );
                        batch.repeat_mode = RepeatMode::RepeatXY;
                        batch.geometry_source = GeometrySource::Sector(*sector_id);
                        if let Some(texture_index) = assets.tile_index(&tile_id) {
                            batch.source = PixelSource::StaticTileIndex(texture_index);
                        }
                        chunk.batches3d.push(batch);

                        vmchunk.add_poly_3d(
                            GeoId::Sector(*sector_id),
                            tile_id,
                            mesh.vertices,
                            mesh.uvs,
                            mesh.indices,
                            0,
                            true,
                        );
                        eprintln!(
                            "[BuilderGraphDebug][emit] sector={} surface={} emitted replacement vertices={} triangles={}",
                            sector_id,
                            surface_id,
                            chunk
                                .batches3d
                                .last()
                                .map(|b| b.vertices.len())
                                .unwrap_or(0),
                            chunk.batches3d.last().map(|b| b.indices.len()).unwrap_or(0)
                        );
                    }
                }
            }
        }
    }

    fn emit_builder_surface_detail_patches(
        scene: &TopologyScene,
        map: &Map,
        assets: &Assets,
        chunk: &mut Chunk,
        vmchunk: &mut scenevm::Chunk,
        detail_sector_ids: &FxHashSet<u32>,
    ) {
        if !detail_sector_ids.is_empty() {
            eprintln!(
                "[BuilderGraphDebug][emit] chunk=({}, {}) detail sectors={:?}",
                chunk.origin.x, chunk.origin.y, detail_sector_ids
            );
        }
        for sector_id in detail_sector_ids {
            let Some(topology_sector) = scene.sectors.get(sector_id) else {
                continue;
            };
            let Some(sector) = map.find_sector(*sector_id) else {
                continue;
            };

            for surface_id in &topology_sector.surfaces {
                let Some(surface) = map.surfaces.get(surface_id) else {
                    continue;
                };
                let mesh_builder = SurfaceMeshBuilder::new(surface);
                for patch in scene.detail_patch_descriptors(*sector_id, *surface_id) {
                    let tile_id = Self::patch_tile_id(sector, &patch, assets);
                    let meshes = mesh_builder.build(&patch.descriptor);
                    eprintln!(
                        "[BuilderGraphDebug][emit] sector={} surface={} detail tile={} material={:?} alias={:?} meshes={}",
                        sector_id,
                        surface_id,
                        tile_id,
                        patch.material_slot,
                        patch.tile_alias,
                        meshes.len()
                    );
                    for mesh in meshes {
                        if mesh.indices.is_empty() {
                            continue;
                        }

                        let mut batch = Batch3D::new(
                            mesh.vertices.clone(),
                            mesh.indices.clone(),
                            mesh.uvs.clone(),
                        );
                        batch.repeat_mode = RepeatMode::RepeatXY;
                        batch.geometry_source = GeometrySource::Sector(*sector_id);
                        if let Some(texture_index) = assets.tile_index(&tile_id) {
                            batch.source = PixelSource::StaticTileIndex(texture_index);
                        }
                        chunk.batches3d.push(batch);

                        vmchunk.add_poly_3d(
                            GeoId::Sector(*sector_id),
                            tile_id,
                            mesh.vertices,
                            mesh.uvs,
                            mesh.indices,
                            0,
                            true,
                        );
                    }
                }
            }
        }
    }

    fn emit_builder_freestanding_sector_details(
        scene: &TopologyScene,
        map: &Map,
        assets: &Assets,
        chunk: &mut Chunk,
        vmchunk: &mut scenevm::Chunk,
        detail_sector_ids: &FxHashSet<u32>,
    ) {
        for sector_id in detail_sector_ids {
            let Some(topology_sector) = scene.sectors.get(sector_id) else {
                continue;
            };
            let Some(sector) = map.find_sector(*sector_id) else {
                continue;
            };
            for surface_id in &topology_sector.surfaces {
                let Some(surface) = map.surfaces.get(surface_id) else {
                    continue;
                };
                for detail in scene
                    .builder_details
                    .iter()
                    .filter(|detail| detail.sector_id == *sector_id)
                    .filter(|detail| detail.surface_id == Some(*surface_id))
                {
                    let BuilderSurfaceDetail::Column {
                        center,
                        height,
                        radius,
                        offset,
                        base_height,
                        cap_height,
                        segments,
                        placement,
                        cut_footprint: _,
                        material_slot,
                        tile_alias,
                    } = &detail.detail
                    else {
                        continue;
                    };
                    if *placement != BuilderDetailPlacement::Freestanding {
                        continue;
                    }

                    let tile_id = Self::sector_detail_material_tile_id(
                        sector,
                        *sector_id,
                        material_slot.as_deref(),
                        tile_alias.as_deref(),
                        assets,
                    );
                    let anchor_uv = detail.host_min_uv + *center;
                    let anchor = surface.uvw_to_world(anchor_uv, *offset);
                    let up = Vec3::new(0.0, 1.0, 0.0);
                    let mut along = surface.edit_uv.right;
                    along.y = 0.0;
                    if along.magnitude() <= 0.001 {
                        along = Vec3::new(1.0, 0.0, 0.0);
                    } else {
                        along = along.normalized();
                    }
                    let mut outward = along.cross(up);
                    if outward.magnitude() <= 0.001 {
                        outward = Vec3::new(0.0, 0.0, 1.0);
                    } else {
                        outward = outward.normalized();
                    }

                    let mut vertices = Vec::new();
                    let mut uvs = Vec::new();
                    let mut indices = Vec::new();
                    append_freestanding_column_mesh(
                        &mut vertices,
                        &mut uvs,
                        &mut indices,
                        anchor,
                        (*height).max(0.01),
                        (*radius).max(0.01),
                        (*base_height).max(0.0),
                        (*cap_height).max(0.0),
                        (*segments as usize).clamp(6, 48),
                        along,
                        up,
                        outward,
                    );
                    if indices.is_empty() {
                        continue;
                    }

                    let mut batch = Batch3D::new(vertices.clone(), indices.clone(), uvs.clone());
                    batch.repeat_mode = RepeatMode::RepeatXY;
                    batch.geometry_source = GeometrySource::Sector(*sector_id);
                    if let Some(texture_index) = assets.tile_index(&tile_id) {
                        batch.source = PixelSource::StaticTileIndex(texture_index);
                    }
                    chunk.batches3d.push(batch);

                    vmchunk.add_poly_3d(
                        GeoId::Sector(*sector_id),
                        tile_id,
                        vertices,
                        uvs,
                        indices,
                        0,
                        true,
                    );
                }
            }
        }
    }

    fn emit_builder_wall_detail_batches(
        scene: &TopologyScene,
        map: &Map,
        assets: &Assets,
        chunk: &mut Chunk,
        vmchunk: &mut scenevm::Chunk,
        detail_linedef_ids: &FxHashSet<u32>,
    ) {
        if !detail_linedef_ids.is_empty() {
            eprintln!(
                "[BuilderGraphDebug][emit] chunk=({}, {}) wall detail linedefs={:?}",
                chunk.origin.x, chunk.origin.y, detail_linedef_ids
            );
        }

        for detail in scene
            .builder_wall_details
            .iter()
            .filter(|detail| detail_linedef_ids.contains(&detail.linedef_id))
        {
            let Some(linedef) = map.find_linedef(detail.linedef_id) else {
                continue;
            };
            let tile_id = Self::wall_detail_tile_id(linedef, detail, assets);
            let mut vertices = Vec::new();
            let mut uvs = Vec::new();
            let mut indices = Vec::new();
            Self::append_wall_detail_mesh(detail, &mut vertices, &mut uvs, &mut indices);
            if indices.is_empty() {
                continue;
            }

            let mut batch = Batch3D::new(vertices.clone(), indices.clone(), uvs.clone());
            batch.repeat_mode = RepeatMode::RepeatXY;
            batch.geometry_source = GeometrySource::Linedef(detail.linedef_id);
            if let Some(texture_index) = assets.tile_index(&tile_id) {
                batch.source = PixelSource::StaticTileIndex(texture_index);
            }
            chunk.batches3d.push(batch);

            vmchunk.add_poly_3d(
                GeoId::Linedef(detail.linedef_id),
                tile_id,
                vertices,
                uvs,
                indices,
                0,
                true,
            );
        }
    }

    fn append_wall_detail_mesh(
        detail: &TopologyBuilderWallDetail,
        vertices: &mut Vec<[f32; 4]>,
        uvs: &mut Vec<[f32; 2]>,
        indices: &mut Vec<(usize, usize, usize)>,
    ) {
        match &detail.detail {
            BuilderSurfaceDetail::Rect {
                min, max, offset, ..
            } => {
                let u0 = min.x.min(max.x);
                let u1 = min.x.max(max.x);
                let v0 = min.y.min(max.y);
                let v1 = min.y.max(max.y);
                if u1 <= u0 || v1 <= v0 {
                    return;
                }
                let size = Vec3::new(u1 - u0, v1 - v0, offset.abs().max(0.04));
                let center = detail.origin
                    + detail.along * ((u0 + u1) * 0.5)
                    + detail.up * ((v0 + v1) * 0.5)
                    + detail.outward * (-*offset + size.z * 0.5);
                append_oriented_box(
                    vertices,
                    uvs,
                    indices,
                    center,
                    size,
                    detail.along,
                    detail.up,
                    detail.outward,
                );
            }
            BuilderSurfaceDetail::Column {
                center,
                height,
                radius,
                offset,
                base_height,
                cap_height,
                segments,
                placement: _,
                cut_footprint: _,
                ..
            } => {
                let height = height.max(0.01);
                let radius = radius.max(0.01);
                let base_height = base_height.max(0.0);
                let cap_height = cap_height.max(0.0);
                let shaft_height = (height - base_height - cap_height).max(0.02);
                let base_center = detail.origin
                    + detail.along * center.x
                    + detail.up * center.y
                    + detail.outward * (-*offset);

                if base_height > 0.0 {
                    append_oriented_box(
                        vertices,
                        uvs,
                        indices,
                        base_center
                            + detail.up * (base_height * 0.5)
                            + detail.outward * (radius * 0.15),
                        Vec3::new(radius * 2.9, base_height, radius * 0.7),
                        detail.along,
                        detail.up,
                        detail.outward,
                    );
                }

                append_oriented_cylinder(
                    vertices,
                    uvs,
                    indices,
                    base_center + detail.up * (base_height + shaft_height * 0.5),
                    shaft_height,
                    radius,
                    (*segments as usize).clamp(6, 48),
                    detail.along,
                    detail.up,
                    detail.outward,
                );

                if cap_height > 0.0 {
                    append_oriented_box(
                        vertices,
                        uvs,
                        indices,
                        base_center
                            + detail.up * (base_height + shaft_height + cap_height * 0.5)
                            + detail.outward * (radius * 0.15),
                        Vec3::new(radius * 2.9, cap_height, radius * 0.7),
                        detail.along,
                        detail.up,
                        detail.outward,
                    );
                }
            }
            BuilderSurfaceDetail::Masonry {
                min,
                max,
                block,
                mortar,
                offset,
                pattern,
                ..
            } => {
                let local_min = Vec2::new(min.x.min(max.x), min.y.min(max.y));
                let local_max = Vec2::new(min.x.max(max.x), min.y.max(max.y));
                let depth = offset.abs().max(0.035);
                for (block_min, block_max) in
                    masonry_block_rects(local_min, local_max, *block, *mortar, *pattern)
                {
                    let u0 = block_min.x;
                    let u1 = block_max.x;
                    let v0 = block_min.y;
                    let v1 = block_max.y;
                    if u1 <= u0 || v1 <= v0 {
                        continue;
                    }
                    let size = Vec3::new(u1 - u0, v1 - v0, depth);
                    let center = detail.origin
                        + detail.along * ((u0 + u1) * 0.5)
                        + detail.up * ((v0 + v1) * 0.5)
                        + detail.outward * (-*offset + size.z * 0.5);
                    append_oriented_box(
                        vertices,
                        uvs,
                        indices,
                        center,
                        size,
                        detail.along,
                        detail.up,
                        detail.outward,
                    );
                }
            }
        }
    }

    fn patch_has_builder_holes(patch: &TopologySurfacePatch) -> bool {
        matches!(
            &patch.descriptor.cap,
            Some(MeshTopology::FilledRegion { holes, .. }) if !holes.is_empty()
        )
    }

    fn sector_surface_tile_id(sector: &crate::Sector, assets: &Assets) -> Uuid {
        let default_tile_id = Uuid::from_str(DEFAULT_TILE_ID).unwrap();
        match sector.properties.get("source") {
            Some(Value::Source(source)) => source
                .tile_from_tile_list(assets)
                .map(|tile| tile.id)
                .unwrap_or(default_tile_id),
            _ => default_tile_id,
        }
    }

    fn patch_tile_id(
        sector: &crate::Sector,
        patch: &TopologySurfacePatch,
        assets: &Assets,
    ) -> Uuid {
        let default_tile_id = Uuid::from_str(DEFAULT_TILE_ID).unwrap();

        if let Some(material_slot) = patch.material_slot.as_deref() {
            let key = format!(
                "builder_material_{}",
                Self::normalize_builder_material_key(material_slot)
            );
            if let Some(Value::Source(source)) = sector.properties.get(&key) {
                return source
                    .tile_from_tile_list(assets)
                    .map(|tile| tile.id)
                    .unwrap_or(default_tile_id);
            }
        }

        if let Some(alias) = patch.tile_alias.as_deref()
            && let Some(tile_id) = Self::tile_id_by_alias(assets, alias, patch.sector_id)
        {
            return tile_id;
        }

        Self::sector_surface_tile_id(sector, assets)
    }

    fn sector_detail_material_tile_id(
        sector: &crate::Sector,
        seed: u32,
        material_slot: Option<&str>,
        tile_alias: Option<&str>,
        assets: &Assets,
    ) -> Uuid {
        let default_tile_id = Uuid::from_str(DEFAULT_TILE_ID).unwrap();

        if let Some(material_slot) = material_slot {
            let key = format!(
                "builder_material_{}",
                Self::normalize_builder_material_key(material_slot)
            );
            if let Some(Value::Source(source)) = sector.properties.get(&key) {
                return source
                    .tile_from_tile_list(assets)
                    .map(|tile| tile.id)
                    .unwrap_or(default_tile_id);
            }
        }

        if let Some(alias) = tile_alias
            && let Some(tile_id) = Self::tile_id_by_alias(assets, alias, seed)
        {
            return tile_id;
        }

        Self::sector_surface_tile_id(sector, assets)
    }

    fn wall_detail_tile_id(
        linedef: &crate::Linedef,
        detail: &TopologyBuilderWallDetail,
        assets: &Assets,
    ) -> Uuid {
        let default_tile_id = Uuid::from_str(DEFAULT_TILE_ID).unwrap();
        let (material_slot, tile_alias) = match &detail.detail {
            BuilderSurfaceDetail::Rect {
                material_slot,
                tile_alias,
                ..
            } => (material_slot.as_deref(), tile_alias.as_deref()),
            BuilderSurfaceDetail::Column {
                material_slot,
                tile_alias,
                placement: _,
                cut_footprint: _,
                ..
            }
            | BuilderSurfaceDetail::Masonry {
                material_slot,
                tile_alias,
                ..
            } => (material_slot.as_deref(), tile_alias.as_deref()),
        };

        if let Some(material_slot) = material_slot {
            let key = format!(
                "builder_material_{}",
                Self::normalize_builder_material_key(material_slot)
            );
            if let Some(Value::Source(source)) = linedef.properties.get(&key) {
                return source
                    .tile_from_tile_list(assets)
                    .map(|tile| tile.id)
                    .unwrap_or(default_tile_id);
            }
        }

        if let Some(alias) = tile_alias
            && let Some(tile_id) = Self::tile_id_by_alias(assets, alias, detail.linedef_id)
        {
            return tile_id;
        }

        match linedef.properties.get("source") {
            Some(Value::Source(source)) => source
                .tile_from_tile_list(assets)
                .map(|tile| tile.id)
                .unwrap_or(default_tile_id),
            _ => default_tile_id,
        }
    }

    fn tile_id_by_alias(assets: &Assets, alias: &str, seed: u32) -> Option<Uuid> {
        let normalized = alias.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return None;
        }
        let mut matches = assets
            .tiles
            .iter()
            .filter(|(_, tile)| {
                tile.alias
                    .split([',', ';', '\n'])
                    .map(str::trim)
                    .any(|part| !part.is_empty() && part.eq_ignore_ascii_case(&normalized))
            })
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        matches.sort();
        if matches.is_empty() {
            None
        } else {
            Some(matches[seed as usize % matches.len()])
        }
    }

    fn normalize_builder_material_key(name: &str) -> String {
        let mut out = String::new();
        let mut prev_is_sep = false;
        for (i, ch) in name.chars().enumerate() {
            if ch.is_ascii_alphanumeric() {
                if ch.is_ascii_uppercase() {
                    if i > 0 && !prev_is_sep {
                        out.push('_');
                    }
                    out.push(ch.to_ascii_lowercase());
                } else {
                    out.push(ch.to_ascii_lowercase());
                }
                prev_is_sep = false;
            } else if !prev_is_sep && !out.is_empty() {
                out.push('_');
                prev_is_sep = true;
            }
        }
        out.trim_matches('_').to_string()
    }
}

impl ChunkBuilder for TopologyBuilder {
    fn new() -> Self {
        Self {
            reference_builder: D3ChunkBuilder::new(),
            cached_scene: None,
        }
    }

    fn build(
        &mut self,
        map: &Map,
        assets: &Assets,
        chunk: &mut Chunk,
        vmchunk: &mut scenevm::Chunk,
    ) {
        let (scene, filtered, replace_sector_ids, detail_sector_ids, detail_linedef_ids) = {
            let scene = self.scene_for_map(map);
            let mut filtered = scene.filtered_map_for_chunk(map, &chunk.bbox);
            let replace_sector_ids = scene.builder_replace_surface_sector_ids(&chunk.bbox);
            let detail_sector_ids = scene.builder_detail_sector_ids(&chunk.bbox);
            let detail_linedef_ids = scene.builder_wall_detail_linedef_ids(map, &chunk.bbox);
            if !scene.builder_cuts.is_empty()
                || !scene.builder_details.is_empty()
                || !scene.builder_wall_details.is_empty()
            {
                eprintln!(
                    "[BuilderGraphDebug][build] chunk=({}, {}) scene cuts={} details={} wall_details={} replace sectors={:?} detail sectors={:?} detail linedefs={:?} filtered sectors before={}",
                    chunk.origin.x,
                    chunk.origin.y,
                    scene.builder_cuts.len(),
                    scene.builder_details.len(),
                    scene.builder_wall_details.len(),
                    replace_sector_ids,
                    detail_sector_ids,
                    detail_linedef_ids,
                    filtered.sectors.len()
                );
            }
            scene.suppress_replace_surface_sectors(&mut filtered, &replace_sector_ids);
            if !scene.builder_cuts.is_empty()
                || !scene.builder_details.is_empty()
                || !scene.builder_wall_details.is_empty()
            {
                eprintln!(
                    "[BuilderGraphDebug][build] chunk=({}, {}) filtered sectors after={}",
                    chunk.origin.x,
                    chunk.origin.y,
                    filtered.sectors.len()
                );
            }
            (
                scene.clone(),
                filtered,
                replace_sector_ids,
                detail_sector_ids,
                detail_linedef_ids,
            )
        };

        Self::emit_builder_replace_surface_patches(
            &scene,
            map,
            assets,
            chunk,
            vmchunk,
            &replace_sector_ids,
        );
        Self::emit_builder_surface_detail_patches(
            &scene,
            map,
            assets,
            chunk,
            vmchunk,
            &detail_sector_ids,
        );
        Self::emit_builder_freestanding_sector_details(
            &scene,
            map,
            assets,
            chunk,
            vmchunk,
            &detail_sector_ids,
        );
        Self::emit_builder_wall_detail_batches(
            &scene,
            map,
            assets,
            chunk,
            vmchunk,
            &detail_linedef_ids,
        );

        self.reference_builder
            .build(&filtered, assets, chunk, vmchunk);
    }

    fn build_collision(
        &mut self,
        map: &Map,
        assets: &Assets,
        chunk_origin: Vec2<i32>,
        chunk_size: i32,
    ) -> ChunkCollision {
        let _ = self.scene_for_map(map);
        self.reference_builder
            .build_collision(map, assets, chunk_origin, chunk_size)
    }

    fn boxed_clone(&self) -> Box<dyn ChunkBuilder> {
        Box::new(self.clone())
    }
}

fn append_mesh_vertex(
    vertices: &mut Vec<[f32; 4]>,
    uvs: &mut Vec<[f32; 2]>,
    p: Vec3<f32>,
) -> usize {
    let idx = vertices.len();
    vertices.push([p.x, p.y, p.z, 1.0]);
    uvs.push([p.x, p.z]);
    idx
}

fn append_mesh_tri(indices: &mut Vec<(usize, usize, usize)>, a: usize, b: usize, c: usize) {
    indices.push((a, b, c));
}

fn append_mesh_quad_reversed(
    indices: &mut Vec<(usize, usize, usize)>,
    a: usize,
    b: usize,
    c: usize,
    d: usize,
) {
    append_mesh_tri(indices, a, d, c);
    append_mesh_tri(indices, a, c, b);
}

fn append_oriented_box(
    vertices: &mut Vec<[f32; 4]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<(usize, usize, usize)>,
    center: Vec3<f32>,
    size: Vec3<f32>,
    along: Vec3<f32>,
    up: Vec3<f32>,
    outward: Vec3<f32>,
) {
    let hx = size.x * 0.5;
    let hy = size.y * 0.5;
    let hz = size.z * 0.5;
    let local = [
        Vec3::new(-hx, -hy, -hz),
        Vec3::new(hx, -hy, -hz),
        Vec3::new(hx, -hy, hz),
        Vec3::new(-hx, -hy, hz),
        Vec3::new(-hx, hy, -hz),
        Vec3::new(hx, hy, -hz),
        Vec3::new(hx, hy, hz),
        Vec3::new(-hx, hy, hz),
    ];
    let mut ids = [0usize; 8];
    for (index, local) in local.iter().enumerate() {
        let world = center + along * local.x + up * local.y + outward * local.z;
        ids[index] = append_mesh_vertex(vertices, uvs, world);
    }
    for (a, b, c, d) in [
        (0usize, 1usize, 5usize, 4usize),
        (1usize, 2usize, 6usize, 5usize),
        (2usize, 3usize, 7usize, 6usize),
        (3usize, 0usize, 4usize, 7usize),
        (4usize, 5usize, 6usize, 7usize),
        (0usize, 3usize, 2usize, 1usize),
    ] {
        append_mesh_quad_reversed(indices, ids[a], ids[b], ids[c], ids[d]);
    }
}

fn append_oriented_cylinder(
    vertices: &mut Vec<[f32; 4]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<(usize, usize, usize)>,
    center: Vec3<f32>,
    length: f32,
    radius: f32,
    segments: usize,
    along: Vec3<f32>,
    up: Vec3<f32>,
    outward: Vec3<f32>,
) {
    let segments = segments.max(6);
    let half = length * 0.5;
    let mut bottom = Vec::with_capacity(segments);
    let mut top = Vec::with_capacity(segments);
    for index in 0..segments {
        let angle = index as f32 / segments as f32 * std::f32::consts::TAU;
        let radial = along * (angle.cos() * radius) + outward * (angle.sin() * radius);
        bottom.push(append_mesh_vertex(
            vertices,
            uvs,
            center - up * half + radial,
        ));
        top.push(append_mesh_vertex(
            vertices,
            uvs,
            center + up * half + radial,
        ));
    }
    for index in 0..segments {
        let next = (index + 1) % segments;
        append_mesh_quad_reversed(indices, bottom[index], top[index], top[next], bottom[next]);
    }
    let bottom_center = append_mesh_vertex(vertices, uvs, center - up * half);
    let top_center = append_mesh_vertex(vertices, uvs, center + up * half);
    for index in 0..segments {
        let next = (index + 1) % segments;
        append_mesh_tri(indices, bottom_center, bottom[next], bottom[index]);
        append_mesh_tri(indices, top_center, top[index], top[next]);
    }
}

fn append_freestanding_column_mesh(
    vertices: &mut Vec<[f32; 4]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<(usize, usize, usize)>,
    anchor: Vec3<f32>,
    height: f32,
    radius: f32,
    base_height: f32,
    cap_height: f32,
    segments: usize,
    along: Vec3<f32>,
    up: Vec3<f32>,
    outward: Vec3<f32>,
) {
    let shaft_height = (height - base_height - cap_height).max(0.02);
    if base_height > 0.0 {
        append_oriented_box(
            vertices,
            uvs,
            indices,
            anchor + up * (base_height * 0.5),
            Vec3::new(radius * 2.9, base_height, radius * 2.9),
            along,
            up,
            outward,
        );
    }
    append_oriented_cylinder(
        vertices,
        uvs,
        indices,
        anchor + up * (base_height + shaft_height * 0.5),
        shaft_height,
        radius,
        segments,
        along,
        up,
        outward,
    );
    if cap_height > 0.0 {
        append_oriented_box(
            vertices,
            uvs,
            indices,
            anchor + up * (base_height + shaft_height + cap_height * 0.5),
            Vec3::new(radius * 2.9, cap_height, radius * 2.9),
            along,
            up,
            outward,
        );
    }
}

fn masonry_block_rects(
    min: Vec2<f32>,
    max: Vec2<f32>,
    block: Vec2<f32>,
    mortar: f32,
    pattern: BuilderMasonryPattern,
) -> Vec<(Vec2<f32>, Vec2<f32>)> {
    let raw_min = min;
    let raw_max = max;
    let min = Vec2::new(raw_min.x.min(raw_max.x), raw_min.y.min(raw_max.y));
    let max = Vec2::new(raw_min.x.max(raw_max.x), raw_min.y.max(raw_max.y));
    let block = Vec2::new(block.x.max(0.001), block.y.max(0.001));
    let mortar = mortar.max(0.0);
    let edge_margin = (mortar * 0.5).max(0.002);
    let gap = mortar * 0.5;
    let area_min = min + Vec2::broadcast(edge_margin);
    let area_max = max - Vec2::broadcast(edge_margin);
    if area_max.x <= area_min.x || area_max.y <= area_min.y {
        return Vec::new();
    }

    let mut rects = Vec::new();
    let mut row = 0usize;
    let mut y0 = area_min.y;
    while y0 < area_max.y - 0.001 && row < 512 {
        let y1 = (y0 + block.y).min(area_max.y);
        let row_offset = match pattern {
            BuilderMasonryPattern::Grid => 0.0,
            BuilderMasonryPattern::RunningBond if row % 2 == 1 => block.x * 0.5,
            BuilderMasonryPattern::RunningBond => 0.0,
        };
        let mut x0 = area_min.x - row_offset;
        let mut col = 0usize;
        while x0 < area_max.x - 0.001 && col < 1024 {
            let x1 = x0 + block.x;
            let clipped_min = Vec2::new(x0.max(area_min.x) + gap, y0 + gap);
            let clipped_max = Vec2::new(x1.min(area_max.x) - gap, y1 - gap);
            if clipped_max.x > clipped_min.x + 0.001 && clipped_max.y > clipped_min.y + 0.001 {
                rects.push((clipped_min, clipped_max));
            }
            x0 += block.x;
            col += 1;
        }
        y0 += block.y;
        row += 1;
    }

    rects
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Linedef, Sector, Surface, Value, Vertex};

    fn map_with_builder_cut() -> Map {
        let mut map = Map::default();
        map.vertices = vec![
            Vertex::new_3d(1, 0.0, 0.0, 0.0),
            Vertex::new_3d(2, 4.0, 0.0, 0.0),
            Vertex::new_3d(3, 4.0, 4.0, 0.0),
            Vertex::new_3d(4, 0.0, 4.0, 0.0),
        ];

        let mut linedefs = vec![
            Linedef::new(1, 1, 2),
            Linedef::new(2, 2, 3),
            Linedef::new(3, 3, 4),
            Linedef::new(4, 4, 1),
        ];
        for linedef in &mut linedefs {
            linedef.sector_ids.push(1);
        }
        map.linedefs = linedefs;

        let mut sector = Sector::new(1, vec![1, 2, 3, 4]);
        sector.properties.set(
            "builder_graph_data",
            Value::Str(
                r#"
name = "Cut Test";
host = sector;

cut rect {
    min = vec2(host.width * 0.25, host.depth * 0.25);
    max = vec2(host.width * 0.75, host.depth * 0.75);
    mode = replace;
};

output = [];
"#
                .to_string(),
            ),
        );
        map.sectors.push(sector);

        let mut surface = Surface::new(1);
        surface.calculate_geometry(&map);
        map.surfaces.insert(surface.id, surface);

        map
    }

    fn map_with_freestanding_column_footprint() -> Map {
        let mut map = Map::default();
        map.vertices = vec![
            Vertex::new_3d(1, 0.0, 0.0, 0.0),
            Vertex::new_3d(2, 4.0, 0.0, 0.0),
            Vertex::new_3d(3, 4.0, 4.0, 0.0),
            Vertex::new_3d(4, 0.0, 4.0, 0.0),
        ];

        let mut linedefs = vec![
            Linedef::new(1, 1, 2),
            Linedef::new(2, 2, 3),
            Linedef::new(3, 3, 4),
            Linedef::new(4, 4, 1),
        ];
        for linedef in &mut linedefs {
            linedef.sector_ids.push(1);
        }
        map.linedefs = linedefs;

        let mut sector = Sector::new(1, vec![1, 2, 3, 4]);
        sector.properties.set(
            "builder_graph_data",
            Value::Str(
                r#"
name = "Footprint Test";
host = sector;

detail column {
    placement = freestanding;
    center = vec2(host.width * 0.5, host.depth * 0.5);
    height = 1.4;
    radius = 0.2;
    base = 0.1;
    cap = 0.1;
    cut_footprint = true;
    material = COLUMN;
};

output = [];
"#
                .to_string(),
            ),
        );
        map.sectors.push(sector);

        let mut surface = Surface::new(1);
        surface.calculate_geometry(&map);
        map.surfaces.insert(surface.id, surface);

        map
    }

    #[test]
    fn topology_scene_collects_builder_cut_masks() {
        let map = map_with_builder_cut();
        let scene = TopologyScene::build(&map);
        let cuts = scene.builder_cuts_for_sector(1).collect::<Vec<_>>();

        assert_eq!(cuts.len(), 1);
        assert_eq!(cuts[0].sector_id, 1);
        assert!(cuts[0].surface_id.is_some());

        let BuilderCutMask::Rect { mode, .. } = &cuts[0].mask else {
            panic!("expected rect cut");
        };
        assert_eq!(*mode, buildergraph::BuilderCutMode::Replace);

        let resolved = scene.resolved_builder_cuts_for_sector(1);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].mode, buildergraph::BuilderCutMode::Replace);
        assert_eq!(resolved[0].loop_uv.len(), 4);
        assert_eq!(resolved[0].loop_uv[0], Vec2::new(-1.0, -1.0));
        assert_eq!(resolved[0].loop_uv[2], Vec2::new(1.0, 1.0));

        let surface_id = cuts[0]
            .surface_id
            .expect("cut should be bound to a surface");
        let patch = scene
            .surface_patch_descriptor(&map, 1, surface_id)
            .expect("surface patch should build");
        let Some(MeshTopology::FilledRegion { outer, holes }) = &patch.descriptor.cap else {
            panic!("expected filled-region cap");
        };
        assert_eq!(outer.len(), 4);
        assert_eq!(holes.len(), 1);
        assert_eq!(holes[0].len(), 4);

        let surface = map.surfaces.get(&surface_id).expect("surface exists");
        let meshes = SurfaceMeshBuilder::new(surface).build(&patch.descriptor);
        assert_eq!(meshes.len(), 1);
        assert!(!meshes[0].indices.is_empty());
    }

    #[test]
    fn topology_scene_identifies_replace_surface_sectors() {
        let map = map_with_builder_cut();
        let scene = TopologyScene::build(&map);
        let sector_ids = scene.builder_replace_surface_sector_ids(&BBox::new(
            Vec2::new(-1.0, -1.0),
            Vec2::new(5.0, 5.0),
        ));

        assert!(sector_ids.contains(&1));

        let mut filtered = map.clone();
        scene.suppress_replace_surface_sectors(&mut filtered, &sector_ids);
        assert!(filtered.find_sector(1).is_none());
        assert!(
            filtered
                .surfaces
                .values()
                .all(|surface| surface.sector_id != 1)
        );
    }

    #[test]
    fn freestanding_column_footprint_rebuilds_surface_with_hole() {
        let map = map_with_freestanding_column_footprint();
        let scene = TopologyScene::build(&map);
        let sector_ids = scene.builder_replace_surface_sector_ids(&BBox::new(
            Vec2::new(-1.0, -1.0),
            Vec2::new(5.0, 5.0),
        ));
        assert!(sector_ids.contains(&1));

        let surface_id = *scene
            .sectors
            .get(&1)
            .expect("topology sector")
            .surfaces
            .iter()
            .next()
            .expect("surface id");
        let patch = scene
            .surface_patch_descriptor(&map, 1, surface_id)
            .expect("surface patch");
        let Some(MeshTopology::FilledRegion { holes, .. }) = &patch.descriptor.cap else {
            panic!("expected filled-region cap");
        };
        assert_eq!(holes.len(), 1);
        assert_eq!(holes[0].len(), 4);
    }

    #[test]
    fn masonry_blocks_are_inset_and_staggered() {
        let rects = masonry_block_rects(
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 1.0),
            Vec2::new(0.5, 0.25),
            0.04,
            BuilderMasonryPattern::RunningBond,
        );

        assert!(!rects.is_empty());
        assert!(rects.iter().all(|(min, max)| {
            min.x > 0.0 && min.y > 0.0 && max.x < 2.0 && max.y < 1.0 && max.x > min.x
        }));

        let first_row_width = rects[0].1.x - rects[0].0.x;
        let second_row_width = rects
            .iter()
            .find(|(min, _)| min.y > 0.25)
            .map(|(min, max)| max.x - min.x)
            .expect("expected a second masonry row");
        assert!(
            second_row_width < first_row_width,
            "running bond should start every other row with a clipped half block"
        );
    }
}
