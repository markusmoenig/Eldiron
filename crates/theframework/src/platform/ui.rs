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
    pub ui: TheUI,
    pub context: TheContext,
    pub requests: Vec<HostRequest>,
}
impl UiSession {
    pub fn new(logical_size: (usize, usize), density: f32) -> Self {
        let mut context = TheContext::new(logical_size.0, logical_size.1, density);
        context.ui.command_paint = true;
        let mut ui = TheUI::new();
        ui.init(&mut context);
        Self {
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
        self.context.ui.relayout = true;
        self.context.ui.redraw_all = true;
    }
    pub fn input(&mut self, event: &InputEvent) -> InputResponse {
        self.ui.prepare_paint_layout(&mut self.context);
        self.ui.dispatch_input(event, &mut self.context)
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
    fn initialize(&mut self, ui: &mut UiSession, images: &mut UiFrameImages<'_>) -> Result<()>;
    fn gpu_recreated(
        &mut self,
        _ui: &mut UiSession,
        _images: &mut UiFrameImages<'_>,
    ) -> Result<()> {
        Ok(())
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
}
impl<A: UiApplication> UiApplicationAdapter<A> {
    pub fn new(application: A) -> Self {
        let size = application.initial_size();
        Self {
            application,
            session: UiSession::new((size.0 as usize, size.1 as usize), 1.),
            images: UiImageCache::default(),
            text_target: None,
        }
    }
    fn platform_state(&mut self, host: &mut HostContext) {
        host.requests.append(&mut self.session.requests);
        host.cursor = self.session.context.cursor_icon();
        host.cursor_visible = self.session.context.cursor_visible();
        // The old widget adapter exposes its bounds. Native EditorView users
        // provide their precise caret/preedit area directly on HostContext.
        let focus = if self
            .session
            .ui
            .focus_widget_supports_text_input(&mut self.session.context)
        {
            self.session.context.ui.focus.clone()
        } else {
            None
        };
        host.text_input = focus.and_then(|id| {
            let area = self
                .session
                .ui
                .get_widget_abs(None, Some(&id.uuid))
                .map(|w| crate::theui::paint::bounds(w.dim()))?;
            let target = if let Some((old, target)) = &self.text_target {
                if *old == id.uuid {
                    *target
                } else {
                    TextInputTarget::new()
                }
            } else {
                TextInputTarget::new()
            };
            self.text_target = Some((id.uuid, target));
            Some(TextInputState { target, area })
        });
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
    fn initialize(&mut self, host: &mut HostContext, images: &mut ImageRegistry) -> Result<()> {
        self.resize(host);
        self.application
            .initialize(&mut self.session, &mut self.images.frame(images))?;
        self.session.context.ui.relayout = true;
        self.platform_state(host);
        Ok(())
    }
    fn gpu_recreated(&mut self, host: &mut HostContext, images: &mut ImageRegistry) -> Result<()> {
        self.resize(host);
        self.application
            .gpu_recreated(&mut self.session, &mut self.images.frame(images))?;
        self.session.context.ui.relayout = true;
        self.platform_state(host);
        Ok(())
    }
    fn resize(&mut self, host: &mut HostContext) {
        self.session.resize(host.physical_size, host.density);
    }
    fn input(&mut self, event: &InputEvent, host: &mut HostContext) -> InputResponse {
        let addressed;
        let event = match event {
            InputEvent::TextFor { target, text } => {
                if self.text_target.as_ref().map(|(_, target)| target) != Some(target) {
                    return InputResponse::default();
                }
                addressed = InputEvent::Text(text.clone());
                &addressed
            }
            InputEvent::CompositionFor {
                target,
                text,
                cursor,
            } => {
                if self.text_target.as_ref().map(|(_, target)| target) != Some(target) {
                    return InputResponse::default();
                }
                addressed = InputEvent::Composition {
                    text: text.clone(),
                    cursor: *cursor,
                };
                &addressed
            }
            _ => event,
        };
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
        request.redraw |= self.session.ui.needs_paint(&self.session.context);
        // Compatibility UI timers/file requesters still need polling. Generic
        // hosts may sleep indefinitely when their application requests no wake.
        request.wake_after = Some(
            request
                .wake_after
                .unwrap_or(web_time::Duration::from_millis(50))
                .min(web_time::Duration::from_millis(50)),
        );
        self.platform_state(host);
        request
    }
    fn frame(&mut self, host: &mut HostContext, images: &mut ImageRegistry) -> Result<PaintList> {
        let list = self
            .application
            .frame(&mut self.session, &mut self.images.frame(images))?;
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
