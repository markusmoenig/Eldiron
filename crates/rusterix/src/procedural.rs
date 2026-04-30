use crate::{Map, MapCamera, PixelSource, Tile, TileRole, Value};
use indexmap::IndexMap;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use theframework::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProcCell {
    Empty,
    Floor,
    Corridor,
    Wall,
    Door,
    Entrance,
    Exit,
}

#[derive(Clone, Copy, Debug)]
struct Room {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl Room {
    fn center(self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }

    fn marker(self) -> (i32, i32) {
        let (x, y) = self.center();
        (x, (y - 1).max(self.y + 1))
    }

    fn intersects(self, other: Self, padding: i32) -> bool {
        self.x - padding < other.x + other.w
            && self.x + self.w + padding > other.x
            && self.y - padding < other.y + other.h
            && self.y + self.h + padding > other.y
    }

    fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }

    fn is_boundary(self, x: i32, y: i32) -> bool {
        self.contains(x, y)
            && (x == self.x || x == self.x + self.w - 1 || y == self.y || y == self.y + self.h - 1)
    }
}

struct RoomCandidate {
    room: Room,
    prev_door: (i32, i32),
    next_door: (i32, i32),
    corridor: Vec<(i32, i32)>,
}

#[derive(Clone, Debug)]
struct ProceduralChoice {
    name: String,
    weight: u32,
}

#[derive(Clone, Debug)]
struct ProceduralCharacterRule {
    chance: f32,
    choices: Vec<ProceduralChoice>,
}

#[derive(Clone, Debug)]
pub struct ProceduralConfig {
    pub enabled: bool,
    pub generator: String,
    pub mode: String,
    pub seed: u64,
    pub style: String,
    pub door_placement: String,
    pub door_randomness: f32,
    pub width: i32,
    pub height: i32,
    pub room_count: i32,
    pub room_min_size: i32,
    pub room_max_size: i32,
    item_choices: HashMap<String, Vec<ProceduralChoice>>,
    character_rules: HashMap<String, ProceduralCharacterRule>,
}

impl Default for ProceduralConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            generator: "connected_rooms".to_string(),
            mode: "2d".to_string(),
            seed: 1,
            style: "stone".to_string(),
            door_placement: "both".to_string(),
            door_randomness: 1.0,
            width: 32,
            height: 32,
            room_count: 6,
            room_min_size: 6,
            room_max_size: 10,
            item_choices: HashMap::new(),
            character_rules: HashMap::new(),
        }
    }
}

impl ProceduralConfig {
    pub fn generated_item_names(&self) -> Vec<String> {
        self.item_choices
            .values()
            .flat_map(|choices| choices.iter().map(|choice| choice.name.clone()))
            .collect()
    }

    pub fn generated_character_names(&self) -> Vec<String> {
        self.character_rules
            .values()
            .flat_map(|rule| rule.choices.iter().map(|choice| choice.name.clone()))
            .collect()
    }

