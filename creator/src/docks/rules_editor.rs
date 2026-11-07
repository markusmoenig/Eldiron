//! Catalogue and property inspector for project rules.
use crate::prelude::*;
use shared::rulesets::document::PathPart;
use shared::rulesets::document::{RulesTable as Table, RulesValue as TomlValue};

const TREE: &str = "Rules Catalogue";
const FORM: &str = "Rules Inspector";
const ENTRY: &str = "Rules Entry ";
const FIELD: &str = "Rules Field ";

pub struct RulesEditor {
    entries: Vec<Vec<String>>,
    fields: Vec<(Vec<String>, TomlValue, Option<usize>, Option<String>)>,
    lists: Vec<(Vec<String>, Option<usize>)>,
    nested_fields: Vec<(Vec<PathPart>, TomlValue)>,
    nested_lists: Vec<(Vec<PathPart>, Option<usize>)>,
    nested_tables: Vec<Vec<PathPart>>,
    icons: Vec<(String, TheRGBABuffer)>,
    choices: std::collections::HashMap<usize, Vec<String>>,
    selected: Option<Vec<String>>,
    form_path: Option<Vec<String>>,
    search: String,
    dirty: bool,
    undo: Vec<shared::rulesets::document::RulesState>,
    redo: Vec<shared::rulesets::document::RulesState>,
    preview_started: std::time::Instant,
    preview_updated: std::time::Instant,
}

fn lookup<'a>(table: &'a Table, path: &[String]) -> Option<&'a TomlValue> {
    let (first, rest) = path.split_first()?;
    let mut value = table.get(first)?;
    for key in rest {
        value = value.as_table()?.get(key)?;
    }
    Some(value)
}

fn replace(table: &mut Table, path: &[String], value: TomlValue) -> Result<(), String> {
    let (last, parents) = path.split_last().ok_or("Empty field path")?;
    let mut current = table;
    for key in parents {
        current = current
            .get_mut(key)
            .and_then(TomlValue::as_table_mut)
            .ok_or("Missing table")?;
    }
    let old = current.get_mut(last).ok_or("Missing field")?;
    *old = value;
    Ok(())
}

fn lookup_mut<'a>(table: &'a mut Table, path: &[String]) -> Option<&'a mut TomlValue> {
    let (first, rest) = path.split_first()?;
    let mut value = table.get_mut(first)?;
    for key in rest {
        value = value.as_table_mut()?.get_mut(key)?;
    }
    Some(value)
}

fn nested_mut<'a>(table: &'a mut Table, path: &[PathPart]) -> Option<&'a mut TomlValue> {
    let (PathPart::Key(first), rest) = path.split_first()? else {
        return None;
    };
    let mut value = table.get_mut(first)?;
    for part in rest {
        value = match part {
            PathPart::Key(key) => value.as_table_mut()?.get_mut(key)?,
            PathPart::Index(index) => value.as_array_mut()?.get_mut(*index)?,
        };
    }
    Some(value)
}

fn nested_form(
    form: &mut dyn TheTextLayoutTrait,
    fields: &mut Vec<(Vec<PathPart>, TomlValue)>,
    lists: &mut Vec<(Vec<PathPart>, Option<usize>)>,
    tables: &mut Vec<Vec<PathPart>>,
    path: Vec<PathPart>,
    value: &TomlValue,
    label: String,
) {
    match value {
        TomlValue::Table(table) => {
            for (key, value) in table {
                let mut child = path.clone();
                child.push(PathPart::Key(key.clone()));
                nested_form(
                    form,
                    fields,
                    lists,
                    tables,
                    child,
                    value,
                    format!("{label} / {}", field_label(key)),
                );
            }
            let index = tables.len();
            tables.push(path);
            let mut name =
                TheTextLineEdit::new(TheId::named(&format!("Rules Nested Name {index}")));
            name.limiter_mut().set_max_width(420);
            form.add_pair(format!("{label} / Field"), Box::new(name));
            let mut kind =
                TheDropdownMenu::new(TheId::named(&format!("Rules Nested Kind {index}")));
            for option in ["Text", "Integer", "Number", "Boolean", "List", "Table"] {
                kind.add_option(option.into());
            }
            form.add_pair("Field type".into(), Box::new(kind));
            for (action, title) in [("Add", "Add Field"), ("Remove", "Remove Field")] {
                let mut button = TheTraybarButton::new(TheId::named(&format!(
                    "Rules Nested Table {action} {index}"
                )));
                button.set_text(title.into());
                form.add_pair(" ".into(), Box::new(button));
            }
        }
        TomlValue::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let mut child = path.clone();
                child.push(PathPart::Index(index));
                nested_form(
                    form,
                    fields,
                    lists,
                    tables,
                    child,
                    item,
                    format!("{label} / {}", index + 1),
                );
                let id = lists.len();
                lists.push((path.clone(), Some(index)));
                for (action, title) in [
                    ("Remove", "Remove"),
                    ("Up", "Move Up"),
                    ("Down", "Move Down"),
                ] {
                    let mut button = TheTraybarButton::new(TheId::named(&format!(
                        "Rules Nested List {action} {id}"
                    )));
                    button.set_text(title.into());
                    form.add_pair(" ".into(), Box::new(button));
                }
            }
            let id = lists.len();
            lists.push((path, None));
            if items.is_empty() {
                let mut kind =
                    TheDropdownMenu::new(TheId::named(&format!("Rules Nested List Kind {id}")));
                for option in ["Text", "Integer", "Number", "Boolean", "Table", "List"] {
                    kind.add_option(option.into());
                }
                form.add_pair("Item type".into(), Box::new(kind));
            }
            let mut button =
                TheTraybarButton::new(TheId::named(&format!("Rules Nested List Add {id}")));
            button.set_text("Add Item".into());
            form.add_pair(label, Box::new(button));
        }
        TomlValue::Boolean(value) => {
            let index = fields.len();
            fields.push((path, TomlValue::Boolean(*value)));
            let mut button =
                TheCheckButton::new(TheId::named(&format!("Rules Nested Boolean {index}")));
            button.set_value(TheValue::Bool(*value));
            form.add_pair(label, Box::new(button));
        }
        _ => {
            let index = fields.len();
            fields.push((path, value.clone()));
            let mut edit =
                TheTextLineEdit::new(TheId::named(&format!("Rules Nested Value {index}")));
            edit.limiter_mut().set_max_width(420);
            edit.set_value(TheValue::Text(field_text(value)));
            form.add_pair(label, Box::new(edit));
        }
    }
}

fn field_text(value: &TomlValue) -> String {
    match value {
        TomlValue::String(s) => s.clone(),
        TomlValue::Float(number) if (*number as f32) as f64 == *number => {
            let mut text = (*number as f32).to_string();
            if !text.contains(['.', 'e', 'E']) {
                text.push_str(".0");
            }
            text
        }
        other => other.to_string(),
    }
}

fn field_label(key: &str) -> String {
    key.split('_')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn domain_field_order(domain: &str, key: &str) -> usize {
    let leading = match domain {
        "actions" => &[
            "name",
            "description",
            "kind",
            "target",
            "range",
            "cooldown",
            "requires",
            "cost",
            "consumes",
            "result",
            "fx",
            "ui",
        ][..],
        "items" => &[
            "name",
            "description",
            "category",
            "slot",
            "rarity",
            "damage",
            "attributes",
            "icon",
            "visual_template",
        ][..],
        "classes" => &[
            "name",
            "description",
            "role",
            "primary_attributes",
            "attributes",
            "progression",
            "unlocks",
            "starting_loadout",
            "allowed_weapons",
            "allowed_armor",
        ][..],
        "races" => &[
            "name",
            "description",
            "default_avatar",
            "traits",
            "languages",
            "attributes",
        ][..],
        "recipes" => &[
            "name",
            "description",
            "skill",
            "required_skill",
            "recommended_skill",
            "difficulty",
            "attribute",
            "station",
            "time",
            "requires",
            "consumes",
            "produces",
        ][..],
        "conditions" => &[
            "name",
            "description",
            "duration",
            "stacking",
            "max_stacks",
            "tags",
            "immune_traits",
            "modifiers",
            "periodic",
            "fx",
        ][..],
        "spells" | "abilities" => {
            &["name", "description", "kind", "school", "damage", "healing"][..]
        }
        _ => &["name", "description", "kind"][..],
    };
    leading
        .iter()
        .position(|field| *field == key)
        .unwrap_or(leading.len())
}

fn domain_field_help(domain: &str, key: &str) -> &'static str {
    match (domain, key) {
        ("actions", "cooldown") => "Time in seconds before this action can be used again.",
        ("actions", "target") => "Which target the action can affect.",
        ("recipes", "time") => "Crafting time in seconds.",
        ("recipes", "consumes") => "Ingredients consumed by the recipe.",
        ("recipes", "produces") => "Items created by the recipe.",
        ("conditions", "duration") => "Duration in seconds.",
        ("conditions", "stacking") => "How repeated applications combine.",
        ("classes", "unlocks") => "Abilities and spells learned at each level.",
        ("classes", "starting_loadout") => "Equipment and inventory provided to new characters.",
        ("races", "default_avatar") => {
            "Avatar used when the character has no explicit visual override."
        }
        ("items", "slot") => "Equipment slot used by this item.",
        _ => "Edit this value and press Return.",
    }
}

