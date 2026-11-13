//! Prefab viewport changes address its temporary source, never region anchors.
use super::*;

impl ToolList {
    pub(super) fn handle_prefab_view_switch(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut ServerContext,
    ) -> bool {
        let TheEvent::IndexChanged(id, index) = event else {
            return false;
        };
        if id.name != "Editor View Switch" || !server.pc.is_prefab() {
            return false;
        }
        let previous = server.editor_view_mode;
        let next = EditorViewMode::from_toolbar_index(*index as i32);
        let Some(map) = project.prefab_editor_map.as_mut() else {
            return true;
        };
        let projection_changed =
            next == EditorViewMode::D2 && (map.camera == MapCamera::TwoDIso) != (*index == 1);
        let center = if previous == EditorViewMode::D2 && projection_changed {
            Some(if map.camera == MapCamera::TwoDIso {
                rusterix::isometric::Projection {
                    origin: Vec2::new(map.offset.x, -map.offset.y),
                    unit: map.grid_size,
                }
                .ground(Vec2::zero())
            } else {
                Vec2::new(-map.offset.x, map.offset.y) / map.grid_size
            })
        } else {
            None
        };
        if previous == EditorViewMode::D2 {
            server.store_edit_view_2d_for_map(map.id, map.offset, map.grid_size);
        }
        server.editor_view_mode = next;
        // The established isolated Prefab renderer uses its orbit camera for
        // every 3D toolbar mode. Camera metadata still describes builder input.
        map.camera = if next == EditorViewMode::D2 {
            if *index == 1 {
                MapCamera::TwoDIso
            } else {
                MapCamera::TwoD
            }
        } else {
            MapCamera::ThreeDIso
        };
        if next == EditorViewMode::D2 {
            if let Some((offset, grid)) = server.load_edit_view_2d_for_map(map.id) {
                map.offset = offset;
                map.grid_size = grid;
            }
            if let Some(center) = center {
                server.center_map_at_grid_pos(Vec2::zero(), center, map);
            }
        }
        map.curr_rectangle = None;
        self.box_select_armed = false;
        server.editor_fly_nav_active = false;
        server.editor_fly_nav_space_down = false;
        server.editing_surface = None;
        // A region's pending camera anchor is not a Prefab camera command.
        // Leave it with that region instead of applying or discarding it here.
        {
            let camera = &mut server.camera;
            camera.move_action = None;
            camera.reset_mouse_tracking();
        }
        crate::utils::editor_scene_full_rebuild(project, server);
        self.sync_props_availability(ui, ctx, project, server);
        self.update_geometry_overlay_3d(project, server);
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Action List"),
            TheValue::Empty,
        ));
        ctx.ui.redraw_all = true;
        true
    }
}
