use serde::{Deserialize, Serialize};

/// A command capability that an adapter can use for permissions and UI warnings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScepterCapability {
    ProjectRead,
    ProjectWrite,
    TileRead,
    TileWrite,
    RegionRead,
    RegionWrite,
    AttributeRead,
    AttributeWrite,
    ScriptRead,
    ScriptWrite,
    Preview,
    Undo,
    Export,
}

/// How a caller refers to a region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RegionRef {
    Id { id: String },
    Name { name: String },
}

impl RegionRef {
    pub fn name(name: impl Into<String>) -> Self {
        Self::Name { name: name.into() }
    }

    pub fn id(id: impl Into<String>) -> Self {
        Self::Id { id: id.into() }
    }
}

/// How a caller refers to a tile or asks Creator to resolve one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileSelector {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl TileSelector {
    pub fn alias(alias: impl Into<String>) -> Self {
        Self {
            alias: Some(alias.into()),
            ..Self::default()
        }
    }

    pub fn style_kind(style: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            style: Some(style.into()),
            kind: Some(kind.into()),
            ..Self::default()
        }
    }
}

/// A point on a 2D tile/grid plane.
pub type GridPoint = [i32; 2];

/// A 2D tile/grid rectangle: x, y, width, height.
pub type GridRect = [i32; 4];

