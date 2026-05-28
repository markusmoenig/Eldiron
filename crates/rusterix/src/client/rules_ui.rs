use crate::{
    Assets, Entity, Item, Value,
    client::command::{ClientCommandBinding, parse_client_command},
};
use toml::Table;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RulesDescription {
    pub title: String,
    pub subtitle: Option<String>,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommandState {
    pub enabled: bool,
    pub cooldown_remaining: f32,
    pub cooldown_total: f32,
    pub disabled_reason: Option<String>,
}

impl Default for CommandState {
    fn default() -> Self {
        Self {
            enabled: true,
            cooldown_remaining: 0.0,
            cooldown_total: 0.0,
            disabled_reason: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContainerUiTemplate {
    pub id: String,
    pub mode: String,
    pub columns: usize,
    pub rows: Option<usize>,
    pub slot_size: i32,
    pub gap: i32,
    pub padding: i32,
    pub title: bool,
    pub background_color: [u8; 4],
    pub border_color: [u8; 4],
    pub slot_color: [u8; 4],
    pub slot_border_color: [u8; 4],
    pub tiles: ContainerUiTiles,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ContainerUiTiles {
    pub top_left: Option<String>,
    pub top: Option<String>,
    pub top_right: Option<String>,
    pub left: Option<String>,
    pub center: Option<String>,
    pub right: Option<String>,
    pub bottom_left: Option<String>,
    pub bottom: Option<String>,
    pub bottom_right: Option<String>,
    pub slot: Option<String>,
}

impl Default for ContainerUiTemplate {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            mode: "procedural".to_string(),
            columns: 4,
            rows: None,
            slot_size: 32,
            gap: 4,
            padding: 8,
            title: true,
            background_color: [10, 12, 15, 230],
            border_color: [98, 105, 116, 255],
            slot_color: [16, 21, 27, 204],
            slot_border_color: [98, 105, 116, 255],
            tiles: ContainerUiTiles::default(),
        }
    }
}

pub fn cooldown_attr_suffix(namespace: &str, id: &str) -> String {
    let mut suffix = namespace.trim().to_ascii_lowercase();
    suffix.push('_');
    for ch in id.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            suffix.push(ch.to_ascii_lowercase());
        } else {
            suffix.push('_');
        }
    }
    while suffix.contains("__") {
        suffix = suffix.replace("__", "_");
    }
    suffix.trim_matches('_').to_string()
}

pub fn cooldown_left_attr(namespace: &str, id: &str) -> String {
    format!("cooldown_left_{}", cooldown_attr_suffix(namespace, id))
}

pub fn cooldown_total_attr(namespace: &str, id: &str) -> String {
    format!("cooldown_total_{}", cooldown_attr_suffix(namespace, id))
}

pub fn describe_item(item: &Item) -> RulesDescription {
    let title = item
        .attributes
        .get_str("name")
        .map(str::to_string)
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| {
            if item.item_type.trim().is_empty() {
                "Item".to_string()
            } else {
                item.item_type.clone()
            }
        });

    let mut tags = Vec::new();
    if let Some(category) = item.attributes.get_str("category") {
        tags.push(title_case(category));
    }
    if let Some(slot) = item.attributes.get_str("slot") {
        tags.push(title_case(&slot.replace('_', " ")));
    }
    if let Some(rarity) = item.attributes.get_str("rarity") {
        tags.push(title_case(rarity));
    }

    let mut lines = Vec::new();
    if let Some(description) = item.attributes.get_str("description")
        && !description.trim().is_empty()
    {
        lines.push(description.trim().to_string());
    }
    if item.stack_quantity() > 1 || item.max_stack() > 1 {
        lines.push(format!(
            "Quantity: {} / {}",
            item.stack_quantity(),
            item.max_stack()
        ));
    }
    if let Some(dmg) = item.attributes.get_float("DMG")
        && dmg > 0.0
    {
        lines.push(format_number_line("Damage", dmg));
    }
    if let Some(armor) = item.attributes.get_float("ARMOR")
        && armor > 0.0
    {
        lines.push(format_number_line("Armor", armor));
    }
    if let Some(kind) = item.attributes.get_str("damage_kind") {
        lines.push(format!("Damage kind: {}", title_case(kind)));
    }
    if let Some(cooldown) = item.attributes.get_float("attack_cooldown")
        && cooldown > 0.0
    {
        lines.push(format!("Cooldown: {:.1}s", cooldown));
    }
    if let Some(ammunition) = item.attributes.get_str("ammunition") {
        lines.push(format!(
            "Uses: {}",
            title_case(&ammunition.replace('_', " "))
        ));
    }
    let container_slots = item.attributes.get_int_default("container_slots", 0).max(0) as usize;
    if item.is_container()
        || item.attributes.get_bool_default("container", false)
        || container_slots > 0
    {
        let used = item.container.as_ref().map(Vec::len).unwrap_or(0);
        let slots = container_slots.max(item.max_capacity as usize).max(used);
        lines.push(format!("Container: {} / {} slots", used, slots.max(1)));
    }

    RulesDescription {
        title,
        subtitle: (!tags.is_empty()).then(|| tags.join(", ")),
        lines,
    }
}

