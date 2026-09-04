use crate::prelude::*;
use std::sync::LazyLock;

pub const BLOCK_OPERATION_PLACE: i32 = 0;
pub const BLOCK_OPERATION_REPLACE: i32 = 1;
pub const BLOCK_OPERATION_ERASE: i32 = 2;
pub const BLOCK_STROKE_LINE: i32 = 0;
pub const BLOCK_STROKE_RECT: i32 = 1;
pub const DEFAULT_BLOCK_HEIGHT_CELLS: i32 = 2;
pub const DEFAULT_BLOCK_SPAN_EXTRA_CELLS: f32 = 0.0;
pub const DEFAULT_BLOCK_DEPTH_EXTRA_CELLS: f32 = 0.0;
pub const BLOCK_SIZE_STEP_CELLS: f32 = 0.25;
pub const BLOCK_COLUMN_SEGMENTS: usize = 12;
const FURNITURE_WOOD: [u8; 4] = [112, 69, 38, 255];
const FURNITURE_DARK_WOOD: [u8; 4] = [73, 42, 25, 255];
const FURNITURE_IRON: [u8; 4] = [58, 61, 64, 255];

pub fn localized_block_asset_name(asset: &BlockAsset) -> String {
    match asset.name_key {
        "block_asset_floor_slab" => fl!("block_asset_floor_slab"),
        "block_asset_floor_wall" => fl!("block_asset_floor_wall"),
        "block_asset_floor_wall_ceiling" => fl!("block_asset_floor_wall_ceiling"),
        "block_asset_floor_corner" => fl!("block_asset_floor_corner"),
        "block_asset_floor_doorway" => fl!("block_asset_floor_doorway"),
        "block_asset_stairs" => fl!("block_asset_stairs"),
        "block_asset_wall" => fl!("block_asset_wall"),
        "block_asset_doorway" => fl!("block_asset_doorway"),
        "block_asset_ceiling_slab" => fl!("block_asset_ceiling_slab"),
        "block_asset_full_block" => fl!("block_asset_full_block"),
        "block_asset_large_block" => fl!("block_asset_large_block"),
        "block_asset_column" => fl!("block_asset_column"),
        "block_asset_plain_column" => fl!("block_asset_plain_column"),
        "block_asset_table" => fl!("block_asset_table"),
        _ => asset.name.to_string(),
    }
}

pub fn localized_block_asset_description(asset: &BlockAsset) -> String {
    match asset.description_key {
        "block_asset_floor_slab_desc" => fl!("block_asset_floor_slab_desc"),
        "block_asset_floor_wall_desc" => fl!("block_asset_floor_wall_desc"),
        "block_asset_floor_wall_ceiling_desc" => fl!("block_asset_floor_wall_ceiling_desc"),
        "block_asset_floor_corner_desc" => fl!("block_asset_floor_corner_desc"),
        "block_asset_floor_doorway_desc" => fl!("block_asset_floor_doorway_desc"),
        "block_asset_stairs_desc" => fl!("block_asset_stairs_desc"),
        "block_asset_wall_desc" => fl!("block_asset_wall_desc"),
        "block_asset_doorway_desc" => fl!("block_asset_doorway_desc"),
        "block_asset_ceiling_slab_desc" => fl!("block_asset_ceiling_slab_desc"),
        "block_asset_full_block_desc" => fl!("block_asset_full_block_desc"),
        "block_asset_large_block_desc" => fl!("block_asset_large_block_desc"),
        "block_asset_column_desc" => fl!("block_asset_column_desc"),
        "block_asset_plain_column_desc" => fl!("block_asset_plain_column_desc"),
        "block_asset_table_desc" => fl!("block_asset_table_desc"),
        _ => asset.description.to_string(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BlockComponentKind {
    Solid,
    Floor,
    Ceiling,
    Wall,
    Column,
    ColumnBase,
    ColumnShaft,
    ColumnCapital,
    DoorPostLeft,
    DoorPostRight,
    DoorLintel,
    Stair,
    TableTop,
    TableLegLeftFront,
    TableLegRightFront,
    TableLegLeftBack,
    TableLegRightBack,
}

#[derive(Clone, Copy)]
pub struct BlockSizing {
    pub height_cells: i32,
    pub span_extra_cells: f32,
    pub depth_extra_cells: f32,
}

impl Default for BlockSizing {
    fn default() -> Self {
        Self {
            height_cells: DEFAULT_BLOCK_HEIGHT_CELLS,
            span_extra_cells: DEFAULT_BLOCK_SPAN_EXTRA_CELLS,
            depth_extra_cells: DEFAULT_BLOCK_DEPTH_EXTRA_CELLS,
        }
    }
}

#[derive(Clone, Copy)]
pub struct BlockBox {
    pub min: Vec3<f32>,
    pub max: Vec3<f32>,
}

#[derive(Clone, Copy)]
pub struct BlockAsset {
    pub id: Uuid,
    pub name: &'static str,
    pub name_key: &'static str,
    pub description: &'static str,
    pub description_key: &'static str,
    pub footprint: Vec3<i32>,
    pub boxes: &'static [BlockBox],
    pub components: &'static [BlockComponentKind],
}

pub fn default_block_asset_id() -> Uuid {
    Uuid::from_u128(0xB10C_0000_0000_0000_0000_0000_0000_0008)
}

const FULL_BLOCK_BOXES: &[BlockBox] = &[BlockBox {
    min: Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    },
    max: Vec3 {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    },
}];

const LARGE_BLOCK_BOXES: &[BlockBox] = &[BlockBox {
    min: Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    },
    max: Vec3 {
        x: 2.0,
        y: 2.0,
        z: 2.0,
    },
}];

const FLOOR_SLAB_BOXES: &[BlockBox] = &[BlockBox {
    min: Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    },
    max: Vec3 {
        x: 1.0,
        y: 0.12,
        z: 1.0,
    },
}];

const CEILING_SLAB_BOXES: &[BlockBox] = &[BlockBox {
    min: Vec3 {
        x: 0.0,
        y: 1.88,
        z: 0.0,
    },
    max: Vec3 {
        x: 1.0,
        y: 2.0,
        z: 1.0,
    },
}];

const WALL_BOXES: &[BlockBox] = &[BlockBox {
    min: Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.82,
    },
    max: Vec3 {
        x: 1.0,
        y: 2.0,
        z: 1.0,
    },
}];

const COLUMN_BOXES: &[BlockBox] = &[
    BlockBox {
        min: Vec3 {
            x: 0.16,
            y: 0.0,
            z: 0.16,
        },
        max: Vec3 {
            x: 0.84,
            y: 0.14,
            z: 0.84,
        },
    },
    BlockBox {
        min: Vec3 {
            x: 0.24,
            y: 0.14,
            z: 0.24,
        },
        max: Vec3 {
            x: 0.76,
            y: 0.28,
            z: 0.76,
        },
    },
    BlockBox {
        min: Vec3 {
            x: 0.34,
            y: 0.28,
            z: 0.34,
        },
        max: Vec3 {
            x: 0.66,
            y: 1.72,
            z: 0.66,
        },
    },
    BlockBox {
        min: Vec3 {
            x: 0.24,
            y: 1.72,
            z: 0.24,
        },
        max: Vec3 {
            x: 0.76,
            y: 1.86,
            z: 0.76,
        },
    },
    BlockBox {
        min: Vec3 {
            x: 0.12,
            y: 1.86,
            z: 0.12,
        },
        max: Vec3 {
            x: 0.88,
            y: 2.0,
            z: 0.88,
        },
    },
];

const PLAIN_COLUMN_BOXES: &[BlockBox] = &[BlockBox {
    min: Vec3 {
        x: 0.28,
        y: 0.0,
        z: 0.28,
    },
    max: Vec3 {
        x: 0.72,
        y: 2.0,
        z: 0.72,
    },
}];

const TABLE_BOXES: &[BlockBox] = &[
    BlockBox {
        min: Vec3 {
            x: 0.0,
            y: 1.84,
            z: 0.0,
        },
        max: Vec3 {
            x: 2.0,
            y: 2.0,
            z: 1.0,
        },
    },
    BlockBox {
        min: Vec3 {
            x: 0.10,
            y: 0.0,
            z: 0.10,
        },
        max: Vec3 {
            x: 0.24,
            y: 1.84,
            z: 0.24,
        },
    },
    BlockBox {
        min: Vec3 {
            x: 1.76,
            y: 0.0,
            z: 0.10,
        },
        max: Vec3 {
            x: 1.90,
            y: 1.84,
            z: 0.24,
        },
    },
    BlockBox {
        min: Vec3 {
            x: 0.10,
            y: 0.0,
            z: 0.76,
        },
        max: Vec3 {
            x: 0.24,
            y: 1.84,
            z: 0.90,
        },
    },
    BlockBox {
        min: Vec3 {
            x: 1.76,
            y: 0.0,
            z: 0.76,
        },
        max: Vec3 {
            x: 1.90,
            y: 1.84,
            z: 0.90,
        },
    },
];

const DOORWAY_BOXES: &[BlockBox] = &[
    BlockBox {
        min: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.82,
        },
        max: Vec3 {
            x: 0.28,
            y: 2.0,
            z: 1.0,
        },
    },
    BlockBox {
        min: Vec3 {
            x: 1.72,
            y: 0.0,
            z: 0.82,
        },
        max: Vec3 {
            x: 2.0,
            y: 2.0,
            z: 1.0,
        },
    },
    BlockBox {
        min: Vec3 {
            x: 0.0,
            y: 1.55,
            z: 0.82,
        },
        max: Vec3 {
            x: 2.0,
            y: 2.0,
            z: 1.0,
        },
    },
];

const FLOOR_WALL_BOXES: &[BlockBox] = &[FLOOR_SLAB_BOXES[0], WALL_BOXES[0]];

const FLOOR_WALL_CEILING_BOXES: &[BlockBox] =
    &[FLOOR_SLAB_BOXES[0], WALL_BOXES[0], CEILING_SLAB_BOXES[0]];

const FLOOR_CORNER_BOXES: &[BlockBox] = &[
    FLOOR_SLAB_BOXES[0],
    WALL_BOXES[0],
    BlockBox {
        min: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        max: Vec3 {
            x: 0.18,
            y: 2.0,
            z: 1.0,
        },
    },
];

const FLOOR_DOORWAY_BOXES: &[BlockBox] = &[
    BlockBox {
        min: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        max: Vec3 {
            x: 2.0,
            y: 0.12,
            z: 1.0,
        },
    },
    DOORWAY_BOXES[0],
    DOORWAY_BOXES[1],
    DOORWAY_BOXES[2],
];

const STAIRS_BOXES: &[BlockBox] = &[
    BlockBox {
        min: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        max: Vec3 {
            x: 1.0,
            y: 0.25,
            z: 0.25,
        },
    },
    BlockBox {
        min: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.25,
        },
        max: Vec3 {
            x: 1.0,
            y: 0.5,
            z: 0.5,
        },
    },
    BlockBox {
        min: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.5,
        },
        max: Vec3 {
            x: 1.0,
            y: 0.75,
            z: 0.75,
        },
    },
    BlockBox {
        min: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.75,
        },
        max: Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
    },
];

const FULL_BLOCK_COMPONENTS: &[BlockComponentKind] = &[BlockComponentKind::Solid];
const LARGE_BLOCK_COMPONENTS: &[BlockComponentKind] = &[BlockComponentKind::Solid];
const FLOOR_SLAB_COMPONENTS: &[BlockComponentKind] = &[BlockComponentKind::Floor];
const CEILING_SLAB_COMPONENTS: &[BlockComponentKind] = &[BlockComponentKind::Ceiling];
const WALL_COMPONENTS: &[BlockComponentKind] = &[BlockComponentKind::Wall];
const COLUMN_COMPONENTS: &[BlockComponentKind] = &[
    BlockComponentKind::ColumnBase,
    BlockComponentKind::ColumnBase,
    BlockComponentKind::ColumnShaft,
    BlockComponentKind::ColumnCapital,
    BlockComponentKind::ColumnCapital,
];
const PLAIN_COLUMN_COMPONENTS: &[BlockComponentKind] = &[BlockComponentKind::ColumnShaft];
const TABLE_COMPONENTS: &[BlockComponentKind] = &[
    BlockComponentKind::TableTop,
    BlockComponentKind::TableLegLeftFront,
    BlockComponentKind::TableLegRightFront,
    BlockComponentKind::TableLegLeftBack,
    BlockComponentKind::TableLegRightBack,
];
const DOORWAY_COMPONENTS: &[BlockComponentKind] = &[
    BlockComponentKind::DoorPostLeft,
    BlockComponentKind::DoorPostRight,
    BlockComponentKind::DoorLintel,
];
const FLOOR_WALL_COMPONENTS: &[BlockComponentKind] =
    &[BlockComponentKind::Floor, BlockComponentKind::Wall];
const FLOOR_WALL_CEILING_COMPONENTS: &[BlockComponentKind] = &[
    BlockComponentKind::Floor,
    BlockComponentKind::Wall,
    BlockComponentKind::Ceiling,
];
const FLOOR_CORNER_COMPONENTS: &[BlockComponentKind] = &[
    BlockComponentKind::Floor,
    BlockComponentKind::Wall,
    BlockComponentKind::Wall,
];
const FLOOR_DOORWAY_COMPONENTS: &[BlockComponentKind] = &[
    BlockComponentKind::Floor,
    BlockComponentKind::DoorPostLeft,
    BlockComponentKind::DoorPostRight,
    BlockComponentKind::DoorLintel,
];
const STAIRS_COMPONENTS: &[BlockComponentKind] = &[
    BlockComponentKind::Stair,
    BlockComponentKind::Stair,
    BlockComponentKind::Stair,
    BlockComponentKind::Stair,
];

