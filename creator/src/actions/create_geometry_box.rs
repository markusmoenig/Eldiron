use crate::editor::RUSTERIX;
use crate::prelude::*;
use rusterix::GeometryObject;
use std::sync::Mutex;

static LAST_GEOMETRY_BOX_SIZE: Mutex<Option<Vec3<f32>>> = Mutex::new(None);

pub struct CreateGeometryBox {
    id: TheId,
    nodeui: TheNodeUI,
}

impl CreateGeometryBox {
    fn last_size() -> Option<Vec3<f32>> {
        LAST_GEOMETRY_BOX_SIZE.lock().ok().and_then(|size| *size)
    }

    fn remember_size(size: Vec3<f32>) {
        if let Ok(mut last_size) = LAST_GEOMETRY_BOX_SIZE.lock() {
            *last_size = Some(size);
        }
    }

    fn snapped(value: f32, step: f32) -> f32 {
        (value / step).round() * step
    }

    fn face_points(
        object: &GeometryObject,
        face: &rusterix::GeometryFace,
    ) -> Option<Vec<Vec3<f32>>> {
        let points = face
            .indices
            .iter()
            .map(|index| {
                object
                    .vertices
                    .get(*index)
                    .map(|vertex| object.transform_point(*vertex))
            })
            .collect::<Option<Vec<_>>>()?;
        (points.len() >= 3).then_some(points)
    }

    fn face_normal(points: &[Vec3<f32>]) -> Option<Vec3<f32>> {
        let first = *points.first()?;
        let mut normal = Vec3::<f32>::zero();
        for index in 1..points.len().saturating_sub(1) {
            normal += (points[index] - first).cross(points[index + 1] - first);
        }
        normal.try_normalized()
    }

