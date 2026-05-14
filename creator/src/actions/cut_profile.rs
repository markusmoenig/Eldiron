use crate::editor::RUSTERIX;
use crate::prelude::*;
use theframework::prelude::FxHashMap;

const PROFILE_ID: &str = "actionProfile";
const AXIS_ID: &str = "actionAxis";
const HEIGHT_ID: &str = "actionHeight";
const MERLON_ID: &str = "actionMerlon";
const CRENEL_ID: &str = "actionCrenel";

pub struct CutProfile {
    id: TheId,
    nodeui: TheNodeUI,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProfileAxis {
    X,
    Z,
}

fn geometry_face(
    indices: Vec<usize>,
    source_face: Option<&rusterix::GeometryFace>,
) -> rusterix::GeometryFace {
    rusterix::GeometryFace {
        indices,
        uvs: Vec::new(),
        auto_uv: true,
        texture_offset: source_face
            .map(|face| face.texture_offset)
            .unwrap_or_else(Vec2::zero),
        texture_scale: source_face
            .map(|face| face.texture_scale)
            .unwrap_or_else(|| Vec2::broadcast(1.0)),
        texture_rotation: source_face.map(|face| face.texture_rotation).unwrap_or(0.0),
        tile: source_face.and_then(|face| face.tile.clone()),
        tiles: FxHashMap::default(),
        surface_points: Vec::new(),
        surface_segments: Vec::new(),
        surface_noise: None,
    }
}

fn local_face_edit_normal(
    object: &rusterix::GeometryObject,
    face: &rusterix::GeometryFace,
) -> Option<Vec3<f32>> {
    if face.indices.len() < 3 {
        return None;
    }
    let first = *object.vertices.get(face.indices[0])?;
    let mut normal = Vec3::<f32>::zero();
    for index in 1..face.indices.len() - 1 {
        let a = *object.vertices.get(face.indices[index])? - first;
        let b = *object.vertices.get(face.indices[index + 1])? - first;
        normal += a.cross(b);
    }
    normal.try_normalized().map(|normal| -normal)
}

fn source_face_for_normal<'a>(
    object: &'a rusterix::GeometryObject,
    direction: Vec3<f32>,
) -> Option<&'a rusterix::GeometryFace> {
    object
        .faces
        .iter()
        .filter_map(|face| local_face_edit_normal(object, face).map(|normal| (face, normal)))
        .max_by(|(_, a), (_, b)| {
            a.dot(direction)
                .partial_cmp(&b.dot(direction))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(face, _)| face)
}

fn selected_profile_object(map: &Map) -> Option<usize> {
    if map.selected_geometry_objects.len() != 1 {
        return None;
    }
    let object_id = map.selected_geometry_objects[0];
    map.geometry_objects
        .iter()
        .position(|object| object.id == object_id)
}

fn object_bounds(object: &rusterix::GeometryObject) -> Option<(Vec3<f32>, Vec3<f32>)> {
    let first = *object.vertices.first()?;
    let mut min = first;
    let mut max = first;
    for point in object.vertices.iter().skip(1) {
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        min.z = min.z.min(point.z);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
        max.z = max.z.max(point.z);
    }
    Some((min, max))
}

fn append_box(
    vertices: &mut Vec<Vec3<f32>>,
    faces: &mut Vec<rusterix::GeometryFace>,
    min: Vec3<f32>,
    max: Vec3<f32>,
    sources: &[Option<rusterix::GeometryFace>; 6],
) {
    let base = vertices.len();
    vertices.extend([
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ]);

    let source = |index: usize| sources[index].as_ref();
    faces.push(geometry_face(
        vec![base, base + 1, base + 2, base + 3],
        source(0),
    ));
    faces.push(geometry_face(
        vec![base + 5, base + 4, base + 7, base + 6],
        source(1),
    ));
    faces.push(geometry_face(
        vec![base + 4, base, base + 3, base + 7],
        source(2),
    ));
    faces.push(geometry_face(
        vec![base + 1, base + 5, base + 6, base + 2],
        source(3),
    ));
    faces.push(geometry_face(
        vec![base + 3, base + 2, base + 6, base + 7],
        source(4),
    ));
    faces.push(geometry_face(
        vec![base + 4, base + 5, base + 1, base],
        source(5),
    ));
}

