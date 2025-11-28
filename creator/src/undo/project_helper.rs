use crate::editor::{DOCKMANAGER, RUSTERIX};
use crate::prelude::*;
use theframework::prelude::*;

/// Generate a tree node for the given region
pub fn gen_region_tree_node(region: &Region) -> TheTreeNode {
    let mut node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(&region.name, region.id));
    node.set_root_mode(false);

    gen_region_tree_items(&mut node, region);

    node
}

/// Generate the items for the region node
pub fn gen_region_tree_items(node: &mut TheTreeNode, region: &Region) {
    node.widgets = vec![];

    // Name
    let mut item = TheTreeItem::new(TheId::named_with_reference("Region Item", region.id));
    item.set_text(fl!("name"));

    let name = format!("Region Item Name Edit: {}", region.name);
    let mut edit = TheTextLineEdit::new(TheId::named_with_id(&name, region.id));
    edit.set_text(region.name.clone());
    item.add_widget_column(200, Box::new(edit));
    node.add_widget(Box::new(item));

    // Settings
    let mut item = TheTreeItem::new(TheId::named_with_reference(
        "Region Settings Item",
        region.id,
    ));
    item.set_text(fl!("settings"));
    node.add_widget(Box::new(item));

    for (id, character) in &region.characters {
        let mut item = TheTreeItem::new(TheId::named_with_id("Region Content List Item", *id));
        item.add_value_column(200, TheValue::Text("Character Instance".to_string()));
        item.set_background_color(TheColor::from(ActionRole::Dock.to_color()));
        item.set_text(character.name.clone());
        node.add_widget(Box::new(item));
    }

    for (id, item_) in &region.items {
        let mut item = TheTreeItem::new(TheId::named_with_id("Region Content List Item", *id));
        item.add_value_column(200, TheValue::Text("Item Instance".to_string()));
        item.set_background_color(TheColor::from(ActionRole::Editor.to_color()));
        item.set_text(item_.name.clone());
        node.add_widget(Box::new(item));
    }
}

/// Returns a TheTreeNode for the character.
pub fn gen_character_tree_node(character: &Character) -> TheTreeNode {
    let mut node: TheTreeNode =
        TheTreeNode::new(TheId::named_with_id(&character.name, character.id));
    node.set_root_mode(false);

    let mut item = TheTreeItem::new(TheId::named_with_reference("Character Item", character.id));
    item.set_text(fl!("name"));

    let mut edit = TheTextLineEdit::new(TheId::named_with_id(
        "Character Item Name Edit",
        character.id,
    ));
    edit.set_text(character.name.clone());
    item.add_widget_column(200, Box::new(edit));

    node.add_widget(Box::new(item));

    let mut item = TheTreeItem::new(TheId::named_with_reference(
        "Character Item Visual Code Edit",
        character.id,
    ));
    item.set_background_color(TheColor::from(ActionRole::Dock.to_color()));
    item.set_text(fl!("visual_script"));
    node.add_widget(Box::new(item));

    let mut item = TheTreeItem::new(TheId::named_with_reference(
        "Character Item Code Edit",
        character.id,
    ));
    item.set_background_color(TheColor::from(ActionRole::Dock.to_color()));
    item.set_text(fl!("python_code"));
    node.add_widget(Box::new(item));

    let mut item = TheTreeItem::new(TheId::named_with_reference(
        "Character Item Data Edit",
        character.id,
    ));
    item.set_background_color(TheColor::from(ActionRole::Dock.to_color()));
    item.set_text(fl!("attributes"));
    node.add_widget(Box::new(item));

    node
}

