use rusterix::prelude::*;
use rusterix::{Command, EntityAction, ServerState};
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, ExternalPrinter};
use shared::project::Project;
use shared::text_game as sg;
use std::collections::BTreeSet;
use std::fs;
use std::io::IsTerminal;
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

struct TerminalApp {
    project: Project,
    assets: Assets,
    server: Server,
    current_map: String,
    last_announced_hour: Option<u8>,
    last_nearby_attackers: BTreeSet<String>,
    auto_attack_target: Option<u32>,
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
        let project: Project = serde_json::from_str(&contents)
            .map_err(|err| format!("Failed to parse {}: {}", path.display(), err))?;

        let current_map = config_string(&project.config, "game", "start_region", "");
        if current_map.is_empty() {
            return Err("Game config is missing [game].start_region".into());
        }

        let mut app = Self {
            project,
            assets: Assets::default(),
            server: Server::default(),
            current_map,
            last_announced_hour: None,
            last_nearby_attackers: BTreeSet::new(),
            auto_attack_target: None,
        };

        app.start_server(false)?;
        app.create_local_player()?;

        for _ in 0..3 {
            app.tick();
            thread::sleep(Duration::from_millis(5));
        }

        app.last_announced_hour = app.current_time_hour_and_label().map(|(hour, _)| hour);
        app.last_nearby_attackers = app.current_nearby_attackers();