    pub fn apply_runtime_overrides<F>(&mut self, mut get_value: F)
    where
        F: FnMut(&str) -> Option<Value>,
    {
        if let Some(value) = get_value("procedural.seed").and_then(value_as_i64) {
            self.seed = value.max(0) as u64;
        }
        if let Some(value) = get_value("procedural.width").and_then(value_as_i64) {
            self.width = (value as i32).clamp(16, 256);
        }
        if let Some(value) = get_value("procedural.height").and_then(value_as_i64) {
            self.height = (value as i32).clamp(16, 256);
        }
        if let Some(value) = get_value("procedural.rooms")
            .or_else(|| get_value("procedural.room_count"))
            .and_then(value_as_i64)
        {
            self.room_count = (value as i32).clamp(1, 128);
        }
        if let Some(value) = get_value("procedural.room_min_size").and_then(value_as_i64) {
            self.room_min_size = (value as i32).clamp(5, 32);
        }
        if let Some(value) = get_value("procedural.room_max_size").and_then(value_as_i64) {
            self.room_max_size = (value as i32).clamp(self.room_min_size, 48);
        }
        if let Some(value) = get_value("procedural.door_randomness").and_then(value_as_f32) {
            self.door_randomness = value.clamp(0.0, 1.0);
        }
        if let Some(value) = get_value("procedural.door_placement").and_then(value_as_string) {
            let value = value.trim().to_ascii_lowercase();
            if matches!(value.as_str(), "entrances" | "exits" | "both") {
                self.door_placement = value;
            }
        }

        let character_kinds = self.character_rules.keys().cloned().collect::<Vec<_>>();
        for kind in character_kinds {
            let chance = get_value(&format!("procedural.characters.{kind}.chance"))
                .or_else(|| get_value(&format!("procedural.characters.{kind}.percentage")))
                .or_else(|| get_value(&format!("procedural.characters.{kind}.percent")))
                .and_then(value_as_f32);
            if let Some(chance) = chance
                && let Some(rule) = self.character_rules.get_mut(&kind)
            {
                rule.chance = if chance > 1.0 {
                    (chance / 100.0).clamp(0.0, 1.0)
                } else {
                    chance.clamp(0.0, 1.0)
                };
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProceduralSpawn {
    pub kind: String,
    pub name: String,
    pub position: Vec3<f32>,
}

#[derive(Default, Clone, Debug)]
pub struct ProceduralBuildOutput {
    pub item_spawns: Vec<ProceduralSpawn>,
    pub character_spawns: Vec<ProceduralSpawn>,
}

struct GeneratedGrid {
    cells: Vec<ProcCell>,
    rooms: Vec<Room>,
}

pub fn parse_procedural_config(config: &str) -> Option<ProceduralConfig> {
    let parsed: toml::Value = toml::from_str(config).ok()?;
    let section = parsed.get("procedural")?.as_table()?;
    parse_procedural_section(section)
}

pub fn parse_procedural_config_table(config: &toml::Table) -> Option<ProceduralConfig> {
    let section = config.get("procedural")?.as_table()?;
    parse_procedural_section(section)
}

fn parse_procedural_section(
    section: &toml::map::Map<String, toml::Value>,
) -> Option<ProceduralConfig> {
    let mut cfg = ProceduralConfig::default();

    if let Some(v) = section.get("enabled").and_then(toml::Value::as_bool) {
        cfg.enabled = v;
    }
    if let Some(v) = section.get("generator").and_then(toml::Value::as_str) {
        cfg.generator = v.trim().to_string();
    }
    if let Some(v) = section.get("mode").and_then(toml::Value::as_str) {
        cfg.mode = v.trim().to_string();
    }
    if let Some(v) = section.get("seed").and_then(toml::Value::as_integer) {
        cfg.seed = v.max(0) as u64;
    }
    if let Some(v) = section.get("style").and_then(toml::Value::as_str) {
        cfg.style = v.trim().to_string();
    }
    if let Some(v) = section.get("door_placement").and_then(toml::Value::as_str) {
        let value = v.trim().to_ascii_lowercase();
        if matches!(value.as_str(), "entrances" | "exits" | "both") {
            cfg.door_placement = value;
        }
    }
    if let Some(v) = section
        .get("door_randomness")
        .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
    {
        cfg.door_randomness = (v as f32).clamp(0.0, 1.0);
    }
    if let Some(v) = section.get("width").and_then(toml::Value::as_integer) {
        cfg.width = (v as i32).clamp(16, 256);
    }
    if let Some(v) = section.get("height").and_then(toml::Value::as_integer) {
        cfg.height = (v as i32).clamp(16, 256);
    }
    if let Some(v) = section
        .get("rooms")
        .or_else(|| section.get("room_count"))
        .and_then(toml::Value::as_integer)
    {
        cfg.room_count = (v as i32).clamp(1, 128);
    }
    if let Some(v) = section
        .get("room_min_size")
        .and_then(toml::Value::as_integer)
    {
        cfg.room_min_size = (v as i32).clamp(5, 32);
    }
    if let Some(v) = section
        .get("room_max_size")
        .and_then(toml::Value::as_integer)
    {
        cfg.room_max_size = (v as i32).clamp(cfg.room_min_size, 48);
    }
    cfg.item_choices = parse_item_choices(section);
    cfg.character_rules = parse_character_rules(section);

    Some(cfg)
}

pub fn bake_connected_rooms(
    map: &mut Map,
    tiles: &IndexMap<Uuid, Tile>,
    cfg: &ProceduralConfig,
) -> ProceduralBuildOutput {
    clear_map_for_build(map);
    let choices = tile_choices(tiles, &cfg.style);
    let generated = generate_grid(cfg);
    let mut tile_rng = StdRng::seed_from_u64(cfg.seed ^ 0x9e37_79b9_7f4a_7c15);
    let mut item_rng = StdRng::seed_from_u64(cfg.seed ^ 0x517c_c1b7_b272_220a);
    let mut character_rng = StdRng::seed_from_u64(cfg.seed ^ 0x6942_758f_2d8f_d7b3);
    let mut output = ProceduralBuildOutput::default();

    if !cfg.character_rules.is_empty() {
        for (room_index, room) in generated.rooms.iter().enumerate() {
            if room_index == 0 || room_index + 1 == generated.rooms.len() {
                continue;
            }
            for (kind, rule) in &cfg.character_rules {
                if rule.chance > 0.0
                    && character_rng.random::<f32>() <= rule.chance
                    && let Some(choice) = choose_weighted_choice(&rule.choices, &mut character_rng)
                {
                    output.character_spawns.push(ProceduralSpawn {
                        kind: kind.clone(),
                        name: choice.name.clone(),
                        position: room_spawn_position(*room, cfg),
                    });
                }
            }
        }
    }

    for y in 0..cfg.height {
        for x in 0..cfg.width {
            let cell = generated.cells[(y * cfg.width + x) as usize];
            let mut kind = match cell {
                ProcCell::Empty => continue,
                ProcCell::Floor | ProcCell::Corridor => "floor",
                ProcCell::Wall => "wall",
                ProcCell::Door => "floor",
                ProcCell::Entrance => "entrance",
                ProcCell::Exit => "exit",
            };
            let marker_name = match cell {
                ProcCell::Entrance => Some("entrance"),
                ProcCell::Exit => Some("exit"),
                _ => None,
            };
            if marker_name.is_some() && !choices.contains_key(kind) {
                kind = "floor";
            }
            if let Some(tile_id) = choose_tile(&choices, kind, &mut tile_rng) {
                add_cell_sector(map, x, y, cfg, kind, tile_id, marker_name);
            }
            if matches!(cell, ProcCell::Door)
                && let Some(choice) = cfg
                    .item_choices
                    .get("door")
                    .and_then(|choices| choose_weighted_choice(choices, &mut item_rng))
            {
                output.item_spawns.push(ProceduralSpawn {
                    kind: "door".to_string(),
                    name: choice.name.clone(),
                    position: cell_world_position(x, y, cfg),
                });
            }
        }
    }

    map.camera = MapCamera::TwoD;
    if let Some(entrance_center) = map
        .sectors
        .iter()
        .find(|sector| sector.name == "entrance")
        .and_then(|sector| sector.center(map))
    {
        // The legacy 2D render path still uses Map::offset for its camera.
        // Procedural rebuilds can move the entrance far away from the authored
        // map offset, so recenter the generated map around the new entrance.
        map.offset.x = -entrance_center.x * map.grid_size;
        map.offset.y = entrance_center.y * map.grid_size;
    }
    map.changed = map.changed.wrapping_add(1);
    map.update_surfaces();
    output
}

fn parse_weighted_choices(table: &toml::map::Map<String, toml::Value>) -> Vec<ProceduralChoice> {
    let mut choices = Vec::new();
    if let Some(name) = table.get("name").and_then(toml::Value::as_str) {
        choices.push(ProceduralChoice {
            name: name.trim().to_string(),
            weight: table
                .get("weight")
                .and_then(toml::Value::as_integer)
                .unwrap_or(1)
                .max(1) as u32,
        });
    }

    choices.retain(|choice| !choice.name.is_empty());
    choices
}

fn parse_item_choices(
    section: &toml::map::Map<String, toml::Value>,
) -> HashMap<String, Vec<ProceduralChoice>> {
    let mut choices = HashMap::new();
    if let Some(items) = section.get("items").and_then(toml::Value::as_array) {
        for value in items {
            let Some(table) = value.as_table() else {
                continue;
            };
            let kind = table
                .get("kind")
                .and_then(toml::Value::as_str)
                .unwrap_or("item")
                .trim()
                .to_ascii_lowercase();
            let kind_choices = parse_weighted_choices(table);
            if !kind_choices.is_empty() {
                choices
                    .entry(kind)
                    .or_insert_with(Vec::new)
                    .extend(kind_choices);
            }
        }
    }
    if let Some(items) = section.get("items").and_then(toml::Value::as_table) {
        for (kind, value) in items {
            let Some(table) = value.as_table() else {
                continue;
            };
            let kind_choices = parse_weighted_choices(table);
            if !kind_choices.is_empty() {
                choices.insert(kind.trim().to_ascii_lowercase(), kind_choices);
            }
        }
    }
    choices
}

fn parse_character_rules(
    section: &toml::map::Map<String, toml::Value>,
) -> HashMap<String, ProceduralCharacterRule> {
    let mut rules = HashMap::new();
    if let Some(characters) = section.get("characters").and_then(toml::Value::as_array) {
        for value in characters {
            let Some(table) = value.as_table() else {
                continue;
            };
            let kind = table
                .get("kind")
                .and_then(toml::Value::as_str)
                .unwrap_or("monster")
                .trim()
                .to_ascii_lowercase();
            let choices = parse_weighted_choices(table);
            if choices.is_empty() {
                continue;
            }
            rules.insert(
                kind,
                ProceduralCharacterRule {
                    chance: parse_probability(table),
                    choices,
                },
            );
        }
    }
    if let Some(characters) = section.get("characters").and_then(toml::Value::as_table) {
        for (kind, value) in characters {
            let Some(table) = value.as_table() else {
                continue;
            };
            let choices = parse_weighted_choices(table);
            if choices.is_empty() {
                continue;
            }
            rules.insert(
                kind.trim().to_ascii_lowercase(),
                ProceduralCharacterRule {
                    chance: parse_probability(table),
                    choices,
                },
            );
        }
    }
    rules
}

fn parse_probability(table: &toml::map::Map<String, toml::Value>) -> f32 {
    let value = table
        .get("chance")
        .or_else(|| table.get("percentage"))
        .or_else(|| table.get("percent"))
        .and_then(|value| {
            value
                .as_float()
                .or_else(|| value.as_integer().map(|value| value as f64))
        })
        .unwrap_or(1.0);
    if value > 1.0 {
        (value as f32 / 100.0).clamp(0.0, 1.0)
    } else {
        (value as f32).clamp(0.0, 1.0)
    }
}

fn value_as_i64(value: Value) -> Option<i64> {
    match value {
        Value::Int(value) => Some(value as i64),
        Value::UInt(value) => Some(value as i64),
        Value::Int64(value) => Some(value),
        Value::Float(value) => Some(value as i64),
        _ => None,
    }
}

fn value_as_f32(value: Value) -> Option<f32> {
    match value {
        Value::Int(value) => Some(value as f32),
        Value::UInt(value) => Some(value as f32),
        Value::Int64(value) => Some(value as f32),
        Value::Float(value) => Some(value),
        _ => None,
    }
}

fn value_as_string(value: Value) -> Option<String> {
    match value {
        Value::Str(value) => Some(value),
        _ => None,
    }
}

fn clear_map_for_build(map: &mut Map) {
    map.vertices.clear();
    map.linedefs.clear();
    map.sectors.clear();
    map.surfaces.clear();
    map.profiles.clear();
    map.softrigs.clear();
    map.editing_rig = None;
    map.soft_animator = None;
    map.dungeon = Default::default();
    map.terrain_organic_layer = Default::default();
    map.properties.remove("tiles");
    map.properties.remove("blend_tiles");
    map.clear_temp();
    map.clear_selection();
}

fn tile_choices(tiles: &IndexMap<Uuid, Tile>, style: &str) -> HashMap<String, Vec<(Uuid, u32)>> {
    let mut choices: HashMap<String, Vec<(Uuid, u32)>> = HashMap::new();
    for (id, tile) in tiles {
        let procedural = &tile.procedural;
        if procedural.kind.trim().is_empty() || procedural.kind.trim() == "none" {
            continue;
        }
        if procedural.style.trim().eq_ignore_ascii_case(style) {
            choices
                .entry(procedural.kind.trim().to_ascii_lowercase())
                .or_default()
                .push((*id, procedural.weight.max(1)));
        }
    }

    for (id, tile) in tiles {
        let procedural = &tile.procedural;
        if procedural.kind.trim().is_empty() || procedural.kind.trim() == "none" {
            continue;
        }
        let key = procedural.kind.trim().to_ascii_lowercase();
        if choices.contains_key(&key) {
            continue;
        }
        choices
            .entry(key)
            .or_default()
            .push((*id, procedural.weight.max(1)));
    }

    if choices.is_empty()
        && let Some((id, _)) = tiles
            .iter()
            .find(|(_, tile)| tile.role == TileRole::Dungeon)
    {
        choices.insert("floor".to_string(), vec![(*id, 1)]);
        choices.insert("wall".to_string(), vec![(*id, 1)]);
    }
    if choices.is_empty()
        && let Some((id, _)) = tiles.first()
    {
        choices.insert("floor".to_string(), vec![(*id, 1)]);
        choices.insert("wall".to_string(), vec![(*id, 1)]);
    }
    choices
}

fn choose_tile(
    choices: &HashMap<String, Vec<(Uuid, u32)>>,
    kind: &str,
    rng: &mut StdRng,
) -> Option<Uuid> {
    let candidates = choices
        .get(kind)
        .or_else(|| match kind {
            "entrance" | "exit" | "door" => choices.get("floor"),
            _ => None,
        })
        .or_else(|| choices.get("floor"))?;
    let total = candidates.iter().map(|(_, w)| *w).sum::<u32>().max(1);
    let mut roll = rng.random_range(0..total);
    for (id, weight) in candidates {
        if roll < *weight {
            return Some(*id);
        }
        roll -= *weight;
    }
    candidates.first().map(|(id, _)| *id)
}

fn choose_weighted_choice<'a>(
    choices: &'a [ProceduralChoice],
    rng: &mut StdRng,
) -> Option<&'a ProceduralChoice> {
    let total = choices
        .iter()
        .map(|choice| choice.weight.max(1))
        .sum::<u32>()
        .max(1);
    let mut roll = rng.random_range(0..total);
    for choice in choices {
        let weight = choice.weight.max(1);
        if roll < weight {
            return Some(choice);
        }
        roll -= weight;
    }
    choices.first()
}

fn generate_grid(cfg: &ProceduralConfig) -> GeneratedGrid {
    let mut rng = StdRng::seed_from_u64(cfg.seed);
    let mut cells = vec![ProcCell::Empty; (cfg.width * cfg.height) as usize];
    let mut rooms: Vec<Room> = Vec::new();

    for _ in 0..80 {
        let w = rng.random_range(cfg.room_min_size..=cfg.room_max_size);
        let h = rng.random_range(cfg.room_min_size..=cfg.room_max_size);
        if cfg.width <= w + 4 || cfg.height <= h + 4 {
            continue;
        }
        let room = Room {
            x: rng.random_range(2..cfg.width - w - 1),
            y: rng.random_range(2..cfg.height - h - 1),
            w,
            h,
        };
        stamp_room(&mut cells, cfg.width, room);
        rooms.push(room);
        break;
    }

    while rooms.len() < cfg.room_count as usize {
        let Some(prev) = rooms.last().copied() else {
            break;
        };
        let mut placed = false;

        for _ in 0..160 {
            let w = rng.random_range(cfg.room_min_size..=cfg.room_max_size);
            let h = rng.random_range(cfg.room_min_size..=cfg.room_max_size);
            let gap = rng.random_range(2..=6);
            let dir = rng.random_range(0..4);
            let Some(candidate) = candidate_next_room(prev, w, h, gap, dir, &mut rng) else {
                continue;
            };

            if !room_in_bounds(cfg, candidate.room)
                || rooms
                    .iter()
                    .any(|other| candidate.room.intersects(*other, 2))
                || has_adjacent_door(
                    &cells,
                    cfg.width,
                    cfg.height,
                    candidate.prev_door.0,
                    candidate.prev_door.1,
                )
                || !corridor_is_clear(&cells, cfg.width, cfg.height, &candidate.corridor)
            {
                continue;
            }

            carve_corridor(&mut cells, cfg.width, cfg.height, &candidate.corridor);
            stamp_room(&mut cells, cfg.width, candidate.room);
            cells[(candidate.prev_door.1 * cfg.width + candidate.prev_door.0) as usize] =
                ProcCell::Floor;
            cells[(candidate.next_door.1 * cfg.width + candidate.next_door.0) as usize] =
                ProcCell::Floor;

            if (cfg.door_placement == "exits" || cfg.door_placement == "both")
                && should_place_door(cfg, &mut rng)
            {
                cells[(candidate.prev_door.1 * cfg.width + candidate.prev_door.0) as usize] =
                    ProcCell::Door;
            }
            if (cfg.door_placement == "entrances" || cfg.door_placement == "both")
                && should_place_door(cfg, &mut rng)
            {
                cells[(candidate.next_door.1 * cfg.width + candidate.next_door.0) as usize] =
                    ProcCell::Door;
            }
            rooms.push(candidate.room);
            placed = true;
            break;
        }

        if !placed {
            break;
        }
    }

    let entrance = rooms.last().map(|room| room.marker());
    let exit = rooms.first().map(|room| room.marker());
    if let Some((x, y)) = entrance
        && in_bounds(cfg.width, cfg.height, x, y)
    {
        cells[(y * cfg.width + x) as usize] = ProcCell::Entrance;
    }
    if let Some((x, y)) = exit
        && in_bounds(cfg.width, cfg.height, x, y)
    {
        cells[(y * cfg.width + x) as usize] = ProcCell::Exit;
    }

    let floorish = |cell: ProcCell| {
        matches!(
            cell,
            ProcCell::Floor
                | ProcCell::Corridor
                | ProcCell::Door
                | ProcCell::Entrance
                | ProcCell::Exit
        )
    };
    let snapshot = cells.clone();
    for y in 0..cfg.height {
        for x in 0..cfg.width {
            let idx = (y * cfg.width + x) as usize;
            if snapshot[idx] != ProcCell::Empty {
                continue;
            }
            let mut adjacent = false;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x + dx;
                    let ny = y + dy;
                    if in_bounds(cfg.width, cfg.height, nx, ny)
                        && floorish(snapshot[(ny * cfg.width + nx) as usize])
                    {
                        adjacent = true;
                    }
                }
            }
            if adjacent {
                cells[idx] = ProcCell::Wall;
            }
        }
    }

