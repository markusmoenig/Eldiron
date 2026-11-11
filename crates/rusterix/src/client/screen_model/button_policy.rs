//! Shared authored-button state transitions. Command execution, targeting and
//! window/runtime effects belong to the containing application controller.
use super::*;
use crate::client::command::ClientCommandBinding;

pub fn ordered_button_ids(buttons: &FxHashMap<u32, Widget>) -> Vec<u32> {
    let mut ids: Vec<_> = buttons.keys().copied().collect();
    ids.sort_unstable();
    ids
}
/// Hit order matches screen painting: the highest visible authored ID is on top.
pub fn topmost_button_at(
    buttons: &FxHashMap<u32, Widget>,
    hidden: &[String],
    point: Vec2<f32>,
) -> Option<u32> {
    buttons
        .iter()
        .filter(|(_, w)| !widget_name_is_hidden(&w.name, hidden) && w.rect.contains(point))
        .map(|(id, _)| *id)
        .max()
}
pub fn is_2d_camera(camera: &PlayerCamera) -> bool {
    matches!(
        camera,
        PlayerCamera::D2 | PlayerCamera::D2Grid | PlayerCamera::D2Iso | PlayerCamera::D2IsoGrid
    )
}
pub fn immediate_2d_intent_camera(camera: Option<PlayerCamera>, persistent: bool) -> bool {
    !persistent && camera.as_ref().is_some_and(is_2d_camera)
}
/// A stale 2D camera must not turn a 3D/persistent targeting selection into a
/// one-shot server intent while the player and view update on adjacent frames.
pub fn immediate_2d_intents(
    player: Option<PlayerCamera>,
    widget: Option<PlayerCamera>,
    persistent: bool,
) -> bool {
    if persistent
        || player.as_ref().is_some_and(|c| !is_2d_camera(c))
        || widget.as_ref().is_some_and(|c| !is_2d_camera(c))
    {
        return false;
    }
    immediate_2d_intent_camera(player.or(widget), persistent)
}
pub fn is_targeting_button(widget: &Widget) -> bool {
    widget.command_slot.is_some()
        || matches!(
            widget.command_binding(),
            Some(ClientCommandBinding::Intent(_) | ClientCommandBinding::RulesAction(_))
        )
}
pub fn select_targeting_button(
    buttons: &FxHashMap<u32, Widget>,
    active: &mut Vec<u32>,
    permanent: &mut Vec<u32>,
    button: u32,
) {
    for list in [active, permanent] {
        list.retain(|id| *id != button && buttons.get(id).is_none_or(|w| !is_targeting_button(w)));
        list.push(button);
    }
}
pub fn apply_button_visibility(widget: &Widget, hidden: &mut Vec<String>) {
    if let Some(hide) = &widget.hide {
        hidden.clone_from(hide);
    }
    if let Some(show) = &widget.show {
        hidden.retain(|token| !show.contains(token));
    }
}
pub fn apply_button_deactivation(
    buttons: &FxHashMap<u32, Widget>,
    active: &mut Vec<u32>,
    permanent: &mut Vec<u32>,
    button: u32,
) {
    let Some(widget) = buttons.get(&button) else {
        return;
    };
    if widget.deactivate.is_empty() {
        return;
    }
    for list in [active, permanent] {
        list.retain(|id| {
            *id != button
                && buttons.get(id).is_none_or(|w| {
                    !widget
                        .deactivate
                        .iter()
                        .any(|token| deactivate_matches(w, token))
                })
        });
        list.push(button);
    }
}
pub fn button_binding_value(
    state: &FxHashMap<String, String>,
    binding: &str,
    value: &str,
    append: bool,
    separator: &str,
    max_parts: Option<usize>,
) -> String {
    let value = value.trim();
    if !append || value.is_empty() {
        return value.to_owned();
    }
    let separator = if separator.is_empty() { " " } else { separator };
    let mut parts: Vec<_> = state
        .get(binding.trim())
        .map(|s| {
            s.split(separator)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    parts.push(value.to_owned());
    if let Some(max_parts) = max_parts.filter(|n| *n > 0) {
        if parts.len() > max_parts {
            parts.drain(0..parts.len() - max_parts);
        }
    }
    parts.join(separator)
}
impl ScreenElements {
    pub fn set_or_append_binding(
        &mut self,
        binding: &str,
        value: &str,
        append: bool,
        separator: &str,
        max_parts: Option<usize>,
    ) {
        let value =
            button_binding_value(&self.ui_state, binding, value, append, separator, max_parts);
        self.set_binding(binding.trim(), &value);
    }
    /// Apply presentation/UI state only, after the caller resolves the command
    /// against the pre-press bindings and checks its enabled state.
    pub fn press_button_state(&mut self, button: u32) -> bool {
        let Some(widget) = self.button_widgets.get(&button) else {
            return false;
        };
        if widget_name_is_hidden(&widget.name, &self.widgets_to_hide) {
            return false;
        }
        let binding = widget
            .binding
            .as_ref()
            .zip(widget.value.as_ref())
            .map(|(binding, value)| {
                (
                    binding.clone(),
                    value.clone(),
                    widget.binding_append,
                    widget.binding_separator.clone(),
                    widget.binding_max_parts,
                )
            });
        let targeting = is_targeting_button(widget);
        apply_button_visibility(widget, &mut self.widgets_to_hide);
        if !self.activated_widgets.contains(&button) {
            self.activated_widgets.push(button);
        }
        apply_button_deactivation(
            &self.button_widgets,
            &mut self.activated_widgets,
            &mut self.permanently_activated_widgets,
            button,
        );
        if targeting {
            select_targeting_button(
                &self.button_widgets,
                &mut self.activated_widgets,
                &mut self.permanently_activated_widgets,
                button,
            );
        }
        if let Some((binding, value, append, separator, max_parts)) = binding {
            self.set_or_append_binding(&binding, &value, append, &separator, max_parts);
        }
        true
    }
    pub fn release_button_state(&mut self) {
        self.activated_widgets
            .clone_from(&self.permanently_activated_widgets);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn button(id: u32, command: &str) -> Widget {
        let mut widget = Widget::new();
        widget.id = id;
        widget.name = format!("Button{id}");
        widget.command = Some(command.into());
        widget.rect = Rect::new(10., 10., 80., 30.);
        widget
    }
    #[test]
    fn pointer_stacking_binding_append_visibility_and_activation_share_one_model() {
        let mut screen = ScreenElements::default();
        let mut first = button(1, "intent.attack");
        first.group = Some("combat".into());
        screen.button_widgets.insert(1, first);
        screen.button_widgets.insert(2, button(2, "intent."));
        let mut rune = button(3, "game.start");
        rune.binding = Some("runes".into());
        rune.value = Some(" LO ".into());
        rune.binding_append = true;
        rune.binding_max_parts = Some(2);
        rune.hide = Some(vec!["Panel*".into(), "Details".into()]);
        rune.show = Some(vec!["Details".into()]);
        rune.deactivate = vec!["COMBAT".into()];
        screen.button_widgets.insert(3, rune);
        assert_eq!(
            topmost_button_at(&screen.button_widgets, &[], Vec2::new(20., 20.)),
            Some(3)
        );
        screen.press_button_state(1);
        assert!(screen.permanently_activated_widgets.contains(&1));
        screen.press_button_state(2);
        assert_eq!(screen.permanently_activated_widgets, vec![2]);
        for _ in 0..4 {
            screen.press_button_state(3);
        }
        assert_eq!(screen.ui_state["runes"], "LO LO");
        assert_eq!(screen.widgets_to_hide, vec!["Panel*"]);
        assert_eq!(
            screen
                .permanently_activated_widgets
                .iter()
                .filter(|id| **id == 3)
                .count(),
            1
        );
        assert_eq!(
            screen
                .activated_widgets
                .iter()
                .filter(|id| **id == 3)
                .count(),
            1
        );
        screen.release_button_state();
        assert_eq!(
            screen.activated_widgets,
            screen.permanently_activated_widgets
        );
        screen.widgets_to_hide.push("Button3".into());
        assert_eq!(
            topmost_button_at(
                &screen.button_widgets,
                &screen.widgets_to_hide,
                Vec2::new(20., 20.)
            ),
            Some(2)
        );
        assert!(!screen.press_button_state(3));
    }
    #[test]
    fn command_binding_values_keep_legacy_append_and_clear_semantics() {
        let mut screen = ScreenElements::default();
        screen.set_or_append_binding(" spell ", "LO", true, "", Some(2));
        screen.set_or_append_binding("spell", "VI", true, "", Some(2));
        screen.set_or_append_binding("spell", "RA", true, "", Some(2));
        assert_eq!(screen.ui_state["spell"], "VI RA");
        screen.set_or_append_binding("spell", "", true, "", Some(2));
        assert_eq!(screen.ui_state["spell"], "");
        screen.set_or_append_binding("spell", "  日本  ", false, "/", None);
        screen.set_or_append_binding("spell", "東京", true, "/", Some(0));
        assert_eq!(screen.ui_state["spell"], "日本/東京");
    }
}
