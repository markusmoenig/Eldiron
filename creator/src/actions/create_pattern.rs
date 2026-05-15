use crate::editor::RUSTERIX;
use crate::prelude::*;
use rusterix::PixelSource;
use theframework::prelude::FxHashMap;

const MODE_ID: &str = "actionMode";
const PATTERN_ID: &str = "actionPattern";
const SEQUENCE_ID: &str = "actionSequence";
const REPEAT_ID: &str = "actionRepeat";
const INTERLEAVE_ID: &str = "actionInterleave";
const SCALE_ID: &str = "actionScale";
const SPACING_X_ID: &str = "actionSpacingX";
const SPACING_Y_ID: &str = "actionSpacingY";
const ROTATION_ID: &str = "actionRotation";
const MARGIN_ID: &str = "actionMargin";
const SIDES_ID: &str = "actionSides";
const ROUNDNESS_ID: &str = "actionRoundness";
const JITTER_ID: &str = "actionJitter";
const SEED_ID: &str = "actionSeed";
const FIT_ROWS_ID: &str = "actionFitRows";
const FIT_COLUMNS_ID: &str = "actionFitColumns";
const RELIEF_HEIGHT_ID: &str = "actionReliefHeight";
const RELIEF_HEIGHT_JITTER_ID: &str = "actionReliefHeightJitter";
const RELIEF_DOME_ID: &str = "actionReliefDome";
const RELIEF_EDGE_DEPTH_ID: &str = "actionReliefEdgeDepth";
const RELIEF_COLOR_JITTER_ID: &str = "actionReliefColorJitter";

