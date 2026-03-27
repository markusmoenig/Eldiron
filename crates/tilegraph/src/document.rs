use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use theframework::prelude::TheColor;

use crate::{TileNodeGraphExchange, TileNodeGraphState, TileNodeKind, TileNodeState};

#[derive(Debug)]
pub enum TileGraphError {
    TomlDeserialize(toml::de::Error),
    TomlSerialize(toml::ser::Error),
    InvalidGraph(String),
}

impl fmt::Display for TileGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TomlDeserialize(err) => write!(f, "failed to parse tile graph TOML: {err}"),
            Self::TomlSerialize(err) => write!(f, "failed to serialize tile graph TOML: {err}"),
            Self::InvalidGraph(err) => write!(f, "invalid tile graph: {err}"),
        }
    }
}

impl std::error::Error for TileGraphError {}

impl From<toml::de::Error> for TileGraphError {
    fn from(value: toml::de::Error) -> Self {
        Self::TomlDeserialize(value)
    }
}

impl From<toml::ser::Error> for TileGraphError {
    fn from(value: toml::ser::Error) -> Self {
        Self::TomlSerialize(value)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PaletteDocument {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    pub colors: Vec<String>,
}

impl PaletteDocument {
    pub fn parsed_colors(&self) -> Vec<TheColor> {
        self.colors.iter().map(|c| TheColor::from_hex(c)).collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileGraphRef<'a> {
    pub node_path: &'a str,
    pub port: &'a str,
}

impl<'a> TileGraphRef<'a> {
    pub fn parse(value: &'a str) -> Option<Self> {
        let (node_path, port) = value.split_once(':')?;
        if node_path.is_empty() || port.is_empty() {
            return None;
        }
        Some(Self { node_path, port })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeEndpoint {
    pub node_kind: String,
    pub node_name: String,
    pub port: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TileGraphDocument {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub grid: String,
    #[serde(default)]
    pub tile_size: String,
    #[serde(default)]
    pub palette: Option<PaletteDocument>,
    #[serde(default)]
    pub node: BTreeMap<String, BTreeMap<String, toml::Table>>,
}

fn default_version() -> u32 {
    1
}

fn parse_dim_string(value: &str, fallback: u16) -> (u16, u16) {
    let Some((a, b)) = value.split_once('x') else {
        return (fallback, fallback);
    };
    let x = a
        .trim()
        .parse::<u16>()
        .ok()
        .filter(|v| *v > 0)
        .unwrap_or(fallback);
    let y = b
        .trim()
        .parse::<u16>()
        .ok()
        .filter(|v| *v > 0)
        .unwrap_or(fallback);
    (x, y)
}

fn table_f32(table: &toml::Table, key: &str, default: f32) -> f32 {
    match table.get(key) {
        Some(value) => value
            .as_float()
            .map(|v| v as f32)
            .or_else(|| value.as_integer().map(|v| v as f32))
            .unwrap_or(default),
        None => default,
    }
}

fn table_u32(table: &toml::Table, key: &str, default: u32) -> u32 {
    match table.get(key).and_then(|value| value.as_integer()) {
        Some(value) => u32::try_from(value).ok().unwrap_or(default),
        None => default,
    }
}

fn table_bool(table: &toml::Table, key: &str, default: bool) -> bool {
    table
        .get(key)
        .and_then(|value| value.as_bool())
        .unwrap_or(default)
}

fn node_kind_from_doc(
    kind: &str,
    table: &toml::Table,
) -> Result<Option<TileNodeKind>, TileGraphError> {
    let kind = kind.to_ascii_lowercase();
    let node = match kind.as_str() {
        "output" => return Ok(None),
        "voronoi" => TileNodeKind::Voronoi {
            scale: table_f32(table, "scale", 0.2),
            seed: table_u32(table, "seed", 1),
            jitter: table_f32(table, "jitter", 1.0),
        },
        "blur" => TileNodeKind::Blur {
            radius: table_f32(table, "radius", 0.012),
        },
        "slope_blur" => TileNodeKind::SlopeBlur {
            radius: table_f32(table, "radius", 0.016),
            amount: table_f32(table, "amount", 0.55),
        },
        "levels" => TileNodeKind::Levels {
            level: table_f32(table, "level", 0.5),
            width: table_f32(table, "width", 0.5),
        },
        "id_random" => TileNodeKind::IdRandom,
        "noise" => TileNodeKind::Noise {
            scale: table_f32(table, "scale", 0.25),
            seed: table_u32(table, "seed", 1),
            wrap: table_bool(table, "wrap", false),
        },
        "multiply" => TileNodeKind::Multiply,
        "subtract" => TileNodeKind::Subtract,
        "add" => TileNodeKind::Add,
        "min" => TileNodeKind::Min,
        "max" => TileNodeKind::Max,
        "material" => TileNodeKind::Material {
            roughness: table_f32(table, "roughness", 0.5),
            metallic: table_f32(table, "metallic", 0.0),
            opacity: table_f32(table, "opacity", 1.0),
            emissive: table_f32(table, "emissive", 0.0),
        },
        "scalar" => TileNodeKind::Scalar {
            value: table_f32(table, "value", 0.5),
        },
        other => {
            return Err(TileGraphError::InvalidGraph(format!(
                "unsupported tilegraph node kind: {other}"
            )));
        }
    };
    Ok(Some(node))
}

fn input_index(kind: &str, input: &str) -> Option<u8> {
    match kind {
        "output" => match input {
            "color" => Some(0),
            "material" => Some(1),
            _ => None,
        },
        "blur" | "slope_blur" | "levels" | "threshold" | "curve" => match input {
            "in" => Some(0),
            _ => None,
        },
        "id_random" => match input {
            "id" => Some(0),
            _ => None,
        },
        "multiply" | "subtract" | "add" | "min" | "max" => match input {
            "a" => Some(0),
            "b" => Some(1),
            _ => None,
        },
        _ => None,
    }
}

fn output_index(kind: &str, output: &str) -> Option<u8> {
    match kind {
        "voronoi" => match output {
            "center" => Some(0),
            "height" => Some(1),
            "cell_id" | "id" => Some(2),
            _ => None,
        },
        "material" => match output {
            "material" => Some(0),
            _ => None,
        },
        "output" => None,
        _ => match output {
            "field" | "mask" | "color" => Some(0),
            _ => None,
        },
    }
}

impl TileGraphDocument {
    pub fn from_toml_str(input: &str) -> Result<Self, TileGraphError> {
        Ok(toml::from_str(input)?)
    }

    pub fn to_toml_pretty(&self) -> Result<String, TileGraphError> {
        Ok(toml::to_string_pretty(self)?)
    }

    pub fn to_exchange(&self) -> Result<TileNodeGraphExchange, TileGraphError> {
        let (grid_w, grid_h) = parse_dim_string(&self.grid, 1);
        let (tile_w, tile_h) = parse_dim_string(&self.tile_size, 32);

        let mut nodes = vec![TileNodeState {
            kind: TileNodeKind::OutputRoot,
            position: (420, 40),
            bypass: false,
            mute: false,
            solo: false,
        }];
        let mut path_to_index: BTreeMap<String, u16> = BTreeMap::new();
        let mut output_refs: Vec<(String, String)> = Vec::new();

        for graph_node in self.iter_nodes() {
            let path = graph_node.path();
            if graph_node.kind == "output" {
                for key in ["color", "material"] {
                    if let Some(value) = graph_node.table.get(key).and_then(|v| v.as_str()) {
                        output_refs.push((key.to_string(), value.to_string()));
                    }
                }
                continue;
            }
            if let Some(kind) = node_kind_from_doc(&graph_node.kind, graph_node.table)? {
                let pos = graph_node.position().unwrap_or((0, 0));
                let index = nodes.len() as u16;
                nodes.push(TileNodeState {
                    kind,
                    position: pos,
                    bypass: false,
                    mute: false,
                    solo: false,
                });
                path_to_index.insert(path, index);
            }
        }

        let mut connections: Vec<(u16, u8, u16, u8)> = Vec::new();
        for graph_node in self.iter_nodes() {
            let dest_path = graph_node.path();
            let dest_kind = graph_node.kind.as_str();

            for (key, value) in graph_node.table {
                let Some(reference) = value.as_str() else {
                    continue;
                };
                let Some(dest_input) = input_index(dest_kind, key) else {
                    continue;
                };
                let parsed = TileGraphRef::parse(reference).ok_or_else(|| {
                    TileGraphError::InvalidGraph(format!(
                        "invalid connection reference: {reference}"
                    ))
                })?;
                let src_index = *path_to_index.get(parsed.node_path).ok_or_else(|| {
                    TileGraphError::InvalidGraph(format!(
                        "unknown source node path: {}",
                        parsed.node_path
                    ))
                })?;
                let (src_kind, _) = parsed.node_path.split_once('.').ok_or_else(|| {
                    TileGraphError::InvalidGraph(format!(
                        "invalid source node path: {}",
                        parsed.node_path
                    ))
                })?;
                let src_output = output_index(src_kind, parsed.port).ok_or_else(|| {
                    TileGraphError::InvalidGraph(format!(
                        "unknown output port '{}' for node kind '{}'",
                        parsed.port, src_kind
                    ))
                })?;
                let dest_index = if dest_kind == "output" {
                    0
                } else {
                    *path_to_index.get(&dest_path).ok_or_else(|| {
                        TileGraphError::InvalidGraph(format!(
                            "unknown destination node path: {dest_path}"
                        ))
                    })?
                };
                connections.push((src_index, src_output, dest_index, dest_input));
            }
        }

        for (input_name, reference) in output_refs {
            let dest_input = input_index("output", &input_name).ok_or_else(|| {
                TileGraphError::InvalidGraph(format!("unknown output input '{input_name}'"))
            })?;
            let parsed = TileGraphRef::parse(&reference).ok_or_else(|| {
                TileGraphError::InvalidGraph(format!("invalid connection reference: {reference}"))
            })?;
            let src_index = *path_to_index.get(parsed.node_path).ok_or_else(|| {
                TileGraphError::InvalidGraph(format!(
                    "unknown source node path: {}",
                    parsed.node_path
                ))
            })?;
            let (src_kind, _) = parsed.node_path.split_once('.').ok_or_else(|| {
                TileGraphError::InvalidGraph(format!(
                    "invalid source node path: {}",
                    parsed.node_path
                ))
            })?;
            let src_output = output_index(src_kind, parsed.port).ok_or_else(|| {
                TileGraphError::InvalidGraph(format!(
                    "unknown output port '{}' for node kind '{}'",
                    parsed.port, src_kind
                ))
            })?;
            connections.push((src_index, src_output, 0, dest_input));
        }

        Ok(TileNodeGraphExchange {
            version: self.version,
            graph_name: self.name.clone(),
            palette_colors: self
                .palette
                .as_ref()
                .map(|p| p.parsed_colors())
                .unwrap_or_default(),
            output_grid_width: grid_w,
            output_grid_height: grid_h,
            tile_pixel_width: tile_w,
            tile_pixel_height: tile_h,
            graph_state: TileNodeGraphState {
                nodes,
                connections,
                offset: (0, 0),
                selected_node: Some(0),
                preview_mode: 0,
            },
        })
    }

    pub fn from_exchange(exchange: &TileNodeGraphExchange) -> Result<Self, TileGraphError> {
        let state = &exchange.graph_state;
        let mut doc = Self {
            version: exchange.version.max(1),
            name: exchange.graph_name.clone(),
            grid: format!(
                "{}x{}",
                exchange.output_grid_width.max(1),
                exchange.output_grid_height.max(1)
            ),
            tile_size: format!(
                "{}x{}",
                exchange.tile_pixel_width.max(1),
                exchange.tile_pixel_height.max(1)
            ),
            palette: (!exchange.palette_colors.is_empty()).then(|| PaletteDocument {
                name: None,
                link: None,
                colors: exchange
                    .palette_colors
                    .iter()
                    .map(|c| {
                        let rgba = c.to_u8_array();
                        format!("#{:02x}{:02x}{:02x}", rgba[0], rgba[1], rgba[2])
                    })
                    .collect(),
            }),
            node: BTreeMap::new(),
        };

        let mut per_kind_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut paths: Vec<Option<String>> = vec![None; state.nodes.len()];
        paths[0] = Some("output.main".to_string());

        for (index, node) in state.nodes.iter().enumerate().skip(1) {
            let kind = export_kind_name(&node.kind).ok_or_else(|| {
                TileGraphError::InvalidGraph(format!(
                    "node kind '{:?}' is not exportable to .tilegraph yet",
                    node.kind
                ))
            })?;
            let count = per_kind_counts.entry(kind).or_default();
            *count += 1;
            let name = if *count == 1 {
                "main".to_string()
            } else {
                count.to_string()
            };
            let path = format!("{kind}.{name}");
            paths[index] = Some(path.clone());

            let table = export_node_table(&node.kind, node.position)?;
            doc.node
                .entry(kind.to_string())
                .or_default()
                .insert(name, table);
        }

        for (src_node, src_terminal, dest_node, dest_terminal) in &state.connections {
            let src_path = paths
                .get(*src_node as usize)
                .and_then(|p| p.as_ref())
                .ok_or_else(|| {
                    TileGraphError::InvalidGraph(format!(
                        "missing source node path for node {}",
                        src_node
                    ))
                })?;
            let src_kind =
                export_kind_name(&state.nodes[*src_node as usize].kind).unwrap_or("output");
            let src_port = export_output_port_name(src_kind, *src_terminal).ok_or_else(|| {
                TileGraphError::InvalidGraph(format!(
                    "unsupported output terminal {} for {}",
                    src_terminal, src_kind
                ))
            })?;
            let reference = format!("{src_path}:{src_port}");

            if *dest_node == 0 {
                let input_name = export_input_name("output", *dest_terminal).ok_or_else(|| {
                    TileGraphError::InvalidGraph(format!(
                        "unsupported output input terminal {}",
                        dest_terminal
                    ))
                })?;
                let output_nodes = doc.node.entry("output".to_string()).or_default();
                let output_table = output_nodes.entry("main".to_string()).or_default();
                output_table.insert(input_name.to_string(), toml::Value::String(reference));
                continue;
            }

            let dest_path = paths
                .get(*dest_node as usize)
                .and_then(|p| p.as_ref())
                .ok_or_else(|| {
                    TileGraphError::InvalidGraph(format!(
                        "missing destination node path for node {}",
                        dest_node
                    ))
                })?;
            let (dest_kind, dest_name) = dest_path.split_once('.').ok_or_else(|| {
                TileGraphError::InvalidGraph(format!("invalid destination path: {dest_path}"))
            })?;
            let input_name = export_input_name(dest_kind, *dest_terminal).ok_or_else(|| {
                TileGraphError::InvalidGraph(format!(
                    "unsupported input terminal {} for {}",
                    dest_terminal, dest_kind
                ))
            })?;

            let table = doc
                .node
                .get_mut(dest_kind)
                .and_then(|g| g.get_mut(dest_name))
                .ok_or_else(|| {
                    TileGraphError::InvalidGraph(format!(
                        "missing destination table for {dest_kind}.{dest_name}"
                    ))
                })?;
            table.insert(input_name.to_string(), toml::Value::String(reference));
        }

        Ok(doc)
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = TileGraphNode<'_>> {
        self.node.iter().flat_map(|(kind, group)| {
            group.iter().map(move |(name, table)| TileGraphNode {
                kind: kind.clone(),
                name: name.clone(),
                table,
            })
        })
    }

    pub fn node(&self, kind: &str, name: &str) -> Option<TileGraphNode<'_>> {
        self.node.get(kind)?.get(name).map(|table| TileGraphNode {
            kind: kind.to_string(),
            name: name.to_string(),
            table,
        })
    }

    pub fn resolve_endpoint(&self, value: &str) -> Option<NodeEndpoint> {
        let parsed = TileGraphRef::parse(value)?;
        let (node_kind, node_name) = parsed.node_path.split_once('.')?;
        self.node(node_kind, node_name)?;
        Some(NodeEndpoint {
            node_kind: node_kind.to_string(),
            node_name: node_name.to_string(),
            port: parsed.port.to_string(),
        })
    }
}

fn export_kind_name(kind: &TileNodeKind) -> Option<&'static str> {
    match kind {
        TileNodeKind::OutputRoot => Some("output"),
        TileNodeKind::Voronoi { .. } => Some("voronoi"),
        TileNodeKind::Blur { .. } => Some("blur"),
        TileNodeKind::SlopeBlur { .. } => Some("slope_blur"),
        TileNodeKind::Levels { .. } => Some("levels"),
        TileNodeKind::IdRandom => Some("id_random"),
        TileNodeKind::Noise { .. } => Some("noise"),
        TileNodeKind::Multiply => Some("multiply"),
        TileNodeKind::Subtract => Some("subtract"),
        TileNodeKind::Add => Some("add"),
        TileNodeKind::Min => Some("min"),
        TileNodeKind::Max => Some("max"),
        TileNodeKind::Material { .. } => Some("material"),
        TileNodeKind::Scalar { .. } => Some("scalar"),
        _ => None,
    }
}

fn export_input_name(kind: &str, input: u8) -> Option<&'static str> {
    match kind {
        "output" => match input {
            0 => Some("color"),
            1 => Some("material"),
            _ => None,
        },
        "blur" | "slope_blur" | "levels" | "threshold" | "curve" => match input {
            0 => Some("in"),
            _ => None,
        },
        "id_random" => match input {
            0 => Some("id"),
            _ => None,
        },
        "multiply" | "subtract" | "add" | "min" | "max" => match input {
            0 => Some("a"),
            1 => Some("b"),
            _ => None,
        },
        _ => None,
    }
}

fn export_output_port_name(kind: &str, output: u8) -> Option<&'static str> {
    match kind {
        "voronoi" => match output {
            0 => Some("center"),
            1 => Some("height"),
            2 => Some("cell_id"),
            _ => None,
        },
        "material" => match output {
            0 => Some("material"),
            _ => None,
        },
        "output" => None,
        _ => match output {
            0 => Some("field"),
            _ => None,
        },
    }
}

fn export_node_table(
    kind: &TileNodeKind,
    position: (i32, i32),
) -> Result<toml::Table, TileGraphError> {
    let mut table = toml::Table::new();
    table.insert(
        "pos".to_string(),
        toml::Value::Array(vec![
            toml::Value::Integer(position.0 as i64),
            toml::Value::Integer(position.1 as i64),
        ]),
    );

    match kind {
        TileNodeKind::Voronoi {
            scale,
            seed,
            jitter,
        } => {
            table.insert("scale".to_string(), toml::Value::Float(*scale as f64));
            table.insert("seed".to_string(), toml::Value::Integer(*seed as i64));
            table.insert("jitter".to_string(), toml::Value::Float(*jitter as f64));
        }
        TileNodeKind::Blur { radius } => {
            table.insert("radius".to_string(), toml::Value::Float(*radius as f64));
        }
        TileNodeKind::SlopeBlur { radius, amount } => {
            table.insert("radius".to_string(), toml::Value::Float(*radius as f64));
            table.insert("amount".to_string(), toml::Value::Float(*amount as f64));
        }
        TileNodeKind::Levels { level, width } => {
            table.insert("level".to_string(), toml::Value::Float(*level as f64));
            table.insert("width".to_string(), toml::Value::Float(*width as f64));
        }
        TileNodeKind::Noise { scale, seed, wrap } => {
            table.insert("scale".to_string(), toml::Value::Float(*scale as f64));
            table.insert("seed".to_string(), toml::Value::Integer(*seed as i64));
            table.insert("wrap".to_string(), toml::Value::Boolean(*wrap));
        }
        TileNodeKind::Material {
            roughness,
            metallic,
            opacity,
            emissive,
        } => {
            table.insert(
                "roughness".to_string(),
                toml::Value::Float(*roughness as f64),
            );
            table.insert("metallic".to_string(), toml::Value::Float(*metallic as f64));
            table.insert("opacity".to_string(), toml::Value::Float(*opacity as f64));
            table.insert("emissive".to_string(), toml::Value::Float(*emissive as f64));
        }
        TileNodeKind::Scalar { value } => {
            table.insert("value".to_string(), toml::Value::Float(*value as f64));
        }
        TileNodeKind::IdRandom
        | TileNodeKind::Multiply
        | TileNodeKind::Subtract
        | TileNodeKind::Add
        | TileNodeKind::Min
        | TileNodeKind::Max => {}
        _ => {
            return Err(TileGraphError::InvalidGraph(format!(
                "node kind '{:?}' is not exportable to .tilegraph yet",
                kind
            )));
        }
    }

    Ok(table)
}

#[derive(Clone, Debug)]
pub struct TileGraphNode<'a> {
    pub kind: String,
    pub name: String,
    pub table: &'a toml::Table,
}

impl<'a> TileGraphNode<'a> {
    pub fn path(&self) -> String {
        format!("{}.{}", self.kind, self.name)
    }

