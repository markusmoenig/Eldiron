use crate::prelude::*;

use crate::editor::CODEGRIDFX;
use codegridfxlib::{Module, ModuleType};

pub struct CodeEditor {
    pub show_template: bool,
    pub content: ContentContext,
    pub last_header_text: String,
}

#[allow(clippy::new_without_default)]
impl CodeEditor {
    pub fn new() -> Self {
        Self {
            show_template: true,
            content: ContentContext::Unknown,
            last_header_text: "".into(),
        }
    }

    /// Set the module based on the given context and template mode.
    pub fn set_module_character(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        character: &Character,
    ) {
        *CODEGRIDFX.write().unwrap() = character.module.clone();
        CODEGRIDFX.write().unwrap().name = character.name.clone();
        CODEGRIDFX
            .write()
            .unwrap()
            .set_module_type(ModuleType::CharacterTemplate);
        CODEGRIDFX.write().unwrap().redraw(ui, ctx);

        self.last_header_text = format!("{} - Character Template", character.name);
        if let Some(text) = ui.get_text("Code Editor Header Text") {
            text.set_text(self.last_header_text.clone());
            ctx.ui.relayout = true;
        }

        if let Some(button) = ui.get_group_button("Code Template Switch") {
            button.set_index(0);
        }

        self.content = ContentContext::CharacterTemplate(character.id);

        self.show_template = true;
    }

    /// Set the module based on the given context and template mode.
    pub fn set_module_character_instance(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        character: &Character,
    ) {
        *CODEGRIDFX.write().unwrap() = character.module.clone();
        CODEGRIDFX.write().unwrap().name = character.name.clone();
        CODEGRIDFX
            .write()
            .unwrap()
            .set_module_type(ModuleType::CharacterInstance);
        CODEGRIDFX.write().unwrap().redraw(ui, ctx);

        self.last_header_text = format!("{} - Character Instance", character.name);
        if let Some(text) = ui.get_text("Code Editor Header Text") {
            text.set_text(self.last_header_text.clone());
            ctx.ui.relayout = true;
        }

        if let Some(button) = ui.get_group_button("Code Template Switch") {
            button.set_index(1);
        }

        self.content = ContentContext::CharacterInstance(character.id);

        self.show_template = false;
    }

    /// Set the module based on the given context and template mode.
    pub fn set_module_item(&mut self, ui: &mut TheUI, ctx: &mut TheContext, item: &Item) {
        *CODEGRIDFX.write().unwrap() = item.module.clone();
        CODEGRIDFX.write().unwrap().name = item.name.clone();
        CODEGRIDFX
            .write()
            .unwrap()
            .set_module_type(ModuleType::ItemTemplate);
        CODEGRIDFX.write().unwrap().redraw(ui, ctx);

        self.last_header_text = format!("{} - Item Template", item.name);
        if let Some(text) = ui.get_text("Code Editor Header Text") {
            text.set_text(self.last_header_text.clone());
            ctx.ui.relayout = true;
        }

        if let Some(button) = ui.get_group_button("Code Template Switch") {
            button.set_index(0);
        }

        self.content = ContentContext::ItemTemplate(item.id);

        self.show_template = true;
    }

    /// Set the module based on the given context and template mode.
    pub fn set_module_item_instance(&mut self, ui: &mut TheUI, ctx: &mut TheContext, item: &Item) {
        *CODEGRIDFX.write().unwrap() = item.module.clone();
        CODEGRIDFX.write().unwrap().name = item.name.clone();
        CODEGRIDFX
            .write()
            .unwrap()
            .set_module_type(ModuleType::CharacterInstance);
        CODEGRIDFX.write().unwrap().redraw(ui, ctx);

        self.last_header_text = "Item Instances are not supported".to_string();
        if let Some(text) = ui.get_text("Code Editor Header Text") {
            text.set_text(self.last_header_text.clone());
            ctx.ui.relayout = true;
        }

        if let Some(button) = ui.get_group_button("Code Template Switch") {
            button.set_index(1);
        }

        self.content = ContentContext::ItemInstance(item.id);
        self.show_template = false;
    }

