//! Explicit offscreen reference capture. Run in its own process so legacy
//! Creator globals start in the same state as the migration host's startup.
use eldiron_creator::editor::Editor;
use theframework::prelude::*;
fn main() {
    let output = std::env::args()
        .nth(1)
        .expect("usage: reference_ui_capture OUTPUT.png");
    let mut ctx = TheContext::new(1200, 800, 1.);
    let mut ui = TheUI::new();
    ui.init(&mut ctx);
    ui.canvas.root = true;
    ui.canvas.set_dim(TheDim::sized(1200, 800), &mut ctx);
    let mut editor = Editor::new();
    editor.init_ui(&mut ui, &mut ctx);
    ui.canvas.layout(1200, 800, &mut ctx);
    ctx.load_system_fonts(editor.fonts_to_load());
    editor.set_cmd_line_args(vec!["eldiron-creator".into()], &mut ctx);
    ui.process_events(&mut ctx);
    ui.update(&mut ctx);
    editor.update_ui(&mut ui, &mut ctx);
    let mut pixels = vec![0; 1200 * 800 * 4];
    ui.draw(&mut pixels, &mut ctx);
    std::fs::write(
        output,
        TheRGBABuffer::from(pixels, 1200, 800).to_png().unwrap(),
    )
    .unwrap();
}
