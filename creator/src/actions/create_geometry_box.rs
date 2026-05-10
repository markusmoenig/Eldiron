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

    fn selected_face_anchor(map: &Map) -> Option<(Vec3<f32>, Vec3<f32>, Vec3<f32>, Vec3<f32>)> {
        let (object_id, face_index) = *map.selected_geometry_faces.first()?;
        let object = map
            .geometry_objects
            .iter()
            .find(|object| object.id == object_id)?;
        let face = object.faces.get(face_index)?;
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

    fn size_for_params(map: &Map, step: f32) -> Vec3<f32> {
        let default_size = Self::last_size().unwrap_or(Vec3::broadcast(step));
        let Some((_, normal, face_min, face_max)) = Self::selected_face_anchor(map) else {
            return default_size;
        };

        let face_size = face_max - face_min;
        let abs = Vec3::new(normal.x.abs(), normal.y.abs(), normal.z.abs());
        let mut size = Vec3::new(
            face_size.x.max(step),
            face_size.y.max(step),
            face_size.z.max(step),
        );

        if abs.x >= abs.y && abs.x >= abs.z {
            size.x = default_size.x.max(0.05);
        } else if abs.y >= abs.x && abs.y >= abs.z {
            size.y = default_size.y.max(0.05);
        } else {
            size.z = default_size.z.max(0.05);
        }

        size
    }

    fn box_bounds_from_face_anchor(
        anchor: Vec3<f32>,
        normal: Vec3<f32>,
        face_min: Vec3<f32>,
        face_max: Vec3<f32>,
        size: Vec3<f32>,
        step: f32,
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
        let size = Vec3::new(
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
        let (min, max) =
            if let Some((anchor, normal, face_min, face_max)) = Self::selected_face_anchor(map) {
                Self::box_bounds_from_face_anchor(anchor, normal, face_min, face_max, size, step)
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
