use crossterm::cursor;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::{Frame, Terminal as RatatuiTerminal};
use rusterix::prelude::*;
use rusterix::{Command, EntityAction, PlayerCamera, ServerState};
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, ExternalPrinter};
use shared::prelude::{TextSession, TextSessionOutput};
use shared::project::Project;
use shared::terminal_screen::TerminalScreenFrame;
use shared::text_game as sg;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use toml::Table;

#[derive(Clone, Copy, PartialEq, Eq)]
enum StartupDisplay {
    Description,
    Room,
    None,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TerminalPlayMode {
    Text,
    Roguelike,
}

impl TerminalPlayMode {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "text" => Ok(Self::Text),
            "roguelike" | "rogue" => Ok(Self::Roguelike),
            other => Err(format!(
                "Unknown terminal mode '{}'. Expected 'text' or 'roguelike'.",
                other
            )),
        }
    }
}

struct TerminalCliOptions {
    path: Option<PathBuf>,
    mode: Option<TerminalPlayMode>,
}

struct TerminalApp {
    project: Project,
    assets: Assets,
    server: Server,
    current_map: String,
    session: TextSession,
    screen_messages: Vec<String>,
    auto_attack_target: Option<u32>,
    server_log_cursor: usize,
}

enum InputEvent {
    Line(String),
    Eof,
    Error(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AutoAttackMode {
    Never,
    OnAttack,
}

impl TerminalApp {
    fn load(path: &Path) -> Result<Self, String> {
        let contents = fs::read_to_string(path)
            .map_err(|err| format!("Failed to read {}: {}", path.display(), err))?;
        let mut project: Project = serde_json::from_str(&contents)
            .map_err(|err| format!("Failed to parse {}: {}", path.display(), err))?;
        project.migrate_default_ruleset();

        let current_map = config_string(&project.config, "game", "start_region", "");
        if current_map.is_empty() {
            return Err("Game config is missing [game].start_region".into());
        }

        let mut app = Self {
            project,
            assets: Assets::default(),
            server: Server::default(),
            current_map,
            session: TextSession::new(),
            screen_messages: Vec::new(),
            auto_attack_target: None,
            server_log_cursor: 0,
        };

        app.start_server(false)?;
        app.create_local_player()?;

        for _ in 0..3 {
            app.tick();
            thread::sleep(Duration::from_millis(5));
        }

        app.session
            .set_current_hour(app.current_time_hour_and_label().map(|(hour, _)| hour));

        Ok(app)
    }

    fn start_server(&mut self, debug: bool) -> Result<(), String> {
        self.server.clear();
        self.server.debug_mode = debug;
        self.server.log_changed = true;
        self.server.print_log_messages = false;
        self.server_log_cursor = 0;

        insert_content_into_maps_mode(&mut self.project, debug);

        if json_module_has_routines(&self.project.world_module) {
            if self.project.world_source.is_empty() {
                if let Some(source) = json_module_build(&self.project.world_module, false) {
                    self.project.world_source = source;
                }
            }
            if debug {
                if let Some(source) = json_module_build(&self.project.world_module, true) {
                    self.project.world_source_debug = source;
                }
            }
        }

        self.assets.rules =
            shared::rulesets::resolve_project_rules(&self.project.config, &self.project.rules)
                .unwrap_or_else(|err| {
                    eprintln!("Ruleset resolution error: {}", err);
                    self.project.rules.clone()
                });
        self.assets.read_rules_metadata();
        self.assets.locales_src = self.project.locales.clone();
        self.assets.audio_fx_src = self.project.audio_fx.clone();
        self.assets.authoring_src = self.project.authoring.clone();
        self.assets.world_source = if debug && !self.project.world_source_debug.is_empty() {
            self.project.world_source_debug.clone()
        } else {
            self.project.world_source.clone()
        };
        self.assets.region_sources.clear();
        self.assets.read_locales();

        self.assets.entities.clear();
        self.assets.character_maps.clear();
        self.assets.entity_tiles.clear();
        self.assets.entity_authoring.clear();
        for character in self.project.characters.values_mut() {
            if json_module_has_routines(&character.module) {
                if character.source.is_empty() {
                    if let Some(source) = json_module_build(&character.module, false) {
                        character.source = source;
                    }
                }
                if debug {
                    if let Some(source) = json_module_build(&character.module, true) {
                        character.source_debug = source;
                    }
                }
            }
            if debug && !character.source_debug.is_empty() {
                self.assets.entities.insert(
                    character.name.clone(),
                    (character.source_debug.clone(), character.data.clone()),
                );
            } else {
                self.assets.entities.insert(
                    character.name.clone(),
                    (character.source.clone(), character.data.clone()),
                );
            }
            if !character.map.vertices.is_empty() {
                self.assets
                    .character_maps
                    .insert(character.name.clone(), character.map.clone());
            }
            self.assets
                .entity_authoring
                .insert(character.name.clone(), character.authoring.clone());
        }

        self.assets.items.clear();
        self.assets.item_maps.clear();
        self.assets.item_tiles.clear();
        self.assets.item_authoring.clear();
        for item in self.project.items.values_mut() {
            if json_module_has_routines(&item.module) {
                if item.source.is_empty() {
                    if let Some(source) = json_module_build(&item.module, false) {
                        item.source = source;
                    }
                }
                if debug {
                    if let Some(source) = json_module_build(&item.module, true) {
                        item.source_debug = source;
                    }
                }
            }
            if debug && !item.source_debug.is_empty() {
                self.assets.items.insert(
                    item.name.clone(),
                    (item.source_debug.clone(), item.data.clone()),
                );
            } else {
                self.assets
                    .items
                    .insert(item.name.clone(), (item.source.clone(), item.data.clone()));
            }
            if !item.map.vertices.is_empty() {
                self.assets
                    .item_maps
                    .insert(item.name.clone(), item.map.clone());
            }
            self.assets
                .item_authoring
                .insert(item.name.clone(), item.authoring.clone());
        }

        self.assets.avatars.clear();
        match shared::rulesets::bundled_avatars_for_project(&self.project.config) {
            Ok(avatars) => {
                for (id, avatar) in avatars {
                    self.assets.avatars.insert(id.to_string(), avatar);
                }
            }
            Err(err) => eprintln!("Ruleset avatar load error: {}", err),
        }
        for avatar in self.project.avatars.values() {
            self.assets
                .avatars
                .insert(avatar.name.clone(), avatar.clone());
        }

        for region in &mut self.project.regions {
            if json_module_has_routines(&region.module) {
                if region.source.is_empty() {
                    if let Some(source) = json_module_build(&region.module, false) {
                        region.source = source;
                    }
                }
                if debug {
                    if let Some(source) = json_module_build(&region.module, true) {
                        region.source_debug = source;
                    }
                }
            }
            self.assets.region_sources.insert(
                region.map.id,
                if debug && !region.source_debug.is_empty() {
                    region.source_debug.clone()
                } else {
                    region.source.clone()
                },
            );
            let region_config =
                shared::project::merge_config_toml(&self.project.config, &region.config);
            self.server.create_region_instance(
                region.name.clone(),
                region.map.clone(),
                &self.assets,
                region_config,
            );
        }

        thread::sleep(Duration::from_millis(10));
        for region in &self.project.regions {
            self.server.set_time(&region.map.id, self.project.time);
        }
        self.server.set_state(ServerState::Running);
        Ok(())
    }

    fn create_local_player(&mut self) -> Result<(), String> {
        if !config_bool(&self.project.config, "game", "auto_create_player", false) {
            return Ok(());
        }

        let region = self
            .project
            .regions
            .iter()
            .find(|region| region.map.name == self.current_map)
            .ok_or_else(|| format!("Start region '{}' not found", self.current_map))?;

        let player_entities = self.player_template_names();
        let entity = region
            .map
            .entities
            .iter()
            .find(|entity| {
                entity
                    .get_attr_string("class_name")
                    .map(|class_name| player_entities.contains(&class_name))
                    .unwrap_or(false)
            })
            .cloned()
            .ok_or_else(|| {
                format!(
                    "No auto-created player entity found in start region '{}'",
                    self.current_map
                )
            })?;

        self.server
            .process_client_commands(vec![Command::CreateEntity(region.map.id, entity)]);
        Ok(())
    }

    fn player_template_names(&self) -> Vec<String> {
        let mut players = Vec::new();
        for (name, (_, data)) in &self.assets.entities {
            if let Ok(table) = data.parse::<Table>()
                && let Some(attributes) = table.get("attributes").and_then(toml::Value::as_table)
                && attributes
                    .get("player")
                    .and_then(toml::Value::as_bool)
                    .unwrap_or(false)
            {
                players.push(name.clone());
            }
        }
        players
    }

    fn current_region_index(&self) -> Option<usize> {
        self.project
            .regions
            .iter()
            .position(|region| region.map.name == self.current_map)
    }

    fn current_region(&self) -> Option<&shared::region::Region> {
        self.project
            .regions
            .iter()
            .find(|region| region.map.name == self.current_map)
    }

    fn tick(&mut self) {
        self.server.system_tick();
        self.server.redraw_tick();

        if let Some(new_region) = self.server.update(&mut self.assets) {
            self.current_map = new_region;
        }

        if let Some(index) = self.current_region_index() {
            let region_id = self.project.regions[index].map.id;
            let time = self.server.get_time(&region_id);
            self.server
                .apply_entities_items(&mut self.project.regions[index].map);
            if let Some(time) = time {
                self.project.time = time;
            }
        }
    }

    fn drain_server_diagnostics(&mut self) -> Vec<String> {
        if !self.server.log_changed {
            return Vec::new();
        }

        let log = self.server.get_log();
        let start = self.server_log_cursor.min(log.len());
        self.server_log_cursor = log.len();
        log[start..]
            .lines()
            .map(str::trim)
            .filter(|line| is_terminal_diagnostic_line(line))
            .map(str::to_string)
            .collect()
    }

    fn current_time_hour_and_label(&self) -> Option<(u8, String)> {
        self.project
            .regions
            .iter()
            .find(|region| region.map.name == self.current_map)
            .and_then(|region| self.server.get_time(&region.map.id))
            .map(|time| {
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
                (time.hours, label)
            })
    }

    fn render_room_text(&mut self) -> String {
        let title_color = authoring_color(&self.project.authoring, "title");
        let object_color = authoring_color(&self.project.authoring, "objects");
        let item_color =
            authoring_color(&self.project.authoring, "items").or_else(|| object_color.clone());
        let corpse_color =
            authoring_color(&self.project.authoring, "corpses").or_else(|| object_color.clone());
        let neutral_character_color =
            authoring_color(&self.project.authoring, "characters").or_else(|| object_color.clone());
        let character_color_rules = authoring_character_color_rules(&self.project.authoring);
        let Some(region) = self.current_region() else {
            return format!("Current region '{}' not found.", self.current_map);
        };
        let Some((_, sector)) = sg::current_player_and_sector(&region.map) else {
            return "No local player found.".into();
        };
        let Some(room) = sg::build_text_room(&region.map, &self.project.authoring) else {
            return "No room text available.".into();
        };

        let mut lines = Vec::new();
        if !room.title.is_empty() {
            lines.push(colorize_terminal_text(&room.title, title_color.as_deref()));
            lines.push(String::new());
        }
        if !room.description.is_empty() {
            lines.push(room.description.clone());
            lines.push(String::new());
        }

        if !room.exits.is_empty() {
            match sg::authoring_exit_presentation(&self.project.authoring) {
                sg::ExitPresentation::List => {
                    lines.push("Exits:".into());
                    for exit in &room.exits {
                        lines.push(format!("  {} - {}", exit.direction, exit.title));
                    }
                }
                sg::ExitPresentation::Sentence => lines.push(sg::render_exit_sentence(&room.exits)),
            }
        }

        let mut live_entities = Vec::new();
        let mut dead_entities = Vec::new();
        for entity in &region.map.entities {
            if entity.is_player() {
                continue;
            }
            if !sg::entity_sector_matches(&region.map, entity, sector) {
                continue;
            }

            if sg::entity_is_dead(entity) {
                let corpse = sg::corpse_name_for_entity(entity);
                if !corpse.trim().is_empty() {
                    dead_entities.push(colorized_presence_phrase(&corpse, corpse_color.as_deref()));
                }
            } else {
                let name = sg::display_name_for_entity(entity);
                if !name.trim().is_empty() {
                    let color = resolve_character_color(entity, &character_color_rules)
                        .or(neutral_character_color.as_deref());
                    live_entities.push(colorized_presence_phrase(&name, color));
                }
            }
        }
        if !live_entities.is_empty() {
            lines.push(sg::render_presence_sentence("You see", &live_entities));
        }
        if !room.nearby_attackers.is_empty() {
            lines.push(sg::render_nearby_attackers_sentence(&room.nearby_attackers));
        }
        if !dead_entities.is_empty() {
            lines.push(sg::render_presence_sentence("You see", &dead_entities));
        }

        let items: Vec<String> = region
            .map
            .items
            .iter()
            .filter(|item| sg::item_sector_matches(&region.map, item, sector))
            .map(sg::display_name_for_item)
            .filter(|name| !name.trim().is_empty())
            .map(|name| colorized_presence_phrase(&name, item_color.as_deref()))
            .collect();
        if !items.is_empty() {
            lines.push(sg::render_presence_sentence("You notice", &items));
        }

        if lines.is_empty() {
            "No room text available.".into()
        } else {
            lines.join("\n")
        }
    }

    fn render_current_sector_description(&mut self) -> Option<String> {
        let Some(region) = self.current_region() else {
            return None;
        };
        sg::render_current_sector_description(&region.map)
    }

    fn move_to_exit(&mut self, exit: &sg::TextExit) -> bool {
        self.auto_attack_target = None;
        self.server.local_player_teleport_pos(exit.target_center);
        self.tick();
        self.current_region().and_then(|region| {
            sg::current_player_and_sector(&region.map).map(|(_, sector)| sector.id)
        }) == Some(exit.target_sector_id)
    }

    fn print_pending_messages(&mut self) -> usize {
        let Some(index) = self.current_region_index() else {
            return 0;
        };
        let region_id = self.project.regions[index].map.id;
        let mut count = 0;

        for (_sender_entity, _sender_item, _receiver, message, _category) in
            self.server.get_messages(&region_id)
        {
            println!("{}", message);
            count += 1;
        }

        for (_sender_entity, _sender_item, message, _category) in self.server.get_says(&region_id) {
            println!("{}", message);
            count += 1;
        }

        count
    }

    fn discard_pending_messages(&mut self) -> usize {
        let Some(index) = self.current_region_index() else {
            return 0;
        };
        let region_id = self.project.regions[index].map.id;
        let message_count = self.server.get_messages(&region_id).len();
        let say_count = self.server.get_says(&region_id).len();
        message_count + say_count
    }

    fn process_auto_attack(&mut self) {
        if authoring_auto_attack_mode(&self.project.authoring) != AutoAttackMode::OnAttack {
            self.auto_attack_target = None;
            self.session.clear_auto_attack_target();
            return;
        }

        let Some(target_id) = self.session.auto_attack_target() else {
            return;
        };

        let Some(region) = self.current_region() else {
            self.auto_attack_target = None;
            self.session.clear_auto_attack_target();
            return;
        };
        let Some((player, sector)) = sg::current_player_and_sector(&region.map) else {
            self.auto_attack_target = None;
            self.session.clear_auto_attack_target();
            return;
        };

        let Some(target) = region
            .map
            .entities
            .iter()
            .find(|entity| entity.id == target_id)
        else {
            self.auto_attack_target = None;
            self.session.clear_auto_attack_target();
            return;
        };
        if sg::entity_is_dead(target) {
            self.auto_attack_target = None;
            self.session.clear_auto_attack_target();
            return;
        }

        let same_sector = sg::entity_sector_matches(&region.map, target, sector);
        if !same_sector {
            self.auto_attack_target = None;
            self.session.clear_auto_attack_target();
            return;
        }

        let distance = player.get_pos_xz().distance(target.get_pos_xz());
        self.server.local_player_action(EntityAction::EntityClicked(
            target_id,
            distance,
            Some("attack".into()),
        ));
    }
}

fn config_table(src: &str) -> Option<Table> {
    src.parse::<Table>().ok()
}

fn json_module_has_routines(module: &serde_json::Value) -> bool {
    module
        .get("routines")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|routines| !routines.is_empty())
}

fn json_module_build(_module: &serde_json::Value, _debug: bool) -> Option<String> {
    None
}

fn config_string(src: &str, section: &str, key: &str, default: &str) -> String {
    config_table(src)
        .and_then(|table| {
            table
                .get(section)
                .and_then(toml::Value::as_table)
                .and_then(|table| table.get(key))
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| default.to_string())
}

fn config_bool(src: &str, section: &str, key: &str, default: bool) -> bool {
    config_table(src)
        .and_then(|table| {
            table
                .get(section)
                .and_then(toml::Value::as_table)
                .and_then(|table| table.get(key))
                .and_then(toml::Value::as_bool)
        })
        .unwrap_or(default)
}

fn config_terminal_mode(src: &str) -> Result<TerminalPlayMode, String> {
    let mode = config_string(src, "game", "terminal_mode", "text");
    TerminalPlayMode::parse(&mode)
}

fn authoring_startup_display(src: &str) -> StartupDisplay {
    config_table(src)
        .and_then(|table| {
            table
                .get("startup")
                .and_then(toml::Value::as_table)
                .and_then(|table| table.get("show"))
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        })
        .map(|value| match value.trim().to_ascii_lowercase().as_str() {
            "none" => StartupDisplay::None,
            "room" => StartupDisplay::Room,
            _ => StartupDisplay::Description,
        })
        .unwrap_or(StartupDisplay::Description)
}

fn authoring_startup_welcome(src: &str) -> Option<String> {
    config_table(src)
        .and_then(|table| {
            table
                .get("startup")
                .and_then(toml::Value::as_table)
                .and_then(|table| table.get("welcome"))
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        })
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn authoring_color(src: &str, key: &str) -> Option<String> {
    config_table(src).and_then(|table| {
        table
            .get("colors")
            .and_then(toml::Value::as_table)
            .and_then(|table| table.get(key))
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

struct TerminalColorRule {
    attr: String,
    op: String,
    value: f32,
    color: String,
}

fn authoring_character_color_rules(src: &str) -> Vec<TerminalColorRule> {
    let Some(table) = config_table(src) else {
        return Vec::new();
    };
    let Some(colors) = table.get("colors").and_then(toml::Value::as_table) else {
        return Vec::new();
    };
    let Some(rules) = colors
        .get("character_rules")
        .and_then(toml::Value::as_array)
    else {
        return Vec::new();
    };

    rules
        .iter()
        .filter_map(toml::Value::as_table)
        .filter_map(|rule| {
            let when = rule.get("when").and_then(toml::Value::as_str)?.trim();
            let color = rule.get("color").and_then(toml::Value::as_str)?.trim();
            let (attr, op, value) = parse_color_rule_expr(when)?;
            if color.is_empty() {
                return None;
            }
            Some(TerminalColorRule {
                attr: attr.to_string(),
                op: op.to_string(),
                value,
                color: color.to_string(),
            })
        })
        .collect()
}

fn parse_color_rule_expr(filter: &str) -> Option<(&str, &str, f32)> {
    for op in ["<=", ">=", "==", "!=", "<", ">"] {
        if let Some(index) = filter.find(op) {
            let lhs = filter[..index].trim();
            let rhs = filter[index + op.len()..].trim();
            let value = rhs.parse::<f32>().ok()?;
            if lhs.is_empty() {
                return None;
            }
            return Some((lhs, op, value));
        }
    }
    None
}

fn resolve_character_color<'a>(entity: &Entity, rules: &'a [TerminalColorRule]) -> Option<&'a str> {
    for rule in rules {
        let attr = entity.attributes.get(&rule.attr)?;
        let lhs = match attr {
            Value::Int(v) => *v as f32,
            Value::Int64(v) => *v as f32,
            Value::UInt(v) => *v as f32,
            Value::Float(v) => *v,
            _ => continue,
        };
        let matched = match rule.op.as_str() {
            "<" => lhs < rule.value,
            "<=" => lhs <= rule.value,
            ">" => lhs > rule.value,
            ">=" => lhs >= rule.value,
            "==" => (lhs - rule.value).abs() < f32::EPSILON,
            "!=" => (lhs - rule.value).abs() >= f32::EPSILON,
            _ => false,
        };
        if matched {
            return Some(rule.color.as_str());
        }
    }
    None
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

fn colorized_presence_phrase(name: &str, color: Option<&str>) -> String {
    colorize_terminal_text(name, color)
}

fn colorize_terminal_text(text: &str, color: Option<&str>) -> String {
    if text.is_empty() || !std::io::stdout().is_terminal() {
        return text.to_string();
    }

    let code = match color.map(|c| c.trim().to_ascii_lowercase()) {
        Some(color) if color == "black" => Some("30"),
        Some(color) if color == "red" => Some("31"),
        Some(color) if color == "green" => Some("32"),
        Some(color) if color == "yellow" => Some("33"),
        Some(color) if color == "blue" => Some("34"),
        Some(color) if color == "magenta" => Some("35"),
        Some(color) if color == "cyan" => Some("36"),
        Some(color) if color == "white" => Some("37"),
        Some(color) if color == "bright_black" || color == "gray" || color == "grey" => Some("90"),
        Some(color) if color == "bright_red" => Some("91"),
        Some(color) if color == "bright_green" => Some("92"),
        Some(color) if color == "bright_yellow" => Some("93"),
        Some(color) if color == "bright_blue" => Some("94"),
        Some(color) if color == "bright_magenta" => Some("95"),
        Some(color) if color == "bright_cyan" => Some("96"),
        Some(color) if color == "bright_white" => Some("97"),
        _ => None,
    };

    if let Some(code) = code {
        format!("\x1b[{}m{}\x1b[0m", code, text)
    } else {
        text.to_string()
    }
}

fn insert_content_into_maps_mode(project: &mut Project, debug: bool) {
    for region in &mut project.regions {
        region.map.entities.clear();
        for instance in region.characters.values_mut() {
            if json_module_has_routines(&instance.module) {
                if instance.source.is_empty() {
                    if let Some(source) = json_module_build(&instance.module, false) {
                        instance.source = source;
                    }
                }
                if debug {
                    if let Some(source) = json_module_build(&instance.module, true) {
                        instance.source_debug = source;
                    }
                }
            }
            let mut entity = Entity {
                creator_id: instance.id,
                position: instance.position,
                orientation: instance.orientation,
                ..Default::default()
            };
            entity.set_attribute("name", Value::Str(instance.name.clone()));
            if let Some(character_template) = project.characters.get(&instance.character_id) {
                entity.set_attribute("name", Value::Str(character_template.name.clone()));
            }
            entity.set_attribute(
                "setup",
                Value::Str(if debug && !instance.source_debug.is_empty() {
                    instance.source_debug.clone()
                } else {
                    instance.source.clone()
                }),
            );
            if let Some(character) = project.characters.get(&instance.character_id) {
                entity.set_attribute("class_name", Value::Str(character.name.clone()));
                apply_entity_data_attributes(&mut entity, &character.data);
            }
            apply_entity_data_attributes(&mut entity, &instance.data);
            region.map.entities.push(entity);
        }

        region.map.items.clear();
        for instance in region.items.values_mut() {
            if json_module_has_routines(&instance.module) {
                if instance.source.is_empty() {
                    if let Some(source) = json_module_build(&instance.module, false) {
                        instance.source = source;
                    }
                }
                if debug {
                    if let Some(source) = json_module_build(&instance.module, true) {
                        instance.source_debug = source;
                    }
                }
            }
            let mut item = Item {
                creator_id: instance.id,
                position: instance.position,
                ..Default::default()
            };
            item.set_attribute("name", Value::Str(instance.name.clone()));
            if let Some(item_template) = project.items.get(&instance.item_id) {
                item.set_attribute("name", Value::Str(item_template.name.clone()));
            }
            item.set_attribute(
                "setup",
                Value::Str(if debug && !instance.source_debug.is_empty() {
                    instance.source_debug.clone()
                } else {
                    instance.source.clone()
                }),
            );
            if let Some(item_template) = project.items.get(&instance.item_id) {
                item.set_attribute("class_name", Value::Str(item_template.name.clone()));
                apply_item_data_attributes(&mut item, &item_template.data);
            }
            apply_item_data_attributes(&mut item, &instance.data);
            region.map.items.push(item);
        }
    }
}

fn apply_entity_data_attributes(entity: &mut Entity, data: &str) {
    let Ok(table) = toml::from_str::<Table>(data) else {
        return;
    };
    let Some(attributes) = table.get("attributes").and_then(toml::Value::as_table) else {
        return;
    };
    for (key, value) in attributes {
        if let Some(value) = source_data_value_to_runtime(value) {
            entity.set_attribute(key, value);
        }
    }
}

fn apply_item_data_attributes(item: &mut Item, data: &str) {
    let Ok(table) = toml::from_str::<Table>(data) else {
        return;
    };
    let Some(attributes) = table.get("attributes").and_then(toml::Value::as_table) else {
        return;
    };
    for (key, value) in attributes {
        if let Some(value) = source_data_value_to_runtime(value) {
            item.set_attribute(key, value);
        }
    }
}

fn source_data_value_to_runtime(value: &toml::Value) -> Option<Value> {
    match value {
        toml::Value::String(value) => Some(Value::Str(value.clone())),
        toml::Value::Integer(value) if *value >= 0 => Some(Value::UInt(*value as u32)),
        toml::Value::Integer(value) => Some(Value::Int(*value as i32)),
        toml::Value::Float(value) => Some(Value::Float(*value as f32)),
        toml::Value::Boolean(value) => Some(Value::Bool(*value)),
        toml::Value::Array(values) => {
            let strings = values
                .iter()
                .filter_map(toml::Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>();
            (!strings.is_empty()).then_some(Value::StrArray(strings))
        }
        _ => None,
    }
}

fn parse_terminal_args(args: &[String]) -> Result<TerminalCliOptions, String> {
    let mut options = TerminalCliOptions {
        path: None,
        mode: None,
    };
    let mut index = 1;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--mode" {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err("Missing value after --mode.".to_string());
            };
            options.mode = Some(TerminalPlayMode::parse(value)?);
        } else if let Some(value) = arg.strip_prefix("--mode=") {
            options.mode = Some(TerminalPlayMode::parse(value)?);
        } else if arg == "--help" || arg == "-h" || arg == "help" {
            return Err(terminal_usage().to_string());
        } else if arg.starts_with('-') {
            return Err(format!("Unknown option '{}'.\n{}", arg, terminal_usage()));
        } else if options.path.is_none() {
            options.path = Some(PathBuf::from(arg));
        } else {
            return Err(format!(
                "Unexpected argument '{}'.\n{}",
                arg,
                terminal_usage()
            ));
        }
        index += 1;
    }
    Ok(options)
}

fn terminal_usage() -> &'static str {
    "Usage:\n\
       eldiron-client-terminal [game.eldiron] [--mode text|roguelike]\n\
       eldiron-client-terminal rules <command> ...\n\
     Modes:\n\
       text       Current room/description terminal play.\n\
       roguelike  Terminal glyph-map play mode for source-authored maps."
}

fn resolve_data_path(path_arg: Option<&PathBuf>) -> Result<PathBuf, String> {
    if let Some(path) = path_arg {
        return Ok(path.clone());
    }

    let cwd = std::env::current_dir().map_err(|err| err.to_string())?;
    let game_path = cwd.join("game.eldiron");
    if game_path.exists() {
        return Ok(game_path);
    }

    let mut candidates = Vec::new();
    for entry in fs::read_dir(&cwd).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        let is_eldiron = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("eldiron"))
            .unwrap_or(false);
        if is_eldiron {
            candidates.push(path);
        }
    }

    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => Err("No .eldiron file found in the current directory.".into()),
        _ => {
            candidates.sort();
            let names: Vec<String> = candidates
                .iter()
                .map(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("<invalid>")
                        .to_string()
                })
                .collect();
            Err(format!(
                "Multiple .eldiron files found in the current directory: {}. Pass the game path explicitly.",
                names.join(", ")
            ))
        }
    }
}

