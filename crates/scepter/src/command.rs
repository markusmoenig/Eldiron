use crate::world::*;
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
    ActionRead,
    ActionExecute,
    ToolRead,
    ToolSelect,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionRenderPreview {
    #[serde(default)]
    pub editor_view: bool,
    #[serde(default)]
    pub camera: Option<PreviewCamera>,
    /// Engine animation tick, using the project's normal tick cadence.
    #[serde(default)]
    pub animation_frame: Option<u32>,
    /// Engine camera: 2d, iso, or 3d.
    #[serde(default)]
    pub view: Option<String>,
    #[serde(default)]
    pub size: Option<[u32; 2]>,
    pub region: RegionRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds: Option<GridRect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoom: Option<f32>,
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionList {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default)]
    pub applicable_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRun {
    pub id: String,
    #[serde(default)]
    pub parameters_toml: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRunScript {
    pub source: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolList {
    #[serde(default)]
    pub include_hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSelect {
    pub id: String,
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

/// Requests with edits must supply the context from project.describe and a unique ID.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScepterRequest {
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub session: Option<String>,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(flatten)]
    pub command: ScepterCommand,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", content = "params", rename_all = "snake_case")]
pub enum EditOperation {
    SectorCreate(SectorCreate),
    EraseCells(EraseCells),
    MapTransform(MapTransform),
    MapCopy(MapTransform),
    MapDelete(MapDelete),
    GeometryCreate(GeometryCreate),
    WallCreate(WallCreate),
    WallUpdate(WallUpdate),
    SectorUpdate(SectorUpdate),
    OpeningPut(OpeningPut),
    OpeningDelete(OpeningDelete),
    RoomCreate(RoomCreate),
    EntityPlace(EntityPlace),
    EntityUpdate(EntityUpdate),
    MaterialPut {
        id: String,
        bundle: String,
    },
    CompositionPut {
        id: String,
        composition: serde_json::Value,
    },
    CompositionCopy {
        id: String,
        source: String,
    },
    SurfaceApply {
        region: RegionRef,
        surface: serde_json::Value,
        material: String,
    },
    PaintCells(RegionPaintCells),
    PaintRect(RegionPaintRect),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    pub operations: Vec<EditOperation>,
    #[serde(default)]
    pub dry_run: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetId {
    pub id: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialValidate {
    pub bundle: String,
}

/// Typed Scepter commands. The serialized command names are the stable protocol names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", content = "params")]
pub enum ScepterCommand {
    #[serde(rename = "editor.context")]
    EditorContext,
    #[serde(rename = "editor.select")]
    EditorSelect(EditorSelection),
    #[serde(rename = "editor.focus")]
    EditorFocus(EditorFocus),
    #[serde(rename = "project.changes")]
    ProjectChanges(ChangeQuery),
    #[serde(rename = "entity.templates")]
    EntityTemplates,
    #[serde(rename = "region.entities")]
    RegionEntities(RegionSnapshot),
    #[serde(rename = "project.transaction")]
    Transaction(Transaction),
    #[serde(rename = "material.list")]
    MaterialList,
    #[serde(rename = "material.get")]
    MaterialGet(AssetId),
    #[serde(rename = "material.validate")]
    MaterialValidate(MaterialValidate),
    #[serde(rename = "composition.get")]
    CompositionGet(AssetId),
    #[serde(rename = "surface.list")]
    SurfaceList(RegionSnapshot),
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
    #[serde(rename = "action.list")]
    ActionList(ActionList),
    #[serde(rename = "action.run")]
    ActionRun(ActionRun),
    #[serde(rename = "action.run_script")]
    ActionRunScript(ActionRunScript),
    #[serde(rename = "tool.list")]
    ToolList(ToolList),
    #[serde(rename = "tool.select")]
    ToolSelect(ToolSelect),
    #[serde(rename = "attributes.get")]
    AttributesGet(AttributesGet),
    #[serde(rename = "attributes.patch")]
    AttributesPatch(AttributesPatch),
    #[serde(rename = "geometry.create_room")]
    GeometryCreateRoom(GeometryCreateRoom),
}

impl ScepterCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::EditorContext => "editor.context",
            Self::EditorSelect(_) => "editor.select",
            Self::EditorFocus(_) => "editor.focus",
            Self::ProjectChanges(_) => "project.changes",
            Self::EntityTemplates => "entity.templates",
            Self::RegionEntities(_) => "region.entities",
            Self::Transaction(_) => "project.transaction",
            Self::MaterialList => "material.list",
            Self::MaterialGet(_) => "material.get",
            Self::MaterialValidate(_) => "material.validate",
            Self::CompositionGet(_) => "composition.get",
            Self::SurfaceList(_) => "surface.list",
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
            Self::ActionList(_) => "action.list",
            Self::ActionRun(_) => "action.run",
            Self::ActionRunScript(_) => "action.run_script",
            Self::ToolList(_) => "tool.list",
            Self::ToolSelect(_) => "tool.select",
            Self::AttributesGet(_) => "attributes.get",
            Self::AttributesPatch(_) => "attributes.patch",
            Self::GeometryCreateRoom(_) => "geometry.create_room",
        }
    }
}

impl ScepterCommand {
    pub fn is_mutation(&self) -> bool {
        matches!(
            self,
            Self::Transaction(Transaction { dry_run: false, .. })
                | Self::ProjectUndo
                | Self::ProjectRedo
                | Self::RegionPaintRect(_)
                | Self::RegionPaintCells(_)
                | Self::ScriptPatch(_)
                | Self::AttributesPatch(_)
                | Self::ActionRun(_)
                | Self::ActionRunScript(_)
                | Self::ToolSelect(_)
                | Self::EditorSelect(_)
                | Self::EditorFocus(_)
                | Self::RegionCreateSector(_)
                | Self::RegionPlaceItem(_)
                | Self::RegionPlaceCharacter(_)
                | Self::GeometryCreateRoom(_)
        )
    }
}
/// Adapter capabilities, including explicit status for historical proposals.
pub fn creator_implements(name: &str) -> bool {
    matches!(
        name,
        "editor.context"
            | "editor.select"
            | "editor.focus"
            | "project.changes"
            | "entity.templates"
            | "region.entities"
            | "region.create_sector"
            | "region.place_item"
            | "region.place_character"
            | "geometry.create_room"
            | "scepter.help"
            | "scepter.list_commands"
            | "scepter.describe_command"
            | "project.describe"
            | "project.undo"
            | "project.redo"
            | "project.transaction"
            | "region.list"
            | "region.snapshot"
            | "region.summary"
            | "region.render_preview"
            | "region.paint_rect"
            | "region.paint_cells"
            | "tile.list"
            | "script.get"
            | "script.patch"
            | "script.validate"
            | "action.list"
            | "action.run"
            | "action.run_script"
            | "tool.list"
            | "tool.select"
            | "attributes.get"
            | "attributes.patch"
            | "material.list"
            | "material.get"
            | "material.validate"
            | "composition.get"
            | "surface.list"
    )
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewCamera {
    #[serde(default)]
    pub azimuth: Option<f32>,
    #[serde(default)]
    pub elevation: Option<f32>,
    #[serde(default)]
    pub fov: Option<f32>,
    #[serde(default)]
    pub pitch: Option<f32>,
    pub center: [f32; 3],
    #[serde(default)]
    pub position: Option<[f32; 3]>,
    #[serde(default)]
    pub scale: Option<f32>,
}