static BLOCK_ASSETS: LazyLock<Vec<BlockAsset>> = LazyLock::new(|| {
    vec![
        BlockAsset {
            id: Uuid::from_u128(0xB10C_0000_0000_0000_0000_0000_0000_0003),
            name: "Floor Slab",
            name_key: "block_asset_floor_slab",
            description: "1-cell floor tile",
            description_key: "block_asset_floor_slab_desc",
            footprint: Vec3 { x: 1, y: 1, z: 1 },
            boxes: FLOOR_SLAB_BOXES,
            components: FLOOR_SLAB_COMPONENTS,
        },
        BlockAsset {
            id: default_block_asset_id(),
            name: "Floor + Wall",
            name_key: "block_asset_floor_wall",
            description: "Floor with one edge wall",
            description_key: "block_asset_floor_wall_desc",
            footprint: Vec3 { x: 1, y: 2, z: 1 },
            boxes: FLOOR_WALL_BOXES,
            components: FLOOR_WALL_COMPONENTS,
        },
        BlockAsset {
            id: Uuid::from_u128(0xB10C_0000_0000_0000_0000_0000_0000_0009),
            name: "Floor + Wall + Ceiling",
            name_key: "block_asset_floor_wall_ceiling",
            description: "Closed 2-high wall cell",
            description_key: "block_asset_floor_wall_ceiling_desc",
            footprint: Vec3 { x: 1, y: 2, z: 1 },
            boxes: FLOOR_WALL_CEILING_BOXES,
            components: FLOOR_WALL_CEILING_COMPONENTS,
        },
        BlockAsset {
            id: Uuid::from_u128(0xB10C_0000_0000_0000_0000_0000_0000_000A),
            name: "Floor + Corner",
            name_key: "block_asset_floor_corner",
            description: "Floor with two edge walls",
            description_key: "block_asset_floor_corner_desc",
            footprint: Vec3 { x: 1, y: 2, z: 1 },
            boxes: FLOOR_CORNER_BOXES,
            components: FLOOR_CORNER_COMPONENTS,
        },
        BlockAsset {
            id: Uuid::from_u128(0xB10C_0000_0000_0000_0000_0000_0000_000B),
            name: "Floor + Doorway",
            name_key: "block_asset_floor_doorway",
            description: "2-cell doorway with floor",
            description_key: "block_asset_floor_doorway_desc",
            footprint: Vec3 { x: 2, y: 2, z: 1 },
            boxes: FLOOR_DOORWAY_BOXES,
            components: FLOOR_DOORWAY_COMPONENTS,
        },
        BlockAsset {
            id: Uuid::from_u128(0xB10C_0000_0000_0000_0000_0000_0000_0007),
            name: "Stairs",
            name_key: "block_asset_stairs",
            description: "1-cell stair block",
            description_key: "block_asset_stairs_desc",
            footprint: Vec3 { x: 1, y: 1, z: 1 },
            boxes: STAIRS_BOXES,
            components: STAIRS_COMPONENTS,
        },
        BlockAsset {
            id: Uuid::from_u128(0xB10C_0000_0000_0000_0000_0000_0000_0004),
            name: "Wall",
            name_key: "block_asset_wall",
            description: "One edge wall, 2 cells high",
            description_key: "block_asset_wall_desc",
            footprint: Vec3 { x: 1, y: 2, z: 1 },
            boxes: WALL_BOXES,
            components: WALL_COMPONENTS,
        },
        BlockAsset {
            id: Uuid::from_u128(0xB10C_0000_0000_0000_0000_0000_0000_0006),
            name: "Doorway",
            name_key: "block_asset_doorway",
            description: "2-cell-wide edge opening",
            description_key: "block_asset_doorway_desc",
            footprint: Vec3 { x: 2, y: 2, z: 1 },
            boxes: DOORWAY_BOXES,
            components: DOORWAY_COMPONENTS,
        },
        BlockAsset {
            id: Uuid::from_u128(0xB10C_0000_0000_0000_0000_0000_0000_000C),
            name: "Ceiling Slab",
            name_key: "block_asset_ceiling_slab",
            description: "2-high ceiling tile",
            description_key: "block_asset_ceiling_slab_desc",
            footprint: Vec3 { x: 1, y: 2, z: 1 },
            boxes: CEILING_SLAB_BOXES,
            components: CEILING_SLAB_COMPONENTS,
        },
        BlockAsset {
            id: Uuid::from_u128(0xB10C_0000_0000_0000_0000_0000_0000_0001),
            name: "Full Block",
            name_key: "block_asset_full_block",
            description: "1x1x1 solid block",
            description_key: "block_asset_full_block_desc",
            footprint: Vec3 { x: 1, y: 1, z: 1 },
            boxes: FULL_BLOCK_BOXES,
            components: FULL_BLOCK_COMPONENTS,
        },
        BlockAsset {
            id: Uuid::from_u128(0xB10C_0000_0000_0000_0000_0000_0000_0002),
            name: "Large Block",
            name_key: "block_asset_large_block",
            description: "2x2x2 solid block",
            description_key: "block_asset_large_block_desc",
            footprint: Vec3 { x: 2, y: 2, z: 2 },
            boxes: LARGE_BLOCK_BOXES,
            components: LARGE_BLOCK_COMPONENTS,
        },
        BlockAsset {
            id: Uuid::from_u128(0xB10C_0000_0000_0000_0000_0000_0000_0005),
            name: "Column",
            name_key: "block_asset_column",
            description: "Column with base and cap",
            description_key: "block_asset_column_desc",
            footprint: Vec3 { x: 1, y: 2, z: 1 },
            boxes: COLUMN_BOXES,
            components: COLUMN_COMPONENTS,
        },
        BlockAsset {
            id: Uuid::from_u128(0xB10C_0000_0000_0000_0000_0000_0000_000D),
            name: "Plain Column",
            name_key: "block_asset_plain_column",
            description: "Column without base or cap",
            description_key: "block_asset_plain_column_desc",
            footprint: Vec3 { x: 1, y: 2, z: 1 },
            boxes: PLAIN_COLUMN_BOXES,
            components: PLAIN_COLUMN_COMPONENTS,
        },
        BlockAsset {
            id: Uuid::from_u128(0xB10C_0000_0000_0000_0000_0000_0000_000E),
            name: "Table",
            name_key: "block_asset_table",
            description: "Resizable table with four legs",
            description_key: "block_asset_table_desc",
            footprint: Vec3 { x: 2, y: 2, z: 1 },
            boxes: TABLE_BOXES,
            components: TABLE_COMPONENTS,
        },
    ]
});

pub fn block_assets() -> &'static [BlockAsset] {
    &BLOCK_ASSETS
}

pub fn block_asset(id: Uuid) -> Option<&'static BlockAsset> {
    block_assets().iter().find(|asset| asset.id == id)
}

fn style_effect_geometry(
    mut object: rusterix::GeometryObject,
    color: [u8; 4],
    material: &str,
) -> rusterix::GeometryObject {
    object
        .properties
        .set("prefab_default_color", Value::Color(TheColor::from(color)));
    object
        .properties
        .set("prefab_material_hint", Value::Str(material.to_string()));
    let material_slot = match material {
        "wood" if color == FURNITURE_DARK_WOOD => "DARK".to_string(),
        "wood" => "WOOD".to_string(),
        "metal" => "METAL".to_string(),
        "wax" => "WAX".to_string(),
        "emissive" => "EMBER".to_string(),
        "ceramic trim" => "TRIM".to_string(),
        other => other.to_ascii_uppercase(),
    };
    object
        .properties
        .set("prefab_material_slot", Value::Str(material_slot));
    object
}

pub fn style_block_asset_object(
    asset: &BlockAsset,
    component: BlockComponentKind,
    object: rusterix::GeometryObject,
) -> rusterix::GeometryObject {
    if asset.name != "Table" {
        return object;
    }
    let color = block_asset_default_color(asset, component).unwrap_or(FURNITURE_WOOD);
    let mut object = style_effect_geometry(object, color, "wood");
    object.properties.set(
        "prefab_material_slot",
        Value::Str(
            if component == BlockComponentKind::TableTop {
                "TOP"
            } else {
                "LEGS"
            }
            .to_string(),
        ),
    );
    object
}

pub fn block_asset_default_color(
    asset: &BlockAsset,
    component: BlockComponentKind,
) -> Option<[u8; 4]> {
    (asset.name == "Table").then_some(if component == BlockComponentKind::TableTop {
        FURNITURE_WOOD
    } else {
        FURNITURE_DARK_WOOD
    })
}

pub fn block_asset_default_surface_source(
    asset: &BlockAsset,
    component: BlockComponentKind,
    palette: &ThePalette,
) -> Option<rusterix::PixelSource> {
    let color = block_asset_default_color(asset, component)?;
    palette
        .find_closest_color_index(&TheColor::from(color))
        .map(|index| rusterix::PixelSource::PaletteIndex(index as u16))
}

pub fn ensure_block_asset_default_palette(project: &mut Project, asset: &BlockAsset) -> bool {
    if asset.name != "Table" {
        return false;
    }
    let before_palette = project.art_palette.clone();
    let before_materials = project.art_palette_materials.clone();
    prefab_palette_slot(project, FURNITURE_WOOD, "wood", "natural");
    prefab_palette_slot(project, FURNITURE_DARK_WOOD, "wood", "natural");
    let top_source = block_asset_default_surface_source(
        asset,
        BlockComponentKind::TableTop,
        &project.art_palette,
    );
    let leg_source = block_asset_default_surface_source(
        asset,
        BlockComponentKind::TableLegLeftFront,
        &project.art_palette,
    );
    let mut geometry_changed = false;
    let mut metadata_changed = false;
    for region in &mut project.regions {
        for object in &mut region.map.geometry_objects {
            if object.properties.get_id("block_asset_id") != Some(asset.id)
                || object
                    .properties
                    .get_int_default("block_default_surface_version", 0)
                    >= 2
            {
                continue;
            }
            let component_index = object
                .properties
                .get_int("block_component_index")
                .unwrap_or_else(|| if object.name.ends_with(" 1") { 0 } else { 1 });
            let source = if component_index == 0 {
                top_source.as_ref()
            } else {
                leg_source.as_ref()
            };
            let component = component_for(asset, component_index.max(0) as usize);
            let color = block_asset_default_color(asset, component).unwrap_or(FURNITURE_WOOD);
            object
                .properties
                .set("prefab_default_color", Value::Color(TheColor::from(color)));
            object
                .properties
                .set("prefab_material_hint", Value::Str("wood".to_string()));
            object.properties.set(
                "prefab_material_slot",
                Value::Str(
                    if component == BlockComponentKind::TableTop {
                        "TOP"
                    } else {
                        "LEGS"
                    }
                    .to_string(),
                ),
            );
            if let Some(source) = source {
                for face in &mut object.faces {
                    if face.tile.is_none() && face.tiles.is_empty() {
                        face.tile = Some(source.clone());
                        face.auto_uv = true;
                        geometry_changed = true;
                    }
                }
            }
            object
                .properties
                .set("block_default_surface_version", Value::Int(2));
            metadata_changed = true;
        }
    }
    project.art_palette != before_palette
        || project.art_palette_materials != before_materials
        || geometry_changed
        || metadata_changed
}

fn effect_geometry_box(
    name: &str,
    min: Vec3<f32>,
    max: Vec3<f32>,
    color: [u8; 4],
    material: &str,
) -> rusterix::GeometryObject {
    let mut object = rusterix::GeometryObject::box_from_bounds(name.to_string(), min, max);
    object.kind = rusterix::GeometryObjectKind::Prop;
    style_effect_geometry(object, color, material)
}

fn effect_geometry_face(indices: Vec<usize>, smoothing_group: u32) -> rusterix::GeometryFace {
    rusterix::GeometryFace {
        id: Uuid::new_v4(),
        paint_surface_id: None,
        uvs: indices.iter().map(|_| Vec2::zero()).collect(),
        indices,
        paint_uvs: Vec::new(),
        auto_uv: true,
        texture_offset: Vec2::zero(),
        texture_scale: Vec2::broadcast(1.0),
        texture_rotation: 0.0,
        tile: None,
        tiles: FxHashMap::default(),
        surface_points: Vec::new(),
        surface_segments: Vec::new(),
        smoothing_group,
    }
}

/// A cylinder aligned between two authored points. This is used instead of
/// placeholder boxes for fixtures whose silhouette depends on their angle.
fn effect_geometry_cylinder(
    name: &str,
    start: Vec3<f32>,
    end: Vec3<f32>,
    radius: f32,
    segments: usize,
    color: [u8; 4],
    material: &str,
) -> rusterix::GeometryObject {
    let axis = (end - start).try_normalized().unwrap_or_else(Vec3::unit_y);
    let reference = if axis.y.abs() < 0.92 {
        Vec3::unit_y()
    } else {
        Vec3::unit_x()
    };
    let side_a = axis
        .cross(reference)
        .try_normalized()
        .unwrap_or_else(Vec3::unit_z);
    let side_b = axis.cross(side_a).normalized();
    let segments = segments.max(6);
    let mut vertices = Vec::with_capacity(segments * 2);
    for center in [start, end] {
        for index in 0..segments {
            let angle = index as f32 / segments as f32 * std::f32::consts::TAU;
            vertices.push(center + side_a * angle.cos() * radius + side_b * angle.sin() * radius);
        }
    }
    let mut faces = Vec::with_capacity(segments + 2);
    for index in 0..segments {
        let next = (index + 1) % segments;
        faces.push(effect_geometry_face(
            vec![index, next, next + segments, index + segments],
            1,
        ));
    }
    faces.push(effect_geometry_face((0..segments).rev().collect(), 0));
    faces.push(effect_geometry_face((segments..segments * 2).collect(), 0));
    let mut object = rusterix::GeometryObject::new(name);
    object.kind = rusterix::GeometryObjectKind::Prop;
    object.vertices = vertices;
    object.faces = faces;
    object.ensure_face_paint_data();
    style_effect_geometry(object, color, material)
}

