use rusterix::Linedef;
use rusterix::prelude::*;
use rusterix::{Command, EntityAction, ServerState};
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, ExternalPrinter};
use shared::project::Project;
use std::collections::BTreeSet;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use toml::Table;
use vek::Vec2;

#[derive(Clone, Copy, PartialEq, Eq)]
enum StartupDisplay {
    Description,
    Room,
    None,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExitPresentation {
    List,
    Sentence,
}

#[derive(Clone)]
struct TextExit {
    direction: String,
    title: String,
    target_title: String,
    target_sector_id: u32,
    target_center: Vec2<f32>,
}

#[derive(Clone)]
enum TextTarget {
    Entity { id: u32, distance: f32 },
    Item { id: u32, distance: f32 },
}

struct TerminalApp {
    project: Project,
    assets: Assets,
    server: Server,
    current_map: String,
    last_announced_hour: Option<u8>,
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
            auto_attack_target: None,
        };

        app.start_server(false)?;
        app.create_local_player()?;

        for _ in 0..3 {
            app.tick();
            thread::sleep(Duration::from_millis(5));
        }

        app.last_announced_hour = app.current_time_hour_and_label().map(|(hour, _)| hour);

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

    fn current_region_mut(&mut self) -> Option<&mut shared::region::Region> {
        self.project
            .regions
            .iter_mut()
            .find(|region| region.map.name == self.current_map)
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
        let probe_distance = authoring_connection_probe_distance(&self.project.authoring);
        let exit_presentation = authoring_exit_presentation(&self.project.authoring);
        let title_color = authoring_color(&self.project.authoring, "title");
        let object_color = authoring_color(&self.project.authoring, "objects");
        let item_color =
            authoring_color(&self.project.authoring, "items").or_else(|| object_color.clone());
        let corpse_color =
            authoring_color(&self.project.authoring, "corpses").or_else(|| object_color.clone());
        let neutral_character_color =
            authoring_color(&self.project.authoring, "characters").or_else(|| object_color.clone());
        let character_color_rules = authoring_character_color_rules(&self.project.authoring);
        let Some(region) = self.current_region_mut() else {
            return format!("Current region '{}' not found.", self.current_map);
        };
        let map = &region.map;

        let Some((_player, sector)) = current_player_and_sector(map) else {
            return "No local player found.".into();
        };

        let (title, description) = sector_text_metadata(sector);
        let mut lines = Vec::new();
        if !title.is_empty() {
            lines.push(colorize_terminal_text(&title, title_color.as_deref()));
            lines.push(String::new());
        }
        if !description.is_empty() {
            lines.push(description);
            lines.push(String::new());
        }

        let exits = resolve_text_exits(map, sector, probe_distance);
        if !exits.is_empty() {
            match exit_presentation {
                ExitPresentation::List => {
                    lines.push("Exits:".into());
                    for exit in exits {
                        lines.push(format!("  {} - {}", exit.direction, exit.title));
                    }
                }
                ExitPresentation::Sentence => {
                    lines.push(render_exit_sentence(&exits));
                }
            }
        }

        let mut live_entities = Vec::new();
        let mut dead_entities = Vec::new();
        for entity in &map.entities {
            if entity.is_player() {
                continue;
            }
            let same_sector = entity
                .get_attr_string("sector")
                .filter(|s| !s.is_empty())
                .map(|s| s == sector.name)
                .unwrap_or_else(|| {
                    map.find_sector_at(entity.get_pos_xz()).map(|s| s.id) == Some(sector.id)
                });
            if !same_sector {
                continue;
            }

            if entity_is_dead(entity) {
                let corpse = corpse_name_for_entity(entity);
                if !corpse.trim().is_empty() {
                    dead_entities.push(colorized_presence_phrase(&corpse, corpse_color.as_deref()));
                }
            } else {
                let name = display_name_for_entity(entity);
                if !name.trim().is_empty() {
                    let color = resolve_character_color(entity, &character_color_rules)
                        .or(neutral_character_color.as_deref());
                    live_entities.push(colorized_presence_phrase(&name, color));
                }
            }
        }
        if !live_entities.is_empty() {
            lines.push(render_presence_sentence("You see", &live_entities));
        }
        if !dead_entities.is_empty() {
            lines.push(render_presence_sentence("You see", &dead_entities));
        }

        let items: Vec<String> = map
            .items
            .iter()
            .filter(|item| {
                item.get_attr_string("sector")
                    .filter(|s| !s.is_empty())
                    .map(|s| s == sector.name)
                    .unwrap_or_else(|| {
                        map.find_sector_at(item.get_pos_xz()).map(|s| s.id) == Some(sector.id)
                    })
            })
            .map(display_name_for_item)
            .filter(|name| !name.trim().is_empty())
            .map(|name| colorized_presence_phrase(&name, item_color.as_deref()))
            .collect();
        if !items.is_empty() {
            lines.push(render_presence_sentence("You notice", &items));
        }

        if lines.is_empty() {
            "No room text available.".into()
        } else {
            lines.join("\n")
        }
    }

