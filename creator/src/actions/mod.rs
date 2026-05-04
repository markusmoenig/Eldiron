use crate::editor::RUSTERIX;
pub use crate::prelude::*;
use rusterix::{PixelSource, ValueContainer};

#[derive(Clone, Debug)]
pub struct ActionMaterialSlot {
    pub label: String,
    pub source: Option<PixelSource>,
}

#[derive(Clone, Debug)]
pub struct ActionItemSlot {
    pub label: String,
    pub assigned_builder_name: Option<String>,
}

pub fn parse_tile_id_pixelsource(text: &str) -> Option<PixelSource> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(id) = Uuid::parse_str(trimmed) {
        return Some(PixelSource::TileId(id));
    }
    {
        let rusterix = RUSTERIX.read().unwrap();
        if let Some((id, _tile)) = rusterix.assets.tiles.iter().find(|(_id, tile)| {
            tile.alias
                .split([',', ';', '\n'])
                .map(str::trim)
                .any(|part| !part.is_empty() && part.eq_ignore_ascii_case(trimmed))
        }) {
            return Some(PixelSource::TileId(*id));
        }
    }
    if let Ok(index) = trimmed.parse::<u16>() {
        return Some(PixelSource::PaletteIndex(index));
    }
    None
}

pub fn source_to_text(source: Option<&Value>) -> String {
    match source {
        Some(Value::Source(PixelSource::TileId(id))) => id.to_string(),
        Some(Value::Source(PixelSource::PaletteIndex(index))) => index.to_string(),
        _ => String::new(),
    }
}

pub fn builder_material_property_key(label: &str) -> String {
    format!("builder_material_{}", normalize_builder_slot_key(label))
}

pub fn builder_item_graph_data_property_key(label: &str) -> String {
    format!(
        "builder_item_{}_graph_data",
        normalize_builder_slot_key(label)
    )
}

pub fn builder_item_graph_id_property_key(label: &str) -> String {
    format!(
        "builder_item_{}_graph_id",
        normalize_builder_slot_key(label)
    )
}

pub fn builder_item_graph_name_property_key(label: &str) -> String {
    format!(
        "builder_item_{}_graph_name",
        normalize_builder_slot_key(label)
    )
}

pub fn current_selection_tool_type(map: &Map) -> MapToolType {
    if !map.selected_vertices.is_empty() {
        MapToolType::Vertex
    } else if !map.selected_linedefs.is_empty() {
        MapToolType::Linedef
    } else {
        MapToolType::Sector
    }
}

fn builder_material_slots_from_properties(
    properties: &ValueContainer,
) -> Option<Vec<ActionMaterialSlot>> {
    let graph_text = properties.get_str_default("builder_graph_data", String::new());
    if graph_text.trim().is_empty() {
        return None;
    }
    if properties
        .get_str_default("builder_graph_target", "sector".to_string())
        .is_empty()
    {
        return None;
    }
    let Ok(graph) = shared::buildergraph::BuilderDocument::from_text(&graph_text) else {
        return None;
    };
    let slot_names = graph.material_slot_names();
    if slot_names.is_empty() {
        return None;
    }
    Some(
        slot_names
            .into_iter()
            .map(|label| {
                let source = match properties.get(&builder_material_property_key(&label)) {
                    Some(Value::Source(source)) => Some(source.clone()),
                    _ => None,
                };
                ActionMaterialSlot { label, source }
            })
            .collect(),
    )
}

fn builder_item_slots_from_properties(properties: &ValueContainer) -> Option<Vec<ActionItemSlot>> {
    let graph_text = properties.get_str_default("builder_graph_data", String::new());
    if graph_text.trim().is_empty() {
        return None;
    }
    if properties
        .get_str_default("builder_graph_target", "sector".to_string())
        .is_empty()
    {
        return None;
    }
    let Ok(graph) = shared::buildergraph::BuilderDocument::from_text(&graph_text) else {
        return None;
    };
    let slot_names = graph.item_slot_names();
    if slot_names.is_empty() {
        return None;
    }
    Some(
        slot_names
            .into_iter()
            .map(|label| ActionItemSlot {
                assigned_builder_name: properties
                    .get_str(&builder_item_graph_name_property_key(&label))
                    .map(str::to_string),
                label,
            })
            .collect(),
    )
}

