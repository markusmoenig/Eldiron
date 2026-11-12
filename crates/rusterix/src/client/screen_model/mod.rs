//! Authored screen compilation and layout, independent of a window, server,
//! scene VM or destination bitmap. Legacy allocation/building is an adapter.
use crate::client::{
    command::command_from_legacy_fields,
    widget::{
        self, BorderGradientDirection, ButtonStateStyle, TextInputWidget, Widget,
        avatar::AvatarWidget,
        choice::{ChoiceOption, ChoiceWidget, ChoiceWidgetKind},
        deco::DecoWidget,
        game::GameWidget,
        messages::MessagesWidget,
        profile::ProfileWidget,
        stat::StatWidget,
        text::TextWidget,
    },
};
use crate::{Assets, Map, Pixel, PixelSource, PlayerCamera, Rect, Texture, Value};
use theframework::prelude::*;
use toml::Table;
mod artwork;
mod bindings;
mod button_policy;
pub use bindings::*;
pub use button_policy::*;
mod choice_controller;
mod container_layout;
mod inventory;
pub use container_layout::*;
pub use inventory::*;
mod compile;
mod cursor;
pub use choice_controller::*;
pub use cursor::*;
mod layout;
mod panels;
mod style;
pub(crate) use artwork::*;
pub use layout::*;
pub use panels::*;
pub(crate) use style::*;

#[derive(Default)]
pub struct ScreenElements {
    pub layout: ScreenLayout,
    fallback_game: bool,
    pub source: Map,
    pub background: Pixel,
    pub animation_frame: usize,
    pub game_widgets: FxHashMap<Uuid, GameWidget>,
    pub button_widgets: FxHashMap<u32, Widget>,
    pub action_bar_button_ids: FxHashMap<u32, Vec<u32>>,
    pub avatar_widgets: FxHashMap<Uuid, AvatarWidget>,
    pub profile_widgets: FxHashMap<Uuid, ProfileWidget>,
    pub stat_widgets: FxHashMap<Uuid, StatWidget>,
    pub text_widgets: FxHashMap<Uuid, TextWidget>,
    pub text_input_widgets: FxHashMap<u32, TextInputWidget>,
    pub choice_widgets: FxHashMap<u32, ChoiceWidget>,
    pub deco_widgets: FxHashMap<Uuid, DecoWidget>,
    pub messages_widgets: Vec<MessagesWidget>,
    pub activated_widgets: Vec<u32>,
    pub permanently_activated_widgets: Vec<u32>,
    pub ui_state: FxHashMap<String, String>,
    pub widgets_to_hide: Vec<String>,
    pub custom_inventory_panel_config: Option<InventoryPanelConfig>,
    pub custom_equipment_panel_config: Option<EquipmentPanelConfig>,
    pub custom_actions_panel_config: Option<CatalogPanelConfig>,
    pub custom_spellbook_config: Option<CatalogPanelConfig>,
    pub toolbar_inventory_panel_config: InventoryPanelConfig,
    pub toolbar_equipment_panel_config: EquipmentPanelConfig,
    pub toolbar_preferences_panel_config: PreferencesPanelConfig,
    pub toolbar_actions_panel_config: CatalogPanelConfig,
    pub toolbar_spellbook_config: CatalogPanelConfig,
}

pub(crate) fn sync_bound_button_activation(
    button_widgets: &FxHashMap<u32, Widget>,
    ui_state: &FxHashMap<String, String>,
    activated_widgets: &mut Vec<u32>,
    permanently_activated_widgets: &mut Vec<u32>,
    binding: &str,
) {
    let selected_value = ui_state.get(binding).map(|value| value.trim());
    let mut selected_button_id = None;

    for (id, widget) in button_widgets.iter() {
        if widget.binding.as_deref() != Some(binding) {
            continue;
        }

        let single_selection = widget
            .selection
            .as_deref()
            .map(|selection| selection.eq_ignore_ascii_case("single"))
            .unwrap_or_else(|| widget.group.is_some());
        if !single_selection {
            continue;
        }

        activated_widgets.retain(|active_id| active_id != id);
        permanently_activated_widgets.retain(|active_id| active_id != id);

        if let (Some(selected), Some(value)) = (selected_value, widget.value.as_deref())
            && value.trim().eq_ignore_ascii_case(selected)
        {
            selected_button_id = Some(*id);
        }
    }

    if let Some(id) = selected_button_id {
        if !activated_widgets.contains(&id) {
            activated_widgets.push(id);
        }
        if !permanently_activated_widgets.contains(&id) {
            permanently_activated_widgets.push(id);
        }
    }
}

