//! Run with --no-default-features --features platform-winit; --smoke exits after
//! real surface presentations. This application has no raster drawing callback.
use theframework::{platform::*, prelude::*, render_wgpu::UiImages};
struct Demo {
    frames: usize,
    smoke: bool,
}
impl GpuApplication for Demo {
    fn title(&self) -> String {
        "Eldiron native WGPU UI".into()
    }
    fn initial_size(&self) -> (u32, u32) {
        (800, 420)
    }
    fn initialize(&mut self, ui: &mut UiSession, _: &mut UiImages) -> renderkit::Result<()> {
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
        ui.ui.update(&mut ui.context);
        if self.smoke && self.frames >= 3 {
            ui.requests.push(HostRequest::Exit);
        }
        FrameRequest::continuous(30.)
    }
    fn frame(
        &mut self,
        ui: &mut UiSession,
        images: &mut UiImages,
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
fn main() {
    theframework::platform::winit::run(Box::new(Demo {
        frames: 0,
        smoke: std::env::args().any(|a| a == "--smoke"),
    }))
    .unwrap();
}