fn rules_command_usage() -> &'static str {
    "Usage:\n\
       eldiron-client-terminal rules check [game.eldiron]\n\
       eldiron-client-terminal rules summary [game.eldiron]\n\
       eldiron-client-terminal rules character <class_id> [race=Race] [level=N] [ATTR=VALUE ...]\n\
       eldiron-client-terminal rules item <item_id> [ATTR=VALUE ...]\n\
       eldiron-client-terminal rules class <class_id>\n\
       eldiron-client-terminal rules recipe <recipe_id>\n\
       eldiron-client-terminal rules xp <level>\n\
       eldiron-client-terminal rules weapon <weapon_id> [ATTR=VALUE ...]\n\
       eldiron-client-terminal rules spell <spell_id> [ATTR=VALUE ...]\n\
       eldiron-client-terminal rules roll <ruleset.path.to.roll> [ATTR=VALUE ...]\n\
     Examples:\n\
       eldiron-client-terminal rules check\n\
       eldiron-client-terminal rules check test_projects/Hideout2D.eldiron\n\
       eldiron-client-terminal rules summary\n\
       eldiron-client-terminal rules character Cleric race=Human level=2\n\
       eldiron-client-terminal rules item training_sword STR=12\n\
       eldiron-client-terminal rules class Warrior\n\
       eldiron-client-terminal rules recipe wooden_arrows\n\
       eldiron-client-terminal rules xp 5\n\
       eldiron-client-terminal rules weapon training_sword STR=12\n\
       eldiron-client-terminal rules spell fire_spark INT=12"
}