pub fn screen_background(config: &Table) -> Pixel {
    let viewport = config.get("viewport").and_then(toml::Value::as_table);
    let color = viewport
        .and_then(|v| {
            v.get("screen_background")
                .and_then(toml::Value::as_str)
                .or_else(|| v.get("background_color_2d").and_then(toml::Value::as_str))
        })
        .unwrap_or("#000000");
    hex_to_rgba_u8(color)
}

pub(crate) fn sector_ui_role(sector: &crate::Sector) -> Option<String> {
    let crate::Value::Str(data) = sector.properties.get("data")? else {
        return None;
    };
    let table = data.parse::<Table>().ok()?;
    table
        .get("ui")
        .and_then(toml::Value::as_table)
        .and_then(|ui| ui.get("role"))
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|role| !role.is_empty() && !role.eq_ignore_ascii_case("none"))
        .map(str::to_string)
}

pub(crate) fn screen_static_layer_render_map(screen: &Map, foreground: bool) -> Map {
    let mut map = screen.clone();
    map.sectors.retain(|sector| {
        if sector_ui_role(sector).is_some() {
            return false;
        }

        let layer = sector.properties.get_int_default("layer", 0);
        if foreground { layer > 0 } else { layer <= 0 }
    });
    map
}

impl ScreenElements {
    /// A region without authored screen UI still uses the same game-view model.
    /// Geometry follows the caller's fixed/responsive layout policy; no display
    /// bitmap, font or GPU resources are allocated here.
    pub fn region_fallback(
        name: &str,
        layout: ScreenLayout,
        background: Pixel,
        ui_state: FxHashMap<String, String>,
    ) -> Self {
        let mut model = Self {
            layout,
            background,
            ui_state,
            fallback_game: true,
            ..Default::default()
        };
        let mut widget = GameWidget {
            name: name.into(),
            rect: Rect::new(
                0.,
                0.,
                layout.viewport.x.max(0) as f32,
                layout.viewport.y.max(0) as f32,
            ),
            toml_str: "[ui]\nrole = \"game\"\n".into(),
            grid_size: layout.grid_size,
            ..Default::default()
        };
        widget.init();
        model.game_widgets.insert(Uuid::nil(), widget);
        model
    }
    /// User-entered values preserve whitespace. Command placeholder consumers
    /// may normalize their own values; this does not mutate authored sources.
    pub fn set_binding(&mut self, binding: &str, value: &str) {
        if binding.trim().is_empty() {
            return;
        }
        let binding = binding.trim();
        self.ui_state.insert(binding.to_owned(), value.to_owned());
        for input in self
            .text_input_widgets
            .values_mut()
            .filter(|w| w.binding.trim() == binding)
        {
            input.text = value.to_owned();
        }
        sync_bound_button_activation(
            &self.button_widgets,
            &self.ui_state,
            &mut self.activated_widgets,
            &mut self.permanently_activated_widgets,
            binding,
        );
    }

