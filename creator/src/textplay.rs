use crate::editor::RUSTERIX;
use crate::prelude::*;
use rusterix::server::message::EntityAction;
use shared::rusterix_utils::warmup_runtime;
use shared::text_game as sg;
use std::collections::BTreeSet;
use toml::Table;

type GameMessage = (Option<u32>, Option<u32>, u32, String, String);
type GameSay = (Option<u32>, Option<u32>, String, String);

#[derive(Clone, Copy, PartialEq, Eq)]
enum AutoAttackMode {
    Never,
    OnAttack,
}

#[derive(Clone)]
struct TextGameColors {
    title: Option<TheColor>,
    message_categories: FxHashMap<String, TheColor>,
}

pub struct TextGameState {
    blocks: Vec<TheTextViewBlock>,
    initialized: bool,
    last_sector_id: Option<u32>,
    last_nearby_attackers: BTreeSet<String>,
    suppress_next_sector_render: Option<u32>,
    suppress_next_description_for_sector: Option<u32>,
    last_announced_hour: Option<u8>,
    auto_attack_target: Option<u32>,
    dirty: bool,
    appended_since_sync: bool,
    force_scroll_to_bottom: bool,
}

impl Default for TextGameState {
    fn default() -> Self {
        Self::new()
    }
}

impl TextGameState {
    pub const INPUT_ID: &'static str = "Text Game Input";
    pub const OUTPUT_ID: &'static str = "Text Game Output";

    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            initialized: false,
            last_sector_id: None,
            last_nearby_attackers: BTreeSet::new(),
            suppress_next_sector_render: None,
            suppress_next_description_for_sector: None,
            last_announced_hour: None,
            auto_attack_target: None,
            dirty: false,
            appended_since_sync: false,
            force_scroll_to_bottom: false,
        }
    }

    pub fn setup_canvas() -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut output = TheTextView::new(TheId::named(Self::OUTPUT_ID));
        output.set_font_size(13.0);
        TheTextViewTrait::set_embedded(&mut output, true);
        output.set_selectable(true);
        output.set_word_wrap(true);
        output.draw_border(false);
        output.draw_background(false);
        output.set_background_override(Some(TheColor::from_hex("#000000")));
        output.set_padding((10, 10, 10, 10));
        canvas.set_widget(output);

        let mut input_canvas = TheCanvas::default();
        let mut input = TheTextLineEdit::new(TheId::named(Self::INPUT_ID));
        input.set_status_text("Enter a text gameplay command and press Return.");
        input.set_font_size(12.5);
        input.limiter_mut().set_max_height(24);
        input_canvas.set_widget(input);
        canvas.set_bottom(input_canvas);

        canvas
    }

    pub fn reset(&mut self) {
        self.blocks.clear();
        self.initialized = false;
        self.last_sector_id = None;
        self.last_nearby_attackers.clear();
        self.suppress_next_sector_render = None;
        self.suppress_next_description_for_sector = None;
        self.last_announced_hour = None;
        self.auto_attack_target = None;
        self.dirty = true;
        self.appended_since_sync = false;
        self.force_scroll_to_bottom = true;
    }

    pub fn activate(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.sync_output(ui, ctx);
        ui.set_widget_value(Self::INPUT_ID, ctx, TheValue::Text(String::new()));
        if let Some(widget) = ui.get_widget(Self::INPUT_ID) {
            let id = widget.id().clone();
            ctx.ui.set_focus(&id);
        }
    }

    pub fn sync_output(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        if !self.dirty {
            return;
        }
        if let Some(view) = ui.get_text_view(Self::OUTPUT_ID) {
            view.set_blocks(self.blocks.clone());
            let output_focused = ctx
                .ui
                .focus
                .as_ref()
                .map(|id| id.name == Self::OUTPUT_ID)
                .unwrap_or(false);
            if (self.appended_since_sync || self.force_scroll_to_bottom) && !output_focused {
                view.scroll_to_bottom();
            }
            view.set_needs_redraw(true);
            ctx.ui.redraw_all = true;
        }
    }

    pub fn handle_input(
        &mut self,
        command: &str,
        project: &mut Project,
        server_ctx: &ServerContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> bool {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            ui.set_widget_value(Self::INPUT_ID, ctx, TheValue::Text(String::new()));
            return false;
        }

        refresh_text_runtime_view(project);

        let colors = authoring_colors(&project.authoring);
        self.push_line(
            &format!("> {}", trimmed),
            colors.message_categories.get("system"),
        );

        let keep_running = self.handle_command(trimmed, project, server_ctx);
        self.sync_output(ui, ctx);
        self.dirty = false;
        self.appended_since_sync = false;
        self.force_scroll_to_bottom = false;
        ui.set_widget_value(Self::INPUT_ID, ctx, TheValue::Text(String::new()));
        if let Some(widget) = ui.get_widget(Self::INPUT_ID) {
            let id = widget.id().clone();
            ctx.ui.set_focus(&id);
        }
        keep_running
    }

    pub fn update(
        &mut self,
        project: &Project,
        server_ctx: &ServerContext,
        messages: &mut Vec<GameMessage>,
        says: &mut Vec<GameSay>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        if !server_ctx.text_game_mode || !server_ctx.game_mode {
            return;
        }

        let colors = authoring_colors(&project.authoring);
        let mut rendered_room_this_update = false;
        let current_sector_id = current_region(project, server_ctx).and_then(|region| {
            sg::current_player_and_sector(&region.map).map(|(_, sector)| sector.id)
        });

        if !self.initialized {
            self.initialized = true;
            self.last_sector_id = current_sector_id;
            self.last_announced_hour = current_time_hour(project, server_ctx);

            match sg::authoring_startup_display(&project.authoring) {
                sg::StartupDisplay::Description => {
                    if let Some(text) = render_current_sector_description(project, server_ctx) {
                        self.push_plain_block(&text);
                    }
                }
                sg::StartupDisplay::Room => {
                    self.push_room_text(project, server_ctx);
                    rendered_room_this_update = true;
                }
                sg::StartupDisplay::None => {}
            }
            self.last_nearby_attackers = current_nearby_attackers(project, server_ctx);
        } else if let Some(current_sector_id) = current_sector_id
            && Some(current_sector_id) != self.last_sector_id
        {
            self.last_sector_id = Some(current_sector_id);
            if self.suppress_next_sector_render == Some(current_sector_id) {
                self.suppress_next_sector_render = None;
            } else {
                self.push_room_text(project, server_ctx);
                rendered_room_this_update = true;
            }
            self.last_nearby_attackers = current_nearby_attackers(project, server_ctx);
        }

        let player_id = current_region(project, server_ctx).and_then(|region| {
            sg::current_player_and_sector(&region.map).map(|(player, _)| player.id)
        });
        let current_description = render_current_sector_description(project, server_ctx);

        for (sender_entity, _sender_item, receiver_id, message, category) in messages.drain(..) {
            if let Some(player_id) = player_id
                && receiver_id != player_id
            {
                continue;
            }
            if authoring_auto_attack_mode(&project.authoring) == AutoAttackMode::OnAttack
                && is_under_attack_message(&message)
                && let (Some(sender_id), Some(player_id)) = (sender_entity, player_id)
                && sender_id != player_id
            {
                self.auto_attack_target = Some(sender_id);
            }
            if !should_print_text_message(&message, &category) {
                continue;
            }
            if current_sector_id.is_some()
                && self.suppress_next_description_for_sector == current_sector_id
                && current_description
                    .as_deref()
                    .map(|text| text.trim() == message.trim())
                    .unwrap_or(false)
            {
                self.suppress_next_description_for_sector = None;
                continue;
            }
            if rendered_room_this_update
                && current_description
                    .as_deref()
                    .map(|text| text.trim() == message.trim())
                    .unwrap_or(false)
            {
                continue;
            }
            self.push_line(
                &message,
                colors
                    .message_categories
                    .get(&category.to_ascii_lowercase()),
            );
        }

        for (_sender_entity, _sender_item, message, _category) in says.drain(..) {
            self.push_plain_line(&message);
        }

        if let Some(label) =
            take_hour_announcement(project, server_ctx, &mut self.last_announced_hour)
        {
            self.push_plain_line(&format!("It is {}.", label));
        }

        let current_nearby_attackers = current_nearby_attackers(project, server_ctx);
        if !rendered_room_this_update {
            let new_attackers: Vec<String> = current_nearby_attackers
                .difference(&self.last_nearby_attackers)
                .cloned()
                .collect();
            if !new_attackers.is_empty() {
                self.push_plain_line(&sg::render_nearby_attacker_appearance_sentence(
                    &new_attackers,
                ));
            }
        }
        self.last_nearby_attackers = current_nearby_attackers;

        self.process_auto_attack(project, server_ctx);
        self.sync_output(ui, ctx);
        self.dirty = false;
        self.appended_since_sync = false;
        self.force_scroll_to_bottom = false;
    }

    fn handle_command(
        &mut self,
        input: &str,
        project: &mut Project,
        server_ctx: &ServerContext,
    ) -> bool {
        let lower = input.to_ascii_lowercase();
        let direction = match lower.as_str() {
            "n" => "north",
            "e" => "east",
            "s" => "south",
            "w" => "west",
            _ => lower.as_str(),
        };

        match input {
            "" | "look" | "l" => self.push_room_text(project, server_ctx),
            "inventory" | "inv" => {
                if let Some(region) = current_region(project, server_ctx) {
                    if let Some(text) = sg::render_player_inventory(&region.map) {
                        self.push_plain_block(&text);
                    } else {
                        self.push_plain_line("No local player found.");
                    }
                } else {
                    self.push_plain_line("Current region not found.");
                }
            }
            _ if matches!(direction, "north" | "south" | "east" | "west") => {
                if move_by_direction(direction, project, server_ctx) {
                    sync_text_runtime(project);
                    self.last_sector_id = current_region(project, server_ctx).and_then(|region| {
                        sg::current_player_and_sector(&region.map).map(|(_, sector)| sector.id)
                    });
                    self.suppress_next_sector_render = self.last_sector_id;
                    self.suppress_next_description_for_sector = self.last_sector_id;
                    self.push_room_text(project, server_ctx);
                } else {
                    self.push_plain_line("You cannot go that way.");
                }
            }
            "help" => {
                self.push_plain_block(
                    [
                        "Commands:",
                        "  look | l           Show the current room",
                        "  inventory | inv    Show your inventory",
                        "  north | east | south | west",
                        "  n | e | s | w",
                        "                     Move through a text exit",
                        "  go <name>          Move by exit direction or exit title",
                        "  <intent> <target>  Trigger a configured player intent on a visible target",
                        "  intent <name>      Set the current player intent",
                    ]
                    .join("\n")
                    .as_str(),
                );
            }
            _ if lower.starts_with("go ") => {
                let target = input["go ".len()..].trim().to_ascii_lowercase();
                if target.is_empty() {
                    self.push_plain_line("Usage: go <direction or exit name>");
                } else if move_by_exit_name(&target, project, server_ctx) {
                    sync_text_runtime(project);
                    self.last_sector_id = current_region(project, server_ctx).and_then(|region| {
                        sg::current_player_and_sector(&region.map).map(|(_, sector)| sector.id)
                    });
                    self.suppress_next_sector_render = self.last_sector_id;
                    self.suppress_next_description_for_sector = self.last_sector_id;
                    self.push_room_text(project, server_ctx);
                } else {
                    self.push_plain_line("No matching exit.");
                }
            }
            _ if input.starts_with("intent ") => {
                let payload = input["intent ".len()..].trim();
                if payload.is_empty() {
                    self.push_plain_line("Usage: intent <name> [target]");
                } else {
                    let mut parts = payload.splitn(2, char::is_whitespace);
                    let intent = parts.next().unwrap_or("").trim();
                    let target = parts.next().map(str::trim).unwrap_or("");
                    if !intent.is_empty() && !target.is_empty() {
                        if let Some(error) =
                            trigger_text_intent(project, server_ctx, intent, target)
                        {
                            self.push_plain_line(&error);
                        } else {
                            sync_text_runtime(project);
                        }
                    } else if !intent.is_empty() {
                        RUSTERIX
                            .write()
                            .unwrap()
                            .server
                            .local_player_action(EntityAction::Intent(intent.to_string()));
                        sync_text_runtime(project);
                    } else {
                        self.push_plain_line("Usage: intent <name> [target]");
                    }
                }
            }
            _ => {
                let mut parts = input.splitn(2, char::is_whitespace);
                let verb =
                    resolve_intent_alias(&project.authoring, parts.next().unwrap_or("").trim());
                let target = parts.next().map(str::trim).unwrap_or("");
                let supported_intents = current_player_supported_intents(project, server_ctx);
                if !verb.is_empty() && !target.is_empty() && supported_intents.contains(&verb) {
                    if let Some(error) = trigger_text_intent(project, server_ctx, &verb, target) {
                        self.push_plain_line(&error);
                    } else {
                        sync_text_runtime(project);
                    }
                } else {
                    self.push_plain_line("Unknown command. Type 'help' for available commands.");
                }
            }
        }

        true
    }

    fn push_room_text(&mut self, project: &Project, server_ctx: &ServerContext) {
        let colors = authoring_colors(&project.authoring);
        let room = render_room_blocks(project, server_ctx, &colors);
        if !room.is_empty() {
            self.blocks.extend(room);
        }
        self.last_nearby_attackers = current_nearby_attackers(project, server_ctx);
    }

    fn push_plain_block(&mut self, text: &str) {
        if text.trim().is_empty() {
            return;
        }
        self.blocks.push(TheTextViewBlock {
            text: format!("{}\n\n", text.trim_end()),
            style: TheTextStyle {
                foreground: None,
                background: None,
                underline: None,
            },
        });
        self.dirty = true;
        self.appended_since_sync = true;
    }

    fn push_plain_line(&mut self, text: &str) {
        if text.trim().is_empty() {
            return;
        }
        self.blocks.push(TheTextViewBlock {
            text: format!("{}\n", text.trim_end()),
            style: TheTextStyle {
                foreground: None,
                background: None,
                underline: None,
            },
        });
        self.dirty = true;
        self.appended_since_sync = true;
    }

    fn push_line(&mut self, text: &str, color: Option<&TheColor>) {
        if text.trim().is_empty() {
            return;
        }
        self.blocks.push(TheTextViewBlock {
            text: format!("{}\n", text.trim_end()),
            style: TheTextStyle {
                foreground: color.cloned(),
                background: None,
                underline: None,
            },
        });
        self.dirty = true;
        self.appended_since_sync = true;
    }

    fn process_auto_attack(&mut self, project: &Project, server_ctx: &ServerContext) {
        if authoring_auto_attack_mode(&project.authoring) != AutoAttackMode::OnAttack {
            self.auto_attack_target = None;
            return;
        }

        let Some(target_id) = self.auto_attack_target else {
            return;
        };
        let Some(region) = current_region(project, server_ctx) else {
            self.auto_attack_target = None;
            return;
        };
        let Some((player, sector)) = sg::current_player_and_sector(&region.map) else {
            self.auto_attack_target = None;
            return;
        };
        let Some(target) = region
            .map
            .entities
            .iter()
            .find(|entity| entity.id == target_id)
        else {
            self.auto_attack_target = None;
            return;
        };
        if sg::entity_is_dead(target) {
            self.auto_attack_target = None;
            return;
        }

        let same_sector = sg::entity_sector_matches(&region.map, target, sector);
        if !same_sector {
            self.auto_attack_target = None;
            return;
        }

        let distance = player.get_pos_xz().distance(target.get_pos_xz());
        RUSTERIX
            .write()
            .unwrap()
            .server
            .local_player_action(EntityAction::EntityClicked(
                target_id,
                distance,
                Some("attack".into()),
            ));
    }
}

