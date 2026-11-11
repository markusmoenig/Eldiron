use super::*;

pub(crate) fn catalog_panel_config(
    config: &toml::Table,
    root: Option<&toml::Table>,
    authored_rect: Option<Rect>,
    assets: &Assets,
) -> CatalogPanelConfig {
    let mut parsed = CatalogPanelConfig::default();
    parsed.columns = config
        .get("columns")
        .and_then(toml::Value::as_integer)
        .unwrap_or(parsed.columns as i64)
        .max(1) as usize;
    parsed.rows = config
        .get("rows")
        .and_then(toml::Value::as_integer)
        .map(|rows| rows.max(1) as usize);
    parsed.cell_size = layout_number(config, "cell_size")
        .or_else(|| layout_number(config, "slot_size"))
        .unwrap_or(parsed.cell_size)
        .max(16.0);
    parsed.spacing = layout_number(config, "spacing")
        .or_else(|| layout_number(config, "gap"))
        .unwrap_or(parsed.spacing)
        .max(0.0);
    parsed.padding = layout_number(config, "padding")
        .unwrap_or(parsed.padding)
        .max(0.0);
    parsed.title_height = layout_number(config, "title_height")
        .unwrap_or(parsed.title_height)
        .max(20.0);
    parsed.tab_height = layout_number(config, "tab_height")
        .unwrap_or(parsed.tab_height)
        .max(18.0);
    parsed.detail_width = layout_number(config, "detail_width")
        .unwrap_or(parsed.detail_width)
        .max(80.0);
    parsed.detail_gap = layout_number(config, "detail_gap")
        .unwrap_or(parsed.detail_gap)
        .max(0.0);
    parsed.icon_inset = layout_number(config, "icon_inset")
        .unwrap_or(parsed.icon_inset)
        .max(0.0);
    parsed.show_names = config
        .get("show_names")
        .and_then(toml::Value::as_bool)
        .unwrap_or(parsed.show_names);
    parsed.show_tabs = config
        .get("show_tabs")
        .and_then(toml::Value::as_bool)
        .unwrap_or(parsed.show_tabs);
    parsed.show_details = config
        .get("show_details")
        .and_then(toml::Value::as_bool)
        .unwrap_or(parsed.show_details);
    parsed.show_assign = config
        .get("show_assign")
        .and_then(toml::Value::as_bool)
        .unwrap_or(parsed.show_assign);
    let text = |key: &str| {
        config
            .get(key)
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };
    parsed.title = text("title").unwrap_or(parsed.title);
    parsed.font = text("font").unwrap_or(parsed.font);
    parsed.title_font = text("title_font").unwrap_or(parsed.title_font);
    parsed.font_size = layout_number(config, "font_size")
        .unwrap_or(parsed.font_size)
        .max(6.0);
    parsed.title_font_size = layout_number(config, "title_font_size")
        .unwrap_or(parsed.title_font_size)
        .max(6.0);
    parsed.small_font_size = layout_number(config, "small_font_size")
        .unwrap_or(parsed.small_font_size)
        .max(6.0);
    parsed.authored_rect = authored_rect;

    let frame = config
        .get("frame")
        .and_then(toml::Value::as_table)
        .or_else(|| root.and_then(|root| root.get("frame").and_then(toml::Value::as_table)));
    let slot = config
        .get("slot")
        .and_then(toml::Value::as_table)
        .or_else(|| root.and_then(|root| root.get("slot").and_then(toml::Value::as_table)));

    if let Some(frame) = frame {
        parsed.background_color = color_from_table(frame, "background_color")
            .or_else(|| color_from_table(frame, "color"))
            .unwrap_or(parsed.background_color);
        parsed.border_color =
            color_from_table(frame, "border_color").unwrap_or(parsed.border_color);
        parsed.border_size = frame
            .get("border_size")
            .and_then(toml::Value::as_integer)
            .unwrap_or(parsed.border_size as i64)
            .max(0) as i32;
        parsed.frame_texture = action_bar_tile_texture(frame, "tile", assets);
        parsed.frame_slice = frame
            .get("slice")
            .and_then(toml::Value::as_integer)
            .unwrap_or(0)
            .max(0) as usize;
    }
    if let Some(slot) = slot {
        parsed.slot_background_color = color_from_table(slot, "background_color")
            .or_else(|| color_from_table(slot, "color"))
            .unwrap_or(parsed.slot_background_color);
        parsed.slot_border_color =
            color_from_table(slot, "border_color").unwrap_or(parsed.slot_border_color);
        parsed.slot_border_size = slot
            .get("border_size")
            .and_then(toml::Value::as_integer)
            .unwrap_or(parsed.slot_border_size as i64)
            .max(0) as i32;
        parsed.slot_texture = action_bar_tile_texture(slot, "normal_tile", assets)
            .or_else(|| action_bar_tile_texture(slot, "tile", assets));
        parsed.slot_slice = slot
            .get("slice")
            .and_then(toml::Value::as_integer)
            .unwrap_or(0)
            .max(0) as usize;
    }
    parsed.title_background_color =
        color_from_table(config, "title_background_color").unwrap_or(parsed.title_background_color);
    parsed.text_color = color_from_table(config, "text_color").unwrap_or(parsed.text_color);
    parsed.muted_text_color =
        color_from_table(config, "muted_text_color").unwrap_or(parsed.muted_text_color);
    parsed.detail_background_color = color_from_table(config, "detail_background_color")
        .unwrap_or(parsed.detail_background_color);
    parsed.tab_background_color =
        color_from_table(config, "tab_background_color").unwrap_or(parsed.tab_background_color);
    parsed.tab_selected_color =
        color_from_table(config, "tab_selected_color").unwrap_or(parsed.tab_selected_color);
    parsed.separator_color =
        color_from_table(config, "separator_color").unwrap_or(parsed.separator_color);
    parsed
}

