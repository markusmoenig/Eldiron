use crate::editor::RUSTERIX;
use crate::prelude::*;
use rusterix::server::message::EntityAction;
use shared::text_game as sg;
use std::collections::BTreeSet;
use toml::Table;

#[derive(Clone, Copy, PartialEq, Eq)]
enum AutoAttackMode {
    Never,
    OnAttack,
}

#[derive(Clone)]
struct TextGameColors {
    title: Option<TheColor>,
    items: Option<TheColor>,
    corpses: Option<TheColor>,
    message_categories: FxHashMap<String, TheColor>,
}

pub struct TextGameState {
    blocks: Vec<TheTextViewBlock>,
    session: TextSession,
    dirty: bool,
    appended_since_sync: bool,
    force_scroll_to_bottom: bool,
    active_input_id: &'static str,
}

impl Default for TextGameState {
    fn default() -> Self {
        Self::new()
    }
}

impl TextGameState {
    pub const GAME_INPUT_ID: &'static str = "Text Game Input";
    pub const GAME_OUTPUT_ID: &'static str = "Text Game Output";
    pub const DOCK_INPUT_ID: &'static str = "Text Game Dock Input";
    pub const DOCK_OUTPUT_ID: &'static str = "Text Game Dock Output";

    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            session: TextSession::new(),
            dirty: false,
            appended_since_sync: false,
            force_scroll_to_bottom: false,
            active_input_id: Self::GAME_INPUT_ID,
        }
    }

    fn setup_canvas_with_ids(output_id: &str, input_id: &str) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut output = TheTextView::new(TheId::named(output_id));
        output.set_font_size(13.0);
        output.set_font_preference(TheFontPreference::Code);
        TheTextViewTrait::set_embedded(&mut output, true);
        output.set_selectable(true);
        output.set_word_wrap(true);
        output.draw_border(false);
        output.draw_background(false);
        output.set_background_override(Some(TheColor::from_hex("#000000")));
        output.set_padding((10, 10, 10, 10));
        canvas.set_widget(output);

        let mut input_canvas = TheCanvas::default();
        let mut input = TheTextLineEdit::new(TheId::named(input_id));
        input.set_status_text("Enter a text gameplay command and press Return.");
        input.set_font_size(12.5);
        input.limiter_mut().set_max_height(24);
        input_canvas.set_widget(input);
        canvas.set_bottom(input_canvas);

        canvas
    }

    pub fn setup_canvas() -> TheCanvas {
        Self::setup_canvas_with_ids(Self::GAME_OUTPUT_ID, Self::GAME_INPUT_ID)
    }

    pub fn setup_dock_canvas() -> TheCanvas {
        Self::setup_canvas_with_ids(Self::DOCK_OUTPUT_ID, Self::DOCK_INPUT_ID)
    }

    pub fn reset(&mut self) {
        self.blocks.clear();
        self.session.reset();
        self.dirty = true;
        self.appended_since_sync = false;
        self.force_scroll_to_bottom = true;
    }

    fn activate_input(&mut self, input_id: &str, ui: &mut TheUI, ctx: &mut TheContext) {
        self.active_input_id = if input_id == Self::DOCK_INPUT_ID {
            Self::DOCK_INPUT_ID
        } else {
            Self::GAME_INPUT_ID
        };
        self.sync_output(ui, ctx);
        ui.set_widget_value(input_id, ctx, TheValue::Text(String::new()));
        if let Some(widget) = ui.get_widget(input_id) {
            let id = widget.id().clone();
            ctx.ui.set_focus(&id);
        }
    }

    pub fn activate(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.activate_input(Self::GAME_INPUT_ID, ui, ctx);
    }

    pub fn activate_dock(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.activate_input(Self::DOCK_INPUT_ID, ui, ctx);
    }

    pub fn sync_output(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        if !self.dirty {
            return;
        }
        for output_id in [Self::GAME_OUTPUT_ID, Self::DOCK_OUTPUT_ID] {
            if let Some(view) = ui.get_text_view(output_id) {
                view.set_blocks(self.blocks.clone());
                let output_focused = ctx
                    .ui
                    .focus
                    .as_ref()
                    .map(|id| id.name == output_id)
                    .unwrap_or(false);
                if (self.appended_since_sync || self.force_scroll_to_bottom) && !output_focused {
                    view.scroll_to_bottom();
                }
                view.set_needs_redraw(true);
                ctx.ui.redraw_all = true;
            }
        }
    }

    pub fn handle_input(
        &mut self,
        input_id: &str,
        command: &str,
        project: &mut Project,
        server_ctx: &ServerContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> bool {
        self.active_input_id = if input_id == Self::DOCK_INPUT_ID {
            Self::DOCK_INPUT_ID
        } else {
            Self::GAME_INPUT_ID
        };
        let trimmed = command.trim();
        if trimmed.is_empty() {
            ui.set_widget_value(input_id, ctx, TheValue::Text(String::new()));
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
        ui.set_widget_value(input_id, ctx, TheValue::Text(String::new()));
        if let Some(widget) = ui.get_widget(input_id) {
            let id = widget.id().clone();
            ctx.ui.set_focus(&id);
        }
        keep_running
    }

    pub fn update(
        &mut self,
        project: &Project,
        server_ctx: &ServerContext,
        messages: &mut Vec<TextGameMessage>,
        says: &mut Vec<TextGameSay>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        if !server_ctx.text_game_mode {
            return;
        }

        let outputs = if let Some(region) = current_region(project, server_ctx) {
            self.session.collect(
                &region.map,
                &project.authoring,
                std::mem::take(messages),
                std::mem::take(says),
                current_time_hour(project, server_ctx),
                current_time_label(project, server_ctx),
                authoring_auto_attack_mode(&project.authoring) == AutoAttackMode::OnAttack,
            )
        } else {
            messages.clear();
            says.clear();
            Vec::new()
        };
        self.apply_outputs(project, server_ctx, &outputs);
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
            "stats" | "stat" => {
                if let Some(region) = current_region(project, server_ctx) {
                    if let Some(text) =
                        sg::render_player_stats(&region.map, &project.authoring, &project.config)
                    {
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
                        "  stats | stat       Show your character stats",
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
                        }
                    } else if !intent.is_empty() {
                        RUSTERIX
                            .write()
                            .unwrap()
                            .server
                            .local_player_action(EntityAction::Intent(intent.to_string()));
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
            self.dirty = true;
            self.appended_since_sync = true;
        }
    }

    fn push_plain_block(&mut self, text: &str) {
        if text.trim().is_empty() {
            return;
        }
        let text = expand_tabs(text, 8);
        self.blocks.push(TheTextViewBlock {
            text: format!("{}\n\n", text.trim_end()),
            style: TheTextStyle {
                foreground: None,
                background: None,
                underline: None,
            },
            spans: Vec::new(),
        });
        self.dirty = true;
        self.appended_since_sync = true;
    }

    fn push_plain_line(&mut self, text: &str) {
        if text.trim().is_empty() {
            return;
        }
        let text = expand_tabs(text, 8);
        self.blocks.push(TheTextViewBlock {
            text: format!("{}\n", text.trim_end()),
            style: TheTextStyle {
                foreground: None,
                background: None,
                underline: None,
            },
            spans: Vec::new(),
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
            spans: Vec::new(),
        });
        self.dirty = true;
        self.appended_since_sync = true;
    }

    fn process_auto_attack(&mut self, project: &Project, server_ctx: &ServerContext) {
        if authoring_auto_attack_mode(&project.authoring) != AutoAttackMode::OnAttack {
            self.session.clear_auto_attack_target();
            return;
        }

        let Some(target_id) = self.session.auto_attack_target() else {
            return;
        };
        let Some(region) = current_region(project, server_ctx) else {
            self.session.clear_auto_attack_target();
            return;
        };
        let Some((player, sector)) = sg::current_player_and_sector(&region.map) else {
            self.session.clear_auto_attack_target();
            return;
        };
        let Some(target) = region
            .map
            .entities
            .iter()
            .find(|entity| entity.id == target_id)
        else {
            self.session.clear_auto_attack_target();
            return;
        };
        if sg::entity_is_dead(target) {
            self.session.clear_auto_attack_target();
            return;
        }

        let same_sector = sg::entity_sector_matches(&region.map, target, sector);
        if !same_sector {
            self.session.clear_auto_attack_target();
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

    fn apply_outputs(
        &mut self,
        project: &Project,
        server_ctx: &ServerContext,
        outputs: &[TextSessionOutput],
    ) {
        let colors = authoring_colors(&project.authoring);
        for output in outputs {
            match output {
                TextSessionOutput::RenderRoom => self.push_room_text(project, server_ctx),
                TextSessionOutput::Plain(text) => self.push_plain_line(text),
                TextSessionOutput::Message { text, category } => self.push_line(
                    text,
                    colors
                        .message_categories
                        .get(&category.to_ascii_lowercase()),
                ),
            }
        }
    }
}

fn expand_tabs(text: &str, tab_width: usize) -> String {
    let mut expanded = String::new();

    for line in text.lines() {
        let mut column = 0usize;
        for ch in line.chars() {
            if ch == '\t' {
                let spaces = tab_width - (column % tab_width);
                for _ in 0..spaces {
                    expanded.push(' ');
                }
                column += spaces;
            } else {
                expanded.push(ch);
                column += 1;
            }
        }
        expanded.push('\n');
    }

    if !text.ends_with('\n') && expanded.ends_with('\n') {
        expanded.pop();
    }

    expanded
}

fn current_region<'a>(
    project: &'a Project,
    server_ctx: &ServerContext,
) -> Option<&'a shared::region::Region> {
    project.get_region_ctx(server_ctx)
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
        blocks.push(presence_block(
            "You see",
            &room.dead_entities,
            colors.corpses.as_ref(),
        ));
    }

    if !room.items.is_empty() {
        blocks.push(presence_block(
            "You notice",
            &room.items,
            colors.items.as_ref(),
        ));
    }

    if let Some(last) = blocks.last_mut()
        && !block_ends_with(last, "\n\n")
    {
        append_block_plain_text(last, "\n");
    }

    blocks
}

fn text_block(text: &str, color: Option<&TheColor>) -> TheTextViewBlock {
    TheTextViewBlock {
        text: text.to_string(),
        style: TheTextStyle {
            foreground: color.cloned(),
            background: None,
            underline: None,
        },
        spans: Vec::new(),
    }
}

fn presence_block(prefix: &str, names: &[String], color: Option<&TheColor>) -> TheTextViewBlock {
    let mut block = TheTextViewBlock::default();
    let plain_style = TheTextStyle {
        foreground: None,
        background: None,
        underline: None,
    };
    let colored_style = TheTextStyle {
        foreground: color.cloned(),
        background: None,
        underline: None,
    };

    let push_plain = |block: &mut TheTextViewBlock, text: String, style: &TheTextStyle| {
        block.spans.push(TheTextViewSpan {
            text,
            style: style.clone(),
        });
    };

    match names.len() {
        0 => {}
        1 => {
            push_plain(&mut block, format!("{} ", prefix), &plain_style);
            push_plain(
                &mut block,
                sg::with_indefinite_article(&names[0]),
                &colored_style,
            );
            push_plain(&mut block, " here.\n".to_string(), &plain_style);
        }
        2 => {
            push_plain(&mut block, format!("{} ", prefix), &plain_style);
            push_plain(
                &mut block,
                sg::with_indefinite_article(&names[0]),
                &colored_style,
            );
            push_plain(&mut block, " and ".to_string(), &plain_style);
            push_plain(
                &mut block,
                sg::with_indefinite_article(&names[1]),
                &colored_style,
            );
            push_plain(&mut block, " here.\n".to_string(), &plain_style);
        }
        _ => {
            push_plain(&mut block, format!("{} ", prefix), &plain_style);
            for (index, name) in names.iter().enumerate() {
                if index > 0 {
                    if index == names.len() - 1 {
                        push_plain(&mut block, "and ".to_string(), &plain_style);
                    } else {
                        push_plain(&mut block, ", ".to_string(), &plain_style);
                    }
                }
                push_plain(
                    &mut block,
                    sg::with_indefinite_article(name),
                    &colored_style,
                );
                if index < names.len() - 2 {
                    push_plain(&mut block, ", ".to_string(), &plain_style);
                } else if index == names.len() - 2 {
                    push_plain(&mut block, " ".to_string(), &plain_style);
                }
            }
            push_plain(&mut block, " here.\n".to_string(), &plain_style);
        }
    }

    block
}

fn append_block_plain_text(block: &mut TheTextViewBlock, text: &str) {
    if text.is_empty() {
        return;
    }
    if block.spans.is_empty() {
        block.text.push_str(text);
    } else {
        block.spans.push(TheTextViewSpan {
            text: text.to_string(),
            style: TheTextStyle {
                foreground: None,
                background: None,
                underline: None,
            },
        });
    }
}

fn block_ends_with(block: &TheTextViewBlock, suffix: &str) -> bool {
    if !block.spans.is_empty() {
        let mut text = String::new();
        for span in &block.spans {
            text.push_str(&span.text);
        }
        text.ends_with(suffix)
    } else {
        block.text.ends_with(suffix)
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
                    if intent.trim().eq_ignore_ascii_case("look") {
                        0.0
                    } else {
                        distance
                    },
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
                    if intent.trim().eq_ignore_ascii_case("look") {
                        0.0
                    } else {
                        distance
                    },
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

fn current_time_label(project: &Project, server_ctx: &ServerContext) -> Option<String> {
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
    Some(label)
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

fn authoring_colors(src: &str) -> TextGameColors {
    let mut message_categories = FxHashMap::default();
    let Some(table) = config_table(src) else {
        return TextGameColors {
            title: None,
            items: None,
            corpses: None,
            message_categories,
        };
    };
    let Some(colors) = table.get("colors").and_then(toml::Value::as_table) else {
        return TextGameColors {
            title: None,
            items: None,
            corpses: None,
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
        items: colors
            .get("items")
            .and_then(toml::Value::as_str)
            .and_then(parse_named_text_color),
        corpses: colors
            .get("corpses")
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
