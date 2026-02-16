pub use crate::prelude::*;

pub mod add_arch;
pub mod apply_tile;
pub mod clear_profile;
pub mod clear_tile;
pub mod create_center_vertex;
pub mod create_linedef;
pub mod create_sector;
pub mod edit_linedef;
pub mod edit_maximize;
pub mod edit_sector;
pub mod edit_vertex;
pub mod extrude_linedef;
pub mod extrude_sector;
pub mod set_tile_material;
// pub mod gen_stone_trim;
pub mod clear_palette;
pub mod copy_tile_id;
pub mod duplicate_tile;
pub mod edit_tile_meta;
pub mod editing_camera;
pub mod editing_slice;
pub mod export_vcode;
pub mod firstp_camera;
pub mod gate_door;
pub mod import_palette;
pub mod import_vcode;
pub mod iso_camera;
pub mod minimize;
pub mod new_tile;
pub mod orbit_camera;
pub mod recess;
pub mod relief;
pub mod remap_tile;
pub mod set_editing_surface;
pub mod split;
pub mod toggle_editing_geo;
pub mod toggle_rect_geo;

#[derive(PartialEq)]
pub enum ActionRole {
    Camera,
    Editor,
    Dock,
}

impl ActionRole {
    pub fn to_color(&self) -> [u8; 4] {
        match self {
            ActionRole::Camera => [160, 175, 190, 255],
            ActionRole::Editor => [195, 170, 150, 255],
            ActionRole::Dock => [200, 195, 150, 255],
            // ActionRole::Profile => [160, 185, 160, 255],
        }
    }
}

#[allow(unused)]
pub trait Action: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn id(&self) -> TheId;
    fn info(&self) -> String;
    fn role(&self) -> ActionRole;

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, map: &Map, ctx: &mut TheContext, server_ctx: &ServerContext) -> bool;

    fn load_params(&mut self, map: &Map) {}
    fn load_params_project(&mut self, project: &Project, server_ctx: &mut ServerContext) {}

    fn apply(
        &self,
        map: &mut Map,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        None
    }

    fn apply_project(
        &self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
    }

    fn params(&self) -> TheNodeUI;

    fn handle_event(
        &mut self,
        event: &TheEvent,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> bool;
}

fn normalize_toml_key(key: &str) -> String {
    let mut out = String::new();
    let mut prev_is_sep = false;

    for (i, ch) in key.chars().enumerate() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() {
                if i > 0 && !prev_is_sep {
                    out.push('_');
                }
                out.push(ch.to_ascii_lowercase());
            } else {
                out.push(ch.to_ascii_lowercase());
            }
            prev_is_sep = false;
        } else if !prev_is_sep && !out.is_empty() {
            out.push('_');
            prev_is_sep = true;
        }
    }

    out.trim_matches('_').to_string()
}

fn action_param_key(id: &str) -> String {
    let key = normalize_toml_key(id);
    key.strip_prefix("action_").unwrap_or(&key).to_string()
}

fn round_f64_3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

fn root_table_prefix(nodeui: &TheNodeUI) -> Option<String> {
    let mut section_stack: Vec<String> = vec![];
    let mut prefix: Option<String> = None;
    let mut saw = false;

    for (_, item) in nodeui.list_items() {
        match item {
            TheNodeUIItem::OpenTree(name) => section_stack.push(normalize_toml_key(name)),
            TheNodeUIItem::CloseTree => {
                section_stack.pop();
            }
            TheNodeUIItem::Text(id, _, _, _, _, _)
            | TheNodeUIItem::Selector(id, _, _, _, _)
            | TheNodeUIItem::FloatEditSlider(id, _, _, _, _, _)
            | TheNodeUIItem::FloatSlider(id, _, _, _, _, _, _)
            | TheNodeUIItem::IntEditSlider(id, _, _, _, _, _)
            | TheNodeUIItem::PaletteSlider(id, _, _, _, _, _)
            | TheNodeUIItem::IntSlider(id, _, _, _, _, _, _)
            | TheNodeUIItem::ColorPicker(id, _, _, _, _)
            | TheNodeUIItem::Checkbox(id, _, _, _) => {
                if section_stack.is_empty() {
                    let key = action_param_key(id);
                    if let Some((p, _)) = key.split_once('_') {
                        let p = p.to_string();
                        match &prefix {
                            None => prefix = Some(p),
                            Some(curr) if curr == &p => {}
                            Some(_) => return None,
                        }
                        saw = true;
                    } else {
                        return None;
                    }
                }
            }
            TheNodeUIItem::Button(_, _, _, _)
            | TheNodeUIItem::Markdown(_, _)
            | TheNodeUIItem::Separator(_)
            | TheNodeUIItem::Icons(_, _, _, _) => {}
        }
    }

    if saw { prefix } else { None }
}