pub(crate) fn inventory_panel_config(
    config: &toml::Table,
    authored_rect: Option<Rect>,
) -> InventoryPanelConfig {
    let mut parsed = InventoryPanelConfig::default();
    parsed.columns = config
        .get("columns")
        .and_then(toml::Value::as_integer)
        .unwrap_or(parsed.columns as i64)
        .max(1) as usize;
    parsed.rows = config
        .get("rows")
        .and_then(toml::Value::as_integer)
        .unwrap_or(parsed.rows as i64)
        .max(1) as usize;
    parsed.cell_size = layout_number(config, "cell_size")
        .or_else(|| layout_number(config, "slot_size"))
        .unwrap_or(parsed.cell_size)
        .max(16.0);
    parsed.spacing = layout_number(config, "spacing")
        .unwrap_or(parsed.spacing)
        .max(0.0);
    parsed.padding = layout_number(config, "padding")
        .unwrap_or(parsed.padding)
        .max(0.0);
    parsed.title_height = layout_number(config, "title_height")
        .unwrap_or(parsed.title_height)
        .max(20.0);
    parsed.tab_height = layout_number(config, "tab_height")
        .unwrap_or(parsed.tab_height)
        .max(20.0);
    parsed.footer_height = layout_number(config, "footer_height")
        .unwrap_or(parsed.footer_height)
        .max(20.0);
    parsed.sort_width = layout_number(config, "sort_width")
        .unwrap_or(parsed.sort_width)
        .max(64.0);
    parsed.font = config
        .get("font")
        .and_then(toml::Value::as_str)
        .unwrap_or("")
        .to_string();
    parsed.font_size = layout_number(config, "font_size")
        .unwrap_or(parsed.font_size)
        .max(1.0);
    parsed.title_font_size = layout_number(config, "title_font_size")
        .unwrap_or(parsed.title_font_size)
        .max(1.0);
    if let Some(title) = config.get("title").and_then(toml::Value::as_str) {
        parsed.title = title.to_string();
    }
    if let Some(categories) = config
        .get("categories")
        .or_else(|| config.get("tabs"))
        .and_then(toml::Value::as_array)
    {
        let categories = categories
            .iter()
            .filter_map(ChoiceWidget::option_from_toml)
            .collect::<Vec<_>>();
        if !categories.is_empty() {
            parsed.categories = categories;
        }
    }
    if let Some(options) = config.get("sort_options").and_then(toml::Value::as_array) {
        let options = options
            .iter()
            .filter_map(ChoiceWidget::option_from_toml)
            .collect::<Vec<_>>();
        if !options.is_empty() {
            parsed.sort_options = options;
        }
    }
    parsed.authored_rect = authored_rect;
    parsed.background_color =
        color_from_table(config, "background_color").unwrap_or(parsed.background_color);
    parsed.title_background_color =
        color_from_table(config, "title_background_color").unwrap_or(parsed.title_background_color);
    parsed.border_color = color_from_table(config, "border_color").unwrap_or(parsed.border_color);
    parsed.text_color = color_from_table(config, "text_color").unwrap_or(parsed.text_color);
    parsed.muted_text_color =
        color_from_table(config, "muted_text_color").unwrap_or(parsed.muted_text_color);
    parsed.slot_background_color =
        color_from_table(config, "slot_background_color").unwrap_or(parsed.slot_background_color);
    parsed.slot_border_color =
        color_from_table(config, "slot_border_color").unwrap_or(parsed.slot_border_color);
    parsed.selected_slot_color =
        color_from_table(config, "selected_slot_color").unwrap_or(parsed.selected_slot_color);
    parsed.tab_background_color =
        color_from_table(config, "tab_background_color").unwrap_or(parsed.tab_background_color);
    parsed.tab_selected_color =
        color_from_table(config, "tab_selected_color").unwrap_or(parsed.tab_selected_color);
    parsed.dropdown_background_color = color_from_table(config, "dropdown_background_color")
        .unwrap_or(parsed.dropdown_background_color);
    parsed.dropdown_panel_color =
        color_from_table(config, "dropdown_panel_color").unwrap_or(parsed.dropdown_panel_color);
    parsed
}