pub fn builder_hud_material_slots_for_selected_sector(
    map: &Map,
) -> Option<Vec<ActionMaterialSlot>> {
    let sector_id = *map.selected_sectors.first()?;
    let sector = map.find_sector(sector_id)?;
    builder_material_slots_from_properties(&sector.properties)
}

pub fn builder_hud_material_slots_for_selected_linedef(
    map: &Map,
) -> Option<Vec<ActionMaterialSlot>> {
    let linedef_id = *map.selected_linedefs.first()?;
    let linedef = map.find_linedef(linedef_id)?;
    builder_material_slots_from_properties(&linedef.properties)
}

pub fn builder_hud_material_slots_for_selected_vertex(
    map: &Map,
) -> Option<Vec<ActionMaterialSlot>> {
    let vertex_id = *map.selected_vertices.first()?;
    let vertex = map.find_vertex(vertex_id)?;
    builder_material_slots_from_properties(&vertex.properties)
}

pub fn builder_hud_item_slots_for_selected_sector(map: &Map) -> Option<Vec<ActionItemSlot>> {
    let sector_id = *map.selected_sectors.first()?;
    let sector = map.find_sector(sector_id)?;
    builder_item_slots_from_properties(&sector.properties)
}

pub fn builder_hud_item_slots_for_selected_linedef(map: &Map) -> Option<Vec<ActionItemSlot>> {
    let linedef_id = *map.selected_linedefs.first()?;
    let linedef = map.find_linedef(linedef_id)?;
    builder_item_slots_from_properties(&linedef.properties)
}

pub fn builder_hud_item_slots_for_selected_vertex(map: &Map) -> Option<Vec<ActionItemSlot>> {
    let vertex_id = *map.selected_vertices.first()?;
    let vertex = map.find_vertex(vertex_id)?;
    builder_item_slots_from_properties(&vertex.properties)
}

pub fn apply_builder_hud_material_to_selection(
    map: &mut Map,
    server_ctx: &ServerContext,
    slot_index: i32,
    source: Option<PixelSource>,
) -> bool {
    if slot_index < 0 {
        return false;
    }
    match server_ctx.curr_map_tool_type {
        MapToolType::Sector => {
            let Some(slot) = builder_hud_material_slots_for_selected_sector(map)
                .and_then(|slots| slots.get(slot_index as usize).cloned())
            else {
                return false;
            };
            let key = builder_material_property_key(&slot.label);
            let mut changed = false;
            for sector_id in map.selected_sectors.clone() {
                if let Some(sector) = map.find_sector_mut(sector_id) {
                    match &source {
                        Some(source) => {
                            let has_changed = match sector.properties.get(&key) {
                                Some(Value::Source(existing)) => existing != source,
                                _ => true,
                            };
                            if has_changed {
                                sector.properties.set(&key, Value::Source(source.clone()));
                                changed = true;
                            }
                        }
                        None => {
                            if sector.properties.contains(&key) {
                                sector.properties.remove(&key);
                                changed = true;
                            }
                        }
                    }
                }
            }
            changed
        }
        MapToolType::Linedef => {
            let Some(slot) = builder_hud_material_slots_for_selected_linedef(map)
                .and_then(|slots| slots.get(slot_index as usize).cloned())
            else {
                return false;
            };
            let key = builder_material_property_key(&slot.label);
            let mut changed = false;
            for linedef_id in map.selected_linedefs.clone() {
                if let Some(linedef) = map.find_linedef_mut(linedef_id) {
                    match &source {
                        Some(source) => {
                            let has_changed = match linedef.properties.get(&key) {
                                Some(Value::Source(existing)) => existing != source,
                                _ => true,
                            };
                            if has_changed {
                                linedef.properties.set(&key, Value::Source(source.clone()));
                                changed = true;
                            }
                        }
                        None => {
                            if linedef.properties.contains(&key) {
                                linedef.properties.remove(&key);
                                changed = true;
                            }
                        }
                    }
                }
            }
            changed
        }
        MapToolType::Vertex => {
            let Some(slot) = builder_hud_material_slots_for_selected_vertex(map)
                .and_then(|slots| slots.get(slot_index as usize).cloned())
            else {
                return false;
            };
            let key = builder_material_property_key(&slot.label);
            let mut changed = false;
            for vertex_id in map.selected_vertices.clone() {
                if let Some(vertex) = map.find_vertex_mut(vertex_id) {
                    match &source {
                        Some(source) => {
                            let has_changed = match vertex.properties.get(&key) {
                                Some(Value::Source(existing)) => existing != source,
                                _ => true,
                            };
                            if has_changed {
                                vertex.properties.set(&key, Value::Source(source.clone()));
                                changed = true;
                            }
                        }
                        None => {
                            if vertex.properties.contains(&key) {
                                vertex.properties.remove(&key);
                                changed = true;
                            }
                        }
                    }
                }
            }
            changed
        }
        _ => false,
    }
}

