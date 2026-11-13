//! Narrow bridge while Creator's document/world/tool owners are extracted.
use super::*;
#[path = "native_viewport.rs"]
mod viewport;
#[path = "native_world.rs"]
mod world;
#[path = "native_play.rs"]
mod play;
#[path = "native_game_input.rs"]
mod game_input;
use play::NativePlayFrame;
use viewport::NativeDisplayFrame;
pub(crate) use viewport::NativeViewport;
use world::NativeWorldFrame;

pub(crate) struct NativeViewTokens {
    minimap: Option<crate::minimap::native_geometry::Token>,
    display: Option<NativeDisplayFrame>,
    play: Option<NativePlayFrame>,
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
            minimap: None,
            display: None,
            play: None,
            world: None,
            geometry,
            appearance,
            material,
        }
    }
    #[cfg(test)]
    pub(crate) fn play_for_test(&self) -> bool {
        self.play.is_some()
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
        self.minimap = self
            .minimap
            .take()
            .filter(|token| receipts.contains(&token.receipt()));
        self.play = self.play.take().filter(|frame| {
            receipts.contains(&frame.receipt)
                && frame.receipts.iter().all(|receipt| receipts.contains(receipt))
        });
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
    /// An unselected/deleted map has no image producer. Keep the native view
    /// empty instead of opting into a viewport-sized CPU bitmap every tick.
    pub(crate) fn native_empty_viewport(&self) -> bool {
        self.native_viewport.enabled
            && !self.server_ctx.game_mode
            && !self.native_display_owns_viewport()
            && self.documents.project.get_map(&self.server_ctx).is_none()
    }

    pub(crate) fn native_scene_tick_redraw(&self, running: bool, tick: bool) -> bool {
        running
            || self.authoring_is_interacting()
            || self.geometry_editor.wants_realtime()
            || (tick
                && self.native_world_owns_viewport()
                && (self.server_ctx.editor_view_mode == EditorViewMode::D2
                    || (self.server_ctx.pc.is_prefab()
                        && self.server_ctx.curr_map_tool_type == MapToolType::Effects)))
    }

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
                .update(&self.documents.project, ui, ctx, &self.server_ctx);
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
                &self.documents.project,
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
        let play = self.prepare_native_play_viewport(ui, ctx, images.registry)?;
        let minimap = self.sidebar.minimap_geometry.prepare(
            &self.documents.project,
            &self.server_ctx,
            ui,
            ctx,
        );
        if self.native_empty_viewport()
            && let Some(view) = crate::utils::map_editor_render_view(ui, &self.server_ctx)
        {
            view.set_image_handle(None);
        }
        let mut tokens = NativeViewTokens::previews(geometry, appearance, material);
        tokens.display = display;
        tokens.world = world;
        tokens.play = play;
        tokens.minimap = minimap;
        Ok(Some(tokens))
    }
    pub(crate) fn native_views_waiting(&self) -> bool {
        self.geometry_editor.gpu_preview_waiting()
            || self.appearance_workspace.gpu_preview_waiting()
            || self.procedural_material.gpu_preview_waiting()
            || self.native_world_waiting()
    }
    pub(crate) fn presented_native_views(
        &mut self,
        tokens: NativeViewTokens,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        self.presented_native_play_viewport(tokens.play, ui, ctx);
        self.sidebar.minimap_geometry.presented(tokens.minimap);
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
        self.sidebar.minimap_geometry.invalidate(ui);
        self.native_viewport.play.invalidate(ui);
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
        &self.documents.project
    }
    pub(crate) fn begin_native_profile_for_test(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> TheId {
        let region = &self.documents.project.regions[0];
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
        self.documents.project.get_map(&self.server_ctx).unwrap().subdivisions
    }
    pub(crate) fn native_profile_geometry_count_for_test(&self) -> usize {
        self.documents.project
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
    pub(crate) fn begin_native_prefab_for_test(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> (TheId, Uuid) {
        // Reuse the geometry actually authored earlier in this assembled run.
        let geometry = self.documents.project.regions[0].map.geometry_objects.clone();
        assert!(!geometry.is_empty());
        let mut min = Vec3::broadcast(f32::INFINITY);
        let mut max = Vec3::broadcast(f32::NEG_INFINITY);
        for object in &geometry {
            for vertex in &object.vertices {
                let point = object.transform_point(*vertex);
                min = min.map2(point, f32::min);
                max = max.map2(point, f32::max);
            }
        }
        let mut asset = rusterix::BlockPropAsset::new_authored("Native Prefab", geometry);
        let origin = Vec3::new((min.x + max.x) * 0.5, max.y + 0.5, (min.z + max.z) * 0.5);
        let attachment = rusterix::BlockPropAttachment {
            position: origin.into_array(),
            ..Default::default()
        };
        let attachment_id = attachment.id;
        let mut mount = rusterix::BlockPropPart::new_authored("FX mount", Vec::new());
        let angle = 0.6f32;
        mount.local_transform = [
            [angle.cos() * 1.2, 0., -angle.sin() * 1.2, 0.],
            [0., 0.8, 0., 0.],
            [angle.sin() * 1.4, 0., angle.cos() * 1.4, 0.],
            [0.25, 0.3, 0.15, 1.],
        ];
        let mut nozzle = rusterix::BlockPropPart::new_authored("FX nozzle", Vec::new());
        nozzle.parent_part_id = Some(mount.id);
        nozzle.local_transform[3][0] = 0.1;
        nozzle.attachments.push(attachment);
        asset
            .particle_effects
            .push(rusterix::BlockPropParticleEffect {
                id: Uuid::new_v4(),
                name: "Native particles".into(),
                part_id: nozzle.id,
                attachment_id,
                enabled: true,
                emitter: Default::default(),
            });
        asset.parts.push(mount);
        asset.parts.push(nozzle);
        let id = asset.id;
        self.server_ctx.curr_block_asset_id = Some(id);
        self.server_ctx.curr_block_asset_name = Some(asset.name.clone());
        self.documents.project.block_props.insert(id, asset);
        crate::block_props::sync_existing_prefab_presentations(&self.documents.project);
        let mut docks = DOCKMANAGER.write().unwrap();
        docks.set_dock(
            "Prefabs".into(),
            ui,
            ctx,
            &self.documents.project,
            &mut self.server_ctx,
        );
        docks.edit_maximize(ui, ctx, &mut self.documents.project, &mut self.server_ctx);
        let view = ui.get_render_view("PrefabView").unwrap().id().clone();
        ctx.ui.set_focus(&view);
        (view, id)
    }
    pub(crate) fn native_prefab_effect_for_test(&self, id: Uuid) -> ([f32; 3], String) {
        let asset = &self.documents.project.block_props[&id];
        let effect = &asset.particle_effects[0];
        let attachment = asset
            .find_part(effect.part_id)
            .unwrap()
            .attachments
            .iter()
            .find(|a| a.id == effect.attachment_id)
            .unwrap();
        (attachment.position, asset.name.clone())
    }
    pub(crate) fn rename_native_prefab_for_test(&mut self, id: Uuid, name: &str) {
        self.documents.project.block_props.get_mut(&id).unwrap().name = name.into();
        crate::block_props::sync_existing_prefab_presentations(&self.documents.project);
    }
    pub(crate) fn begin_native_play_for_test(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> usize {
        let mut project = Self::new_project(crate::project_templates::ProjectDimension::ThreeD);
        // An ordinary authored field verifies embedded native text/IME ownership.
        let settings = eldiron_runtime::PlayerSettings::parse(&project.config).unwrap();
        let screen = project.screens.values_mut().next().unwrap();
        let (left, top) = rusterix::utils::align_screen_to_grid(
            settings.viewport.0 as f32,
            settings.viewport.1 as f32,
            settings.grid_size,
        );
        let subdivisions = screen.map.subdivisions;
        screen.map.subdivisions = settings.grid_size * 2.;
        let id = crate::project_templates::rectangle(
            &mut screen.map,
            left + 16. / settings.grid_size,
            top + 16. / settings.grid_size,
            200. / settings.grid_size,
            32. / settings.grid_size,
        );
        let widget = screen.map.find_sector_mut(id).unwrap();
        widget.name = "Native name field".into();
        widget.properties.set(
            "data",
            Value::Str("[ui]\nrole='input'\nbind='start.name'\ndefault='Éowyn'".into()),
        );
        screen.map.subdivisions = subdivisions;
        let mut icons = false;
        let mut redraw = false;
        assert!(self.open_project_as_session(project, None, ui, ctx, &mut icons, &mut redraw));
        self.server_ctx.game_mode = false;
        self.server_ctx.game_input_mode = false;
        self.server_ctx.text_game_mode = false;
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Set Tool"),
            TheValue::Text("tool.geometry".into()),
        ));
        self.documents.active_session
    }
    pub(crate) fn native_text_output_for_test(&self, text: &str) -> bool {
        self.text_game.contains_output(text)
    }
    pub(crate) fn native_document_instance_for_test(&self) -> Uuid {
        self.server_ctx.document_instance
    }
    pub(crate) fn native_play_stats_for_test(&self) -> Option<(u64, Uuid, usize)> {
        let session = self.play_session.as_ref()?;
        Some((
            session.pump.tick_sequence(),
            session.pump.session_id(),
            self.pending_game_messages
                .iter()
                .filter(|m| m.3 == "Native Play delivery checkpoint")
                .count(),
        ))
    }
    pub(crate) fn queue_native_play_message_for_test(&mut self) {
        let id = self.documents.project.regions[0].map.id;
        let mut runtime = RUSTERIX.write().unwrap();
        let region = *runtime
            .server
            .region_id_map
            .get(&id)
            .expect("real started Creator region");
        runtime.server.messages.entry(region).or_default().push((
            None,
            None,
            0,
            "Native Play delivery checkpoint".into(),
            "status".into(),
        ));
    }
    pub(crate) fn native_play_saved_map_for_test(
        &self,
        index: usize,
    ) -> &Vec<shared::region::Region> {
        &self.documents.sessions[index].project.regions
    }
    pub(crate) fn native_world_vertices_for_test(&self) -> usize {
        self.documents.project
            .get_map(&self.server_ctx)
            .unwrap()
            .vertices
            .len()
    }
}
