use crate::prelude::*;
use theframework::prelude::*;

// #[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum ProjectUndoAtom {
    MapEdit(ProjectContext, Box<Map>, Box<Map>),
    AddRegion(Region),
    RemoveRegion(usize, Region),
    RenameRegion(Uuid, String, String),
    AddRegionCharacterInstance(Uuid, Character),
    RemoveRegionCharacterInstance(usize, Uuid, Character),
    MoveRegionCharacterInstance(Uuid, Uuid, Vec3<f32>, Vec3<f32>), // region, instance, from, to
    AddRegionItemInstance(Uuid, Item),
    RemoveRegionItemInstance(usize, Uuid, Item),
    MoveRegionItemInstance(Uuid, Uuid, Vec3<f32>, Vec3<f32>), // region, instance, from, to
    AddCharacter(Character),
    RemoveCharacter(usize, Character),
    RenameCharacter(Uuid, String, String),
    AddItem(Item),
    RemoveItem(usize, Item),
    RenameItem(Uuid, String, String),
    AddTilemap(Tilemap),
    RemoveTilemap(usize, Tilemap),
    RenameTilemap(Uuid, String, String),
    EditTilemapGridSize(Uuid, i32, i32),
    AddScreen(Screen),
    RemoveScreen(usize, Screen),
    RenameScreen(Uuid, String, String),
    AddAsset(Asset),
    RemoveAsset(usize, Asset),
    RenameAsset(Uuid, String, String),
    AddAvatar(Avatar),
    RemoveAvatar(usize, Avatar),
    RenameAvatar(Uuid, String, String),
    EditAvatarResolution(Uuid, u16, u16),
    EditAvatarPerspectiveCount(Uuid, AvatarPerspectiveCount, AvatarPerspectiveCount),
    AddAvatarAnimation(Uuid, AvatarAnimation),
    RemoveAvatarAnimation(Uuid, usize, AvatarAnimation),
    RenameAvatarAnimation(Uuid, Uuid, String, String),
    EditAvatarAnimationFrameCount(Uuid, Uuid, usize, usize),
    EditAvatarAnimationSpeed(Uuid, Uuid, f32, f32),
    PaletteEdit(ThePalette, ThePalette),
    TileEdit(rusterix::Tile, rusterix::Tile),
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
            AddRegionCharacterInstance(_, character) => {
                format!("Add Region Character Instance: {}", character.name)
            }
            RemoveRegionCharacterInstance(_, _, character) => {
                format!("Remove Region Character Instance: {}", character.name)
            }
            MoveRegionCharacterInstance(_, _, _, _) => "Move Region Character Instance".into(),
            AddRegionItemInstance(_, item) => {
                format!("Add Region Item Instance: {}", item.name)
            }
            RemoveRegionItemInstance(_, _, item) => {
                format!("Remove Region Item Instance: {}", item.name)
            }
            MoveRegionItemInstance(_, _, _, _) => "Move Region Item Instance".into(),
            AddCharacter(character) => format!("Add Character: {}", character.name),
            RemoveCharacter(_, character) => format!("Remove Character: {}", character.name),
            RenameCharacter(_, old, new) => format!("Rename Character: {} -> {}", old, new),
            AddItem(character) => format!("Add Item: {}", character.name),
            RemoveItem(_, character) => format!("Remove Item: {}", character.name),
            RenameItem(_, old, new) => format!("Rename Item: {} -> {}", old, new),
            AddTilemap(tilemap) => format!("Add Tilemap: {}", tilemap.name),
            RemoveTilemap(_, tilemap) => format!("Remove Tilemap: {}", tilemap.name),
            RenameTilemap(_, old, new) => format!("Rename Tilemap: {} -> {}", old, new),
            EditTilemapGridSize(_, old, new) => {
                format!("Edit Tilemap Grid Size: {} -> {}", old, new)
            }
            AddScreen(screen) => format!("Add Screen: {}", screen.name),
            RemoveScreen(_, screen) => format!("Remove Screen: {}", screen.name),
            RenameScreen(_, old, new) => format!("Rename Screen: {} -> {}", old, new),
            AddAsset(asset) => format!("Add Asset: {}", asset.name),
            RemoveAsset(_, asset) => format!("Remove Asset: {}", asset.name),
            RenameAsset(_, old, new) => format!("Rename Asset: {} -> {}", old, new),
            AddAvatar(avatar) => format!("Add Avatar: {}", avatar.name),
            RemoveAvatar(_, avatar) => format!("Remove Avatar: {}", avatar.name),
            RenameAvatar(_, old, new) => format!("Rename Avatar: {} -> {}", old, new),
            EditAvatarResolution(_, old, new) => {
                format!("Edit Avatar Resolution: {} -> {}", old, new)
            }
            EditAvatarPerspectiveCount(_, old, new) => {
                format!("Edit Avatar Perspectives: {:?} -> {:?}", old, new)
            }
            AddAvatarAnimation(_, anim) => format!("Add Animation: {}", anim.name),
            RemoveAvatarAnimation(_, _, anim) => format!("Remove Animation: {}", anim.name),
            RenameAvatarAnimation(_, _, old, new) => {
                format!("Rename Animation: {} -> {}", old, new)
            }
            EditAvatarAnimationFrameCount(_, _, old, new) => {
                format!("Edit Animation Frames: {} -> {}", old, new)
            }
            EditAvatarAnimationSpeed(_, _, old, new) => {
                format!("Edit Animation Speed: {:.2} -> {:.2}", old, new)
            }
            PaletteEdit(_old, _new) => format!("Palette Changed"),
            TileEdit(_old, _new) => format!("Tile Changed"),
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
                if let Some(map) = project.get_map_mut(server_ctx) {
                    *map = *old.clone();
                    map.clear_temp();
                    if pc.is_region() {
                        update_region(ctx);
                        map.update_surfaces();
                    }
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
                            if let Some(widget) = region_node.widgets[0].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Text(old.clone()));
                                }
                            }
                        }
                    }
                }
            }
            AddRegionCharacterInstance(region_id, character) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(region_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_regions_id)
                    {
                        region_node.remove_widget_by_uuid(&character.id);
                    }

                    if let Some(region) = project.get_region_mut(region_id) {
                        region.characters.shift_remove(&character.id);
                        region.map.entities.retain(|e| e.creator_id != character.id);
                    }

                    if let Some(region) = project.get_region(region_id) {
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
            RemoveRegionCharacterInstance(index, region_id, character) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    let character = character.clone();

                    if let Some(region) = project.get_region_mut(region_id) {
                        region
                            .characters
                            .insert_before(*index, character.id, character);

                        if let Some(region_node) = tree_layout.get_node_by_id_mut(&region_id) {
                            gen_region_tree_items(region_node, region);
                        }
                    }
                    shared::rusterix_utils::insert_content_into_maps(project);
                }
            }
            MoveRegionCharacterInstance(region_id, instance_id, from, _to) => {
                move_region_character_pos(
                    project,
                    ui,
                    ctx,
                    server_ctx,
                    *region_id,
                    *instance_id,
                    *from,
                );
            }
            AddRegionItemInstance(region_id, item) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(region_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_regions_id)
                    {
                        region_node.remove_widget_by_uuid(&item.id);
                    }

                    if let Some(region) = project.get_region_mut(region_id) {
                        region.items.shift_remove(&item.id);
                        region.map.items.retain(|e| e.creator_id != item.id);
                    }

                    if let Some(region) = project.get_region(region_id) {
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
            RemoveRegionItemInstance(index, region_id, item) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    let item = item.clone();

                    if let Some(region) = project.get_region_mut(region_id) {
                        region.items.insert_before(*index, item.id, item);

                        if let Some(region_node) = tree_layout.get_node_by_id_mut(&region_id) {
                            gen_region_tree_items(region_node, region);
                        }
                    }
                    shared::rusterix_utils::insert_content_into_maps(project);
                }
            }
            MoveRegionItemInstance(region_id, instance_id, from, _to) => {
                move_region_item_pos(
                    project,
                    ui,
                    ctx,
                    server_ctx,
                    *region_id,
                    *instance_id,
                    *from,
                );
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
                            if let Some(widget) = region_node.widgets[0].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Text(old.clone()));
                                }
                            }
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
                            if let Some(widget) = region_node.widgets[0].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Text(old.clone()));
                                }
                            }
                        }
                    }
                }
            }
            AddTilemap(tilemap) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(tilemap_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_tilemaps_id)
                    {
                        // Find the index of the tilemap in the project
                        if let Some(index) =
                            project.tilemaps.iter().position(|t| t.id == tilemap.id)
                        {
                            project.tilemaps.remove(index);
                        }
                        tilemap_node.remove_child_by_uuid(&tilemap.id);
                    }
                }
            }
            RemoveTilemap(index, tilemap) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    let tilemap = tilemap.clone();

                    let mut node = gen_tilemap_tree_node(&tilemap);
                    node.set_open(true);
                    if let Some(tilemap_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_tilemaps_id)
                    {
                        tilemap_node.add_child_at(*index, node);
                    }
                    project.tilemaps.insert(*index, tilemap.clone());

                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::Tilemap(tilemap.id),
                    );
                    update_region(ctx);
                }
            }
            RenameTilemap(id, old, _new) => {
                if let Some(tilemap) = project.get_tilemap_mut(*id) {
                    tilemap.name = old.clone();
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(tilemap_node) = tree_layout.get_node_by_id_mut(id) {
                            tilemap_node.widget.set_value(TheValue::Text(old.clone()));
                            if let Some(widget) = tilemap_node.widgets[0].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Text(old.clone()));
                                }
                            }
                        }
                    }
                }
            }
            EditTilemapGridSize(id, old, _new) => {
                if let Some(tilemap) = project.get_tilemap_mut(*id) {
                    tilemap.grid_size = *old;
                    // Update the tree node widget
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(node) = tree_layout.get_node_by_id_mut(id) {
                            if let Some(widget) = node.widgets[1].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Int(*old));

                                    ctx.ui.send(TheEvent::Custom(
                                        TheId::named("Tilemap Grid Size Changed"),
                                        TheValue::Int(*old),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            AddScreen(screen) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(screen_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_screens_id)
                    {
                        project.remove_screen(&screen.id);
                        screen_node.remove_child_by_uuid(&screen.id);
                    }
                }
            }
            RemoveScreen(index, screen) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    let screen = screen.clone();

                    let mut node = gen_screen_tree_node(&screen);
                    node.set_open(true);
                    if let Some(screen_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_screens_id)
                    {
                        screen_node.add_child_at(*index, node);
                    }
                    let screen_id: Uuid = screen.id;
                    project.screens.insert_before(*index, screen_id, screen);

                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::Screen(screen_id),
                    );
                    update_region(ctx);
                }
            }
            RenameScreen(id, old, _new) => {
                if let Some(screen) = project.screens.get_mut(id) {
                    screen.name = old.clone();
                    screen.map.name = old.clone();
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(screen_node) = tree_layout.get_node_by_id_mut(&screen.id) {
                            screen_node.widget.set_value(TheValue::Text(old.clone()));
                            if let Some(widget) = screen_node.widgets[0].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Text(old.clone()));
                                }
                            }
                        }
                    }
                }
            }
            AddAsset(asset) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(asset_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_assets_id)
                    {
                        project.remove_asset(&asset.id);
                        asset_node.remove_child_by_uuid(&asset.id);
                    }
                }
            }
            RemoveAsset(index, asset) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    let asset = asset.clone();

                    let mut node = gen_asset_tree_node(&asset);
                    node.set_open(true);
                    if let Some(asset_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_assets_id)
                    {
                        asset_node.add_child_at(*index, node);
                    }
                    let asset_id: Uuid = asset.id;
                    project.assets.insert_before(*index, asset_id, asset);

                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::Asset(asset_id),
                    );
                    update_region(ctx);
                }
            }
            RenameAsset(id, old, _new) => {
                if let Some(asset) = project.assets.get_mut(id) {
                    asset.name = old.clone();
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(asset_node) = tree_layout.get_node_by_id_mut(&asset.id) {
                            asset_node.widget.set_value(TheValue::Text(old.clone()));
                            if let Some(widget) = asset_node.widgets[0].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Text(old.clone()));
                                }
                            }
                        }
                    }
                }
            }
            AddAvatar(avatar) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(avatar_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_avatars_id)
                    {
                        project.remove_avatar(&avatar.id);
                        avatar_node.remove_child_by_uuid(&avatar.id);
                    }
                }
            }
            RemoveAvatar(index, avatar) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    let avatar = avatar.clone();

                    let mut node = gen_avatar_tree_node(&avatar);
                    node.set_open(true);
                    if let Some(avatar_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_avatars_id)
                    {
                        avatar_node.add_child_at(*index, node);
                    }
                    let avatar_id: Uuid = avatar.id;
                    project.avatars.insert_before(*index, avatar_id, avatar);

                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::Avatar(avatar_id),
                    );
                    update_region(ctx);
                }
            }
            RenameAvatar(id, old, _new) => {
                if let Some(avatar) = project.avatars.get_mut(id) {
                    avatar.name = old.clone();
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(avatar_node) = tree_layout.get_node_by_id_mut(&avatar.id) {
                            avatar_node.widget.set_value(TheValue::Text(old.clone()));
                            if let Some(widget) = avatar_node.widgets[0].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Text(old.clone()));
                                }
                            }
                        }
                    }
                }
            }
            EditAvatarResolution(id, old, _new) => {
                if let Some(avatar) = project.avatars.get_mut(id) {
                    avatar.set_resolution(*old);
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(avatar_node) = tree_layout.get_node_by_id_mut(id) {
                            if let Some(widget) = avatar_node.widgets[1].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Int(*old as i32));
                                }
                            }
                        }
                    }
                }
            }
            EditAvatarPerspectiveCount(id, old, _new) => {
                if let Some(avatar) = project.avatars.get_mut(id) {
                    avatar.set_perspective_count(*old);
                }
                rebuild_avatar_tree_node(id, project, ui, server_ctx);
            }
            AddAvatarAnimation(avatar_id, anim) => {
                // Undo: remove the animation
                if let Some(avatar) = project.avatars.get_mut(avatar_id) {
                    avatar.animations.retain(|a| a.id != anim.id);
                    rebuild_avatar_tree_node(avatar_id, project, ui, server_ctx);
                }
            }
            RemoveAvatarAnimation(avatar_id, index, anim) => {
                // Undo: re-insert the animation at its original index
                if let Some(avatar) = project.avatars.get_mut(avatar_id) {
                    let idx = (*index).min(avatar.animations.len());
                    avatar.animations.insert(idx, anim.clone());
                    rebuild_avatar_tree_node(avatar_id, project, ui, server_ctx);
                }
            }
            RenameAvatarAnimation(_avatar_id, anim_id, old, _new) => {
                if let Some(avatar) = project
                    .avatars
                    .values_mut()
                    .find(|a| a.animations.iter().any(|anim| anim.id == *anim_id))
                {
                    if let Some(anim) = avatar.animations.iter_mut().find(|a| a.id == *anim_id) {
                        anim.name = old.clone();
                    }
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(anim_node) = tree_layout.get_node_by_id_mut(anim_id) {
                            let label = format!("{} - Animation", old);
                            anim_node.widget.set_value(TheValue::Text(label));
                            if let Some(widget) = anim_node.widgets[0].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Text(old.clone()));
                                }
                            }
                        }
                    }
                }
            }
            EditAvatarAnimationFrameCount(avatar_id, anim_id, old, _new) => {
                if let Some(avatar) = project.avatars.get_mut(avatar_id) {
                    avatar.set_animation_frame_count(anim_id, *old);
                }
                rebuild_animation_tree_node(avatar_id, anim_id, project, ui, server_ctx);
            }
            EditAvatarAnimationSpeed(avatar_id, anim_id, old, _new) => {
                if let Some(avatar) = project.avatars.get_mut(avatar_id) {
                    if let Some(anim) = avatar.animations.iter_mut().find(|a| a.id == *anim_id) {
                        anim.speed = *old;
                    }
                }
                rebuild_animation_tree_node(avatar_id, anim_id, project, ui, server_ctx);
            }
            PaletteEdit(old, _new) => {
                let sel = project.palette.current_index;
                project.palette = old.clone();
                project.palette.current_index = sel;
                apply_palette(ui, ctx, server_ctx, project);
            }
            TileEdit(old, _new) => {
                project.tiles.insert(old.id, old.clone());
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Tiles"),
                    TheValue::Empty,
                ));
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
                if let Some(map) = project.get_map_mut(server_ctx) {
                    *map = *new.clone();
                    map.clear_temp();
                    if pc.is_region() {
                        update_region(ctx);
                        map.update_surfaces();
                    }
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
                            if let Some(widget) = region_node.widgets[0].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Text(new.clone()));
                                }
                            }
                        }
                    }
                }
            }
            AddRegionCharacterInstance(region_id, character) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    let character = character.clone();

                    if let Some(region) = project.get_region_mut(region_id) {
                        region.characters.insert(character.id, character);

                        if let Some(region_node) = tree_layout.get_node_by_id_mut(&region_id) {
                            gen_region_tree_items(region_node, region);
                        }
                    }
                    shared::rusterix_utils::insert_content_into_maps(project);
                }
            }
            MoveRegionCharacterInstance(region_id, instance_id, _from, to) => {
                move_region_character_pos(
                    project,
                    ui,
                    ctx,
                    server_ctx,
                    *region_id,
                    *instance_id,
                    *to,
                );
            }
            RemoveRegionCharacterInstance(_, region_id, character) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(region_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_regions_id)
                    {
                        region_node.remove_widget_by_uuid(&character.id);
                    }

                    if let Some(region) = project.get_region_mut(region_id) {
                        region.characters.shift_remove(&character.id);
                        region.map.entities.retain(|e| e.creator_id != character.id);
                    }

                    if let Some(region) = project.get_region(region_id) {
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
            AddRegionItemInstance(region_id, item) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    let item = item.clone();

                    if let Some(region) = project.get_region_mut(region_id) {
                        region.items.insert(item.id, item);

                        if let Some(region_node) = tree_layout.get_node_by_id_mut(&region_id) {
                            gen_region_tree_items(region_node, region);
                        }
                    }
                    shared::rusterix_utils::insert_content_into_maps(project);
                }
            }
            MoveRegionItemInstance(region_id, instance_id, _from, to) => {
                move_region_item_pos(project, ui, ctx, server_ctx, *region_id, *instance_id, *to);
            }
            RemoveRegionItemInstance(_, region_id, item) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(region_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_regions_id)
                    {
                        region_node.remove_widget_by_uuid(&item.id);
                    }

                    if let Some(region) = project.get_region_mut(region_id) {
                        region.items.shift_remove(&item.id);
                        region.map.items.retain(|e| e.creator_id != item.id);
                    }

                    if let Some(region) = project.get_region(region_id) {
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
            AddCharacter(character) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_characters_id)
                    {
                        let mut character = character.clone();

                        if let Some(bytes) = crate::Embedded::get("eldrin/character.eldrin") {
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
                            if let Some(widget) = region_node.widgets[0].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Text(new.clone()));
                                }
                            }
                        }
                    }
                }
            }
            AddItem(item) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_items_id) {
                        let mut item = item.clone();

                        if let Some(bytes) = crate::Embedded::get("eldrin/item.eldrin") {
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
                            if let Some(widget) = item_node.widgets[0].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Text(new.clone()));
                                }
                            }
                        }
                    }
                }
            }
            AddTilemap(tilemap) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_tilemaps_id)
                    {
                        let tilemap = tilemap.clone();

                        let mut tilemap_node = gen_tilemap_tree_node(&tilemap);
                        tilemap_node.set_open(true);
                        node.add_child(tilemap_node);

                        let tilemap_id = tilemap.id;
                        project.add_tilemap(tilemap);

                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Tilemap(tilemap_id),
                        );
                        update_region(ctx);
                    }
                }
            }
            RemoveTilemap(_, tilemap) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(tilemap_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_tilemaps_id)
                    {
                        tilemap_node.remove_child_by_uuid(&tilemap.id);
                    }
                    // Find the index of the tilemap in the project
                    if let Some(index) = project.tilemaps.iter().position(|t| t.id == tilemap.id) {
                        project.tilemaps.remove(index);
                    }

                    if let Some(first_tilemap) = project.tilemaps.first() {
                        if let Some(tilemap_node) =
                            tree_layout.get_node_by_id_mut(&first_tilemap.id)
                        {
                            tilemap_node.set_open(true);
                        }
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Tilemap(first_tilemap.id),
                        );
                    }
                }
            }
            RenameTilemap(id, _old, new) => {
                if let Some(tilemap) = project.get_tilemap_mut(*id) {
                    tilemap.name = new.clone();
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(tilemap_node) = tree_layout.get_node_by_id_mut(id) {
                            tilemap_node.widget.set_value(TheValue::Text(new.clone()));
                            if let Some(widget) = tilemap_node.widgets[0].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Text(new.clone()));
                                }
                            }
                        }
                    }
                }
            }
            EditTilemapGridSize(id, _old, new) => {
                if let Some(tilemap) = project.get_tilemap_mut(*id) {
                    tilemap.grid_size = *new;
                    // Update the tree node widget
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(node) = tree_layout.get_node_by_id_mut(id) {
                            if let Some(widget) = node.widgets[1].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Int(*new));

                                    ctx.ui.send(TheEvent::Custom(
                                        TheId::named("Tilemap Grid Size Changed"),
                                        TheValue::Int(*new),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            AddScreen(screen) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_screens_id)
                    {
                        let screen = screen.clone();

                        let mut screen_node = gen_screen_tree_node(&screen);
                        screen_node.set_open(true);
                        node.add_child(screen_node);

                        let screen_id = screen.id;
                        project.add_screen(screen);

                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Screen(screen_id),
                        );
                        update_region(ctx);
                    }
                }
            }
            RemoveScreen(_, screen) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(screen_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_screens_id)
                    {
                        screen_node.remove_child_by_uuid(&screen.id);
                    }
                    project.remove_screen(&screen.id);

                    if let Some(first_screen) = project.screens.first() {
                        if let Some(screen_node) = tree_layout.get_node_by_id_mut(first_screen.0) {
                            screen_node.set_open(true);
                        }
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Screen(*first_screen.0),
                        );
                    }
                }
            }
            RenameScreen(id, _old, new) => {
                if let Some(screen) = project.screens.get_mut(id) {
                    screen.name = new.clone();
                    screen.map.name = new.clone();
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(screen_node) = tree_layout.get_node_by_id_mut(id) {
                            screen_node.widget.set_value(TheValue::Text(new.clone()));
                            if let Some(widget) = screen_node.widgets[0].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Text(new.clone()));
                                }
                            }
                        }
                    }
                }
            }
            AddAsset(asset) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_assets_id) {
                        let asset = asset.clone();

                        let mut asset_node = gen_asset_tree_node(&asset);
                        asset_node.set_open(true);
                        node.add_child(asset_node);

                        let asset_id = asset.id;
                        project.add_asset(asset);

                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Asset(asset_id),
                        );
                        update_region(ctx);
                    }
                }
            }
            RemoveAsset(_, asset) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(asset_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_assets_id)
                    {
                        asset_node.remove_child_by_uuid(&asset.id);
                    }
                    project.remove_asset(&asset.id);

                    if let Some(first_asset) = project.assets.first() {
                        if let Some(asset_node) = tree_layout.get_node_by_id_mut(first_asset.0) {
                            asset_node.set_open(true);
                        }
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Asset(*first_asset.0),
                        );
                    }
                }
            }
            RenameAsset(id, _old, new) => {
                if let Some(asset) = project.assets.get_mut(id) {
                    asset.name = new.clone();
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(asset_node) = tree_layout.get_node_by_id_mut(id) {
                            asset_node.widget.set_value(TheValue::Text(new.clone()));
                            if let Some(widget) = asset_node.widgets[0].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Text(new.clone()));
                                }
                            }
                        }
                    }
                }
            }
            AddAvatar(avatar) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_avatars_id)
                    {
                        let avatar = avatar.clone();

                        let mut avatar_node = gen_avatar_tree_node(&avatar);
                        avatar_node.set_open(true);
                        node.add_child(avatar_node);

                        let avatar_id = avatar.id;
                        project.add_avatar(avatar);

                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Avatar(avatar_id),
                        );
                        update_region(ctx);
                    }
                }
            }
            RemoveAvatar(_, avatar) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(avatar_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_avatars_id)
                    {
                        avatar_node.remove_child_by_uuid(&avatar.id);
                    }
                    project.remove_avatar(&avatar.id);

                    if let Some(first_avatar) = project.avatars.first() {
                        if let Some(avatar_node) = tree_layout.get_node_by_id_mut(first_avatar.0) {
                            avatar_node.set_open(true);
                        }
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Avatar(*first_avatar.0),
                        );
                    }
                }
            }
            RenameAvatar(id, _old, new) => {
                if let Some(avatar) = project.avatars.get_mut(id) {
                    avatar.name = new.clone();
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(avatar_node) = tree_layout.get_node_by_id_mut(id) {
                            avatar_node.widget.set_value(TheValue::Text(new.clone()));
                            if let Some(widget) = avatar_node.widgets[0].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Text(new.clone()));
                                }
                            }
                        }
                    }
                }
            }
            EditAvatarResolution(id, _old, new) => {
                if let Some(avatar) = project.avatars.get_mut(id) {
                    avatar.set_resolution(*new);
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(avatar_node) = tree_layout.get_node_by_id_mut(id) {
                            if let Some(widget) = avatar_node.widgets[1].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Int(*new as i32));
                                }
                            }
                        }
                    }
                }
            }
            EditAvatarPerspectiveCount(id, _old, new) => {
                if let Some(avatar) = project.avatars.get_mut(id) {
                    avatar.set_perspective_count(*new);
                }
                rebuild_avatar_tree_node(id, project, ui, server_ctx);
            }
            AddAvatarAnimation(avatar_id, anim) => {
                // Redo: add the animation (already has correct perspectives/frames)
                if let Some(avatar) = project.avatars.get_mut(avatar_id) {
                    avatar.animations.push(anim.clone());
                    rebuild_avatar_tree_node(avatar_id, project, ui, server_ctx);
                }
            }
            RemoveAvatarAnimation(avatar_id, _, anim) => {
                // Redo: remove the animation
                if let Some(avatar) = project.avatars.get_mut(avatar_id) {
                    avatar.animations.retain(|a| a.id != anim.id);
                    rebuild_avatar_tree_node(avatar_id, project, ui, server_ctx);
                }
            }
            RenameAvatarAnimation(_avatar_id, anim_id, _old, new) => {
                if let Some(avatar) = project
                    .avatars
                    .values_mut()
                    .find(|a| a.animations.iter().any(|anim| anim.id == *anim_id))
                {
                    if let Some(anim) = avatar.animations.iter_mut().find(|a| a.id == *anim_id) {
                        anim.name = new.clone();
                    }
                    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                        if let Some(anim_node) = tree_layout.get_node_by_id_mut(anim_id) {
                            let label = format!("{} - Animation", new);
                            anim_node.widget.set_value(TheValue::Text(label));
                            if let Some(widget) = anim_node.widgets[0].as_tree_item() {
                                if let Some(embedded) = widget.embedded_widget_mut() {
                                    embedded.set_value(TheValue::Text(new.clone()));
                                }
                            }
                        }
                    }
                }
            }
            EditAvatarAnimationFrameCount(avatar_id, anim_id, _old, new) => {
                if let Some(avatar) = project.avatars.get_mut(avatar_id) {
                    avatar.set_animation_frame_count(anim_id, *new);
                }
                rebuild_animation_tree_node(avatar_id, anim_id, project, ui, server_ctx);
            }
            EditAvatarAnimationSpeed(avatar_id, anim_id, _old, new) => {
                if let Some(avatar) = project.avatars.get_mut(avatar_id) {
                    if let Some(anim) = avatar.animations.iter_mut().find(|a| a.id == *anim_id) {
                        anim.speed = *new;
                    }
                }
                rebuild_animation_tree_node(avatar_id, anim_id, project, ui, server_ctx);
            }
            PaletteEdit(_old, new) => {
                let sel = project.palette.current_index;
                project.palette = new.clone();
                project.palette.current_index = sel;
                apply_palette(ui, ctx, server_ctx, project);
            }
            TileEdit(_old, new) => {
                project.tiles.insert(new.id, new.clone());
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Tiles"),
                    TheValue::Empty,
                ));
            }
        }
    }
}