/// A 3D point in region/world units.
pub type Point3 = [f32; 3];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionPaintRect {
    pub region: RegionRef,
    pub tile: TileSelector,
    pub rect: GridRect,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub select: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace_existing: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionPaintOutline {
    pub region: RegionRef,
    pub tile: TileSelector,
    pub rect: GridRect,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionPaintCells {
    pub region: RegionRef,
    pub tile: TileSelector,
    pub cells: Vec<GridPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub select: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace_existing: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionCreateSector {
    pub region: RegionRef,
    pub name: String,
    pub polygon: Vec<GridPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionPlaceItem {
    pub region: RegionRef,
    pub template: String,
    pub at: GridPoint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionPlaceCharacter {
    pub region: RegionRef,
    pub template: String,
    pub at: GridPoint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionRenderPreview {
    pub region: RegionRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds: Option<GridRect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoom: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionSnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<RegionRef>,
    #[serde(default)]
    pub include_tiles: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionSummary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<RegionRef>,
    #[serde(default = "default_true")]
    pub include_ascii: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileList {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileContactSheet {
    pub tiles: Vec<TileSelector>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileCreateFromRgba {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// Base64-encoded RGBA8 pixel data. A later adapter can also accept binary frames.
    pub rgba_base64: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub procedural_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub procedural_style: Option<String>,
    #[serde(default)]
    pub blocking: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileMetadataPatch {
    pub tile: TileSelector,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub procedural_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub procedural_style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub procedural_weight: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocking: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileGroupMember {
    pub tile: TileSelector,
    pub at: GridPoint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileGroupCreate {
    pub name: String,
    pub size: GridPoint,
    #[serde(default)]
    pub members: Vec<TileGroupMember>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TilesetInspect {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tileset: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TilesetGridDetect {
    pub tileset: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TilesetListUnimported {
    pub tileset: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TilesetTileMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub procedural_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub procedural_style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub procedural_weight: Option<u32>,
    #[serde(default)]
    pub blocking: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TilesetImportTile {
    pub tileset: String,
    pub cell: GridPoint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<TilesetTileMeta>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TilesetImportAnim {
    pub tileset: String,
    pub cells: Vec<GridPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<TilesetTileMeta>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TilesetImportMulti {
    pub tileset: String,
    pub rect: GridRect,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TilesetImportSpec {
    Tile {
        cell: GridPoint,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        meta: Option<TilesetTileMeta>,
    },
    Anim {
        cells: Vec<GridPoint>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        meta: Option<TilesetTileMeta>,
    },
    Multi {
        rect: GridRect,
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tags: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TilesetImportBatch {
    pub tileset: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_size: Option<GridPoint>,
    pub imports: Vec<TilesetImportSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptTargetKind {
    World,
    Region,
    Character,
    Item,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptTarget {
    pub kind: ScriptTargetKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<RegionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptGet {
    pub target: ScriptTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptPatch {
    pub target: ScriptTarget,
    pub patch: String,
    #[serde(default)]
    pub validate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptValidate {
    pub target: ScriptTarget,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttributesGet {
    pub target: ScriptTarget,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttributesPatch {
    pub target: ScriptTarget,
    #[serde(default)]
    pub values: serde_json::Map<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove: Vec<String>,
    #[serde(default)]
    pub validate: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeometryCreateRoom {
    pub region: RegionRef,
    pub name: String,
    pub rect: GridRect,
    pub height: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall_tile: Option<TileSelector>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor_tile: Option<TileSelector>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeometryPlaceBuilderAsset {
    pub region: RegionRef,
    pub asset: String,
    pub at: Point3,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<String>,
}

/// Typed Scepter commands. The serialized command names are the stable protocol names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", content = "params")]
pub enum ScepterCommand {
    #[serde(rename = "scepter.help")]
    ScepterHelp,
    #[serde(rename = "scepter.list_commands")]
    ScepterListCommands,
    #[serde(rename = "scepter.describe_command")]
    ScepterDescribeCommand { name: String },
    #[serde(rename = "project.describe")]
    ProjectDescribe,
    #[serde(rename = "project.undo")]
    ProjectUndo,
    #[serde(rename = "project.redo")]
    ProjectRedo,
    #[serde(rename = "region.list")]
    RegionList,
    #[serde(rename = "region.snapshot")]
    RegionSnapshot(RegionSnapshot),
    #[serde(rename = "region.summary")]
    RegionSummary(RegionSummary),
    #[serde(rename = "region.render_preview")]
    RegionRenderPreview(RegionRenderPreview),
    #[serde(rename = "region.paint_rect")]
    RegionPaintRect(RegionPaintRect),
    #[serde(rename = "region.paint_outline")]
    RegionPaintOutline(RegionPaintOutline),
    #[serde(rename = "region.paint_cells")]
    RegionPaintCells(RegionPaintCells),
    #[serde(rename = "region.create_sector")]
    RegionCreateSector(RegionCreateSector),
    #[serde(rename = "region.place_item")]
    RegionPlaceItem(RegionPlaceItem),
    #[serde(rename = "region.place_character")]
    RegionPlaceCharacter(RegionPlaceCharacter),
    #[serde(rename = "tile.list")]
    TileList(TileList),
    #[serde(rename = "tile.contact_sheet")]
    TileContactSheet(TileContactSheet),
    #[serde(rename = "tile.create_from_rgba")]
    TileCreateFromRgba(TileCreateFromRgba),
    #[serde(rename = "tile.set_meta")]
    TileSetMeta(TileMetadataPatch),
    #[serde(rename = "tile_group.create")]
    TileGroupCreate(TileGroupCreate),
    #[serde(rename = "tileset.list")]
    TilesetList,
    #[serde(rename = "tileset.inspect")]
    TilesetInspect(TilesetInspect),
    #[serde(rename = "tileset.grid_detect")]
    TilesetGridDetect(TilesetGridDetect),
    #[serde(rename = "tileset.list_unimported")]
    TilesetListUnimported(TilesetListUnimported),
    #[serde(rename = "tileset.import_tile")]
    TilesetImportTile(TilesetImportTile),
    #[serde(rename = "tileset.import_anim")]
    TilesetImportAnim(TilesetImportAnim),
    #[serde(rename = "tileset.import_multi")]
    TilesetImportMulti(TilesetImportMulti),
    #[serde(rename = "tileset.import_batch")]
    TilesetImportBatch(TilesetImportBatch),
    #[serde(rename = "script.get")]
    ScriptGet(ScriptGet),
    #[serde(rename = "script.patch")]
    ScriptPatch(ScriptPatch),
    #[serde(rename = "script.validate")]
    ScriptValidate(ScriptValidate),
    #[serde(rename = "attributes.get")]
    AttributesGet(AttributesGet),
    #[serde(rename = "attributes.patch")]
    AttributesPatch(AttributesPatch),
    #[serde(rename = "geometry.create_room")]
    GeometryCreateRoom(GeometryCreateRoom),
    #[serde(rename = "geometry.place_builder_asset")]
    GeometryPlaceBuilderAsset(GeometryPlaceBuilderAsset),
}

impl ScepterCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::ScepterHelp => "scepter.help",
            Self::ScepterListCommands => "scepter.list_commands",
            Self::ScepterDescribeCommand { .. } => "scepter.describe_command",
            Self::ProjectDescribe => "project.describe",
            Self::ProjectUndo => "project.undo",
            Self::ProjectRedo => "project.redo",
            Self::RegionList => "region.list",
            Self::RegionSnapshot(_) => "region.snapshot",
            Self::RegionSummary(_) => "region.summary",
            Self::RegionRenderPreview(_) => "region.render_preview",
            Self::RegionPaintRect(_) => "region.paint_rect",
            Self::RegionPaintOutline(_) => "region.paint_outline",
            Self::RegionPaintCells(_) => "region.paint_cells",
            Self::RegionCreateSector(_) => "region.create_sector",
            Self::RegionPlaceItem(_) => "region.place_item",
            Self::RegionPlaceCharacter(_) => "region.place_character",
            Self::TileList(_) => "tile.list",
            Self::TileContactSheet(_) => "tile.contact_sheet",
            Self::TileCreateFromRgba(_) => "tile.create_from_rgba",
            Self::TileSetMeta(_) => "tile.set_meta",
            Self::TileGroupCreate(_) => "tile_group.create",
            Self::TilesetList => "tileset.list",
            Self::TilesetInspect(_) => "tileset.inspect",
            Self::TilesetGridDetect(_) => "tileset.grid_detect",
            Self::TilesetListUnimported(_) => "tileset.list_unimported",
            Self::TilesetImportTile(_) => "tileset.import_tile",
            Self::TilesetImportAnim(_) => "tileset.import_anim",
            Self::TilesetImportMulti(_) => "tileset.import_multi",
            Self::TilesetImportBatch(_) => "tileset.import_batch",
            Self::ScriptGet(_) => "script.get",
            Self::ScriptPatch(_) => "script.patch",
            Self::ScriptValidate(_) => "script.validate",
            Self::AttributesGet(_) => "attributes.get",
            Self::AttributesPatch(_) => "attributes.patch",
            Self::GeometryCreateRoom(_) => "geometry.create_room",
            Self::GeometryPlaceBuilderAsset(_) => "geometry.place_builder_asset",
        }
    }
}
