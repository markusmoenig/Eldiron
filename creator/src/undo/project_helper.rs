use crate::editor::{DOCKMANAGER, PALETTE, RUSTERIX, SCENEMANAGER, TOOLLIST};
use crate::prelude::*;
use rusterix::material_library::MATERIAL_PRESET_NAMES;
use theframework::prelude::*;

const CHARACTER_INSTANCE_TREE_PALETTE_SLOT: usize = ActionGroup::Prefab.palette_slot();
const ITEM_INSTANCE_TREE_PALETTE_SLOT: usize = ActionGroup::Bake.palette_slot();
const AREA_TREE_PALETTE_SLOT: usize = ActionGroup::Geometry.palette_slot();
const RULESET_ITEM_TREE_PALETTE_SLOT: usize = ITEM_INSTANCE_TREE_PALETTE_SLOT;

fn tiles_or_authoring_dock() -> String {
    let toollist = TOOLLIST.read().unwrap();
    if toollist.authoring_mode {
        "Authoring".into()
    } else if !toollist.editor_mode {
        match toollist.game_tools[toollist.curr_game_tool]
            .id()
            .name
            .as_str()
        {
            _ => "Tiles".into(),
        }
    } else {
        "Tiles".into()
    }
}

fn update_project_export_context_menu(ui: &mut TheUI, pc: ProjectContext, project: &Project) {
    let Some(id) = pc.id() else {
        ui.set_widget_context_menu("Project Export Menu", None);
        return;
    };

    let mut menu = TheContextMenu::default();
    if pc.is_region() {
        menu.add(TheContextMenuItem::new(
            fl!("export_region"),
            TheId::named_with_reference("Export Region", id),
        ));
    } else if pc.is_character() {
        menu.add(TheContextMenuItem::new(
            fl!("export_character"),
            TheId::named_with_reference("Export Character", id),
        ));
    } else if matches!(
        pc,
        ProjectContext::Avatar(_) | ProjectContext::AvatarAnimation(_, _, _)
    ) {
        menu.add(TheContextMenuItem::new(
            fl!("export_avatar_atlas"),
            TheId::named_with_reference("Export Avatar Atlas", id),
        ));
        menu.add(TheContextMenuItem::new(
            fl!("export_avatar_json"),
            TheId::named_with_reference("Export Avatar JSON", id),
        ));
    } else if pc.is_item() {
        menu.add(TheContextMenuItem::new(
            fl!("export_item"),
            TheId::named_with_reference("Export Item", id),
        ));
    } else if pc.is_tilemap() {
        menu.add(TheContextMenuItem::new(
            fl!("export_tileset"),
            TheId::named_with_reference("Export Tileset", id),
        ));
    } else if pc.is_screen() {
        menu.add(TheContextMenuItem::new(
            fl!("export_screen"),
            TheId::named_with_reference("Export Screen", id),
        ));
    } else if pc.is_asset()
        && let Some(asset) = project.assets.get(&id)
    {
        match &asset.buffer {
            AssetBuffer::Audio(_) => menu.add(TheContextMenuItem::new(
                fl!("export_audio_asset"),
                TheId::named_with_reference("Export Audio Asset", id),
            )),
            _ => menu.add(TheContextMenuItem::new(
                fl!("export_font_asset"),
                TheId::named_with_reference("Export Font Asset", id),
            )),
        }
    }

    if menu.items.is_empty() {
        ui.set_widget_context_menu("Project Export Menu", None);
    } else {
        ui.set_widget_context_menu("Project Export Menu", Some(menu));
    }
}

fn data_attr_bool(data: &str, key: &str) -> bool {
    let Ok(value) = data.parse::<toml::Value>() else {
        return false;
    };

    let attrs = value
        .get("attributes")
        .and_then(|v| v.as_table())
        .or_else(|| value.as_table());

    let Some(attrs) = attrs else {
        return false;
    };

    match attrs.get(key) {
        Some(toml::Value::Boolean(b)) => *b,
        Some(toml::Value::Integer(i)) => *i != 0,
        Some(toml::Value::String(s)) => {
            let s = s.trim().to_ascii_lowercase();
            s == "true" || s == "1" || s == "yes"
        }
        _ => false,
    }
}

fn data_has_attr(data: &str, key: &str) -> bool {
    let Ok(value) = data.parse::<toml::Value>() else {
        return false;
    };

    let attrs = value
        .get("attributes")
        .and_then(|v| v.as_table())
        .or_else(|| value.as_table());

    attrs.is_some_and(|attrs| attrs.contains_key(key))
}

/// Generate a tree node for the given region
/// Named places that can own behavior: 2D sectors and named 3D geometry areas.
pub fn region_areas(region: &Region) -> Vec<(Uuid, String)> {
    let mut areas: Vec<(Uuid, String)> = region
        .map
        .sectors
        .iter()
        .filter(|sector| !sector.name.trim().is_empty())
        .map(|sector| (sector.creator_id, sector.name.clone()))
        .collect();
    areas.extend(
        region
            .map
            .geometry_objects
            .iter()
            .filter(|object| {
                !object.name.trim().is_empty() && object.properties.get_bool_default("area", false)
            })
            .map(|object| (object.id, object.name.clone())),
    );
    areas
}

