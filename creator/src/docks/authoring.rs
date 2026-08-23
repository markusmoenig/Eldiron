use crate::docks::code_undo::{CodeUndo, CodeUndoAtom};
use crate::prelude::*;
use theframework::prelude::*;
use theframework::theui::thewidget::thetextedit::TheTextEditState;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum EntityKey {
    RegionSector(Uuid, Uuid),
    RegionLinedef(Uuid, Uuid),
    RegionGeometryObject(Uuid, Uuid),
    CharacterTemplate(Uuid),
    ItemTemplate(Uuid),
    PrefabAsset(Uuid),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AuthoringTarget {
    Sector(Uuid, u32, Uuid),
    Linedef(Uuid, u32, Uuid),
    GeometryObject(Uuid, Uuid),
    CharacterTemplate(Uuid),
    ItemTemplate(Uuid),
    PrefabAsset(Uuid),
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
            Self::GeometryObject(region_id, object_id) => {
                EntityKey::RegionGeometryObject(region_id, object_id)
            }
            Self::CharacterTemplate(id) => EntityKey::CharacterTemplate(id),
            Self::ItemTemplate(id) => EntityKey::ItemTemplate(id),
            Self::PrefabAsset(id) => EntityKey::PrefabAsset(id),
        }
    }

    fn title(self) -> String {
        match self {
            Self::Sector(_, id, _) => format!("{} {}", fl!("authoring_target_sector"), id),
            Self::Linedef(_, id, _) => format!("{} {}", fl!("authoring_target_linedef"), id),
            Self::GeometryObject(_, _) => fl!("authoring_target_geometry_object"),
            Self::CharacterTemplate(_) => fl!("authoring_target_character"),
            Self::ItemTemplate(_) => fl!("authoring_target_item"),
            Self::PrefabAsset(_) => fl!("authoring_target_prefab"),
        }
    }

    fn region_id(self) -> Option<Uuid> {
        match self {
            Self::Sector(region_id, ..)
            | Self::Linedef(region_id, ..)
            | Self::GeometryObject(region_id, ..) => Some(region_id),
            Self::CharacterTemplate(_) | Self::ItemTemplate(_) | Self::PrefabAsset(_) => None,
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

    fn reset_for_project_switch(&mut self) {
        self.entity_undos.clear();
        self.current_entity = None;
        self.prev_state = None;
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
            AuthoringTarget::PrefabAsset(..) => {
                "title = \"\"\ndescription = \"\"\"\n\"\"\"\n".to_string()
            }
            _ => "title = \"\"\ndescription = \"\"\"\n\"\"\"\n".to_string(),
        }
    }

    fn current_target(
        &self,
        project: &Project,
        server_ctx: &ServerContext,
    ) -> Option<AuthoringTarget> {
        // Linked Prefab geometry is rendered with derived object IDs and is not
        // stored in `map.geometry_objects`. The Geometry tool records the
        // owning instance instead, so resolve that selection back to the shared
        // source asset whose authoring data all linked instances use.
        if let Some(region) = project.get_region(&server_ctx.curr_region)
            && let Some(instance_id) = region.map.selected_block_prop_instances.first()
            && let Some(instance) = region
                .map
                .block_prop_instances
                .iter()
                .find(|instance| instance.id == *instance_id)
            && project.block_props.contains_key(&instance.asset_id)
        {
            return Some(AuthoringTarget::PrefabAsset(instance.asset_id));
        }

        if server_ctx.block_tool_active
            && let Some(asset_id) = server_ctx.curr_block_asset_id
            && project.block_props.contains_key(&asset_id)
        {
            return Some(AuthoringTarget::PrefabAsset(asset_id));
        }

        let region = project.get_region(&server_ctx.curr_region)?;
        let map = &region.map;

        // A 3D object selection is the most specific map authoring target. It
        // must win over stale 2D selections when switching editor views.
        if let Some(object_id) = map.selected_geometry_objects.first().copied()
            && map
                .geometry_objects
                .iter()
                .any(|object| object.id == object_id)
        {
            return Some(AuthoringTarget::GeometryObject(
                server_ctx.curr_region,
                object_id,
            ));
        }

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
            | ProjectContext::CharacterPreviewRigging(id)
                if project.characters.contains_key(&id) =>
            {
                return Some(AuthoringTarget::CharacterTemplate(id));
            }
            ProjectContext::Item(id)
            | ProjectContext::ItemData(id)
            | ProjectContext::ItemCode(id)
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
            AuthoringTarget::GeometryObject(_, object_id) => project
                .get_region(&target.region_id()?)?
                .map
                .geometry_objects
                .iter()
                .find(|object| object.id == object_id)
                .map(|object| object.properties.get_str_default("data", "".into())),
            AuthoringTarget::CharacterTemplate(id) => project
                .characters
                .get(&id)
                .map(|character| character.authoring.clone()),
            AuthoringTarget::ItemTemplate(id) => {
                project.items.get(&id).map(|item| item.authoring.clone())
            }
            AuthoringTarget::PrefabAsset(id) => project
                .block_props
                .get(&id)
                .map(|asset| asset.authoring.clone()),
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
            AuthoringTarget::GeometryObject(_, object_id) => project
                .get_region(&target.region_id().unwrap())
                .and_then(|region| {
                    region
                        .map
                        .geometry_objects
                        .iter()
                        .find(|object| object.id == object_id)
                })
                .map(|object| object.name.clone())
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
            AuthoringTarget::PrefabAsset(id) => project
                .block_props
                .get(&id)
                .map(|asset| asset.name.clone())
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
            AuthoringTarget::GeometryObject(_, object_id) => {
                if let Some(region) = project.get_region_mut(&target.region_id().unwrap())
                    && let Some(object) = region
                        .map
                        .geometry_objects
                        .iter_mut()
                        .find(|object| object.id == object_id)
                {
                    object.properties.set("data".into(), Value::Str(text));
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
            AuthoringTarget::PrefabAsset(id) => {
                if let Some(asset) = project.block_props.get_mut(&id) {
                    asset.authoring = text;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn project_with_selected_geometry_object() -> (Project, ServerContext, Uuid) {
        let mut project = Project::default();
        let region = &mut project.regions[0];
        let region_id = region.id;
        let object = rusterix::GeometryObject::box_("Ancient Pillar", Vec3::zero(), Vec3::one());
        let object_id = object.id;
        region.map.geometry_objects.push(object);
        region.map.selected_geometry_objects = vec![object_id];

        let mut server_ctx = ServerContext::default();
        server_ctx.curr_region = region_id;
        server_ctx.pc = ProjectContext::Region(region_id);
        (project, server_ctx, object_id)
    }

    #[test]
    fn selected_geometry_object_is_an_authoring_target() {
        let (project, server_ctx, object_id) = project_with_selected_geometry_object();
        let dock = AuthoringDock::new();

        assert_eq!(
            dock.current_target(&project, &server_ctx),
            Some(AuthoringTarget::GeometryObject(
                server_ctx.curr_region,
                object_id
            ))
        );
    }

    #[test]
    fn geometry_object_authoring_uses_its_data_property() {
        let (mut project, server_ctx, object_id) = project_with_selected_geometry_object();
        let dock = AuthoringDock::new();
        let target = AuthoringTarget::GeometryObject(server_ctx.curr_region, object_id);

        assert_eq!(
            dock.read_target_text(&project, target),
            Some(dock.template_for_target(target))
        );

        let authored = "title = \"Pillar\"\ndescription = \"Weathered stone.\"\n".to_string();
        dock.write_current(&mut project, &server_ctx, authored.clone());

        assert_eq!(dock.read_target_text(&project, target), Some(authored));
        assert_eq!(
            dock.target_display_title(&project, target),
            "Ancient Pillar"
        );
    }

    #[test]
    fn selected_linked_prefab_instance_targets_shared_asset_authoring() {
        let mut project = Project::default();
        let region_id = project.regions[0].id;
        let source = rusterix::GeometryObject::box_("Source", Vec3::zero(), Vec3::one());
        let asset = rusterix::BlockPropAsset::new_authored("Linked Pillar", vec![source]);
        let asset_id = asset.id;
        project.block_props.insert(asset_id, asset);

        let instance = rusterix::BlockPropInstance::new(asset_id);
        let instance_id = instance.id;
        project.regions[0].map.block_prop_instances.push(instance);
        project.regions[0].map.selected_block_prop_instances = vec![instance_id];

        let mut server_ctx = ServerContext::default();
        server_ctx.curr_region = region_id;
        let dock = AuthoringDock::new();

        assert_eq!(
            dock.current_target(&project, &server_ctx),
            Some(AuthoringTarget::PrefabAsset(asset_id))
        );

        let authored = "title = \"Pillar\"\ndescription = \"Shared.\"\n".to_string();
        dock.write_current(&mut project, &server_ctx, authored.clone());
        assert_eq!(project.block_props[&asset_id].authoring, authored);
    }
}