fn current_region<'a>(
    project: &'a Project,
    server_ctx: &ServerContext,
) -> Option<&'a shared::region::Region> {
    project.get_region_ctx(server_ctx)
}

fn sync_text_runtime(project: &mut Project) {
    let mut rusterix = RUSTERIX.write().unwrap();
    warmup_runtime(&mut rusterix, project, 1);
}

fn refresh_text_runtime_view(project: &mut Project) {
    let mut rusterix = RUSTERIX.write().unwrap();
    for region in &mut project.regions {
        rusterix.server.apply_entities_items(&mut region.map);
        if let Some(time) = rusterix.server.get_time(&region.map.id) {
            rusterix.client.set_server_time(time);
            project.time = time;
        }
    }
}

fn render_room_blocks(
    project: &Project,
    server_ctx: &ServerContext,
    colors: &TextGameColors,
) -> Vec<TheTextViewBlock> {
    let mut blocks = Vec::new();
    let Some(region) = current_region(project, server_ctx) else {
        return blocks;
    };
    let Some(room) = sg::build_text_room(&region.map, &project.authoring) else {
        return blocks;
    };

    if !room.title.is_empty() {
        blocks.push(text_block(
            &format!("{}\n\n", room.title),
            colors.title.as_ref(),
        ));
    }
    if !room.description.is_empty() {
        blocks.push(text_block(&format!("{}\n\n", room.description), None));
    }

    if !room.exits.is_empty() {
        let exits_text = match sg::authoring_exit_presentation(&project.authoring) {
            sg::ExitPresentation::List => {
                let mut lines = vec!["Exits:".to_string()];
                for exit in &room.exits {
                    lines.push(format!("  {} - {}", exit.direction, exit.title));
                }
                lines.join("\n")
            }
            sg::ExitPresentation::Sentence => sg::render_exit_sentence(&room.exits),
        };
        blocks.push(text_block(&format!("{}\n", exits_text), None));
    }

    if !room.live_entities.is_empty() {
        blocks.push(text_block(
            &format!(
                "{}\n",
                sg::render_presence_sentence("You see", &room.live_entities)
            ),
            None,
        ));
    }

    if !room.nearby_attackers.is_empty() {
        blocks.push(text_block(
            &format!(
                "{}\n",
                sg::render_nearby_attackers_sentence(&room.nearby_attackers)
            ),
            None,
        ));
    }

    if !room.dead_entities.is_empty() {
        blocks.push(text_block(
            &format!(
                "{}\n",
                sg::render_presence_sentence("You see", &room.dead_entities)
            ),
            None,
        ));
    }

    if !room.items.is_empty() {
        blocks.push(text_block(
            &format!(
                "{}\n",
                sg::render_presence_sentence("You notice", &room.items)
            ),
            None,
        ));
    }

    if let Some(last) = blocks.last_mut()
        && !last.text.ends_with("\n\n")
    {
        last.text.push('\n');
    }

    blocks
}