pub fn apply_builder_item_to_selection(
    map: &mut Map,
    server_ctx: &ServerContext,
    slot_index: i32,
    asset: &BuilderGraphAsset,
) -> bool {
    if slot_index < 0 {
        return false;
    }
    match server_ctx.curr_map_tool_type {
        MapToolType::Sector => {
            let Some(slot) = builder_hud_item_slots_for_selected_sector(map)
                .and_then(|slots| slots.get(slot_index as usize).cloned())
            else {
                return false;
            };
            let data_key = builder_item_graph_data_property_key(&slot.label);
            let id_key = builder_item_graph_id_property_key(&slot.label);
            let name_key = builder_item_graph_name_property_key(&slot.label);
            let mut changed = false;
            for sector_id in map.selected_sectors.clone() {
                if let Some(sector) = map.find_sector_mut(sector_id) {
                    let data_changed = sector
                        .properties
                        .get_str(&data_key)
                        .map(|v| v != asset.graph_data)
                        .unwrap_or(true);
                    let id_changed = sector
                        .properties
                        .get_id(&id_key)
                        .map(|v| v != asset.id)
                        .unwrap_or(true);
                    let name_changed = sector
                        .properties
                        .get_str(&name_key)
                        .map(|v| v != asset.graph_name)
                        .unwrap_or(true);
                    if data_changed || id_changed || name_changed {
                        sector
                            .properties
                            .set(&data_key, Value::Str(asset.graph_data.clone()));
                        sector.properties.set(&id_key, Value::Id(asset.id));
                        sector
                            .properties
                            .set(&name_key, Value::Str(asset.graph_name.clone()));
                        changed = true;
                    }
                }
            }
            changed
        }
        MapToolType::Linedef => {
            let Some(slot) = builder_hud_item_slots_for_selected_linedef(map)
                .and_then(|slots| slots.get(slot_index as usize).cloned())
            else {
                return false;
            };
            let data_key = builder_item_graph_data_property_key(&slot.label);
            let id_key = builder_item_graph_id_property_key(&slot.label);
            let name_key = builder_item_graph_name_property_key(&slot.label);
            let mut changed = false;
            for linedef_id in map.selected_linedefs.clone() {
                if let Some(linedef) = map.find_linedef_mut(linedef_id) {
                    let data_changed = linedef
                        .properties
                        .get_str(&data_key)
                        .map(|v| v != asset.graph_data)
                        .unwrap_or(true);
                    let id_changed = linedef
                        .properties
                        .get_id(&id_key)
                        .map(|v| v != asset.id)
                        .unwrap_or(true);
                    let name_changed = linedef
                        .properties
                        .get_str(&name_key)
                        .map(|v| v != asset.graph_name)
                        .unwrap_or(true);
                    if data_changed || id_changed || name_changed {
                        linedef
                            .properties
                            .set(&data_key, Value::Str(asset.graph_data.clone()));
                        linedef.properties.set(&id_key, Value::Id(asset.id));
                        linedef
                            .properties
                            .set(&name_key, Value::Str(asset.graph_name.clone()));
                        changed = true;
                    }
                }
            }
            changed
        }
        MapToolType::Vertex => {
            let Some(slot) = builder_hud_item_slots_for_selected_vertex(map)
                .and_then(|slots| slots.get(slot_index as usize).cloned())
            else {
                return false;
            };
            let data_key = builder_item_graph_data_property_key(&slot.label);
            let id_key = builder_item_graph_id_property_key(&slot.label);
            let name_key = builder_item_graph_name_property_key(&slot.label);
            let mut changed = false;
            for vertex_id in map.selected_vertices.clone() {
                if let Some(vertex) = map.find_vertex_mut(vertex_id) {
                    let data_changed = vertex
                        .properties
                        .get_str(&data_key)
                        .map(|v| v != asset.graph_data)
                        .unwrap_or(true);
                    let id_changed = vertex
                        .properties
                        .get_id(&id_key)
                        .map(|v| v != asset.id)
                        .unwrap_or(true);
                    let name_changed = vertex
                        .properties
                        .get_str(&name_key)
                        .map(|v| v != asset.graph_name)
                        .unwrap_or(true);
                    if data_changed || id_changed || name_changed {
                        vertex
                            .properties
                            .set(&data_key, Value::Str(asset.graph_data.clone()));
                        vertex.properties.set(&id_key, Value::Id(asset.id));
                        vertex
                            .properties
                            .set(&name_key, Value::Str(asset.graph_name.clone()));
                        changed = true;
                    }
                }
            }
            changed
        }
        _ => false,
    }
}