pub struct CreatePattern {
    id: TheId,
    nodeui: TheNodeUI,
    pattern_source_override: Option<Option<PixelSource>>,
    background_source_override: Option<Option<PixelSource>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PatternMode {
    Guide,
    Relief,
}

impl PatternMode {
    fn from_index(index: i32) -> Self {
        match index {
            1 => Self::Relief,
            _ => Self::Guide,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PatternShape {
    Disc,
    Triangle,
    Quad,
    Line,
    Tile,
    Cobble,
}

#[derive(Clone, Debug)]
struct FacePlane {
    origin: Vec3<f32>,
    axis_u: Vec3<f32>,
    axis_v: Vec3<f32>,
    polygon: Vec<Vec2<f32>>,
    min: Vec2<f32>,
    max: Vec2<f32>,
    normal: Vec3<f32>,
}

#[derive(Clone, Debug)]
struct PatternSettings {
    mode: PatternMode,
    shapes: Vec<PatternShape>,
    repeat: bool,
    interleave: bool,
    scale: f32,
    spacing_x: f32,
    spacing_y: f32,
    rotation: f32,
    margin: f32,
    sides: usize,
    roundness: f32,
    jitter: f32,
    seed: u32,
    rows: usize,
    columns: usize,
    relief: ReliefSettings,
}

#[derive(Clone, Debug)]
struct ReliefSettings {
    height: f32,
    height_jitter: f32,
    dome: f32,
    edge_depth: f32,
    color_jitter: f32,
}

#[derive(Clone, Debug)]
struct PatternStamp {
    row: usize,
    column: usize,
    center: Vec2<f32>,
    points: Vec<Vec2<f32>>,
    closed: bool,
}

impl PatternShape {
    fn from_index(index: i32) -> Self {
        match index {
            1 => Self::Triangle,
            2 => Self::Quad,
            3 => Self::Line,
            4 => Self::Tile,
            5 => Self::Cobble,
            _ => Self::Disc,
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "disc" | "circle" => Some(Self::Disc),
            "tri" | "triangle" => Some(Self::Triangle),
            "quad" | "rect" | "rectangle" | "square" => Some(Self::Quad),
            "line" => Some(Self::Line),
            "tile" => Some(Self::Tile),
            "cobble" | "cobbles" | "cobblestone" | "cobblestones" => Some(Self::Cobble),
            _ => None,
        }
    }
}

fn selected_face_ids(map: &Map, server_ctx: &ServerContext) -> Vec<(Uuid, usize)> {
    if server_ctx.get_map_context() != MapContext::Region
        || server_ctx.editor_view_mode == EditorViewMode::D2
    {
        return Vec::new();
    }
    map.selected_geometry_faces.clone()
}

fn geometry_face_plane(
    object: &rusterix::GeometryObject,
    face: &rusterix::GeometryFace,
) -> Option<FacePlane> {
    if face.indices.len() < 3 {
        return None;
    }

    let world_vertices = face
        .indices
        .iter()
        .filter_map(|index| {
            object
                .vertices
                .get(*index)
                .map(|vertex| object.transform_point(*vertex))
        })
        .collect::<Vec<_>>();
    if world_vertices.len() != face.indices.len() {
        return None;
    }

    let origin = *world_vertices.first()?;
    let mut axis_u = None;
    for point in world_vertices.iter().skip(1) {
        let edge = *point - origin;
        if edge.magnitude_squared() > 1e-6 {
            axis_u = edge.try_normalized();
            break;
        }
    }
    let axis_u = axis_u?;

    let mut normal = Vec3::<f32>::zero();
    for index in 1..world_vertices.len().saturating_sub(1) {
        let a = world_vertices[index] - origin;
        let b = world_vertices[index + 1] - origin;
        normal += a.cross(b);
    }
    // Match the existing ridge/groove convention: surface relief grows out of
    // the editable face, which is opposite the raw geometry winding normal.
    let normal = -normal.try_normalized()?;
    let axis_v = normal.cross(axis_u).try_normalized()?;

    let mut polygon = Vec::with_capacity(face.indices.len());
    let mut min = Vec2::broadcast(f32::INFINITY);
    let mut max = Vec2::broadcast(f32::NEG_INFINITY);
    for point in &world_vertices {
        let rel = *point - origin;
        let projected = Vec2::new(rel.dot(axis_u), rel.dot(axis_v));
        min.x = min.x.min(projected.x);
        min.y = min.y.min(projected.y);
        max.x = max.x.max(projected.x);
        max.y = max.y.max(projected.y);
        polygon.push(projected);
    }

    Some(FacePlane {
        origin,
        axis_u,
        axis_v,
        polygon,
        min,
        max,
        normal,
    })
}

fn point_in_polygon(point: Vec2<f32>, polygon: &[Vec2<f32>]) -> bool {
    if polygon.len() < 3 {
        return false;
    }

    let mut inside = false;
    let mut previous = polygon.len() - 1;
    for current in 0..polygon.len() {
        let a = polygon[current];
        let b = polygon[previous];
        let edge = b - a;
        let len_sq = edge.dot(edge);
        if len_sq > 1e-6 {
            let t = ((point - a).dot(edge) / len_sq).clamp(0.0, 1.0);
            if (point - (a + edge * t)).magnitude_squared() <= 1e-6 {
                return true;
            }
        }
        if (a.y > point.y) != (b.y > point.y) {
            let x = (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x;
            if point.x < x {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

fn pattern_settings(nodeui: &TheNodeUI) -> PatternSettings {
    let sequence = nodeui.get_text_value(SEQUENCE_ID).unwrap_or_default();
    let mut shapes = sequence
        .split([',', ';', '\n'])
        .filter_map(PatternShape::parse)
        .collect::<Vec<_>>();
    if shapes.is_empty() {
        shapes.push(PatternShape::from_index(
            nodeui.get_i32_value(PATTERN_ID).unwrap_or(0),
        ));
    }

    let scale = nodeui.get_f32_value(SCALE_ID).unwrap_or(1.0).max(0.01);
    PatternSettings {
        mode: PatternMode::from_index(nodeui.get_i32_value(MODE_ID).unwrap_or(0)),
        shapes,
        repeat: nodeui.get_bool_value(REPEAT_ID).unwrap_or(false),
        interleave: nodeui.get_bool_value(INTERLEAVE_ID).unwrap_or(false),
        scale,
        spacing_x: nodeui
            .get_f32_value(SPACING_X_ID)
            .unwrap_or(scale)
            .max(0.01),
        spacing_y: nodeui
            .get_f32_value(SPACING_Y_ID)
            .unwrap_or(scale)
            .max(0.01),
        rotation: nodeui
            .get_f32_value(ROTATION_ID)
            .unwrap_or(0.0)
            .to_radians(),
        margin: nodeui.get_f32_value(MARGIN_ID).unwrap_or(0.0).max(0.0),
        sides: nodeui.get_i32_value(SIDES_ID).unwrap_or(16).clamp(3, 64) as usize,
        roundness: nodeui
            .get_f32_value(ROUNDNESS_ID)
            .unwrap_or(0.65)
            .clamp(0.0, 1.0),
        jitter: nodeui
            .get_f32_value(JITTER_ID)
            .unwrap_or(0.15)
            .clamp(0.0, 0.95),
        seed: nodeui.get_i32_value(SEED_ID).unwrap_or(0).max(0) as u32,
        rows: nodeui.get_i32_value(FIT_ROWS_ID).unwrap_or(0).max(0) as usize,
        columns: nodeui.get_i32_value(FIT_COLUMNS_ID).unwrap_or(0).max(0) as usize,
        relief: ReliefSettings {
            height: nodeui
                .get_f32_value(RELIEF_HEIGHT_ID)
                .unwrap_or(0.08)
                .max(0.0),
            height_jitter: nodeui
                .get_f32_value(RELIEF_HEIGHT_JITTER_ID)
                .unwrap_or(0.02)
                .max(0.0),
            dome: nodeui
                .get_f32_value(RELIEF_DOME_ID)
                .unwrap_or(0.4)
                .clamp(0.0, 1.0),
            edge_depth: nodeui.get_f32_value(RELIEF_EDGE_DEPTH_ID).unwrap_or(0.0),
            color_jitter: nodeui
                .get_f32_value(RELIEF_COLOR_JITTER_ID)
                .unwrap_or(0.0)
                .clamp(0.0, 1.0),
        },
    }
}

fn rotate(point: Vec2<f32>, radians: f32) -> Vec2<f32> {
    if radians.abs() <= 1e-6 {
        return point;
    }
    let (sin, cos) = radians.sin_cos();
    Vec2::new(point.x * cos - point.y * sin, point.x * sin + point.y * cos)
}

fn regular_polygon(sides: usize, radius: f32, rotation: f32) -> Vec<Vec2<f32>> {
    let sides = sides.max(3);
    (0..sides)
        .map(|index| {
            let angle = rotation + std::f32::consts::TAU * (index as f32 / sides as f32)
                - std::f32::consts::FRAC_PI_2;
            Vec2::new(angle.cos() * radius, angle.sin() * radius)
        })
        .collect()
}

fn rotate_stamp_around(stamp: &mut PatternStamp, pivot: Vec2<f32>, radians: f32) {
    if radians.abs() <= 1e-6 {
        return;
    }
    let old_center = stamp.center;
    stamp.center = pivot + rotate(stamp.center - pivot, radians);
    for point in &mut stamp.points {
        let absolute = old_center + *point;
        let rotated = pivot + rotate(absolute - pivot, radians);
        *point = rotated - stamp.center;
    }
}

fn hash_unit(seed: u32, row: usize, column: usize, salt: u32) -> f32 {
    let mut value = seed
        ^ ((row as u32).wrapping_mul(0x9E37_79B9))
        ^ ((column as u32).wrapping_mul(0x85EB_CA6B))
        ^ salt.wrapping_mul(0xC2B2_AE35);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7FEB_352D);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846C_A68B);
    value ^= value >> 16;
    (value as f32 / u32::MAX as f32) * 2.0 - 1.0
}

fn cobble_points(settings: &PatternSettings, row: usize, column: usize) -> Vec<Vec2<f32>> {
    let sides = settings.sides.max(8);
    let jitter = settings.jitter;
    let width_factor = (1.0 + hash_unit(settings.seed, row, column, 1) * jitter * 0.28).max(0.25);
    let height_factor = (1.0 + hash_unit(settings.seed, row, column, 2) * jitter * 0.28).max(0.25);
    let half_w = (settings.scale * 0.5 * width_factor).min(settings.spacing_x * 0.45);
    let half_h = (settings.scale * 0.5 * height_factor).min(settings.spacing_y * 0.45);
    let exponent = 8.0 - settings.roundness * 6.0;

    (0..sides)
        .map(|index| {
            let angle = std::f32::consts::TAU * (index as f32 / sides as f32);
            let (sin, cos) = angle.sin_cos();
            let wobble =
                1.0 + hash_unit(settings.seed, row, column, index as u32 + 10) * jitter * 0.18;
            Vec2::new(
                cos.signum() * cos.abs().powf(2.0 / exponent) * half_w * wobble,
                sin.signum() * sin.abs().powf(2.0 / exponent) * half_h * wobble,
            )
        })
        .collect()
}

fn stamp_points(
    shape: PatternShape,
    settings: &PatternSettings,
    row: usize,
    column: usize,
) -> Vec<Vec2<f32>> {
    let half = settings.scale * 0.5;
    match shape {
        PatternShape::Disc => regular_polygon(settings.sides, half, 0.0),
        PatternShape::Triangle => regular_polygon(3, half, 0.0),
        PatternShape::Quad | PatternShape::Tile => [
            Vec2::new(-half, -half),
            Vec2::new(half, -half),
            Vec2::new(half, half),
            Vec2::new(-half, half),
        ]
        .into_iter()
        .collect(),
        PatternShape::Line => [Vec2::new(-half, 0.0), Vec2::new(half, 0.0)]
            .into_iter()
            .collect(),
        PatternShape::Cobble => cobble_points(settings, row, column),
    }
}

fn append_stamp(
    face: &mut rusterix::GeometryFace,
    object_id: Uuid,
    face_index: usize,
    plane: &FacePlane,
    center: Vec2<f32>,
    points: &[Vec2<f32>],
    closed: bool,
    selected_points: &mut Vec<(Uuid, usize, usize)>,
    selected_segments: &mut Vec<(Uuid, usize, usize)>,
) -> bool {
    if points.is_empty() {
        return false;
    }
    if points
        .iter()
        .any(|point| !point_in_polygon(center + *point, &plane.polygon))
    {
        return false;
    }

    let start = face.surface_points.len();
    for point in points {
        let local = plane.origin
            + plane.axis_u * (center.x + point.x)
            + plane.axis_v * (center.y + point.y);
        face.surface_points.push(rusterix::GeometrySurfacePoint {
            position: local,
            mode: rusterix::GeometrySurfacePointMode::Corner,
        });
        selected_points.push((object_id, face_index, face.surface_points.len() - 1));
    }

    let segment_count = if closed {
        points.len()
    } else {
        points.len().saturating_sub(1)
    };
    for offset in 0..segment_count {
        face.surface_segments
            .push(rusterix::GeometrySurfaceSegment {
                start: start + offset,
                end: start + ((offset + 1) % points.len()),
                mode: rusterix::GeometrySurfaceSegmentMode::Line,
                curve_amount: 0.35,
            });
        selected_segments.push((object_id, face_index, face.surface_segments.len() - 1));
    }
    true
}

fn push_generated_face(
    object: &mut rusterix::GeometryObject,
    indices: Vec<usize>,
    tile: Option<PixelSource>,
) {
    let uvs = indices.iter().map(|_| Vec2::zero()).collect();
    object.faces.push(rusterix::GeometryFace {
        indices,
        uvs,
        auto_uv: true,
        texture_offset: Vec2::zero(),
        texture_scale: Vec2::broadcast(1.0),
        texture_rotation: 0.0,
        tile,
        tiles: FxHashMap::default(),
        surface_points: Vec::new(),
        surface_segments: Vec::new(),
        surface_noise: None,
    });
}

fn jittered_source(
    source: Option<&PixelSource>,
    amount: f32,
    seed: u32,
    row: usize,
    column: usize,
) -> Option<PixelSource> {
    match source.cloned() {
        Some(PixelSource::PaletteIndex(index)) if amount > 0.0 => {
            let delta = (hash_unit(seed, row, column, 300) * amount * 8.0).round() as i32;
            Some(PixelSource::PaletteIndex(
                (index as i32 + delta).clamp(0, 255) as u16,
            ))
        }
        other => other,
    }
}

fn append_relief_stamp(
    relief: &mut rusterix::GeometryObject,
    plane: &FacePlane,
    stamp: &PatternStamp,
    settings: &PatternSettings,
    source: Option<&PixelSource>,
) -> bool {
    if !stamp.closed || stamp.points.len() < 3 {
        return false;
    }
    if stamp
        .points
        .iter()
        .any(|point| !point_in_polygon(stamp.center + *point, &plane.polygon))
    {
        return false;
    }

    let height_variation =
        hash_unit(settings.seed, stamp.row, stamp.column, 200) * settings.relief.height_jitter;
    let height = (settings.relief.height + height_variation).max(0.0);
    let edge_height = height * (1.0 - settings.relief.dome);
    let base_height = settings.relief.edge_depth;
    let nudge = 0.003;
    let tile = jittered_source(
        source,
        settings.relief.color_jitter,
        settings.seed,
        stamp.row,
        stamp.column,
    );

    let base = relief.vertices.len();
    for point in &stamp.points {
        let planar = plane.origin
            + plane.axis_u * (stamp.center.x + point.x)
            + plane.axis_v * (stamp.center.y + point.y);
        relief
            .vertices
            .push(planar + plane.normal * (base_height + nudge));
    }
    for point in &stamp.points {
        let planar = plane.origin
            + plane.axis_u * (stamp.center.x + point.x)
            + plane.axis_v * (stamp.center.y + point.y);
        relief
            .vertices
            .push(planar + plane.normal * (edge_height + nudge));
    }
    let center_vertex = plane.origin
        + plane.axis_u * stamp.center.x
        + plane.axis_v * stamp.center.y
        + plane.normal * (height + nudge);
    let center_index = relief.vertices.len();
    relief.vertices.push(center_vertex);

    let count = stamp.points.len();
    for index in 0..count {
        let next = (index + 1) % count;
        push_generated_face(
            relief,
            vec![
                base + index,
                base + next,
                base + count + next,
                base + count + index,
            ],
            tile.clone(),
        );
        push_generated_face(
            relief,
            vec![base + count + index, base + count + next, center_index],
            tile.clone(),
        );
    }
    true
}

fn fit_count(available: f32, footprint: f32, spacing: f32, requested: usize) -> usize {
    if available + 1e-5 < footprint {
        return 0;
    }
    let auto = ((available - footprint) / spacing).floor().max(0.0) as usize + 1;
    if requested > 0 {
        requested.min(auto).max(1)
    } else {
        auto.max(1)
    }
}

fn pattern_stamps(plane: &FacePlane, settings: &PatternSettings) -> Vec<PatternStamp> {
    let available = (plane.max - plane.min) - Vec2::broadcast(settings.margin * 2.0);
    if available.x <= 0.0 || available.y <= 0.0 {
        return Vec::new();
    }

    let footprint = settings.scale;
    let (rows, columns) = if settings.repeat {
        (
            fit_count(available.y, footprint, settings.spacing_y, settings.rows),
            fit_count(available.x, footprint, settings.spacing_x, settings.columns),
        )
    } else {
        (1, 1)
    };
    if rows == 0 || columns == 0 {
        return Vec::new();
    }

    let total_width = footprint + (columns.saturating_sub(1) as f32) * settings.spacing_x;
    let total_height = footprint + (rows.saturating_sub(1) as f32) * settings.spacing_y;
    let start_x =
        plane.min.x + settings.margin + (available.x - total_width) * 0.5 + footprint * 0.5;
    let start_y =
        plane.min.y + settings.margin + (available.y - total_height) * 0.5 + footprint * 0.5;
    let pattern_center = Vec2::new(
        start_x + (columns.saturating_sub(1) as f32) * settings.spacing_x * 0.5,
        start_y + (rows.saturating_sub(1) as f32) * settings.spacing_y * 0.5,
    );

    let mut stamps = Vec::new();
    for row in 0..rows {
        for column in 0..columns {
            let sequence_index = row * columns + column;
            let shape = settings.shapes[sequence_index % settings.shapes.len()];
            let mut center = Vec2::new(
                start_x + column as f32 * settings.spacing_x,
                start_y + row as f32 * settings.spacing_y,
            );
            if settings.interleave && settings.repeat && row % 2 == 1 {
                center.x += settings.spacing_x * 0.5;
            }
            if shape == PatternShape::Cobble {
                let gap_x = (settings.spacing_x - settings.scale).max(0.0);
                let gap_y = (settings.spacing_y - settings.scale).max(0.0);
                center.x +=
                    hash_unit(settings.seed, row, column, 100) * settings.jitter * gap_x * 0.35;
                center.y +=
                    hash_unit(settings.seed, row, column, 101) * settings.jitter * gap_y * 0.35;
            }
            let mut stamp = PatternStamp {
                row,
                column,
                center,
                points: stamp_points(shape, settings, row, column),
                closed: shape != PatternShape::Line,
            };
            rotate_stamp_around(&mut stamp, pattern_center, settings.rotation);
            stamps.push(stamp);
        }
    }
    stamps
}

fn append_tile_grid_pattern(
    face: &mut rusterix::GeometryFace,
    object_id: Uuid,
    face_index: usize,
    plane: &FacePlane,
    settings: &PatternSettings,
    selected_points: &mut Vec<(Uuid, usize, usize)>,
    selected_segments: &mut Vec<(Uuid, usize, usize)>,
    stagger: bool,
) -> usize {
    let available = (plane.max - plane.min) - Vec2::broadcast(settings.margin * 2.0);
    if available.x <= 0.0 || available.y <= 0.0 {
        return 0;
    }

    let cell_width = settings.spacing_x.max(settings.scale).max(0.01);
    let cell_height = settings.spacing_y.max(0.01);
    let columns = if settings.columns > 0 {
        settings
            .columns
            .min((available.x / cell_width).floor().max(1.0) as usize)
    } else {
        (available.x / cell_width).floor().max(1.0) as usize
    };
    let rows = if settings.rows > 0 {
        settings
            .rows
            .min((available.y / cell_height).floor().max(1.0) as usize)
    } else {
        (available.y / cell_height).floor().max(1.0) as usize
    };
    if rows == 0 || columns == 0 {
        return 0;
    }

    let total_width = columns as f32 * cell_width;
    let total_height = rows as f32 * cell_height;
    let min_x = plane.min.x + settings.margin + (available.x - total_width) * 0.5;
    let max_x = min_x + total_width;
    let min_y = plane.min.y + settings.margin + (available.y - total_height) * 0.5;
    let max_y = min_y + total_height;
    let pattern_center = Vec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
    let mut added = 0usize;

    let rotate_grid_points = |points: &mut [Vec2<f32>]| {
        if settings.rotation.abs() > 1e-6 {
            for point in points {
                *point = pattern_center + rotate(*point - pattern_center, settings.rotation);
            }
        }
    };

    for row in 0..=rows {
        let y = min_y + row as f32 * cell_height;
        let mut points = [Vec2::new(min_x, y), Vec2::new(max_x, y)];
        rotate_grid_points(&mut points);
        if append_stamp(
            face,
            object_id,
            face_index,
            plane,
            Vec2::zero(),
            &points,
            false,
            selected_points,
            selected_segments,
        ) {
            added += 1;
        }
    }

    for row in 0..rows {
        let y0 = min_y + row as f32 * cell_height;
        let y1 = (y0 + cell_height).min(max_y);
        let offset = if stagger && row % 2 == 1 {
            cell_width * 0.5
        } else {
            0.0
        };
        let first = if offset > 0.0 { -1isize } else { 0isize };
        for column in first..=columns as isize {
            let x = min_x + column as f32 * cell_width + offset;
            if x < min_x - 1e-5 || x > max_x + 1e-5 {
                continue;
            }
            let mut points = [
                Vec2::new(x.clamp(min_x, max_x), y0),
                Vec2::new(x.clamp(min_x, max_x), y1),
            ];
            rotate_grid_points(&mut points);
            if append_stamp(
                face,
                object_id,
                face_index,
                plane,
                Vec2::zero(),
                &points,
                false,
                selected_points,
                selected_segments,
            ) {
                added += 1;
            }
        }
    }

    added
}

fn create_pattern_on_face(
    object: &mut rusterix::GeometryObject,
    face_index: usize,
    settings: &PatternSettings,
) -> Option<(usize, Vec<(Uuid, usize, usize)>, Vec<(Uuid, usize, usize)>)> {
    let face_snapshot = object.faces.get(face_index)?.clone();
    let plane = geometry_face_plane(object, &face_snapshot)?;
    let object_id = object.id;
    let face = object.faces.get_mut(face_index)?;
    let mut added = 0usize;
    let mut selected_points = Vec::new();
    let mut selected_segments = Vec::new();

    if matches!(settings.shapes.first(), Some(PatternShape::Tile)) && settings.shapes.len() == 1 {
        added = append_tile_grid_pattern(
            face,
            object_id,
            face_index,
            &plane,
            settings,
            &mut selected_points,
            &mut selected_segments,
            settings.interleave,
        );
        return (added > 0).then_some((added, selected_points, selected_segments));
    }

    for stamp in pattern_stamps(&plane, settings) {
        if append_stamp(
            face,
            object_id,
            face_index,
            &plane,
            stamp.center,
            &stamp.points,
            stamp.closed,
            &mut selected_points,
            &mut selected_segments,
        ) {
            added += 1;
        }
    }

    (added > 0).then_some((added, selected_points, selected_segments))
}

fn create_relief_on_face(
    source_object: &rusterix::GeometryObject,
    face_index: usize,
    settings: &PatternSettings,
    pattern_source: Option<&PixelSource>,
) -> Option<rusterix::GeometryObject> {
    let face = source_object.faces.get(face_index)?;
    let plane = geometry_face_plane(source_object, face)?;
    let stamps = pattern_stamps(&plane, settings);
    if stamps.is_empty() {
        return None;
    }

    let mut relief = rusterix::GeometryObject::new("Pattern Relief");
    relief.kind = rusterix::GeometryObjectKind::Generated;
    relief.solid = false;

    let mut added = 0usize;
    for stamp in &stamps {
        if append_relief_stamp(&mut relief, &plane, stamp, settings, pattern_source) {
            added += 1;
        }
    }
    (added > 0).then_some(relief)
}

fn selected_face_source(map: &Map) -> Option<PixelSource> {
    map.selected_geometry_faces
        .first()
        .and_then(|(object_id, face_index)| {
            map.geometry_objects
                .iter()
                .find(|object| object.id == *object_id)
                .and_then(|object| object.faces.get(*face_index))
        })
        .and_then(|face| {
            face.tile
                .clone()
                .or_else(|| face.tiles.values().next().cloned())
        })
}

fn pattern_minimap_segments(
    object: &rusterix::GeometryObject,
    face_index: usize,
    settings: &PatternSettings,
) -> Vec<ActionMinimapSegment> {
    let Some(face) = object.faces.get(face_index) else {
        return Vec::new();
    };
    let Some(plane) = geometry_face_plane(object, face) else {
        return Vec::new();
    };

    let mut segments = Vec::new();
    if matches!(settings.shapes.first(), Some(PatternShape::Tile)) && settings.shapes.len() == 1 {
        let mut face_clone = face.clone();
        let mut selected_points = Vec::new();
        let mut selected_segments = Vec::new();
        append_tile_grid_pattern(
            &mut face_clone,
            object.id,
            face_index,
            &plane,
            settings,
            &mut selected_points,
            &mut selected_segments,
            settings.interleave,
        );
        for (_, _, segment_index) in selected_segments {
            let Some(segment) = face_clone.surface_segments.get(segment_index) else {
                continue;
            };
            let Some(start) = face_clone.surface_points.get(segment.start) else {
                continue;
            };
            let Some(end) = face_clone.surface_points.get(segment.end) else {
                continue;
            };
            let start = object.transform_point(start.position);
            let end = object.transform_point(end.position);
            segments.push(ActionMinimapSegment {
                start: Vec2::new(start.x, start.z),
                end: Vec2::new(end.x, end.z),
            });
        }
        return segments;
    }

    for stamp in pattern_stamps(&plane, settings) {
        let segment_count = if stamp.closed {
            stamp.points.len()
        } else {
            stamp.points.len().saturating_sub(1)
        };
        for index in 0..segment_count {
            let a = stamp.points[index];
            let b = stamp.points[(index + 1) % stamp.points.len()];
            let a = object.transform_point(
                plane.origin
                    + plane.axis_u * (stamp.center.x + a.x)
                    + plane.axis_v * (stamp.center.y + a.y),
            );
            let b = object.transform_point(
                plane.origin
                    + plane.axis_u * (stamp.center.x + b.x)
                    + plane.axis_v * (stamp.center.y + b.y),
            );
            segments.push(ActionMinimapSegment {
                start: Vec2::new(a.x, a.z),
                end: Vec2::new(b.x, b.z),
            });
        }
    }
    segments
}

fn build_nodeui() -> TheNodeUI {
    let mut nodeui = TheNodeUI::default();
    nodeui.add_item(TheNodeUIItem::Markdown(
        "desc".into(),
        fl!("action_create_pattern_desc"),
    ));
    nodeui.add_item(TheNodeUIItem::Selector(
        MODE_ID.into(),
        "Mode".into(),
        "".into(),
        vec!["guide".into(), "relief".into()],
        0,
    ));
    nodeui.add_item(TheNodeUIItem::Selector(
        PATTERN_ID.into(),
        "Pattern".into(),
        "Base pattern shape.".into(),
        vec![
            "disc".into(),
            "triangle".into(),
            "quad".into(),
            "line".into(),
            "tile".into(),
            "cobble".into(),
        ],
        0,
    ));
    nodeui.add_item(TheNodeUIItem::Text(
        SEQUENCE_ID.into(),
        "Sequence".into(),
        "Optional comma sequence, for example disc,triangle. Empty uses Pattern.".into(),
        String::new(),
        None,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::Checkbox(
        REPEAT_ID.into(),
        "Repeat".into(),
        "Repeat the pattern across the selected face.".into(),
        false,
    ));
    nodeui.add_item(TheNodeUIItem::Checkbox(
        INTERLEAVE_ID.into(),
        "Interleave".into(),
        "Offset every second repeated row by half the X spacing.".into(),
        false,
    ));
    nodeui.add_item(TheNodeUIItem::OpenTree("Shape".into()));
    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
        SCALE_ID.into(),
        "Scale".into(),
        "".into(),
        1.0,
        0.01..=128.0,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
        ROTATION_ID.into(),
        "Rotation".into(),
        "".into(),
        0.0,
        -360.0..=360.0,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
        MARGIN_ID.into(),
        "Margin".into(),
        "".into(),
        0.0,
        0.0..=128.0,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::IntEditSlider(
        SIDES_ID.into(),
        "Sides".into(),
        "".into(),
        16,
        3..=64,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
        ROUNDNESS_ID.into(),
        "Roundness".into(),
        "Used by cobble patterns.".into(),
        0.65,
        0.0..=1.0,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
        JITTER_ID.into(),
        "Jitter".into(),
        "Used by cobble patterns.".into(),
        0.15,
        0.0..=0.95,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::IntEditSlider(
        SEED_ID.into(),
        "Seed".into(),
        "Used by cobble patterns.".into(),
        0,
        0..=99999,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::CloseTree);
    nodeui.add_item(TheNodeUIItem::OpenTree("Spacing".into()));
    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
        SPACING_X_ID.into(),
        "X".into(),
        "".into(),
        1.0,
        0.01..=128.0,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
        SPACING_Y_ID.into(),
        "Y".into(),
        "".into(),
        1.0,
        0.01..=128.0,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::CloseTree);
    nodeui.add_item(TheNodeUIItem::OpenTree("Relief".into()));
    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
        RELIEF_HEIGHT_ID.into(),
        "Height".into(),
        "".into(),
        0.08,
        0.0..=32.0,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
        RELIEF_HEIGHT_JITTER_ID.into(),
        "Height Jitter".into(),
        "".into(),
        0.02,
        0.0..=32.0,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
        RELIEF_DOME_ID.into(),
        "Dome".into(),
        "".into(),
        0.4,
        0.0..=1.0,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
        RELIEF_EDGE_DEPTH_ID.into(),
        "Edge Depth".into(),
        "".into(),
        0.0,
        -32.0..=32.0,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
        RELIEF_COLOR_JITTER_ID.into(),
        "Color Jitter".into(),
        "".into(),
        0.0,
        0.0..=1.0,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::CloseTree);
    nodeui.add_item(TheNodeUIItem::OpenTree("Fit".into()));
    nodeui.add_item(TheNodeUIItem::IntEditSlider(
        FIT_ROWS_ID.into(),
        "Rows".into(),
        "0 means automatic.".into(),
        0,
        0..=512,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::IntEditSlider(
        FIT_COLUMNS_ID.into(),
        "Columns".into(),
        "0 means automatic.".into(),
        0,
        0..=512,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::CloseTree);
    nodeui
}

impl Action for CreatePattern {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named(&fl!("action_create_pattern")),
            nodeui: build_nodeui(),
            pattern_source_override: None,
            background_source_override: None,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_create_pattern_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        !selected_face_ids(map, server_ctx).is_empty()
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let selected_faces = selected_face_ids(map, server_ctx);
        if selected_faces.is_empty() {
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                fl!("status_create_pattern_needs_face"),
            ));
            return None;
        }

        let settings = pattern_settings(&self.nodeui);
        let prev = map.clone();
        let mut added = 0usize;
        let mut selected_points = Vec::new();
        let mut selected_segments = Vec::new();

        if settings.mode == PatternMode::Relief {
            let default_source = selected_face_source(map);
            let pattern_source = self
                .pattern_source_override
                .clone()
                .unwrap_or_else(|| default_source.clone());
            let background_source = self.background_source_override.clone().unwrap_or(None);
            let mut relief_objects = Vec::new();
            for (object_id, face_index) in &selected_faces {
                let Some(object) = map
                    .geometry_objects
                    .iter()
                    .find(|object| object.id == *object_id)
                else {
                    continue;
                };
                if let Some(relief) =
                    create_relief_on_face(object, *face_index, &settings, pattern_source.as_ref())
                {
                    added += relief.faces.len();
                    relief_objects.push(relief);
                }
            }

            if added > 0 {
                if let Some(background_source) = background_source {
                    for (object_id, face_index) in &selected_faces {
                        let Some(object) = map
                            .geometry_objects
                            .iter_mut()
                            .find(|object| object.id == *object_id)
                        else {
                            continue;
                        };
                        let Some(face) = object.faces.get_mut(*face_index) else {
                            continue;
                        };
                        face.tile = Some(background_source.clone());
                        face.tiles.clear();
                    }
                }

                map.clear_selection();
                for relief in relief_objects {
                    let id = relief.id;
                    map.selected_geometry_objects.push(id);
                    map.geometry_objects.push(relief);
                }
            }
        } else {
            for (object_id, face_index) in selected_faces {
                let Some(object) = map
                    .geometry_objects
                    .iter_mut()
                    .find(|object| object.id == object_id)
                else {
                    continue;
                };
                if let Some((count, points, segments)) =
                    create_pattern_on_face(object, face_index, &settings)
                {
                    added += count;
                    selected_points.extend(points);
                    selected_segments.extend(segments);
                }
            }
        }

        if added == 0 {
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                fl!("status_create_pattern_no_fit"),
            ));
            return None;
        }

        if settings.mode == PatternMode::Guide {
            map.selected_geometry_surface_points = selected_points;
            map.selected_geometry_surface_segments = selected_segments;
        }
        map.changed = map.changed.wrapping_add(1);
        RUSTERIX.write().unwrap().set_dirty();
        RUSTERIX.write().unwrap().set_overlay_dirty();
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Map Selection Changed"),
            TheValue::Empty,
        ));
        ctx.ui.send(TheEvent::SetStatusText(
            TheId::empty(),
            format!("{} {}", added, fl!("status_create_pattern_created")),
        ));
        Some(ProjectUndoAtom::MapEdit(
            server_ctx.pc,
            Box::new(prev),
            Box::new(map.clone()),
        ))
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        let _ = self.nodeui.handle_event(event);
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Minimap"),
            TheValue::Empty,
        ));
        // Pattern stamping appends editable guides, so parameter edits should not auto-apply.
        false
    }

