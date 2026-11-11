//! Widget-free Rust window host. Run with --no-default-features --features
//! platform-winit; --smoke verifies three real surface presentations.
use renderkit::{ImageRegistry, Result};
use theframework::{
    paint::{PaintList, Rect},
    platform::*,
    text::{TextOptions, TextSystem},
    text_edit::{EditorColors, EditorView},
};
struct Demo {
    fonts: TextSystem,
    input: EditorView,
    frames: usize,
    presentations: usize,
    smoke: bool,
    input_smoke: bool,
    uploaded: Option<u64>,
    deadline: Option<web_time::Instant>,
}
impl Demo {
    fn prepare(&mut self, host: &mut HostContext) {
        self.input.prepare(
            &mut self.fonts,
            TextOptions {
                family: "Roboto",
                size: 22.,
                width: None,
                line_height: None,
            },
            Rect::new(24., 24., (host.logical_size[0] - 48.).max(0.), 48.),
            true,
        );
        host.text_input = self.input.text_input_state(&mut self.fonts);
    }
}
impl GpuApplication for Demo {
    fn title(&self) -> String {
        if self.input_smoke {
            "Eldiron input validation".into()
        } else {
            "Eldiron native application host".into()
        }
    }
    fn initial_size(&self) -> (u32, u32) {
        (720, 180)
    }
    fn initialize(&mut self, host: &mut HostContext, _: &mut ImageRegistry) -> Result<()> {
        self.fonts
            .register_font(include_bytes!("../embedded/fonts/Roboto-Bold.ttf").to_vec());
        self.input.editor.set_text(
            &mut self.fonts,
            if self.input_smoke {
                ""
            } else {
                "Rust WGPU — العربية 東京 e\u{301}"
            },
        );
        self.input.set_focused(&mut self.fonts, true);
        self.prepare(host);
        Ok(())
    }
    fn resize(&mut self, host: &mut HostContext) {
        self.prepare(host);
    }
    fn input(&mut self, event: &InputEvent, host: &mut HostContext) -> InputResponse {
        let response = self.input.input(&mut self.fonts, event);
        host.text_input = self.input.text_input_state(&mut self.fonts);
        if self.input_smoke {
            eprintln!("native input {event:?} -> {:?}", self.input.editor.text());
            if self.input.editor.text() == "Native input" {
                eprintln!("Native OS keyboard input passed");
                host.requests.push(HostRequest::Exit);
            }
        }
        response.input
    }
    fn presented(&mut self, _host: &mut HostContext) {
        self.presentations += 1;
        if self.smoke {
            eprintln!(
                "widget-free surface presentation {} acknowledged",
                self.presentations
            );
        }
    }
    fn update(&mut self, host: &mut HostContext, _: web_time::Duration) -> FrameRequest {
        if self
            .deadline
            .is_some_and(|deadline| web_time::Instant::now() >= deadline)
        {
            host.requests.push(HostRequest::Fail(if self.input_smoke {
                "Native OS keyboard validation timed out".into()
            } else {
                format!(
                    "Native presentation validation timed out: {} successful surface presentations",
                    self.presentations
                )
            }));
            return FrameRequest::idle();
        }
        if self.smoke && self.presentations >= 3 {
            host.requests.push(HostRequest::Exit);
        }
        if self.smoke {
            FrameRequest::continuous(30.)
        } else if self.input_smoke {
            FrameRequest {
                redraw: false,
                wake_after: Some(web_time::Duration::from_millis(100)),
            }
        } else {
            FrameRequest::idle()
        }
    }
    fn frame(&mut self, host: &mut HostContext, images: &mut ImageRegistry) -> Result<PaintList> {
        let mut list = PaintList::default();
        list.rect(
            Rect::new(0., 0., host.logical_size[0], host.logical_size[1]),
            0.,
            [0.03, 0.05, 0.08, 1.],
        );
        list.rect(self.input.bounds(), 4., [0.1, 0.14, 0.2, 1.]);
        self.input
            .paint(&mut self.fonts, &mut list, EditorColors::default(), true);
        list.validate().unwrap();
        assert_eq!(images.context().transfers().readback_bytes, 0);
        let uploads = images.context().transfers().uploaded_bytes;
        if self.smoke && self.presentations > 1 {
            assert_eq!(self.uploaded, Some(uploads));
        }
        self.uploaded = Some(uploads);
        self.frames += 1;
        if self.smoke {
            eprintln!(
                "widget-free native frame {}: {} paint operations; no UI session or CPU framebuffer",
                self.frames,
                list.ops.len()
            );
        }
        Ok(list)
    }
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    theframework::platform::winit::run(Box::new(Demo {
        fonts: TextSystem::default(),
        input: EditorView::new(false),
        frames: 0,
        presentations: 0,
        smoke: std::env::args().any(|s| s == "--smoke"),
        input_smoke: std::env::args().any(|s| s == "--input-smoke"),
        uploaded: None,
        deadline: if std::env::args().any(|s| s == "--smoke" || s == "--input-smoke") {
            Some(web_time::Instant::now() + web_time::Duration::from_secs(10))
        } else {
            None
        },
    }))?;
    Ok(())
}
