use crate::docks::code_undo::{CodeUndo, CodeUndoAtom};
use crate::prelude::*;
use theframework::prelude::*;
use theframework::theui::thewidget::thetextedit::TheTextEditState;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum EntityKey {
    RegionSector(Uuid, Uuid),
    RegionLinedef(Uuid, Uuid),
    CharacterTemplate(Uuid),
    ItemTemplate(Uuid),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AuthoringTarget {
    Sector(Uuid, u32, Uuid),
    Linedef(Uuid, u32, Uuid),
    CharacterTemplate(Uuid),
    ItemTemplate(Uuid),
}

impl AuthoringTarget {
    fn entity_key(self) -> EntityKey {
        match self {
            Self::Sector(region_id, _, creator_id) => {
                EntityKey::RegionSector(region_id, creator_id)
            }
            Self::Linedef(region_id, _, creator_id) => {
                EntityKey::RegionLinedef(region_id, creator_id)
            }
            Self::CharacterTemplate(id) => EntityKey::CharacterTemplate(id),
            Self::ItemTemplate(id) => EntityKey::ItemTemplate(id),
        }
    }

    fn title(self) -> String {
        match self {
            Self::Sector(_, id, _) => format!("{} {}", fl!("authoring_target_sector"), id),
            Self::Linedef(_, id, _) => format!("{} {}", fl!("authoring_target_linedef"), id),
            Self::CharacterTemplate(_) => fl!("authoring_target_character"),
            Self::ItemTemplate(_) => fl!("authoring_target_item"),
        }
    }

    fn region_id(self) -> Option<Uuid> {
        match self {
            Self::Sector(region_id, ..) | Self::Linedef(region_id, ..) => Some(region_id),
            Self::CharacterTemplate(_) | Self::ItemTemplate(_) => None,
        }
    }
}

pub struct AuthoringDock {
    entity_undos: FxHashMap<EntityKey, CodeUndo>,
    current_entity: Option<EntityKey>,
    max_undo: usize,
    prev_state: Option<TheTextEditState>,
}

impl Dock for AuthoringDock {
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

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        toolbar_hlayout.set_padding(3);

        let mut title = TheText::new(TheId::named("Authoring Dock Title"));
        title.set_text(fl!("authoring_select_prompt"));
        title.set_text_size(12.0);
        toolbar_hlayout.add_widget(Box::new(title));

        toolbar_canvas.set_layout(toolbar_hlayout);
        center.set_top(toolbar_canvas);

        let mut textedit = TheTextAreaEdit::new(TheId::named("DockAuthoringEditor"));
        if let Some(bytes) = crate::Embedded::get("parser/TOML.sublime-syntax") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                textedit.add_syntax_from_string(source);
                textedit.set_code_type("TOML");
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
        textedit.use_global_statusbar(true);
        textedit.set_font_size(14.0);
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
        self.refresh_from_selection(ui, ctx, project, server_ctx);
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match event {
            TheEvent::ValueChanged(id, value) if id.name == "DockAuthoringEditor" => {
                if let Some(edit) = ui.get_text_area_edit("DockAuthoringEditor")
                    && let Some(prev) = &self.prev_state
                {
                    let current_state = edit.get_state();
                    let atom = CodeUndoAtom::TextEdit(prev.clone(), current_state.clone());
                    self.add_undo(atom, ctx);
                    self.prev_state = Some(current_state);
                }

                if let Some(text) = value.to_string() {
                    self.write_current(project, server_ctx, text);
                }
                true
            }
            TheEvent::Custom(id, _) if id.name == "Map Selection Changed" => {
                self.refresh_from_selection(ui, ctx, project, server_ctx);
                false
            }
            TheEvent::StateChanged(id, _)
                if id.name == "Region Content List Item"
                    || id.name == "Screen Content List Item"
                    || id.name == "Character Item"
                    || id.name == "Character Item Name Edit"
                    || id.name == "Character Item Data Edit"
                    || id.name == "Item Item"
                    || id.name == "Item Item Name Edit"
                    || id.name == "Item Item Data Edit" =>
            {
                self.refresh_from_content_item(ui, ctx, project, server_ctx, id);
                false
            }
            _ => false,
        }
    }

    fn supports_undo(&self) -> bool {
        true
    }

    fn has_changes(&self) -> bool {
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
        if let Some(entity_key) = self.current_entity
            && let Some(undo) = self.entity_undos.get_mut(&entity_key)
            && let Some(edit) = ui.get_text_area_edit("DockAuthoringEditor")
        {
            undo.undo(edit);
            self.prev_state = Some(edit.get_state());
            self.set_undo_state_to_ui(ctx);
            self.write_current(project, server_ctx, edit.text());
        }
    }

    fn redo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(entity_key) = self.current_entity
            && let Some(undo) = self.entity_undos.get_mut(&entity_key)
            && let Some(edit) = ui.get_text_area_edit("DockAuthoringEditor")
        {
            undo.redo(edit);
            self.prev_state = Some(edit.get_state());
            self.set_undo_state_to_ui(ctx);
            self.write_current(project, server_ctx, edit.text());
        }
    }

    fn set_undo_state_to_ui(&self, ctx: &mut TheContext) {
        if let Some(entity_key) = self.current_entity
            && let Some(undo) = self.entity_undos.get(&entity_key)
        {
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

        ctx.ui.set_disabled("Undo");
        ctx.ui.set_disabled("Redo");
    }
}

