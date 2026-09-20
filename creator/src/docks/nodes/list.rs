use super::*;
const LIST: &str = "Node Catalog List";

pub fn has_catalog(pc: ProjectContext) -> bool {
    NodesDock::owner(pc).is_some()
}
pub fn node_available(pc: ProjectContext, key: &str) -> bool {
    if !has_catalog(pc) {
        return false;
    }
    match key {
        "event" | "filter" | "time_range" => true,
        "player_camera" | "routine" | "random_walk" | "resume_routine" | "go_to" | "lookout"
        | "engage" | "use_action" => matches!(
            pc,
            ProjectContext::Character(_)
                | ProjectContext::CharacterCode(_)
                | ProjectContext::RegionCharacterInstance(_, _)
        ),
        "say" => !matches!(
            pc,
            ProjectContext::WorldCode | ProjectContext::RegionCode(_)
        ),
        _ => false,
    }
}
pub fn node_help(key: &str) -> String {
    match key {
        "time_range" => fl!("node_time_range_help"),
        "go_to" => fl!("node_go_to_help"),
        "lookout" => fl!("node_lookout_help"),
        "engage" => fl!("node_engage_help"),
        "use_action" => fl!("node_use_action_help"),
        "event" => fl!("node_event_help"),
        "filter" => fl!("node_filter_help"),
        "say" => fl!("node_say_help"),
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
    let defs = catalog::definitions();
    for (_key, title, color) in [
        ("event", fl!("node_group_events"), [16, 112, 98, 255]),
        ("filter", fl!("node_group_logic"), [164, 98, 35, 255]),
        ("say", fl!("node_group_actions"), [35, 87, 134, 255]),
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
