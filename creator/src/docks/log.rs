// use crate::editor::RUSTERIX;
use crate::prelude::*;
use theframework::prelude::*;

pub struct LogDock;

impl Dock for LogDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut textedit = TheTextAreaEdit::new(TheId::named("LogEdit"));

        if let Some(bytes) = crate::Embedded::get("parser/gruvbox-dark.tmTheme") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                textedit.add_theme_from_string(source);
                textedit.set_code_theme("Gruvbox Dark");
            }
        }

        if let Some(bytes) = crate::Embedded::get("parser/log.sublime-syntax") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                textedit.add_syntax_from_string(source);
                textedit.set_code_type("Eldiron Log");
            }
        }

        textedit.set_continuous(true);
        textedit.display_line_number(true);
        textedit.use_global_statusbar(true);
        textedit.set_font_size(14.0);
        textedit.readonly(true);
        // Handled manually, but this dock is read-only
        textedit.set_supports_undo(false);
        textedit.readonly(true);

        center.set_widget(textedit);

        center
    }

    fn activate(
        &mut self,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &Project,
        _server_ctx: &mut ServerContext,
    ) {
    }

    fn supports_actions(&self) -> bool {
        false
    }
}
