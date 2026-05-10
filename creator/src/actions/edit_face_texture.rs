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
    fn selected_faces(map: &Map) -> Vec<(usize, usize)> {
        let mut faces = Vec::new();
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

        faces
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
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            OFFSET_Y_ID.into(),
            "Offset Y".into(),
            "".into(),
            0.0,
            -64.0..=64.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            SCALE_X_ID.into(),
            "Scale X".into(),
            "".into(),
            1.0,
            0.05..=64.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            SCALE_Y_ID.into(),
            "Scale Y".into(),
            "".into(),
            1.0,
            0.05..=64.0,
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
            && (!map.selected_geometry_faces.is_empty() || !map.selected_geometry_objects.is_empty())
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
        let selected_faces = Self::selected_faces(map);
        if selected_faces.is_empty() {
            return None;
        }

        let mut changed = false;
        for (object_index, face_index) in &selected_faces {
            let Some(face) = map
                .geometry_objects
                .get(*object_index)
                .and_then(|object| object.faces.get(*face_index))
            else {
                continue;
            };
            if (face.texture_offset - offset).magnitude_squared() > 0.000001
                || (face.texture_scale - scale).magnitude_squared() > 0.000001
                || (face.texture_rotation - rotation).abs() > 0.0001
            {
                changed = true;
                break;
            }
        }
        if !changed {
            return None;
        }

        let prev = map.clone();
        for (object_index, face_index) in selected_faces {
            let Some(face) = map
                .geometry_objects
                .get_mut(object_index)
                .and_then(|object| object.faces.get_mut(face_index))
            else {
                continue;
            };
            face.texture_offset = offset;
            face.texture_scale = scale;
            face.texture_rotation = rotation;
        }

        map.update_surfaces();
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
        _project: &mut Project,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        self.nodeui.handle_event(event)
    }
}