fn run_rules_command(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("check") => run_rules_check_command(&args[1..]),
        Some("summary") => run_rules_summary_command(&args[1..]),
        Some("character") => run_rules_character_command(&args[1..]),
        Some("item") => run_rules_item_command(&args[1..]),
        Some("class") => run_rules_class_command(&args[1..]),
        Some("recipe") => run_rules_recipe_command(&args[1..]),
        Some("xp") => run_rules_xp_command(&args[1..]),
        Some("weapon") => run_rules_weapon_command(&args[1..]),
        Some("spell") => run_rules_spell_command(&args[1..]),
        Some("roll") => run_rules_roll_command(&args[1..]),
        _ => Err(rules_command_usage().into()),
    }
}

fn optional_rules_project_path(args: &[String]) -> Result<Option<&Path>, String> {
    match args {
        [] => Ok(None),
        [path] => Ok(Some(Path::new(path))),
        _ => Err(rules_command_usage().into()),
    }
}

fn project_rules_table(path: &Path) -> Result<toml::Table, String> {
    let contents = fs::read_to_string(path)
        .map_err(|err| format!("Failed to read {}: {}", path.display(), err))?;
    let mut project: Project = serde_json::from_str(&contents)
        .map_err(|err| format!("Failed to parse {}: {}", path.display(), err))?;
    project.migrate_default_ruleset();
    let rules = shared::rulesets::resolve_project_rules(&project.config, &project.rules)?;
    rules
        .parse::<toml::Table>()
        .map_err(|err| format!("Resolved ruleset TOML parse error: {}", err))
}

fn rules_table_for_terminal_command(args: &[String]) -> Result<(toml::Table, String), String> {
    if let Some(path) = optional_rules_project_path(args)? {
        Ok((project_rules_table(path)?, path.display().to_string()))
    } else {
        Ok((official_rules_table()?, "bundled official ruleset".into()))
    }
}

fn format_rules_list(values: &[String]) -> String {
    if values.is_empty() {
        "-".into()
    } else {
        values.join(", ")
    }
}

fn format_rules_catalog_summary(
    source: &str,
    catalog: &shared::rulesets::RulesetCatalog,
) -> String {
    format!(
        "source: {}\n\
         id: {}\n\
         version: {}\n\
         schema: {}\n\
         races: {} ({})\n\
         classes: {} ({})\n\
         professions: {} ({})\n\
         skills: {} ({})\n\
         resources: {} ({})\n\
         recipes: {} ({})\n\
         actions: {} ({})\n\
         abilities: {} ({})\n\
         spells: {} ({})\n\
         weapons: {} ({})\n\
         armor: {} ({})\n\
         clothing: {} ({})\n\
         item templates: {}\n\
         fx presets: {} ({})",
        source,
        catalog.id.as_deref().unwrap_or("-"),
        catalog.version.as_deref().unwrap_or("-"),
        catalog.schema_version.as_deref().unwrap_or("-"),
        catalog.races.len(),
        format_rules_list(&catalog.races),
        catalog.classes.len(),
        format_rules_list(&catalog.classes),
        catalog.professions.len(),
        format_rules_list(&catalog.professions),
        catalog.skills.len(),
        format_rules_list(&catalog.skills),
        catalog.resources.len(),
        format_rules_list(&catalog.resources),
        catalog.recipes.len(),
        format_rules_list(&catalog.recipes),
        catalog.actions.len(),
        format_rules_list(&catalog.actions),
        catalog.abilities.len(),
        format_rules_list(&catalog.abilities),
        catalog.spells.len(),
        format_rules_list(&catalog.spells),
        catalog.weapons.len(),
        format_rules_list(&catalog.weapons),
        catalog.armor.len(),
        format_rules_list(&catalog.armor),
        catalog.clothing.len(),
        format_rules_list(&catalog.clothing),
        catalog.item_templates.len(),
        catalog.fx_presets.len(),
        format_rules_list(&catalog.fx_presets),
    )
}

fn format_rules_validation_report(
    source: &str,
    report: &shared::rulesets::RulesetValidationReport,
) -> String {
    let mut lines = vec![format!(
        "ruleset check: {} ({} errors, {} warnings)",
        if report.is_ok() { "ok" } else { "failed" },
        report.error_count(),
        report.warning_count()
    )];
    lines.push(format!("source: {}", source));

    for issue in &report.issues {
        let severity = match issue.severity {
            shared::rulesets::RulesetValidationSeverity::Error => "error",
            shared::rulesets::RulesetValidationSeverity::Warning => "warning",
        };
        lines.push(format!("{} {}: {}", severity, issue.path, issue.message));
    }

    lines.join("\n")
}

fn run_rules_summary_command(args: &[String]) -> Result<(), String> {
    let (rules, source) = rules_table_for_terminal_command(args)?;
    let catalog = shared::rulesets::ruleset_catalog(&rules);
    println!("{}", format_rules_catalog_summary(&source, &catalog));
    Ok(())
}

fn run_rules_check_command(args: &[String]) -> Result<(), String> {
    let (rules, source) = rules_table_for_terminal_command(args)?;
    let report = shared::rulesets::validate_ruleset(&rules);
    println!("{}", format_rules_validation_report(&source, &report));
    if report.error_count() > 0 {
        return Err("Ruleset check failed.".into());
    }
    Ok(())
}

fn parse_rules_attributes(
    args: &[String],
) -> Result<shared::rulesets::RulesetAttributeMap, String> {
    let mut attributes = shared::rulesets::RulesetAttributeMap::new();
    for raw in args {
        let Some((key, value)) = raw.split_once('=') else {
            return Err(format!("Attribute '{}' must use ATTR=VALUE syntax.", raw));
        };
        let key = key.trim();
        if key.is_empty() {
            return Err(format!("Attribute '{}' has an empty name.", raw));
        }
        let value = value
            .trim()
            .parse::<f32>()
            .map_err(|_| format!("Attribute '{}' has a non-numeric value.", raw))?;
        attributes.insert(key.to_string(), value);
    }
    Ok(attributes)
}

fn official_rules_table() -> Result<toml::Table, String> {
    shared::rulesets::latest_official_ruleset()
        .parse::<toml::Table>()
        .map_err(|err| format!("Official ruleset TOML parse error: {}", err))
}

fn format_roll_summary(label: &str, summary: &shared::rulesets::RulesetRollSummary) -> String {
    let attr_line = if let Some(attribute) = summary.spec.bonus_attribute.as_deref() {
        format!(
            "{}={} => +{} every {}",
            attribute, summary.attribute_value, summary.attribute_bonus, summary.spec.bonus_every
        )
    } else {
        "none".into()
    };
    let kind_line = summary
        .spec
        .damage_kind
        .as_deref()
        .map(|kind| format!("\ndamage kind: {}", kind))
        .unwrap_or_default();

    format!(
        "{}\nroll: {}\nbonus: {}\nattribute bonus: {}\ntotal bonus: {}\nmin: {}\nmax: {}\naverage: {:.2}{}",
        label,
        summary.spec.roll,
        summary.spec.bonus,
        attr_line,
        summary.total_bonus,
        summary.minimum,
        summary.maximum,
        summary.average,
        kind_line
    )
}

fn run_rules_roll_command(args: &[String]) -> Result<(), String> {
    let Some(path) = args.first() else {
        return Err(rules_command_usage().into());
    };
    let path_parts = path
        .split('.')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if path_parts.is_empty() {
        return Err(rules_command_usage().into());
    }

    let attributes = parse_rules_attributes(&args[1..])?;
    let rules = official_rules_table()?;
    let summary = shared::rulesets::summarize_roll_path(&rules, &path_parts, &attributes)?;
    println!("{}", format_roll_summary(path, &summary));
    Ok(())
}

fn run_rules_weapon_command(args: &[String]) -> Result<(), String> {
    let Some(weapon_id) = args.first() else {
        return Err(rules_command_usage().into());
    };
    let attributes = parse_rules_attributes(&args[1..])?;
    let rules = official_rules_table()?;
    let summary = shared::rulesets::summarize_weapon_damage(&rules, weapon_id, &attributes)?;
    println!(
        "{}",
        format_roll_summary(&format!("weapon: {}", weapon_id), &summary)
    );
    Ok(())
}

fn run_rules_spell_command(args: &[String]) -> Result<(), String> {
    let Some(spell_id) = args.first() else {
        return Err(rules_command_usage().into());
    };
    let attributes = parse_rules_attributes(&args[1..])?;
    let rules = official_rules_table()?;
    let (kind, summary) = shared::rulesets::summarize_spell_roll(&rules, spell_id, &attributes)?;
    println!(
        "{}",
        format_roll_summary(&format!("spell: {} ({})", spell_id, kind.label()), &summary)
    );
    Ok(())
}

fn run_rules_xp_command(args: &[String]) -> Result<(), String> {
    let Some(level) = args.first() else {
        return Err(rules_command_usage().into());
    };
    let level = level
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("Level '{}' is not a positive integer.", level))?;
    let rules = official_rules_table()?;
    let Some(xp) = shared::rulesets::ruleset_xp_for_level(&rules, level) else {
        return Err(format!("No XP entry for level {}.", level));
    };
    println!("level: {}\nrequired xp: {}", level, xp);
    Ok(())
}

fn join_or_dash(values: &[String]) -> String {
    if values.is_empty() {
        "-".into()
    } else {
        values.join(", ")
    }
}

