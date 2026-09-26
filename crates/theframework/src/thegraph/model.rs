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

/// One column of a list control. The control doubles as the cell prototype: it
/// decides how a cell is drawn and edited, and seeds a newly added row.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GraphListColumn {
    pub id: String,
    pub label: String,
    pub control: GraphControlValue,
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
    /// Repeating rows for multi-value parameters, such as several attribute
    /// writes or a table of event reactions. The trailing row adds a new entry.
    List {
        columns: Vec<GraphListColumn>,
        rows: Vec<Vec<GraphControlValue>>,
    },
}
/// Vertical pitch of one list entry.
pub const LIST_ROW_PITCH: f32 = 40.;
/// Column header strip above a list's rows.
pub const LIST_HEADER_PITCH: f32 = 30.;
/// Height of the title bar. Terminals and rows start below it.
pub const HEADER_PITCH: f32 = 58.;
/// Vertical pitch reserved per terminal, so a node without parameters still has
/// room to lay out its outputs.
pub const PORT_PITCH: f32 = 30.;
/// Space reserved above a control rect for its row label.
pub const ROW_LABEL_PITCH: f32 = 32.;
/// Pointer fraction at which a list's delete affordance starts.
pub const LIST_DELETE_FRACTION: f32 = 0.88;
/// Minimum vertical pitch of one row, including its label and spacing.
/// Existing documents store the old 62-unit pitch, so the floor is applied when
/// geometry is computed rather than by migrating every saved graph.
pub const MIN_ROW_PITCH: f32 = 76.;
/// The air inside a node. `Comfortable` is the original look; `Compact` halves
/// it so a branch of dialogue-sized nodes stays readable when zoomed out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphMetrics {
    /// Title bar height; body rows start below it.
    pub header: f32,
    /// Space above the first body row.
    pub body: f32,
    /// Minimum pitch of one row.
    pub row: f32,
    /// Space reserved above a row's control for its label.
    pub label: f32,
    pub list_header: f32,
    pub list_row: f32,
    /// Pitch reserved per terminal on a node with parameters.
    pub port: f32,
    /// Tighter pitch for the terminals of a folded node.
    pub folded_port: f32,
    /// Horizontal inset of a row's control.
    pub pad: f32,
    /// Height of the drawn title band inside the header.
    pub title_band: f32,
    pub title_size: f32,
    /// Body text, such as a text row's value.
    pub text_size: f32,
    pub label_size: f32,
    pub cell_size: f32,
    pub header_size: f32,
    /// Port labels sit outside the node in tighter space than body text.
    pub port_size: f32,
}
impl GraphMetrics {
    pub const STANDARD: Self = Self {
        header: HEADER_PITCH,
        body: 60.,
        row: MIN_ROW_PITCH,
        label: ROW_LABEL_PITCH,
        list_header: LIST_HEADER_PITCH,
        list_row: LIST_ROW_PITCH,
        port: PORT_PITCH,
        folded_port: 16.,
        pad: 18.,
        title_band: 35.,
        title_size: 18.,
        text_size: 14.,
        label_size: 12.,
        cell_size: 12.,
        header_size: 11.,
        port_size: 10.,
    };
    /// Where a row's label sits above its control rect.
    pub fn label_offset(&self) -> f32 {
        -(self.label * 0.6)
    }
}
impl Default for GraphMetrics {
    fn default() -> Self {
        Self::STANDARD
    }
}
/// Serialization helper: leave a `false` flag out of the saved document.
pub fn is_false(value: &bool) -> bool {
    !*value
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
                MIN_ROW_PITCH
            },
            id: Uuid::new_v4(),
            label: label.into(),
            value,
        }
    }
    /// Row extent including the label space and, for lists, one add-row. Lists
    /// grow with their contents, so their stored height is only a fallback.
    pub fn effective_height(&self, metrics: &GraphMetrics) -> f32 {
        match &self.value {
            GraphControlValue::List { rows, .. } => {
                metrics.label + metrics.list_header + metrics.list_row * (rows.len() as f32 + 1.)
            }
            _ => self.height.max(metrics.row),
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
    /// Folded to its title bar only. The rows stay in the document, so the
    /// author expands the node again instead of losing parameters. Left out of
    /// saved files while it is off, so untouched graphs keep their shape.
    #[serde(default, skip_serializing_if = "is_false")]
    pub folded: bool,
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
            folded: false,
        }
    }
    /// Terminals on the left and right edges.
    pub fn terminals(&self) -> usize {
        self.ports
            .iter()
            .filter(|port| matches!(port.side, PortSide::Left | PortSide::Right))
            .count()
    }
    pub fn height(&self, metrics: &GraphMetrics) -> f32 {
        let terminals = self.terminals() as f32;
        if self.folded {
            // Folded keeps its terminals spread out enough to stay clickable.
            return metrics
                .header
                .max(metrics.header * 0.5 + terminals * metrics.folded_port);
        }
        let body = metrics.body
            + self
                .rows
                .iter()
                .map(|row| row.effective_height(metrics))
                .sum::<f32>();
        // A parameter-less node is only a title bar; give its terminals room
        // instead of crowding them into the header.
        body.max(metrics.body + terminals * metrics.port)
    }
    pub fn rect(&self, metrics: &GraphMetrics) -> GraphRect {
        GraphRect {
            origin: self.position,
            size: [self.width, self.height(metrics)],
        }
    }
    pub fn row_rect(&self, index: usize, metrics: &GraphMetrics) -> GraphRect {
        let pad = metrics.pad;
        GraphRect {
            origin: [
                self.position[0] + pad,
                self.position[1]
                    + metrics.header
                    + self
                        .rows
                        .iter()
                        .take(index)
                        .map(|row| row.effective_height(metrics))
                        .sum::<f32>(),
            ],
            size: [
                self.width - pad * 2.,
                self.rows
                    .get(index)
                    .map(|row| row.effective_height(metrics) - metrics.label)
                    .unwrap_or(30.),
            ],
        }
    }
    /// Region in the title bar that folds or expands the node.
    pub fn fold_handle(&self, metrics: &GraphMetrics) -> GraphRect {
        GraphRect {
            origin: [self.position[0] + self.width - 30., self.position[1]],
            size: [30., metrics.title_band],
        }
    }
    /// Where a terminal that is not anchored to a row sits. A node without a
    /// laid out body has no rows to align to, so its terminals are placed between
    /// the title bar and the footer instead of across the whole node.
    fn terminal_offset(&self, t: f32, metrics: &GraphMetrics) -> f32 {
        let height = self.height(metrics);
        if (self.folded || self.rows.is_empty()) && height > metrics.header {
            metrics.header + t * (height - metrics.header)
        } else {
            t * height
        }
    }
    pub fn port_position(&self, port: &GraphPort, metrics: &GraphMetrics) -> Point {
        let t = port.position.clamp(0., 1.);
        let anchored = if self.folded {
            None
        } else {
            port.row
                .and_then(|id| self.rows.iter().position(|row| row.id == id))
                .map(|i| {
                    let rect = self.row_rect(i, metrics);
                    rect.origin[1] - self.position[1] + rect.size[1] * 0.5
                })
        };
        let y = anchored.unwrap_or_else(|| self.terminal_offset(t, metrics));
        match port.side {
            PortSide::Left => [self.position[0], self.position[1] + y],
            PortSide::Right => [self.position[0] + self.width, self.position[1] + y],
            PortSide::Top => [self.position[0] + t * self.width, self.position[1]],
            PortSide::Bottom => [
                self.position[0] + t * self.width,
                self.position[1] + self.height(metrics),
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
    pub fn metrics(&self) -> GraphMetrics {
        GraphMetrics::STANDARD
    }
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
    /// Frame a set of nodes inside a view of `size`, leaving a margin. Used to
    /// centre the branch an author just picked.
    pub fn fit_to_nodes(
        &mut self,
        doc: &GraphDocument,
        size: [f32; 2],
        nodes: &std::collections::HashSet<GraphId>,
    ) {
        let mut min = [f32::MAX; 2];
        let mut max = [f32::MIN; 2];
        let mut found = false;
        let metrics = doc.metrics();
        for node in doc.nodes.iter().filter(|node| nodes.contains(&node.id)) {
            let rect = node.rect(&metrics);
            min[0] = min[0].min(rect.origin[0]);
            min[1] = min[1].min(rect.origin[1]);
            max[0] = max[0].max(rect.origin[0] + rect.size[0]);
            max[1] = max[1].max(rect.origin[1] + rect.size[1]);
            found = true;
        }
        if !found || size[0] <= 1. || size[1] <= 1. {
            return;
        }
        const MARGIN: f32 = 60.;
        let width = (max[0] - min[0] + MARGIN * 2.).max(1.);
        let height = (max[1] - min[1] + MARGIN * 2.).max(1.);
        self.zoom = (size[0] / width).min(size[1] / height).clamp(0.25, 1.5);
        let center = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
        self.pan = [
            size[0] * 0.5 - center[0] * self.zoom,
            size[1] * 0.5 - center[1] * self.zoom,
        ];
    }
}
