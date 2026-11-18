//! UI hover uses the same acknowledged owner stack as clicks and drops.
use super::*;
use rusterix::client::{
    command::{ClientCommandBinding, parse_client_command},
    rules_ui,
};
#[derive(Clone, Debug, PartialEq)]
enum Source {
    Item(item_pointer::ItemSlot),
    Command { command: String, actor: Option<u32> },
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Target {
    owner: DropOwner,
    rect: Rect,
    source: Source,
    below: bool,
    action: Option<super::super::hud::model::HudAction>,
}
impl GameScreenController {
    pub(super) fn hover_targets(&self, world: &RuntimeView<'_>) -> Vec<Target> {
        let mut targets: Vec<_> = self
            .item_slots(world)
            .into_iter()
            .map(|slot| Target {
                owner: match slot.control {
                    ItemControl::Button(id) => DropOwner::Button(id),
                    ItemControl::Hud(id) => DropOwner::Hud(id),
                },
                rect: slot.rect,
                source: Source::Item(slot),
                below: false,
                action: None,
            })
            .collect();
        let map = world.map.unwrap_or(&self.empty_map);
        for id in screen_model::ordered_button_ids(&self.elements.button_widgets) {
            let widget = &self.elements.button_widgets[&id];
            if targets
                .iter()
                .any(|target| target.owner == DropOwner::Button(id))
            {
                continue;
            }
            let actor = world.party_entity(map, widget.party.as_deref());
            if let Some(command) = screen_model::resolved_widget_command(
                widget,
                world.assets,
                actor,
                &self.elements.ui_state,
            ) {
                if matches!(
                    parse_client_command(&command),
                    Some(ClientCommandBinding::Control(_)) | None
                ) {
                    continue;
                }
                targets.push(Target {
                    owner: DropOwner::Button(id),
                    rect: widget.rect,
                    source: Source::Command {
                        command,
                        actor: actor.map(|actor| actor.id),
                    },
                    below: true,
                    action: None,
                });
            }
        }
        for panel in self.hud.frames(world, &self.elements) {
            if panel.detail.is_some() {
                continue;
            }
            let actor = match panel.id {
                super::super::hud::HudPanelId::Catalog { entity, .. } => entity,
                _ => None,
            };
            for card in &panel.cards {
                targets.push(Target {
                    owner: DropOwner::Hud(
                        panel.control(super::super::hud::HudPart::CatalogEntry(card.index)),
                    ),
                    rect: card.hit,
                    source: Source::Command {
                        command: card.command.clone(),
                        actor,
                    },
                    below: false,
                    action: Some(card.action.clone()),
                });
            }
        }
        targets
    }
    pub(in crate::game_screen) fn hover_content(
        &self,
        world: &RuntimeView<'_>,
    ) -> Option<super::super::tooltip::Content> {
        if !self.presentation_frame_matches(world)
            || self.pointer_modal()
            || self.pointer_capture.is_some()
            || self.keyboard_capture.is_some()
            || self.world.capture.is_some()
            || self.world.ui_item.is_some()
            || self.hud.gesturing()
        {
            return None;
        }
        let point = self.pointer_visual_position()?;
        let target = self.acknowledged_hover_target(world, point)?;
        let frame = self.world.displayed.as_ref()?;
        let (mut description, state, command) = match &target.source {
            Source::Item(slot) => (
                rules_ui::describe_item(slot.source.resolve(world.map?)?, world.assets),
                None,
                None,
            ),
            Source::Command { command, actor } => {
                let actor =
                    actor.and_then(|id| world.map?.entities.iter().find(|actor| actor.id == id));
                (
                    rules_ui::describe_command(world.assets, actor, command),
                    Some(rules_ui::command_state(world.assets, actor, command)),
                    Some(command),
                )
            }
        };
        if let Some(labels) = command.and_then(|command| self.tooltip_shortcuts.get(command)) {
            rules_ui::add_shortcut_line(&mut description, labels.clone());
        }
        (!description.title.trim().is_empty()).then(|| super::super::tooltip::Content {
            key: (
                frame.binding,
                format!("{:?}/{:?}/{:?}", target.owner, target.source, target.action),
            ),
            description,
            state,
            anchor: target.rect,
            below: target.below,
        })
    }
}

impl GameScreenController {
    fn acknowledged_hover_target(
        &self,
        world: &RuntimeView<'_>,
        point: [f32; 2],
    ) -> Option<&Target> {
        if !self.presentation_frame_matches(world) || self.pointer_modal() {
            return None;
        }
        let frame = self.world.displayed.as_ref()?;
        let owner = frame
            .controls
            .iter()
            .rev()
            .find(|control| control.rect.contains(point))?
            .owner?;
        if self
            .pointer_controls(world)
            .iter()
            .rev()
            .find(|control| control.rect.contains(point))?
            .owner
            != Some(owner)
        {
            return None;
        }
        let target = frame
            .hovers
            .iter()
            .find(|target| target.owner == owner && rect(target.rect).contains(point))?;
        if !self.hover_targets(world).contains(target) {
            return None;
        }
        Some(target)
    }
}

impl GameScreenController {
    /// Only the reference Drop intent targets carried-item controls. Container
    /// children keep their addressed pickup behavior and never borrow this cursor.
    pub(in crate::game_screen) fn ui_item_cursor_target(
        &self,
        world: &RuntimeView<'_>,
    ) -> Option<(bool, bool, bool)> {
        if !self.ui_frame_matches(world)
            || self.pointer_modal()
            || !screen_model::owned_item_cursor_intent(
                screen_model::selected_intent(
                    &self.elements.activated_widgets,
                    &self.elements.button_widgets,
                    &self.intent,
                )
                .as_deref(),
            )
        {
            return None;
        }
        let point = self.pointer_visual_position()?;
        if self.pressed_owned_item_cursor(world) {
            return Some((false, true, true));
        }
        if self.pointer_capture.is_some()
            || self.keyboard_capture.is_some()
            || self.world.ui_item.is_some()
            || self.hud.gesturing()
        {
            return None;
        }
        let target = self.acknowledged_hover_target(world, point)?;
        match &target.source {
            Source::Item(slot) if matches!(slot.source,rusterix::item_interaction::ItemSource::Direct(address) if address.owner.is_some()) => {
                Some((false, true, false))
            }
            _ => None,
        }
    }
}