    fn set_params_from_nodeui(&mut self, nodeui: TheNodeUI) -> bool {
        self.nodeui = nodeui;
        true
    }

    fn hud_material_slots(
        &self,
        map: &Map,
        server_ctx: &ServerContext,
    ) -> Option<Vec<ActionMaterialSlot>> {
        if server_ctx.get_map_context() != MapContext::Region
            || server_ctx.editor_view_mode == EditorViewMode::D2
        {
            return None;
        }
        let default_source = selected_face_source(map);
        Some(vec![
            ActionMaterialSlot {
                label: "PAT".to_string(),
                source: self
                    .pattern_source_override
                    .clone()
                    .unwrap_or_else(|| default_source.clone()),
            },
            ActionMaterialSlot {
                label: "BACK".to_string(),
                source: self
                    .background_source_override
                    .clone()
                    .unwrap_or(default_source),
            },
        ])
    }

    fn set_hud_material_source(
        &mut self,
        _map: &Map,
        _server_ctx: &ServerContext,
        slot_index: i32,
        source: PixelSource,
    ) -> bool {
        let source = Some(source);
        match slot_index {
            0 if self.pattern_source_override.as_ref() != Some(&source) => {
                self.pattern_source_override = Some(source);
                true
            }
            1 if self.background_source_override.as_ref() != Some(&source) => {
                self.background_source_override = Some(source);
                true
            }
            _ => false,
        }
    }

