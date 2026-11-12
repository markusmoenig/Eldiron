//! Common graphical player application for native windows, foreign layers and
//! Creator Play. Runtime, CPU screen state and GPU resources have distinct owners.
mod audio;
mod timing;
mod viewport;
use crate::player_session::PlayerSession;
pub use audio::AudioServices;
use eldiron_presentation::{
    GameFonts, GameScreenController, SceneViews, ScreenCapture, ScreenEffect, ScreenTextResponse,
    ScreenTextTarget, ViewId,
    screen::{ContentRevision, ScreenImageCache},
};
use eldiron_runtime::{Project, RegionOutputs, RuntimeView};
use renderkit::{ImageRegistry, RenderError, Result};
use rusterix::{
    ServerState,
    client::{command::ClientCommandBinding, screen_model::ScreenLayout},
};
use scenevm::{SharedAtlas, prelude::Vec2};
use std::time::Duration;
use theframework::{
    paint::{PaintList, PaintOp, Rect},
    platform::*,
    text_edit::EditorClipboard,
};
use uuid::Uuid;
pub use viewport::ScreenViewport;

/// The assembled host supplies platform/output services. Batches are borrowed
/// exactly once after the active screen has received its messages/choices.
/// Audio, world text and container consumers must not drain the server again.
pub trait PlayerServices {
    fn update(&mut self, _world: &RuntimeView<'_>, _revision: u64, _elapsed: Duration) {}
    fn user_gesture(&mut self, _world: &RuntimeView<'_>, _revision: u64) {}
    fn set_suspended(&mut self, _suspended: bool) {}
    fn wake_after(&self) -> Option<Duration> {
        None
    }
    fn region_events(&mut self, region: Uuid, world: &RuntimeView<'_>, events: RegionOutputs<'_>);
    fn screen_effect(
        &mut self,
        world: &RuntimeView<'_>,
        effect: &ScreenEffect,
    ) -> std::result::Result<(), String>;
}
struct PlayerGpu {
    owner: u64,
    views: SceneViews,
    sources: ScreenImageCache,
}
impl PlayerGpu {
    fn new(images: &ImageRegistry) -> Self {
        Self {
            owner: images.owner_id(),
            views: SceneViews::new(images, SharedAtlas::new(64, 64)),
            sources: ScreenImageCache::default(),
        }
    }
}
pub struct PlayerApplication {
    pub player: PlayerSession,
    pub screen: GameScreenController,
    pub fonts: GameFonts,
    services: Box<dyn PlayerServices>,
    gpu: Option<PlayerGpu>,
    retired_views: Vec<ViewId>,
    viewport: ScreenViewport,
    input_metrics: Option<((u32, u32), u32)>,
    revision: ContentRevision,
    visual_elapsed: Duration,
    frames: timing::FrameSchedule,
    frame_count: u64,
    modifiers: Modifiers,
    control_capture: Option<ScreenCapture>,
    pending_clipboard: Option<(ScreenTextTarget, u64)>,
    failure: Option<String>,
    suspended: bool,
    resume_simulation: bool,
}
impl PlayerApplication {
    /// CPU startup occurs once, independently of whether the host has a surface.
    /// A surface/device replacement calls `gpu_recreated`, never this constructor.
    pub fn new(
        project: Project,
        debug: bool,
        services: Box<dyn PlayerServices>,
    ) -> std::result::Result<Self, String> {
        let mut player = PlayerSession::new(project, debug)?;
        let size = player.settings.viewport;
        let layout = ScreenLayout {
            screen_responsive: true,
            reference_viewport: Vec2::new(
                size.0.min(i32::MAX as u32) as i32,
                size.1.min(i32::MAX as u32) as i32,
            ),
            viewport: Vec2::new(
                size.0.min(i32::MAX as u32) as i32,
                size.1.min(i32::MAX as u32) as i32,
            ),
            grid_size: player.settings.grid_size,
        };
        let world = player.runtime.view();
        for name in [&player.settings.start_screen, &player.settings.play_screen]
            .into_iter()
            .filter(|s| !s.is_empty())
        {
            if world.screen(name).is_none() {
                return Err(format!("Configured player screen '{name}' does not exist"));
            }
        }
        let name = if !player.settings.start_screen.is_empty() {
            &player.settings.start_screen
        } else {
            &player.settings.play_screen
        };
        let mut screen = if name.is_empty() {
            GameScreenController::region_fallback(&world, layout, Default::default())?
        } else {
            GameScreenController::from_screen(name, &world, layout)?
        };
        let mut fonts = GameFonts::default();
        screen.prepare(&world, &mut fonts, ContentRevision(1), 0)?;
        player.auto_start(!screen.elements.game_widgets.is_empty())?;
        Ok(Self {
            player,
            screen,
            fonts,
            services,
            gpu: None,
            retired_views: Vec::new(),
            viewport: ScreenViewport::fit(
                [size.0 as f32, size.1 as f32],
                [size.0 as f32, size.1 as f32],
            ),
            revision: ContentRevision(1),
            input_metrics: None,
            visual_elapsed: Duration::ZERO,
            frames: timing::FrameSchedule::default(),
            frame_count: 0,
            modifiers: Modifiers::default(),
            control_capture: None,
            pending_clipboard: None,
            failure: None,
            suspended: false,
            resume_simulation: false,
        })
    }
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
    pub fn viewport(&self) -> ScreenViewport {
        self.viewport
    }
    fn fail(&mut self, host: &mut HostContext, error: String) {
        if self.failure.is_none() {
            host.requests.push(HostRequest::Fail(error.clone()));
            self.failure = Some(error);
        }
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    /// A host menu or action catalogue dispatches the same semantic commands as
    /// keyboard input. No synthetic pointer or duplicate gameplay path is needed.
    pub fn command(&mut self, binding: ClientCommandBinding) -> std::result::Result<(), String> {
        let activation = self
            .screen
            .activate_binding(&self.player.runtime.view(), binding)?;
        self.apply(activation.effects)
    }
    pub fn navigate(&mut self, name: &str) -> std::result::Result<(), String> {
        let world = self.player.runtime.view();
        let mut next = GameScreenController::from_screen_with_state(
            name,
            &world,
            self.screen.elements.layout,
            self.screen.elements.ui_state.clone(),
        )?;
        next.prepare(
            &world,
            &mut self.fonts,
            self.revision,
            self.screen.animation_frame,
        )?;
        // Only identities belonging to the last GPU frame need delayed cleanup.
        // Repeated navigation while occluded cannot retain whole old screens.
        self.retired_views.extend(self.screen.detach_game_views());
        next.speech = std::mem::take(&mut self.screen.speech);
        next.speech.invalidate_geometry();
        next.world_texts = std::mem::take(&mut self.screen.world_texts);
        next.world_texts.invalidate_geometry();
        self.screen = next;
        self.pending_clipboard = None;
        let stop = self.player.cancel_controls();
        self.player.submit_action(stop)?;
        self.player
            .auto_start(!self.screen.elements.game_widgets.is_empty())?;
        Ok(())
    }
    fn sync_host(&mut self, host: &mut HostContext) -> std::result::Result<(), String> {
        let metrics = (host.physical_size, host.density.to_bits());
        if self.input_metrics != Some(metrics) {
            self.screen.invalidate_world_input();
            self.input_metrics = Some(metrics);
        }
        let logical = (
            host.logical_size[0].round().max(0.) as u32,
            host.logical_size[1].round().max(0.) as u32,
        );
        self.screen.resize(logical);
        let animation_frame = self.animation_frame();
        self.screen.prepare(
            &self.player.runtime.view(),
            &mut self.fonts,
            self.revision,
            animation_frame,
        )?;
        let capture = self.screen.control_capture();
        if capture != self.control_capture {
            let stop = self.player.cancel_controls();
            self.player.submit_action(stop)?;
        }
        self.control_capture = capture;
        let size = self.screen.elements.layout.viewport;
        let viewport = ScreenViewport::fit([size.x as f32, size.y as f32], host.logical_size);
        if viewport != self.viewport {
            self.screen.invalidate_world_input();
        }
        self.viewport = viewport;
        host.text_input = if self.viewport.scale > 0. {
            self.screen
                .text_input_state(&mut self.fonts)
                .map(|mut input| {
                    input.area = self.viewport.host_rect(input.area);
                    input
                })
        } else {
            None
        };
        if self.suspended {
            host.text_input = None;
        }
        Ok(())
    }
    fn animation_frame(&self) -> usize {
        (self.visual_elapsed.as_secs_f64() * self.player.settings.target_fps).floor() as usize
    }
    fn clipboard(&mut self, response: &ScreenTextResponse, host: &mut HostContext) {
        if let (Some(widget), Some(clipboard)) = (response.target, &response.editor.clipboard) {
            match clipboard {
                EditorClipboard::Read { request } => {
                    self.pending_clipboard = Some((widget, *request));
                    host.requests
                        .push(HostRequest::ClipboardRead { request: *request });
                }
                EditorClipboard::Write { text, cut } => {
                    if let Some(request) = cut {
                        self.pending_clipboard = Some((widget, *request));
                    }
                    host.requests.push(HostRequest::ClipboardWrite {
                        text: text.clone(),
                        request: *cut,
                    });
                }
            }
        }
    }
    fn apply(&mut self, effects: Vec<ScreenEffect>) -> std::result::Result<(), String> {
        for effect in effects {
            match effect {
                ScreenEffect::Action(action) => self.player.submit_action(action)?,
                ScreenEffect::Command(ClientCommandBinding::Game(command)) => {
                    let transition = self
                        .player
                        .game_command(&command, &mut self.screen.elements.ui_state)?;
                    if let Some(name) = transition.screen {
                        self.navigate(&name)?;
                    }
                }
                ScreenEffect::Command(ClientCommandBinding::Screen(command)) => {
                    let name = self.player.screen_command(&command)?;
                    self.navigate(&name)?;
                }
                other => self
                    .services
                    .screen_effect(&self.player.runtime.view(), &other)?,
            }
        }
        Ok(())
    }
    fn route(
        &mut self,
        event: &InputEvent,
        host: &mut HostContext,
    ) -> std::result::Result<InputResponse, String> {
        if self.suspended && !matches!(event, InputEvent::Focus(false)) {
            return Ok(InputResponse::default());
        }
        if matches!(
            event,
            InputEvent::Key {
                pressed: true,
                repeat: false,
                ..
            } | InputEvent::PointerButton { pressed: true, .. }
        ) {
            self.services
                .user_gesture(&self.player.runtime.view(), self.revision.0);
        }
        if let InputEvent::Clipboard { request, result } = event {
            let Some((widget, pending)) = self.pending_clipboard else {
                return Ok(InputResponse::default());
            };
            if *request != pending {
                return Ok(InputResponse::default());
            }
            self.pending_clipboard = None;
            let result = self.screen.complete_editor_clipboard(
                &mut self.fonts,
                widget,
                *request,
                result.clone(),
            );
            self.sync_host(host)?;
            return Ok(result.input);
        }
        if let InputEvent::Modifiers(modifiers) = event {
            self.modifiers = *modifiers;
        }
        if matches!(event, InputEvent::Focus(false)) {
            self.modifiers = Modifiers::default();
        }
        let Some(event) = self.viewport.input(event) else {
            return Ok(InputResponse::default());
        };
        let mut result = if self.screen.route_world_first(&event) {
            Default::default()
        } else {
            self.screen
                .input(&self.player.runtime.view(), &mut self.fonts, &event)?
        };
        let blocked = result.input.consumed || self.screen.captures_world_input();
        let world = self.screen.world_input(
            &self.player.runtime.view(),
            self.gpu.as_ref().map(|gpu| &gpu.views),
            &event,
            blocked,
        );
        result.input.consumed |= world.input.consumed;
        result.input.redraw |= world.input.redraw;
        result.cancel_controls |= world.cancel_controls;
        result.effects.extend(world.effects);
        if result.cancel_controls {
            let action = self.player.cancel_controls();
            self.player.submit_action(action)?;
            // The interaction's stop was submitted before its own effects (for
            // example a held movement button). Do not append another Off while
            // synchronizing the same capture transition afterward.
        }
        if let Some(text) = &result.text {
            self.clipboard(text, host);
        }
        self.apply(result.effects)?;
        self.control_capture = self.screen.control_capture();
        let mut response = result.input;
        // A modifier pressed after movement must not swallow its release.
        // Focus/modal transitions already cancel held state; releases remain
        // safe when the original press was consumed by a text field.
        if let InputEvent::Key {
            physical,
            logical,
            pressed: false,
            repeat: false,
        } = &event
        {
            if let Some(key) = binding_key(*physical, logical) {
                if let Some(action) = self.player.control_key(&key, false) {
                    self.player.submit_action(action)?;
                    response.consumed = true;
                    response.redraw = true;
                }
            }
        }
        if !response.consumed
            && !self.screen.captures_world_input()
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
            {
                if let Some(key) = binding_key(physical, &logical) {
                    if let Some(binding) = self.player.binding_for_key(&key) {
                        response.consumed = true;
                        if !repeat && pressed {
                            if matches!(binding, ClientCommandBinding::Control(_)) {
                                if let Some(action) = self.player.control_key(&key, true) {
                                    self.player.submit_action(action)?;
                                }
                            } else {
                                let activation = self
                                    .screen
                                    .activate_binding(&self.player.runtime.view(), binding)?;
                                self.apply(activation.effects)?;
                            }
                            response.redraw = true;
                        }
                    }
                }
            }
        }
        self.sync_host(host)?;
        Ok(response)
    }
    fn advance(
        &mut self,
        elapsed: Duration,
        host: &mut HostContext,
    ) -> std::result::Result<(), String> {
        if self.suspended {
            return Ok(());
        }
        self.visual_elapsed = self.visual_elapsed.saturating_add(elapsed);
        self.screen.world_texts.advance(elapsed);
        self.player.advance(elapsed);
        let isometric=self.player.runtime.view().map.and_then(|map|map.entities.iter().find(|e|e.is_player()))
            .is_some_and(|e|matches!(e.attributes.get("player_camera"),Some(rusterix::Value::PlayerCamera(camera)) if camera.is_isometric_2d()));
        if let Some(action) = self.player.set_isometric_controls(isometric) {
            self.player.submit_action(action)?;
        }
        let current = self.player.runtime.view().map.map(|m| m.id);
        self.services
            .update(&self.player.runtime.view(), self.revision.0, elapsed);
        // Every addressed region batch is drained once, including off-screen
        // regions. Services can decide which audio/world-text outputs to retain.
        let ids = self
            .player
            .runtime
            .project
            .regions
            .iter()
            .map(|r| r.map.id)
            .collect::<Vec<_>>();
        for id in ids {
            let events = self.player.runtime.take_events(&id);
            let world = self
                .player
                .runtime
                .view_region(&id)
                .ok_or("Event region is no longer in the session")?;
            if current == Some(id) {
                self.screen.receive(&world, &events);
            }
            self.services.region_events(id, &world, events.outputs());
        }
        self.sync_host(host)
    }
}
impl GpuApplication for PlayerApplication {
    fn title(&self) -> String {
        self.player.runtime.project.name.clone()
    }
    fn initial_size(&self) -> (u32, u32) {
        self.player.settings.window_size()
    }
    fn initialize(&mut self, host: &mut HostContext, images: &mut ImageRegistry) -> Result<()> {
        if self.gpu.is_some() {
            return Err(RenderError("Player GPU is already initialized".into()));
        }
        self.gpu = Some(PlayerGpu::new(images));
        self.sync_host(host).map_err(RenderError)
    }
    fn gpu_recreated(&mut self, host: &mut HostContext, images: &mut ImageRegistry) -> Result<()> {
        self.screen.invalidate_world_input();
        if let Some(gpu) = self
            .gpu
            .as_mut()
            .filter(|gpu| gpu.owner == images.owner_id())
        {
            self.screen
                .release_game_views(&mut gpu.views, images)
                .map_err(RenderError)?;
            for id in self.retired_views.drain(..) {
                gpu.views.remove(id, images)?;
            }
            gpu.sources.clear(images).map_err(RenderError)?;
        }
        self.gpu = Some(PlayerGpu::new(images));
        self.retired_views.clear();
        self.sync_host(host).map_err(RenderError)
    }
    fn resize(&mut self, host: &mut HostContext) {
        if let Err(error) = self.sync_host(host) {
            self.fail(host, error);
        }
    }
    fn input(&mut self, event: &InputEvent, host: &mut HostContext) -> InputResponse {
        match self.route(event, host) {
            Ok(response) => response,
            Err(error) => {
                self.fail(host, error);
                InputResponse {
                    consumed: true,
                    redraw: true,
                }
            }
        }
    }
    fn update(&mut self, host: &mut HostContext, elapsed: Duration) -> FrameRequest {
        if let Err(error) = self.advance(elapsed, host) {
            self.fail(host, error);
        }
        if self.suspended {
            FrameRequest::idle()
        } else {
            self.frames.request(
                self.visual_elapsed,
                self.player.settings.target_fps,
                self.services.wake_after(),
            )
        }
    }
    fn frame(&mut self, host: &mut HostContext, images: &mut ImageRegistry) -> Result<PaintList> {
        if let Some(error) = &self.failure {
            return Err(RenderError(error.clone()));
        }
        if self.suspended || host.physical_size.0 == 0 || host.physical_size.1 == 0 {
            return Ok(PaintList::default());
        }
        self.sync_host(host).map_err(RenderError)?;
        let gpu = self
            .gpu
            .as_mut()
            .ok_or_else(|| RenderError("Initialize player presentation before painting".into()))?;
        if gpu.owner != images.owner_id() {
            return Err(RenderError(
                "Player resources belong to a different presenter; call gpu_recreated".into(),
            ));
        }
        for id in self.retired_views.drain(..) {
            gpu.views.remove(id, images)?;
        }
        let world = self.player.runtime.view();
        self.screen
            .prepare_game_views(&world, &mut gpu.views, images)
            .map_err(RenderError)?;
        let mut source_frame = gpu.sources.begin_frame(images).map_err(RenderError)?;
        let paint = self
            .screen
            .paint(&world, &mut self.fonts, &mut source_frame)
            .map_err(RenderError)?;
        let mut list = PaintList::default();
        list.ops.push(PaintOp::PushClip(Rect::new(
            0.,
            0.,
            host.logical_size[0],
            host.logical_size[1],
        )));
        list.ops
            .push(PaintOp::PushTransform(self.viewport.transform()));
        list.ops.extend(paint.ops);
        list.ops.extend([PaintOp::PopTransform, PaintOp::PopClip]);
        self.frame_count += 1;
        Ok(list)
    }
    fn suspended(&mut self, host: &mut HostContext) {
        if self.suspended {
            return;
        }
        self.suspended = true;
        self.services.set_suspended(true);
        self.route(&InputEvent::Focus(false), host)
            .unwrap_or_else(|error| {
                self.fail(host, error);
                InputResponse::default()
            });
        self.resume_simulation = self.player.runtime.server.state == ServerState::Running;
        if self.resume_simulation {
            self.player.runtime.server.pause();
        }
        self.player.runtime.clock.reset_elapsed();
    }
    fn resumed(&mut self, host: &mut HostContext) {
        self.screen.resume_world_input();
        if self.suspended {
            self.services.set_suspended(false);
        }
        self.suspended = false;
        if self.resume_simulation {
            self.player.runtime.server.continue_instances();
            self.resume_simulation = false;
        }
        self.player.runtime.clock.reset_elapsed();
        if let Err(error) = self.sync_host(host) {
            self.fail(host, error);
        }
    }
    fn presented(&mut self, _host: &mut HostContext) {
        self.screen.acknowledge_world_frame(
            &self.player.runtime.view(),
            self.gpu.as_ref().map(|gpu| &gpu.views),
        );
    }
}
fn binding_key(physical: PhysicalKey, logical: &LogicalKey) -> Option<String> {
    use PhysicalKey::*;
    Some(match physical {
        Letter(letter) => (letter.to_ascii_lowercase() as char).to_string(),
        Digit(digit) => char::from(b'0' + digit.min(9)).to_string(),
        Function(number) => format!("f{number}"),
        Escape => "escape".into(),
        Enter => "enter".into(),
        Space => "space".into(),
        Tab => "tab".into(),
        Backspace => "backspace".into(),
        Delete => "delete".into(),
        Left => "left".into(),
        Right => "right".into(),
        Up => "up".into(),
        Down => "down".into(),
        Home => "home".into(),
        End => "end".into(),
        PageUp => "pageup".into(),
        PageDown => "pagedown".into(),
        _ => {
            if let LogicalKey::Character(key) = logical {
                key.to_ascii_lowercase()
            } else {
                return None;
            }
        }
    })
}

#[cfg(test)]
mod tests;
