use crate::editor::RUSTERIX;
use crate::prelude::*;

const ENABLED_ID: &str = "actionFaceEmissionEnabled";
const COLOR_ID: &str = "actionFaceEmissionColor";
const INTENSITY_ID: &str = "actionFaceEmissionIntensity";
const START_DISTANCE_ID: &str = "actionFaceEmissionStartDistance";
const END_DISTANCE_ID: &str = "actionFaceEmissionEndDistance";
const OFFSET_ID: &str = "actionFaceEmissionOffset";
const FLICKER_ID: &str = "actionFaceEmissionFlicker";

pub struct EditFaceEmission {
    id: TheId,
    nodeui: TheNodeUI,
}

impl EditFaceEmission {
    fn selected_surface_ids(map: &Map) -> Vec<Uuid> {
        let mut ids = Vec::new();
        for (object_id, face_index) in &map.selected_geometry_faces {
            let Some(face) = map
                .geometry_objects
                .iter()
                .find(|object| object.id == *object_id)
                .and_then(|object| object.faces.get(*face_index))
            else {
                continue;
            };
            let id = rusterix::geometry_face_effective_paint_surface_id(face);
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids
    }

    fn color(&self) -> [f32; 3] {
        let Some(TheNodeUIItem::ColorPicker(_, _, _, color, _)) = self.nodeui.get_item(COLOR_ID)
        else {
            return rusterix::FaceEmission::default().color;
        };
        [color.r, color.g, color.b]
    }

    fn values(&self) -> (bool, rusterix::FaceEmission) {
        let defaults = rusterix::FaceEmission::default();
        let end_distance = self
            .nodeui
            .get_f32_value(END_DISTANCE_ID)
            .unwrap_or(defaults.end_distance)
            .max(0.05);
        (
            self.nodeui.get_bool_value(ENABLED_ID).unwrap_or(false),
            rusterix::FaceEmission {
                color: self.color(),
                intensity: self
                    .nodeui
                    .get_f32_value(INTENSITY_ID)
                    .unwrap_or(defaults.intensity)
                    .max(0.0),
                start_distance: self
                    .nodeui
                    .get_f32_value(START_DISTANCE_ID)
                    .unwrap_or(defaults.start_distance)
                    .clamp(0.0, end_distance),
                end_distance,
                offset: self
                    .nodeui
                    .get_f32_value(OFFSET_ID)
                    .unwrap_or(defaults.offset),
                flicker: self
                    .nodeui
                    .get_f32_value(FLICKER_ID)
                    .unwrap_or(defaults.flicker)
                    .max(0.0),
            },
        )
    }

    fn apply_values(map: &mut Map, enabled: bool, emission: &rusterix::FaceEmission) -> bool {
        let ids = Self::selected_surface_ids(map);
        if ids.is_empty() {
            return false;
        }
        let mut changed = false;
        for id in ids {
            if enabled {
                if map.face_emissions.get(&id) != Some(emission) {
                    map.face_emissions.insert(id, emission.clone());
                    changed = true;
                }
            } else if map.face_emissions.remove(&id).is_some() {
                changed = true;
            }
        }
        changed
    }
}

impl Action for EditFaceEmission {
    fn new() -> Self
    where
        Self: Sized,
    {
        let defaults = rusterix::FaceEmission::default();
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_edit_face_emission_desc"),
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            ENABLED_ID.into(),
            fl!("action_face_emission_enabled"),
            String::new(),
            false,
        ));
        nodeui.add_item(TheNodeUIItem::ColorPicker(
            COLOR_ID.into(),
            fl!("action_face_emission_color"),
            String::new(),
            TheColor::from_vec3(Vec3::from(defaults.color)),
            true,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            INTENSITY_ID.into(),
            fl!("action_face_emission_intensity"),
            String::new(),
            defaults.intensity,
            0.0..=20.0,
            true,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            START_DISTANCE_ID.into(),
            fl!("action_face_emission_soft_radius"),
            String::new(),
            defaults.start_distance,
            0.0..=16.0,
            true,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            END_DISTANCE_ID.into(),
            fl!("action_face_emission_range"),
            String::new(),
            defaults.end_distance,
            0.05..=64.0,
            true,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            OFFSET_ID.into(),
            fl!("action_face_emission_offset"),
            String::new(),
            defaults.offset,
            -2.0..=2.0,
            true,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            FLICKER_ID.into(),
            fl!("action_face_emission_flicker"),
            String::new(),
            defaults.flicker,
            0.0..=1.0,
            true,
        ));
        Self {
            id: TheId::named(&fl!("action_edit_face_emission")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_edit_face_emission_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
            && !map.selected_geometry_faces.is_empty()
    }

    fn load_params(&mut self, map: &Map) {
        let Some(surface_id) = Self::selected_surface_ids(map).first().copied() else {
            return;
        };
        let existing = map.face_emissions.get(&surface_id);
        let emission = existing.cloned().unwrap_or_default();
        self.nodeui.set_bool_value(ENABLED_ID, existing.is_some());
        self.nodeui.set_f32_value(INTENSITY_ID, emission.intensity);
        self.nodeui
            .set_f32_value(START_DISTANCE_ID, emission.start_distance);
        self.nodeui
            .set_f32_value(END_DISTANCE_ID, emission.end_distance);
        self.nodeui.set_f32_value(OFFSET_ID, emission.offset);
        self.nodeui.set_f32_value(FLICKER_ID, emission.flicker);
        if let Some(TheNodeUIItem::ColorPicker(_, _, _, color, _)) =
            self.nodeui.get_item_mut(COLOR_ID)
        {
            *color = TheColor::from_vec3(Vec3::from(emission.color));
        }
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let previous = map.clone();
        let (enabled, emission) = self.values();
        if !Self::apply_values(map, enabled, &emission) {
            return None;
        }
        map.changed = map.changed.wrapping_add(1);
        RUSTERIX.write().unwrap().set_dirty();
        RUSTERIX.write().unwrap().set_overlay_dirty();
        Some(ProjectUndoAtom::MapEdit(
            server_ctx.pc,
            Box::new(previous),
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
