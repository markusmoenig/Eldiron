//! Shared panel paint metadata. Family configuration stays outside the painter.
use super::*;
#[derive(Clone, PartialEq)]
pub(in crate::game_screen) struct PanelStyle {
    pub background: [u8; 4],
    pub title_background: [u8; 4],
    pub border: [u8; 4],
    pub text: [u8; 4],
    pub muted: [u8; 4],
    pub slot: [u8; 4],
    pub slot_border: [u8; 4],
    pub occupied: [u8; 4],
    pub selected: [u8; 4],
    pub font: String,
    pub title_font: String,
    pub border_width: f32,
    pub slot_border_width: f32,
    pub font_size: f32,
    pub title_size: f32,
    pub quantity_size: f32,
    pub scroll_step: f32,
}
impl PanelStyle {
    pub fn preferences(c: &screen_model::PreferencesPanelConfig) -> Self {
        Self {
            background: c.background_color,
            title_background: c.title_background_color,
            border: c.border_color,
            text: c.text_color,
            muted: c.muted_text_color,
            slot: [13, 16, 16, 245],
            slot_border: c.border_color,
            occupied: c.border_color,
            selected: [238, 214, 118, 255],
            font: family(&c.font),
            title_font: family(&c.font),
            border_width: 1.,
            slot_border_width: 1.,
            font_size: c.font_size,
            title_size: c.title_font_size,
            quantity_size: c.font_size,
            scroll_step: c.row_height,
        }
    }
    pub fn catalog(c: &screen_model::CatalogPanelConfig) -> Self {
        Self {
            background: c.background_color,
            title_background: c.title_background_color,
            border: c.border_color,
            text: c.text_color,
            muted: c.muted_text_color,
            slot: c.slot_background_color,
            slot_border: c.slot_border_color,
            occupied: c.slot_border_color,
            selected: [238, 214, 118, 255],
            font: family(&c.font),
            title_font: family(if c.title_font.trim().is_empty() {
                &c.font
            } else {
                &c.title_font
            }),
            border_width: c.border_size as f32,
            slot_border_width: c.slot_border_size as f32,
            font_size: c.font_size,
            title_size: c.title_font_size,
            quantity_size: c.small_font_size,
            scroll_step: (c.cell_size + c.spacing).max(8.),
        }
    }

    pub fn container(t: &rusterix::client::rules_ui::ContainerUiTemplate) -> Self {
        Self {
            background: t.background_color,
            title_background: [20, 24, 30, 220],
            border: t.border_color,
            text: [236, 233, 214, 255],
            muted: [156, 149, 130, 255],
            slot: t.slot_color,
            slot_border: t.slot_border_color,
            occupied: t.slot_border_color,
            selected: [238, 214, 118, 255],
            font: "Roboto".into(),
            title_font: "Roboto".into(),
            border_width: 1.,
            slot_border_width: 1.,
            font_size: 14.,
            title_size: 14.,
            quantity_size: 10.,
            scroll_step: t.slot_size.max(8) as f32 + t.gap.max(0) as f32,
        }
    }
    pub fn inventory(c: &screen_model::InventoryPanelConfig) -> Self {
        Self {
            background: c.background_color,
            title_background: c.title_background_color,
            border: c.border_color,
            text: c.text_color,
            muted: c.muted_text_color,
            slot: c.slot_background_color,
            slot_border: c.slot_border_color,
            occupied: c.slot_border_color,
            selected: c.selected_slot_color,
            font: family(&c.font),
            title_font: family(&c.font),
            border_width: 1.,
            slot_border_width: 1.,
            font_size: c.font_size,
            title_size: c.title_font_size,
            quantity_size: 10.,
            scroll_step: (c.cell_size + c.spacing).max(8.),
        }
    }
    pub fn equipment(c: &screen_model::EquipmentPanelConfig) -> Self {
        Self {
            background: c.background_color,
            title_background: c.title_background_color,
            border: c.border_color,
            text: c.text_color,
            muted: c.muted_text_color,
            slot: c.slot_background_color,
            slot_border: c.slot_border_color,
            occupied: c.occupied_slot_color,
            selected: c.occupied_slot_color,
            font: family(&c.font),
            title_font: family(&c.font),
            border_width: 1.,
            slot_border_width: 1.,
            font_size: c.font_size,
            title_size: c.title_font_size,
            quantity_size: 10.,
            scroll_step: (c.slot_size + c.spacing).max(8.),
        }
    }
}
fn family(font: &str) -> String {
    if font.trim().is_empty() {
        "Roboto".into()
    } else {
        font.into()
    }
}
