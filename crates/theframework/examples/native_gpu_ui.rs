//! Run with --no-default-features --features platform-winit,ui; --smoke exits after
//! real surface presentations. This application has no raster drawing callback.
use theframework::{platform::*, prelude::*, render_wgpu::UiFrameImages};
struct Demo {
    frames: usize,
    smoke: bool,
    presentations: usize,
    deadline: Option<web_time::Instant>,
}
impl UiApplication for Demo {
    fn title(&self) -> String {
        "Eldiron native WGPU UI".into()
    }
    fn initial_size(&self) -> (u32, u32) {
        (800, 420)
    }
    fn initialize(
        &mut self,
        ui: &mut UiSession,
        _: &mut UiFrameImages<'_>,
    ) -> renderkit::Result<()> {
        let mut layout = TheVLayout::new(TheId::named("native-root"));
        let mut title = TheText::new(TheId::named("heading"));
        title.set_text("Eldiron · native WGPU".into());
        title.set_text_size(22.);
        layout.widgets().push(Box::new(title));
        let mut button = TheTraybarButton::new(TheId::named("run"));
        button.set_text("Run project".into());
        layout.widgets().push(Box::new(button));
        ui.ui.canvas.set_layout(layout);
        Ok(())
    }
    fn update(&mut self, ui: &mut UiSession, _: web_time::Duration) -> FrameRequest {
        let changed = ui.ui.update(&mut ui.context);
        if self
            .deadline
            .is_some_and(|deadline| web_time::Instant::now() >= deadline)
        {
            ui.requests.push(HostRequest::Fail(format!(
                "Native UI presentation validation timed out: {} successful surface presentations",
                self.presentations
            )));
        }
        if self.smoke && self.presentations >= 3 {
            ui.requests.push(HostRequest::Exit);
        }
        if self.smoke {
            FrameRequest::continuous(30.)
        } else {
            FrameRequest {
                redraw: changed,
                wake_after: None,
            }
        }
    }
    fn presented(&mut self, _: &mut UiSession) {
        self.presentations += 1;
        if self.smoke {
            eprintln!(
                "native UI surface presentation {} acknowledged",
                self.presentations
            );
        }
    }
    fn frame(
        &mut self,
        ui: &mut UiSession,
        images: &mut UiFrameImages<'_>,
    ) -> renderkit::Result<theframework::paint::PaintList> {
        let list = ui
            .paint(images)
            .map_err(|e| renderkit::RenderError(e.to_string()))?;
        assert!(ui.ui.canvas.buffer.pixels().is_empty());
        if self.smoke {
            for name in ["heading", "run"] {
                let dim = *ui.ui.get_widget(name).unwrap().dim();
                eprintln!(
                    "native widget {name}: {dim:?}; logical {}x{}",
                    ui.context.width, ui.context.height
                );
                assert!(
                    dim.width > 0 && dim.height > 0,
                    "native host must lay out every demo widget"
                );
            }
        }
        self.frames += 1;
        if self.smoke {
            eprintln!(
                "native GPU UI frame {}: {} paint operations; zero canvas pixels",
                self.frames,
                list.ops.len()
            );
        }
        Ok(list)
    }
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    theframework::platform::winit::run(Box::new(UiApplicationAdapter::new(Demo {
        frames: 0,
        smoke: std::env::args().any(|a| a == "--smoke"),
        presentations: 0,
        deadline: std::env::args()
            .any(|a| a == "--smoke")
            .then(|| web_time::Instant::now() + web_time::Duration::from_secs(10)),
    })))?;
    Ok(())
}