    pub fn input_ref(&self, input_name: &str) -> Option<TileGraphRef<'a>> {
        let value = self.table.get(input_name)?.as_str()?;
        TileGraphRef::parse(value)
    }

    pub fn position(&self) -> Option<(i32, i32)> {
        let pos = self.table.get("pos")?.as_array()?;
        if pos.len() != 2 {
            return None;
        }
        let x = pos[0].as_integer()? as i32;
        let y = pos[1].as_integer()? as i32;
        Some((x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"
version = 1
name = "Voronoi Height Wall"
grid = "3x3"
tile_size = "32x32"

[palette]
name = "Steam Lords"
link = "https://lospec.com/palette-list/steam-lords"
colors = ["#213b25", "#3a604a"]

[node.voronoi.main]
pos = [60, 120]
scale = 0.28
seed = 11
jitter = 0.92

[node.blur.main]
pos = [240, 60]
in = "voronoi.main:height"
radius = 0.014

[node.output.main]
color = "blur.main:field"
"##;

    #[test]
    fn parses_human_readable_graph_document() {
        let doc = TileGraphDocument::from_toml_str(SAMPLE).unwrap();
        assert_eq!(doc.version, 1);
        assert_eq!(doc.name, "Voronoi Height Wall");
        assert_eq!(
            doc.palette.as_ref().unwrap().name.as_deref(),
            Some("Steam Lords")
        );

        let blur = doc.node("blur", "main").unwrap();
        let input_ref = blur.input_ref("in").unwrap();
        assert_eq!(input_ref.node_path, "voronoi.main");
        assert_eq!(input_ref.port, "height");

        let endpoint = doc.resolve_endpoint("blur.main:field").unwrap();
        assert_eq!(endpoint.node_kind, "blur");
        assert_eq!(endpoint.node_name, "main");
        assert_eq!(endpoint.port, "field");
    }
}