    GeneratedGrid { cells, rooms }
}

fn should_place_door(cfg: &ProceduralConfig, rng: &mut StdRng) -> bool {
    cfg.door_randomness >= 1.0 || rng.random::<f32>() <= cfg.door_randomness
}

fn stamp_room(cells: &mut [ProcCell], width: i32, room: Room) {
    for y in room.y..room.y + room.h {
        for x in room.x..room.x + room.w {
            let idx = (y * width + x) as usize;
            cells[idx] = if room.is_boundary(x, y) {
                ProcCell::Wall
            } else {
                ProcCell::Floor
            };
        }
    }
}

fn candidate_next_room(
    prev: Room,
    w: i32,
    h: i32,
    gap: i32,
    dir: i32,
    rng: &mut StdRng,
) -> Option<RoomCandidate> {
    let max_offset = ((prev.w.max(prev.h) + w.max(h)) / 2).clamp(3, 10);
    match dir {
        0 => {
            let anchor_y = rng.random_range(prev.y + 1..=prev.y + prev.h - 2);
            let room = Room {
                x: prev.x + prev.w + gap,
                y: prev.y + random_room_offset(rng, max_offset),
                w,
                h,
            };
            let next_y = clamp_to_room_inner(anchor_y, room.y, room.h);
            Some(RoomCandidate {
                room,
                prev_door: (prev.x + prev.w - 1, anchor_y),
                next_door: (room.x, next_y),
                corridor: connector_path((prev.x + prev.w, anchor_y), (room.x - 1, next_y), rng),
            })
        }
        1 => {
            let anchor_y = rng.random_range(prev.y + 1..=prev.y + prev.h - 2);
            let room = Room {
                x: prev.x - gap - w,
                y: prev.y + random_room_offset(rng, max_offset),
                w,
                h,
            };
            let next_y = clamp_to_room_inner(anchor_y, room.y, room.h);
            Some(RoomCandidate {
                room,
                prev_door: (prev.x, anchor_y),
                next_door: (room.x + room.w - 1, next_y),
                corridor: connector_path((prev.x - 1, anchor_y), (room.x + room.w, next_y), rng),
            })
        }
        2 => {
            let anchor_x = rng.random_range(prev.x + 1..=prev.x + prev.w - 2);
            let room = Room {
                x: prev.x + random_room_offset(rng, max_offset),
                y: prev.y + prev.h + gap,
                w,
                h,
            };
            let next_x = clamp_to_room_inner(anchor_x, room.x, room.w);
            Some(RoomCandidate {
                room,
                prev_door: (anchor_x, prev.y + prev.h - 1),
                next_door: (next_x, room.y),
                corridor: connector_path((anchor_x, prev.y + prev.h), (next_x, room.y - 1), rng),
            })
        }
        3 => {
            let anchor_x = rng.random_range(prev.x + 1..=prev.x + prev.w - 2);
            let room = Room {
                x: prev.x + random_room_offset(rng, max_offset),
                y: prev.y - gap - h,
                w,
                h,
            };
            let next_x = clamp_to_room_inner(anchor_x, room.x, room.w);
            Some(RoomCandidate {
                room,
                prev_door: (anchor_x, prev.y),
                next_door: (next_x, room.y + room.h - 1),
                corridor: connector_path((anchor_x, prev.y - 1), (next_x, room.y + room.h), rng),
            })
        }
        _ => None,
    }
}

