//! Optional TheUI application adapter. Window/surface hosts do not own widgets.
use super::*;
use crate::{
    paint::PaintList,
    prelude::*,
    render_wgpu::{UiFrameImages, UiImageCache},
    theui::paint::{UiImageResources, UiPaintError},
};
use renderkit::{ImageRegistry, RenderError, Result};
pub struct UiSession {
    pub wake: ApplicationWake,
    pub ui: TheUI,
    pub context: TheContext,
    pub requests: Vec<HostRequest>,
}
impl UiSession {
    pub fn new(logical_size: (usize, usize), density: f32) -> Self {
        let profile = std::env::var_os("ELDIRON_PROFILE_STARTUP")
            .is_some()
            .then(web_time::Instant::now);
        let mut context = TheContext::new(logical_size.0, logical_size.1, density);
        if let Some(start) = profile {
            eprintln!(
                "Native startup UI context: {:.2}ms",
                start.elapsed().as_secs_f64() * 1000.
            );
        }
        context.ui.command_paint = true;
        let mut ui = TheUI::new();
        ui.init(&mut context);
        Self {
            wake: Default::default(),
            ui,
            context,
            requests: Vec::new(),
        }
    }

    pub fn resize(&mut self, physical: (u32, u32), density: f32) {
        let density = if density.is_finite() && density > 0. {
            density
        } else {
            1.
        };
        self.context.width = (physical.0 as f32 / density).round() as usize;
        self.context.height = (physical.1 as f32 / density).round() as usize;
        self.context.scale_factor = density;
        self.context.framebuffer_width = physical.0 as usize;
        self.context.framebuffer_height = physical.1 as usize;
        self.context.ui.invalidate_layout();
        self.context.ui.invalidate_paint();
    }
    pub fn input(&mut self, event: &InputEvent) -> InputResponse {
        self.ui.prepare_paint_layout(&mut self.context);
        self.ui.dispatch_input(event, &mut self.context)
    }
    /// Snapshot only the foremost shared editor; modal/focus changes retire its
    /// native input target before a host can issue a range-based operation.
    pub fn native_text_snapshot(&mut self) -> Option<crate::text_edit::NativeTextSnapshot> {
        self.ui.prepare_paint_layout(&mut self.context);
        self.ui.process_events(&mut self.context);
        if !self.ui.synchronize_editor_scope(&mut self.context) {
            return None;
        }
        let id = self.context.ui.focus.clone()?;
        self.ui
            .get_widget_abs(None, Some(&id.uuid))?
            .editor_native_text_snapshot(&mut self.context)
    }
    pub fn native_text_query(
        &mut self,
        request: &crate::text_edit::NativeTextRequest,
    ) -> Option<crate::text_edit::NativeTextAnswer> {
        self.ui.prepare_paint_layout(&mut self.context);
        self.ui.process_events(&mut self.context);
        if !self.ui.synchronize_editor_scope(&mut self.context) {
            return None;
        }
        let id = self.context.ui.focus.clone()?;
        let transform = self
            .context
            .ui
            .input_transforms
            .get(&id.uuid)
            .copied()
            .unwrap_or_default();
        let local = request.map_hit(transform.inverse()?);
        self.ui
            .get_widget_abs(None, Some(&id.uuid))?
            .editor_native_text_query(&local, &mut self.context)
            .map(|answer| answer.transformed(transform))
    }
    pub fn paint(
        &mut self,
        images: &mut dyn UiImageResources,
    ) -> std::result::Result<PaintList, UiPaintError> {
        self.ui.paint_commands(&mut self.context, images)
    }
}

