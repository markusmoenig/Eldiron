use crate::actions::geometry_face_ops::{face_uvs_for_indices, surface_segment_points};
use crate::editor::RUSTERIX;
use crate::prelude::*;
use rusterix::PixelSource;
use theframework::prelude::FxHashMap;

pub struct CreateRidge {
    id: TheId,
    nodeui: TheNodeUI,
}

pub struct CreateGroove {
    id: TheId,
    nodeui: TheNodeUI,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RidgeShape {
    Box,
    Triangle,
}

impl RidgeShape {
    fn from_index(index: i32) -> Self {
        match index {
            1 => Self::Triangle,
            _ => Self::Box,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SurfaceReliefKind {
    Ridge,
    Groove,
}

fn face_normal(
    object: &rusterix::GeometryObject,
    face: &rusterix::GeometryFace,
) -> Option<Vec3<f32>> {
    if face.indices.len() < 3 {
        return None;
    }
    let first = object.transform_point(*object.vertices.get(face.indices[0])?);
    let mut normal = Vec3::<f32>::zero();
    for index in 1..face.indices.len().saturating_sub(1) {
        let a = object.transform_point(*object.vertices.get(face.indices[index])?) - first;
        let b = object.transform_point(*object.vertices.get(face.indices[index + 1])?) - first;
        normal += a.cross(b);
    }
    normal.try_normalized()
}

fn selected_surface_segments(map: &Map) -> Vec<(Uuid, usize, usize)> {
    let mut selections = map.selected_geometry_surface_segments.clone();
    for (object_id, face_index, point_index) in &map.selected_geometry_surface_points {
        let Some(object) = map
            .geometry_objects
            .iter()
            .find(|object| object.id == *object_id)
        else {
            continue;
        };
        let Some(face) = object.faces.get(*face_index) else {
            continue;
        };
        for (segment_index, segment) in face.surface_segments.iter().enumerate() {
            if segment.start == *point_index || segment.end == *point_index {
                selections.push((*object_id, *face_index, segment_index));
            }
        }
    }
    selections.sort_unstable();
    selections.dedup();
    selections
}

fn push_face(
    object: &mut rusterix::GeometryObject,
    indices: Vec<usize>,
    tile: Option<PixelSource>,
) {
    let uvs = face_uvs_for_indices(object, &indices);
    object.faces.push(rusterix::GeometryFace {
        indices,
        uvs,
        auto_uv: true,
        tile,
        tiles: FxHashMap::default(),
        surface_points: Vec::new(),
        surface_segments: Vec::new(),
    });
}

fn host_face_tile(face: &rusterix::GeometryFace) -> Option<PixelSource> {
    face.tile
        .clone()
        .or_else(|| face.tiles.values().next().cloned())
}

#[allow(clippy::too_many_arguments)]
fn push_box_ridge_segment(
    ridge: &mut rusterix::GeometryObject,
    a: Vec3<f32>,
    b: Vec3<f32>,
    side: Vec3<f32>,
    lift: Vec3<f32>,
    nudge: Vec3<f32>,
    half_width: f32,
    tile: Option<PixelSource>,
) {
    let base = ridge.vertices.len();
    ridge.vertices.extend([
        a - side * half_width + nudge,
        a + side * half_width + nudge,
        b + side * half_width + nudge,
        b - side * half_width + nudge,
        a - side * half_width + lift,
        a + side * half_width + lift,
        b + side * half_width + lift,
        b - side * half_width + lift,
    ]);

    push_face(
        ridge,
        vec![base + 4, base + 7, base + 6, base + 5],
        tile.clone(),
    );
    push_face(
        ridge,
        vec![base, base + 3, base + 7, base + 4],
        tile.clone(),
    );
    push_face(
        ridge,
        vec![base + 1, base + 5, base + 6, base + 2],
        tile.clone(),
    );
    push_face(
        ridge,
        vec![base, base + 4, base + 5, base + 1],
        tile.clone(),
    );
    push_face(ridge, vec![base + 3, base + 2, base + 6, base + 7], tile);
}

#[allow(clippy::too_many_arguments)]
fn push_triangle_ridge_segment(
    ridge: &mut rusterix::GeometryObject,
    a: Vec3<f32>,
    b: Vec3<f32>,
    side: Vec3<f32>,
    lift: Vec3<f32>,
    nudge: Vec3<f32>,
    half_width: f32,
    tile: Option<PixelSource>,
) {
    let base = ridge.vertices.len();
    ridge.vertices.extend([
        a - side * half_width + nudge,
        a + side * half_width + nudge,
        b + side * half_width + nudge,
        b - side * half_width + nudge,
        a + lift,
        b + lift,
    ]);

    push_face(
        ridge,
        vec![base, base + 3, base + 5, base + 4],
        tile.clone(),
    );
    push_face(
        ridge,
        vec![base + 1, base + 4, base + 5, base + 2],
        tile.clone(),
    );
    push_face(ridge, vec![base, base + 4, base + 1], tile.clone());
    push_face(ridge, vec![base + 3, base + 2, base + 5], tile);
}

#[allow(clippy::too_many_arguments)]
fn push_box_groove_segment(
    ridge: &mut rusterix::GeometryObject,
    a: Vec3<f32>,
    b: Vec3<f32>,
    side: Vec3<f32>,
    normal: Vec3<f32>,
    nudge: Vec3<f32>,
    half_width: f32,
    height: f32,
    tile: Option<PixelSource>,
) {
    let floor_half_width = (half_width * 0.42).max(0.002);
    let lip_lift = normal * height.max(0.004);
    let floor_lift = normal * 0.004;
    let base = ridge.vertices.len();
    ridge.vertices.extend([
        a - side * half_width + nudge + lip_lift,
        a - side * floor_half_width + nudge + floor_lift,
        a + side * floor_half_width + nudge + floor_lift,
        a + side * half_width + nudge + lip_lift,
        b - side * half_width + nudge + lip_lift,
        b - side * floor_half_width + nudge + floor_lift,
        b + side * floor_half_width + nudge + floor_lift,
        b + side * half_width + nudge + lip_lift,
    ]);

    push_face(
        ridge,
        vec![base, base + 4, base + 5, base + 1],
        tile.clone(),
    );
    push_face(
        ridge,
        vec![base + 1, base + 5, base + 6, base + 2],
        tile.clone(),
    );
    push_face(
        ridge,
        vec![base + 2, base + 6, base + 7, base + 3],
        tile.clone(),
    );
    push_face(
        ridge,
        vec![base, base + 1, base + 2, base + 3],
        tile.clone(),
    );
    push_face(ridge, vec![base + 4, base + 7, base + 6, base + 5], tile);
}

#[allow(clippy::too_many_arguments)]
fn push_triangle_groove_segment(
    ridge: &mut rusterix::GeometryObject,
    a: Vec3<f32>,
    b: Vec3<f32>,
    side: Vec3<f32>,
    normal: Vec3<f32>,
    nudge: Vec3<f32>,
    half_width: f32,
    height: f32,
    tile: Option<PixelSource>,
) {
    let lip_lift = normal * height.max(0.004);
    let floor_lift = normal * 0.004;
    let base = ridge.vertices.len();
    ridge.vertices.extend([
        a - side * half_width + nudge + lip_lift,
        a + floor_lift + nudge,
        a + side * half_width + nudge + lip_lift,
        b - side * half_width + nudge + lip_lift,
        b + floor_lift + nudge,
        b + side * half_width + nudge + lip_lift,
    ]);

    push_face(
        ridge,
        vec![base, base + 3, base + 4, base + 1],
        tile.clone(),
    );
    push_face(
        ridge,
        vec![base + 1, base + 4, base + 5, base + 2],
        tile.clone(),
    );
    push_face(ridge, vec![base, base + 1, base + 2], tile.clone());
    push_face(ridge, vec![base + 3, base + 5, base + 4], tile);
}

fn create_surface_relief_geometry(
    map: &mut Map,
    width: f32,
    height: f32,
    shape: RidgeShape,
    kind: SurfaceReliefKind,
) -> bool {
    let width = width.max(0.01);
    let height = height.max(0.01);
    let selections = selected_surface_segments(map);
    if selections.is_empty() {
        return false;
    }

    let mut ridge = rusterix::GeometryObject::new(match kind {
        SurfaceReliefKind::Ridge => "Ridge",
        SurfaceReliefKind::Groove => "Groove",
    });
    let mut changed = false;

    for (object_id, face_index, segment_index) in selections {
        let Some(source_object) = map
            .geometry_objects
            .iter()
            .find(|object| object.id == object_id)
        else {
            continue;
        };
        let Some(face) = source_object.faces.get(face_index) else {
            continue;
        };
        let Some(segment) = face.surface_segments.get(segment_index) else {
            continue;
        };
        let Some(normal) = face_normal(source_object, face).map(|normal| -normal) else {
            continue;
        };
        let Some(points) = surface_segment_points(face, segment, normal, 8).map(|points| {
            points
                .into_iter()
                .map(|point| source_object.transform_point(point))
                .collect::<Vec<_>>()
        }) else {
            continue;
        };
        let half_width = width * 0.5;
        let lift = normal * height;
        let nudge = normal * 0.002;

        let tile = host_face_tile(face);
        for window in points.windows(2) {
            let a = window[0];
            let b = window[1];
            let Some(direction) = (b - a).try_normalized() else {
                continue;
            };
            let Some(side) = normal.cross(direction).try_normalized() else {
                continue;
            };
            match (kind, shape) {
                (SurfaceReliefKind::Ridge, RidgeShape::Box) => {
                    push_box_ridge_segment(&mut ridge, a, b, side, lift, nudge, half_width, tile.clone())
                }
                (SurfaceReliefKind::Ridge, RidgeShape::Triangle) => {
                    push_triangle_ridge_segment(&mut ridge, a, b, side, lift, nudge, half_width, tile.clone())
                }
                (SurfaceReliefKind::Groove, RidgeShape::Box) => push_box_groove_segment(
                    &mut ridge, a, b, side, normal, nudge, half_width, height, tile.clone(),
                ),
                (SurfaceReliefKind::Groove, RidgeShape::Triangle) => push_triangle_groove_segment(
                    &mut ridge, a, b, side, normal, nudge, half_width, height, tile.clone(),
                ),
            }
        }
        changed = true;
    }

    if !changed {
        return false;
    }

    let ridge_id = ridge.id;
    let ridge_face_count = ridge.faces.len();
    map.geometry_objects.push(ridge);
    map.clear_selection();
    map.selected_geometry_objects.push(ridge_id);
    map.selected_geometry_faces = (0..ridge_face_count)
        .map(|face_index| (ridge_id, face_index))
        .collect();
    true
}

fn build_nodeui(description: String) -> TheNodeUI {
    let mut nodeui = TheNodeUI::default();
    nodeui.add_item(TheNodeUIItem::Markdown("desc".into(), description));
    nodeui.add_item(TheNodeUIItem::Selector(
        "actionCreateRidgeShape".into(),
        "Shape".into(),
        "".into(),
        vec!["Box".into(), "Triangle".into()],
        0,
    ));
    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
        "actionCreateRidgeWidth".into(),
        "Width".into(),
        "".into(),
        0.12,
        0.01..=32.0,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
        "actionCreateRidgeHeight".into(),
        "Height".into(),
        "".into(),
        0.08,
        0.01..=32.0,
        false,
    ));

    nodeui
}

fn selected_shape(nodeui: &TheNodeUI) -> RidgeShape {
    RidgeShape::from_index(nodeui.get_i32_value("actionCreateRidgeShape").unwrap_or(0))
}

fn selected_dimensions(nodeui: &TheNodeUI, map: &Map) -> (f32, f32) {
    let width = nodeui
        .get_f32_value("actionCreateRidgeWidth")
        .unwrap_or_else(|| 1.0 / map.subdivisions.max(1.0) * 0.12);
    let height = nodeui
        .get_f32_value("actionCreateRidgeHeight")
        .unwrap_or_else(|| 1.0 / map.subdivisions.max(1.0) * 0.08);
    (width, height)
}

fn apply_surface_relief(
    nodeui: &TheNodeUI,
    map: &mut Map,
    ctx: &mut TheContext,
    server_ctx: &mut ServerContext,
    kind: SurfaceReliefKind,
) -> Option<ProjectUndoAtom> {
    let (width, height) = selected_dimensions(nodeui, map);
    let shape = selected_shape(nodeui);
    let prev = map.clone();
    if !create_surface_relief_geometry(map, width, height, shape, kind) {
        return None;
    }

    RUSTERIX.write().unwrap().set_dirty();
    RUSTERIX.write().unwrap().set_overlay_dirty();
    ctx.ui.send(TheEvent::Custom(
        TheId::named("Map Selection Changed"),
        TheValue::Empty,
    ));
    Some(ProjectUndoAtom::MapEdit(
        server_ctx.pc,
        Box::new(prev),
        Box::new(map.clone()),
    ))
}

fn load_default_dimensions(nodeui: &mut TheNodeUI, map: &Map) {
    let step = 1.0 / map.subdivisions.max(1.0);
    nodeui.set_f32_value("actionCreateRidgeWidth", (step * 0.12).max(0.01));
    nodeui.set_f32_value("actionCreateRidgeHeight", (step * 0.08).max(0.01));
}

fn surface_relief_is_applicable(map: &Map, server_ctx: &ServerContext) -> bool {
    server_ctx.get_map_context() == MapContext::Region
        && server_ctx.editor_view_mode != EditorViewMode::D2
        && (!map.selected_geometry_surface_segments.is_empty()
            || !map.selected_geometry_surface_points.is_empty())
}

impl Action for CreateRidge {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named(&fl!("action_create_ridge")),
            nodeui: build_nodeui(fl!("action_create_ridge_desc")),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_create_ridge_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        surface_relief_is_applicable(map, server_ctx)
    }

    fn load_params(&mut self, map: &Map) {
        load_default_dimensions(&mut self.nodeui, map);
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        apply_surface_relief(&self.nodeui, map, ctx, server_ctx, SurfaceReliefKind::Ridge)
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
        self.nodeui.handle_event(event)
    }
}

impl Action for CreateGroove {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named(&fl!("action_create_groove")),
            nodeui: build_nodeui(fl!("action_create_groove_desc")),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_create_groove_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        surface_relief_is_applicable(map, server_ctx)
    }

    fn load_params(&mut self, map: &Map) {
        load_default_dimensions(&mut self.nodeui, map);
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        apply_surface_relief(
            &self.nodeui,
            map,
            ctx,
            server_ctx,
            SurfaceReliefKind::Groove,
        )
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
        self.nodeui.handle_event(event)
    }
}