fn random_room_offset(rng: &mut StdRng, max_offset: i32) -> i32 {
    if rng.random_bool(0.25) {
        0
    } else {
        let offset = rng.random_range(2..=max_offset.max(2));
        if rng.random_bool(0.5) {
            offset
        } else {
            -offset
        }
    }
}

fn clamp_to_room_inner(value: i32, start: i32, size: i32) -> i32 {
    value.clamp(start + 1, start + size - 2)
}

fn connector_path(outside0: (i32, i32), outside1: (i32, i32), rng: &mut StdRng) -> Vec<(i32, i32)> {
    let bend = if outside0.0 == outside1.0 || outside0.1 == outside1.1 {
        outside1
    } else if rng.random_bool(0.5) {
        (outside1.0, outside0.1)
    } else {
        (outside0.0, outside1.1)
    };

    let mut cells = line_cells(outside0, bend);
    let tail = line_cells(bend, outside1);
    cells.extend(tail.into_iter().skip(1));
    cells
}

fn line_cells(from: (i32, i32), to: (i32, i32)) -> Vec<(i32, i32)> {
    let mut cells = vec![];
    let mut x = from.0;
    let mut y = from.1;
    let step_x = (to.0 - from.0).signum();
    let step_y = (to.1 - from.1).signum();
    loop {
        cells.push((x, y));
        if x == to.0 && y == to.1 {
            break;
        }
        if x != to.0 {
            x += step_x;
        } else if y != to.1 {
            y += step_y;
        }
    }
    cells
}

