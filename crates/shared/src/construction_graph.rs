//! Reusable, typed construction patterns. The graph is an authoring asset; its projection onto a
//! surface is saved with the map so headless clients never need an editor graph to render it.
use crate::project::Project;
use rusterix::{
    PixelSource,
    map::wall::{WallAreaSurface, WallMasonryPattern, WallStyle, WallSurfaceSubdivision},
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use theframework::prelude::{TheColor, Uuid};
use theframework::thegraph::{
    GraphConnection, GraphControlValue, GraphDefinitions, GraphDocument, GraphNode,
    GraphNodeDefinition, GraphPort, GraphRow, PortDirection, PortSide,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConstructionPatternAsset {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub kind: ConstructionPatternKind,
    pub graph: GraphDocument,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum ConstructionPatternKind {
    #[default]
    Surface,
    Wall,
    Pattern,
}

impl ConstructionPatternAsset {
    pub fn new(name: impl Into<String>) -> Self {
        Self::new_surface(name, 0.25, 0.08)
    }

    pub fn new_surface(name: impl Into<String>, elevation: f32, thickness: f32) -> Self {
        let name = name.into();
        let mut graph = surface_graph(elevation, thickness);
        set_root_name(&mut graph, &name);
        tidy_graph(&mut graph);
        Self {
            id: Uuid::new_v4(),
            name,
            kind: ConstructionPatternKind::Surface,
            graph,
        }
    }

    /// An editable starter branch fitted to the newly authored surface.
    pub fn from_surface(name: impl Into<String>, surface: &WallAreaSurface) -> Self {
        let mut asset = Self::new_surface(name, surface.elevation, surface.thickness);
        let defs = definitions();
        let old_material = asset.graph.nodes[1].ports[0].id;
        asset
            .graph
            .connections
            .retain(|connection| connection.from != old_material);
        asset.graph.nodes[1] = defs.node("noise").unwrap().instantiate([300.0, 200.0]);
        let noise = asset.graph.nodes[1].clone();
        let default_material = defs.node("gradient").unwrap().instantiate([580.0, 200.0]);
        connect(
            &mut asset.graph,
            &noise,
            "value",
            &default_material,
            "value_in",
        );
        let root = asset.graph.nodes[0].clone();
        if let Some(scale) = surface.texture_scale {
            if let Some(row) = asset.graph.nodes[2]
                .rows
                .iter_mut()
                .find(|row| row.key.as_deref() == Some("texture_scale"))
            {
                if let GraphControlValue::Number { value, .. } = &mut row.value {
                    *value = scale;
                }
            }
        }
        let output = asset.graph.nodes[2].clone();
        connect(
            &mut asset.graph,
            &default_material,
            "material",
            &output,
            "top",
        );
        connect(
            &mut asset.graph,
            &default_material,
            "material",
            &output,
            "sides",
        );
        let mut subdivision = defs.node("subdivide").unwrap().instantiate([300.0, 0.0]);
        if let Some(row) = subdivision
            .rows
            .iter_mut()
            .find(|row| row.key.as_deref() == Some("cell_size"))
        {
            if let GraphControlValue::Number { value, .. } = &mut row.value {
                *value = 0.5;
            }
        }
        asset.graph.connections.retain(|connection| {
            connection.to
                != output
                    .ports
                    .iter()
                    .find(|port| port.key.as_deref() == Some("geometry"))
                    .unwrap()
                    .id
        });
        connect(
            &mut asset.graph,
            &root,
            "geometry",
            &subdivision,
            "geometry_in",
        );
        connect(
            &mut asset.graph,
            &subdivision,
            "geometry",
            &output,
            "geometry",
        );
        asset.graph.nodes.push(subdivision);
        asset.graph.nodes.push(default_material);
        for (key, source, y) in [
            ("top", surface.source.as_ref(), 200.0),
            ("sides", surface.side_source.as_ref(), 400.0),
        ] {
            let Some(source) = source else { continue };
            let (definition, kind, value) = match source {
                PixelSource::Color(color) => {
                    let rgba = color.to_u8_array();
                    (
                        "color",
                        "color",
                        format!("#{:02x}{:02x}{:02x}", rgba[0], rgba[1], rgba[2]),
                    )
                }
                PixelSource::PaletteIndex(index) => ("color", "palette", index.to_string()),
                PixelSource::TileId(id) => ("tile", "tile", id.to_string()),
                _ => continue,
            };
            let mut node = defs.node(definition).unwrap().instantiate([300.0, y]);
            node.rows[0].value = GraphControlValue::Custom {
                kind: kind.into(),
                data: value.into(),
            };
            if key == "top" {
                let gradient_id = asset
                    .graph
                    .nodes
                    .iter()
                    .find(|node| node.definition.as_deref() == Some("gradient"))
                    .unwrap()
                    .id;
                let ports: HashSet<_> = asset
                    .graph
                    .nodes
                    .iter()
                    .filter(|node| {
                        node.id == gradient_id || node.definition.as_deref() == Some("noise")
                    })
                    .flat_map(|node| node.ports.iter().map(|port| port.id))
                    .collect();
                asset.graph.connections.retain(|connection| {
                    !ports.contains(&connection.from) && !ports.contains(&connection.to)
                });
                asset.graph.nodes.retain(|node| {
                    node.id != gradient_id && node.definition.as_deref() != Some("noise")
                });
            }
            let target = output
                .ports
                .iter()
                .find(|port| port.key.as_deref() == Some(key))
                .unwrap()
                .id;
            asset
                .graph
                .connections
                .retain(|connection| connection.to != target);
            connect(&mut asset.graph, &node, "material", &output, key);
            asset.graph.nodes.push(node);
        }
        tidy_graph(&mut asset.graph);
        asset
    }

    pub fn new_wall(name: impl Into<String>, style: &WallStyle) -> Self {
        let name = name.into();
        let mut graph = default_wall_graph(style);
        set_root_name(&mut graph, &name);
        tidy_graph(&mut graph);
        Self {
            id: Uuid::new_v4(),
            name,
            kind: ConstructionPatternKind::Wall,
            graph,
        }
    }

    pub fn new_pattern(name: impl Into<String>, style: &WallStyle) -> Self {
        let name = name.into();
        let mut graph = pattern_graph(style);
        set_root_name(&mut graph, &name);
        tidy_graph(&mut graph);
        Self {
            id: Uuid::new_v4(),
            name,
            kind: ConstructionPatternKind::Pattern,
            graph,
        }
    }
}

pub fn tidy_graph(graph: &mut GraphDocument) -> bool {
    let nodes = graph.nodes.iter().map(|node| node.id).collect();
    theframework::thegraph::layout_branch(graph, &nodes)
}

/// Upgrade the short-lived material-output Noise nodes without losing saved connections/settings.
pub fn normalize_noise_graph(graph: &mut GraphDocument) {
    let defs = definitions();
    let mut additions = Vec::new();
    for noise in &mut graph.nodes {
        if noise.definition.as_deref() != Some("noise") {
            continue;
        }
        let Some(port) = noise
            .ports
            .iter_mut()
            .find(|port| port.kind == "surface.material")
        else {
            continue;
        };
        let old_output = port.id;
        port.kind = "surface.value".into();
        port.key = Some("value".into());
        port.label = "Value".into();
        let mut gradient = defs
            .node("gradient")
            .unwrap()
            .instantiate([noise.position[0] + 280.0, noise.position[1]]);
        for row in &mut gradient.rows {
            if let Some(old) = noise.rows.iter().find(|old| {
                old.key == row.key
                    || matches!(
                        (old.key.as_deref(), row.key.as_deref()),
                        (Some("noise_low"), Some("color_low"))
                            | (Some("noise_high"), Some("color_high"))
                    )
            }) {
                row.value = old.value.clone();
                row.key = old.key.clone();
            }
        }
        noise
            .rows
            .retain(|row| !matches!(row.key.as_deref(), Some("noise_low" | "noise_high")));
        let input = gradient
            .ports
            .iter()
            .find(|port| port.key.as_deref() == Some("value_in"))
            .unwrap()
            .id;
        let output = gradient
            .ports
            .iter()
            .find(|port| port.key.as_deref() == Some("material"))
            .unwrap()
            .id;
        for connection in &mut graph.connections {
            if connection.from == old_output {
                connection.from = output;
            }
        }
        graph.connections.push(GraphConnection {
            id: Uuid::new_v4(),
            from: old_output,
            to: input,
        });
        additions.push(gradient);
    }
    graph.nodes.extend(additions);
    normalize_gradient_colors(graph);
}

pub fn set_root_name(graph: &mut GraphDocument, name: &str) {
    if let Some(root) = graph.nodes.iter_mut().find(|node| {
        matches!(
            node.definition.as_deref(),
            Some("wall" | "surface" | "pattern")
        )
    }) {
        if let Some(row) = root
            .rows
            .iter_mut()
            .find(|row| row.key.as_deref() == Some("name"))
        {
            row.value = GraphControlValue::Text(name.into());
        } else {
            let mut row = GraphRow::new("name", GraphControlValue::Text(name.into()));
            row.key = Some("name".into());
            root.rows.insert(0, row);
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurfacePatternProjection {
    pub elevation: Option<f32>,
    pub thickness: Option<f32>,
    pub clearance: Option<f32>,
    pub texture_scale: Option<f32>,
    pub top_source: Option<PixelSource>,
    pub side_source: Option<PixelSource>,
    pub subdivision: Option<WallSurfaceSubdivision>,
}

#[derive(Default)]
struct GeometryProjection {
    clearance: Option<f32>,
    subdivision: Option<WallSurfaceSubdivision>,
    pattern_id: Option<Uuid>,
}

fn node(
    id: &str,
    title: &str,
    color: [u8; 4],
    rows: Vec<(&str, GraphControlValue)>,
    ports: Vec<(&str, PortDirection, &str)>,
) -> GraphNode {
    let mut node = GraphNode::new(title, [0.0, 0.0], color);
    node.definition = Some(id.into());
    node.width = match id {
        "pattern_ref" | "surface_pattern_ref" => 260.0,
        "color" | "tile" => 155.0,
        _ => 220.0,
    };
    node.rows = rows
        .into_iter()
        .map(|(key, value)| {
            let mut row = GraphRow::new(key, value);
            row.key = Some(key.into());
            row
        })
        .collect();
    node.ports = ports
        .into_iter()
        .enumerate()
        .map(|(index, (key, direction, kind))| {
            let side = if direction == PortDirection::Input {
                PortSide::Left
            } else {
                PortSide::Right
            };
            let mut port = GraphPort::new(key, direction, side, (index as f32 + 1.0) / 4.0);
            port.key = Some(key.into());
            port.kind = kind.into();
            port
        })
        .collect();
    node
}

/// The port kinds are deliberately independent of behavior flow and entity values.
pub fn definitions() -> GraphDefinitions {
    let mut definitions = GraphDefinitions::default();
    let templates = [
        node(
            "surface",
            "Surface",
            [47, 124, 136, 255],
            vec![
                (
                    "elevation",
                    GraphControlValue::Number {
                        value: 0.25,
                        min: -100.0,
                        max: 100.0,
                        step: 0.05,
                    },
                ),
                (
                    "thickness",
                    GraphControlValue::Number {
                        value: 0.08,
                        min: 0.005,
                        max: 10.0,
                        step: 0.01,
                    },
                ),
            ],
            vec![("geometry", PortDirection::Output, "surface.geometry")],
        ),
        node(
            "inset",
            "Inset",
            [52, 112, 158, 255],
            vec![(
                "clearance",
                GraphControlValue::Number {
                    value: 0.015,
                    min: 0.0,
                    max: 2.0,
                    step: 0.005,
                },
            )],
            vec![
                ("geometry_in", PortDirection::Input, "surface.geometry"),
                ("geometry", PortDirection::Output, "surface.geometry"),
            ],
        ),
        node(
            "subdivide",
            "Subdivide",
            [52, 112, 158, 255],
            vec![
                (
                    "cell_size",
                    GraphControlValue::Number {
                        value: 1.0,
                        min: 0.25,
                        max: 8.0,
                        step: 0.25,
                    },
                ),
                (
                    "gap",
                    GraphControlValue::Number {
                        value: 0.02,
                        min: 0.0,
                        max: 0.2,
                        step: 0.005,
                    },
                ),
            ],
            vec![
                ("geometry_in", PortDirection::Input, "surface.geometry"),
                ("alternate", PortDirection::Input, "surface.material"),
                ("grout", PortDirection::Input, "surface.material"),
                ("geometry", PortDirection::Output, "surface.geometry"),
            ],
        ),
        node(
            "surface_pattern_ref",
            "Pattern Reference",
            [52, 112, 158, 255],
            vec![(
                "pattern",
                GraphControlValue::Custom {
                    kind: "pattern".into(),
                    data: String::new().into(),
                },
            )],
            vec![
                ("geometry_in", PortDirection::Input, "surface.geometry"),
                ("geometry", PortDirection::Output, "surface.geometry"),
            ],
        ),
        node(
            "noise",
            "Noise",
            [132, 95, 152, 255],
            vec![
                (
                    "noise_mode",
                    GraphControlValue::Choice {
                        options: vec!["Value".into(), "Voronoi".into()],
                        selected: 0,
                    },
                ),
                (
                    "noise_scale",
                    GraphControlValue::Number {
                        value: 8.0,
                        min: 1.0,
                        max: 32.0,
                        step: 1.0,
                    },
                ),
                (
                    "noise_seed",
                    GraphControlValue::Number {
                        value: 0.0,
                        min: 0.0,
                        max: 65535.0,
                        step: 1.0,
                    },
                ),
            ],
            vec![("value", PortDirection::Output, "surface.value")],
        ),
        node(
            "gradient",
            "Gradient",
            [132, 95, 152, 255],
            vec![
                (
                    "color_low",
                    GraphControlValue::Custom {
                        kind: "color".into(),
                        data: "#414141".into(),
                    },
                ),
                (
                    "color_high",
                    GraphControlValue::Custom {
                        kind: "color".into(),
                        data: "#737373".into(),
                    },
                ),
            ],
            vec![
                ("value_in", PortDirection::Input, "surface.value"),
                ("material", PortDirection::Output, "surface.material"),
            ],
        ),
        node(
            "color",
            "Color",
            [132, 95, 152, 255],
            vec![(
                "color",
                GraphControlValue::Custom {
                    kind: "color".into(),
                    data: "#685e50".into(),
                },
            )],
            vec![("material", PortDirection::Output, "surface.material")],
        ),
        node(
            "tile",
            "Tile",
            [132, 95, 152, 255],
            vec![(
                "tile_id",
                GraphControlValue::Custom {
                    kind: "tile".into(),
                    data: String::new().into(),
                },
            )],
            vec![("material", PortDirection::Output, "surface.material")],
        ),
        node(
            "output",
            "Surface Output",
            [45, 130, 99, 255],
            vec![(
                "texture_scale",
                GraphControlValue::Number {
                    value: 1.0,
                    min: 0.001,
                    max: 64.0,
                    step: 0.1,
                },
            )],
            vec![
                ("geometry", PortDirection::Input, "surface.geometry"),
                ("top", PortDirection::Input, "surface.material"),
                ("sides", PortDirection::Input, "surface.material"),
            ],
        ),
        node(
            "wall",
            "Wall",
            [47, 124, 136, 255],
            wall_base_rows(&WallStyle::default()),
            vec![("build", PortDirection::Output, "wall.build")],
        ),
        node(
            "masonry",
            "Masonry",
            [52, 112, 158, 255],
            wall_masonry_rows(&WallStyle::default()),
            vec![
                ("build_in", PortDirection::Input, "wall.build"),
                ("stone", PortDirection::Input, "surface.material"),
                ("mortar", PortDirection::Input, "surface.material"),
                ("build", PortDirection::Output, "wall.build"),
            ],
        ),
        node(
            "pattern",
            "Pattern",
            [91, 86, 151, 255],
            vec![],
            vec![("build", PortDirection::Output, "wall.build")],
        ),
        node(
            "pattern_output",
            "Pattern Output",
            [45, 130, 99, 255],
            vec![],
            vec![("build_in", PortDirection::Input, "wall.build")],
        ),
        node(
            "pattern_scale",
            "Pattern Scale",
            [52, 112, 158, 255],
            vec![
                (
                    "width",
                    GraphControlValue::Number {
                        value: 1.0,
                        min: 0.1,
                        max: 10.0,
                        step: 0.1,
                    },
                ),
                (
                    "height",
                    GraphControlValue::Number {
                        value: 1.0,
                        min: 0.1,
                        max: 10.0,
                        step: 0.1,
                    },
                ),
            ],
            vec![
                ("build_in", PortDirection::Input, "wall.build"),
                ("build", PortDirection::Output, "wall.build"),
            ],
        ),
        node(
            "stone_variants",
            "Stone Variants",
            [132, 95, 152, 255],
            vec![(
                "amount",
                GraphControlValue::Number {
                    value: 0.5,
                    min: 0.0,
                    max: 1.0,
                    step: 0.05,
                },
            )],
            vec![
                ("base", PortDirection::Input, "surface.material"),
                ("alternate", PortDirection::Input, "surface.material"),
                ("alternate_2", PortDirection::Input, "surface.material"),
                ("alternate_3", PortDirection::Input, "surface.material"),
                ("material", PortDirection::Output, "surface.material"),
            ],
        ),
        node(
            "pattern_ref",
            "Pattern Reference",
            [52, 112, 158, 255],
            vec![(
                "pattern",
                GraphControlValue::Custom {
                    kind: "pattern".into(),
                    data: String::new().into(),
                },
            )],
            vec![
                ("build_in", PortDirection::Input, "wall.build"),
                ("build", PortDirection::Output, "wall.build"),
            ],
        ),
        node(
            "wall_finish",
            "Wall Finish",
            [52, 112, 158, 255],
            wall_finish_rows(&WallStyle::default()),
            vec![
                ("build_in", PortDirection::Input, "wall.build"),
                ("frame", PortDirection::Input, "surface.material"),
                ("build", PortDirection::Output, "wall.build"),
            ],
        ),
        node(
            "wall_output",
            "Wall Output",
            [45, 130, 99, 255],
            vec![],
            vec![("build_in", PortDirection::Input, "wall.build")],
        ),
    ];
    for template in templates {
        definitions
            .register_node(GraphNodeDefinition::from_template(
                template.definition.as_deref().unwrap_or_default(),
                "Construction",
                &template,
            ))
            .expect("valid built-in construction node");
    }
    definitions
}

fn connect(
    graph: &mut GraphDocument,
    from: &GraphNode,
    from_key: &str,
    to: &GraphNode,
    to_key: &str,
) {
    let from = from
        .ports
        .iter()
        .find(|port| port.key.as_deref() == Some(from_key))
        .unwrap()
        .id;
    let to = to
        .ports
        .iter()
        .find(|port| port.key.as_deref() == Some(to_key))
        .unwrap()
        .id;
    graph.connections.push(GraphConnection {
        id: Uuid::new_v4(),
        from,
        to,
    });
}

pub fn default_surface_graph() -> GraphDocument {
    surface_graph(0.25, 0.08)
}

pub fn surface_graph(elevation: f32, thickness: f32) -> GraphDocument {
    let definitions = definitions();
    let mut surface = definitions.node("surface").unwrap().instantiate([0.0, 0.0]);
    for (key, value) in [("elevation", elevation), ("thickness", thickness)] {
        if let Some(row) = surface
            .rows
            .iter_mut()
            .find(|row| row.key.as_deref() == Some(key))
        {
            if let GraphControlValue::Number { value: current, .. } = &mut row.value {
                *current = value;
            }
        }
    }
    let color = definitions.node("color").unwrap().instantiate([0.0, 220.0]);
    let output = definitions
        .node("output")
        .unwrap()
        .instantiate([330.0, 0.0]);
    let mut graph = GraphDocument::default();
    connect(&mut graph, &surface, "geometry", &output, "geometry");
    connect(&mut graph, &color, "material", &output, "top");
    graph.nodes = vec![surface, color, output];
    graph
}

fn wall_base_rows(style: &WallStyle) -> Vec<(&'static str, GraphControlValue)> {
    vec![
        (
            "height",
            GraphControlValue::Number {
                value: style.height,
                min: 0.1,
                max: 100.0,
                step: 0.1,
            },
        ),
        (
            "thickness",
            GraphControlValue::Number {
                value: style.thickness,
                min: 0.01,
                max: 10.0,
                step: 0.05,
            },
        ),
        (
            "texture_scale",
            GraphControlValue::Number {
                value: style.texture_scale,
                min: 0.001,
                max: 64.0,
                step: 0.1,
            },
        ),
    ]
}

fn wall_masonry_rows(style: &WallStyle) -> Vec<(&'static str, GraphControlValue)> {
    let number = |value, min, max, step| GraphControlValue::Number {
        value,
        min,
        max,
        step,
    };
    vec![
        (
            "masonry",
            GraphControlValue::Choice {
                options: vec!["Brick".into(), "Stone Blocks".into(), "Rubble".into()],
                selected: match style.masonry {
                    WallMasonryPattern::Brick => 0,
                    WallMasonryPattern::StoneBlocks => 1,
                    WallMasonryPattern::Rubble => 2,
                },
            },
        ),
        ("brick_width", number(style.brick_width, 0.05, 10.0, 0.05)),
        ("brick_height", number(style.brick_height, 0.05, 10.0, 0.05)),
        ("mortar_gap", number(style.mortar_gap, 0.0, 0.2, 0.005)),
        (
            "course_offset",
            number(style.alternating_course_offset, 0.0, 1.0, 0.05),
        ),
    ]
}

fn wall_finish_rows(style: &WallStyle) -> Vec<(&'static str, GraphControlValue)> {
    let number = |value, min, max, step| GraphControlValue::Number {
        value,
        min,
        max,
        step,
    };
    vec![
        ("bevel", number(style.bevel, 0.0, 1.0, 0.005)),
        ("irregularity", number(style.irregularity, 0.0, 1.0, 0.05)),
        ("damage", number(style.damage, 0.0, 1.0, 0.05)),
        (
            "stone_variation",
            number(style.stone_variation, 0.0, 1.0, 0.05),
        ),
        ("frame_width", number(style.frame_width, 0.0, 10.0, 0.05)),
        ("frame_depth", number(style.frame_depth, 0.0, 10.0, 0.05)),
        (
            "arch_stones",
            number(style.arch_stones as f32, 1.0, 64.0, 1.0),
        ),
    ]
}

fn source_from_token(token: &str) -> Result<Option<PixelSource>, String> {
    if token.is_empty() {
        return Ok(None);
    }
    if let Some(index) = token.strip_prefix("palette:") {
        return index
            .parse::<u16>()
            .map(|index| Some(PixelSource::PaletteIndex(index)))
            .map_err(|_| "Invalid palette color".into());
    }
    if let Some(id) = token.strip_prefix("tile:") {
        return Uuid::parse_str(id)
            .map(|id| Some(PixelSource::TileId(id)))
            .map_err(|_| "Invalid tile".into());
    }
    let hex = token.trim_start_matches('#');
    if hex.len() != 6 {
        return Err("Invalid pattern color".into());
    }
    let values = (0..3)
        .map(|index| u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Invalid pattern color")?;
    Ok(Some(PixelSource::Color(TheColor::new(
        values[0] as f32 / 255.0,
        values[1] as f32 / 255.0,
        values[2] as f32 / 255.0,
        1.0,
    ))))
}

pub fn pattern_graph(style: &WallStyle) -> GraphDocument {
    let defs = definitions();
    let pattern = defs.node("pattern").unwrap().instantiate([0.0, 0.0]);
    let mut masonry = defs.node("masonry").unwrap().instantiate([300.0, 0.0]);
    let mut finish = defs.node("wall_finish").unwrap().instantiate([600.0, 0.0]);
    for (node, rows) in [
        (&mut masonry, wall_masonry_rows(style)),
        (&mut finish, wall_finish_rows(style)),
    ] {
        for (key, value) in rows {
            if let Some(row) = node
                .rows
                .iter_mut()
                .find(|row| row.key.as_deref() == Some(key))
            {
                row.value = value;
            }
        }
    }
    let output = defs
        .node("pattern_output")
        .unwrap()
        .instantiate([900.0, 0.0]);
    let mut graph = GraphDocument::default();
    connect(&mut graph, &pattern, "build", &masonry, "build_in");
    connect(&mut graph, &masonry, "build", &finish, "build_in");
    connect(&mut graph, &finish, "build", &output, "build_in");
    for (key, source, y, target) in
        std::iter::once(("stone", style.stone_source.as_ref(), 420.0, &masonry))
            .chain(
                style
                    .stone_variants
                    .iter()
                    .enumerate()
                    .map(|(index, source)| {
                        (
                            "stone",
                            Some(source),
                            610.0 + index as f32 * 190.0,
                            &masonry,
                        )
                    }),
            )
            .chain([
                ("mortar", style.mortar_source.as_ref(), 1200.0, &masonry),
                ("frame", style.frame_source.as_ref(), 1390.0, &finish),
            ])
    {
        let Some(source) = source else { continue };
        let (definition, kind, value) = match source {
            PixelSource::Color(color) => {
                let rgba = color.to_u8_array();
                (
                    "color",
                    "color",
                    format!("#{:02x}{:02x}{:02x}", rgba[0], rgba[1], rgba[2]),
                )
            }
            PixelSource::PaletteIndex(index) => ("color", "palette", index.to_string()),
            PixelSource::TileId(id) => ("tile", "tile", id.to_string()),
            _ => continue,
        };
        let mut material_node = defs.node(definition).unwrap().instantiate([300.0, y]);
        material_node.rows[0].value = GraphControlValue::Custom {
            kind: kind.into(),
            data: value.into(),
        };
        connect(&mut graph, &material_node, "material", target, key);
        graph.nodes.push(material_node);
    }
    graph.nodes.extend([pattern, masonry, finish, output]);
    graph
}

pub fn wall_reference_graph(style: &WallStyle, pattern_id: Uuid) -> GraphDocument {
    let defs = definitions();
    let mut wall = defs.node("wall").unwrap().instantiate([0.0, 0.0]);
    for (key, value) in wall_base_rows(style) {
        if let Some(row) = wall
            .rows
            .iter_mut()
            .find(|row| row.key.as_deref() == Some(key))
        {
            row.value = value;
        }
    }
    let mut reference = defs.node("pattern_ref").unwrap().instantiate([300.0, 0.0]);
    reference.rows[0].value = GraphControlValue::Custom {
        kind: "pattern".into(),
        data: pattern_id.to_string().into(),
    };
    let output = defs.node("wall_output").unwrap().instantiate([600.0, 0.0]);
    let mut graph = GraphDocument::default();
    connect(&mut graph, &wall, "build", &reference, "build_in");
    connect(&mut graph, &reference, "build", &output, "build_in");
    graph.nodes.extend([wall, reference, output]);
    graph
}

fn same_pattern(a: &WallStyle, b: &WallStyle) -> bool {
    let (mut a, mut b) = (a.clone(), b.clone());
    let default = WallStyle::default();
    for style in [&mut a, &mut b] {
        style.height = default.height;
        style.thickness = default.thickness;
        style.texture_scale = default.texture_scale;
        style.variation_seed = default.variation_seed;
    }
    a == b
}

pub fn default_wall_graph(style: &WallStyle) -> GraphDocument {
    let definitions = definitions();
    let mut wall = definitions.node("wall").unwrap().instantiate([0.0, 0.0]);
    let mut masonry = definitions
        .node("masonry")
        .unwrap()
        .instantiate([320.0, 0.0]);
    let mut finish = definitions
        .node("wall_finish")
        .unwrap()
        .instantiate([640.0, 0.0]);
    for (node, rows) in [
        (&mut wall, wall_base_rows(style)),
        (&mut masonry, wall_masonry_rows(style)),
        (&mut finish, wall_finish_rows(style)),
    ] {
        for (key, value) in rows {
            if let Some(row) = node
                .rows
                .iter_mut()
                .find(|row| row.key.as_deref() == Some(key))
            {
                row.value = value;
            }
        }
    }
    let output = definitions
        .node("wall_output")
        .unwrap()
        .instantiate([960.0, 0.0]);
    let mut graph = GraphDocument::default();
    connect(&mut graph, &wall, "build", &masonry, "build_in");
    connect(&mut graph, &masonry, "build", &finish, "build_in");
    connect(&mut graph, &finish, "build", &output, "build_in");
    for (key, source, y, target) in [
        ("stone", style.stone_source.as_ref(), 400.0, &masonry),
        ("mortar", style.mortar_source.as_ref(), 580.0, &masonry),
        ("frame", style.frame_source.as_ref(), 760.0, &finish),
    ] {
        let Some(source) = source else {
            continue;
        };
        let (definition, kind, value) = match source {
            PixelSource::Color(color) => {
                let rgba = color.to_u8_array();
                (
                    "color",
                    "color",
                    format!("#{:02x}{:02x}{:02x}", rgba[0], rgba[1], rgba[2]),
                )
            }
            PixelSource::PaletteIndex(index) => ("color", "palette", index.to_string()),
            PixelSource::TileId(id) => ("tile", "tile", id.to_string()),
            _ => continue,
        };
        let mut material = definitions
            .node(definition)
            .unwrap()
            .instantiate([320.0, y]);
        material.rows[0].value = GraphControlValue::Custom {
            kind: kind.into(),
            data: value.into(),
        };
        connect(&mut graph, &material, "material", target, key);
        graph.nodes.push(material);
    }
    graph.nodes.extend([wall, masonry, finish, output]);
    graph
}

fn incoming<'a>(
    graph: &'a GraphDocument,
    node: &GraphNode,
    key: &str,
) -> Result<Option<&'a GraphNode>, String> {
    let port = node
        .ports
        .iter()
        .find(|port| port.key.as_deref() == Some(key))
        .ok_or_else(|| format!("{} is missing its {key} input", node.title))?;
    let links: Vec<_> = graph
        .connections
        .iter()
        .filter(|link| link.to == port.id)
        .collect();
    if links.len() > 1 {
        return Err(format!("{} has multiple {key} inputs", node.title));
    }
    let Some(link) = links.first() else {
        return Ok(None);
    };
    let (source, source_port) = graph.port(link.from).ok_or("Missing source port")?;
    if source_port.direction != PortDirection::Output || source_port.kind != port.kind {
        return Err(format!("Incompatible {key} connection"));
    }
    Ok(Some(source))
}

/// A material input may have several sources. Their connection order defines
/// the base material followed by the variants; all other inputs stay singular.
fn incoming_materials<'a>(
    graph: &'a GraphDocument,
    node: &GraphNode,
    key: &str,
) -> Result<Vec<&'a GraphNode>, String> {
    let port = node
        .ports
        .iter()
        .find(|port| port.key.as_deref() == Some(key))
        .ok_or_else(|| format!("{} is missing its {key} input", node.title))?;
    graph
        .connections
        .iter()
        .filter(|link| link.to == port.id)
        .map(|link| {
            let (source, source_port) = graph.port(link.from).ok_or("Missing source port")?;
            if source_port.direction != PortDirection::Output || source_port.kind != port.kind {
                return Err(format!("Incompatible {key} connection"));
            }
            Ok(source)
        })
        .collect()
}

fn number(node: &GraphNode, key: &str) -> Result<f32, String> {
    match node
        .rows
        .iter()
        .find(|row| row.key.as_deref() == Some(key))
        .map(|row| &row.value)
    {
        Some(GraphControlValue::Number { value, .. }) if value.is_finite() => Ok(*value),
        _ => Err(format!("{} needs a finite {key}", node.title)),
    }
}

fn geometry(
    graph: &GraphDocument,
    node: &GraphNode,
    seen: &mut Vec<Uuid>,
) -> Result<GeometryProjection, String> {
    if seen.contains(&node.id) {
        return Err("Construction pattern has a cycle".into());
    }
    seen.push(node.id);
    let result = match node.definition.as_deref() {
        Some("surface") => Ok(GeometryProjection::default()),
        Some("inset") => {
            let input = incoming(graph, node, "geometry_in")?.ok_or("Inset needs geometry")?;
            let mut projected = geometry(graph, input, seen)?;
            projected.clearance = Some(number(node, "clearance")?.max(0.0));
            Ok(projected)
        }
        Some("subdivide") => {
            let input = incoming(graph, node, "geometry_in")?.ok_or("Subdivide needs geometry")?;
            let mut projected = geometry(graph, input, seen)?;
            let cell_size = number(node, "cell_size")?;
            if cell_size <= 0.0 {
                return Err("Subdivision cell size must be positive".into());
            }
            let alternate_source = incoming(graph, node, "alternate")?
                .map(|node| material(graph, node))
                .transpose()?;
            let grout_source = incoming(graph, node, "grout")?
                .map(|node| material(graph, node))
                .transpose()?;
            projected.subdivision = Some(WallSurfaceSubdivision {
                cell_size,
                cell_size_v: None,
                gap: number(node, "gap")?.clamp(0.0, 0.2),
                alternate_source,
                grout_source,
            });
            Ok(projected)
        }
        Some("surface_pattern_ref") => {
            let input = incoming(graph, node, "geometry_in")?
                .ok_or("Pattern Reference needs surface geometry")?;
            let mut projected = geometry(graph, input, seen)?;
            let id = node
                .rows
                .iter()
                .find(|row| row.key.as_deref() == Some("pattern"))
                .and_then(|row| match &row.value {
                    GraphControlValue::Custom { data, .. } => data.as_str(),
                    _ => None,
                })
                .and_then(|id| Uuid::parse_str(id).ok())
                .ok_or("Select a Pattern branch")?;
            projected.pattern_id = Some(id);
            Ok(projected)
        }
        _ => Err(format!("{} is not a geometry node", node.title)),
    };
    seen.pop();
    result
}

fn gradient_color(node: &GraphNode, key: &str) -> Result<[u8; 4], String> {
    let mut color = node.clone();
    color.definition = Some("color".into());
    color.rows = node
        .rows
        .iter()
        .filter(|row| row.key.as_deref() == Some(key))
        .cloned()
        .collect();
    for row in &mut color.rows {
        row.key = Some("color".into());
    }
    match material(&GraphDocument::default(), &color)? {
        PixelSource::Color(color) => Ok(color.to_u8_array()),
        _ => Err("Gradient requires a color selection".into()),
    }
}

/// Replace the initial grayscale limits with editable colors while retaining their appearance.
fn normalize_gradient_colors(graph: &mut GraphDocument) {
    for node in &mut graph.nodes {
        if node.definition.as_deref() != Some("gradient") {
            continue;
        }
        for (old, new) in [("noise_low", "color_low"), ("noise_high", "color_high")] {
            if let Some(row) = node
                .rows
                .iter_mut()
                .find(|row| row.key.as_deref() == Some(old))
            {
                if let GraphControlValue::Number { value, .. } = row.value {
                    let gray = (value.clamp(0., 1.) * 255.).round() as u8;
                    row.key = Some(new.into());
                    row.label = new.into();
                    row.value = GraphControlValue::Custom {
                        kind: "color".into(),
                        data: format!("#{gray:02x}{gray:02x}{gray:02x}").into(),
                    };
                }
            }
        }
    }
}

fn material(graph: &GraphDocument, node: &GraphNode) -> Result<PixelSource, String> {
    if node.definition.as_deref() == Some("gradient") {
        let noise = incoming(graph, node, "value_in")?.ok_or("Gradient needs a Value field")?;
        if noise.definition.as_deref() != Some("noise") {
            return Err("Gradient needs a noise value field".into());
        }
        let voronoi = matches!(
            noise
                .rows
                .iter()
                .find(|row| row.key.as_deref() == Some("noise_mode"))
                .map(|row| &row.value),
            Some(GraphControlValue::Choice { selected: 1, .. })
        );
        return Ok(PixelSource::Noise(
            rusterix::map::pixelsource::NoiseMaterial {
                voronoi,
                scale: number(noise, "noise_scale")?.round().clamp(1.0, 32.0) as u32,
                seed: number(noise, "noise_seed")?.round().clamp(0.0, 65535.0) as u32,
                low: 65,
                high: 115,
                low_color: Some(gradient_color(node, "color_low")?),
                high_color: Some(gradient_color(node, "color_high")?),
            },
        ));
    }

    if node.definition.as_deref() == Some("tile") {
        let Some(value) = node
            .rows
            .iter()
            .find(|row| row.key.as_deref() == Some("tile_id"))
            .map(|row| &row.value)
        else {
            return Err("Tile needs a tile ID".into());
        };
        let id = match value {
            GraphControlValue::Custom { kind, data } if kind == "tile" => {
                data.as_str().unwrap_or_default()
            }
            GraphControlValue::Text(id) => id.as_str(),
            _ => return Err("Tile needs a tile selection".into()),
        };
        return Uuid::parse_str(id.trim())
            .map(PixelSource::TileId)
            .map_err(|_| "Tile needs a valid tile ID".into());
    }
    if node.definition.as_deref() != Some("color") {
        return Err(format!("{} is not a material node", node.title));
    }
    let Some(value) = node
        .rows
        .iter()
        .find(|row| matches!(row.key.as_deref(), Some("color" | "hex")))
        .map(|row| &row.value)
    else {
        return Err("Color needs a color selection".into());
    };
    if let GraphControlValue::Custom { kind, data } = value {
        if kind == "palette" {
            let index = data
                .as_str()
                .unwrap_or_default()
                .parse::<u16>()
                .map_err(|_| "Color needs a valid palette entry")?;
            return Ok(PixelSource::PaletteIndex(index));
        }
    }
    let hex = match value {
        GraphControlValue::Custom { kind, data } if kind == "color" => {
            data.as_str().unwrap_or_default()
        }
        GraphControlValue::Text(hex) => hex.as_str(),
        _ => return Err("Color needs a color selection".into()),
    };
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return Err("Color needs six hex digits".into());
    }
    let components = (0..3)
        .map(|index| {
            u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
                .map_err(|_| "Color needs six hex digits".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PixelSource::Color(TheColor::new(
        components[0] as f32 / 255.0,
        components[1] as f32 / 255.0,
        components[2] as f32 / 255.0,
        1.0,
    )))
}

fn pattern_stone_material(
    graph: &GraphDocument,
    node: &GraphNode,
) -> Result<(PixelSource, Option<(Vec<PixelSource>, f32)>), String> {
    if node.definition.as_deref() != Some("stone_variants") {
        return Ok((material(graph, node)?, None));
    }
    let base = incoming(graph, node, "base")?.ok_or("Stone Variants needs a base material")?;
    let mut alternates = Vec::new();
    for port in ["alternate", "alternate_2", "alternate_3"] {
        if let Some(alternate) = incoming(graph, node, port)? {
            alternates.push(material(graph, alternate)?);
        }
    }
    if alternates.is_empty() {
        return Err("Stone Variants needs an alternate material".into());
    }
    Ok((
        material(graph, base)?,
        Some((alternates, number(node, "amount")?.clamp(0.0, 1.0))),
    ))
}

type PatternOperator = fn(&GraphDocument, &GraphNode, &mut WallStyle) -> Result<(), String>;

// Pattern operations are registered independently of the host. Both wall and surface projections
// consume the resulting style; adding an operator only requires a node definition and handler.
const PATTERN_OPERATORS: &[(&str, PatternOperator)] = &[
    ("masonry", apply_masonry),
    ("wall_finish", apply_wall_finish),
    ("pattern_scale", apply_pattern_scale),
];

fn pattern_operator(id: &str) -> Option<PatternOperator> {
    PATTERN_OPERATORS
        .iter()
        .find(|(name, _)| *name == id)
        .map(|(_, handler)| *handler)
}

fn apply_masonry(
    graph: &GraphDocument,
    node: &GraphNode,
    style: &mut WallStyle,
) -> Result<(), String> {
    style.masonry = match node
        .rows
        .iter()
        .find(|row| row.key.as_deref() == Some("masonry"))
        .map(|row| &row.value)
    {
        Some(GraphControlValue::Choice { selected: 0, .. }) => WallMasonryPattern::Brick,
        Some(GraphControlValue::Choice { selected: 1, .. }) => WallMasonryPattern::StoneBlocks,
        Some(GraphControlValue::Choice { selected: 2, .. }) => WallMasonryPattern::Rubble,
        _ => return Err("Masonry needs a pattern".into()),
    };
    style.brick_width = number(node, "brick_width")?.clamp(0.05, 10.0);
    style.brick_height = number(node, "brick_height")?.clamp(0.05, 10.0);
    style.mortar_gap = number(node, "mortar_gap")?.clamp(0.0, 0.2);
    style.alternating_course_offset = number(node, "course_offset")?.clamp(0.0, 1.0);
    let stones = incoming_materials(graph, node, "stone")?;
    if let Some(source) = stones.first() {
        let (base, variation) = pattern_stone_material(graph, source)?;
        style.stone_source = Some(base);
        if let Some((alternates, amount)) = variation {
            style.stone_variants = alternates;
            style.stone_variation = amount;
        } else {
            style.stone_variants = stones[1..]
                .iter()
                .map(|source| material(graph, source))
                .collect::<Result<Vec<_>, _>>()?;
        }
    }
    if let Some(source) = incoming(graph, node, "mortar")? {
        style.mortar_source = Some(material(graph, source)?);
    }
    Ok(())
}

fn apply_wall_finish(
    graph: &GraphDocument,
    node: &GraphNode,
    style: &mut WallStyle,
) -> Result<(), String> {
    style.bevel = number(node, "bevel")?.clamp(0.0, 1.0);
    style.irregularity = number(node, "irregularity")?.clamp(0.0, 1.0);
    style.damage = number(node, "damage")?.clamp(0.0, 1.0);
    style.stone_variation = number(node, "stone_variation")?.clamp(0.0, 1.0);
    style.frame_width = number(node, "frame_width")?.clamp(0.0, 10.0);
    style.frame_depth = number(node, "frame_depth")?.clamp(0.0, 10.0);
    style.arch_stones = number(node, "arch_stones")?.round().clamp(1.0, 64.0) as u16;
    if let Some(source) = incoming(graph, node, "frame")? {
        style.frame_source = Some(material(graph, source)?);
    }
    Ok(())
}

fn apply_pattern_scale(
    _: &GraphDocument,
    node: &GraphNode,
    style: &mut WallStyle,
) -> Result<(), String> {
    style.brick_width = (style.brick_width * number(node, "width")?).clamp(0.05, 10.0);
    style.brick_height = (style.brick_height * number(node, "height")?).clamp(0.05, 10.0);
    Ok(())
}

pub fn compile_surface(graph: &GraphDocument) -> Result<SurfacePatternProjection, String> {
    compile_surface_inner(graph, None)
}

pub fn compile_surface_with_patterns(
    graph: &GraphDocument,
    patterns: &indexmap::IndexMap<Uuid, ConstructionPatternAsset>,
) -> Result<SurfacePatternProjection, String> {
    compile_surface_inner(graph, Some(patterns))
}

fn compile_surface_inner(
    graph: &GraphDocument,
    patterns: Option<&indexmap::IndexMap<Uuid, ConstructionPatternAsset>>,
) -> Result<SurfacePatternProjection, String> {
    let outputs: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| node.definition.as_deref() == Some("output") && !node.disabled)
        .collect();
    if outputs.len() != 1 {
        return Err("A surface pattern needs one active Surface Output".into());
    }
    let output = outputs[0];
    let geometry_node =
        incoming(graph, output, "geometry")?.ok_or("Surface Output needs geometry")?;
    let geometry = geometry(graph, geometry_node, &mut Vec::new())?;
    let surfaces: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| node.definition.as_deref() == Some("surface") && !node.disabled)
        .collect();
    if surfaces.len() != 1 {
        return Err("A surface pattern needs one active Surface node".into());
    }
    let top_source = incoming(graph, output, "top")?
        .map(|node| material(graph, node))
        .transpose()?;
    let side_source = incoming(graph, output, "sides")?
        .map(|node| material(graph, node))
        .transpose()?;
    let texture_scale = number(output, "texture_scale")?;
    if texture_scale <= 0.0 {
        return Err("Texture scale must be positive".into());
    }
    let mut projection = SurfacePatternProjection {
        elevation: surfaces[0]
            .rows
            .iter()
            .any(|row| row.key.as_deref() == Some("elevation"))
            .then(|| number(surfaces[0], "elevation"))
            .transpose()?,
        thickness: surfaces[0]
            .rows
            .iter()
            .any(|row| row.key.as_deref() == Some("thickness"))
            .then(|| number(surfaces[0], "thickness").map(|value| value.max(0.005)))
            .transpose()?,
        clearance: geometry.clearance,
        texture_scale: Some(texture_scale),
        top_source,
        side_source,
        subdivision: geometry.subdivision,
    };
    if let Some(id) = geometry.pattern_id {
        let asset = patterns
            .and_then(|patterns| patterns.get(&id))
            .ok_or("Referenced surface pattern is missing")?;
        if asset.kind != ConstructionPatternKind::Pattern {
            return Err("Surface reference must point to a Pattern branch".into());
        }
        let style = compile_pattern(&asset.graph, &WallStyle::default())?;
        // The host owns elevation and thickness. Pattern geometry and materials provide defaults;
        // explicit Surface Output materials remain local overrides.
        if projection.subdivision.is_none() {
            projection.subdivision = Some(WallSurfaceSubdivision {
                cell_size: style.brick_width,
                cell_size_v: Some(style.brick_height),
                gap: style.mortar_gap,
                alternate_source: style.stone_variants.first().cloned(),
                grout_source: style.mortar_source.clone(),
            });
        }
        if projection.top_source.is_none() {
            projection.top_source = style.stone_source;
        }
        if projection.side_source.is_none() {
            projection.side_source = style.frame_source;
        }
    }
    Ok(projection)
}

pub fn apply_to_surface(surface: &mut WallAreaSurface, projection: &SurfacePatternProjection) {
    if let Some(elevation) = projection.elevation {
        surface.elevation = elevation;
    }
    if let Some(thickness) = projection.thickness {
        surface.thickness = thickness;
    }
    if let Some(clearance) = projection.clearance {
        surface.clearance = clearance;
    }
    if let Some(texture_scale) = projection.texture_scale {
        surface.texture_scale = Some(texture_scale);
    }
    if let Some(source) = &projection.top_source {
        surface.source = Some(source.clone());
    }
    surface.side_source = projection.side_source.clone();
    surface.subdivision = projection.subdivision.clone();
}

pub fn compile_wall(graph: &GraphDocument, base: &WallStyle) -> Result<WallStyle, String> {
    compile_wall_inner(graph, base, None, &mut Vec::new())
}

pub fn compile_wall_in_project(
    graph: &GraphDocument,
    base: &WallStyle,
    project: &Project,
) -> Result<WallStyle, String> {
    compile_wall_inner(
        graph,
        base,
        Some(&project.construction_patterns),
        &mut Vec::new(),
    )
}

pub fn compile_wall_with_patterns(
    graph: &GraphDocument,
    base: &WallStyle,
    patterns: &indexmap::IndexMap<Uuid, ConstructionPatternAsset>,
) -> Result<WallStyle, String> {
    compile_wall_inner(graph, base, Some(patterns), &mut Vec::new())
}

pub fn compile_pattern(graph: &GraphDocument, base: &WallStyle) -> Result<WallStyle, String> {
    let roots: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| node.definition.as_deref() == Some("pattern") && !node.disabled)
        .collect();
    if roots.len() != 1 {
        return Err("A pattern branch needs one active Pattern node".into());
    }
    let node = roots[0];
    let outputs: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| node.definition.as_deref() == Some("pattern_output") && !node.disabled)
        .collect();
    if outputs.len() == 1 {
        let mut current = incoming(graph, outputs[0], "build_in")?
            .ok_or("Pattern Output needs a geometry chain")?;
        let mut chain = Vec::new();
        loop {
            if current.disabled
                || chain
                    .iter()
                    .any(|previous: &&GraphNode| previous.id == current.id)
            {
                return Err("Pattern branch has a disabled node or cycle".into());
            }
            chain.push(current);
            if current.id == node.id {
                break;
            }
            if current
                .definition
                .as_deref()
                .and_then(pattern_operator)
                .is_none()
            {
                return Err("Pattern geometry chain contains an unsupported node".into());
            }
            current = incoming(graph, current, "build_in")?
                .ok_or("Pattern geometry node needs a Build input")?;
        }
        chain.reverse();
        let mut style = base.clone();
        for current in chain.iter().skip(1) {
            let operator = current
                .definition
                .as_deref()
                .and_then(pattern_operator)
                .ok_or("Pattern geometry chain contains an unsupported node")?;
            operator(graph, current, &mut style)?;
        }
        if let Some(masonry) = chain
            .iter()
            .find(|node| node.definition.as_deref() == Some("masonry"))
        {
            if let Some(stone) = incoming_materials(graph, masonry, "stone")?.first() {
                if let (_, Some((_, amount))) = pattern_stone_material(graph, stone)? {
                    style.stone_variation = amount;
                }
            }
        }
        return Ok(style);
    }
    if !outputs.is_empty()
        || !node
            .rows
            .iter()
            .any(|row| row.key.as_deref() == Some("masonry"))
    {
        return Err("Pattern branch needs one active Pattern Output".into());
    }
    // Read saved single-node patterns until ensure_wall_patterns migrates them.
    let mut style = base.clone();
    style.masonry = match node
        .rows
        .iter()
        .find(|row| row.key.as_deref() == Some("masonry"))
        .map(|row| &row.value)
    {
        Some(GraphControlValue::Choice { selected: 0, .. }) => WallMasonryPattern::Brick,
        Some(GraphControlValue::Choice { selected: 1, .. }) => WallMasonryPattern::StoneBlocks,
        Some(GraphControlValue::Choice { selected: 2, .. }) => WallMasonryPattern::Rubble,
        _ => return Err("Pattern needs a masonry type".into()),
    };
    style.brick_width = number(node, "brick_width")?.clamp(0.05, 10.0);
    style.brick_height = number(node, "brick_height")?.clamp(0.05, 10.0);
    style.mortar_gap = number(node, "mortar_gap")?.clamp(0.0, 0.2);
    style.alternating_course_offset = number(node, "course_offset")?.clamp(0.0, 1.0);
    style.bevel = number(node, "bevel")?.clamp(0.0, 1.0);
    style.irregularity = number(node, "irregularity")?.clamp(0.0, 1.0);
    style.damage = number(node, "damage")?.clamp(0.0, 1.0);
    style.stone_variation = number(node, "stone_variation")?.clamp(0.0, 1.0);
    style.frame_width = number(node, "frame_width")?.clamp(0.0, 10.0);
    style.frame_depth = number(node, "frame_depth")?.clamp(0.0, 10.0);
    style.arch_stones = number(node, "arch_stones")?.round().clamp(1.0, 64.0) as u16;
    for (key, destination) in [
        ("stone", &mut style.stone_source),
        ("mortar", &mut style.mortar_source),
        ("frame", &mut style.frame_source),
    ] {
        let row = node
            .rows
            .iter()
            .find(|row| row.key.as_deref() == Some(key))
            .ok_or("Pattern needs a material field")?;
        let GraphControlValue::Custom { kind, data } = &row.value else {
            return Err("Pattern needs a material selection".into());
        };
        if kind != "material" {
            return Err("Pattern needs a material selection".into());
        }
        *destination = source_from_token(data.as_str().unwrap_or_default())?;
    }
    Ok(style)
}

