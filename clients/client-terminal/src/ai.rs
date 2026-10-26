//! Optional AI adapter. Every action goes through the existing text-game runtime.
use super::*;
use shared::ai::{Action, Event as AiEvent, Exit, Scene, Target};

pub(super) fn configure(app: &mut TerminalApp, provider: Option<&str>) -> Result<(), String> {
    let provider = match provider.unwrap_or("environment") {
        "disabled" => None,
        "chatgpt" => Some(shared::ai::chatgpt::Manager::local()?.provider()?),
        _ => shared::ai::provider::CompatibleProvider::from_env()?,
    };
    app.ai.set_provider(provider);
    Ok(())
}
fn exit_id(exit: &sg::TextExit) -> String {
    format!("exit:{}:{}", exit.target_sector_id, exit.direction)
}
fn exits(app: &TerminalApp) -> Vec<sg::TextExit> {
    app.current_region()
        .and_then(|r| {
            sg::current_player_and_sector(&r.map).map(|(_, s)| {
                sg::resolve_text_exits(
                    &r.map,
                    s,
                    sg::authoring_connection_probe_distance(&app.project.authoring),
                )
            })
        })
        .unwrap_or_default()
}
pub(super) fn scene(app: &TerminalApp) -> Result<Scene, String> {
    let region = app
        .current_region()
        .ok_or("Current region is unavailable.")?;
    let room =
        sg::build_text_room(&region.map, &app.project.authoring).ok_or("No local player found.")?;
    let player = region
        .map
        .entities
        .iter()
        .find(|e| e.is_player())
        .ok_or("No local player found.")?;
    let sector = sg::current_player_and_sector(&region.map).map(|(_, s)| s);
    let mut intents: Vec<_> = current_player_supported_intents(app).into_iter().collect();
    if !intents.iter().any(|i| i == "look") {
        intents.push("look".into());
    }
    let mut targets = vec![];
    for entity in &region.map.entities {
        let in_room = sector
            .map(|s| sg::entity_sector_matches(&region.map, entity, s))
            .unwrap_or_else(|| player.get_pos_xz().distance(entity.get_pos_xz()) <= 6.);
        if !entity.is_player() && !sg::entity_is_dead(entity) && in_room {
            targets.push(Target {
                id: format!("entity:{}", entity.id),
                title: sg::display_name_for_entity(entity),
                kind: "character".into(),
                intents: intents.clone(),
            });
        }
    }
    for item in &region.map.items {
        let in_room = sector
            .map(|s| sg::item_sector_matches(&region.map, item, s))
            .unwrap_or_else(|| player.get_pos_xz().distance(item.get_pos_xz()) <= 6.);
        if in_room {
            targets.push(Target {
                id: format!("item:{}", item.id),
                title: sg::display_name_for_item(item),
                kind: "item".into(),
                intents: intents.clone(),
            });
        }
    }
    targets.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(Scene {
        location: format!(
            "{}:{}:{}:{}",
            app.ai_session,
            region.map.id,
            sector.map(|s| s.id.to_string()).unwrap_or_default(),
            player.id
        ),
        title: room.title,
        description: room.description,
        inventory: sg::render_player_inventory(&region.map).unwrap_or_default(),
        stats: sg::render_player_stats(
            &region.map,
            &app.project.authoring,
            &app.project.config,
            &app.assets.rules,
        )
        .unwrap_or_default(),
        people: room.live_entities,
        remains: room.dead_entities,
        targets,
        exits: room
            .exits
            .iter()
            .map(|e| Exit {
                id: exit_id(e),
                title: e.title.clone(),
                direction: e.direction.clone(),
            })
            .collect(),
    })
}
fn explicit(app: &TerminalApp, input: &str) -> bool {
    let verb = input
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        verb.as_str(),
        "" | "look"
            | "l"
            | "inventory"
            | "inv"
            | "help"
            | "quit"
            | "exit"
            | "go"
            | "intent"
            | "wait"
            | "."
            | "north"
            | "south"
            | "east"
            | "west"
            | "n"
            | "s"
            | "e"
            | "w"
            | "forward"
            | "f"
            | "backward"
            | "b"
            | "left"
            | "right"
            | "save"
            | "load"
    ) || current_player_supported_intents(app)
        .contains(&resolve_intent_alias(&app.project.authoring, &verb))
}
pub(super) fn input(app: &mut TerminalApp, input: &str) -> (bool, Vec<String>) {
    let input = input.trim();
    if matches!(input, "cancel" | "/cancel") {
        let cancelled = app.ai.cancel();
        return (
            true,
            vec![
                if cancelled {
                    "AI request cancelled. Completed game actions are kept."
                } else {
                    "No AI request is pending."
                }
                .into(),
            ],
        );
    }
    if !app.ai.enabled() || explicit(app, input) {
        app.ai.cancel();
        let (result, mut text) = handle_command(app, input);
        if input == "help" && app.ai.enabled() {
            text.push("You can also describe an action naturally. Type cancel to discard a pending AI reply.".into());
        }
        app.ai.remember(input, &text.join("\n"));
        return (result, text);
    }
    match scene(app).and_then(|scene| app.ai.begin(input, scene)) {
        Ok(()) => (true, vec![]),
        Err(e) => (true, vec![e]),
    }
}
fn execute(app: &mut TerminalApp, action: Action) -> Vec<String> {
    let current = match scene(app) {
        Ok(s) => s,
        Err(e) => return vec![e],
    };
    if !current.allows(&action) {
        return vec!["That action is no longer available.".into()];
    }
    match action {
        Action::Look {} => handle_command(app, "look").1,
        Action::Inventory {} => handle_command(app, "inventory").1,
        Action::Wait {} => handle_command(app, "wait").1,
        Action::Move { exit } => {
            let exit = exits(app).into_iter().find(|e| exit_id(e) == exit);
            if exit.as_ref().is_some_and(|e| app.move_to_exit(e)) {
                after_text_movement(app)
            } else {
                vec!["You cannot go that way.".into()]
            }
        }
        Action::Interact { target, intent } => {
            let Some(region) = app.current_region() else {
                return vec!["Current region is unavailable.".into()];
            };
            let Some(player) = region.map.entities.iter().find(|e| e.is_player()) else {
                return vec!["No local player found.".into()];
            };
            let target =
                if let Some(id) = target
                    .strip_prefix("entity:")
                    .and_then(|v| v.parse::<u32>().ok())
                {
                    region.map.entities.iter().find(|e| e.id == id).map(|e| {
                        sg::TextTarget::Entity {
                            id,
                            distance: player.get_pos_xz().distance(e.get_pos_xz()),
                        }
                    })
                } else if let Some(id) = target
                    .strip_prefix("item:")
                    .and_then(|v| v.parse::<u32>().ok())
                {
                    region
                        .map
                        .items
                        .iter()
                        .find(|i| i.id == id)
                        .map(|i| sg::TextTarget::Item {
                            id,
                            distance: player.get_pos_xz().distance(i.get_pos_xz()),
                        })
                } else {
                    None
                };
            match target {
                Some(target) => dispatch_text_intent(app, &intent, target),
                None => vec!["That target is no longer here.".into()],
            }
        }
    }
}
pub(super) fn poll(app: &mut TerminalApp) -> Vec<String> {
    if !app.ai.is_busy() {
        return vec![];
    }
    let mut assistant = std::mem::take(&mut app.ai);
    let event = assistant.poll_with(|| scene(app));
    app.ai = assistant;
    match event {
        None => vec![],
        Some(AiEvent::Reply(text) | AiEvent::Error(text)) => vec![text],
        Some(AiEvent::Action(action)) => {
            let mut result = execute(app, action);
            if result.iter().all(|s| s.trim().is_empty()) {
                result.push(
                    "The command was sent to the game; it produced no immediate text.".into(),
                );
            }
            let next = scene(app);
            if let Err(error) = next.and_then(|s| app.ai.narrate_result(&result.join("\n"), s)) {
                app.ai.cancel();
                result.push(format!("AI narration unavailable: {error}"));
            }
            result
        }
    }
}
#[cfg(test)]
mod tests;
