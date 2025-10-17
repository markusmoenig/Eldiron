use crate::prelude::*;

use crate::editor::{CODEGRIDFX, SHADEGRIDFX, SHADERBUFFER};
use codegridfx::{Module, ModuleType};

#[derive(Debug, PartialEq)]
pub enum VisibleCodePanel {
    None,
    Code,
    Shade,
}

pub struct CodeEditor {
    pub show_template: bool,
    pub code_content: ContentContext,
    pub shader_content: ContentContext,
    pub last_header_text: String,

    pub last_data_header_text: String,

    pub active_panel: VisibleCodePanel,
}

#[allow(clippy::new_without_default)]
impl CodeEditor {
    pub fn new() -> Self {
        Self {
            show_template: true,
            code_content: ContentContext::Unknown,
            shader_content: ContentContext::Unknown,
            last_header_text: "".into(),

            last_data_header_text: "".into(),
            active_panel: VisibleCodePanel::None,
        }
    }

    /// Set the shader to the current selection state.
    pub fn set_shader_for_current_geometry(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &ServerContext,
    ) {
        self.active_panel = VisibleCodePanel::Shade;

        if server_ctx.get_map_context() == MapContext::Shader {
            if let Some(curr_shader_id) = server_ctx.curr_material_id {
                if let Some(shader) = project.shaders.get(&curr_shader_id) {
                    self.shader_content = ContentContext::Shader(curr_shader_id);
                    crate::utils::draw_shader_into(shader, &mut SHADERBUFFER.write().unwrap());
                }
            }
        } else {
            // let mut has_sector = false;
            /*
            if let Some(map) = project.get_map_mut(server_ctx) {
                if let Some(sector_id) = map.selected_sectors.first() {
                    if let Some(sector) = map.find_sector(*sector_id) {
                        has_sector = true;

                        // *SHADEGRIDFX.write().unwrap() = sector.module.clone();
                        self.shader_content = ContentContext::Sector(sector.creator_id);
                    }
                }
            }*/

            // if !has_sector {
            //*SHADEGRIDFX.write().unwrap() = Module::as_type(ModuleType::Sector);
            //self.shader_content = ContentContext::Unknown;
            // }

            //self.shader_content = ContentContext::Sh;

            SHADEGRIDFX
                .write()
                .unwrap()
                .set_module_type(ModuleType::Shader);
        }
        SHADEGRIDFX.write().unwrap().redraw(ui, ctx);
    }

    /*
    /// Set the shader to the given sector.
    pub fn set_shader_sector(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        sector: &rusterix::Sector,
    ) {
        *SHADEGRIDFX.write().unwrap() = sector.module.clone();
        self.shader_content = ContentContext::Sector(sector.creator_id);

        SHADEGRIDFX
            .write()
            .unwrap()
            .set_module_type(ModuleType::Sector);
        SHADEGRIDFX.write().unwrap().redraw(ui, ctx);
    }*/

    /// Clear the current shader
    pub fn clear_shader(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        *SHADEGRIDFX.write().unwrap() = Module::default();
        self.shader_content = ContentContext::Unknown;

        SHADEGRIDFX
            .write()
            .unwrap()
            .set_module_type(ModuleType::Unknown);
        SHADEGRIDFX.write().unwrap().redraw(ui, ctx);
    }

    /// Set the shader to the given material.
    pub fn set_shader_material(&mut self, ui: &mut TheUI, ctx: &mut TheContext, material: &Module) {
        *SHADEGRIDFX.write().unwrap() = material.clone();
        self.shader_content = ContentContext::Shader(material.id);

        println!("set_shader_material");

        crate::utils::draw_shader_into(material, &mut SHADERBUFFER.write().unwrap());
        SHADEGRIDFX
            .write()
            .unwrap()
            .set_module_type(ModuleType::Shader);
        SHADEGRIDFX.write().unwrap().redraw(ui, ctx);

        let mut module = SHADEGRIDFX.write().unwrap();
        crate::utils::draw_shader_into(&module, &mut SHADERBUFFER.write().unwrap());

        module.set_shader_background(SHADERBUFFER.read().unwrap().clone(), ui, ctx);
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

        let mut is_player = false;

        // Get all player entities
        match character.data.parse::<Table>() {
            Ok(data) => {
                if let Some(game) = data.get("attributes").and_then(toml::Value::as_table) {
                    if let Some(value) = game.get("player") {
                        if let Some(v) = value.as_bool() {
                            is_player = v;
                        }
                    }
                }
            }
            _ => {}
        }
        CODEGRIDFX.write().unwrap().player = is_player;
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

        self.last_data_header_text = format!("{} - Character Template", character.name);
        if let Some(text) = ui.get_text("Data Editor Header Text") {
            text.set_text(self.last_data_header_text.clone());
            ctx.ui.relayout = true;
        }

        if let Some(button) = ui.get_group_button("Code Template Switch") {
            button.set_index(0);
        }

        self.code_content = ContentContext::CharacterTemplate(character.id);

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

        self.code_content = ContentContext::CharacterInstance(character.id);

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

        self.last_data_header_text = format!("{} - Item Template", item.name);
        if let Some(text) = ui.get_text("Data Editor Header Text") {
            text.set_text(self.last_data_header_text.clone());
            ctx.ui.relayout = true;
        }

        if let Some(button) = ui.get_group_button("Code Template Switch") {
            button.set_index(0);
        }

        self.code_content = ContentContext::ItemTemplate(item.id);

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

        self.code_content = ContentContext::ItemInstance(item.id);
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
        let handled = match self.code_content {
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
        *CODEGRIDFX.write().unwrap() = Module::as_type(ModuleType::CharacterTemplate);
        CODEGRIDFX.write().unwrap().redraw(ui, ctx);

        if let Some(text) = ui.get_text("Code Editor Header Text") {
            self.last_header_text = "Undefined".to_string();
            text.set_text(self.last_header_text.clone());
            ctx.ui.relayout = true;
        }

        self.code_content = ContentContext::Unknown;
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
