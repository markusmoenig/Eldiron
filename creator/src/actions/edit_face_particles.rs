use crate::editor::RUSTERIX;
use crate::prelude::*;

const ENABLED_ID: &str = "actionFaceParticlesEnabled";
const AMOUNT_ID: &str = "actionFaceParticlesAmount";
const SIZE_ID: &str = "actionFaceParticlesSize";
const SPEED_ID: &str = "actionFaceParticlesSpeed";
const LIFETIME_ID: &str = "actionFaceParticlesLifetime";
const DRIFT_ID: &str = "actionFaceParticlesDrift";
const OFFSET_ID: &str = "actionFaceParticlesOffset";
const PALETTE_LINKED_ID: &str = "actionFaceParticlesPaletteLinked";
const PALETTE_COLORS_ID: &str = "actionFaceParticlesPaletteColors";
const COLOR_IDS: [&str; 4] = [
    "actionFaceParticlesColor1",
    "actionFaceParticlesColor2",
    "actionFaceParticlesColor3",
    "actionFaceParticlesColor4",
];

pub struct EditFaceParticles {
    id: TheId,
    nodeui: TheNodeUI,
}

impl EditFaceParticles {
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

    fn picker_color(&self, id: &str, fallback: [u8; 4]) -> [u8; 4] {
        let Some(TheNodeUIItem::ColorPicker(_, _, _, color, _)) = self.nodeui.get_item(id) else {
            return fallback;
        };
        color.to_u8_array()
    }

    fn values(&self) -> (bool, rusterix::FaceParticleEmission) {
        let mut settings = rusterix::FaceParticleEmission::default();
        let default_emitter = settings.emitter.clone();
        let size = self
            .nodeui
            .get_f32_value(SIZE_ID)
            .unwrap_or((default_emitter.radius_range.0 + default_emitter.radius_range.1) * 0.5)
            .max(0.005);
        let speed = self
            .nodeui
            .get_f32_value(SPEED_ID)
            .unwrap_or((default_emitter.speed_range.0 + default_emitter.speed_range.1) * 0.5)
            .max(0.0);
        let lifetime = self
            .nodeui
            .get_f32_value(LIFETIME_ID)
            .unwrap_or((default_emitter.lifetime_range.0 + default_emitter.lifetime_range.1) * 0.5)
            .max(0.05);
        let fallback_ramp = default_emitter
            .color_ramp
            .unwrap_or([default_emitter.color; 4]);
        let mut ramp = [
            self.picker_color(COLOR_IDS[0], fallback_ramp[0]),
            self.picker_color(COLOR_IDS[1], fallback_ramp[1]),
            self.picker_color(COLOR_IDS[2], fallback_ramp[2]),
            self.picker_color(COLOR_IDS[3], fallback_ramp[3]),
        ];

        settings.emitter.rate = self
            .nodeui
            .get_f32_value(AMOUNT_ID)
            .unwrap_or(default_emitter.rate)
            .max(0.0);
        settings.emitter.radius_range = (size * 0.6, size * 1.4);
        settings.emitter.speed_range = (speed * 0.6, speed * 1.4);
        settings.emitter.lifetime_range = (lifetime * 0.7, lifetime * 1.3);
        settings.emitter.turbulence = self
            .nodeui
            .get_f32_value(DRIFT_ID)
            .unwrap_or(default_emitter.turbulence)
            .max(0.0);
        settings.palette_indices = if self
            .nodeui
            .get_bool_value(PALETTE_LINKED_ID)
            .unwrap_or(false)
        {
            let mut indices = [None; 4];
            if let Some(TheNodeUIItem::PaletteIndexRowPicker(_, _, _, values, palette)) =
                self.nodeui.get_item(PALETTE_COLORS_ID)
            {
                for (slot, value) in values.iter().take(4).enumerate() {
                    indices[slot] = Some((*value).clamp(0, u16::MAX as i32) as u16);
                    if let Some(Some(color)) = palette.colors.get((*value).max(0) as usize) {
                        ramp[slot] = color.to_u8_array();
                    }
                }
            }
            indices
        } else {
            [None; 4]
        };
        settings.emitter.color = ramp[1];
        settings.emitter.color_ramp = Some(ramp);
        settings.offset = self
            .nodeui
            .get_f32_value(OFFSET_ID)
            .unwrap_or(settings.offset);

        (
            self.nodeui.get_bool_value(ENABLED_ID).unwrap_or(false),
            settings,
        )
    }

    fn apply_values(
        map: &mut Map,
        enabled: bool,
        settings: &rusterix::FaceParticleEmission,
    ) -> bool {
        let ids = Self::selected_surface_ids(map);
        if ids.is_empty() {
            return false;
        }
        let mut changed = false;
        for id in ids {
            if enabled {
                if map.face_particle_emissions.get(&id) != Some(settings) {
                    map.face_particle_emissions.insert(id, settings.clone());
                    changed = true;
                }
            } else if map.face_particle_emissions.remove(&id).is_some() {
                changed = true;
            }
        }
        changed
    }

    fn set_color(&mut self, id: &str, color: [u8; 4]) {
        if let Some(TheNodeUIItem::ColorPicker(_, _, _, value, _)) = self.nodeui.get_item_mut(id) {
            *value = TheColor::from(color);
        }
    }
}

