use crate::prelude::*;
use std::sync::LazyLock;

pub const BLOCK_OPERATION_PLACE: i32 = 0;
pub const BLOCK_OPERATION_REPLACE: i32 = 1;
pub const BLOCK_OPERATION_ERASE: i32 = 2;
pub const BLOCK_STROKE_LINE: i32 = 0;
pub const BLOCK_STROKE_RECT: i32 = 1;
pub const DEFAULT_BLOCK_HEIGHT_CELLS: i32 = 2;
pub const DEFAULT_BLOCK_SPAN_EXTRA_CELLS: i32 = 0;
pub const BLOCK_COLUMN_SEGMENTS: usize = 12;

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
    TableLegLeft,
    TableLegRight,
}

#[derive(Clone, Copy)]
pub struct BlockSizing {
    pub height_cells: i32,
    pub span_extra_cells: i32,
}

impl Default for BlockSizing {
    fn default() -> Self {
        Self {
            height_cells: DEFAULT_BLOCK_HEIGHT_CELLS,
            span_extra_cells: DEFAULT_BLOCK_SPAN_EXTRA_CELLS,
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
    BlockComponentKind::TableLegLeft,
    BlockComponentKind::TableLegRight,
    BlockComponentKind::TableLegLeft,
    BlockComponentKind::TableLegRight,
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

pub fn block_sizing_from_context(server_ctx: &ServerContext) -> BlockSizing {
    BlockSizing {
        height_cells: server_ctx.block_height_cells.max(1),
        span_extra_cells: server_ctx.block_span_extra_cells.max(0),
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
            | BlockComponentKind::TableLegLeft
            | BlockComponentKind::TableLegRight
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
            | BlockComponentKind::TableLegLeft
            | BlockComponentKind::TableLegRight
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
    let extra = sizing.span_extra_cells.max(0) as f32;

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
        BlockComponentKind::TableLegLeft | BlockComponentKind::TableLegRight => {
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
            BlockComponentKind::DoorPostLeft | BlockComponentKind::TableLegLeft => {
                block_box.min.x -= extra;
                block_box.max.x -= extra;
            }
            BlockComponentKind::DoorPostRight | BlockComponentKind::TableLegRight => {
                block_box.min.x += extra;
                block_box.max.x += extra;
            }
            BlockComponentKind::Column
            | BlockComponentKind::ColumnBase
            | BlockComponentKind::ColumnShaft
            | BlockComponentKind::ColumnCapital => {}
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
            span_extra_cells: 1,
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
            span_extra_cells: 0,
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
            span_extra_cells: 0,
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
    fn table_adjusts_height_and_width_without_thickening_its_legs() {
        let asset = block_assets()
            .iter()
            .find(|asset| asset.name == "Table")
            .expect("Table block asset");
        let sizing = BlockSizing {
            height_cells: 3,
            span_extra_cells: 1,
        };

        let top = adjusted_block_box(asset, 0, sizing).unwrap();
        let left_front = adjusted_block_box(asset, 1, sizing).unwrap();
        let right_front = adjusted_block_box(asset, 2, sizing).unwrap();
        let left_back = adjusted_block_box(asset, 3, sizing).unwrap();
        let right_back = adjusted_block_box(asset, 4, sizing).unwrap();

        assert!(asset_supports_height(asset));
        assert!(asset_supports_width(asset));
        assert_close(top.min.x, -1.0);
        assert_close(top.max.x, 3.0);
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
    }
}
