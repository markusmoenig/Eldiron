// use crate::editor::DOCKMANAGER;
use crate::prelude::*;
use theframework::prelude::*;

// #[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ProjectUndoAtom {
    AddRegion(Uuid),
    RemoveRegion(usize, Region),
    RenameRegion(Uuid, String, String),
    AddCharacter(Uuid),
    RemoveCharacter(usize, Character),
}

use ProjectUndoAtom::*;

impl ProjectUndoAtom {
    pub fn undo(
        &self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        match self {
            AddRegion(id) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(region_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_regions_id)
                    {
                        project.remove_region(&id);
                        region_node.remove_child_by_uuid(id);
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
            AddCharacter(id) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(region_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_characters_id)
                    {
                        project.remove_character(id);
                        region_node.remove_child_by_uuid(id);
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
            AddRegion(id) => {
                // Add Region
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    let mut region = Region::new();
                    region.id = *id;
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
            AddCharacter(id) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_characters_id)
                    {
                        let mut character = Character::default();
                        character
                            .module
                            .set_module_type(codegridfx::ModuleType::CharacterTemplate);
                        character.id = *id;

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

                        let mut character_node = gen_character_tree_node(&character);
                        character_node.set_open(true);
                        node.add_child(character_node);

                        project.add_character(character);

                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Character(*id),
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
        }
    }
}
