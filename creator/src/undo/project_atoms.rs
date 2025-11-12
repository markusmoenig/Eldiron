// use crate::editor::DOCKMANAGER;
use crate::prelude::*;
use theframework::prelude::*;

// #[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum ProjectUndoAtom {
    MapEdit(ProjectContext, Box<Map>, Box<Map>),
    AddRegion(Region),
    RemoveRegion(usize, Region),
    RenameRegion(Uuid, String, String),
    AddCharacter(Character),
    RemoveCharacter(usize, Character),
    RenameCharacter(Uuid, String, String),
    AddItem(Item),
    RemoveItem(usize, Item),
    RenameItem(Uuid, String, String),
}

use ProjectUndoAtom::*;

impl ProjectUndoAtom {
    /// Returns the ProjectContext for the MapEdit
    pub fn pc(&self) -> Option<ProjectContext> {
        match self {
            MapEdit(pc, _, _) => Some(*pc),
            _ => None,
        }
    }

    /// Descriptive text of the undo atom.
    pub fn to_string(&self) -> String {
        match self {
            MapEdit(_, _, _) => "Map Edit".to_string(),
            AddRegion(region) => format!("Add Region: {}", region.name),
            RemoveRegion(_, region) => format!("Remove Region: {}", region.name),
            RenameRegion(_, old, new) => format!("Rename Region: {} -> {}", old, new),
            AddCharacter(character) => format!("Add Character: {}", character.name),
            RemoveCharacter(_, character) => format!("Remove Character: {}", character.name),
            RenameCharacter(_, old, new) => format!("Rename Character: {} -> {}", old, new),
            AddItem(character) => format!("Add Item: {}", character.name),
            RemoveItem(_, character) => format!("Remove Item: {}", character.name),
            RenameItem(_, old, new) => format!("Rename Item: {} -> {}", old, new),
        }
    }

    pub fn undo(
        &self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        match self {
            MapEdit(pc, old, _new) => {
                set_project_context(ctx, ui, project, server_ctx, *pc);
                if let Some(map) = project.get_map_pc_mut(server_ctx) {
                    *map = *old.clone();
                    update_region(ctx);
                }
            }
            AddRegion(region) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(region_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_regions_id)
                    {
                        project.remove_region(&region.id);
                        region_node.remove_child_by_uuid(&region.id);
                    }
                }
            }
            RemoveRegion(index, region) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    let mut region = region.clone();
                    region.map.name = region.name.clone();

                    let mut node = gen_region_tree_node(&region);
                    node.set_open(true);
                    if let Some(region_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_regions_id)
                    {
                        region_node.add_child_at(*index, node);
                    }
                    let region_id: Uuid = region.id;
                    project.regions.insert(*index, region);