fn parse_rules_character_value(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.eq_ignore_ascii_case("true") {
        Value::Bool(true)
    } else if trimmed.eq_ignore_ascii_case("false") {
        Value::Bool(false)
    } else if let Ok(value) = trimmed.parse::<i32>() {
        Value::Int(value)
    } else if let Ok(value) = trimmed.parse::<f32>() {
        Value::Float(value)
    } else {
        Value::Str(trimmed.to_string())
    }
}

fn set_rules_character_arg(entity: &mut Entity, key: &str, value: &str) -> Result<(), String> {
    let normalized_key = match key.trim().to_ascii_lowercase().as_str() {
        "" => return Err("Character argument has an empty key.".into()),
        "level" => "LEVEL".to_string(),
        "race" => "race".to_string(),
        "class" => "class".to_string(),
        _ => key.trim().to_string(),
    };
    entity.set_attribute(&normalized_key, parse_rules_character_value(value));
    Ok(())
}

fn rules_character_entity(args: &[String]) -> Result<Entity, String> {
    let Some(class_id) = args.first() else {
        return Err(rules_command_usage().into());
    };
    let mut entity = Entity::new();

    if let Some((key, value)) = class_id.split_once('=') {
        set_rules_character_arg(&mut entity, key, value)?;
    } else {
        entity.set_attribute("class", Value::Str(class_id.trim().to_string()));
    }

    for raw in &args[1..] {
        let Some((key, value)) = raw.split_once('=') else {
            return Err(format!(
                "Character argument '{}' must use KEY=VALUE syntax.",
                raw
            ));
        };
        set_rules_character_arg(&mut entity, key, value)?;
    }

    Ok(entity)
}

fn rules_character_numeric_attributes(entity: &Entity) -> shared::rulesets::RulesetAttributeMap {
    let mut attributes = shared::rulesets::RulesetAttributeMap::new();
    for key in entity.attributes.keys() {
        let Some(value) = entity.attributes.get(key) else {
            continue;
        };
        let number = match value {
            Value::Int(value) => Some(*value as f32),
            Value::UInt(value) => Some(*value as f32),
            Value::Int64(value) => Some(*value as f32),
            Value::Float(value) => Some(*value),
            _ => None,
        };
        if let Some(number) = number {
            attributes.insert(key.clone(), number);
        }
    }
    attributes
}

fn rules_character_string_array(entity: &Entity, key: &str) -> Vec<String> {
    match entity.attributes.get(key) {
        Some(Value::StrArray(values)) => values.clone(),
        Some(Value::Str(value)) if !value.trim().is_empty() => vec![value.trim().to_string()],
        _ => Vec::new(),
    }
}

fn rules_character_attribute_lines(entity: &Entity) -> Vec<String> {
    let preferred = [
        "HP",
        "MAX_HP",
        "MP",
        "MAX_MP",
        "STR",
        "DEX",
        "INT",
        "WIS",
        "VIT",
        "POWER",
        "DMG",
        "ARMOR",
        "RESIST",
        "INIT",
        "SPEED",
        "LEVEL",
        "EXP",
        "inventory_slots",
    ];
    let mut lines = Vec::new();
    let mut seen = BTreeSet::new();
    for key in preferred {
        if let Some(value) = entity.attributes.get(key) {
            lines.push(format!("{}: {}", key, value));
            seen.insert(key.to_string());
        }
    }

    let mut extra = entity
        .attributes
        .keys()
        .filter(|key| {
            !seen.contains(*key)
                && !matches!(
                    key.as_str(),
                    "race"
                        | "class"
                        | "start_equipped_items"
                        | "start_items"
                        | "abilities"
                        | "spells"
                )
        })
        .cloned()
        .collect::<Vec<_>>();
    extra.sort();
    for key in extra {
        if let Some(value) = entity.attributes.get(&key) {
            lines.push(format!("{}: {}", key, value));
        }
    }
    lines
}

fn rules_item_table<'a>(
    rules: &'a toml::Table,
    item_id: &str,
) -> Option<(&'static str, &'a toml::value::Table)> {
    for group in [
        "weapons",
        "armor",
        "clothing",
        "ammunition",
        "reagents",
        "materials",
        "resources",
    ] {
        if let Some(table) = shared::rulesets::ruleset_table_at_path(rules, &["items", group])
            .and_then(|items| items.get(item_id))
            .and_then(toml::Value::as_table)
        {
            return Some((group, table));
        }
    }
    None
}

fn table_string(table: &toml::value::Table, key: &str) -> Option<String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn rules_item_label(rules: &toml::Table, item_id: &str) -> String {
    if let Some((group, item)) = rules_item_table(rules, item_id) {
        let name = table_string(item, "name").unwrap_or_else(|| item_id.to_string());
        let slot = table_string(item, "slot").unwrap_or_else(|| "-".into());
        let category = table_string(item, "category").unwrap_or_else(|| "-".into());
        format!("{}: {} ({}, {})", slot, name, group, category)
    } else {
        format!("?: {}", item_id)
    }
}

fn rules_item_kind(group: &str) -> String {
    group.strip_suffix('s').unwrap_or(group).to_string()
}

fn toml_value_inline(value: &toml::Value) -> String {
    match value {
        toml::Value::String(value) => value.clone(),
        toml::Value::Integer(value) => value.to_string(),
        toml::Value::Float(value) => value.to_string(),
        toml::Value::Boolean(value) => value.to_string(),
        toml::Value::Array(values) => values
            .iter()
            .map(toml_value_inline)
            .collect::<Vec<_>>()
            .join(", "),
        _ => value.to_string(),
    }
}

fn table_lines_sorted(table: &toml::value::Table) -> Vec<String> {
    let mut lines = table
        .iter()
        .filter(|(_, value)| !value.is_table())
        .map(|(key, value)| format!("{}: {}", key, toml_value_inline(value)))
        .collect::<Vec<_>>();
    lines.sort();
    lines
}

fn rules_item_category_lines(
    rules: &toml::Table,
    group: &str,
    item: &toml::value::Table,
) -> Vec<String> {
    let Some(category) = table_string(item, "category") else {
        return Vec::new();
    };
    let category_group = match group {
        "weapons" => "weapon_categories",
        "armor" | "clothing" => "armor_categories",
        _ => return Vec::new(),
    };
    shared::rulesets::ruleset_table_at_path(rules, &["equipment", category_group, &category])
        .map(table_lines_sorted)
        .unwrap_or_default()
}

fn rules_item_visual_lines(item: &toml::value::Table) -> Vec<String> {
    let visual_keys = [
        "color",
        "avatar_channels",
        "visual_template",
        "icon_template",
        "icon_shape",
        "icon",
        "rig_template",
        "rig_scale",
        "rig_pivot",
        "rig_layer",
        "rig_flip_back",
        "blade_color",
        "blade_color_index",
        "grip_color",
        "grip_color_index",
        "accent_color",
        "accent_color_index",
        "highlight_color",
        "highlight_color_index",
    ];
    let mut lines = visual_keys
        .iter()
        .filter_map(|key| {
            item.get(*key)
                .map(|value| format!("{}: {}", key, toml_value_inline(value)))
        })
        .collect::<Vec<_>>();
    lines.sort();
    lines
}

fn format_rules_item(
    rules: &toml::Table,
    item_id: &str,
    attributes: &shared::rulesets::RulesetAttributeMap,
) -> Result<String, String> {
    let Some((group, item)) = rules_item_table(rules, item_id) else {
        return Err(format!("Item '{}' was not found.", item_id));
    };

    let name = table_string(item, "name").unwrap_or_else(|| item_id.to_string());
    let kind = rules_item_kind(group);
    let category = table_string(item, "category").unwrap_or_else(|| "-".into());
    let slot = table_string(item, "slot").unwrap_or_else(|| "-".into());
    let rarity = table_string(item, "rarity").unwrap_or_else(|| "-".into());
    let description = table_string(item, "description");

    let mut out = vec![
        format!("item: {}", name),
        format!("id: {}", item_id),
        format!("kind: {}", kind),
        format!("category: {}", category),
        format!("slot: {}", slot),
        format!("rarity: {}", rarity),
    ];
    if let Some(description) = description {
        out.push(format!("description: {}", description));
    }

    out.push(String::new());
    out.push("damage:".into());
    if item.get("damage").and_then(toml::Value::as_table).is_some() {
        match shared::rulesets::summarize_roll_path(
            rules,
            &["items", group, item_id, "damage"],
            attributes,
        ) {
            Ok(summary) => out.push(format_roll_compact(&summary)),
            Err(err) => out.push(format!("invalid: {}", err)),
        }
    } else {
        out.push("-".into());
    }

    out.push(String::new());
    out.push("attributes:".into());
    if let Some(attrs) = item.get("attributes").and_then(toml::Value::as_table) {
        let lines = table_lines_sorted(attrs);
        if lines.is_empty() {
            out.push("-".into());
        } else {
            out.extend(lines);
        }
    } else {
        out.push("-".into());
    }

    out.push(String::new());
    out.push("category rules:".into());
    let category_lines = rules_item_category_lines(rules, group, item);
    if category_lines.is_empty() {
        out.push("-".into());
    } else {
        out.extend(category_lines);
    }

    out.push(String::new());
    out.push("visual:".into());
    let visual_lines = rules_item_visual_lines(item);
    if visual_lines.is_empty() {
        out.push("-".into());
    } else {
        out.extend(visual_lines);
    }

    Ok(out.join("\n"))
}

fn run_rules_item_command(args: &[String]) -> Result<(), String> {
    let Some(item_id) = args.first() else {
        return Err(rules_command_usage().into());
    };
    let attributes = parse_rules_attributes(&args[1..])?;
    let rules = official_rules_table()?;
    println!("{}", format_rules_item(&rules, item_id, &attributes)?);
    Ok(())
}

fn rules_recipe_table<'a>(
    rules: &'a toml::Table,
    recipe_id: &str,
) -> Option<&'a toml::value::Table> {
    shared::rulesets::ruleset_table_at_path(rules, &["recipes"])
        .and_then(|recipes| recipes.get(recipe_id))
        .and_then(toml::Value::as_table)
}