    fn face_anchor(
        object: &GeometryObject,
        face: &rusterix::GeometryFace,
    ) -> Option<(Vec3<f32>, Vec3<f32>, Vec3<f32>, Vec3<f32>)> {
        let points = Self::face_points(object, face)?;
        let center = points
            .iter()
            .copied()
            .fold(Vec3::zero(), |sum, point| sum + point)
            / points.len() as f32;
        let normal = -Self::face_normal(&points)?;
        let mut min = Vec3::broadcast(f32::INFINITY);
        let mut max = Vec3::broadcast(f32::NEG_INFINITY);
        for point in &points {
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.y);
            min.z = min.z.min(point.z);
            max.x = max.x.max(point.x);
            max.y = max.y.max(point.y);
            max.z = max.z.max(point.z);
        }
        Some((center, normal, min, max))
    }

    fn selected_edge_face_anchor(
        map: &Map,
    ) -> Option<(Vec3<f32>, Vec3<f32>, Vec3<f32>, Vec3<f32>)> {
        if map.selected_geometry_vertices.len() != 2 {
            return None;
        }

        let (object_id, a_index) = map.selected_geometry_vertices[0];
        let (b_object_id, b_index) = map.selected_geometry_vertices[1];
        if object_id != b_object_id || a_index == b_index {
            return None;
        }

        let object = map
            .geometry_objects
            .iter()
            .find(|object| object.id == object_id)?;
        let a = object.transform_point(*object.vertices.get(a_index)?);
        let b = object.transform_point(*object.vertices.get(b_index)?);
        if (a.y - b.y).abs() > 0.001 {
            return None;
        }

        let edge = b - a;
        if edge.x.abs().max(edge.z.abs()) <= 0.001 {
            return None;
        }

        let selected_edge = if a_index < b_index {
            (a_index, b_index)
        } else {
            (b_index, a_index)
        };
        let edge_y = a.y.max(b.y);
        let mut best_face = None;
        let mut best_horizontal = 0.0f32;
        for face in &object.faces {
            if face.indices.len() < 3 {
                continue;
            }
            let has_edge = (0..face.indices.len()).any(|index| {
                let edge = (
                    face.indices[index],
                    face.indices[(index + 1) % face.indices.len()],
                );
                let edge = if edge.0 < edge.1 {
                    edge
                } else {
                    (edge.1, edge.0)
                };
                edge == selected_edge
            });
            if !has_edge {
                continue;
            }
            let points = Self::face_points(object, face)?;
            let normal = Self::face_normal(&points)?;
            let horizontal = Vec2::new(normal.x, normal.z).magnitude();
            let face_max_y = points
                .iter()
                .map(|point| point.y)
                .fold(f32::NEG_INFINITY, f32::max);
            if horizontal < 0.5 || (face_max_y - edge_y).abs() > 0.001 {
                continue;
            }
            if horizontal <= best_horizontal {
                continue;
            }
            best_horizontal = horizontal;
            best_face = Some(face);
        }

        Self::face_anchor(object, best_face?)
    }

    fn selected_face_anchor(map: &Map) -> Option<(Vec3<f32>, Vec3<f32>, Vec3<f32>, Vec3<f32>)> {
        let (object_id, face_index) = *map.selected_geometry_faces.first()?;
        let object = map
            .geometry_objects
            .iter()
            .find(|object| object.id == object_id)?;
        let face = object.faces.get(face_index)?;
        Self::face_anchor(object, face)
    }

    fn size_for_anchor(
        default_size: Vec3<f32>,
        step: f32,
        normal: Vec3<f32>,
        face_min: Vec3<f32>,
        face_max: Vec3<f32>,
        force_step_thickness: bool,
    ) -> Vec3<f32> {
        let thickness = if force_step_thickness {
            step.max(0.05)
        } else {
            0.0
        };
        let fallback_thickness = |value: f32| {
            if force_step_thickness {
                thickness
            } else {
                value.max(0.05)
            }
        };

        let face_size = face_max - face_min;
        let abs = Vec3::new(normal.x.abs(), normal.y.abs(), normal.z.abs());
        let mut size = Vec3::new(
            face_size.x.max(step),
            face_size.y.max(step),
            face_size.z.max(step),
        );

        if abs.x >= abs.y && abs.x >= abs.z {
            size.x = fallback_thickness(default_size.x);
        } else if abs.y >= abs.x && abs.y >= abs.z {
            size.y = fallback_thickness(default_size.y);
        } else {
            size.z = fallback_thickness(default_size.z);
        }

        size
    }

    fn size_for_params(map: &Map, step: f32) -> Vec3<f32> {
        let default_size = Self::last_size().unwrap_or(Vec3::broadcast(step));
        let edge_anchor = !map.selected_geometry_vertices.is_empty();
        let anchor = if edge_anchor {
            Self::selected_edge_face_anchor(map)
        } else {
            Self::selected_face_anchor(map)
        };
        let Some((_, normal, face_min, face_max)) = anchor else {
            return default_size;
        };

        Self::size_for_anchor(default_size, step, normal, face_min, face_max, edge_anchor)
    }

    fn box_bounds_from_face_anchor(
        anchor: Vec3<f32>,
        normal: Vec3<f32>,
        face_min: Vec3<f32>,
        face_max: Vec3<f32>,
        size: Vec3<f32>,
        step: f32,
        snap_bounds: bool,
    ) -> (Vec3<f32>, Vec3<f32>) {
        let abs = Vec3::new(normal.x.abs(), normal.y.abs(), normal.z.abs());
        let mut min = face_min;
        let mut max = face_max;

        if abs.x >= abs.y && abs.x >= abs.z {
            if normal.x >= 0.0 {
                min.x = anchor.x;
                max.x = anchor.x + size.x;
            } else {
                min.x = anchor.x - size.x;
                max.x = anchor.x;
            }
        } else if abs.y >= abs.x && abs.y >= abs.z {
            if normal.y >= 0.0 {
                min.y = anchor.y;
                max.y = anchor.y + size.y;
            } else {
                min.y = anchor.y - size.y;
                max.y = anchor.y;
            }
        } else if normal.z >= 0.0 {
            min.z = anchor.z;
            max.z = anchor.z + size.z;
        } else {
            min.z = anchor.z - size.z;
            max.z = anchor.z;
        }

        if snap_bounds {
            min = Vec3::new(
                Self::snapped(min.x, step),
                Self::snapped(min.y, step),
                Self::snapped(min.z, step),
            );
            max = Vec3::new(
                Self::snapped(max.x, step),
                Self::snapped(max.y, step),
                Self::snapped(max.z, step),
            );
        } else {
            let _ = step;
        }
        (
            Vec3::new(min.x.min(max.x), min.y.min(max.y), min.z.min(max.z)),
            Vec3::new(min.x.max(max.x), min.y.max(max.y), min.z.max(max.z)),
        )
    }

    fn viewport_center_position(
        ui: &mut TheUI,
        server_ctx: &mut ServerContext,
    ) -> Option<Vec3<f32>> {
        let render_view = ui.get_render_view("PolyView")?;
        let dim = *render_view.dim();
        if dim.width <= 0 || dim.height <= 0 {
            return None;
        }

        let rusterix = RUSTERIX.write().unwrap();
        let (ray_origin, ray_dir) = rusterix.scene_handler.vm.ray_from_uv_with_size(
            dim.width as u32,
            dim.height as u32,
            [0.5, 0.5],
        )?;
        drop(rusterix);

        server_ctx.hover_ray_origin_3d = Some(ray_origin);
        server_ctx.hover_ray_dir_3d = Some(ray_dir);

        if ray_dir.y.abs() > 1e-6 {
            let t = -ray_origin.y / ray_dir.y;
            if t.is_finite() && t >= 0.0 {
                return Some(ray_origin + ray_dir * t);
            }
        }

        None
    }
}

