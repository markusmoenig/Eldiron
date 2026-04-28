use crate::editor::UNDOMANAGER;
use crate::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rusterix::{PixelSource, Tile, TileRole, Value};
use std::collections::HashMap;

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
struct ProceduralItemChoice {
    name: String,
    weight: u32,
}

#[derive(Clone, Debug)]
struct ProceduralCharacterRule {
    chance: f32,
    choices: Vec<ProceduralItemChoice>,
}

#[derive(Clone, Debug)]
struct ProceduralConfig {
    enabled: bool,
    generator: String,
    mode: String,
    seed: u64,
    style: String,
    door_placement: String,
    door_randomness: f32,
    width: i32,
    height: i32,
    room_count: i32,
    room_min_size: i32,
    room_max_size: i32,
    item_choices: HashMap<String, Vec<ProceduralItemChoice>>,
    character_rules: HashMap<String, ProceduralCharacterRule>,
}

struct GeneratedGrid {
    cells: Vec<ProcCell>,
    rooms: Vec<Room>,
}

struct ProceduralItemSpawn {
    kind: String,
    position: Vec3<f32>,
}

struct ProceduralCharacterSpawn {
    kind: String,
    position: Vec3<f32>,
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

pub struct BuildProcedural {
    id: TheId,
    nodeui: TheNodeUI,
}

impl BuildProcedural {
    fn parse_config(config: &str) -> Option<ProceduralConfig> {
        let parsed: toml::Value = toml::from_str(config).ok()?;
        let section = parsed.get("procedural")?.as_table()?;
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
            if value == "entrances" || value == "exits" || value == "both" {
                cfg.door_placement = value;
            }
        }
        if let Some(v) = section
            .get("door_randomness")
            .and_then(toml::Value::as_float)
        {
            cfg.door_randomness = (v as f32).clamp(0.0, 1.0);
        } else if let Some(v) = section
            .get("door_randomness")
            .and_then(toml::Value::as_integer)
        {
            cfg.door_randomness = (v as f32).clamp(0.0, 1.0);
        }
        if let Some(v) = section.get("width").and_then(toml::Value::as_integer) {
            cfg.width = (v as i32).clamp(16, 256);
        }
        if let Some(v) = section.get("height").and_then(toml::Value::as_integer) {
            cfg.height = (v as i32).clamp(16, 256);
        }
        if let Some(v) = section.get("room_count").and_then(toml::Value::as_integer) {
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
        cfg.item_choices = Self::parse_item_choices(section);
        cfg.character_rules = Self::parse_character_rules(section);

        Some(cfg)
    }

    fn parse_weighted_choices(
        table: &toml::map::Map<String, toml::Value>,
    ) -> Vec<ProceduralItemChoice> {
        let mut choices = Vec::new();
        if let Some(name) = table.get("name").and_then(toml::Value::as_str) {
            let weight = table
                .get("weight")
                .and_then(toml::Value::as_integer)
                .unwrap_or(1)
                .max(1) as u32;
            choices.push(ProceduralItemChoice {
                name: name.trim().to_string(),
                weight,
            });
        }

        if let Some(names) = table.get("names").and_then(toml::Value::as_array) {
            let weights = table.get("weights").and_then(toml::Value::as_array);
            for (index, name) in names.iter().enumerate() {
                let Some(name) = name.as_str() else {
                    continue;
                };
                let weight = weights
                    .and_then(|weights| weights.get(index))
                    .and_then(toml::Value::as_integer)
                    .unwrap_or(1)
                    .max(1) as u32;
                choices.push(ProceduralItemChoice {
                    name: name.trim().to_string(),
                    weight,
                });
            }
        }

        if let Some(choice_tables) = table.get("choices").and_then(toml::Value::as_array) {
            for choice in choice_tables {
                let Some(choice) = choice.as_table() else {
                    continue;
                };
                let Some(name) = choice.get("name").and_then(toml::Value::as_str) else {
                    continue;
                };
                let weight = choice
                    .get("weight")
                    .and_then(toml::Value::as_integer)
                    .unwrap_or(1)
                    .max(1) as u32;
                choices.push(ProceduralItemChoice {
                    name: name.trim().to_string(),
                    weight,
                });
            }
        }

        choices.retain(|choice| !choice.name.is_empty());
        choices
    }