pub fn container_template_for_item(assets: &Assets, item: &Item) -> ContainerUiTemplate {
    let template_id = item
        .attributes
        .get_str("container_template")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("default");
    let Ok(root) = assets.rules.parse::<Table>() else {
        return ContainerUiTemplate::default();
    };

    let default = table_at(&root, &["ui", "container_templates", "default"])
        .map(|table| parse_container_template("default", table, ContainerUiTemplate::default()))
        .unwrap_or_default();
    if template_id == "default" {
        return default;
    }

    table_at(&root, &["ui", "container_templates", template_id])
        .map(|table| parse_container_template(template_id, table, default.clone()))
        .unwrap_or(default)
}

pub fn describe_command(
    assets: &Assets,
    actor: Option<&Entity>,
    command: &str,
) -> RulesDescription {
    let Some(binding) = parse_client_command(command) else {
        return RulesDescription {
            title: command.to_string(),
            subtitle: Some("Command".to_string()),
            lines: Vec::new(),
        };
    };

    match binding {
        ClientCommandBinding::Control(action) => RulesDescription {
            title: title_case(&format!("{:?}", action)),
            subtitle: Some("Control".to_string()),
            lines: Vec::new(),
        },
        ClientCommandBinding::Intent(intent) => {
            let title = if intent.trim().is_empty() {
                "Walk".to_string()
            } else {
                title_case(&intent.replace(['_', ':'], " "))
            };
            RulesDescription {
                title,
                subtitle: None,
                lines: Vec::new(),
            }
        }
        ClientCommandBinding::Ui(command) => RulesDescription {
            title: title_case(&command.replace('_', " ")),
            subtitle: Some("Interface".to_string()),
            lines: Vec::new(),
        },
        ClientCommandBinding::RulesAction(action_id) => {
            let Ok(root) = assets.rules.parse::<Table>() else {
                return fallback_rules_action_description(&action_id);
            };
            let Some(action) = table_at(&root, &["actions", &action_id]) else {
                return fallback_rules_action_description(&action_id);
            };
            describe_rules_action(&root, &action_id, action, actor)
        }
    }
}

pub fn command_state(assets: &Assets, actor: Option<&Entity>, command: &str) -> CommandState {
    let Some(actor) = actor else {
        return CommandState::default();
    };
    let Some(binding) = parse_client_command(command) else {
        return CommandState::default();
    };

    match binding {
        ClientCommandBinding::RulesAction(action_id) => {
            rules_action_state(assets, actor, &action_id)
        }
        ClientCommandBinding::Intent(intent) => {
            let mut state = CommandState::default();
            if !intent.trim().is_empty() {
                apply_cooldown_from_actor(actor, "intent", &intent, &mut state);
            }
            state
        }
        _ => CommandState::default(),
    }
}

fn rules_action_state(assets: &Assets, actor: &Entity, action_id: &str) -> CommandState {
    let mut state = CommandState::default();
    apply_cooldown_from_actor(actor, "rules", action_id, &mut state);

    let Ok(root) = assets.rules.parse::<Table>() else {
        return state;
    };
    let Some(action) = table_at(&root, &["actions", action_id]) else {
        state.enabled = false;
        state.disabled_reason = Some("Unknown action".to_string());
        return state;
    };

    if let Some(requires) = action.get("requires").and_then(toml::Value::as_table) {
        if let Some(ability) = requires
            .get("ability")
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            && !entity_has_list_value(actor, "abilities", ability)
        {
            state.enabled = false;
            state.disabled_reason = Some("Ability not known".to_string());
        }
        if let Some(spell) = requires
            .get("spell")
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            apply_cooldown_from_actor(actor, "spell", spell, &mut state);
            if !entity_has_list_value(actor, "spells", spell) {
                state.enabled = false;
                state.disabled_reason = Some("Spell not known".to_string());
            }
        }
    }

    if let Some(cost) = action.get("cost").and_then(toml::Value::as_table) {
        for (key, value) in cost {
            let required = value_number(value).round().max(0.0) as i32;
            if required <= 0 {
                continue;
            }
            let current = actor.attributes.get_int_default(key, 0);
            if current < required {
                state.enabled = false;
                state.disabled_reason = Some(format!("Need {} {}", required, key));
                break;
            }
        }
    }

    if let Some(consumes) = action.get("consumes").and_then(toml::Value::as_array) {
        for entry in consumes.iter().filter_map(toml::Value::as_table) {
            let Some(item_id) = entry
                .get("item")
                .and_then(toml::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            let quantity = entry
                .get("quantity")
                .map(value_number)
                .unwrap_or(1.0)
                .round()
                .max(1.0) as i32;
            if inventory_item_quantity(actor, item_id) < quantity {
                state.enabled = false;
                state.disabled_reason = Some(format!(
                    "Need {} {}",
                    quantity,
                    title_case(&item_id.replace('_', " "))
                ));
                break;
            }
        }
    }

    state
}