fn effect_surface_defaults(object: &rusterix::GeometryObject) -> ([u8; 4], String, String) {
    let name = object.name.to_ascii_lowercase();
    let inferred = if name.contains("iron")
        || name.contains("grate")
        || name.contains("metal")
        || name.contains("plate")
        || name.contains("basket")
        || name.contains("rail")
    {
        ([48, 52, 57, 255], "metal")
    } else if name.contains("wood")
        || name.contains("log")
        || name.contains("torch")
        || name.contains("table")
        || name.contains("door")
    {
        ([91, 49, 24, 255], "wood")
    } else if name.contains("wax") || name.contains("candle") {
        ([224, 207, 164, 255], "wax")
    } else if name.contains("ember") || name.contains("burning") {
        ([142, 48, 20, 255], "emissive")
    } else if name.contains("stone")
        || name.contains("wall")
        || name.contains("floor")
        || name.contains("ceiling")
        || name.contains("column")
        || name.contains("block")
        || name.contains("stair")
    {
        ([112, 106, 94, 255], "stone")
    } else {
        ([126, 118, 104, 255], "default")
    };
    let color = object
        .properties
        .get_color_default("prefab_default_color", TheColor::from(inferred.0));
    let material = object
        .properties
        .get_str_default("prefab_material_hint", inferred.1.to_string());
    let finish = if material == "metal" {
        "matte"
    } else {
        "natural"
    };
    (color.to_u8_array(), material, finish.to_string())
}

pub fn prefab_object_default_color(object: &rusterix::GeometryObject) -> [u8; 4] {
    effect_surface_defaults(object).0
}

fn palette_color_distance(a: [u8; 4], b: [u8; 4]) -> u32 {
    a[..3]
        .iter()
        .zip(&b[..3])
        .map(|(a, b)| {
            let delta = *a as i32 - *b as i32;
            (delta * delta) as u32
        })
        .sum()
}

fn prefab_palette_slot(project: &mut Project, color: [u8; 4], material: &str, finish: &str) -> u16 {
    project.ensure_art_palette_materials_len();
    let best_matching_material = project
        .art_palette
        .colors
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            let entry = entry.as_ref()?;
            let palette_material = project.art_palette_materials.get(index)?;
            (palette_material.preset == material)
                .then_some((index, palette_color_distance(entry.to_u8_array(), color)))
        })
        .min_by_key(|(_, distance)| *distance)
        .filter(|(_, distance)| *distance <= 1_200)
        .map(|(index, _)| index);
    if let Some(index) = best_matching_material {
        return index as u16;
    }

    if let Some(index) = project
        .art_palette
        .colors
        .iter()
        .position(|entry| entry.as_ref().is_none_or(|color| color.a <= f32::EPSILON))
    {
        project.art_palette.colors[index] = Some(TheColor::from(color));
        project.ensure_art_palette_materials_len();
        if let Some(palette_material) = project.art_palette_materials.get_mut(index) {
            palette_material.preset = material.to_string();
            palette_material.finish = finish.to_string();
        }
        return index as u16;
    }

    project
        .art_palette
        .find_closest_color_index(&TheColor::from(color))
        .unwrap_or(project.art_palette.current_index as usize) as u16
}

/// Resolve only genuinely unassigned faces. Existing tile, color, material,
/// and painted-face assignments remain exactly as authored.
pub fn ensure_prefab_default_surfaces(project: &mut Project, asset_id: Uuid) -> bool {
    let Some(mut asset) = project.block_props.get(&asset_id).cloned() else {
        return false;
    };
    let mut changed = false;
    for part in &mut asset.parts {
        let objects = match &mut part.geometry_source {
            rusterix::BlockPropGeometrySource::Authored { geometry_objects } => geometry_objects,
            rusterix::BlockPropGeometrySource::Recipe {
                generated_cache, ..
            } => generated_cache,
        };
        for object in objects {
            let (color, material, finish) = effect_surface_defaults(object);
            let mut palette_slot = None;
            for face in &mut object.faces {
                if face.tile.is_some() || !face.tiles.is_empty() {
                    continue;
                }
                let slot = *palette_slot
                    .get_or_insert_with(|| prefab_palette_slot(project, color, &material, &finish));
                face.tile = Some(rusterix::PixelSource::PaletteIndex(slot));
                changed = true;
            }
        }
    }
    if changed {
        project.block_props.insert(asset_id, asset);
    }
    changed
}

pub(crate) fn fire_emitter(rate: f32, scale: f32) -> rusterix::ParticleEmitterDef {
    let mut emitter = rusterix::ParticleEmitterDef::default();
    emitter.rate = rate;
    emitter.spread = 0.48;
    emitter.lifetime_range = (0.32, 0.86);
    emitter.radius_range = (0.025 * scale, 0.075 * scale);
    emitter.speed_range = (0.28 * scale, 0.78 * scale);
    emitter.spawn_area = [0.055 * scale, 0.015, 0.055 * scale];
    emitter.color = [255, 154, 61, 255];
    emitter.color_ramp = Some([
        [255, 245, 174, 255],
        [255, 190, 68, 255],
        [238, 84, 25, 220],
        [56, 12, 5, 0],
    ]);
    emitter.flame_base = true;
    emitter.size_curve = [0.72, 1.0, 0.68, 0.12];
    emitter.opacity_curve = [1.0, 0.92, 0.58, 0.0];
    emitter.gravity = [0.0, 0.16, 0.0];
    emitter.turbulence = 0.22;
    emitter
}

/// The authored flame used by Stonefall's wall torch. Keep the Prefab editor's
/// Flame preset on the same visual baseline as the shipped dungeon content.
pub(crate) fn stonefall_torch_flame_emitter() -> rusterix::ParticleEmitterDef {
    let mut emitter = rusterix::ParticleEmitterDef::default();
    emitter.direction = Vec3::new(0.0, 1.0, 0.0);
    emitter.spread = 0.48;
    emitter.rate = 25.0;
    emitter.color = [255, 154, 61, 255];
    emitter.color_ramp = Some([
        [255, 242, 168, 255],
        [255, 193, 79, 255],
        [240, 100, 31, 255],
        [64, 16, 8, 255],
    ]);
    emitter.color_variation = 20;
    emitter.lifetime_range = (0.32, 0.78);
    emitter.radius_range = (0.025, 0.065);
    emitter.speed_range = (0.28, 0.72);
    emitter.spawn_area = [0.045, 0.015, 0.035];
    emitter.flame_base = true;
    emitter
}

pub(crate) fn smoke_emitter(rate: f32, scale: f32) -> rusterix::ParticleEmitterDef {
    let mut emitter = rusterix::ParticleEmitterDef::default();
    emitter.rate = rate;
    emitter.spread = 0.28;
    emitter.lifetime_range = (1.5, 3.4);
    emitter.radius_range = (0.07 * scale, 0.20 * scale);
    emitter.speed_range = (0.12 * scale, 0.32 * scale);
    emitter.spawn_area = [0.06 * scale, 0.02, 0.06 * scale];
    emitter.color = [92, 92, 88, 120];
    emitter.color_ramp = Some([
        [58, 54, 48, 135],
        [88, 84, 78, 105],
        [120, 118, 112, 54],
        [140, 140, 140, 0],
    ]);
    emitter.size_curve = [0.65, 1.0, 1.45, 1.9];
    emitter.opacity_curve = [0.15, 0.82, 0.48, 0.0];
    emitter.gravity = [0.0, 0.08, 0.0];
    emitter.turbulence = 0.12;
    emitter
}

fn make_effect_prefab(
    id: Uuid,
    name: &str,
    geometry: Vec<rusterix::GeometryObject>,
    attachment_position: [f32; 3],
    placement_mode: rusterix::BlockPropPlacementMode,
    particles: Vec<(&str, rusterix::ParticleEmitterDef)>,
    light: bool,
) -> rusterix::BlockPropAsset {
    let part_id = Uuid::from_u128(id.as_u128().wrapping_add(0x100));
    let attachment_id = Uuid::from_u128(id.as_u128().wrapping_add(0x200));
    let mut part = rusterix::BlockPropPart::new_authored("Body", geometry);
    part.id = part_id;
    part.attachments.push(rusterix::BlockPropAttachment {
        id: attachment_id,
        name: "Effect origin".to_string(),
        position: attachment_position,
        direction: [0.0, 1.0, 0.0],
        up: [0.0, 0.0, 1.0],
    });
    let mut asset = rusterix::BlockPropAsset::new(name);
    asset.id = id;
    asset.alias = name.to_ascii_lowercase().replace(' ', "-");
    asset.category = "Effects".to_string();
    asset.tags = vec!["effect".to_string()];
    asset.parts.push(part);
    asset.placement.mode = placement_mode;
    asset.placement.snap_to_surfaces = true;
    asset.placement.snap_to_grid = placement_mode == rusterix::BlockPropPlacementMode::Ground;
    asset.placement.surface_offset = if placement_mode == rusterix::BlockPropPlacementMode::Wall {
        0.015
    } else {
        0.0
    };
    for (index, (particle_name, emitter)) in particles.into_iter().enumerate() {
        asset
            .particle_effects
            .push(rusterix::BlockPropParticleEffect {
                id: Uuid::from_u128(id.as_u128().wrapping_add(0x300 + index as u128)),
                name: particle_name.to_string(),
                part_id,
                attachment_id,
                enabled: true,
                emitter,
            });
    }
    if light {
        asset.light_effects.push(rusterix::BlockPropLightEffect {
            id: Uuid::from_u128(id.as_u128().wrapping_add(0x400)),
            name: "Fire light".to_string(),
            part_id,
            attachment_id,
            enabled: true,
            color: [255, 151, 65, 255],
            intensity: 2.4,
            range: 4.5,
            flicker: 0.22,
            lift: 0.05,
        });
    }
    asset
}

const PREFAB_AUTO_SIZE_TAG: &str = "auto-size";
const FURNITURE_VERSION_TAG: &str = "furniture-v5";
const DECORATION_VERSION_TAG: &str = "decoration-v2";

fn furniture_top_face_refs(object: &rusterix::GeometryObject) -> Vec<rusterix::BlockPropFaceRef> {
    let top = object
        .vertices
        .iter()
        .map(|vertex| vertex.y)
        .fold(f32::NEG_INFINITY, f32::max);
    object
        .faces
        .iter()
        .filter_map(|face| {
            (!face.indices.is_empty()
                && face.indices.iter().all(|index| {
                    object
                        .vertices
                        .get(*index)
                        .is_some_and(|vertex| (vertex.y - top).abs() <= 0.001)
                }))
            .then_some(rusterix::BlockPropFaceRef {
                object_id: object.id,
                face_id: face.id,
            })
        })
        .collect()
}

fn furniture_geometry_rounded_box(
    name: &str,
    min: Vec3<f32>,
    max: Vec3<f32>,
    radius: f32,
    color: [u8; 4],
    material: &str,
) -> rusterix::GeometryObject {
    let mut object = rusterix::GeometryObject::rounded_box_from_bounds(
        name.to_string(),
        min,
        max,
        radius,
        2,
        true,
    );
    object.kind = rusterix::GeometryObjectKind::Prop;
    style_effect_geometry(object, color, material)
}

fn furniture_geometry_barrel_body(
    name: &str,
    color: [u8; 4],
    material: &str,
) -> rusterix::GeometryObject {
    let segments = 16usize;
    let rings = [
        (0.05, 0.43),
        (0.26, 0.48),
        (0.68, 0.53),
        (1.00, 0.55),
        (1.32, 0.53),
        (1.74, 0.48),
        (1.92, 0.43),
    ];
    let mut object = rusterix::GeometryObject::new(name.to_string());
    for (y, radius) in rings {
        for index in 0..segments {
            let angle = index as f32 / segments as f32 * std::f32::consts::TAU;
            object
                .vertices
                .push(Vec3::new(angle.cos() * radius, y, angle.sin() * radius));
        }
    }
    for ring in 0..rings.len() - 1 {
        for index in 0..segments {
            let next = (index + 1) % segments;
            let lower = ring * segments;
            let upper = (ring + 1) * segments;
            object.faces.push(effect_geometry_face(
                vec![lower + index, lower + next, upper + next, upper + index],
                1,
            ));
        }
    }
    object
        .faces
        .push(effect_geometry_face((0..segments).rev().collect(), 0));
    let top = (rings.len() - 1) * segments;
    object
        .faces
        .push(effect_geometry_face((top..top + segments).collect(), 0));
    object.kind = rusterix::GeometryObjectKind::Prop;
    style_effect_geometry(object, color, material)
}

fn decoration_geometry_revolved(
    name: &str,
    rings: &[(f32, f32)],
    close_bottom: bool,
    close_top: bool,
    color: [u8; 4],
    material: &str,
) -> rusterix::GeometryObject {
    let segments = 20usize;
    let mut object = rusterix::GeometryObject::new(name.to_string());
    for (y, radius) in rings {
        for index in 0..segments {
            let angle = index as f32 / segments as f32 * std::f32::consts::TAU;
            object
                .vertices
                .push(Vec3::new(angle.cos() * radius, *y, angle.sin() * radius));
        }
    }
    for ring in 0..rings.len().saturating_sub(1) {
        for index in 0..segments {
            let next = (index + 1) % segments;
            let lower = ring * segments;
            let upper = (ring + 1) * segments;
            object.faces.push(effect_geometry_face(
                vec![lower + index, lower + next, upper + next, upper + index],
                1,
            ));
        }
    }
    if close_bottom && !rings.is_empty() {
        object
            .faces
            .push(effect_geometry_face((0..segments).rev().collect(), 0));
    }
    if close_top && !rings.is_empty() {
        let top = (rings.len() - 1) * segments;
        object
            .faces
            .push(effect_geometry_face((top..top + segments).collect(), 0));
    }
    object.kind = rusterix::GeometryObjectKind::Prop;
    object.ensure_face_paint_data();
    style_effect_geometry(object, color, material)
}