fn room_in_bounds(cfg: &ProceduralConfig, room: Room) -> bool {
    room.x >= 2
        && room.y >= 2
        && room.x + room.w < cfg.width - 2
        && room.y + room.h < cfg.height - 2
}

fn corridor_is_clear(cells: &[ProcCell], width: i32, height: i32, corridor: &[(i32, i32)]) -> bool {
    corridor.iter().all(|&(x, y)| {
        in_bounds(width, height, x, y) && cells[(y * width + x) as usize] == ProcCell::Empty
    })
}

fn has_adjacent_door(cells: &[ProcCell], width: i32, height: i32, x: i32, y: i32) -> bool {
    for dy in -1..=1 {
        for dx in -1..=1 {
            let nx = x + dx;
            let ny = y + dy;
            if in_bounds(width, height, nx, ny)
                && cells[(ny * width + nx) as usize] == ProcCell::Door
            {
                return true;
            }
        }
    }
    false
}

fn carve_corridor(cells: &mut [ProcCell], width: i32, height: i32, corridor: &[(i32, i32)]) {
    for &(x, y) in corridor {
        if in_bounds(width, height, x, y) {
            let idx = (y * width + x) as usize;
            if matches!(cells[idx], ProcCell::Empty | ProcCell::Corridor) {
                cells[idx] = ProcCell::Corridor;
            }
        }
    }
}