/// Map-space centre of a named place, used to centre the view on it. 2D
/// sectors report their polygon centre; named 3D areas use their bounding box.
pub fn area_center(region: &Region, area: Uuid) -> Option<(f32, f32)> {
    if let Some(sector) = region
        .map
        .sectors
        .iter()
        .find(|sector| sector.creator_id == area)
    {
        return sector
            .center(&region.map)
            .map(|center| (center.x, center.y));
    }
    region
        .map
        .geometry_objects
        .iter()
        .find(|object| object.id == area)
        .and_then(|object| object.bbox())
        .map(|bbox| {
            (
                (bbox.min.x + bbox.max.x) * 0.5,
                (bbox.min.y + bbox.max.y) * 0.5,
            )
        })
}

/// The region that contains a named place, found by its stable id.
pub fn region_of_area(project: &Project, area: Uuid) -> Option<Uuid> {
    project
        .regions
        .iter()
        .find(|region| region_areas(region).iter().any(|(id, _)| *id == area))
        .map(|region| region.id)
}

pub fn gen_region_tree_node(region: &Region) -> TheTreeNode {
    let mut node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(&region.name, region.id));
    node.set_root_mode(false);

    gen_region_tree_items(&mut node, region);

    node
}

/// Refresh instance/place rows without changing the active region or tree expansion.
pub fn refresh_region_tree_widgets(ui: &mut TheUI, project: &Project) {
    if let Some(tree) = ui.get_tree_layout("Project Tree") {
        for region in &project.regions {
            if let Some(node) = tree.get_node_by_id_mut(&region.id) {
                gen_region_tree_items(node, region);
            }
        }
    }
}

/// Apply transaction/undo row differences while keeping unrelated in-progress name edits.
pub fn refresh_region_tree_diff(ui: &mut TheUI, before: &Project, after: &Project) {
    if let Some(tree) = ui.get_tree_layout("Project Tree") {
        for region in &after.regions {
            let old = before.regions.iter().find(|r| r.id == region.id);
            let rows = |r: &Region| {
                (
                    r.name.clone(),
                    r.characters
                        .values()
                        .map(|c| (c.id, c.name.clone()))
                        .collect::<Vec<_>>(),
                    r.items
                        .values()
                        .map(|i| (i.id, i.name.clone()))
                        .collect::<Vec<_>>(),
                    region_areas(r),
                )
            };
            if old.is_some_and(|r| rows(r) == rows(region)) {
                continue;
            }
            if let Some(node) = tree.get_node_by_id_mut(&region.id) {
                let name = if old.is_some_and(|r| r.name == region.name) && !node.widgets.is_empty()
                {
                    Some(node.widgets.remove(0))
                } else {
                    None
                };
                gen_region_tree_items(node, region);
                if let Some(name) = name {
                    node.widgets[0] = name;
                }
            }
        }
    }
}

/// Region-level Settings and Behaviour Nodes are hidden for now. The graph
/// entry has no usable actions yet (`node_available` filters every action out
/// for region owners) and the settings panel is superseded by the
/// project/region workflow. The code and the sidebar handlers stay in place, so
/// flipping this to `true` brings both entries back.
const SHOW_REGION_SETTINGS_AND_BEHAVIOR_NODES: bool = false;

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

    if SHOW_REGION_SETTINGS_AND_BEHAVIOR_NODES {
        // Settings
        let mut item = TheTreeItem::new(TheId::named_with_reference(
            "Region Settings Item",
            region.id,
        ));
        item.set_text(fl!("settings"));
        node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named_with_reference("Region Code Item", region.id));
        item.set_background_palette(ActionGroups, ActionRole::Dock.palette_slot());
        item.set_text(fl!("behavior_nodes"));
        node.add_widget(Box::new(item));
    }

    for (id, character) in &region.characters {
        let mut item = TheTreeItem::new(TheId::named_with_id("Region Content List Item", *id));
        item.add_value_column(200, TheValue::Text(fl!("character_instance")));
        item.set_background_palette(ActionGroups, CHARACTER_INSTANCE_TREE_PALETTE_SLOT);
        item.set_text(character.name.clone());
        node.add_widget(Box::new(item));
    }

    for (id, item_) in &region.items {
        let mut item = TheTreeItem::new(TheId::named_with_id("Region Content List Item", *id));
        item.add_value_column(200, TheValue::Text(fl!("item_instance")));
        item.set_background_palette(ActionGroups, ITEM_INSTANCE_TREE_PALETTE_SLOT);
        item.set_text(item_.name.clone());
        node.add_widget(Box::new(item));
    }

    // Named places are listed after the instances. Selecting one opens the
    // behavior graph that place owns.
    for (area_id, name) in region_areas(region) {
        let mut item = TheTreeItem::new(TheId::named_with_reference("Region Area Item", area_id));
        item.add_value_column(200, TheValue::Text(fl!("node_area")));
        item.set_background_palette(ActionGroups, AREA_TREE_PALETTE_SLOT);
        item.set_text(name);
        node.add_widget(Box::new(item));
    }
}

