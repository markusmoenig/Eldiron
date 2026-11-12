//! Native assembly smoke host. Canonical launchers switch after HUD/input parity.
use eldiron_presentation::ScreenEffect;
use eldiron_runtime::{RegionOutputs, RuntimeView};
use renderkit::{ImageRegistry, Result as GpuResult};
use scenevm_unified_app::native_player::{AudioServices, PlayerApplication, PlayerServices};
use std::time::Duration;
use theframework::{paint::PaintList, platform::*};
struct Services;
impl PlayerServices for Services {
    fn region_events(&mut self, _: uuid::Uuid, _: &RuntimeView<'_>, events: RegionOutputs<'_>) {
        // This explicit smoke consumer verifies screen/runtime integration.
        // Audio is owned by AudioServices; text/speech by the screen controller.
        // Container/HUD service adoption remains explicit work.
        if !events.containers.is_empty() {
            eprintln!(
                "native smoke outputs: {} audio, {} world text, {} containers",
                events.audio.len(),
                events.world_text.len(),
                events.containers.len()
            );
        }
    }
    fn screen_effect(&mut self, _: &RuntimeView<'_>, effect: &ScreenEffect) -> Result<(), String> {
        Err(format!(
            "Native smoke service has no handler for {effect:?}"
        ))
    }
}
struct NativeExample {
    player: PlayerApplication,
    smoke: bool,
    presentations: usize,
    deadline: Option<std::time::Instant>,
}
impl GpuApplication for NativeExample {
    fn title(&self) -> String {
        self.player.title()
    }
    fn initial_size(&self) -> (u32, u32) {
        self.player.initial_size()
    }
    fn initialize(&mut self, h: &mut HostContext, i: &mut ImageRegistry) -> GpuResult<()> {
        self.player.initialize(h, i)
    }
    fn gpu_recreated(&mut self, h: &mut HostContext, i: &mut ImageRegistry) -> GpuResult<()> {
        self.player.gpu_recreated(h, i)
    }
    fn resize(&mut self, h: &mut HostContext) {
        self.player.resize(h)
    }
    fn input(&mut self, e: &InputEvent, h: &mut HostContext) -> InputResponse {
        self.player.input(e, h)
    }
    fn presented(&mut self, h: &mut HostContext) {
        self.presentations += 1;
        self.player.presented(h);
        if self.smoke {
            eprintln!(
                "native player surface presentation {} acknowledged",
                self.presentations
            );
        }
    }
    fn update(&mut self, h: &mut HostContext, elapsed: Duration) -> FrameRequest {
        let request = self.player.update(h, elapsed);
        if self
            .deadline
            .is_some_and(|deadline| std::time::Instant::now() >= deadline)
        {
            h.requests.push(HostRequest::Fail(format!("Native player presentation validation timed out: {} successful surface presentations",self.presentations)));
        }
        if self.smoke
            && self.presentations >= 3
            && self.player.player.runtime.server.has_local_player()
        {
            h.requests.push(HostRequest::Exit);
        }
        request
    }
    fn frame(&mut self, h: &mut HostContext, i: &mut ImageRegistry) -> GpuResult<PaintList> {
        let paint = self.player.frame(h, i)?;
        if self.smoke {
            assert_eq!(i.context().transfers().readback_bytes, 0);
            eprintln!(
                "native player frame {}: {} operations, {} direct scene views, no framebuffer readback",
                self.player.frame_count(),
                paint.ops.len(),
                self.player.screen.view_images().len()
            );
        }
        Ok(paint)
    }
    fn suspended(&mut self, h: &mut HostContext) {
        self.player.suspended(h)
    }
    fn resumed(&mut self, h: &mut HostContext) {
        self.player.resumed(h)
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|a| a == "--smoke");
    let path = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .unwrap_or("game.eldiron".into());
    let mut project = shared::project_io::decode_project(&std::fs::read(path)?)?;
    project.migrate_default_ruleset();
    let app = PlayerApplication::new(project, false, Box::new(AudioServices::new(Services)))?;
    theframework::platform::winit::run(Box::new(NativeExample {
        player: app,
        smoke,
        presentations: 0,
        deadline: smoke.then(|| std::time::Instant::now() + Duration::from_secs(10)),
    }))?;
    Ok(())
}