fn current_nearby_attackers(project: &Project, server_ctx: &ServerContext) -> BTreeSet<String> {
    let Some(region) = current_region(project, server_ctx) else {
        return BTreeSet::new();
    };
    sg::build_text_room(&region.map, &project.authoring)
        .map(|room| room.nearby_attackers.into_iter().collect())
        .unwrap_or_default()
}

fn render_current_sector_description(
    project: &Project,
    server_ctx: &ServerContext,
) -> Option<String> {
    let region = current_region(project, server_ctx)?;
    sg::render_current_sector_description(&region.map)
}

fn text_block(text: &str, color: Option<&TheColor>) -> TheTextViewBlock {
    TheTextViewBlock {
        text: text.to_string(),
        style: TheTextStyle {
            foreground: color.cloned(),
            background: None,
            underline: None,
        },
    }
}

fn move_by_direction(direction: &str, project: &Project, server_ctx: &ServerContext) -> bool {
    let Some(region) = current_region(project, server_ctx) else {
        return false;
    };
    let Some((_, sector)) = sg::current_player_and_sector(&region.map) else {
        return false;
    };
    let exits = sg::resolve_text_exits(
        &region.map,
        sector,
        sg::authoring_connection_probe_distance(&project.authoring),
    );
    if let Some(exit) = exits.into_iter().find(|exit| exit.direction == direction) {
        RUSTERIX
            .write()
            .unwrap()
            .server
            .local_player_teleport_pos(exit.target_center);
        return true;
    }
    false
}