    fn parse_item_choices(
        section: &toml::map::Map<String, toml::Value>,
    ) -> HashMap<String, Vec<ProceduralItemChoice>> {
        let mut choices = HashMap::new();
        let Some(items) = section.get("items").and_then(toml::Value::as_table) else {
            return choices;
        };

        for (kind, value) in items {
            let Some(table) = value.as_table() else {
                continue;
            };
            let kind_choices = Self::parse_weighted_choices(table);

            if !kind_choices.is_empty() {
                choices.insert(kind.trim().to_ascii_lowercase(), kind_choices);
            }
        }

        choices
    }

    fn parse_probability(table: &toml::map::Map<String, toml::Value>) -> f32 {
        let raw = table
            .get("chance")
            .or_else(|| table.get("percentage"))
            .or_else(|| table.get("percent"));
        let value = raw
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

    fn parse_character_rules(
        section: &toml::map::Map<String, toml::Value>,
    ) -> HashMap<String, ProceduralCharacterRule> {
        let mut rules = HashMap::new();
        let Some(characters) = section.get("characters").and_then(toml::Value::as_table) else {
            return rules;
        };

        for (kind, value) in characters {
            let Some(table) = value.as_table() else {
                continue;
            };
            let choices = Self::parse_weighted_choices(table);
            if choices.is_empty() {
                continue;
            }
            rules.insert(
                kind.trim().to_ascii_lowercase(),
                ProceduralCharacterRule {
                    chance: Self::parse_probability(table),
                    choices,
                },
            );
        }

        rules
    }

    fn tile_choices(
        tiles: &IndexMap<Uuid, Tile>,
        style: &str,
    ) -> HashMap<String, Vec<(Uuid, u32)>> {
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
            Self::stamp_room(&mut cells, cfg.width, room);
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
                let Some(candidate) = Self::candidate_next_room(prev, w, h, gap, dir, &mut rng)
                else {
                    continue;
                };

                if !Self::room_in_bounds(cfg, candidate.room)
                    || rooms
                        .iter()
                        .any(|other| candidate.room.intersects(*other, 2))
                    || Self::has_adjacent_door(
                        &cells,
                        cfg.width,
                        cfg.height,
                        candidate.prev_door.0,
                        candidate.prev_door.1,
                    )
                    || !Self::corridor_is_clear(&cells, cfg.width, cfg.height, &candidate.corridor)
                {
                    continue;
                }

                Self::carve_corridor(&mut cells, cfg.width, cfg.height, &candidate.corridor);
                Self::stamp_room(&mut cells, cfg.width, candidate.room);
                cells[(candidate.prev_door.1 * cfg.width + candidate.prev_door.0) as usize] =
                    ProcCell::Floor;
                cells[(candidate.next_door.1 * cfg.width + candidate.next_door.0) as usize] =
                    ProcCell::Floor;

                if (cfg.door_placement == "exits" || cfg.door_placement == "both")
                    && Self::should_place_door(cfg, &mut rng)
                {
                    cells[(candidate.prev_door.1 * cfg.width + candidate.prev_door.0) as usize] =
                        ProcCell::Door;
                }
                if (cfg.door_placement == "entrances" || cfg.door_placement == "both")
                    && Self::should_place_door(cfg, &mut rng)
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
        if let Some((x, y)) = entrance {
            if Self::in_bounds(cfg.width, cfg.height, x, y) {
                cells[(y * cfg.width + x) as usize] = ProcCell::Entrance;
            }
        }
        if let Some((x, y)) = exit {
            if Self::in_bounds(cfg.width, cfg.height, x, y) {
                cells[(y * cfg.width + x) as usize] = ProcCell::Exit;
            }
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
                        if nx >= 0
                            && ny >= 0
                            && nx < cfg.width
                            && ny < cfg.height
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
                    y: prev.y + Self::random_room_offset(rng, max_offset),
                    w,
                    h,
                };
                let next_y = Self::clamp_to_room_inner(anchor_y, room.y, room.h);
                let prev_door = (prev.x + prev.w - 1, anchor_y);
                let next_door = (room.x, next_y);
                let outside0 = (prev.x + prev.w, anchor_y);
                let outside1 = (room.x - 1, next_y);
                Some(RoomCandidate {
                    room,
                    prev_door,
                    next_door,
                    corridor: Self::connector_path(outside0, outside1, rng),
                })
            }
            1 => {
                let anchor_y = rng.random_range(prev.y + 1..=prev.y + prev.h - 2);
                let room = Room {
                    x: prev.x - gap - w,
                    y: prev.y + Self::random_room_offset(rng, max_offset),
                    w,
                    h,
                };
                let next_y = Self::clamp_to_room_inner(anchor_y, room.y, room.h);
                let prev_door = (prev.x, anchor_y);
                let next_door = (room.x + room.w - 1, next_y);
                let outside0 = (prev.x - 1, anchor_y);
                let outside1 = (room.x + room.w, next_y);
                Some(RoomCandidate {
                    room,
                    prev_door,
                    next_door,
                    corridor: Self::connector_path(outside0, outside1, rng),
                })
            }
            2 => {
                let anchor_x = rng.random_range(prev.x + 1..=prev.x + prev.w - 2);
                let room = Room {
                    x: prev.x + Self::random_room_offset(rng, max_offset),
                    y: prev.y + prev.h + gap,
                    w,
                    h,
                };
                let next_x = Self::clamp_to_room_inner(anchor_x, room.x, room.w);
                let prev_door = (anchor_x, prev.y + prev.h - 1);
                let next_door = (next_x, room.y);
                let outside0 = (anchor_x, prev.y + prev.h);
                let outside1 = (next_x, room.y - 1);
                Some(RoomCandidate {
                    room,
                    prev_door,
                    next_door,
                    corridor: Self::connector_path(outside0, outside1, rng),
                })
            }
            3 => {
                let anchor_x = rng.random_range(prev.x + 1..=prev.x + prev.w - 2);
                let room = Room {
                    x: prev.x + Self::random_room_offset(rng, max_offset),
                    y: prev.y - gap - h,
                    w,
                    h,
                };
                let next_x = Self::clamp_to_room_inner(anchor_x, room.x, room.w);
                let prev_door = (anchor_x, prev.y);
                let next_door = (next_x, room.y + room.h - 1);
                let outside0 = (anchor_x, prev.y - 1);
                let outside1 = (next_x, room.y + room.h);
                Some(RoomCandidate {
                    room,
                    prev_door,
                    next_door,
                    corridor: Self::connector_path(outside0, outside1, rng),
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

    fn connector_path(
        outside0: (i32, i32),
        outside1: (i32, i32),
        rng: &mut StdRng,
    ) -> Vec<(i32, i32)> {
        let bend = if outside0.0 == outside1.0 || outside0.1 == outside1.1 {
            outside1
        } else if rng.random_bool(0.5) {
            (outside1.0, outside0.1)
        } else {
            (outside0.0, outside1.1)
        };

        let mut cells = Self::line_cells(outside0, bend);
        let tail = Self::line_cells(bend, outside1);
        cells.extend(tail.into_iter().skip(1));
        cells
    }

    fn room_in_bounds(cfg: &ProceduralConfig, room: Room) -> bool {
        room.x >= 2
            && room.y >= 2
            && room.x + room.w < cfg.width - 2
            && room.y + room.h < cfg.height - 2
    }

    fn corridor_is_clear(
        cells: &[ProcCell],
        width: i32,
        height: i32,
        corridor: &[(i32, i32)],
    ) -> bool {
        for &(x, y) in corridor {
            if !Self::in_bounds(width, height, x, y)
                || cells[(y * width + x) as usize] != ProcCell::Empty
            {
                return false;
            }
        }
        true
    }

    fn has_adjacent_door(cells: &[ProcCell], width: i32, height: i32, x: i32, y: i32) -> bool {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let nx = x + dx;
                let ny = y + dy;
                if Self::in_bounds(width, height, nx, ny)
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
            if Self::in_bounds(width, height, x, y) {
                let idx = (y * width + x) as usize;
                if matches!(cells[idx], ProcCell::Empty | ProcCell::Corridor) {
                    cells[idx] = ProcCell::Corridor;
                }
            }
        }
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

    fn in_bounds(width: i32, height: i32, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < width && y < height
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

    fn cell_world_position(x: i32, y: i32, cfg: &ProceduralConfig) -> Vec3<f32> {
        let ox = -(cfg.width as f32) * 0.5;
        let oy = -(cfg.height as f32) * 0.5;
        Vec3::new(ox + x as f32 + 0.5, 0.0, oy + y as f32 + 0.5)
    }

    fn room_spawn_position(room: Room, cfg: &ProceduralConfig) -> Vec3<f32> {
        let (x, y) = room.center();
        Self::cell_world_position(x, y, cfg)
    }

    fn is_generated_item_instance(item: &Item) -> bool {
        let Ok(parsed) = toml::from_str::<toml::Value>(&item.data) else {
            return false;
        };
        parsed
            .get("procedural")
            .and_then(toml::Value::as_table)
            .and_then(|section| section.get("generated"))
            .and_then(toml::Value::as_bool)
            .unwrap_or(false)
    }

    fn clear_generated_region_items(region: &mut Region) {
        let generated = region
            .items
            .iter()
            .filter_map(|(id, item)| Self::is_generated_item_instance(item).then_some(*id))
            .collect::<Vec<_>>();
        for id in generated {
            region.items.shift_remove(&id);
        }
    }

    fn is_generated_character_instance(character: &Character) -> bool {
        let Ok(parsed) = toml::from_str::<toml::Value>(&character.data) else {
            return false;
        };
        parsed
            .get("procedural")
            .and_then(toml::Value::as_table)
            .and_then(|section| section.get("generated"))
            .and_then(toml::Value::as_bool)
            .unwrap_or(false)
    }

    fn clear_generated_region_characters(region: &mut Region) {
        let generated = region
            .characters
            .iter()
            .filter_map(|(id, character)| {
                Self::is_generated_character_instance(character).then_some(*id)
            })
            .collect::<Vec<_>>();
        for id in generated {
            region.characters.shift_remove(&id);
        }
    }

    fn choose_weighted_choice<'a>(
        choices: &'a [ProceduralItemChoice],
        rng: &mut StdRng,
    ) -> Option<&'a ProceduralItemChoice> {
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

    fn choose_item_template(
        item_templates: &IndexMap<Uuid, Item>,
        cfg: &ProceduralConfig,
        kind: &str,
        rng: &mut StdRng,
    ) -> Option<(Uuid, String)> {
        let choices = cfg.item_choices.get(kind)?;
        let choice = Self::choose_weighted_choice(choices, rng)?;
        item_templates
            .iter()
            .find(|(_, item)| item.name == choice.name)
            .map(|(id, item)| (*id, item.name.clone()))
    }

    fn choose_character_template(
        character_templates: &IndexMap<Uuid, Character>,
        choices: &[ProceduralItemChoice],
        rng: &mut StdRng,
    ) -> Option<(Uuid, String)> {
        let choice = Self::choose_weighted_choice(choices, rng)?;
        character_templates
            .iter()
            .find(|(_, character)| character.name == choice.name)
            .map(|(id, character)| (*id, character.name.clone()))
    }

    fn add_generated_region_items(
        region: &mut Region,
        item_templates: &IndexMap<Uuid, Item>,
        cfg: &ProceduralConfig,
        spawns: Vec<ProceduralItemSpawn>,
    ) {
        let mut rng = StdRng::seed_from_u64(cfg.seed ^ 0x517c_c1b7_b272_220a);
        for spawn in spawns {
            let Some((item_id, name)) =
                Self::choose_item_template(item_templates, cfg, &spawn.kind, &mut rng)
            else {
                continue;
            };
            let mut item = Item {
                item_id,
                name,
                position: spawn.position,
                ..Default::default()
            };
            item.data = format!(
                "[procedural]\ngenerated = true\ngenerator = \"{}\"\nkind = \"{}\"\n",
                cfg.generator, spawn.kind
            );
            region.items.insert(item.id, item);
        }
    }

    fn add_generated_region_characters(
        region: &mut Region,
        character_templates: &IndexMap<Uuid, Character>,
        cfg: &ProceduralConfig,
        spawns: Vec<ProceduralCharacterSpawn>,
    ) {
        let mut rng = StdRng::seed_from_u64(cfg.seed ^ 0x2b63_1f0d_c8bd_842f);
        for spawn in spawns {
            let Some(rule) = cfg.character_rules.get(&spawn.kind) else {
                continue;
            };
            let Some((character_id, name)) =
                Self::choose_character_template(character_templates, &rule.choices, &mut rng)
            else {
                continue;
            };
            let mut character = Character {
                character_id,
                name,
                position: spawn.position,
                ..Default::default()
            };
            character.data = format!(
                "[procedural]\ngenerated = true\ngenerator = \"{}\"\nkind = \"{}\"\n",
                cfg.generator, spawn.kind
            );
            region.characters.insert(character.id, character);
        }
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

    fn bake_connected_rooms(
        map: &mut Map,
        tiles: &IndexMap<Uuid, Tile>,
        cfg: &ProceduralConfig,
    ) -> (Vec<ProceduralItemSpawn>, Vec<ProceduralCharacterSpawn>) {
        Self::clear_map_for_build(map);
        let choices = Self::tile_choices(tiles, &cfg.style);
        let generated = Self::generate_grid(cfg);
        let mut tile_rng = StdRng::seed_from_u64(cfg.seed ^ 0x9e37_79b9_7f4a_7c15);
        let mut character_rng = StdRng::seed_from_u64(cfg.seed ^ 0x6942_758f_2d8f_d7b3);
        let mut item_spawns = Vec::new();
        let mut character_spawns = Vec::new();

        if !cfg.character_rules.is_empty() {
            for (room_index, room) in generated.rooms.iter().enumerate() {
                if room_index == 0 || room_index + 1 == generated.rooms.len() {
                    continue;
                }
                for (kind, rule) in &cfg.character_rules {
                    if rule.chance > 0.0 && character_rng.random::<f32>() <= rule.chance {
                        character_spawns.push(ProceduralCharacterSpawn {
                            kind: kind.clone(),
                            position: Self::room_spawn_position(*room, cfg),
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
                if let Some(tile_id) = Self::choose_tile(&choices, kind, &mut tile_rng) {
                    Self::add_cell_sector(map, x, y, cfg, kind, tile_id, marker_name);
                }
                if matches!(cell, ProcCell::Door) {
                    item_spawns.push(ProceduralItemSpawn {
                        kind: "door".to_string(),
                        position: Self::cell_world_position(x, y, cfg),
                    });
                }
            }
        }

        map.camera = MapCamera::TwoD;
        map.changed = map.changed.wrapping_add(1);
        map.update_surfaces();
        (item_spawns, character_spawns)
    }
}

impl Action for BuildProcedural {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_build_procedural_help"),
        ));
        Self {
            id: TheId::named(&fl!("action_build_procedural")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_build_procedural_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.pc.is_region()
    }

    fn apply_project(
        &self,
        project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        let Some(region_id) = server_ctx.pc.id() else {
            return;
        };
        let Some(region) = project.get_region(&region_id) else {
            return;
        };
        let Some(cfg) = Self::parse_config(&region.config) else {
            eprintln!("Build Procedural: missing [procedural] region settings.");
            return;
        };
        if !cfg.enabled {
            eprintln!("Build Procedural: [procedural].enabled is false.");
            return;
        }
        if cfg.generator != "connected_rooms" {
            eprintln!(
                "Build Procedural: unsupported generator '{}'. Expected 'connected_rooms'.",
                cfg.generator
            );
            return;
        }
        if cfg.mode != "2d" {
            eprintln!(
                "Build Procedural: mode = \"{}\" is not implemented yet.",
                cfg.mode
            );
            return;
        }

        let tiles = project.tiles.clone();
        let item_templates = project.items.clone();
        let character_templates = project.characters.clone();
        let Some(region) = project.get_region_mut(&region_id) else {
            return;
        };
        let old_map = region.map.clone();
        Self::clear_generated_region_items(region);
        Self::clear_generated_region_characters(region);
        let (item_spawns, character_spawns) =
            Self::bake_connected_rooms(&mut region.map, &tiles, &cfg);
        Self::add_generated_region_items(region, &item_templates, &cfg, item_spawns);
        Self::add_generated_region_characters(region, &character_templates, &cfg, character_spawns);
        let new_map = region.map.clone();
        let map_changed = old_map.vertices != new_map.vertices
            || old_map.linedefs != new_map.linedefs
            || old_map.sectors != new_map.sectors;
        if map_changed {
            UNDOMANAGER.write().unwrap().add_undo(
                ProjectUndoAtom::MapEdit(server_ctx.pc, Box::new(old_map), Box::new(new_map)),
                ctx,
            );
        }
        shared::rusterix_utils::insert_content_into_maps(project);
        crate::utils::editor_scene_full_rebuild(project, server_ctx);
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Minimap"),
            TheValue::Empty,
        ));
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _project: &mut Project,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        self.nodeui.handle_event(event)
    }
}
