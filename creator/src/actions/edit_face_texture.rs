use crate::editor::RUSTERIX;
use crate::prelude::*;
use vek::Vec2;

const OFFSET_X_ID: &str = "actionFaceTextureOffsetX";
const OFFSET_Y_ID: &str = "actionFaceTextureOffsetY";
const SCALE_X_ID: &str = "actionFaceTextureScaleX";
const SCALE_Y_ID: &str = "actionFaceTextureScaleY";
const ROTATION_ID: &str = "actionFaceTextureRotation";

pub struct EditFaceTexture {
    id: TheId,
    nodeui: TheNodeUI,
}

impl EditFaceTexture {
    fn texture_param_id(id: &TheId) -> bool {
        matches!(
            id.name.as_str(),
            OFFSET_X_ID | OFFSET_Y_ID | SCALE_X_ID | SCALE_Y_ID | ROTATION_ID
        )
    }

    fn texture_values(&self) -> (Vec2<f32>, Vec2<f32>, f32) {
        let offset = Vec2::new(
            self.nodeui.get_f32_value(OFFSET_X_ID).unwrap_or(0.0),
            self.nodeui.get_f32_value(OFFSET_Y_ID).unwrap_or(0.0),
        );
        let scale = Vec2::new(
            self.nodeui
                .get_f32_value(SCALE_X_ID)
                .unwrap_or(1.0)
                .max(0.05),
            self.nodeui
                .get_f32_value(SCALE_Y_ID)
                .unwrap_or(1.0)
                .max(0.05),
        );
        let rotation = self.nodeui.get_f32_value(ROTATION_ID).unwrap_or(0.0);

        (offset, scale, rotation)
    }

    fn selected_faces(map: &Map) -> Vec<(usize, usize)> {
        let mut faces = Vec::new();

        for (object_id, face_index) in &map.selected_geometry_faces {
            if let Some(object_index) = map
                .geometry_objects
                .iter()
                .position(|object| object.id == *object_id)
            {
                let face = (object_index, *face_index);
                if !faces.contains(&face)
                    && *face_index < map.geometry_objects[object_index].faces.len()
                {
                    faces.push(face);
                }
            }
        }

        if !faces.is_empty() {
            return faces;
        }

        for object_id in &map.selected_geometry_objects {
            if let Some(object_index) = map
                .geometry_objects
                .iter()
                .position(|object| object.id == *object_id)
            {
                for face_index in 0..map.geometry_objects[object_index].faces.len() {
                    faces.push((object_index, face_index));
                }
            }
        }

        faces
    }

    fn apply_values(map: &mut Map, offset: Vec2<f32>, scale: Vec2<f32>, rotation: f32) -> bool {
        let selected_faces = Self::selected_faces(map);
        if selected_faces.is_empty() {
            return false;
        }

        let mut changed = false;
        for (object_index, face_index) in selected_faces {
            let Some(face) = map
                .geometry_objects
                .get_mut(object_index)
                .and_then(|object| object.faces.get_mut(face_index))
            else {
                continue;
            };

            if (face.texture_offset - offset).magnitude_squared() > 0.000001 {
                face.texture_offset = offset;
                changed = true;
            }
            if (face.texture_scale - scale).magnitude_squared() > 0.000001 {
                face.texture_scale = scale;
                changed = true;
            }
            if (face.texture_rotation - rotation).abs() > 0.0001 {
                face.texture_rotation = rotation;
                changed = true;
            }
        }

        changed
    }
}