/// Returns a TheTreeNode for the character.
pub fn gen_character_tree_node(character: &Character) -> TheTreeNode {
    let mut node: TheTreeNode =
        TheTreeNode::new(TheId::named_with_id(&character.name, character.id));
    node.set_root_mode(false);
    if data_attr_bool(&character.data, "player") {
        node.set_background_palette(ActionGroups, ActionRole::Camera.palette_slot());
    }

    let mut item = TheTreeItem::new(TheId::named_with_reference("Character Item", character.id));
    item.set_text(fl!("name"));

    let mut edit = TheTextLineEdit::new(TheId::named_with_id(
        "Character Item Name Edit",
        character.id,
    ));
    edit.set_text(character.name.clone());
    item.add_widget_column(200, Box::new(edit));

    node.add_widget(Box::new(item));

    let mut appearance = TheTreeItem::new(TheId::named_with_reference(
        "Character Appearance",
        character.id,
    ));
    appearance.set_text("Appearance".into());
    appearance.set_status_text("Body, rig, material slots and saved equipment preview");
    node.add_widget(Box::new(appearance));

    let mut item = TheTreeItem::new(TheId::named_with_reference(
        "Character Item Code Edit",
        character.id,
    ));
    item.set_background_palette(ActionGroups, ActionRole::Dock.palette_slot());
    item.set_text(fl!("behavior_nodes"));
    node.add_widget(Box::new(item));

    let mut item = TheTreeItem::new(TheId::named_with_reference(
        "Character Item Data Edit",
        character.id,
    ));
    item.set_background_palette(ActionGroups, ActionRole::Dock.palette_slot());
    item.set_text(fl!("entity_nodes"));
    node.add_widget(Box::new(item));

    node
}

/// Returns a TheTreeNode for the item.
pub fn resolved_item_default_icon_frames(item: &Item, project: &Project) -> Vec<rusterix::Texture> {
    resolved_item_default_icon_frames_for_state(item, project, true)
}

pub fn resolved_item_default_icon_frames_for_state(
    item: &Item,
    project: &Project,
    on: bool,
) -> Vec<rusterix::Texture> {
    if let Ok(Some(frames)) = shared::ruleset_materials::frames_for_item(project, item, on) {
        return frames;
    }
    let mut default_assets = RUSTERIX.read().unwrap().assets.clone();
    default_assets.item_icons.clear();
    default_assets.project_item_icon_keys.clear();
    if let Ok(states) = shared::rulesets::bundled_item_icon_states_for_project(&project.config) {
        for (item_id, state, frames) in states {
            let item_id = item_id.trim().to_ascii_lowercase();
            let state = state.trim().to_ascii_lowercase();
            default_assets
                .item_icons
                .insert(format!("{item_id}:{state}"), frames.clone());
            if state == "on" {
                default_assets.item_icons.insert(item_id, frames);
            }
        }
    }
    shared::rusterix_utils::insert_procedural_ruleset_icons(&mut default_assets, project);

    let mut runtime_item = rusterix::Item::default();
    rusterix::server::data::apply_item_data(&mut runtime_item, &item.data);

    let resolve_tile_frames = |source: &rusterix::PixelSource| {
        source
            .tile_from_tile_list(&default_assets)
            .or_else(|| {
                let mut assets = rusterix::server::assets::Assets::new();
                for tile in project.tiles.values() {
                    assets.tiles.insert(tile.id, tile.clone());
                }
                source.tile_from_tile_list(&assets)
            })
            .filter(|tile| !tile.textures.is_empty())
            .map(|tile| tile.textures)
    };

    // A state-specific world tile is also the default editable icon for that
    // state. Custom icon frames remain stored on the item; Load Default calls
    // this resolver again and therefore reloads the current mapped tile.
    if let Some(source) = runtime_item.tile_source_for_active_state(on)
        && let Some(frames) = resolve_tile_frames(&source)
    {
        return frames;
    }

    let state = if on { "on" } else { "off" };
    let keys = [
        runtime_item.attributes.get_str("ruleset_path"),
        runtime_item.attributes.get_str("ruleset_id"),
        runtime_item.attributes.get_str("class_name"),
        runtime_item.attributes.get_str("name"),
    ];
    if let Some(frames) = keys
        .into_iter()
        .flatten()
        .map(|key| format!("{}:{state}", key.trim().to_ascii_lowercase()))
        .find_map(|key| default_assets.item_icons.get(&key).cloned())
    {
        return frames;
    }
    if !on {
        return Vec::new();
    }

    if let Some(rusterix::Value::Source(source)) = runtime_item.attributes.get("source")
        && let Some(frames) = resolve_tile_frames(source)
    {
        return frames;
    }

    rusterix::client::widget::Widget::item_generated_icon_square(&default_assets, &runtime_item)
        .map(|(size, rgba)| vec![rusterix::Texture::new(rgba, size as usize, size as usize)])
        .unwrap_or_default()
}