    pub fn render_layer(&self, foreground: bool) -> Map {
        screen_static_layer_render_map(&self.source, foreground)
    }
    /// Resize geometry in place, preserving UI values and controller state.
    /// Fixed authored screens retain their reference viewport.
    pub fn relayout(&mut self, surface: Vec2<i32>) -> bool {
        if !self.layout.screen_responsive || self.layout.viewport == surface {
            return false;
        }
        self.layout.viewport = surface;
        if self.fallback_game {
            if let Some(widget) = self.game_widgets.get_mut(&Uuid::nil()) {
                widget.rect = Rect::new(0., 0., surface.x.max(0) as f32, surface.y.max(0) as f32);
                widget.force_dynamics_rebuild = true;
            }
            return true;
        }
        for ScreenPlacement {
            role,
            sector_id,
            creator_id,
            name: _,
            rect,
            table,
        } in placements(&self.source, self.layout)
        {
            match role.as_str() {
                "game" => {
                    if let Some(widget) = self.game_widgets.get_mut(&creator_id) {
                        widget.rect = rect;
                        widget.force_dynamics_rebuild = true;
                    }
                }
                "button" => {
                    if let Some(widget) = self.button_widgets.get_mut(&sector_id) {
                        widget.rect = rect;
                    }
                }
                "input" => {
                    if let Some(widget) = self.text_input_widgets.get_mut(&sector_id) {
                        widget.rect = rect;
                    }
                }
                "tab_bar" | "dropdown" => {
                    if let Some(widget) = self.choice_widgets.get_mut(&sector_id) {
                        widget.rect = rect;
                    }
                }
                "messages" => {
                    for widget in self
                        .messages_widgets
                        .iter_mut()
                        .filter(|w| w.id == sector_id)
                    {
                        widget.rect = rect;
                    }
                }
                "avatar" => {
                    if let Some(widget) = self.avatar_widgets.get_mut(&creator_id) {
                        widget.rect = rect;
                    }
                }
                "profile" => {
                    if let Some(widget) = self.profile_widgets.get_mut(&creator_id) {
                        widget.rect = rect;
                    }
                }
                "stat" => {
                    if let Some(widget) = self.stat_widgets.get_mut(&creator_id) {
                        widget.rect = rect;
                    }
                }
                "text" => {
                    if let Some(widget) = self.text_widgets.get_mut(&creator_id) {
                        widget.rect = rect;
                    }
                }
                "deco" => {
                    if let Some(widget) = self.deco_widgets.get_mut(&creator_id) {
                        widget.rect = rect;
                    }
                }
                "action_bar" => {
                    if let Some(widget) = self.deco_widgets.get_mut(&creator_id) {
                        widget.rect = rect;
                        widget.separators = action_bar_group_separators(rect, &table);
                    }
                    if let Some(ids) = self.action_bar_button_ids.get(&sector_id) {
                        for (id, rect) in ids.iter().zip(action_bar_slot_rects(rect, &table)) {
                            if let Some(widget) = self.button_widgets.get_mut(id) {
                                widget.rect = rect;
                            }
                        }
                    }
                }
                "spellbook" => {
                    if let Some(config) = &mut self.custom_spellbook_config {
                        config.authored_rect = Some(rect);
                    }
                }
                "actions" => {
                    if let Some(config) = &mut self.custom_actions_panel_config {
                        config.authored_rect = Some(rect);
                    }
                }
                "inventory" => {
                    if let Some(config) = &mut self.custom_inventory_panel_config {
                        config.authored_rect = Some(rect);
                    }
                }
                "equipment" => {
                    if let Some(config) = &mut self.custom_equipment_panel_config {
                        config.authored_rect = Some(rect);
                    }
                }
                _ => {}
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sector(map: &mut Map, id: u32, data: &str) -> Uuid {
        let x = id as f32 * 4.;
        let vertices =
            [(x, 0.), (x + 3., 0.), (x + 3., 2.), (x, 2.)].map(|(x, y)| map.add_vertex_at(x, y));
        let lines = (0..4)
            .map(|i| map.create_linedef_manual(vertices[i], vertices[(i + 1) % 4]))
            .collect();
        let mut sector = crate::Sector::new(id, lines);
        sector.name = format!("Element{id}");
        sector.properties.set("data", Value::Str(data.into()));
        let key = sector.creator_id;
        map.sectors.push(sector);
        key
    }
    fn layout(responsive: bool) -> ScreenLayout {
        ScreenLayout {
            screen_responsive: responsive,
            reference_viewport: Vec2::new(640, 360),
            viewport: Vec2::new(800, 600),
            grid_size: 16.,
        }
    }
    fn assert_no_display_pixels(model: &ScreenElements) {
        for w in model.game_widgets.values() {
            assert!(w.buffer.pixels().is_empty());
            assert!(w.upscale_buffer.pixels().is_empty());
            assert!(w.text_font.is_none());
        }
        for w in &model.messages_widgets {
            assert!(w.buffer.pixels().is_empty());
            assert!(w.font.is_none());
        }
        for w in model.avatar_widgets.values() {
            assert!(w.buffer.pixels().is_empty());
        }
        for w in model.profile_widgets.values() {
            assert!(w.buffer.pixels().is_empty());
        }
        for w in model.stat_widgets.values() {
            assert!(w.buffer.pixels().is_empty());
        }
        for w in model.text_widgets.values() {
            assert!(w.buffer.pixels().is_empty());
            assert!(w.font.is_none());
        }
        for w in model.deco_widgets.values() {
            assert!(w.buffer.pixels().is_empty());
        }
    }
    #[test]
    fn compiler_prepares_all_screen_families_without_display_storage_or_document_mutation() {
        let mut screen = Map::default();
        for (id, role) in [
            "game",
            "messages",
            "avatar",
            "profile",
            "stat",
            "text",
            "deco",
            "button",
            "input",
            "dropdown",
            "tab_bar",
            "inventory",
            "equipment",
            "actions",
            "spellbook",
        ]
        .iter()
        .enumerate()
        {
            let extra = match *role {
                "tab_bar" => "binding='category'\ntabs=['All', 'Magic']",
                "dropdown" => "binding='option'\noptions=['One', 'Two']",
                _ => "",
            };
            sector(
                &mut screen,
                id as u32,
                &format!("[ui]\nrole='{role}'\n{extra}"),
            );
        }
        let before = serde_json::to_string(&screen).unwrap();
        let assets = Assets::default();
        let model = ScreenElements::compile(
            &screen,
            &assets,
            layout(true),
            [10, 20, 30, 255],
            FxHashMap::default(),
            3,
        );
        assert_eq!(model.game_widgets.len(), 1);
        assert_eq!(model.messages_widgets.len(), 1);
        assert_eq!(model.avatar_widgets.len(), 1);
        assert_eq!(model.profile_widgets.len(), 1);
        assert_eq!(model.stat_widgets.len(), 1);
        assert_eq!(model.text_widgets.len(), 1);
        assert_eq!(model.deco_widgets.len(), 1);
        assert_eq!(model.button_widgets.len(), 1);
        assert_eq!(model.text_input_widgets.len(), 1);
        assert_eq!(model.choice_widgets.len(), 2);
        assert!(model.custom_inventory_panel_config.is_some());
        assert!(model.custom_equipment_panel_config.is_some());
        assert!(model.custom_actions_panel_config.is_some());
        assert!(model.custom_spellbook_config.is_some());
        assert_eq!(
            model.game_widgets.values().next().unwrap().rect,
            Rect::new(0., 0., 800., 600.)
        );
        assert_no_display_pixels(&model);
        assert_eq!(serde_json::to_string(&screen).unwrap(), before);
    }
    #[test]
    fn responsive_generated_action_bar_relayouts_without_resetting_ui_or_allocating_pixels() {
        let mut screen = Map::default();
        let bar = sector(
            &mut screen,
            1,
            "[ui]\nrole='action_bar'\nslot_size=40\nspacing=4\npadding=8\nbuttons=['intent.look', 'ui.inventory']",
        );
        sector(
            &mut screen,
            2,
            "[ui]\nrole='input'\nbind='start.name'\ndefault='Before'",
        );
        let assets = Assets::default();
        let mut model = ScreenElements::compile(
            &screen,
            &assets,
            layout(true),
            [0; 4],
            FxHashMap::default(),
            0,
        );
        let ids = model.action_bar_button_ids[&1].clone();
        let original = model.button_widgets[&ids[0]].rect;
        model
            .ui_state
            .insert("start.name".into(), "Typed value".into());
        model.text_input_widgets.get_mut(&2).unwrap().text = "Typed value".into();
        assert!(model.relayout(Vec2::new(1000, 700)));
        assert_eq!(model.action_bar_button_ids[&1], ids);
        assert_eq!(model.ui_state["start.name"], "Typed value");
        assert_eq!(model.text_input_widgets[&2].text, "Typed value");
        let resized = model.button_widgets[&ids[0]].rect;
        assert_eq!(
            (resized.x - original.x, resized.y - original.y),
            (100., 100.)
        );
        assert_eq!(
            model.deco_widgets[&bar].rect.y,
            model.button_widgets[&ids[0]].rect.y - 8.
        );
        assert!(!model.relayout(Vec2::new(1000, 700)));
        assert_no_display_pixels(&model);
    }
    #[test]
    fn render_layers_are_private_normalized_copies_and_fixed_screens_keep_authored_geometry() {
        let mut screen = Map::default();
        sector(&mut screen, 1, "[ui]\nrole='none'\nlayer=-1");
        sector(&mut screen, 2, "[ui]\nrole='none'\nlayer=2");
        sector(&mut screen, 3, "[ui]\nrole='button'\nlayer=3");
        let assets = Assets::default();
        let before = serde_json::to_string(&screen).unwrap();
        let mut fixed = layout(false);
        fixed.viewport = fixed.reference_viewport;
        let mut model =
            ScreenElements::compile(&screen, &assets, fixed, [0; 4], FxHashMap::default(), 0);
        assert_eq!(
            model
                .render_layer(false)
                .sectors
                .iter()
                .map(|s| s.id)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(
            model
                .render_layer(true)
                .sectors
                .iter()
                .map(|s| s.id)
                .collect::<Vec<_>>(),
            vec![2]
        );
        assert_eq!(serde_json::to_string(&screen).unwrap(), before);
        let authored = model.button_widgets[&3].rect;
        assert!(!model.relayout(Vec2::new(1600, 900)));
        assert_eq!(model.button_widgets[&3].rect, authored);
        assert_no_display_pixels(&model);
    }
}
