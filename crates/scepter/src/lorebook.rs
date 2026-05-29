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
        for command in built_in_commands() {
            lorebook.commands.insert(command.name.clone(), command);
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
        ScepterCommandMeta::new(
            "tile_group.create",
            "Create a multi-tile group from existing tiles and optional tags.",
        )
        .params(vec![
            ScepterParamMeta::new("name", "Group name shown in Creator.", true, "string"),
            ScepterParamMeta::new("size", "Group width and height in cells.", true, "[integer; 2]"),
            ScepterParamMeta::new(
                "members",
                "Tiles and their x, y positions within the group.",
                false,
                "TileGroupMember[]",
            ),
            ScepterParamMeta::new("tags", "Optional searchable group tags.", false, "string[]"),
        ])
        .capabilities(vec![TileRead, TileWrite])
        .previewable()
        .undoable(),
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
        ScepterCommandMeta::new(
            "geometry.create_room",
            "Create a high-level 3D room primitive with optional floor and wall tiles.",
        )
        .capabilities(vec![RegionWrite, TileRead])
        .previewable()
        .undoable(),
        ScepterCommandMeta::new(
            "geometry.place_builder_asset",
            "Place a reusable builder asset or prop in a 3D region.",
        )
        .capabilities(vec![RegionWrite, ProjectRead])
        .previewable()
        .undoable(),
    ]
}