pub fn gen_item_tree_node(item_: &Item, _project: &Project) -> TheTreeNode {
    let mut node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(&item_.name, item_.id));
    if let Some(header) = node.widget.as_any().downcast_mut::<TheSnapperbar>() {
        header.set_drag_id(Some(TheId::named_with_reference("Item Item", item_.id)));
    }
    node.set_root_mode(false);
    if data_has_attr(&item_.data, "ruleset_path") {
        node.set_background_palette(ActionGroups, RULESET_ITEM_TREE_PALETTE_SLOT);
    } else if data_attr_bool(&item_.data, "is_spell") {
        node.set_background_palette(ActionGroups, ActionRole::Editor.palette_slot());
    }

    let mut item = TheTreeItem::new(TheId::named_with_reference("Item Item", item_.id));
    item.set_text(fl!("name"));

    let mut edit = TheTextLineEdit::new(TheId::named_with_id("Item Item Name Edit", item_.id));
    edit.set_text(item_.name.clone());
    item.add_widget_column(200, Box::new(edit));

    node.add_widget(Box::new(item));

    let mut appearance = TheTreeItem::new(TheId::named_with_reference("Item Appearance", item_.id));
    appearance.set_text("Appearance".into());
    appearance.set_status_text("Model, material slots, icons and equipped preview");
    node.add_widget(Box::new(appearance));

    let mut item = TheTreeItem::new(TheId::named_with_reference("Item Item Code Edit", item_.id));
    item.set_background_palette(ActionGroups, ActionRole::Dock.palette_slot());
    item.set_text(fl!("behavior_nodes"));
    node.add_widget(Box::new(item));

    let mut item = TheTreeItem::new(TheId::named_with_reference("Item Item Data Edit", item_.id));
    item.set_background_palette(ActionGroups, ActionRole::Dock.palette_slot());
    item.set_text(fl!("entity_nodes"));
    node.add_widget(Box::new(item));

    node
}

