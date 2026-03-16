use rusterix::prelude::*;
use rusterix::{Command, EntityAction, ServerState};
use shared::project::Project;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use toml::Table;

struct TerminalApp {
    project: Project,
    assets: Assets,
    server: Server,
    current_map: String,
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
        };

        app.start_server(false)?;
        app.create_local_player()?;

        for _ in 0..3 {
            app.tick();
            thread::sleep(Duration::from_millis(5));
        }

        Ok(app)
    }

    fn start_server(&mut self, debug: bool) -> Result<(), String> {
        self.server.clear();
        self.server.debug_mode = debug;
        self.server.log_changed = true;

        insert_content_into_maps_mode(&mut self.project, debug);

        self.assets.rules = self.project.rules.clone();
        self.assets.locales_src = self.project.locales.clone();
        self.assets.audio_fx_src = self.project.audio_fx.clone();
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

    fn render_room_text(&mut self) -> String {
        let Some(region) = self.current_region_mut() else {
            return format!("Current region '{}' not found.", self.current_map);
        };
        let map = &region.map;

        let Some(player) = map.entities.iter().find(|entity| entity.is_player()) else {
            return "No local player found.".into();
        };

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
        };

        let Some(sector) = sector else {
            return "No current room.".into();
        };

        let (title, description) = sector_text_metadata(sector);
        let mut lines = Vec::new();
        if !title.is_empty() {
            lines.push(title);
            lines.push(String::new());
        }
        if !description.is_empty() {
            lines.push(description);
            lines.push(String::new());
        }

        let exits: Vec<String> = sector
            .linedefs
            .iter()
            .filter_map(|linedef_id| map.find_linedef(*linedef_id))
            .map(|linedef| linedef.name.clone())
            .filter(|name| !name.trim().is_empty())
            .collect();
        if !exits.is_empty() {
            lines.push(format!("Exits: {}", exits.join(", ")));
        }

        let entities: Vec<String> = map
            .entities
            .iter()
            .filter(|entity| !entity.is_player())
            .filter(|entity| {
                entity
                    .get_attr_string("sector")
                    .filter(|s| !s.is_empty())
                    .map(|s| s == sector.name)
                    .unwrap_or_else(|| {
                        map.find_sector_at(entity.get_pos_xz()).map(|s| s.id) == Some(sector.id)
                    })
            })
            .map(display_name_for_entity)
            .filter(|name| !name.trim().is_empty())
            .collect();
        if !entities.is_empty() {
            lines.push(format!("Characters: {}", entities.join(", ")));
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
            .collect();
        if !items.is_empty() {
            lines.push(format!("Items: {}", items.join(", ")));
        }

        if let Some(time) = self
            .project
            .regions
            .iter()
            .find(|region| region.map.name == self.current_map)
            .and_then(|region| self.server.get_time(&region.map.id))
        {
            lines.push(String::new());
            lines.push(format!("Time: {}", time.to_time24()));
        }

        if lines.is_empty() {
            "No room text available.".into()
        } else {
            lines.join("\n")
        }
    }

    fn print_pending_messages(&mut self) {
        let Some(index) = self.current_region_index() else {
            return;
        };
        let region_id = self.project.regions[index].map.id;

        for (_sender_entity, _sender_item, _receiver, message, _category) in
            self.server.get_messages(&region_id)
        {
            println!("{}", message);
        }

        for (_sender_entity, _sender_item, message, _category) in self.server.get_says(&region_id) {
            println!("{}", message);
        }
    }
}

fn display_name_for_entity(entity: &Entity) -> String {
    entity
        .get_attr_string("name")
        .or_else(|| entity.get_attr_string("class_name"))
        .unwrap_or_else(|| format!("Entity {}", entity.id))
}

fn display_name_for_item(item: &Item) -> String {
    item.get_attr_string("name")
        .or_else(|| item.get_attr_string("class_name"))
        .unwrap_or_else(|| format!("Item {}", item.id))
}

fn sector_text_metadata(sector: &Sector) -> (String, String) {
    let mut title = sector.name.clone();
    let mut description = String::new();

    if let Some(Value::Str(data)) = sector.properties.get("data")
        && let Ok(table) = data.parse::<toml::Table>()
    {
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

    (title, description)
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

fn print_help() {
    println!("Commands:");
    println!("  look | l           Show the current room");
    println!("  wait | .           Advance one game tick");
    println!("  forward | f        Send a forward action");
    println!("  backward | b       Send a backward action");
    println!("  left               Send a left action");
    println!("  right              Send a right action");
    println!("  intent <name>      Set the current player intent");
    println!("  help               Show this help");
    println!("  quit | exit        Leave the client");
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

    println!("Loaded {}", path.display());
    println!();
    println!("{}", app.render_room_text());
    app.print_pending_messages();
    println!();
    print_help();

    let stdin = io::stdin();
    loop {
        print!("\n> ");
        let _ = io::stdout().flush();

        let mut input = String::new();
        if stdin.read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();
        match input {
            "" | "look" | "l" => {
                println!();
                println!("{}", app.render_room_text());
                app.print_pending_messages();
            }
            "wait" | "." => {
                app.tick();
                app.print_pending_messages();
                println!();
                println!("{}", app.render_room_text());
            }
            "forward" | "f" => {
                app.server.local_player_action(EntityAction::Forward);
                app.tick();
                app.server.local_player_action(EntityAction::Off);
                app.tick();
                app.print_pending_messages();
                println!();
                println!("{}", app.render_room_text());
            }
            "backward" | "b" => {
                app.server.local_player_action(EntityAction::Backward);
                app.tick();
                app.server.local_player_action(EntityAction::Off);
                app.tick();
                app.print_pending_messages();
                println!();
                println!("{}", app.render_room_text());
            }
            "left" => {
                app.server.local_player_action(EntityAction::Left);
                app.tick();
                app.server.local_player_action(EntityAction::Off);
                app.tick();
                app.print_pending_messages();
                println!();
                println!("{}", app.render_room_text());
            }
            "right" => {
                app.server.local_player_action(EntityAction::Right);
                app.tick();
                app.server.local_player_action(EntityAction::Off);
                app.tick();
                app.print_pending_messages();
                println!();
                println!("{}", app.render_room_text());
            }
            "help" => print_help(),
            "quit" | "exit" => break,
            _ if input.starts_with("intent ") => {
                let intent = input["intent ".len()..].trim();
                if intent.is_empty() {
                    println!("Usage: intent <name>");
                } else {
                    app.server
                        .local_player_action(EntityAction::Intent(intent.to_string()));
                    app.tick();
                    app.print_pending_messages();
                    println!();
                    println!("{}", app.render_room_text());
                }
            }
            _ => {
                println!("Unknown command. Type 'help' for available commands.");
            }
        }
    }

    app.server.stop();
}
