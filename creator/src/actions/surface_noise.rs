use crate::editor::RUSTERIX;
use crate::prelude::*;
use rusterix::PixelSource;

const SURFACE_NOISE_SCALE_ID: &str = "actionNoiseScale";
const SURFACE_NOISE_AMOUNT_ID: &str = "actionNoiseAmount";
const SURFACE_NOISE_SEED_ID: &str = "actionNoiseSeed";

pub struct SurfaceNoise {
    id: TheId,
    nodeui: TheNodeUI,
    source_override: Option<Option<PixelSource>>,
}

fn selected_face_ids(map: &Map, server_ctx: &ServerContext) -> Vec<(Uuid, usize)> {
    if server_ctx.get_map_context() != MapContext::Region
        || server_ctx.editor_view_mode == EditorViewMode::D2
    {
        return Vec::new();
    }
    map.selected_geometry_faces.clone()
}

fn build_surface_noise_nodeui() -> TheNodeUI {
    let mut nodeui = TheNodeUI::default();
    nodeui.add_item(TheNodeUIItem::Markdown(
        "desc".into(),
        fl!("action_surface_noise_desc"),
    ));
    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
        SURFACE_NOISE_SCALE_ID.into(),
        "Scale".into(),
        "".into(),
        1.0,
        0.05..=500.0,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
        SURFACE_NOISE_AMOUNT_ID.into(),
        "Amount".into(),
        "".into(),
        0.35,
        0.0..=1.0,
        false,
    ));
    nodeui.add_item(TheNodeUIItem::IntEditSlider(
        SURFACE_NOISE_SEED_ID.into(),
        "Seed".into(),
        "".into(),
        0,
        0..=65535,
        false,
    ));
    nodeui
}

impl Action for SurfaceNoise {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named(&fl!("action_surface_noise")),
            nodeui: build_surface_noise_nodeui(),
            source_override: None,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_surface_noise_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        !selected_face_ids(map, server_ctx).is_empty()
    }

    fn load_params(&mut self, map: &Map) {
        let Some((object_id, face_index)) = map.selected_geometry_faces.first().copied() else {
            return;
        };
        let Some(object) = map
            .geometry_objects
            .iter()
            .find(|object| object.id == object_id)
        else {
            return;
        };
        let Some(noise) = object
            .faces
            .get(face_index)
            .and_then(|face| face.surface_noise.as_ref())
        else {
            self.source_override = None;
            return;
        };
        self.nodeui
            .set_f32_value(SURFACE_NOISE_SCALE_ID, noise.scale);
        self.nodeui
            .set_f32_value(SURFACE_NOISE_AMOUNT_ID, noise.amount);
        self.nodeui.set_i32_value(SURFACE_NOISE_SEED_ID, noise.seed);
        self.source_override = None;
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
                fl!("status_surface_noise_needs_face"),
            ));
            return None;
        }

        let source = self.source_override.clone().unwrap_or_else(|| {
            selected_faces.first().and_then(|(object_id, face_index)| {
                map.geometry_objects
                    .iter()
                    .find(|object| object.id == *object_id)
                    .and_then(|object| object.faces.get(*face_index))
                    .and_then(|face| face.surface_noise.as_ref())
                    .and_then(|noise| noise.source.clone())
            })
        });

        if source.is_none() {
            return clear_surface_noise_on_faces(map, ctx, server_ctx, selected_faces);
        }

        let noise = rusterix::GeometrySurfaceNoise {
            scale: self
                .nodeui
                .get_f32_value(SURFACE_NOISE_SCALE_ID)
                .unwrap_or(1.0)
                .max(0.05),
            amount: self
                .nodeui
                .get_f32_value(SURFACE_NOISE_AMOUNT_ID)
                .unwrap_or(0.35)
                .clamp(0.0, 1.0),
            seed: self
                .nodeui
                .get_i32_value(SURFACE_NOISE_SEED_ID)
                .unwrap_or(0),
            source,
        };

        let prev = map.clone();
        let mut changed = false;
        for (object_id, face_index) in selected_faces {
            let Some(object) = map
                .geometry_objects
                .iter_mut()
                .find(|object| object.id == object_id)
            else {
                continue;
            };
            let Some(face) = object.faces.get_mut(face_index) else {
                continue;
            };
            if face.surface_noise.as_ref() != Some(&noise) {
                face.surface_noise = Some(noise.clone());
                changed = true;
            }
        }

        if !changed {
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

    fn hud_material_slots(
        &self,
        map: &Map,
        server_ctx: &ServerContext,
    ) -> Option<Vec<ActionMaterialSlot>> {
        if selected_face_ids(map, server_ctx).is_empty() {
            return None;
        }
        let selected_source =
            map.selected_geometry_faces
                .first()
                .and_then(|(object_id, face_index)| {
                    map.geometry_objects
                        .iter()
                        .find(|object| object.id == *object_id)
                        .and_then(|object| object.faces.get(*face_index))
                        .and_then(|face| face.surface_noise.as_ref())
                        .and_then(|noise| noise.source.clone())
                });
        let source = self.source_override.clone().unwrap_or(selected_source);
        Some(vec![ActionMaterialSlot {
            label: "NOISE".to_string(),
            source,
        }])
    }

    fn set_hud_material_from_tile(
        &mut self,
        map: &Map,
        server_ctx: &ServerContext,
        slot_index: i32,
        tile_id: Uuid,
    ) -> bool {
        self.set_hud_material_source(map, server_ctx, slot_index, PixelSource::TileId(tile_id))
    }

    fn set_hud_material_source(
        &mut self,
        _map: &Map,
        _server_ctx: &ServerContext,
        slot_index: i32,
        source: PixelSource,
    ) -> bool {
        if slot_index != 0 {
            return false;
        }
        let source = Some(source);
        if self.source_override.as_ref() == Some(&source) {
            return false;
        }
        self.source_override = Some(source);
        true
    }

    fn clear_hud_material_slot(
        &mut self,
        _map: &Map,
        _server_ctx: &ServerContext,
        slot_index: i32,
    ) -> bool {
        if slot_index != 0 {
            return false;
        }
        if self.source_override == Some(None) {
            return false;
        }
        self.source_override = Some(None);
        true
    }
}

