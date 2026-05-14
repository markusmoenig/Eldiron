use crate::editor::RUSTERIX;
use crate::prelude::*;

const PATTERN_ID: &str = "actionPattern";
const SEQUENCE_ID: &str = "actionSequence";
const REPEAT_ID: &str = "actionRepeat";
const SCALE_ID: &str = "actionScale";
const SPACING_X_ID: &str = "actionSpacingX";
const SPACING_Y_ID: &str = "actionSpacingY";
const ROTATION_ID: &str = "actionRotation";
const MARGIN_ID: &str = "actionMargin";
const SIDES_ID: &str = "actionSides";
const FIT_ROWS_ID: &str = "actionFitRows";
const FIT_COLUMNS_ID: &str = "actionFitColumns";

pub struct CreatePattern {
    id: TheId,
    nodeui: TheNodeUI,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PatternShape {
    Disc,
    Triangle,
    Quad,
    Line,
    Brick,
    Tile,
}

#[derive(Clone, Debug)]
struct FacePlane {
    origin: Vec3<f32>,
    axis_u: Vec3<f32>,
    axis_v: Vec3<f32>,
    polygon: Vec<Vec2<f32>>,
    min: Vec2<f32>,
    max: Vec2<f32>,
}

#[derive(Clone, Debug)]
struct PatternSettings {
    shapes: Vec<PatternShape>,
    repeat: bool,
    scale: f32,
    spacing_x: f32,
    spacing_y: f32,
    rotation: f32,
    margin: f32,
    sides: usize,
    rows: usize,
    columns: usize,
}

impl PatternShape {
    fn from_index(index: i32) -> Self {
        match index {
            1 => Self::Triangle,
            2 => Self::Quad,
            3 => Self::Line,
            4 => Self::Brick,
            5 => Self::Tile,
            _ => Self::Disc,
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "disc" | "circle" => Some(Self::Disc),
            "tri" | "triangle" => Some(Self::Triangle),
            "quad" | "rect" | "rectangle" | "square" => Some(Self::Quad),
            "line" => Some(Self::Line),
            "brick" | "bricks" => Some(Self::Brick),
            "tile" => Some(Self::Tile),
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

    let origin = *object.vertices.get(*face.indices.first()?)?;
    let mut axis_u = None;
    for index in face.indices.iter().skip(1) {
        let edge = *object.vertices.get(*index)? - origin;
        if edge.magnitude_squared() > 1e-6 {
            axis_u = edge.try_normalized();
            break;
        }
    }
    let axis_u = axis_u?;

    let mut normal = Vec3::<f32>::zero();
    for index in 1..face.indices.len().saturating_sub(1) {
        let a = *object.vertices.get(face.indices[index])? - origin;
        let b = *object.vertices.get(face.indices[index + 1])? - origin;
        normal += a.cross(b);
    }
    let normal = normal.try_normalized()?;
    let axis_v = normal.cross(axis_u).try_normalized()?;

    let mut polygon = Vec::with_capacity(face.indices.len());
    let mut min = Vec2::broadcast(f32::INFINITY);
    let mut max = Vec2::broadcast(f32::NEG_INFINITY);
    for index in &face.indices {
        let rel = *object.vertices.get(*index)? - origin;
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
        shapes,
        repeat: nodeui.get_bool_value(REPEAT_ID).unwrap_or(false),
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
        rows: nodeui.get_i32_value(FIT_ROWS_ID).unwrap_or(0).max(0) as usize,
        columns: nodeui.get_i32_value(FIT_COLUMNS_ID).unwrap_or(0).max(0) as usize,
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

fn stamp_points(shape: PatternShape, settings: &PatternSettings) -> Vec<Vec2<f32>> {
    let half = settings.scale * 0.5;
    match shape {
        PatternShape::Disc => regular_polygon(settings.sides, half, settings.rotation),
        PatternShape::Triangle => regular_polygon(3, half, settings.rotation),
        PatternShape::Quad | PatternShape::Brick | PatternShape::Tile => [
            Vec2::new(-half, -half),
            Vec2::new(half, -half),
            Vec2::new(half, half),
            Vec2::new(-half, half),
        ]
        .into_iter()
        .map(|point| rotate(point, settings.rotation))
        .collect(),
        PatternShape::Line => [Vec2::new(-half, 0.0), Vec2::new(half, 0.0)]
            .into_iter()
            .map(|point| rotate(point, settings.rotation))
            .collect(),
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

fn append_brick_pattern(
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

    let brick_width = settings.spacing_x.max(settings.scale).max(0.01);
    let brick_height = settings.spacing_y.max(0.01);
    let columns = if settings.columns > 0 {
        settings
            .columns
            .min((available.x / brick_width).floor().max(1.0) as usize)
    } else {
        (available.x / brick_width).floor().max(1.0) as usize
    };
    let rows = if settings.rows > 0 {
        settings
            .rows
            .min((available.y / brick_height).floor().max(1.0) as usize)
    } else {
        (available.y / brick_height).floor().max(1.0) as usize
    };
    if rows == 0 || columns == 0 {
        return 0;
    }

    let total_width = columns as f32 * brick_width;
    let total_height = rows as f32 * brick_height;
    let min_x = plane.min.x + settings.margin + (available.x - total_width) * 0.5;
    let max_x = min_x + total_width;
    let min_y = plane.min.y + settings.margin + (available.y - total_height) * 0.5;
    let max_y = min_y + total_height;
    let mut added = 0usize;

    for row in 0..=rows {
        let y = min_y + row as f32 * brick_height;
        let points = [Vec2::new(min_x, y), Vec2::new(max_x, y)];
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
        let y0 = min_y + row as f32 * brick_height;
        let y1 = (y0 + brick_height).min(max_y);
        let offset = if stagger && row % 2 == 1 {
            brick_width * 0.5
        } else {
            0.0
        };
        let first = if offset > 0.0 { -1isize } else { 0isize };
        for column in first..=columns as isize {
            let x = min_x + column as f32 * brick_width + offset;
            if x < min_x - 1e-5 || x > max_x + 1e-5 {
                continue;
            }
            let points = [
                Vec2::new(x.clamp(min_x, max_x), y0),
                Vec2::new(x.clamp(min_x, max_x), y1),
            ];
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
    let available = (plane.max - plane.min) - Vec2::broadcast(settings.margin * 2.0);
    if available.x <= 0.0 || available.y <= 0.0 {
        return None;
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
        return None;
    }

    let total_width = footprint + (columns.saturating_sub(1) as f32) * settings.spacing_x;
    let total_height = footprint + (rows.saturating_sub(1) as f32) * settings.spacing_y;
    let start_x =
        plane.min.x + settings.margin + (available.x - total_width) * 0.5 + footprint * 0.5;
    let start_y =
        plane.min.y + settings.margin + (available.y - total_height) * 0.5 + footprint * 0.5;

    let object_id = object.id;
    let face = object.faces.get_mut(face_index)?;
    let mut added = 0usize;
    let mut selected_points = Vec::new();
    let mut selected_segments = Vec::new();

    if matches!(
        settings.shapes.first(),
        Some(PatternShape::Brick | PatternShape::Tile)
    ) {
        let stagger = settings.shapes.first() == Some(&PatternShape::Brick);
        added = append_brick_pattern(
            face,
            object_id,
            face_index,
            &plane,
            settings,
            &mut selected_points,
            &mut selected_segments,
            stagger,
        );
        return (added > 0).then_some((added, selected_points, selected_segments));
    }

    for row in 0..rows {
        for column in 0..columns {
            let sequence_index = row * columns + column;
            let shape = settings.shapes[sequence_index % settings.shapes.len()];
            let center = Vec2::new(
                start_x + column as f32 * settings.spacing_x,
                start_y + row as f32 * settings.spacing_y,
            );
            let points = stamp_points(shape, settings);
            let closed = shape != PatternShape::Line;
            if append_stamp(
                face,
                object_id,
                face_index,
                &plane,
                center,
                &points,
                closed,
                &mut selected_points,
                &mut selected_segments,
            ) {
                added += 1;
            }
        }
    }

    (added > 0).then_some((added, selected_points, selected_segments))
}

fn build_nodeui() -> TheNodeUI {
    let mut nodeui = TheNodeUI::default();
    nodeui.add_item(TheNodeUIItem::Markdown(
        "desc".into(),
        fl!("action_create_pattern_desc"),
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
            "brick".into(),
            "tile".into(),
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

        if added == 0 {
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                fl!("status_create_pattern_no_fit"),
            ));
            return None;
        }

        map.selected_geometry_surface_points = selected_points;
        map.selected_geometry_surface_segments = selected_segments;
        map.changed = map.changed.wrapping_add(1);
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
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        let _ = self.nodeui.handle_event(event);
        // Pattern stamping appends editable guides, so parameter edits should not auto-apply.
        false
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
    fn create_pattern_brick_generates_mortar_lines() {
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
        assert_eq!(face.surface_segments.len(), 8);
        assert_eq!(map.selected_geometry_surface_segments.len(), 8);
    }

    #[test]
    fn create_pattern_tile_generates_unstaggered_grid() {
        let (mut map, _object_id, face_index) = selected_box_top_face();
        let mut action = CreatePattern::new();
        action.nodeui.set_i32_value(PATTERN_ID, 5);
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
    fn create_pattern_toml_uses_short_grouped_keys() {
        let action = CreatePattern::new();
        let toml = crate::actions::nodeui_to_toml(&action.nodeui);
        assert!(toml.contains(
            "# \"disc\", \"triangle\", \"quad\", \"line\", \"brick\", \"tile\"\npattern = \"disc\"\nsequence = \"\"\nrepeat = false\n"
        ));
        assert!(toml.contains("[shape]\nscale = 1.0\nrotation = 0.0\nmargin = 0.0\nsides = 16\n"));
        assert!(toml.contains("[spacing]\nx = 1.0\ny = 1.0\n"));
        assert!(toml.contains("[fit]\nrows = 0\ncolumns = 0\n"));
        assert!(!toml.contains("pattern_repeat"));
    }
}
