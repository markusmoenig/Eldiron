use crate::Surface;
use crate::{Map, PixelSource, Value, ValueContainer};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use theframework::prelude::TheColor;
use theframework::prelude::Uuid;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug, Default, Hash)]
pub struct DungeonTileKind(pub u16);

impl DungeonTileKind {
    pub const NORTH: u16 = 1;
    pub const EAST: u16 = 2;
    pub const SOUTH: u16 = 4;
    pub const WEST: u16 = 8;
    pub const DOOR_NORTH: u16 = 16;
    pub const DOOR_EAST: u16 = 32;
    pub const DOOR_SOUTH: u16 = 64;
    pub const DOOR_WEST: u16 = 128;

    pub const FLOOR: Self = Self(0);
    pub const WALL_N: Self = Self(Self::NORTH);
    pub const WALL_E: Self = Self(Self::EAST);
    pub const WALL_S: Self = Self(Self::SOUTH);
    pub const WALL_W: Self = Self(Self::WEST);
    pub const WALL_NS: Self = Self(Self::NORTH | Self::SOUTH);
    pub const WALL_EW: Self = Self(Self::EAST | Self::WEST);
    pub const WALL_NE: Self = Self(Self::NORTH | Self::EAST);
    pub const WALL_ES: Self = Self(Self::EAST | Self::SOUTH);
    pub const WALL_SW: Self = Self(Self::SOUTH | Self::WEST);
    pub const WALL_WN: Self = Self(Self::WEST | Self::NORTH);
    pub const WALL_NES: Self = Self(Self::NORTH | Self::EAST | Self::SOUTH);
    pub const WALL_ESW: Self = Self(Self::EAST | Self::SOUTH | Self::WEST);
    pub const WALL_SWN: Self = Self(Self::SOUTH | Self::WEST | Self::NORTH);
    pub const WALL_WNE: Self = Self(Self::WEST | Self::NORTH | Self::EAST);
    pub const WALL_NESW: Self = Self(Self::NORTH | Self::EAST | Self::SOUTH | Self::WEST);
    pub const DOOR_N: Self = Self(Self::DOOR_NORTH);
    pub const DOOR_E: Self = Self(Self::DOOR_EAST);
    pub const DOOR_S: Self = Self(Self::DOOR_SOUTH);
    pub const DOOR_W: Self = Self(Self::DOOR_WEST);

    pub fn all() -> &'static [DungeonTileKind] {
        &[
            Self::FLOOR,
            Self::WALL_N,
            Self::WALL_E,
            Self::WALL_S,
            Self::WALL_W,
            Self::WALL_NS,
            Self::WALL_EW,
            Self::WALL_NE,
            Self::WALL_ES,
            Self::WALL_SW,
            Self::WALL_WN,
            Self::WALL_NES,
            Self::WALL_ESW,
            Self::WALL_SWN,
            Self::WALL_WNE,
            Self::WALL_NESW,
            Self::DOOR_N,
            Self::DOOR_E,
            Self::DOOR_S,
            Self::DOOR_W,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self.0 {
            0 => "Floor",
            Self::NORTH => "North wall",
            Self::EAST => "East wall",
            Self::SOUTH => "South wall",
            Self::WEST => "West wall",
            Self::DOOR_NORTH => "North door",
            Self::DOOR_EAST => "East door",
            Self::DOOR_SOUTH => "South door",
            Self::DOOR_WEST => "West door",
            x if x == Self::NORTH | Self::SOUTH => "North + South walls",
            x if x == Self::EAST | Self::WEST => "East + West walls",
            x if x == Self::NORTH | Self::EAST => "North + East walls",
            x if x == Self::EAST | Self::SOUTH => "East + South walls",
            x if x == Self::SOUTH | Self::WEST => "South + West walls",
            x if x == Self::WEST | Self::NORTH => "West + North walls",
            x if x == Self::NORTH | Self::EAST | Self::SOUTH => "North + East + South walls",
            x if x == Self::EAST | Self::SOUTH | Self::WEST => "East + South + West walls",
            x if x == Self::SOUTH | Self::WEST | Self::NORTH => "South + West + North walls",
            x if x == Self::WEST | Self::NORTH | Self::EAST => "West + North + East walls",
            _ => "Four walls",
        }
    }