fn describe_rules_action(
    root: &Table,
    action_id: &str,
    action: &Table,
    actor: Option<&Entity>,
) -> RulesDescription {
    let title = action
        .get("name")
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| title_case(&action_id.replace('_', " ")));

    let subtitle = action
        .get("kind")
        .and_then(toml::Value::as_str)
        .map(title_case)
        .or_else(|| Some("Rules Action".to_string()));

    let mut lines = Vec::new();
    if let Some(description) = action.get("description").and_then(toml::Value::as_str)
        && !description.trim().is_empty()
    {
        lines.push(description.trim().to_string());
    }
    if let Some(target) = action.get("target").and_then(toml::Value::as_str) {
        lines.push(format!("Target: {}", title_case(&target.replace('_', " "))));
    }
    if let Some(range) = action.get("range") {
        let text = if range.as_str() == Some("weapon") {
            actor
                .and_then(|entity| current_weapon_range(root, entity))
                .map(|range| format!("{:.1}", range))
                .unwrap_or_else(|| "weapon".to_string())
        } else {
            format_value(range)
        };
        lines.push(format!("Range: {}", text));
    }
    if let Some(cooldown) = action.get("cooldown").map(value_number)
        && cooldown > 0.0
    {
        lines.push(format!("Cooldown: {:.1}s", cooldown));
    }
    if let Some(cost) = action.get("cost").and_then(toml::Value::as_table) {
        let parts = cost
            .iter()
            .map(|(key, value)| format!("{} {}", format_value(value), key))
            .collect::<Vec<_>>();
        if !parts.is_empty() {
            lines.push(format!("Cost: {}", parts.join(", ")));
        }
    }
    if let Some(consumes) = action.get("consumes").and_then(toml::Value::as_array) {
        let parts = consumes
            .iter()
            .filter_map(toml::Value::as_table)
            .filter_map(|entry| {
                let item = entry.get("item")?.as_str()?;
                let quantity = entry.get("quantity").map(value_number).unwrap_or(1.0);
                Some(format!(
                    "{} {}",
                    format_clean_number(quantity),
                    title_case(&item.replace('_', " "))
                ))
            })
            .collect::<Vec<_>>();
        if !parts.is_empty() {
            lines.push(format!("Consumes: {}", parts.join(", ")));
        }
    }

    RulesDescription {
        title,
        subtitle,
        lines,
    }
}

fn apply_cooldown_from_actor(actor: &Entity, namespace: &str, id: &str, state: &mut CommandState) {
    let remaining = actor
        .attributes
        .get_float(&cooldown_left_attr(namespace, id))
        .unwrap_or(0.0)
        .max(0.0);
    if remaining <= 0.0 {
        return;
    }
    let total = actor
        .attributes
        .get_float(&cooldown_total_attr(namespace, id))
        .unwrap_or(remaining)
        .max(remaining);
    state.enabled = false;
    state.cooldown_remaining = state.cooldown_remaining.max(remaining);
    state.cooldown_total = state.cooldown_total.max(total);
}

fn fallback_rules_action_description(action_id: &str) -> RulesDescription {
    RulesDescription {
        title: title_case(&action_id.replace('_', " ")),
        subtitle: Some("Rules Action".to_string()),
        lines: Vec::new(),
    }
}

fn table_at<'a>(root: &'a Table, path: &[&str]) -> Option<&'a Table> {
    let mut value = root.get(*path.first()?)?;
    for part in &path[1..] {
        value = value.as_table()?.get(*part)?;
    }
    value.as_table()
}