fn clear_surface_noise_on_faces(
    map: &mut Map,
    ctx: &mut TheContext,
    server_ctx: &mut ServerContext,
    selected_faces: Vec<(Uuid, usize)>,
) -> Option<ProjectUndoAtom> {
    let prev = map.clone();
    let mut changed = false;
    for (object_id, face_index) in selected_faces {
        let Some(object) = map
            .geometry_objects
            .iter_mut()
            .find(|object| object.id == object_id)
        else {
            continue;
        };
        let Some(face) = object.faces.get_mut(face_index) else {
            continue;
        };
        if face.surface_noise.take().is_some() {
            changed = true;
        }
    }

    if !changed {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_noise_only_applies_to_selected_faces() {
        let mut map = Map::default();
        let object = rusterix::GeometryObject::box_from_bounds(
            "Box",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
        );
        let object_id = object.id;
        map.geometry_objects.push(object);
        map.selected_geometry_faces.push((object_id, 2));

        let mut action = SurfaceNoise::new();
        action.source_override = Some(Some(PixelSource::TileId(Uuid::new_v4())));
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
        for (index, face) in map.geometry_objects[0].faces.iter().enumerate() {
            if index == 2 {
                assert!(face.surface_noise.is_some());
            } else {
                assert!(face.surface_noise.is_none());
            }
        }
    }

    #[test]
    fn surface_noise_clear_slot_removes_selected_face_noise() {
        let mut map = Map::default();
        let mut object = rusterix::GeometryObject::box_from_bounds(
            "Box",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
        );
        let object_id = object.id;
        object.faces[1].surface_noise = Some(rusterix::GeometrySurfaceNoise {
            scale: 2.0,
            amount: 0.5,
            seed: 7,
            source: Some(PixelSource::TileId(Uuid::new_v4())),
        });
        object.faces[2].surface_noise = Some(rusterix::GeometrySurfaceNoise {
            scale: 2.0,
            amount: 0.5,
            seed: 7,
            source: Some(PixelSource::TileId(Uuid::new_v4())),
        });
        map.geometry_objects.push(object);
        map.selected_geometry_faces.push((object_id, 2));

        let mut action = SurfaceNoise::new();
        action.source_override = Some(None);
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
        assert!(map.geometry_objects[0].faces[1].surface_noise.is_some());
        assert!(map.geometry_objects[0].faces[2].surface_noise.is_none());
    }

    #[test]
    fn surface_noise_exposes_noise_hud_material_slot() {
        let mut map = Map::default();
        let object = rusterix::GeometryObject::box_from_bounds(
            "Box",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
        );
        let object_id = object.id;
        map.geometry_objects.push(object);
        map.selected_geometry_faces.push((object_id, 2));

        let mut action = SurfaceNoise::new();
        let mut server_ctx = ServerContext::default();
        server_ctx.pc = ProjectContext::Region(Uuid::new_v4());
        server_ctx.editor_view_mode = EditorViewMode::Iso;

        let tile_id = Uuid::new_v4();
        assert!(action.set_hud_material_from_tile(&map, &server_ctx, 0, tile_id));
        let slots = action.hud_material_slots(&map, &server_ctx).unwrap();
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].label, "NOISE");
        assert_eq!(slots[0].source, Some(PixelSource::TileId(tile_id)));
        assert!(action.clear_hud_material_slot(&map, &server_ctx, 0));
        let slots = action.hud_material_slots(&map, &server_ctx).unwrap();
        assert_eq!(slots[0].source, None);
    }
}