impl AuthoringDock {
    fn target_from_content_item(
        &self,
        project: &Project,
        server_ctx: &ServerContext,
        id: &TheId,
    ) -> Option<AuthoringTarget> {
        if matches!(
            id.name.as_str(),
            "Character Item" | "Character Item Name Edit" | "Character Item Data Edit"
        ) && project.characters.contains_key(&id.references)
        {
            return Some(AuthoringTarget::CharacterTemplate(id.references));
        }

        if matches!(
            id.name.as_str(),
            "Item Item" | "Item Item Name Edit" | "Item Item Data Edit"
        ) && project.items.contains_key(&id.references)
        {
            return Some(AuthoringTarget::ItemTemplate(id.references));
        }

        let region = project.get_region(&server_ctx.curr_region)?;
        if id.name == "Screen Content List Item" || id.name == "Region Content List Item" {
            if let Some(sector) = region
                .map
                .sectors
                .iter()
                .find(|sector| sector.creator_id == id.uuid)
            {
                return Some(AuthoringTarget::Sector(
                    server_ctx.curr_region,
                    sector.id,
                    sector.creator_id,
                ));
            }
            if let Some(linedef) = region
                .map
                .linedefs
                .iter()
                .find(|linedef| linedef.creator_id == id.uuid)
            {
                return Some(AuthoringTarget::Linedef(
                    server_ctx.curr_region,
                    linedef.id,
                    linedef.creator_id,
                ));
            }
        }

        None
    }

    fn template_for_target(&self, target: AuthoringTarget) -> String {
        match target {
            AuthoringTarget::CharacterTemplate(..) => {
                "title = \"\"\ndescription = \"\"\"\n\"\"\"\n\n[mode.active]\ndescription = \"\"\"\n\"\"\"\n\n[mode.dead]\ndescription = \"\"\"\n\"\"\"\n"
                    .to_string()
            }
            AuthoringTarget::ItemTemplate(..) => {
                "title = \"\"\ndescription = \"\"\"\n\"\"\"\n\n[state.off]\ndescription = \"\"\"\n\"\"\"\n\n[state.on]\ndescription = \"\"\"\n\"\"\"\n"
                    .to_string()
            }
            _ => "title = \"\"\ndescription = \"\"\"\n\"\"\"\n".to_string(),
        }
    }

    fn current_target(
        &self,
        project: &Project,
        server_ctx: &ServerContext,
    ) -> Option<AuthoringTarget> {
        let region = project.get_region(&server_ctx.curr_region)?;
        let map = &region.map;

        if let Some(sector_id) = map.selected_sectors.first().copied()
            && let Some(sector) = map.find_sector(sector_id)
        {
            return Some(AuthoringTarget::Sector(
                server_ctx.curr_region,
                sector.id,
                sector.creator_id,
            ));
        }

        if let Some(linedef_id) = map.selected_linedefs.first().copied()
            && let Some(linedef) = map.find_linedef(linedef_id)
        {
            return Some(AuthoringTarget::Linedef(
                server_ctx.curr_region,
                linedef.id,
                linedef.creator_id,
            ));
        }

        match server_ctx.pc {
            ProjectContext::Character(id)
            | ProjectContext::CharacterData(id)
            | ProjectContext::CharacterCode(id)
            | ProjectContext::CharacterVisualCode(id)
            | ProjectContext::CharacterPreviewRigging(id)
                if project.characters.contains_key(&id) =>
            {
                return Some(AuthoringTarget::CharacterTemplate(id));
            }
            ProjectContext::Item(id)
            | ProjectContext::ItemData(id)
            | ProjectContext::ItemCode(id)
            | ProjectContext::ItemVisualCode(id)
                if project.items.contains_key(&id) =>
            {
                return Some(AuthoringTarget::ItemTemplate(id));
            }
            _ => {}
        }

        match server_ctx.curr_region_content {
            ContentContext::Sector(creator_id) => {
                if let Some(sector) = map
                    .sectors
                    .iter()
                    .find(|sector| sector.creator_id == creator_id)
                {
                    return Some(AuthoringTarget::Sector(
                        server_ctx.curr_region,
                        sector.id,
                        sector.creator_id,
                    ));
                }
            }
            _ => {}
        }

        match server_ctx.curr_character {
            ContentContext::CharacterTemplate(id) if project.characters.contains_key(&id) => {
                return Some(AuthoringTarget::CharacterTemplate(id));
            }
            _ => {}
        }

        match server_ctx.curr_item {
            ContentContext::ItemTemplate(id) if project.items.contains_key(&id) => {
                return Some(AuthoringTarget::ItemTemplate(id));
            }
            _ => {}
        }

        None
    }

