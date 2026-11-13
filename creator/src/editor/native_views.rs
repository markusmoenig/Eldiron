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
    scene_minimap: Option<crate::minimap::native_scene::Token>,
    palette_minimap: Option<crate::minimap::native_palette::Token>,
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
            scene_minimap: None,
            palette_minimap: None,
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
        self.palette_minimap = self.palette_minimap.take().filter(|token| receipts.contains(&token.receipt()));
        self.scene_minimap = self
            .scene_minimap
            .take()
            .filter(|token| receipts.contains(&token.receipt()));
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
    pub(crate) fn bind_native_reference_context(
        &mut self,
        context: std::sync::Arc<renderkit::GpuContext>,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        self.scepter_preview.bind_context(context.clone());
        if self.server_ctx.reference.bind_context(context) {
            self.procedural_material.reset_model_gpu();
        }
    }
    pub(super) fn native_minimap_asset_key(&self, assets: &rusterix::Assets) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut key = std::collections::hash_map::DefaultHasher::new();
        assets.tiles_revision.hash(&mut key);
        assets.block_props_revision.hash(&mut key);
        assets.config.hash(&mut key);
        serde_json::to_vec(&assets.palette).expect("serializable palette").hash(&mut key);
        self.documents.project.procedural_materials.revision().hash(&mut key);
        self.documents.project.procedural_appearances.revision().hash(&mut key);
        self.procedural_appearance_revision.hash(&mut key);
        crate::minimap::native_scene::overview_map(&self.documents.project, &self.server_ctx)
            .and_then(|(map, _)| assets.terrain_paint.get(&map.id))
            .map(|paint| paint.key).hash(&mut key);
        key.finish()
    }
    pub(crate) fn advance_native_minimap_source(&mut self) -> bool {
        let bridge = self.server_ctx.reference.read();
        let assets = self.native_minimap_asset_key(&bridge.assets);
        self.sidebar.minimap_scene.advance_source(
            &self.documents.project, &self.server_ctx, &bridge.assets, assets,
        )
    }

    /// An unselected/deleted map has no image producer. Keep the native view
    /// empty instead of opting into a viewport-sized CPU bitmap every tick.
    pub(crate) fn native_empty_viewport(&self) -> bool {
        self.native_viewport.enabled
            && !self.server_ctx.game_mode
            && !self.native_display_owns_viewport()
            && self.native_world_map_id().is_none()
    }
    /// An asset/code dock replaces the world canvas. Its hidden scene widget
    /// cannot opt into legacy rendering merely because it has no active GPU
    /// world producer. Retire derived display storage while that canvas is hidden.
    pub(crate) fn native_hidden_viewport(&self, ui: &mut TheUI, name: &str) -> bool {
        if !self.native_viewport.enabled { return false; }
        let Some(id) = ui.get_render_view(name).map(|view|view.id().uuid) else { return true; };
        let mut visible = false;
        ui.canvas.visit_visible_widgets(&mut |widget| {
            if widget.id().uuid == id && widget.dim().is_valid() { visible = true; }
        });
        if !visible && let Some(view) = ui.get_render_view(name) {
            view.set_image_handle(None);
        }
        !visible
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
        self.server_ctx.tools.write().enable_native_display();
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
                self.procedural_appearance_revision, &self.server_ctx.reference);
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
        let palette_minimap = self.sidebar.minimap_palette.prepare(&self.documents.project, &self.server_ctx, ui, ctx);
        let scene_minimap = {
            // Borrow matching main-world CPU geometry, or the overview's own
            // completed parent source when editing an isolated map. GPU view,
            // camera and targets always belong to the overview.
            let bridge = self.server_ctx.reference.read();
            let asset_key = self.native_minimap_asset_key(&bridge.assets);
            let map = crate::minimap::native_scene::overview_map(&self.documents.project, &self.server_ctx)
                .map(|(map, _)| map);
            let ready = map.is_some_and(|map| {
                let builds = self.server_ctx.jobs.scene();
                !builds.has_pending_geometry() && builds.source_matches(map.id, map.camera)
            });
            self.sidebar.minimap_scene.prepare(
                &self.documents.project, &self.server_ctx, ui, ctx,
                &bridge.assets, asset_key, bridge.scene_handler.build_index.snapshot(),
                ready, bridge.client.animation_frame, images.registry,
            )?
        };
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
        tokens.scene_minimap = scene_minimap;
        tokens.palette_minimap = palette_minimap;
        Ok(Some(tokens))
    }
    pub(crate) fn native_views_waiting(&self) -> bool {
        self.sidebar.minimap_scene.waiting()
            || self.geometry_editor.gpu_preview_waiting()
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
        self.sidebar.minimap_scene.presented(tokens.scene_minimap);
        self.sidebar.minimap_palette.presented(tokens.palette_minimap);
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
        self.sidebar.minimap_scene.device_reset(ui);
        self.sidebar.minimap_palette.invalidate(ui);
        self.native_viewport.play.invalidate(ui);
        self.invalidate_native_display_viewport(ui);
        self.native_viewport.world.device_reset(ui, &mut self.server_ctx.camera, &self.server_ctx.tools);
        self.geometry_editor.invalidate_gpu_preview(ui);
        self.appearance_workspace.invalidate_gpu_preview(ui);
        self.procedural_material.invalidate_gpu_preview(ui);
    }
}
#[cfg(test)]
impl Editor {
    pub(crate) fn controllers_for_test(&self) -> &CreatorContext { &self.server_ctx }
    pub(crate) fn native_pixel_dock_for_test(&mut self, ui: &mut TheUI, ctx: &mut TheContext, open: bool) {
        if open {
            self.server_ctx.game_mode = false;
            self.server_ctx.game_input_mode = false;
            self.server_ctx.text_game_mode = false;
            let region = self.documents.project.regions.first().unwrap().id;
            self.server_ctx.pc = ProjectContext::Region(region);
            self.server_ctx.curr_region = region;
            self.server_ctx.editing_surface = None;
            self.server_ctx.profile_view = None;
            assert!(self.documents.project.get_map(&self.server_ctx).is_some());
            self.server_ctx.docks.write().set_editor_dock("Tiles".into(), ui, ctx, &self.documents.project, &mut self.server_ctx);
        } else {
            self.server_ctx.docks.write().minimize(ui, ctx, &self.documents.project, &mut self.server_ctx);
        }
    }
    pub(crate) fn native_camera_mode_for_test(&mut self, mode: EditorViewMode) -> EditorViewMode {
        std::mem::replace(&mut self.server_ctx.editor_view_mode, mode)
    }
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
            let actions = self.server_ctx.actions.read();
            let action = actions
                .get_action_by_command_id("geometry.create_revolve")
                .unwrap();
            (action.id().uuid, action.editor_display())
        };
        self.server_ctx.curr_action_id = Some(id);
        ui.dialog = None;
        self.server_ctx.tools.write()
            .begin_action_edit(id, self.server_ctx.pc, display, ctx, &self.server_ctx.actions);
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
        self.server_ctx.tools.read()
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
        crate::block_props::sync_existing_prefab_presentations(&self.documents.project, &self.server_ctx.jobs, &self.server_ctx.reference);
        let mut docks = self.server_ctx.docks.write();
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
        crate::block_props::sync_existing_prefab_presentations(&self.documents.project, &self.server_ctx.jobs, &self.server_ctx.reference);
    }
    pub(crate) fn begin_native_play_for_test(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> usize {
        let mut project = Self::new_project(crate::project_templates::ProjectDimension::ThreeD);
        let mut unseen = Region::new();
        unseen.name = "Unseen runtime region".into();
        unseen.map.name = unseen.name.clone();
        project.regions.push(unseen);
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
            session.pump().tick_sequence(),
            session.pump().session_id(),
            self.pending_game_messages
                .iter()
                .filter(|m| m.3 == "Native Play delivery checkpoint")
                .count(),
        ))
    }
    pub(crate) fn native_owned_runtime_for_test(&self) -> Option<(bool, bool, std::time::Duration)> {
        let play = self.play_session.as_ref()?;
        let bridge = self.server_ctx.reference.read();
        Some((play.owned(), bridge.server.state == rusterix::ServerState::Off && play.state(&bridge.server) == rusterix::ServerState::Running, play.clock_step()))
    }
    pub(crate) fn publish_native_runtime_sources_for_test(&mut self) {
        let mut bridge = self.server_ctx.reference.write();
        bridge.assets.item_models.insert("Owned catalog checkpoint".into(), "new CPU source".into());
        bridge.assets.avatars.insert("Owned catalog checkpoint".into(), rusterix::Avatar::default());
        bridge.assets.mark_visual_sources_changed();
    }
    pub(crate) fn native_runtime_sources_received_for_test(&self) -> bool {
        let bridge = self.server_ctx.reference.read();
        let play = self.play_session.as_ref().unwrap();
        let assets = play.assets(&bridge.assets);
        !std::ptr::eq(assets, &bridge.assets)
            && assets.item_models.get("Owned catalog checkpoint").map(String::as_str) == Some("new CPU source")
            && assets.avatars.revision() == bridge.assets.avatars.revision()
    }
    pub(crate) fn native_audio_commands_for_test(&self) -> u64 {
        self.play_session.as_ref().unwrap().audio_stats().unwrap().delivered_commands
    }
    pub(crate) fn queue_native_unseen_audio_for_test(&mut self) {
        let play = self.play_session.as_mut().unwrap();
        let mut bridge = self.server_ctx.reference.write();
        let bridge = &mut *bridge;
        let parts = play.parts(&mut bridge.server, &mut bridge.assets);
        let region = parts.server.region_id_map[&parts.project.regions[1].map.id];
        parts.server.audio_commands.entry(region).or_default().push(rusterix::server::message::AudioCommand::SetBusVolume {
            bus: "Owned audio checkpoint".into(), volume: 0.5,
        });
    }
    pub(crate) fn queue_native_play_message_for_test(&mut self) {
        let play = self.play_session.as_mut().unwrap();
        let mut bridge = self.server_ctx.reference.write();
        let bridge = &mut *bridge;
        let parts = play.parts(&mut bridge.server, &mut bridge.assets);
        let id = parts.project.regions[0].map.id;
        let region = parts.server.region_id_map[&id];
        parts.server.messages.entry(region).or_default().push((None, None, 0, "Native Play delivery checkpoint".into(), "status".into()));
    }
    pub(crate) fn native_console_text_for_test(&self) -> String { self.sidebar.console_text_for_test() }
    pub(crate) fn native_console_runtime_link_for_test(&self) -> Option<TheValue> { self.sidebar.console_runtime_link_for_test() }
    pub(crate) fn tag_native_console_actor_for_test(&mut self) {
        let play = self.play_session.as_mut().unwrap();
        let mut bridge = self.server_ctx.reference.write();
        let bridge = &mut *bridge;
        let parts = play.parts(&mut bridge.server, &mut bridge.assets);
        let map = &mut parts.project.regions[0].map;
        let id = parts.server.region_id_map[&map.id];
        parts.server.entities.get_mut(&id).unwrap().first_mut().unwrap().attributes.set("name", Value::Str("Console session actor".into()));
        parts.server.apply_entities_items(map);
    }
    pub(crate) fn native_play_saved_map_for_test(
        &self,
        index: usize,
    ) -> &Vec<shared::region::Region> {
        &self.documents.sessions[index].project.regions
    }
    pub(crate) fn native_play_authoring_checkpoint_for_test(&mut self) -> (serde_json::Value, u64) {
        self.sync_active_session_from_editor();
        (
            serde_json::to_value(&self.documents.sessions[self.documents.active_session].project)
                .unwrap(),
            self.server_ctx.history.read().revision(),
        )
    }
    pub(crate) fn inject_native_play_projection_for_test(&mut self) {
        let play = self.play_session.as_mut().unwrap();
        let mut bridge = self.server_ctx.reference.write();
        let bridge = &mut *bridge;
        let parts = play.parts(&mut bridge.server, &mut bridge.assets);
        let mut map = parts.project.regions[0].map.clone();
        let region = parts.server.region_id_map[&map.id];
        map.properties.set("native-runtime-only", Value::Bool(true)); map.add_vertex_at(1000., 1000.);
        parts.server.runtime_maps.insert(region, map);
    }
    pub(crate) fn native_play_projection_received_for_test(&self) -> bool {
        self.play_session.as_ref().is_some_and(|play| {
            let runtime = &play.project().regions[0].map;
            let authored = &self.documents.project.regions[0].map;
            let view = play.authoring_map(authored);
            runtime
                .properties
                .get_bool_default("native-runtime-only", false)
                && runtime.vertices.len() == authored.vertices.len() + 1
                && view.vertices.len() == authored.vertices.len()
                && !view
                    .properties
                    .get_bool_default("native-runtime-only", false)
        })
    }
    pub(crate) fn edit_native_play_authoring_for_test(&mut self, ctx: &mut TheContext) {
        let region = &mut self.documents.project.regions[0];
        let before = region.map.clone();
        region
            .map
            .properties
            .set("native-authored-during-play", Value::Bool(true));
        self.server_ctx.history.write().add_undo(
            ProjectUndoAtom::MapEdit(
                ProjectContext::Region(region.id),
                Box::new(before),
                Box::new(region.map.clone()),
            ),
            ctx,
        );
    }
    pub(crate) fn native_world_vertices_for_test(&self) -> usize {
        self.documents.project
            .get_map(&self.server_ctx)
            .unwrap()
            .vertices
            .len()
    }
}

impl NativeViewport {
    pub(crate) fn receive_world_speech(
        &mut self,
        world: &eldiron_runtime::RuntimeView<'_>,
        events: &eldiron_runtime::RegionEvents,
    ) {
        if self.enabled {
            self.world.receive_speech(world, events);
        }
    }
    pub(crate) fn clear_world_speech(&mut self) {
        self.world.clear_speech();
    }
}
