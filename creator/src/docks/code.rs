use crate::prelude::*;
use theframework::prelude::*;

pub struct CodeDock {}

impl Dock for CodeDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {}
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut textedit = TheTextAreaEdit::new(TheId::named("DockCodeEditor"));
        textedit.set_code_type("Python");
        textedit.set_continuous(true);
        textedit.display_line_number(true);
        textedit.set_code_theme("base16-eighties.dark");
        textedit.use_global_statusbar(true);
        textedit.set_font_size(14.0);
        center.set_widget(textedit);

        center
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(id) = server_ctx.pc.id() {
            if server_ctx.pc.is_character() {
                if let Some(character) = project.characters.get(&id) {
                    ui.set_widget_value(
                        "DockCodeEditor",
                        ctx,
                        TheValue::Text(character.source.clone()),
                    );
                }
            } else if server_ctx.pc.is_item() {
                if let Some(item) = project.items.get(&id) {
                    println!("{}", item.source);
                    ui.set_widget_value("DockCodeEditor", ctx, TheValue::Text(item.source.clone()));
                }
            }
        }
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::ValueChanged(id, value) => {
                if id.name == "DockCodeEditor" {
                    if let Some(id) = server_ctx.pc.id() {
                        if server_ctx.pc.is_character() {
                            if let Some(code) = value.to_string() {
                                if let Some(character) = project.characters.get_mut(&id) {
                                    character.source = code.clone();
                                    character.source_debug = code;
                                    redraw = true;
                                }
                            }
                        } else if server_ctx.pc.is_item() {
                            if let Some(code) = value.to_string() {
                                if let Some(item) = project.items.get_mut(&id) {
                                    item.source = code.clone();
                                    item.source_debug = code;
                                    redraw = true;
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        redraw
    }
}
