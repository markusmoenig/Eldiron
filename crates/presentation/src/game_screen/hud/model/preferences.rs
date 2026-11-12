//! Preferences remain presentation-owned and share the acknowledged HUD stack.
use super::*;
use actor_panels::actor_frame;
use screen_model::{PreferenceSetting, TooltipDelay};
impl HudState {
    pub(in crate::game_screen::hud) fn settings(&self) -> screen_model::UiPreferences {
        self.preferences
    }
    pub(super) fn toggle_preferences(&mut self, command: &str) -> bool {
        if !matches!(
            command.trim().to_ascii_lowercase().as_str(),
            "preferences" | "settings"
        ) {
            return false;
        }
        let was_open = self.preferences_panel.is_some();
        self.close_all();
        if !was_open {
            self.serial = self
                .serial
                .checked_add(1)
                .expect("HUD incarnation exhausted");
            self.preferences_panel = Some(OpenActorPanel {
                incarnation: self.serial,
                position: self.panel_positions.get(&PanelKind::Preferences).copied(),
                scroll: [0.; 2],
            });
        }
        true
    }
    pub(super) fn activate_preferences(
        &mut self,
        action: &HudAction,
        elements: &mut ScreenElements,
    ) -> bool {
        if self.preferences_panel.is_none() {
            return false;
        }
        match action {
            HudAction::PreferencePopup(setting) => {
                self.preferences_popup = if self.preferences_popup == Some(*setting) {
                    None
                } else {
                    Some(*setting)
                };
            }
            HudAction::Tooltips(value) => {
                self.preferences.tooltips_enabled = *value;
                elements.set_binding("preferences.tooltips", if *value { "on" } else { "off" });
                self.preferences_popup = None;
            }
            HudAction::TooltipDelay(value) => {
                self.preferences.tooltip_delay = *value;
                elements.set_binding(
                    "preferences.tooltip_delay",
                    &value.label().to_ascii_lowercase(),
                );
                self.preferences_popup = None;
            }
            HudAction::ResetPositions => {
                self.panel_positions.clear();
                self.positions.clear();
                for open in [
                    &mut self.inventory,
                    &mut self.equipment,
                    &mut self.preferences_panel,
                ]
                .into_iter()
                .flatten()
                {
                    open.position = None;
                }
                if let Some(open) = &mut self.catalog {
                    open.panel.position = None;
                }
                if let Some(open) = &mut self.open {
                    open.position = open.default_position;
                }
                self.preferences_popup = None;
            }
            _ => return false,
        }
        true
    }
    pub(super) fn preferences_frame(&self, elements: &ScreenElements) -> Option<PanelFrame> {
        let open = self.preferences_panel.as_ref()?;
        let c = &elements.toolbar_preferences_panel_config;
        let size = elements.layout.viewport;
        let geometry = screen_model::preferences_panel_geometry(c, open.position, size);
        let mut frame = actor_frame(
            HudPanelId::Preferences {
                incarnation: open.incarnation,
            },
            geometry.rect,
            c.title_height,
            c.padding,
            c.title.clone(),
            PanelStyle::preferences(c),
            size,
        );
        for (text, bounds) in ["Tooltips", "Tooltip delay"]
            .into_iter()
            .zip(geometry.labels)
        {
            frame.labels.push(PanelLabel {
                text: text.into(),
                rect: bounds,
                size: c.font_size,
                color: c.muted_text_color,
                centered: false,
                content: false,
            });
        }
        let mut option = |part, bounds, action, text: &str, selected, popup| {
            frame.button(
                part,
                bounds,
                action,
                text.into(),
                [13, 16, 16, 245],
                selected,
                popup,
            )
        };
        option(
            HudPart::Preference(PreferenceSetting::Tooltips),
            geometry.tooltips,
            HudAction::PreferencePopup(PreferenceSetting::Tooltips),
            if self.preferences.tooltips_enabled {
                "On"
            } else {
                "Off"
            },
            false,
            false,
        );
        option(
            HudPart::Preference(PreferenceSetting::TooltipDelay),
            geometry.delay,
            HudAction::PreferencePopup(PreferenceSetting::TooltipDelay),
            self.preferences.tooltip_delay.label(),
            false,
            false,
        );
        option(
            HudPart::ResetPositions,
            geometry.reset,
            HudAction::ResetPositions,
            "Reset floating window positions",
            false,
            false,
        );
        if let Some(setting) = self.preferences_popup {
            let (anchor, count) = match setting {
                PreferenceSetting::Tooltips => (geometry.tooltips, 2),
                PreferenceSetting::TooltipDelay => (geometry.delay, 3),
            };
            let height = (c.row_height * count as f32).min(frame.rect.height);
            let row_height = height / count as f32;
            let y = (anchor.y + anchor.height)
                .min(frame.rect.y + frame.rect.height - height)
                .max(frame.rect.y);
            let popup = Rect::new(anchor.x, y, anchor.width, height);
            frame.popup = Some(popup);
            for index in 0..count {
                let bounds = Rect::new(
                    popup.x,
                    y + index as f32 * row_height,
                    popup.width,
                    row_height,
                );
                let (action, text, selected) = match setting {
                    PreferenceSetting::Tooltips => {
                        let value = index == 0;
                        (
                            HudAction::Tooltips(value),
                            if value { "On" } else { "Off" },
                            self.preferences.tooltips_enabled == value,
                        )
                    }
                    PreferenceSetting::TooltipDelay => {
                        let value = [
                            TooltipDelay::Instant,
                            TooltipDelay::Short,
                            TooltipDelay::Normal,
                        ][index];
                        (
                            HudAction::TooltipDelay(value),
                            value.label(),
                            self.preferences.tooltip_delay == value,
                        )
                    }
                };
                frame.button(
                    HudPart::PreferenceOption(setting, index),
                    bounds,
                    action,
                    text.into(),
                    [13, 16, 16, 255],
                    selected,
                    true,
                );
            }
        }
        Some(frame)
    }
}