fn in_bounds(width: i32, height: i32, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && x < width && y < height
}

fn cell_world_position(x: i32, y: i32, cfg: &ProceduralConfig) -> Vec3<f32> {
    let ox = -(cfg.width as f32) * 0.5;
    let oy = -(cfg.height as f32) * 0.5;
    Vec3::new(ox + x as f32 + 0.5, 0.0, oy + y as f32 + 0.5)
}

fn room_spawn_position(room: Room, cfg: &ProceduralConfig) -> Vec3<f32> {
    let (x, y) = room.center();
    cell_world_position(x, y, cfg)
}

fn add_cell_sector(
    map: &mut Map,
    x: i32,
    y: i32,
    cfg: &ProceduralConfig,
    kind: &str,
    tile_id: Uuid,
    marker_name: Option<&str>,
) -> bool {
    let ox = -(cfg.width as f32) * 0.5;
    let oy = -(cfg.height as f32) * 0.5;
    let x0 = ox + x as f32;
    let y0 = oy + y as f32;
    let x1 = x0 + 1.0;
    let y1 = y0 + 1.0;

    let v0 = map.add_vertex_at(x0, y0);
    let v1 = map.add_vertex_at(x1, y0);
    let v2 = map.add_vertex_at(x1, y1);
    let v3 = map.add_vertex_at(x0, y1);

    let existing_sectors = map.find_sectors_with_vertex_indices(&[v0, v1, v2, v3]);
    let layer = existing_sectors
        .iter()
        .filter_map(|sector_id| map.find_sector(*sector_id).and_then(|sector| sector.layer))
        .max()
        .unwrap_or(0);

    map.possible_polygon.clear();
    let _ = map.create_linedef_manual(v0, v1);
    let _ = map.create_linedef_manual(v1, v2);
    let _ = map.create_linedef_manual(v2, v3);
    let _ = map.create_linedef_manual(v3, v0);
    let Some(sector_id) = map.close_polygon_manual() else {
        return false;
    };

    if let Some(sector) = map.find_sector_mut(sector_id) {
        sector.name = marker_name.map(str::to_string).unwrap_or_default();
        sector.layer = Some(layer + 1);
        sector.properties.set("rect", Value::Bool(true));
        sector
            .properties
            .set("procedural_generated", Value::Bool(true));
        sector
            .properties
            .set("procedural_generator", Value::Str(cfg.generator.clone()));
        sector
            .properties
            .set("procedural_style", Value::Str(cfg.style.clone()));
        sector
            .properties
            .set("procedural_kind", Value::Str(kind.to_string()));
        sector
            .properties
            .set("source", Value::Source(PixelSource::TileId(tile_id)));
        sector.properties.set("tile_mode", Value::Int(0));
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_fingerprint(cfg: &ProceduralConfig) -> Vec<u8> {
        generate_grid(cfg)
            .cells
            .into_iter()
            .map(|cell| match cell {
                ProcCell::Empty => 0,
                ProcCell::Floor => 1,
                ProcCell::Corridor => 2,
                ProcCell::Wall => 3,
                ProcCell::Door => 4,
                ProcCell::Entrance => 5,
                ProcCell::Exit => 6,
            })
            .collect()
    }

    #[test]
    fn connected_rooms_changes_with_runtime_seed() {
        let base = ProceduralConfig {
            seed: 4,
            style: String::new(),
            width: 48,
            height: 48,
            room_count: 6,
            room_min_size: 5,
            room_max_size: 9,
            door_placement: "exits".into(),
            door_randomness: 0.7,
            ..ProceduralConfig::default()
        };

        let mut first = base.clone();
        first.seed = first.seed.wrapping_add(1);
        let mut second = base.clone();
        second.seed = second.seed.wrapping_add(2);

        assert_ne!(grid_fingerprint(&first), grid_fingerprint(&second));
    }
}
