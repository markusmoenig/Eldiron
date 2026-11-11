//! Declarative screen activation. The application applies runtime/navigation/HUD
//! requests; this controller never calls a server or advances the world.
use super::*;
use rusterix::{
    EntityAction, Value,
    client::{
        command::{ClientCommandBinding, parse_client_command},
        rules_ui,
    },
};

#[derive(Clone, Debug, PartialEq)]
pub enum ScreenEffect {
    /// Game, screen-navigation and application/HUD commands.
    Command(ClientCommandBinding),
    Action(EntityAction),
    ToggleContainer {
        region: Uuid,
        item: u32,
        owner: u32,
        anchor: Uv,
    },
}
#[derive(Default, Debug, PartialEq)]
pub struct ScreenActivation {
    pub accepted: bool,
    pub effects: Vec<ScreenEffect>,
}
impl GameScreenController {
    fn immediate_intents(&self, world: &RuntimeView<'_>) -> Result<bool, String> {
        let config = world
            .config()
            .parse::<toml::Table>()
            .map_err(|e| e.to_string())?;
        let game = config.get("game");
        let persistent = [
            "persistent_intents",
            "click_intents_2d",
            "persistent_2d_intents",
        ]
        .iter()
        .any(|key| {
            game.and_then(|g| g.get(*key))
                .and_then(toml::Value::as_bool)
                .unwrap_or(false)
        });
        let player = world
            .map
            .and_then(|m| m.entities.iter().find(|e| e.is_player()))
            .and_then(|e| match e.attributes.get("player_camera") {
                Some(Value::PlayerCamera(camera)) => Some(camera.clone()),
                _ => None,
            });
        let widget = self
            .elements
            .game_widgets
            .values()
            .find(|w| !screen_model::is_2d_camera(&w.camera))
            .or_else(|| self.elements.game_widgets.values().next())
            .map(|w| w.camera.clone());
        Ok(screen_model::immediate_2d_intents(
            player, widget, persistent,
        ))
    }
    /// Keyboard/action-catalog bindings use the same authored button policy
    /// when a matching button exists. Otherwise they update targeting state and
    /// emit the command without inventing an invisible widget.
    pub fn activate_binding(
        &mut self,
        world: &RuntimeView<'_>,
        binding: ClientCommandBinding,
    ) -> Result<ScreenActivation, String> {
        let map = world.map.unwrap_or(&self.empty_map);
        let matched = screen_model::ordered_button_ids(&self.elements.button_widgets)
            .into_iter()
            .rev()
            .find(|id| {
                let widget = &self.elements.button_widgets[id];
                !screen_model::widget_name_is_hidden(&widget.name, &self.elements.widgets_to_hide)
                    && screen_model::resolved_widget_command(
                        widget,
                        world.assets,
                        screen_model::resolve_party_entity(map, widget.party.as_deref()),
                        &self.elements.ui_state,
                    )
                    .as_deref()
                    .and_then(parse_client_command)
                    .as_ref()
                        == Some(&binding)
            });
        if let Some(id) = matched {
            return self.activate_button(world, id);
        }
        let actor = screen_model::resolve_party_entity(map, None);
        if !rules_ui::command_state(world.assets, actor, &binding.command_string()).enabled {
            return Ok(ScreenActivation::default());
        }
        let mut effects = Vec::new();
        if let Some(intent) = binding.intent_payload() {
            self.intent = intent;
            for active in [
                &mut self.elements.activated_widgets,
                &mut self.elements.permanently_activated_widgets,
            ] {
                active.retain(|id| {
                    self.elements
                        .button_widgets
                        .get(id)
                        .is_none_or(|w| !screen_model::is_targeting_button(w))
                });
            }
            if !self.intent.trim().is_empty() && self.immediate_intents(world)? {
                effects.push(ScreenEffect::Action(EntityAction::Intent(
                    self.intent.clone(),
                )));
            }
        } else if let ClientCommandBinding::Control(action) = binding {
            effects.push(ScreenEffect::Action(action));
        } else {
            effects.push(ScreenEffect::Command(binding));
        }
        self.geometry = None;
        Ok(ScreenActivation {
            accepted: true,
            effects,
        })
    }
    /// Activate a committed click/shortcut. A drag controller invokes this only
    /// after deciding it was a click, not while beginning an item drag.
    /// Command placeholders resolve before the authored binding transaction.
    pub fn activate_button(
        &mut self,
        world: &RuntimeView<'_>,
        button: u32,
    ) -> Result<ScreenActivation, String> {
        let Some(widget) = self.elements.button_widgets.get(&button) else {
            return Ok(ScreenActivation::default());
        };
        if screen_model::widget_name_is_hidden(&widget.name, &self.elements.widgets_to_hide) {
            return Ok(ScreenActivation::default());
        }
        let map = world.map.unwrap_or(&self.empty_map);
        let actor = screen_model::resolve_party_entity(map, widget.party.as_deref());
        let command = screen_model::contextual_open_close_command(
            screen_model::resolved_widget_command(
                widget,
                world.assets,
                actor,
                &self.elements.ui_state,
            ),
            self.hovered_item,
            map,
        );
        if command
            .as_deref()
            .is_some_and(|c| !rules_ui::command_state(world.assets, actor, c).enabled)
        {
            return Ok(ScreenActivation::default());
        }
        let binding = command
            .as_deref()
            .map(|c| {
                parse_client_command(c)
                    .ok_or_else(|| format!("Unknown command '{c}' on button '{}'", widget.name))
            })
            .transpose()?;
        let camera = widget
            .camera
            .clone()
            .map(|camera| (widget.camera_target.clone(), camera));
        let player_camera = widget.player_camera.clone();
        let previous_intent = (!self.intent.trim().is_empty()).then(|| self.intent.clone());
        let item = actor.and_then(|actor| {
            let item = if let Some(index) = widget.inventory_index {
                actor.inventory.get(index).and_then(Option::as_ref)
            } else {
                widget
                    .equipped_slot
                    .as_deref()
                    .and_then(|slot| actor.get_equipped_item(slot))
            }?;
            Some((
                actor.id,
                item.id,
                item.is_container()
                    || item.attributes.get_bool_default("container", false)
                    || item.attributes.get_int_default("container_slots", 0) > 0,
            ))
        });
        let anchor = rect(widget.rect);
        let immediate = self.immediate_intents(world)?;
        let mut effects = Vec::new();
        let mut action = None;
        match binding {
            Some(ClientCommandBinding::Control(control)) => action = Some(control),
            Some(ClientCommandBinding::Intent(intent)) => {
                self.intent = intent.clone();
                if !intent.trim().is_empty() && immediate {
                    action = Some(EntityAction::Intent(intent));
                }
            }
            Some(ClientCommandBinding::RulesAction(action_id)) => {
                self.intent = format!("action:{action_id}");
                if immediate {
                    action = Some(EntityAction::Intent(self.intent.clone()));
                }
            }
            Some(command) => effects.push(ScreenEffect::Command(command)),
            None => {}
        }
        self.elements.press_button_state(button);
        self.geometry = None;
        if let Some((owner, item, container)) = item {
            if container && previous_intent.is_none() {
                effects.push(ScreenEffect::ToggleContainer {
                    region: map.id,
                    item,
                    owner,
                    anchor,
                });
                action = None;
            } else {
                action = Some(EntityAction::ItemClicked(
                    item,
                    0.,
                    previous_intent,
                    Some(owner),
                ));
            }
        }
        if let Some((target, camera)) = camera {
            for widget in self
                .elements
                .game_widgets
                .values_mut()
                .filter(|w| target.as_ref().is_none_or(|name| &w.name == name))
            {
                widget.set_camera_mode(camera.clone());
            }
        }
        if let Some(camera) = player_camera {
            action = Some(EntityAction::SetPlayerCamera(camera));
        }
        if let Some(action) = action {
            effects.push(ScreenEffect::Action(action));
        }
        Ok(ScreenActivation {
            accepted: true,
            effects,
        })
    }
}