fn move_by_exit_name(target: &str, project: &Project, server_ctx: &ServerContext) -> bool {
    let Some(region) = current_region(project, server_ctx) else {
        return false;
    };
    let Some((_, sector)) = sg::current_player_and_sector(&region.map) else {
        return false;
    };
    let exits = sg::resolve_text_exits(
        &region.map,
        sector,
        sg::authoring_connection_probe_distance(&project.authoring),
    );
    if let Some(exit) = exits.into_iter().find(|exit| {
        exit.direction == target
            || exit.title.to_ascii_lowercase() == target
            || exit.target_title.to_ascii_lowercase() == target
    }) {
        RUSTERIX
            .write()
            .unwrap()
            .server
            .local_player_teleport_pos(exit.target_center);
        return true;
    }
    false
}

fn trigger_text_intent(
    project: &Project,
    server_ctx: &ServerContext,
    intent: &str,
    query: &str,
) -> Option<String> {
    let Some(region) = current_region(project, server_ctx) else {
        return Some("Current region not found.".into());
    };
    let Some((_player, sector)) = sg::current_player_and_sector(&region.map) else {
        return Some("No local player found.".into());
    };
    let target = match sg::resolve_text_target(&region.map, sector, query) {
        Ok(target) => target,
        Err(err) => return Some(err),
    };

    match target {
        sg::TextTarget::Entity { id, distance } => {
            RUSTERIX
                .write()
                .unwrap()
                .server
                .local_player_action(EntityAction::EntityClicked(
                    id,
                    distance,
                    Some(intent.trim().to_string()),
                ));
        }
        sg::TextTarget::Item { id, distance } => {
            RUSTERIX
                .write()
                .unwrap()
                .server
                .local_player_action(EntityAction::ItemClicked(
                    id,
                    distance,
                    Some(intent.trim().to_string()),
                ));
        }
    }

    None
}

