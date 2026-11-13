//! Creator's authored Game-tool presentation on the common player controller.
//! The CPU runtime facade is borrowed explicitly until GameSession storage cutover.
use super::*;
use eldiron_presentation::{
    GameFonts, GameScreenController, SceneViews, ScreenCapture, ScreenEffect, ScreenTextTarget,
    ScreenViewport, ViewId,
    screen::{ContentRevision, ScreenImageCache},
};
use eldiron_runtime::{PlayerControls, PlayerSettings, RegionEvents, RuntimeView};
use rusterix::{
    EntityAction, ServerState,
    client::{command::ClientCommandBinding, screen_model::ScreenLayout},
};
use std::{
    collections::VecDeque,
    hash::{Hash, Hasher},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use theframework::{
    paint::{FrameReceipt, PaintList, PaintOp, Rect},
    platform::*,
    text_edit::{EditorClipboard, NativeTextAnswer, NativeTextRequest, NativeTextSnapshot},
};
static NEXT_OWNER: AtomicU64 = AtomicU64::new(0x504C_4159_0000);
#[derive(Clone, PartialEq)]
struct PlayBinding {
    document: Uuid,
    runtime: Uuid,
    view: TheId,
    rect: [i32; 4],
    density: u32,
}
impl PlayBinding {
    fn size(&self) -> (u32, u32) {
        (self.rect[2].max(0) as u32, self.rect[3].max(0) as u32)
    }
    fn contains(&self, p: [f32; 2]) -> bool {
        p[0] >= self.rect[0] as f32
            && p[1] >= self.rect[1] as f32
            && p[0] < (self.rect[0] + self.rect[2]) as f32
            && p[1] < (self.rect[1] + self.rect[3]) as f32
    }
}
pub(super) struct NativePlayFrame {
    binding: PlayBinding,
    serial: u64,
    pub(super) receipt: FrameReceipt,
    pub(super) receipts: Vec<FrameReceipt>,
}
struct PlayGpu {
    owner: u64,
    session: Uuid,
    scenes: SceneViews,
    sources: ScreenImageCache,
}
struct PlayScreen {
    session: Uuid,
    screen: GameScreenController,
    fonts: GameFonts,
    settings: PlayerSettings,
    controls: PlayerControls,
    requested: bool,
    pending: VecDeque<EntityAction>,
    retired: Vec<ViewId>,
    key: u64,
    revision: ContentRevision,
    capture: Option<ScreenCapture>,
    clipboard: Option<(ScreenTextTarget, u64)>,
}
impl PlayScreen {
    fn new(
        world: &RuntimeView<'_>,
        input: rusterix::input_action::ClientAction,
        requested: bool,
        selected: &str,
    ) -> Result<Self, String> {
        let settings = PlayerSettings::parse(world.config())?;
        let layout = ScreenLayout {
            screen_responsive: true,
            reference_viewport: Vec2::new(
                settings.viewport.0.min(i32::MAX as u32) as i32,
                settings.viewport.1.min(i32::MAX as u32) as i32,
            ),
            viewport: Vec2::new(
                settings.viewport.0.min(i32::MAX as u32) as i32,
                settings.viewport.1.min(i32::MAX as u32) as i32,
            ),
            grid_size: settings.grid_size,
        };
        let name = if !selected.is_empty() {
            selected
        } else if !settings.start_screen.is_empty() {
            &settings.start_screen
        } else {
            &settings.play_screen
        };
        let screen = if name.is_empty() {
            GameScreenController::region_fallback(world, layout, Default::default())?
        } else {
            GameScreenController::from_screen(name, world, layout)?
        };
        Ok(Self {
            session: world.session_id,
            screen,
            fonts: GameFonts::default(),
            settings,
            controls: input.into(),
            requested,
            pending: VecDeque::new(),
            retired: Vec::new(),
            key: 0,
            revision: ContentRevision(1),
            capture: None,
            clipboard: None,
        })
    }
    fn navigate(&mut self, world: &RuntimeView<'_>, name: &str) -> Result<(), String> {
        let mut next = if name.is_empty() {
            GameScreenController::region_fallback(
                world,
                self.screen.elements.layout,
                self.screen.elements.ui_state.clone(),
            )?
        } else {
            GameScreenController::from_screen_with_state(
                name,
                world,
                self.screen.elements.layout,
                self.screen.elements.ui_state.clone(),
            )?
        };
        next.retain_session_presentation_from(&mut self.screen, &mut self.fonts);
        if let Err(error) = next.prepare(
            world,
            &mut self.fonts,
            self.revision,
            self.screen.animation_frame,
        ) {
            self.screen
                .retain_session_presentation_from(&mut next, &mut self.fonts);
            return Err(error);
        }
        self.retired.extend(self.screen.detach_game_views());
        self.screen = next;
        self.clipboard = None;
        self.capture = None;
        Ok(())
    }
    fn submit(
        &mut self,
        server: &mut rusterix::Server,
        action: EntityAction,
    ) -> Result<(), String> {
        if !self.requested || (server.state != ServerState::Running && action != EntityAction::Off)
        {
            return Ok(());
        }
        if server.has_local_player() {
            for action in self.pending.drain(..) {
                server.local_player_action(action);
            }
            server.local_player_action(action);
        } else {
            if eldiron_runtime::is_movement_state(&action)
                && self
                    .pending
                    .back()
                    .is_some_and(eldiron_runtime::is_movement_state)
            {
                self.pending.pop_back();
            }
            if self.pending.len() >= 256 {
                return Err(
                    "Player registration has not completed; startup input queue is full".into(),
                );
            }
            self.pending.push_back(action);
        }
        Ok(())
    }
    fn cancel(&mut self, server: &mut rusterix::Server) {
        let stop = self.controls.cancel_controls();
        let _ = self.submit(server, stop);
        self.screen.invalidate_world_input();
        self.clipboard = None;
        self.capture = None;
    }
}
#[derive(Default)]
pub(crate) struct NativePlay {
    binding: Option<PlayBinding>,
    screen: Option<PlayScreen>,
    gpu: Option<PlayGpu>,
    published: Option<(TheId, Arc<PaintList>)>,
    viewport: Option<ScreenViewport>,
    presented: Option<PlayBinding>,
    owner: u64,
    serial: u64,
    pointer_capture: bool,
    modifiers: Modifiers,
    focused: Option<bool>,
    input_blocked: bool,
}
impl NativePlay {
    fn clear_paint(&mut self, ui: &mut TheUI) {
        if let Some((id, list)) = self.published.take()
            && let Some(view) = ui.get_render_view(&id.name)
            && view.id().uuid == id.uuid
        {
            view.clear_paint_list(&list);
        }
    }
    fn unbind(&mut self, ui: &mut TheUI, server: &mut rusterix::Server) {
        self.clear_paint(ui);
        if let Some(screen) = &mut self.screen {
            screen.cancel(server);
        }
        self.binding = None;
        self.presented = None;
        self.viewport = None;
        self.pointer_capture = false;
        self.modifiers = Modifiers::default();
    }
    pub(super) fn invalidate(&mut self, ui: &mut TheUI) {
        self.clear_paint(ui);
        if let Some(screen) = &mut self.screen {
            screen.screen.invalidate_world_input();
            screen.clipboard = None;
        }
        self.presented = None;
        self.viewport = None;
        self.pointer_capture = false;
        self.serial = self.serial.checked_add(1).expect("Play serial exhausted");
    }
    fn sync(
        &mut self,
        binding: Option<PlayBinding>,
        world: &RuntimeView<'_>,
        input: rusterix::input_action::ClientAction,
        requested: bool,
        selected: &str,
    ) -> Result<bool, String> {
        if self
            .screen
            .as_ref()
            .is_some_and(|s| s.session != world.session_id)
        {
            self.screen = None;
            self.presented = None;
            self.pointer_capture = false;
        }
        let created = self.screen.is_none() && binding.is_some();
        if created {
            self.screen = Some(PlayScreen::new(world, input, requested, selected)?);
        }
        if self.binding != binding {
            if let Some(screen) = &mut self.screen {
                screen.screen.invalidate_world_input();
            }
            self.presented = None;
            self.pointer_capture = false;
            self.binding = binding;
        }
        Ok(created)
    }
    fn advance(
        &mut self,
        world: &RuntimeView<'_>,
        ticks: u64,
        elapsed: std::time::Duration,
    ) -> Result<(), String> {
        let Some(owner) = &mut self.screen else {
            return Ok(());
        };
        if owner.session != world.session_id {
            return Ok(());
        }
        let key = content_key(world)?;
        if owner.key != key {
            let settings = PlayerSettings::parse(world.config())?;
            let old_layout = owner.screen.elements.layout;
            let old_revision = owner.revision;
            owner.screen.elements.layout.reference_viewport = Vec2::new(
                settings.viewport.0.min(i32::MAX as u32) as i32,
                settings.viewport.1.min(i32::MAX as u32) as i32,
            );
            owner.screen.elements.layout.grid_size = settings.grid_size;
            owner.revision.0 = owner
                .revision
                .0
                .checked_add(1)
                .ok_or("Play content revision exhausted")?;
            let name = owner.screen.name.clone();
            if let Err(error) = owner.navigate(
                world,
                if world.screen(&name).is_some() {
                    &name
                } else {
                    ""
                },
            ) {
                owner.screen.elements.layout = old_layout;
                owner.revision = old_revision;
                return Err(error);
            }
            owner.key = key;
            owner.settings = settings;
            self.presented = None;
        }
        owner.screen.world_texts.advance(elapsed);
        if let Some(binding) = &self.binding {
            let size = binding.size();
            owner.screen.resize(size);
            owner
                .screen
                .prepare(world, &mut owner.fonts, owner.revision, ticks as usize)?;
            let layout = owner.screen.elements.layout.viewport;
            let viewport = ScreenViewport::fit(
                [layout.x as f32, layout.y as f32],
                [size.0 as f32, size.1 as f32],
            );
            if self.viewport != Some(viewport) {
                owner.screen.invalidate_world_input();
                self.presented = None;
                self.viewport = Some(viewport);
            }
            let labels = owner
                .screen
                .tooltip_commands(world)
                .into_iter()
                .filter_map(|command| {
                    let binding = rusterix::input_binding::parse_client_command(&command)?;
                    Some((
                        command,
                        owner.controls.shortcut_labels_for_binding(&binding),
                    ))
                })
                .collect();
            owner.screen.set_tooltip_shortcuts(labels);
        }
        Ok(())
    }
    pub(crate) fn receive(&mut self, world: &RuntimeView<'_>, events: &RegionEvents) {
        if let Some(owner) = &mut self.screen
            && owner.session == world.session_id
        {
            owner.screen.receive(world, events);
        }
    }
    fn paint(
        &mut self,
        world: &RuntimeView<'_>,
        images: &mut renderkit::ImageRegistry,
    ) -> Result<Option<(Arc<PaintList>, NativePlayFrame)>, String> {
        if self.binding.is_none() {
            if let Some(gpu) = &mut self.gpu
                && gpu.owner == images.owner_id()
            {
                if let Some(owner) = &mut self.screen {
                    owner.screen.release_game_views(&mut gpu.scenes, images)?;
                    owner.retired.clear();
                }
                gpu.scenes.clear(images).map_err(|e| e.to_string())?;
                gpu.sources.clear(images)?;
            }
            return Ok(None);
        }
        let binding = self.binding.clone().unwrap();
        let owner = self
            .screen
            .as_mut()
            .ok_or("Prepare the Play controller before painting")?;
        if self
            .gpu
            .as_ref()
            .is_none_or(|g| g.owner != images.owner_id())
        {
            owner.screen.detach_game_views();
            owner.retired.clear();
            self.gpu = Some(PlayGpu {
                owner: images.owner_id(),
                session: owner.session,
                scenes: SceneViews::new(images, scenevm::SharedAtlas::new(64, 64)),
                sources: ScreenImageCache::default(),
            });
        }
        let gpu = self.gpu.as_mut().unwrap();
        if gpu.session != owner.session {
            gpu.scenes.clear(images).map_err(|e| e.to_string())?;
            gpu.sources.clear(images)?;
            gpu.session = owner.session;
        }
        for id in owner.retired.drain(..) {
            gpu.scenes.remove(id, images).map_err(|e| e.to_string())?;
        }
        owner
            .screen
            .prepare_game_views(world, &mut gpu.scenes, images)?;
        let mut sources = gpu.sources.begin_frame(images)?;
        let paint = owner.screen.paint(world, &mut owner.fonts, &mut sources)?;
        let mut list = PaintList::default();
        list.ops.push(PaintOp::PushClip(Rect::new(
            0.,
            0.,
            binding.rect[2] as f32,
            binding.rect[3] as f32,
        )));
        list.ops.push(PaintOp::PushTransform(
            self.viewport
                .ok_or("Prepare Play viewport before painting")?
                .transform(),
        ));
        list.ops.extend(paint.ops);
        list.ops.extend([PaintOp::PopTransform, PaintOp::PopClip]);
        if self.owner == 0 {
            self.owner = NEXT_OWNER.fetch_add(1, Ordering::Relaxed);
        }
        self.serial = self
            .serial
            .checked_add(1)
            .ok_or("Play frame serial exhausted")?;
        let receipt = FrameReceipt {
            owner: self.owner,
            serial: self.serial,
        };
        list.ops.push(PaintOp::Receipt(receipt));
        let receipts = list.receipts().collect();
        Ok(Some((
            Arc::new(list),
            NativePlayFrame {
                binding,
                serial: self.serial,
                receipt,
                receipts,
            },
        )))
    }
}
struct HashWriter(std::hash::DefaultHasher);
impl std::io::Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.write(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn content_key(world: &RuntimeView<'_>) -> Result<u64, String> {
    let mut h = HashWriter(Default::default());
    world.config().hash(&mut h.0);
    world.assets.tiles_revision.hash(&mut h.0);
    world.assets.block_props_revision.hash(&mut h.0);
    world.assets.rules.hash(&mut h.0);
    for screen in world.screens() {
        serde_json::to_writer(&mut h, &(screen.id, screen.responsive, screen.map))
            .map_err(|e| e.to_string())?;
    }
    for font in world.fonts() {
        font.id.hash(&mut h.0);
        font.name.hash(&mut h.0);
        font.bytes.hash(&mut h.0);
    }
    serde_json::to_writer(&mut h, &world.assets.palette).map_err(|e| e.to_string())?;
    Ok(h.0.finish())
}
impl Editor {
    pub(crate) fn native_play_owns_viewport(&self) -> bool {
        self.native_viewport.enabled
            && self.server_ctx.game_mode
            && !self.server_ctx.text_game_mode
            && self.play_session.as_ref().is_some_and(|play| {
                self.sessions
                    .get(self.active_session)
                    .is_some_and(|doc| doc.id == play.document)
            })
            && RUSTERIX.read().unwrap().server.state != ServerState::Off
    }
    fn native_play_binding(&self, ui: &mut TheUI, ctx: &TheContext) -> Option<PlayBinding> {
        if !self.native_play_owns_viewport() || crate::geometry_editor::is_visible(ui) {
            return None;
        }
        let play = self.play_session.as_ref()?;
        let view = ui.get_render_view("PolyView")?;
        let dim = *view.dim();
        (dim.width > 0 && dim.height > 0).then(|| PlayBinding {
            document: play.document,
            runtime: play.pump.session_id(),
            view: view.id().clone(),
            rect: [dim.x, dim.y, dim.width, dim.height],
            density: ctx.scale_factor.to_bits(),
        })
    }
    pub(crate) fn sync_native_play(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        elapsed: std::time::Duration,
    ) -> Result<(), String> {
        let binding = self.native_play_binding(ui, ctx);
        let mut bridge = RUSTERIX.write().unwrap();
        let blocked = ui.dialog.is_some() || ui.popover.is_some() || ui.context_menu.is_some();
        let lost_focus = binding.as_ref().is_some_and(|binding| {
            ctx.ui
                .focus
                .as_ref()
                .is_none_or(|id| id.uuid != binding.view.uuid)
        }) && self
            .native_viewport
            .play
            .screen
            .as_ref()
            .is_some_and(|owner| owner.controls.held_count() > 0);
        if self.native_viewport.play.binding != binding
            || self.native_viewport.play.input_blocked != blocked
            || lost_focus
        {
            if let Some(owner) = &mut self.native_viewport.play.screen {
                owner.cancel(&mut bridge.server);
            }
            self.native_viewport.play.pointer_capture = false;
        }
        if blocked != self.native_viewport.play.input_blocked {
            self.native_viewport.play.presented = None;
            self.native_viewport.play.serial = self
                .native_viewport
                .play
                .serial
                .checked_add(1)
                .expect("Play serial exhausted");
        }
        self.native_viewport.play.input_blocked = blocked;
        let Some(play) = self.play_session.as_ref().filter(|s| {
            self.sessions
                .get(self.active_session)
                .is_some_and(|d| d.id == s.document)
        }) else {
            self.native_viewport.play.unbind(ui, &mut bridge.server);
            self.native_viewport.play.screen = None;
            return Ok(());
        };
        let world = play.pump.view(
            &self.project,
            &bridge.server,
            &bridge.assets,
            &bridge.client.current_map,
        );
        let input = bridge.client.player_input_snapshot();
        let requested = bridge.client.local_player_requested();
        let selected = bridge.client.current_screen_name();
        let created =
            self.native_viewport
                .play
                .sync(binding.clone(), &world, input, requested, selected)?;
        if created {
            let outputs = RegionEvents {
                messages: std::mem::take(&mut self.pending_game_messages),
                says: std::mem::take(&mut self.pending_game_says),
                choices: std::mem::take(&mut self.pending_game_choices),
                ..Default::default()
            };
            self.native_viewport.play.receive(&world, &outputs);
        } else if binding.is_some() {
            self.pending_game_messages.clear();
            self.pending_game_says.clear();
            self.pending_game_choices.clear();
        }
        self.native_viewport
            .play
            .advance(&world, play.pump.tick_sequence(), elapsed)?;
        let owner = &mut self.native_viewport.play;

        if let Some(owner) = &mut owner.screen {
            let isometric=world.map.and_then(|m|m.entities.iter().find(|e|e.is_player())).is_some_and(|e|matches!(e.attributes.get("player_camera"),Some(Value::PlayerCamera(camera)) if camera.is_isometric_2d()));
            if let Some(action) = owner.controls.set_isometric_controls(isometric) {
                owner.submit(&mut bridge.server, action)?;
            }
            if bridge.server.state == ServerState::Running && bridge.server.has_local_player() {
                for action in owner.pending.drain(..) {
                    bridge.server.local_player_action(action);
                }
            }
        }
        Ok(())
    }
    pub(super) fn prepare_native_play_viewport(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        images: &mut renderkit::ImageRegistry,
    ) -> Result<Option<NativePlayFrame>, String> {
        self.sync_native_play(ui, ctx, std::time::Duration::ZERO)?;
        self.native_viewport.play.clear_paint(ui);
        let Some(play) = &self.play_session else {
            if let Some(gpu) = &mut self.native_viewport.play.gpu
                && gpu.owner == images.owner_id()
            {
                gpu.scenes.clear(images).map_err(|e| e.to_string())?;
                gpu.sources.clear(images)?;
            }
            return Ok(None);
        };
        let bridge = RUSTERIX.read().unwrap();
        let world = play.pump.view(
            &self.project,
            &bridge.server,
            &bridge.assets,
            &bridge.client.current_map,
        );
        let Some((list, frame)) = self.native_viewport.play.paint(&world, images)? else {
            return Ok(None);
        };
        let view = ui
            .get_render_view(&frame.binding.view.name)
            .ok_or("Play view removed during preparation")?;
        view.set_paint_list(Some(list.clone()))
            .map_err(|e| e.to_string())?;
        self.native_viewport.play.published = Some((frame.binding.view.clone(), list));
        Ok(Some(frame))
    }
    pub(super) fn presented_native_play_viewport(
        &mut self,
        frame: Option<NativePlayFrame>,
        ui: &mut TheUI,
        ctx: &TheContext,
    ) {
        let binding = self.native_play_binding(ui, ctx);
        let owner = &mut self.native_viewport.play;
        let Some(frame) = frame else {
            owner.presented = None;
            if let Some(screen) = &mut owner.screen {
                screen.screen.invalidate_world_input();
            }
            return;
        };
        if owner.input_blocked
            || binding.as_ref() != Some(&frame.binding)
            || frame.serial != owner.serial
        {
            return;
        }
        let Some(play) = &self.play_session else {
            return;
        };
        let bridge = RUSTERIX.read().unwrap();
        let world = play.pump.view(
            &self.project,
            &bridge.server,
            &bridge.assets,
            &bridge.client.current_map,
        );
        if let Some(screen) = &mut owner.screen {
            screen
                .screen
                .acknowledge_world_frame(&world, owner.gpu.as_ref().map(|g| &g.scenes));
            owner.presented = binding;
        }
    }
    pub(crate) fn reset_native_play_gpu(
        &mut self,
        images: &mut renderkit::ImageRegistry,
    ) -> Result<(), String> {
        let play = &mut self.native_viewport.play;
        if let Some(mut gpu) = play.gpu.take().filter(|gpu| gpu.owner == images.owner_id()) {
            gpu.scenes.clear(images).map_err(|e| e.to_string())?;
            gpu.sources.clear(images)?;
        }
        if let Some(owner) = &mut play.screen {
            owner.screen.detach_game_views();
            owner.retired.clear();
        }
        play.presented = None;
        Ok(())
    }
    pub(crate) fn cancel_native_play_input(&mut self, ui: &mut TheUI) {
        let mut bridge = RUSTERIX.write().unwrap();
        let owner = &mut self.native_viewport.play;
        if let Some(screen) = &mut owner.screen {
            screen.cancel(&mut bridge.server);
        }
        owner.invalidate(ui);
    }
}
struct PlayRuntime<'a> {
    pump: &'a eldiron_runtime::SessionPump,
    project: &'a Project,
    bridge: &'a mut rusterix::Rusterix,
}
impl PlayRuntime<'_> {
    fn world(&self) -> RuntimeView<'_> {
        self.pump.view(
            self.project,
            &self.bridge.server,
            &self.bridge.assets,
            &self.bridge.client.current_map,
        )
    }
}
impl NativePlay {
    fn apply(
        &mut self,
        runtime: &mut PlayRuntime<'_>,
        effects: Vec<ScreenEffect>,
    ) -> Result<(), String> {
        let owner = self.screen.as_mut().ok_or("Play screen is not active")?;
        for effect in effects {
            if owner.screen.handle_hud_effect(&runtime.world(), &effect) {
                continue;
            }
            match effect {
                ScreenEffect::Action(action) => owner.submit(&mut runtime.bridge.server, action)?,
                ScreenEffect::Command(ClientCommandBinding::Screen(command)) => {
                    let name = command
                        .trim()
                        .strip_prefix("goto.")
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .ok_or("Invalid screen navigation command")?;
                    owner.navigate(&runtime.world(), name)?;
                    let stop = owner.controls.cancel_controls();
                    owner.submit(&mut runtime.bridge.server, stop)?;
                    self.presented = None;
                }
                ScreenEffect::Command(ClientCommandBinding::Game(command)) => {
                    if owner.requested {
                        continue;
                    }
                    let class = if command == "start" {
                        owner.screen.elements.ui_state.get("start.class").cloned()
                    } else {
                        Some(
                            command
                                .strip_prefix("start_class.")
                                .filter(|s| !s.is_empty())
                                .ok_or("Unknown game start command")?
                                .to_string(),
                        )
                    };
                    if let Some(class) = &class {
                        owner
                            .screen
                            .elements
                            .ui_state
                            .insert("start.class".into(), class.clone());
                    }
                    let map = runtime
                        .bridge
                        .assets
                        .maps
                        .get(&runtime.bridge.client.current_map)
                        .ok_or("Player start map is unavailable")?;
                    let name = owner.screen.elements.ui_state.get("start.name").cloned();
                    let prepared = rusterix::player_start::prepare_player_start(
                        &runtime.bridge.assets,
                        map,
                        eldiron_runtime::PlayerStartOptions {
                            class: class.as_deref(),
                            name: name.as_deref(),
                            entrance: true,
                        },
                    )?;
                    owner
                        .controls
                        .configure(&prepared.info, &runtime.bridge.assets);
                    runtime
                        .bridge
                        .server
                        .process_client_commands(vec![prepared.command]);
                    owner.requested = true;
                    if !owner.settings.play_screen.is_empty() {
                        let name = owner.settings.play_screen.clone();
                        owner.navigate(&runtime.world(), &name)?;
                        self.presented = None;
                    }
                }
                ScreenEffect::Command(ClientCommandBinding::Ui(_)) => {}
                other => return Err(format!("Unhandled Creator Play effect: {other:?}")),
            }
        }
        Ok(())
    }
    fn route(
        &mut self,
        runtime: &mut PlayRuntime<'_>,
        event: &InputEvent,
        requests: &mut Vec<HostRequest>,
    ) -> Result<InputResponse, String> {
        if let InputEvent::Modifiers(modifiers) = event {
            self.modifiers = *modifiers;
        }
        if matches!(event, InputEvent::Focus(false)) {
            self.modifiers = Modifiers::default();
            self.pointer_capture = false;
        }
        let Some(owner) = &mut self.screen else {
            return Ok(InputResponse::default());
        };
        if matches!(event, InputEvent::Focus(true)) {
            owner.screen.resume_world_input();
        }
        if let InputEvent::Focus(focused) = event {
            self.focused = Some(*focused);
        }
        if matches!(event, InputEvent::Focus(false)) {
            self.presented = None;
            self.serial = self.serial.checked_add(1).expect("Play serial exhausted");
        }
        if let InputEvent::Clipboard { request, result } = event {
            let Some((target, pending)) = owner.clipboard.filter(|(_, pending)| pending == request)
            else {
                return Ok(InputResponse::default());
            };
            owner.clipboard = None;
            return Ok(owner
                .screen
                .complete_editor_clipboard(&mut owner.fonts, target, pending, result.clone())
                .input);
        }
        let mut result = if owner.screen.route_world_first(event) {
            Default::default()
        } else {
            owner
                .screen
                .input(&runtime.world(), &mut owner.fonts, event)?
        };
        let world = owner.screen.world_input(
            &runtime.world(),
            self.gpu.as_ref().map(|g| &g.scenes),
            event,
            result.input.consumed || owner.screen.captures_world_input(),
        );
        result.input.consumed |= world.input.consumed;
        result.input.redraw |= world.input.redraw;
        result.cancel_controls |= world.cancel_controls;
        result.effects.extend(world.effects);
        if result.cancel_controls {
            let stop = owner.controls.cancel_controls();
            owner.submit(&mut runtime.bridge.server, stop)?;
        }
        if let Some(response) = &result.text
            && let (Some(target), Some(clipboard)) = (response.target, &response.editor.clipboard)
        {
            match clipboard {
                EditorClipboard::Read { request } => {
                    owner.clipboard = Some((target, *request));
                    requests.push(HostRequest::ClipboardRead { request: *request });
                }
                EditorClipboard::Write { text, cut } => {
                    if let Some(request) = cut {
                        owner.clipboard = Some((target, *request));
                    }
                    requests.push(HostRequest::ClipboardWrite {
                        text: text.clone(),
                        request: *cut,
                    });
                }
            }
        }
        let mut input = result.input;
        self.apply(runtime, result.effects)?;
        let owner = self.screen.as_mut().unwrap();
        if let InputEvent::Key {
            physical,
            logical,
            pressed: false,
            repeat: false,
        } = event
            && let Some(key) = binding_key(*physical, logical)
            && let Some(action) = owner.controls.control_key(&key, false)
        {
            owner.submit(&mut runtime.bridge.server, action)?;
            input.consumed = true;
            input.redraw = true;
        }
        if !input.consumed
            && !owner.screen.captures_world_input()
            && !self.modifiers.control
            && !self.modifiers.super_key
            && !self.modifiers.alt
        {
            if let InputEvent::Key {
                physical,
                logical,
                pressed,
                repeat,
            } = event
                && let Some(key) = binding_key(*physical, logical)
                && let Some(binding) = owner.controls.binding_for_key(&key)
            {
                input.consumed = true;
                if *pressed && !*repeat {
                    if matches!(binding, ClientCommandBinding::Control(_)) {
                        if let Some(action) = owner.controls.control_key(&key, true) {
                            owner.submit(&mut runtime.bridge.server, action)?;
                        }
                    } else {
                        let effects = owner
                            .screen
                            .activate_binding(&runtime.world(), binding)?
                            .effects;
                        self.apply(runtime, effects)?;
                    }
                    input.redraw = true;
                }
            }
        }
        let owner = self.screen.as_mut().unwrap();
        let capture = owner.screen.control_capture();
        if capture != owner.capture && !result.cancel_controls {
            let stop = owner.controls.cancel_controls();
            owner.submit(&mut runtime.bridge.server, stop)?;
        }
        owner.capture = capture;
        Ok(input)
    }
}
// Physical identities are shared with the common player, including composed text.
use eldiron_presentation::player_binding_key as binding_key;
impl Editor {
    pub(crate) fn native_play_raw_input(
        &mut self,
        event: &InputEvent,
        session: &mut UiSession,
    ) -> Option<InputResponse> {
        let binding = self.native_play_binding(&mut session.ui, &session.context)?;
        let inside = match event {
            InputEvent::PointerMoved { position } | InputEvent::PointerButton { position, .. } => {
                Some(binding.contains(*position))
            }
            _ => None,
        };
        let owned = self.native_viewport.play.pointer_capture;
        if inside == Some(false) && !owned {
            return None;
        }
        let focus = session
            .context
            .ui
            .focus
            .as_ref()
            .is_some_and(|id| id.uuid == binding.view.uuid);
        if inside.is_none()
            && !focus
            && !matches!(
                event,
                InputEvent::Focus(_) | InputEvent::Modifiers(_) | InputEvent::Clipboard { .. }
            )
        {
            return None;
        }
        if session.ui.dialog.is_some()
            || session.ui.popover.is_some()
            || session.ui.context_menu.is_some()
        {
            self.cancel_native_play_input(&mut session.ui);
            return None;
        }
        if let Err(error) = self.sync_native_play(
            &mut session.ui,
            &mut session.context,
            std::time::Duration::ZERO,
        ) {
            session
                .context
                .ui
                .send(TheEvent::SetStatusText(TheId::empty(), error));
            return Some(InputResponse {
                consumed: true,
                redraw: true,
            });
        }
        if let InputEvent::PointerButton { pressed: true, .. } = event {
            session.context.ui.set_focus(&binding.view);
        }
        let control_event = matches!(
            event,
            InputEvent::Focus(_) | InputEvent::Modifiers(_) | InputEvent::Clipboard { .. }
        );
        if (self.native_viewport.play.presented.as_ref() != Some(&binding)
            || self.native_viewport.play.focused == Some(false))
            && !control_event
        {
            return Some(InputResponse {
                consumed: true,
                redraw: true,
            });
        }
        let viewport = self.native_viewport.play.viewport?;
        let mut mapping = viewport;
        mapping.offset[0] += binding.rect[0] as f32;
        mapping.offset[1] += binding.rect[1] as f32;
        let event = mapping.input(event)?;
        if matches!(event, InputEvent::PointerButton { pressed: true, .. }) {
            self.native_viewport.play.pointer_capture = true;
        }
        if matches!(event, InputEvent::PointerButton { pressed: false, .. }) {
            self.native_viewport.play.pointer_capture = false;
        }
        let mut bridge = RUSTERIX.write().unwrap();
        let play = self.play_session.as_ref()?;
        let mut runtime = PlayRuntime {
            pump: &play.pump,
            project: &self.project,
            bridge: &mut bridge,
        };
        let result = self
            .native_viewport
            .play
            .route(&mut runtime, &event, &mut session.requests);
        match result {
            Ok(response) => {
                if let Some(owner) = &self.native_viewport.play.screen {
                    session
                        .context
                        .set_cursor_visible(!owner.screen.uses_custom_cursor());
                }
                Some(response)
            }
            Err(error) => {
                session
                    .context
                    .ui
                    .send(TheEvent::SetStatusText(TheId::empty(), error));
                Some(InputResponse {
                    consumed: true,
                    redraw: true,
                })
            }
        }
    }
    fn native_play_text_mapping(
        &mut self,
        ui: &mut TheUI,
        ctx: &TheContext,
    ) -> Option<theframework::paint::Transform> {
        let binding = self.native_play_binding(ui, ctx)?;
        if self.native_viewport.play.focused == Some(false)
            || self.native_viewport.play.input_blocked
            || ui.context_menu.is_some()
            || ui.dialog.is_some()
            || ui.popover.is_some()
            || ctx
                .ui
                .focus
                .as_ref()
                .is_none_or(|id| id.uuid != binding.view.uuid)
            || self.native_viewport.play.presented.as_ref() != Some(&binding)
        {
            return None;
        }
        let mut mapping = self.native_viewport.play.viewport?;
        mapping.offset[0] += binding.rect[0] as f32;
        mapping.offset[1] += binding.rect[1] as f32;
        Some(mapping.transform())
    }
    pub(crate) fn native_play_text_input(
        &mut self,
        ui: &mut TheUI,
        ctx: &TheContext,
    ) -> Option<TextInputState> {
        let transform = self.native_play_text_mapping(ui, ctx)?;
        let owner = self.native_viewport.play.screen.as_mut()?;
        let mut input = owner.screen.text_input_state(&mut owner.fonts)?;
        let a = transform.point([input.area.x, input.area.y]);
        let b = transform.point([
            input.area.x + input.area.width,
            input.area.y + input.area.height,
        ]);
        input.area = Rect::new(a[0], a[1], b[0] - a[0], b[1] - a[1]);
        Some(input)
    }
    pub(crate) fn native_play_text_snapshot(
        &mut self,
        ui: &mut TheUI,
        ctx: &TheContext,
    ) -> Option<NativeTextSnapshot> {
        self.native_play_text_mapping(ui, ctx)?;
        let owner = self.native_viewport.play.screen.as_mut()?;
        owner.screen.native_text_snapshot(&mut owner.fonts)
    }
    pub(crate) fn native_play_text_query(
        &mut self,
        ui: &mut TheUI,
        ctx: &TheContext,
        request: &NativeTextRequest,
    ) -> Option<NativeTextAnswer> {
        let transform = self.native_play_text_mapping(ui, ctx)?;
        let local = request.map_hit(transform.inverse()?);
        let owner = self.native_viewport.play.screen.as_mut()?;
        owner
            .screen
            .native_text_query(&mut owner.fonts, &local)
            .map(|answer| answer.transformed(transform))
    }
}

#[cfg(test)]
impl Editor {
    pub(crate) fn native_game_stats_for_test(&self) -> Option<(Uuid, usize, usize, bool)> {
        let owner = self.native_viewport.play.screen.as_ref()?;
        let gpu = self.native_viewport.play.gpu.as_ref()?;
        let views = owner.screen.view_images();
        Some((
            owner.session,
            owner.controls.held_count(),
            views.len(),
            views
                .values()
                .any(|image| gpu.scenes.has_cpu_target_storage(image.view).unwrap()),
        ))
    }
    pub(crate) fn native_game_input_for_test(&self) -> Option<[f32; 2]> {
        let owner = self.native_viewport.play.screen.as_ref()?;
        let input = owner.screen.elements.text_input_widgets.values().next()?;
        let binding = self.native_viewport.play.binding.as_ref()?;
        let mut viewport = self.native_viewport.play.viewport?;
        viewport.offset[0] += binding.rect[0] as f32;
        viewport.offset[1] += binding.rect[1] as f32;
        let rect = viewport.host_rect(Rect::new(
            input.rect.x,
            input.rect.y,
            input.rect.width,
            input.rect.height,
        ));
        Some([rect.x + rect.width * 0.5, rect.y + rect.height * 0.5])
    }
}
