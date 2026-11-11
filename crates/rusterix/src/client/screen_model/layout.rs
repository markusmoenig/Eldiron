use super::*;

pub(crate) fn layout_number(layout: &toml::Table, key: &str) -> Option<f32> {
    layout.get(key).and_then(|value| {
        value
            .as_float()
            .map(|value| value as f32)
            .or_else(|| value.as_integer().map(|value| value as f32))
    })
}

pub(crate) fn parse_action_bar_buttons(buttons: &[toml::Value]) -> Vec<ActionBarButtonConfig> {
    buttons
        .iter()
        .filter_map(|value| {
            if let Some(command) = value.as_str().map(str::trim).filter(|v| !v.is_empty()) {
                return Some(ActionBarButtonConfig {
                    command: Some(command.to_string()),
                    ..Default::default()
                });
            }

            let button = value.as_table()?;
            let text = |key: &str| {
                button
                    .get(key)
                    .and_then(toml::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
            };
            Some(ActionBarButtonConfig {
                command: text("command"),
                command_slot: text("command_slot").or_else(|| text("slot")),
                label: text("label").or_else(|| text("text")).unwrap_or_default(),
                show_icon: button.get("show_icon").and_then(toml::Value::as_bool),
            })
        })
        .collect()
}

pub(crate) fn action_bar_groups(table: &toml::Table) -> Vec<ActionBarGroupConfig> {
    let Some(ui) = table.get("ui").and_then(toml::Value::as_table) else {
        return vec![];
    };
    let default_slot_size = layout_number(ui, "slot_size").unwrap_or(52.0).max(1.0);
    let default_spacing = layout_number(ui, "spacing").unwrap_or(4.0).max(0.0);

    if let Some(groups) = ui.get("groups").and_then(toml::Value::as_array) {
        let parsed: Vec<_> = groups
            .iter()
            .filter_map(toml::Value::as_table)
            .filter_map(|group| {
                let buttons = group
                    .get("buttons")
                    .and_then(toml::Value::as_array)
                    .map(|buttons| parse_action_bar_buttons(buttons))
                    .unwrap_or_default();
                if buttons.is_empty() {
                    return None;
                }
                let align = match group
                    .get("align")
                    .and_then(toml::Value::as_str)
                    .unwrap_or("center")
                    .trim()
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "left" => ActionBarGroupAlign::Left,
                    "right" => ActionBarGroupAlign::Right,
                    _ => ActionBarGroupAlign::Center,
                };
                Some(ActionBarGroupConfig {
                    align,
                    slot_size: layout_number(group, "slot_size")
                        .unwrap_or(default_slot_size)
                        .max(1.0),
                    spacing: layout_number(group, "spacing")
                        .unwrap_or(default_spacing)
                        .max(0.0),
                    buttons,
                })
            })
            .collect();
        if !parsed.is_empty() {
            return parsed;
        }
    }

    ui.get("buttons")
        .and_then(toml::Value::as_array)
        .map(|buttons| ActionBarGroupConfig {
            align: ActionBarGroupAlign::Center,
            slot_size: default_slot_size,
            spacing: default_spacing,
            buttons: parse_action_bar_buttons(buttons),
        })
        .filter(|group| !group.buttons.is_empty())
        .into_iter()
        .collect()
}

pub(crate) fn action_bar_buttons(table: &toml::Table) -> Vec<ActionBarButtonConfig> {
    action_bar_groups(table)
        .into_iter()
        .flat_map(|group| group.buttons)
        .collect()
}

pub(crate) fn action_bar_group_width(group: &ActionBarGroupConfig) -> f32 {
    group.slot_size * group.buttons.len() as f32
        + group.spacing * group.buttons.len().saturating_sub(1) as f32
}

pub(crate) fn action_bar_band_width(
    groups: &[ActionBarGroupConfig],
    align: ActionBarGroupAlign,
    group_spacing: f32,
) -> f32 {
    let widths: Vec<_> = groups
        .iter()
        .filter(|group| group.align == align)
        .map(action_bar_group_width)
        .collect();
    widths.iter().sum::<f32>() + group_spacing * widths.len().saturating_sub(1) as f32
}

