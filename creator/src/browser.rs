//! Async browser bootstrap around the same Creator application and GPU host.
//! This does not select the legacy Winit/Pixels application or Canvas2D output.
use crate::native_application::NativeCreatorApplication;
use renderkit::{ImageRegistry, RenderError, Result};
use theframework::{paint::PaintList, platform::*, prelude::*};
use web_time::Duration;
mod files;
fn status(state: &str, message: Option<&str>) {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    if let Some(root) = document.document_element() {
        let _ = root.set_attribute("data-eldiron-state", state);
    }
    let panel = document.get_element_by_id("eldiron-status").or_else(|| {
        let panel = document.create_element("div").ok()?;
        panel.set_id("eldiron-status");
        let _ = panel.set_attribute("role", "status");
        let _ = panel.set_attribute("style", "position:fixed;inset:0;display:grid;place-items:center;background:#18181b;color:#e4e4e7;font:16px system-ui;padding:2rem;z-index:1;white-space:pre-wrap");
        document.body()?.append_child(&panel).ok()?;
        Some(panel)
    });
    if let Some(panel) = panel {
        panel.set_text_content(message);
        if message.is_none() {
            panel.remove();
        }
    }
}
fn report(error: impl std::fmt::Display) {
    let message = format!("Cannot run Eldiron Creator: {error}");
    web_sys::console::error_1(&message.clone().into());
    status("error", Some(&message));
}
pub fn start() {
    std::panic::set_hook(Box::new(|info| {
        console_error_panic_hook::hook(info);
        report("The editor stopped unexpectedly. See the browser console for details.");
    }));
    status("loading", Some("Loading Creator…"));
    wasm_bindgen_futures::spawn_local(async {
        if let Err(error) = load().await {
            report(error);
        }
    });
}
async fn load() -> std::result::Result<(), String> {
    let window = web_sys::window().ok_or("Browser window unavailable")?;
    let document = window.document().ok_or("Browser document unavailable")?;
    let query = web_sys::UrlSearchParams::new_with_str(
        &window.location().search().map_err(|e| format!("{e:?}"))?,
    )
    .map_err(|e| format!("{e:?}"))?;
    let initial = if let Some(path) = query.get("project").filter(|path| !path.trim().is_empty()) {
        let bytes = crate::host_services::fetch_bytes(&path, 1024 * 1024 * 1024).await?;
        let name = path
            .split('?')
            .next()
            .unwrap_or(&path)
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or("Project.eldiron")
            .to_owned();
        Some((bytes, name))
    } else {
        None
    };
    let _ = crate::i18n::select_system_locales();
    let style = document
        .create_element("style")
        .map_err(|e| format!("{e:?}"))?;
    style.set_text_content(Some("html,body{margin:0;width:100%;height:100%;overflow:hidden;background:#18181b}canvas{display:block;width:100vw!important;height:100vh!important;outline:none}"));
    document
        .body()
        .ok_or("Browser document body unavailable")?
        .append_child(&style)
        .map_err(|e| format!("{e:?}"))?;
    let mut application = NativeCreatorApplication::new(vec!["eldiron-creator-web".into()], false);
    application.host_enable_document_io();
    let files = files::Files::new().map_err(|e| format!("{e:?}"))?;
    status("starting", Some("Starting graphics…"));
    winit::run(Box::new(BrowserCreator {
        app: UiApplicationAdapter::new(application),
        initial,
        files,
        presentations: 0,
    }))
    .map_err(|e| e.to_string())
}
struct BrowserCreator {
    app: UiApplicationAdapter<NativeCreatorApplication>,
    initial: Option<(Vec<u8>, String)>,
    files: files::Files,
    presentations: u64,
}
impl GpuApplication for BrowserCreator {
    fn host_failed(&mut self, error: &RenderError) {
        report(error);
    }
    fn title(&self) -> String {
        self.app.title()
    }
    fn initial_size(&self) -> (u32, u32) {
        self.app.initial_size()
    }
    fn window_configuration(&self) -> WindowConfiguration {
        self.app.window_configuration()
    }
    fn document_edited(&self) -> bool {
        self.app.document_edited()
    }
    fn initialize(&mut self, host: &mut HostContext, images: &mut ImageRegistry) -> Result<()> {
        self.app.initialize(host, images)?;
        if let Some((bytes, name)) = self.initial.take() {
            // Initialization queues the default New-project chooser. Drain it
            // before the addressed byte load dismisses the outgoing dialog.
            self.app.update(host, Duration::ZERO);
            let document = self
                .app
                .application
                .host_document_info()
                .ok_or_else(|| RenderError("No initial document".into()))?
                .document;
            self.app
                .application
                .host_load_document(
                    &mut self.app.session,
                    document,
                    &bytes,
                    Some(name.into()),
                    true,
                )
                .map_err(RenderError)?;
        }
        Ok(())
    }
    fn gpu_recreated(&mut self, host: &mut HostContext, images: &mut ImageRegistry) -> Result<()> {
        self.app.gpu_recreated(host, images)
    }
    fn resize(&mut self, host: &mut HostContext) {
        self.app.resize(host);
    }
    fn native_text_snapshot(
        &mut self,
        host: &mut HostContext,
    ) -> Option<theframework::text_edit::NativeTextSnapshot> {
        self.app.native_text_snapshot(host)
    }
    fn native_text_query(
        &mut self,
        host: &mut HostContext,
        request: &theframework::text_edit::NativeTextRequest,
    ) -> Option<theframework::text_edit::NativeTextAnswer> {
        self.app.native_text_query(host, request)
    }
    fn input(&mut self, event: &InputEvent, host: &mut HostContext) -> InputResponse {
        self.app.input(event, host)
    }
    fn update(&mut self, host: &mut HostContext, elapsed: Duration) -> FrameRequest {
        self.files.apply(&mut self.app);
        let mut request = self.app.update(host, elapsed);
        self.files.set_dirty(self.app.document_edited());
        for (kind, document) in self.app.application.host_document_requests() {
            if let Err(error) = self.files.request(kind, document, &mut self.app) {
                self.app
                    .session
                    .context
                    .ui
                    .send(TheEvent::SetStatusText(TheId::empty(), error));
            }
            request.redraw = true;
        }
        for action in &host.requests {
            if let HostRequest::Fail(error) = action {
                report(error);
            }
        }
        request
    }
    fn frame(&mut self, host: &mut HostContext, images: &mut ImageRegistry) -> Result<PaintList> {
        let paint = self.app.frame(host, images).inspect_err(|e| report(e))?;
        if let Some(root) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.document_element())
        {
            let _ = root.set_attribute(
                "data-eldiron-readback-bytes",
                &images.context().transfers().readback_bytes.to_string(),
            );
            let transfers = images.context().transfers();
            let _ = root.set_attribute("data-eldiron-asset-bake-readback-bytes", &transfers.asset_bake_readback_bytes.to_string());
            let _ = root.set_attribute("data-eldiron-asset-bake-readback-requests", &transfers.asset_bake_readback_requests.to_string());
            let _ = root.set_attribute(
                "data-eldiron-gpu-generation",
                &images.context().generation().to_string(),
            );
            let _ = root.set_attribute(
                "data-eldiron-root-cpu-bytes",
                &self.app.session.ui.canvas.buffer.pixels().len().to_string(),
            );
            if let Some((session, ticks, state)) = self.app.application.host_runtime_status() {
                let _ = root.set_attribute("data-eldiron-runtime", &session.to_string());
                let _ = root.set_attribute("data-eldiron-ticks", &ticks.to_string());
                let _ = root.set_attribute(
                    "data-eldiron-runtime-state",
                    match state {
                        rusterix::ServerState::Off => "Off",
                        rusterix::ServerState::Running => "Running",
                        rusterix::ServerState::Paused => "Paused",
                    },
                );
            } else {
                for name in [
                    "data-eldiron-runtime",
                    "data-eldiron-ticks",
                    "data-eldiron-runtime-state",
                ] {
                    let _ = root.remove_attribute(name);
                }
            }
            if root.has_attribute("data-eldiron-inspect-view") {
                let _ = root.set_attribute(
                    "data-eldiron-view-source",
                    &self.app.application.host_view_diagnostics().to_string(),
                );
            } else {
                let _ = root.remove_attribute("data-eldiron-view-source");
            }
            if let Some(info) = self.app.application.host_document_info() {
                let _ = root.set_attribute("data-eldiron-document", &info.document.to_string());
                let _ = root.set_attribute("data-eldiron-dirty", &info.dirty.to_string());
            }
        }
        Ok(paint)
    }
    fn presented(&mut self, host: &mut HostContext) {
        self.app.presented(host);
        self.presentations += 1;
        if self.presentations == 1 {
            status("editing", None);
        }
        if let Some(root) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.document_element())
        {
            let _ = root.set_attribute(
                "data-eldiron-presentations",
                &self.presentations.to_string(),
            );
        }
    }
    fn close_requested(&mut self, host: &mut HostContext) -> bool {
        self.app.close_requested(host)
    }
    fn suspended(&mut self, host: &mut HostContext) {
        self.app.suspended(host);
    }
    fn resumed(&mut self, host: &mut HostContext) {
        self.app.resumed(host);
    }
}