fn compile_wall_inner(
    graph: &GraphDocument,
    base: &WallStyle,
    patterns: Option<&indexmap::IndexMap<Uuid, ConstructionPatternAsset>>,
    visited: &mut Vec<Uuid>,
) -> Result<WallStyle, String> {
    let outputs: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| node.definition.as_deref() == Some("wall_output") && !node.disabled)
        .collect();
    if outputs.len() != 1 {
        return Err("A wall pattern needs one active Wall Output".into());
    }
    let output = outputs[0];
    let input = if output
        .ports
        .iter()
        .any(|port| port.key.as_deref() == Some("build_in"))
    {
        "build_in"
    } else {
        "style"
    }; // Existing saved wall graphs are migrated on load.
    let mut current = incoming(graph, output, input)?.ok_or("Wall Output needs a Wall branch")?;
    let mut chain = Vec::new();
    loop {
        if current.disabled || chain.iter().any(|node: &&GraphNode| node.id == current.id) {
            return Err("Wall branch has a disabled node or cycle".into());
        }
        chain.push(current);
        match current.definition.as_deref() {
            Some("wall") => break,
            Some("masonry" | "wall_finish" | "pattern_ref") => {
                current = incoming(graph, current, "build_in")?
                    .ok_or("Wall pattern needs a Build input")?;
            }
            _ => return Err("Wall branch contains an unsupported build node".into()),
        }
    }
    chain.reverse();
    let wall = chain[0];
    if chain
        .iter()
        .enumerate()
        .any(|(index, node)| node.definition.as_deref() == Some("pattern_ref") && index != 1)
        || chain
            .iter()
            .filter(|node| node.definition.as_deref() == Some("pattern_ref"))
            .count()
            > 1
    {
        return Err("Place one Pattern Reference directly after Wall".into());
    }
    let masonry = chain
        .iter()
        .find(|node| node.definition.as_deref() == Some("masonry"))
        .copied()
        .or_else(|| {
            wall.rows
                .iter()
                .any(|row| row.key.as_deref() == Some("masonry"))
                .then_some(wall)
        });
    let finish = chain
        .iter()
        .find(|node| node.definition.as_deref() == Some("wall_finish"))
        .copied()
        .or_else(|| {
            wall.rows
                .iter()
                .any(|row| row.key.as_deref() == Some("bevel"))
                .then_some(wall)
        });
    let mut style = base.clone();
    for reference in chain
        .iter()
        .filter(|node| node.definition.as_deref() == Some("pattern_ref"))
    {
        let id = reference
            .rows
            .iter()
            .find(|row| row.key.as_deref() == Some("pattern"))
            .and_then(|row| match &row.value {
                GraphControlValue::Custom { data, .. } => data.as_str(),
                _ => None,
            })
            .and_then(|id| Uuid::parse_str(id).ok())
            .ok_or("Select a wall pattern")?;
        if visited.contains(&id) {
            return Err("Construction pattern reference cycle".into());
        }
        let asset = patterns
            .and_then(|patterns| patterns.get(&id))
            .ok_or("Referenced wall pattern is missing")?;
        if asset.kind == ConstructionPatternKind::Pattern {
            style = compile_pattern(&asset.graph, &style)?;
        } else if asset.kind == ConstructionPatternKind::Wall {
            // Read early construction assets until they have been migrated on load.
            visited.push(id);
            let result = compile_wall_inner(&asset.graph, &style, patterns, visited);
            visited.pop();
            style = result?;
        } else {
            return Err("Reference must point to a Pattern branch".into());
        }
    }
    style.height = number(wall, "height")?.clamp(0.1, 100.0);
    style.thickness = number(wall, "thickness")?.clamp(0.01, 10.0);
    style.texture_scale = number(wall, "texture_scale")?.clamp(0.001, 64.0);
    if let Some(masonry) = masonry {
        style.brick_width = number(masonry, "brick_width")?.clamp(0.05, 10.0);
        style.brick_height = number(masonry, "brick_height")?.clamp(0.05, 10.0);
        style.mortar_gap = number(masonry, "mortar_gap")?.clamp(0.0, 0.2);
        style.alternating_course_offset = number(masonry, "course_offset")?.clamp(0.0, 1.0);
        style.masonry = match masonry
            .rows
            .iter()
            .find(|row| row.key.as_deref() == Some("masonry"))
            .map(|row| &row.value)
        {
            Some(GraphControlValue::Choice { selected: 0, .. }) => WallMasonryPattern::Brick,
            Some(GraphControlValue::Choice { selected: 1, .. }) => WallMasonryPattern::StoneBlocks,
            Some(GraphControlValue::Choice { selected: 2, .. }) => WallMasonryPattern::Rubble,
            _ => return Err("Masonry needs a pattern".into()),
        };
    }
    if let Some(finish) = finish {
        style.bevel = number(finish, "bevel")?.clamp(0.0, 1.0);
        style.irregularity = number(finish, "irregularity")?.clamp(0.0, 1.0);
        style.damage = number(finish, "damage")?.clamp(0.0, 1.0);
        style.stone_variation = number(finish, "stone_variation")?.clamp(0.0, 1.0);
        style.frame_width = number(finish, "frame_width")?.clamp(0.0, 10.0);
        style.frame_depth = number(finish, "frame_depth")?.clamp(0.0, 10.0);
        style.arch_stones = number(finish, "arch_stones")?.round().clamp(1.0, 64.0) as u16;
    }
    if let Some(masonry) = masonry {
        for (key, destination) in [
            ("stone", &mut style.stone_source),
            ("mortar", &mut style.mortar_source),
        ] {
            if let Some(source) = incoming(graph, masonry, key)? {
                *destination = Some(material(graph, source)?);
            }
        }
    }
    if let Some(finish) = finish {
        if let Some(source) = incoming(graph, finish, "frame")? {
            style.frame_source = Some(material(graph, source)?);
        }
    }
    Ok(style)
}