pub(crate) fn action_bar_authored_rect(authored: Rect, table: &toml::Table) -> Rect {
    let groups = action_bar_groups(table);
    if groups.is_empty() {
        return authored;
    }
    let Some(ui) = table.get("ui").and_then(toml::Value::as_table) else {
        return authored;
    };
    if !ui
        .get("auto_size")
        .and_then(toml::Value::as_bool)
        .unwrap_or(true)
    {
        return authored;
    }

    let padding = layout_number(ui, "padding").unwrap_or(8.0).max(0.0);
    let edge_padding = layout_number(ui, "edge_padding")
        .unwrap_or(padding)
        .max(0.0);
    let group_spacing = layout_number(ui, "group_spacing").unwrap_or(18.0).max(0.0);
    let left_width = action_bar_band_width(&groups, ActionBarGroupAlign::Left, group_spacing);
    let center_width = action_bar_band_width(&groups, ActionBarGroupAlign::Center, group_spacing);
    let right_width = action_bar_band_width(&groups, ActionBarGroupAlign::Right, group_spacing);
    let content_width = if center_width > 0.0 {
        let side_width = left_width.max(right_width);
        center_width
            + if side_width > 0.0 {
                (side_width + group_spacing) * 2.0
            } else {
                0.0
            }
    } else {
        left_width
            + right_width
            + if left_width > 0.0 && right_width > 0.0 {
                group_spacing
            } else {
                0.0
            }
    };
    let max_slot_size = groups
        .iter()
        .map(|group| group.slot_size)
        .fold(1.0_f32, f32::max);
    Rect::new(
        authored.x,
        authored.y,
        (edge_padding * 2.0 + content_width).max(1.0),
        (padding * 2.0 + max_slot_size).max(1.0),
    )
}

pub(crate) fn action_bar_slot_rects(rect: Rect, table: &toml::Table) -> Vec<Rect> {
    let groups = action_bar_groups(table);
    if groups.is_empty() {
        return vec![];
    }
    let Some(ui) = table.get("ui").and_then(toml::Value::as_table) else {
        return vec![];
    };
    let padding = layout_number(ui, "padding").unwrap_or(8.0).max(0.0);
    let edge_padding = layout_number(ui, "edge_padding")
        .unwrap_or(padding)
        .max(0.0);
    let group_spacing = layout_number(ui, "group_spacing").unwrap_or(18.0).max(0.0);
    let band_width = |align| action_bar_band_width(&groups, align, group_spacing);
    let center_width = band_width(ActionBarGroupAlign::Center);
    let right_width = band_width(ActionBarGroupAlign::Right);
    let mut left_cursor = rect.x + edge_padding;
    let mut center_cursor = rect.x + (rect.width - center_width) * 0.5;
    let mut right_cursor = rect.x + rect.width - edge_padding - right_width;
    let mut group_rects: Vec<Vec<Rect>> = vec![vec![]; groups.len()];

    for (group_index, group) in groups.iter().enumerate() {
        let cursor = match group.align {
            ActionBarGroupAlign::Left => &mut left_cursor,
            ActionBarGroupAlign::Center => &mut center_cursor,
            ActionBarGroupAlign::Right => &mut right_cursor,
        };
        let y = rect.y + (rect.height - group.slot_size) * 0.5;
        for button_index in 0..group.buttons.len() {
            group_rects[group_index].push(Rect::new(
                *cursor + button_index as f32 * (group.slot_size + group.spacing),
                y,
                group.slot_size,
                group.slot_size,
            ));
        }
        *cursor += action_bar_group_width(group) + group_spacing;
    }

    group_rects.into_iter().flatten().collect()
}

