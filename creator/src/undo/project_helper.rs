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
        item.add_value_column(200, TheValue::Text(fl!("character_instance")));
        item.set_background_color(TheColor::from(ActionRole::Dock.to_color()));
        item.set_text(character.name.clone());
        node.add_widget(Box::new(item));
    }

    for (id, item_) in &region.items {
        let mut item = TheTreeItem::new(TheId::named_with_id("Region Content List Item", *id));
        item.add_value_column(200, TheValue::Text(fl!("item_instance")));
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
    item.set_text(fl!("eldrin_scripting"));
    node.add_widget(Box::new(item));

    let mut item = TheTreeItem::new(TheId::named_with_reference(
        "Character Item Data Edit",
        character.id,
    ));
    item.set_background_color(TheColor::from(ActionRole::Dock.to_color()));
    item.set_text(fl!("attributes"));
    node.add_widget(Box::new(item));

    let mut item = TheTreeItem::new(TheId::named_with_reference(
        "Character Item Preview Rigging Edit",
        character.id,
    ));
    item.set_background_color(TheColor::from(ActionRole::Dock.to_color()));
    item.set_text(fl!("preview_rigging"));
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
    item.set_text(fl!("eldrin_scripting"));
    node.add_widget(Box::new(item));

    let mut item = TheTreeItem::new(TheId::named_with_reference("Item Item Data Edit", item_.id));
    item.set_background_color(TheColor::from(ActionRole::Dock.to_color()));
    item.set_text(fl!("attributes"));
    node.add_widget(Box::new(item));

    node
}

/// Rebuilds the tree node for an avatar in the Project Tree.
pub fn rebuild_avatar_tree_node(
    avatar_id: &Uuid,
    project: &Project,
    ui: &mut TheUI,
    server_ctx: &ServerContext,
) {
    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
        if let Some(avatars_node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_avatars_id) {
            let index = avatars_node
                .childs
                .iter()
                .position(|child| child.id.uuid == *avatar_id);
            avatars_node.remove_child_by_uuid(avatar_id);

            if let Some(avatar) = project.avatars.get(avatar_id) {
                let mut node = gen_avatar_tree_node(avatar);
                node.set_open(true);
                if let Some(idx) = index {
                    avatars_node.add_child_at(idx, node);
                } else {
                    avatars_node.add_child(node);
                }
            }
        }
    }
}

/// Rebuilds the tree node for an animation within its avatar node.
pub fn rebuild_animation_tree_node(
    avatar_id: &Uuid,
    anim_id: &Uuid,
    project: &Project,
    ui: &mut TheUI,
    server_ctx: &ServerContext,
) {
    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
        if let Some(avatars_node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_avatars_id) {
            if let Some(avatar_node) = avatars_node
                .childs
                .iter_mut()
                .find(|c| c.id.uuid == *avatar_id)
            {
                let index = avatar_node
                    .childs
                    .iter()
                    .position(|child| child.id.uuid == *anim_id);
                avatar_node.remove_child_by_uuid(anim_id);

                if let Some(avatar) = project.avatars.get(avatar_id) {
                    if let Some(anim) = avatar.animations.iter().find(|a| a.id == *anim_id) {
                        let mut anim_node = gen_avatar_animation_node(anim);
                        anim_node.set_open(true);
                        if let Some(idx) = index {
                            avatar_node.add_child_at(idx, anim_node);
                        } else {
                            avatar_node.add_child(anim_node);
                        }
                    }
                }
            }
        }
    }
}

/// Returns a TheTreeNode for the avatar item.
pub fn gen_avatar_tree_node(avatar: &Avatar) -> TheTreeNode {
    let mut node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(&avatar.name, avatar.id));
    node.set_root_mode(false);

    let mut item = TheTreeItem::new(TheId::named_with_reference("Avatar Item", avatar.id));
    item.set_text(fl!("name"));

    let mut edit = TheTextLineEdit::new(TheId::named_with_id("Avatar Item Name Edit", avatar.id));
    edit.set_text(avatar.name.clone());
    item.add_widget_column(200, Box::new(edit));

    node.add_widget(Box::new(item));

    let mut item = TheTreeItem::new(TheId::named_with_reference(
        "Avatar Item Resolution",
        avatar.id,
    ));
    item.set_text("Resolution".to_string());

    let mut edit = TheTextLineEdit::new(TheId::named_with_reference(
        "Avatar Item Resolution Edit",
        avatar.id,
    ));
    edit.set_value(TheValue::Int(avatar.resolution as i32));
    item.add_widget_column(200, Box::new(edit));

    node.add_widget(Box::new(item));

    let mut item = TheTreeItem::new(TheId::named_with_reference(
        "Avatar Perspectives",
        avatar.id,
    ));
    item.set_text("Perspectives".to_string());

    let mut drop_down = TheDropdownMenu::new(TheId::named_with_reference(
        "Avatar Perspective Count",
        avatar.id,
    ));
    drop_down.add_option("1".to_string());
    drop_down.add_option("4".to_string());
    drop_down.set_selected_index(match avatar.perspective_count {
        AvatarPerspectiveCount::One => 0,
        AvatarPerspectiveCount::Four => 1,
    });
    item.add_widget_column(200, Box::new(drop_down));

    node.add_widget(Box::new(item));

    let mut item = TheTreeItem::new(TheId::named_with_reference("Avatar Animations", avatar.id));
    item.set_text("Animations".to_string());

    let mut add_button = TheTraybarButton::new(TheId::named_with_reference(
        "Avatar Add Animation",
        avatar.id,
    ));
    add_button.set_text("Add".to_string());
    item.add_widget_column(200, Box::new(add_button));

    node.add_widget(Box::new(item));

    // Add existing animations as child nodes
    for animation in &avatar.animations {
        let anim_node = gen_avatar_animation_node(animation);
        node.add_child(anim_node);
    }

    node
}