fn display_key_for_storage(
    action_key: &str,
    section_stack: &[String],
    root_prefix: Option<&str>,
) -> String {
    if let Some(section) = section_stack.last() {
        let needle = section.clone() + "_";
        if let Some(pos) = action_key.find(&needle) {
            let start = pos + needle.len();
            if start < action_key.len() {
                return action_key[start..].to_string();
            }
        }
        if let Some(stripped) = action_key.strip_prefix(&needle) {
            return stripped.to_string();
        }
        return action_key.to_string();
    }

    if let Some(prefix) = root_prefix {
        let needle = prefix.to_string() + "_";
        if let Some(stripped) = action_key.strip_prefix(&needle) {
            return stripped.to_string();
        }
    }

    action_key.to_string()
}

fn candidate_input_keys(
    action_key: &str,
    section_stack: &[String],
    root_prefix: Option<&str>,
) -> Vec<String> {
    let mut keys = vec![display_key_for_storage(
        action_key,
        section_stack,
        root_prefix,
    )];
    keys.push(action_key.to_string());

    if let Some(section) = section_stack.last() {
        let needle = section.clone() + "_";
        if let Some(stripped) = action_key.strip_prefix(&needle) {
            keys.push(stripped.to_string());
        }
        if let Some(pos) = action_key.find(&needle) {
            let start = pos + needle.len();
            if start < action_key.len() {
                keys.push(action_key[start..].to_string());
            }
        }
    }

    keys.sort();
    keys.dedup();
    keys
}

fn section_table<'a>(table: &'a toml::Table, path: &[String]) -> Option<&'a toml::Table> {
    if path.is_empty() {
        return Some(table);
    }

    let key = &path[0];
    let value = table.get(key)?;
    let sub = value.as_table()?;
    section_table(sub, &path[1..])
}