fn current_player_supported_intents(
    project: &Project,
    server_ctx: &ServerContext,
) -> BTreeSet<String> {
    current_region(project, server_ctx)
        .map(|region| sg::current_player_supported_intents(project, &region.map))
        .unwrap_or_default()
}

fn current_time_hour(project: &Project, server_ctx: &ServerContext) -> Option<u8> {
    let region = current_region(project, server_ctx)?;
    RUSTERIX
        .read()
        .unwrap()
        .server
        .get_time(&region.map.id)
        .map(|time| time.hours)
}

fn take_hour_announcement(
    project: &Project,
    server_ctx: &ServerContext,
    last_announced_hour: &mut Option<u8>,
) -> Option<String> {
    let region = current_region(project, server_ctx)?;
    let time = RUSTERIX.read().unwrap().server.get_time(&region.map.id)?;
    let label = if time.minutes == 0 {
        let period = if time.hours >= 12 { "PM" } else { "AM" };
        let hour = if time.hours % 12 == 0 {
            12
        } else {
            time.hours % 12
        };
        format!("{} {}", hour, period)
    } else {
        time.to_time12()
    };
    let previous_hour = last_announced_hour.replace(time.hours);
    if previous_hour == Some(time.hours) {
        None
    } else {
        Some(label)
    }
}
fn config_table(src: &str) -> Option<Table> {
    src.parse::<Table>().ok()
}