pub fn clear_builder_item_from_selection(
    map: &mut Map,
    server_ctx: &ServerContext,
    slot_index: i32,
) -> bool {
    if slot_index < 0 {
        return false;
    }
    match server_ctx.curr_map_tool_type {
        MapToolType::Sector => {
            let Some(slot) = builder_hud_item_slots_for_selected_sector(map)
                .and_then(|slots| slots.get(slot_index as usize).cloned())
            else {
                return false;
            };
            let keys = [
                builder_item_graph_data_property_key(&slot.label),
                builder_item_graph_id_property_key(&slot.label),
                builder_item_graph_name_property_key(&slot.label),
            ];
            let mut changed = false;
            for sector_id in map.selected_sectors.clone() {
                if let Some(sector) = map.find_sector_mut(sector_id) {
                    for key in &keys {
                        if sector.properties.contains(key) {
                            sector.properties.remove(key);
                            changed = true;
                        }
                    }
                }
            }
            changed
        }
        MapToolType::Linedef => {
            let Some(slot) = builder_hud_item_slots_for_selected_linedef(map)
                .and_then(|slots| slots.get(slot_index as usize).cloned())
            else {
                return false;
            };
            let keys = [
                builder_item_graph_data_property_key(&slot.label),
                builder_item_graph_id_property_key(&slot.label),
                builder_item_graph_name_property_key(&slot.label),
            ];
            let mut changed = false;
            for linedef_id in map.selected_linedefs.clone() {
                if let Some(linedef) = map.find_linedef_mut(linedef_id) {
                    for key in &keys {
                        if linedef.properties.contains(key) {
                            linedef.properties.remove(key);
                            changed = true;
                        }
                    }
                }
            }
            changed
        }
        MapToolType::Vertex => {
            let Some(slot) = builder_hud_item_slots_for_selected_vertex(map)
                .and_then(|slots| slots.get(slot_index as usize).cloned())
            else {
                return false;
            };
            let keys = [
                builder_item_graph_data_property_key(&slot.label),
                builder_item_graph_id_property_key(&slot.label),
                builder_item_graph_name_property_key(&slot.label),
            ];
            let mut changed = false;
            for vertex_id in map.selected_vertices.clone() {
                if let Some(vertex) = map.find_vertex_mut(vertex_id) {
                    for key in &keys {
                        if vertex.properties.contains(key) {
                            vertex.properties.remove(key);
                            changed = true;
                        }
                    }
                }
            }
            changed
        }
        _ => false,
    }
}

