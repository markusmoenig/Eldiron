//! Narrow bridge while Creator's document/world/tool owners are extracted.
use super::*;
#[path = "native_viewport.rs"]
mod viewport;
#[path = "native_world.rs"]
mod world;
use viewport::NativeDisplayFrame;
pub(crate) use viewport::NativeViewport;
use world::NativeWorldFrame;

pub(crate) struct NativeViewTokens {
    display: Option<NativeDisplayFrame>,
    world: Option<std::sync::Arc<NativeWorldFrame>>,
    pub(crate) geometry: Option<crate::geometry_editor::PreviewToken>,
    pub(crate) appearance: Option<crate::appearance_workspace::PreviewToken>,
    pub(crate) material: Option<crate::procedural_material::PreviewToken>,
}
impl NativeViewTokens {
    pub(crate) fn previews(
        geometry: Option<crate::geometry_editor::PreviewToken>,
        appearance: Option<crate::appearance_workspace::PreviewToken>,
        material: Option<crate::procedural_material::PreviewToken>,
    ) -> Self {
        Self {
            display: None,
            world: None,
            geometry,
            appearance,
            material,
        }
    }
    #[cfg(test)]
    pub(crate) fn profile_for_test(&self) -> bool {
        self.display.is_some()
    }
    #[cfg(test)]
    pub(crate) fn world_for_test(&self) -> bool {
        self.world.is_some()
    }
    pub(crate) fn retain_painted(&mut self, list: &theframework::paint::PaintList) {
        let receipts = list.receipts().collect::<std::collections::HashSet<_>>();
        self.world = self.world.take().filter(|frame| {
            receipts.contains(&frame.receipt)
                && frame
                    .receipts
                    .iter()
                    .all(|receipt| receipts.contains(receipt))
        });
        self.display = self
            .display
            .take()
            .filter(|display| receipts.contains(&display.frame.receipt()));
        self.geometry = self
            .geometry
            .filter(|token| receipts.contains(&token.receipt()));
        self.appearance = self
            .appearance
            .filter(|token| receipts.contains(&token.receipt()));
        self.material = self
            .material
            .filter(|token| receipts.contains(&token.receipt()));
    }
}
impl Editor {
    pub(crate) fn enable_native_views(&mut self) {
        self.native_viewport.enabled = true;
        TOOLLIST.write().unwrap().enable_native_display();
        self.synchronize_native_document();
        self.geometry_editor.enable_gpu_preview();
        self.appearance_workspace.enable_gpu_preview();
        self.procedural_material.enable_gpu_preview();
    }
    pub(crate) fn prepare_native_views(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        images: &mut theframework::render_wgpu::UiFrameImages<'_>,
    ) -> Result<Option<NativeViewTokens>, String> {
        // Layout is current here; source/pose requests remain CPU preparation.
        if self.geometry_editor.wants_realtime() {
            self.geometry_editor
                .update(&self.project, ui, ctx, &self.server_ctx);
        }
        let geometry = match self
            .geometry_editor
            .prepare_gpu_preview(ui, ctx, images.registry)
        {
            Ok(token) => token,
            Err(error) => {
                self.geometry_editor.report_gpu_preview_error(error, ctx);
                self.geometry_editor.native_preview_fallback_token()
            }
        };
        // An already queued appearance description is refreshed only after the
        // current layout, so physical output and view bounds use one frame.
        if self.appearance_workspace.visible(ui) {
            self.appearance_workspace.update(
                &self.project,
                ui,
                ctx,
                self.procedural_appearance_revision,
            );
        }
        let appearance = self
            .appearance_workspace
            .prepare_gpu_preview(ui, ctx, images.registry);
        let material = self.procedural_material.prepare_gpu_preview(
            ui,
            ctx,
            images.registry,
            self.server_ctx.animation_counter,
        );
        let display = self.prepare_native_display_viewport(ui, ctx, images)?;
        let world = self.prepare_native_world_viewport(ui, ctx, images)?;
        let mut tokens = NativeViewTokens::previews(geometry, appearance, material);
        tokens.display = display;
        tokens.world = world;
        Ok(Some(tokens))
    }
    pub(crate) fn native_views_waiting(&self) -> bool {
        self.geometry_editor.gpu_preview_waiting()
            || self.appearance_workspace.gpu_preview_waiting()
            || self.procedural_material.gpu_preview_waiting()
    }
    pub(crate) fn presented_native_views(
        &mut self,
        tokens: NativeViewTokens,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        self.presented_native_display_viewport(tokens.display, ui, ctx);
        self.presented_native_world_viewport(tokens.world, ui, ctx);
        if let Some(token) = tokens.geometry {
            self.geometry_editor.presented_gpu_preview(token);
        }
        if let Some(token) = tokens.appearance {
            self.appearance_workspace
                .presented_gpu_preview(token, ui, ctx);
        }
        if let Some(token) = tokens.material {
            self.procedural_material.presented_gpu_preview(token);
        }
    }
    pub(crate) fn invalidate_native_views(&mut self, ui: &mut TheUI) {
        self.invalidate_native_display_viewport(ui);
        self.native_viewport.world.device_reset(ui);
        self.geometry_editor.invalidate_gpu_preview(ui);
        self.appearance_workspace.invalidate_gpu_preview(ui);
        self.procedural_material.invalidate_gpu_preview(ui);
    }
}
#[cfg(test)]
impl Editor {
    pub(crate) fn project_for_native_test(&self) -> &Project {
        &self.project
    }
    pub(crate) fn begin_native_profile_for_test(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> TheId {
        let region = &self.project.regions[0];
        self.server_ctx.pc = ProjectContext::Region(region.id);
        self.server_ctx.curr_region = region.id;
        self.server_ctx.editor_view_mode = EditorViewMode::Orbit;
        self.server_ctx.game_mode = false;
        self.server_ctx.game_input_mode = false;
        self.server_ctx.text_game_mode = false;
        self.synchronize_native_document();
        let (id, display) = {
            let actions = ACTIONLIST.read().unwrap();
            let action = actions
                .get_action_by_command_id("geometry.create_revolve")
                .unwrap();
            (action.id().uuid, action.editor_display())
        };
        self.server_ctx.curr_action_id = Some(id);
        ui.dialog = None;
        TOOLLIST
            .write()
            .unwrap()
            .begin_action_edit(id, self.server_ctx.pc, display, ctx);
        crate::utils::map_editor_render_view(ui, &self.server_ctx)
            .unwrap()
            .id()
            .clone()
    }
    pub(crate) fn native_profile_grid_for_test(&self) -> f32 {
        self.project.get_map(&self.server_ctx).unwrap().subdivisions
    }
    pub(crate) fn native_profile_geometry_count_for_test(&self) -> usize {
        self.project
            .get_map(&self.server_ctx)
            .unwrap()
            .geometry_objects
            .len()
    }
    pub(crate) fn has_native_profile_for_test(&self) -> bool {
        TOOLLIST
            .read()
            .unwrap()
            .active_editor_display_owner(&self.server_ctx)
            .is_some()
    }
    pub(crate) fn begin_native_world_for_test(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> TheId {
        // Use the real queued camera/tool commands after Return's queued
        // Object-tool selection, preserving normal application event ordering.
        ctx.ui.send(TheEvent::IndexChanged(
            TheId::named("Editor View Switch"),
            0,
        ));
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Set Tool"),
            TheValue::Text("tool.vertex".into()),
        ));
        let id = crate::utils::map_editor_render_view(ui, &self.server_ctx)
            .unwrap()
            .id()
            .clone();
        ctx.ui.set_focus(&id);
        ctx.ui.redraw_all = true;
        id
    }
    pub(crate) fn native_world_vertices_for_test(&self) -> usize {
        self.project
            .get_map(&self.server_ctx)
            .unwrap()
            .vertices
            .len()
    }
}