    fn read_target_text(&self, project: &Project, target: AuthoringTarget) -> Option<String> {
        let text = match target {
            AuthoringTarget::Sector(_, id, _) => project
                .get_region(&target.region_id()?)?
                .map
                .find_sector(id)
                .map(|sector| sector.properties.get_str_default("data", "".into())),
            AuthoringTarget::Linedef(_, id, _) => project
                .get_region(&target.region_id()?)?
                .map
                .find_linedef(id)
                .map(|linedef| linedef.properties.get_str_default("data", "".into())),
            AuthoringTarget::CharacterTemplate(id) => project
                .characters
                .get(&id)
                .map(|character| character.authoring.clone()),
            AuthoringTarget::ItemTemplate(id) => {
                project.items.get(&id).map(|item| item.authoring.clone())
            }
        }?;

        if text.trim().is_empty() {
            Some(self.template_for_target(target))
        } else {
            Some(text)
        }
    }

    fn target_display_title(&self, project: &Project, target: AuthoringTarget) -> String {
        match target {
            AuthoringTarget::Sector(_, id, _) => project
                .get_region(&target.region_id().unwrap())
                .and_then(|region| region.map.find_sector(id))
                .map(|sector| sector.name.clone())
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| target.title()),
            AuthoringTarget::Linedef(_, id, _) => project
                .get_region(&target.region_id().unwrap())
                .and_then(|region| region.map.find_linedef(id))
                .map(|linedef| linedef.name.clone())
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| target.title()),
            AuthoringTarget::CharacterTemplate(id) => project
                .characters
                .get(&id)
                .map(|character| character.name.clone())
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| target.title()),
            AuthoringTarget::ItemTemplate(id) => project
                .items
                .get(&id)
                .map(|item| item.name.clone())
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| target.title()),
        }
    }

    fn write_current(&self, project: &mut Project, server_ctx: &ServerContext, text: String) {
        let Some(target) = self.current_target(project, server_ctx) else {
            return;
        };

        match target {
            AuthoringTarget::Sector(_, id, _) => {
                if let Some(region) = project.get_region_mut(&target.region_id().unwrap())
                    && let Some(sector) = region.map.find_sector_mut(id)
                {
                    sector.properties.set("data".into(), Value::Str(text));
                }
            }
            AuthoringTarget::Linedef(_, id, _) => {
                if let Some(region) = project.get_region_mut(&target.region_id().unwrap())
                    && let Some(linedef) = region.map.find_linedef_mut(id)
                {
                    linedef.properties.set("data".into(), Value::Str(text));
                }
            }
            AuthoringTarget::CharacterTemplate(id) => {
                if let Some(character) = project.characters.get_mut(&id) {
                    character.authoring = text;
                }
            }
            AuthoringTarget::ItemTemplate(id) => {
                if let Some(item) = project.items.get_mut(&id) {
                    item.authoring = text;
                }
            }
        }
    }

    fn refresh_from_selection(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &ServerContext,
    ) {
        let target = self.current_target(project, server_ctx);
        self.apply_target_to_ui(target, ui, ctx, project);
    }

    fn refresh_from_content_item(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &ServerContext,
        id: &TheId,
    ) {
        let target = self
            .target_from_content_item(project, server_ctx, id)
            .or_else(|| self.current_target(project, server_ctx));
        self.apply_target_to_ui(target, ui, ctx, project);
    }

    fn apply_target_to_ui(
        &mut self,
        target: Option<AuthoringTarget>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
    ) {
        let text = target
            .and_then(|target| self.read_target_text(project, target))
            .unwrap_or_default();
        ui.set_widget_value("DockAuthoringEditor", ctx, TheValue::Text(text));

        let title = target
            .map(|target| {
                format!(
                    "{} {}",
                    fl!("authoring_title_prefix"),
                    self.target_display_title(project, target)
                )
            })
            .unwrap_or_else(|| fl!("authoring_select_prompt"));
        ui.set_widget_value("Authoring Dock Title", ctx, TheValue::Text(title));

        self.current_entity = target.map(|target| target.entity_key());
        self.set_undo_state_to_ui(ctx);

        if let Some(edit) = ui.get_text_area_edit("DockAuthoringEditor") {
            self.prev_state = Some(edit.get_state());
        } else {
            self.prev_state = None;
        }
    }

    fn add_undo(&mut self, atom: CodeUndoAtom, ctx: &mut TheContext) {
        if let Some(entity_key) = self.current_entity {
            let undo = self
                .entity_undos
                .entry(entity_key)
                .or_insert_with(CodeUndo::new);
            undo.add(atom);
            if undo.stack.len() > self.max_undo {
                undo.stack.remove(0);
                undo.index -= 1;
            }
            self.set_undo_state_to_ui(ctx);
        }
    }
}