impl Action for CreateGeometryBox {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_create_geometry_box_desc"),
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionGeometryWidth".into(),
            "Width".into(),
            "".into(),
            1.0,
            0.05..=256.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionGeometryHeight".into(),
            "Height".into(),
            "".into(),
            1.0,
            0.05..=256.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionGeometryDepth".into(),
            "Depth".into(),
            "".into(),
            1.0,
            0.05..=256.0,
            false,
        ));

        Self {
            id: TheId::named(&fl!("action_create_geometry_box")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_create_geometry_box_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
    }

    fn load_params(&mut self, map: &Map) {
        let step = ServerContext::edit_grid_step(map.subdivisions);
        let size = Self::size_for_params(map, step);
        self.nodeui.set_f32_value("actionGeometryWidth", size.x);
        self.nodeui.set_f32_value("actionGeometryHeight", size.y);
        self.nodeui.set_f32_value("actionGeometryDepth", size.z);
    }

    fn apply(
        &self,
        map: &mut Map,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let prev = map.clone();
        let step = ServerContext::edit_grid_step(map.subdivisions);
        let mut size = Vec3::new(
            self.nodeui
                .get_f32_value("actionGeometryWidth")
                .unwrap_or(step)
                .max(0.05),
            self.nodeui
                .get_f32_value("actionGeometryHeight")
                .unwrap_or(step)
                .max(0.05),
            self.nodeui
                .get_f32_value("actionGeometryDepth")
                .unwrap_or(step)
                .max(0.05),
        );
        let (min, max) = if !map.selected_geometry_vertices.is_empty() {
            let (anchor, normal, face_min, face_max) = Self::selected_edge_face_anchor(map)?;
            size = Self::size_for_anchor(size, step, normal, face_min, face_max, true);
            Self::box_bounds_from_face_anchor(anchor, normal, face_min, face_max, size, step, false)
        } else if let Some((anchor, normal, face_min, face_max)) = Self::selected_face_anchor(map) {
            Self::box_bounds_from_face_anchor(anchor, normal, face_min, face_max, size, step, true)
        } else {
            let position = Self::viewport_center_position(ui, server_ctx)
                .or(map.curr_grid_pos_3d)
                .unwrap_or(server_ctx.geo_hit_pos);
            let min = Vec3::new(
                Self::snapped(position.x, step),
                Self::snapped(position.y, step),
                Self::snapped(position.z, step),
            );
            (min, min + size)
        };
        let object = GeometryObject::box_from_bounds("Box", min, max);
        let id = object.id;
        Self::remember_size(size);

        map.geometry_objects.push(object);
        map.clear_selection();
        map.selected_geometry_objects.push(id);
        server_ctx.curr_map_tool_type = MapToolType::Selection;
        _ctx.ui.send(TheEvent::Custom(
            TheId::named("Set Tool"),
            TheValue::Text("Object Tool".into()),
        ));
        _ctx.ui.send(TheEvent::Custom(
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
    fn selected_floor_edge_creates_wall_box_outside_face() {
        let mut map = Map::new();
        let floor = GeometryObject::box_from_bounds(
            "Floor",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(4.0, 1.0, 4.0),
        );
        let floor_id = floor.id;
        map.geometry_objects.push(floor);
        map.selected_geometry_vertices = vec![(floor_id, 3), (floor_id, 2)];

        let (anchor, normal, face_min, face_max) =
            CreateGeometryBox::selected_edge_face_anchor(&map).unwrap();
        let (min, max) = CreateGeometryBox::box_bounds_from_face_anchor(
            anchor,
            normal,
            face_min,
            face_max,
            Vec3::new(0.5, 2.0, 0.5),
            0.5,
            false,
        );

        assert_eq!(min, Vec3::new(0.0, 0.0, -0.5));
        assert_eq!(max, Vec3::new(4.0, 1.0, 0.0));
    }

    #[test]
    fn selected_floor_edge_preserves_off_grid_attachment_plane() {
        let mut map = Map::new();
        let floor = GeometryObject::box_from_bounds(
            "Floor",
            Vec3::new(3.5, 0.5, 0.5),
            Vec3::new(7.5, 1.5, 5.5),
        );
        let floor_id = floor.id;
        map.geometry_objects.push(floor);
        map.selected_geometry_vertices = vec![(floor_id, 2), (floor_id, 6)];

        let (anchor, normal, face_min, face_max) =
            CreateGeometryBox::selected_edge_face_anchor(&map).unwrap();
        let (min, max) = CreateGeometryBox::box_bounds_from_face_anchor(
            anchor,
            normal,
            face_min,
            face_max,
            Vec3::new(1.0, 1.0, 5.0),
            1.0,
            false,
        );

        assert_eq!(min, Vec3::new(7.5, 0.5, 0.5));
        assert_eq!(max, Vec3::new(8.5, 1.5, 5.5));
    }

    #[test]
    fn selected_floor_edge_thickness_uses_current_grid_step() {
        let mut map = Map::new();
        let floor = GeometryObject::box_from_bounds(
            "Floor",
            Vec3::new(3.5, 0.5, 0.5),
            Vec3::new(7.5, 1.5, 5.5),
        );
        let floor_id = floor.id;
        map.geometry_objects.push(floor);
        map.selected_geometry_vertices = vec![(floor_id, 2), (floor_id, 6)];

        let (_, normal, face_min, face_max) =
            CreateGeometryBox::selected_edge_face_anchor(&map).unwrap();
        let size = CreateGeometryBox::size_for_anchor(
            Vec3::new(0.25, 3.0, 0.25),
            1.0,
            normal,
            face_min,
            face_max,
            true,
        );

        assert_eq!(size.x, 1.0);
        assert_eq!(size.y, 1.0);
        assert_eq!(size.z, 5.0);
    }
}
