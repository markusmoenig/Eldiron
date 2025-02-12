use crate::prelude::*;

pub struct CodeEditor {}

#[allow(clippy::new_without_default)]
impl CodeEditor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build(&mut self) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut textedit = TheTextAreaEdit::new(TheId::named("CodeEdit"));
        textedit.set_continuous(true);
        textedit.display_line_number(true);
        textedit.set_code_type("Python");
        textedit.set_code_theme("base16-eighties.dark");
        textedit.use_global_statusbar(true);
        textedit.set_font_size(14.0);
        center.set_widget(textedit);

        center
    }

    pub fn build_data(&mut self) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut textedit = TheTextAreaEdit::new(TheId::named("DataEdit"));
        textedit.set_continuous(true);
        textedit.display_line_number(true);
        textedit.set_code_type("toml");
        textedit.set_code_theme("base16-eighties.dark");
        textedit.use_global_statusbar(true);
        textedit.set_font_size(14.0);
        center.set_widget(textedit);

        center
    }

    /*
    pub fn handle_event(
        &mut self,
        _event: &TheEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        // let redraw = false;
        // #[allow(clippy::single_match)]
        // match event {
        //     _ => {}
        // }

        // redraw
        false
    }*/
}