fn move_region_character_pos(
    project: &mut Project,
    ui: &mut TheUI,
    ctx: &mut TheContext,
    server_ctx: &mut ServerContext,
    region_id: Uuid,
    instance_id: Uuid,
    pos: Vec3<f32>,
) {
    set_project_context(
        ctx,
        ui,
        project,
        server_ctx,
        ProjectContext::Region(region_id),
    );

    if let Some(region) = project.get_region_mut(&region_id) {
        if let Some(instance) = region.characters.get_mut(&instance_id) {
            instance.position = pos;
        }
        for entity in region.map.entities.iter_mut() {
            if entity.creator_id == instance_id {
                entity.position = pos;
            }
        }
    }

    if let Some(map) = project.get_map_mut(server_ctx) {
        for entity in map.entities.iter_mut() {
            if entity.creator_id == instance_id {
                entity.position = pos;
            }
        }
    }
}

fn move_region_item_pos(
    project: &mut Project,
    ui: &mut TheUI,
    ctx: &mut TheContext,
    server_ctx: &mut ServerContext,
    region_id: Uuid,
    instance_id: Uuid,
    pos: Vec3<f32>,
) {
    set_project_context(
        ctx,
        ui,
        project,
        server_ctx,
        ProjectContext::Region(region_id),
    );

    if let Some(region) = project.get_region_mut(&region_id) {
        if let Some(instance) = region.items.get_mut(&instance_id) {
            instance.position = pos;
        }
        for item in region.map.items.iter_mut() {
            if item.creator_id == instance_id {
                item.position = pos;
            }
        }
    }

    if let Some(map) = project.get_map_mut(server_ctx) {
        for item in map.items.iter_mut() {
            if item.creator_id == instance_id {
                item.position = pos;
            }
        }
    }
}