pub(crate) fn action_bar_group_separators(rect: Rect, table: &toml::Table) -> Vec<f32> {
    let groups = action_bar_groups(table);
    let slots = action_bar_slot_rects(rect, table);
    let mut slot_index = 0;
    let mut bounds = Vec::with_capacity(groups.len());

    for group in groups {
        let group_slots = &slots[slot_index..slot_index + group.buttons.len()];
        slot_index += group.buttons.len();
        let Some(first) = group_slots.first() else {
            continue;
        };
        let last = group_slots.last().unwrap_or(first);
        bounds.push((first.x, last.x + last.width));
    }
    bounds.sort_by(|left, right| left.0.total_cmp(&right.0));

    bounds
        .windows(2)
        .filter_map(|pair| {
            let gap_start = pair[0].1;
            let gap_end = pair[1].0;
            (gap_end > gap_start).then_some((gap_start + gap_end) * 0.5)
        })
        .collect()
}

pub fn placements(screen: &Map, layout: ScreenLayout) -> Vec<ScreenPlacement> {
    let (start_x, start_y) = crate::utils::align_screen_to_grid(
        layout.reference_viewport.x as f32,
        layout.reference_viewport.y as f32,
        layout.grid_size,
    );

    let mut resolved = Vec::new();
    for sector in &screen.sectors {
        let Some(crate::Value::Str(data)) = sector.properties.get("data") else {
            continue;
        };
        let Ok(table) = data.parse::<toml::Table>() else {
            continue;
        };
        let Some(role) = table
            .get("ui")
            .and_then(toml::Value::as_table)
            .and_then(|ui| ui.get("role"))
            .and_then(toml::Value::as_str)
            .map(str::to_string)
        else {
            continue;
        };
        let bb = sector.bounding_box(screen);
        let mut authored = Rect::new(
            (bb.min.x - start_x) * layout.grid_size,
            (bb.min.y - start_y) * layout.grid_size,
            bb.size().x * layout.grid_size,
            bb.size().y * layout.grid_size,
        );
        if role == "action_bar" {
            authored = action_bar_authored_rect(authored, &table);
        }
        resolved.push(ScreenPlacement {
            rect: layout.resolve(authored, &role, &table),
            role,
            sector_id: sector.id,
            creator_id: sector.creator_id,
            name: sector.name.clone(),
            table,
        });
    }

    resolved
}

#[derive(Clone, Debug, Default)]
pub struct ActionBarButtonConfig {
    pub command: Option<String>,
    pub command_slot: Option<String>,
    pub label: String,
    pub show_icon: Option<bool>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActionBarGroupAlign {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Clone, Debug)]
pub struct ActionBarGroupConfig {
    pub align: ActionBarGroupAlign,
    pub slot_size: f32,
    pub spacing: f32,
    pub buttons: Vec<ActionBarButtonConfig>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ScreenAnchor {
    #[default]
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ScreenLayout {
    pub screen_responsive: bool,
    pub reference_viewport: Vec2<i32>,
    pub viewport: Vec2<i32>,
    pub grid_size: f32,
}

pub struct ScreenPlacement {
    pub role: String,
    pub sector_id: u32,
    pub creator_id: Uuid,
    pub name: String,
    pub rect: Rect,
    pub table: Table,
}

impl ScreenAnchor {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "top_left" => Some(Self::TopLeft),
            "top" | "top_center" => Some(Self::TopCenter),
            "top_right" => Some(Self::TopRight),
            "left" | "center_left" | "middle_left" => Some(Self::CenterLeft),
            "center" | "middle" => Some(Self::Center),
            "right" | "center_right" | "middle_right" => Some(Self::CenterRight),
            "bottom_left" => Some(Self::BottomLeft),
            "bottom" | "bottom_center" => Some(Self::BottomCenter),
            "bottom_right" => Some(Self::BottomRight),
            _ => None,
        }
    }
}