/// UI-specific callbacks are optional; a game can implement GpuApplication
/// directly and own only its runtime, screen model and paint/image services.
pub trait UiApplication {
    fn title(&self) -> String {
        "Eldiron".into()
    }
    fn initial_size(&self) -> (u32, u32) {
        (1200, 700)
    }
    fn window_configuration(&self) -> WindowConfiguration {
        WindowConfiguration::default()
    }
    fn document_edited(&self) -> bool {
        false
    }
    fn window_moved(&mut self, _position: (i32, i32), _ui: &mut UiSession) {}
    fn window_resized(&mut self, _ui: &mut UiSession) {}
    fn initialize(&mut self, ui: &mut UiSession, images: &mut UiFrameImages<'_>) -> Result<()>;
    fn gpu_recreated(
        &mut self,
        _ui: &mut UiSession,
        _images: &mut UiFrameImages<'_>,
    ) -> Result<()> {
        Ok(())
    }
    /// Embedded application controllers receive neutral input before widget
    /// wheel conversion; returning None delegates to the normal UI path.
    fn raw_input(&mut self, _event: &InputEvent, _ui: &mut UiSession) -> Option<InputResponse> {
        None
    }
    fn application_text_input(&mut self, _ui: &mut UiSession) -> Option<TextInputState> {
        None
    }
    fn application_text_snapshot(
        &mut self,
        _ui: &mut UiSession,
    ) -> Option<crate::text_edit::NativeTextSnapshot> {
        None
    }
    fn application_text_query(
        &mut self,
        _ui: &mut UiSession,
        _request: &crate::text_edit::NativeTextRequest,
    ) -> Option<crate::text_edit::NativeTextAnswer> {
        None
    }
    fn input(&mut self, event: &InputEvent, ui: &mut UiSession) -> InputResponse {
        ui.input(event)
    }
    fn update(&mut self, ui: &mut UiSession, _elapsed: web_time::Duration) -> FrameRequest {
        FrameRequest {
            redraw: ui.ui.update(&mut ui.context),
            wake_after: None,
        }
    }
    fn frame(&mut self, ui: &mut UiSession, images: &mut UiFrameImages<'_>) -> Result<PaintList> {
        ui.paint(images).map_err(|e| RenderError(e.to_string()))
    }
    fn presented(&mut self, _ui: &mut UiSession) {}
    fn close_requested(&mut self, _ui: &mut UiSession) -> bool {
        true
    }
    fn suspended(&mut self, _ui: &mut UiSession) {}
    fn resumed(&mut self, _ui: &mut UiSession) {}
}
pub struct UiApplicationAdapter<A> {
    pub application: A,
    pub session: UiSession,
    images: UiImageCache,
    text_target: Option<(theframework_core::Uuid, TextInputTarget)>,
    native_editor: bool,
    application_text: bool,
}
impl<A: UiApplication> UiApplicationAdapter<A> {
    pub fn new(application: A) -> Self {
        let size = application.initial_size();
        Self {
            application,
            session: UiSession::new((size.0 as usize, size.1 as usize), 1.),
            images: UiImageCache::default(),
            text_target: None,
            native_editor: false,
            application_text: false,
        }
    }
    fn platform_state(&mut self, host: &mut HostContext) {
        self.session.ui.process_events(&mut self.session.context);
        self.session
            .ui
            .synchronize_editor_scope(&mut self.session.context);
        host.requests.append(&mut self.session.requests);
        host.requests.extend(self.session.context.ui.image_clipboard.drain(..).map(HostRequest::ClipboardImage));
        host.cursor = self.session.context.cursor_icon();
        host.cursor_visible = self.session.context.cursor_visible();
        for request in self.session.context.ui.editor_clipboard.drain(..) {
            host.requests.push(match request {
                crate::text_edit::EditorClipboard::Read { request } => {
                    HostRequest::ClipboardRead { request }
                }
                crate::text_edit::EditorClipboard::Write { text, cut } => {
                    HostRequest::ClipboardWrite { text, request: cut }
                }
            });
        }
        self.native_editor = false;
        let focus = self.session.context.ui.focus.clone();
        host.text_input = focus.and_then(|id| {
            let widget = self.session.ui.get_widget_abs(None, Some(&id.uuid))?;
            if !widget.supports_text_input() {
                return None;
            }
            if widget.uses_native_editor() {
                self.native_editor = true;
                let mut input = widget.editor_input_state(&mut self.session.context)?;
                let transform = self
                    .session
                    .context
                    .ui
                    .input_transforms
                    .get(&id.uuid)
                    .copied()
                    .unwrap_or_default();
                let a = transform.point([input.area.x, input.area.y]);
                let b = transform.point([
                    input.area.x + input.area.width,
                    input.area.y + input.area.height,
                ]);
                input.area = crate::paint::Rect::new(
                    a[0].min(b[0]),
                    a[1].min(b[1]),
                    (a[0] - b[0]).abs(),
                    (a[1] - b[1]).abs(),
                )
                .intersection(crate::paint::Rect::new(
                    0.,
                    0.,
                    self.session.context.width as f32,
                    self.session.context.height as f32,
                ));
                if input.area.width <= 0. || input.area.height <= 0. {
                    return None;
                }
                self.text_target = Some((id.uuid, input.target));
                return Some(input);
            }
            let area = crate::theui::paint::bounds(widget.dim());
            let target = self
                .text_target
                .filter(|(old, _)| *old == id.uuid)
                .map(|(_, target)| target)
                .unwrap_or_default();
            self.text_target = Some((id.uuid, target));
            Some(TextInputState { target, area })
        });
        self.application_text = false;
        if let Some(input) = self.application.application_text_input(&mut self.session) {
            self.application_text = true;
            self.native_editor = true;
            self.text_target = Some((theframework_core::Uuid::nil(), input.target));
            host.text_input = Some(input);
        }
        if host.text_input.is_none() {
            self.text_target = None;
        }
    }
}
impl<A: UiApplication> GpuApplication for UiApplicationAdapter<A> {
    fn title(&self) -> String {
        self.application.title()
    }
    fn initial_size(&self) -> (u32, u32) {
        self.application.initial_size()
    }
    fn window_configuration(&self) -> WindowConfiguration {
        self.application.window_configuration()
    }
    fn document_edited(&self) -> bool {
        self.application.document_edited()
    }
    fn window_moved(&mut self, position: (i32, i32), host: &mut HostContext) {
        self.application.window_moved(position, &mut self.session);
        self.platform_state(host);
    }
    fn window_resized(&mut self, host: &mut HostContext) {
        self.application.window_resized(&mut self.session);
        self.platform_state(host);
    }
    fn initialize(&mut self, host: &mut HostContext, images: &mut ImageRegistry) -> Result<()> {
        self.session.wake = host.wake.clone();
        self.resize(host);
        self.application
            .initialize(&mut self.session, &mut self.images.frame(images))?;
        self.session.context.ui.invalidate_layout();
        self.platform_state(host);
        Ok(())
    }
    fn gpu_recreated(&mut self, host: &mut HostContext, images: &mut ImageRegistry) -> Result<()> {
        self.resize(host);
        self.application
            .gpu_recreated(&mut self.session, &mut self.images.frame(images))?;
        self.session.context.ui.invalidate_layout();
        self.platform_state(host);
        Ok(())
    }
    fn resize(&mut self, host: &mut HostContext) {
        self.session.resize(host.physical_size, host.density);
    }
    fn native_text_snapshot(
        &mut self,
        host: &mut HostContext,
    ) -> Option<crate::text_edit::NativeTextSnapshot> {
        let source = if self.application_text {
            self.application.application_text_snapshot(&mut self.session)
        } else {
            self.session.native_text_snapshot()
        };
        self.platform_state(host);
        source
    }
    fn native_text_query(
        &mut self,
        host: &mut HostContext,
        request: &crate::text_edit::NativeTextRequest,
    ) -> Option<crate::text_edit::NativeTextAnswer> {
        let answer = if self.application_text {
            self.application.application_text_query(&mut self.session, request)
        } else {
            self.session.native_text_query(request)
        };
        self.platform_state(host);
        answer
    }
    fn input(&mut self, event: &InputEvent, host: &mut HostContext) -> InputResponse {
        let addressed;
        let event = match event {
            InputEvent::TextFor { target, text } => {
                if self.text_target.as_ref().map(|(_, target)| target) != Some(target) {
                    return InputResponse::default();
                }
                if self.native_editor {
                    event
                } else {
                    addressed = InputEvent::Text(text.clone());
                    &addressed
                }
            }
            InputEvent::CompositionFor {
                target,
                text,
                cursor,
            } => {
                if self.text_target.as_ref().map(|(_, target)| target) != Some(target) {
                    return InputResponse::default();
                }
                if self.native_editor {
                    event
                } else {
                    addressed = InputEvent::Composition {
                        text: text.clone(),
                        cursor: *cursor,
                    };
                    &addressed
                }
            }
            InputEvent::NativeText { target, .. } => {
                if !self.native_editor
                    || self.text_target.as_ref().map(|(_, owner)| owner) != Some(target)
                {
                    return InputResponse::default();
                }
                event
            }
            _ => event,
        };
        if let Some(response) = self.application.raw_input(event, &mut self.session) {
            self.platform_state(host);
            return response;
        }
        // The legacy widget adapter expects wheel ticks converted to its pixel
        // convention. Neutral application inputs keep unscaled line units.
        let legacy;
        let event = if let InputEvent::Scroll {
            delta,
            precise: false,
        } = event
        {
            legacy = InputEvent::Scroll {
                delta: [delta[0] * 20., delta[1] * 20.],
                precise: false,
            };
            &legacy
        } else {
            event
        };
        let response = self.application.input(event, &mut self.session);
        self.platform_state(host);
        response
    }
    fn update(&mut self, host: &mut HostContext, elapsed: web_time::Duration) -> FrameRequest {
        crate::platform_services::service_files(&mut self.session.context.ui);
        let mut request = self.application.update(&mut self.session, elapsed);
        self.platform_state(host);
        request.redraw |= self.session.ui.needs_paint(&self.session.context);
        if let Some(due) = self.session.ui.next_wake_after(&self.session.context) {
            request.wake_after = Some(request.wake_after.map_or(due, |app| app.min(due)));
        }
        request
    }
    fn frame(&mut self, host: &mut HostContext, images: &mut ImageRegistry) -> Result<PaintList> {
        let mut resources = self.images.frame(images);
        let list = self.application.frame(&mut self.session, &mut resources)?;
        // This sees the assembled application list, including retained view and
        // overlay contributions; individual UI subtrees must not retire images.
        resources
            .finish_frame(&list)
            .map_err(|e| RenderError(e.to_string()))?;
        self.session.context.ui.scene_completed();
        self.platform_state(host);
        Ok(list)
    }
    fn presented(&mut self, host: &mut HostContext) {
        self.application.presented(&mut self.session);
        self.platform_state(host);
    }
    fn close_requested(&mut self, host: &mut HostContext) -> bool {
        let close = self.application.close_requested(&mut self.session);
        self.platform_state(host);
        close
    }
    fn suspended(&mut self, host: &mut HostContext) {
        self.application.suspended(&mut self.session);
        self.platform_state(host);
    }
    fn resumed(&mut self, host: &mut HostContext) {
        self.application.resumed(&mut self.session);
        self.platform_state(host);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Application;
    impl UiApplication for Application {
        fn initialize(&mut self, _: &mut UiSession, _: &mut UiFrameImages<'_>) -> Result<()> {
            Ok(())
        }
    }
    #[test]
    fn legacy_ui_adapter_rejects_callbacks_for_previous_focus_tokens() {
        let mut adapter = UiApplicationAdapter::new(Application);
        let mut host = HostContext::new((400, 120));
        let a = TheId::named("A");
        let b = TheId::named("B");
        let mut row = TheHLayout::new(TheId::named("Editors"));
        row.widgets()
            .push(Box::new(TheTextLineEdit::new(a.clone())));
        row.widgets()
            .push(Box::new(TheTextLineEdit::new(b.clone())));
        adapter.session.ui.canvas.set_layout(row);
        adapter
            .session
            .ui
            .prepare_paint_layout(&mut adapter.session.context);
        adapter.session.context.ui.set_focus(&a);
        adapter.platform_state(&mut host);
        let old = host.text_input.unwrap().target;
        assert!(
            adapter
                .input(
                    &InputEvent::TextFor {
                        target: old,
                        text: "First".into()
                    },
                    &mut host
                )
                .consumed
        );
        assert_eq!(
            adapter.session.ui.get_text_line_edit("A").unwrap().text(),
            "First"
        );
        adapter.session.context.ui.set_focus(&b);
        adapter.platform_state(&mut host);
        let current = host.text_input.unwrap().target;
        assert_ne!(old, current);
        assert!(
            !adapter
                .input(
                    &InputEvent::TextFor {
                        target: old,
                        text: "stale".into()
                    },
                    &mut host
                )
                .consumed
        );
        assert_eq!(
            adapter.session.ui.get_text_line_edit("B").unwrap().text(),
            ""
        );
        assert!(
            adapter
                .input(
                    &InputEvent::TextFor {
                        target: current,
                        text: "Second".into()
                    },
                    &mut host
                )
                .consumed
        );
        adapter.session.context.ui.clear_focus();
        adapter.platform_state(&mut host);
        assert!(host.text_input.is_none());
        adapter.session.context.ui.set_focus(&b);
        adapter.platform_state(&mut host);
        assert_ne!(host.text_input.unwrap().target, current);
        assert!(
            !adapter
                .input(
                    &InputEvent::CompositionFor {
                        target: current,
                        text: "old".into(),
                        cursor: None
                    },
                    &mut host
                )
                .consumed
        );
        assert_eq!(
            adapter.session.ui.get_text_line_edit("B").unwrap().text(),
            "Second"
        );
    }
}

#[cfg(test)]
#[test]
fn window_metadata_and_user_events_survive_ui_metrics_reconstruction_without_persisting_it() {
    struct WindowApp {
        events: Vec<(i32, i32)>,
    }
    impl UiApplication for WindowApp {
        fn initialize(&mut self, _: &mut UiSession, _: &mut UiFrameImages<'_>) -> Result<()> {
            Ok(())
        }
        fn window_configuration(&self) -> WindowConfiguration {
            WindowConfiguration {
                minimum_size: Some((1200, 720)),
                position: Some((-5, 7)),
                icon: Some((vec![255; 4], 1, 1)),
            }
        }
        fn document_edited(&self) -> bool {
            true
        }
        fn window_moved(&mut self, p: (i32, i32), _: &mut UiSession) {
            self.events.push(p);
        }
        fn window_resized(&mut self, ui: &mut UiSession) {
            self.events
                .push((ui.context.width as i32, ui.context.height as i32));
        }
    }
    let mut adapter = UiApplicationAdapter::new(WindowApp { events: Vec::new() });
    let config = GpuApplication::window_configuration(&adapter);
    assert_eq!(config.minimum_size, Some((1200, 720)));
    assert_eq!(config.position, Some((-5, 7)));
    assert_eq!(config.icon, Some((vec![255; 4], 1, 1)));
    assert!(GpuApplication::document_edited(&adapter));
    let mut host = HostContext::new((800, 600));
    host.resize((800, 600), 1.25);
    GpuApplication::resize(&mut adapter, &mut host);
    assert!(
        adapter.application.events.is_empty(),
        "restoring GPU/UI metrics must not persist a user window preference"
    );
    GpuApplication::window_moved(&mut adapter, (-4, 8), &mut host);
    GpuApplication::window_resized(&mut adapter, &mut host);
    assert_eq!(adapter.application.events, vec![(-4, 8), (640, 480)]);
}
