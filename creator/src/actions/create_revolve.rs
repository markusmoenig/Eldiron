use crate::actions::geometry_face_ops::face_uvs_for_indices;
use crate::editor_display::{EditorDisplay, EditorProfile2D, EditorProfilePreset};
use crate::prelude::*;

const SEGMENTS_ID: &str = "actionRevolveSegments";
const SMOOTH_ID: &str = "actionRevolveSmooth";

pub struct CreateRevolve {
    id: TheId,
    nodeui: TheNodeUI,
    profile: EditorProfile2D,
}

#[derive(Clone, Debug)]
enum RevolveRing {
    Axis(usize),
    Radial(Vec<usize>),
}

impl CreateRevolve {
    fn barrel_profile() -> Vec<Vec2<f32>> {
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(0.85, 0.0),
            Vec2::new(1.0, 0.25),
            Vec2::new(1.05, 1.0),
            Vec2::new(1.0, 1.75),
            Vec2::new(0.85, 2.0),
            Vec2::new(0.0, 2.0),
        ]
    }

    fn pottery_profile() -> Vec<Vec2<f32>> {
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(0.52, 0.0),
            Vec2::new(0.76, 0.16),
            Vec2::new(0.9, 0.55),
            Vec2::new(0.84, 1.05),
            Vec2::new(0.58, 1.38),
            Vec2::new(0.42, 1.58),
            Vec2::new(0.5, 1.68),
            Vec2::new(0.34, 1.68),
            Vec2::new(0.28, 1.48),
            Vec2::new(0.45, 1.28),
            Vec2::new(0.64, 1.0),
            Vec2::new(0.7, 0.58),
            Vec2::new(0.56, 0.28),
            Vec2::new(0.0, 0.2),
        ]
    }

    fn cup_profile() -> Vec<Vec2<f32>> {
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(0.52, 0.0),
            Vec2::new(0.58, 0.12),
            Vec2::new(0.62, 0.9),
            Vec2::new(0.68, 1.04),
            Vec2::new(0.48, 1.04),
            Vec2::new(0.44, 0.88),
            Vec2::new(0.42, 0.18),
            Vec2::new(0.0, 0.14),
        ]
    }

    fn default_profile() -> EditorProfile2D {
        let barrel = Self::barrel_profile();
        let pottery = Self::pottery_profile();
        let cup = Self::cup_profile();
        let mut profile = EditorProfile2D::new("Revolve Profile", barrel.clone());
        profile.presets = vec![
            EditorProfilePreset::new("Barrel", barrel),
            EditorProfilePreset::new("Pottery", pottery),
            EditorProfilePreset::new("Cup", cup),
        ];
        profile
    }

    fn clean_profile(profile: &EditorProfile2D) -> Vec<Vec2<f32>> {
        let mut points = Vec::with_capacity(profile.points.len());
        for point in &profile.points {
            if points.last().is_some_and(|previous: &Vec2<f32>| {
                (*previous - *point).magnitude_squared() <= 1e-10
            }) {
                continue;
            }
            points.push(*point);
        }
        points
    }

    fn face(indices: Vec<usize>, smoothing_group: u32) -> rusterix::GeometryFace {
        rusterix::GeometryFace {
            id: Uuid::new_v4(),
            paint_surface_id: None,
            uvs: Vec::new(),
            paint_uvs: Vec::new(),
            auto_uv: true,
            texture_offset: Vec2::zero(),
            texture_scale: Vec2::broadcast(1.0),
            texture_rotation: 0.0,
            tile: None,
            tiles: FxHashMap::default(),
            surface_points: Vec::new(),
            surface_segments: Vec::new(),
            indices,
            smoothing_group,
        }
    }

    pub(crate) fn build_object(
        profile: &EditorProfile2D,
        segments: usize,
        smooth: bool,
    ) -> Option<rusterix::GeometryObject> {
        if !profile.is_valid() || segments < 3 {
            return None;
        }
        let points = Self::clean_profile(profile);
        if points.len() < 2 {
            return None;
        }

        let mut object = rusterix::GeometryObject::new("Revolved Shape");
        object.kind = rusterix::GeometryObjectKind::Generated;
        let mut rings = Vec::with_capacity(points.len());
        for point in &points {
            let radius = (point.x - profile.axis_x).max(0.0);
            if radius <= 1e-5 {
                let index = object.vertices.len();
                object
                    .vertices
                    .push(Vec3::new(profile.axis_x, point.y, 0.0));
                rings.push(RevolveRing::Axis(index));
            } else {
                let mut ring = Vec::with_capacity(segments);
                for segment in 0..segments {
                    let angle = std::f32::consts::TAU * segment as f32 / segments as f32;
                    ring.push(object.vertices.len());
                    object.vertices.push(Vec3::new(
                        profile.axis_x + radius * angle.cos(),
                        point.y,
                        radius * angle.sin(),
                    ));
                }
                rings.push(RevolveRing::Radial(ring));
            }
        }

        for profile_index in 0..rings.len() - 1 {
            let flat = (points[profile_index + 1].y - points[profile_index].y).abs() <= 1e-5;
            let smoothing_group = if smooth && !flat { 1 } else { 0 };
            match (&rings[profile_index], &rings[profile_index + 1]) {
                (RevolveRing::Axis(_), RevolveRing::Axis(_)) => {}
                (RevolveRing::Axis(axis), RevolveRing::Radial(upper)) => {
                    for segment in 0..segments {
                        object.faces.push(Self::face(
                            vec![*axis, upper[segment], upper[(segment + 1) % segments]],
                            smoothing_group,
                        ));
                    }
                }
                (RevolveRing::Radial(lower), RevolveRing::Axis(axis)) => {
                    for segment in 0..segments {
                        object.faces.push(Self::face(
                            vec![lower[segment], *axis, lower[(segment + 1) % segments]],
                            smoothing_group,
                        ));
                    }
                }
                (RevolveRing::Radial(lower), RevolveRing::Radial(upper)) => {
                    for segment in 0..segments {
                        object.faces.push(Self::face(
                            vec![
                                lower[segment],
                                upper[segment],
                                upper[(segment + 1) % segments],
                                lower[(segment + 1) % segments],
                            ],
                            smoothing_group,
                        ));
                    }
                }
            }
        }

        if object.faces.is_empty() {
            return None;
        }
        for face_index in 0..object.faces.len() {
            let indices = object.faces[face_index].indices.clone();
            object.faces[face_index].uvs = face_uvs_for_indices(&object, &indices);
        }
        object.ensure_face_paint_data();

        let serialized_profile = profile
            .points
            .iter()
            .map(|point| [point.x, point.y])
            .collect::<Vec<_>>();
        object
            .properties
            .set("generator", rusterix::Value::Str("revolve".to_string()));
        object.properties.set(
            "revolve_profile",
            rusterix::Value::Str(serde_json::to_string(&serialized_profile).ok()?),
        );
        object
            .properties
            .set("revolve_segments", rusterix::Value::Int(segments as i32));
        object
            .properties
            .set("revolve_smooth", rusterix::Value::Bool(smooth));
        Some(object)
    }
}

