use crate::docks::code_undo::*;
use crate::prelude::*;
use theframework::prelude::*;
use theframework::theui::thewidget::thetextedit::TheTextEditState;

/// Unique identifier for entities being edited
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntityKey {
    Character(Uuid),
    Item(Uuid),
}

pub struct CodeDock {
    // Per-entity undo stacks
    entity_undos: FxHashMap<EntityKey, CodeUndo>,
    current_entity: Option<EntityKey>,
    max_undo: usize,
    prev_state: Option<TheTextEditState>,
}

impl Dock for CodeDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            entity_undos: FxHashMap::default(),
            current_entity: None,
            max_undo: 30,
            prev_state: None,
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut textedit = TheTextAreaEdit::new(TheId::named("DockCodeEditor"));

        if let Some(bytes) = crate::Embedded::get("parser/eldrin.sublime-syntax") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                textedit.add_syntax_from_string(source);
                textedit.set_code_type("Eldrin Script");
            }
        }

        if let Some(bytes) = crate::Embedded::get("parser/gruvbox-dark.tmTheme") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                textedit.add_theme_from_string(source);
                textedit.set_code_theme("Gruvbox Dark");
            }
        }

        textedit.set_continuous(true);
        textedit.display_line_number(true);
        // textedit.set_code_theme("base16-eighties.dark");
        textedit.use_global_statusbar(true);
        textedit.set_font_size(14.0);
        // Tell the widget we handle undo/redo manually here
        textedit.set_supports_undo(false);
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
                    // Switch to this entity's undo stack
                    self.switch_to_entity(EntityKey::Character(id), ctx);
                }
            } else if server_ctx.pc.is_item() {
                if let Some(item) = project.items.get(&id) {
                    ui.set_widget_value("DockCodeEditor", ctx, TheValue::Text(item.source.clone()));
                    // Switch to this entity's undo stack
                    self.switch_to_entity(EntityKey::Item(id), ctx);
                }
            }
        }

        // Store initial state for undo
        if let Some(edit) = ui.get_text_area_edit("DockCodeEditor") {
            self.prev_state = Some(edit.get_state());
        }
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::ValueChanged(id, value) => {
                if id.name == "DockCodeEditor" {
                    if let Some(edit) = ui.get_text_area_edit("DockCodeEditor") {
                        // Add undo atom before applying the change
                        if let Some(prev) = &self.prev_state {
                            let current_state = edit.get_state();
                            let atom = CodeUndoAtom::TextEdit(prev.clone(), current_state.clone());
                            self.add_undo(atom, ctx);
                            self.prev_state = Some(current_state);
                        }
                    }

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

    fn supports_undo(&self) -> bool {
        true
    }

    fn has_changes(&self) -> bool {
        // Check if any entity has changes (index >= 0, meaning not fully undone)
        self.entity_undos.values().any(|undo| undo.has_changes())
    }

    fn undo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(entity_key) = self.current_entity {
            if let Some(undo) = self.entity_undos.get_mut(&entity_key) {
                if let Some(edit) = ui.get_text_area_edit("DockCodeEditor") {
                    undo.undo(edit);
                    self.prev_state = Some(edit.get_state());
                    self.set_undo_state_to_ui(ctx);

                    // Update the project with the undone text
                    self.update_project_code(ui, project, server_ctx);
                }
            }
        }
    }

    fn redo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(entity_key) = self.current_entity {
            if let Some(undo) = self.entity_undos.get_mut(&entity_key) {
                if let Some(edit) = ui.get_text_area_edit("DockCodeEditor") {
                    undo.redo(edit);
                    self.prev_state = Some(edit.get_state());
                    self.set_undo_state_to_ui(ctx);

                    // Update the project with the redone text
                    self.update_project_code(ui, project, server_ctx);
                }
            }
        }
    }

    fn set_undo_state_to_ui(&self, ctx: &mut TheContext) {
        if let Some(entity_key) = self.current_entity {
            if let Some(undo) = self.entity_undos.get(&entity_key) {
                if undo.has_undo() {
                    ctx.ui.set_enabled("Undo");
                } else {
                    ctx.ui.set_disabled("Undo");
                }

                if undo.has_redo() {
                    ctx.ui.set_enabled("Redo");
                } else {
                    ctx.ui.set_disabled("Redo");
                }
                return;
            }
        }

        // No entity selected or no undo stack
        ctx.ui.set_disabled("Undo");
        ctx.ui.set_disabled("Redo");
    }
}

impl CodeDock {
    /// Switch to a different entity and update undo button states
    fn switch_to_entity(&mut self, entity_key: EntityKey, ctx: &mut TheContext) {
        self.current_entity = Some(entity_key);
        self.set_undo_state_to_ui(ctx);
    }

    /// Add an undo atom to the current entity's undo stack
    fn add_undo(&mut self, atom: CodeUndoAtom, ctx: &mut TheContext) {
        if let Some(entity_key) = self.current_entity {
            let undo = self
                .entity_undos
                .entry(entity_key)
                .or_insert_with(CodeUndo::new);
            undo.add(atom);
            undo.truncate_to_limit(self.max_undo);
            self.set_undo_state_to_ui(ctx);
        }
    }

    /// Update the project with the current text state
    fn update_project_code(
        &mut self,
        ui: &mut TheUI,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(id) = server_ctx.pc.id() {
            if let Some(edit) = ui.get_text_area_edit("DockCodeEditor") {
                let state = edit.get_state();
                let text = state.rows.join("\n");

                if server_ctx.pc.is_character() {
                    if let Some(character) = project.characters.get_mut(&id) {
                        character.source = text.clone();
                        character.source_debug = text;
                    }
                } else if server_ctx.pc.is_item() {
                    if let Some(item) = project.items.get_mut(&id) {
                        item.source = text.clone();
                        item.source_debug = text;
                    }
                }
            }
        }
    }
}