/// Converts action parameter UI to TOML.
/// OpenTree / CloseTree items define nested TOML sections.
pub fn nodeui_to_toml(nodeui: &TheNodeUI) -> String {
    fn upsert(entries: &mut Vec<(String, toml::Value)>, key: String, value: toml::Value) {
        if let Some((_, existing)) = entries.iter_mut().find(|(k, _)| *k == key) {
            *existing = value;
        } else {
            entries.push((key, value));
        }
    }

    fn section_entries_mut<'a>(
        sections: &'a mut Vec<(String, Vec<(String, toml::Value)>)>,
        name: &str,
    ) -> &'a mut Vec<(String, toml::Value)> {
        if let Some(index) = sections.iter().position(|(n, _)| n == name) {
            return &mut sections[index].1;
        }
        sections.push((name.to_string(), Vec::new()));
        let last = sections.len() - 1;
        &mut sections[last].1
    }

    let mut root_action_entries: Vec<(String, toml::Value)> = vec![];
    let mut sections: Vec<(String, Vec<(String, toml::Value)>)> = vec![];
    let mut section_stack: Vec<String> = vec![];
    let mut has_editable_values = false;
    let root_prefix = root_table_prefix(nodeui);

    for (_, item) in nodeui.list_items() {
        match item {
            TheNodeUIItem::OpenTree(name) => {
                section_stack.push(normalize_toml_key(name));
            }
            TheNodeUIItem::CloseTree => {
                section_stack.pop();
            }
            TheNodeUIItem::Text(id, _, _, value, _, _) => {
                let action_key = action_param_key(id);
                let key =
                    display_key_for_storage(&action_key, &section_stack, root_prefix.as_deref());
                let val = toml::Value::String(value.clone());
                if section_stack.is_empty() {
                    upsert(&mut root_action_entries, key, val);
                } else {
                    let section_name = section_stack.join(".");
                    let entries = section_entries_mut(&mut sections, &section_name);
                    upsert(entries, key, val);
                }
                has_editable_values = true;
            }
            TheNodeUIItem::Selector(id, _, _, values, index) => {
                let action_key = action_param_key(id);
                let key =
                    display_key_for_storage(&action_key, &section_stack, root_prefix.as_deref());
                let selected = if (*index as usize) < values.len() {
                    toml::Value::String(values[*index as usize].clone())
                } else {
                    toml::Value::Integer(*index as i64)
                };
                if section_stack.is_empty() {
                    upsert(&mut root_action_entries, key, selected);
                } else {
                    let section_name = section_stack.join(".");
                    let entries = section_entries_mut(&mut sections, &section_name);
                    upsert(entries, key, selected);
                }
                has_editable_values = true;
            }
            TheNodeUIItem::FloatEditSlider(id, _, _, value, _, _)
            | TheNodeUIItem::FloatSlider(id, _, _, value, _, _, _) => {
                let action_key = action_param_key(id);
                let key =
                    display_key_for_storage(&action_key, &section_stack, root_prefix.as_deref());
                let val = toml::Value::Float(round_f64_3(*value as f64));
                if section_stack.is_empty() {
                    upsert(&mut root_action_entries, key, val);
                } else {
                    let section_name = section_stack.join(".");
                    let entries = section_entries_mut(&mut sections, &section_name);
                    upsert(entries, key, val);
                }
                has_editable_values = true;
            }
            TheNodeUIItem::IntEditSlider(id, _, _, value, _, _)
            | TheNodeUIItem::PaletteSlider(id, _, _, value, _, _)
            | TheNodeUIItem::IntSlider(id, _, _, value, _, _, _) => {
                let action_key = action_param_key(id);
                let key =
                    display_key_for_storage(&action_key, &section_stack, root_prefix.as_deref());
                let val = toml::Value::Integer(*value as i64);
                if section_stack.is_empty() {
                    upsert(&mut root_action_entries, key, val);
                } else {
                    let section_name = section_stack.join(".");
                    let entries = section_entries_mut(&mut sections, &section_name);
                    upsert(entries, key, val);
                }
                has_editable_values = true;
            }
            TheNodeUIItem::ColorPicker(id, _, _, value, _) => {
                let action_key = action_param_key(id);
                let key =
                    display_key_for_storage(&action_key, &section_stack, root_prefix.as_deref());
                let val = toml::Value::String(value.to_hex());
                if section_stack.is_empty() {
                    upsert(&mut root_action_entries, key, val);
                } else {
                    let section_name = section_stack.join(".");
                    let entries = section_entries_mut(&mut sections, &section_name);
                    upsert(entries, key, val);
                }
                has_editable_values = true;
            }
            TheNodeUIItem::Checkbox(id, _, _, value) => {
                let action_key = action_param_key(id);
                let key =
                    display_key_for_storage(&action_key, &section_stack, root_prefix.as_deref());
                let val = toml::Value::Boolean(*value);
                if section_stack.is_empty() {
                    upsert(&mut root_action_entries, key, val);
                } else {
                    let section_name = section_stack.join(".");
                    let entries = section_entries_mut(&mut sections, &section_name);
                    upsert(entries, key, val);
                }
                has_editable_values = true;
            }
            TheNodeUIItem::Button(_, _, _, _)
            | TheNodeUIItem::Markdown(_, _)
            | TheNodeUIItem::Separator(_)
            | TheNodeUIItem::Icons(_, _, _, _) => {}
        }
    }

    if !has_editable_values {
        return String::new();
    }

    let mut out = String::new();

    if !root_action_entries.is_empty() {
        out.push_str("[action]\n");
        for (key, value) in &root_action_entries {
            out.push_str(&format!("{key} = {value}\n"));
        }
    }

    for (section, entries) in &sections {
        if entries.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!("[{section}]\n"));
        for (key, value) in entries {
            out.push_str(&format!("{key} = {value}\n"));
        }
    }

    out
}