/// Keep the item catalog compact; appearance controls live in one workspace.
pub fn item_tree_category(item: &Item) -> String {
    if data_attr_bool(&item.data, "is_spell") {
        return "Spells".into();
    }
    if crate::appearance_workspace::data::attribute(&item.data, "category").as_deref()
        == Some("utility")
    {
        return "Utility".into();
    }
    if let Some(path) = crate::appearance_workspace::data::attribute(&item.data, "ruleset_path") {
        let parts = path.split('.').collect::<Vec<_>>();
        let group = if parts.first() == Some(&"items") && parts.len() >= 3 {
            parts[1]
        } else {
            "ruleset_items"
        };
        if group == "tools" {
            return "Utility".into();
        }
        return group
            .split('_')
            .map(|word| {
                let mut chars = word.chars();
                chars
                    .next()
                    .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" ");
    }
    "Custom Items".into()
}
pub fn append_item_tree(root: &mut TheTreeNode, item: &Item, project: &Project, open: bool) {
    let category = item_tree_category(item);
    let id = Uuid::from_u128(
        (0x4150504954454d47u128 << 64)
            | rusterix::procedural_avatar::fingerprint(&category) as u128,
    );
    if !root.childs.iter().any(|n| n.id.uuid == id) {
        let mut group = TheTreeNode::new(TheId::named_with_id(&category, id));
        group.set_root_mode(false);
        group.set_open(false);
        root.add_child(group);
    }
    let group = root.childs.iter_mut().find(|n| n.id.uuid == id).unwrap();
    let mut node = gen_item_tree_node(item, project);
    node.set_open(open);
    group.add_child(node);
    if open {
        group.set_open(true);
    }
}
/// Remove the definition and its category when the last item is removed.
pub fn remove_item_tree(root: &mut TheTreeNode, id: &Uuid) {
    root.remove_child_by_uuid(id);
    root.childs.retain(|group| !group.childs.is_empty());
}
pub fn populate_item_tree(root: &mut TheTreeNode, project: &Project) {
    fn remember(node: &TheTreeNode, states: &mut std::collections::HashMap<Uuid, bool>) {
        states.insert(node.id.uuid, node.open);
        for child in &node.childs {
            remember(child, states);
        }
    }
    fn restore(node: &mut TheTreeNode, states: &std::collections::HashMap<Uuid, bool>) {
        if let Some(open) = states.get(&node.id.uuid) {
            node.set_open(*open);
        }
        for child in &mut node.childs {
            restore(child, states);
        }
    }
    let mut states = std::collections::HashMap::new();
    remember(root, &mut states);
    root.clear();
    let mut items = project.items.values().collect::<Vec<_>>();
    items.sort_by_key(|i| (item_tree_category(i), i.name.to_lowercase()));
    for item in items {
        append_item_tree(root, item, project, false);
    }
    restore(root, &states);
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
    node.widget.set_context_menu(Some(TheContextMenu {
        items: vec![
            TheContextMenuItem::new(
                fl!("export_avatar_atlas"),
                TheId::named_with_reference("Export Avatar Atlas", avatar.id),
            ),
            TheContextMenuItem::new(
                fl!("export_avatar_json"),
                TheId::named_with_reference("Export Avatar JSON", avatar.id),
            ),
        ],
        ..Default::default()
    }));

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
    drop_down.add_option("8".to_string());
    drop_down.set_selected_index(match avatar.perspective_count {
        AvatarPerspectiveCount::One => 0,
        AvatarPerspectiveCount::Four => 1,
        AvatarPerspectiveCount::Eight => 2,
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
        let dir_name = perspective.direction.label();

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

    let mut item = TheTreeItem::new(TheId::named_with_reference(
        "Screen Settings Item",
        screen.id,
    ));
    item.set_text(fl!("settings"));
    node.add_widget(Box::new(item));

    for sector in &screen.map.sectors {
        if !sector.name.is_empty() {
            let mut item = TheTreeItem::new(TheId::named_with_id_and_reference(
                "Screen Content List Item",
                sector.creator_id,
                screen.id,
            ));
            item.add_value_column(200, TheValue::Text("Widget".to_string()));
            item.set_background_palette(ActionGroups, ActionRole::Dock.palette_slot());
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
pub(crate) fn palette_material_values(project: &Project) -> Vec<[f32; 4]> {
    project
        .palette_materials
        .iter()
        .map(|m| m.rmoe_values())
        .collect()
}

pub(crate) fn palette_material_ids(project: &Project) -> Vec<u8> {
    project
        .palette_materials
        .iter()
        .map(|m| m.material_id())
        .collect()
}

pub fn palette_status_text(
    index: usize,
    color: Option<&TheColor>,
    material: Option<&shared::project::PaletteMaterial>,
) -> String {
    fn preset_label(value: &str) -> String {
        match value.trim().to_ascii_lowercase().as_str() {
            "stone" => fl!("material_preset_stone"),
            "dirt" => fl!("material_preset_dirt"),
            "wood" => fl!("material_preset_wood"),
            "metal" => fl!("material_preset_metal"),
            "glass" => fl!("material_preset_glass"),
            "water" => fl!("material_preset_water"),
            "mirror" => fl!("material_preset_mirror"),
            "emissive" => fl!("material_preset_emissive"),
            "fabric" => fl!("material_preset_fabric"),
            "plastic" => fl!("material_preset_plastic"),
            "foliage" => fl!("material_preset_foliage"),
            "skin" => fl!("material_preset_skin"),
            "bone" => fl!("material_preset_bone"),
            "wax" => fl!("material_preset_wax"),
            _ => fl!("material_preset_default"),
        }
    }

    fn finish_label(value: &str) -> String {
        match value.trim().to_ascii_lowercase().as_str() {
            "matte" => fl!("material_finish_matte"),
            "polished" => fl!("material_finish_polished"),
            "wet" => fl!("material_finish_wet"),
            _ => fl!("material_finish_natural"),
        }
    }

    if let Some(color) = color {
        let material = material.cloned().unwrap_or_default();
        format!(
            "Palette Index {}. Color {}. Material: {}, Finish: {}.",
            index,
            color.to_hex(),
            preset_label(&material.preset),
            finish_label(&material.finish)
        )
    } else {
        format!("Palette Index {}. Empty", index)
    }
}

pub fn refresh_palette_runtime(project: &Project) {
    *PALETTE.write().unwrap() = project.palette.clone();
    let (tile_list, tile_indices) = {
        let mut rusterix = RUSTERIX.write().unwrap();
        rusterix.assets.palette = project.palette.clone();
        shared::rusterix_utils::sync_item_icon_assets(&mut rusterix, project);
        rusterix.assets.palette_materials = palette_material_values(project);
        rusterix.assets.palette_material_ids = palette_material_ids(project);
        rusterix.set_tiles(project.tiles.clone(), true);
        rusterix.set_dirty();
        (
            rusterix.assets.tile_list.clone(),
            rusterix.assets.tile_indices.clone(),
        )
    };
    SCENEMANAGER
        .write()
        .unwrap()
        .set_tile_list(tile_list, tile_indices);
    SCENEMANAGER.write().unwrap().set_palette(
        project.palette.clone(),
        palette_material_values(project),
        palette_material_ids(project),
    );
}

pub fn apply_palette(
    ui: &mut TheUI,
    ctx: &mut TheContext,
    server_ctx: &mut ServerContext,
    project: &mut Project,
) {
    if project.palette.current_index as usize >= project.palette.colors.len() {
        project.palette.current_index = project.palette.colors.len().saturating_sub(1) as u16;
    }
    project.ensure_palette_materials_len();
    // UI refresh is read-only for colors, including transparent/unused slots.
    // Rewriting Some(transparent) to None invalidates every saved material bake.
    *PALETTE.write().unwrap() = project.palette.clone();

    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
        if let Some(palette_node) = tree_layout.get_node_by_id_mut(&server_ctx.tree_palette_id) {
            palette_node.clear();

            let mut art_label = TheTreeItem::new(TheId::named("Palette Header"));
            art_label.set_text(fl!("palette"));
            art_label.set_background_palette(ActionGroups, ActionRole::Editor.palette_slot());
            palette_node.add_widget(Box::new(art_label));

            let mut art_item = TheTreeIcons::new(TheId::named("Palette Item"));
            let art_count = project.palette.colors.len();
            art_item.set_icon_count(art_count);
            art_item.set_icons_per_row(17);
            art_item.set_palette(&project.palette);
            art_item.set_icon_count(art_count);
            for index in 0..art_count {
                art_item.set_status_text_for(
                    index,
                    palette_status_text(
                        index,
                        project.palette.colors[index].as_ref(),
                        project.palette_materials.get(index),
                    ),
                );
            }
            art_item.set_selected_index(Some(project.palette.current_index as usize));

            palette_node.add_widget(Box::new(art_item));
        }
    }
    if let Some(widget) = ui.get_widget("Palette Item") {
        widget.set_needs_redraw(true);
    }
    ctx.ui.relayout = true;

    if let Some(palette_picker) = ui.get_palette_picker("Palette Picker") {
        palette_picker.set_palette(project.palette.clone());
        palette_picker.set_index(project.palette.current_index as usize);
    }
    if let Some(widget) = ui.get_widget("Palette Dock Picker")
        && let Some(board) = widget
            .as_any()
            .downcast_mut::<crate::docks::palette::PaletteDockBoard>()
    {
        board.set_palette(project.palette.clone());
        board.set_materials(project.palette_materials.clone());
        board.set_index(project.palette.current_index as usize);
    }

    let index = project.palette.current_index as usize;
    let hex = project.palette[index]
        .as_ref()
        .map(TheColor::to_hex)
        .unwrap_or_default();

    if let Some(widget) = ui.get_widget("Palette Hex Edit") {
        widget.set_value(TheValue::Text(hex.clone()));
    }
    if let Some(widget) = ui.get_widget("Palette Dock Hex Edit") {
        widget.set_value(TheValue::Text(hex));
    }

    if let Some(color) = project.palette[index].clone()
        && let Some(widget) = ui.get_widget("Palette Color Picker")
    {
        widget.set_value(TheValue::ColorObject(color));
    }

    if let Some(widget) = ui.get_widget("Palette Color Picker") {
        widget.limiter_mut().set_max_size(Vec2::new(200, 200));
        widget.set_needs_redraw(true);
    }
    for id in [
        "Palette Color Picker",
        "Palette Hex Edit",
        "Palette Clear",
        "Palette Import",
    ] {
        ui.set_widget_disabled_state(id, ctx, false);
    }

    if let Some(widget) = ui.get_widget("Palette Index Text") {
        widget.set_value(TheValue::Text(format!(
            "{:03}",
            project.palette.current_index
        )));
    }
    if let Some(widget) = ui.get_widget("Palette Dock Index") {
        widget.set_value(TheValue::Text(format!(
            "Palette {}",
            project.palette.current_index
        )));
    }

    let material = project
        .palette_materials
        .get(index)
        .cloned()
        .unwrap_or_default();
    let finish_values = ["natural", "matte", "polished", "wet"];
    let preset_index = MATERIAL_PRESET_NAMES
        .iter()
        .position(|value| value.eq_ignore_ascii_case(material.preset.trim()))
        .unwrap_or(0) as i32;
    let finish_index = finish_values
        .iter()
        .position(|value| value.eq_ignore_ascii_case(material.finish.trim()))
        .unwrap_or(0) as i32;
    if let Some(widget) = ui.get_widget("Palette Dock Material Preset") {
        widget.set_value(TheValue::Int(preset_index));
    }
    if let Some(widget) = ui.get_widget("Palette Dock Material Finish") {
        widget.set_value(TheValue::Int(finish_index));
    }

    ctx.ui.send(TheEvent::Custom(
        TheId::named("Update Minimap"),
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
    let active_dock = crate::editor::DOCKMANAGER.read().unwrap().dock.clone();
    if active_dock == "Prefabs" {
        crate::docks::prefab_nodes::sync_node_list(ui, ctx);
    } else if active_dock != "Construction" {
        crate::docks::nodes::sync_node_list(ui, ctx, pc);
    }
    update_project_export_context_menu(ui, pc, project);

    let duplicate_allowed = matches!(
        pc,
        ProjectContext::Region(_)
            | ProjectContext::RegionSettings(_)
            | ProjectContext::RegionCode(_)
            | ProjectContext::RegionCharacterInstance(_, _)
            | ProjectContext::RegionItemInstance(_, _)
            | ProjectContext::WorldCode
            | ProjectContext::Character(_)
            | ProjectContext::CharacterCode(_)
            | ProjectContext::CharacterData(_)
            | ProjectContext::CharacterPreviewRigging(_)
            | ProjectContext::Item(_)
            | ProjectContext::ItemCode(_)
            | ProjectContext::ItemData(_)
            | ProjectContext::Screen(_)
            | ProjectContext::ScreenSettings(_)
            | ProjectContext::ScreenWidget(_, _)
            | ProjectContext::Avatar(_)
            | ProjectContext::AvatarAnimation(_, _, _)
    );
    ui.set_widget_disabled_state("Project Duplicate", ctx, !duplicate_allowed);

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
            DOCKMANAGER.write().unwrap().set_dock(
                tiles_or_authoring_dock(),
                ui,
                ctx,
                project,
                server_ctx,
            );
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
        ProjectContext::RegionCode(id) => {
            if let Some(region) = project.get_region(&id) {
                server_ctx.curr_region = id;
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Region Behavior: {}", region.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Nodes".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::RegionArea(region_id, area) => {
            if let Some(region) = project.get_region(&region_id) {
                server_ctx.curr_region = region_id;
                let name = region_areas(region)
                    .into_iter()
                    .find(|(id, _)| *id == area)
                    .map(|(_, name)| name)
                    .unwrap_or_default();
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Area Behavior: {}", name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Nodes".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::RegionCharacterInstance(id, instance) => {
            if let Some(region) = project.get_region(&id) {
                server_ctx.curr_region = id;
                let name = region
                    .characters
                    .get(&instance)
                    .map(|character| character.name.clone())
                    .unwrap_or_default();
                // An instance graph is layered over the template graph, so say
                // which of the two this instance is actually running.
                let owns_instance = project
                    .node_graphs
                    .contains_key(&format!("behavior/region/{id}/character/{instance}"));
                let source = if owns_instance {
                    "instance graph (overrides template)"
                } else {
                    "template graph"
                };
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("{name} · {source}")),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Nodes".into(), ui, ctx, project, server_ctx);
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
            DOCKMANAGER.write().unwrap().set_dock(
                tiles_or_authoring_dock(),
                ui,
                ctx,
                project,
                server_ctx,
            );
        }
        ProjectContext::Character(id) => {
            if let Some(region) = project.characters.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Character: {}", region.name)),
                );
            }
            DOCKMANAGER.write().unwrap().set_dock(
                tiles_or_authoring_dock(),
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
                .set_dock("Nodes".into(), ui, ctx, project, server_ctx);
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
                .set_dock("Nodes".into(), ui, ctx, project, server_ctx);
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
            DOCKMANAGER.write().unwrap().set_dock(
                tiles_or_authoring_dock(),
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
                .set_dock("Nodes".into(), ui, ctx, project, server_ctx);
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
                .set_dock("Nodes".into(), ui, ctx, project, server_ctx);
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
            // Screens are always edited in 2D preview mode.
            server_ctx.editor_view_mode = EditorViewMode::D2;
            ui.set_widget_value("Editor View Switch", ctx, TheValue::Int(0));
            if let Some(screen) = project.screens.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Screen: {}", screen.name)),
                );
            }
            DOCKMANAGER.write().unwrap().set_dock(
                tiles_or_authoring_dock(),
                ui,
                ctx,
                project,
                server_ctx,
            );
        }
        ProjectContext::ScreenSettings(id) => {
            server_ctx.editor_view_mode = EditorViewMode::D2;
            ui.set_widget_value("Editor View Switch", ctx, TheValue::Int(0));
            if let Some(screen) = project.screens.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Screen Settings: {}", screen.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Data".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::ScreenWidget(id, _widget_id) => {
            // Screens are always edited in 2D preview mode.
            server_ctx.editor_view_mode = EditorViewMode::D2;
            ui.set_widget_value("Editor View Switch", ctx, TheValue::Int(0));
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
            DOCKMANAGER.write().unwrap().set_dock(
                tiles_or_authoring_dock(),
                ui,
                ctx,
                project,
                server_ctx,
            );
        }

        ProjectContext::Avatar(id) => {
            if let Some(avatar) = project.avatars.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Avatar: {}", avatar.name)),
                );
            }
            DOCKMANAGER.write().unwrap().set_dock(
                tiles_or_authoring_dock(),
                ui,
                ctx,
                project,
                server_ctx,
            );
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
            DOCKMANAGER.write().unwrap().set_dock(
                tiles_or_authoring_dock(),
                ui,
                ctx,
                project,
                server_ctx,
            );
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
        ProjectContext::WorldCode => {
            ui.set_widget_value(
                "Project Context",
                ctx,
                TheValue::Text(fl!("world_behavior_nodes")),
            );
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Nodes".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::GameRules => {
            ui.set_widget_value("Project Context", ctx, TheValue::Text("Game Rules".into()));
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Rules".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::GameLocales => {
            ui.set_widget_value(
                "Project Context",
                ctx,
                TheValue::Text("Game Locales".into()),
            );
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Data".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::GameAudioFx => {
            ui.set_widget_value(
                "Project Context",
                ctx,
                TheValue::Text("Game Audio FX".into()),
            );
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Data".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::GameAuthoring => {
            ui.set_widget_value(
                "Project Context",
                ctx,
                TheValue::Text("Game Authoring".into()),
            );
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Data".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::GameShortcuts => {
            ui.set_widget_value(
                "Project Context",
                ctx,
                TheValue::Text("Game Shortcuts".into()),
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

    if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
        if let Some(new_id) = pc.id() {
            if let Some(node) = tree_layout.get_node_by_id_mut(&new_id)
                && let Some(snapper) = node.widget.as_any().downcast_mut::<TheSnapperbar>()
            {
                snapper.set_selected(true);
            }
        } else {
            match pc {
                ProjectContext::ProjectSettings => {
                    let target_id = {
                        let mut found: Option<TheId> = None;
                        for node in &mut tree_layout.get_root().childs {
                            if node.id.name == fl!("game") {
                                node.set_open(true);
                                for widget in &node.widgets {
                                    if widget.id().name == "Project Settings" {
                                        found = Some(widget.id().clone());
                                        break;
                                    }
                                }
                                break;
                            }
                        }
                        found
                    };
                    if let Some(id) = target_id {
                        tree_layout.new_item_selected(id);
                    }
                }
                ProjectContext::WorldCode => {
                    let target_id = {
                        let mut found: Option<TheId> = None;
                        for node in &mut tree_layout.get_root().childs {
                            if node.id.name == fl!("game") {
                                node.set_open(true);
                                for widget in &node.widgets {
                                    if widget.id().name == "World Code" {
                                        found = Some(widget.id().clone());
                                        break;
                                    }
                                }
                                break;
                            }
                        }
                        found
                    };
                    if let Some(id) = target_id {
                        tree_layout.new_item_selected(id);
                    }
                }
                ProjectContext::GameRules => {
                    let target_id = {
                        let mut found: Option<TheId> = None;
                        for node in &mut tree_layout.get_root().childs {
                            if node.id.name == fl!("game") {
                                node.set_open(true);
                                for widget in &node.widgets {
                                    if widget.id().name == "Game Rules" {
                                        found = Some(widget.id().clone());
                                        break;
                                    }
                                }
                                break;
                            }
                        }
                        found
                    };
                    if let Some(id) = target_id {
                        tree_layout.new_item_selected(id);
                    }
                }
                ProjectContext::GameLocales => {
                    let target_id = {
                        let mut found: Option<TheId> = None;
                        for node in &mut tree_layout.get_root().childs {
                            if node.id.name == fl!("game") {
                                node.set_open(true);
                                for widget in &node.widgets {
                                    if widget.id().name == "Game Locales" {
                                        found = Some(widget.id().clone());
                                        break;
                                    }
                                }
                                break;
                            }
                        }
                        found
                    };
                    if let Some(id) = target_id {
                        tree_layout.new_item_selected(id);
                    }
                }
                ProjectContext::GameAudioFx => {
                    let target_id = {
                        let mut found: Option<TheId> = None;
                        for node in &mut tree_layout.get_root().childs {
                            if node.id.name == fl!("game") {
                                node.set_open(true);
                                for widget in &node.widgets {
                                    if widget.id().name == "Game Audio FX" {
                                        found = Some(widget.id().clone());
                                        break;
                                    }
                                }
                                break;
                            }
                        }
                        found
                    };
                    if let Some(id) = target_id {
                        tree_layout.new_item_selected(id);
                    }
                }
                ProjectContext::GameAuthoring => {
                    let target_id = {
                        let mut found: Option<TheId> = None;
                        for node in &mut tree_layout.get_root().childs {
                            if node.id.name == fl!("game") {
                                node.set_open(true);
                                for widget in &node.widgets {
                                    if widget.id().name == "Game Authoring" {
                                        found = Some(widget.id().clone());
                                        break;
                                    }
                                }
                                break;
                            }
                        }
                        found
                    };
                    if let Some(id) = target_id {
                        tree_layout.new_item_selected(id);
                    }
                }
                ProjectContext::GameShortcuts => {
                    let target_id = {
                        let mut found: Option<TheId> = None;
                        for node in &mut tree_layout.get_root().childs {
                            if node.id.name == fl!("game") {
                                node.set_open(true);
                                for widget in &node.widgets {
                                    if widget.id().name == "Game Shortcuts" {
                                        found = Some(widget.id().clone());
                                        break;
                                    }
                                }
                                break;
                            }
                        }
                        found
                    };
                    if let Some(id) = target_id {
                        tree_layout.new_item_selected(id);
                    }
                }
                _ => {}
            }
        }
    }

    ctx.ui.send(TheEvent::Custom(
        TheId::named("Update Action List"),
        TheValue::Empty,
    ));

    if let Some(list) = ui.get_vlayout("Tool List Layout") {
        TOOLLIST.write().unwrap().set_active_editor(list, ctx);
    }

    ctx.ui.relayout = true;
}

#[cfg(test)]
mod palette_refresh_tests {
    use super::*;
    #[test]
    fn showing_palette_preserves_transparent_slots_and_saved_bakes() {
        let mut project = Project::default();
        project.palette.colors[5] = Some(TheColor::from_u8(30, 50, 80, 0));
        let id = Uuid::new_v4();
        project.procedural_material_cache_keys.insert(id, 123);
        let colors = serde_json::to_string(&project.palette.colors).unwrap();
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(100, 100, 1.);
        apply_palette(
            &mut ui,
            &mut ctx,
            &mut ServerContext::default(),
            &mut project,
        );
        assert_eq!(
            serde_json::to_string(&project.palette.colors).unwrap(),
            colors
        );
        assert_eq!(project.procedural_material_cache_keys[&id], 123);
    }
}