/// Give existing authored walls node-backed styles without changing their rendered values.
/// Equal styles reuse one graph, so large maps do not get one asset per span.
pub fn ensure_wall_patterns(project: &mut Project) {
    let patterns = &mut project.construction_patterns;
    for asset in patterns
        .values_mut()
        .filter(|asset| asset.kind == ConstructionPatternKind::Pattern)
    {
        flatten_stone_variant_nodes(&mut asset.graph);
    }
    // Upgrade the first single-node Pattern assets without changing their projected style.
    for asset in patterns
        .values_mut()
        .filter(|asset| asset.kind == ConstructionPatternKind::Pattern)
    {
        if asset
            .graph
            .nodes
            .iter()
            .any(|node| node.definition.as_deref() == Some("pattern_output"))
        {
            continue;
        }
        if let Ok(style) = compile_pattern(&asset.graph, &WallStyle::default()) {
            asset.graph = pattern_graph(&style);
            set_root_name(&mut asset.graph, &asset.name);
        }
    }
    // Saved wall graphs from the first construction editor become a small Wall branch
    // referring to a separate, reusable Pattern branch.
    let snapshot = patterns.clone();
    let legacy: Vec<_> = snapshot
        .iter()
        .filter_map(|(id, asset)| {
            if asset.kind != ConstructionPatternKind::Wall
                || !asset.graph.nodes.iter().any(|node| {
                    matches!(node.definition.as_deref(), Some("masonry" | "wall_finish"))
                        || node.definition.as_deref() == Some("wall")
                            && node
                                .rows
                                .iter()
                                .any(|row| row.key.as_deref() == Some("masonry"))
                })
            {
                return None;
            }
            compile_wall_with_patterns(&asset.graph, &WallStyle::default(), &snapshot)
                .ok()
                .map(|style| (*id, style))
        })
        .collect();
    let mut reusable_patterns: Vec<(WallStyle, Uuid)> = Vec::new();
    let mut converted = Vec::new();
    for (id, style) in legacy {
        let pattern_id = if let Some((_, id)) = reusable_patterns
            .iter()
            .find(|(existing, _)| same_pattern(existing, &style))
        {
            *id
        } else {
            let pattern = ConstructionPatternAsset::new_pattern(
                format!("Pattern {}", patterns.len() + 1),
                &style,
            );
            let id = pattern.id;
            patterns.insert(id, pattern);
            reusable_patterns.push((style.clone(), id));
            id
        };
        if let Some(asset) = patterns.get_mut(&id) {
            asset.graph = wall_reference_graph(&style, pattern_id);
        }
        converted.push((id, pattern_id));
    }
    for asset in patterns
        .values_mut()
        .filter(|asset| asset.kind == ConstructionPatternKind::Wall)
    {
        for node in &mut asset.graph.nodes {
            if node.definition.as_deref() != Some("pattern_ref") {
                continue;
            }
            for row in &mut node.rows {
                if row.key.as_deref() != Some("pattern") {
                    continue;
                }
                if let GraphControlValue::Custom { data, .. } = &mut row.value {
                    if let Some(old) = data.as_str().and_then(|id| Uuid::parse_str(id).ok()) {
                        if let Some((_, replacement)) = converted.iter().find(|(id, _)| *id == old)
                        {
                            *data = replacement.to_string().into();
                        }
                    }
                }
            }
        }
    }
    let mut reusable: Vec<(WallStyle, Uuid)> = Vec::new();
    let mut pattern_for = |style: &WallStyle| {
        if let Some((_, id)) = reusable.iter().find(|(existing, _)| existing == style) {
            return *id;
        }
        let pattern_id = if let Some((_, id)) = reusable_patterns
            .iter()
            .find(|(existing, _)| same_pattern(existing, style))
        {
            *id
        } else {
            let pattern = ConstructionPatternAsset::new_pattern(
                format!("Pattern {}", patterns.len() + 1),
                style,
            );
            let id = pattern.id;
            patterns.insert(id, pattern);
            reusable_patterns.push((style.clone(), id));
            id
        };
        let mut asset =
            ConstructionPatternAsset::new_wall(format!("Wall {}", patterns.len() + 1), style);
        asset.graph = wall_reference_graph(style, pattern_id);
        let id = asset.id;
        patterns.insert(id, asset);
        reusable.push((style.clone(), id));
        id
    };
    for region in &mut project.regions {
        for assembly in &mut region.map.wall_assemblies {
            if assembly.pattern_id.is_none() {
                assembly.pattern_id = Some(pattern_for(&assembly.style));
            }
            for span in &mut assembly.spans {
                if span.pattern_id.is_some() {
                    continue;
                }
                if span.style_override.as_ref() == Some(&assembly.style) {
                    span.style_override = None;
                } else if let Some(style) = span.style_override.as_ref() {
                    span.pattern_id = Some(pattern_for(style));
                }
            }
        }
    }
    for asset in patterns.values_mut() {
        let name = asset.name.clone();
        let has_name = asset.graph.nodes.iter().any(|node| {
            node.rows
                .iter()
                .any(|row| row.key.as_deref() == Some("name"))
        });
        if !has_name {
            set_root_name(&mut asset.graph, &name);
        }
    }
}