fn make_decoration_prefab(
    id: Uuid,
    name: &str,
    mut geometry: Vec<rusterix::GeometryObject>,
    footprint: [u32; 3],
    placement_mode: rusterix::BlockPropPlacementMode,
) -> rusterix::BlockPropAsset {
    for (object_index, object) in geometry.iter_mut().enumerate() {
        object.id = Uuid::from_u128(
            id.as_u128()
                .wrapping_add(0x1000 + object_index as u128 * 0x100),
        );
        for (face_index, face) in object.faces.iter_mut().enumerate() {
            face.id = Uuid::from_u128(id.as_u128().wrapping_add(
                0x10_0000 + object_index as u128 * 0x1_0000 + face_index as u128 * 0x100,
            ));
        }
    }
    let mut part = rusterix::BlockPropPart::new_authored("Decoration", geometry);
    part.id = Uuid::from_u128(id.as_u128().wrapping_add(0x100));

    let mut asset = rusterix::BlockPropAsset::new(name);
    asset.id = id;
    asset.alias = name.to_ascii_lowercase().replace(' ', "-");
    asset.category = "Decoration".to_string();
    asset.tags = vec![
        "decoration".to_string(),
        "placeable".to_string(),
        PREFAB_AUTO_SIZE_TAG.to_string(),
        DECORATION_VERSION_TAG.to_string(),
    ];
    asset.parts.push(part);
    asset.placement.mode = placement_mode;
    asset.placement.snap_to_grid = placement_mode == rusterix::BlockPropPlacementMode::Ground;
    asset.placement.snap_to_surfaces = true;
    asset.placement.surface_offset = match placement_mode {
        rusterix::BlockPropPlacementMode::Wall => 0.012,
        rusterix::BlockPropPlacementMode::AnySurface => 0.006,
        _ => 0.0,
    };
    asset.placement.footprint = footprint;
    asset
}

fn make_furniture_prefab(
    id: Uuid,
    name: &str,
    mut geometry: Vec<rusterix::GeometryObject>,
    footprint: [u32; 3],
    support_surfaces: &[(&str, usize, Option<u32>)],
) -> rusterix::BlockPropAsset {
    for (object_index, object) in geometry.iter_mut().enumerate() {
        object.id = Uuid::from_u128(
            id.as_u128()
                .wrapping_add(0x1000 + object_index as u128 * 0x100),
        );
        for (face_index, face) in object.faces.iter_mut().enumerate() {
            face.id = Uuid::from_u128(id.as_u128().wrapping_add(
                0x10_0000 + object_index as u128 * 0x1_0000 + face_index as u128 * 0x100,
            ));
        }
    }
    let part_id = Uuid::from_u128(id.as_u128().wrapping_add(0x100));
    let mut part = rusterix::BlockPropPart::new_authored("Furniture", geometry);
    part.id = part_id;

    let mut asset = rusterix::BlockPropAsset::new(name);
    asset.id = id;
    asset.alias = name.to_ascii_lowercase().replace(' ', "-");
    asset.category = "Furniture".to_string();
    asset.tags = vec![
        "furniture".to_string(),
        PREFAB_AUTO_SIZE_TAG.to_string(),
        FURNITURE_VERSION_TAG.to_string(),
    ];
    asset.placement.mode = rusterix::BlockPropPlacementMode::Ground;
    asset.placement.snap_to_grid = true;
    asset.placement.snap_to_surfaces = true;
    asset.placement.footprint = footprint;
    asset.parts.push(part);

    for (index, (surface_name, object_index, capacity)) in
        support_surfaces.iter().copied().enumerate()
    {
        let face_refs = asset.parts[0]
            .geometry_source
            .geometry_objects()
            .get(object_index)
            .map(furniture_top_face_refs)
            .unwrap_or_default();
        if face_refs.is_empty() {
            continue;
        }
        asset
            .support_surfaces
            .push(rusterix::BlockPropSupportSurface {
                id: Uuid::from_u128(id.as_u128().wrapping_add(0x500 + index as u128 * 0x100)),
                name: surface_name.to_string(),
                part_id,
                shape: rusterix::BlockPropSemanticShape::Faces(face_refs),
                snap_spacing: 0.25,
                allowed_item_tags: vec!["placeable".to_string()],
                capacity,
                occupancy_policy: rusterix::BlockPropOccupancyPolicy::RejectOverlap,
            });
    }
    asset
}

pub fn prefab_uses_auto_sizing(asset: &rusterix::BlockPropAsset) -> bool {
    asset.tags.iter().any(|tag| tag == PREFAB_AUTO_SIZE_TAG)
}

/// Apply the shared block height/width/depth controls to an opted-in linked
/// Prefab instance. The authored footprint is its natural size, so zero width
/// and depth extras leave it unchanged while height remains expressed in cells.
pub fn apply_prefab_auto_sizing(
    asset: &rusterix::BlockPropAsset,
    instance: &mut rusterix::BlockPropInstance,
    sizing: BlockSizing,
) {
    if !prefab_uses_auto_sizing(asset) {
        return;
    }
    let base_width = asset.placement.footprint[0].max(1) as f32;
    let base_height = asset.placement.footprint[1].max(1) as f32;
    let base_depth = asset.placement.footprint[2].max(1) as f32;
    let scale = Vec3::new(
        (base_width + sizing.span_extra_cells.max(0.0) * 2.0) / base_width,
        sizing.height_cells.max(1) as f32 / base_height,
        (base_depth + sizing.depth_extra_cells.max(0.0) * 2.0) / base_depth,
    );
    for row in 0..3 {
        instance.world_transform[0][row] *= scale.x;
        instance.world_transform[1][row] *= scale.y;
        instance.world_transform[2][row] *= scale.z;
    }
    instance
        .parameter_overrides
        .set("height_cells", Value::Int(sizing.height_cells.max(1)));
    instance.parameter_overrides.set(
        "width_extra_cells",
        Value::Float(sizing.span_extra_cells.max(0.0)),
    );
    instance.parameter_overrides.set(
        "depth_extra_cells",
        Value::Float(sizing.depth_extra_cells.max(0.0)),
    );
}

