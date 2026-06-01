use crate::docks::code_undo::*;
use crate::prelude::*;
use rusterix::prelude::{EldrinDebugEntry, EldrinDebugModule, EldrinDebugTarget};
use theframework::prelude::*;
use theframework::theui::thewidget::thetextedit::TheTextEditState;

/// Unique identifier for entities being edited
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntityKey {
    World,
    Region(Uuid),
    CharacterInstance(Uuid, Uuid),
    Character(Uuid),
    ItemInstance(Uuid, Uuid),
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
        if server_ctx.pc.is_world_code() {
            ui.set_widget_value(
                "DockCodeEditor",
                ctx,
                TheValue::Text(project.world_source.clone()),
            );
            self.switch_to_entity(EntityKey::World, ctx);
        } else if let Some(region_id) = server_ctx.pc.id() {
            if server_ctx.pc.is_region_code() {
                if let Some(region) = project.get_region(&region_id) {
                    ui.set_widget_value(
                        "DockCodeEditor",
                        ctx,
                        TheValue::Text(region.source.clone()),
                    );
                    self.switch_to_entity(EntityKey::Region(region_id), ctx);
                }
            } else if let Some(instance_id) = server_ctx.pc.get_region_character_instance_id() {
                if let Some(region) = project.get_region(&region_id)
                    && let Some(character_instance) = region.characters.get(&instance_id)
                {
                    ui.set_widget_value(
                        "DockCodeEditor",
                        ctx,
                        TheValue::Text(character_instance.source.clone()),
                    );
                    self.switch_to_entity(
                        EntityKey::CharacterInstance(region_id, instance_id),
                        ctx,
                    );
                }
            } else if let Some(instance_id) = server_ctx.pc.get_region_item_instance_id() {
                if let Some(region) = project.get_region(&region_id)
                    && let Some(item_instance) = region.items.get(&instance_id)
                {
                    ui.set_widget_value(
                        "DockCodeEditor",
                        ctx,
                        TheValue::Text(item_instance.source.clone()),
                    );
                    self.switch_to_entity(EntityKey::ItemInstance(region_id, instance_id), ctx);
                }
            } else if server_ctx.pc.is_character() {
                let id = region_id;
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
                let id = region_id;
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

                    if server_ctx.pc.is_world_code() {
                        if let Some(code) = value.to_string() {
                            project.world_source = code.clone();
                            project.world_source_debug = code;
                            redraw = true;
                        }
                    } else if let Some(id) = server_ctx.pc.id() {
                        if server_ctx.pc.is_region_code() {
                            if let Some(code) = value.to_string()
                                && let Some(region) = project.get_region_mut(&id)
                            {
                                region.source = code.clone();
                                region.source_debug = code;
                                redraw = true;
                            }
                        } else if let Some(instance_id) =
                            server_ctx.pc.get_region_character_instance_id()
                        {
                            if let Some(code) = value.to_string()
                                && let Some(region) = project.get_region_mut(&id)
                                && let Some(character) = region.characters.get_mut(&instance_id)
                            {
                                character.source = code.clone();
                                character.source_debug = code;
                                redraw = true;
                            }
                        } else if let Some(instance_id) =
                            server_ctx.pc.get_region_item_instance_id()
                        {
                            if let Some(code) = value.to_string()
                                && let Some(region) = project.get_region_mut(&id)
                                && let Some(item) = region.items.get_mut(&instance_id)
                            {
                                item.source = code.clone();
                                item.source_debug = code;
                                redraw = true;
                            }
                        } else if server_ctx.pc.is_character() {
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

    fn apply_eldrin_debug_data(
        &mut self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &Project,
        server_ctx: &ServerContext,
        debug: &EldrinDebugModule,
    ) {
        let Some(target) = self.runtime_debug_target(project, server_ctx) else {
            if let Some(edit) = ui.get_text_area_edit("DockCodeEditor") {
                edit.set_debug_line(None);
                edit.set_debug_lines(&[]);
            }
            return;
        };

        if let Some(edit) = ui.get_text_area_edit("DockCodeEditor") {
            let source = edit.text();
            let lines = Self::debug_lines_for(debug, &target, &source);
            edit.set_debug_line(None);
            edit.set_debug_lines(&lines);
        }
    }

    fn supports_undo(&self) -> bool {
        true
    }

    fn has_changes(&self) -> bool {
        // Check if any entity has changes (index >= 0, meaning not fully undone)
        self.entity_undos.values().any(|undo| undo.has_changes())
    }

    fn mark_saved(&mut self) {
        for undo in self.entity_undos.values_mut() {
            undo.index = -1;
        }
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
        if let Some(edit) = ui.get_text_area_edit("DockCodeEditor") {
            let state = edit.get_state();
            let text = state.rows.join("\n");

            if server_ctx.pc.is_world_code() {
                project.world_source = text.clone();
                project.world_source_debug = text;
            } else if let Some(id) = server_ctx.pc.id() {
                if server_ctx.pc.is_region_code() {
                    if let Some(region) = project.get_region_mut(&id) {
                        region.source = text.clone();
                        region.source_debug = text;
                    }
                } else if let Some(instance_id) = server_ctx.pc.get_region_character_instance_id() {
                    if let Some(region) = project.get_region_mut(&id)
                        && let Some(character) = region.characters.get_mut(&instance_id)
                    {
                        character.source = text.clone();
                        character.source_debug = text;
                    }
                } else if let Some(instance_id) = server_ctx.pc.get_region_item_instance_id() {
                    if let Some(region) = project.get_region_mut(&id)
                        && let Some(item) = region.items.get_mut(&instance_id)
                    {
                        item.source = text.clone();
                        item.source_debug = text;
                    }
                } else if server_ctx.pc.is_character() {
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

    fn runtime_debug_target(
        &self,
        project: &Project,
        server_ctx: &ServerContext,
    ) -> Option<EldrinDebugTarget> {
        match self.current_entity {
            Some(EntityKey::World) => Some(EldrinDebugTarget::World),
            Some(EntityKey::Region(region_id)) => project
                .regions
                .iter()
                .position(|region| region.id == region_id)
                .map(|index| EldrinDebugTarget::Region(index as u32)),
            Some(EntityKey::CharacterInstance(region_id, instance_id)) => project
                .get_region(&region_id)?
                .map
                .entities
                .iter()
                .find(|entity| entity.creator_id == instance_id)
                .map(|entity| EldrinDebugTarget::Entity(entity.id)),
            Some(EntityKey::ItemInstance(region_id, instance_id)) => project
                .get_region(&region_id)?
                .map
                .items
                .iter()
                .find(|item| item.creator_id == instance_id)
                .map(|item| EldrinDebugTarget::Item(item.id)),
            Some(EntityKey::Character(template_id)) => {
                let region = project.get_region(&server_ctx.curr_region)?;
                region
                    .characters
                    .values()
                    .find(|instance| instance.character_id == template_id)
                    .and_then(|instance| {
                        region
                            .map
                            .entities
                            .iter()
                            .find(|entity| entity.creator_id == instance.id)
                            .map(|entity| EldrinDebugTarget::Entity(entity.id))
                    })
            }
            Some(EntityKey::Item(template_id)) => {
                let region = project.get_region(&server_ctx.curr_region)?;
                region
                    .items
                    .values()
                    .find(|instance| instance.item_id == template_id)
                    .and_then(|instance| {
                        region
                            .map
                            .items
                            .iter()
                            .find(|item| item.creator_id == instance.id)
                            .map(|item| EldrinDebugTarget::Item(item.id))
                    })
            }
            None => None,
        }
    }

    fn debug_lines_for(
        debug: &EldrinDebugModule,
        target: &EldrinDebugTarget,
        source: &str,
    ) -> Vec<(usize, Option<bool>, Vec<String>)> {
        let mut rows: Vec<(usize, Option<bool>, Vec<String>)> = vec![];

        if let Some(frame) = debug.latest_frame_for(target) {
            let suppressed_lines = Self::false_branch_body_lines(source, &frame.entries);
            for entry in &frame.entries {
                match entry {
                    EldrinDebugEntry::ExecutedLine { line } => {
                        if suppressed_lines.contains(line) {
                            continue;
                        }
                        if let Some(row) = line.checked_sub(1) {
                            Self::ensure_debug_row(&mut rows, row);
                        }
                    }
                    EldrinDebugEntry::Branch { line, taken } => {
                        if !taken {
                            continue;
                        }
                        if let Some(row) = line.checked_sub(1) {
                            let index = Self::ensure_debug_row(&mut rows, row);
                            rows[index].1 = Some(*taken);
                        }
                    }
                    EldrinDebugEntry::Value { line, name, value } => {
                        if suppressed_lines.contains(line) {
                            continue;
                        }
                        if let Some(row) = line.checked_sub(1) {
                            if Self::is_internal_debug_name(name) {
                                continue;
                            }
                            let index = Self::ensure_debug_row(&mut rows, row);
                            let prefix = format!("{name} = ");
                            rows[index].2.retain(|value| !value.starts_with(&prefix));
                            rows[index].2.push(format!("{name} = {value}"));
                            if rows[index].2.len() > 6 {
                                rows[index].2.remove(0);
                            }
                        }
                    }
                }
            }
        }

        rows
    }

    fn false_branch_body_lines(source: &str, entries: &[EldrinDebugEntry]) -> FxHashSet<usize> {
        let lines = source.lines().collect::<Vec<_>>();
        let mut suppressed = FxHashSet::default();
        for entry in entries {
            let EldrinDebugEntry::Branch { line, taken: false } = entry else {
                continue;
            };
            if let Some(end_line) = Self::branch_body_end_line(&lines, *line) {
                for suppressed_line in line.saturating_add(1)..end_line {
                    suppressed.insert(suppressed_line);
                }
            }
        }
        suppressed
    }

    fn branch_body_end_line(lines: &[&str], branch_line: usize) -> Option<usize> {
        let mut depth = 0usize;
        let mut found_body = false;
        for (index, line) in lines.iter().enumerate().skip(branch_line.saturating_sub(1)) {
            for ch in line.chars() {
                match ch {
                    '{' => {
                        found_body = true;
                        depth += 1;
                    }
                    '}' if found_body => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            return Some(index + 1);
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }

    fn ensure_debug_row(rows: &mut Vec<(usize, Option<bool>, Vec<String>)>, row: usize) -> usize {
        if let Some(index) = rows.iter().position(|(existing, _, _)| *existing == row) {
            let entry = rows.remove(index);
            rows.push(entry);
        } else {
            rows.push((row, None, vec![]));
        }
        if rows.len() > 24 {
            rows.remove(0);
        }
        rows.len() - 1
    }

    fn is_internal_debug_name(name: &str) -> bool {
        name.starts_with("__")
            || name
                .split('.')
                .any(|segment| segment.starts_with("__") || segment.starts_with("_cgfx"))
    }
}