impl Action for EditFaceParticles {
    fn new() -> Self
    where
        Self: Sized,
    {
        let defaults = rusterix::FaceParticleEmission::default();
        let emitter = &defaults.emitter;
        let ramp = emitter.color_ramp.unwrap_or([emitter.color; 4]);
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_edit_face_particles_desc"),
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            ENABLED_ID.into(),
            fl!("action_face_particles_enabled"),
            String::new(),
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            AMOUNT_ID.into(),
            fl!("action_face_particles_amount"),
            String::new(),
            emitter.rate,
            0.0..=200.0,
            true,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            SIZE_ID.into(),
            fl!("action_face_particles_size"),
            String::new(),
            (emitter.radius_range.0 + emitter.radius_range.1) * 0.5,
            0.005..=1.0,
            true,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            SPEED_ID.into(),
            fl!("action_face_particles_speed"),
            String::new(),
            (emitter.speed_range.0 + emitter.speed_range.1) * 0.5,
            0.0..=4.0,
            true,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            LIFETIME_ID.into(),
            fl!("action_face_particles_lifetime"),
            String::new(),
            (emitter.lifetime_range.0 + emitter.lifetime_range.1) * 0.5,
            0.05..=12.0,
            true,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            DRIFT_ID.into(),
            fl!("action_face_particles_drift"),
            String::new(),
            emitter.turbulence,
            0.0..=2.0,
            true,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            OFFSET_ID.into(),
            fl!("action_face_particles_offset"),
            String::new(),
            defaults.offset,
            -2.0..=2.0,
            true,
        ));
        for (index, id) in COLOR_IDS.iter().enumerate() {
            nodeui.add_item(TheNodeUIItem::ColorPicker(
                (*id).into(),
                format!("{} {}", fl!("action_face_particles_colors"), index + 1),
                String::new(),
                TheColor::from(ramp[index]),
                true,
            ));
        }
        nodeui.add_item(TheNodeUIItem::Checkbox(
            PALETTE_LINKED_ID.into(),
            fl!("action_face_particles_palette_linked"),
            fl!("status_action_face_particles_palette_linked"),
            false,
        ));
        nodeui.add_item(TheNodeUIItem::PaletteIndexRowPicker(
            PALETTE_COLORS_ID.into(),
            fl!("action_face_particles_palette_colors"),
            fl!("status_action_face_particles_palette_colors"),
            vec![0; 4],
            ThePalette::default(),
        ));
        Self {
            id: TheId::named(&fl!("action_edit_face_particles")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_edit_face_particles_desc")
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
        let existing = map.face_particle_emissions.get(&surface_id);
        let settings = existing.cloned().unwrap_or_default();
        let emitter = &settings.emitter;
        self.nodeui.set_bool_value(ENABLED_ID, existing.is_some());
        self.nodeui.set_f32_value(AMOUNT_ID, emitter.rate);
        self.nodeui.set_f32_value(
            SIZE_ID,
            (emitter.radius_range.0 + emitter.radius_range.1) * 0.5,
        );
        self.nodeui.set_f32_value(
            SPEED_ID,
            (emitter.speed_range.0 + emitter.speed_range.1) * 0.5,
        );
        self.nodeui.set_f32_value(
            LIFETIME_ID,
            (emitter.lifetime_range.0 + emitter.lifetime_range.1) * 0.5,
        );
        self.nodeui.set_f32_value(DRIFT_ID, emitter.turbulence);
        self.nodeui.set_f32_value(OFFSET_ID, settings.offset);
        let ramp = emitter.color_ramp.unwrap_or([emitter.color; 4]);
        for (index, id) in COLOR_IDS.iter().enumerate() {
            self.set_color(id, ramp[index]);
        }
        let palette_linked = settings.palette_indices.iter().any(Option::is_some);
        self.nodeui
            .set_bool_value(PALETTE_LINKED_ID, palette_linked);
        if palette_linked
            && let Some(TheNodeUIItem::PaletteIndexRowPicker(_, _, _, values, _)) =
                self.nodeui.get_item_mut(PALETTE_COLORS_ID)
        {
            *values = settings
                .palette_indices
                .iter()
                .map(|index| index.unwrap_or(0) as i32)
                .collect();
        }
    }

    fn load_params_project(&mut self, project: &Project, server_ctx: &mut ServerContext) {
        let settings = project
            .get_map(server_ctx)
            .and_then(|map| {
                Self::selected_surface_ids(map)
                    .first()
                    .copied()
                    .map(|id| (map, id))
            })
            .and_then(|(map, id)| map.face_particle_emissions.get(&id));
        let fallback = rusterix::FaceParticleEmission::default();
        let settings = settings.unwrap_or(&fallback);
        let ramp = settings
            .emitter
            .color_ramp
            .unwrap_or([settings.emitter.color; 4]);
        if let Some(TheNodeUIItem::PaletteIndexRowPicker(_, _, _, values, palette)) =
            self.nodeui.get_item_mut(PALETTE_COLORS_ID)
        {
            *palette = project.art_palette.clone();
            *values = (0..4)
                .map(|slot| {
                    settings.palette_indices[slot]
                        .map(i32::from)
                        .or_else(|| {
                            project
                                .art_palette
                                .find_closest_color_index(&TheColor::from(ramp[slot]))
                                .map(|index| index as i32)
                        })
                        .unwrap_or(project.art_palette.current_index as i32)
                })
                .collect();
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
        let (enabled, settings) = self.values();
        if !Self::apply_values(map, enabled, &settings) {
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
