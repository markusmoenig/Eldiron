use crate::{
    Assets, Entity, Item, Value,
    client::command::{ClientCommandBinding, parse_client_command},
};
use eldiron_ruleset::{
    ResolvedAction, ResolvedActionAttributePredicate, ResolvedActionEffect,
    ResolvedActionEffectRecipient, ResolvedActionEffectValue, ResolvedActionKind,
    ResolvedActionModification, ResolvedActionModificationField, ResolvedActionPredicateValue,
    ResolvedActionRange, ResolvedActionRequirement, ResolvedCondition,
    ResolvedConditionPeriodicEffect, ResolvedDerivedStat, evaluate_formula,
    resolve_attribute_roles, resolve_condition, resolve_conditions, resolve_derived_stats,
};
use std::collections::{BTreeMap, BTreeSet};
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

pub fn describe_item(item: &Item, assets: &Assets) -> RulesDescription {
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
    if let Some(quality) = item_percent_attr(item, "quality") {
        lines.push(format!("Quality: {}%", quality));
    }
    if let Some(condition) = item_percent_attr(item, "condition") {
        lines.push(format!("Condition: {}%", condition));
    }
    if let Some(damage) = item_damage_line(item) {
        lines.push(damage);
    } else if let Some(dmg) = assets
        .ruleset_attribute_role("weapon_damage")
        .and_then(|attribute| item.attributes.get_float(&attribute))
        && dmg > 0.0
    {
        lines.push(format_number_line("Damage", dmg));
    }
    if let Some(armor) = assets
        .ruleset_attribute_role("armor")
        .and_then(|attribute| item.attributes.get_float(&attribute))
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

fn item_percent_attr(item: &Item, key: &str) -> Option<i32> {
    item.attributes
        .get_float(key)
        .map(|value| value.round().clamp(0.0, 100.0) as i32)
}

fn item_damage_line(item: &Item) -> Option<String> {
    let roll = item
        .attributes
        .get_str("damage_roll")
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let mut parts = vec![roll.to_string()];
    if let Some(bonus) = item.attributes.get_float("damage_bonus")
        && bonus.abs() > f32::EPSILON
    {
        let sign = if bonus < 0.0 { "-" } else { "+" };
        parts.push(format!("{} {}", sign, format_clean_number(bonus.abs())));
    }
    if let Some(attribute) = item
        .attributes
        .get_str("damage_bonus_attribute")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let every = item
            .attributes
            .get_float("damage_bonus_every")
            .unwrap_or(1.0)
            .max(1.0);
        parts.push(format!("+ {}/{}", attribute, format_clean_number(every)));
    }
    Some(format!("Damage: {}", parts.join(" ")))
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
        ClientCommandBinding::Screen(command) => RulesDescription {
            title: title_case(&command.replace(['_', '.'], " ")),
            subtitle: Some("Screen".to_string()),
            lines: Vec::new(),
        },
        ClientCommandBinding::Game(command) => RulesDescription {
            title: title_case(&command.replace(['_', '.'], " ")),
            subtitle: Some("Game".to_string()),
            lines: Vec::new(),
        },
        ClientCommandBinding::RulesAction(action_id) => {
            let Ok(root) = assets.rules.parse::<Table>() else {
                return fallback_rules_action_description(&action_id);
            };
            let Ok(Some(action)) = eldiron_ruleset::resolve_action(&root, &action_id) else {
                return fallback_rules_action_description(&action_id);
            };
            describe_rules_action(&root, &action, actor)
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

fn action_attribute_predicate_matches(
    value: Option<&Value>,
    predicate: &ResolvedActionAttributePredicate,
) -> bool {
    let Some(value) = value else {
        return false;
    };
    match value {
        Value::Bool(value) => predicate.matches_bool(*value),
        Value::Int(value) => predicate.matches_number(*value as f32),
        Value::UInt(value) => predicate.matches_number(*value as f32),
        Value::Int64(value) => predicate.matches_number(*value as f32),
        Value::Float(value) => predicate.matches_number(*value),
        Value::Str(value) => predicate.matches_string(value),
        Value::StrArray(values) => predicate.matches_strings(values),
        _ => false,
    }
}

fn condition_attribute_suffix(condition_id: &str) -> String {
    let mut suffix = "condition_".to_string();
    for ch in condition_id.trim().chars() {
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

fn effective_rules_attribute(root: &Table, entity: &Entity, attribute: &str) -> f32 {
    let conditions = resolve_conditions(root).unwrap_or_default();
    let derived_stats = resolve_derived_stats(root).unwrap_or_default();
    let level_attribute = resolve_attribute_roles(root)
        .ok()
        .and_then(|roles| roles.get("level").map(str::to_string));
    effective_rules_attribute_inner(
        entity,
        attribute,
        &conditions,
        &derived_stats,
        level_attribute.as_deref(),
        &mut BTreeSet::new(),
    )
}

fn effective_rules_attribute_inner(
    entity: &Entity,
    attribute: &str,
    conditions: &BTreeMap<String, ResolvedCondition>,
    derived_stats: &BTreeMap<String, ResolvedDerivedStat>,
    level_attribute: Option<&str>,
    visiting: &mut BTreeSet<String>,
) -> f32 {
    let raw = entity.attributes.get_float_default(attribute, 0.0);
    let visit_key = attribute.to_ascii_lowercase();
    if !visiting.insert(visit_key.clone()) {
        return raw;
    }
    let derived = derived_stats.get(attribute).or_else(|| {
        derived_stats
            .values()
            .find(|stat| stat.id.eq_ignore_ascii_case(attribute))
    });
    let mut resolved = derived
        .and_then(|stat| {
            evaluate_formula(&stat.formula, |name| match name {
                "base" => raw,
                "level" => level_attribute
                    .map(|attribute| entity.attributes.get_float_default(attribute, 1.0))
                    .unwrap_or(1.0),
                dependency => effective_rules_attribute_inner(
                    entity,
                    dependency,
                    conditions,
                    derived_stats,
                    level_attribute,
                    visiting,
                ),
            })
            .map(|mut value| {
                if let Some(minimum) = stat.minimum {
                    value = value.max(minimum);
                }
                if let Some(maximum) = stat.maximum {
                    value = value.min(maximum);
                }
                value
            })
        })
        .unwrap_or(raw);
    visiting.remove(&visit_key);
    resolved = apply_rules_condition_modifiers(entity, attribute, conditions, resolved);
    resolved
}

fn apply_rules_condition_modifiers(
    entity: &Entity,
    attribute: &str,
    conditions: &BTreeMap<String, ResolvedCondition>,
    base: f32,
) -> f32 {
    let base = base as f64;
    let active = match entity.attributes.get("conditions") {
        Some(Value::StrArray(ids)) => ids.clone(),
        Some(Value::Str(ids)) => ids
            .split(',')
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    };
    let mut add = 0.0f64;
    let mut multiply = 1.0f64;
    let mut minimum: Option<f64> = None;
    let mut maximum: Option<f64> = None;
    for condition_id in active {
        let Some(condition) = conditions.get(&condition_id) else {
            continue;
        };
        let suffix = condition_attribute_suffix(&condition_id);
        let stacks = entity
            .attributes
            .get_float_default(&format!("{}_stacks", suffix), 1.0)
            .round()
            .max(1.0) as usize;
        for modifier in condition
            .modifiers
            .iter()
            .filter(|modifier| modifier.attribute.eq_ignore_ascii_case(attribute))
        {
            add += modifier.add as f64 * stacks as f64;
            multiply *= (modifier.multiply as f64).powi(stacks.min(i32::MAX as usize) as i32);
            if let Some(value) = modifier.minimum {
                minimum = Some(minimum.map_or(value as f64, |current| current.max(value as f64)));
            }
            if let Some(value) = modifier.maximum {
                maximum = Some(maximum.map_or(value as f64, |current| current.min(value as f64)));
            }
        }
    }
    let mut value = (base + add) * multiply;
    if let Some(minimum) = minimum {
        value = value.max(minimum);
    }
    if let Some(maximum) = maximum {
        value = value.min(maximum);
    }
    value.clamp(f32::MIN as f64, f32::MAX as f64) as f32
}

fn action_entity_attribute_predicate_matches(
    root: &Table,
    entity: &Entity,
    id: &str,
    predicate: &ResolvedActionAttributePredicate,
) -> bool {
    if matches!(
        predicate,
        ResolvedActionAttributePredicate::AtLeast(_)
            | ResolvedActionAttributePredicate::AtMost(_)
            | ResolvedActionAttributePredicate::Equals(ResolvedActionPredicateValue::Number(_))
            | ResolvedActionAttributePredicate::NotEquals(ResolvedActionPredicateValue::Number(_))
    ) {
        return predicate.matches_number(effective_rules_attribute(root, entity, id));
    }
    action_attribute_predicate_matches(entity.attributes.get(id), predicate)
}

fn action_predicate_value_label(value: &ResolvedActionPredicateValue) -> String {
    match value {
        ResolvedActionPredicateValue::Bool(value) => value.to_string(),
        ResolvedActionPredicateValue::Number(value) => format_clean_number(*value),
        ResolvedActionPredicateValue::String(value) => value.clone(),
    }
}

fn action_attribute_requirement_reason(
    id: &str,
    predicate: &ResolvedActionAttributePredicate,
) -> String {
    let name = title_case(&id.replace('_', " "));
    match predicate {
        ResolvedActionAttributePredicate::Equals(value) => {
            format!("Need {} = {}", name, action_predicate_value_label(value))
        }
        ResolvedActionAttributePredicate::NotEquals(value) => {
            format!("Need {} != {}", name, action_predicate_value_label(value))
        }
        ResolvedActionAttributePredicate::AtLeast(value) => {
            format!("Need {} at least {}", name, format_clean_number(*value))
        }
        ResolvedActionAttributePredicate::AtMost(value) => {
            format!("Need {} at most {}", name, format_clean_number(*value))
        }
        ResolvedActionAttributePredicate::Contains(value) => {
            format!("Need {} containing {}", name, value)
        }
        ResolvedActionAttributePredicate::NotContains(value) => {
            format!("Need {} not containing {}", name, value)
        }
    }
}

fn rules_action_state(assets: &Assets, actor: &Entity, action_id: &str) -> CommandState {
    let mut state = CommandState::default();
    apply_cooldown_from_actor(actor, "rules", action_id, &mut state);

    let Ok(root) = assets.rules.parse::<Table>() else {
        return state;
    };
    let Ok(Some(action)) = eldiron_ruleset::resolve_action(&root, action_id) else {
        state.enabled = false;
        state.disabled_reason = Some("Unknown action".to_string());
        return state;
    };

    for requirement in &action.requirements {
        match requirement {
            ResolvedActionRequirement::Ability(ability) => {
                if !entity_has_list_value(actor, "abilities", ability) {
                    state.enabled = false;
                    state.disabled_reason = Some(
                        future_class_unlock_reason(&root, actor, "abilities", ability)
                            .unwrap_or_else(|| "Ability not learned".to_string()),
                    );
                }
            }
            ResolvedActionRequirement::Spell(spell) => {
                apply_cooldown_from_actor(actor, "spell", spell, &mut state);
                if !entity_has_list_value(actor, "spells", spell) {
                    state.enabled = false;
                    state.disabled_reason = Some(
                        future_class_unlock_reason(&root, actor, "spells", spell)
                            .unwrap_or_else(|| "Spell not learned".to_string()),
                    );
                }
            }
            ResolvedActionRequirement::Skill { id, minimum } => {
                if actor.attributes.get_float_default(id, 0.0) < *minimum as f32 {
                    state.enabled = false;
                    state.disabled_reason = Some(format!(
                        "Need {} {}",
                        minimum,
                        title_case(&id.replace('_', " "))
                    ));
                }
            }
            ResolvedActionRequirement::Profession(_) => {}
            ResolvedActionRequirement::Attribute { id, predicate } => {
                if !action_entity_attribute_predicate_matches(&root, actor, id, predicate) {
                    state.enabled = false;
                    state.disabled_reason =
                        Some(action_attribute_requirement_reason(id, predicate));
                }
            }
            ResolvedActionRequirement::TargetAttribute { .. } => {}
        }
    }

    for cost in &action.resource_costs {
        if cost.amount <= 0.0 {
            continue;
        }
        let current = actor.attributes.get_float_default(&cost.resource, 0.0);
        if current < cost.amount {
            state.enabled = false;
            state.disabled_reason = Some(format!(
                "Need {} {}",
                format_clean_number(cost.amount),
                cost.resource
            ));
            break;
        }
    }

    for cost in &action.item_costs {
        if inventory_item_quantity(actor, &cost.item) < cost.quantity as i32 {
            state.enabled = false;
            state.disabled_reason = Some(format!(
                "Need {} {}",
                cost.quantity,
                title_case(&cost.item.replace('_', " "))
            ));
            break;
        }
    }

    state
}

fn describe_rules_action(
    root: &Table,
    action: &ResolvedAction,
    actor: Option<&Entity>,
) -> RulesDescription {
    let subtitle = match &action.kind {
        ResolvedActionKind::Custom(kind) => title_case(kind),
        kind => title_case(kind.id()),
    };

    let mut lines = Vec::new();
    if let Some(description) = action.description.as_deref()
        && !description.trim().is_empty()
    {
        lines.push(description.trim().to_string());
    }
    lines.push(format!(
        "Target: {}",
        title_case(&action.target.id().replace('_', " "))
    ));
    for requirement in &action.requirements {
        if let ResolvedActionRequirement::TargetAttribute { id, predicate } = requirement {
            let reason = action_attribute_requirement_reason(id, predicate);
            lines.push(format!(
                "Requires Target: {}",
                reason.strip_prefix("Need ").unwrap_or(&reason)
            ));
        }
    }
    match &action.range {
        ResolvedActionRange::Default => {}
        ResolvedActionRange::Fixed(range) => {
            lines.push(format!("Range: {}", format_clean_number(*range)));
        }
        ResolvedActionRange::Weapon { .. } => {
            let range = actor
                .and_then(|entity| current_weapon_range(root, entity))
                .map(format_clean_number)
                .unwrap_or_else(|| "weapon".to_string());
            lines.push(format!("Range: {}", range));
        }
        ResolvedActionRange::Source { source, fallback } => {
            lines.push(format!(
                "Range: {} ({})",
                format_clean_number(*fallback),
                title_case(&source.replace('_', " "))
            ));
        }
    }
    if action.cooldown_seconds > 0.0 {
        lines.push(format!("Cooldown: {:.1}s", action.cooldown_seconds));
    }
    if !action.resource_costs.is_empty() {
        let parts = action
            .resource_costs
            .iter()
            .map(|cost| format!("{} {}", format_clean_number(cost.amount), cost.resource))
            .collect::<Vec<_>>();
        lines.push(format!("Cost: {}", parts.join(", ")));
    }
    if !action.item_costs.is_empty() {
        let parts = action
            .item_costs
            .iter()
            .map(|cost| {
                format!(
                    "{} {}",
                    cost.quantity,
                    title_case(&cost.item.replace('_', " "))
                )
            })
            .collect::<Vec<_>>();
        lines.push(format!("Consumes: {}", parts.join(", ")));
    }
    for effect in &action.effects {
        match effect {
            ResolvedActionEffect::ApplyCondition { condition, .. } => {
                if let Ok(Some(condition)) = resolve_condition(root, condition) {
                    let duration = if condition.duration_seconds > 0.0 {
                        format!(" for {:.1}s", condition.duration_seconds)
                    } else {
                        String::new()
                    };
                    let modifiers = condition
                        .modifiers
                        .iter()
                        .map(|modifier| {
                            let attribute = title_case(&modifier.attribute.replace('_', " "));
                            if modifier.multiply == 1.0
                                && modifier.minimum.is_none()
                                && modifier.maximum.is_none()
                            {
                                let value = format_clean_number(modifier.add);
                                return format!(
                                    "{}{} {}",
                                    if modifier.add >= 0.0 { "+" } else { "" },
                                    value,
                                    attribute
                                );
                            }
                            let mut operations = Vec::new();
                            if modifier.add != 0.0 {
                                operations.push(format!(
                                    "{}{}",
                                    if modifier.add >= 0.0 { "+" } else { "" },
                                    format_clean_number(modifier.add)
                                ));
                            }
                            if modifier.multiply != 1.0 {
                                operations
                                    .push(format!("×{}", format_clean_number(modifier.multiply)));
                            }
                            if let Some(minimum) = modifier.minimum {
                                operations.push(format!("min {}", format_clean_number(minimum)));
                            }
                            if let Some(maximum) = modifier.maximum {
                                operations.push(format!("max {}", format_clean_number(maximum)));
                            }
                            format!("{}: {}", attribute, operations.join(", "))
                        })
                        .collect::<Vec<_>>();
                    let modifier = if modifiers.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", modifiers.join(", "))
                    };
                    lines.push(format!(
                        "Applies: {}{}{}",
                        condition.name, duration, modifier
                    ));
                    if let Some(periodic) = &condition.periodic {
                        let effects = periodic
                            .effects
                            .iter()
                            .map(|effect| match effect {
                                ResolvedConditionPeriodicEffect::Damage {
                                    amount,
                                    damage_kind,
                                } => format!(
                                    "{} {} damage",
                                    format_clean_number(*amount),
                                    title_case(&damage_kind.replace('_', " "))
                                ),
                                ResolvedConditionPeriodicEffect::Healing { amount } => {
                                    format!("{} healing", format_clean_number(*amount))
                                }
                                ResolvedConditionPeriodicEffect::Modify {
                                    field,
                                    add,
                                    minimum,
                                    maximum,
                                } => {
                                    let mut effect = format!(
                                        "{}{} {}",
                                        if *add >= 0.0 { "+" } else { "" },
                                        format_clean_number(*add),
                                        title_case(&field.id().replace('_', " "))
                                    );
                                    if let Some(minimum) = minimum {
                                        effect.push_str(&format!(
                                            " (min {})",
                                            format_clean_number(*minimum)
                                        ));
                                    }
                                    if let Some(maximum) = maximum {
                                        effect.push_str(&format!(
                                            " (max {})",
                                            format_clean_number(*maximum)
                                        ));
                                    }
                                    effect
                                }
                            })
                            .collect::<Vec<_>>();
                        lines.push(format!(
                            "Every {:.1}s per stack: {}",
                            periodic.interval_seconds,
                            effects.join(", ")
                        ));
                    }
                } else {
                    lines.push(format!(
                        "Applies: {}",
                        title_case(&condition.replace('_', " "))
                    ));
                }
            }
            ResolvedActionEffect::RemoveCondition { condition, .. } => {
                let name = resolve_condition(root, condition)
                    .ok()
                    .flatten()
                    .map(|condition| condition.name)
                    .unwrap_or_else(|| title_case(&condition.replace('_', " ")));
                lines.push(format!("Removes: {}", name));
            }
            _ => {}
        }
    }
    for effect in &action.effects {
        let ResolvedActionEffect::Modify {
            recipient,
            field,
            operation,
            minimum,
            maximum,
            maximum_attribute,
        } = effect
        else {
            continue;
        };
        let recipient = match recipient {
            ResolvedActionEffectRecipient::Actor => "Actor",
            ResolvedActionEffectRecipient::Target => "Target",
        };
        let field = match field {
            ResolvedActionModificationField::Attribute(id)
            | ResolvedActionModificationField::Resource(id) => title_case(&id.replace('_', " ")),
        };
        let operation = match operation {
            ResolvedActionModification::Add(value) => {
                let value = format_clean_number(*value);
                if value.starts_with('-') {
                    value
                } else {
                    format!("+{}", value)
                }
            }
            ResolvedActionModification::Set(ResolvedActionEffectValue::Bool(value)) => {
                format!("= {}", value)
            }
            ResolvedActionModification::Set(ResolvedActionEffectValue::Integer(value)) => {
                format!("= {}", value)
            }
            ResolvedActionModification::Set(ResolvedActionEffectValue::Float(value)) => {
                format!("= {}", format_clean_number(*value))
            }
            ResolvedActionModification::Set(ResolvedActionEffectValue::String(value)) => {
                format!("= {}", value)
            }
        };
        let mut clamps = Vec::new();
        if let Some(minimum) = minimum {
            clamps.push(format!("min {}", format_clean_number(*minimum)));
        }
        if let Some(maximum) = maximum {
            clamps.push(format!("max {}", format_clean_number(*maximum)));
        }
        if let Some(maximum_attribute) = maximum_attribute {
            clamps.push(format!(
                "max {}",
                title_case(&maximum_attribute.replace('_', " "))
            ));
        }
        let clamp = if clamps.is_empty() {
            String::new()
        } else {
            format!(" ({})", clamps.join(", "))
        };
        lines.push(format!(
            "Effect: {} {} {}{}",
            recipient, field, operation, clamp
        ));
    }

    RulesDescription {
        title: action.name.clone(),
        subtitle: Some(subtitle),
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

fn future_class_unlock_reason(
    root: &Table,
    actor: &Entity,
    unlock_key: &str,
    id: &str,
) -> Option<String> {
    let class_id = actor
        .attributes
        .get_str("class")
        .or_else(|| actor.attributes.get_str("class_name"))?
        .trim();
    if class_id.is_empty() {
        return None;
    }

    let classes = root.get("classes")?.as_table()?;
    let class = classes
        .get(class_id)
        .or_else(|| {
            classes
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(class_id))
                .map(|(_, value)| value)
        })?
        .as_table()?;
    let unlocks = class.get("unlocks")?.as_table()?;
    let actor_level = resolve_attribute_roles(root)
        .ok()
        .and_then(|roles| roles.get("level").map(str::to_string))
        .map(|attribute| actor.attributes.get_int_default(&attribute, 1))
        .unwrap_or(1)
        .max(1) as u32;
    let mut levels: Vec<u32> = unlocks
        .iter()
        .filter_map(|(level_key, value)| {
            let level = level_key.strip_prefix("level_")?.parse::<u32>().ok()?;
            let values = value.as_table()?.get(unlock_key)?.as_array()?;
            values
                .iter()
                .filter_map(toml::Value::as_str)
                .any(|entry| entry.trim().eq_ignore_ascii_case(id))
                .then_some(level)
        })
        .filter(|level| *level > actor_level)
        .collect();
    levels.sort_unstable();
    levels
        .first()
        .map(|level| format!("Available at level {}", level))
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
    let policy = eldiron_ruleset::resolve_equipment_policy(root).ok()?;
    let weapon = policy.weapon_slots.iter().find_map(|slot| {
        entity
            .equipped
            .iter()
            .find(|(equipped_slot, _)| equipped_slot.eq_ignore_ascii_case(slot))
            .map(|(_, item)| item)
    })?;

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
    fn rules_action_state_reports_future_ability_unlock_level() {
        let mut assets = Assets::new();
        assets.rules = r#"
            [attributes]
            progression = ["RANK"]
            [attributes.roles]
            level = "RANK"

            [actions.power_strike]
            name = "Power Strike"
            requires = { ability = "power_strike" }

            [classes.Warrior.unlocks.level_1]
            abilities = ["basic_attack"]

            [classes.Warrior.unlocks.level_2]
            abilities = ["power_strike"]
        "#
        .to_string();

        let mut actor = Entity::new();
        actor.set_attribute("class", Value::Str("Warrior".to_string()));
        actor.set_attribute("RANK", Value::Int(1));
        actor.set_attribute(
            "abilities",
            Value::StrArray(vec!["basic_attack".to_string()]),
        );

        let state = command_state(&assets, Some(&actor), "rules.power_strike");

        assert!(!state.enabled);
        assert_eq!(
            state.disabled_reason.as_deref(),
            Some("Available at level 2")
        );
    }

    #[test]
    fn rules_action_state_reports_future_spell_unlock_level() {
        let mut assets = Assets::new();
        assets.rules = r#"
            [attributes]
            progression = ["RANK"]
            [attributes.roles]
            level = "RANK"

            [actions.holy_light]
            name = "Holy Light"
            requires = { spell = "holy_light" }

            [classes.Cleric.unlocks.level_1]
            spells = ["minor_heal"]

            [classes.Cleric.unlocks.level_2]
            spells = ["holy_light"]
        "#
        .to_string();

        let mut actor = Entity::new();
        actor.set_attribute("class", Value::Str("Cleric".to_string()));
        actor.set_attribute("RANK", Value::Int(1));
        actor.set_attribute("spells", Value::StrArray(vec!["minor_heal".to_string()]));

        let state = command_state(&assets, Some(&actor), "rules.holy_light");

        assert!(!state.enabled);
        assert_eq!(
            state.disabled_reason.as_deref(),
            Some("Available at level 2")
        );
    }

    #[test]
    fn rules_action_state_reports_custom_attribute_predicates() {
        let mut assets = Assets::new();
        assets.rules = r#"
            [conditions.inspired]
            modifiers = [{ attribute = "karma", add = 1 }]

            [derived_stats.karma]
            formula = "base + resolve"

            [actions.haunt]
            name = "Haunt"
            requires = { attributes = [
                { id = "traits", contains = "undead" },
                { id = "karma", at_least = 10 },
            ], target_attributes = [
                { id = "mode", not_equals = "warded" },
            ] }
            result = {
                script = "haunt",
                modify = [
                    { recipient = "actor", resource = "stamina", add = -2, minimum = 0 },
                ],
            }
        "#
        .to_string();

        let mut actor = Entity::new();
        actor.set_attribute("traits", Value::StrArray(vec!["undead".to_string()]));
        actor.set_attribute("karma", Value::Int(8));
        actor.set_attribute("resolve", Value::Int(1));

        let state = command_state(&assets, Some(&actor), "rules.haunt");
        assert!(!state.enabled);
        assert_eq!(
            state.disabled_reason.as_deref(),
            Some("Need Karma at least 10")
        );

        actor.set_attribute("conditions", Value::StrArray(vec!["inspired".to_string()]));
        actor.set_attribute("condition_inspired_stacks", Value::Int(1));
        let state = command_state(&assets, Some(&actor), "rules.haunt");
        assert!(state.enabled);
        assert!(state.disabled_reason.is_none());

        let description = describe_command(&assets, Some(&actor), "rules.haunt");
        assert!(
            description
                .lines
                .iter()
                .any(|line| line == "Effect: Actor Stamina -2 (min 0)")
        );
        assert!(
            description
                .lines
                .iter()
                .any(|line| line == "Requires Target: Mode != warded")
        );
    }

    #[test]
    fn rules_action_description_explains_condition_effects() {
        let mut assets = Assets::new();
        assets.rules = r#"
            [conditions.guarded]
            name = "Guarded"
            duration = 2
            stacking = "refresh"
            modifiers = [
                { attribute = "ARMOR", add = 2 },
                { attribute = "SPEED", multiply = 0.5, minimum = 1 },
            ]

            [conditions.guarded.periodic]
            interval = 1
            effects = [{ resource = "STAMINA", add = 1, maximum = 10 }]

            [actions.guard]
            name = "Guard"
            kind = "stance"
            target = "self"
            cooldown = 3
            result = { apply_condition = "guarded" }
        "#
        .to_string();

        let description = describe_command(&assets, None, "rules.guard");

        assert_eq!(description.title, "Guard");
        assert!(
            description.lines.iter().any(|line| {
                line == "Applies: Guarded for 2.0s (+2 Armor, Speed: ×0.5, min 1)"
            })
        );
        assert!(
            description
                .lines
                .iter()
                .any(|line| line == "Every 1.0s per stack: +1 Stamina (max 10)")
        );
    }

    #[test]
    fn item_description_uses_ruleset_item_attrs() {
        let mut item = Item::new();
        item.item_type = "Fallback".to_string();
        item.set_attribute("name", Value::Str("Wooden Arrows".to_string()));
        item.set_attribute("category", Value::Str("arrow".to_string()));
        item.set_attribute("quantity", Value::Int(12));
        item.set_attribute("max_stack", Value::Int(99));
        item.set_attribute("quality", Value::Int(73));
        item.set_attribute("condition", Value::Int(88));
        item.set_attribute("damage_roll", Value::Str("1d6".to_string()));
        item.set_attribute("damage_bonus", Value::Int(1));
        item.set_attribute("damage_bonus_attribute", Value::Str("DEX".to_string()));
        item.set_attribute("damage_bonus_every", Value::Int(4));
        item.set_attribute("damage_kind", Value::Str("physical".to_string()));

        let mut assets = Assets::new();
        assets.rules = r#"
            [attributes]
            combat = ["DMG", "ARMOR"]
            [attributes.roles]
            weapon_damage = "DMG"
            armor = "ARMOR"
        "#
        .to_string();
        let description = describe_item(&item, &assets);

        assert_eq!(description.title, "Wooden Arrows");
        assert!(
            description
                .lines
                .iter()
                .any(|line| line == "Quantity: 12 / 99")
        );
        assert!(description.lines.iter().any(|line| line == "Quality: 73%"));
        assert!(
            description
                .lines
                .iter()
                .any(|line| line == "Condition: 88%")
        );
        assert!(
            description
                .lines
                .iter()
                .any(|line| line == "Damage: 1d6 + 1 + DEX/4")
        );
        assert!(
            description
                .lines
                .iter()
                .any(|line| line == "Damage kind: Physical")
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

        let description = describe_item(&bag, &Assets::new());
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