fn crenellation_segments(length: f32, merlon: f32, crenel: f32) -> Vec<(f32, f32)> {
    let merlon = merlon.max(0.01);
    let crenel = crenel.max(0.01);
    let pattern = merlon + crenel;
    let mut count = ((length + crenel) / pattern).floor().max(1.0) as usize;
    while count > 1 {
        let used = count as f32 * merlon + (count - 1) as f32 * crenel;
        if used <= length + 1e-4 {
            break;
        }
        count -= 1;
    }

    let used = count as f32 * merlon + count.saturating_sub(1) as f32 * crenel;
    let offset = ((length - used) * 0.5).max(0.0);
    (0..count)
        .map(|index| {
            let start = offset + index as f32 * pattern;
            (start, (start + merlon).min(length))
        })
        .filter(|(start, end)| *end - *start > 1e-4)
        .collect()
}

fn cut_crenellation_into_object(
    object: &mut rusterix::GeometryObject,
    axis: ProfileAxis,
    height: f32,
    merlon: f32,
    crenel: f32,
) -> bool {
    let Some((min, max)) = object_bounds(object) else {
        return false;
    };
    let size = max - min;
    if size.x <= 0.01 || size.y <= 0.01 || size.z <= 0.01 {
        return false;
    }

    let cut_height = height.clamp(0.01, size.y);
    let base_top = max.y - cut_height;
    let length = if axis == ProfileAxis::X {
        size.x
    } else {
        size.z
    };
    let segments = crenellation_segments(length, merlon, crenel);
    if segments.is_empty() {
        return false;
    }

    let sources = [
        source_face_for_normal(object, Vec3::new(0.0, 0.0, -1.0)).cloned(),
        source_face_for_normal(object, Vec3::new(0.0, 0.0, 1.0)).cloned(),
        source_face_for_normal(object, Vec3::new(-1.0, 0.0, 0.0)).cloned(),
        source_face_for_normal(object, Vec3::new(1.0, 0.0, 0.0)).cloned(),
        source_face_for_normal(object, Vec3::new(0.0, 1.0, 0.0)).cloned(),
        source_face_for_normal(object, Vec3::new(0.0, -1.0, 0.0)).cloned(),
    ];

    let mut vertices = Vec::new();
    let mut faces = Vec::new();

    if base_top > min.y + 1e-4 {
        append_box(
            &mut vertices,
            &mut faces,
            min,
            Vec3::new(max.x, base_top, max.z),
            &sources,
        );
    }

    for (start, end) in segments {
        let (box_min, box_max) = if axis == ProfileAxis::X {
            (
                Vec3::new(min.x + start, base_top, min.z),
                Vec3::new(min.x + end, max.y, max.z),
            )
        } else {
            (
                Vec3::new(min.x, base_top, min.z + start),
                Vec3::new(max.x, max.y, min.z + end),
            )
        };
        append_box(&mut vertices, &mut faces, box_min, box_max, &sources);
    }

    object.vertices = vertices;
    object.faces = faces;
    true
}

pub fn cut_profile_into_selected_object(
    map: &mut Map,
    profile_index: i32,
    axis_index: i32,
    height: f32,
    merlon: f32,
    crenel: f32,
) -> bool {
    if profile_index != 0 {
        return false;
    }
    let Some(object_index) = selected_profile_object(map) else {
        return false;
    };
    let axis = {
        let Some(object) = map.geometry_objects.get(object_index) else {
            return false;
        };
        let Some((min, max)) = object_bounds(object) else {
            return false;
        };
        match axis_index {
            1 => ProfileAxis::X,
            2 => ProfileAxis::Z,
            _ if max.x - min.x >= max.z - min.z => ProfileAxis::X,
            _ => ProfileAxis::Z,
        }
    };
    let Some(object) = map.geometry_objects.get_mut(object_index) else {
        return false;
    };
    if !cut_crenellation_into_object(object, axis, height, merlon, crenel) {
        return false;
    }

    map.selected_geometry_objects = vec![object.id];
    map.selected_geometry_faces.clear();
    map.selected_geometry_vertices.clear();
    map.selected_geometry_surface_points.clear();
    map.selected_geometry_surface_segments.clear();
    true
}

