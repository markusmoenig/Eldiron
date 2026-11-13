//! Explicit authoring camera gestures. Pointer motion is logical and scoped to
//! its accepted view/document; raw device deltas belong only to captured look.
use super::*;
use theframework::platform::{InputEvent, PointerButton, UiSession};
impl Editor {
    pub(super) fn handle_native_viewcube_input(
        &mut self,
        binding: &Binding,
        input: MapEvent,
        ctx: &mut TheContext,
    ) -> bool {
        let world = &mut self.native_viewport.world;
        if let Some(capture) = &mut world.cube_capture {
            if capture.binding == *binding {
                match input {
                    MapEvent::MapDragged(at) => {
                        let delta = at - capture.origin;
                        if delta.x as i64 * delta.x as i64 + delta.y as i64 * delta.y as i64 >= 4 {
                            capture.orbiting = true;
                        }
                        if capture.orbiting {
                            let camera = &mut self.server_ctx.camera;
                            #[cfg(target_os = "macos")]
                            camera.mouse_dragged_viewcube_macos_primary(&at);
                            #[cfg(not(target_os = "macos"))]
                            camera.mouse_dragged_orbit(&at);
                            ctx.ui.redraw_all = true;
                        }
                        return true;
                    }
                    MapEvent::MapUp(_) | MapEvent::MapEscape => {
                        let capture = world.cube_capture.take().unwrap();
                        self.server_ctx.camera.reset_mouse_tracking();
                        if matches!(input, MapEvent::MapUp(_)) && !capture.orbiting {
                            snap_cube(capture.face, ctx, &mut self.server_ctx.camera);
                        }
                        return true;
                    }
                    _ => {}
                }
            }
        }
        if let MapEvent::MapClicked(at) = input {
            if !self.server_ctx.game_input_mode
                && self.server_ctx.tools.read().viewcube_navigation_enabled()
            {
                let pick = world.cube.pick(
                    binding.cube_target(world.epoch),
                    &self.server_ctx.camera.orbit_camera,
                    at,
                );
                match pick {
                    CubePick::Unavailable => {
                        ctx.ui.redraw_all = true;
                        return true;
                    }
                    CubePick::Face(face) => {
                        #[cfg(target_os = "macos")]
                        {
                            self.server_ctx.camera.reset_mouse_tracking();
                            self.server_ctx.camera.mouse_dragged_orbit(&at);
                            world.cube_capture = Some(CubeCapture {
                                binding: binding.clone(),
                                origin: at,
                                face,
                                orbiting: false,
                            });
                        }
                        #[cfg(not(target_os = "macos"))]
                        snap_cube(face, ctx, &mut self.server_ctx.camera);
                        return true;
                    }
                    CubePick::Outside => {}
                }
            }
        }
        if binding.paste.is_some()
            && binding.mode != EditorViewMode::D2
            && !self.server_ctx.game_input_mode
            && self.server_ctx.tools.read().viewcube_navigation_enabled()
        {
            if let MapEvent::MapHover(at) = input {
                if !matches!(
                    world.cube.pick(
                        binding.cube_target(world.epoch),
                        &self.server_ctx.camera.orbit_camera,
                        at
                    ),
                    CubePick::Outside
                ) {
                    return true;
                }
            }
        }
        false
    }
    pub(crate) fn track_native_world_navigation(
        &mut self,
        event: &InputEvent,
        session: &mut UiSession,
    ) {
        match event {
            InputEvent::Focus(false)
            | InputEvent::PointerLeft
            | InputEvent::PointerButton {
                button: PointerButton::Secondary,
                pressed: false,
                ..
            } => {
                self.native_viewport.world.relative_navigation = None;
            }
            InputEvent::PointerButton {
                button: PointerButton::Secondary,
                pressed: true,
                position,
            } => {
                self.native_viewport.world.relative_navigation = None;
                if self.server_ctx.game_input_mode
                    || session.ui.dialog.is_some()
                    || session.ui.popover.is_some()
                    || session.ui.context_menu.is_some()
                    || self.native_viewport.window_focused == Some(false)
                {
                    return;
                }
                let Some(binding) = self
                    .native_world_binding(&mut session.ui, &session.context)
                    .filter(|binding| {
                        matches!(binding.mode, EditorViewMode::Iso | EditorViewMode::Orbit)
                    })
                else {
                    return;
                };
                let inside = session
                    .ui
                    .get_widget_at_coord(Vec2::new(position[0] as i32, position[1] as i32))
                    .is_some_and(|widget| widget.id().uuid == binding.view.uuid);
                let presented = self
                    .native_viewport
                    .world
                    .presented
                    .as_ref()
                    .is_some_and(|frame| frame.ready && frame.binding.capture_compatible(&binding));
                if inside && presented {
                    session.context.ui.set_focus(&binding.view);
                    self.native_viewport.world.relative_navigation = Some(Navigation {
                        binding,
                        pointer: *position,
                    });
                }
            }
            InputEvent::PointerButton { pressed: true, .. } => {
                self.native_viewport.world.relative_navigation = None;
            }
            InputEvent::PointerMoved { position } => {
                let Some(mut owner) = self.native_viewport.world.relative_navigation.take() else {
                    return;
                };
                if session.ui.dialog.is_some()
                    || session.ui.popover.is_some()
                    || session.ui.context_menu.is_some()
                    || self.native_viewport.window_focused == Some(false)
                {
                    return;
                }
                let Some(next) = self
                    .native_world_binding(&mut session.ui, &session.context)
                    .filter(|next| owner.binding.capture_compatible(next))
                else {
                    return;
                };
                let delta = Vec2::new(
                    position[0] - owner.pointer[0],
                    position[1] - owner.pointer[1],
                );
                owner.pointer = *position;
                if delta.x.is_finite() && delta.y.is_finite() {
                    if next.context.is_prefab() || next.mode == EditorViewMode::Orbit {
                        self.server_ctx.camera.mouse_delta_orbit(delta);
                    } else if let Some(region) = self
                        .documents
                        .project
                        .get_region_mut(&self.server_ctx.curr_region)
                    {
                        self.server_ctx.camera.pan_3d_by_delta_f32(
                            region,
                            &self.server_ctx,
                            delta,
                            Vec2::new(next.size.0 as i32, next.size.1 as i32),
                        );
                    }
                    session.context.ui.redraw_all = true;
                }
                self.native_viewport.world.relative_navigation = Some(owner);
            }
            _ => (),
        }
    }
    pub(crate) fn native_world_relative_motion(
        &mut self,
        delta: [f32; 2],
        session: &mut UiSession,
    ) -> bool {
        if session.ui.dialog.is_some()
            || session.ui.popover.is_some()
            || session.ui.context_menu.is_some()
            || self.native_viewport.window_focused == Some(false)
        {
            return false;
        }
        let focus = session.context.ui.focus.as_ref().map(|id| id.uuid);
        let focused = crate::utils::map_editor_render_view(&mut session.ui, &self.server_ctx)
            .is_some_and(|view| Some(view.id().uuid) == focus);
        let game_look = self.server_ctx.native_game_input
            && focused
            && self.wants_mouse_motion()
            && self
                .native_game_input_frame(&mut session.ui, &session.context)
                .is_some();
        let fly = self.native_world_owns_viewport()
            && self.server_ctx.editor_view_mode == EditorViewMode::FirstP
            && self.server_ctx.editor_fly_nav_active
            && self.server_ctx.editor_fly_nav_mouse_down
            && focused;
        // Iso/Orbit navigation already consumes logical pointer deltas above.
        // Also applying raw device movement would double it and reintroduce DPI
        // and platform-dependent movement over unrelated controls.
        if !(game_look || fly) {
            return false;
        }
        self.mouse_motion(delta[0], delta[1], &mut session.context)
    }
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn native_view_diagnostics(&self) -> serde_json::Value {
        let camera = &self.server_ctx.camera;
        let region = self
            .documents
            .project
            .get_region(&self.server_ctx.curr_region);
        serde_json::json!({
            "position": region.map(|r| r.editing_position_3d.into_array()),
            "center": region.map(|r| r.editing_look_at_3d.into_array()),
            "mode": match self.server_ctx.editor_view_mode { EditorViewMode::D2 => "2d", EditorViewMode::Iso => "iso", EditorViewMode::Orbit => "orbit", EditorViewMode::FirstP => "first-person" },
            "navigation": self.native_viewport.world.relative_navigation.is_some(), "moving": camera.move_action.is_some(), "fly": self.server_ctx.editor_fly_nav_active,
            "fly_mouse": self.server_ctx.editor_fly_nav_mouse_down,
            "world": self.native_world_owns_viewport(),
            "paste": {
                "active": self.server_ctx.paste_clipboard.is_some(),
                "authored_vertices": self.documents.project.get_map(&self.server_ctx).map(|map| map.vertices.len()),
                "projected_vertices": self.native_viewport.world.paste.as_ref().map(|preview| preview.map.vertices.len()),
                "ready": self.native_viewport.world.presented.as_ref().is_some_and(|frame| frame.binding.paste.is_some() && frame.ready),
                "target": self.server_ctx.hover_cursor_3d.map(|point| point.into_array()),
            },
            "speech": self.native_viewport.world.speech.diagnostics(),
        })
    }
}