fn authoring_auto_attack_mode(src: &str) -> AutoAttackMode {
    config_table(src)
        .and_then(|table| {
            table
                .get("combat")
                .and_then(toml::Value::as_table)
                .and_then(|table| table.get("auto_attack"))
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        })
        .map(|value| match value.trim().to_ascii_lowercase().as_str() {
            "on_attack" => AutoAttackMode::OnAttack,
            _ => AutoAttackMode::Never,
        })
        .unwrap_or(AutoAttackMode::Never)
}

fn resolve_intent_alias(src: &str, verb: &str) -> String {
    let normalized = verb.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return normalized;
    }

    let Some(table) = config_table(src) else {
        return normalized;
    };
    let Some(alias_table) = table.get("alias").and_then(toml::Value::as_table) else {
        return normalized;
    };

    for (canonical, aliases) in alias_table {
        if canonical.trim().eq_ignore_ascii_case(&normalized) {
            return canonical.trim().to_ascii_lowercase();
        }
        if let Some(values) = aliases.as_array() {
            for alias in values.iter().filter_map(toml::Value::as_str) {
                if alias.trim().eq_ignore_ascii_case(&normalized) {
                    return canonical.trim().to_ascii_lowercase();
                }
            }
        }
    }

    normalized
}

fn should_print_text_message(message: &str, category: &str) -> bool {
    !(category == "warning" && message.trim() == "{system.cant_do_that_yet}")
}

