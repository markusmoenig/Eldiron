//! Main authoring-world viewport on the host device. CPU source/controller
//! bridges remain explicit while the remaining Creator owners are extracted.
use super::*;
use crate::editor_overlay::procedural_gpu::GpuIcons;
use crate::hud::{Hud, HudInputs, HudStyle};
use crate::tools::overlay_frame::{
    ScopedToolOverlayEvent, ToolOverlayInputState, ToolOverlayRoute,
};
use crate::tools::{ToolOverlayFrame, ToolOverlayInputs, ToolOverlayTarget};
use eldiron_presentation::{SceneViews, ViewId};
use rusterix::scene_build_index::SceneBuildSnapshot;
use std::sync::Arc;
use theframework::paint::{FrameReceipt, PaintList, PaintOp, Rect, Sampling};
use theframework::render_wgpu::{UiFrameImages, UiImageCache};

#[derive(Clone, PartialEq)]
struct AssetKey {
    tiles: u64,
    config: String,
    palette: ThePalette,
    material: u64,
    appearance: u64,
    geometry: u64,
}
#[derive(Clone, PartialEq)]
struct Binding {
    document: Uuid,
    view: TheId,
    tool: Uuid,
    context: ProjectContext,
    size: (usize, usize),
    map: Uuid,
    density: u32,
    assets: AssetKey,
    slots: Option<Vec<(String, Option<rusterix::PixelSource>)>>,
    style: HudStyle,
}
impl Binding {
    fn target(&self, epoch: u64) -> ToolOverlayTarget {
        ToolOverlayTarget {
            document: self.document,
            tool: self.tool,
            view: self.view.uuid,
            revision: epoch,
            context: self.context,
            size: self.size,
            visible: true,
        }
    }
}
#[derive(Clone, PartialEq)]
struct SourceStamp {
    map: Uuid,
    changed: u32,
    grid: u32,
    subdivisions: u32,
    offset: [u32; 2],
    camera: MapCamera,
}
impl SourceStamp {
    fn new(map: &Map) -> Self {
        Self {
            map: map.id,
            changed: map.changed,
            grid: map.grid_size.to_bits(),
            subdivisions: map.subdivisions.to_bits(),
            offset: [map.offset.x.to_bits(), map.offset.y.to_bits()],
            camera: map.camera,
        }
    }
}
#[derive(PartialEq)]
struct SceneInputKey {
    stamp: SourceStamp,
    size: (usize, usize),
    tool: MapToolType,
    hover: (Option<u32>, Option<u32>, Option<u32>),
    cursor: Option<Vec2<f32>>,
    vertices: Vec<u32>,
    linedefs: Vec<u32>,
    sectors: Vec<u32>,
    entity: Option<Uuid>,
    rectangle: Option<(Vec2<f32>, Vec2<f32>)>,
    line: (Option<Vec2<f32>>, Option<Vec2<f32>>),
    grid: bool,
    hour: u32,
    animation: usize,
    canvas: Option<[RGBA; 3]>,
    build: ValueContainer,
    runtime: rusterix::server::message::RuntimeRenderState,
}
struct WorldGpu {
    registry: u64,
    document: Option<Uuid>,
    scenes: SceneViews,
    view: ViewId,
    configured: Option<AssetKey>,
    scene_key: Option<SceneInputKey>,
    scene_source: Option<Arc<SceneBuildSnapshot>>,
    guides: rusterix::scenebuilder::d2preview::D2PreviewBuilder,
    guide_scene: rusterix::Scene,
    icons: GpuIcons,
    images: UiImageCache,
    last_root_images: PaintList,
}
impl WorldGpu {
    fn new(images: &renderkit::ImageRegistry) -> Result<Self, String> {
        let mut scenes = SceneViews::new(images, scenevm::SharedAtlas::new(1024, 1024));
        let view = ViewId::new();
        scenes.create(view, (0, 0)).map_err(|e| e.to_string())?;
        Ok(Self {
            registry: images.owner_id(),
            document: None,
            scenes,
            view,
            configured: None,
            scene_key: None,
            scene_source: None,
            guides: Default::default(),
            guide_scene: Default::default(),
            icons: GpuIcons::new(images.context().clone(), 16 * 1024 * 1024, 512)
                .map_err(|e| e.to_string())?,
            images: UiImageCache::default(),
            last_root_images: PaintList::default(),
        })
    }
    fn finish(
        &mut self,
        images: &mut renderkit::ImageRegistry,
        root: Option<&PaintList>,
    ) -> Result<(), String> {
        if images.owner_id() != self.registry {
            return Ok(());
        }
        let mut retained = PaintList::default();
        if root.is_none() {
            for image in self
                .last_root_images
                .image_handles()
                .filter(|id| images.get(*id).is_some())
            {
                retained.image(image, Rect::new(0., 0., 1., 1.), Sampling::Nearest);
            }
        }
        let list = root.unwrap_or(&retained);
        self.images
            .finish_frame(images, list)
            .map_err(|e| e.to_string())?;
        self.icons
            .finish_frame(images, list)
            .map_err(|e| e.to_string())?;
        if let Some(root) = root {
            // Retain only exact resource pins for cancellation, not another copy
            // of the application's entire command/glyph stream.
            let mut pins = PaintList::default();
            let mut seen = std::collections::HashSet::new();
            for image in root.image_handles().filter(|id| seen.insert(*id)) {
                pins.image(image, Rect::new(0., 0., 1., 1.), Sampling::Nearest);
            }
            self.last_root_images = pins;
        }
        Ok(())
    }
}
pub(super) struct NativeWorldFrame {
    binding: Binding,
    target: ToolOverlayTarget,
    stamp: SourceStamp,
    source: Arc<SceneBuildSnapshot>,
    ready: bool,
    overlay: Arc<ToolOverlayFrame>,
    pub(super) receipts: Vec<FrameReceipt>,
    pub(super) receipt: FrameReceipt,
}
pub(crate) struct NativeWorld {
    binding: Option<Binding>,
    epoch: u64,
    serial: u64,
    receipt_owner: u64,
    presented: Option<Arc<NativeWorldFrame>>,
    overlay_input: ToolOverlayInputState,
    world_capture: Option<Binding>,
    events: std::collections::VecDeque<ScopedToolOverlayEvent>,
    published: Option<(TheId, Arc<PaintList>)>,
    gpu: Option<WorldGpu>,
}
impl Default for NativeWorld {
    fn default() -> Self {
        Self {
            binding: None,
            epoch: 0,
            serial: 0,
            receipt_owner: theframework::paint::new_receipt_owner(),
            presented: None,
            overlay_input: Default::default(),
            world_capture: None,
            events: Default::default(),
            published: None,
            gpu: None,
        }
    }
}
impl NativeWorld {
    fn bind(&mut self, next: Option<Binding>, ui: &mut TheUI) {
        if self.binding == next {
            return;
        }
        self.invalidate(ui);
        self.binding = next;
        if self.binding.is_some() {
            TOOLLIST
                .write()
                .unwrap()
                .get_current_tool()
                .set_native_overlay_input(true);
        }
    }
    pub(super) fn invalidate(&mut self, ui: &mut TheUI) {
        if let Some((id, list)) = self.published.take()
            && let Some(view) = ui.get_render_view(&id.name)
            && view.id().uuid == id.uuid
        {
            view.clear_paint_list(&list);
        }
        if self.binding.take().is_some() {
            let mut tools = TOOLLIST.write().unwrap();
            for tool in &mut tools.game_tools {
                tool.set_native_overlay_input(false);
                tool.invalidate_overlay();
            }
        }
        self.epoch = self
            .epoch
            .checked_add(1)
            .expect("world binding epoch exhausted");
        self.presented = None;
        self.overlay_input.invalidate();
        self.world_capture = None;
        self.events.clear();
    }
    pub(super) fn device_reset(&mut self, ui: &mut TheUI) {
        self.invalidate(ui);
        // The replaced host registry owns old handles and teardown.
        self.gpu = None;
    }
}
fn slots(
    map: &Map,
    server: &ServerContext,
) -> Option<Vec<(String, Option<rusterix::PixelSource>)>> {
    crate::actions::named_geometry_hud_material_slots_for_selection(map)
        .or_else(|| crate::actions::prefab_hud_material_slots_for_selected_instances(map))
        .or_else(|| {
            let actions = ACTIONLIST.read().unwrap();
            actions
                .get_action_by_id(server.curr_action_id?)?
                .hud_material_slots(map, server)
        })
        .map(|slots| slots.into_iter().map(|s| (s.label, s.source)).collect())
}
impl Editor {
    pub(crate) fn invalidate_native_world_viewport(&mut self, ui: &mut TheUI) {
        self.native_viewport.world.invalidate(ui);
    }
    pub(crate) fn native_world_owns_viewport(&self) -> bool {
        if !self.native_viewport.enabled
            || self.native_display_owns_viewport()
            || self.server_ctx.game_mode
            || self.server_ctx.game_input_mode
            || self.server_ctx.text_game_mode
            || self.server_ctx.editor_view_mode != EditorViewMode::D2
            || !self.server_ctx.pc.is_region()
            || self.server_ctx.world_mode
            || self.server_ctx.profile_view.is_some()
            || self.server_ctx.editing_surface.is_some()
            || self.server_ctx.paste_clipboard.is_some()
        {
            return false;
        }
        let Some(map) = self.project.get_map(&self.server_ctx) else {
            return false;
        };
        if map.camera != MapCamera::TwoD {
            return false;
        }
        let tools = TOOLLIST.read().unwrap();
        if tools.editor_mode
            || !matches!(
                tools.current_game_tool_command_id(),
                Some(
                    "tool.vertex"
                        | "tool.linedef"
                        | "tool.sector"
                        | "tool.entity"
                        | "tool.geometry"
                        | "tool.blocks"
                )
            )
        {
            return false;
        }
        let bridge = RUSTERIX.read().unwrap();
        // Named remaining adapters: runtime speech/visibility and isometric,
        // custom contexts/tools and Play are adopted in subsequent mode ports.
        bridge.server.state == rusterix::ServerState::Off
            && bridge.client.messages_to_draw.is_empty()
            && (bridge.scene_handler.settings.visibility_range_2d <= 0.
                || bridge.scene_handler.settings.visibility_alpha_2d <= 0.)
    }
    fn native_world_binding(&self, ui: &mut TheUI, ctx: &TheContext) -> Option<Binding> {
        if !self.native_world_owns_viewport() || crate::geometry_editor::is_visible(ui) {
            return None;
        }
        let map = self.project.get_map(&self.server_ctx)?;
        let document = self.sessions.get(self.active_session)?.id;
        let tool = TOOLLIST.write().unwrap().get_current_tool().id().uuid;
        let view = crate::utils::map_editor_render_view(ui, &self.server_ctx)?;
        let dim = *view.dim();
        if dim.width <= 0 || dim.height <= 0 {
            return None;
        }
        let view = view.id().clone();
        let slots = slots(map, &self.server_ctx);
        let bridge = RUSTERIX.read().unwrap();
        Some(Binding {
            document,
            view,
            tool,
            context: self.server_ctx.pc,
            size: (dim.width as usize, dim.height as usize),
            map: map.id,
            density: ctx.scale_factor.to_bits(),
            style: HudStyle::current(),
            slots,
            assets: AssetKey {
                tiles: bridge.assets.tiles_revision,
                config: bridge.assets.config.clone(),
                palette: bridge.assets.palette.clone(),
                material: self.project.procedural_materials.revision(),
                appearance: self.project.procedural_appearances.revision(),
                geometry: self.procedural_appearance_revision,
            },
        })
    }
    pub(super) fn prepare_native_world_viewport(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        images: &mut UiFrameImages<'_>,
    ) -> Result<Option<Arc<NativeWorldFrame>>, String> {
        let next = self.native_world_binding(ui, ctx);
        self.native_viewport.world.bind(next.clone(), ui);
        let Some(binding) = next else {
            return Ok(None);
        };
        if ctx
            .ui
            .focus
            .as_ref()
            .is_some_and(|id| id.name == binding.view.name && id.uuid != binding.view.uuid)
        {
            ctx.ui.set_focus(&binding.view);
        }
        let map = self
            .project
            .get_map(&self.server_ctx)
            .ok_or("missing native world map")?;
        let world = &mut self.native_viewport.world;
        if world
            .gpu
            .as_ref()
            .is_none_or(|gpu| gpu.registry != images.registry.owner_id())
        {
            if world.gpu.is_some() {
                world.presented = None;
                world.overlay_input.invalidate();
                world.world_capture = None;
                world.events.clear();
                world.epoch = world
                    .epoch
                    .checked_add(1)
                    .expect("world binding epoch exhausted");
                TOOLLIST
                    .write()
                    .unwrap()
                    .get_current_tool()
                    .invalidate_overlay();
            }
            world.gpu = Some(WorldGpu::new(images.registry)?);
        }
        let target = binding.target(world.epoch);
        let gpu = world.gpu.as_mut().unwrap();
        if gpu.document != Some(binding.document) {
            // Reopened/copied documents may reuse map IDs and local revisions.
            // Neither atlas residency nor builder caches can cross that lifetime.
            gpu.document = Some(binding.document);
            gpu.configured = None;
            gpu.scene_key = None;
            gpu.scene_source = None;
            gpu.guides = Default::default();
            gpu.guide_scene = Default::default();
        }
        // CPU-only bridge inputs: no legacy VM is borrowed, swapped or rendered.
        let bridge = RUSTERIX.read().unwrap();
        let source = bridge.scene_handler.build_index.snapshot();
        let ready = {
            let builds = SCENEMANAGER.read().unwrap();
            !builds.has_pending_geometry() && builds.source_matches(map.id, map.camera)
        };
        if gpu.configured.as_ref() != Some(&binding.assets) {
            gpu.scenes
                .configure_editor(gpu.view, &bridge.assets)
                .map_err(|e| e.to_string())?;
            gpu.configured = Some(binding.assets.clone());
            gpu.scene_key = None;
        }
        let key = SceneInputKey {
            stamp: SourceStamp::new(map),
            size: binding.size,
            tool: self.server_ctx.curr_map_tool_type,
            hover: self.server_ctx.hover,
            cursor: self.server_ctx.hover_cursor,
            vertices: map.selected_vertices.clone(),
            linedefs: map.selected_linedefs.clone(),
            sectors: map.selected_sectors.clone(),
            entity: map.selected_entity_item,
            rectangle: map.curr_rectangle,
            line: (map.curr_grid_pos, map.curr_mouse_pos),
            grid: self.server_ctx.show_editor_grid,
            hour: bridge.client.server_time.to_f32().to_bits(),
            animation: bridge.client.animation_frame,
            canvas: bridge.client.editor_canvas_colors,
            build: self.build_values.clone(),
            runtime: bridge.render_state_for_view(map, true),
        };
        let same_scene = gpu.scene_key.as_ref() == Some(&key)
            && gpu
                .scene_source
                .as_ref()
                .is_some_and(|old| Arc::ptr_eq(old, &source));
        if !same_scene {
            let source_changed = gpu
                .scenes
                .apply_editor_source(gpu.view, source.clone())
                .map_err(|e| e.to_string())?;
            let mut description = bridge
                .client
                .editor_view_2d(map, (binding.size.0 as u32, binding.size.1 as u32));
            if let rusterix::editor_view::EditorViewFrame::D2(frame) = &mut description {
                frame.draw_grid = self.server_ctx.show_editor_grid;
                frame.canvas_colors = bridge.client.editor_canvas_colors;
            }
            let scene = gpu.scenes.scene_mut(gpu.view).map_err(|e| e.to_string())?;
            scene.runtime_render_state = key.runtime.clone();
            if source_changed {
                scene.billboards = bridge.scene_handler.billboards.clone();
            }
            scene.settings.backend_2d = rusterix::render_settings::RendererBackend::Raster;
            gpu.guides.set_properties(&self.build_values);
            gpu.guides
                .set_map_tool_type(self.server_ctx.curr_map_tool_type);
            gpu.guides
                .set_map_hover_info(self.server_ctx.hover, self.server_ctx.hover_cursor);
            gpu.guides.draw_grid = self.server_ctx.show_editor_grid;
            gpu.guides.show_entity_markers = true;
            gpu.guides.build_entities_items(
                map,
                &bridge.assets,
                &mut gpu.guide_scene,
                Vec2::new(binding.size.0 as f32, binding.size.1 as f32),
                &None,
                scene,
                false,
            );
            scene.build_dynamics_2d(
                map,
                bridge.client.animation_frame,
                &bridge.assets,
                &Default::default(),
            );
            gpu.scenes
                .prepare_editor(gpu.view, &description, images.registry)
                .map_err(|e| e.to_string())?;
            gpu.scenes
                .render(gpu.view, map.changed as u64, images.registry)
                .map_err(|e| e.to_string())?;
            gpu.scene_key = Some(key);
            gpu.scene_source = Some(source.clone());
        }
        let output = gpu
            .scenes
            .output(gpu.view)
            .ok_or("native world has no visible output")?;
        let material_slots = binding.slots.as_ref().map(|slots| {
            slots
                .iter()
                .map(|(label, source)| crate::actions::ActionMaterialSlot {
                    label: label.clone(),
                    source: source.clone(),
                })
                .collect::<Vec<_>>()
        });
        let light = ctx
            .ui
            .icon("light_small")
            .map(|icon| crate::hud::HudImageSource {
                width: icon.pixel_width(),
                height: icon.pixel_height(),
                pixels: icon.pixels(),
            });
        let overlay = Arc::new(
            TOOLLIST
                .write()
                .unwrap()
                .get_current_tool()
                .prepare_overlay(ToolOverlayInputs {
                    target,
                    style: &binding.style,
                    hud: HudInputs {
                        map,
                        server: &self.server_ctx,
                        assets: &bridge.assets,
                        selected_id: None,
                        material_slots: material_slots.as_deref(),
                        light,
                    },
                })
                .ok_or("native world tool has no overlay producer")?,
        );
        let mut list = PaintList::default();
        list.image(
            output.image,
            Rect::new(0., 0., binding.size.0 as f32, binding.size.1 as f32),
            Sampling::Nearest,
        );
        let mut resources = gpu.icons.frame(&mut gpu.images, images.registry);
        list.ops.extend(
            overlay
                .paint(&mut ctx.text, &mut resources, ctx.scale_factor)
                .map_err(|e| e.to_string())?
                .ops,
        );
        let help = crate::mapeditor::geometry_selection_status_text(map, &self.server_ctx);
        list.ops.extend(
            Hud::prepare_guidance(binding.size, help.as_deref(), binding.style.colors)
                .paint(&mut ctx.text, &mut resources, ctx.scale_factor)
                .map_err(|e| e.to_string())?
                .ops,
        );
        world.serial = world
            .serial
            .checked_add(1)
            .expect("world frame serial exhausted");
        let receipt = FrameReceipt {
            owner: world.receipt_owner,
            serial: world.serial,
        };
        list.ops.push(PaintOp::Receipt(receipt));
        let receipts = list.receipts().collect();
        let list = Arc::new(list);
        let view = ui
            .get_render_view(&binding.view.name)
            .ok_or("native world view removed")?;
        if view.id().uuid != binding.view.uuid {
            return Err("native world widget replaced during preparation".into());
        }
        view.set_paint_list(Some(list.clone()))
            .map_err(|e| e.to_string())?;
        world.published = Some((binding.view.clone(), list));
        Ok(Some(Arc::new(NativeWorldFrame {
            binding,
            target,
            stamp: SourceStamp::new(map),
            source,
            ready,
            overlay,
            receipts,
            receipt,
        })))
    }
    pub(super) fn presented_native_world_viewport(
        &mut self,
        frame: Option<Arc<NativeWorldFrame>>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        let next = self.native_world_binding(ui, ctx);
        self.native_viewport.world.bind(next.clone(), ui);
        let world = &mut self.native_viewport.world;
        let Some(frame) = frame else {
            if next.is_some() {
                TOOLLIST
                    .write()
                    .unwrap()
                    .get_current_tool()
                    .invalidate_overlay();
                world.presented = None;
            }
            return;
        };
        if next.as_ref() != Some(&frame.binding) || frame.target.revision != world.epoch {
            return;
        }
        let Some(map) = self.project.get_map(&self.server_ctx) else {
            return;
        };
        if SourceStamp::new(map) != frame.stamp {
            return;
        }
        let accepted = TOOLLIST
            .write()
            .unwrap()
            .get_current_tool()
            .presented_overlay(
                &frame.overlay,
                &frame.receipts,
                frame.target,
                map,
                &mut self.server_ctx,
            );
        if accepted {
            world.presented = Some(frame);
        } else {
            ctx.ui.redraw_all = true;
        }
    }
    pub(crate) fn finish_native_viewport_frame(
        &mut self,
        root: Option<&PaintList>,
        images: &mut renderkit::ImageRegistry,
    ) -> Result<(), String> {
        if let Some(gpu) = &mut self.native_viewport.world.gpu {
            gpu.finish(images, root)?;
        }
        Ok(())
    }
    pub(crate) fn take_native_world_event(
        &mut self,
        ui: &mut TheUI,
        ctx: &TheContext,
    ) -> Option<TheEvent> {
        while let Some(event) = self.native_viewport.world.events.pop_front() {
            let next = self.native_world_binding(ui, ctx);
            self.native_viewport.world.bind(next.clone(), ui);
            let Some(binding) = next else {
                continue;
            };
            let target = binding.target(self.native_viewport.world.epoch);
            let Some(map) = self.project.get_map(&self.server_ctx) else {
                continue;
            };
            let mut tools = TOOLLIST.write().unwrap();
            if let Some(event) = event.take_if_current(
                tools.get_current_tool().as_mut(),
                target,
                map,
                &self.server_ctx,
            ) {
                return Some(event);
            }
        }
        None
    }
    pub(crate) fn handle_native_world_input(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> bool {
        if !self.native_viewport.enabled {
            return false;
        }
        if !matches!(
            event,
            TheEvent::RenderViewClicked(..)
                | TheEvent::RenderViewDragged(..)
                | TheEvent::RenderViewUp(..)
                | TheEvent::RenderViewHoverChanged(..)
                | TheEvent::KeyDown(..)
                | TheEvent::KeyCodeDown(..)
        ) {
            return false;
        }
        let next = self.native_world_binding(ui, ctx);
        self.native_viewport.world.bind(next.clone(), ui);
        let Some(binding) = next else {
            return false;
        };
        let focused = ui.dialog.is_none()
            && ui.popover.is_none()
            && ctx
                .ui
                .focus
                .as_ref()
                .is_some_and(|id| id.uuid == binding.view.uuid);
        let (id, input) = match event {
            TheEvent::RenderViewClicked(id, at) => (Some(id), MapEvent::MapClicked(*at)),
            TheEvent::RenderViewDragged(id, at) => (Some(id), MapEvent::MapDragged(*at)),
            TheEvent::RenderViewUp(id, at) => (Some(id), MapEvent::MapUp(*at)),
            TheEvent::RenderViewHoverChanged(id, at) => (Some(id), MapEvent::MapHover(*at)),
            TheEvent::KeyDown(TheValue::Char(key))
                if focused && !ui.logo && !ui.ctrl && !ui.alt =>
            {
                (None, MapEvent::MapKey(*key))
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Delete)) if focused => {
                (None, MapEvent::MapDelete)
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Escape)) if focused => {
                (None, MapEvent::MapEscape)
            }
            _ => return false,
        };
        if let Some(id) = id {
            if id.name != binding.view.name {
                return false;
            }
            if id.uuid != binding.view.uuid {
                return true;
            }
        }
        let world = &mut self.native_viewport.world;
        if matches!(input, MapEvent::MapClicked(_)) {
            world.world_capture = None;
        }
        let frame = world.presented.clone();
        let target = binding.target(world.epoch);
        let Some(map) = self.project.get_map_mut(&self.server_ctx) else {
            return true;
        };
        let mut tools = TOOLLIST.write().unwrap();
        let tool = tools.get_current_tool().as_mut();
        let route = world.overlay_input.route(
            tool,
            frame.as_ref().map(|f| &f.overlay),
            target,
            &input,
            focused,
            map,
            &self.server_ctx,
        );
        match route {
            ToolOverlayRoute::Unavailable => {
                ctx.ui.redraw_all = true;
                return true;
            }
            ToolOverlayRoute::Consumed { redraw, command } => {
                ctx.ui.redraw_all |= redraw;
                if let Some(command) = command
                    && let Some(effects) =
                        command.apply(tool, target, map, &mut self.server_ctx, ui, ctx)
                {
                    ctx.ui.redraw_all |=
                        effects.redraw || effects.scene_dirty || effects.overlay_dirty;
                    world.events.extend(effects.events);
                }
                return true;
            }
            ToolOverlayRoute::Outside { redraw } => ctx.ui.redraw_all |= redraw,
        }
        drop(tools);
        let continuation = matches!(
            input,
            MapEvent::MapDragged(_) | MapEvent::MapUp(_) | MapEvent::MapEscape
        );
        if continuation && world.world_capture.as_ref() == Some(&binding) {
            if matches!(input, MapEvent::MapUp(_) | MapEvent::MapEscape) {
                world.world_capture = None;
            }
            return false;
        }
        let fresh = frame.as_ref().is_some_and(|frame| {
            frame.ready
                && frame.stamp == SourceStamp::new(map)
                && {
                    let builds = SCENEMANAGER.read().unwrap();
                    !builds.has_pending_geometry() && builds.source_matches(map.id, map.camera)
                }
                && Arc::ptr_eq(
                    &frame.source,
                    &RUSTERIX
                        .read()
                        .unwrap()
                        .scene_handler
                        .build_index
                        .snapshot(),
                )
        });
        if !fresh || matches!(input, MapEvent::MapDragged(_) | MapEvent::MapUp(_)) {
            ctx.ui.redraw_all = true;
            return true;
        }
        if let MapEvent::MapClicked(at) = input {
            // Hover may have been discarded while source geometry was pending.
            // Resolve this fresh click against its acknowledged 2D transform,
            // rather than reusing a hit from an earlier pointer location/view.
            let size = Vec2::new(binding.size.0 as f32, binding.size.1 as f32);
            let position = Vec2::new(at.x as f32, at.y as f32);
            self.server_ctx.geo_hit = None;
            self.server_ctx.hover_ray_origin_3d = None;
            self.server_ctx.hover_ray_dir_3d = None;
            if matches!(
                self.server_ctx.curr_map_tool_type,
                MapToolType::Vertex | MapToolType::Linedef | MapToolType::Sector
            ) {
                self.server_ctx.hover = self.server_ctx.geometry_at_for_tool(
                    size,
                    position,
                    map,
                    self.server_ctx.curr_map_tool_type,
                );
            }
            self.server_ctx.hover_cursor =
                Some(
                    self.server_ctx
                        .local_to_map_grid(size, position, map, map.subdivisions),
                );
            world.world_capture = Some(binding);
        }
        false
    }
}

#[cfg(test)]
impl Editor {
    pub(crate) fn native_world_has_cpu_storage_for_test(&self) -> bool {
        let gpu = self
            .native_viewport
            .world
            .gpu
            .as_ref()
            .expect("world GPU owner");
        gpu.scenes.has_cpu_target_storage(gpu.view).unwrap()
    }
}