/// Returns a TheTreeNode for an avatar animation.
pub fn gen_avatar_animation_node(animation: &AvatarAnimation) -> TheTreeNode {
    let label = format!("{} - Animation", animation.name);
    let mut node = TheTreeNode::new(TheId::named_with_id(&label, animation.id));
    node.set_root_mode(false);

    // Name
    let mut item = TheTreeItem::new(TheId::named_with_reference(
        "Avatar Animation Item",
        animation.id,
    ));
    item.set_text(fl!("name"));

    let mut edit = TheTextLineEdit::new(TheId::named_with_reference(
        "Avatar Animation Name Edit",
        animation.id,
    ));
    edit.set_text(animation.name.clone());
    item.add_widget_column(200, Box::new(edit));
    node.add_widget(Box::new(item));

    // Frame count
    let mut item = TheTreeItem::new(TheId::named_with_reference(
        "Avatar Animation Frames",
        animation.id,
    ));
    item.set_text("Frames".to_string());

    let frame_count = if let Some(p) = animation.perspectives.first() {
        p.frames.len() as i32
    } else {
        0
    };
    let mut edit = TheTextLineEdit::new(TheId::named_with_reference(
        "Avatar Animation Frame Count Edit",
        animation.id,
    ));
    edit.set_value(TheValue::Int(frame_count));
    item.add_widget_column(200, Box::new(edit));
    node.add_widget(Box::new(item));

    // Speed
    let mut item = TheTreeItem::new(TheId::named_with_reference(
        "Avatar Animation Speed",
        animation.id,
    ));
    item.set_text("Speed".to_string());

    let mut edit = TheTextLineEdit::new(TheId::named_with_reference(
        "Avatar Animation Speed Edit",
        animation.id,
    ));
    edit.set_value(TheValue::Float(animation.speed));
    item.add_widget_column(200, Box::new(edit));
    node.add_widget(Box::new(item));

    // Perspective child nodes
    for (persp_index, perspective) in animation.perspectives.iter().enumerate() {
        let dir_name = match perspective.direction {
            AvatarDirection::Front => "Front",
            AvatarDirection::Back => "Back",
            AvatarDirection::Left => "Left",
            AvatarDirection::Right => "Right",
        };

        let mut persp_node = TheTreeNode::new(TheId::named(dir_name));
        persp_node.set_root_mode(false);
        persp_node.set_open(true);

        let mut icons = TheTreeIcons::new(TheId::named_with_reference(
            &format!("Avatar Perspective Icons {}", persp_index),
            animation.id,
        ));
        icons.set_icon_size(24);
        icons.set_icons_per_row(10);
        icons.set_icon_count(perspective.frames.len());
        for (i, frame) in perspective.frames.iter().enumerate() {
            icons.set_icon(i, frame.texture.to_rgba());
        }
        icons.set_selected_index(None);
        persp_node.add_widget(Box::new(icons));

        node.add_child(persp_node);
    }

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

/// Apply the current palette to the tree.
pub fn apply_palette(
    ui: &mut TheUI,
    _ctx: &mut TheContext,
    server_ctx: &mut ServerContext,
    project: &mut Project,
) {
    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
        if let Some(palette_node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_palette_id) {
            palette_node.widgets.clear();
            palette_node.childs.clear();

            let mut item = TheTreeIcons::new(TheId::named("Palette Item"));
            item.set_icon_count(256);
            item.set_icons_per_row(17);
            item.set_palette(&project.palette);

            palette_node.add_widget(Box::new(item));
        }
    }
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
                server_ctx.curr_region = id;
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("{}: {}", fl!("region"), region.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Tiles".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::RegionSettings(id) => {
            if let Some(region) = project.get_region(&id) {
                server_ctx.curr_region = id;
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
                server_ctx.curr_region = id;
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
                server_ctx.curr_region = id;
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
        ProjectContext::CharacterPreviewRigging(id) => {
            if let Some(region) = project.characters.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Character Preview Rigging: {}", region.name)),
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
        ProjectContext::Avatar(id) => {
            if let Some(avatar) = project.avatars.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Avatar: {}", avatar.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Tiles".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::AvatarAnimation(avatar_id, anim_id, frame) => {
            if let Some(avatar) = project.avatars.get(&avatar_id) {
                if let Some(anim) = avatar.animations.iter().find(|a| a.id == anim_id) {
                    ui.set_widget_value(
                        "Project Context",
                        ctx,
                        TheValue::Text(format!(
                            "{} - {} (Frame {})",
                            avatar.name, anim.name, frame
                        )),
                    );
                }
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
        ProjectContext::DebugLog => {
            // ui.set_widget_value("LogEdit", ctx, TheValue::Text(log));
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Log".into(), ui, ctx, project, server_ctx);
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