    fn render_current_sector_description(&mut self) -> Option<String> {
        let Some(region) = self.current_region_mut() else {
            return None;
        };
        let map = &region.map;

        let (_, sector) = current_player_and_sector(map)?;

        let (_, description) = sector_text_metadata(sector);
        if description.trim().is_empty() {
            None
        } else {
            Some(description.trim_end().to_string())
        }
    }

    fn move_to_exit(&mut self, exit: &TextExit) -> bool {
        self.auto_attack_target = None;
        self.server.local_player_teleport_pos(exit.target_center);

        for _ in 0..4 {
            self.tick();
            if let Some(region) = self.current_region() {
                if let Some((_, sector)) = current_player_and_sector(&region.map)
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
        let Some((player, sector)) = current_player_and_sector(&region.map) else {
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
        if entity_is_dead(target) {
            self.auto_attack_target = None;
            return;
        }

        let same_sector = target
            .get_attr_string("sector")
            .filter(|s| !s.is_empty())
            .map(|s| s == sector.name)
            .unwrap_or_else(|| {
                region.map.find_sector_at(target.get_pos_xz()).map(|s| s.id) == Some(sector.id)
            });
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

fn display_name_for_entity(entity: &Entity) -> String {
    entity
        .get_attr_string("name")
        .or_else(|| entity.get_attr_string("class_name"))
        .unwrap_or_else(|| format!("Entity {}", entity.id))
}

fn entity_is_dead(entity: &Entity) -> bool {
    entity
        .get_attr_string("mode")
        .map(|mode| mode.eq_ignore_ascii_case("dead"))
        .unwrap_or(false)
}

fn corpse_name_for_entity(entity: &Entity) -> String {
    let name = display_name_for_entity(entity);
    if name.trim().is_empty() {
        String::new()
    } else {
        format!("corpse of {}", sentence_case_exit_title(&name))
    }
}

fn display_name_for_item(item: &Item) -> String {
    item.get_attr_string("name")
        .or_else(|| item.get_attr_string("class_name"))
        .unwrap_or_else(|| format!("Item {}", item.id))
}

fn normalize_target_name(text: &str) -> String {
    text.trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c.is_ascii_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn target_matches(name: &str, query: &str) -> bool {
    let name_norm = normalize_target_name(name);
    let query_norm = normalize_target_name(query);
    !query_norm.is_empty()
        && (name_norm == query_norm
            || name_norm.starts_with(&(query_norm.clone() + " "))
            || name_norm.contains(&format!(" {}", query_norm)))
}

fn resolve_text_target(map: &Map, sector: &Sector, query: &str) -> Result<TextTarget, String> {
    let Some((player, _)) = current_player_and_sector(map) else {
        return Err("No local player found.".into());
    };
    let player_pos = player.get_pos_xz();
    let mut matches: Vec<(String, TextTarget)> = Vec::new();

    for entity in &map.entities {
        if entity.is_player() {
            continue;
        }
        if entity_is_dead(entity) {
            continue;
        }
        let same_sector = entity
            .get_attr_string("sector")
            .filter(|s| !s.is_empty())
            .map(|s| s == sector.name)
            .unwrap_or_else(|| {
                map.find_sector_at(entity.get_pos_xz()).map(|s| s.id) == Some(sector.id)
            });
        if !same_sector {
            continue;
        }
        let name = display_name_for_entity(entity);
        if target_matches(&name, query) {
            matches.push((
                name,
                TextTarget::Entity {
                    id: entity.id,
                    distance: player_pos.distance(entity.get_pos_xz()),
                },
            ));
        }
    }

    for item in &map.items {
        let same_sector = item
            .get_attr_string("sector")
            .filter(|s| !s.is_empty())
            .map(|s| s == sector.name)
            .unwrap_or_else(|| {
                map.find_sector_at(item.get_pos_xz()).map(|s| s.id) == Some(sector.id)
            });
        if !same_sector {
            continue;
        }
        let name = display_name_for_item(item);
        if target_matches(&name, query) {
            matches.push((
                name,
                TextTarget::Item {
                    id: item.id,
                    distance: player_pos.distance(item.get_pos_xz()),
                },
            ));
        }
    }

    if matches.is_empty() {
        return Err(format!("You do not see '{}' here.", query.trim()));
    }
    if matches.len() > 1 {
        matches.sort_by(|a, b| a.0.cmp(&b.0));
        let names = matches
            .into_iter()
            .map(|m| m.0)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!("Be more specific: {}", names));
    }

    Ok(matches.remove(0).1)
}

fn current_player_and_sector(map: &Map) -> Option<(&Entity, &Sector)> {
    let player = map.entities.iter().find(|entity| entity.is_player())?;
    let player_pos = player.get_pos_xz();
    let current_sector_name = player
        .get_attr_string("sector")
        .filter(|s| !s.is_empty())
        .or_else(|| map.find_sector_at(player_pos).map(|s| s.name.clone()))
        .unwrap_or_default();

    let sector = if !current_sector_name.is_empty() {
        map.sectors
            .iter()
            .find(|sector| sector.name == current_sector_name)
            .or_else(|| map.find_sector_at(player_pos))
    } else {
        map.find_sector_at(player_pos)
    }?;

    Some((player, sector))
}

fn sector_text_metadata(sector: &Sector) -> (String, String) {
    let mut title = sector.name.clone();
    let mut description = String::new();

    if let Some(Value::Str(data)) = sector.properties.get("data")
        && let Ok(table) = data.parse::<toml::Table>()
    {
        if let Some(value) = table.get("title").and_then(toml::Value::as_str)
            && !value.trim().is_empty()
        {
            title = value.to_string();
        }
        if let Some(value) = table.get("description").and_then(toml::Value::as_str)
            && !value.trim().is_empty()
        {
            description = value.to_string();
        }

        for section in ["text_adventure", "text", "ui"] {
            if let Some(group) = table.get(section).and_then(toml::Value::as_table) {
                if let Some(value) = group.get("title").and_then(toml::Value::as_str)
                    && !value.trim().is_empty()
                {
                    title = value.to_string();
                }
                if let Some(value) = group.get("description").and_then(toml::Value::as_str)
                    && !value.trim().is_empty()
                {
                    description = value.to_string();
                }
            }
        }
    }

    (title.trim().to_string(), description.trim_end().to_string())
}

fn linedef_text_metadata(linedef: &Linedef) -> (String, String) {
    let mut title = linedef.name.clone();
    let mut description = String::new();

    if let Some(Value::Str(data)) = linedef.properties.get("data")
        && let Ok(table) = data.parse::<toml::Table>()
    {
        if let Some(value) = table.get("title").and_then(toml::Value::as_str)
            && !value.trim().is_empty()
        {
            title = value.to_string();
        }
        if let Some(value) = table.get("description").and_then(toml::Value::as_str)
            && !value.trim().is_empty()
        {
            description = value.to_string();
        }

        for section in ["text_adventure", "text", "ui"] {
            if let Some(group) = table.get(section).and_then(toml::Value::as_table) {
                if let Some(value) = group.get("title").and_then(toml::Value::as_str)
                    && !value.trim().is_empty()
                {
                    title = value.to_string();
                }
                if let Some(value) = group.get("description").and_then(toml::Value::as_str)
                    && !value.trim().is_empty()
                {
                    description = value.to_string();
                }
            }
        }
    }

    (title.trim().to_string(), description.trim_end().to_string())
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

fn authoring_connection_probe_distance(src: &str) -> f32 {
    config_table(src)
        .and_then(|table| {
            table
                .get("connections")
                .and_then(toml::Value::as_table)
                .and_then(|table| table.get("probe_distance"))
                .and_then(|value| {
                    value
                        .as_float()
                        .or_else(|| value.as_integer().map(|v| v as f64))
                })
        })
        .map(|value| value as f32)
        .unwrap_or(1.5)
}

fn authoring_exit_presentation(src: &str) -> ExitPresentation {
    config_table(src)
        .and_then(|table| {
            table
                .get("exits")
                .and_then(toml::Value::as_table)
                .and_then(|table| table.get("style"))
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        })
        .map(|value| match value.trim().to_ascii_lowercase().as_str() {
            "sentence" => ExitPresentation::Sentence,
            _ => ExitPresentation::List,
        })
        .unwrap_or(ExitPresentation::List)
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

fn render_exit_sentence(exits: &[TextExit]) -> String {
    let parts: Vec<String> = exits
        .iter()
        .map(|exit| {
            let subject = if !exit.target_title.trim().is_empty() {
                sentence_case_exit_title(&exit.target_title)
            } else {
                sentence_case_exit_title(&exit.title)
            };
            format!("{} to the {}", subject, exit.direction)
        })
        .collect();

    match parts.len() {
        0 => String::new(),
        1 => format!("You see {}.", with_indefinite_article(&parts[0])),
        2 => format!(
            "You see {} and {}.",
            with_indefinite_article(&parts[0]),
            with_indefinite_article(&parts[1])
        ),
        _ => {
            let mut sentence = String::from("You see ");
            for (index, part) in parts.iter().enumerate() {
                let article_part = with_indefinite_article(part);
                if index == parts.len() - 1 {
                    sentence.push_str("and ");
                    sentence.push_str(&article_part);
                } else {
                    sentence.push_str(&article_part);
                    sentence.push_str(", ");
                }
            }
            sentence.push('.');
            sentence
        }
    }
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

fn sentence_case_exit_title(title: &str) -> String {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut chars = trimmed.chars();
    let first = chars.next().unwrap();
    let mut out = first.to_ascii_lowercase().to_string();
    out.push_str(chars.as_str());
    out
}

fn render_presence_sentence(prefix: &str, names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => format!("{} {} here.", prefix, names[0]),
        2 => format!("{} {} and {} here.", prefix, names[0], names[1]),
        _ => {
            let mut sentence = format!("{} ", prefix);
            for (index, part) in names.iter().enumerate() {
                if index == names.len() - 1 {
                    sentence.push_str("and ");
                    sentence.push_str(part);
                } else {
                    sentence.push_str(part);
                    sentence.push_str(", ");
                }
            }
            sentence.push_str(" here.");
            sentence
        }
    }
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

fn cardinal_direction(from: Vec2<f32>, to: Vec2<f32>) -> String {
    let delta = to - from;
    if delta.x.abs() > delta.y.abs() {
        if delta.x >= 0.0 { "east" } else { "west" }
    } else if delta.y >= 0.0 {
        "south"
    } else {
        "north"
    }
    .to_string()
}

fn probe_nearest_sector(map: &Map, origin: Vec2<f32>, max_distance: f32) -> Option<&Sector> {
    let mut best: Option<(&Sector, f32)> = None;

    for sector in &map.sectors {
        if sector.layer.is_some() {
            continue;
        }
        if let Some(distance) = sector.signed_distance(map, origin) {
            let distance = distance.abs();
            if distance <= max_distance {
                match best {
                    Some((_, best_distance)) if distance >= best_distance => {}
                    _ => best = Some((sector, distance)),
                }
            }
        }
    }

    best.map(|(sector, _)| sector)
}

fn resolve_text_exits(map: &Map, sector: &Sector, probe_distance: f32) -> Vec<TextExit> {
    let mut exits = Vec::new();
    let Some(current_center) = sector.center(map) else {
        return exits;
    };

    for linedef in &map.linedefs {
        let (line_title, line_description) = linedef_text_metadata(linedef);
        if line_title.is_empty() && line_description.is_empty() {
            continue;
        }

        let Some(v0) = map.get_vertex(linedef.start_vertex) else {
            continue;
        };
        let Some(v1) = map.get_vertex(linedef.end_vertex) else {
            continue;
        };

        let edge = v1 - v0;
        if edge.magnitude_squared() <= f32::EPSILON {
            continue;
        }

        let endpoint_a = probe_nearest_sector(map, v0, probe_distance);
        let endpoint_b = probe_nearest_sector(map, v1, probe_distance);

        let side_a;
        let side_b;
        let (front_sector, back_sector) = if let (Some(a), Some(b)) = (endpoint_a, endpoint_b) {
            (a, b)
        } else {
            let midpoint = (v0 + v1) * 0.5;
            let normal = Vec2::new(-edge.y, edge.x).normalized();
            side_a = probe_nearest_sector(
                map,
                midpoint - normal * (probe_distance * 0.5),
                probe_distance,
            );
            side_b = probe_nearest_sector(
                map,
                midpoint + normal * (probe_distance * 0.5),
                probe_distance,
            );
            let (Some(a), Some(b)) = (side_a, side_b) else {
                continue;
            };
            (a, b)
        };

        if front_sector.id == back_sector.id {
            continue;
        }

        let target_sector = if front_sector.id == sector.id {
            back_sector
        } else if back_sector.id == sector.id {
            front_sector
        } else {
            continue;
        };

        let Some(target_center) = target_sector.center(map) else {
            continue;
        };
        let direction = cardinal_direction(current_center, target_center);
        let (target_title, _) = sector_text_metadata(target_sector);
        let title = if !line_title.is_empty() {
            line_title.clone()
        } else if !target_title.is_empty() {
            target_title.clone()
        } else {
            target_sector.name.clone()
        };

        exits.push(TextExit {
            direction,
            title,
            target_title,
            target_sector_id: target_sector.id,
            target_center,
        });
    }

    exits.sort_by_key(|exit| match exit.direction.as_str() {
        "north" => 0,
        "east" => 1,
        "south" => 2,
        "west" => 3,
        _ => 4,
    });
    exits.dedup_by(|a, b| a.direction == b.direction && a.target_sector_id == b.target_sector_id);
    exits
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
        .and_then(|region| current_player_and_sector(&region.map).map(|(player, _)| player.id));
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
    let Some((_player, sector)) = current_player_and_sector(&region.map) else {
        return vec!["No local player found.".into()];
    };

    let target = match resolve_text_target(&region.map, sector, query) {
        Ok(target) => target,
        Err(err) => return vec![err],
    };

    match target {
        TextTarget::Entity { id, distance } => {
            app.server.local_player_action(EntityAction::EntityClicked(
                id,
                distance,
                Some(intent.trim().to_string()),
            ));
        }
        TextTarget::Item { id, distance } => {
            app.server.local_player_action(EntityAction::ItemClicked(
                id,
                distance,
                Some(intent.trim().to_string()),
            ));
        }
    }

    app.tick();
    collect_region_output(app, true)
}

fn current_player_supported_intents(app: &TerminalApp) -> BTreeSet<String> {
    let mut intents = BTreeSet::new();
    let Some(region) = app.current_region() else {
        return intents;
    };
    let Some((player, _)) = current_player_and_sector(&region.map) else {
        return intents;
    };
    let Some(class_name) = player.get_attr_string("class_name") else {
        return intents;
    };
    let Some(character) = app
        .project
        .characters
        .values()
        .find(|c| c.name == class_name)
    else {
        return intents;
    };
    let Ok(table) = character.data.parse::<Table>() else {
        return intents;
    };
    let Some(input) = table.get("input").and_then(toml::Value::as_table) else {
        return intents;
    };

    for value in input.values().filter_map(toml::Value::as_str) {
        let trimmed = value.trim();
        let lower = trimmed.to_ascii_lowercase();
        if let Some(inner) = lower
            .strip_prefix("intent(")
            .and_then(|v| v.strip_suffix(')'))
            .map(str::trim)
        {
            let inner = inner.trim_matches('"').trim_matches('\'').trim();
            if !inner.is_empty() {
                intents.insert(inner.to_string());
            }
        }
    }

    intents
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
                if let Some((_, sector)) = current_player_and_sector(&region.map) {
                    let exits = resolve_text_exits(
                        &region.map,
                        sector,
                        authoring_connection_probe_distance(&app.project.authoring),
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
                    if let Some((_, sector)) = current_player_and_sector(&region.map) {
                        let exits = resolve_text_exits(
                            &region.map,
                            sector,
                            authoring_connection_probe_distance(&app.project.authoring),
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
                current_player_and_sector(&region.map).map(|(player, _)| player.id)
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