fn parse_container_template(
    id: &str,
    table: &Table,
    mut template: ContainerUiTemplate,
) -> ContainerUiTemplate {
    template.id = id.to_string();
    if let Some(mode) = table.get("mode").and_then(toml::Value::as_str) {
        template.mode = mode.trim().to_string();
    }
    if let Some(columns) = table.get("columns").map(value_number) {
        template.columns = (columns.round() as i32).max(1) as usize;
    }
    template.rows = table
        .get("rows")
        .map(value_number)
        .map(|value| (value.round() as i32).max(1) as usize)
        .or(template.rows);
    if let Some(slot_size) = table.get("slot_size").map(value_number) {
        template.slot_size = (slot_size.round() as i32).max(8);
    }
    if let Some(gap) = table.get("gap").map(value_number) {
        template.gap = (gap.round() as i32).max(0);
    }
    if let Some(padding) = table.get("padding").map(value_number) {
        template.padding = (padding.round() as i32).max(0);
    }
    if let Some(title) = table.get("title").and_then(toml::Value::as_bool) {
        template.title = title;
    }
    if let Some(color) = table
        .get("background_color")
        .and_then(toml::Value::as_str)
        .and_then(parse_hex_rgba)
    {
        template.background_color = color;
    }
    if let Some(color) = table
        .get("border_color")
        .and_then(toml::Value::as_str)
        .and_then(parse_hex_rgba)
    {
        template.border_color = color;
    }
    if let Some(color) = table
        .get("slot_color")
        .and_then(toml::Value::as_str)
        .and_then(parse_hex_rgba)
    {
        template.slot_color = color;
    }
    if let Some(color) = table
        .get("slot_border_color")
        .and_then(toml::Value::as_str)
        .and_then(parse_hex_rgba)
    {
        template.slot_border_color = color;
    }
    if let Some(tiles) = table.get("tiles").and_then(toml::Value::as_table) {
        template.tiles = parse_container_tiles(tiles, template.tiles);
    }
    template
}

fn parse_container_tiles(table: &Table, mut tiles: ContainerUiTiles) -> ContainerUiTiles {
    for (key, target) in [
        ("top_left", &mut tiles.top_left),
        ("top", &mut tiles.top),
        ("top_right", &mut tiles.top_right),
        ("left", &mut tiles.left),
        ("center", &mut tiles.center),
        ("right", &mut tiles.right),
        ("bottom_left", &mut tiles.bottom_left),
        ("bottom", &mut tiles.bottom),
        ("bottom_right", &mut tiles.bottom_right),
        ("slot", &mut tiles.slot),
    ] {
        if let Some(value) = table
            .get(key)
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            *target = Some(value.to_string());
        }
    }
    tiles
}

fn parse_hex_rgba(value: &str) -> Option<[u8; 4]> {
    let value = value.trim().trim_start_matches('#');
    if value.len() != 6 && value.len() != 8 {
        return None;
    }
    let r = u8::from_str_radix(&value[0..2], 16).ok()?;
    let g = u8::from_str_radix(&value[2..4], 16).ok()?;
    let b = u8::from_str_radix(&value[4..6], 16).ok()?;
    let a = if value.len() == 8 {
        u8::from_str_radix(&value[6..8], 16).ok()?
    } else {
        255
    };
    Some([r, g, b, a])
}

fn value_number(value: &toml::Value) -> f32 {
    value
        .as_float()
        .or_else(|| value.as_integer().map(|value| value as f64))
        .unwrap_or(0.0) as f32
}

fn format_value(value: &toml::Value) -> String {
    if let Some(text) = value.as_str() {
        text.to_string()
    } else {
        format_clean_number(value_number(value))
    }
}

fn format_clean_number(value: f32) -> String {
    if (value - value.round()).abs() < f32::EPSILON {
        format!("{}", value.round() as i32)
    } else {
        format!("{:.1}", value)
    }
}

fn format_number_line(label: &str, value: f32) -> String {
    format!("{}: {}", label, format_clean_number(value))
}