impl ScreenLayout {
    pub fn resolve(&self, authored: Rect, role: &str, table: &toml::Table) -> Rect {
        if !self.screen_responsive {
            return authored;
        }
        if role.eq_ignore_ascii_case("game") {
            return Rect::new(
                0.0,
                0.0,
                self.viewport.x.max(1) as f32,
                self.viewport.y.max(1) as f32,
            );
        }

        let layout = table.get("layout").and_then(toml::Value::as_table);
        let explicit_anchor = layout
            .and_then(|layout| layout.get("anchor"))
            .and_then(toml::Value::as_str)
            .and_then(ScreenAnchor::parse);
        let implicit_action_bar_anchor =
            explicit_anchor.is_none() && role.eq_ignore_ascii_case("action_bar");
        let Some(anchor) = explicit_anchor
            .or_else(|| implicit_action_bar_anchor.then_some(ScreenAnchor::BottomCenter))
        else {
            return authored;
        };

        let reference_w = self.reference_viewport.x.max(1) as f32;
        let reference_h = self.reference_viewport.y.max(1) as f32;
        let viewport_w = self.viewport.x.max(1) as f32;
        let viewport_h = self.viewport.y.max(1) as f32;
        let authored_right = authored.x + authored.width;
        let authored_bottom = authored.y + authored.height;
        let default_x = if implicit_action_bar_anchor {
            0.0
        } else {
            match anchor {
                ScreenAnchor::TopLeft | ScreenAnchor::CenterLeft | ScreenAnchor::BottomLeft => {
                    authored.x
                }
                ScreenAnchor::TopCenter | ScreenAnchor::Center | ScreenAnchor::BottomCenter => {
                    authored.x + authored.width * 0.5 - reference_w * 0.5
                }
                ScreenAnchor::TopRight | ScreenAnchor::CenterRight | ScreenAnchor::BottomRight => {
                    authored_right - reference_w
                }
            }
        };
        let default_y = if implicit_action_bar_anchor {
            -16.0
        } else {
            match anchor {
                ScreenAnchor::TopLeft | ScreenAnchor::TopCenter | ScreenAnchor::TopRight => {
                    authored.y
                }
                ScreenAnchor::CenterLeft | ScreenAnchor::Center | ScreenAnchor::CenterRight => {
                    authored.y + authored.height * 0.5 - reference_h * 0.5
                }
                ScreenAnchor::BottomLeft
                | ScreenAnchor::BottomCenter
                | ScreenAnchor::BottomRight => authored_bottom - reference_h,
            }
        };
        let offset_x = layout
            .and_then(|layout| layout_number(layout, "x"))
            .unwrap_or(default_x);
        let offset_y = layout
            .and_then(|layout| layout_number(layout, "y"))
            .unwrap_or(default_y);

        let x = match anchor {
            ScreenAnchor::TopLeft | ScreenAnchor::CenterLeft | ScreenAnchor::BottomLeft => offset_x,
            ScreenAnchor::TopCenter | ScreenAnchor::Center | ScreenAnchor::BottomCenter => {
                viewport_w * 0.5 - authored.width * 0.5 + offset_x
            }
            ScreenAnchor::TopRight | ScreenAnchor::CenterRight | ScreenAnchor::BottomRight => {
                viewport_w - authored.width + offset_x
            }
        };
        let y = match anchor {
            ScreenAnchor::TopLeft | ScreenAnchor::TopCenter | ScreenAnchor::TopRight => offset_y,
            ScreenAnchor::CenterLeft | ScreenAnchor::Center | ScreenAnchor::CenterRight => {
                viewport_h * 0.5 - authored.height * 0.5 + offset_y
            }
            ScreenAnchor::BottomLeft | ScreenAnchor::BottomCenter | ScreenAnchor::BottomRight => {
                viewport_h - authored.height + offset_y
            }
        };
        let fill_width = role.eq_ignore_ascii_case("action_bar")
            && table
                .get("ui")
                .and_then(toml::Value::as_table)
                .and_then(|ui| ui.get("fill_width"))
                .and_then(toml::Value::as_bool)
                .unwrap_or(false);
        if fill_width {
            Rect::new(0.0, y, viewport_w, authored.height)
        } else {
            Rect::new(x, y, authored.width, authored.height)
        }
    }
}
