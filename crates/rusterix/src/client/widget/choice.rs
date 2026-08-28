use crate::{Assets, Pixel, Rect, client::draw2d};
use draw2d::Draw2D;
use theframework::prelude::*;
use toml::Table;

use super::Widget;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChoiceWidgetKind {
    TabBar,
    Dropdown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChoiceOption {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChoiceInteraction {
    None,
    Consumed,
    Selected(String),
}

pub struct ChoiceWidget {
    pub name: String,
    pub id: u32,
    pub rect: Rect,
    pub kind: ChoiceWidgetKind,
    pub binding: String,
    pub options: Vec<ChoiceOption>,
    pub selected: usize,
    pub open: bool,
    pub font: String,
    pub font_size: f32,
    pub spacing: f32,
    pub text_padding: f32,
    pub item_height: f32,
    pub indicator_size: f32,
    pub equal_widths: bool,
    pub open_upwards: bool,
    pub background_color: Pixel,
    pub hover_color: Pixel,
    pub selected_color: Pixel,
    pub panel_color: Pixel,
    pub border_color: Pixel,
    pub text_color: Pixel,
    pub muted_text_color: Pixel,
    pub indicator_color: Pixel,
    pub border_size: i32,
}

impl ChoiceWidget {
    pub fn from_ui(
        name: String,
        id: u32,
        rect: Rect,
        kind: ChoiceWidgetKind,
        ui: &Table,
    ) -> Option<Self> {
        let binding = ui
            .get("binding")
            .or_else(|| ui.get("bind"))
            .and_then(toml::Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if binding.is_empty() {
            return None;
        }

        let options = ui
            .get(if kind == ChoiceWidgetKind::TabBar {
                "tabs"
            } else {
                "options"
            })
            .or_else(|| ui.get("items"))
            .or_else(|| ui.get("options"))
            .or_else(|| ui.get("tabs"))
            .and_then(toml::Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Self::option_from_toml)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if options.is_empty() {
            return None;
        }

        let default_value = ui
            .get("default")
            .or_else(|| ui.get("value"))
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let selected = default_value
            .and_then(|value| {
                options
                    .iter()
                    .position(|option| option.value.eq_ignore_ascii_case(value))
            })
            .unwrap_or(0);

        Some(Self {
            name,
            id,
            rect,
            kind,
            binding,
            options,
            selected,
            open: false,
            font: Self::string(ui, "font").unwrap_or_default(),
            font_size: Self::number(ui, "font_size").unwrap_or(14.0).max(1.0),
            spacing: Self::number(ui, "spacing").unwrap_or(0.0).max(0.0),
            text_padding: Self::number(ui, "text_padding")
                .or_else(|| Self::number(ui, "padding"))
                .unwrap_or(10.0)
                .max(0.0),
            item_height: Self::number(ui, "item_height")
                .unwrap_or(rect.height.max(24.0))
                .max(1.0),
            indicator_size: Self::number(ui, "indicator_size")
                .unwrap_or(if kind == ChoiceWidgetKind::TabBar {
                    2.0
                } else {
                    0.0
                })
                .max(0.0),
            equal_widths: ui
                .get("equal_widths")
                .and_then(toml::Value::as_bool)
                .unwrap_or(false),
            open_upwards: ui
                .get("open_upwards")
                .and_then(toml::Value::as_bool)
                .unwrap_or(false),
            background_color: Self::color(ui, "background_color", [10, 13, 13, 230]),
            hover_color: Self::color(ui, "hover_color", [29, 33, 32, 245]),
            selected_color: Self::color(ui, "selected_color", [22, 29, 30, 245]),
            panel_color: Self::color(ui, "panel_color", [8, 11, 11, 248]),
            border_color: Self::color(ui, "border_color", [77, 67, 47, 255]),
            text_color: Self::color(ui, "text_color", [222, 214, 190, 255]),
            muted_text_color: Self::color(ui, "muted_text_color", [156, 149, 130, 255]),
            indicator_color: Self::color(ui, "indicator_color", [190, 156, 91, 255]),
            border_size: ui
                .get("border_size")
                .and_then(toml::Value::as_integer)
                .unwrap_or(1)
                .max(0) as i32,
        })
    }

    pub(crate) fn option_from_toml(value: &toml::Value) -> Option<ChoiceOption> {
        if let Some(value) = value.as_str() {
            let value = value.trim();
            return (!value.is_empty()).then(|| ChoiceOption {
                label: value.to_string(),
                value: value.to_ascii_lowercase(),
            });
        }

        let table = value.as_table()?;
        let value = table
            .get("value")
            .or_else(|| table.get("id"))
            .and_then(toml::Value::as_str)?
            .trim();
        if value.is_empty() {
            return None;
        }
        let label = table
            .get("label")
            .or_else(|| table.get("text"))
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .unwrap_or(value);
        Some(ChoiceOption {
            label: label.to_string(),
            value: value.to_string(),
        })
    }

    fn string(table: &Table, key: &str) -> Option<String> {
        table
            .get(key)
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    fn number(table: &Table, key: &str) -> Option<f32> {
        table.get(key).and_then(|value| match value {
            toml::Value::Float(value) => Some(*value as f32),
            toml::Value::Integer(value) => Some(*value as f32),
            _ => None,
        })
    }

    fn color(table: &Table, key: &str, fallback: Pixel) -> Pixel {
        table
            .get(key)
            .and_then(toml::Value::as_str)
            .map(Self::hex_to_rgba)
            .unwrap_or(fallback)
    }

    fn hex_to_rgba(hex: &str) -> Pixel {
        let hex = hex.trim().trim_start_matches('#');
        match hex.len() {
            6 => [
                u8::from_str_radix(&hex[0..2], 16).unwrap_or(255),
                u8::from_str_radix(&hex[2..4], 16).unwrap_or(255),
                u8::from_str_radix(&hex[4..6], 16).unwrap_or(255),
                255,
            ],
            8 => [
                u8::from_str_radix(&hex[0..2], 16).unwrap_or(255),
                u8::from_str_radix(&hex[2..4], 16).unwrap_or(255),
                u8::from_str_radix(&hex[4..6], 16).unwrap_or(255),
                u8::from_str_radix(&hex[6..8], 16).unwrap_or(255),
            ],
            _ => [255, 255, 255, 255],
        }
    }

    pub fn selected_value(&self) -> &str {
        self.options
            .get(self.selected)
            .map(|option| option.value.as_str())
            .unwrap_or("")
    }

    pub fn sync_value(&mut self, value: Option<&str>) {
        let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
            return;
        };
        if let Some(index) = self
            .options
            .iter()
            .position(|option| option.value.eq_ignore_ascii_case(value))
        {
            self.selected = index;
        }
    }

    pub fn popup_rect(&self) -> Rect {
        let height = self.item_height * self.options.len() as f32;
        let y = if self.open_upwards {
            self.rect.y - height
        } else {
            self.rect.y + self.rect.height
        };
        Rect::new(self.rect.x, y, self.rect.width, height)
    }

    pub fn contains_interactive_point(&self, point: Vec2<f32>) -> bool {
        self.rect.contains(point)
            || (self.kind == ChoiceWidgetKind::Dropdown
                && self.open
                && self.popup_rect().contains(point))
    }

    pub fn interact(&mut self, point: Vec2<f32>) -> ChoiceInteraction {
        match self.kind {
            ChoiceWidgetKind::TabBar => {
                let Some(index) = self.tab_index_at(point) else {
                    return ChoiceInteraction::None;
                };
                self.selected = index;
                ChoiceInteraction::Selected(self.selected_value().to_string())
            }
            ChoiceWidgetKind::Dropdown => {
                if self.open {
                    if let Some(index) = self.dropdown_index_at(point) {
                        self.selected = index;
                        self.open = false;
                        return ChoiceInteraction::Selected(self.selected_value().to_string());
                    }
                    self.open = false;
                    return ChoiceInteraction::Consumed;
                }
                if self.rect.contains(point) {
                    self.open = true;
                    return ChoiceInteraction::Consumed;
                }
                ChoiceInteraction::None
            }
        }
    }

    fn tab_rect(&self, index: usize) -> Rect {
        let count = self.options.len().max(1) as f32;
        let total_spacing = self.spacing * (count - 1.0);
        let available = (self.rect.width - total_spacing).max(1.0);
        if self.equal_widths {
            let width = (available / count).max(1.0);
            return Rect::new(
                self.rect.x + index as f32 * (width + self.spacing),
                self.rect.y,
                width,
                self.rect.height,
            );
        }

        let desired = self
            .options
            .iter()
            .map(|option| {
                let glyphs = option.label.chars().count().max(1) as f32;
                (glyphs * self.font_size * 0.58 + self.text_padding * 2.0).max(self.font_size * 2.0)
            })
            .collect::<Vec<_>>();
        let desired_total = desired.iter().sum::<f32>().max(1.0);
        let extra = (available - desired_total).max(0.0) / count;
        let scale = if desired_total > available {
            available / desired_total
        } else {
            1.0
        };
        let widths = desired
            .iter()
            .map(|width| width * scale + extra)
            .collect::<Vec<_>>();
        let x = self.rect.x + widths.iter().take(index).sum::<f32>() + self.spacing * index as f32;
        let width = widths.get(index).copied().unwrap_or(available);
        Rect::new(x, self.rect.y, width, self.rect.height)
    }

    fn tab_index_at(&self, point: Vec2<f32>) -> Option<usize> {
        self.options
            .iter()
            .enumerate()
            .find_map(|(index, _)| self.tab_rect(index).contains(point).then_some(index))
    }

    fn dropdown_item_rect(&self, index: usize) -> Rect {
        let popup = self.popup_rect();
        Rect::new(
            popup.x,
            popup.y + index as f32 * self.item_height,
            popup.width,
            self.item_height,
        )
    }

    fn dropdown_index_at(&self, point: Vec2<f32>) -> Option<usize> {
        self.options.iter().enumerate().find_map(|(index, _)| {
            self.dropdown_item_rect(index)
                .contains(point)
                .then_some(index)
        })
    }

    fn resolved_font<'a>(&self, assets: &'a Assets) -> Option<&'a fontdue::Font> {
        if let Some(font) = assets.fonts.get(self.font.trim()) {
            Some(font)
        } else {
            Widget::fallback_font()
        }
    }

    pub fn draw(
        &self,
        buffer: &mut TheRGBABuffer,
        assets: &Assets,
        draw2d: &Draw2D,
        cursor: Vec2<f32>,
    ) {
        let Some(font) = self.resolved_font(assets) else {
            return;
        };
        match self.kind {
            ChoiceWidgetKind::TabBar => {
                for (index, option) in self.options.iter().enumerate() {
                    let rect = self.tab_rect(index);
                    let selected = index == self.selected;
                    let hovered = rect.contains(cursor);
                    let background = if selected {
                        self.selected_color
                    } else if hovered {
                        self.hover_color
                    } else {
                        self.background_color
                    };
                    self.draw_item(
                        buffer,
                        draw2d,
                        font,
                        rect,
                        &option.label,
                        background,
                        selected,
                    );
                }
            }
            ChoiceWidgetKind::Dropdown => {
                let hovered = self.rect.contains(cursor);
                let background = if self.open {
                    self.selected_color
                } else if hovered {
                    self.hover_color
                } else {
                    self.background_color
                };
                let label = self
                    .options
                    .get(self.selected)
                    .map(|option| option.label.as_str())
                    .unwrap_or("");
                self.draw_item(
                    buffer, draw2d, font, self.rect, label, background, self.open,
                );
                self.draw_dropdown_marker(buffer, draw2d, font);

                if self.open {
                    let popup = self.popup_rect();
                    self.fill_rect(buffer, draw2d, popup, self.panel_color);
                    self.outline_rect(buffer, draw2d, popup);
                    for (index, option) in self.options.iter().enumerate() {
                        let rect = self.dropdown_item_rect(index);
                        let selected = index == self.selected;
                        let hovered = rect.contains(cursor);
                        let background = if selected {
                            self.selected_color
                        } else if hovered {
                            self.hover_color
                        } else {
                            self.panel_color
                        };
                        self.draw_item(
                            buffer,
                            draw2d,
                            font,
                            rect,
                            &option.label,
                            background,
                            selected,
                        );
                    }
                }
            }
        }
    }

    fn draw_item(
        &self,
        buffer: &mut TheRGBABuffer,
        draw2d: &Draw2D,
        font: &fontdue::Font,
        rect: Rect,
        label: &str,
        background: Pixel,
        selected: bool,
    ) {
        self.fill_rect(buffer, draw2d, rect, background);
        self.outline_rect(buffer, draw2d, rect);
        let safe = (
            0,
            0,
            buffer.dim().width.max(0) as isize,
            buffer.dim().height.max(0) as isize,
        );
        let stride = buffer.stride();
        let text_width = (rect.width - self.text_padding * 2.0).round().max(1.0) as isize;
        // Draw2D's ellipsis loop expects enough room for at least "...".
        if text_width > draw2d.get_text_size(font, self.font_size, "...").0 as isize {
            draw2d.text_rect_blend_safe(
                buffer.pixels_mut(),
                &(
                    (rect.x + self.text_padding).round() as isize,
                    rect.y.round() as isize,
                    text_width,
                    rect.height.round().max(1.0) as isize,
                ),
                stride,
                font,
                self.font_size,
                label,
                if selected {
                    &self.text_color
                } else {
                    &self.muted_text_color
                },
                if self.kind == ChoiceWidgetKind::TabBar {
                    draw2d::TheHorizontalAlign::Center
                } else {
                    draw2d::TheHorizontalAlign::Left
                },
                draw2d::TheVerticalAlign::Center,
                &safe,
            );
        }
        if selected && self.indicator_size > 0.0 {
            self.fill_rect(
                buffer,
                draw2d,
                Rect::new(
                    rect.x,
                    rect.y + rect.height - self.indicator_size,
                    rect.width,
                    self.indicator_size,
                ),
                self.indicator_color,
            );
        }
    }

    fn draw_dropdown_marker(
        &self,
        buffer: &mut TheRGBABuffer,
        draw2d: &Draw2D,
        font: &fontdue::Font,
    ) {
        let safe = (
            0,
            0,
            buffer.dim().width.max(0) as isize,
            buffer.dim().height.max(0) as isize,
        );
        let stride = buffer.stride();
        let marker_width = self.rect.height.max(18.0);
        draw2d.text_rect_blend_safe(
            buffer.pixels_mut(),
            &(
                (self.rect.x + self.rect.width - marker_width).round() as isize,
                self.rect.y.round() as isize,
                marker_width.round() as isize,
                self.rect.height.round() as isize,
            ),
            stride,
            font,
            self.font_size,
            if self.open { "▴" } else { "▾" },
            &self.text_color,
            draw2d::TheHorizontalAlign::Center,
            draw2d::TheVerticalAlign::Center,
            &safe,
        );
    }

    fn fill_rect(&self, buffer: &mut TheRGBABuffer, draw2d: &Draw2D, rect: Rect, color: Pixel) {
        let safe = (
            0,
            0,
            buffer.dim().width.max(0) as isize,
            buffer.dim().height.max(0) as isize,
        );
        let stride = buffer.stride();
        draw2d.blend_rect_safe(
            buffer.pixels_mut(),
            &(
                rect.x.round() as isize,
                rect.y.round() as isize,
                rect.width.round().max(1.0) as isize,
                rect.height.round().max(1.0) as isize,
            ),
            stride,
            &color,
            &safe,
        );
    }

    fn outline_rect(&self, buffer: &mut TheRGBABuffer, draw2d: &Draw2D, rect: Rect) {
        if self.border_size <= 0 {
            return;
        }
        let x = rect.x.max(0.0).round() as usize;
        let y = rect.y.max(0.0).round() as usize;
        let right = (rect.x + rect.width)
            .ceil()
            .max(0.0)
            .min(buffer.dim().width.max(0) as f32) as usize;
        let bottom = (rect.y + rect.height)
            .ceil()
            .max(0.0)
            .min(buffer.dim().height.max(0) as f32) as usize;
        let width = right.saturating_sub(x);
        let height = bottom.saturating_sub(y);
        if width == 0 || height == 0 {
            return;
        }
        let stride = buffer.stride();
        draw2d.rect_outline_thickness(
            buffer.pixels_mut(),
            &(x, y, width, height),
            stride,
            &self.border_color,
            self.border_size as usize,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_bar_parses_string_and_table_options() {
        let ui = r#"
binding = "inventory.category"
default = "weapons"
tabs = ["All", { label = "Weapons", value = "weapons" }]
"#
        .parse::<Table>()
        .unwrap();
        let widget = ChoiceWidget::from_ui(
            "Categories".to_string(),
            1,
            Rect::new(0.0, 0.0, 200.0, 30.0),
            ChoiceWidgetKind::TabBar,
            &ui,
        )
        .unwrap();
        assert_eq!(widget.options.len(), 2);
        assert_eq!(widget.selected_value(), "weapons");
        assert!(widget.tab_rect(1).width > widget.tab_rect(0).width);
    }

    #[test]
    fn dropdown_selects_open_item() {
        let ui = r#"
binding = "inventory.sort"
options = ["Newest", "Name"]
"#
        .parse::<Table>()
        .unwrap();
        let mut widget = ChoiceWidget::from_ui(
            "Sort".to_string(),
            2,
            Rect::new(10.0, 10.0, 100.0, 30.0),
            ChoiceWidgetKind::Dropdown,
            &ui,
        )
        .unwrap();
        assert_eq!(
            widget.interact(Vec2::new(20.0, 20.0)),
            ChoiceInteraction::Consumed
        );
        assert_eq!(
            widget.interact(Vec2::new(20.0, 75.0)),
            ChoiceInteraction::Selected("name".to_string())
        );
    }
}