pub(crate) fn equipment_panel_config(
    config: &toml::Table,
    authored_rect: Option<Rect>,
) -> EquipmentPanelConfig {
    let mut parsed = EquipmentPanelConfig::default();
    parsed.padding = layout_number(config, "padding")
        .unwrap_or(parsed.padding)
        .max(0.0);
    parsed.title_height = layout_number(config, "title_height")
        .unwrap_or(parsed.title_height)
        .max(20.0);
    parsed.slot_size = layout_number(config, "slot_size")
        .or_else(|| layout_number(config, "cell_size"))
        .unwrap_or(parsed.slot_size)
        .max(20.0);
    parsed.spacing = layout_number(config, "spacing")
        .unwrap_or(parsed.spacing)
        .max(0.0);
    parsed.column_gap = layout_number(config, "column_gap")
        .unwrap_or(parsed.column_gap)
        .max(0.0);
    parsed.label_width = layout_number(config, "label_width")
        .unwrap_or(parsed.label_width)
        .max(0.0);
    parsed.avatar_width = layout_number(config, "avatar_width")
        .unwrap_or(parsed.avatar_width)
        .max(24.0);
    parsed.avatar_height = layout_number(config, "avatar_height")
        .unwrap_or(parsed.avatar_height)
        .max(24.0);
    parsed.avatar_scale = layout_number(config, "avatar_scale")
        .unwrap_or(parsed.avatar_scale)
        .clamp(0.25, 4.0);
    parsed.font = config
        .get("font")
        .and_then(toml::Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    parsed.font_size = layout_number(config, "font_size")
        .unwrap_or(parsed.font_size)
        .max(6.0);
    parsed.title_font_size = layout_number(config, "title_font_size")
        .unwrap_or(parsed.title_font_size)
        .max(6.0);
    if let Some(title) = config.get("title").and_then(toml::Value::as_str) {
        parsed.title = title.trim().to_string();
    }
    let slots = |key: &str| {
        config
            .get(key)
            .and_then(toml::Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    parsed.left_slots = slots("left_slots");
    parsed.right_slots = slots("right_slots");
    parsed.authored_rect = authored_rect;
    parsed.background_color =
        color_from_table(config, "background_color").unwrap_or(parsed.background_color);
    parsed.title_background_color =
        color_from_table(config, "title_background_color").unwrap_or(parsed.title_background_color);
    parsed.border_color = color_from_table(config, "border_color").unwrap_or(parsed.border_color);
    parsed.text_color = color_from_table(config, "text_color").unwrap_or(parsed.text_color);
    parsed.muted_text_color =
        color_from_table(config, "muted_text_color").unwrap_or(parsed.muted_text_color);
    parsed.slot_background_color =
        color_from_table(config, "slot_background_color").unwrap_or(parsed.slot_background_color);
    parsed.slot_border_color =
        color_from_table(config, "slot_border_color").unwrap_or(parsed.slot_border_color);
    parsed.occupied_slot_color = color_from_table(config, "occupied_slot_color")
        .or_else(|| color_from_table(config, "selected_slot_color"))
        .unwrap_or(parsed.occupied_slot_color);
    parsed
}

pub(crate) fn preferences_panel_config(config: &toml::Table) -> PreferencesPanelConfig {
    let mut parsed = PreferencesPanelConfig::default();
    parsed.width = layout_number(config, "width")
        .unwrap_or(parsed.width)
        .max(180.0);
    parsed.padding = layout_number(config, "padding")
        .unwrap_or(parsed.padding)
        .max(0.0);
    parsed.title_height = layout_number(config, "title_height")
        .unwrap_or(parsed.title_height)
        .max(20.0);
    parsed.row_height = layout_number(config, "row_height")
        .unwrap_or(parsed.row_height)
        .max(20.0);
    parsed.font = config
        .get("font")
        .and_then(toml::Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    parsed.font_size = layout_number(config, "font_size")
        .unwrap_or(parsed.font_size)
        .max(6.0);
    parsed.title_font_size = layout_number(config, "title_font_size")
        .unwrap_or(parsed.title_font_size)
        .max(6.0);
    if let Some(title) = config.get("title").and_then(toml::Value::as_str) {
        parsed.title = title.trim().to_string();
    }
    parsed.background_color =
        color_from_table(config, "background_color").unwrap_or(parsed.background_color);
    parsed.title_background_color =
        color_from_table(config, "title_background_color").unwrap_or(parsed.title_background_color);
    parsed.border_color = color_from_table(config, "border_color").unwrap_or(parsed.border_color);
    parsed.text_color = color_from_table(config, "text_color").unwrap_or(parsed.text_color);
    parsed.muted_text_color =
        color_from_table(config, "muted_text_color").unwrap_or(parsed.muted_text_color);
    parsed
}

#[derive(Clone)]
pub struct InventoryPanelConfig {
    pub columns: usize,
    pub rows: usize,
    pub cell_size: f32,
    pub spacing: f32,
    pub padding: f32,
    pub title_height: f32,
    pub tab_height: f32,
    pub footer_height: f32,
    pub sort_width: f32,
    pub font: String,
    pub font_size: f32,
    pub title_font_size: f32,
    pub title: String,
    pub categories: Vec<ChoiceOption>,
    pub sort_options: Vec<ChoiceOption>,
    pub authored_rect: Option<Rect>,
    pub background_color: Pixel,
    pub title_background_color: Pixel,
    pub border_color: Pixel,
    pub text_color: Pixel,
    pub muted_text_color: Pixel,
    pub slot_background_color: Pixel,
    pub slot_border_color: Pixel,
    pub selected_slot_color: Pixel,
    pub tab_background_color: Pixel,
    pub tab_selected_color: Pixel,
    pub dropdown_background_color: Pixel,
    pub dropdown_panel_color: Pixel,
}

impl Default for InventoryPanelConfig {
    fn default() -> Self {
        Self {
            columns: 8,
            rows: 5,
            cell_size: 48.0,
            spacing: 4.0,
            padding: 10.0,
            title_height: 38.0,
            tab_height: 30.0,
            footer_height: 34.0,
            sort_width: 132.0,
            font: String::new(),
            font_size: 14.0,
            title_font_size: 20.0,
            title: "Inventory".to_string(),
            categories: vec![
                ChoiceOption {
                    label: "All".to_string(),
                    value: "all".to_string(),
                },
                ChoiceOption {
                    label: "Equipment".to_string(),
                    value: "equipment".to_string(),
                },
                ChoiceOption {
                    label: "Consumables".to_string(),
                    value: "consumables".to_string(),
                },
                ChoiceOption {
                    label: "Materials".to_string(),
                    value: "materials".to_string(),
                },
                ChoiceOption {
                    label: "Misc".to_string(),
                    value: "misc".to_string(),
                },
            ],
            sort_options: vec![
                ChoiceOption {
                    label: "Newest".to_string(),
                    value: "newest".to_string(),
                },
                ChoiceOption {
                    label: "Name".to_string(),
                    value: "name".to_string(),
                },
                ChoiceOption {
                    label: "Value".to_string(),
                    value: "value".to_string(),
                },
                ChoiceOption {
                    label: "Quantity".to_string(),
                    value: "quantity".to_string(),
                },
            ],
            authored_rect: None,
            background_color: [8, 10, 10, 246],
            title_background_color: [12, 14, 14, 246],
            border_color: [104, 88, 55, 255],
            text_color: [222, 214, 190, 255],
            muted_text_color: [156, 149, 130, 255],
            slot_background_color: [13, 16, 16, 238],
            slot_border_color: [77, 67, 47, 255],
            selected_slot_color: [238, 214, 118, 255],
            tab_background_color: [10, 13, 13, 230],
            tab_selected_color: [22, 29, 30, 245],
            dropdown_background_color: [10, 13, 13, 245],
            dropdown_panel_color: [8, 11, 11, 252],
        }
    }
}

#[derive(Clone)]
pub struct EquipmentPanelConfig {
    pub padding: f32,
    pub title_height: f32,
    pub slot_size: f32,
    pub spacing: f32,
    pub column_gap: f32,
    pub label_width: f32,
    pub avatar_width: f32,
    pub avatar_height: f32,
    pub avatar_scale: f32,
    pub font: String,
    pub font_size: f32,
    pub title_font_size: f32,
    pub title: String,
    pub left_slots: Vec<String>,
    pub right_slots: Vec<String>,
    pub authored_rect: Option<Rect>,
    pub background_color: Pixel,
    pub title_background_color: Pixel,
    pub border_color: Pixel,
    pub text_color: Pixel,
    pub muted_text_color: Pixel,
    pub slot_background_color: Pixel,
    pub slot_border_color: Pixel,
    pub occupied_slot_color: Pixel,
}

impl Default for EquipmentPanelConfig {
    fn default() -> Self {
        Self {
            padding: 10.0,
            title_height: 38.0,
            slot_size: 52.0,
            spacing: 8.0,
            column_gap: 12.0,
            label_width: 70.0,
            avatar_width: 150.0,
            avatar_height: 300.0,
            avatar_scale: 1.0,
            font: String::new(),
            font_size: 13.0,
            title_font_size: 20.0,
            title: "Equipment".to_string(),
            left_slots: Vec::new(),
            right_slots: Vec::new(),
            authored_rect: None,
            background_color: [8, 10, 10, 246],
            title_background_color: [12, 14, 14, 246],
            border_color: [104, 88, 55, 255],
            text_color: [222, 214, 190, 255],
            muted_text_color: [156, 149, 130, 255],
            slot_background_color: [13, 16, 16, 238],
            slot_border_color: [77, 67, 47, 255],
            occupied_slot_color: [238, 214, 118, 255],
        }
    }
}

#[derive(Clone)]
pub struct PreferencesPanelConfig {
    pub width: f32,
    pub padding: f32,
    pub title_height: f32,
    pub row_height: f32,
    pub font: String,
    pub font_size: f32,
    pub title_font_size: f32,
    pub title: String,
    pub background_color: Pixel,
    pub title_background_color: Pixel,
    pub border_color: Pixel,
    pub text_color: Pixel,
    pub muted_text_color: Pixel,
}

impl Default for PreferencesPanelConfig {
    fn default() -> Self {
        Self {
            width: 290.0,
            padding: 10.0,
            title_height: 34.0,
            row_height: 32.0,
            font: String::new(),
            font_size: 13.0,
            title_font_size: 18.0,
            title: "Preferences".to_string(),
            background_color: [8, 10, 10, 246],
            title_background_color: [12, 14, 14, 246],
            border_color: [104, 88, 55, 255],
            text_color: [222, 214, 190, 255],
            muted_text_color: [156, 149, 130, 255],
        }
    }
}

#[derive(Clone)]
pub struct CatalogPanelConfig {
    pub columns: usize,
    pub rows: Option<usize>,
    pub cell_size: f32,
    pub spacing: f32,
    pub padding: f32,
    pub title_height: f32,
    pub tab_height: f32,
    pub detail_width: f32,
    pub detail_gap: f32,
    pub icon_inset: f32,
    pub show_names: bool,
    pub show_tabs: bool,
    pub show_details: bool,
    pub show_assign: bool,
    pub title: String,
    pub font: String,
    pub title_font: String,
    pub font_size: f32,
    pub title_font_size: f32,
    pub small_font_size: f32,
    pub authored_rect: Option<Rect>,
    pub background_color: Pixel,
    pub title_background_color: Pixel,
    pub border_color: Pixel,
    pub border_size: i32,
    pub text_color: Pixel,
    pub muted_text_color: Pixel,
    pub slot_background_color: Pixel,
    pub slot_border_color: Pixel,
    pub slot_border_size: i32,
    pub detail_background_color: Pixel,
    pub tab_background_color: Pixel,
    pub tab_selected_color: Pixel,
    pub separator_color: Pixel,
    pub frame_texture: Option<Texture>,
    pub frame_slice: usize,
    pub slot_texture: Option<Texture>,
    pub slot_slice: usize,
}

impl Default for CatalogPanelConfig {
    fn default() -> Self {
        Self {
            columns: 5,
            rows: None,
            cell_size: 72.0,
            spacing: 8.0,
            padding: 12.0,
            title_height: 38.0,
            tab_height: 30.0,
            detail_width: 196.0,
            detail_gap: 12.0,
            icon_inset: 6.0,
            show_names: false,
            show_tabs: true,
            show_details: true,
            show_assign: false,
            title: String::new(),
            font: String::new(),
            title_font: String::new(),
            font_size: 14.0,
            title_font_size: 20.0,
            small_font_size: 12.0,
            authored_rect: None,
            background_color: [8, 10, 10, 246],
            title_background_color: [12, 14, 14, 246],
            border_color: [104, 88, 55, 255],
            border_size: 1,
            text_color: [222, 214, 190, 255],
            muted_text_color: [156, 149, 130, 255],
            slot_background_color: [13, 16, 16, 238],
            slot_border_color: [77, 67, 47, 255],
            slot_border_size: 1,
            detail_background_color: [8, 11, 11, 230],
            tab_background_color: [10, 13, 13, 230],
            tab_selected_color: [22, 29, 30, 245],
            separator_color: [77, 67, 47, 255],
            frame_texture: None,
            frame_slice: 0,
            slot_texture: None,
            slot_slice: 0,
        }
    }
}

impl CatalogPanelConfig {
    pub(crate) fn actions_default() -> Self {
        Self {
            title: "Actions".to_string(),
            show_names: true,
            show_details: false,
            show_assign: true,
            ..Self::default()
        }
    }
}