    /// Switch between template / instance
    pub fn switch_module_to(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &ServerContext,
        template: bool,
    ) {
        let handled = match self.content {
            ContentContext::CharacterTemplate(_) => {
                if !template {
                    if let ContentContext::CharacterInstance(inst_id) = server_ctx.cc {
                        if let Some(region) = project.get_region_ctx(server_ctx) {
                            if let Some(character) = region.characters.get(&inst_id) {
                                self.set_module_character_instance(ui, ctx, character);
                                ui.set_widget_value(
                                    "CodeEdit",
                                    ctx,
                                    TheValue::Text(character.source.clone()),
                                );
                            }
                        }
                    }
                }
                true
            }
            ContentContext::CharacterInstance(id) => {
                if template {
                    // Switch from instance to template
                    let mut temp_id = None;
                    if let Some(region) = project.get_region_ctx(server_ctx) {
                        if let Some(temp) = region.characters.get(&id) {
                            temp_id = Some(temp.character_id);
                        }
                    }
                    if let Some(temp_id) = temp_id {
                        if let Some(character) = project.characters.get(&temp_id) {
                            self.set_module_character(ui, ctx, character);

                            ui.set_widget_value(
                                "CodeEdit",
                                ctx,
                                TheValue::Text(character.source.clone()),
                            );
                        }
                    }
                }
                true
            }
            ContentContext::ItemTemplate(_) => {
                if !template {
                    if let ContentContext::ItemInstance(inst_id) = server_ctx.cc {
                        if let Some(region) = project.get_region_ctx(server_ctx) {
                            if let Some(item) = region.items.get(&inst_id) {
                                self.set_module_item_instance(ui, ctx, item);
                                ui.set_widget_value(
                                    "CodeEdit",
                                    ctx,
                                    TheValue::Text(item.source.clone()),
                                );
                            }
                        }
                    }
                }
                true
            }
            ContentContext::ItemInstance(id) => {
                if template {
                    // Switch from instance to template
                    let mut temp_id = None;
                    if let Some(region) = project.get_region_ctx(server_ctx) {
                        if let Some(temp) = region.items.get(&id) {
                            temp_id = Some(temp.item_id);
                        }
                    }
                    if let Some(temp_id) = temp_id {
                        if let Some(item) = project.items.get(&temp_id) {
                            self.set_module_item(ui, ctx, item);

                            ui.set_widget_value(
                                "CodeEdit",
                                ctx,
                                TheValue::Text(item.source.clone()),
                            );
                        }
                    }
                }
                true
            }
            _ => false,
        };

        if !handled {
            self.clear_module(ui, ctx);
        }

        self.show_template = template;
    }

    /// Clear the module
    pub fn clear_module(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        *CODEGRIDFX.write().unwrap() = Module::default();
        CODEGRIDFX.write().unwrap().redraw(ui, ctx);

        if let Some(text) = ui.get_text("Code Editor Header Text") {
            self.last_header_text = "Undefined".to_string();
            text.set_text(self.last_header_text.clone());
            ctx.ui.relayout = true;
        }

        self.content = ContentContext::Unknown;
    }

    pub fn build(&mut self) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut textedit = TheTextAreaEdit::new(TheId::named("CodeEdit"));
        textedit.set_continuous(true);
        textedit.display_line_number(true);
        textedit.as_code_editor("Python", TheCodeEditorSettings::default());
        textedit.set_code_theme("base16-eighties.dark");
        textedit.use_global_statusbar(true);
        textedit.set_font_size(14.0);
        center.set_widget(textedit);

        center
    }

    pub fn build_data(&mut self) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut textedit = TheTextAreaEdit::new(TheId::named("DataEdit"));
        // textedit.as_code_editor(
        //     "TOML",
        //     TheCodeEditorSettings {
        //         indicate_space: false,
        //         ..Default::default()
        //     },
        // );
        if let Some(bytes) = crate::Embedded::get("parser/TOML.sublime-syntax") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                textedit.add_syntax_from_string(source);
                textedit.set_code_type("TOML");
            }
        }
        textedit.set_continuous(true);
        textedit.display_line_number(true);
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