fn recipe_item_quantity_lines(
    rules: &toml::Table,
    recipe: &toml::value::Table,
    key: &str,
) -> Vec<String> {
    recipe
        .get(key)
        .and_then(toml::Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(toml::Value::as_table)
                .filter_map(|entry| {
                    let item_id = entry.get("item")?.as_str()?.trim();
                    if item_id.is_empty() {
                        return None;
                    }
                    let quantity = entry
                        .get("quantity")
                        .and_then(toml::Value::as_integer)
                        .unwrap_or(1)
                        .max(1);
                    Some(format!(
                        "{} x{}",
                        rules_item_label(rules, item_id),
                        quantity
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn format_rules_recipe(rules: &toml::Table, recipe_id: &str) -> Result<String, String> {
    let Some(recipe) = rules_recipe_table(rules, recipe_id) else {
        return Err(format!("Recipe '{}' was not found.", recipe_id));
    };
    let name = table_string(recipe, "name").unwrap_or_else(|| recipe_id.to_string());
    let description = table_string(recipe, "description");
    let skill = table_string(recipe, "skill").unwrap_or_else(|| "-".into());
    let required_skill = table_number(recipe, "required_skill")
        .map(|value| format!("{:.0}", value))
        .unwrap_or_else(|| "-".into());
    let difficulty = table_number(recipe, "difficulty")
        .map(|value| format!("{:.0}", value))
        .unwrap_or_else(|| "-".into());
    let attribute = table_string(recipe, "attribute").unwrap_or_else(|| "-".into());
    let profession_hint = table_string(recipe, "profession_hint").unwrap_or_else(|| "-".into());
    let class_hint = table_string(recipe, "class_hint").unwrap_or_else(|| "-".into());
    let required_spell = recipe
        .get("requires")
        .and_then(toml::Value::as_table)
        .and_then(|requires| requires.get("spell"))
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("-");
    let station = table_string(recipe, "station").unwrap_or_else(|| "-".into());
    let time = table_number(recipe, "time")
        .map(|value| format!("{:.1}", value))
        .unwrap_or_else(|| "-".into());

    let mut out = vec![
        format!("recipe: {}", name),
        format!("id: {}", recipe_id),
        format!("skill: {}", skill),
        format!("required_skill: {}", required_skill),
        format!("difficulty: {}", difficulty),
        format!("attribute: {}", attribute),
        format!("profession_hint: {}", profession_hint),
        format!("class_hint: {}", class_hint),
        format!("requires_spell: {}", required_spell),
        format!("station: {}", station),
        format!("time: {}", time),
    ];
    if let Some(description) = description {
        out.push(format!("description: {}", description));
    }

    out.push(String::new());
    out.push("consumes:".into());
    let consumes = recipe_item_quantity_lines(rules, recipe, "consumes");
    if consumes.is_empty() {
        out.push("-".into());
    } else {
        out.extend(consumes);
    }

    out.push(String::new());
    out.push("produces:".into());
    let produces = recipe_item_quantity_lines(rules, recipe, "produces");
    if produces.is_empty() {
        out.push("-".into());
    } else {
        out.extend(produces);
    }

    Ok(out.join("\n"))
}

fn run_rules_recipe_command(args: &[String]) -> Result<(), String> {
    let Some(recipe_id) = args.first() else {
        return Err(rules_command_usage().into());
    };
    let rules = official_rules_table()?;
    println!("{}", format_rules_recipe(&rules, recipe_id)?);
    Ok(())
}

fn rules_character_loadout_lines(rules: &toml::Table, entity: &Entity, key: &str) -> Vec<String> {
    rules_character_string_array(entity, key)
        .iter()
        .map(|item_id| rules_item_label(rules, item_id))
        .collect()
}

fn format_roll_compact(summary: &shared::rulesets::RulesetRollSummary) -> String {
    let attr = summary
        .spec
        .bonus_attribute
        .as_deref()
        .map(|attr| {
            format!(
                ", {}={} gives +{} every {}",
                attr, summary.attribute_value, summary.attribute_bonus, summary.spec.bonus_every
            )
        })
        .unwrap_or_default();
    let kind = summary
        .spec
        .damage_kind
        .as_deref()
        .map(|kind| format!(", {}", kind))
        .unwrap_or_default();
    format!(
        "{} + {}{}{} => min {}, max {}, avg {:.2}",
        summary.spec.roll,
        summary.spec.bonus,
        attr,
        kind,
        summary.minimum,
        summary.maximum,
        summary.average
    )
}

fn table_number(table: &toml::value::Table, key: &str) -> Option<f32> {
    table.get(key).and_then(|value| {
        value
            .as_float()
            .map(|value| value as f32)
            .or_else(|| value.as_integer().map(|value| value as f32))
    })
}

fn rules_action_for_spell<'a>(
    rules: &'a toml::Table,
    spell_id: &str,
) -> Option<&'a toml::value::Table> {
    let actions = shared::rulesets::ruleset_table_at_path(rules, &["actions"])?;
    actions
        .get(spell_id)
        .and_then(toml::Value::as_table)
        .filter(|action| {
            action
                .get("requires")
                .and_then(toml::Value::as_table)
                .and_then(|requires| requires.get("spell"))
                .and_then(toml::Value::as_str)
                .is_some_and(|value| value.trim() == spell_id)
        })
        .or_else(|| {
            actions
                .values()
                .filter_map(toml::Value::as_table)
                .find(|action| {
                    action
                        .get("requires")
                        .and_then(toml::Value::as_table)
                        .and_then(|requires| requires.get("spell"))
                        .and_then(toml::Value::as_str)
                        .is_some_and(|value| value.trim() == spell_id)
                })
        })
}

fn rules_action_costs(action: Option<&toml::value::Table>) -> String {
    let Some(cost) = action
        .and_then(|action| action.get("cost"))
        .and_then(toml::Value::as_table)
    else {
        return "-".into();
    };
    let mut costs = cost
        .iter()
        .filter_map(|(key, value)| {
            value
                .as_integer()
                .map(|value| format!("{}={}", key, value))
                .or_else(|| value.as_float().map(|value| format!("{}={}", key, value)))
        })
        .collect::<Vec<_>>();
    costs.sort();
    if costs.is_empty() {
        "-".into()
    } else {
        costs.join(", ")
    }
}

fn rules_character_spell_lines(
    rules: &toml::Table,
    entity: &Entity,
    numeric_attrs: &shared::rulesets::RulesetAttributeMap,
) -> Vec<String> {
    rules_character_string_array(entity, "spells")
        .into_iter()
        .map(|spell_id| {
            let action = rules_action_for_spell(rules, &spell_id);
            let cooldown = action.and_then(|action| table_number(action, "cooldown"));
            let range = action.and_then(|action| table_number(action, "range"));
            let action_bits = format!(
                "cost {}, cooldown {}, range {}",
                rules_action_costs(action),
                cooldown
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".into()),
                range
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".into())
            );

            match shared::rulesets::summarize_spell_roll(rules, &spell_id, numeric_attrs) {
                Ok((kind, summary)) => format!(
                    "{}: {}, {}; {}",
                    spell_id,
                    kind.label(),
                    format_roll_compact(&summary),
                    action_bits
                ),
                Err(_) => format!("{}: {}", spell_id, action_bits),
            }
        })
        .collect()
}

fn rules_character_combat_lines(
    rules: &toml::Table,
    entity: &Entity,
    numeric_attrs: &shared::rulesets::RulesetAttributeMap,
) -> Vec<String> {
    let mut lines = Vec::new();
    if let Ok(unarmed) =
        shared::rulesets::summarize_roll_path(rules, &["combat", "unarmed_damage"], numeric_attrs)
    {
        lines.push(format!("unarmed: {}", format_roll_compact(&unarmed)));
    }

    for item_id in rules_character_string_array(entity, "start_equipped_items") {
        let Some((group, _)) = rules_item_table(rules, &item_id) else {
            continue;
        };
        if group != "weapons" {
            continue;
        }
        if let Ok(summary) =
            shared::rulesets::summarize_weapon_damage(rules, &item_id, numeric_attrs)
        {
            lines.push(format!(
                "{}: {}",
                rules_item_label(rules, &item_id),
                format_roll_compact(&summary)
            ));
        }
    }

    lines
}

fn format_rules_character(rules: &toml::Table, entity: &Entity) -> String {
    let race = entity
        .attributes
        .get_str("race")
        .map(str::to_string)
        .unwrap_or_else(|| "-".into());
    let class = entity
        .attributes
        .get_str("class")
        .map(str::to_string)
        .unwrap_or_else(|| "-".into());
    let level = entity.attributes.get_int_default("LEVEL", 1);
    let numeric_attrs = rules_character_numeric_attributes(entity);
    let role = shared::rulesets::summarize_class(rules, &class)
        .ok()
        .and_then(|summary| summary.role);

    let sections = vec![
        ("attributes", rules_character_attribute_lines(entity)),
        (
            "abilities",
            rules_character_string_array(entity, "abilities"),
        ),
        ("spells", rules_character_string_array(entity, "spells")),
        (
            "spell details",
            rules_character_spell_lines(rules, entity, &numeric_attrs),
        ),
        (
            "starting equipment",
            rules_character_loadout_lines(rules, entity, "start_equipped_items"),
        ),
        (
            "starting inventory",
            rules_character_loadout_lines(rules, entity, "start_items"),
        ),
        (
            "combat",
            rules_character_combat_lines(rules, entity, &numeric_attrs),
        ),
    ];

    let mut out = vec![
        format!("class: {}", class),
        format!("race: {}", race),
        format!("level: {}", level),
    ];
    if let Some(role) = role {
        out.push(format!("role: {}", role));
    }

    for (heading, lines) in sections {
        out.push(String::new());
        out.push(format!("{}:", heading));
        if lines.is_empty() {
            out.push("-".into());
        } else {
            out.extend(lines);
        }
    }

    out.join("\n")
}

fn run_rules_character_command(args: &[String]) -> Result<(), String> {
    let rules = official_rules_table()?;
    let mut entity = rules_character_entity(args)?;
    rusterix::server::region::apply_ruleset_character_defaults(&rules, &mut entity);
    println!("{}", format_rules_character(&rules, &entity));
    Ok(())
}

fn run_rules_class_command(args: &[String]) -> Result<(), String> {
    let Some(class_id) = args.first() else {
        return Err(rules_command_usage().into());
    };
    let rules = shared::rulesets::latest_official_ruleset()
        .parse::<toml::Table>()
        .map_err(|err| format!("Official ruleset TOML parse error: {}", err))?;
    let summary = shared::rulesets::summarize_class(&rules, class_id)?;
    let mut attributes = summary
        .attributes
        .iter()
        .map(|(key, value)| format!("{}={}", key, value))
        .collect::<Vec<_>>();
    attributes.sort();
    let unlocks = summary
        .level_unlocks
        .iter()
        .map(|(level, values)| format!("{}: {}", level, join_or_dash(values)))
        .collect::<Vec<_>>()
        .join("\n");
    let loadout = summary
        .starting_loadout
        .iter()
        .map(|(category, values)| format!("{}: {}", category, join_or_dash(values)))
        .collect::<Vec<_>>()
        .join("\n");

    println!(
        "class: {}\nrole: {}\ndescription: {}\nprimary attributes: {}\nallowed weapons: {}\nallowed armor: {}\nabilities: {}\nspells: {}\nattributes: {}\nunlocks:\n{}\nstarting loadout:\n{}",
        summary.id,
        summary.role.as_deref().unwrap_or("-"),
        summary.description.as_deref().unwrap_or("-"),
        join_or_dash(&summary.primary_attributes),
        join_or_dash(&summary.allowed_weapons),
        join_or_dash(&summary.allowed_armor),
        join_or_dash(&summary.abilities),
        join_or_dash(&summary.spells),
        join_or_dash(&attributes),
        if unlocks.is_empty() {
            "-".into()
        } else {
            unlocks
        },
        if loadout.is_empty() {
            "-".into()
        } else {
            loadout
        },
    );
    Ok(())
}

fn print_with_printer(printer: &mut impl ExternalPrinter, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    let mut chunk = text.to_string();
    if !chunk.ends_with('\n') {
        chunk.push('\n');
    }
    let _ = printer.print(chunk);
}

fn authoring_message_category_color(src: &str, category: &str) -> Option<String> {
    let normalized = category.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }

    config_table(src).and_then(|table| {
        table
            .get("colors")
            .and_then(toml::Value::as_table)
            .and_then(|colors| colors.get("message_categories"))
            .and_then(toml::Value::as_table)
            .and_then(|table| table.get(&normalized))
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn colorize_terminal_category(text: &str, category: &str, authoring: &str) -> String {
    if text.is_empty() || !std::io::stdout().is_terminal() {
        return text.to_string();
    }

    let configured = authoring_message_category_color(authoring, category);
    let fallback = match category.trim().to_ascii_lowercase().as_str() {
        "success" => Some("bright_green"),
        "warning" => Some("bright_yellow"),
        "severe" => Some("bright_red"),
        "error" => Some("bright_red"),
        "system" => Some("cyan"),
        "multiple_choice" => Some("bright_magenta"),
        _ => None,
    };

    colorize_terminal_text(text, configured.as_deref().or(fallback))
}

fn should_print_terminal_message(message: &str, category: &str) -> bool {
    if category == "warning" && message.trim() == "{system.cant_do_that_yet}" {
        return false;
    }
    true
}

fn collect_region_output(app: &mut TerminalApp, include_room: bool) -> Vec<String> {
    let Some(index) = app.current_region_index() else {
        return Vec::new();
    };
    let region_id = app.project.regions[index].map.id;
    let Some(map) = app.current_region().map(|region| region.map.clone()) else {
        return Vec::new();
    };
    let outputs = app.session.collect(
        &map,
        &app.project.authoring,
        app.server.get_messages(&region_id),
        app.server.get_says(&region_id),
        app.current_time_hour_and_label().map(|(hour, _)| hour),
        app.current_time_hour_and_label().map(|(_, label)| label),
        authoring_auto_attack_mode(&app.project.authoring) == AutoAttackMode::OnAttack,
    );
    app.auto_attack_target = app.session.auto_attack_target();

    let mut rendered = Vec::new();
    let mut saw_death = false;
    for entry in outputs {
        match entry {
            TextSessionOutput::RenderRoom => {
                if include_room {
                    rendered.push(app.render_room_text());
                }
            }
            TextSessionOutput::Plain(text) => rendered.push(text),
            TextSessionOutput::Message { text, category } => {
                if should_print_terminal_message(&text, &category) {
                    rendered.push(colorize_terminal_category(
                        &text,
                        &category,
                        &app.project.authoring,
                    ));
                    if text.trim() == "You died. Try again!" {
                        saw_death = true;
                    }
                }
            }
        }
    }
    if saw_death {
        app.auto_attack_target = None;
        app.session.clear_auto_attack_target();
        app.discard_pending_messages();
    }
    rendered
}

fn collect_roguelike_screen_output(app: &mut TerminalApp) -> Vec<String> {
    let Some(index) = app.current_region_index() else {
        return Vec::new();
    };
    let region_id = app.project.regions[index].map.id;
    let mut output = Vec::new();

    for (_sender_entity, _sender_item, _receiver, message, category) in
        app.server.get_messages(&region_id)
    {
        if should_print_terminal_message(&message, &category) {
            output.push(message);
        }
    }

    for (_sender_entity, _sender_item, message, _category) in app.server.get_says(&region_id) {
        if !message.trim().is_empty() {
            output.push(message);
        }
    }

    if !output.is_empty() {
        app.screen_messages.extend(output.iter().cloned());
        let keep_from = app.screen_messages.len().saturating_sub(8);
        if keep_from > 0 {
            app.screen_messages.drain(0..keep_from);
        }
    }

    output
}

fn roguelike_screen_message(app: &TerminalApp, immediate: Option<&str>) -> Option<String> {
    if let Some(message) = immediate
        && !message.trim().is_empty()
    {
        return Some(message.trim().to_string());
    }
    (!app.screen_messages.is_empty()).then(|| app.screen_messages.join("\n"))
}

fn trigger_text_intent(app: &mut TerminalApp, intent: &str, query: &str) -> Vec<String> {
    let Some(region) = app.current_region() else {
        return vec![format!("Current region '{}' not found.", app.current_map)];
    };
    let Some((_player, sector)) = sg::current_player_and_sector(&region.map) else {
        return vec!["No local player found.".into()];
    };

    let target = match sg::resolve_text_target(&region.map, sector, query) {
        Ok(target) => target,
        Err(err) => return vec![err],
    };

    match target {
        sg::TextTarget::Entity { id, distance } => {
            app.server.local_player_action(EntityAction::EntityClicked(
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
            app.server.local_player_action(EntityAction::ItemClicked(
                id,
                if intent.trim().eq_ignore_ascii_case("look") {
                    0.0
                } else {
                    distance
                },
                Some(intent.trim().to_string()),
                None,
            ));
        }
    }

    app.tick();
    let mut output = collect_region_output(app, false);
    if output.is_empty() && !intent.trim().eq_ignore_ascii_case("look") {
        app.tick();
        output = collect_region_output(app, false);
    }
    output
}

fn current_player_supported_intents(app: &TerminalApp) -> BTreeSet<String> {
    app.current_region()
        .map(|region| sg::current_player_supported_intents(&app.project, &region.map))
        .unwrap_or_default()
}

fn tick_duration(project: &Project) -> Duration {
    let tick_ms = config_table(&project.config)
        .and_then(|table| {
            table
                .get("game")
                .and_then(toml::Value::as_table)
                .and_then(|table| table.get("game_tick_ms"))
                .and_then(|value| value.as_integer())
        })
        .unwrap_or(250)
        .max(1) as u64;
    Duration::from_millis(tick_ms)
}

fn handle_command(app: &mut TerminalApp, input: &str) -> (bool, Vec<String>) {
    let mut output = Vec::new();
    let lower = input.to_ascii_lowercase();
    let direction = match lower.as_str() {
        "n" => "north",
        "e" => "east",
        "s" => "south",
        "w" => "west",
        _ => lower.as_str(),
    };

    match input {
        "" | "look" | "l" => {
            output.push(app.render_room_text());
        }
        _ if matches!(direction, "north" | "south" | "east" | "west") => {
            let mut moved = false;
            if let Some(region) = app.current_region() {
                if let Some((_, sector)) = sg::current_player_and_sector(&region.map) {
                    let exits = sg::resolve_text_exits(
                        &region.map,
                        sector,
                        sg::authoring_connection_probe_distance(&app.project.authoring),
                    );
                    if let Some(exit) = exits.into_iter().find(|exit| exit.direction == direction) {
                        moved = app.move_to_exit(&exit);
                    }
                }
            }

            if moved {
                if let Some(map) = app.current_region().map(|region| region.map.clone()) {
                    for entry in app.session.after_movement(&map, &app.project.authoring) {
                        match entry {
                            TextSessionOutput::RenderRoom => output.push(app.render_room_text()),
                            TextSessionOutput::Plain(text) => output.push(text),
                            TextSessionOutput::Message { text, category } => {
                                output.push(colorize_terminal_category(
                                    &text,
                                    &category,
                                    &app.project.authoring,
                                ))
                            }
                        }
                    }
                }
                output.extend(collect_region_output(app, false));
            } else {
                output.push("You cannot go that way.".into());
            }
        }
        "wait" | "." => {
            app.process_auto_attack();
            app.tick();
            output.extend(collect_region_output(app, true));
        }
        "forward" | "f" => {
            app.auto_attack_target = None;
            app.server.local_player_action(EntityAction::Forward);
            app.tick();
            app.server.local_player_action(EntityAction::Off);
            app.tick();
            output.extend(collect_region_output(app, true));
        }
        "backward" | "b" => {
            app.auto_attack_target = None;
            app.server.local_player_action(EntityAction::Backward);
            app.tick();
            app.server.local_player_action(EntityAction::Off);
            app.tick();
            output.extend(collect_region_output(app, true));
        }
        "left" => {
            app.auto_attack_target = None;
            app.server.local_player_action(EntityAction::Left);
            app.tick();
            app.server.local_player_action(EntityAction::Off);
            app.tick();
            output.extend(collect_region_output(app, true));
        }
        "right" => {
            app.auto_attack_target = None;
            app.server.local_player_action(EntityAction::Right);
            app.tick();
            app.server.local_player_action(EntityAction::Off);
            app.tick();
            output.extend(collect_region_output(app, true));
        }
        "inventory" | "inv" => {
            if let Some(region) = app.current_region() {
                if let Some(text) = sg::render_player_inventory(&region.map) {
                    output.push(text);
                } else {
                    output.push("No local player found.".into());
                }
            } else {
                output.push(format!("Current region '{}' not found.", app.current_map));
            }
        }
        "help" => output.push(
            [
                "Commands:",
                "  look | l           Show the current room",
                "  inventory | inv    Show your inventory",
                "  north | east | south | west",
                "  n | e | s | w",
                "                     Move through a text exit",
                "  go <name>          Move by exit direction or title",
                "  <intent> <target>  Trigger a configured player intent on a visible target",
                "  intent <name>      Set the current player intent",
                "  help               Show this help",
                "  quit | exit        Leave the client",
            ]
            .join("\n"),
        ),
        "quit" | "exit" => return (false, output),
        _ if lower.starts_with("go ") => {
            let target = input["go ".len()..].trim().to_ascii_lowercase();
            if target.is_empty() {
                output.push("Usage: go <direction or exit name>".into());
            } else {
                let mut moved = false;
                if let Some(region) = app.current_region() {
                    if let Some((_, sector)) = sg::current_player_and_sector(&region.map) {
                        let exits = sg::resolve_text_exits(
                            &region.map,
                            sector,
                            sg::authoring_connection_probe_distance(&app.project.authoring),
                        );
                        if let Some(exit) = exits.into_iter().find(|exit| {
                            exit.direction == target
                                || exit.title.to_ascii_lowercase() == target
                                || exit.target_title.to_ascii_lowercase() == target
                        }) {
                            moved = app.move_to_exit(&exit);
                        }
                    }
                }

                if moved {
                    if let Some(map) = app.current_region().map(|region| region.map.clone()) {
                        for entry in app.session.after_movement(&map, &app.project.authoring) {
                            match entry {
                                TextSessionOutput::RenderRoom => {
                                    output.push(app.render_room_text())
                                }
                                TextSessionOutput::Plain(text) => output.push(text),
                                TextSessionOutput::Message { text, category } => {
                                    output.push(colorize_terminal_category(
                                        &text,
                                        &category,
                                        &app.project.authoring,
                                    ))
                                }
                            }
                        }
                    }
                    output.extend(collect_region_output(app, false));
                } else {
                    output.push("No matching exit.".into());
                }
            }
        }
        _ if input.starts_with("intent ") => {
            let payload = input["intent ".len()..].trim();
            if payload.is_empty() {
                output.push("Usage: intent <name> [target]".into());
            } else {
                let mut parts = payload.splitn(2, char::is_whitespace);
                let intent = parts.next().unwrap_or("").trim();
                let target = parts.next().map(str::trim).unwrap_or("");
                if !intent.is_empty() && !target.is_empty() {
                    output.extend(trigger_text_intent(app, intent, target));
                } else if !intent.is_empty() {
                    app.server
                        .local_player_action(EntityAction::Intent(intent.to_string()));
                    app.tick();
                    output.extend(collect_region_output(app, true));
                } else {
                    output.push("Usage: intent <name> [target]".into());
                }
            }
        }
        _ => {
            let mut parts = input.splitn(2, char::is_whitespace);
            let verb =
                resolve_intent_alias(&app.project.authoring, parts.next().unwrap_or("").trim());
            let target = parts.next().map(str::trim).unwrap_or("");
            let supported_intents = current_player_supported_intents(app);
            if !verb.is_empty() && !target.is_empty() && supported_intents.contains(&verb) {
                output.extend(trigger_text_intent(app, &verb, target));
            } else {
                output.push("Unknown command. Type 'help' for available commands.".into());
            }
        }
    }

    (true, output)
}

fn run_blocking_cli(mut app: TerminalApp) {
    let mut editor = match DefaultEditor::new() {
        Ok(editor) => editor,
        Err(err) => {
            eprintln!("Failed to initialize terminal editor: {}", err);
            std::process::exit(1);
        }
    };

    loop {
        let input = match editor.readline("> ") {
            Ok(input) => {
                let trimmed = input.trim().to_string();
                if !trimmed.is_empty() {
                    let _ = editor.add_history_entry(trimmed.as_str());
                }
                trimmed
            }
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                eprintln!("Input error: {}", err);
                break;
            }
        };

        let (keep_running, output) = handle_command(&mut app, &input);
        for block in output {
            if !block.trim().is_empty() {
                println!();
                println!("{}", block);
            }
        }
        if !keep_running {
            clear_terminal_prompt_line();
            break;
        }
    }

    app.server.stop();
}

fn run_realtime_cli(mut app: TerminalApp) {
    let mut editor = match DefaultEditor::new() {
        Ok(editor) => editor,
        Err(err) => {
            eprintln!("Failed to initialize terminal editor: {}", err);
            std::process::exit(1);
        }
    };
    let mut printer = match editor.create_external_printer() {
        Ok(printer) => printer,
        Err(_) => {
            run_blocking_cli(app);
            return;
        }
    };

    let (tx, rx) = mpsc::channel::<InputEvent>();
    let (ack_tx, ack_rx) = mpsc::channel::<bool>();
    thread::spawn(move || {
        loop {
            let input = match editor.readline("> ") {
                Ok(input) => {
                    let trimmed = input.trim().to_string();
                    if !trimmed.is_empty() {
                        let _ = editor.add_history_entry(trimmed.as_str());
                    }
                    InputEvent::Line(trimmed)
                }
                Err(ReadlineError::Interrupted) => continue,
                Err(ReadlineError::Eof) => InputEvent::Eof,
                Err(err) => InputEvent::Error(err.to_string()),
            };

            let end = !matches!(input, InputEvent::Line(_));
            if tx.send(input).is_err() {
                break;
            }
            if end {
                break;
            }
            match ack_rx.recv() {
                Ok(true) => {}
                _ => break,
            }
        }
    });

    let tick_dt = tick_duration(&app.project);
    let mut next_tick = Instant::now() + tick_dt;
    let mut keep_running = true;

    while keep_running {
        match rx.recv_timeout(Duration::from_millis(25)) {
            Ok(InputEvent::Line(input)) => {
                let (still_running, output) = handle_command(&mut app, &input);
                keep_running = still_running;
                for block in output {
                    print_with_printer(&mut printer, &block);
                }
                next_tick = Instant::now() + tick_dt;
                let _ = ack_tx.send(keep_running);
            }
            Ok(InputEvent::Eof) => break,
            Ok(InputEvent::Error(err)) => {
                eprintln!("Input error: {}", err);
                break;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }

        let now = Instant::now();
        while keep_running && now >= next_tick {
            app.process_auto_attack();
            app.tick();
            for block in collect_region_output(&mut app, false) {
                print_with_printer(&mut printer, &block);
            }
            next_tick += tick_dt;
        }
    }

    clear_terminal_prompt_line();
    app.server.stop();
}

fn clear_terminal_prompt_line() {
    if std::io::stdout().is_terminal() {
        print!("\r\x1b[2K");
    }
    println!();
}

fn run_terminal_app(app: TerminalApp, mode: TerminalPlayMode) {
    match mode {
        TerminalPlayMode::Text => run_text_terminal_app(app),
        TerminalPlayMode::Roguelike => run_roguelike_terminal_app(app),
    }
}

fn run_roguelike_terminal_app(mut app: TerminalApp) {
    app.server
        .local_player_action(EntityAction::SetPlayerCamera(PlayerCamera::D2Grid));
    app.tick();
    if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        run_roguelike_raw_terminal_app(app);
    } else {
        run_roguelike_line_terminal_app(app);
    }
}

fn run_roguelike_line_terminal_app(mut app: TerminalApp) {
    let mut editor = match DefaultEditor::new() {
        Ok(editor) => editor,
        Err(err) => {
            eprintln!("Failed to initialize terminal editor: {}", err);
            std::process::exit(1);
        }
    };

    let startup_message = diagnostic_message(app.drain_server_diagnostics());
    println!(
        "{}",
        render_roguelike_view_with_message(&app, startup_message.as_deref())
    );
    loop {
        let input = match editor.readline("rogue> ") {
            Ok(input) => {
                let trimmed = input.trim().to_string();
                if !trimmed.is_empty() {
                    let _ = editor.add_history_entry(trimmed.as_str());
                }
                trimmed
            }
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                eprintln!("Input error: {}", err);
                break;
            }
        };

        let (keep_running, message) = handle_roguelike_input(&mut app, &input);
        let mut message = message.unwrap_or_default();
        append_roguelike_diagnostics(&mut app, &mut message);
        if !keep_running {
            clear_terminal_prompt_line();
            break;
        }
        println!();
        println!(
            "{}",
            render_roguelike_view_with_message(
                &app,
                (!message.trim().is_empty()).then_some(message.as_str())
            )
        );
    }

    app.server.stop();
}

fn run_roguelike_raw_terminal_app(mut app: TerminalApp) {
    if let Err(err) = terminal::enable_raw_mode() {
        eprintln!("Failed to enable raw terminal mode: {}", err);
        run_roguelike_line_terminal_app(app);
        return;
    }

    let mut stdout = io::stdout();
    if let Err(err) = execute!(stdout, EnterAlternateScreen, cursor::Hide) {
        let _ = terminal::disable_raw_mode();
        eprintln!("Failed to enter terminal screen: {}", err);
        run_roguelike_line_terminal_app(app);
        return;
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match RatatuiTerminal::new(backend) {
        Ok(terminal) => terminal,
        Err(err) => {
            let _ = terminal::disable_raw_mode();
            eprintln!("Failed to initialize terminal UI: {}", err);
            app.server.stop();
            return;
        }
    };

    let tick_duration = tick_duration(&app.project);
    let mut last_view = render_roguelike_view(&app);
    let startup_message = diagnostic_message(app.drain_server_diagnostics());
    if let Err(err) = draw_roguelike_tui(&mut terminal, &app, startup_message.as_deref()) {
        restore_roguelike_terminal(&mut terminal);
        eprintln!("Failed to draw terminal view: {}", err);
        app.server.stop();
        return;
    }

    loop {
        match event::poll(tick_duration) {
            Ok(true) => match event::read() {
                Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    let Some(input) = key_event_to_input(key.code) else {
                        continue;
                    };
                    let (keep_running, message) = handle_roguelike_input(&mut app, &input);
                    if !keep_running {
                        break;
                    }
                    let mut message = message.unwrap_or_default();
                    append_roguelike_diagnostics(&mut app, &mut message);
                    let message = (!message.trim().is_empty()).then_some(message);
                    if let Err(err) = draw_roguelike_tui(&mut terminal, &app, message.as_deref()) {
                        restore_roguelike_terminal(&mut terminal);
                        eprintln!("Failed to draw terminal view: {}", err);
                        app.server.stop();
                        return;
                    }
                    last_view = render_roguelike_view(&app);
                }
                Ok(_) => {}
                Err(err) => {
                    restore_roguelike_terminal(&mut terminal);
                    eprintln!("Input error: {}", err);
                    app.server.stop();
                    return;
                }
            },
            Ok(false) => {
                app.tick();
                let mut output = collect_roguelike_screen_output(&mut app);
                output.extend(app.drain_server_diagnostics());
                let view = render_roguelike_view(&app);
                if !output.is_empty() || view != last_view {
                    let message = (!output.is_empty()).then(|| output.join("\n"));
                    if let Err(err) = draw_roguelike_tui(&mut terminal, &app, message.as_deref()) {
                        restore_roguelike_terminal(&mut terminal);
                        eprintln!("Failed to draw terminal view: {}", err);
                        app.server.stop();
                        return;
                    }
                    last_view = view;
                }
            }
            Err(err) => {
                restore_roguelike_terminal(&mut terminal);
                eprintln!("Input poll error: {}", err);
                app.server.stop();
                return;
            }
        }
    }

    restore_roguelike_terminal(&mut terminal);
    app.server.stop();
}

fn restore_roguelike_terminal(terminal: &mut RatatuiTerminal<CrosstermBackend<io::Stdout>>) {
    let _ = terminal::disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen, cursor::Show);
    let _ = terminal.show_cursor();
}

fn draw_roguelike_tui(
    terminal: &mut RatatuiTerminal<CrosstermBackend<io::Stdout>>,
    app: &TerminalApp,
    message: Option<&str>,
) -> io::Result<()> {
    terminal
        .draw(|frame| render_roguelike_tui_frame(frame, app, message))
        .map(|_| ())
}

fn render_roguelike_tui_frame(frame: &mut Frame, app: &TerminalApp, message: Option<&str>) {
    let area = frame.area();
    let Some(region) = app.current_region() else {
        frame.render_widget(
            Paragraph::new(format!("Current region '{}' not found.", app.current_map))
                .block(Block::default().borders(Borders::ALL).title("Eldiron")),
            area,
        );
        return;
    };

    let screen_frame = TerminalScreenFrame {
        header: String::new(),
        hint: String::new(),
        message: roguelike_screen_message(app, message),
    };

    let Some(layout) = shared::terminal_screen::project_terminal_screen_layout(&app.project) else {
        frame.render_widget(
            Paragraph::new(shared::terminal_screen::render_roguelike_view(
                region,
                &screen_frame,
            ))
            .block(Block::default().borders(Borders::ALL).title("Eldiron"))
            .wrap(Wrap { trim: false }),
            area,
        );
        return;
    };

    for widget in &layout.widgets {
        let rect = terminal_rect_to_tui(&widget.rect, &layout, area);
        if rect.width == 0 || rect.height == 0 {
            continue;
        }
        render_roguelike_tui_widget(frame, rect, widget, region, &screen_frame);
    }
}

fn terminal_rect_to_tui(
    rect: &shared::terminal_screen::TerminalRect,
    layout: &shared::terminal_screen::TerminalScreenLayout,
    area: Rect,
) -> Rect {
    let layout_width = layout.width.max(1) as u32;
    let layout_height = layout.height.max(1) as u32;
    let area_width = area.width as u32;
    let area_height = area.height as u32;

    let x0 = area.x as u32 + rect.x as u32 * area_width / layout_width;
    let y0 = area.y as u32 + rect.y as u32 * area_height / layout_height;
    let x1 = area.x as u32 + (rect.x + rect.width) as u32 * area_width / layout_width;
    let y1 = area.y as u32 + (rect.y + rect.height) as u32 * area_height / layout_height;

    Rect {
        x: x0.min(u16::MAX as u32) as u16,
        y: y0.min(u16::MAX as u32) as u16,
        width: x1.saturating_sub(x0).min(u16::MAX as u32) as u16,
        height: y1.saturating_sub(y0).min(u16::MAX as u32) as u16,
    }
}

fn render_roguelike_tui_widget(
    frame: &mut Frame,
    rect: Rect,
    widget: &shared::terminal_screen::TerminalWidget,
    region: &shared::region::Region,
    screen_frame: &TerminalScreenFrame,
) {
    let lines = shared::terminal_screen::terminal_widget_lines(widget, region, screen_frame);
    let text = lines.join("\n");
    match widget.role.as_str() {
        "game" => {
            frame.render_widget(
                Paragraph::new(text).style(Style::default().fg(Color::LightGreen)),
                rect,
            );
        }
        "messages" => {
            frame.render_widget(
                Paragraph::new(text)
                    .block(terminal_block(&widget.name).style(Style::default().fg(Color::Cyan)))
                    .style(Style::default().fg(Color::White))
                    .wrap(Wrap { trim: false }),
                rect,
            );
        }
        "text" => {
            frame.render_widget(
                Paragraph::new(text)
                    .style(Style::default().fg(Color::Gray))
                    .wrap(Wrap { trim: false }),
                rect,
            );
        }
        "stat" => {
            frame.render_widget(
                Paragraph::new(text)
                    .style(Style::default().fg(Color::LightBlue))
                    .wrap(Wrap { trim: false }),
                rect,
            );
        }
        "avatar" => {
            frame.render_widget(
                Paragraph::new(text)
                    .block(terminal_block(&widget.name))
                    .style(Style::default().fg(Color::LightMagenta))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: false }),
                rect,
            );
        }
        "button" => {
            frame.render_widget(
                Paragraph::new(text)
                    .block(terminal_block(""))
                    .style(Style::default().fg(Color::Yellow))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true }),
                rect,
            );
        }
        "deco" => {
            frame.render_widget(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .style(Style::default().bg(Color::Black)),
                rect,
            );
        }
        _ => {}
    }
}

fn terminal_block(title: &str) -> Block<'_> {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    if title.trim().is_empty() {
        block
    } else {
        block.title(Line::from(Span::styled(
            title.to_string(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )))
    }
}

