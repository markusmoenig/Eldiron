#![windows_subsystem = "windows"]

use rustapi::editor::Editor;
use theframework::*;

fn main() {
    let args: Vec<_> = std::env::args().collect();

    unsafe {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    #[cfg(not(target_arch = "wasm32"))]
    let _ = rustapi::i18n::select_system_locales();

    let editor = Editor::new();
    let mut app = TheApp::new();
    app.set_cmd_line_args(args);

    let () = app.run(Box::new(editor));
}