impl Action for EditFaceTexture {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_edit_face_texture_desc"),
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            OFFSET_X_ID.into(),
            "Offset X".into(),
            "".into(),
            0.0,
            -64.0..=64.0,
            true,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            OFFSET_Y_ID.into(),
            "Offset Y".into(),
            "".into(),
            0.0,
            -64.0..=64.0,
            true,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            SCALE_X_ID.into(),
            "Scale X".into(),
            "".into(),
            1.0,
            0.05..=64.0,
            true,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            SCALE_Y_ID.into(),
            "Scale Y".into(),
            "".into(),
            1.0,
            0.05..=64.0,
            true,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            ROTATION_ID.into(),
            "Rotation".into(),
            "".into(),
            0.0,
            -360.0..=360.0,
            true,
        ));

        Self {
            id: TheId::named(&fl!("action_edit_face_texture")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_edit_face_texture_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
            && (!map.selected_geometry_faces.is_empty()
                || !map.selected_geometry_objects.is_empty())
    }

    fn load_params(&mut self, map: &Map) {
        let Some((object_index, face_index)) = Self::selected_faces(map).first().copied() else {
            return;
        };
        let Some(face) = map
            .geometry_objects
            .get(object_index)
            .and_then(|object| object.faces.get(face_index))
        else {
            return;
        };

        self.nodeui
            .set_f32_value(OFFSET_X_ID, face.texture_offset.x);
        self.nodeui
            .set_f32_value(OFFSET_Y_ID, face.texture_offset.y);
        self.nodeui
            .set_f32_value(SCALE_X_ID, face.texture_scale.x.max(0.05));
        self.nodeui
            .set_f32_value(SCALE_Y_ID, face.texture_scale.y.max(0.05));
        self.nodeui
            .set_f32_value(ROTATION_ID, face.texture_rotation);
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let (offset, scale, rotation) = self.texture_values();
        let prev = map.clone();
        if !Self::apply_values(map, offset, scale, rotation) {
            return None;
        }

        map.update_surfaces();
        map.changed += 1;
        RUSTERIX.write().unwrap().set_dirty();
        RUSTERIX.write().unwrap().set_overlay_dirty();
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
        project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let changed = self.nodeui.handle_event(event);
        if !changed {
            return false;
        }

        let TheEvent::ValueChanged(id, _) = event else {
            return true;
        };
        if !Self::texture_param_id(id) {
            return true;
        }

        let (offset, scale, rotation) = self.texture_values();
        let changed = {
            let Some(map) = project.get_map_mut(server_ctx) else {
                return true;
            };
            let changed = Self::apply_values(map, offset, scale, rotation);
            if changed {
                map.update_surfaces();
                map.changed += 1;
            }
            changed
        };

        if changed {
            crate::utils::editor_scene_full_rebuild(project, server_ctx);
            RUSTERIX.write().unwrap().set_overlay_dirty();
            ctx.ui.redraw_all = true;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn face_texture_edit_prefers_explicit_face_selection_over_object_selection() {
        let mut map = Map::default();
        let object = rusterix::GeometryObject::box_from_bounds(
            "Box",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
        );
        let object_id = object.id;
        map.geometry_objects.push(object);
        map.selected_geometry_objects.push(object_id);
        map.selected_geometry_faces.push((object_id, 2));

        assert!(EditFaceTexture::apply_values(
            &mut map,
            Vec2::new(0.25, 0.5),
            Vec2::new(2.0, 3.0),
            45.0,
        ));

        for (index, face) in map.geometry_objects[0].faces.iter().enumerate() {
            if index == 2 {
                assert_eq!(face.texture_offset, Vec2::new(0.25, 0.5));
                assert_eq!(face.texture_scale, Vec2::new(2.0, 3.0));
                assert_eq!(face.texture_rotation, 45.0);
            } else {
                assert_eq!(face.texture_offset, Vec2::zero());
                assert_eq!(face.texture_scale, Vec2::broadcast(1.0));
                assert_eq!(face.texture_rotation, 0.0);
            }
        }
    }

    #[test]
    fn face_texture_edit_uses_all_object_faces_without_face_selection() {
        let mut map = Map::default();
        let object = rusterix::GeometryObject::box_from_bounds(
            "Box",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
        );
        let object_id = object.id;
        map.geometry_objects.push(object);
        map.selected_geometry_objects.push(object_id);

        assert!(EditFaceTexture::apply_values(
            &mut map,
            Vec2::new(0.25, 0.5),
            Vec2::new(2.0, 3.0),
            45.0,
        ));

        for face in &map.geometry_objects[0].faces {
            assert_eq!(face.texture_offset, Vec2::new(0.25, 0.5));
            assert_eq!(face.texture_scale, Vec2::new(2.0, 3.0));
            assert_eq!(face.texture_rotation, 45.0);
        }
    }
}
