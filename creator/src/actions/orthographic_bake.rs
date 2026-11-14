use crate::prelude::*;

const SAMPLE_COUNT_ID: &str = "actionBakeSamples";

fn bake_applicable(server_ctx: &ServerContext) -> bool {
    server_ctx.get_map_context() == MapContext::Region
        && !server_ctx.pc.is_prefab()
        && server_ctx.editor_view_mode == EditorViewMode::Iso
}

pub struct RenderOrthographicBake {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for RenderOrthographicBake {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::IntEditSlider(
            SAMPLE_COUNT_ID.into(),
            "".into(),
            "".into(),
            32,
            1..=256,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_bake_render_desc"),
        ));
        Self {
            id: TheId::named(&fl!("action_bake_render")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_bake_render_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn map_scope(&self) -> ActionMapScope {
        ActionMapScope::RegionWorld
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        bake_applicable(server_ctx)
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let samples = self
            .nodeui
            .get_i32_value(SAMPLE_COUNT_ID)
            .unwrap_or(32)
            .clamp(1, 256) as u32;
        let progress = {
            let mut bake = server_ctx.bake.write();
            bake.request_render(map.id, samples);
            bake.progress_text()
        };
        // Preserve the CPU reference adapter's build invalidation without
        // constructing a renderer for data-only action consumers.
        if let Some(mut reference) = server_ctx.reference.existing_mut() {
            reference.set_dirty();
        }
        server_ctx.background_progress = progress;
        ctx.ui.redraw_all = true;
        ctx.ui.send(TheEvent::SetStatusText(
            TheId::empty(),
            fl!("status_bake_requested"),
        ));
        None
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

pub struct ToggleOrthographicBakeVisibility {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ToggleOrthographicBakeVisibility {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_bake_toggle_visibility_desc"),
        ));
        Self {
            id: TheId::named(&fl!("action_bake_toggle_visibility")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_bake_toggle_visibility_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn map_scope(&self) -> ActionMapScope {
        ActionMapScope::RegionWorld
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        bake_applicable(server_ctx)
    }

    fn apply(
        &self,
        _map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let status = match _server_ctx.bake.write().toggle_visibility() {
            Some(true) => fl!("status_bake_visible"),
            Some(false) => fl!("status_bake_hidden"),
            None => fl!("status_bake_missing"),
        };
        ctx.ui.redraw_all = true;
        ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
        None
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

pub struct ClearOrthographicBake {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ClearOrthographicBake {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_bake_clear_desc"),
        ));
        Self {
            id: TheId::named(&fl!("action_bake_clear")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_bake_clear_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn map_scope(&self) -> ActionMapScope {
        ActionMapScope::RegionWorld
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        bake_applicable(server_ctx)
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let before = map.clone();
        // This action edits the map synchronously. No delayed persisted clear
        // may erase a subsequent undo or a different document's bake.
        *server_ctx.bake.write() = rusterix::OrthographicBakeController::default();
        map.orthographic_bake = None;
        map.changed = map.changed.wrapping_add(1);
        server_ctx.background_progress = None;
        ctx.ui.redraw_all = true;
        ctx.ui.send(TheEvent::SetStatusText(
            TheId::empty(),
            fl!("status_bake_cleared"),
        ));
        Some(ProjectUndoAtom::MapEdit(
            self.map_scope().undo_context(server_ctx),
            Box::new(before),
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
pub(crate) mod tests {
    use super::*;

    #[test]
    fn bake_actions_are_available_only_in_the_orthographic_region_view() {
        let mut server_ctx = ServerContext::default();
        server_ctx.editor_view_mode = EditorViewMode::Iso;
        assert!(bake_applicable(&server_ctx));

        server_ctx.editor_view_mode = EditorViewMode::Orbit;
        assert!(!bake_applicable(&server_ctx));
    }

    #[test]
    fn bake_actions_own_transient_state_without_creating_a_reference_renderer() {
        let mut first = ServerContext::default();
        let second = ServerContext::default();
        let mut map = Map::new();
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(64, 64, 1.);
        let authored = serde_json::to_vec(&map).unwrap();
        assert!(!first.bake.initialized());
        assert!(!second.bake.initialized());
        assert!(!first.reference.initialized());
        RenderOrthographicBake::new().apply(&mut map, &mut ui, &mut ctx, &mut first);
        assert_eq!(
            *first.bake.read().status(),
            rusterix::OrthographicBakeStatus::Requested
        );
        assert_eq!(
            *second.bake.read().status(),
            rusterix::OrthographicBakeStatus::Missing
        );
        assert_eq!(serde_json::to_vec(&map).unwrap(), authored);
        let camera = scenevm::Camera3D::iso();
        {
            let mut bake = first.bake.write();
            let work = bake
                .prepare_work(map.id, 1, 1, camera, Some((0., 1., 0., 1.)))
                .unwrap()
                .unwrap();
            assert_eq!(
                *bake.status(),
                rusterix::OrthographicBakeStatus::Rendering {
                    sample: 0,
                    total: 32
                }
            );
            bake.preview_current_tile(
                &[32, 40, 48, 255].repeat((work.tile_size * work.tile_size) as usize),
            )
            .unwrap();
            assert!(bake.presentation_source(map.id, &camera).is_some());
        }
        ToggleOrthographicBakeVisibility::new().apply(&mut map, &mut ui, &mut ctx, &mut first);
        assert!(
            first
                .bake
                .read()
                .presentation_source(map.id, &camera)
                .is_none()
        );
        ToggleOrthographicBakeVisibility::new().apply(&mut map, &mut ui, &mut ctx, &mut first);
        assert!(
            first
                .bake
                .read()
                .presentation_source(map.id, &camera)
                .is_some()
        );
        let clear = ClearOrthographicBake::new()
            .apply(&mut map, &mut ui, &mut ctx, &mut first)
            .unwrap();
        assert!(matches!(clear, ProjectUndoAtom::MapEdit(_, _, _)));
        assert_eq!(
            *first.bake.read().status(),
            rusterix::OrthographicBakeStatus::Missing
        );
        assert!(first.bake.write().take_persisted_update().is_none());
        assert_eq!(map.changed, 1);
        RenderOrthographicBake::new().apply(&mut map, &mut ui, &mut ctx, &mut first);
        first.reset_bake_for_project_switch();
        assert_eq!(
            *first.bake.read().status(),
            rusterix::OrthographicBakeStatus::Missing
        );
        assert!(
            first.bake.write().take_persisted_update().is_none(),
            "switch cannot clear the next document"
        );
        assert!(!first.reference.initialized());
        assert!(!second.reference.initialized());
    }
    #[cfg(feature = "native-render")]
    pub(crate) fn check_parent_bake_commands_and_history(
        gpu: std::sync::Arc<renderkit::GpuContext>,
    ) {
        let mut project = Project::new();
        let region_id = project.regions[0].id;
        let parent_id = project.regions[0].map.id;
        let child = Map::new();
        let child_id = child.id;
        let mut surface = rusterix::Surface::new(0);
        surface.profile = Some(child_id);
        project.regions[0].map.profiles.insert(child_id, child);
        project.regions[0]
            .map
            .surfaces
            .insert(surface.id, surface.clone());
        let mut server = ServerContext::default();
        server.reference.bind_context(gpu);
        server.curr_region = region_id;
        server.pc = ProjectContext::Region(region_id);
        server.editor_view_mode = EditorViewMode::Iso;
        server.geometry_edit_mode = GeometryEditMode::Detail;
        server.active_detail_surface = Some(surface);
        assert_eq!(project.get_map(&server).unwrap().id, child_id);
        let original_child =
            serde_json::to_vec(&project.regions[0].map.profiles[&child_id]).unwrap();
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(128, 128, 1.);
        let mut sidebar = crate::sidebar::Sidebar::new();
        let request = |command: &str| crate::actionscript::EditorActionRequest {
            command_id: command.into(),
            parameters_toml: String::new(),
        };
        sidebar
            .execute_action_command(
                &request("bake.render"),
                &mut ui,
                &mut ctx,
                &mut project,
                &mut server,
            )
            .unwrap();
        let mut bake = server.bake.write();
        assert!(
            bake.prepare_work(child_id, 1, 1, scenevm::Camera3D::iso(), None)
                .unwrap()
                .is_none()
        );
        let mut work = bake
            .prepare_work(
                parent_id,
                1,
                1,
                scenevm::Camera3D::iso(),
                Some((0., 1., 0., 1.)),
            )
            .unwrap()
            .unwrap();
        while !bake.finish_work_sample(&work).unwrap() {
            work = bake
                .prepare_work(parent_id, 1, 1, scenevm::Camera3D::iso(), None)
                .unwrap()
                .unwrap();
        }
        let n = (work.tile_size * work.tile_size) as usize;
        bake.commit_work(
            &work,
            [40, 50, 60, 255].repeat(n),
            vec![3.; n],
            [40, 50, 60, 255].repeat(n),
            [128, 255, 128, 255].repeat(n),
            [200, 32, 255, 0].repeat(n),
        )
        .unwrap();
        project.regions[0].map.orthographic_bake =
            bake.take_persisted_update_for(parent_id).unwrap();
        drop(bake);
        let authored_bake = project.regions[0].map.orthographic_bake.clone();
        sidebar
            .execute_action_command(
                &request("bake.clear"),
                &mut ui,
                &mut ctx,
                &mut project,
                &mut server,
            )
            .unwrap();
        assert!(project.regions[0].map.orthographic_bake.is_none());
        assert!(server.history.read().has_unsaved());
        server
            .history
            .write()
            .undo(&mut server, &mut project, &mut ui, &mut ctx);
        assert_eq!(project.regions[0].map.id, parent_id);
        assert_eq!(project.regions[0].map.orthographic_bake, authored_bake);
        assert_eq!(project.get_map(&server).unwrap().id, child_id);
        assert_eq!(
            serde_json::to_vec(&project.regions[0].map.profiles[&child_id]).unwrap(),
            original_child
        );
        server
            .history
            .write()
            .redo(&mut server, &mut project, &mut ui, &mut ctx);
        assert!(project.regions[0].map.orthographic_bake.is_none());
        assert_eq!(
            serde_json::to_vec(&project.regions[0].map.profiles[&child_id]).unwrap(),
            original_child
        );
    }
}
