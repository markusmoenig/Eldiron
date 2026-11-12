//! Display-link driven host for the common player. No Winit, VM swapping or
//! destination pixels. The native wrapper owns its layer and services requests.
use super::*;
use theframework::platform::{GpuPresenter, PresentResult};

pub struct ForeignPlayerHost {
    pub presenter: GpuPresenter<'static>,
    pub host: HostContext,
    pub application: Option<PlayerApplication>,
    source: Vec<u8>,
    dirty: bool,
    suspended: bool,
    presentations: u64,
}
impl ForeignPlayerHost {
    pub fn new(presenter: GpuPresenter<'static>, size: (u32, u32), density: f32) -> Self {
        let mut host = HostContext::new(size);
        host.resize(size, density);
        Self {
            presenter,
            host,
            application: None,
            source: Vec::new(),
            dirty: true,
            suspended: false,
            presentations: 0,
        }
    }
    /// Decode/migrate a runtime copy; the saved player document remains the
    /// exact authored input, including archive entries and legacy JSON bytes.
    /// Failed loads preserve the live session, registry and pending input owner.
    pub fn load(&mut self, source: &[u8], debug: bool) -> Result<()> {
        let mut project = shared::project_io::decode_project(source).map_err(RenderError)?;
        project.migrate_default_ruleset();
        let _ = project.sync_ruleset_items();
        self.install(
            source.to_vec(),
            project,
            debug,
            Box::new(AudioServices::new(StandardServices)),
        )
    }
    fn install(
        &mut self,
        source: Vec<u8>,
        project: Project,
        debug: bool,
        services: Box<dyn PlayerServices>,
    ) -> Result<()> {
        let mut next = PlayerApplication::new(project, debug, services).map_err(RenderError)?;
        let mut images = ImageRegistry::new(self.presenter.context());
        let mut host = HostContext::new(self.host.physical_size);
        host.resize(self.host.physical_size, self.host.density);
        next.initialize(&mut host, &mut images)?;
        if self.suspended {
            next.suspended(&mut host);
        }
        // Old images and clipboard/IME identities retire together. A host must
        // discard queued old platform requests when its document is replaced.
        self.application = Some(next);
        self.presenter.images = images;
        self.host = host;
        self.source = source;
        self.dirty = true;
        Ok(())
    }
    pub fn source(&self) -> &[u8] {
        &self.source
    }
    pub fn presentations(&self) -> u64 {
        self.presentations
    }
    pub fn resize(&mut self, size: (u32, u32), density: f32) {
        self.host.resize(size, density);
        self.presenter.surface.resize(size.0, size.1);
        if let Some(app) = &mut self.application {
            app.resize(&mut self.host);
        }
        self.dirty = true;
    }
    pub fn native_text_snapshot(&mut self) -> Option<theframework::text_edit::NativeTextSnapshot> {
        self.application
            .as_mut()?
            .native_text_snapshot(&mut self.host)
    }
    pub fn native_text_query(
        &mut self,
        request: &theframework::text_edit::NativeTextRequest,
    ) -> Option<theframework::text_edit::NativeTextAnswer> {
        self.application
            .as_mut()?
            .native_text_query(&mut self.host, request)
    }
    pub fn input(&mut self, event: &InputEvent) -> InputResponse {
        let response = self
            .application
            .as_mut()
            .map_or(InputResponse::default(), |app| {
                app.input(event, &mut self.host)
            });
        self.dirty |= response.redraw;
        response
    }
    pub fn update(&mut self, elapsed: Duration) -> FrameRequest {
        let request = self
            .application
            .as_mut()
            .map_or(FrameRequest::idle(), |app| {
                app.update(&mut self.host, elapsed)
            });
        self.dirty |= request.redraw;
        request
    }
    /// Update simulation separately, including when drawable acquisition defers.
    /// None means an unchanged/suspended host made no presentation attempt.
    pub fn present(&mut self) -> Result<Option<PresentResult>> {
        if let Some(error) = self.application.as_ref().and_then(|app| app.failure()) {
            return Err(RenderError(error.into()));
        }
        if self.suspended || !self.dirty {
            return Ok(None);
        }
        let application = &mut self.application;
        let host = &mut self.host;
        let result = self
            .presenter
            .present_with(host.density, [0., 0., 0., 1.], |images| {
                if let Some(app) = application {
                    app.frame(host, images)
                } else {
                    Ok(PaintList::default())
                }
            })?;
        if result == PresentResult::Presented {
            self.presentations += 1;
            self.dirty = false;
            if let Some(app) = &mut self.application {
                app.presented(&mut self.host);
            }
        }
        Ok(Some(result))
    }
    pub fn suspend(&mut self) {
        if self.suspended {
            return;
        }
        self.suspended = true;
        self.presenter.surface.suspend();
        if let Some(app) = &mut self.application {
            app.suspended(&mut self.host);
        }
    }
    pub fn resume(&mut self) {
        if !self.suspended {
            return;
        }
        self.suspended = false;
        self.presenter.surface.resume();
        if let Some(app) = &mut self.application {
            app.resumed(&mut self.host);
        }
        self.dirty = true;
    }
    /// Surface/device replacement preserves the exact CPU session/controller.
    /// The caller keeps the previous backing layer alive until this returns.
    pub fn replace_presenter(&mut self, mut presenter: GpuPresenter<'static>) -> Result<()> {
        self.host
            .resize(presenter.surface.physical_size(), self.host.density);
        if self.suspended {
            presenter.surface.suspend();
        }
        if let Some(app) = &mut self.application {
            app.gpu_recreated(&mut self.host, &mut presenter.images)?;
        }
        self.presenter = presenter;
        self.dirty = true;
        Ok(())
    }
}

#[cfg(all(test, target_os = "macos"))]
#[path = "foreign_tests.rs"]
mod tests;