impl Action for CutProfile {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_cut_profile_desc"),
        ));
        nodeui.add_item(TheNodeUIItem::Selector(
            PROFILE_ID.into(),
            "Profile".into(),
            "Profile shape to cut into the selected object.".into(),
            vec!["crenellation".into()],
            0,
        ));
        nodeui.add_item(TheNodeUIItem::Selector(
            AXIS_ID.into(),
            "Axis".into(),
            "Length axis for the repeated profile.".into(),
            vec!["auto".into(), "x".into(), "z".into()],
            0,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            HEIGHT_ID.into(),
            "Height".into(),
            "Vertical cut depth from the object top.".into(),
            0.5,
            0.01..=256.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            MERLON_ID.into(),
            "Merlon".into(),
            "Solid battlement block width.".into(),
            1.0,
            0.01..=256.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            CRENEL_ID.into(),
            "Crenel".into(),
            "Gap width between battlement blocks.".into(),
            1.0,
            0.01..=256.0,
            false,
        ));

        Self {
            id: TheId::named(&fl!("action_cut_profile")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_cut_profile_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
            && map.selected_geometry_objects.len() == 1
    }

    fn load_params(&mut self, map: &Map) {
        let step = ServerContext::edit_grid_step(map.subdivisions).max(0.01);
        self.nodeui.set_i32_value(PROFILE_ID, 0);
        self.nodeui.set_i32_value(AXIS_ID, 0);
        self.nodeui.set_f32_value(HEIGHT_ID, step);
        self.nodeui.set_f32_value(MERLON_ID, step);
        self.nodeui.set_f32_value(CRENEL_ID, step);
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let profile = self.nodeui.get_i32_value(PROFILE_ID).unwrap_or(0);
        let axis = self.nodeui.get_i32_value(AXIS_ID).unwrap_or(0);
        let height = self
            .nodeui
            .get_f32_value(HEIGHT_ID)
            .unwrap_or_else(|| ServerContext::edit_grid_step(map.subdivisions))
            .max(0.01);
        let merlon = self
            .nodeui
            .get_f32_value(MERLON_ID)
            .unwrap_or_else(|| ServerContext::edit_grid_step(map.subdivisions))
            .max(0.01);
        let crenel = self
            .nodeui
            .get_f32_value(CRENEL_ID)
            .unwrap_or_else(|| ServerContext::edit_grid_step(map.subdivisions))
            .max(0.01);

        let prev = map.clone();
        if !cut_profile_into_selected_object(map, profile, axis, height, merlon, crenel) {
            return None;
        }

        {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.set_dirty();
            rusterix.set_overlay_dirty();
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cut_profile_crenellates_one_selected_object() {
        let mut map = Map::new();
        let object = rusterix::GeometryObject::box_from_bounds(
            "wall",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(5.0, 2.0, 1.0),
        );
        let object_id = object.id;
        map.geometry_objects.push(object);
        map.selected_geometry_objects = vec![object_id];

        assert!(cut_profile_into_selected_object(
            &mut map, 0, 0, 0.5, 1.0, 1.0
        ));
        assert_eq!(map.geometry_objects.len(), 1);
        assert_eq!(map.selected_geometry_objects, vec![object_id]);
        assert!(map.selected_geometry_faces.is_empty());

        let object = &map.geometry_objects[0];
        assert_eq!(object.vertices.len(), 32);
        assert_eq!(object.faces.len(), 24);
    }

    #[test]
    fn cut_profile_requires_one_selected_object() {
        let mut map = Map::new();
        let object = rusterix::GeometryObject::box_from_bounds(
            "wall",
            Vec3::zero(),
            Vec3::new(2.0, 1.0, 1.0),
        );
        map.geometry_objects.push(object);

        assert!(!cut_profile_into_selected_object(
            &mut map, 0, 0, 0.5, 1.0, 1.0
        ));
    }

    #[test]
    fn cut_profile_toml_uses_short_keys() {
        let action = CutProfile::new();
        let toml = crate::actions::nodeui_to_toml(&action.nodeui);

        assert!(toml.contains("profile = \"crenellation\""));
        assert!(toml.contains("axis = \"auto\""));
        assert!(toml.contains("height = 0.5"));
        assert!(toml.contains("merlon = 1.0"));
        assert!(toml.contains("crenel = 1.0"));
        assert!(!toml.contains("profile_profile"));
        assert!(!toml.contains("profile_axis"));
    }
}