fn append_roguelike_diagnostics(app: &mut TerminalApp, output: &mut String) {
    let diagnostics = app.drain_server_diagnostics();
    if diagnostics.is_empty() {
        return;
    }
    if !output.trim().is_empty() {
        output.push_str("\n\n");
    }
    output.push_str(&diagnostics.join("\n"));
}

fn diagnostic_message(diagnostics: Vec<String>) -> Option<String> {
    (!diagnostics.is_empty()).then(|| diagnostics.join("\n"))
}

fn is_terminal_diagnostic_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("[error]")
        || lower.contains("[warning]")
        || lower.contains("[warn]")
        || lower.contains("compile error")
        || lower.contains("runtime error")
        || lower.contains("compiling character")
        || lower.contains("compiling item")
        || lower.contains("compiling region")
        || lower.contains("event error")
}

fn key_event_to_input(code: KeyCode) -> Option<String> {
    match code {
        KeyCode::Char(ch) => Some(ch.to_ascii_lowercase().to_string()),
        KeyCode::Up => Some("up".to_string()),
        KeyCode::Down => Some("down".to_string()),
        KeyCode::Left => Some("left".to_string()),
        KeyCode::Right => Some("right".to_string()),
        KeyCode::Esc => Some("quit".to_string()),
        KeyCode::Enter => Some("look".to_string()),
        _ => None,
    }
}

