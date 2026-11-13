//! Application host driven by a native display link rather than Winit.
//! The wrapper retains its layer; Rust alone acquires and presents drawables.
use super::*;
use renderkit::Result;
use web_time::{Duration, Instant};

pub struct ForeignApplicationHost<A: GpuApplication> {
    pub application: A,
    pub host: HostContext,
    pub presenter: GpuPresenter<'static>,
    dirty: bool,
    suspended: bool,
    presentations: u64,
    last_update: Instant,
    next_update: Option<Instant>,
}

impl<A: GpuApplication> ForeignApplicationHost<A> {
    pub fn new(
        mut application: A,
        mut presenter: GpuPresenter<'static>,
        density: f32,
    ) -> Result<Self> {
        let mut host = HostContext::new(presenter.surface.physical_size());
        host.resize(presenter.surface.physical_size(), density);
        application.initialize(&mut host, &mut presenter.images)?;
        let now = Instant::now();
        Ok(Self {
            application,
            host,
            presenter,
            dirty: true,
            suspended: false,
            presentations: 0,
            last_update: now,
            next_update: Some(now),
        })
    }

    pub fn request_redraw(&mut self) {
        self.dirty = true;
        self.next_update = Some(Instant::now());
    }

    pub fn input(&mut self, event: &InputEvent) -> InputResponse {
        let response = self.application.input(event, &mut self.host);
        self.dirty |= response.redraw;
        self.next_update = Some(Instant::now());
        response
    }

    /// Simulation/polling advances even if no drawable is available. Display
    /// callbacks faster than the requested cadence do not multiply CPU updates.
    pub fn update(&mut self) {
        if self.suspended {
            return;
        }
        let now = Instant::now();
        if self.next_update.is_some_and(|deadline| now >= deadline) {
            let request = self
                .application
                .update(&mut self.host, now - self.last_update);
            self.last_update = now;
            self.next_update = request
                .wake_after
                .map(|delay| now + delay.max(Duration::from_millis(1)));
            self.dirty |= request.redraw;
        }
    }

    pub fn present(&mut self) -> Result<Option<PresentResult>> {
        if self.suspended || !self.dirty {
            return Ok(None);
        }
        let app = &mut self.application;
        let host = &mut self.host;
        let result = self
            .presenter
            .present_with(host.density, [0., 0., 0., 1.], |images| {
                app.frame(host, images)
            })?;
        if result == PresentResult::Presented {
            self.dirty = false;
            self.presentations += 1;
            self.application.presented(&mut self.host);
        }
        Ok(Some(result))
    }

    pub fn presentations(&self) -> u64 {
        self.presentations
    }

    pub fn resize(&mut self, size: (u32, u32), density: f32) {
        let old = (self.host.physical_size, self.host.density);
        self.host.resize(size, density);
        if old == (self.host.physical_size, self.host.density) {
            return;
        }
        self.presenter.surface.resize(size.0, size.1);
        self.application.resize(&mut self.host);
        self.application.window_resized(&mut self.host);
        self.request_redraw();
    }

    pub fn suspend(&mut self) {
        if !self.suspended {
            self.suspended = true;
            self.application.suspended(&mut self.host);
            self.presenter.surface.suspend();
        }
    }

    pub fn resume(&mut self) {
        if self.suspended {
            self.suspended = false;
            self.presenter.surface.resume();
            self.application.resumed(&mut self.host);
            // Suppression must not feed an artificial elapsed-time catch-up.
            self.last_update = Instant::now();
            self.request_redraw();
        }
    }

    /// CPU application and native input identities survive device replacement.
    /// Retain the old backing layer until this returns successfully.
    pub fn replace_presenter(&mut self, mut presenter: GpuPresenter<'static>) -> Result<()> {
        self.application
            .gpu_recreated(&mut self.host, &mut presenter.images)?;
        if self.suspended {
            presenter.surface.suspend();
        }
        self.presenter = presenter;
        self.resize(self.presenter.surface.physical_size(), self.host.density);
        self.request_redraw();
        Ok(())
    }
}