/// Choices are derived from the active document, and the current value is
/// always retained so custom rules never disappear from the editor.
fn domain_choices(rules: &Table, path: &[String], key: &str, current: &str) -> Vec<String> {
    let domain = path.first().map(String::as_str).unwrap_or("");
    let section = match key {
        "ability" | "abilities" => Some("abilities"),
        "spell" | "spells" => Some("spells"),
        "recipe" => Some("recipes"),
        "apply_condition" | "condition" => Some("conditions"),
        "class_hint" => Some("classes"),
        "profession_hint" => Some("professions"),
        "trait" | "traits" | "immune_traits" => Some("traits"),
        "skill" => Some("skills"),
        "race" => Some("races"),
        "class" => Some("classes"),
        "visual_template" => Some("visual_templates"),
        _ => None,
    };
    let mut options: Vec<String> = if let Some(section) = section {
        rules
            .get(section)
            .and_then(TomlValue::as_table)
            .map(|t| t.keys().cloned().collect())
            .unwrap_or_default()
    } else if matches!(
        key,
        "attribute" | "bonus_attribute" | "maximum_attribute" | "primary_attributes"
    ) {
        rules
            .get("attributes")
            .and_then(|v| v.get("defaults"))
            .and_then(TomlValue::as_table)
            .map(|t| {
                t.keys()
                    .filter(|key| key.chars().all(|c| !c.is_alphabetic() || c.is_uppercase()))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    } else if key == "item"
        || (domain == "classes" && path.iter().any(|part| part == "starting_loadout"))
    {
        rules
            .get("items")
            .and_then(TomlValue::as_table)
            .map(|groups| {
                groups
                    .values()
                    .filter_map(TomlValue::as_table)
                    .flat_map(|items| items.keys().cloned())
                    .collect()
            })
            .unwrap_or_default()
    } else if key == "damage_kind" {
        rules
            .get("combat")
            .and_then(|v| v.get("kinds"))
            .and_then(TomlValue::as_table)
            .map(|t| t.keys().cloned().collect())
            .unwrap_or_default()
    } else if key == "default_avatar" {
        rules
            .get("assets")
            .and_then(|v| v.get("avatars"))
            .and_then(TomlValue::as_table)
            .map(|t| t.keys().cloned().collect())
            .unwrap_or_default()
    } else if domain == "items" && key == "slot" {
        rules
            .get("equipment")
            .and_then(TomlValue::as_table)
            .map(|equipment| {
                ["weapon_slots", "armor_slots"]
                    .into_iter()
                    .filter_map(|k| equipment.get(k).and_then(TomlValue::as_array))
                    .flatten()
                    .filter_map(TomlValue::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    } else if (domain == "items" && key == "category")
        || (domain == "classes" && matches!(key, "allowed_weapons" | "allowed_armor"))
    {
        let group = if key == "allowed_armor" || path.get(1).is_some_and(|part| part == "armor") {
            "armor_categories"
        } else {
            "weapon_categories"
        };
        rules
            .get("equipment")
            .and_then(|v| v.get(group))
            .and_then(TomlValue::as_table)
            .map(|t| t.keys().cloned().collect())
            .unwrap_or_default()
    } else if domain == "conditions" && key == "stacking" {
        ["replace", "refresh", "stack", "ignore"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    } else if domain == "items" && key == "rig_layer" {
        ["front", "back"].iter().map(|s| s.to_string()).collect()
    } else {
        Vec::new()
    };
    options.sort();
    options.dedup();
    if !options.is_empty() && !options.iter().any(|option| option == current) {
        options.push(current.into());
    }
    options
}

fn add_domain_choice(
    form: &mut dyn TheTextLayoutTrait,
    fields: &mut Vec<(Vec<String>, TomlValue, Option<usize>, Option<String>)>,
    choices: &mut std::collections::HashMap<usize, Vec<String>>,
    rules: &Table,
    path: &[String],
    value: &TomlValue,
    index: Option<usize>,
    key: Option<String>,
    label: &str,
) -> bool {
    let Some(raw) = value.as_str() else {
        return false;
    };
    let field_key = key
        .as_deref()
        .or_else(|| path.last().map(String::as_str))
        .unwrap_or("");
    let options = domain_choices(rules, path, field_key, raw);
    if options.is_empty() {
        return false;
    }
    let field = fields.len();
    let mut menu = TheDropdownMenu::new(TheId::named(&format!("Rules Choice {field}")));
    menu.limiter_mut().set_max_width(420);
    for option in &options {
        menu.add_option(option.clone());
    }
    menu.set_selected_index(options.iter().position(|value| value == raw).unwrap_or(0) as i32);
    form.add_pair(label.into(), Box::new(menu));
    let mut custom = TheTextLineEdit::new(TheId::named(&format!("{FIELD}{field}")));
    custom.limiter_mut().set_max_width(420);
    custom.set_value(TheValue::Text(raw.into()));
    custom.set_status_text("Enter a custom identifier if it is not in the list");
    form.add_pair("Custom value".into(), Box::new(custom));
    choices.insert(field, options);
    fields.push((path.to_vec(), value.clone(), index, key));
    true
}

fn parse_field(text: &str, old: &TomlValue) -> Result<TomlValue, String> {
    if matches!(old, TomlValue::String(_)) {
        return Ok(TomlValue::String(text.into()));
    }
    let wrapper = format!("value = {text}");
    let parsed: Table = wrapper.parse::<Table>().map_err(|e| e.to_string())?;
    let value = parsed.get("value").ok_or("Missing value")?.clone();
    if std::mem::discriminant(&value) != std::mem::discriminant(old) {
        return Err("Value must keep its original type".into());
    }
    Ok(value)
}

impl RulesEditor {
    fn catalogue(&mut self, ui: &mut TheUI, project: &Project) {
        self.entries.clear();
        let Ok(doc) = project.rules_document.document() else {
            return;
        };
        let Some(tree) = ui.get_tree_layout(TREE) else {
            return;
        };
        let root = tree.get_root();
        root.clear();
        for (category, value) in doc.table() {
            let Some(table) = value.as_table() else {
                continue;
            };
            let category_start = self.entries.len();
            let mut group = TheTreeNode::new(TheId::named(category));
            group.set_open(!self.search.is_empty());
            for (name, value) in table {
                if (category == "items" || (category == "fx" && name == "presets"))
                    && value.as_table().is_some()
                {
                    let mut sub = TheTreeNode::new(TheId::named(&format!("{category}/{name}")));
                    sub.set_open(!self.search.is_empty());
                    let sub_start = self.entries.len();
                    for (id, _) in value.as_table().unwrap() {
                        if !format!("{category} {name} {id}")
                            .to_lowercase()
                            .contains(&self.search)
                        {
                            continue;
                        }
                        let path = vec![category.clone(), name.clone(), id.clone()];
                        let mut item = TheTreeItem::new(TheId::named(&format!(
                            "{ENTRY}{}",
                            self.entries.len()
                        )));
                        item.set_text(id.clone());
                        sub.add_widget(Box::new(item));
                        self.entries.push(path);
                    }
                    if self.search.is_empty() || self.entries.len() != sub_start {
                        group.add_child(sub);
                    }
                } else {
                    if !format!("{category} {name}")
                        .to_lowercase()
                        .contains(&self.search)
                    {
                        continue;
                    }
                    let path = vec![category.clone(), name.clone()];
                    let mut item =
                        TheTreeItem::new(TheId::named(&format!("{ENTRY}{}", self.entries.len())));
                    item.set_text(name.clone());
                    group.add_widget(Box::new(item));
                    self.entries.push(path);
                }
            }
            if self.search.is_empty() || self.entries.len() != category_start {
                root.add_child(group);
            }
        }
    }

    fn form(&mut self, ui: &mut TheUI, project: &Project) {
        self.fields.clear();
        self.lists.clear();
        self.nested_fields.clear();
        self.nested_lists.clear();
        self.nested_tables.clear();
        self.choices.clear();
        if let Some(form) = ui.get_text_layout(FORM) {
            let offset = if self.form_path == self.selected {
                form.scroll_offset()
            } else {
                0
            };
            form.clear();
            form.set_scroll_offset(offset);
        }
        self.form_path = self.selected.clone();
        let Ok(doc) = project.rules_document.document() else {
            return;
        };
        let Some(path) = self.selected.clone() else {
            return;
        };
        let Some(value) = lookup(doc.table(), &path) else {
            return;
        };
        self.icons.clear();
        if let Ok(textures) = shared::rulesets::bundled_textures_for_project(&project.config) {
            for (id, texture) in textures {
                self.icons.push((
                    id.into(),
                    TheRGBABuffer::from(texture.data, texture.width as u32, texture.height as u32),
                ));
            }
        }
        for (id, tile) in &project.tiles {
            if let Some(texture) = tile.textures.first() {
                self.icons.push((
                    id.to_string(),
                    TheRGBABuffer::from(
                        texture.data.clone(),
                        texture.width as u32,
                        texture.height as u32,
                    ),
                ));
            }
        }
        self.icons.sort_by(|a, b| a.0.cmp(&b.0));
        let Some(form) = ui.get_text_layout(FORM) else {
            return;
        };
        if path.len() == 3 && path[0] == "fx" && path[1] == "presets" {
            if let Some(emitter) = value.get("emitter") {
                if let Ok(json) = serde_json::to_value(emitter) {
                    if let Ok(def) = serde_json::from_value::<rusterix::ParticleEmitterDef>(json) {
                        let mut emitter = def.instantiate(Vec3::zero(), def.direction);
                        let scale = value
                            .get("size_scale")
                            .and_then(TomlValue::as_float)
                            .unwrap_or(1.0) as f32;
                        emitter.radius_range.0 *= scale;
                        emitter.radius_range.1 *= scale;
                        let image = crate::docks::particle_preview::render_particle_emitter_preview(
                            &emitter, 280, 160, 1.0,
                        );
                        let mut preview = TheIconView::new(TheId::named("Rules Particle Preview"));
                        preview.limiter_mut().set_max_size(Vec2::new(280, 160));
                        preview.set_rgba_tile(TheRGBATile::buffer(image));
                        form.add_pair("Preview".into(), Box::new(preview));
                    }
                }
            }
        }
        if value.is_table() {
            let title = match path.first().map(String::as_str) {
                Some("actions") => "Action",
                Some("items") => "Item",
                Some("classes") => "Class",
                Some("races") => "Race",
                Some("conditions") => "Condition",
                Some("recipes") => "Recipe",
                Some("fx") => "Particle Preset",
                _ => "Definition",
            };
            let mut heading = TheText::new(TheId::empty());
            heading.set_text(format!(
                "{title}: {}",
                path.last().map(String::as_str).unwrap_or_default()
            ));
            form.add_pair(String::new(), Box::new(heading));
            let mut identifier = TheTextLineEdit::new(TheId::named("Rules Definition ID"));
            identifier.limiter_mut().set_max_width(420);
            identifier.set_value(TheValue::Text(path.last().cloned().unwrap_or_default()));
            identifier.set_status_text("Stable identifier. Renaming does not update references.");
            form.add_pair("Identifier".into(), Box::new(identifier));
            let mut rename = TheTraybarButton::new(TheId::named("Rules Rename Definition"));
            rename.set_text("Rename".into());
            form.add_pair(" ".into(), Box::new(rename));
            let mut name = TheTextLineEdit::new(TheId::named("Rules New Field Name"));
            name.limiter_mut().set_max_width(420);
            name.set_status_text(
                "Field name, or a dotted path to a nested table, for example ui.icon",
            );
            form.add_pair("New field".into(), Box::new(name));
            let mut kind = TheDropdownMenu::new(TheId::named("Rules New Field Type"));
            kind.limiter_mut().set_max_width(420);
            for name in ["Text", "Integer", "Number", "Boolean", "List", "Table"] {
                kind.add_option(name.into());
            }
            form.add_pair("Type".into(), Box::new(kind));
            let mut button = TheTraybarButton::new(TheId::named("Rules Add Field"));
            button.set_text("Add Field".into());
            form.add_pair(" ".into(), Box::new(button));
            let mut remove = TheTraybarButton::new(TheId::named("Rules Remove Field"));
            remove.set_text("Remove Field".into());
            remove.set_status_text("Remove the field named above. Undo can restore it.");
            form.add_pair(" ".into(), Box::new(remove));
        }
        fn add(
            form: &mut dyn TheTextLayoutTrait,
            fields: &mut Vec<(Vec<String>, TomlValue, Option<usize>, Option<String>)>,
            lists: &mut Vec<(Vec<String>, Option<usize>)>,
            nested_fields: &mut Vec<(Vec<PathPart>, TomlValue)>,
            nested_lists: &mut Vec<(Vec<PathPart>, Option<usize>)>,
            nested_tables: &mut Vec<Vec<PathPart>>,
            icons: &[(String, TheRGBABuffer)],
            choices: &mut std::collections::HashMap<usize, Vec<String>>,
            rules: &Table,
            path: Vec<String>,
            value: &TomlValue,
            label: String,
        ) {
            if let Some(table) = value.as_table() {
                let mut entries: Vec<_> = table.iter().collect();
                entries.sort_by_key(|(key, _)| {
                    (
                        domain_field_order(path.first().map(String::as_str).unwrap_or(""), key),
                        *key,
                    )
                });
                for (key, child) in entries {
                    let mut next = path.clone();
                    next.push(key.clone());
                    let label = if label.is_empty() {
                        match (path.first().map(String::as_str), key.as_str()) {
                            (Some("actions"), "cooldown") => "Cooldown (seconds)".into(),
                            (Some("actions"), "range") => "Range (tiles or weapon)".into(),
                            (Some("recipes"), "time") => "Time (seconds)".into(),
                            (Some("conditions"), "duration") => "Duration (seconds)".into(),
                            _ => field_label(key),
                        }
                    } else {
                        format!("{label} / {}", field_label(key))
                    };
                    add(
                        form,
                        fields,
                        lists,
                        nested_fields,
                        nested_lists,
                        nested_tables,
                        icons,
                        choices,
                        rules,
                        next,
                        child,
                        label,
                    );
                }
            } else if let Some(items) = value.as_array() {
                for (index, item) in items.iter().enumerate() {
                    if let Some(record) = item.as_table() {
                        for (key, value) in record {
                            if value.is_table() || value.is_array() {
                                let mut nested: Vec<_> =
                                    path.iter().cloned().map(PathPart::Key).collect();
                                nested.push(PathPart::Index(index));
                                nested.push(PathPart::Key(key.clone()));
                                nested_form(
                                    form,
                                    nested_fields,
                                    nested_lists,
                                    nested_tables,
                                    nested,
                                    value,
                                    format!("{label} / {} / {}", index + 1, field_label(key)),
                                );
                                continue;
                            }
                            let item_label =
                                format!("{label} / {} / {}", index + 1, field_label(key));
                            if add_domain_choice(
                                form,
                                fields,
                                choices,
                                rules,
                                &path,
                                value,
                                Some(index),
                                Some(key.clone()),
                                &item_label,
                            ) {
                                continue;
                            }
                            let mut edit = TheTextLineEdit::new(TheId::named(&format!(
                                "{FIELD}{}",
                                fields.len()
                            )));
                            edit.limiter_mut().set_max_width(420);
                            edit.set_value(TheValue::Text(field_text(value)));
                            form.add_pair(item_label, Box::new(edit));
                            fields.push((
                                path.clone(),
                                value.clone(),
                                Some(index),
                                Some(key.clone()),
                            ));
                        }
                        let record_id = lists.len();
                        let mut name = TheTextLineEdit::new(TheId::named(&format!(
                            "Rules Record Name {record_id}"
                        )));
                        name.limiter_mut().set_max_width(420);
                        name.set_status_text("Name of a field to add to this list item");
                        form.add_pair("New item field".into(), Box::new(name));
                        let mut kind = TheDropdownMenu::new(TheId::named(&format!(
                            "Rules Record Type {record_id}"
                        )));
                        kind.limiter_mut().set_max_width(420);
                        for kind_name in ["Text", "Integer", "Number", "Boolean", "List", "Table"] {
                            kind.add_option(kind_name.into());
                        }
                        form.add_pair("Item field type".into(), Box::new(kind));
                        let mut button = TheTraybarButton::new(TheId::named(&format!(
                            "Rules Record Field {record_id}"
                        )));
                        button.set_text("Add Item Field".into());
                        form.add_pair(" ".into(), Box::new(button));
                    } else if item.is_array() {
                        let mut nested: Vec<_> = path.iter().cloned().map(PathPart::Key).collect();
                        nested.push(PathPart::Index(index));
                        nested_form(
                            form,
                            nested_fields,
                            nested_lists,
                            nested_tables,
                            nested,
                            item,
                            format!("{label} / {}", index + 1),
                        );
                    } else {
                        let item_label = format!("{label} / {}", index + 1);
                        if add_domain_choice(
                            form,
                            fields,
                            choices,
                            rules,
                            &path,
                            item,
                            Some(index),
                            None,
                            &item_label,
                        ) {
                            // The row still needs its remove and reorder controls below.
                        } else {
                            let mut edit = TheTextLineEdit::new(TheId::named(&format!(
                                "{FIELD}{}",
                                fields.len()
                            )));
                            edit.limiter_mut().set_max_width(420);
                            edit.set_value(TheValue::Text(field_text(item)));
                            form.add_pair(item_label, Box::new(edit));
                            fields.push((path.clone(), item.clone(), Some(index), None));
                        }
                    }
                    let mut button = TheTraybarButton::new(TheId::named(&format!(
                        "Rules List Remove {}",
                        lists.len()
                    )));
                    button.set_text("Remove".into());
                    form.add_pair(" ".into(), Box::new(button));
                    for (action, label) in [("Up", "Move Up"), ("Down", "Move Down")] {
                        let mut button = TheTraybarButton::new(TheId::named(&format!(
                            "Rules List {action} {}",
                            lists.len()
                        )));
                        button.set_text(label.into());
                        form.add_pair(" ".into(), Box::new(button));
                    }
                    lists.push((path.clone(), Some(index)));
                }
                if items.is_empty() {
                    let mut kind = TheDropdownMenu::new(TheId::named(&format!(
                        "Rules List Type {}",
                        lists.len()
                    )));
                    kind.limiter_mut().set_max_width(420);
                    for name in ["Text", "Integer", "Number", "Boolean", "Table", "List"] {
                        kind.add_option(name.into());
                    }
                    form.add_pair("Item type".into(), Box::new(kind));
                }
                let mut button =
                    TheTraybarButton::new(TheId::named(&format!("Rules List Add {}", lists.len())));
                button.set_text("Add Item".into());
                form.add_pair(label, Box::new(button));
                lists.push((path, None));
            } else {
                if let TomlValue::Boolean(checked) = value {
                    let mut toggle = TheCheckButton::new(TheId::named(&format!(
                        "Rules Boolean {}",
                        fields.len()
                    )));
                    toggle.set_value(TheValue::Bool(*checked));
                    form.add_pair(label, Box::new(toggle));
                    fields.push((path, value.clone(), None, None));
                    return;
                }
                if let Some(raw) = value.as_str() {
                    let key = path.last().map(String::as_str).unwrap_or("");
                    if key == "icon"
                        || key.ends_with("_icon")
                        || (path.first().is_some_and(|p| p == "icons") && key == "texture")
                    {
                        let mut menu = TheDropdownMenu::new(TheId::named(&format!(
                            "Rules Icon {}",
                            fields.len()
                        )));
                        menu.limiter_mut().set_max_width(420);
                        menu.add_option("None".into());
                        for (id, _) in icons {
                            menu.add_option(id.clone());
                        }
                        menu.set_selected_index(
                            icons
                                .iter()
                                .position(|(id, _)| id == raw)
                                .map(|i| i as i32 + 1)
                                .unwrap_or(0),
                        );
                        form.add_pair(label.clone(), Box::new(menu));
                        if let Some((_, image)) = icons.iter().find(|(id, _)| id == raw) {
                            let mut preview = TheIconView::new(TheId::empty());
                            preview.limiter_mut().set_max_size(Vec2::new(48, 48));
                            preview.set_rgba_tile(TheRGBATile::buffer(image.clone()));
                            form.add_pair("Icon preview".into(), Box::new(preview));
                        }
                        let mut grid = TheIconGridView::new(TheId::named(&format!(
                            "Rules Icon Grid {}",
                            fields.len()
                        )));
                        grid.limiter_mut().set_max_width(420);
                        grid.limiter_mut().set_min_height(160);
                        grid.limiter_mut().set_max_height(160);
                        grid.set_show_labels(false);
                        grid.set_cell_size(44);
                        grid.set_icon_size(32);
                        grid.set_items(
                            icons
                                .iter()
                                .map(|(id, image)| {
                                    let mut item = TheIconGridItem::new(id.clone());
                                    item.status = id.clone();
                                    item.icon = Some(image.clone());
                                    item
                                })
                                .collect(),
                        );
                        grid.set_selected(icons.iter().position(|(id, _)| id == raw));
                        form.add_pair("Choose artwork".into(), Box::new(grid));
                        fields.push((path, value.clone(), None, None));
                        return;
                    }
                    if add_domain_choice(
                        form, fields, choices, rules, &path, value, None, None, &label,
                    ) {
                        return;
                    }
                    let section = match key {
                        "ability" => Some("abilities"),
                        "spell" => Some("spells"),
                        "recipe" => Some("recipes"),
                        "apply_condition" => Some("conditions"),
                        _ => None,
                    };
                    let mut options: Vec<String> = if let Some(section) = section {
                        rules
                            .get(section)
                            .and_then(TomlValue::as_table)
                            .map(|t| t.keys().cloned().collect())
                            .unwrap_or_default()
                    } else if path.first().is_some_and(|v| v == "actions") && key == "target" {
                        [
                            "self",
                            "hostile_entity",
                            "hostile_or_neutral_entity",
                            "friendly_entity",
                            "friendly_or_self",
                            "any_entity",
                            "ground_item",
                            "inventory_item",
                            "any_item",
                            "resource_node",
                            "world_position",
                        ]
                        .iter()
                        .map(|s| s.to_string())
                        .collect()
                    } else if path.first().is_some_and(|v| v == "actions") && key == "kind" {
                        [
                            "attack",
                            "spell",
                            "interaction",
                            "gather",
                            "craft",
                            "stance",
                        ]
                        .iter()
                        .map(|s| s.to_string())
                        .collect()
                    } else if key == "preset" {
                        rules
                            .get("fx")
                            .and_then(|v| v.get("presets"))
                            .and_then(TomlValue::as_table)
                            .map(|t| t.keys().cloned().collect())
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };
                    if !options.is_empty() {
                        if !options.iter().any(|v| v == raw) {
                            options.push(raw.into());
                        }
                        let index = fields.len();
                        let mut menu =
                            TheDropdownMenu::new(TheId::named(&format!("Rules Choice {index}")));
                        menu.limiter_mut().set_max_width(420);
                        for option in &options {
                            menu.add_option(option.clone());
                        }
                        menu.set_selected_index(
                            options.iter().position(|v| v == raw).unwrap_or(0) as i32
                        );
                        form.add_pair(label, Box::new(menu));
                        let mut custom =
                            TheTextLineEdit::new(TheId::named(&format!("{FIELD}{index}")));
                        custom.limiter_mut().set_max_width(420);
                        custom.set_value(TheValue::Text(raw.into()));
                        custom.set_status_text("Enter a custom value if it is not in the list");
                        form.add_pair("Custom value".into(), Box::new(custom));
                        choices.insert(index, options);
                        fields.push((path, value.clone(), None, None));
                        return;
                    }
                }
                let mut edit =
                    TheTextLineEdit::new(TheId::named(&format!("{FIELD}{}", fields.len())));
                edit.limiter_mut().set_max_width(420);
                edit.set_value(TheValue::Text(field_text(value)));
                edit.set_status_text(domain_field_help(
                    path.first().map(String::as_str).unwrap_or(""),
                    path.last().map(String::as_str).unwrap_or(""),
                ));
                form.add_pair(label, Box::new(edit));
                fields.push((path, value.clone(), None, None));
            }
        }
        add(
            form,
            &mut self.fields,
            &mut self.lists,
            &mut self.nested_fields,
            &mut self.nested_lists,
            &mut self.nested_tables,
            &self.icons,
            &mut self.choices,
            doc.table(),
            path,
            value,
            String::new(),
        );
    }
}

impl Dock for RulesEditor {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            fields: Vec::new(),
            lists: Vec::new(),
            nested_fields: Vec::new(),
            nested_lists: Vec::new(),
            nested_tables: Vec::new(),
            icons: Vec::new(),
            choices: Default::default(),
            selected: None,
            form_path: None,
            search: String::new(),
            dirty: false,
            undo: Vec::new(),
            redo: Vec::new(),
            preview_started: std::time::Instant::now(),
            preview_updated: std::time::Instant::now(),
        }
    }
    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();
        let mut left = TheCanvas::new();
        let mut tree = TheTreeLayout::new(TheId::named(TREE));
        tree.set_headerless(true);
        tree.limiter_mut().set_min_width(260);
        tree.limiter_mut().set_max_width(260);
        tree.set_background_color(Some(TheThemeColors::DefaultWidgetDarkBackground));
        left.set_layout(tree);
        let mut search = TheCanvas::new();
        let mut search_edit = TheTextLineEdit::new(TheId::named("Rules Search"));
        search_edit.limiter_mut().set_max_width(260);
        search_edit.set_status_text("Filter rules by category or identifier");
        search.set_widget(search_edit);
        left.set_top(search);
        canvas.set_left(left);
        let mut form = TheTextLayout::new(TheId::named(FORM));
        form.set_margin(Vec4::new(10, 8, 10, 8));
        form.set_padding(7);
        // Nested paths such as "Unlocks / Level 4 / Abilities / 1" must fit
        // without wrapping over the following control's label.
        form.set_fixed_text_width(270);
        canvas.set_layout(form);
        let mut top = TheCanvas::new();
        top.set_widget(TheTraybar::new(TheId::empty()));
        let mut bar = TheHLayout::new(TheId::empty());
        bar.set_background_color(None);
        bar.set_margin(Vec4::new(8, 1, 8, 1));
        bar.set_padding(4);
        for (id, title) in [
            ("Rules New", "New"),
            ("Rules Duplicate", "Duplicate"),
            ("Rules Delete", "Delete"),
            ("Rules Save Checkpoint", "Checkpoint"),
            ("Rules Restore", "Restore All"),
            ("Rules Recover", "Recover"),
        ] {
            let mut button = TheTraybarButton::new(TheId::named(id));
            button.set_text(title.into());
            button.set_fixed_size(false);
            bar.add_widget(Box::new(button));
        }
        top.set_layout(bar);
        canvas.set_top(top);
        canvas
    }
    fn default_state(&self) -> DockDefaultState {
        DockDefaultState::Maximized
    }
    fn has_changes(&self) -> bool {
        self.dirty
    }
    fn supports_undo(&self) -> bool {
        true
    }
    fn set_undo_state_to_ui(&self, ctx: &mut TheContext) {
        if self.undo.is_empty() {
            ctx.ui.set_disabled("Undo");
        } else {
            ctx.ui.set_enabled("Undo");
        }
        if self.redo.is_empty() {
            ctx.ui.set_disabled("Redo");
        } else {
            ctx.ui.set_enabled("Redo");
        }
    }
    fn mark_saved(&mut self) {
        self.dirty = false;
    }
    fn reset_for_project_switch(&mut self) {
        self.selected = None;
        self.form_path = None;
        self.entries.clear();
        self.fields.clear();
        self.lists.clear();
        self.nested_fields.clear();
        self.nested_lists.clear();
        self.nested_tables.clear();
        self.icons.clear();
        self.choices.clear();
        self.search.clear();
        self.dirty = false;
        self.undo.clear();
        self.redo.clear();
    }
    fn undo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server: &mut ServerContext,
    ) {
        if let Some(previous) = self.undo.pop() {
            self.redo.push(project.rules_document.clone());
            project.rules_document = previous;
            self.catalogue(ui, project);
            self.form(ui, project);
            self.dirty = true;
            self.set_undo_state_to_ui(ctx);
            if let Err(error) = crate::docks::nodes::sync_live_rules(project) {
                ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
            }
        }
    }
    fn redo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server: &mut ServerContext,
    ) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(project.rules_document.clone());
            project.rules_document = next;
            self.catalogue(ui, project);
            self.form(ui, project);
            self.dirty = true;
            self.set_undo_state_to_ui(ctx);
            if let Err(error) = crate::docks::nodes::sync_live_rules(project) {
                ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
            }
        }
    }
    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        _server: &mut ServerContext,
    ) {
        ui.set_widget_value("Rules Search", ctx, TheValue::Text(self.search.clone()));
        self.catalogue(ui, project);
        self.form(ui, project);
    }
    fn poll_background(
        &mut self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &mut Project,
        _server: &mut ServerContext,
    ) -> bool {
        let Some(path) = self
            .selected
            .as_ref()
            .filter(|p| p.len() == 3 && p[0] == "fx" && p[1] == "presets")
        else {
            return false;
        };
        if self.preview_updated.elapsed().as_millis() < 80 {
            return false;
        }
        self.preview_updated = std::time::Instant::now();
        let Ok(doc) = project.rules_document.document() else {
            return false;
        };
        let Some(preset) = lookup(doc.table(), path) else {
            return false;
        };
        let Some(value) = preset.get("emitter") else {
            return false;
        };
        let Ok(json) = serde_json::to_value(value) else {
            return false;
        };
        let Ok(def) = serde_json::from_value::<rusterix::ParticleEmitterDef>(json) else {
            return false;
        };
        let Some(preview) = ui.get_icon_view("Rules Particle Preview") else {
            return false;
        };
        let mut emitter = def.instantiate(Vec3::zero(), def.direction);
        let scale = preset
            .get("size_scale")
            .and_then(TomlValue::as_float)
            .unwrap_or(1.0) as f32;
        emitter.radius_range.0 *= scale;
        emitter.radius_range.1 *= scale;
        let time = 0.4 + self.preview_started.elapsed().as_secs_f32() % 2.0;
        let image = crate::docks::particle_preview::render_particle_emitter_preview(
            &emitter, 280, 160, time,
        );
        preview.set_rgba_tile(TheRGBATile::buffer(image));
        true
    }
    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server: &mut ServerContext,
    ) -> bool {
        match event {
            TheEvent::ValueChanged(id, value)
                if id.name.starts_with("Rules Nested Value ")
                    || id.name.starts_with("Rules Nested Boolean ") =>
            {
                let prefix = if id.name.starts_with("Rules Nested Value ") {
                    "Rules Nested Value "
                } else {
                    "Rules Nested Boolean "
                };
                let Some((path, old)) = id
                    .name
                    .strip_prefix(prefix)
                    .and_then(|s| s.parse::<usize>().ok())
                    .and_then(|i| self.nested_fields.get(i))
                    .cloned()
                else {
                    return false;
                };
                let replacement = match value {
                    TheValue::Text(text) => parse_field(text, &old),
                    TheValue::Bool(value) => Ok(TomlValue::Boolean(*value)),
                    _ => return false,
                };
                let before = project.rules_document.clone();
                let result = replacement.and_then(|value| {
                    project.rules_document.edit(|table| {
                        *nested_mut(table, &path).ok_or("Missing value")? = value;
                        Ok(())
                    })
                });
                if let Err(error) = result {
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                    return false;
                }
                if before != project.rules_document {
                    self.undo.push(before);
                    self.redo.clear();
                    self.dirty = true;
                    self.set_undo_state_to_ui(ctx);
                }
                if let Err(error) = crate::docks::nodes::sync_live_rules(project) {
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                }
                return true;
            }
            TheEvent::IndexChanged(id, selected) if id.name.starts_with("Rules Icon Grid ") => {
                let Some((path, _, _, _)) = id
                    .name
                    .strip_prefix("Rules Icon Grid ")
                    .and_then(|s| s.parse::<usize>().ok())
                    .and_then(|i| self.fields.get(i))
                    .cloned()
                else {
                    return false;
                };
                let Some(value) = self.icons.get(*selected).map(|(id, _)| id.clone()) else {
                    return false;
                };
                let before = project.rules_document.clone();
                if let Err(error) = project
                    .rules_document
                    .edit(|table| replace(table, &path, TomlValue::String(value)))
                {
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                    return false;
                }
                if before != project.rules_document {
                    self.undo.push(before);
                    self.redo.clear();
                    self.dirty = true;
                    self.set_undo_state_to_ui(ctx);
                }
                self.form(ui, project);
                if let Err(error) = crate::docks::nodes::sync_live_rules(project) {
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                }
                return true;
            }
            TheEvent::IndexChanged(id, selected) if id.name.starts_with("Rules Choice ") => {
                let Some(index) = id
                    .name
                    .strip_prefix("Rules Choice ")
                    .and_then(|s| s.parse::<usize>().ok())
                else {
                    return false;
                };
                let Some((path, _, item_index, item_key)) = self.fields.get(index).cloned() else {
                    return false;
                };
                let Some(value) = self
                    .choices
                    .get(&index)
                    .and_then(|v| v.get(*selected))
                    .cloned()
                else {
                    return false;
                };
                let display_value = value.clone();
                let before = project.rules_document.clone();
                if let Err(error) = project.rules_document.edit(|table| {
                    if let Some(index) = item_index {
                        let item = lookup_mut(table, &path)
                            .and_then(TomlValue::as_array_mut)
                            .and_then(|items| items.get_mut(index))
                            .ok_or("Missing item")?;
                        if let Some(key) = item_key {
                            *item
                                .as_table_mut()
                                .and_then(|record| record.get_mut(&key))
                                .ok_or("Missing field")? = TomlValue::String(value);
                        } else {
                            *item = TomlValue::String(value);
                        }
                        Ok(())
                    } else {
                        replace(table, &path, TomlValue::String(value))
                    }
                }) {
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                    return false;
                }
                if before != project.rules_document {
                    self.undo.push(before);
                    self.redo.clear();
                    self.dirty = true;
                    self.set_undo_state_to_ui(ctx);
                }
                if let Some(field) = self.fields.get_mut(index) {
                    field.1 = TomlValue::String(display_value.clone());
                }
                if let Some(widget) = ui.get_widget(&format!("{FIELD}{index}")) {
                    widget.set_value(TheValue::Text(display_value));
                }
                if let Err(error) = crate::docks::nodes::sync_live_rules(project) {
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                }
                return true;
            }
            TheEvent::ValueChanged(id, TheValue::Bool(checked))
                if id.name.starts_with("Rules Boolean ") =>
            {
                let Some((path, _, _, _)) = id
                    .name
                    .strip_prefix("Rules Boolean ")
                    .and_then(|s| s.parse::<usize>().ok())
                    .and_then(|i| self.fields.get(i))
                    .cloned()
                else {
                    return false;
                };
                let before = project.rules_document.clone();
                if let Err(error) = project
                    .rules_document
                    .edit(|table| replace(table, &path, TomlValue::Boolean(*checked)))
                {
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                    return false;
                }
                if before != project.rules_document {
                    self.undo.push(before);
                    self.redo.clear();
                    self.dirty = true;
                    self.set_undo_state_to_ui(ctx);
                }
                if let Err(error) = crate::docks::nodes::sync_live_rules(project) {
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                }
                return true;
            }
            TheEvent::IndexChanged(id, selected) if id.name.starts_with("Rules Icon ") => {
                let Some((path, _, _, _)) = id
                    .name
                    .strip_prefix("Rules Icon ")
                    .and_then(|s| s.parse::<usize>().ok())
                    .and_then(|i| self.fields.get(i))
                    .cloned()
                else {
                    return false;
                };
                let value = if *selected == 0 {
                    String::new()
                } else {
                    self.icons
                        .get(*selected - 1)
                        .map(|(id, _)| id.clone())
                        .unwrap_or_default()
                };
                let before = project.rules_document.clone();
                if let Err(error) = project
                    .rules_document
                    .edit(|table| replace(table, &path, TomlValue::String(value)))
                {
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                    return false;
                }
                if before != project.rules_document {
                    self.undo.push(before);
                    self.redo.clear();
                    self.dirty = true;
                    self.set_undo_state_to_ui(ctx);
                }
                self.form(ui, project);
                if let Err(error) = crate::docks::nodes::sync_live_rules(project) {
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                }
                return true;
            }
            TheEvent::ValueChanged(id, TheValue::Text(text)) if id.name == "Rules Search" => {
                self.search = text.trim().to_lowercase();
                self.catalogue(ui, project);
                return true;
            }
            TheEvent::NewListItemSelected(id, layout) if layout.name == TREE => {
                if let Some(index) = id
                    .name
                    .strip_prefix(ENTRY)
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    self.selected = self.entries.get(index).cloned();
                    self.form(ui, project);
                    return true;
                }
            }
            TheEvent::ValueChanged(id, TheValue::Text(text)) if id.name.starts_with(FIELD) => {
                if let Some((path, old, item_index, item_key)) = id
                    .name
                    .strip_prefix(FIELD)
                    .and_then(|s| s.parse::<usize>().ok())
                    .and_then(|i| self.fields.get(i))
                    .cloned()
                {
                    let before = project.rules_document.clone();
                    let result = parse_field(text, &old).and_then(|value| {
                        project.rules_document.edit(|table| {
                            if let Some(index) = item_index {
                                let list = lookup_mut(table, &path)
                                    .and_then(TomlValue::as_array_mut)
                                    .ok_or("Missing list")?;
                                let item = list.get_mut(index).ok_or("Missing item")?;
                                if let Some(key) = item_key {
                                    *item
                                        .as_table_mut()
                                        .and_then(|t| t.get_mut(&key))
                                        .ok_or("Missing field")? = value;
                                } else {
                                    *item = value;
                                }
                                Ok(())
                            } else {
                                replace(table, &path, value)
                            }
                        })
                    });
                    match result {
                        Ok(()) => {
                            if project.rules_document != before {
                                self.undo.push(before);
                                self.redo.clear();
                                self.set_undo_state_to_ui(ctx);
                            }
                            self.dirty = true;
                            self.fields[id.name[FIELD.len()..].parse::<usize>().unwrap()].1 =
                                parse_field(text, &old).unwrap();
                            if self
                                .choices
                                .contains_key(&id.name[FIELD.len()..].parse::<usize>().unwrap())
                                || self.selected.as_ref().is_some_and(|p| {
                                    p.len() == 3 && p[0] == "fx" && p[1] == "presets"
                                })
                            {
                                self.form(ui, project);
                            }
                            if let Err(error) = crate::docks::nodes::sync_live_rules(project) {
                                ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                            }
                            return true;
                        }
                        Err(error) => ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error)),
                    }
                }
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) => {
                let before = project.rules_document.clone();
                match id.name.as_str() {
                    _ if id.name.starts_with("Rules Nested List ") => {
                        let Some(rest) = id.name.strip_prefix("Rules Nested List ") else {
                            return false;
                        };
                        let Some((action, suffix)) = rest.split_once(' ') else {
                            return false;
                        };
                        let Some((path, index)) = suffix
                            .parse::<usize>()
                            .ok()
                            .and_then(|i| self.nested_lists.get(i))
                            .cloned()
                        else {
                            return false;
                        };
                        let kind = ui
                            .get_drop_down_menu(&format!("Rules Nested List Kind {suffix}"))
                            .map(|w| w.selected_index())
                            .unwrap_or(0);
                        let result = project.rules_document.edit(|table| {
                            let list = nested_mut(table, &path)
                                .and_then(TomlValue::as_array_mut)
                                .ok_or("Missing list")?;
                            match (action, index) {
                                ("Add", None) => {
                                    let initial = match kind {
                                        1 => TomlValue::Integer(0),
                                        2 => TomlValue::Float(0.0),
                                        3 => TomlValue::Boolean(false),
                                        4 => TomlValue::Table(Table::new()),
                                        5 => TomlValue::Array(Vec::new()),
                                        _ => TomlValue::String(String::new()),
                                    };
                                    list.push(list.last().cloned().unwrap_or(initial));
                                }
                                ("Remove", Some(i)) if i < list.len() => {
                                    list.remove(i);
                                }
                                ("Up", Some(i)) if i > 0 && i < list.len() => list.swap(i, i - 1),
                                ("Down", Some(i)) if i + 1 < list.len() => list.swap(i, i + 1),
                                ("Up" | "Down", Some(_)) => {}
                                _ => return Err("Invalid list operation".into()),
                            }
                            Ok(())
                        });
                        if let Err(error) = result {
                            ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                            return false;
                        }
                    }
                    _ if id.name.starts_with("Rules Nested Table ") => {
                        let Some(rest) = id.name.strip_prefix("Rules Nested Table ") else {
                            return false;
                        };
                        let Some((action, suffix)) = rest.split_once(' ') else {
                            return false;
                        };
                        let Some(path) = suffix
                            .parse::<usize>()
                            .ok()
                            .and_then(|i| self.nested_tables.get(i))
                            .cloned()
                        else {
                            return false;
                        };
                        let name = ui
                            .get_widget(&format!("Rules Nested Name {suffix}"))
                            .and_then(|w| w.value().to_string())
                            .unwrap_or_default();
                        let name = name.trim();
                        if name.is_empty() {
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                "Enter a field name".into(),
                            ));
                            return false;
                        }
                        let kind = ui
                            .get_drop_down_menu(&format!("Rules Nested Kind {suffix}"))
                            .map(|w| w.selected_index())
                            .unwrap_or(0);
                        let value = match kind {
                            1 => TomlValue::Integer(0),
                            2 => TomlValue::Float(0.0),
                            3 => TomlValue::Boolean(false),
                            4 => TomlValue::Array(Vec::new()),
                            5 => TomlValue::Table(Table::new()),
                            _ => TomlValue::String(String::new()),
                        };
                        let result = project.rules_document.edit(|table| {
                            let target = nested_mut(table, &path)
                                .and_then(TomlValue::as_table_mut)
                                .ok_or("Missing table")?;
                            if action == "Remove" {
                                target.remove(name).ok_or("Field does not exist")?;
                            } else if action == "Add" {
                                if target.contains_key(name) {
                                    return Err("Field already exists".into());
                                }
                                target.insert(name.into(), value);
                            } else {
                                return Err("Invalid field operation".into());
                            }
                            Ok(())
                        });
                        if let Err(error) = result {
                            ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                            return false;
                        }
                    }
                    "Rules New" => {
                        let parents = self
                            .selected
                            .as_ref()
                            .map(|p| p[..p.len().saturating_sub(1)].to_vec())
                            .filter(|p| !p.is_empty())
                            .unwrap_or_else(|| vec!["actions".into()]);
                        let mut new_path = parents.clone();
                        let result = project.rules_document.edit(|table| {
                            let mut parent = table;
                            for key in &parents {
                                parent = parent
                                    .get_mut(key)
                                    .and_then(TomlValue::as_table_mut)
                                    .ok_or("Select a category first")?;
                            }
                            let mut name = "new_definition".to_string();
                            let mut index = 1;
                            while parent.contains_key(&name) {
                                index += 1;
                                name = format!("new_definition{index}");
                            }
                            parent.insert(name.clone(), TomlValue::Table(Table::new()));
                            new_path.push(name);
                            Ok(())
                        });
                        if let Err(error) = result {
                            ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                            return false;
                        }
                        self.selected = Some(new_path);
                    }
                    "Rules Rename Definition" => {
                        let Some(path) = self.selected.clone() else {
                            return false;
                        };
                        let name = ui
                            .get_widget("Rules Definition ID")
                            .and_then(|w| w.value().to_string())
                            .unwrap_or_default();
                        let name = name.trim();
                        if name.is_empty() {
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                "Enter an identifier".into(),
                            ));
                            return false;
                        }
                        let result = project.rules_document.edit(|table| {
                            let (old, parents) = path.split_last().ok_or("No selection")?;
                            let mut parent = table;
                            for key in parents {
                                parent = parent
                                    .get_mut(key)
                                    .and_then(TomlValue::as_table_mut)
                                    .ok_or("Missing definition")?;
                            }
                            if old == name {
                                return Ok(());
                            }
                            if parent.contains_key(name) {
                                return Err("Identifier already exists".into());
                            }
                            let value = parent.remove(old).ok_or("Missing definition")?;
                            parent.insert(name.into(), value);
                            Ok(())
                        });
                        if let Err(error) = result {
                            ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                            return false;
                        }
                        let mut new_path = path;
                        *new_path.last_mut().unwrap() = name.into();
                        self.selected = Some(new_path);
                    }
                    "Rules Duplicate" | "Rules Delete" => {
                        let Some(path) = self.selected.clone() else {
                            return false;
                        };
                        let deleting = id.name == "Rules Delete";
                        project.rules_document.checkpoint();
                        let mut new_path = path.clone();
                        let result = project.rules_document.edit(|table| {
                            let (key, parents) = path.split_last().ok_or("No selection")?;
                            let mut parent = table;
                            for part in parents {
                                parent = parent
                                    .get_mut(part)
                                    .and_then(TomlValue::as_table_mut)
                                    .ok_or("Missing definition")?;
                            }
                            if deleting {
                                parent.remove(key).ok_or("Missing definition")?;
                            } else {
                                let value = parent.get(key).ok_or("Missing definition")?.clone();
                                let mut index = 1;
                                let mut new_key = format!("{key}_copy");
                                while parent.contains_key(&new_key) {
                                    index += 1;
                                    new_key = format!("{key}_copy{index}");
                                }
                                parent.insert(new_key.clone(), value);
                                *new_path.last_mut().unwrap() = new_key;
                            }
                            Ok(())
                        });
                        if let Err(error) = result {
                            ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                            return false;
                        }
                        self.selected = if deleting { None } else { Some(new_path) };
                    }
                    "Rules Add Field" | "Rules Remove Field" => {
                        let Some(path) = self.selected.clone() else {
                            return false;
                        };
                        let name = ui
                            .get_widget("Rules New Field Name")
                            .and_then(|w| w.value().to_string())
                            .unwrap_or_default();
                        let name = name.trim();
                        if name.is_empty() {
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                "Enter a field name".into(),
                            ));
                            return false;
                        }
                        let kind = ui
                            .get_drop_down_menu("Rules New Field Type")
                            .map(|w| w.selected_index())
                            .unwrap_or(0);
                        let value = match kind {
                            1 => TomlValue::Integer(0),
                            2 => TomlValue::Float(0.0),
                            3 => TomlValue::Boolean(false),
                            4 => TomlValue::Array(Vec::new()),
                            5 => TomlValue::Table(Table::new()),
                            _ => TomlValue::String(String::new()),
                        };
                        let result = project.rules_document.edit(|table| {
                            let mut target = lookup_mut(table, &path)
                                .and_then(TomlValue::as_table_mut)
                                .ok_or("Select a definition")?;
                            let parts = name.split('.').collect::<Vec<_>>();
                            let (key, parents) = parts.split_last().ok_or("Enter a field name")?;
                            if key.is_empty() {
                                return Err("Enter a field name".into());
                            }
                            for parent in parents {
                                target = target
                                    .get_mut(*parent)
                                    .and_then(TomlValue::as_table_mut)
                                    .ok_or("Parent table does not exist")?;
                            }
                            if id.name == "Rules Remove Field" {
                                target.remove(*key).ok_or("Field does not exist")?;
                            } else {
                                if target.contains_key(*key) {
                                    return Err("Field already exists".into());
                                }
                                target.insert((*key).into(), value);
                            }
                            Ok(())
                        });
                        if let Err(error) = result {
                            ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                            return false;
                        }
                    }
                    "Rules Save Checkpoint" => project.rules_document.checkpoint(),
                    "Rules Restore" => project.rules_document.restore_original(),
                    "Rules Recover" => {
                        project.rules_document.recover();
                    }
                    _ if id.name.starts_with("Rules Record Field ") => {
                        let Some(suffix) = id.name.strip_prefix("Rules Record Field ") else {
                            return false;
                        };
                        let Some((path, Some(index))) = suffix
                            .parse::<usize>()
                            .ok()
                            .and_then(|i| self.lists.get(i))
                            .cloned()
                        else {
                            return false;
                        };
                        let name = ui
                            .get_widget(&format!("Rules Record Name {suffix}"))
                            .and_then(|w| w.value().to_string())
                            .unwrap_or_default();
                        let name = name.trim();
                        if name.is_empty() {
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                "Enter an item field name".into(),
                            ));
                            return false;
                        }
                        let kind = ui
                            .get_drop_down_menu(&format!("Rules Record Type {suffix}"))
                            .map(|w| w.selected_index())
                            .unwrap_or(0);
                        let value = match kind {
                            1 => TomlValue::Integer(0),
                            2 => TomlValue::Float(0.0),
                            3 => TomlValue::Boolean(false),
                            4 => TomlValue::Array(Vec::new()),
                            5 => TomlValue::Table(Table::new()),
                            _ => TomlValue::String(String::new()),
                        };
                        let result = project.rules_document.edit(|table| {
                            let record = lookup_mut(table, &path)
                                .and_then(TomlValue::as_array_mut)
                                .and_then(|a| a.get_mut(index))
                                .and_then(TomlValue::as_table_mut)
                                .ok_or("Missing list item")?;
                            if record.contains_key(name) {
                                return Err("Field already exists".into());
                            }
                            record.insert(name.into(), value);
                            Ok(())
                        });
                        if let Err(error) = result {
                            ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                            return false;
                        }
                    }
                    _ if id.name.starts_with("Rules List Add ")
                        || id.name.starts_with("Rules List Remove ")
                        || id.name.starts_with("Rules List Up ")
                        || id.name.starts_with("Rules List Down ") =>
                    {
                        let prefix = if id.name.starts_with("Rules List Add ") {
                            "Rules List Add "
                        } else if id.name.starts_with("Rules List Up ") {
                            "Rules List Up "
                        } else if id.name.starts_with("Rules List Down ") {
                            "Rules List Down "
                        } else {
                            "Rules List Remove "
                        };
                        let Some((path, index)) = id
                            .name
                            .strip_prefix(prefix)
                            .and_then(|s| s.parse::<usize>().ok())
                            .and_then(|i| self.lists.get(i))
                            .cloned()
                        else {
                            return false;
                        };
                        let selected_type = id
                            .name
                            .strip_prefix(prefix)
                            .and_then(|suffix| {
                                ui.get_drop_down_menu(&format!("Rules List Type {suffix}"))
                            })
                            .map(|menu| menu.selected_index())
                            .unwrap_or(0);
                        let result = project.rules_document.edit(|table| {
                            let list = lookup_mut(table, &path)
                                .and_then(TomlValue::as_array_mut)
                                .ok_or("Missing list")?;
                            if let Some(index) = index {
                                if index >= list.len() {
                                    return Err("Missing item".into());
                                }
                                if prefix == "Rules List Up " {
                                    if index > 0 {
                                        list.swap(index, index - 1);
                                    }
                                } else if prefix == "Rules List Down " {
                                    if index + 1 < list.len() {
                                        list.swap(index, index + 1);
                                    }
                                } else {
                                    list.remove(index);
                                }
                            } else {
                                let initial = match selected_type {
                                    1 => TomlValue::Integer(0),
                                    2 => TomlValue::Float(0.0),
                                    3 => TomlValue::Boolean(false),
                                    4 => TomlValue::Table(Table::new()),
                                    5 => TomlValue::Array(Vec::new()),
                                    _ => TomlValue::String(String::new()),
                                };
                                list.push(list.last().cloned().unwrap_or(initial));
                            }
                            Ok(())
                        });
                        if let Err(error) = result {
                            ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                            return false;
                        }
                    }
                    _ => return false,
                }
                self.catalogue(ui, project);
                self.form(ui, project);
                if project.rules_document != before {
                    self.undo.push(before);
                    self.redo.clear();
                    self.set_undo_state_to_ui(ctx);
                }
                self.dirty = true;
                if let Err(error) = crate::docks::nodes::sync_live_rules(project) {
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                }
                return true;
            }
            _ => {}
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_edits_preserve_types_and_unrelated_fields() {
        let mut table: Table = "[action]\nname = 'Attack'\ncooldown = 1.0\ntags = ['one']\n"
            .parse()
            .unwrap();
        let path = vec!["action".into(), "cooldown".into()];
        let old = lookup(&table, &path).unwrap().clone();
        assert!(parse_field("bad", &old).is_err());
        assert!(parse_field("2", &old).is_err());
        replace(&mut table, &path, parse_field("2.5", &old).unwrap()).unwrap();
        assert_eq!(lookup(&table, &path).unwrap().as_float(), Some(2.5));
        assert_eq!(table["action"]["name"].as_str(), Some("Attack"));
        assert_eq!(table["action"]["tags"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn float_fields_show_short_readable_values_without_changing_type() {
        let value = TomlValue::Float(0.45_f32 as f64);
        assert_eq!(field_text(&value), "0.45");
        assert_eq!(field_text(&TomlValue::Float(1.0)), "1.0");
        assert!(matches!(
            parse_field(&field_text(&value), &value),
            Ok(TomlValue::Float(_))
        ));
    }

    #[test]
    fn filtered_catalogue_omits_empty_categories() {
        let mut dock = RulesEditor::new();
        let mut ctx = TheContext::new(1200, 650, 1.0);
        let mut ui = TheUI::new();
        ui.canvas = dock.setup(&mut ctx);
        let project = Project::new();
        dock.search = "fire_burst".into();
        dock.catalogue(&mut ui, &project);
        let root = ui.get_tree_layout(TREE).unwrap().get_root();
        assert_eq!(root.childs.len(), 1);
        assert_eq!(root.childs[0].id.name, "fx");
        assert_eq!(root.childs[0].childs.len(), 1);
    }

    #[test]
    fn activation_restores_search_widget_after_project_switch() {
        let mut dock = RulesEditor::new();
        let mut ctx = TheContext::new(1200, 650, 1.0);
        let mut ui = TheUI::new();
        ui.canvas = dock.setup(&mut ctx);
        ui.set_widget_value("Rules Search", &mut ctx, TheValue::Text("stale".into()));
        dock.reset_for_project_switch();
        dock.activate(
            &mut ui,
            &mut ctx,
            &Project::new(),
            &mut ServerContext::default(),
        );
        assert_eq!(
            ui.get_widget("Rules Search").unwrap().value().to_string(),
            Some(String::new())
        );
    }

    #[test]
    fn inspector_adds_fields_and_edits_list_entries() {
        let mut dock = RulesEditor::new();
        let mut ctx = TheContext::new(1200, 650, 1.0);
        let mut ui = TheUI::new();
        ui.canvas = dock.setup(&mut ctx);
        let mut project = Project::new();
        let mut server = ServerContext::default();
        dock.selected = Some(vec!["races".into(), "Skeleton".into()]);
        dock.form(&mut ui, &project);
        ui.set_widget_value(
            "Rules New Field Name",
            &mut ctx,
            TheValue::Text("custom_note".into()),
        );
        assert!(dock.handle_event(
            &TheEvent::StateChanged(TheId::named("Rules Add Field"), TheWidgetState::Clicked),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server
        ));
        let doc = project.rules_document.document().unwrap();
        assert_eq!(
            doc.table()["races"]["Skeleton"]["custom_note"].as_str(),
            Some("")
        );
        let list_index = dock
            .lists
            .iter()
            .position(|(path, index)| path.last().is_some_and(|s| s == "traits") && index.is_none())
            .unwrap();
        assert!(dock.handle_event(
            &TheEvent::StateChanged(
                TheId::named(&format!("Rules List Add {list_index}")),
                TheWidgetState::Clicked
            ),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server
        ));
        assert_eq!(
            project.rules_document.document().unwrap().table()["races"]["Skeleton"]["traits"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        dock.undo(&mut ui, &mut ctx, &mut project, &mut server);
        assert_eq!(
            project.rules_document.document().unwrap().table()["races"]["Skeleton"]["traits"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn new_definition_can_be_renamed_and_saved() {
        let mut dock = RulesEditor::new();
        let mut ctx = TheContext::new(1200, 650, 1.0);
        let mut ui = TheUI::new();
        ui.canvas = dock.setup(&mut ctx);
        let mut project = Project::new();
        let mut server = ServerContext::default();
        dock.selected = Some(vec!["races".into(), "Skeleton".into()]);
        assert!(dock.handle_event(
            &TheEvent::StateChanged(TheId::named("Rules New"), TheWidgetState::Clicked),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server
        ));
        ui.set_widget_value(
            "Rules Definition ID",
            &mut ctx,
            TheValue::Text("Robot".into()),
        );
        assert!(dock.handle_event(
            &TheEvent::StateChanged(
                TheId::named("Rules Rename Definition"),
                TheWidgetState::Clicked
            ),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server
        ));
        let saved: Project =
            serde_json::from_str(&serde_json::to_string(&project).unwrap()).unwrap();
        assert!(
            saved.rules_document.document().unwrap().table()["races"]
                .as_table()
                .unwrap()
                .contains_key("Robot")
        );
    }

    #[test]
    fn nested_lists_and_records_edit_reorder_and_survive_reload() {
        let mut dock = RulesEditor::new();
        let mut ctx = TheContext::new(1200, 650, 1.0);
        let mut ui = TheUI::new();
        ui.canvas = dock.setup(&mut ctx);
        let mut project = Project::new();
        let mut server = ServerContext::default();
        let source: Table =
            "v = [{ matrix = [[1, 2], [3, 4]], metadata = { flags = [true, false] } }]"
                .parse()
                .unwrap();
        project
            .rules_document
            .edit(|table| {
                table["actions"]["basic_attack"]
                    .as_table_mut()
                    .unwrap()
                    .insert("custom".into(), source["v"].clone());
                Ok(())
            })
            .unwrap();
        dock.selected = Some(vec!["actions".into(), "basic_attack".into()]);
        dock.form(&mut ui, &project);
        let path = vec![
            PathPart::Key("actions".into()),
            PathPart::Key("basic_attack".into()),
            PathPart::Key("custom".into()),
            PathPart::Index(0),
            PathPart::Key("matrix".into()),
            PathPart::Index(0),
            PathPart::Index(1),
        ];
        let field = dock
            .nested_fields
            .iter()
            .position(|(p, _)| p == &path)
            .unwrap();
        assert!(dock.handle_event(
            &TheEvent::ValueChanged(
                TheId::named(&format!("Rules Nested Value {field}")),
                TheValue::Text("9".into())
            ),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server
        ));
        let list_path = path[..path.len() - 1].to_vec();
        let list = dock
            .nested_lists
            .iter()
            .position(|(p, i)| p == &list_path && *i == Some(1))
            .unwrap();
        assert!(dock.handle_event(
            &TheEvent::StateChanged(
                TheId::named(&format!("Rules Nested List Up {list}")),
                TheWidgetState::Clicked
            ),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server
        ));
        let restored: Project =
            serde_json::from_str(&serde_json::to_string(&project).unwrap()).unwrap();
        assert_eq!(
            restored.rules_document.document().unwrap().table()["actions"]["basic_attack"]
                ["custom"][0]["matrix"][0][0]
                .as_integer(),
            Some(9)
        );
        dock.undo(&mut ui, &mut ctx, &mut project, &mut server);
        assert_eq!(
            project.rules_document.document().unwrap().table()["actions"]["basic_attack"]["custom"]
                [0]["matrix"][0][1]
                .as_integer(),
            Some(9)
        );
    }

    #[test]
    fn recipe_ingredient_and_class_unlock_choices_update_only_the_selected_item() {
        let mut dock = RulesEditor::new();
        let mut ctx = TheContext::new(1200, 650, 1.0);
        let mut ui = TheUI::new();
        ui.canvas = dock.setup(&mut ctx);
        let mut project = Project::new();
        let mut server = ServerContext::default();
        dock.selected = Some(vec!["recipes".into(), "wooden_arrows".into()]);
        dock.form(&mut ui, &project);
        let field = dock
            .fields
            .iter()
            .position(|(p, _, i, key)| {
                p.last().is_some_and(|s| s == "consumes")
                    && *i == Some(0)
                    && key.as_deref() == Some("item")
            })
            .unwrap();
        let choice = dock.choices[&field]
            .iter()
            .position(|s| s == "feather")
            .unwrap();
        assert!(dock.handle_event(
            &TheEvent::IndexChanged(TheId::named(&format!("Rules Choice {field}")), choice),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server
        ));
        let doc = project.rules_document.document().unwrap();
        assert_eq!(
            doc.table()["recipes"]["wooden_arrows"]["consumes"][0]["item"].as_str(),
            Some("feather")
        );
        assert_eq!(
            doc.table()["recipes"]["wooden_arrows"]["consumes"][0]["quantity"].as_integer(),
            Some(1)
        );
        dock.selected = Some(vec!["classes".into(), "Warrior".into()]);
        dock.form(&mut ui, &project);
        let field = dock
            .fields
            .iter()
            .position(|(p, _, i, _)| {
                p.iter().any(|s| s == "level_1")
                    && p.last().is_some_and(|s| s == "abilities")
                    && *i == Some(0)
            })
            .unwrap();
        assert!(dock.choices[&field].iter().any(|s| s == "power_strike"));
    }

    #[test]
    fn particle_preset_inspector_provides_preview() {
        let mut dock = RulesEditor::new();
        let mut ctx = TheContext::new(1200, 650, 1.0);
        let mut ui = TheUI::new();
        ui.canvas = dock.setup(&mut ctx);
        let project = Project::new();
        dock.selected = Some(vec!["fx".into(), "presets".into(), "hit_burst".into()]);
        dock.form(&mut ui, &project);
        assert!(ui.get_icon_view("Rules Particle Preview").is_some());
    }

    #[test]
    fn inspector_edits_list_record_field_without_toml_syntax() {
        let mut dock = RulesEditor::new();
        let mut ctx = TheContext::new(1200, 650, 1.0);
        let mut ui = TheUI::new();
        ui.canvas = dock.setup(&mut ctx);
        let mut project = Project::new();
        let mut server = ServerContext::default();
        dock.selected = Some(vec!["actions".into(), "blessing".into()]);
        dock.form(&mut ui, &project);
        let index = dock
            .fields
            .iter()
            .position(|(path, _, list, key)| {
                path.last().is_some_and(|s| s == "consumes")
                    && *list == Some(0)
                    && key.as_deref() == Some("quantity")
            })
            .unwrap();
        assert!(dock.handle_event(
            &TheEvent::ValueChanged(
                TheId::named(&format!("{FIELD}{index}")),
                TheValue::Text("2".into())
            ),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server
        ));
        let doc = project.rules_document.document().unwrap();
        assert_eq!(
            doc.table()["actions"]["blessing"]["consumes"][0]["quantity"].as_integer(),
            Some(2)
        );
        assert_eq!(
            doc.table()["actions"]["blessing"]["consumes"][0]["item"].as_str(),
            Some("consecrated_oil")
        );
    }

    #[test]
    fn action_choices_and_icon_preview_follow_selected_definition() {
        let mut dock = RulesEditor::new();
        let mut ctx = TheContext::new(1200, 650, 1.0);
        let mut ui = TheUI::new();
        ui.canvas = dock.setup(&mut ctx);
        let project = Project::new();
        dock.selected = Some(vec!["actions".into(), "basic_attack".into()]);
        dock.form(&mut ui, &project);
        let index = dock
            .fields
            .iter()
            .position(|(path, _, _, _)| path.last().is_some_and(|s| s == "target"))
            .unwrap();
        assert!(dock.choices[&index].iter().any(|s| s == "friendly_or_self"));
        assert!(
            dock.fields
                .iter()
                .any(|(path, _, _, _)| path.last().is_some_and(|s| s == "icon"))
        );
        assert!(dock.icons.iter().any(|(id, _)| id == "basic_attack"));
    }

    #[test]
    fn visual_icon_selection_updates_document_and_undo() {
        let mut dock = RulesEditor::new();
        let mut ctx = TheContext::new(1200, 650, 1.0);
        let mut ui = TheUI::new();
        ui.canvas = dock.setup(&mut ctx);
        let mut project = Project::new();
        let mut server = ServerContext::default();
        dock.selected = Some(vec!["actions".into(), "basic_attack".into()]);
        dock.form(&mut ui, &project);
        let field = dock
            .fields
            .iter()
            .position(|(path, _, _, _)| path.last().is_some_and(|s| s == "icon"))
            .unwrap();
        let icon = dock
            .icons
            .iter()
            .position(|(id, _)| id == "minor_heal")
            .unwrap();
        assert!(dock.handle_event(
            &TheEvent::IndexChanged(TheId::named(&format!("Rules Icon Grid {field}")), icon),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server
        ));
        assert_eq!(
            project.rules_document.document().unwrap().table()["actions"]["basic_attack"]["ui"]
                ["icon"]
                .as_str(),
            Some("minor_heal")
        );
        dock.undo(&mut ui, &mut ctx, &mut project, &mut server);
        assert_eq!(
            project.rules_document.document().unwrap().table()["actions"]["basic_attack"]["ui"]
                ["icon"]
                .as_str(),
            Some("basic_attack")
        );
    }

    #[test]
    fn inspector_renders_without_clipping_toolbar() {
        let mut dock = RulesEditor::new();
        let mut ctx = TheContext::new(1200, 700, 1.0);
        let mut ui = TheUI::new();
        ui.init(&mut ctx);
        if std::env::var_os("ELDIRON_RULES_EDITOR_LIGHT").is_some() {
            ui.set_theme(Box::new(TheLightTheme::new()), &mut ctx);
        }
        ui.canvas = dock.setup(&mut ctx);
        let project = Project::new();
        dock.selected = Some(vec!["actions".into(), "basic_attack".into()]);
        dock.catalogue(&mut ui, &project);
        dock.form(&mut ui, &project);
        let mut pixels = vec![0; 1200 * 700 * 4];
        ui.draw(&mut pixels, &mut ctx);
        for id in [
            "Rules New",
            "Rules Duplicate",
            "Rules Delete",
            "Rules Save Checkpoint",
            "Rules Restore",
            "Rules Recover",
        ] {
            let dim = *ui.get_widget(id).unwrap().dim();
            assert!(
                dim.width > 30 && dim.height > 10,
                "{id} is clipped: {dim:?}"
            );
        }
        if let Ok(path) = std::env::var("ELDIRON_RULES_EDITOR_SNAPSHOT") {
            std::fs::write(
                path,
                TheRGBABuffer::from(pixels, 1200, 700).to_png().unwrap(),
            )
            .unwrap();
        }
    }
}