fn handle_roguelike_input(app: &mut TerminalApp, input: &str) -> (bool, Option<String>) {
    let command = input.trim().to_ascii_lowercase();
    match command.as_str() {
        "" | "look" | "l" => (true, None),
        "quit" | "exit" | "q" => (false, None),
        "help" | "?" => (
            true,
            Some(
                [
                    "Roguelike mode commands:",
                    "  movement keys come from the active player's [input] table",
                    "  look - redraw",
                    "  wait or . - advance one tick",
                    "  quit - leave the client",
                ]
                .join("\n"),
            ),
        ),
        "wait" | "." => {
            app.tick();
            let output = collect_roguelike_screen_output(app);
            (true, (!output.is_empty()).then(|| output.join("\n")))
        }
        _ => {
            if let Some(action) = roguelike_input_action(app, &command) {
                let message = apply_roguelike_player_action(app, action);
                (true, (!message.trim().is_empty()).then_some(message))
            } else if let Some(intent) = roguelike_input_intent(app, &command) {
                let message = set_roguelike_player_intent(app, &intent);
                (true, (!message.trim().is_empty()).then_some(message))
            } else {
                (
                    true,
                    Some(format!(
                        "Unknown roguelike command '{}'. Type 'help' for commands.",
                        input
                    )),
                )
            }
        }
    }
}

fn roguelike_input_action(app: &TerminalApp, key: &str) -> Option<EntityAction> {
    let input = active_player_input_map(app);
    let command = input.get(key)?;
    roguelike_control_action(command)
}

fn roguelike_input_intent(app: &TerminalApp, key: &str) -> Option<String> {
    let input = active_player_input_map(app);
    let command = input.get(key)?;
    roguelike_intent_command(command)
}

fn roguelike_control_action(command: &str) -> Option<EntityAction> {
    let command = unwrap_roguelike_command(command);
    match command {
        "control.forward" | "action(forward)" | "control(forward)" | "forward" => {
            Some(EntityAction::Forward)
        }
        "control.backward" | "action(backward)" | "control(backward)" | "backward" => {
            Some(EntityAction::Backward)
        }
        "control.left" | "action(left)" | "control(left)" | "left" => Some(EntityAction::Left),
        "control.right" | "action(right)" | "control(right)" | "right" => Some(EntityAction::Right),
        _ => None,
    }
}

fn roguelike_intent_command(command: &str) -> Option<String> {
    let command = unwrap_roguelike_command(command);
    if let Some(intent) = command
        .strip_prefix("intent(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return Some(intent.trim().to_ascii_lowercase());
    }
    command
        .strip_prefix("intent.")
        .map(|intent| intent.trim().to_ascii_lowercase())
}

fn unwrap_roguelike_command(command: &str) -> &str {
    command
        .trim()
        .strip_prefix("command(")
        .and_then(|value| value.strip_suffix(')'))
        .unwrap_or(command)
        .trim()
}

fn set_roguelike_player_intent(app: &mut TerminalApp, intent: &str) -> String {
    let intent = intent.trim();
    app.server
        .local_player_action(EntityAction::Intent(intent.to_string()));
    app.tick();
    let output = collect_roguelike_screen_output(app);
    if !output.is_empty() {
        return output.join("\n");
    }
    if intent.is_empty() {
        "Intent cleared.".to_string()
    } else {
        format!("Intent set: {}. Press a direction to target it.", intent)
    }
}

fn active_player_input_map(app: &TerminalApp) -> BTreeMap<String, String> {
    let mut bindings = BTreeMap::new();
    let Some(region) = app.current_region() else {
        return bindings;
    };
    let Some(player) = region.map.entities.iter().find(|entity| entity.is_player()) else {
        return bindings;
    };
    let Some(class_name) = player.get_attr_string("class_name") else {
        return bindings;
    };
    let Some((_, data)) = app.assets.entities.get(&class_name) else {
        return bindings;
    };
    let Ok(table) = data.parse::<Table>() else {
        return bindings;
    };
    let Some(input) = table.get("input").and_then(toml::Value::as_table) else {
        return bindings;
    };
    for (key, value) in input {
        if let Some(command) = value.as_str() {
            bindings.insert(key.trim().to_ascii_lowercase(), command.trim().to_string());
        }
    }
    bindings
}

