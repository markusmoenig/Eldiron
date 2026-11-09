use crate::ScepterCapability;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;

/// One command parameter entry in the Scepter Lorebook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScepterParamMeta {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub schema: String,
}

impl ScepterParamMeta {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
        schema: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            required,
            schema: schema.into(),
        }
    }
}

/// Machine-readable help for a single Scepter command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScepterCommandMeta {
    #[serde(default)]
    pub implemented: bool,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub params: Vec<ScepterParamMeta>,
    #[serde(default)]
    pub previewable: bool,
    #[serde(default)]
    pub undoable: bool,
    #[serde(default)]
    pub capabilities: Vec<ScepterCapability>,
    #[serde(default)]
    pub examples: Vec<serde_json::Value>,
}

impl ScepterCommandMeta {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            implemented: false,
            description: description.into(),
            params: Vec::new(),
            previewable: false,
            undoable: false,
            capabilities: Vec::new(),
            examples: Vec::new(),
        }
    }

    pub fn params(mut self, params: Vec<ScepterParamMeta>) -> Self {
        self.params = params;
        self
    }

    pub fn capabilities(mut self, capabilities: Vec<ScepterCapability>) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn previewable(mut self) -> Self {
        self.previewable = true;
        self
    }

    pub fn undoable(mut self) -> Self {
        self.undoable = true;
        self
    }

    pub fn examples(mut self, examples: Vec<serde_json::Value>) -> Self {
        self.examples = examples;
        self
    }
}

/// The live Scepter command catalog.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScepterLorebook {
    pub commands: BTreeMap<String, ScepterCommandMeta>,
}

impl ScepterLorebook {
    pub fn built_in() -> Self {
        let mut lorebook = Self::default();
        for mut command in built_in_commands() {
            command.implemented = crate::creator_implements(&command.name);
            lorebook.commands.insert(command.name.clone(), command);
        }
        for (name, description) in [
            (
                "editor.context",
                "Read current region, viewport, tool, camera, selection, stream and change cursor.",
            ),
            (
                "editor.select",
                "Select explicit map target IDs. Requires request_id, session and revision. params: {region?,targets,vertices?,linedefs?,faces?,geometry_vertices?}.",
            ),
            (
                "editor.focus",
                "Focus a region and camera. params: {region?,view?,center:[x,y,z],position?,scale?}. Requires edit context.",
            ),
            (
                "project.changes",
                "Bounded change feed. params: {after:sequence,stream?:UUID,limit?:64}. Handle reset_required by reading context/snapshots.",
            ),
            (
                "entity.templates",
                "List project/ruleset item and character templates and setup TOML.",
            ),
            (
                "region.entities",
                "Read placed character/item instances with positions, template IDs and TOML. params: {region?}.",
            ),
            (
                "project.transaction",
                "Atomic material, composition, surface, sector, geometry, wall/opening, room, entity and paint operations. Prepares off the UI thread, rechecks revision, one undo. Requires request_id, session and revision. params: {operations:[{op,params}],dry_run:false}.",
            ),
            (
                "material.list",
                "Project palette and material/composition IDs.",
            ),
            (
                "material.get",
                "Read editable MZ bundle, composition and map binding. params: {id:UUID}.",
            ),
            (
                "material.validate",
                "Validate an MZ PBR bundle against the project palette. params: {bundle:string}.",
            ),
            (
                "composition.get",
                "Read layered composition and map binding. params: {id:UUID}.",
            ),
            (
                "surface.list",
                "Read map geometry and material bindings. params: {region:{id:UUID}}.",
            ),
        ] {
            let mut meta = ScepterCommandMeta::new(name, description);
            meta.implemented = true;
            meta.capabilities = if matches!(
                name,
                "project.transaction" | "editor.select" | "editor.focus"
            ) {
                if name == "project.transaction" {
                    vec![
                        ScepterCapability::ProjectWrite,
                        ScepterCapability::RegionWrite,
                        ScepterCapability::Undo,
                    ]
                } else {
                    vec![ScepterCapability::ProjectWrite]
                }
            } else {
                vec![ScepterCapability::ProjectRead]
            };
            meta.undoable = name == "project.transaction";
            meta.params = match name {
                "project.transaction" => vec![
                    ScepterParamMeta::new(
                        "operations",
                        "1–128 typed {op,params} operations. Ops: material_put, composition_put/copy, surface_apply, paint_cells/rect, erase_cells, sector_create/update, geometry_create, entity_place/update, map_copy/transform/delete. Map targets: sectors(integer IDs), geometry/characters/items(UUIDs). 3D uses X/Y/Z, 2D X/Z; rotation_y is degrees.",
                        true,
                        "array",
                    ),
                    ScepterParamMeta::new(
                        "dry_run",
                        "Validate without publishing or undo",
                        false,
                        "boolean",
                    ),
                ],
                "project.changes" => vec![
                    ScepterParamMeta::new("after", "Last consumed sequence", false, "integer"),
                    ScepterParamMeta::new(
                        "stream",
                        "Stream UUID from editor.context; handle reset_required",
                        false,
                        "string",
                    ),
                    ScepterParamMeta::new("limit", "1–128 events (default 64)", false, "integer"),
                ],
                "editor.select" => vec![
                    ScepterParamMeta::new(
                        "targets",
                        "Region map targets: sectors, geometry, walls, characters, items",
                        true,
                        "object",
                    ),
                    ScepterParamMeta::new(
                        "region",
                        "{id:UUID} or {name:unique name}; active region by default",
                        false,
                        "object",
                    ),
                    ScepterParamMeta::new(
                        "faces",
                        "[object UUID, face index] pairs; also supports geometry_vertices and vertices/linedefs",
                        false,
                        "array",
                    ),
                ],
                "editor.focus" => vec![
                    ScepterParamMeta::new(
                        "center",
                        "World X/Y/Z focus",
                        true,
                        "[number,number,number]",
                    ),
                    ScepterParamMeta::new(
                        "view",
                        "2d, 2d_iso, iso, orbit or firstp",
                        false,
                        "string",
                    ),
                    ScepterParamMeta::new("region", "Region reference", false, "object"),
                    ScepterParamMeta::new(
                        "position",
                        "3D camera anchor",
                        false,
                        "[number,number,number]",
                    ),
                    ScepterParamMeta::new("scale", "2D zoom or isometric scale", false, "number"),
                ],
                "region.entities" | "surface.list" => vec![ScepterParamMeta::new(
                    "region",
                    "Region reference",
                    true,
                    "object",
                )],
                _ => vec![],
            };

            lorebook.commands.insert(name.into(), meta);
        }
        lorebook
    }