static BUNDLED_PREFABS: LazyLock<Vec<rusterix::BlockPropAsset>> = LazyLock::new(|| {
    let wall_torch_id = Uuid::from_u128(0xB10C_EFFE_0000_0000_0000_0000_0000_0001);
    let campfire_id = Uuid::from_u128(0xB10C_EFFE_0000_0000_0000_0000_0000_0002);
    let vapor_grate_id = Uuid::from_u128(0xB10C_EFFE_0000_0000_0000_0000_0000_0003);
    let candle_cluster_id = Uuid::from_u128(0xB10C_EFFE_0000_0000_0000_0000_0000_0004);
    let iron_brazier_id = Uuid::from_u128(0xB10C_EFFE_0000_0000_0000_0000_0000_0005);
    const IRON: [u8; 4] = [48, 52, 57, 255];
    const WOOD: [u8; 4] = [91, 49, 24, 255];
    const CHARRED: [u8; 4] = [45, 24, 17, 255];
    const EMBER: [u8; 4] = [142, 48, 20, 255];
    const WAX: [u8; 4] = [224, 207, 164, 255];
    const CERAMIC: [u8; 4] = [205, 194, 164, 255];
    const CERAMIC_TRIM: [u8; 4] = [68, 105, 124, 255];
    const GLASS: [u8; 4] = [151, 204, 215, 165];
    const FABRIC: [u8; 4] = [122, 43, 38, 255];
    const FABRIC_TRIM: [u8; 4] = [198, 151, 68, 255];
    vec![
        make_effect_prefab(
            wall_torch_id,
            "Wall Torch",
            vec![
                effect_geometry_cylinder(
                    "Wall plate",
                    Vec3::new(0.0, 0.48, -0.055),
                    Vec3::new(0.0, 0.48, 0.045),
                    0.17,
                    12,
                    IRON,
                    "metal",
                ),
                effect_geometry_cylinder(
                    "Iron bracket",
                    Vec3::new(0.0, 0.48, 0.03),
                    Vec3::new(0.0, 0.74, 0.29),
                    0.035,
                    10,
                    IRON,
                    "metal",
                ),
                effect_geometry_cylinder(
                    "Iron socket",
                    Vec3::new(0.0, 0.68, 0.24),
                    Vec3::new(0.0, 0.82, 0.37),
                    0.105,
                    12,
                    IRON,
                    "metal",
                ),
                effect_geometry_cylinder(
                    "Wooden torch",
                    Vec3::new(0.0, 0.73, 0.29),
                    Vec3::new(0.0, 1.23, 0.74),
                    0.055,
                    12,
                    WOOD,
                    "wood",
                ),
                effect_geometry_cylinder(
                    "Iron torch band",
                    Vec3::new(0.0, 1.10, 0.62),
                    Vec3::new(0.0, 1.17, 0.68),
                    0.073,
                    12,
                    IRON,
                    "metal",
                ),
                effect_geometry_cylinder(
                    "Burning tip",
                    Vec3::new(0.0, 1.18, 0.69),
                    Vec3::new(0.0, 1.34, 0.83),
                    0.095,
                    12,
                    CHARRED,
                    "wood",
                ),
            ],
            [0.0, 1.36, 0.85],
            rusterix::BlockPropPlacementMode::Wall,
            vec![
                ("Flame", stonefall_torch_flame_emitter()),
                ("Smoke", smoke_emitter(5.0, 0.7)),
            ],
            true,
        ),
        make_effect_prefab(
            campfire_id,
            "Campfire",
            vec![
                effect_geometry_box(
                    "Log A",
                    Vec3::new(-0.62, 0.06, -0.12),
                    Vec3::new(0.62, 0.22, 0.12),
                    WOOD,
                    "wood",
                ),
                effect_geometry_box(
                    "Log B",
                    Vec3::new(-0.12, 0.12, -0.62),
                    Vec3::new(0.12, 0.28, 0.62),
                    WOOD,
                    "wood",
                ),
                effect_geometry_box(
                    "Embers",
                    Vec3::new(-0.30, 0.02, -0.30),
                    Vec3::new(0.30, 0.13, 0.30),
                    EMBER,
                    "emissive",
                ),
            ],
            [0.0, 0.24, 0.0],
            rusterix::BlockPropPlacementMode::Ground,
            vec![
                ("Flame", fire_emitter(46.0, 1.5)),
                ("Smoke", smoke_emitter(8.0, 1.25)),
            ],
            true,
        ),
        make_effect_prefab(
            vapor_grate_id,
            "Vapor Grate",
            vec![
                effect_geometry_box(
                    "Grate rim north",
                    Vec3::new(-0.65, 0.00, -0.65),
                    Vec3::new(0.65, 0.08, -0.50),
                    IRON,
                    "metal",
                ),
                effect_geometry_box(
                    "Grate rim south",
                    Vec3::new(-0.65, 0.00, 0.50),
                    Vec3::new(0.65, 0.08, 0.65),
                    IRON,
                    "metal",
                ),
                effect_geometry_box(
                    "Grate rim west",
                    Vec3::new(-0.65, 0.00, -0.50),
                    Vec3::new(-0.50, 0.08, 0.50),
                    IRON,
                    "metal",
                ),
                effect_geometry_box(
                    "Grate rim east",
                    Vec3::new(0.50, 0.00, -0.50),
                    Vec3::new(0.65, 0.08, 0.50),
                    IRON,
                    "metal",
                ),
                effect_geometry_box(
                    "Grate bar A",
                    Vec3::new(-0.34, 0.02, -0.50),
                    Vec3::new(-0.25, 0.07, 0.50),
                    IRON,
                    "metal",
                ),
                effect_geometry_box(
                    "Grate bar B",
                    Vec3::new(-0.05, 0.02, -0.50),
                    Vec3::new(0.05, 0.07, 0.50),
                    IRON,
                    "metal",
                ),
                effect_geometry_box(
                    "Grate bar C",
                    Vec3::new(0.25, 0.02, -0.50),
                    Vec3::new(0.34, 0.07, 0.50),
                    IRON,
                    "metal",
                ),
            ],
            [0.0, 0.04, 0.0],
            rusterix::BlockPropPlacementMode::Ground,
            vec![("Vapor", {
                let mut vapor = smoke_emitter(16.0, 1.5);
                vapor.color = [158, 178, 184, 105];
                vapor
            })],
            false,
        ),
        {
            let mut candles = make_effect_prefab(
                candle_cluster_id,
                "Candle Cluster",
                vec![
                    effect_geometry_cylinder(
                        "Iron candle tray",
                        Vec3::new(0.0, 0.015, 0.0),
                        Vec3::new(0.0, 0.065, 0.0),
                        0.38,
                        16,
                        IRON,
                        "metal",
                    ),
                    effect_geometry_cylinder(
                        "Tall candle",
                        Vec3::new(-0.12, 0.06, 0.03),
                        Vec3::new(-0.12, 0.62, 0.03),
                        0.075,
                        12,
                        WAX,
                        "wax",
                    ),
                    effect_geometry_cylinder(
                        "Short candle",
                        Vec3::new(0.13, 0.06, 0.10),
                        Vec3::new(0.13, 0.40, 0.10),
                        0.09,
                        12,
                        WAX,
                        "wax",
                    ),
                    effect_geometry_cylinder(
                        "Rear candle",
                        Vec3::new(0.06, 0.06, -0.14),
                        Vec3::new(0.06, 0.50, -0.14),
                        0.065,
                        12,
                        WAX,
                        "wax",
                    ),
                ],
                [-0.12, 0.65, 0.03],
                rusterix::BlockPropPlacementMode::Ground,
                vec![("Tall candle flame", fire_emitter(15.0, 0.42))],
                true,
            );
            let part_id = candles.parts[0].id;
            for (index, (name, position)) in [
                ("Short candle flame", [0.13, 0.43, 0.10]),
                ("Rear candle flame", [0.06, 0.53, -0.14]),
            ]
            .into_iter()
            .enumerate()
            {
                let attachment_id = Uuid::from_u128(
                    candle_cluster_id
                        .as_u128()
                        .wrapping_add(0x210 + index as u128),
                );
                candles.parts[0]
                    .attachments
                    .push(rusterix::BlockPropAttachment {
                        id: attachment_id,
                        name: name.to_string(),
                        position,
                        direction: [0.0, 1.0, 0.0],
                        up: [0.0, 0.0, 1.0],
                    });
                candles
                    .particle_effects
                    .push(rusterix::BlockPropParticleEffect {
                        id: Uuid::from_u128(
                            candle_cluster_id
                                .as_u128()
                                .wrapping_add(0x310 + index as u128),
                        ),
                        name: name.to_string(),
                        part_id,
                        attachment_id,
                        enabled: true,
                        emitter: fire_emitter(13.0, 0.36),
                    });
            }
            candles
        },
        make_effect_prefab(
            iron_brazier_id,
            "Iron Brazier",
            vec![
                effect_geometry_cylinder(
                    "Iron foot",
                    Vec3::new(0.0, 0.02, 0.0),
                    Vec3::new(0.0, 0.10, 0.0),
                    0.43,
                    16,
                    IRON,
                    "metal",
                ),
                effect_geometry_cylinder(
                    "Iron pedestal",
                    Vec3::new(0.0, 0.08, 0.0),
                    Vec3::new(0.0, 0.38, 0.0),
                    0.12,
                    12,
                    IRON,
                    "metal",
                ),
                effect_geometry_cylinder(
                    "Iron fire bowl",
                    Vec3::new(0.0, 0.33, 0.0),
                    Vec3::new(0.0, 0.50, 0.0),
                    0.52,
                    16,
                    IRON,
                    "metal",
                ),
                effect_geometry_cylinder(
                    "Ember bed",
                    Vec3::new(0.0, 0.49, 0.0),
                    Vec3::new(0.0, 0.55, 0.0),
                    0.40,
                    16,
                    EMBER,
                    "emissive",
                ),
                effect_geometry_box(
                    "Cage rail north",
                    Vec3::new(-0.48, 0.66, -0.47),
                    Vec3::new(0.48, 0.72, -0.42),
                    IRON,
                    "metal",
                ),
                effect_geometry_box(
                    "Cage rail south",
                    Vec3::new(-0.48, 0.66, 0.42),
                    Vec3::new(0.48, 0.72, 0.47),
                    IRON,
                    "metal",
                ),
                effect_geometry_box(
                    "Cage rail west",
                    Vec3::new(-0.47, 0.66, -0.42),
                    Vec3::new(-0.42, 0.72, 0.42),
                    IRON,
                    "metal",
                ),
                effect_geometry_box(
                    "Cage rail east",
                    Vec3::new(0.42, 0.66, -0.42),
                    Vec3::new(0.47, 0.72, 0.42),
                    IRON,
                    "metal",
                ),
                effect_geometry_box(
                    "Cage post northwest",
                    Vec3::new(-0.48, 0.45, -0.48),
                    Vec3::new(-0.40, 0.76, -0.40),
                    IRON,
                    "metal",
                ),
                effect_geometry_box(
                    "Cage post northeast",
                    Vec3::new(0.40, 0.45, -0.48),
                    Vec3::new(0.48, 0.76, -0.40),
                    IRON,
                    "metal",
                ),
                effect_geometry_box(
                    "Cage post southwest",
                    Vec3::new(-0.48, 0.45, 0.40),
                    Vec3::new(-0.40, 0.76, 0.48),
                    IRON,
                    "metal",
                ),
                effect_geometry_box(
                    "Cage post southeast",
                    Vec3::new(0.40, 0.45, 0.40),
                    Vec3::new(0.48, 0.76, 0.48),
                    IRON,
                    "metal",
                ),
            ],
            [0.0, 0.57, 0.0],
            rusterix::BlockPropPlacementMode::Ground,
            vec![
                ("Flame", fire_emitter(52.0, 1.15)),
                ("Smoke", smoke_emitter(7.0, 0.85)),
            ],
            true,
        ),
        make_decoration_prefab(
            Uuid::from_u128(0xB10C_DEC0_0000_0000_0000_0000_0000_0001),
            "Ceramic Plate",
            vec![
                decoration_geometry_revolved(
                    "Plate body",
                    &[(0.015, 0.14), (0.030, 0.34), (0.075, 0.45), (0.105, 0.43)],
                    true,
                    true,
                    CERAMIC,
                    "ceramic",
                ),
                decoration_geometry_revolved(
                    "Painted rim",
                    &[(0.096, 0.36), (0.108, 0.43)],
                    false,
                    false,
                    CERAMIC_TRIM,
                    "ceramic trim",
                ),
            ],
            [1, 2, 1],
            rusterix::BlockPropPlacementMode::AnySurface,
        ),
        make_decoration_prefab(
            Uuid::from_u128(0xB10C_DEC0_0000_0000_0000_0000_0000_0002),
            "Ceramic Bowl",
            vec![
                decoration_geometry_revolved(
                    "Bowl body",
                    &[(0.015, 0.20), (0.08, 0.31), (0.28, 0.44), (0.48, 0.51)],
                    true,
                    false,
                    CERAMIC,
                    "ceramic",
                ),
                decoration_geometry_revolved(
                    "Painted bowl rim",
                    &[(0.455, 0.49), (0.50, 0.53)],
                    false,
                    false,
                    CERAMIC_TRIM,
                    "ceramic trim",
                ),
            ],
            [1, 2, 1],
            rusterix::BlockPropPlacementMode::AnySurface,
        ),
        make_decoration_prefab(
            Uuid::from_u128(0xB10C_DEC0_0000_0000_0000_0000_0000_0003),
            "Drinking Goblet",
            vec![
                effect_geometry_cylinder(
                    "Glass foot",
                    Vec3::new(0.0, 0.01, 0.0),
                    Vec3::new(0.0, 0.055, 0.0),
                    0.25,
                    16,
                    GLASS,
                    "glass",
                ),
                effect_geometry_cylinder(
                    "Glass stem",
                    Vec3::new(0.0, 0.04, 0.0),
                    Vec3::new(0.0, 0.39, 0.0),
                    0.045,
                    12,
                    GLASS,
                    "glass",
                ),
                decoration_geometry_revolved(
                    "Glass cup",
                    &[(0.34, 0.10), (0.43, 0.22), (0.68, 0.33), (0.88, 0.30)],
                    true,
                    false,
                    GLASS,
                    "glass",
                ),
            ],
            [1, 2, 1],
            rusterix::BlockPropPlacementMode::AnySurface,
        ),
        make_decoration_prefab(
            Uuid::from_u128(0xB10C_DEC0_0000_0000_0000_0000_0000_0004),
            "Floor Carpet",
            vec![
                furniture_geometry_rounded_box(
                    "Carpet field",
                    Vec3::new(-1.0, 0.012, -1.5),
                    Vec3::new(1.0, 0.075, 1.5),
                    0.055,
                    FABRIC,
                    "fabric",
                ),
                furniture_geometry_rounded_box(
                    "Carpet center stripe",
                    Vec3::new(-0.10, 0.073, -1.34),
                    Vec3::new(0.10, 0.093, 1.34),
                    0.025,
                    FABRIC_TRIM,
                    "trim",
                ),
                furniture_geometry_rounded_box(
                    "Carpet north border",
                    Vec3::new(-0.88, 0.073, -1.39),
                    Vec3::new(0.88, 0.093, -1.24),
                    0.025,
                    FABRIC_TRIM,
                    "trim",
                ),
                furniture_geometry_rounded_box(
                    "Carpet south border",
                    Vec3::new(-0.88, 0.073, 1.24),
                    Vec3::new(0.88, 0.093, 1.39),
                    0.025,
                    FABRIC_TRIM,
                    "trim",
                ),
            ],
            [2, 2, 3],
            rusterix::BlockPropPlacementMode::Ground,
        ),
        make_decoration_prefab(
            Uuid::from_u128(0xB10C_DEC0_0000_0000_0000_0000_0000_0005),
            "Wall Carpet",
            vec![
                furniture_geometry_rounded_box(
                    "Tapestry field",
                    Vec3::new(-0.88, -0.92, 0.00),
                    Vec3::new(0.88, 0.92, 0.045),
                    0.045,
                    FABRIC,
                    "fabric",
                ),
                furniture_geometry_rounded_box(
                    "Tapestry vertical ornament",
                    Vec3::new(-0.10, -0.78, 0.044),
                    Vec3::new(0.10, 0.78, 0.070),
                    0.025,
                    FABRIC_TRIM,
                    "trim",
                ),
                effect_geometry_cylinder(
                    "Tapestry hanging rod",
                    Vec3::new(-1.02, 0.98, 0.055),
                    Vec3::new(1.02, 0.98, 0.055),
                    0.055,
                    12,
                    FURNITURE_DARK_WOOD,
                    "wood",
                ),
                effect_geometry_cylinder(
                    "Tapestry bottom rod",
                    Vec3::new(-0.94, -0.96, 0.050),
                    Vec3::new(0.94, -0.96, 0.050),
                    0.035,
                    10,
                    FURNITURE_DARK_WOOD,
                    "wood",
                ),
            ],
            [2, 2, 1],
            rusterix::BlockPropPlacementMode::Wall,
        ),
        make_furniture_prefab(
            Uuid::from_u128(0xB10C_F012_0000_0000_0000_0000_0000_0001),
            "Chair",
            vec![
                furniture_geometry_rounded_box(
                    "Seat",
                    Vec3::new(-0.46, 0.82, -0.46),
                    Vec3::new(0.46, 1.00, 0.46),
                    0.08,
                    FURNITURE_WOOD,
                    "wood",
                ),
                effect_geometry_cylinder(
                    "Front left leg",
                    Vec3::new(-0.40, 0.0, -0.40),
                    Vec3::new(-0.32, 0.84, -0.32),
                    0.065,
                    10,
                    FURNITURE_DARK_WOOD,
                    "wood",
                ),
                effect_geometry_cylinder(
                    "Front right leg",
                    Vec3::new(0.40, 0.0, -0.40),
                    Vec3::new(0.32, 0.84, -0.32),
                    0.065,
                    10,
                    FURNITURE_DARK_WOOD,
                    "wood",
                ),
                effect_geometry_cylinder(
                    "Back left post",
                    Vec3::new(-0.40, 0.0, 0.40),
                    Vec3::new(-0.32, 2.0, 0.34),
                    0.07,
                    10,
                    FURNITURE_DARK_WOOD,
                    "wood",
                ),
                effect_geometry_cylinder(
                    "Back right post",
                    Vec3::new(0.40, 0.0, 0.40),
                    Vec3::new(0.32, 2.0, 0.34),
                    0.07,
                    10,
                    FURNITURE_DARK_WOOD,
                    "wood",
                ),
                furniture_geometry_rounded_box(
                    "Back rest",
                    Vec3::new(-0.31, 1.28, 0.29),
                    Vec3::new(0.31, 1.76, 0.39),
                    0.045,
                    FURNITURE_WOOD,
                    "wood",
                ),
                effect_geometry_cylinder(
                    "Front stretcher",
                    Vec3::new(-0.34, 0.38, -0.35),
                    Vec3::new(0.34, 0.38, -0.35),
                    0.035,
                    8,
                    FURNITURE_DARK_WOOD,
                    "wood",
                ),
            ],
            [1, 2, 1],
            &[("Seat", 0, Some(1))],
        ),
        make_furniture_prefab(
            Uuid::from_u128(0xB10C_F012_0000_0000_0000_0000_0000_0002),
            "Open Cupboard",
            vec![
                furniture_geometry_rounded_box(
                    "Cupboard top",
                    Vec3::new(-1.0, 1.82, -0.52),
                    Vec3::new(1.0, 2.0, 0.52),
                    0.07,
                    FURNITURE_WOOD,
                    "wood",
                ),
                furniture_geometry_rounded_box(
                    "Left side",
                    Vec3::new(-0.96, 0.16, -0.48),
                    Vec3::new(-0.80, 1.84, 0.48),
                    0.045,
                    FURNITURE_WOOD,
                    "wood",
                ),
                furniture_geometry_rounded_box(
                    "Right side",
                    Vec3::new(0.80, 0.16, -0.48),
                    Vec3::new(0.96, 1.84, 0.48),
                    0.045,
                    FURNITURE_WOOD,
                    "wood",
                ),
                furniture_geometry_rounded_box(
                    "Back",
                    Vec3::new(-0.80, 0.16, -0.48),
                    Vec3::new(0.80, 1.82, -0.40),
                    0.025,
                    FURNITURE_DARK_WOOD,
                    "wood",
                ),
                furniture_geometry_rounded_box(
                    "Shelf",
                    Vec3::new(-0.80, 0.90, -0.40),
                    Vec3::new(0.80, 1.04, 0.45),
                    0.04,
                    FURNITURE_WOOD,
                    "wood",
                ),
                furniture_geometry_rounded_box(
                    "Cupboard base",
                    Vec3::new(-0.88, 0.12, -0.44),
                    Vec3::new(0.88, 0.28, 0.48),
                    0.05,
                    FURNITURE_DARK_WOOD,
                    "wood",
                ),
                effect_geometry_cylinder(
                    "Left foot",
                    Vec3::new(-0.68, 0.0, 0.0),
                    Vec3::new(-0.68, 0.16, 0.0),
                    0.10,
                    10,
                    FURNITURE_DARK_WOOD,
                    "wood",
                ),
                effect_geometry_cylinder(
                    "Right foot",
                    Vec3::new(0.68, 0.0, 0.0),
                    Vec3::new(0.68, 0.16, 0.0),
                    0.10,
                    10,
                    FURNITURE_DARK_WOOD,
                    "wood",
                ),
            ],
            [2, 2, 1],
            &[
                ("Cupboard top", 0, Some(8)),
                ("Shelf", 4, Some(8)),
                ("Bottom shelf", 5, Some(8)),
            ],
        ),
        make_furniture_prefab(
            Uuid::from_u128(0xB10C_F012_0000_0000_0000_0000_0000_0003),
            "Storage Chest",
            vec![
                furniture_geometry_rounded_box(
                    "Chest lid",
                    Vec3::new(-1.0, 1.66, -0.52),
                    Vec3::new(1.0, 2.0, 0.52),
                    0.13,
                    FURNITURE_WOOD,
                    "wood",
                ),
                furniture_geometry_rounded_box(
                    "Chest body",
                    Vec3::new(-0.92, 0.22, -0.46),
                    Vec3::new(0.92, 1.70, 0.46),
                    0.09,
                    FURNITURE_WOOD,
                    "wood",
                ),
                furniture_geometry_rounded_box(
                    "Chest base",
                    Vec3::new(-1.0, 0.0, -0.50),
                    Vec3::new(1.0, 0.22, 0.50),
                    0.07,
                    FURNITURE_DARK_WOOD,
                    "wood",
                ),
                furniture_geometry_rounded_box(
                    "Left iron band",
                    Vec3::new(-0.72, 0.20, -0.49),
                    Vec3::new(-0.58, 1.89, -0.465),
                    0.012,
                    FURNITURE_IRON,
                    "metal",
                ),
                furniture_geometry_rounded_box(
                    "Right iron band",
                    Vec3::new(0.58, 0.20, -0.49),
                    Vec3::new(0.72, 1.89, -0.465),
                    0.012,
                    FURNITURE_IRON,
                    "metal",
                ),
                furniture_geometry_rounded_box(
                    "Latch",
                    Vec3::new(-0.13, 1.28, -0.54),
                    Vec3::new(0.13, 1.66, -0.465),
                    0.035,
                    FURNITURE_IRON,
                    "metal",
                ),
                effect_geometry_cylinder(
                    "Left handle mount",
                    Vec3::new(-0.82, 0.96, -0.47),
                    Vec3::new(-0.82, 1.12, -0.51),
                    0.045,
                    8,
                    FURNITURE_IRON,
                    "metal",
                ),
                effect_geometry_cylinder(
                    "Right handle mount",
                    Vec3::new(0.82, 0.96, -0.47),
                    Vec3::new(0.82, 1.12, -0.51),
                    0.045,
                    8,
                    FURNITURE_IRON,
                    "metal",
                ),
            ],
            [2, 2, 1],
            &[("Chest lid", 0, Some(8))],
        ),
        make_furniture_prefab(
            Uuid::from_u128(0xB10C_F012_0000_0000_0000_0000_0000_0004),
            "Barrel",
            vec![
                effect_geometry_cylinder(
                    "Barrel top",
                    Vec3::new(0.0, 1.88, 0.0),
                    Vec3::new(0.0, 2.0, 0.0),
                    0.48,
                    16,
                    FURNITURE_WOOD,
                    "wood",
                ),
                furniture_geometry_barrel_body("Barrel body", FURNITURE_WOOD, "wood"),
                effect_geometry_cylinder(
                    "Lower hoop",
                    Vec3::new(0.0, 0.22, 0.0),
                    Vec3::new(0.0, 0.36, 0.0),
                    0.51,
                    16,
                    FURNITURE_IRON,
                    "metal",
                ),
                effect_geometry_cylinder(
                    "Upper hoop",
                    Vec3::new(0.0, 1.64, 0.0),
                    Vec3::new(0.0, 1.78, 0.0),
                    0.51,
                    16,
                    FURNITURE_IRON,
                    "metal",
                ),
            ],
            [1, 2, 1],
            &[("Barrel top", 0, Some(4))],
        ),
        make_furniture_prefab(
            Uuid::from_u128(0xB10C_F012_0000_0000_0000_0000_0000_0005),
            "Bench",
            vec![
                furniture_geometry_rounded_box(
                    "Bench seat",
                    Vec3::new(-1.0, 0.82, -0.46),
                    Vec3::new(1.0, 1.0, 0.46),
                    0.08,
                    FURNITURE_WOOD,
                    "wood",
                ),
                effect_geometry_cylinder(
                    "Left front leg",
                    Vec3::new(-0.76, 0.0, -0.34),
                    Vec3::new(-0.68, 0.84, -0.30),
                    0.075,
                    10,
                    FURNITURE_DARK_WOOD,
                    "wood",
                ),
                effect_geometry_cylinder(
                    "Right front leg",
                    Vec3::new(0.76, 0.0, -0.34),
                    Vec3::new(0.68, 0.84, -0.30),
                    0.075,
                    10,
                    FURNITURE_DARK_WOOD,
                    "wood",
                ),
                furniture_geometry_rounded_box(
                    "Back rail",
                    Vec3::new(-0.86, 1.28, 0.31),
                    Vec3::new(0.86, 1.72, 0.43),
                    0.055,
                    FURNITURE_WOOD,
                    "wood",
                ),
                effect_geometry_cylinder(
                    "Left back post",
                    Vec3::new(-0.82, 0.0, 0.38),
                    Vec3::new(-0.76, 2.0, 0.36),
                    0.085,
                    10,
                    FURNITURE_DARK_WOOD,
                    "wood",
                ),
                effect_geometry_cylinder(
                    "Right back post",
                    Vec3::new(0.82, 0.0, 0.38),
                    Vec3::new(0.76, 2.0, 0.36),
                    0.085,
                    10,
                    FURNITURE_DARK_WOOD,
                    "wood",
                ),
                effect_geometry_cylinder(
                    "Front stretcher",
                    Vec3::new(-0.72, 0.38, -0.32),
                    Vec3::new(0.72, 0.38, -0.32),
                    0.045,
                    8,
                    FURNITURE_DARK_WOOD,
                    "wood",
                ),
            ],
            [2, 2, 1],
            &[("Bench seat", 0, Some(4))],
        ),
    ]
});

