use super::*;
const LIST: &str = "Node Catalog List";

pub fn has_catalog(pc: ProjectContext) -> bool {
    NodesDock::owner(pc).is_some()
}
pub fn node_available(pc: ProjectContext, key: &str) -> bool {
    if !has_catalog(pc) {
        return false;
    }
    if entity::is_configuration(pc) {
        return matches!(
            key,
            "entity"
                | "entity_appearance"
                | "entity_body"
                | "entity_inventory"
                | "entity_attribute"
                | "entity_light"
        ) || (matches!(pc, ProjectContext::CharacterData(_))
            && matches!(key, "entity_player" | "entity_input" | "entity_inputs"))
            || (matches!(pc, ProjectContext::ItemData(_)) && key == "entity_ruleset_item");
    }
    match key {
        "event" | "on_event" | "filter" | "time_range" => true,
        "quest_state" | "set_quest" | "quest_guard" | "item_guard" | "player_attribute_guard" => {
            matches!(
                pc,
                ProjectContext::Character(_)
                    | ProjectContext::CharacterCode(_)
                    | ProjectContext::RegionCharacterInstance(_, _)
                    | ProjectContext::RegionArea(_, _)
            )
        }
        // A named place owns its reactions, so its entry is On Area.
        "on_area" => matches!(pc, ProjectContext::RegionArea(_, _)),
        "on_enter_area" => !matches!(pc, ProjectContext::RegionArea(_, _)),
        "player_camera" | "routine" | "random_walk" | "resume_routine" | "go_to" | "lookout"
        | "engage" | "use_action" => matches!(
            pc,
            ProjectContext::Character(_)
                | ProjectContext::CharacterCode(_)
                | ProjectContext::RegionCharacterInstance(_, _)
        ),
        "say" | "set_attribute" | "teleport" | "message" | "state" | "add_item" | "drop_items"
        | "set_emit_light" | "notify_in" | "entities_in_radius" | "dialog" | "prompt"
        | "inventory_has" | "offer_inventory" => !matches!(
            pc,
            ProjectContext::WorldCode | ProjectContext::RegionCode(_)
        ),
        _ => false,
    }
}
pub fn node_help(key: &str) -> String {
    if key.starts_with("entity") {
        return entity::help(key);
    }
    match key {
        "time_range" => fl!("node_time_range_help"),
        "go_to" => fl!("node_go_to_help"),
        "lookout" => fl!("node_lookout_help"),
        "engage" => fl!("node_engage_help"),
        "use_action" => fl!("node_use_action_help"),
        "event" => fl!("node_event_help"),
        "on_enter_area" => fl!("node_on_enter_area_help"),
        "on_area" => fl!("node_on_area_help"),
        "filter" => fl!("node_filter_help"),
        "say" => fl!("node_say_help"),
        "set_attribute" => fl!("node_set_attribute_help"),
        "teleport" => fl!("node_teleport_help"),
        "message" => fl!("node_message_help"),
        "state" => fl!("node_state_help"),
        "on_event" => fl!("node_on_event_help"),
        "add_item" => fl!("node_add_item_help"),
        "drop_items" => fl!("node_drop_items_help"),
        "set_emit_light" => fl!("node_set_emit_light_help"),
        "notify_in" => fl!("node_notify_in_help"),
        "entities_in_radius" => fl!("node_entities_in_radius_help"),
        "dialog" => fl!("node_dialog_help"),
        "dialogue" => fl!("node_dialogue_help"),
        "prompt" => fl!("node_prompt_help"),
        "quest_guard" => fl!("node_quest_guard_help"),
        "item_guard" => fl!("node_item_guard_help"),
        "player_attribute_guard" => fl!("node_player_attribute_guard_help"),
        "talk" => fl!("node_talk_help"),
        "quest_state" => fl!("node_quest_state_help"),
        "set_quest" => fl!("node_set_quest_help"),
        "inventory_has" => fl!("node_inventory_has_help"),
        "offer_inventory" => fl!("node_offer_inventory_help"),
        "player_camera" => fl!("node_player_camera_help"),
        "routine" => fl!("node_routine_help"),
        "random_walk" => fl!("node_random_walk_help"),
        "resume_routine" => fl!("node_resume_routine_help"),
        _ => String::new(),
    }
}
pub fn node_list_canvas() -> TheCanvas {
    let mut canvas = TheCanvas::new();
    canvas.set_layout(TheListLayout::new(TheId::named(LIST)));
    let mut top = TheCanvas::new();
    top.set_widget(TheTraybar::new(TheId::empty()));
    let mut layout = TheHLayout::new(TheId::empty());
    layout.set_margin(Vec4::new(10, 1, 5, 1));
    layout.set_background_color(None);
    let mut text = TheText::new(TheId::named("Node Catalog Heading"));
    text.set_text(fl!("node_list"));
    text.set_status_text(&fl!("node_list_help"));
    layout.add_widget(Box::new(text));
    top.set_layout(layout);
    canvas.set_top(top);
    canvas.top_is_expanding = false;
    canvas
}
pub fn sync_node_list(ui: &mut TheUI, ctx: &mut TheContext, pc: ProjectContext) {
    let Some(list) = ui.get_list_layout(LIST) else {
        return;
    };
    list.clear();
    let defs = if entity::is_configuration(pc) {
        entity::definitions(&Project::default())
    } else {
        catalog::definitions()
    };
    for (_key, title, color) in [
        ("event", fl!("node_group_events"), [16, 112, 98, 255]),
        ("filter", fl!("node_group_logic"), [164, 98, 35, 255]),
        ("say", fl!("node_group_actions"), [35, 87, 134, 255]),
        (
            "entity",
            fl!("entity_group_configuration"),
            [91, 86, 151, 255],
        ),
        (
            "entity_input",
            fl!("entity_group_input"),
            [35, 87, 134, 255],
        ),
    ] {
        let nodes: Vec<_> = defs
            .nodes()
            .filter(|n| n.category == title && node_available(pc, &n.id))
            .collect();
        if nodes.is_empty() {
            continue;
        }
        for node in nodes {
            let mut item = TheListItem::new(TheId::named(&format!("Node Catalog/{}", node.id)));
            item.set_text(node.title.clone());
            item.set_status_text(&node_help(&node.id));
            item.set_background_color(TheColor::from_u8_array([
                color[0] / 2,
                color[1] / 2,
                color[2] / 2,
                255,
            ]));
            item.set_text_color([235, 235, 235, 255]);
            list.add_item(item, ctx);
        }
    }
    if !has_catalog(pc) {
        let mut item = TheListItem::new(TheId::named("Node Catalog Empty"));
        item.set_text(fl!("node_list_empty"));
        list.add_item(item, ctx);
    }
    ctx.ui.redraw_all = true;
}