/// Applies TOML values back to action parameter UI.
/// Unknown keys/sections are ignored.
pub fn apply_toml_to_nodeui(nodeui: &mut TheNodeUI, source: &str) -> Result<(), String> {
    let root: toml::Table = toml::from_str(source).map_err(|e| e.to_string())?;
    let action_root = root
        .get("action")
        .and_then(|v| v.as_table())
        .unwrap_or(&root);
    let mut section_stack: Vec<String> = vec![];
    let items: Vec<TheNodeUIItem> = nodeui.list_items().map(|(_, item)| item.clone()).collect();
    let root_prefix = root_table_prefix(nodeui);

    for item in items {
        match item {
            TheNodeUIItem::OpenTree(name) => {
                section_stack.push(normalize_toml_key(&name));
            }
            TheNodeUIItem::CloseTree => {
                section_stack.pop();
            }
            TheNodeUIItem::Text(id, _, _, _, _, _) => {
                let action_key = action_param_key(&id);
                let table = if section_stack.is_empty() {
                    Some(action_root)
                } else {
                    section_table(&root, &section_stack)
                        .or_else(|| section_table(action_root, &section_stack))
                };
                if let Some(table) = table {
                    for key in
                        candidate_input_keys(&action_key, &section_stack, root_prefix.as_deref())
                    {
                        if let Some(toml::Value::String(v)) = table.get(&key) {
                            nodeui.set_text_value(&id, v.clone());
                            break;
                        }
                    }
                }
            }
            TheNodeUIItem::Selector(id, _, _, values, _) => {
                let action_key = action_param_key(&id);
                let table = if section_stack.is_empty() {
                    Some(action_root)
                } else {
                    section_table(&root, &section_stack)
                        .or_else(|| section_table(action_root, &section_stack))
                };
                if let Some(table) = table {
                    for key in
                        candidate_input_keys(&action_key, &section_stack, root_prefix.as_deref())
                    {
                        if let Some(value) = table.get(&key) {
                            match value {
                                toml::Value::Integer(v) => nodeui.set_i32_value(&id, *v as i32),
                                toml::Value::String(name) => {
                                    if let Some(index) = values.iter().position(|v| v == name) {
                                        nodeui.set_i32_value(&id, index as i32);
                                    }
                                }
                                _ => {}
                            }
                            break;
                        }
                    }
                }
            }
            TheNodeUIItem::FloatEditSlider(id, _, _, _, _, _)
            | TheNodeUIItem::FloatSlider(id, _, _, _, _, _, _) => {
                let action_key = action_param_key(&id);
                let table = if section_stack.is_empty() {
                    Some(action_root)
                } else {
                    section_table(&root, &section_stack)
                        .or_else(|| section_table(action_root, &section_stack))
                };
                if let Some(table) = table {
                    for key in
                        candidate_input_keys(&action_key, &section_stack, root_prefix.as_deref())
                    {
                        if let Some(value) = table.get(&key) {
                            match value {
                                toml::Value::Float(v) => nodeui.set_f32_value(&id, *v as f32),
                                toml::Value::Integer(v) => nodeui.set_f32_value(&id, *v as f32),
                                _ => {}
                            }
                            break;
                        }
                    }
                }
            }
            TheNodeUIItem::IntEditSlider(id, _, _, _, _, _)
            | TheNodeUIItem::PaletteSlider(id, _, _, _, _, _)
            | TheNodeUIItem::IntSlider(id, _, _, _, _, _, _) => {
                let action_key = action_param_key(&id);
                let table = if section_stack.is_empty() {
                    Some(action_root)
                } else {
                    section_table(&root, &section_stack)
                        .or_else(|| section_table(action_root, &section_stack))
                };
                if let Some(table) = table {
                    for key in
                        candidate_input_keys(&action_key, &section_stack, root_prefix.as_deref())
                    {
                        if let Some(value) = table.get(&key) {
                            if let toml::Value::Integer(v) = value {
                                nodeui.set_i32_value(&id, *v as i32);
                            }
                            break;
                        }
                    }
                }
            }
            TheNodeUIItem::ColorPicker(id, _, _, _, _) => {
                let action_key = action_param_key(&id);
                let table = if section_stack.is_empty() {
                    Some(action_root)
                } else {
                    section_table(&root, &section_stack)
                        .or_else(|| section_table(action_root, &section_stack))
                };
                if let Some(table) = table {
                    for key in
                        candidate_input_keys(&action_key, &section_stack, root_prefix.as_deref())
                    {
                        if let Some(toml::Value::String(v)) = table.get(&key) {
                            if let Some(TheNodeUIItem::ColorPicker(_, _, _, color, _)) =
                                nodeui.get_item_mut(&id)
                            {
                                *color = TheColor::from_hex(v);
                            }
                            break;
                        }
                    }
                }
            }
            TheNodeUIItem::Checkbox(id, _, _, _) => {
                let action_key = action_param_key(&id);
                let table = if section_stack.is_empty() {
                    Some(action_root)
                } else {
                    section_table(&root, &section_stack)
                        .or_else(|| section_table(action_root, &section_stack))
                };
                if let Some(table) = table {
                    for key in
                        candidate_input_keys(&action_key, &section_stack, root_prefix.as_deref())
                    {
                        if let Some(toml::Value::Boolean(v)) = table.get(&key) {
                            nodeui.set_bool_value(&id, *v);
                            break;
                        }
                    }
                }
            }
            TheNodeUIItem::Button(_, _, _, _)
            | TheNodeUIItem::Markdown(_, _)
            | TheNodeUIItem::Separator(_)
            | TheNodeUIItem::Icons(_, _, _, _) => {}
        }
    }

    Ok(())
}