        Ok(app)
    }

    fn start_server(&mut self, debug: bool) -> Result<(), String> {
        self.server.clear();
        self.server.debug_mode = debug;
        self.server.log_changed = true;
        self.server.print_log_messages = false;

        insert_content_into_maps_mode(&mut self.project, debug);

        self.assets.rules = self.project.rules.clone();
        self.assets.locales_src = self.project.locales.clone();
        self.assets.audio_fx_src = self.project.audio_fx.clone();
        self.assets.authoring_src = self.project.authoring.clone();
        self.assets.read_locales();

        self.assets.entities.clear();
        self.assets.character_maps.clear();
        self.assets.entity_tiles.clear();
        for character in self.project.characters.values_mut() {
            if !character.module.routines.is_empty() {
                if character.source.is_empty() {
                    character.source = character.module.build(false);
                }
                if debug {
                    character.source_debug = character.module.build(true);
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
        }

        self.assets.items.clear();
        self.assets.item_maps.clear();
        self.assets.item_tiles.clear();
        for item in self.project.items.values_mut() {
            if !item.module.routines.is_empty() {
                if item.source.is_empty() {
                    item.source = item.module.build(false);
                }
                if debug {
                    item.source_debug = item.module.build(true);
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
        }

        self.assets.avatars.clear();
        for avatar in self.project.avatars.values() {
            self.assets
                .avatars
                .insert(avatar.name.clone(), avatar.clone());
        }

        for region in &mut self.project.regions {
            self.server.create_region_instance(
                region.name.clone(),
                region.map.clone(),
                &self.assets,
                self.project.config.clone(),
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

    fn take_hour_announcement(&mut self) -> Option<String> {
        let (hour, label) = self.current_time_hour_and_label()?;
        let previous_hour = self.last_announced_hour.replace(hour);
        if previous_hour == Some(hour) {
            None
        } else {
            Some(format!("It is {}.", label))
        }
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

    fn current_nearby_attackers(&self) -> BTreeSet<String> {
        let Some(region) = self.current_region() else {
            return BTreeSet::new();
        };
        sg::build_text_room(&region.map, &self.project.authoring)
            .map(|room| room.nearby_attackers.into_iter().collect())
            .unwrap_or_default()
    }

    fn move_to_exit(&mut self, exit: &sg::TextExit) -> bool {
        self.auto_attack_target = None;
        self.server.local_player_teleport_pos(exit.target_center);

        for _ in 0..4 {
            self.tick();
            if let Some(region) = self.current_region() {
                if let Some((_, sector)) = sg::current_player_and_sector(&region.map)
                    && sector.id == exit.target_sector_id
                {
                    return true;
                }
            }
        }

        false
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
            return;
        }

        let Some(target_id) = self.auto_attack_target else {
            return;
        };

        let Some(region) = self.current_region() else {
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

fn with_indefinite_article(text: &str) -> String {
    let lower = text.trim_start().to_ascii_lowercase();
    if lower.starts_with("your ")
        || lower.starts_with("my ")
        || lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("this ")
        || lower.starts_with("that ")
    {
        return text.to_string();
    }

    let first = text
        .trim_start()
        .chars()
        .next()
        .map(|c| c.to_ascii_lowercase());
    let article = match first {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    };
    format!("{} {}", article, text)
}

fn colorized_presence_phrase(name: &str, color: Option<&str>) -> String {
    colorize_terminal_text(&with_indefinite_article(name), color)
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
            if !instance.module.routines.is_empty() {
                if instance.source.is_empty() {
                    instance.source = instance.module.build(false);
                }
                if debug {
                    instance.source_debug = instance.module.build(true);
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
            }
            region.map.entities.push(entity);
        }

        region.map.items.clear();
        for instance in region.items.values_mut() {
            if !instance.module.routines.is_empty() {
                if instance.source.is_empty() {
                    instance.source = instance.module.build(false);
                }
                if debug {
                    instance.source_debug = instance.module.build(true);
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
            }
            region.map.items.push(item);
        }
    }
}

fn resolve_data_path(args: &[String]) -> Result<PathBuf, String> {
    if let Some(arg) = args.get(1) {
        return Ok(PathBuf::from(arg));
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

fn print_with_printer(printer: &mut impl ExternalPrinter, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    let _ = printer.print(text.to_string());
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

fn is_under_attack_message(message: &str) -> bool {
    message.trim_start().starts_with("You are under attack by ")
}

fn collect_region_output(app: &mut TerminalApp, include_room: bool) -> Vec<String> {
    let mut output = Vec::new();
    let Some(index) = app.current_region_index() else {
        return output;
    };
    let region_id = app.project.regions[index].map.id;
    let player_id = app
        .current_region()
        .and_then(|region| sg::current_player_and_sector(&region.map).map(|(player, _)| player.id));
    let current_description = if include_room {
        app.render_current_sector_description()
    } else {
        None
    };
    for (sender_entity, _sender_item, receiver_id, message, category) in
        app.server.get_messages(&region_id)
    {
        if let Some(player_id) = player_id
            && receiver_id != player_id
        {
            continue;
        }
        if authoring_auto_attack_mode(&app.project.authoring) == AutoAttackMode::OnAttack
            && is_under_attack_message(&message)
            && let (Some(sender_id), Some(player_id)) = (sender_entity, player_id)
            && sender_id != player_id
        {
            app.auto_attack_target = Some(sender_id);
        }
        if should_print_terminal_message(&message, &category) {
            if include_room
                && current_description
                    .as_deref()
                    .map(|text| text.trim() == message.trim())
                    .unwrap_or(false)
            {
                continue;
            }
            output.push(colorize_terminal_category(
                &message,
                &category,
                &app.project.authoring,
            ));
        }
    }
    for (_sender_entity, _sender_item, message, _category) in app.server.get_says(&region_id) {
        output.push(message);
    }
    if include_room {
        output.push(app.render_room_text());
        app.last_nearby_attackers = app.current_nearby_attackers();
    } else {
        let current_nearby_attackers = app.current_nearby_attackers();
        let new_attackers: Vec<String> = current_nearby_attackers
            .difference(&app.last_nearby_attackers)
            .cloned()
            .collect();
        if !new_attackers.is_empty() {
            output.push(sg::render_nearby_attacker_appearance_sentence(
                &new_attackers,
            ));
        }
        app.last_nearby_attackers = current_nearby_attackers;
    }
    if let Some(message) = app.take_hour_announcement() {
        output.push(message);
    }
    output
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
                distance,
                Some(intent.trim().to_string()),
            ));
        }
        sg::TextTarget::Item { id, distance } => {
            app.server.local_player_action(EntityAction::ItemClicked(
                id,
                distance,
                Some(intent.trim().to_string()),
            ));
        }
    }

    app.tick();
    let mut output = collect_region_output(app, false);
    if output.is_empty() {
        app.tick();
        output = collect_region_output(app, false);
    }
    if output.is_empty() {
        output.push(app.render_room_text());
        app.last_nearby_attackers = app.current_nearby_attackers();
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
                output.extend(collect_region_output(app, true));
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
        "help" => output.push(
            [
                "Commands:",
                "  look | l           Show the current room",
                "  wait | .           Advance one game tick",
                "  north | east | south | west",
                "  n | e | s | w",
                "                     Move through a text exit",
                "  go <name>          Move by exit direction or title",
                "  <intent> <target>  Trigger a configured player intent on a visible target",
                "  forward | f        Send a forward action",
                "  backward | b       Send a backward action",
                "  left               Send a left action",
                "  right              Send a right action",
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
                    output.extend(collect_region_output(app, true));
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
            let Some(index) = app.current_region_index() else {
                next_tick += tick_dt;
                continue;
            };
            let region_id = app.project.regions[index].map.id;
            let player_id = app.current_region().and_then(|region| {
                sg::current_player_and_sector(&region.map).map(|(player, _)| player.id)
            });
            for (sender_entity, _sender_item, receiver_id, message, category) in
                app.server.get_messages(&region_id)
            {
                if let Some(player_id) = player_id
                    && receiver_id != player_id
                {
                    continue;
                }
                if authoring_auto_attack_mode(&app.project.authoring) == AutoAttackMode::OnAttack
                    && is_under_attack_message(&message)
                    && let (Some(sender_id), Some(player_id)) = (sender_entity, player_id)
                    && sender_id != player_id
                {
                    app.auto_attack_target = Some(sender_id);
                }
                if should_print_terminal_message(&message, &category) {
                    print_with_printer(
                        &mut printer,
                        &colorize_terminal_category(&message, &category, &app.project.authoring),
                    );
                }
            }
            for (_sender_entity, _sender_item, message, _category) in
                app.server.get_says(&region_id)
            {
                print_with_printer(&mut printer, &message);
            }
            if let Some(message) = app.take_hour_announcement() {
                print_with_printer(&mut printer, &message);
            }
            next_tick += tick_dt;
        }
    }

    app.server.stop();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = match resolve_data_path(&args) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };

    let mut app = match TerminalApp::load(&path) {
        Ok(app) => app,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };

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
            println!("{}", app.render_room_text());
            let _ = app.discard_pending_messages();
        }
    }

    if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        run_realtime_cli(app);
    } else {
        run_blocking_cli(app);
    }
}