pub fn bundled_prefabs() -> &'static [rusterix::BlockPropAsset] {
    &BUNDLED_PREFABS
}

pub fn bundled_prefab(id: Uuid) -> Option<&'static rusterix::BlockPropAsset> {
    bundled_prefabs().iter().find(|asset| asset.id == id)
}

/// Bring project-owned copies of bundled Prefabs forward when their shipped
/// geometry schema changes. User-authored copies with different IDs are never
/// touched.
pub fn upgrade_bundled_prefab_geometry(project: &mut Project, asset_id: Uuid) -> bool {
    let Some(project_asset) = project.block_props.get(&asset_id) else {
        return false;
    };
    let Some(bundled) = bundled_prefab(asset_id) else {
        return false;
    };
    let bundled_schema_tag = bundled.tags.iter().find(|tag| {
        tag.as_str() == FURNITURE_VERSION_TAG || tag.as_str() == DECORATION_VERSION_TAG
    });
    if bundled_schema_tag.is_some_and(|schema_tag| !project_asset.tags.contains(schema_tag)) {
        let Some(project_asset) = project.block_props.get_mut(&asset_id) else {
            return false;
        };
        project_asset.parts = bundled.parts.clone();
        project_asset.support_surfaces = bundled.support_surfaces.clone();
        project_asset.placement = bundled.placement.clone();
        project_asset.tags = bundled.tags.clone();
        for region in &mut project.regions {
            for instance in &mut region.map.block_prop_instances {
                if instance.asset_id != asset_id {
                    continue;
                }
                for (old_label, new_label) in [("DARK WOOD", "DARK"), ("CERAMIC TRIM", "TRIM")] {
                    let old_key = rusterix::block_prop_material_override_key(old_label);
                    let new_key = rusterix::block_prop_material_override_key(new_label);
                    if let Some(value) = instance.overrides.remove(&old_key) {
                        if !instance.overrides.contains(&new_key) {
                            instance.overrides.set(&new_key, value);
                        }
                    }
                }
            }
        }
        return true;
    }

    // Migrate the original three-box Wall Torch placeholder without replacing
    // the user's particle or light edits on the project-owned Prefab.
    let object_names = project_asset
        .parts
        .first()
        .map(|part| {
            part.geometry_source
                .geometry_objects()
                .iter()
                .map(|object| object.name.as_str())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if object_names != ["Wall plate", "Torch stem", "Fire basket"] {
        return false;
    }
    let Some(project_part) = project
        .block_props
        .get_mut(&asset_id)
        .and_then(|asset| asset.parts.first_mut())
    else {
        return false;
    };
    let Some(bundled_part) = bundled.parts.first() else {
        return false;
    };
    project_part.geometry_source = bundled_part.geometry_source.clone();
    project_part.attachments = bundled_part.attachments.clone();
    true
}

pub fn upgrade_all_bundled_prefabs(project: &mut Project) -> bool {
    let asset_ids = project.block_props.keys().copied().collect::<Vec<_>>();
    let bundled_changed = asset_ids.into_iter().fold(false, |changed, asset_id| {
        let upgraded = upgrade_bundled_prefab_geometry(project, asset_id);
        let surfaced =
            bundled_prefab(asset_id).is_some() && ensure_prefab_default_surfaces(project, asset_id);
        changed || upgraded || surfaced
    });
    let table_palette_changed = block_assets()
        .iter()
        .find(|asset| asset.name == "Table")
        .is_some_and(|asset| ensure_block_asset_default_palette(project, asset));
    bundled_changed || table_palette_changed
}

fn prefab_surface_hit_and_normal(
    map: &Map,
    server_ctx: &ServerContext,
) -> Option<(Vec3<f32>, Option<Vec3<f32>>)> {
    let hit = server_ctx
        .hover_surface_hit_pos
        .or(server_ctx.hover_cursor_3d)
        .or_else(|| server_ctx.geo_hit.map(|_| server_ctx.geo_hit_pos))?;
    let normal = server_ctx.hover_surface_normal;
    if let Some(scenevm::GeoId::GeometryObject(object_id)) = server_ctx.geo_hit
        && let Some((wall_hit, wall_normal)) =
            map.wall_surface_frame_for_geometry_object(object_id, hit, normal)
    {
        return Some((wall_hit, Some(wall_normal)));
    }
    Some((hit, normal))
}

pub fn prefab_surface_placement_valid(
    asset: &rusterix::BlockPropAsset,
    map: &Map,
    server_ctx: &ServerContext,
) -> bool {
    let has_hit = server_ctx.hover_surface_hit_pos.is_some()
        || server_ctx.hover_cursor_3d.is_some()
        || server_ctx.geo_hit.is_some();
    match asset.placement.mode {
        rusterix::BlockPropPlacementMode::Ground => has_hit,
        rusterix::BlockPropPlacementMode::Free => has_hit,
        rusterix::BlockPropPlacementMode::AnySurface => {
            has_hit && server_ctx.hover_surface_normal.is_some()
        }
        rusterix::BlockPropPlacementMode::Wall => {
            has_hit
                && prefab_surface_hit_and_normal(map, server_ctx)
                    .and_then(|(_, normal)| normal)
                    .is_some_and(|normal| normal.y.abs() <= 0.72)
        }
    }
}

/// Build the live placement preview frame for a surface-mounted Prefab.
pub fn surface_prefab_preview_instance(
    asset: &rusterix::BlockPropAsset,
    map: &Map,
    server_ctx: &ServerContext,
) -> Option<rusterix::BlockPropInstance> {
    if !prefab_surface_placement_valid(asset, map, server_ctx) {
        return None;
    }
    let (hit, normal) = prefab_surface_hit_and_normal(map, server_ctx)?;
    if asset.placement.mode == rusterix::BlockPropPlacementMode::Free {
        let mut instance = rusterix::BlockPropInstance::new(asset.id);
        instance.world_transform[3][0] = hit.x;
        instance.world_transform[3][1] = hit.y;
        instance.world_transform[3][2] = hit.z;
        apply_prefab_auto_sizing(asset, &mut instance, block_sizing_from_context(server_ctx));
        return Some(instance);
    }
    let normal = normal?.try_normalized()?;
    if asset.placement.mode == rusterix::BlockPropPlacementMode::Wall && normal.y.abs() > 0.72 {
        return None;
    }
    let (mut right, mut up, forward) =
        if asset.placement.mode == rusterix::BlockPropPlacementMode::Wall {
            let forward = Vec3::new(normal.x, 0.0, normal.z).try_normalized()?;
            (
                Vec3::unit_y().cross(forward).try_normalized()?,
                Vec3::unit_y(),
                forward,
            )
        } else {
            let up = normal;
            let reference = if up.z.abs() < 0.9 {
                Vec3::unit_z()
            } else {
                Vec3::unit_x()
            };
            let forward = (reference - up * reference.dot(up)).try_normalized()?;
            (up.cross(forward).try_normalized()?, up, forward)
        };
    let angle =
        server_ctx.block_rotation_quarters.rem_euclid(4) as f32 * std::f32::consts::FRAC_PI_2;
    let (sin, cos) = angle.sin_cos();
    let rotated_right = right * cos + up * sin;
    let rotated_up = up * cos - right * sin;
    right = rotated_right;
    up = rotated_up;
    let origin = hit + normal * asset.placement.surface_offset;
    let mut instance = rusterix::BlockPropInstance::new(asset.id);
    for (column, axis) in [right, up, forward].into_iter().enumerate() {
        instance.world_transform[column][0] = axis.x;
        instance.world_transform[column][1] = axis.y;
        instance.world_transform[column][2] = axis.z;
    }
    instance.world_transform[3][0] = origin.x;
    instance.world_transform[3][1] = origin.y;
    instance.world_transform[3][2] = origin.z;
    apply_prefab_auto_sizing(asset, &mut instance, block_sizing_from_context(server_ctx));
    Some(instance)
}

/// Convert one immutable built-in catalog entry into an ordinary project
/// Prefab. Editing therefore uses the same geometry/effect pipeline as every
/// user-authored asset instead of a reduced, hard-coded representation.
pub fn editable_prefab_from_block_asset(asset: &BlockAsset) -> rusterix::BlockPropAsset {
    let geometry = asset
        .boxes
        .iter()
        .enumerate()
        .filter_map(|(index, _)| {
            adjusted_rotated_bounds(asset, index, BlockSizing::default(), 0).map(|(min, max)| {
                let mut object = rusterix::GeometryObject::box_from_bounds(
                    format!("{} Part {}", asset.name, index + 1),
                    min,
                    max,
                );
                object = style_block_asset_object(asset, component_for(asset, index), object);
                object.kind = rusterix::GeometryObjectKind::Prop;
                object
            })
        })
        .collect::<Vec<_>>();
    let mut prefab =
        rusterix::BlockPropAsset::new_authored(localized_block_asset_name(asset), geometry);
    prefab.alias = asset.name.to_ascii_lowercase().replace(' ', "-");
    prefab.category = "Built-in Copy".to_string();
    prefab.placement.footprint = [
        asset.footprint.x.max(1) as u32,
        asset.footprint.y.max(1) as u32,
        asset.footprint.z.max(1) as u32,
    ];
    prefab
}

pub fn block_sizing_from_context(server_ctx: &ServerContext) -> BlockSizing {
    BlockSizing {
        height_cells: server_ctx.block_height_cells.max(1),
        span_extra_cells: server_ctx.block_span_extra_cells.max(0.0),
        depth_extra_cells: server_ctx.block_depth_extra_cells.max(0.0),
    }
}

pub fn component_supports_height(component: BlockComponentKind) -> bool {
    matches!(
        component,
        BlockComponentKind::Solid
            | BlockComponentKind::Wall
            | BlockComponentKind::Column
            | BlockComponentKind::ColumnShaft
            | BlockComponentKind::ColumnCapital
            | BlockComponentKind::DoorPostLeft
            | BlockComponentKind::DoorPostRight
            | BlockComponentKind::DoorLintel
            | BlockComponentKind::Ceiling
            | BlockComponentKind::TableTop
            | BlockComponentKind::TableLegLeftFront
            | BlockComponentKind::TableLegRightFront
            | BlockComponentKind::TableLegLeftBack
            | BlockComponentKind::TableLegRightBack
    )
}

pub fn component_supports_width(component: BlockComponentKind) -> bool {
    matches!(
        component,
        BlockComponentKind::Solid
            | BlockComponentKind::Floor
            | BlockComponentKind::Ceiling
            | BlockComponentKind::Wall
            | BlockComponentKind::DoorPostLeft
            | BlockComponentKind::DoorPostRight
            | BlockComponentKind::DoorLintel
            | BlockComponentKind::Stair
            | BlockComponentKind::TableTop
            | BlockComponentKind::TableLegLeftFront
            | BlockComponentKind::TableLegRightFront
            | BlockComponentKind::TableLegLeftBack
            | BlockComponentKind::TableLegRightBack
    )
}

pub fn component_supports_depth(component: BlockComponentKind) -> bool {
    matches!(
        component,
        BlockComponentKind::TableTop
            | BlockComponentKind::TableLegLeftFront
            | BlockComponentKind::TableLegRightFront
            | BlockComponentKind::TableLegLeftBack
            | BlockComponentKind::TableLegRightBack
    )
}

pub fn asset_supports_height(asset: &BlockAsset) -> bool {
    asset
        .components
        .iter()
        .copied()
        .any(component_supports_height)
}

pub fn asset_supports_width(asset: &BlockAsset) -> bool {
    asset
        .components
        .iter()
        .copied()
        .any(component_supports_width)
}

pub fn asset_supports_depth(asset: &BlockAsset) -> bool {
    asset
        .components
        .iter()
        .copied()
        .any(component_supports_depth)
}

fn component_for(asset: &BlockAsset, index: usize) -> BlockComponentKind {
    asset
        .components
        .get(index)
        .copied()
        .unwrap_or(BlockComponentKind::Solid)
}

pub fn block_component_kind(asset: &BlockAsset, index: usize) -> BlockComponentKind {
    component_for(asset, index)
}

pub fn component_uses_cylinder(component: BlockComponentKind) -> bool {
    matches!(component, BlockComponentKind::ColumnShaft)
}

pub fn cylinder_vertices_and_faces(
    min: Vec3<f32>,
    max: Vec3<f32>,
    segments: usize,
) -> (Vec<Vec3<f32>>, Vec<Vec<usize>>) {
    let segments = segments.max(6);
    let center_x = (min.x + max.x) * 0.5;
    let center_z = (min.z + max.z) * 0.5;
    let radius = ((max.x - min.x).abs().min((max.z - min.z).abs()) * 0.5).max(0.01);

    let mut vertices = Vec::with_capacity(segments * 2);
    for y in [min.y, max.y] {
        for index in 0..segments {
            let angle = index as f32 / segments as f32 * std::f32::consts::TAU;
            vertices.push(Vec3::new(
                center_x + angle.cos() * radius,
                y,
                center_z + angle.sin() * radius,
            ));
        }
    }

    let mut faces = Vec::with_capacity(segments + 2);
    for index in 0..segments {
        let next = (index + 1) % segments;
        faces.push(vec![index, next, next + segments, index + segments]);
    }
    faces.push((0..segments).rev().collect());
    faces.push((segments..segments * 2).collect());
    (vertices, faces)
}

pub fn adjusted_block_box(
    asset: &BlockAsset,
    index: usize,
    sizing: BlockSizing,
) -> Option<BlockBox> {
    let mut block_box = *asset.boxes.get(index)?;
    let component = component_for(asset, index);
    let height = sizing.height_cells.max(1) as f32;
    let extra = sizing.span_extra_cells.max(0.0);
    let depth_extra = sizing.depth_extra_cells.max(0.0);

    match component {
        BlockComponentKind::Solid
        | BlockComponentKind::Wall
        | BlockComponentKind::Column
        | BlockComponentKind::DoorPostLeft
        | BlockComponentKind::DoorPostRight => {
            block_box.max.y = block_box.min.y + height;
        }
        BlockComponentKind::ColumnShaft => {
            let top_offset = (2.0 - block_box.max.y).max(0.0);
            block_box.max.y = (height - top_offset).max(block_box.min.y + 0.1);
        }
        BlockComponentKind::DoorLintel | BlockComponentKind::Ceiling => {
            let thickness = (block_box.max.y - block_box.min.y).max(0.01);
            block_box.max.y = height;
            block_box.min.y = (block_box.max.y - thickness).max(0.0);
        }
        BlockComponentKind::ColumnCapital => {
            let delta = height - 2.0;
            block_box.min.y += delta;
            block_box.max.y += delta;
        }
        BlockComponentKind::TableTop => {
            let thickness = (block_box.max.y - block_box.min.y).max(0.01);
            block_box.max.y = height;
            block_box.min.y = (height - thickness).max(0.0);
        }
        BlockComponentKind::TableLegLeftFront
        | BlockComponentKind::TableLegRightFront
        | BlockComponentKind::TableLegLeftBack
        | BlockComponentKind::TableLegRightBack => {
            let top_thickness = 0.16;
            block_box.max.y = (height - top_thickness).max(block_box.min.y + 0.1);
        }
        BlockComponentKind::Floor | BlockComponentKind::Stair | BlockComponentKind::ColumnBase => {}
    }

    if extra > 0.0 {
        match component {
            BlockComponentKind::Solid
            | BlockComponentKind::Floor
            | BlockComponentKind::Ceiling
            | BlockComponentKind::Wall
            | BlockComponentKind::DoorLintel
            | BlockComponentKind::Stair
            | BlockComponentKind::TableTop => {
                block_box.min.x -= extra;
                block_box.max.x += extra;
            }
            BlockComponentKind::DoorPostLeft
            | BlockComponentKind::TableLegLeftFront
            | BlockComponentKind::TableLegLeftBack => {
                block_box.min.x -= extra;
                block_box.max.x -= extra;
            }
            BlockComponentKind::DoorPostRight
            | BlockComponentKind::TableLegRightFront
            | BlockComponentKind::TableLegRightBack => {
                block_box.min.x += extra;
                block_box.max.x += extra;
            }
            BlockComponentKind::Column
            | BlockComponentKind::ColumnBase
            | BlockComponentKind::ColumnShaft
            | BlockComponentKind::ColumnCapital => {}
        }
    }

    if depth_extra > 0.0 {
        match component {
            BlockComponentKind::TableTop => {
                block_box.min.z -= depth_extra;
                block_box.max.z += depth_extra;
            }
            BlockComponentKind::TableLegLeftFront | BlockComponentKind::TableLegRightFront => {
                block_box.min.z -= depth_extra;
                block_box.max.z -= depth_extra;
            }
            BlockComponentKind::TableLegLeftBack | BlockComponentKind::TableLegRightBack => {
                block_box.min.z += depth_extra;
                block_box.max.z += depth_extra;
            }
            _ => {}
        }
    }

    Some(block_box)
}

pub fn adjusted_rotated_bounds(
    asset: &BlockAsset,
    index: usize,
    sizing: BlockSizing,
    quarter_turns: i32,
) -> Option<(Vec3<f32>, Vec3<f32>)> {
    adjusted_block_box(asset, index, sizing)
        .map(|block_box| rotated_bounds(block_box, asset.footprint, quarter_turns))
}

/// Horizontal snap spacing used by the Block/Prefab placement tool.
///
/// Construction blocks keep their authored cell size because that value also
/// controls their physical scale. Prefabs that opt into grid snapping instead
/// follow the map's shared editor grid.
pub fn block_tool_horizontal_grid_step(
    asset_id: Option<Uuid>,
    prefab_assets: &IndexMap<Uuid, rusterix::BlockPropAsset>,
    map: &Map,
    server_ctx: &ServerContext,
) -> f32 {
    if let Some(asset_id) = asset_id
        && block_asset(asset_id).is_none()
        && prefab_assets
            .get(&asset_id)
            .is_some_and(|asset| asset.placement.snap_to_grid)
    {
        return ServerContext::edit_grid_step(map.subdivisions).max(0.01);
    }

    server_ctx.block_grid_cell_size.max(0.05)
}

pub fn block_grid_plane_hit(server_ctx: &ServerContext) -> Option<Vec3<f32>> {
    let cell_size = server_ctx.block_grid_cell_size.max(0.05);
    let grid_y = server_ctx.block_grid_level as f32 * cell_size;
    let ray_origin = server_ctx.hover_ray_origin_3d?;
    let ray_dir = server_ctx.hover_ray_dir_3d?;
    if ray_dir.y.abs() <= 1e-6 {
        return None;
    }
    let t = (grid_y - ray_origin.y) / ray_dir.y;
    (t >= 0.0).then_some(ray_origin + ray_dir * t)
}

pub fn block_surface_base_y(server_ctx: &ServerContext, fallback_y: f32) -> Option<f32> {
    let normal = server_ctx.hover_surface_normal?;
    if normal.y.abs() <= 0.55 {
        return None;
    }
    let hit = server_ctx
        .hover_surface_hit_pos
        .or_else(|| server_ctx.geo_hit.map(|_| server_ctx.geo_hit_pos))?;
    if !hit.y.is_finite() || hit.y + 0.001 < fallback_y {
        return None;
    }
    Some(hit.y)
}

pub fn block_stroke_cells(start: Vec3<i32>, end: Vec3<i32>, stroke_mode: i32) -> Vec<Vec3<i32>> {
    if stroke_mode == BLOCK_STROKE_RECT {
        let min_x = start.x.min(end.x);
        let max_x = start.x.max(end.x);
        let min_z = start.z.min(end.z);
        let max_z = start.z.max(end.z);
        let mut cells = Vec::new();
        for z in min_z..=max_z {
            for x in min_x..=max_x {
                cells.push(Vec3::new(x, start.y, z));
            }
        }
        return cells;
    }

    let mut cells = Vec::new();
    let mut x = start.x;
    let mut z = start.z;
    let dx = (end.x - start.x).abs();
    let dz = -(end.z - start.z).abs();
    let sx = if start.x < end.x { 1 } else { -1 };
    let sz = if start.z < end.z { 1 } else { -1 };
    let mut err = dx + dz;

    loop {
        cells.push(Vec3::new(x, start.y, z));
        if x == end.x && z == end.z {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dz {
            err += dz;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            z += sz;
        }
    }
    cells
}

fn rotate_corner(point: Vec3<f32>, footprint: Vec3<i32>, quarter_turns: i32) -> Vec3<f32> {
    let mut p = point;
    let mut size_x = footprint.x as f32;
    let mut size_z = footprint.z as f32;
    for _ in 0..quarter_turns.rem_euclid(4) {
        p = Vec3::new(size_z - p.z, p.y, p.x);
        std::mem::swap(&mut size_x, &mut size_z);
    }
    p
}

pub fn rotated_bounds(
    block_box: BlockBox,
    footprint: Vec3<i32>,
    quarter_turns: i32,
) -> (Vec3<f32>, Vec3<f32>) {
    let corners = [
        Vec3::new(block_box.min.x, block_box.min.y, block_box.min.z),
        Vec3::new(block_box.max.x, block_box.min.y, block_box.min.z),
        Vec3::new(block_box.min.x, block_box.min.y, block_box.max.z),
        Vec3::new(block_box.max.x, block_box.min.y, block_box.max.z),
        Vec3::new(block_box.min.x, block_box.max.y, block_box.min.z),
        Vec3::new(block_box.max.x, block_box.max.y, block_box.min.z),
        Vec3::new(block_box.min.x, block_box.max.y, block_box.max.z),
        Vec3::new(block_box.max.x, block_box.max.y, block_box.max.z),
    ];

    let mut min = Vec3::broadcast(f32::INFINITY);
    let mut max = Vec3::broadcast(f32::NEG_INFINITY);
    for corner in corners {
        let p = rotate_corner(corner, footprint, quarter_turns);
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        min.z = min.z.min(p.z);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
        max.z = max.z.max(p.z);
    }
    (min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: f32, b: f32) {
        assert!((a - b).abs() < 0.0001, "expected {a} to be close to {b}");
    }

    #[test]
    fn widening_doorway_moves_posts_instead_of_thickening_them() {
        let asset = block_assets()
            .iter()
            .find(|asset| asset.name == "Doorway")
            .expect("Doorway block asset");
        let sizing = BlockSizing {
            height_cells: 2,
            span_extra_cells: 1.0,
            depth_extra_cells: 0.0,
        };

        let left = adjusted_block_box(asset, 0, sizing).unwrap();
        let right = adjusted_block_box(asset, 1, sizing).unwrap();
        let lintel = adjusted_block_box(asset, 2, sizing).unwrap();

        assert_close(left.max.x - left.min.x, 0.28);
        assert_close(right.max.x - right.min.x, 0.28);
        assert_close(left.min.x, -1.0);
        assert_close(left.max.x, -0.72);
        assert_close(right.min.x, 2.72);
        assert_close(right.max.x, 3.0);
        assert_close(lintel.min.x, -1.0);
        assert_close(lintel.max.x, 3.0);
    }

    #[test]
    fn taller_column_keeps_base_and_cap_proportions() {
        let asset = block_assets()
            .iter()
            .find(|asset| asset.name == "Column")
            .expect("Column block asset");
        let sizing = BlockSizing {
            height_cells: 4,
            span_extra_cells: 0.0,
            depth_extra_cells: 0.0,
        };

        let base = adjusted_block_box(asset, 0, sizing).unwrap();
        let shaft = adjusted_block_box(asset, 2, sizing).unwrap();
        let lower_cap = adjusted_block_box(asset, 3, sizing).unwrap();
        let upper_cap = adjusted_block_box(asset, 4, sizing).unwrap();

        assert_close(base.min.y, 0.0);
        assert_close(base.max.y, 0.14);
        assert_close(shaft.min.y, 0.28);
        assert_close(shaft.max.y, 3.72);
        assert_close(lower_cap.min.y, 3.72);
        assert_close(lower_cap.max.y, 3.86);
        assert_close(upper_cap.min.y, 3.86);
        assert_close(upper_cap.max.y, 4.0);
    }

    #[test]
    fn plain_column_is_a_single_resizable_shaft() {
        let asset = block_assets()
            .iter()
            .find(|asset| asset.name == "Plain Column")
            .expect("Plain Column block asset");
        let sizing = BlockSizing {
            height_cells: 4,
            span_extra_cells: 0.0,
            depth_extra_cells: 0.0,
        };

        let shaft = adjusted_block_box(asset, 0, sizing).unwrap();

        assert_eq!(asset.boxes.len(), 1);
        assert_close(shaft.min.x, 0.28);
        assert_close(shaft.max.x, 0.72);
        assert_close(shaft.min.y, 0.0);
        assert_close(shaft.max.y, 4.0);
        assert_close(shaft.min.z, 0.28);
        assert_close(shaft.max.z, 0.72);
    }

    #[test]
    fn table_adjusts_all_dimensions_without_thickening_its_legs() {
        let asset = block_assets()
            .iter()
            .find(|asset| asset.name == "Table")
            .expect("Table block asset");
        let sizing = BlockSizing {
            height_cells: 3,
            span_extra_cells: 1.0,
            depth_extra_cells: 2.0,
        };

        let top = adjusted_block_box(asset, 0, sizing).unwrap();
        let left_front = adjusted_block_box(asset, 1, sizing).unwrap();
        let right_front = adjusted_block_box(asset, 2, sizing).unwrap();
        let left_back = adjusted_block_box(asset, 3, sizing).unwrap();
        let right_back = adjusted_block_box(asset, 4, sizing).unwrap();

        assert!(asset_supports_height(asset));
        assert!(asset_supports_width(asset));
        assert!(asset_supports_depth(asset));
        assert_close(top.min.x, -1.0);
        assert_close(top.max.x, 3.0);
        assert_close(top.min.z, -2.0);
        assert_close(top.max.z, 3.0);
        assert_close(top.min.y, 2.84);
        assert_close(top.max.y, 3.0);

        for leg in [left_front, right_front, left_back, right_back] {
            assert_close(leg.max.x - leg.min.x, 0.14);
            assert_close(leg.min.y, 0.0);
            assert_close(leg.max.y, top.min.y);
        }
        assert_close(left_front.min.x, -0.90);
        assert_close(left_back.min.x, -0.90);
        assert_close(right_front.max.x, 2.90);
        assert_close(right_back.max.x, 2.90);
        assert_close(left_front.min.z, -1.90);
        assert_close(right_front.min.z, -1.90);
        assert_close(left_back.max.z, 2.90);
        assert_close(right_back.max.z, 2.90);
    }

    #[test]
    fn table_has_default_wood_materials() {
        let asset = block_assets()
            .iter()
            .find(|asset| asset.name == "Table")
            .expect("Table block asset");
        let mut project = Project::default();
        let mut region = Region::default();
        let mut existing_top = rusterix::GeometryObject::box_("Table 1", Vec3::zero(), Vec3::one());
        existing_top
            .properties
            .set("block_asset_id", Value::Id(asset.id));
        region.map.geometry_objects.push(existing_top);
        project.regions.push(region);
        assert!(ensure_block_asset_default_palette(&mut project, asset));
        let top = style_block_asset_object(
            asset,
            BlockComponentKind::TableTop,
            rusterix::GeometryObject::box_("Top", Vec3::zero(), Vec3::one()),
        );
        let leg = style_block_asset_object(
            asset,
            BlockComponentKind::TableLegLeftFront,
            rusterix::GeometryObject::box_("Leg", Vec3::zero(), Vec3::one()),
        );
        let top_source = block_asset_default_surface_source(
            asset,
            BlockComponentKind::TableTop,
            &project.art_palette,
        )
        .expect("Table top palette source");
        let leg_source = block_asset_default_surface_source(
            asset,
            BlockComponentKind::TableLegLeftFront,
            &project.art_palette,
        )
        .expect("Table leg palette source");

        let rusterix::PixelSource::PaletteIndex(top_index) = top_source else {
            panic!("Table top should use a palette-backed source");
        };
        let rusterix::PixelSource::PaletteIndex(leg_index) = leg_source else {
            panic!("Table leg should use a palette-backed source");
        };
        assert_ne!(top_index, leg_index);
        assert_eq!(
            project.art_palette.colors[top_index as usize]
                .as_ref()
                .map(TheColor::to_u8_array),
            Some(FURNITURE_WOOD)
        );
        assert_eq!(
            project.art_palette.colors[leg_index as usize]
                .as_ref()
                .map(TheColor::to_u8_array),
            Some(FURNITURE_DARK_WOOD)
        );
        assert_eq!(top.properties.get_str("prefab_material_slot"), Some("TOP"));
        assert_eq!(leg.properties.get_str("prefab_material_slot"), Some("LEGS"));
        let migrated_top = &project.regions[0].map.geometry_objects[0];
        assert!(
            migrated_top
                .faces
                .iter()
                .all(|face| face.tile == Some(rusterix::PixelSource::PaletteIndex(top_index)))
        );
        assert_eq!(
            migrated_top
                .properties
                .get_int("block_default_surface_version"),
            Some(2)
        );
        assert_eq!(
            migrated_top.properties.get_str("prefab_material_slot"),
            Some("TOP")
        );
        assert!(!ensure_block_asset_default_palette(&mut project, asset));
    }

    #[test]
    fn prefab_placement_uses_the_shared_grid_without_rescaling_blocks() {
        let mut map = Map::default();
        map.subdivisions = 8.0;
        let mut server_ctx = ServerContext::new();
        server_ctx.block_grid_cell_size = 1.0;

        let mut prefab = rusterix::BlockPropAsset::new("Candle");
        prefab.placement.snap_to_grid = true;
        let prefab_id = prefab.id;
        let prefab_assets = IndexMap::from_iter([(prefab_id, prefab)]);

        assert_close(
            block_tool_horizontal_grid_step(Some(prefab_id), &prefab_assets, &map, &server_ctx),
            0.125,
        );
        assert_close(
            block_tool_horizontal_grid_step(
                Some(default_block_asset_id()),
                &prefab_assets,
                &map,
                &server_ctx,
            ),
            1.0,
        );
    }

    #[test]
    fn bundled_furniture_has_placeable_support_surfaces() {
        for name in ["Chair", "Open Cupboard", "Storage Chest", "Barrel", "Bench"] {
            let asset = bundled_prefabs()
                .iter()
                .find(|asset| asset.name == name)
                .unwrap_or_else(|| panic!("missing bundled furniture Prefab {name}"));
            assert!(prefab_uses_auto_sizing(asset));
            assert!(!asset.support_surfaces.is_empty());
            assert!(asset.support_surfaces.iter().all(|surface| {
                surface
                    .allowed_item_tags
                    .iter()
                    .any(|tag| tag == "placeable")
                    && matches!(&surface.shape, rusterix::BlockPropSemanticShape::Faces(_))
            }));
        }
    }

    #[test]
    fn bundled_decoration_uses_the_expected_surface_modes_and_material_slots() {
        let plate = bundled_prefabs()
            .iter()
            .find(|asset| asset.name == "Ceramic Plate")
            .expect("Ceramic Plate decoration");
        let carpet = bundled_prefabs()
            .iter()
            .find(|asset| asset.name == "Floor Carpet")
            .expect("Floor Carpet decoration");
        let tapestry = bundled_prefabs()
            .iter()
            .find(|asset| asset.name == "Wall Carpet")
            .expect("Wall Carpet decoration");

        assert_eq!(plate.category, "Decoration");
        assert_eq!(
            plate.placement.mode,
            rusterix::BlockPropPlacementMode::AnySurface
        );
        assert_eq!(
            carpet.placement.mode,
            rusterix::BlockPropPlacementMode::Ground
        );
        assert_eq!(
            tapestry.placement.mode,
            rusterix::BlockPropPlacementMode::Wall
        );
        assert!(prefab_uses_auto_sizing(plate));
        let plate_slots = rusterix::block_prop_asset_material_slots(plate)
            .into_iter()
            .map(|(label, _)| label)
            .collect::<Vec<_>>();
        assert_eq!(plate_slots, vec!["CERAMIC".to_string(), "TRIM".to_string()]);
        let tapestry_slots = rusterix::block_prop_asset_material_slots(tapestry)
            .into_iter()
            .map(|(label, _)| label)
            .collect::<Vec<_>>();
        assert!(tapestry_slots.iter().any(|slot| slot == "FABRIC"));
        assert!(tapestry_slots.iter().any(|slot| slot == "TRIM"));
        assert!(tapestry_slots.iter().any(|slot| slot == "DARK"));
    }

    #[test]
    fn furniture_uses_detailed_geometry_and_the_cupboard_has_one_shelf() {
        let chair = bundled_prefabs()
            .iter()
            .find(|asset| asset.name == "Chair")
            .expect("Chair furniture Prefab");
        assert!(
            chair.parts[0]
                .geometry_source
                .geometry_objects()
                .iter()
                .any(|object| object.faces.len() > 6)
        );

        let cupboard = bundled_prefabs()
            .iter()
            .find(|asset| asset.name == "Open Cupboard")
            .expect("Open Cupboard furniture Prefab");
        let shelf_names = cupboard.parts[0]
            .geometry_source
            .geometry_objects()
            .iter()
            .filter(|object| object.name.contains("Shelf"))
            .map(|object| object.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(shelf_names, ["Shelf"]);
        assert!(
            cupboard
                .support_surfaces
                .iter()
                .any(|surface| surface.name == "Shelf")
        );
        let back = cupboard.parts[0]
            .geometry_source
            .geometry_objects()
            .iter()
            .find(|object| object.name == "Back")
            .expect("Cupboard back");
        let back_max_z = back
            .vertices
            .iter()
            .map(|vertex| vertex.z)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            back_max_z < 0.0,
            "the cupboard opening should face south (+Z)"
        );
    }

    #[test]
    fn furniture_exposes_each_authored_material_as_a_separate_slot() {
        let chest = bundled_prefabs()
            .iter()
            .find(|asset| asset.name == "Storage Chest")
            .expect("Storage Chest furniture Prefab");
        let labels = rusterix::block_prop_asset_material_slots(chest)
            .into_iter()
            .map(|(label, _)| label)
            .collect::<Vec<_>>();

        assert!(labels.iter().any(|label| label == "WOOD"));
        assert!(labels.iter().any(|label| label == "DARK"));
        assert!(labels.iter().any(|label| label == "METAL"));
    }

    #[test]
    fn furniture_prefab_uses_shared_height_width_and_depth_controls() {
        let asset = bundled_prefabs()
            .iter()
            .find(|asset| asset.name == "Chair")
            .expect("Chair furniture Prefab");
        let mut instance = rusterix::BlockPropInstance::new(asset.id);
        apply_prefab_auto_sizing(
            asset,
            &mut instance,
            BlockSizing {
                height_cells: 3,
                span_extra_cells: 0.5,
                depth_extra_cells: 1.0,
            },
        );

        assert_close(instance.world_transform[0][0], 2.0);
        assert_close(instance.world_transform[1][1], 1.5);
        assert_close(instance.world_transform[2][2], 3.0);
        assert_eq!(
            instance.parameter_overrides.get_int("height_cells"),
            Some(3)
        );
        assert_eq!(
            instance.parameter_overrides.get_float("width_extra_cells"),
            Some(0.5)
        );
        assert_eq!(
            instance.parameter_overrides.get_float("depth_extra_cells"),
            Some(1.0)
        );
    }
}