fn title_case(value: &str) -> String {
    value
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    first.to_uppercase().collect::<String>()
                        + chars.as_str().to_ascii_lowercase().as_str()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn entity_has_list_value(entity: &Entity, key: &str, needle: &str) -> bool {
    match entity.attributes.get(key) {
        Some(Value::StrArray(values)) => values.iter().any(|value| value.trim() == needle),
        Some(Value::Str(value)) => value.split(',').map(str::trim).any(|value| value == needle),
        _ => false,
    }
}

fn inventory_item_quantity(entity: &Entity, ruleset_id: &str) -> i32 {
    entity
        .iter_inventory()
        .filter(|(_, item)| ruleset_item_matches_id(item, ruleset_id))
        .map(|(_, item)| item.stack_quantity().max(1))
        .sum()
}

fn ruleset_item_matches_id(item: &Item, ruleset_id: &str) -> bool {
    item.attributes
        .get_str("ruleset_id")
        .or_else(|| item.attributes.get_str("class_name"))
        .or_else(|| item.attributes.get_str("name"))
        .map(str::trim)
        .is_some_and(|value| value.eq_ignore_ascii_case(ruleset_id))
}

fn current_weapon_range(root: &Table, entity: &Entity) -> Option<f32> {
    let weapon = entity
        .equipped
        .iter()
        .find(|(slot, _)| {
            matches!(
                slot.trim().to_ascii_lowercase().as_str(),
                "main_hand" | "mainhand" | "weapon" | "hand_main" | "off_hand" | "offhand"
            )
        })
        .map(|(_, item)| item)?;

    let own_range = weapon
        .attributes
        .get_float("range")
        .filter(|value| *value > 0.0);
    if own_range.is_some() {
        return own_range;
    }

    let category = weapon.attributes.get_str("category")?.trim();
    root.get("equipment")
        .and_then(toml::Value::as_table)
        .and_then(|equipment| equipment.get("weapon_categories"))
        .and_then(toml::Value::as_table)
        .and_then(|categories| categories.get(category))
        .and_then(toml::Value::as_table)
        .and_then(|category| category.get("range"))
        .map(value_number)
        .filter(|value| *value > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Entity, Value};

    #[test]
    fn rules_action_state_reads_spell_cooldown_attrs() {
        let mut assets = Assets::new();
        assets.rules = r#"
            [actions.minor_heal]
            name = "Minor Heal"
            kind = "spell"
            requires = { spell = "minor_heal" }
            cooldown = 4.0

            [spells.minor_heal]
            name = "Minor Heal"
        "#
        .to_string();

        let mut actor = Entity::new();
        actor.set_attribute("spells", Value::StrArray(vec!["minor_heal".to_string()]));
        actor.set_attribute("cooldown_left_spell_minor_heal", Value::Float(1.5));
        actor.set_attribute("cooldown_total_spell_minor_heal", Value::Float(4.0));

        let state = command_state(&assets, Some(&actor), "rules.minor_heal");

        assert!(!state.enabled);
        assert_eq!(state.cooldown_remaining, 1.5);
        assert_eq!(state.cooldown_total, 4.0);
    }

    #[test]
    fn item_description_uses_ruleset_item_attrs() {
        let mut item = Item::new();
        item.item_type = "Fallback".to_string();
        item.set_attribute("name", Value::Str("Wooden Arrows".to_string()));
        item.set_attribute("category", Value::Str("arrow".to_string()));
        item.set_attribute("quantity", Value::Int(12));
        item.set_attribute("max_stack", Value::Int(99));

        let description = describe_item(&item);

        assert_eq!(description.title, "Wooden Arrows");
        assert!(
            description
                .lines
                .iter()
                .any(|line| line == "Quantity: 12 / 99")
        );
    }

    #[test]
    fn container_description_and_template_use_rules() {
        let mut assets = Assets::new();
        assets.rules = r##"
            [ui.container_templates.default]
            columns = 4
            slot_size = 32
            background_color = "#0a0c0fe6"

            [ui.container_templates.bag_small]
            columns = 3
            rows = 2
            slot_size = 40
            gap = 6
            padding = 10
            title = false
            slot_color = "#112233cc"

            [ui.container_templates.bag_small.tiles]
            top_left = "bag_tl"
            slot = "bag_slot"
        "##
        .to_string();

        let mut bag = Item::new();
        bag.set_attribute("name", Value::Str("Small Bag".to_string()));
        bag.set_attribute("container", Value::Bool(true));
        bag.set_attribute("container_slots", Value::Int(6));
        bag.set_attribute("container_template", Value::Str("bag_small".to_string()));
        bag.apply_container_attributes();

        let description = describe_item(&bag);
        assert!(
            description
                .lines
                .iter()
                .any(|line| line == "Container: 0 / 6 slots")
        );

        let template = container_template_for_item(&assets, &bag);
        assert_eq!(template.id, "bag_small");
        assert_eq!(template.columns, 3);
        assert_eq!(template.rows, Some(2));
        assert_eq!(template.slot_size, 40);
        assert_eq!(template.gap, 6);
        assert_eq!(template.padding, 10);
        assert!(!template.title);
        assert_eq!(template.slot_color, [17, 34, 51, 204]);
        assert_eq!(template.tiles.top_left.as_deref(), Some("bag_tl"));
        assert_eq!(template.tiles.slot.as_deref(), Some("bag_slot"));
    }
}