fn is_under_attack_message(message: &str) -> bool {
    message.trim_start().starts_with("You are under attack by ")
}

fn authoring_colors(src: &str) -> TextGameColors {
    let mut message_categories = FxHashMap::default();
    let Some(table) = config_table(src) else {
        return TextGameColors {
            title: None,
            message_categories,
        };
    };
    let Some(colors) = table.get("colors").and_then(toml::Value::as_table) else {
        return TextGameColors {
            title: None,
            message_categories,
        };
    };

    if let Some(categories) = colors
        .get("message_categories")
        .and_then(toml::Value::as_table)
    {
        for (key, value) in categories {
            if let Some(color) = value.as_str().and_then(parse_named_text_color) {
                message_categories.insert(key.to_ascii_lowercase(), color);
            }
        }
    }

    TextGameColors {
        title: colors
            .get("title")
            .and_then(toml::Value::as_str)
            .and_then(parse_named_text_color),
        message_categories,
    }
}

fn parse_named_text_color(name: &str) -> Option<TheColor> {
    let normalized = name.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }

    let hex = match normalized.as_str() {
        "black" => "#000000",
        "red" => "#cd3131",
        "green" => "#0dbc79",
        "yellow" => "#949800",
        "blue" => "#2472c8",
        "magenta" => "#bc3fbc",
        "cyan" => "#11a8cd",
        "white" => "#e5e5e5",
        "bright_black" => "#666666",
        "bright_red" => "#f14c4c",
        "bright_green" => "#23d18b",
        "bright_yellow" => "#f5f543",
        "bright_blue" => "#3b8eea",
        "bright_magenta" => "#d670d6",
        "bright_cyan" => "#29b8db",
        "bright_white" => "#ffffff",
        value if value.starts_with('#') => value,
        _ => return None,
    };

    Some(TheColor::from_hex(hex))
}