/// Early pattern graphs used an extra mixer for stone colors. Connect its
/// materials straight to Masonry so existing projects use the same authoring UI.
fn flatten_stone_variant_nodes(graph: &mut GraphDocument) {
    let original = graph.clone();
    for variant in original
        .nodes
        .iter()
        .filter(|node| node.definition.as_deref() == Some("stone_variants"))
    {
        let mut sources = Vec::new();
        for key in ["base", "alternate", "alternate_2", "alternate_3"] {
            if let Ok(Some(source)) = incoming(&original, variant, key) {
                if let Some(port) = source.ports.iter().find(|port| {
                    port.direction == PortDirection::Output && port.kind == "surface.material"
                }) {
                    sources.push(port.id);
                }
            }
        }
        if sources.is_empty() {
            continue;
        }
        let Some(output) = variant.ports.iter().find(|port| {
            port.direction == PortDirection::Output && port.kind == "surface.material"
        }) else {
            continue;
        };
        let recipients: Vec<_> = original
            .connections
            .iter()
            .filter(|link| link.from == output.id)
            .map(|link| link.to)
            .collect();
        let ports: HashSet<_> = variant.ports.iter().map(|port| port.id).collect();
        graph
            .connections
            .retain(|link| !ports.contains(&link.from) && !ports.contains(&link.to));
        graph.nodes.retain(|node| node.id != variant.id);
        for to in recipients {
            for from in &sources {
                graph.connections.push(GraphConnection {
                    id: Uuid::new_v4(),
                    from: *from,
                    to,
                });
            }
        }
        if let Ok(amount) = number(variant, "amount") {
            for row in graph
                .nodes
                .iter_mut()
                .filter(|node| node.definition.as_deref() == Some("wall_finish"))
                .flat_map(|node| &mut node.rows)
                .filter(|row| row.key.as_deref() == Some("stone_variation"))
            {
                if let GraphControlValue::Number { value, .. } = &mut row.value {
                    *value = amount;
                }
            }
        }
    }
}