                    server_ctx.curr_region = region_id;
                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::Region(region_id),
                    );
                    update_region(ctx);
                }
            }
            RenameRegion(id, old, _new) => {
                if let Some(region) = project.get_region_mut(id) {
                    region.name = old.clone();
                    region.map.name = old.clone();
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(region_node) = tree_layout.get_node_by_id_mut(&region.id) {
                            region_node.widget.set_value(TheValue::Text(old.clone()));
                        }
                    }
                }
            }
            AddCharacter(character) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(region_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_characters_id)
                    {
                        project.remove_character(&character.id);
                        region_node.remove_child_by_uuid(&character.id);
                    }
                }
            }
            RemoveCharacter(index, character) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    let character = character.clone();

                    let mut node = gen_character_tree_node(&character);
                    node.set_open(true);
                    if let Some(character_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_characters_id)
                    {
                        character_node.add_child_at(*index, node);
                    }
                    let character_id: Uuid = character.id;
                    project
                        .characters
                        .insert_before(*index, character_id, character);

                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::Character(character_id),
                    );
                    update_region(ctx);
                }
            }
            RenameCharacter(id, old, _new) => {
                if let Some(character) = project.characters.get_mut(id) {
                    character.name = old.clone();
                    character.map.name = old.clone();
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(region_node) = tree_layout.get_node_by_id_mut(&character.id) {
                            region_node.widget.set_value(TheValue::Text(old.clone()));
                        }
                    }
                }
            }
            AddItem(item) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(region_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_items_id)
                    {
                        project.remove_item(&item.id);
                        region_node.remove_child_by_uuid(&item.id);
                    }
                }
            }
            RemoveItem(index, item) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    let item = item.clone();

                    let mut node = gen_item_tree_node(&item);
                    node.set_open(true);
                    if let Some(item_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_items_id)
                    {
                        item_node.add_child_at(*index, node);
                    }
                    let item_id: Uuid = item.id;
                    project.items.insert_before(*index, item_id, item);

                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::Item(item_id),
                    );
                    update_region(ctx);
                }
            }
            RenameItem(id, old, _new) => {
                if let Some(item) = project.items.get_mut(id) {
                    item.name = old.clone();
                    item.map.name = old.clone();
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(region_node) = tree_layout.get_node_by_id_mut(&item.id) {
                            region_node.widget.set_value(TheValue::Text(old.clone()));
                        }
                    }
                }
            }
        }
    }

    pub fn redo(
        &self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        match self {
            MapEdit(pc, _old, new) => {
                set_project_context(ctx, ui, project, server_ctx, *pc);
                if let Some(map) = project.get_map_pc_mut(server_ctx) {
                    *map = *new.clone();
                    update_region(ctx);
                }
            }
            AddRegion(region) => {
                // Add Region
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    let mut region = region.clone();
                    region.map.name = region.name.clone();

                    let mut node = gen_region_tree_node(&region);
                    node.set_open(true);
                    if let Some(region_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_regions_id)
                    {
                        region_node.add_child(node);
                    }
                    let region_id: Uuid = region.id;
                    project.regions.push(region);

                    server_ctx.curr_region = region_id;
                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::Region(region_id),
                    );
                    update_region(ctx);
                }
            }
            RemoveRegion(_, region) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(region_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_regions_id)
                    {
                        region_node.remove_child_by_uuid(&region.id);
                    }
                    project.remove_region(&region.id);

                    if let Some(region) = project.regions.first() {
                        if let Some(region_node) = tree_layout.get_node_by_id_mut(&region.id) {
                            region_node.set_open(true);
                        }
                        server_ctx.curr_region = region.id;
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Region(region.id),
                        );
                        update_region(ctx);
                    }
                }
            }
            RenameRegion(id, _old, new) => {
                if let Some(region) = project.get_region_mut(id) {
                    region.name = new.clone();
                    region.map.name = new.clone();
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(region_node) = tree_layout.get_node_by_id_mut(&region.id) {
                            region_node.widget.set_value(TheValue::Text(new.clone()));
                        }
                    }
                }
            }
            AddCharacter(character) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_characters_id)
                    {
                        let mut character = character.clone();

                        if let Some(bytes) = crate::Embedded::get("python/basecharacter.py") {
                            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                                character.source = source.to_string();
                            }
                        }

                        if let Some(bytes) = crate::Embedded::get("toml/character.toml") {
                            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                                character.data = source.to_string();
                            }
                        }

                        character
                            .module
                            .set_module_type(codegridfx::ModuleType::CharacterTemplate);

                        let mut character_node = gen_character_tree_node(&character);
                        character_node.set_open(true);
                        node.add_child(character_node);

                        let character_id = character.id;
                        project.add_character(character);

                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Character(character_id),
                        );
                        update_region(ctx);
                    }
                }
            }
            RemoveCharacter(_, character) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(character_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_characters_id)
                    {
                        character_node.remove_child_by_uuid(&character.id);
                    }
                    project.remove_character(&character.id);

                    if let Some(character) = project.characters.first() {
                        if let Some(character_node) = tree_layout.get_node_by_id_mut(character.0) {
                            character_node.set_open(true);
                        }
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Character(*character.0),
                        );
                    }
                }
            }
            RenameCharacter(id, _old, new) => {
                if let Some(character) = project.characters.get_mut(id) {
                    character.name = new.clone();
                    character.map.name = new.clone();
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(region_node) = tree_layout.get_node_by_id_mut(&character.id) {
                            region_node.widget.set_value(TheValue::Text(new.clone()));
                        }
                    }
                }
            }
            AddItem(item) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_items_id) {
                        let mut item = item.clone();

                        if let Some(bytes) = crate::Embedded::get("python/baseitem.py") {
                            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                                item.source = source.to_string();
                            }
                        }

                        if let Some(bytes) = crate::Embedded::get("toml/item.toml") {
                            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                                item.data = source.to_string();
                            }
                        }

                        item.module
                            .set_module_type(codegridfx::ModuleType::ItemTemplate);

                        let mut item_node = gen_item_tree_node(&item);
                        item_node.set_open(true);
                        node.add_child(item_node);

                        let item_id = item.id;
                        project.add_item(item);

                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Character(item_id),
                        );
                        update_region(ctx);
                    }
                }
            }
            RemoveItem(_, item) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(character_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_items_id)
                    {
                        character_node.remove_child_by_uuid(&item.id);
                    }
                    project.remove_character(&item.id);

                    if let Some(item) = project.items.first() {
                        if let Some(item_node) = tree_layout.get_node_by_id_mut(item.0) {
                            item_node.set_open(true);
                        }
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Item(*item.0),
                        );
                    }
                }
            }
            RenameItem(id, _old, new) => {
                if let Some(item) = project.items.get_mut(id) {
                    item.name = new.clone();
                    item.map.name = new.clone();
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(item_node) = tree_layout.get_node_by_id_mut(&item.id) {
                            item_node.widget.set_value(TheValue::Text(new.clone()));
                        }
                    }
                }
            }
        }
    }
}
