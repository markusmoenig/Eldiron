use super::*;
use uuid::Uuid;

/// One completed gesture produces one reversible edit. The host can persist/undo it.
#[derive(Clone, Debug, PartialEq)]
pub enum GraphEdit {
    AddNode {
        node: GraphNode,
    },
    Move {
        node: GraphId,
        before: Point,
        after: Point,
    },
    SetValue {
        node: GraphId,
        row: GraphId,
        before: GraphControlValue,
        after: GraphControlValue,
    },
    Connection {
        connection: GraphConnection,
        added: bool,
    },
    ReplaceNode {
        before: GraphNode,
        after: GraphNode,
    },
}
impl GraphEdit {
    pub fn apply(&self, doc: &mut GraphDocument, forward: bool) {
        match self {
            Self::AddNode { node } => {
                doc.nodes.retain(|n| n.id != node.id);
                if forward {
                    doc.nodes.push(node.clone());
                }
            }
            Self::ReplaceNode { before, after } => {
                if let Some(n) = doc.nodes.iter_mut().find(|n| n.id == before.id) {
                    *n = if forward { after } else { before }.clone();
                }
            }
            Self::Move {
                node,
                before,
                after,
            } => {
                if let Some(n) = doc.nodes.iter_mut().find(|n| n.id == *node) {
                    n.position = if forward { *after } else { *before };
                }
            }
            Self::SetValue {
                node,
                row,
                before,
                after,
            } => {
                if let Some(r) = doc
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == *node)
                    .and_then(|n| n.rows.iter_mut().find(|r| r.id == *row))
                {
                    r.value = if forward { after } else { before }.clone();
                }
            }
            Self::Connection { connection, added } => {
                doc.connections.retain(|c| c.id != connection.id);
                if *added == forward {
                    doc.connections.push(connection.clone());
                }
            }
        }
    }
}
#[derive(Clone, Debug)]
enum Gesture {
    Pan {
        start: Point,
        before: Point,
    },
    Move {
        node: GraphId,
        start: Point,
        before: Point,
    },
    Control {
        node: GraphId,
        row: GraphId,
        before: GraphControlValue,
    },
    Wire(GraphId),
}
#[derive(Default)]
pub struct GraphEditor {
    pub viewport: GraphViewport,
    pub selected: Option<GraphId>,
    pub selected_connection: Option<GraphId>,
    pub cursor: Point,
    pub error: Option<String>,
    gesture: Option<Gesture>,
    pub(crate) text_focus: Option<GraphTextFocus>,
    edits: Vec<GraphEdit>,
}
impl GraphEditor {
    pub(crate) fn record_edit(&mut self, edit: GraphEdit) {
        self.edits.push(edit);
    }
    pub fn take_edits(&mut self) -> Vec<GraphEdit> {
        std::mem::take(&mut self.edits)
    }
    pub fn pending_wire(&self) -> Option<GraphId> {
        if let Some(Gesture::Wire(id)) = self.gesture {
            Some(id)
        } else {
            None
        }
    }
    pub fn port_at(&self, doc: &GraphDocument, screen: Point) -> Option<GraphId> {
        doc.nodes.iter().rev().find_map(|n| {
            n.ports
                .iter()
                .find(|p| {
                    let a = self.viewport.to_screen(n.port_position(p));
                    (a[0] - screen[0]).hypot(a[1] - screen[1])
                        <= (7. * self.viewport.zoom()).max(7.)
                })
                .map(|p| p.id)
        })
    }
    pub fn pointer_down(
        &mut self,
        doc: &mut GraphDocument,
        screen: Point,
        controls: &dyn GraphControls,
    ) {
        self.finish_text(doc, true);
        self.cursor = screen;
        self.error = None;
        let p = self.viewport.to_graph(screen);
        if let Some(port) = self.port_at(doc, screen) {
            self.gesture = Some(Gesture::Wire(port));
            return;
        }
        self.selected_connection = None;
        if let Some(n) = doc.nodes.iter().rev().find(|n| n.rect().contains(p)) {
            self.selected = Some(n.id);
            for (i, r) in n.rows.iter().enumerate() {
                let rect = n.row_rect(i);
                if rect.contains(p) {
                    if r.binding.is_some() {
                        return;
                    }
                    if let GraphControlValue::Text(value) = &r.value {
                        self.gesture = None;
                        self.text_focus = Some(GraphTextFocus {
                            node: n.id,
                            row: r.id,
                            original: value.clone(),
                            caret: value.len(),
                            anchor: 0,
                        });
                        return;
                    }
                    self.gesture = Some(Gesture::Control {
                        node: n.id,
                        row: r.id,
                        before: r.value.clone(),
                    });
                    self.update_control(doc, screen, controls, true);
                    return;
                }
            }
            self.gesture = Some(Gesture::Move {
                node: n.id,
                start: p,
                before: n.position,
            });
        } else {
            self.selected = None;
            self.selected_connection = doc
                .connections
                .iter()
                .rev()
                .find(|c| {
                    let Some(points) = connection_curve(doc, c, &self.viewport) else {
                        return false;
                    };
                    (0..40).any(|i| {
                        distance_segment(
                            screen,
                            bezier(points, i as f32 / 40.),
                            bezier(points, (i + 1) as f32 / 40.),
                        ) < 6.
                    })
                })
                .map(|c| c.id);
            if self.selected_connection.is_none() {
                self.gesture = Some(Gesture::Pan {
                    start: screen,
                    before: self.viewport.pan,
                });
            }
        }
    }
    fn update_control(
        &mut self,
        doc: &mut GraphDocument,
        screen: Point,
        controls: &dyn GraphControls,
        press: bool,
    ) {
        let Some(Gesture::Control { node, row, .. }) = self.gesture else {
            return;
        };
        let p = self.viewport.to_graph(screen);
        if let Some(n) = doc.nodes.iter_mut().find(|n| n.id == node) {
            if let Some(i) = n.rows.iter().position(|r| r.id == row) {
                let rect = n.row_rect(i);
                let fraction = (p[0] - rect.origin[0]) / rect.size[0];
                let input = if press {
                    GraphControlInput::Press { fraction }
                } else {
                    GraphControlInput::Drag { fraction }
                };
                if let Some(v) = controls.interact(&n.rows[i].value, input) {
                    n.rows[i].value = v;
                }
            }
        }
    }
    pub fn pointer_move(
        &mut self,
        doc: &mut GraphDocument,
        screen: Point,
        controls: &dyn GraphControls,
    ) {
        self.cursor = screen;
        match self.gesture.clone() {
            Some(Gesture::Pan { start, before }) => {
                self.viewport.pan = [
                    before[0] + screen[0] - start[0],
                    before[1] + screen[1] - start[1],
                ]
            }
            Some(Gesture::Move {
                node,
                start,
                before,
            }) => {
                let p = self.viewport.to_graph(screen);
                if let Some(n) = doc.nodes.iter_mut().find(|n| n.id == node) {
                    n.position = [before[0] + p[0] - start[0], before[1] + p[1] - start[1]];
                }
            }
            Some(Gesture::Control { .. }) => self.update_control(doc, screen, controls, false),
            _ => {}
        }
    }
    pub fn pointer_up(
        &mut self,
        doc: &mut GraphDocument,
        screen: Point,
        policy: &dyn GraphConnectionPolicy,
    ) {
        self.cursor = screen;
        match self.gesture.take() {
            Some(Gesture::Move { node, before, .. }) => {
                if let Some(n) = doc.nodes.iter().find(|n| n.id == node) {
                    if before != n.position {
                        self.edits.push(GraphEdit::Move {
                            node,
                            before,
                            after: n.position,
                        });
                    }
                }
            }
            Some(Gesture::Control { node, row, before }) => {
                if let Some(r) = doc
                    .nodes
                    .iter()
                    .find(|n| n.id == node)
                    .and_then(|n| n.rows.iter().find(|r| r.id == row))
                {
                    if before != r.value {
                        self.edits.push(GraphEdit::SetValue {
                            node,
                            row,
                            before,
                            after: r.value.clone(),
                        });
                    }
                }
            }
            Some(Gesture::Wire(a)) => {
                if let Some(b) = self.port_at(doc, screen) {
                    let (from, to) = if doc
                        .port(a)
                        .is_some_and(|(_, p)| p.direction == PortDirection::Output)
                    {
                        (a, b)
                    } else {
                        (b, a)
                    };
                    match doc
                        .validate_connection(from, to)
                        .and_then(|()| policy.validate(doc, from, to))
                    {
                        Ok(()) => {
                            let connection = GraphConnection {
                                id: Uuid::new_v4(),
                                from,
                                to,
                            };
                            doc.connections.push(connection.clone());
                            self.edits.push(GraphEdit::Connection {
                                connection,
                                added: true,
                            });
                        }
                        Err(e) => self.error = Some(e),
                    }
                }
            }
            _ => {}
        }
    }
    pub fn cancel(&mut self, doc: &mut GraphDocument) {
        self.finish_text(doc, false);
        match self.gesture.take() {
            Some(Gesture::Move { node, before, .. }) => {
                if let Some(n) = doc.nodes.iter_mut().find(|n| n.id == node) {
                    n.position = before;
                }
            }
            Some(Gesture::Control { node, row, before }) => {
                if let Some(r) = doc
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == node)
                    .and_then(|n| n.rows.iter_mut().find(|r| r.id == row))
                {
                    r.value = before;
                }
            }
            Some(Gesture::Pan { before, .. }) => self.viewport.pan = before,
            _ => {}
        }
    }
    pub fn delete_connection(&mut self, doc: &mut GraphDocument) {
        if let Some(id) = self.selected_connection.take() {
            if let Some(i) = doc.connections.iter().position(|c| c.id == id) {
                self.edits.push(GraphEdit::Connection {
                    connection: doc.connections.remove(i),
                    added: false,
                });
            }
        }
    }
}
pub fn connection_curve(
    doc: &GraphDocument,
    c: &GraphConnection,
    view: &GraphViewport,
) -> Option<[Point; 4]> {
    let (a, p) = doc.port(c.from)?;
    let (b, q) = doc.port(c.to)?;
    Some(port_curve(
        view.to_screen(a.port_position(p)),
        p.side,
        view.to_screen(b.port_position(q)),
        q.side,
    ))
}
pub fn port_curve(a: Point, side_a: PortSide, b: Point, side_b: PortSide) -> [Point; 4] {
    let distance = ((b[0] - a[0]).hypot(b[1] - a[1]) * 0.4).clamp(30., 180.);
    let n = side_a.normal();
    let m = side_b.normal();
    [
        a,
        [a[0] + n[0] * distance, a[1] + n[1] * distance],
        [b[0] + m[0] * distance, b[1] + m[1] * distance],
        b,
    ]
}
pub fn bezier(p: [Point; 4], t: f32) -> Point {
    let u = 1. - t;
    std::array::from_fn(|i| {
        u * u * u * p[0][i]
            + 3. * u * u * t * p[1][i]
            + 3. * u * t * t * p[2][i]
            + t * t * t * p[3][i]
    })
}
fn distance_segment(p: Point, a: Point, b: Point) -> f32 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let t = (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1])
        / (d[0] * d[0] + d[1] * d[1]).max(0.0001))
    .clamp(0., 1.);
    (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1])
}
