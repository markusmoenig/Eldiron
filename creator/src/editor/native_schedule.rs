//! Application update deadlines and worker notifications. GPU view preparation
//! does not own service polling, simulation timing or idle scheduling.
use super::*;

impl Editor {
    pub(crate) fn bind_native_wake(&mut self, wake: theframework::platform::ApplicationWake) {
        self.palette_switch.bind_wake(wake.clone());
        self.procedural_material.bind_wake(wake.clone());
        #[cfg(all(
            feature = "self-update",
            any(target_os = "macos", target_os = "windows", target_os = "linux")
        ))]
        {
            self.self_update_tx = self.self_update_tx.clone().with_wake(wake.clone());
        }
        self.application_wake = Some(wake);
    }

    pub(crate) fn native_update_wake_after(&self, settling: bool) -> Option<std::time::Duration> {
        let play = self.play_session.is_some()
            || self.server_ctx.shared.game_mode
            || self.server_ctx.shared.game_input_mode;
        let camera = self.native_world_owns_viewport()
            && !self.server_ctx.shared.game_input_mode
            && self.server_ctx.shared.editor_view_mode != EditorViewMode::D2
            && !self.server_ctx.shared.pc.is_prefab()
            && (self.server_ctx.shared.editor_fly_nav_active
                || self.server_ctx.camera.move_action.is_some());
        let scene_work = (!self.documents.project.regions.is_empty() || play)
            && self.server_ctx.jobs.has_pending_scene();
        let frames = settling
            || play
            || camera
            || self.authoring_is_interacting()
            || self.native_views_waiting()
            || scene_work;
        let ticks = play
            || self.native_scene_tick_redraw(false, true)
            || self
                .server_ctx
                .docks
                .read()
                .current_dock_supports_minimap_animation();
        self.update_tracker
            .wake_after(frames, ticks)
            .into_iter()
            .chain(self.native_runtime_wake_after())
            .min()
    }
}
