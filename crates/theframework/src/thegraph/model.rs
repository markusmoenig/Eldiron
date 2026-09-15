use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type GraphId = Uuid;
pub type Point = [f32; 2];
pub type GraphColor = [u8; 4];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphRect {
    pub origin: Point,
    pub size: Point,
}
impl GraphRect {
    pub fn contains(self, p: Point) -> bool {
        p[0] >= self.origin[0]
            && p[1] >= self.origin[1]
            && p[0] <= self.origin[0] + self.size[0]
            && p[1] <= self.origin[1] + self.size[1]
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum PortSide {
    Left,
    Right,
    Top,
    Bottom,
}
impl PortSide {
    pub fn normal(self) -> Point {
        match self {
            Self::Left => [-1., 0.],
            Self::Right => [1., 0.],
            Self::Top => [0., -1.],
            Self::Bottom => [0., 1.],
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum PortDirection {
    Input,
    Output,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GraphPort {
    #[serde(default)]
    pub key: Option<String>,
    pub id: GraphId,
    pub label: String,
    pub direction: PortDirection,
    pub side: PortSide,
    /// Stable body row to align with; otherwise use normalized edge position.
    pub row: Option<GraphId>,
    pub position: f32,
    pub kind: String,
}
impl GraphPort {
    pub fn new(label: &str, direction: PortDirection, side: PortSide, position: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            key: None,
            label: label.into(),
            direction,
            side,
            row: None,
            position,
            kind: "flow".into(),
        }
    }
}

/// Serializable control data; custom controls use `Custom` and a host control registry.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GraphControlValue {
    Number {
        value: f32,
        min: f32,
        max: f32,
        step: f32,
    },
    Choice {
        options: Vec<String>,
        selected: usize,
    },
    Toggle(bool),
    /// Asset key resolved by the painter's host, not raw pixels in the document.
    Preview {
        asset: String,
        caption: String,
    },
    Label(String),
    /// Single-line, focusable text field.
    Text(String),
    Custom {
        kind: String,
        data: serde_json::Value,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GraphRow {
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub value_type: Option<super::GraphValueType>,
    #[serde(default)]
    pub binding: Option<super::GraphEventBinding>,
    /// Vertical extent in graph units, including label and spacing.
    pub height: f32,
    pub id: GraphId,
    pub label: String,
    pub value: GraphControlValue,
}
impl GraphRow {
    pub fn new(label: &str, value: GraphControlValue) -> Self {
        Self {
            key: None,
            value_type: None,
            binding: None,
            height: if matches!(value, GraphControlValue::Preview { .. }) {
                94.
            } else {
                62.
            },
            id: Uuid::new_v4(),
            label: label.into(),
            value,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GraphNode {
    #[serde(default)]
    pub definition: Option<String>,
    pub id: GraphId,
    pub title: String,
    pub position: Point,
    pub width: f32,
    pub color: GraphColor,
    pub rows: Vec<GraphRow>,
    pub ports: Vec<GraphPort>,
}
impl GraphNode {
    pub fn new(title: &str, position: Point, color: GraphColor) -> Self {
        Self {
            id: Uuid::new_v4(),
            definition: None,
            title: title.into(),
            position,
            width: 240.,
            color,
            rows: vec![],
            ports: vec![],
        }
    }
    pub fn height(&self) -> f32 {
        60. + self.rows.iter().map(|r| r.height.max(40.)).sum::<f32>()
    }
    pub fn rect(&self) -> GraphRect {
        GraphRect {
            origin: self.position,
            size: [self.width, self.height()],
        }
    }
    pub fn row_rect(&self, index: usize) -> GraphRect {
        GraphRect {
            origin: [
                self.position[0] + 18.,
                self.position[1]
                    + 58.
                    + self
                        .rows
                        .iter()
                        .take(index)
                        .map(|r| r.height.max(40.))
                        .sum::<f32>(),
            ],
            size: [
                self.width - 36.,
                self.rows
                    .get(index)
                    .map(|r| r.height.max(40.) - 32.)
                    .unwrap_or(30.),
            ],
        }
    }
    pub fn port_position(&self, port: &GraphPort) -> Point {
        let t = port.position.clamp(0., 1.);
        let y = port
            .row
            .and_then(|id| self.rows.iter().position(|r| r.id == id))
            .map(|i| {
                let r = self.row_rect(i);
                r.origin[1] - self.position[1] + r.size[1] * 0.5
            })
            .unwrap_or(t * self.height());
        match port.side {
            PortSide::Left => [self.position[0], self.position[1] + y],
            PortSide::Right => [self.position[0] + self.width, self.position[1] + y],
            PortSide::Top => [self.position[0] + t * self.width, self.position[1]],
            PortSide::Bottom => [
                self.position[0] + t * self.width,
                self.position[1] + self.height(),
            ],
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GraphConnection {
    pub id: GraphId,
    pub from: GraphId,
    pub to: GraphId,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GraphDocument {
    pub version: u32,
    pub nodes: Vec<GraphNode>,
    pub connections: Vec<GraphConnection>,
}
impl Default for GraphDocument {
    fn default() -> Self {
        Self {
            version: 1,
            nodes: vec![],
            connections: vec![],
        }
    }
}
impl GraphDocument {
    pub fn port(&self, id: GraphId) -> Option<(&GraphNode, &GraphPort)> {
        self.nodes
            .iter()
            .find_map(|n| n.ports.iter().find(|p| p.id == id).map(|p| (n, p)))
    }
    pub fn validate_connection(&self, from: GraphId, to: GraphId) -> Result<(), String> {
        let (a, p) = self.port(from).ok_or("Missing source")?;
        let (b, q) = self.port(to).ok_or("Missing destination")?;
        if a.id == b.id {
            return Err("Self connection".into());
        }
        if p.direction != PortDirection::Output || q.direction != PortDirection::Input {
            return Err("Connect an output to an input".into());
        }
        if p.kind != q.kind {
            return Err("Incompatible port kinds".into());
        }
        if self
            .connections
            .iter()
            .any(|c| c.from == from && c.to == to)
        {
            return Err("Connection already exists".into());
        }
        Ok(())
    }
}

/// Host gameplay policy supplements structural validation (cycles, cardinality, etc.).
pub trait GraphConnectionPolicy {
    fn validate(&self, doc: &GraphDocument, from: GraphId, to: GraphId) -> Result<(), String>;
}
pub struct AllowGraphConnections;
impl GraphConnectionPolicy for AllowGraphConnections {
    fn validate(&self, _: &GraphDocument, _: GraphId, _: GraphId) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GraphCondition {
    Unknown,
    True,
    False,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GraphExecution {
    Idle,
    Running,
    Waiting,
    Suspended,
    Completed,
    Failed,
}
#[derive(Clone, Debug)]
pub struct GraphObservation {
    pub condition: GraphCondition,
    pub execution: GraphExecution,
    pub text: String,
}
impl Default for GraphObservation {
    fn default() -> Self {
        Self {
            condition: GraphCondition::Unknown,
            execution: GraphExecution::Idle,
            text: String::new(),
        }
    }
}
/// Read-only inspection. Implementations must not execute gameplay or draw random samples.
pub trait GraphContext {
    /// Authoring diagnostics, independent of execution state and condition truth.
    fn diagnostic(&self, _node: &GraphNode) -> Option<String> {
        None
    }
    fn node_title(&self, _node: &GraphNode) -> Option<String> {
        None
    }
    fn row_label(&self, _node: &GraphNode, _row: &GraphRow) -> Option<String> {
        None
    }
    fn observe(&self, node: &GraphNode) -> GraphObservation;
    fn node_active(&self, _node: GraphId) -> bool {
        false
    }
    fn connection_active(&self, _connection: GraphId) -> bool {
        false
    }
}
impl GraphContext for () {
    fn observe(&self, _: &GraphNode) -> GraphObservation {
        GraphObservation::default()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GraphViewport {
    pub pan: Point,
    zoom: f32,
}
impl Default for GraphViewport {
    fn default() -> Self {
        Self {
            pan: [40., 40.],
            zoom: 1.,
        }
    }
}
impl GraphViewport {
    pub fn zoom(&self) -> f32 {
        self.zoom
    }
    pub fn to_screen(&self, p: Point) -> Point {
        [
            p[0] * self.zoom + self.pan[0],
            p[1] * self.zoom + self.pan[1],
        ]
    }
    pub fn to_graph(&self, p: Point) -> Point {
        [
            (p[0] - self.pan[0]) / self.zoom,
            (p[1] - self.pan[1]) / self.zoom,
        ]
    }
    pub fn zoom_at(&mut self, cursor: Point, factor: f32) {
        if !factor.is_finite() || factor <= 0. {
            return;
        }
        let anchor = self.to_graph(cursor);
        self.zoom = (self.zoom * factor).clamp(0.25, 3.);
        self.pan = [
            cursor[0] - anchor[0] * self.zoom,
            cursor[1] - anchor[1] * self.zoom,
        ];
    }
}