fn active_player_intent(app: &TerminalApp) -> Option<String> {
    let region = app.current_region()?;
    let player = region
        .map
        .entities
        .iter()
        .find(|entity| entity.is_player())?;
    let intent = player.get_attr_string("intent")?;
    let intent = intent.trim();
    (!intent.is_empty()).then(|| intent.to_string())
}

fn apply_roguelike_player_action(app: &mut TerminalApp, action: EntityAction) -> String {
    let Some((x, y)) = roguelike_player_cell(app) else {
        return "No local player found.".to_string();
    };
    let had_intent = active_player_intent(app).is_some();
    if !had_intent && let Some((dx, dy)) = roguelike_action_direction(&action) {
        let target_x = x + dx;
        let target_y = y + dy;
        let Some(terrain) = roguelike_terrain(app) else {
            return "No source terrain metadata found for this region.".to_string();
        };
        if is_roguelike_blocked(&terrain, target_x, target_y) {
            return "Blocked.".to_string();
        }
    }

    app.auto_attack_target = None;
    app.server.local_player_action(action);
    app.tick();
    if !had_intent {
        app.server.local_player_action(EntityAction::Off);
    }
    for _ in 0..20 {
        thread::sleep(Duration::from_millis(16));
        app.tick();
        if had_intent {
            if active_player_intent(app).is_none() {
                break;
            }
        } else if active_player_cell_has_settled(app, x, y) {
            break;
        }
    }
    let mut output = collect_roguelike_screen_output(app);
    if output.is_empty() && had_intent {
        for _ in 0..3 {
            thread::sleep(Duration::from_millis(16));
            app.tick();
            output = collect_roguelike_screen_output(app);
            if !output.is_empty() {
                break;
            }
        }
    }
    output.join("\n")
}

fn active_player_cell_has_settled(app: &TerminalApp, previous_x: i32, previous_y: i32) -> bool {
    roguelike_player_cell(app)
        .map(|(x, y)| x != previous_x || y != previous_y)
        .unwrap_or(true)
}

fn roguelike_action_direction(action: &EntityAction) -> Option<(i32, i32)> {
    match action {
        EntityAction::Forward => Some((0, -1)),
        EntityAction::Backward => Some((0, 1)),
        EntityAction::Left => Some((-1, 0)),
        EntityAction::Right => Some((1, 0)),
        _ => None,
    }
}

fn render_roguelike_view(app: &TerminalApp) -> String {
    render_roguelike_view_with_message(app, None)
}

fn render_roguelike_view_with_message(app: &TerminalApp, message: Option<&str>) -> String {
    let Some(region) = app.current_region() else {
        return format!("Current region '{}' not found.", app.current_map);
    };
    shared::terminal_screen::render_roguelike_screen(
        &app.project,
        region,
        &TerminalScreenFrame {
            header: String::new(),
            hint: String::new(),
            message: roguelike_screen_message(app, message),
        },
    )
}

fn roguelike_terrain(app: &TerminalApp) -> Option<Vec<Vec<char>>> {
    let region = app.current_region()?;
    shared::terminal_screen::source_terrain(&region.map)
}

fn roguelike_player_cell(app: &TerminalApp) -> Option<(i32, i32)> {
    let region = app.current_region()?;
    let player = region
        .map
        .entities
        .iter()
        .find(|entity| entity.is_player())?;
    Some(world_to_cell(player.position.x, player.position.z))
}

fn world_to_cell(x: f32, z: f32) -> (i32, i32) {
    shared::terminal_screen::world_to_cell(x, z)
}

fn is_roguelike_blocked(terrain: &[Vec<char>], x: i32, y: i32) -> bool {
    shared::terminal_screen::is_roguelike_blocked(terrain, x, y)
}

fn run_text_terminal_app(mut app: TerminalApp) {
    let welcome = authoring_startup_welcome(&app.project.authoring);
    let printed_startup = welcome.is_some();

    if let Some(welcome) = welcome {
        println!("{}", welcome);
    }

    match authoring_startup_display(&app.project.authoring) {
        StartupDisplay::None => {}
        StartupDisplay::Description => {
            let startup_messages = app.print_pending_messages();
            if startup_messages == 0
                && let Some(description) = app.render_current_sector_description()
            {
                if printed_startup {
                    println!();
                }
                println!("{}", description);
            }
        }
        StartupDisplay::Room => {
            if printed_startup {
                println!();
            }
            let _ = app.discard_pending_messages();
            if let Some(map) = app.current_region().map(|region| region.map.clone()) {
                for entry in app.session.startup(
                    &map,
                    &app.project.authoring,
                    app.current_time_hour_and_label().map(|(hour, _)| hour),
                ) {
                    match entry {
                        TextSessionOutput::RenderRoom => println!("{}", app.render_room_text()),
                        TextSessionOutput::Plain(text) => println!("{}", text),
                        TextSessionOutput::Message { text, category } => {
                            println!(
                                "{}",
                                colorize_terminal_category(
                                    &text,
                                    &category,
                                    &app.project.authoring
                                )
                            )
                        }
                    }
                }
            } else {
                println!("{}", app.render_room_text());
            }
        }
    }

    if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        run_realtime_cli(app);
    } else {
        run_blocking_cli(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_rules_summary_for_catalog() {
        let rules = official_rules_table().unwrap();
        let catalog = shared::rulesets::ruleset_catalog(&rules);
        let summary = format_rules_catalog_summary("test", &catalog);

        assert!(summary.contains("source: test"));
        assert!(summary.contains("id: eldiron.official"));
        assert!(summary.contains("classes: 4"));
        assert!(summary.contains("professions: 7"));
        assert!(summary.contains("skills: 7"));
        assert!(summary.contains("resources: 3"));
        assert!(summary.contains("recipes: 3"));
        assert!(summary.contains("spells: 3"));
    }

    #[test]
    fn formats_rules_validation_report_status() {
        let rules = official_rules_table().unwrap();
        let report = shared::rulesets::validate_ruleset(&rules);
        let output = format_rules_validation_report("test", &report);

        assert!(output.contains("ruleset check: ok (0 errors, 0 warnings)"));
        assert!(output.contains("source: test"));
    }

    #[test]
    fn formats_resolved_rules_character() {
        let rules = official_rules_table().unwrap();
        let mut entity =
            rules_character_entity(&["Cleric".into(), "race=Human".into(), "level=2".into()])
                .unwrap();
        rusterix::server::region::apply_ruleset_character_defaults(&rules, &mut entity);
        let output = format_rules_character(&rules, &entity);

        assert!(output.contains("class: Cleric"));
        assert!(output.contains("race: Human"));
        assert!(output.contains("level: 2"));
        assert!(output.contains("MAX_HP: 19"));
        assert!(output.contains("holy_light"));
        assert!(output.contains("cost MP=4"));
        assert!(output.contains("main_hand: Novice Mace"));

        let mut ranger =
            rules_character_entity(&["Ranger".into(), "race=Human".into(), "level=1".into()])
                .unwrap();
        rusterix::server::region::apply_ruleset_character_defaults(&rules, &mut ranger);
        let output = format_rules_character(&rules, &ranger);
        assert!(output.contains("class: Ranger"));
        assert!(output.contains("main_hand: Hunting Bow"));
        assert!(output.contains("Wooden Arrows"));

        let mut citizen = rules_character_entity(&[
            "Citizen".into(),
            "race=Human".into(),
            "level=1".into(),
            "profession=Blacksmith".into(),
        ])
        .unwrap();
        rusterix::server::region::apply_ruleset_character_defaults(&rules, &mut citizen);
        let output = format_rules_character(&rules, &citizen);
        assert!(output.contains("class: Citizen"));
        assert!(output.contains("role: civilian"));
        assert!(output.contains("profession: Blacksmith"));
        assert!(output.contains("Linen Shirt"));
    }

    #[test]
    fn formats_rules_item_details() {
        let rules = official_rules_table().unwrap();
        let attributes = shared::rulesets::RulesetAttributeMap::from([("STR".into(), 12.0)]);
        let sword = format_rules_item(&rules, "training_sword", &attributes).unwrap();

        assert!(sword.contains("item: Training Sword"));
        assert!(sword.contains("kind: weapon"));
        assert!(sword.contains("blunt wooden practice sword"));
        assert!(sword.contains("1d6 + 1"));
        assert!(sword.contains("STR=12 gives +3 every 4"));
        assert!(sword.contains("visual_template: sword_diagonal"));
        assert!(sword.contains("blade_color_index: 10"));

        let bow_attrs = shared::rulesets::RulesetAttributeMap::from([("DEX".into(), 12.0)]);
        let bow = format_rules_item(&rules, "hunting_bow", &bow_attrs).unwrap();
        assert!(bow.contains("item: Hunting Bow"));
        assert!(bow.contains("category: bow"));
        assert!(bow.contains("simple wooden bow"));
        assert!(bow.contains("DEX=12 gives +3 every 4"));
        assert!(bow.contains("range: 6"));
        assert!(bow.contains("visual_template: bow_diagonal"));

        let shirt = format_rules_item(
            &rules,
            "linen_shirt",
            &shared::rulesets::RulesetAttributeMap::new(),
        )
        .unwrap();
        assert!(shirt.contains("kind: clothing"));
        assert!(shirt.contains("avatar_channels: torso, arms"));
        assert!(shirt.contains("crafting_family: tailoring"));

        let arrows = format_rules_item(
            &rules,
            "wooden_arrows",
            &shared::rulesets::RulesetAttributeMap::new(),
        )
        .unwrap();
        assert!(arrows.contains("kind: ammunition"));
        assert!(arrows.contains("quantity: 20"));

        let herb = format_rules_item(
            &rules,
            "blessed_herb",
            &shared::rulesets::RulesetAttributeMap::new(),
        )
        .unwrap();
        assert!(herb.contains("item: Blessed Herb"));
        assert!(herb.contains("kind: reagent"));
        assert!(herb.contains("quantity: 3"));
        assert!(herb.contains("visual_template: herb_sprig"));

        let wood = format_rules_item(
            &rules,
            "green_wood",
            &shared::rulesets::RulesetAttributeMap::new(),
        )
        .unwrap();
        assert!(wood.contains("item: Green Wood"));
        assert!(wood.contains("kind: material"));
        assert!(wood.contains("quantity: 5"));

        let node = format_rules_item(
            &rules,
            "wild_herb_node",
            &shared::rulesets::RulesetAttributeMap::new(),
        )
        .unwrap();
        assert!(node.contains("item: Wild Herb Node"));
        assert!(node.contains("kind: resource"));
        assert!(node.contains("resource_action"));

        let wood_node = format_rules_item(
            &rules,
            "green_wood_node",
            &shared::rulesets::RulesetAttributeMap::new(),
        )
        .unwrap();
        assert!(wood_node.contains("item: Green Wood Node"));
        assert!(wood_node.contains("kind: resource"));
        assert!(wood_node.contains("gather_wood"));

        let nest = format_rules_item(
            &rules,
            "bird_nest_node",
            &shared::rulesets::RulesetAttributeMap::new(),
        )
        .unwrap();
        assert!(nest.contains("item: Bird Nest Node"));
        assert!(nest.contains("kind: resource"));
        assert!(nest.contains("gather_feathers"));
    }

    #[test]
    fn formats_rules_recipe_details() {
        let rules = official_rules_table().unwrap();
        let output = format_rules_recipe(&rules, "wooden_arrows").unwrap();

        assert!(output.contains("recipe: Wooden Arrows"));
        assert!(output.contains("skill: fletching"));
        assert!(output.contains("required_skill: 0"));
        assert!(output.contains("difficulty: 10"));
        assert!(output.contains("profession_hint: Fletcher"));
        assert!(output.contains("material: Green Wood"));
        assert!(output.contains("material: Feather"));
        assert!(output.contains("ammunition: Wooden Arrows"));
        assert!(output.contains("x10"));

        let herb = format_rules_recipe(&rules, "blessed_herb").unwrap();
        assert!(herb.contains("skill: restoration"));
        assert!(herb.contains("class_hint: Cleric"));
        assert!(herb.contains("requires_spell: minor_heal"));
        assert!(herb.contains("material: Wild Herb"));
        assert!(herb.contains("reagent: Blessed Herb"));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("rules") {
        if let Err(err) = run_rules_command(&args[2..]) {
            eprintln!("{}", err);
            std::process::exit(1);
        }
        return;
    }

    let cli_options = match parse_terminal_args(&args) {
        Ok(options) => options,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };

    let path = match resolve_data_path(cli_options.path.as_ref()) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };

    let app = match TerminalApp::load(&path) {
        Ok(app) => app,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };

    let mode = match cli_options
        .mode
        .map(Ok)
        .unwrap_or_else(|| config_terminal_mode(&app.project.config))
    {
        Ok(mode) => mode,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };

    run_terminal_app(app, mode);
}
