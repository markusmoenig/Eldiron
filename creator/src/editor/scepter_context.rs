//! Shared view context and a bounded, cursor-based collaboration change feed.
use super::scepter_authoring::{parse_id, region_index};
use super::*;
use eldiron_scepter::{ChangeQuery, EditorFocus, EditorSelection};
use serde_json::{Value as Json, json};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

pub(super) struct Tracking {
    stream: Uuid,
    sequence: u64,
    events: VecDeque<Json>,
    last_client: Option<Instant>,
    sampled: Option<Instant>,
    view: Json,
    stamp: u64,
    revision: String,
}
impl Default for Tracking {
    fn default() -> Self {
        Self {
            stream: Uuid::new_v4(),
            sequence: 0,
            events: VecDeque::new(),
            last_client: None,
            sampled: None,
            view: Json::Null,
            stamp: 0,
            revision: String::new(),
        }
    }
}
impl Tracking {
    fn push(&mut self, session: &str, kind: &str, revision: &str, context: Json) {
        self.sequence += 1;
        self.events.push_back(json!({"sequence":self.sequence,"session":session,"kind":kind,"revision":revision,"context":context}));
        while self.events.len() > 256 {
            self.events.pop_front();
        }
    }
}
impl Editor {
    pub(super) fn scepter_view(&self, ui: &mut TheUI) -> Json {
        let cameras = EDITCAMERA.read().unwrap();
        let viewport = ui.get_widget("PolyView").map(|w| {
            let d = w.dim();
            json!({"x":d.x,"y":d.y,"width":d.width,"height":d.height})
        });
        let tools = TOOLLIST.read().unwrap();
        let map = self.project.get_map(&self.server_ctx);
        let region = self
            .project
            .regions
            .iter()
            .find(|r| r.id == self.server_ctx.curr_region);
        json!({"session":self.scepter_session(),"region":region.map(|r|json!({"id":r.id,"name":r.name,"map":r.map.id})),
            "project_context":format!("{:?}",self.server_ctx.pc),"map_context":format!("{:?}",self.server_ctx.get_map_context()),
            "view":match self.server_ctx.editor_view_mode{EditorViewMode::D2=>if self.project.get_map(&self.server_ctx).is_some_and(|m|m.camera==rusterix::MapCamera::TwoDIso){"2d_iso"}else{"2d"},EditorViewMode::Iso=>"iso",EditorViewMode::Orbit=>"orbit",EditorViewMode::FirstP=>"firstp"},
            "tool":{"command":tools.current_game_tool_command_id(),"editor_mode":tools.editor_mode,"editor_index":tools.curr_editor_tool},
            "selected_tile":self.server_ctx.curr_tile_id,"viewport":viewport,"camera_parameters":{"iso_scale":cameras.iso_camera.scale,"iso_azimuth":cameras.iso_camera.azimuth_deg,"iso_elevation":cameras.iso_camera.elevation_deg,"orbit_distance":cameras.orbit_camera.distance},
            "camera":region.map(|r|json!({"position":r.editing_position_3d.into_array(),"center":r.editing_look_at_3d.into_array(),"offset":r.map.offset.into_array(),"grid_size":r.map.grid_size})),
            "selection":map.map(|m|json!({"vertices":m.selected_vertices,"linedefs":m.selected_linedefs,"sectors":m.selected_sectors,"geometry":m.selected_geometry_objects,"faces":m.selected_geometry_faces,"geometry_vertices":m.selected_geometry_vertices,"entity":m.selected_entity_item})),
            "cursor":map.map(|m|json!({"grid":m.curr_grid_pos.map(|p|p.into_array()),"world":m.curr_grid_pos_3d.map(|p|p.into_array())}))})
    }
    pub(super) fn scepter_observe(&mut self, force: bool, ui: &mut TheUI) {
        let now = Instant::now();
        if !force
            && (self
                .scepter_tracking
                .last_client
                .is_none_or(|t| now.duration_since(t) > Duration::from_secs(120))
                || self
                    .scepter_tracking
                    .sampled
                    .is_some_and(|t| now.duration_since(t) < Duration::from_millis(200)))
        {
            return;
        }
        let view = self.scepter_view(ui);
        let mut hasher = DefaultHasher::new();
        UNDOMANAGER.read().unwrap().revision().hash(&mut hasher);
        self.project
            .procedural_materials
            .revision()
            .hash(&mut hasher);
        self.project
            .procedural_appearances
            .revision()
            .hash(&mut hasher);
        for r in &self.project.regions {
            r.id.hash(&mut hasher);
            r.map.changed.hash(&mut hasher);
            r.characters.len().hash(&mut hasher);
            r.items.len().hash(&mut hasher);
        }
        self.project.config.hash(&mut hasher);
        serde_json::to_string(&self.project.palette.colors)
            .unwrap()
            .hash(&mut hasher);
        let stamp = hasher.finish();
        let revision = if force || stamp != self.scepter_tracking.stamp {
            self.scepter_revision()
        } else {
            self.scepter_tracking.revision.clone()
        };
        let session = self.scepter_session();
        if revision != self.scepter_tracking.revision {
            self.scepter_tracking.push(&session,"content",&revision,json!({"regions":self.project.regions.iter().map(|r|json!({"id":r.id,"map":r.map.id,"changed":r.map.changed})).collect::<Vec<_>>()}));
        }
        if view != self.scepter_tracking.view {
            let kind = if view["session"] != self.scepter_tracking.view["session"] {
                "session"
            } else if view["selection"] != self.scepter_tracking.view["selection"] {
                "selection"
            } else {
                "view"
            };
            self.scepter_tracking
                .push(&session, kind, &revision, view.clone());
        }
        self.scepter_tracking.view = view;
        self.scepter_tracking.stamp = stamp;
        self.scepter_tracking.revision = revision;
        self.scepter_tracking.sampled = Some(now);
    }
    pub(super) fn scepter_context(&mut self, ui: &mut TheUI) -> Json {
        self.scepter_tracking.last_client = Some(Instant::now());
        self.scepter_observe(true, ui);
        let mut v = self.scepter_view(ui);
        v["revision"] = json!(self.scepter_revision());
        if let Some(w) = ui.get_widget("PolyView") {
            let d = w.dim();
            v["viewport"] = json!({"x":d.x,"y":d.y,"width":d.width,"height":d.height});
        }
        v["stream"] = json!(self.scepter_tracking.stream);
        v["cursor_sequence"] = json!(self.scepter_tracking.sequence);
        json!({"context":v})
    }
    pub(super) fn scepter_changes(&mut self, q: ChangeQuery, ui: &mut TheUI) -> Json {
        let context = self.scepter_context(ui);
        let t = &self.scepter_tracking;
        let reset = q
            .stream
            .as_ref()
            .is_some_and(|s| *s != t.stream.to_string())
            || q.after > t.sequence
            || t.events
                .front()
                .is_some_and(|e| q.after.saturating_add(1) < e["sequence"].as_u64().unwrap());
        let events = if reset {
            vec![]
        } else {
            t.events
                .iter()
                .filter(|e| e["sequence"].as_u64().unwrap() > q.after)
                .take(q.limit.unwrap_or(64).clamp(1, 128))
                .cloned()
                .collect::<Vec<_>>()
        };
        let next = if reset {
            t.sequence
        } else {
            events
                .last()
                .and_then(|e| e["sequence"].as_u64())
                .unwrap_or(q.after)
        };
        json!({"stream":t.stream,"events":events,"next":next,"latest":t.sequence,"has_more":next<t.sequence,"reset_required":reset,"context":context["context"]})
    }
    fn scepter_activate_region(&mut self, i: usize, ui: &mut TheUI, ctx: &mut TheContext) {
        let id = self.project.regions[i].id;
        if self.server_ctx.curr_region != id
            || self.server_ctx.get_map_context() != MapContext::Region
        {
            self.server_ctx.curr_region = id;
            self.server_ctx.editing_pos_buffer = None;
            crate::undo::project_helper::set_project_context(
                ctx,
                ui,
                &self.project,
                &mut self.server_ctx,
                ProjectContext::Region(id),
            );
            let _ = crate::utils::update_region_settings(&mut self.project, &mut self.server_ctx);
            crate::utils::editor_scene_full_rebuild(&self.project, &self.server_ctx);
        }
    }
    fn scepter_context_region(&self, region: Option<&RegionRef>) -> Result<usize, String> {
        if let Some(r) = region {
            region_index(&self.project, r)
        } else {
            self.project
                .regions
                .iter()
                .position(|r| r.id == self.server_ctx.curr_region)
                .ok_or("No active region".into())
        }
    }
    pub(super) fn scepter_select(
        &mut self,
        c: EditorSelection,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> Result<Json, String> {
        let i = self.scepter_context_region(c.region.as_ref())?;
        super::scepter_map::validate_targets(&self.project, i, &c.targets)?;
        if c.targets.characters.len() + c.targets.items.len() > 1 {
            return Err("Select one entity at a time".into());
        }
        let map = &self.project.regions[i].map;
        if c.vertices.iter().any(|id| map.find_vertex(*id).is_none())
            || c.linedefs.iter().any(|id| map.find_linedef(*id).is_none())
        {
            return Err("Selected vertex/line does not exist".into());
        }
        let faces = c
            .faces
            .iter()
            .map(|(id, index)| {
                let id = parse_id(id)?;
                if !map
                    .geometry_objects
                    .iter()
                    .any(|o| o.id == id && o.faces.get(*index).is_some())
                {
                    return Err("Selected face does not exist".to_string());
                }
                Ok((id, *index))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let vertices = c
            .geometry_vertices
            .iter()
            .map(|(id, index)| {
                let id = parse_id(id)?;
                if !map
                    .geometry_objects
                    .iter()
                    .any(|o| o.id == id && o.vertices.get(*index).is_some())
                {
                    return Err("Selected geometry vertex does not exist".to_string());
                }
                Ok((id, *index))
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.scepter_activate_region(i, ui, ctx);
        let map = &mut self.project.regions[i].map;
        map.clear_selection();
        map.selected_vertices = c.vertices;
        map.selected_linedefs = c.linedefs;
        map.selected_sectors = c.targets.sectors;
        map.selected_geometry_objects = c
            .targets
            .geometry
            .iter()
            .map(|s| parse_id(s))
            .collect::<Result<_, _>>()?;
        map.selected_geometry_faces = faces;
        map.selected_geometry_vertices = vertices;
        map.geometry_selection_mode = if map.selected_geometry_faces.is_empty() {
            0
        } else {
            2
        };
        map.selected_entity_item = c
            .targets
            .characters
            .first()
            .or(c.targets.items.first())
            .map(|s| parse_id(s))
            .transpose()?;
        let pc = if let Some(entity) = self.project.regions[i].map.selected_entity_item {
            if self.project.regions[i].characters.contains_key(&entity) {
                ProjectContext::RegionCharacterInstance(self.project.regions[i].id, entity)
            } else {
                ProjectContext::RegionItemInstance(self.project.regions[i].id, entity)
            }
        } else {
            ProjectContext::Region(self.project.regions[i].id)
        };
        if self.server_ctx.pc != pc {
            crate::undo::project_helper::set_project_context(
                ctx,
                ui,
                &self.project,
                &mut self.server_ctx,
                pc,
            );
        }
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Minimap"),
            TheValue::Empty,
        ));
        TOOLLIST
            .write()
            .unwrap()
            .update_geometry_overlay_3d(&mut self.project, &mut self.server_ctx);
        Ok(self.scepter_context(ui))
    }
    pub(super) fn scepter_focus(
        &mut self,
        c: EditorFocus,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> Result<Json, String> {
        let i = self.scepter_context_region(c.region.as_ref())?;
        let center = super::scepter_map::point(c.center)?;
        let pos = c.position.map(super::scepter_map::point).transpose()?;
        let view = match c.view.as_deref() {
            None => self.server_ctx.editor_view_mode,
            Some("2d") | Some("2d_iso") => EditorViewMode::D2,
            Some("iso") => EditorViewMode::Iso,
            Some("orbit") => EditorViewMode::Orbit,
            Some("firstp") | Some("3d") => EditorViewMode::FirstP,
            _ => return Err("view must be 2d, 2d_iso, iso, orbit or firstp".into()),
        };
        if c.scale
            .is_some_and(|s| !s.is_finite() || !(0.1..=1024.).contains(&s))
        {
            return Err("Camera scale must be 0.1..1024".into());
        }
        let camera = if c.view.as_deref() == Some("2d_iso")
            || (c.view.is_none()
                && self.project.regions[i].map.camera == rusterix::MapCamera::TwoDIso)
        {
            rusterix::MapCamera::TwoDIso
        } else {
            rusterix::MapCamera::TwoD
        };
        let view_index = view.toolbar_index(camera);
        self.scepter_activate_region(i, ui, ctx);
        // Switch synchronously before setting the requested anchor; the normal
        // lifecycle restores saved cameras, which must not overwrite this focus.
        TOOLLIST.write().unwrap().handle_event(
            &TheEvent::IndexChanged(TheId::named("Editor View Switch"), view_index as usize),
            ui,
            ctx,
            &mut self.project,
            &mut self.server_ctx,
        );
        let region = &mut self.project.regions[i];
        region.editing_look_at_3d = center;
        region.editing_position_3d = if view == EditorViewMode::FirstP {
            pos.unwrap_or(center - Vec3::new(0., 0., 4.))
        } else {
            center
        };
        if view == EditorViewMode::D2 {
            if let Some(scale) = c.scale {
                region.map.grid_size = scale;
            }
            self.server_ctx.center_map_at_grid_pos(
                Vec2::zero(),
                Vec2::new(center.x, center.z),
                &mut region.map,
            );
        } else {
            use rusterix::D3Camera;
            let mut cameras = EDITCAMERA.write().unwrap();
            if let Some(scale) = c.scale {
                cameras.iso_camera.set_parameter_f32("scale", scale);
                cameras.orbit_camera.set_parameter_f32("distance", scale);
            }
            cameras.move_action = None;
        }
        ui.set_widget_value("Editor View Switch", ctx, TheValue::Int(view_index));
        if view == EditorViewMode::D2 {
            self.server_ctx.store_edit_view_2d_for_map(
                region.map.id,
                region.map.offset,
                region.map.grid_size,
            );
        } else {
            self.server_ctx.store_edit_view_for_map(
                region.map.id,
                view,
                region.editing_position_3d,
                region.editing_look_at_3d,
            );
            match view {
                EditorViewMode::Iso => {
                    region.editing_position_iso_3d = Some(region.editing_position_3d);
                    region.editing_look_at_iso_3d = Some(region.editing_look_at_3d);
                    let scale = EDITCAMERA.read().unwrap().iso_camera.scale;
                    region.editing_iso_scale = Some(scale);
                    self.server_ctx
                        .store_edit_view_iso_scale_for_map(region.map.id, scale);
                }
                EditorViewMode::Orbit => {
                    region.editing_position_orbit_3d = Some(region.editing_position_3d);
                    region.editing_look_at_orbit_3d = Some(region.editing_look_at_3d);
                    region.editing_orbit_distance =
                        Some(EDITCAMERA.read().unwrap().orbit_camera.distance);
                }
                EditorViewMode::FirstP => {
                    region.editing_position_firstp_3d = Some(region.editing_position_3d);
                    region.editing_look_at_firstp_3d = Some(region.editing_look_at_3d);
                }
                _ => {}
            }
        }
        update_region(ctx);
        Ok(self.scepter_context(ui))
    }
    pub(super) fn scepter_preview_request(
        &self,
        mut c: RegionRenderPreview,
        ui: &mut TheUI,
    ) -> Result<RegionRenderPreview, String> {
        if !c.editor_view {
            return Ok(c);
        }
        use rusterix::D3Camera;
        let i = region_index(&self.project, &c.region)?;
        if self.project.regions[i].id != self.server_ctx.curr_region
            || self.server_ctx.get_map_context() != MapContext::Region
        {
            return Err("editor_view requires the current map region".into());
        }
        let r = &self.project.regions[i];
        let cameras = EDITCAMERA.read().unwrap();
        let d = ui
            .get_widget("PolyView")
            .map(|w| w.dim().clone())
            .unwrap_or(TheDim::sized(768, 512));
        let factor = (d.width.max(1) as f32 / 1024.)
            .max(d.height.max(1) as f32 / 1024.)
            .max(1.);
        let size = c.size.unwrap_or([
            (d.width as f32 / factor).round().clamp(64., 1024.) as u32,
            (d.height as f32 / factor).round().clamp(64., 1024.) as u32,
        ]);
        c.size = Some(size);
        let mut camera = eldiron_scepter::PreviewCamera {
            center: r.editing_position_3d.into_array(),
            position: None,
            scale: None,
            azimuth: None,
            elevation: None,
            fov: None,
            pitch: None,
        };
        match self.server_ctx.editor_view_mode {
            EditorViewMode::D2 => {
                c.view = Some("2d".into());
                let grid = r.map.grid_size.max(0.1);
                camera.center = [-r.map.offset.x / grid, 0., r.map.offset.y / grid];
                c.zoom = Some(grid * size[1] as f32 / d.height.max(1) as f32);
                let w = size[0] as f32 / c.zoom.unwrap();
                let h = size[1] as f32 / c.zoom.unwrap();
                c.bounds = Some([
                    (camera.center[0] - w / 2.).floor() as i32,
                    (camera.center[2] - h / 2.).floor() as i32,
                    w.ceil().max(1.) as i32,
                    h.ceil().max(1.) as i32,
                ]);
            }
            EditorViewMode::Iso => {
                c.view = Some("iso".into());
                camera.scale = Some(cameras.iso_camera.scale);
                camera.azimuth = Some(cameras.iso_camera.azimuth_deg);
                camera.elevation = Some(cameras.iso_camera.elevation_deg);
            }
            EditorViewMode::Orbit => {
                c.view = Some("3d".into());
                let mut orbit = cameras.orbit_camera.clone();
                orbit.set_parameter_vec3("center", r.editing_position_3d);
                camera.position = Some(orbit.position().into_array());
                camera.fov = Some(orbit.fov);
            }
            EditorViewMode::FirstP => {
                c.view = Some("3d".into());
                camera.center = (r.editing_look_at_3d + Vec3::new(0., 1.5, 0.)).into_array();
                camera.position =
                    Some((r.editing_position_3d + Vec3::new(0., 1.5, 0.)).into_array());
                camera.fov = Some(cameras.firstp_camera.fov);
                camera.pitch = Some(cameras.firstp_camera.pitch);
            }
        }
        c.camera = Some(camera);
        Ok(c)
    }
    pub(super) fn scepter_entity_templates(&self) -> Json {
        json!({"characters":self.project.characters.values().map(|c|json!({"id":c.id,"name":c.name,"data":c.data,"authoring":c.authoring})).collect::<Vec<_>>(),
            "items":self.project.items.values().map(|c|json!({"id":c.id,"name":c.name,"data":c.data,"authoring":c.authoring})).collect::<Vec<_>>()})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scepter_feed_reports_selection_and_content_and_detects_expired_cursors() {
        let mut editor = Editor::new();
        editor.project = Project::default();
        editor.server_ctx.curr_region = editor.project.regions[0].id;
        editor.server_ctx.pc = ProjectContext::Region(editor.server_ctx.curr_region);
        let mut ui = TheUI::new();
        let start = editor.scepter_context(&mut ui);
        let cursor = start["context"]["cursor_sequence"].as_u64().unwrap();
        let stream = start["context"]["stream"].as_str().unwrap().to_string();
        editor.project.regions[0].map.selected_vertices = vec![42];
        let changes = editor.scepter_changes(
            ChangeQuery {
                after: cursor,
                stream: Some(stream.clone()),
                limit: None,
            },
            &mut ui,
        );
        assert!(
            changes["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["kind"] == "selection")
        );
        assert_eq!(changes["reset_required"], false);
        let cursor = changes["next"].as_u64().unwrap();
        editor.project.regions[0].map.changed += 1;
        let changes = editor.scepter_changes(
            ChangeQuery {
                after: cursor,
                stream: Some(stream.clone()),
                limit: None,
            },
            &mut ui,
        );
        assert!(
            changes["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["kind"] == "content")
        );
        for n in 0..270 {
            editor.project.regions[0].map.selected_vertices = vec![n];
            editor.scepter_observe(true, &mut ui);
        }
        let changes = editor.scepter_changes(
            ChangeQuery {
                after: 1,
                stream: Some(stream),
                limit: None,
            },
            &mut ui,
        );
        assert_eq!(changes["reset_required"], true);
        let changes = editor.scepter_changes(
            ChangeQuery {
                after: 0,
                stream: Some(Uuid::new_v4().to_string()),
                limit: None,
            },
            &mut ui,
        );
        assert_eq!(changes["reset_required"], true);
    }
}

#[cfg(test)]
mod focus_tests {
    use super::*;
    #[test]
    fn scepter_focus_survives_normal_view_switching_and_preview_uses_fractional_zoom() {
        let mut editor = Editor::new();
        editor.project = Project::default();
        editor.server_ctx.curr_region = editor.project.regions[0].id;
        editor.server_ctx.pc = ProjectContext::Region(editor.server_ctx.curr_region);
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(768, 512, 1.);
        let revision = editor.scepter_revision();
        for view in ["iso", "2d", "iso", "2d"] {
            let focus = EditorFocus {
                region: None,
                view: Some(view.into()),
                center: [20., 0., -12.],
                position: None,
                scale: Some(if view == "2d" { 0.45 } else { 12. }),
            };
            editor.scepter_focus(focus, &mut ui, &mut ctx).unwrap();
            let request: RegionRenderPreview = serde_json::from_value(
                json!({"region":{"id":editor.server_ctx.curr_region},"editor_view":true}),
            )
            .unwrap();
            let preview = editor.scepter_preview_request(request, &mut ui).unwrap();
            for (actual, expected) in preview
                .camera
                .as_ref()
                .unwrap()
                .center
                .into_iter()
                .zip([20., 0., -12.])
            {
                assert!((actual - expected).abs() < 0.0001);
            }
            if view == "2d" {
                assert!((preview.zoom.unwrap() - 0.45).abs() < 0.0001);
            } else {
                assert_eq!(preview.camera.unwrap().scale, Some(12.));
            }
        }
        assert_eq!(
            revision,
            editor.scepter_revision(),
            "Camera changes are view state"
        );
    }
}