    pub fn has_north(self) -> bool {
        self.0 & Self::NORTH != 0
    }
    pub fn has_east(self) -> bool {
        self.0 & Self::EAST != 0
    }
    pub fn has_south(self) -> bool {
        self.0 & Self::SOUTH != 0
    }
    pub fn has_west(self) -> bool {
        self.0 & Self::WEST != 0
    }
    pub fn has_door_north(self) -> bool {
        self.0 & Self::DOOR_NORTH != 0
    }
    pub fn has_door_east(self) -> bool {
        self.0 & Self::DOOR_EAST != 0
    }
    pub fn has_door_south(self) -> bool {
        self.0 & Self::DOOR_SOUTH != 0
    }
    pub fn has_door_west(self) -> bool {
        self.0 & Self::DOOR_WEST != 0
    }
    pub fn is_door(self) -> bool {
        self.0 & (Self::DOOR_NORTH | Self::DOOR_EAST | Self::DOOR_SOUTH | Self::DOOR_WEST) != 0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DungeonCell {
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub kind: DungeonTileKind,
    #[serde(default)]
    pub floor_base: f32,
    #[serde(default = "default_height", alias = "ceiling_height")]
    pub height: f32,
    #[serde(default)]
    pub standalone: bool,
    #[serde(default = "default_door_width")]
    pub door_width: i32,
    #[serde(default = "default_door_depth")]
    pub door_depth: f32,
    #[serde(default = "default_door_height")]
    pub door_height: f32,
    #[serde(default)]
    pub door_open_mode: i32,
    #[serde(default = "default_door_item")]
    pub door_item: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DungeonLayer {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub floor_base: f32,
    #[serde(default = "default_height", alias = "ceiling_height")]
    pub height: f32,
    #[serde(default)]
    pub cells: Vec<DungeonCell>,
}

impl Default for DungeonLayer {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Layer 0".to_string(),
            floor_base: 0.0,
            height: default_height(),
            cells: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DungeonMap {
    #[serde(default)]
    pub active_layer: usize,
    #[serde(default)]
    pub layers: Vec<DungeonLayer>,
}

impl DungeonMap {
    pub fn active_layer(&self) -> Option<&DungeonLayer> {
        self.layers.get(self.active_layer)
    }

    pub fn active_layer_mut(&mut self) -> Option<&mut DungeonLayer> {
        self.layers.get_mut(self.active_layer)
    }

    pub fn ensure_active_layer_mut(&mut self) -> &mut DungeonLayer {
        if self.layers.is_empty() {
            self.layers.push(DungeonLayer::default());
            self.active_layer = 0;
        } else if self.active_layer >= self.layers.len() {
            self.active_layer = self.layers.len() - 1;
        }
        &mut self.layers[self.active_layer]
    }

    pub fn upsert_active_cell(
        &mut self,
        x: i32,
        y: i32,
        kind: DungeonTileKind,
        floor_base: f32,
        height: f32,
        standalone: bool,
        door_width: i32,
        door_depth: f32,
        door_height: f32,
        door_open_mode: i32,
        door_item: String,
    ) -> bool {
        let layer = self.ensure_active_layer_mut();
        layer.floor_base = floor_base;
        layer.height = height;
        if let Some(cell) = layer
            .cells
            .iter_mut()
            .find(|cell| cell.x == x && cell.y == y)
        {
            if cell.kind == kind
                && (cell.floor_base - floor_base).abs() < 0.0001
                && (cell.height - height).abs() < 0.0001
                && cell.standalone == standalone
                && cell.door_width == door_width
                && (cell.door_depth - door_depth).abs() < 0.0001
                && (cell.door_height - door_height).abs() < 0.0001
                && cell.door_open_mode == door_open_mode
                && cell.door_item == door_item
            {
                return false;
            }
            cell.kind = kind;
            cell.floor_base = floor_base;
            cell.height = height;
            cell.standalone = standalone;
            cell.door_width = door_width;
            cell.door_depth = door_depth;
            cell.door_height = door_height;
            cell.door_open_mode = door_open_mode;
            cell.door_item = door_item;
            true
        } else {
            layer.cells.push(DungeonCell {
                x,
                y,
                kind,
                floor_base,
                height,
                standalone,
                door_width,
                door_depth,
                door_height,
                door_open_mode,
                door_item,
            });
            true
        }
    }

    pub fn remove_active_cell(&mut self, x: i32, y: i32) -> bool {
        let layer = self.ensure_active_layer_mut();
        let prev_len = layer.cells.len();
        layer.cells.retain(|cell| !(cell.x == x && cell.y == y));
        prev_len != layer.cells.len()
    }
}

const fn default_height() -> f32 {
    4.0
}

const fn default_door_width() -> i32 {
    2
}

const fn default_door_depth() -> f32 {
    0.5
}

const fn default_door_height() -> f32 {
    2.25
}

fn default_door_item() -> String {
    "Door Handler".to_string()
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DungeonDoorOpenMode {
    #[default]
    Auto = 0,
    SlideUp = 1,
    SlideDown = 2,
    SlideLeft = 3,
    SlideRight = 4,
    SplitSides = 5,
}

impl DungeonDoorOpenMode {
    pub fn all() -> &'static [&'static str] {
        &[
            "Auto",
            "Slide Up",
            "Slide Down",
            "Slide Left",
            "Slide Right",
            "Split Sides",
        ]
    }

    pub fn from_i32(value: i32) -> Self {
        match value {
            1 => Self::SlideUp,
            2 => Self::SlideDown,
            3 => Self::SlideLeft,
            4 => Self::SlideRight,
            5 => Self::SplitSides,
            _ => Self::Auto,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::SlideUp => "slide_up",
            Self::SlideDown => "slide_down",
            Self::SlideLeft => "slide_left",
            Self::SlideRight => "slide_right",
            Self::SplitSides => "split_sides",
        }
    }
}

const DUNGEON_GENERATOR: &str = "dungeon_tool";
const FLOOR_SOURCE_COLOR: [u8; 3] = [84, 108, 132];
const CEILING_SOURCE_COLOR: [u8; 3] = [108, 108, 108];
const WALL_SOURCE_COLOR: [u8; 3] = [194, 172, 146];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct HeightKey {
    floor_bits: u32,
    height_bits: u32,
    standalone: bool,
}

#[derive(Clone, Copy, Debug)]
struct MergedRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    standalone: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum GeneratedPieceKey {
    Horizontal {
        layer_id: Uuid,
        part: String,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        standalone: bool,
    },
    Wall {
        layer_id: Uuid,
        edge: String,
        start: (i32, i32),
        end: (i32, i32),
        floor_bits: u32,
        height_bits: u32,
        standalone: bool,
    },
}

#[derive(Clone)]
struct PreservedGeneratedPiece {
    sector_properties: ValueContainer,
    surface: Option<Surface>,
}

#[derive(Clone, Debug)]
struct Doorway {
    edge_name: &'static str,
    start: (f32, f32),
    end: (f32, f32),
    floor_base: f32,
    height: f32,
    door_height: f32,
    door_depth: f32,
    open_mode: DungeonDoorOpenMode,
    item: String,
    standalone: bool,
}

pub fn rebuild_generated_geometry(map: &mut Map, create_floor: bool, create_ceiling: bool) {
    let preserved = collect_preserved_generated_pieces(map);
    delete_generated_geometry(map);

    let Some(layer) = map.dungeon.active_layer().cloned() else {
        return;
    };

    let rectangles = merge_cell_rectangles(&layer.cells);

    if create_floor {
        for (height_key, rects) in &rectangles {
            for rect in rects {
                generate_horizontal_sector(
                    map,
                    layer.id,
                    *rect,
                    f32::from_bits(height_key.floor_bits),
                    "floor",
                    PixelSource::Color(TheColor::from_u8_array_3(FLOOR_SOURCE_COLOR)),
                    rect.standalone,
                    preserved.get(&GeneratedPieceKey::Horizontal {
                        layer_id: layer.id,
                        part: "floor".to_string(),
                        x: rect.x,
                        y: rect.y,
                        width: rect.width,
                        height: rect.height,
                        standalone: rect.standalone,
                    }),
                );
            }
        }
    }

    if create_ceiling {
        for (height_key, rects) in &rectangles {
            for rect in rects {
                generate_horizontal_sector(
                    map,
                    layer.id,
                    *rect,
                    f32::from_bits(height_key.floor_bits) + f32::from_bits(height_key.height_bits),
                    "ceiling",
                    PixelSource::Color(TheColor::from_u8_array_3(CEILING_SOURCE_COLOR)),
                    rect.standalone,
                    preserved.get(&GeneratedPieceKey::Horizontal {
                        layer_id: layer.id,
                        part: "ceiling".to_string(),
                        x: rect.x,
                        y: rect.y,
                        width: rect.width,
                        height: rect.height,
                        standalone: rect.standalone,
                    }),
                );
            }
        }
    }

    for wall_strip in collect_wall_strips(&layer.cells) {
        generate_wall_sector(
            map,
            layer.id,
            wall_strip.edge_name,
            wall_strip.start,
            wall_strip.end,
            wall_strip.floor_base,
            wall_strip.height,
            wall_strip.standalone,
            preserved.get(&GeneratedPieceKey::Wall {
                layer_id: layer.id,
                edge: wall_strip.edge_name.to_string(),
                start: (
                    (wall_strip.start.0 * 1000.0).round() as i32,
                    (wall_strip.start.1 * 1000.0).round() as i32,
                ),
                end: (
                    (wall_strip.end.0 * 1000.0).round() as i32,
                    (wall_strip.end.1 * 1000.0).round() as i32,
                ),
                floor_bits: wall_strip.floor_base.to_bits(),
                height_bits: wall_strip.height.to_bits(),
                standalone: wall_strip.standalone,
            }),
        );
    }

    for doorway in collect_doorways(&layer.cells) {
        generate_doorway_geometry(map, layer.id, doorway);
    }

    map.sanitize();
    map.update_surfaces();
}

fn delete_generated_geometry(map: &mut Map) {
    let sector_ids: Vec<u32> = map
        .sectors
        .iter()
        .filter(|sector| {
            sector
                .properties
                .get_str_default("generated_by", String::new())
                == DUNGEON_GENERATOR
        })
        .map(|sector| sector.id)
        .collect();

    if !sector_ids.is_empty() {
        map.delete_elements(&[], &[], &sector_ids);
    }
}

fn collect_preserved_generated_pieces(
    map: &Map,
) -> HashMap<GeneratedPieceKey, PreservedGeneratedPiece> {
    let mut preserved = HashMap::new();

    for sector in &map.sectors {
        if sector
            .properties
            .get_str_default("generated_by", String::new())
            != DUNGEON_GENERATOR
        {
            continue;
        }

        let Some(key) = generated_piece_key_from_sector(sector) else {
            continue;
        };
        let surface = map.get_surface_for_sector_id(sector.id).cloned();
        preserved.insert(
            key,
            PreservedGeneratedPiece {
                sector_properties: sector.properties.clone(),
                surface,
            },
        );
    }

    preserved
}

fn generated_piece_key_from_sector(sector: &crate::Sector) -> Option<GeneratedPieceKey> {
    let layer_id = sector.properties.get_id("dungeon_layer_id")?;
    let part = sector
        .properties
        .get_str_default("dungeon_part", String::new());
    let standalone = sector
        .properties
        .get_bool_default("dungeon_standalone", false);

    if part == "floor" || part == "ceiling" {
        Some(GeneratedPieceKey::Horizontal {
            layer_id,
            part,
            x: sector.properties.get_int("dungeon_x")?,
            y: sector.properties.get_int("dungeon_y")?,
            width: sector.properties.get_int("dungeon_width")?,
            height: sector.properties.get_int("dungeon_height")?,
            standalone,
        })
    } else if part == "wall" {
        let edge = sector
            .properties
            .get_str_default("dungeon_edge", String::new());
        Some(GeneratedPieceKey::Wall {
            layer_id,
            edge,
            start: (
                sector.properties.get_int("dungeon_start_x")?,
                sector.properties.get_int("dungeon_start_y")?,
            ),
            end: (
                sector.properties.get_int("dungeon_end_x")?,
                sector.properties.get_int("dungeon_end_y")?,
            ),
            floor_bits: sector.properties.get_float("floor_base")?.to_bits(),
            height_bits: sector.properties.get_float("height")?.to_bits(),
            standalone,
        })
    } else {
        None
    }
}

fn merge_cell_rectangles(cells: &[DungeonCell]) -> BTreeMap<HeightKey, Vec<MergedRect>> {
    let mut grouped: BTreeMap<HeightKey, HashSet<(i32, i32)>> = BTreeMap::new();
    for cell in cells {
        grouped
            .entry(HeightKey {
                floor_bits: cell.floor_base.to_bits(),
                height_bits: cell.height.to_bits(),
                standalone: cell.standalone,
            })
            .or_default()
            .insert((cell.x, cell.y));
    }

    let mut merged = BTreeMap::new();
    for (key, positions) in grouped {
        let mut remaining = positions;
        let mut rects = Vec::new();

        while let Some(&(x, y)) = remaining.iter().min_by_key(|&&(x, y)| (y, x)) {
            let (width, height) = if key.standalone {
                (1, 1)
            } else {
                let mut width = 1;
                while remaining.contains(&(x + width, y)) {
                    width += 1;
                }

                let mut height = 1;
                'rows: loop {
                    let next_y = y + height;
                    for dx in 0..width {
                        if !remaining.contains(&(x + dx, next_y)) {
                            break 'rows;
                        }
                    }
                    height += 1;
                }
                (width, height)
            };

            for dy in 0..height {
                for dx in 0..width {
                    remaining.remove(&(x + dx, y + dy));
                }
            }

            rects.push(MergedRect {
                x,
                y,
                width,
                height,
                standalone: key.standalone,
            });
        }

        merged.insert(key, rects);
    }

    merged
}

#[derive(Clone, Copy, Debug)]
struct WallStrip {
    edge_name: &'static str,
    start: (f32, f32),
    end: (f32, f32),
    floor_base: f32,
    height: f32,
    standalone: bool,
}

fn collect_wall_strips(cells: &[DungeonCell]) -> Vec<WallStrip> {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct WallStripKey {
        edge_name: &'static str,
        coord: i32,
        floor_bits: u32,
        height_bits: u32,
        standalone: bool,
    }

    let mut horizontal: BTreeMap<WallStripKey, BTreeSet<i32>> = BTreeMap::new();
    let mut vertical: BTreeMap<WallStripKey, BTreeSet<i32>> = BTreeMap::new();

    for cell in cells {
        let wall_key = |edge_name, coord| WallStripKey {
            edge_name,
            coord,
            floor_bits: cell.floor_base.to_bits(),
            height_bits: cell.height.to_bits(),
            standalone: cell.standalone,
        };

        if cell.kind.has_north() {
            horizontal
                .entry(wall_key("north", cell.y))
                .or_default()
                .insert(cell.x);
        }
        if cell.kind.has_south() {
            horizontal
                .entry(wall_key("south", cell.y + 1))
                .or_default()
                .insert(cell.x);
        }
        if cell.kind.has_west() {
            vertical
                .entry(wall_key("west", cell.x))
                .or_default()
                .insert(cell.y);
        }
        if cell.kind.has_east() {
            vertical
                .entry(wall_key("east", cell.x + 1))
                .or_default()
                .insert(cell.y);
        }
    }

    let mut strips = Vec::new();
    for (key, starts) in horizontal {
        append_wall_runs(
            &mut strips,
            key.edge_name,
            key.coord,
            starts,
            true,
            f32::from_bits(key.floor_bits),
            f32::from_bits(key.height_bits),
            key.standalone,
        );
    }
    for (key, starts) in vertical {
        append_wall_runs(
            &mut strips,
            key.edge_name,
            key.coord,
            starts,
            false,
            f32::from_bits(key.floor_bits),
            f32::from_bits(key.height_bits),
            key.standalone,
        );
    }
    strips
}

fn collect_doorways(cells: &[DungeonCell]) -> Vec<Doorway> {
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct DoorKey {
        edge_name: &'static str,
        coord: i32,
        floor_bits: u32,
        height_bits: u32,
        door_height_bits: u32,
        door_depth_bits: u32,
        open_mode: i32,
        item: String,
        standalone: bool,
    }

    let mut horizontal: BTreeMap<DoorKey, BTreeSet<i32>> = BTreeMap::new();
    let mut vertical: BTreeMap<DoorKey, BTreeSet<i32>> = BTreeMap::new();

    for cell in cells {
        let key = |edge_name, coord| DoorKey {
            edge_name,
            coord,
            floor_bits: cell.floor_base.to_bits(),
            height_bits: cell.height.to_bits(),
            door_height_bits: cell.door_height.to_bits(),
            door_depth_bits: cell.door_depth.to_bits(),
            open_mode: cell.door_open_mode,
            item: cell.door_item.clone(),
            standalone: cell.standalone,
        };
        if cell.kind.has_door_north() {
            horizontal
                .entry(key("north", cell.y))
                .or_default()
                .insert(cell.x);
        }
        if cell.kind.has_door_south() {
            horizontal
                .entry(key("south", cell.y + 1))
                .or_default()
                .insert(cell.x);
        }
        if cell.kind.has_door_west() {
            vertical
                .entry(key("west", cell.x))
                .or_default()
                .insert(cell.y);
        }
        if cell.kind.has_door_east() {
            vertical
                .entry(key("east", cell.x + 1))
                .or_default()
                .insert(cell.y);
        }
    }

    let mut doorways = Vec::new();
    for (key, starts) in horizontal {
        append_door_runs(
            &mut doorways,
            key.edge_name,
            key.coord,
            key.floor_bits,
            key.height_bits,
            key.door_height_bits,
            key.door_depth_bits,
            key.open_mode,
            key.item.clone(),
            key.standalone,
            starts,
            true,
        );
    }
    for (key, starts) in vertical {
        append_door_runs(
            &mut doorways,
            key.edge_name,
            key.coord,
            key.floor_bits,
            key.height_bits,
            key.door_height_bits,
            key.door_depth_bits,
            key.open_mode,
            key.item.clone(),
            key.standalone,
            starts,
            false,
        );
    }
    doorways
}

fn append_door_runs(
    doorways: &mut Vec<Doorway>,
    edge_name: &'static str,
    coord: i32,
    floor_bits: u32,
    height_bits: u32,
    door_height_bits: u32,
    door_depth_bits: u32,
    open_mode: i32,
    item: String,
    standalone: bool,
    starts: BTreeSet<i32>,
    horizontal: bool,
) {
    let mut iter = starts.into_iter();
    let Some(mut run_start) = iter.next() else {
        return;
    };
    let mut run_end = run_start + 1;
    for start in iter {
        if !standalone && start == run_end {
            run_end += 1;
        } else {
            doorways.push(build_doorway(
                edge_name,
                coord,
                run_start,
                run_end,
                horizontal,
                f32::from_bits(floor_bits),
                f32::from_bits(height_bits),
                f32::from_bits(door_height_bits),
                f32::from_bits(door_depth_bits),
                DungeonDoorOpenMode::from_i32(open_mode),
                item.clone(),
                standalone,
            ));
            run_start = start;
            run_end = start + 1;
        }
    }
    doorways.push(build_doorway(
        edge_name,
        coord,
        run_start,
        run_end,
        horizontal,
        f32::from_bits(floor_bits),
        f32::from_bits(height_bits),
        f32::from_bits(door_height_bits),
        f32::from_bits(door_depth_bits),
        DungeonDoorOpenMode::from_i32(open_mode),
        item,
        standalone,
    ));
}

fn build_doorway(
    edge_name: &'static str,
    coord: i32,
    run_start: i32,
    run_end: i32,
    horizontal: bool,
    floor_base: f32,
    height: f32,
    door_height: f32,
    door_depth: f32,
    open_mode: DungeonDoorOpenMode,
    item: String,
    standalone: bool,
) -> Doorway {
    let (start, end) = if horizontal {
        (
            (run_start as f32, coord as f32),
            (run_end as f32, coord as f32),
        )
    } else {
        (
            (coord as f32, run_start as f32),
            (coord as f32, run_end as f32),
        )
    };
    Doorway {
        edge_name,
        start,
        end,
        floor_base,
        height,
        door_height,
        door_depth,
        open_mode,
        item,
        standalone,
    }
}

fn append_wall_runs(
    strips: &mut Vec<WallStrip>,
    edge_name: &'static str,
    coord: i32,
    starts: BTreeSet<i32>,
    horizontal: bool,
    floor_base: f32,
    height: f32,
    standalone: bool,
) {
    let mut iter = starts.into_iter();
    let Some(mut run_start) = iter.next() else {
        return;
    };
    let mut run_end = run_start + 1;

    for start in iter {
        if !standalone && start == run_end {
            run_end += 1;
        } else {
            strips.push(build_wall_strip(
                edge_name, coord, run_start, run_end, horizontal, floor_base, height, standalone,
            ));
            run_start = start;
            run_end = start + 1;
        }
    }

    strips.push(build_wall_strip(
        edge_name, coord, run_start, run_end, horizontal, floor_base, height, standalone,
    ));
}

fn build_wall_strip(
    edge_name: &'static str,
    coord: i32,
    run_start: i32,
    run_end: i32,
    horizontal: bool,
    floor_base: f32,
    height: f32,
    standalone: bool,
) -> WallStrip {
    let (start, end) = if horizontal {
        (
            (run_start as f32, coord as f32),
            (run_end as f32, coord as f32),
        )
    } else {
        (
            (coord as f32, run_start as f32),
            (coord as f32, run_end as f32),
        )
    };

    WallStrip {
        edge_name,
        start,
        end,
        floor_base,
        height,
        standalone,
    }
}

fn generate_horizontal_sector(
    map: &mut Map,
    layer_id: Uuid,
    rect: MergedRect,
    z: f32,
    part: &str,
    source: PixelSource,
    standalone: bool,
    preserved: Option<&PreservedGeneratedPiece>,
) {
    let x0 = rect.x as f32;
    let y0 = rect.y as f32;
    let x1 = (rect.x + rect.width) as f32;
    let y1 = (rect.y + rect.height) as f32;

    let v0 = map.add_vertex_at_3d(x0, y0, z, false);
    let v1 = map.add_vertex_at_3d(x1, y0, z, false);
    let v2 = map.add_vertex_at_3d(x1, y1, z, false);
    let v3 = map.add_vertex_at_3d(x0, y1, z, false);

    map.possible_polygon.clear();
    let _l0 = map.create_linedef_manual(v0, v1);
    let _l1 = map.create_linedef_manual(v1, v2);
    let _l2 = map.create_linedef_manual(v2, v3);
    let _l3 = map.create_linedef_manual(v3, v0);
    let sector_id = map.close_polygon_manual();
    map.possible_polygon.clear();

    let Some(sector_id) = sector_id else {
        return;
    };

    let had_preserved = preserved.is_some();
    if let Some(sector) = map.find_sector_mut(sector_id) {
        if let Some(preserved) = preserved {
            sector.properties = preserved.sector_properties.clone();
        }
        if !had_preserved {
            sector
                .properties
                .set("source", Value::Source(source.clone()));
            sector
                .properties
                .set("floor_source", Value::Source(source.clone()));
        }
        sector
            .properties
            .set("generated_by", Value::Str(DUNGEON_GENERATOR.to_string()));
        sector
            .properties
            .set("dungeon_layer_id", Value::Id(layer_id));
        sector
            .properties
            .set("dungeon_part", Value::Str(part.to_string()));
        sector.properties.set("dungeon_x", Value::Int(rect.x));
        sector.properties.set("dungeon_y", Value::Int(rect.y));
        sector
            .properties
            .set("dungeon_width", Value::Int(rect.width));
        sector
            .properties
            .set("dungeon_height", Value::Int(rect.height));
        sector
            .properties
            .set("dungeon_standalone", Value::Bool(standalone));
        sector.properties.set("floor_height", Value::Float(z));
        sector.properties.set("floor_base", Value::Float(z));
        sector.properties.set("visible", Value::Bool(true));
    }

    let mut surface = preserved
        .and_then(|preserved| preserved.surface.clone())
        .unwrap_or_else(|| Surface::new(sector_id));
    surface.sector_id = sector_id;
    surface.calculate_geometry(map);
    map.surfaces.insert(surface.id, surface);
}

fn generate_wall_sector(
    map: &mut Map,
    layer_id: Uuid,
    edge_name: &str,
    start: (f32, f32),
    end: (f32, f32),
    floor_base: f32,
    height: f32,
    standalone: bool,
    preserved: Option<&PreservedGeneratedPiece>,
) {
    let z0 = floor_base;
    let z1 = (floor_base + height).max(floor_base + 0.1);
    let v0 = map.add_vertex_at_3d(start.0, start.1, z0, false);
    let v1 = map.add_vertex_at_3d(end.0, end.1, z0, false);
    let v2 = map.add_vertex_at_3d(end.0, end.1, z1, false);
    let v3 = map.add_vertex_at_3d(start.0, start.1, z1, false);

    map.possible_polygon.clear();
    let _ = map.create_linedef_manual(v0, v1);
    let _ = map.create_linedef_manual(v1, v2);
    let _ = map.create_linedef_manual(v2, v3);
    let _ = map.create_linedef_manual(v3, v0);
    let sector_id = map.close_polygon_manual();
    map.possible_polygon.clear();

    let Some(sector_id) = sector_id else {
        return;
    };

    let had_preserved = preserved.is_some();
    if let Some(sector) = map.find_sector_mut(sector_id) {
        if let Some(preserved) = preserved {
            sector.properties = preserved.sector_properties.clone();
        }
        if !had_preserved {
            sector.properties.set(
                "source",
                Value::Source(PixelSource::Color(TheColor::from_u8_array_3(
                    WALL_SOURCE_COLOR,
                ))),
            );
        }
        sector
            .properties
            .set("generated_by", Value::Str(DUNGEON_GENERATOR.to_string()));
        sector
            .properties
            .set("dungeon_layer_id", Value::Id(layer_id));
        sector
            .properties
            .set("dungeon_edge", Value::Str(edge_name.to_string()));
        sector.properties.set(
            "dungeon_start_x",
            Value::Int((start.0 * 1000.0).round() as i32),
        );
        sector.properties.set(
            "dungeon_start_y",
            Value::Int((start.1 * 1000.0).round() as i32),
        );
        sector
            .properties
            .set("dungeon_end_x", Value::Int((end.0 * 1000.0).round() as i32));
        sector
            .properties
            .set("dungeon_end_y", Value::Int((end.1 * 1000.0).round() as i32));
        sector
            .properties
            .set("dungeon_part", Value::Str("wall".to_string()));
        sector
            .properties
            .set("floor_base", Value::Float(floor_base));
        sector.properties.set("height", Value::Float(height));
        sector
            .properties
            .set("dungeon_standalone", Value::Bool(standalone));
        sector
            .properties
            .set("ceiling_height", Value::Float(floor_base + height));
        sector.properties.set("visible", Value::Bool(true));
    }

    let mut surface = preserved
        .and_then(|preserved| preserved.surface.clone())
        .unwrap_or_else(|| Surface::new(sector_id));
    surface.sector_id = sector_id;
    surface.calculate_geometry(map);
    map.surfaces.insert(surface.id, surface);
}

fn generate_doorway_geometry(map: &mut Map, layer_id: Uuid, doorway: Doorway) {
    let door_height = doorway.door_height.clamp(0.5, doorway.height.max(0.5));
    let panel_top = doorway.floor_base + door_height;
    let ceiling = doorway.floor_base + doorway.height;
    let depth = doorway.door_depth.max(0.05);
    let (dx, dy) = match doorway.edge_name {
        "north" => (0.0, depth / 2.0),
        "south" => (0.0, -depth / 2.0),
        "west" => (depth / 2.0, 0.0),
        "east" => (-depth / 2.0, 0.0),
        _ => (0.0, 0.0),
    };

    generate_wall_sector(
        map,
        layer_id,
        doorway.edge_name,
        (doorway.start.0 + dx, doorway.start.1 + dy),
        (doorway.end.0 + dx, doorway.end.1 + dy),
        doorway.floor_base,
        door_height,
        doorway.standalone,
        None,
    );
    if let Some(sector) = map.sectors.last_mut() {
        sector
            .properties
            .set("dungeon_part", Value::Str("door_panel".to_string()));
        if doorway.item.is_empty() {
            sector.properties.remove("item");
        } else {
            sector
                .properties
                .set("item", Value::Str(doorway.item.clone()));
        }
        sector.properties.set(
            "dungeon_door_mode",
            Value::Str(doorway.open_mode.as_str().to_string()),
        );
        sector
            .properties
            .set("dungeon_door_depth", Value::Float(depth));
        sector
            .properties
            .set("dungeon_door_height", Value::Float(door_height));
        sector.properties.set("blocking", Value::Bool(true));
    }

    if ceiling > panel_top + 0.05 {
        generate_wall_sector(
            map,
            layer_id,
            doorway.edge_name,
            doorway.start,
            doorway.end,
            panel_top,
            ceiling - panel_top,
            doorway.standalone,
            None,
        );
        if let Some(sector) = map.sectors.last_mut() {
            sector
                .properties
                .set("dungeon_part", Value::Str("door_lintel".to_string()));
            sector.properties.set(
                "dungeon_door_mode",
                Value::Str(doorway.open_mode.as_str().to_string()),
            );
        }
    }
}
