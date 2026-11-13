//! Explicit offscreen reference capture. Run in its own process so legacy
//! Creator globals start in the same state as the migration host's startup.
use eldiron_creator::editor::Editor;
use theframework::prelude::*;
fn main() {
    let output = std::env::args()
        .nth(1)
        .expect("usage: reference_ui_capture OUTPUT.png");
    let start = std::time::Instant::now();
    let mut stage = start;
    let profile = std::env::var_os("ELDIRON_PROFILE_STARTUP").is_some();
    let mut ctx = TheContext::new(1200, 800, 1.);
    if profile {
        eprintln!(
            "Reference startup UI context: {:.2}ms",
            stage.elapsed().as_secs_f64() * 1000.
        );
    }
    stage = std::time::Instant::now();
    let mut ui = TheUI::new();
    ui.init(&mut ctx);
    ui.canvas.root = true;
    ui.canvas.set_dim(TheDim::sized(1200, 800), &mut ctx);
    let mut editor = Editor::new();
    editor.init_ui(&mut ui, &mut ctx);
    if profile {
        eprintln!(
            "Reference startup init UI: {:.2}ms",
            stage.elapsed().as_secs_f64() * 1000.
        );
    }
    stage = std::time::Instant::now();
    ui.canvas.layout(1200, 800, &mut ctx);
    if profile {
        eprintln!(
            "Reference startup layout: {:.2}ms",
            stage.elapsed().as_secs_f64() * 1000.
        );
    }
    stage = std::time::Instant::now();
    ctx.load_system_fonts(editor.fonts_to_load());
    if profile {
        eprintln!(
            "Reference startup system fonts: {:.2}ms",
            stage.elapsed().as_secs_f64() * 1000.
        );
    }
    editor.set_cmd_line_args(vec!["eldiron-creator".into()], &mut ctx);
    ui.process_events(&mut ctx);
    ui.update(&mut ctx);
    editor.update_ui(&mut ui, &mut ctx);
    let mut pixels = vec![0; 1200 * 800 * 4];
    ui.draw(&mut pixels, &mut ctx);
    if profile {
        eprintln!(
            "Reference startup first offscreen frame: {:.2}ms",
            start.elapsed().as_secs_f64() * 1000.
        );
    }
    std::fs::write(
        output,
        TheRGBABuffer::from(pixels, 1200, 800).to_png().unwrap(),
    )
    .unwrap();
}
