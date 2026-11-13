//! The displayed world and the selected editor sub-map have different scopes.
//! A 3D profile selection edits a child while its world still shows the region.
use super::*;

pub(super) fn map<'a>(project: &'a Project, server: &ServerContext) -> Option<&'a Map> {
    if server.editor_view_mode != EditorViewMode::D2 && server.pc.is_region() {
        project
            .get_region(&server.curr_region)
            .map(|region| &region.map)
    } else {
        project.get_map(server)
    }
}

pub(super) fn map_mut<'a>(project: &'a mut Project, server: &ServerContext) -> Option<&'a mut Map> {
    if server.editor_view_mode != EditorViewMode::D2 && server.pc.is_region() {
        project
            .get_region_mut(&server.curr_region)
            .map(|region| &mut region.map)
    } else {
        project.get_map_mut(server)
    }
}

impl Editor {
    pub(crate) fn native_world_map_id(&self) -> Option<Uuid> {
        map(&self.documents.project, &self.server_ctx).map(|map| map.id)
    }
}

/// Freeze explicit detail/editing planes with the viewport capture. The cached
/// footprint is intentionally absent from Surface serialization, so hash it too.
#[derive(Clone, PartialEq)]
pub(super) struct DetailScope {
    mode: GeometryEditMode,
    active: Option<u64>,
    editing: Option<u64>,
}
impl DetailScope {
    pub(super) fn current(server: &ServerContext) -> Self {
        use std::hash::{Hash, Hasher};
        let stamp = |surface: &rusterix::Surface| {
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            serde_json::to_vec(surface)
                .expect("serializable surface")
                .hash(&mut hash);
            for point in &surface.world_vertices {
                [point.x.to_bits(), point.y.to_bits(), point.z.to_bits()].hash(&mut hash);
            }
            hash.finish()
        };
        Self {
            mode: server.geometry_edit_mode,
            active: server.active_detail_surface.as_ref().map(stamp),
            editing: server.editing_surface.as_ref().map(stamp),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_keeps_parent_geometry_while_detail_and_2d_editors_keep_their_child() {
        let mut project = Project::new();
        let mut region = Region::new();
        let mut profile = Map::new();
        profile.add_vertex_at(200., 300.);
        let profile_id = profile.id;
        let mut surface = rusterix::Surface::new(0);
        surface.profile = Some(profile_id);
        let parent_id = region.map.id;
        region.map.add_vertex_at(1., 2.);
        region.map.profiles.insert(profile_id, profile);
        region.map.surfaces.insert(surface.id, surface.clone());
        let region_id = region.id;
        project.regions = vec![region];
        let mut server = ServerContext::default();
        server.curr_region = region_id;
        server.pc = ProjectContext::Region(region_id);
        server.active_detail_surface = Some(surface.clone());
        server.geometry_edit_mode = GeometryEditMode::Detail;
        for mode in [
            EditorViewMode::Orbit,
            EditorViewMode::Iso,
            EditorViewMode::FirstP,
        ] {
            server.editor_view_mode = mode;
            assert_eq!(project.get_map(&server).unwrap().id, profile_id);
            assert_eq!(
                map(&project, &server).unwrap().id,
                parent_id,
                "3D world geometry cannot be replaced by its selected flat profile"
            );
            map_mut(&mut project, &server).unwrap().offset = Vec2::new(7., 9.);
            assert_eq!(project.get_map(&server).unwrap().offset, Vec2::zero());
        }
        let child = project.regions[0].map.profiles.remove(&profile_id).unwrap();
        assert!(project.get_map(&server).is_none());
        assert_eq!(
            map(&project, &server).unwrap().id,
            parent_id,
            "a removed detail profile cannot erase the still-existing world"
        );
        project.regions[0].map.profiles.insert(profile_id, child);
        server.editor_view_mode = EditorViewMode::D2;
        server.editing_surface = Some(surface);
        assert_eq!(map(&project, &server).unwrap().id, profile_id);
        map_mut(&mut project, &server).unwrap().offset = Vec2::new(3., 5.);
        assert_eq!(project.regions[0].map.offset, Vec2::new(7., 9.));
        let prefab = Map::new();
        let prefab_map_id = prefab.id;
        project.prefab_editor_map = Some(prefab);
        server.pc = ProjectContext::Prefab(Uuid::new_v4());
        for mode in [EditorViewMode::D2, EditorViewMode::Orbit] {
            server.editor_view_mode = mode;
            assert_eq!(map(&project, &server).unwrap().id, prefab_map_id);
            assert_eq!(map_mut(&mut project, &server).unwrap().id, prefab_map_id);
        }
        server.pc = ProjectContext::Region(Uuid::new_v4());
        server.curr_region = Uuid::new_v4();
        assert!(map(&project, &server).is_none());
        assert!(map_mut(&mut project, &server).is_none());
    }
    #[test]
    fn detail_scope_revokes_changed_plane_and_unserialized_footprint() {
        let mut server = ServerContext::default();
        server.geometry_edit_mode = GeometryEditMode::Detail;
        let mut surface = rusterix::Surface::new(7);
        surface.profile = Some(Uuid::new_v4());
        server.active_detail_surface = Some(surface.clone());
        let initial = DetailScope::current(&server);
        server
            .active_detail_surface
            .as_mut()
            .unwrap()
            .edit_uv
            .origin
            .x = 3.;
        assert!(DetailScope::current(&server) != initial);
        server.active_detail_surface = Some(surface.clone());
        assert!(DetailScope::current(&server) == initial);
        let authored = serde_json::to_vec(&surface).unwrap();
        surface.world_vertices.push(Vec3::new(1., 2., 3.));
        assert_eq!(serde_json::to_vec(&surface).unwrap(), authored);
        server.active_detail_surface = Some(surface);
        assert!(DetailScope::current(&server) != initial);
        server.active_detail_surface = None;
        assert!(DetailScope::current(&server) != initial);
    }
}

#[cfg(test)]
pub(crate) struct DetailFixture {
    original: Map,
    staged: Vec<u8>,
    mode: GeometryEditMode,
    active: Option<rusterix::Surface>,
    editing: Option<rusterix::Surface>,
    parent: Uuid,
    profile: Uuid,
}

#[cfg(test)]
impl Editor {
    pub(crate) fn begin_native_surface_detail_for_test(&mut self) -> DetailFixture {
        assert!(self.server_ctx.editor_view_mode != EditorViewMode::D2);
        assert_eq!(self.server_ctx.curr_map_tool_type, MapToolType::Vertex);
        let mode = self.server_ctx.geometry_edit_mode;
        let active = self.server_ctx.active_detail_surface.clone();
        let editing = self.server_ctx.editing_surface.clone();
        let parent = map_mut(&mut self.documents.project, &self.server_ctx).unwrap();
        let original = parent.clone();
        parent.clear_selection();
        parent.clear_temp();
        let host = crate::project_templates::rectangle(parent, 3., 3., 2., 2.);
        let mut profile = Map::new();
        crate::project_templates::rectangle(&mut profile, -0.5, -0.5, 1., 1.);
        profile.clear_selection();
        let profile_id = profile.id;
        let mut surface = rusterix::Surface::new(host);
        surface.calculate_geometry(parent);
        surface.profile = Some(profile.id);
        parent.profiles.insert(profile.id, profile);
        parent.surfaces.insert(surface.id, surface.clone());
        parent.changed += 1;
        let parent_id = parent.id;
        let staged = serde_json::to_vec(parent).unwrap();
        self.server_ctx.geometry_edit_mode = GeometryEditMode::Detail;
        self.server_ctx.active_detail_surface = Some(surface.clone());
        self.server_ctx.editing_surface = Some(surface);
        self.server_ctx.hover = (None, None, None);
        self.server_ctx.geo_hit = None;
        self.server_ctx.hover_cursor_3d = None;
        DetailFixture {
            original,
            staged,
            mode,
            active,
            editing,
            parent: parent_id,
            profile: profile_id,
        }
    }

    pub(crate) fn native_surface_detail_frame_for_test(
        &self,
        fixture: &DetailFixture,
    ) -> Option<(TheId, Vec2<i32>, theframework_core::ImageHandle)> {
        let world = &self.native_viewport.world;
        let frame = world.presented.as_ref().filter(|frame| frame.ready)?;
        let parent = map(&self.documents.project, &self.server_ctx).unwrap();
        assert_eq!(frame.binding.map, fixture.parent);
        assert_eq!(frame.stamp.map, fixture.parent);
        assert_eq!(parent.id, fixture.parent);
        assert_eq!(
            self.documents.project.get_map(&self.server_ctx).unwrap().id,
            fixture.profile
        );
        assert_eq!(
            serde_json::to_vec(parent).unwrap(),
            fixture.staged,
            "preparing a detail view cannot change authored parent/profile content"
        );
        let published = world.source.as_ref().unwrap().snapshot();
        assert!(Arc::ptr_eq(&published, &frame.source));
        assert!(
            published.chunks().values().any(|chunk| chunk
                .polys3d_map
                .keys()
                .any(|id| matches!(id, scenevm::GeoId::GeometryObject(_)))),
            "the parent world's populated geometry must remain in the source"
        );
        let surface = self.server_ctx.active_detail_surface.as_ref().unwrap();
        let vertex = &parent.profiles[&fixture.profile].vertices[0];
        let point = surface.uv_to_world(Vec2::new(vertex.x, -vertex.y));
        let camera = self.native_world_camera().unwrap();
        let scene = world
            .gpu
            .as_ref()
            .unwrap()
            .scenes
            .scene(frame.output.view)
            .unwrap();
        let marker = &scene.overlay_3d.polys3d_map[&scenevm::GeoId::Vertex(vertex.id)][0];
        let center = marker.vertices.iter().fold(Vec3::zero(), |sum, vertex| {
            sum + Vec3::new(vertex[0], vertex[1], vertex[2])
        }) / marker.vertices.len() as f32;
        assert!(
            (center - point).magnitude() < 0.01,
            "profile handle must be projected onto its actual parent surface"
        );
        let camera = camera.as_scenevm_camera_for_surface(
            frame.binding.size.0 as f32,
            frame.binding.size.1 as f32,
        );
        let at = crate::tools::geometry::project_geometry_point_to_screen(
            &camera,
            point,
            frame.binding.size.0 as f32,
            frame.binding.size.1 as f32,
        )
        .unwrap();
        Some((
            frame.binding.view.clone(),
            at.map(|v| v.round() as i32),
            frame.output.image,
        ))
    }

    pub(crate) fn check_native_surface_detail_selection_for_test(&self, fixture: &DetailFixture) {
        let parent = map(&self.documents.project, &self.server_ctx).unwrap();
        let staged: Map = serde_json::from_slice(&fixture.staged).unwrap();
        assert_eq!(
            serde_json::to_value(&parent.vertices).unwrap(),
            serde_json::to_value(&staged.vertices).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&parent.linedefs).unwrap(),
            serde_json::to_value(&staged.linedefs).unwrap()
        );
        let profile = &parent.profiles[&fixture.profile];
        assert_eq!(
            profile.selected_vertices,
            vec![profile.vertices[0].id],
            "the actual click must select the child profile handle, not a parent vertex"
        );
        assert!(parent.selected_vertices.is_empty());
        assert_eq!(
            serde_json::to_value(&profile.vertices).unwrap(),
            serde_json::to_value(&staged.profiles[&fixture.profile].vertices).unwrap()
        );
    }

    pub(crate) fn remove_native_surface_detail_child_for_test(
        &mut self,
        fixture: &DetailFixture,
    ) -> Vec<u8> {
        let parent = map_mut(&mut self.documents.project, &self.server_ctx).unwrap();
        parent.profiles.remove(&fixture.profile).unwrap();
        parent.changed += 1;
        serde_json::to_vec(parent).unwrap()
    }

    pub(crate) fn native_missing_detail_child_ready_for_test(
        &self,
        fixture: &DetailFixture,
        expected: &[u8],
    ) -> bool {
        let parent = map(&self.documents.project, &self.server_ctx).unwrap();
        assert_eq!(serde_json::to_vec(parent).unwrap(), expected);
        assert!(self.documents.project.get_map(&self.server_ctx).is_none());
        assert_eq!(self.native_world_map_id(), Some(fixture.parent));
        self.native_viewport
            .world
            .presented
            .as_ref()
            .is_some_and(|frame| frame.ready && frame.binding.map == fixture.parent)
    }

    pub(crate) fn finish_native_surface_detail_for_test(
        &mut self,
        fixture: DetailFixture,
        ui: &mut TheUI,
    ) {
        self.documents
            .project
            .get_region_mut(&self.server_ctx.curr_region)
            .unwrap()
            .map = fixture.original;
        self.server_ctx.geometry_edit_mode = fixture.mode;
        self.server_ctx.active_detail_surface = fixture.active;
        self.server_ctx.editing_surface = fixture.editing;
        self.server_ctx.hover = (None, None, None);
        self.server_ctx.geo_hit = None;
        self.server_ctx.hover_cursor_3d = None;
        self.invalidate_native_world_viewport(ui);
    }
}
