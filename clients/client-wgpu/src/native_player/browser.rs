//! Browser bootstrap for the common graphical application. Project fetching and
//! GPU creation are asynchronous; the host owns the WebGPU canvas/surface.
use super::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

fn status(state: &str, message: Option<&str>) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
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
        let _ = panel.set_attribute("hidden", "");
        if message.is_some() {
            let _ = panel.remove_attribute("hidden");
        }
        // Author styles cannot make a hidden loading panel cover the canvas.
        if message.is_none() {
            panel.remove();
        }
    }
}
fn report(error: impl std::fmt::Display) {
    let message = format!("Cannot start Eldiron Player: {error}");
    web_sys::console::error_1(&JsValue::from_str(&message));
    status("error", Some(&message));
}
pub fn start() {
    std::panic::set_hook(Box::new(|info| {
        console_error_panic_hook::hook(info);
        report("The game stopped unexpectedly. See the browser console for details.");
    }));
    status("loading", Some("Loading game…"));
    wasm_bindgen_futures::spawn_local(async {
        if let Err(error) = load().await {
            report(error);
        }
    });
}
async fn load() -> std::result::Result<(), String> {
    let window = web_sys::window().ok_or("Browser window is unavailable")?;
    let document = window.document().ok_or("Browser document is unavailable")?;
    let query =
        web_sys::UrlSearchParams::new_with_str(&window.location().search().map_err(js_error)?)
            .map_err(js_error)?;
    let path = query
        .get("project")
        .filter(|path| !path.trim().is_empty())
        .unwrap_or_else(|| "game.eldiron".into());
    let response = JsFuture::from(window.fetch_with_str(&path))
        .await
        .map_err(js_error)?
        .dyn_into::<web_sys::Response>()
        .map_err(js_error)?;
    if !response.ok() {
        return Err(format!("Cannot load {path}: HTTP {}", response.status()));
    }
    let buffer = JsFuture::from(response.array_buffer().map_err(js_error)?)
        .await
        .map_err(js_error)?;
    let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
    let mut project = shared::project_io::decode_project(&bytes)
        .map_err(|error| format!("Cannot load {path}: {error}"))?;
    project.migrate_default_ruleset();
    let _ = project.sync_ruleset_items();
    let debug = query
        .get("debug")
        .is_some_and(|value| matches!(value.as_str(), "1" | "true"));
    let player = PlayerApplication::new(
        project,
        debug,
        Box::new(AudioServices::new(StandardServices)),
    )?;
    // Winit observes CSS canvas size/DPR changes. This launcher owns a full-page
    // canvas; embedded foreign hosts retain their separate sizing contract.
    let style = document.create_element("style").map_err(js_error)?;
    style.set_text_content(Some("html,body{margin:0;width:100%;height:100%;overflow:hidden;background:#18181b}canvas{display:block;width:100vw!important;height:100vh!important;outline:none}"));
    document
        .body()
        .ok_or("Browser document body is unavailable")?
        .append_child(&style)
        .map_err(js_error)?;
    status("starting", Some("Starting graphics…"));
    theframework::platform::winit::run(Box::new(BrowserApplication {
        player,
        presentations: 0,
    }))
    .map_err(|error| error.to_string())
}
fn js_error(error: JsValue) -> String {
    error.as_string().unwrap_or_else(|| format!("{error:?}"))
}
struct BrowserApplication {
    player: PlayerApplication,
    presentations: u64,
}
impl GpuApplication for BrowserApplication {
    fn host_failed(&mut self, error: &RenderError) {
        report(error);
    }
    fn title(&self) -> String {
        self.player.title()
    }
    fn initial_size(&self) -> (u32, u32) {
        self.player.initial_size()
    }
    fn initialize(&mut self, host: &mut HostContext, images: &mut ImageRegistry) -> Result<()> {
        self.player
            .initialize(host, images)
            .inspect_err(|error| report(error))
    }
    fn gpu_recreated(&mut self, host: &mut HostContext, images: &mut ImageRegistry) -> Result<()> {
        self.player
            .gpu_recreated(host, images)
            .inspect_err(|error| report(error))
    }
    fn resize(&mut self, host: &mut HostContext) {
        self.player.resize(host)
    }
    fn native_text_snapshot(
        &mut self,
        host: &mut HostContext,
    ) -> Option<theframework::text_edit::NativeTextSnapshot> {
        self.player.native_text_snapshot(host)
    }
    fn native_text_query(
        &mut self,
        host: &mut HostContext,
        request: &theframework::text_edit::NativeTextRequest,
    ) -> Option<theframework::text_edit::NativeTextAnswer> {
        self.player.native_text_query(host, request)
    }
    fn input(&mut self, event: &InputEvent, host: &mut HostContext) -> InputResponse {
        self.player.input(event, host)
    }
    fn update(&mut self, host: &mut HostContext, elapsed: Duration) -> FrameRequest {
        let request = self.player.update(host, elapsed);
        for action in &host.requests {
            if let HostRequest::Fail(error) = action {
                report(error);
            }
        }
        request
    }
    fn frame(&mut self, host: &mut HostContext, images: &mut ImageRegistry) -> Result<PaintList> {
        let before = images.context().transfers().readback_bytes;
        let frame = self
            .player
            .frame(host, images)
            .inspect_err(|error| report(error))?;
        if images.context().transfers().readback_bytes != before {
            let error =
                RenderError("Browser player frame performed an implicit color readback".into());
            report(&error);
            return Err(error);
        }
        if let Some(root) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.document_element())
        {
            let _ = root.set_attribute(
                "data-eldiron-readback-bytes",
                &images.context().transfers().readback_bytes.to_string(),
            );
            let _ = root.set_attribute(
                "data-eldiron-gpu-generation",
                &images.context().generation().to_string(),
            );
        }
        Ok(frame)
    }
    fn presented(&mut self, host: &mut HostContext) {
        self.player.presented(host);
        self.presentations += 1;
        if self.presentations == 1 {
            status("playing", None);
        }
        if let Some(root) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.document_element())
        {
            let _ = root.set_attribute(
                "data-eldiron-presentations",
                &self.presentations.to_string(),
            );
            let _ = root.set_attribute(
                "data-eldiron-session",
                &self.player.player.runtime.view().session_id.to_string(),
            );
            let _ = root.set_attribute(
                "data-eldiron-ticks",
                &self.player.player.runtime.tick_sequence().to_string(),
            );
        }
    }
    fn suspended(&mut self, host: &mut HostContext) {
        self.player.suspended(host)
    }
    fn resumed(&mut self, host: &mut HostContext) {
        self.player.resumed(host)
    }
}