pub fn set_nodeui_icon_tile_id(
    nodeui: &mut TheNodeUI,
    item_name: &str,
    index: usize,
    tile_id: Uuid,
) {
    if let Some(TheNodeUIItem::Icons(_, _, _, items)) = nodeui.get_item_mut(item_name)
        && index < items.len()
    {
        items[index].2 = tile_id;
    }
}

pub fn clear_nodeui_icon_tile_id(nodeui: &mut TheNodeUI, item_name: &str, index: usize) {
    if let Some(TheNodeUIItem::Icons(_, _, _, items)) = nodeui.get_item_mut(item_name)
        && index < items.len()
    {
        items[index].0 = TheRGBABuffer::new(TheDim::sized(36, 36));
        items[index].2 = Uuid::nil();
    }
}

pub mod add_arch;
pub mod apply_tile;
pub mod build_procedural;
pub mod build_room;
pub mod build_shaft;
pub mod build_stairs;
pub mod clear_palette;
pub mod clear_profile;
pub mod clear_tile;
pub mod copy_tile_id;
pub mod copy_vcode;
pub mod create_campfire;
pub mod create_center_vertex;
pub mod create_fence;
pub mod create_linedef;
pub mod create_palisade;
pub mod create_prop;
pub mod create_roof;
pub mod create_sector;
pub mod create_stairs;
pub mod cut_hole;
pub mod duplicate;
pub mod duplicate_tile;
pub mod edit_linedef;
pub mod edit_maximize;
pub mod edit_sector;
pub mod edit_tile_meta;
pub mod edit_vertex;
pub mod editing_camera;
pub mod editing_slice;
pub mod export_vcode;
pub mod extrude_linedef;
pub mod extrude_sector;
pub mod filter_editing_geo;
pub mod firstp_camera;
pub mod gate_door;
pub mod import_palette;
pub mod import_vcode;
pub mod iso_camera;
pub mod make_sector_rectangular;
pub mod minimize;
pub mod new_tile;
pub mod orbit_camera;
pub mod paste_vcode;
pub mod recess;
pub mod relief;
pub mod remap_tile;
pub mod set_tile_material;
pub mod split;
pub mod toggle_editing_geo;
pub mod toggle_rect_geo;
pub mod window;

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

    fn hud_material_slots(
        &self,
        _map: &Map,
        _server_ctx: &ServerContext,
    ) -> Option<Vec<ActionMaterialSlot>> {
        None
    }

    fn set_hud_material_from_tile(
        &mut self,
        _map: &Map,
        _server_ctx: &ServerContext,
        _slot_index: i32,
        _tile_id: Uuid,
    ) -> bool {
        false
    }

    fn clear_hud_material_slot(
        &mut self,
        _map: &Map,
        _server_ctx: &ServerContext,
        _slot_index: i32,
    ) -> bool {
        false
    }

    fn preserves_hud_material_slots(&self) -> bool {
        false
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

fn normalize_builder_slot_key(key: &str) -> String {
    if !key.chars().any(|ch| ch.is_ascii_lowercase()) {
        let mut out = String::new();
        let mut prev_is_sep = false;
        for ch in key.chars() {
            if ch.is_ascii_alphanumeric() {
                out.push(ch.to_ascii_lowercase());
                prev_is_sep = false;
            } else if !prev_is_sep && !out.is_empty() {
                out.push('_');
                prev_is_sep = true;
            }
        }
        return out.trim_matches('_').to_string();
    }

    normalize_toml_key(key)
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
            | TheNodeUIItem::PaletteIndexPicker(id, _, _, _, _)
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

fn special_action_section_key(action_key: &str) -> Option<(&'static str, &'static str)> {
    match action_key {
        "iso_hide_on_enter" => Some(("iso", "hide_on_enter")),
        _ => None,
    }
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
    fn upsert(
        entries: &mut Vec<(String, toml::Value, Option<String>)>,
        key: String,
        value: toml::Value,
        comment: Option<String>,
    ) {
        if let Some((_, existing, existing_comment)) =
            entries.iter_mut().find(|(k, _, _)| *k == key)
        {
            *existing = value;
            *existing_comment = comment;
        } else {
            entries.push((key, value, comment));
        }
    }

    fn selector_options_comment(values: &[String]) -> String {
        let quoted = values
            .iter()
            .map(|v| format!("\"{}\"", v.replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(", ");
        format!("# {quoted}")
    }

    fn parse_string_array(value: &str) -> Vec<String> {
        if let Ok(toml_value) = value.parse::<toml::Value>()
            && let toml::Value::Array(items) = toml_value
        {
            let parsed: Vec<String> = items
                .iter()
                .filter_map(|item| item.as_str().map(|s| s.trim().to_string()))
                .filter(|s| !s.is_empty())
                .collect();
            if !parsed.is_empty() {
                return parsed;
            }
        }

        value
            .split(',')
            .map(|item| item.trim())
            .filter(|item| !item.is_empty())
            .map(|item| item.trim_matches('"').to_string())
            .collect()
    }

    fn section_entries_mut<'a>(
        sections: &'a mut Vec<(String, Vec<(String, toml::Value, Option<String>)>)>,
        name: &str,
    ) -> &'a mut Vec<(String, toml::Value, Option<String>)> {
        if let Some(index) = sections.iter().position(|(n, _)| n == name) {
            return &mut sections[index].1;
        }
        sections.push((name.to_string(), Vec::new()));
        let last = sections.len() - 1;
        &mut sections[last].1
    }

    let mut root_action_entries: Vec<(String, toml::Value, Option<String>)> = vec![];
    let mut sections: Vec<(String, Vec<(String, toml::Value, Option<String>)>)> = vec![];
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
                let (target_section, key) = if section_stack.is_empty() {
                    if let Some((section, special_key)) = special_action_section_key(&action_key) {
                        (Some(section.to_string()), special_key.to_string())
                    } else {
                        (
                            None,
                            display_key_for_storage(
                                &action_key,
                                &section_stack,
                                root_prefix.as_deref(),
                            ),
                        )
                    }
                } else {
                    (
                        Some(section_stack.join(".")),
                        display_key_for_storage(
                            &action_key,
                            &section_stack,
                            root_prefix.as_deref(),
                        ),
                    )
                };
                let val = if action_key == "iso_hide_on_enter" {
                    toml::Value::Array(
                        parse_string_array(value)
                            .into_iter()
                            .map(toml::Value::String)
                            .collect(),
                    )
                } else {
                    toml::Value::String(value.clone())
                };
                if let Some(section_name) = target_section {
                    let entries = section_entries_mut(&mut sections, &section_name);
                    upsert(entries, key, val, None);
                } else {
                    upsert(&mut root_action_entries, key, val, None);
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
                let comment = Some(selector_options_comment(values));
                if section_stack.is_empty() {
                    upsert(&mut root_action_entries, key, selected, comment);
                } else {
                    let section_name = section_stack.join(".");
                    let entries = section_entries_mut(&mut sections, &section_name);
                    upsert(entries, key, selected, comment);
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
                    upsert(&mut root_action_entries, key, val, None);
                } else {
                    let section_name = section_stack.join(".");
                    let entries = section_entries_mut(&mut sections, &section_name);
                    upsert(entries, key, val, None);
                }
                has_editable_values = true;
            }
            TheNodeUIItem::IntEditSlider(id, _, _, value, _, _)
            | TheNodeUIItem::PaletteSlider(id, _, _, value, _, _)
            | TheNodeUIItem::PaletteIndexPicker(id, _, _, value, _)
            | TheNodeUIItem::IntSlider(id, _, _, value, _, _, _) => {
                let action_key = action_param_key(id);
                let key =
                    display_key_for_storage(&action_key, &section_stack, root_prefix.as_deref());
                let val = toml::Value::Integer(*value as i64);
                if section_stack.is_empty() {
                    upsert(&mut root_action_entries, key, val, None);
                } else {
                    let section_name = section_stack.join(".");
                    let entries = section_entries_mut(&mut sections, &section_name);
                    upsert(entries, key, val, None);
                }
                has_editable_values = true;
            }
            TheNodeUIItem::ColorPicker(id, _, _, value, _) => {
                let action_key = action_param_key(id);
                let key =
                    display_key_for_storage(&action_key, &section_stack, root_prefix.as_deref());
                let val = toml::Value::String(value.to_hex());
                if section_stack.is_empty() {
                    upsert(&mut root_action_entries, key, val, None);
                } else {
                    let section_name = section_stack.join(".");
                    let entries = section_entries_mut(&mut sections, &section_name);
                    upsert(entries, key, val, None);
                }
                has_editable_values = true;
            }
            TheNodeUIItem::Checkbox(id, _, _, value) => {
                let action_key = action_param_key(id);
                let key =
                    display_key_for_storage(&action_key, &section_stack, root_prefix.as_deref());
                let val = toml::Value::Boolean(*value);
                if section_stack.is_empty() {
                    upsert(&mut root_action_entries, key, val, None);
                } else {
                    let section_name = section_stack.join(".");
                    let entries = section_entries_mut(&mut sections, &section_name);
                    upsert(entries, key, val, None);
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
        for (key, value, comment) in &root_action_entries {
            if let Some(comment) = comment {
                out.push_str(comment);
                out.push('\n');
            }
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
        for (key, value, comment) in entries {
            if let Some(comment) = comment {
                out.push_str(comment);
                out.push('\n');
            }
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
                    if let Some((section, _)) = special_action_section_key(&action_key) {
                        section_table(&root, &[section.to_string()]).or(Some(action_root))
                    } else {
                        Some(action_root)
                    }
                } else {
                    section_table(&root, &section_stack)
                        .or_else(|| section_table(action_root, &section_stack))
                };
                if let Some(table) = table {
                    let mut keys =
                        candidate_input_keys(&action_key, &section_stack, root_prefix.as_deref());
                    if section_stack.is_empty()
                        && let Some((_, special_key)) = special_action_section_key(&action_key)
                    {
                        keys.push(special_key.to_string());
                        keys.sort();
                        keys.dedup();
                    }
                    for key in keys {
                        if let Some(value) = table.get(&key) {
                            match value {
                                toml::Value::String(v) => {
                                    nodeui.set_text_value(&id, v.clone());
                                    break;
                                }
                                toml::Value::Integer(v) => {
                                    nodeui.set_text_value(&id, v.to_string());
                                    break;
                                }
                                toml::Value::Float(v) => {
                                    nodeui.set_text_value(&id, v.to_string());
                                    break;
                                }
                                toml::Value::Array(items) if action_key == "iso_hide_on_enter" => {
                                    let joined = items
                                        .iter()
                                        .filter_map(|item| item.as_str())
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    nodeui.set_text_value(&id, joined);
                                    break;
                                }
                                _ => {}
                            }
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
            | TheNodeUIItem::PaletteIndexPicker(id, _, _, _, _)
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
            | TheNodeUIItem::PaletteIndexPicker(id, _, _, value, _)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_material_keys_keep_acronym_slots_readable() {
        assert_eq!(
            builder_material_property_key("PLANK"),
            "builder_material_plank"
        );
        assert_eq!(
            builder_material_property_key("ROOF_PLANK"),
            "builder_material_roof_plank"
        );
        assert_eq!(
            builder_material_property_key("roofMat"),
            "builder_material_roof_mat"
        );
    }
}