/// Refresh linked surfaces before deriving runtime meshes. The caller rebuilds wall geometry once
/// after all projections. Invalid/missing assets retain the last valid projected values.
pub fn synchronize(project: &mut Project) -> Vec<String> {
    for asset in project.construction_patterns.values_mut() {
        normalize_noise_graph(&mut asset.graph);
    }
    let compiled: std::collections::HashMap<_, _> = project
        .construction_patterns
        .iter()
        .filter(|(_, asset)| asset.kind == ConstructionPatternKind::Surface)
        .map(|(id, asset)| {
            (
                *id,
                compile_surface_with_patterns(&asset.graph, &project.construction_patterns),
            )
        })
        .collect();
    let mut errors = Vec::new();
    for region in &mut project.regions {
        for assembly in &mut region.map.wall_assemblies {
            if let Some(id) = assembly.pattern_id {
                match project.construction_patterns.get(&id) {
                    Some(asset) if asset.kind == ConstructionPatternKind::Wall => {
                        match compile_wall_with_patterns(
                            &asset.graph,
                            &assembly.style,
                            &project.construction_patterns,
                        ) {
                            Ok(style) => assembly.style = style,
                            Err(error) => errors.push(format!("{}: {error}", region.name)),
                        }
                    }
                    _ => errors.push(format!("{}: missing wall pattern {id}", region.name)),
                }
            }
            for span in &mut assembly.spans {
                let Some(id) = span.pattern_id else {
                    continue;
                };
                match project.construction_patterns.get(&id) {
                    Some(asset) if asset.kind == ConstructionPatternKind::Wall => {
                        let base = span.style_override.as_ref().unwrap_or(&assembly.style);
                        match compile_wall_with_patterns(
                            &asset.graph,
                            base,
                            &project.construction_patterns,
                        ) {
                            Ok(style) => span.style_override = Some(style),
                            Err(error) => errors.push(format!("{}: {error}", region.name)),
                        }
                    }
                    _ => errors.push(format!("{}: missing wall pattern {id}", region.name)),
                }
            }
            for surface in &mut assembly.area_surfaces {
                let Some(id) = surface.pattern_id else {
                    continue;
                };
                match compiled.get(&id) {
                    Some(Ok(projection)) => {
                        apply_to_surface(surface, projection);
                    }
                    Some(Err(error)) => errors.push(format!("{}: {error}", region.name)),
                    None => errors.push(format!(
                        "{}: missing construction pattern {id}",
                        region.name
                    )),
                }
            }
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    #[test]
    fn noise_flows_through_gradient_from_left_to_right() {
        let surface = WallAreaSurface::new(Vec::new());
        let asset = ConstructionPatternAsset::from_surface("Floor", &surface);
        for link in &asset.graph.connections {
            asset.graph.validate_connection(link.from, link.to).unwrap();
        }
        let noise = asset
            .graph
            .nodes
            .iter()
            .find(|n| n.definition.as_deref() == Some("noise"))
            .unwrap();
        let gradient = asset
            .graph
            .nodes
            .iter()
            .find(|n| n.definition.as_deref() == Some("gradient"))
            .unwrap();
        let output = asset
            .graph
            .nodes
            .iter()
            .find(|n| n.definition.as_deref() == Some("output"))
            .unwrap();
        assert_eq!(noise.ports[0].kind, "surface.value");
        assert!(noise.position[0] < gradient.position[0]);
        assert!(gradient.position[0] < output.position[0]);
        assert!(
            noise
                .rows
                .iter()
                .all(|row| row.key.as_deref() != Some("noise_low"))
        );
    }

    #[test]
    fn saved_noise_materials_upgrade_without_losing_mapping() {
        let mut graph = default_surface_graph();
        let defs = definitions();
        let color = graph.nodes[1].clone();
        let mut noise = defs.node("noise").unwrap().instantiate(color.position);
        noise.ports[0].id = color.ports[0].id;
        noise.ports[0].key = Some("material".into());
        noise.ports[0].kind = "surface.material".into();
        let mut mapping = defs.node("gradient").unwrap().instantiate([0., 0.]);
        mapping.rows[0].key = Some("noise_low".into());
        mapping.rows[0].value = GraphControlValue::Number {
            value: 0.1,
            min: 0.,
            max: 1.,
            step: 0.01,
        };
        noise.rows.extend(mapping.rows);
        graph.nodes[1] = noise;
        normalize_noise_graph(&mut graph);
        for link in &graph.connections {
            graph.validate_connection(link.from, link.to).unwrap();
        }
        assert!(
            matches!(compile_surface(&graph).unwrap().top_source, Some(PixelSource::Noise(noise)) if noise.colors().0 == [26, 26, 26, 255])
        );
        let upgraded = graph.clone();
        normalize_noise_graph(&mut graph);
        assert_eq!(graph, upgraded);
    }

    #[test]
    fn new_surface_noise_is_compiled_and_modes_are_editable() {
        let surface = WallAreaSurface::new(Vec::new());
        let mut asset = ConstructionPatternAsset::from_surface("Floor", &surface);
        let projection = compile_surface(&asset.graph).unwrap();
        assert!(
            matches!(projection.top_source, Some(PixelSource::Noise(ref noise)) if !noise.voronoi)
        );
        assert_eq!(projection.top_source, projection.side_source);
        let node = asset
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.definition.as_deref() == Some("noise"))
            .unwrap();
        if let GraphControlValue::Choice { selected, .. } = &mut node.rows[0].value {
            *selected = 1;
        }
        assert!(
            matches!(compile_surface(&asset.graph).unwrap().top_source, Some(PixelSource::Noise(noise)) if noise.voronoi)
        );
    }

    #[test]
    fn starter_surface_branch_preserves_fit_and_has_editable_tiles() {
        let mut surface = WallAreaSurface::new(Vec::new());
        surface.elevation = 4.33;
        surface.thickness = 0.08;
        surface.texture_scale = Some(0.75);
        let tile = Uuid::new_v4();
        surface.source = Some(PixelSource::TileId(tile));
        let asset = ConstructionPatternAsset::from_surface("Ceiling", &surface);
        for connection in &asset.graph.connections {
            asset
                .graph
                .validate_connection(connection.from, connection.to)
                .unwrap();
        }
        let projection = compile_surface(&asset.graph).unwrap();
        assert_eq!(projection.elevation, Some(surface.elevation));
        assert_eq!(projection.thickness, Some(surface.thickness));
        assert_eq!(projection.texture_scale, surface.texture_scale);
        assert_eq!(projection.top_source, Some(PixelSource::TileId(tile)));
        assert_eq!(projection.subdivision.unwrap().cell_size, 0.5);
        assert!(
            asset
                .graph
                .nodes
                .iter()
                .any(|node| node.definition.as_deref() == Some("tile"))
        );
    }

    use super::*;
    use rusterix::map::wall::WallAssembly;
    #[test]
    fn default_pattern_projects_color_and_texture_scale() {
        let graph = default_surface_graph();
        let projection = compile_surface(&graph).unwrap();
        assert!(projection.top_source.is_some());
        assert!(projection.side_source.is_none());
        assert_eq!(projection.texture_scale, Some(1.0));
    }

    #[test]
    fn invalid_color_does_not_change_surface() {
        let mut graph = default_surface_graph();
        let color = graph
            .nodes
            .iter_mut()
            .find(|node| node.definition.as_deref() == Some("color"))
            .unwrap();
        color.rows[0].value = GraphControlValue::Text("oops".into());
        assert!(compile_surface(&graph).is_err());
    }

    #[test]
    fn one_pattern_projects_to_wall_and_surface() {
        let style = WallStyle::default();
        let pattern = ConstructionPatternAsset::new_pattern("Shared stone", &style);
        let id = pattern.id;
        let mut patterns = indexmap::IndexMap::new();
        patterns.insert(id, pattern);

        let wall = wall_reference_graph(&style, id);
        let wall_style = compile_wall_with_patterns(&wall, &style, &patterns).unwrap();

        let mut surface = default_surface_graph();
        let defs = definitions();
        let mut reference = defs
            .node("surface_pattern_ref")
            .unwrap()
            .instantiate([160.0, 0.0]);
        reference.rows[0].value = GraphControlValue::Custom {
            kind: "pattern".into(),
            data: id.to_string().into(),
        };
        let root = surface
            .nodes
            .iter()
            .find(|node| node.definition.as_deref() == Some("surface"))
            .unwrap()
            .clone();
        let output = surface
            .nodes
            .iter()
            .find(|node| node.definition.as_deref() == Some("output"))
            .unwrap()
            .clone();
        let geometry_port = output
            .ports
            .iter()
            .find(|port| port.key.as_deref() == Some("geometry"))
            .unwrap()
            .id;
        let top_port = output
            .ports
            .iter()
            .find(|port| port.key.as_deref() == Some("top"))
            .unwrap()
            .id;
        surface
            .connections
            .retain(|link| link.to != geometry_port && link.to != top_port);
        connect(&mut surface, &root, "geometry", &reference, "geometry_in");
        connect(&mut surface, &reference, "geometry", &output, "geometry");
        surface.nodes.push(reference);

        let projection = compile_surface_with_patterns(&surface, &patterns).unwrap();
        assert_eq!(projection.top_source, wall_style.stone_source);
        assert_eq!(
            projection.subdivision.as_ref().unwrap().cell_size,
            wall_style.brick_width
        );
        assert_eq!(
            projection.subdivision.as_ref().unwrap().cell_size_v,
            Some(wall_style.brick_height)
        );
        assert_eq!(
            projection.subdivision.as_ref().unwrap().gap,
            wall_style.mortar_gap
        );
        assert!(compile_surface_with_patterns(&surface, &indexmap::IndexMap::new()).is_err());
    }

    #[test]
    fn inset_and_side_material_follow_typed_connections() {
        let mut graph = default_surface_graph();
        let defs = definitions();
        let mut inset = defs.node("inset").unwrap().instantiate([160.0, 0.0]);
        inset.rows[0].value = GraphControlValue::Number {
            value: 0.2,
            min: 0.0,
            max: 2.0,
            step: 0.005,
        };
        let tile_id = Uuid::new_v4();
        let mut tile = defs.node("tile").unwrap().instantiate([160.0, 250.0]);
        tile.rows[0].value = GraphControlValue::Text(tile_id.to_string());
        let surface = graph
            .nodes
            .iter()
            .find(|node| node.definition.as_deref() == Some("surface"))
            .unwrap()
            .clone();
        let output = graph
            .nodes
            .iter()
            .find(|node| node.definition.as_deref() == Some("output"))
            .unwrap()
            .clone();
        let geometry_input = output
            .ports
            .iter()
            .find(|port| port.key.as_deref() == Some("geometry"))
            .unwrap()
            .id;
        graph.connections.retain(|link| link.to != geometry_input);
        connect(&mut graph, &surface, "geometry", &inset, "geometry_in");
        connect(&mut graph, &inset, "geometry", &output, "geometry");
        connect(&mut graph, &tile, "material", &output, "sides");
        graph.nodes.extend([inset, tile]);
        let projection = compile_surface(&graph).unwrap();
        assert_eq!(projection.clearance, Some(0.2));
        assert_eq!(projection.side_source, Some(PixelSource::TileId(tile_id)));
    }

    #[test]
    fn linked_surfaces_refresh_and_keep_last_valid_projection() {
        let mut project = Project::default();
        project.regions.clear();
        let pattern = ConstructionPatternAsset::new("Stone");
        let pattern_id = pattern.id;
        project.construction_patterns.insert(pattern_id, pattern);
        let mut assembly = WallAssembly::new("Room");
        let mut surface = WallAreaSurface::new(Vec::new());
        surface.pattern_id = Some(pattern_id);
        assembly.area_surfaces.push(surface);
        let mut region = crate::region::Region::new();
        region.map.wall_assemblies.clear();
        region.map.wall_assemblies.push(assembly);
        project.regions.push(region);

        assert!(synchronize(&mut project).is_empty());
        let original = project.regions[0].map.wall_assemblies[0].area_surfaces[0]
            .source
            .clone();
        assert!(original.is_some());

        let graph = &mut project
            .construction_patterns
            .get_mut(&pattern_id)
            .unwrap()
            .graph;
        graph
            .nodes
            .iter_mut()
            .find(|node| node.definition.as_deref() == Some("color"))
            .unwrap()
            .rows[0]
            .value = GraphControlValue::Text("broken".into());
        assert_eq!(synchronize(&mut project).len(), 1);
        assert_eq!(
            project.regions[0].map.wall_assemblies[0].area_surfaces[0].source,
            original
        );
    }

    #[test]
    fn one_wall_pattern_updates_multiple_assemblies_and_spans() {
        let mut project = Project::default();
        project.regions.clear();
        let mut pattern = ConstructionPatternAsset::new_wall("Shared stone", &WallStyle::default());
        let id = pattern.id;
        let wall = pattern
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.definition.as_deref() == Some("wall"))
            .unwrap();
        wall.rows
            .iter_mut()
            .find(|row| row.key.as_deref() == Some("height"))
            .unwrap()
            .value = GraphControlValue::Number {
            value: 4.0,
            min: 0.1,
            max: 100.0,
            step: 0.1,
        };
        project.construction_patterns.insert(id, pattern);
        let mut first = WallAssembly::new("First");
        first.pattern_id = Some(id);
        let mut second = WallAssembly::new("Second");
        let a = second.add_node(vek::Vec3::new(0.0, 0.0, 0.0));
        let b = second.add_node(vek::Vec3::new(1.0, 0.0, 0.0));
        let span_id = second.add_span(a, b).unwrap();
        second.span_mut(span_id).unwrap().pattern_id = Some(id);
        let mut region = crate::region::Region::new();
        region.map.wall_assemblies = vec![first, second];
        project.regions.push(region);
        assert!(synchronize(&mut project).is_empty());
        let walls = &project.regions[0].map.wall_assemblies;
        assert_eq!(walls[0].style.height, 4.0);
        assert_eq!(
            walls[1]
                .span(span_id)
                .unwrap()
                .style_override
                .as_ref()
                .unwrap()
                .height,
            4.0
        );
    }

    #[test]
    fn existing_equal_wall_styles_share_one_migrated_graph() {
        let mut project = Project::default();
        project.regions.clear();
        let mut region = crate::region::Region::new();
        region.map.wall_assemblies = vec![WallAssembly::new("A"), WallAssembly::new("B")];
        project.regions.push(region);
        ensure_wall_patterns(&mut project);
        let walls = &project.regions[0].map.wall_assemblies;
        assert_eq!(walls[0].pattern_id, walls[1].pattern_id);
        assert_eq!(project.construction_patterns.len(), 2);
        assert_eq!(
            project
                .construction_patterns
                .values()
                .filter(|asset| asset.kind == ConstructionPatternKind::Pattern)
                .count(),
            1
        );
        ensure_wall_patterns(&mut project);
        assert_eq!(project.construction_patterns.len(), 2);
    }

    #[test]
    fn shared_modular_pattern_updates_every_linked_wall() {
        let mut project = Project::default();
        project.regions.clear();
        let style = WallStyle::default();
        let mut pattern = ConstructionPatternAsset::new_pattern("Stone", &style);
        let pattern_id = pattern.id;
        assert!(pattern.graph.nodes.len() > 1);
        let row = pattern
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.definition.as_deref() == Some("masonry"))
            .unwrap()
            .rows
            .iter_mut()
            .find(|row| row.key.as_deref() == Some("brick_width"))
            .unwrap();
        row.value = GraphControlValue::Number {
            value: 0.35,
            min: 0.05,
            max: 10.0,
            step: 0.05,
        };
        project.construction_patterns.insert(pattern_id, pattern);
        let mut wall = ConstructionPatternAsset::new_wall("Shared wall", &style);
        wall.graph = wall_reference_graph(&style, pattern_id);
        let wall_id = wall.id;
        project.construction_patterns.insert(wall_id, wall);
        let mut region = crate::region::Region::new();
        region.map.wall_assemblies = vec![WallAssembly::new("A"), WallAssembly::new("B")];
        for assembly in &mut region.map.wall_assemblies {
            assembly.pattern_id = Some(wall_id);
        }
        project.regions.push(region);
        assert!(synchronize(&mut project).is_empty());
        assert!(
            project.regions[0]
                .map
                .wall_assemblies
                .iter()
                .all(|assembly| assembly.style.brick_width == 0.35)
        );
    }

    #[test]
    fn pattern_material_input_changes_the_projected_stone() {
        let mut base_style = WallStyle::default();
        base_style.stone_variants.clear();
        let mut graph = pattern_graph(&base_style);
        let masonry = graph
            .nodes
            .iter()
            .find(|node| node.definition.as_deref() == Some("masonry"))
            .unwrap();
        let stone = incoming(&graph, masonry, "stone").unwrap().unwrap().id;
        let source = graph
            .nodes
            .iter_mut()
            .find(|node| node.id == stone)
            .unwrap();
        source.rows[0].value = GraphControlValue::Custom {
            kind: "palette".into(),
            data: "7".into(),
        };

        let style = compile_pattern(&graph, &WallStyle::default()).unwrap();
        assert_eq!(style.stone_source, Some(PixelSource::PaletteIndex(7)));
    }

    #[test]
    fn pattern_scale_composes_with_masonry() {
        let mut graph = pattern_graph(&WallStyle::default());
        let masonry = graph
            .nodes
            .iter()
            .find(|node| node.definition.as_deref() == Some("masonry"))
            .unwrap()
            .clone();
        let finish = graph
            .nodes
            .iter()
            .find(|node| node.definition.as_deref() == Some("wall_finish"))
            .unwrap()
            .clone();
        let from = masonry
            .ports
            .iter()
            .find(|port| port.key.as_deref() == Some("build"))
            .unwrap()
            .id;
        let to = finish
            .ports
            .iter()
            .find(|port| port.key.as_deref() == Some("build_in"))
            .unwrap()
            .id;
        graph
            .connections
            .retain(|connection| connection.from != from || connection.to != to);
        let mut scale = definitions()
            .node("pattern_scale")
            .unwrap()
            .instantiate([450.0, 0.0]);
        if let GraphControlValue::Number { value, .. } = &mut scale.rows[0].value {
            *value = 2.0;
        }
        connect(&mut graph, &masonry, "build", &scale, "build_in");
        connect(&mut graph, &scale, "build", &finish, "build_in");
        graph.nodes.push(scale);

        let style = compile_pattern(&graph, &WallStyle::default()).unwrap();
        assert_eq!(style.brick_width, WallStyle::default().brick_width * 2.0);
    }

    #[test]
    fn stone_variants_node_projects_material_and_amount() {
        let mut base_style = WallStyle::default();
        base_style.stone_variants.clear();
        let mut graph = pattern_graph(&base_style);
        let masonry = graph
            .nodes
            .iter()
            .find(|node| node.definition.as_deref() == Some("masonry"))
            .unwrap()
            .clone();
        let base = incoming(&graph, &masonry, "stone")
            .unwrap()
            .unwrap()
            .clone();
        let stone_port = masonry
            .ports
            .iter()
            .find(|port| port.key.as_deref() == Some("stone"))
            .unwrap()
            .id;
        graph
            .connections
            .retain(|connection| connection.to != stone_port);
        let defs = definitions();
        let mut alternate = defs.node("color").unwrap().instantiate([100.0, 700.0]);
        alternate.rows[0].value = GraphControlValue::Custom {
            kind: "palette".into(),
            data: "9".into(),
        };
        let mut variants = defs
            .node("stone_variants")
            .unwrap()
            .instantiate([320.0, 700.0]);
        if let GraphControlValue::Number { value, .. } = &mut variants.rows[0].value {
            *value = 0.7;
        }
        connect(&mut graph, &base, "material", &variants, "base");
        connect(&mut graph, &alternate, "material", &variants, "alternate");
        connect(&mut graph, &variants, "material", &masonry, "stone");
        graph.nodes.extend([alternate, variants]);

        let style = compile_pattern(&graph, &WallStyle::default()).unwrap();
        assert_eq!(style.stone_variants, vec![PixelSource::PaletteIndex(9)]);
        assert_eq!(style.stone_variation, 0.7);
        flatten_stone_variant_nodes(&mut graph);
        assert!(
            !graph
                .nodes
                .iter()
                .any(|node| node.definition.as_deref() == Some("stone_variants"))
        );
        assert_eq!(
            compile_pattern(&graph, &WallStyle::default()).unwrap(),
            style
        );
    }

    #[test]
    fn saved_pattern_variants_connect_directly_to_masonry() {
        let mut style = WallStyle::default();
        style.stone_source = Some(PixelSource::PaletteIndex(1));
        style.stone_variants = (2..=4).map(PixelSource::PaletteIndex).collect();
        style.stone_variation = 0.65;
        let graph = pattern_graph(&style);
        let masonry = graph
            .nodes
            .iter()
            .find(|node| node.definition.as_deref() == Some("masonry"))
            .unwrap();
        assert_eq!(
            incoming_materials(&graph, masonry, "stone").unwrap().len(),
            4
        );
        assert!(
            !graph
                .nodes
                .iter()
                .any(|node| node.definition.as_deref() == Some("stone_variants"))
        );
        let compiled = compile_pattern(&graph, &style).unwrap();
        assert_eq!(compiled.stone_variants, style.stone_variants);
        assert_eq!(compiled.stone_variation, 0.65);
    }

    #[test]
    fn wall_dimensions_do_not_duplicate_shared_pattern() {
        let mut project = Project::default();
        project.regions.clear();
        let mut region = crate::region::Region::new();
        let first = WallAssembly::new("Low");
        let mut second = WallAssembly::new("High");
        second.style.height = first.style.height + 1.0;
        region.map.wall_assemblies = vec![first, second];
        project.regions.push(region);

        ensure_wall_patterns(&mut project);

        assert_eq!(
            project
                .construction_patterns
                .values()
                .filter(|asset| asset.kind == ConstructionPatternKind::Pattern)
                .count(),
            1
        );
        assert_eq!(
            project
                .construction_patterns
                .values()
                .filter(|asset| asset.kind == ConstructionPatternKind::Wall)
                .count(),
            2
        );
        assert!(synchronize(&mut project).is_empty());
        assert_ne!(
            project.regions[0].map.wall_assemblies[0].style.height,
            project.regions[0].map.wall_assemblies[1].style.height
        );
    }
}