/// Returns a TheTreeNode for the item.
pub fn gen_item_tree_node(item_: &Item) -> TheTreeNode {
    let mut node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(&item_.name, item_.id));
    node.set_root_mode(false);

    let mut item = TheTreeItem::new(TheId::named_with_reference("Item Item", item_.id));
    item.set_text(fl!("name"));

    let mut edit = TheTextLineEdit::new(TheId::named_with_id("Item Item Name Edit", item_.id));
    edit.set_text(item_.name.clone());
    item.add_widget_column(200, Box::new(edit));

    node.add_widget(Box::new(item));

    let mut item = TheTreeItem::new(TheId::named_with_reference(
        "Item Item Visual Code Edit",
        item_.id,
    ));
    item.set_background_color(TheColor::from(ActionRole::Dock.to_color()));
    item.set_text(fl!("visual_script"));
    node.add_widget(Box::new(item));

    let mut item = TheTreeItem::new(TheId::named_with_reference("Item Item Code Edit", item_.id));
    item.set_background_color(TheColor::from(ActionRole::Dock.to_color()));
    item.set_text(fl!("python_code"));
    node.add_widget(Box::new(item));

    let mut item = TheTreeItem::new(TheId::named_with_reference("Item Item Data Edit", item_.id));
    item.set_background_color(TheColor::from(ActionRole::Dock.to_color()));
    item.set_text(fl!("attributes"));
    node.add_widget(Box::new(item));

    node
}

/// Returns a TheTreeNode for the tilemap item.
pub fn gen_tilemap_tree_node(tilemap: &Tilemap) -> TheTreeNode {
    let mut node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(&tilemap.name, tilemap.id));
    node.set_root_mode(false);

    let mut item = TheTreeItem::new(TheId::named_with_reference("Tileset Item", tilemap.id));
    item.set_text(fl!("name"));

    let mut edit = TheTextLineEdit::new(TheId::named_with_id("Tilemap Item Name Edit", tilemap.id));
    edit.set_text(tilemap.name.clone());
    item.add_widget_column(200, Box::new(edit));

    node.add_widget(Box::new(item));

    let mut item = TheTreeItem::new(TheId::named_with_reference(
        "Tilemap Item Code Edit",
        tilemap.id,
    ));
    item.set_text(fl!("grid_size"));

    let mut edit = TheTextLineEdit::new(TheId::named_with_reference(
        "Tilemap Item Grid Edit",
        tilemap.id,
    ));
    edit.set_value(TheValue::Int(tilemap.grid_size));
    item.add_widget_column(200, Box::new(edit));

    node.add_widget(Box::new(item));

    node
}

/// Returns a TheTreeNode for the screen.
pub fn gen_screen_tree_node(screen: &Screen) -> TheTreeNode {
    let mut node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(&screen.name, screen.id));
    node.set_root_mode(false);

    gen_screen_tree_items(&mut node, screen);

    node
}

/// Generate the items for the screen node
pub fn gen_screen_tree_items(node: &mut TheTreeNode, screen: &Screen) {
    node.widgets = vec![];

    let mut item = TheTreeItem::new(TheId::named_with_reference("Screen Item", screen.id));
    item.set_text(fl!("name"));

    let mut edit = TheTextLineEdit::new(TheId::named_with_id("Screen Item Name Edit", screen.id));
    edit.set_text(screen.name.clone());
    item.add_widget_column(200, Box::new(edit));

    node.add_widget(Box::new(item));

    for sector in &screen.map.sectors {
        if !sector.name.is_empty() {
            let mut item = TheTreeItem::new(TheId::named_with_id_and_reference(
                "Screen Content List Item",
                sector.creator_id,
                screen.id,
            ));
            item.add_value_column(200, TheValue::Text("Widget".to_string()));
            item.set_background_color(TheColor::from(ActionRole::Dock.to_color()));
            item.set_text(sector.name.clone());
            node.add_widget(Box::new(item));
        }
    }
}

/// Returns a TheTreeNode for the screen.
pub fn gen_asset_tree_node(asset: &Asset) -> TheTreeNode {
    let mut node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(&asset.name, asset.id));
    node.set_root_mode(false);

    let mut item = TheTreeItem::new(TheId::named_with_reference("Asset Item", asset.id));
    item.set_text(fl!("name"));

    let mut edit = TheTextLineEdit::new(TheId::named_with_id("Asset Item Name Edit", asset.id));
    edit.set_text(asset.name.clone());
    item.add_widget_column(200, Box::new(edit));

    node.add_widget(Box::new(item));

    node
}

/// Rerender the current region.
pub fn update_region(ctx: &mut TheContext) {
    ctx.ui.send(TheEvent::Custom(
        TheId::named("Update Minimap"),
        TheValue::Empty,
    ));

    RUSTERIX.write().unwrap().set_dirty();

    ctx.ui.send(TheEvent::Custom(
        TheId::named("Render SceneManager Map"),
        TheValue::Empty,
    ));
}