impl Action for CreateRevolve {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            "Edit a side profile in the viewport and revolve it around the vertical axis.".into(),
        ));
        nodeui.add_item(TheNodeUIItem::IntEditSlider(
            SEGMENTS_ID.into(),
            "Segments".into(),
            "Radial resolution".into(),
            16,
            3..=128,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            SMOOTH_ID.into(),
            "Smooth sides".into(),
            "Share normals along non-horizontal profile sections".into(),
            true,
        ));
        Self {
            id: TheId::named("Create Revolved Shape"),
            nodeui,
            profile: Self::default_profile(),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        "Create an editable rounded shape from a 2D side profile.".to_string()
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
    }

    fn editor_display(&self) -> Option<EditorDisplay> {
        Some(EditorDisplay::Profile2D(self.profile.clone()))
    }

    fn update_editor_display(&mut self, display: &EditorDisplay) -> bool {
        let EditorDisplay::Profile2D(profile) = display;
        if !profile.is_valid() || *profile == self.profile {
            return false;
        }
        self.profile = profile.clone();
        true
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let previous = map.clone();
        let segments = self
            .nodeui
            .get_i32_value(SEGMENTS_ID)
            .unwrap_or(16)
            .clamp(3, 128) as usize;
        let smooth = self.nodeui.get_bool_value(SMOOTH_ID).unwrap_or(true);
        let mut object = Self::build_object(&self.profile, segments, smooth)?;

        let position = if server_ctx.pc.is_prefab() {
            Vec3::zero()
        } else {
            map.curr_grid_pos_3d.unwrap_or(server_ctx.geo_hit_pos)
        };
        for vertex in &mut object.vertices {
            *vertex += position;
        }
        let object_id = object.id;
        map.geometry_objects.push(object);
        map.clear_selection();
        map.selected_geometry_objects.push(object_id);
        server_ctx.curr_map_tool_type = MapToolType::Selection;
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Set Tool"),
            TheValue::Text("tool.geometry".into()),
        ));
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Map Selection Changed"),
            TheValue::Empty,
        ));

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revolved_profile_deduplicates_axis_vertices() {
        let profile = CreateRevolve::default_profile();
        let object = CreateRevolve::build_object(&profile, 8, true).unwrap();

        // Two axis points plus five radial rings.
        assert_eq!(object.vertices.len(), 2 + 5 * 8);
        assert_eq!(object.faces.len(), 6 * 8);
    }

    #[test]
    fn revolved_caps_stay_flat_and_sides_share_normals() {
        let profile = CreateRevolve::default_profile();
        let object = CreateRevolve::build_object(&profile, 8, true).unwrap();

        assert!(object.faces.iter().any(|face| face.smoothing_group == 0));
        assert!(object.faces.iter().any(|face| face.smoothing_group == 1));
    }

    #[test]
    fn generated_object_retains_its_profile_recipe() {
        let profile = CreateRevolve::default_profile();
        let object = CreateRevolve::build_object(&profile, 12, true).unwrap();

        assert_eq!(object.properties.get_str("generator"), Some("revolve"));
        assert!(object.properties.get("revolve_profile").is_some());
        assert_eq!(object.properties.get_int_default("revolve_segments", 0), 12);
    }

    #[test]
    fn built_in_presets_are_valid_revolve_profiles() {
        let profile = CreateRevolve::default_profile();
        assert_eq!(profile.presets.len(), 3);
        for preset in &profile.presets {
            let mut candidate = profile.clone();
            candidate.points = preset.points.clone();
            assert!(candidate.is_valid(), "{} preset is invalid", preset.name);
            assert!(CreateRevolve::build_object(&candidate, 12, true).is_some());
        }
    }
}