const BRANCH_LIST: &str = "Node Branches List";

/// Left panel listing the graph's branches, one trigger each.
pub fn branch_list_canvas() -> TheCanvas {
    let mut canvas = TheCanvas::new();
    let mut list = TheListLayout::new(TheId::named(BRANCH_LIST));
    list.limiter_mut().set_min_width(190);
    list.limiter_mut().set_max_width(190);
    canvas.set_layout(list);
    let mut top = TheCanvas::new();
    top.set_widget(TheTraybar::new(TheId::empty()));
    let mut layout = TheHLayout::new(TheId::empty());
    layout.set_margin(Vec4::new(10, 1, 5, 1));
    layout.set_background_color(None);
    let mut text = TheText::new(TheId::named("Node Branches Heading"));
    text.set_text(fl!("node_branches"));
    text.set_status_text(&fl!("node_branches_help"));
    layout.add_widget(Box::new(text));
    top.set_layout(layout);
    canvas.set_top(top);
    canvas.top_is_expanding = false;
    canvas
}

/// One branch row: `(item id, label, active)`.
pub fn sync_branch_list(ui: &mut TheUI, ctx: &mut TheContext, items: &[(String, String, bool)]) {
    ui.set_widget_value(
        "Node Branches Heading",
        ctx,
        TheValue::Text(fl!("node_branches")),
    );
    let Some(list) = ui.get_list_layout(BRANCH_LIST) else {
        return;
    };
    list.clear();
    for (id, label, active) in items {
        let mut item = TheListItem::new(TheId::named(id));
        item.set_text(label.clone());
        item.set_status_text(label);
        let tint = if *active {
            [64, 96, 120, 255]
        } else {
            [40, 42, 44, 255]
        };
        item.set_background_color(TheColor::from_u8_array(tint));
        item.set_text_color([235, 235, 235, 255]);
        list.add_item(item, ctx);
    }
    if items.is_empty() {
        // `Branch/None` is not a UUID, so a click on it is ignored.
        let mut item = TheListItem::new(TheId::named("Branch/None"));
        item.set_text(fl!("node_branch_none"));
        item.set_text_color([163, 169, 166, 255]);
        list.add_item(item, ctx);
    }
    ctx.ui.redraw_all = true;
}