/// Set the project context and the current docker.
pub fn set_project_context(
    ctx: &mut TheContext,
    ui: &mut TheUI,
    project: &Project,
    server_ctx: &mut ServerContext,
    pc: ProjectContext,
) {
    // println!("set_project_context {:?}", pc);
    let mut old_project_id = None;
    if let Some(old_id) = server_ctx.pc.id() {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(node) = tree_layout.get_node_by_id_mut(&old_id) {
                if let Some(snapper) = node.widget.as_any().downcast_mut::<TheSnapperbar>() {
                    snapper.set_selected(false);
                }
            }
        }
        old_project_id = Some(old_id);
    }

    server_ctx.pc = pc;

    match pc {
        ProjectContext::Region(id) => {
            if let Some(region) = project.get_region(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Region: {}", region.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Tiles".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::RegionSettings(id) => {
            if let Some(region) = project.get_region(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Region Settings: {}", region.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Data".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::RegionCharacterInstance(id, _) => {
            if let Some(region) = project.get_region(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Region ({}) Character", region.name)),
                );
            }
            DOCKMANAGER.write().unwrap().set_dock(
                "Visual Code".into(),
                ui,
                ctx,
                project,
                server_ctx,
            );
        }
        ProjectContext::RegionItemInstance(id, _) => {
            if let Some(region) = project.get_region(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Region ({}) Item", region.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Tiles".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::Character(id) => {
            if let Some(region) = project.characters.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Character: {}", region.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Tiles".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::CharacterVisualCode(id) => {
            if let Some(region) = project.characters.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Character: {}", region.name)),
                );
            }
            DOCKMANAGER.write().unwrap().set_dock(
                "Visual Code".into(),
                ui,
                ctx,
                project,
                server_ctx,
            );
        }
        ProjectContext::CharacterCode(id) => {
            if let Some(region) = project.characters.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Character: {}", region.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Code".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::CharacterData(id) => {
            if let Some(region) = project.characters.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Character: {}", region.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Data".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::Item(id) => {
            if let Some(item) = project.items.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Item: {}", item.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Tiles".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::ItemVisualCode(id) => {
            if let Some(item) = project.items.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Item: {}", item.name)),
                );
            }
            DOCKMANAGER.write().unwrap().set_dock(
                "Visual Code".into(),
                ui,
                ctx,
                project,
                server_ctx,
            );
        }
        ProjectContext::ItemCode(id) => {
            if let Some(item) = project.items.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Item: {}", item.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Code".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::ItemData(id) => {
            if let Some(item) = project.items.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Item: {}", item.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Data".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::Tilemap(id) => {
            if let Some(tilemap) = project.get_tilemap(id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Tilemap: {}", tilemap.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Tilemap".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::Screen(id) => {
            if let Some(screen) = project.screens.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Screen: {}", screen.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Tiles".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::ScreenWidget(id, _widget_id) => {
            if let Some(screen) = project.screens.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Screen ({}) Widget", screen.name,)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Data".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::Asset(id) => {
            if let Some(asset) = project.assets.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Asset: {}", asset.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Tiles".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::ProjectSettings => {
            ui.set_widget_value(
                "Project Context",
                ctx,
                TheValue::Text("Project Settings".into()),
            );
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Data".into(), ui, ctx, project, server_ctx);
        }
        _ => {}
    }

    // If the region changed, update it
    if pc.is_region() {
        if old_project_id != pc.id() {
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Update Minimap"),
                TheValue::Empty,
            ));

            RUSTERIX.write().unwrap().set_dirty();

            ctx.ui.send(TheEvent::Custom(
                TheId::named("Render SceneManager Map"),
                TheValue::Empty,
            ));
        }
    }

    if let Some(new_id) = pc.id() {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(node) = tree_layout.get_node_by_id_mut(&new_id) {
                if let Some(snapper) = node.widget.as_any().downcast_mut::<TheSnapperbar>() {
                    snapper.set_selected(true);
                }
            }
        }
    }

    ctx.ui.send(TheEvent::Custom(
        TheId::named("Update Action List"),
        TheValue::Empty,
    ));

    ctx.ui.relayout = true;
}