    pub fn list_commands(&self) -> Vec<&str> {
        self.commands.keys().map(String::as_str).collect()
    }

    pub fn describe_command(&self, name: &str) -> Option<&ScepterCommandMeta> {
        self.commands.get(name)
    }
}

pub fn built_in_commands() -> Vec<ScepterCommandMeta> {
    use ScepterCapability::*;

    vec![
        ScepterCommandMeta::new(
            "scepter.help",
            "Return high-level help for Eldiron Scepter and its Lorebook.",
        )
        .capabilities(vec![ProjectRead]),
        ScepterCommandMeta::new(
            "scepter.list_commands",
            "List all commands currently exposed by the Scepter Lorebook.",
        )
        .capabilities(vec![ProjectRead]),
        ScepterCommandMeta::new(
            "scepter.describe_command",
            "Describe one command, including parameters, examples, and capabilities.",
        )
        .params(vec![ScepterParamMeta::new(
            "name",
            "Stable command name such as region.paint_rect.",
            true,
            "string",
        )])
        .capabilities(vec![ProjectRead]),
        ScepterCommandMeta::new("project.describe", "Describe the open Eldiron project.")
            .capabilities(vec![ProjectRead]),
        ScepterCommandMeta::new("project.undo", "Undo the last undoable Creator change.")
            .capabilities(vec![Undo, ProjectWrite])
            .undoable(),
        ScepterCommandMeta::new("project.redo", "Redo the next redoable Creator change.")
            .capabilities(vec![Undo, ProjectWrite])
            .undoable(),
        ScepterCommandMeta::new("region.list", "List regions in the open project.")
            .capabilities(vec![RegionRead]),
        ScepterCommandMeta::new(
            "region.snapshot",
            "Read a normalized 2D authoring snapshot for a region, including sectors, linedefs, vertices, material sources, and resolved tile metadata.",
        )
        .params(vec![
            ScepterParamMeta::new(
                "region",
                "Optional region id or name. Defaults to Creator's current region.",
                false,
                "RegionRef",
            ),
            ScepterParamMeta::new(
                "include_tiles",
                "Include a project tile lookup table in the response.",
                false,
                "boolean",
            ),
        ])
        .capabilities(vec![RegionRead, TileRead])
        .examples(vec![json!({
            "command": "region.snapshot",
            "params": {
                "region": { "name": "Harbor" },
                "include_tiles": true
            }
        })]),
        ScepterCommandMeta::new(
            "region.summary",
            "Read a compact AI-oriented 2D map understanding summary with bounds, landmarks, source usage, blocking/walkable roles, entities, items, and a coarse visual layout.",
        )
        .params(vec![
            ScepterParamMeta::new(
                "region",
                "Optional region id or name. Defaults to Creator's current region.",
                false,
                "RegionRef",
            ),
            ScepterParamMeta::new(
                "include_ascii",
                "Include a coarse ASCII overview of tile roles and placed actors/items.",
                false,
                "boolean",
            ),
        ])
        .capabilities(vec![RegionRead, TileRead])
        .examples(vec![json!({
            "command": "region.summary",
            "params": {
                "region": { "name": "Dungeon" },
                "include_ascii": true
            }
        })]),
        ScepterCommandMeta::new(
            "region.render_preview",
            "Render a preview image for a region or region bounds.",
        )
        .params(vec![
            ScepterParamMeta::new("region", "Region id or name.", true, "RegionRef"),
            ScepterParamMeta::new(
                "bounds",
                "Optional x, y, width, height region bounds.",
                false,
                "[integer; 4]",
            ),
            ScepterParamMeta::new("zoom", "Optional integer preview zoom.", false, "integer"),
        ])
        .capabilities(vec![RegionRead, Preview])
        .previewable()
        .examples(vec![json!({
            "command": "region.render_preview",
            "params": {
                "region": { "name": "Harbor" },
                "bounds": [-12, -24, 20, 16],
                "zoom": 2
            }
        })]),
        ScepterCommandMeta::new(
            "region.paint_rect",
            "Paint a rectangular area in a 2D region using a tile selector.",
        )
        .params(vec![
            ScepterParamMeta::new("region", "Region id or name.", true, "RegionRef"),
            ScepterParamMeta::new(
                "tile",
                "Tile id, alias, or semantic query.",
                true,
                "TileSelector",
            ),
            ScepterParamMeta::new("rect", "x, y, width, height.", true, "[integer; 4]"),
            ScepterParamMeta::new("layer", "Optional generated layer name.", false, "string"),
            ScepterParamMeta::new(
                "select",
                "Select painted sectors in Creator after applying. Defaults to false.",
                false,
                "boolean",
            ),
            ScepterParamMeta::new(
                "replace_existing",
                "Clear existing drawable tile sectors overlapping target cells before painting. Defaults to true.",
                false,
                "boolean",
            ),
        ])
        .capabilities(vec![RegionWrite, TileRead])
        .previewable()
        .undoable()
        .examples(vec![json!({
            "command": "region.paint_rect",
            "params": {
                "region": { "name": "Harbor" },
                "tile": { "alias": "stone_floor_dark" },
                "rect": [4, 4, 12, 8]
            }
        })]),
        ScepterCommandMeta::new(
            "region.paint_outline",
            "Paint the outline of a rectangular area in a 2D region.",
        )
        .capabilities(vec![RegionWrite, TileRead])
        .previewable()
        .undoable(),
        ScepterCommandMeta::new(
            "region.paint_cells",
            "Paint explicit 2D grid cells in a region, replacing existing cells by default.",
        )
        .params(vec![
            ScepterParamMeta::new("region", "Region id or name.", true, "RegionRef"),
            ScepterParamMeta::new(
                "tile",
                "Tile id, alias, or semantic query.",
                true,
                "TileSelector",
            ),
            ScepterParamMeta::new("cells", "Array of [x, y] grid cells.", true, "[[integer; 2]]"),
            ScepterParamMeta::new("layer", "Optional generated layer name.", false, "string"),
            ScepterParamMeta::new(
                "select",
                "Select painted sectors in Creator after applying. Defaults to false.",
                false,
                "boolean",
            ),
            ScepterParamMeta::new(
                "replace_existing",
                "Clear existing drawable tile sectors overlapping target cells before painting. Defaults to true.",
                false,
                "boolean",
            ),
        ])
        .capabilities(vec![RegionWrite, TileRead])
        .previewable()
        .undoable()
        .examples(vec![json!({
            "command": "region.paint_cells",
            "params": {
                "region": { "name": "Harbor" },
                "tile": { "role": "Road" },
                "cells": [[-3, -14], [-3, -13], [-3, -12]],
                "replace_existing": true
            }
        })]),
        ScepterCommandMeta::new(
            "region.create_sector",
            "Create a named gameplay sector from a polygon.",
        )
        .capabilities(vec![RegionWrite])
        .previewable()
        .undoable(),
        ScepterCommandMeta::new(
            "region.place_item",
            "Place a region-local item instance from an item template.",
        )
        .capabilities(vec![RegionWrite, ProjectRead])
        .previewable()
        .undoable(),
        ScepterCommandMeta::new(
            "region.place_character",
            "Place a region-local character instance from a character template.",
        )
        .capabilities(vec![RegionWrite, ProjectRead])
        .previewable()
        .undoable(),
        ScepterCommandMeta::new("tile.list", "List tiles by role, style, kind, or metadata.")
            .capabilities(vec![TileRead]),
        ScepterCommandMeta::new(
            "tile.contact_sheet",
            "Render a contact sheet for visual tile selection.",
        )
        .capabilities(vec![TileRead, Preview])
        .previewable(),
        ScepterCommandMeta::new(
            "tile.create_from_rgba",
            "Create a tile from RGBA8 pixel data and optional metadata.",
        )
        .capabilities(vec![TileWrite])
        .previewable()
        .undoable(),
        ScepterCommandMeta::new(
            "tile.set_meta",
            "Update tile metadata such as alias, role, blocking, and procedural tags.",
        )
        .params(vec![
            ScepterParamMeta::new("tile", "Tile id, alias, or semantic query.", true, "TileSelector"),
            ScepterParamMeta::new("alias", "Optional replacement alias.", false, "string"),
            ScepterParamMeta::new("role", "Optional Creator tile role.", false, "string"),
            ScepterParamMeta::new(
                "procedural_kind",
                "Optional procedural kind such as floor, wall, door, tree, or prop.",
                false,
                "string",
            ),
            ScepterParamMeta::new(
                "procedural_style",
                "Optional procedural style such as stone, forest, crypt, or town.",
                false,
                "string",
            ),
            ScepterParamMeta::new("blocking", "Optional 2D collision flag.", false, "boolean"),
        ])
        .capabilities(vec![TileRead, TileWrite])
        .previewable()
        .undoable()
        .examples(vec![json!({
            "command": "tile.set_meta",
            "params": {
                "tile": { "alias": "stone_floor_02" },
                "role": "Dungeon",
                "procedural_kind": "floor",
                "procedural_style": "crypt",
                "blocking": false
            }
        })]),
        ScepterCommandMeta::new("tileset.list", "List tilesets available to Creator.")
            .capabilities(vec![TileRead]),
        ScepterCommandMeta::new(
            "tileset.inspect",
            "Inspect a tileset image, its grid metadata, and imported tile coverage.",
        )
        .params(vec![ScepterParamMeta::new(
            "tileset",
            "Optional tileset id, filename, or display name.",
            false,
            "string",
        )])
        .capabilities(vec![TileRead, Preview])
        .previewable(),
        ScepterCommandMeta::new(
            "tileset.grid_detect",
            "Detect or suggest a tileset grid for cell-based import.",
        )
        .params(vec![ScepterParamMeta::new(
            "tileset",
            "Tileset id, filename, or display name.",
            true,
            "string",
        )])
        .capabilities(vec![TileRead, Preview])
        .previewable(),
        ScepterCommandMeta::new(
            "tileset.list_unimported",
            "List tileset cells or ranges that have not yet been imported into the tile system.",
        )
        .params(vec![ScepterParamMeta::new(
            "tileset",
            "Tileset id, filename, or display name.",
            true,
            "string",
        )])
        .capabilities(vec![TileRead]),
        ScepterCommandMeta::new(
            "tileset.import_tile",
            "Import one tileset cell as a tile with optional metadata.",
        )
        .params(vec![
            ScepterParamMeta::new("tileset", "Tileset id, filename, or display name.", true, "string"),
            ScepterParamMeta::new("cell", "Cell x and y in the tileset grid.", true, "[integer; 2]"),
            ScepterParamMeta::new("meta", "Optional tile metadata.", false, "TilesetTileMeta"),
        ])
        .capabilities(vec![TileRead, TileWrite])
        .previewable()
        .undoable(),
        ScepterCommandMeta::new(
            "tileset.import_anim",
            "Import multiple tileset cells as one animated tile.",
        )
        .params(vec![
            ScepterParamMeta::new("tileset", "Tileset id, filename, or display name.", true, "string"),
            ScepterParamMeta::new("cells", "Ordered animation cells.", true, "[integer; 2][]"),
            ScepterParamMeta::new("meta", "Optional tile metadata.", false, "TilesetTileMeta"),
        ])
        .capabilities(vec![TileRead, TileWrite])
        .previewable()
        .undoable(),
        ScepterCommandMeta::new(
            "tileset.import_multi",
            "Import a rectangular tileset range as a multi-tile group.",
        )
        .params(vec![
            ScepterParamMeta::new("tileset", "Tileset id, filename, or display name.", true, "string"),
            ScepterParamMeta::new("rect", "x, y, width, height in tileset cells.", true, "[integer; 4]"),
            ScepterParamMeta::new("name", "Created group name.", true, "string"),
            ScepterParamMeta::new("tags", "Optional searchable group tags.", false, "string[]"),
        ])
        .capabilities(vec![TileRead, TileWrite])
        .previewable()
        .undoable(),
        ScepterCommandMeta::new(
            "tileset.import_batch",
            "Import a planned batch of single tiles, animations, and multi-tile groups from one tileset.",
        )
        .params(vec![
            ScepterParamMeta::new("tileset", "Tileset id, filename, or display name.", true, "string"),
            ScepterParamMeta::new("grid_size", "Optional cell width and height override.", false, "[integer; 2]"),
            ScepterParamMeta::new("imports", "Tile, animation, and multi import specs.", true, "TilesetImportSpec[]"),
        ])
        .capabilities(vec![TileRead, TileWrite])
        .previewable()
        .undoable()
        .examples(vec![json!({
            "command": "tileset.import_batch",
            "params": {
                "tileset": "dungeon_a.png",
                "grid_size": [32, 32],
                "imports": [
                    {
                        "kind": "tile",
                        "cell": [4, 2],
                        "meta": {
                            "alias": "crypt_floor_cracked",
                            "role": "Dungeon",
                            "procedural_kind": "floor",
                            "procedural_style": "crypt"
                        }
                    },
                    {
                        "kind": "multi",
                        "rect": [8, 2, 2, 2],
                        "name": "crypt_pillar_cluster",
                        "tags": ["crypt", "pillar"]
                    }
                ]
            }
        })]),
        ScepterCommandMeta::new("script.get", "Read an Eldrin script from a target.")
            .params(vec![ScepterParamMeta::new(
                "target",
                "World, region, character, or item target. Character/item targets use project templates unless a region is supplied.",
                true,
                "ScriptTarget",
            )])
            .capabilities(vec![ScriptRead])
            .examples(vec![json!({
                "command": "script.get",
                "params": {
                    "target": {
                        "kind": "character",
                        "region": { "name": "Harbor" },
                        "name": "Old Smuggler"
                    }
                }
            })]),
        ScepterCommandMeta::new(
            "script.patch",
            "Apply a patch to an Eldrin script, optionally validating before apply.",
        )
        .params(vec![
            ScepterParamMeta::new(
                "target",
                "World, region, character, or item target.",
                true,
                "ScriptTarget",
            ),
            ScepterParamMeta::new(
                "patch",
                "Replacement Eldrin source for this first executable version.",
                true,
                "string",
            ),
            ScepterParamMeta::new(
                "validate",
                "Parse/validate before applying when validation support exists.",
                false,
                "boolean",
            ),
        ])
        .capabilities(vec![ScriptRead, ScriptWrite])
        .previewable()
        .undoable()
        .examples(vec![json!({
            "command": "script.patch",
            "params": {
                "target": { "kind": "item", "name": "Sign" },
                "patch": "on examine {\n    say(\"Weathered letters mark the path.\")\n}",
                "validate": true
            }
        })]),
        ScepterCommandMeta::new(
            "script.validate",
            "Validate Eldrin source for a target without applying it.",
        )
        .capabilities(vec![ScriptRead, Preview])
        .previewable(),
        ScepterCommandMeta::new(
            "action.list",
            "List Creator actions using stable non-localized ids, groups, applicability, and UI metadata.",
        )
        .params(vec![
            ScepterParamMeta::new(
                "group",
                "Optional stable group id such as camera, bake, face, or tile.",
                false,
                "string",
            ),
            ScepterParamMeta::new(
                "applicable_only",
                "Only return actions applicable to Creator's current context.",
                false,
                "boolean",
            ),
        ])
        .capabilities(vec![ActionRead])
        .examples(vec![json!({
            "command": "action.list",
            "params": { "group": "face", "applicable_only": true }
        })]),
        ScepterCommandMeta::new(
            "action.run",
            "Run one Creator action by stable id through its normal applicability, parameter, undo, and project hooks.",
        )
        .params(vec![
            ScepterParamMeta::new(
                "id",
                "Stable action id returned by action.list.",
                true,
                "string",
            ),
            ScepterParamMeta::new(
                "parameters_toml",
                "Optional action parameters using the same TOML representation as Creator's sidebar.",
                false,
                "string",
            ),
        ])
        .capabilities(vec![ActionExecute, ProjectWrite])
        .undoable()
        .examples(vec![json!({
            "command": "action.run",
            "params": {
                "id": "face.extrude",
                "parameters_toml": "amount = 2"
            }
        })]),
        ScepterCommandMeta::new(
            "action.run_script",
            "Run an Eldrin editor-automation script containing editor_action(id, parameters_toml) and editor_tool(id) calls.",
        )
        .params(vec![ScepterParamMeta::new(
            "source",
            "Eldrin source executed by the Creator action host.",
            true,
            "string",
        )])
        .capabilities(vec![ActionExecute, ToolSelect, ScriptRead, ProjectWrite])
        .undoable()
        .examples(vec![json!({
            "command": "action.run_script",
            "params": {
                "source": "editor_tool(\"tool.geometry\");\neditor_action(\"camera.isometric\", \"\");\neditor_action(\"face.extrude\", \"amount = 2\");"
            }
        })]),
        ScepterCommandMeta::new(
            "tool.list",
            "List Creator's long-lived interaction tools using stable non-localized ids.",
        )
        .params(vec![ScepterParamMeta::new(
            "include_hidden",
            "Include tools that are currently unavailable in the active editor context.",
            false,
            "boolean",
        )])
        .capabilities(vec![ToolRead]),
        ScepterCommandMeta::new(
            "tool.select",
            "Select a Creator interaction tool through its normal deactivate/activate lifecycle.",
        )
        .params(vec![ScepterParamMeta::new(
            "id",
            "Stable tool id returned by tool.list.",
            true,
            "string",
        )])
        .capabilities(vec![ToolSelect])
        .examples(vec![json!({
            "command": "tool.select",
            "params": { "id": "tool.geometry" }
        })]),
        ScepterCommandMeta::new(
            "attributes.get",
            "Read TOML attributes from a character or item template/instance.",
        )
        .params(vec![ScepterParamMeta::new(
            "target",
            "Character or item target. Add region to select a placed instance.",
            true,
            "ScriptTarget",
        )])
        .capabilities(vec![AttributeRead])
        .examples(vec![json!({
            "command": "attributes.get",
            "params": {
                "target": { "kind": "character", "name": "Orc" }
            }
        })]),
        ScepterCommandMeta::new(
            "attributes.patch",
            "Patch the [attributes] table of a character or item template/instance.",
        )
        .params(vec![
            ScepterParamMeta::new(
                "target",
                "Character or item target. Add region to select a placed instance.",
                true,
                "ScriptTarget",
            ),
            ScepterParamMeta::new(
                "values",
                "Attribute key/value pairs to set under [attributes]. JSON strings, numbers, booleans, arrays, and objects are converted to TOML.",
                false,
                "object",
            ),
            ScepterParamMeta::new(
                "remove",
                "Attribute keys to remove from [attributes].",
                false,
                "string[]",
            ),
            ScepterParamMeta::new(
                "validate",
                "Validate resulting TOML before applying.",
                false,
                "boolean",
            ),
        ])
        .capabilities(vec![AttributeRead, AttributeWrite])
        .previewable()
        .undoable()
        .examples(vec![json!({
            "command": "attributes.patch",
            "params": {
                "target": {
                    "kind": "character",
                    "region": { "name": "Harbor" },
                    "name": "Harbor Lookout"
                },
                "values": {
                    "faction": "dock_watch",
                    "visible": true,
                    "radius": 0.5
                },
                "remove": ["temporary_note"],
                "validate": true
            }
        })]),

    ]
}