/// Converts current NodeUI values into `(id, TheValue)` pairs suitable for
/// replay through `Action::handle_event` using `TheEvent::ValueChanged`.
pub fn nodeui_to_value_pairs(nodeui: &TheNodeUI) -> Vec<(String, TheValue)> {
    let mut out: Vec<(String, TheValue)> = Vec::new();
    for (_, item) in nodeui.list_items() {
        match item {
            TheNodeUIItem::Text(id, _, _, value, _, _) => {
                out.push((id.clone(), TheValue::Text(value.clone())));
            }
            TheNodeUIItem::Selector(id, _, _, _, value) => {
                out.push((id.clone(), TheValue::Int(*value)));
            }
            TheNodeUIItem::FloatEditSlider(id, _, _, value, _, _)
            | TheNodeUIItem::FloatSlider(id, _, _, value, _, _, _) => {
                out.push((id.clone(), TheValue::Float(*value)));
            }
            TheNodeUIItem::IntEditSlider(id, _, _, value, _, _)
            | TheNodeUIItem::PaletteSlider(id, _, _, value, _, _)
            | TheNodeUIItem::IntSlider(id, _, _, value, _, _, _) => {
                out.push((id.clone(), TheValue::Int(*value)));
            }
            TheNodeUIItem::ColorPicker(id, _, _, value, _) => {
                out.push((id.clone(), TheValue::ColorObject(value.clone())));
            }
            TheNodeUIItem::Checkbox(id, _, _, value) => {
                out.push((id.clone(), TheValue::Bool(*value)));
            }
            TheNodeUIItem::Button(_, _, _, _)
            | TheNodeUIItem::Markdown(_, _)
            | TheNodeUIItem::Separator(_)
            | TheNodeUIItem::Icons(_, _, _, _)
            | TheNodeUIItem::OpenTree(_)
            | TheNodeUIItem::CloseTree => {}
        }
    }
    out
}