    fn clear_hud_material_slot(
        &mut self,
        _map: &Map,
        _server_ctx: &ServerContext,
        slot_index: i32,
    ) -> bool {
        match slot_index {
            0 if self.pattern_source_override != Some(None) => {
                self.pattern_source_override = Some(None);
                true
            }
            1 if self.background_source_override != Some(None) => {
                self.background_source_override = Some(None);
                true
            }
            _ => false,
        }
    }

    fn preserves_hud_material_slots(&self) -> bool {
        true
    }

    fn minimap_preview_segments(
        &self,
        map: &Map,
        server_ctx: &ServerContext,
    ) -> Vec<ActionMinimapSegment> {
        if selected_face_ids(map, server_ctx).is_empty() {
            return Vec::new();
        }
        let settings = pattern_settings(&self.nodeui);
        let mut segments = Vec::new();
        for (object_id, face_index) in selected_face_ids(map, server_ctx) {
            let Some(object) = map
                .geometry_objects
                .iter()
                .find(|object| object.id == object_id)
            else {
                continue;
            };
            segments.extend(pattern_minimap_segments(object, face_index, &settings));
        }
        segments
    }

    fn uses_minimap_preview(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selected_box_top_face() -> (Map, Uuid, usize) {
        let mut map = Map::default();
        let object = rusterix::GeometryObject::box_from_bounds(
            "Box",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(4.0, 1.0, 4.0),
        );
        let object_id = object.id;
        map.geometry_objects.push(object);
        let face_index = 2;
        map.selected_geometry_faces.push((object_id, face_index));
        (map, object_id, face_index)
    }

    #[test]
    fn create_pattern_adds_centered_disc_surface_detail() {
        let (mut map, _object_id, face_index) = selected_box_top_face();
        let action = CreatePattern::new();
        let mut ui = TheUI::default();
        let mut ctx = TheContext::new(64, 64, 1.0);
        let mut server_ctx = ServerContext::default();
        server_ctx.pc = ProjectContext::Region(Uuid::new_v4());
        server_ctx.editor_view_mode = EditorViewMode::Iso;

        assert!(
            action
                .apply(&mut map, &mut ui, &mut ctx, &mut server_ctx)
                .is_some()
        );
        let face = &map.geometry_objects[0].faces[face_index];
        assert_eq!(face.surface_points.len(), 16);
        assert_eq!(face.surface_segments.len(), 16);
        assert_eq!(map.selected_geometry_surface_segments.len(), 16);
    }

    #[test]
    fn create_pattern_repeats_alternating_shapes() {
        let (mut map, _object_id, face_index) = selected_box_top_face();
        let mut action = CreatePattern::new();
        action
            .nodeui
            .set_text_value(SEQUENCE_ID, "disc,triangle".to_string());
        action.nodeui.set_bool_value(REPEAT_ID, true);
        action.nodeui.set_f32_value(SCALE_ID, 0.5);
        action.nodeui.set_f32_value(SPACING_X_ID, 1.0);
        action.nodeui.set_f32_value(SPACING_Y_ID, 1.0);
        action.nodeui.set_i32_value(FIT_ROWS_ID, 1);
        action.nodeui.set_i32_value(FIT_COLUMNS_ID, 2);

        let mut ui = TheUI::default();
        let mut ctx = TheContext::new(64, 64, 1.0);
        let mut server_ctx = ServerContext::default();
        server_ctx.pc = ProjectContext::Region(Uuid::new_v4());
        server_ctx.editor_view_mode = EditorViewMode::Iso;

        assert!(
            action
                .apply(&mut map, &mut ui, &mut ctx, &mut server_ctx)
                .is_some()
        );
        let face = &map.geometry_objects[0].faces[face_index];
        assert_eq!(face.surface_points.len(), 19);
        assert_eq!(face.surface_segments.len(), 19);
    }

    #[test]
    fn create_pattern_tile_interleave_generates_staggered_mortar_lines() {
        let (mut map, _object_id, face_index) = selected_box_top_face();
        let mut action = CreatePattern::new();
        action.nodeui.set_i32_value(PATTERN_ID, 4);
        action.nodeui.set_bool_value(REPEAT_ID, true);
        action.nodeui.set_bool_value(INTERLEAVE_ID, true);
        action.nodeui.set_f32_value(SCALE_ID, 1.0);
        action.nodeui.set_f32_value(SPACING_X_ID, 1.0);
        action.nodeui.set_f32_value(SPACING_Y_ID, 0.5);
        action.nodeui.set_i32_value(FIT_ROWS_ID, 2);
        action.nodeui.set_i32_value(FIT_COLUMNS_ID, 2);

        let mut ui = TheUI::default();
        let mut ctx = TheContext::new(64, 64, 1.0);
        let mut server_ctx = ServerContext::default();
        server_ctx.pc = ProjectContext::Region(Uuid::new_v4());
        server_ctx.editor_view_mode = EditorViewMode::Iso;

        assert!(
            action
                .apply(&mut map, &mut ui, &mut ctx, &mut server_ctx)
                .is_some()
        );
        let face = &map.geometry_objects[0].faces[face_index];
        assert_eq!(face.surface_segments.len(), 8);
        assert_eq!(map.selected_geometry_surface_segments.len(), 8);
    }

    #[test]
    fn create_pattern_tile_generates_unstaggered_grid() {
        let (mut map, _object_id, face_index) = selected_box_top_face();
        let mut action = CreatePattern::new();
        action.nodeui.set_i32_value(PATTERN_ID, 4);
        action.nodeui.set_bool_value(REPEAT_ID, true);
        action.nodeui.set_f32_value(SCALE_ID, 1.0);
        action.nodeui.set_f32_value(SPACING_X_ID, 1.0);
        action.nodeui.set_f32_value(SPACING_Y_ID, 0.5);
        action.nodeui.set_i32_value(FIT_ROWS_ID, 2);
        action.nodeui.set_i32_value(FIT_COLUMNS_ID, 2);

        let mut ui = TheUI::default();
        let mut ctx = TheContext::new(64, 64, 1.0);
        let mut server_ctx = ServerContext::default();
        server_ctx.pc = ProjectContext::Region(Uuid::new_v4());
        server_ctx.editor_view_mode = EditorViewMode::Iso;

        assert!(
            action
                .apply(&mut map, &mut ui, &mut ctx, &mut server_ctx)
                .is_some()
        );
        let face = &map.geometry_objects[0].faces[face_index];
        assert_eq!(face.surface_segments.len(), 9);
        assert_eq!(map.selected_geometry_surface_segments.len(), 9);
    }

    #[test]
    fn create_pattern_cobble_generates_irregular_closed_loops() {
        let (mut map, _object_id, face_index) = selected_box_top_face();
        let mut action = CreatePattern::new();
        action.nodeui.set_i32_value(PATTERN_ID, 5);
        action.nodeui.set_bool_value(REPEAT_ID, true);
        action.nodeui.set_f32_value(SCALE_ID, 0.8);
        action.nodeui.set_f32_value(SPACING_X_ID, 1.0);
        action.nodeui.set_f32_value(SPACING_Y_ID, 1.0);
        action.nodeui.set_i32_value(SIDES_ID, 12);
        action.nodeui.set_f32_value(JITTER_ID, 0.3);
        action.nodeui.set_i32_value(SEED_ID, 7);
        action.nodeui.set_i32_value(FIT_ROWS_ID, 1);
        action.nodeui.set_i32_value(FIT_COLUMNS_ID, 2);

        let mut ui = TheUI::default();
        let mut ctx = TheContext::new(64, 64, 1.0);
        let mut server_ctx = ServerContext::default();
        server_ctx.pc = ProjectContext::Region(Uuid::new_v4());
        server_ctx.editor_view_mode = EditorViewMode::Iso;

        assert!(
            action
                .apply(&mut map, &mut ui, &mut ctx, &mut server_ctx)
                .is_some()
        );
        let face = &map.geometry_objects[0].faces[face_index];
        assert_eq!(face.surface_points.len(), 24);
        assert_eq!(face.surface_segments.len(), 24);
        assert_eq!(map.selected_geometry_surface_segments.len(), 24);
        assert!(face.surface_points[0].position != face.surface_points[12].position);
    }

    #[test]
    fn create_pattern_rotation_rotates_stamp_layout() {
        let (map, _object_id, face_index) = selected_box_top_face();
        let object = &map.geometry_objects[0];
        let plane = geometry_face_plane(object, &object.faces[face_index]).unwrap();
        let mut action = CreatePattern::new();
        action.nodeui.set_bool_value(REPEAT_ID, true);
        action.nodeui.set_f32_value(SCALE_ID, 0.5);
        action.nodeui.set_f32_value(SPACING_X_ID, 1.0);
        action.nodeui.set_f32_value(SPACING_Y_ID, 1.0);
        action.nodeui.set_f32_value(ROTATION_ID, 90.0);
        action.nodeui.set_i32_value(FIT_ROWS_ID, 1);
        action.nodeui.set_i32_value(FIT_COLUMNS_ID, 2);

        let stamps = pattern_stamps(&plane, &pattern_settings(&action.nodeui));

        assert_eq!(stamps.len(), 2);
        assert!((stamps[0].center.x - stamps[1].center.x).abs() < 0.001);
        assert!((stamps[0].center.y - stamps[1].center.y).abs() > 0.5);
    }

    #[test]
    fn create_pattern_relief_generates_raised_pattern_object() {
        let (mut map, _object_id, _face_index) = selected_box_top_face();
        let mut action = CreatePattern::new();
        action.nodeui.set_i32_value(MODE_ID, 1);
        action.nodeui.set_i32_value(PATTERN_ID, 5);
        action.nodeui.set_bool_value(REPEAT_ID, true);
        action.nodeui.set_f32_value(SCALE_ID, 0.8);
        action.nodeui.set_f32_value(SPACING_X_ID, 1.0);
        action.nodeui.set_f32_value(SPACING_Y_ID, 1.0);
        action.nodeui.set_i32_value(SIDES_ID, 8);
        action.nodeui.set_i32_value(FIT_ROWS_ID, 1);
        action.nodeui.set_i32_value(FIT_COLUMNS_ID, 2);
        action.pattern_source_override = Some(Some(PixelSource::PaletteIndex(8)));
        action.background_source_override = Some(Some(PixelSource::PaletteIndex(2)));

        let mut ui = TheUI::default();
        let mut ctx = TheContext::new(64, 64, 1.0);
        let mut server_ctx = ServerContext::default();
        server_ctx.pc = ProjectContext::Region(Uuid::new_v4());
        server_ctx.editor_view_mode = EditorViewMode::Iso;

        assert!(
            action
                .apply(&mut map, &mut ui, &mut ctx, &mut server_ctx)
                .is_some()
        );
        assert_eq!(map.geometry_objects.len(), 2);
        assert!(map.geometry_objects[1].faces.len() > 2);
        assert_eq!(
            map.selected_geometry_objects,
            vec![map.geometry_objects[1].id]
        );
        assert!(map.selected_geometry_faces.is_empty());
        assert_eq!(
            map.geometry_objects[0].faces[2].tile,
            Some(PixelSource::PaletteIndex(2))
        );
    }

    #[test]
    fn create_pattern_toml_uses_short_grouped_keys() {
        let action = CreatePattern::new();
        let toml = crate::actions::nodeui_to_toml(&action.nodeui);
        assert!(toml.contains(
            "# \"guide\", \"relief\"\nmode = \"guide\"\n# \"disc\", \"triangle\", \"quad\", \"line\", \"tile\", \"cobble\"\npattern = \"disc\"\nsequence = \"\"\nrepeat = false\ninterleave = false\n"
        ));
        assert!(toml.contains(
            "[shape]\nscale = 1.0\nrotation = 0.0\nmargin = 0.0\nsides = 16\nroundness = 0.65\njitter = 0.15\nseed = 0\n"
        ));
        assert!(toml.contains("[spacing]\nx = 1.0\ny = 1.0\n"));
        assert!(toml.contains(
            "[relief]\nheight = 0.08\nheight_jitter = 0.02\ndome = 0.4\nedge_depth = 0.0\ncolor_jitter = 0.0\n"
        ));
        assert!(toml.contains("[fit]\nrows = 0\ncolumns = 0\n"));
        assert!(!toml.contains("pattern_repeat"));
    }
}
