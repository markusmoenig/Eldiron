//! Run: cargo run -p theframework --example behavior_graph --features ui
//! Headless: append -- --snapshot /tmp/behavior-graph.png
use std::collections::BTreeMap;
#[path = "behavior_graph/catalog.rs"]
mod catalog;
use theframework::{prelude::*, thegraph::*};

struct DemoControls;
impl GraphControls for DemoControls {
    fn label(&self, v: &GraphControlValue) -> String {
        match v {
            GraphControlValue::Custom { kind, data } if kind == "time" => {
                let minutes = data.as_u64().unwrap_or(0);
                format!("{:02}:{:02}", minutes / 60, minutes % 60)
            }
            GraphControlValue::Choice { .. } => format!("‹  {}  ›", BasicGraphControls.label(v)),
            _ => BasicGraphControls.label(v),
        }
    }
    fn interact(
        &self,
        v: &GraphControlValue,
        input: GraphControlInput,
    ) -> Option<GraphControlValue> {
        if let GraphControlValue::Custom { kind, .. } = v {
            if kind == "time" {
                let (GraphControlInput::Press { fraction, .. }
                | GraphControlInput::Drag { fraction, .. }) = input;
                let minutes = ((fraction.clamp(0., 1.) * 95.).round() as u32) * 15;
                return Some(GraphControlValue::Custom {
                    kind: kind.clone(),
                    data: minutes.into(),
                });
            }
        }
        BasicGraphControls.interact(v, input)
    }
}
struct PreviewContext {
    minutes: u32,
    office: bool,
    trace: bool,
    text_node: GraphId,
    time_node: GraphId,
    walk_node: GraphId,
    wire: GraphId,
}
impl GraphContext for PreviewContext {
    fn observe(&self, n: &GraphNode) -> GraphObservation {
        if n.id == self.text_node {
            let matches =
                matches!(&n.rows[0].value,GraphControlValue::Text(text) if text == "garden");
            return GraphObservation {
                condition: if matches {
                    GraphCondition::True
                } else {
                    GraphCondition::False
                },
                text: "Event destination: garden".into(),
                ..Default::default()
            };
        }
        if n.id == self.time_node || n.definition.as_deref() == Some("time_range") {
            let read = |i: usize| match &n.rows[i].value {
                GraphControlValue::Custom { data, .. } => data.as_u64().unwrap_or(0) as u32,
                _ => 0,
            };
            let start = read(0);
            let end = read(1);
            let inside = if start < end {
                self.minutes >= start && self.minutes < end
            } else if start > end {
                self.minutes >= start || self.minutes < end
            } else {
                false
            };
            return GraphObservation {
                condition: if inside {
                    GraphCondition::True
                } else {
                    GraphCondition::False
                },
                execution: GraphExecution::Idle,
                text: if inside {
                    "Inside time range"
                } else {
                    "Outside time range"
                }
                .into(),
            };
        }
        if n.id == self.walk_node || n.definition.as_deref() == Some("random_walk") {
            let destination = match &n.rows[0].value {
                GraphControlValue::Choice { options, selected } => {
                    options.get(*selected).map(String::as_str).unwrap_or("")
                }
                _ => "",
            };
            let area = if self.office { "Office" } else { "Garden" };
            let inside = destination == area;
            return GraphObservation {
                condition: if inside {
                    GraphCondition::True
                } else {
                    GraphCondition::False
                },
                execution: if self.trace {
                    GraphExecution::Running
                } else {
                    GraphExecution::Idle
                },
                text: format!("{} {}", if inside { "In" } else { "Outside" }, destination),
            };
        }
        GraphObservation {
            text: "Preview context".into(),
            ..Default::default()
        }
    }
    fn connection_active(&self, id: GraphId) -> bool {
        self.trace && id == self.wire
    }
}
struct DefinitionPreview<'a> {
    base: &'a PreviewContext,
    definitions: &'a GraphDefinitions,
    doc: &'a GraphDocument,
    samples: &'a BTreeMap<GraphId, GraphEventSample>,
}
impl GraphContext for DefinitionPreview<'_> {
    fn diagnostic(&self, n: &GraphNode) -> Option<String> {
        n.rows
            .iter()
            .find_map(|r| self.definitions.resolve(self.doc, n, r, self.samples).err())
    }
    fn observe(&self, n: &GraphNode) -> GraphObservation {
        for row in &n.rows {
            if let Err(error) = self.definitions.validate_binding(self.doc, n, row) {
                return GraphObservation {
                    execution: GraphExecution::Idle,
                    text: error,
                    ..Default::default()
                };
            }
        }
        if n.definition.as_deref() == Some("text_equals") {
            let row = n
                .rows
                .iter()
                .find(|r| r.key.as_deref() == Some("value"))
                .unwrap();
            let expected = n
                .rows
                .iter()
                .find(|r| r.key.as_deref() == Some("expected"))
                .unwrap();
            return match self.definitions.resolve(self.doc, n, row, self.samples) {
                Ok(Some(GraphDatum::Text(value))) => GraphObservation {
                    condition: if matches!(&expected.value,GraphControlValue::Text(t) if t==&value)
                    {
                        GraphCondition::True
                    } else {
                        GraphCondition::False
                    },
                    text: format!("Value: {value}"),
                    ..Default::default()
                },
                Ok(_) => GraphObservation {
                    text: "No event preview value".into(),
                    ..Default::default()
                },
                Err(error) => GraphObservation {
                    execution: GraphExecution::Idle,
                    text: error,
                    ..Default::default()
                },
            };
        }
        if n.definition.as_deref() == Some("event") {
            return GraphObservation {
                text: "Event entry · preview only".into(),
                ..Default::default()
            };
        }
        self.base.observe(n)
    }
    fn node_title(&self, n: &GraphNode) -> Option<String> {
        self.definitions.selected_event(n).map(|id| {
            format!(
                "On {}",
                self.definitions
                    .event(id)
                    .map(|e| e.label.as_str())
                    .unwrap_or(id)
            )
        })
    }
    fn row_label(&self, n: &GraphNode, r: &GraphRow) -> Option<String> {
        if let Some(b) = &r.binding {
            return Some(self.definitions.binding_label(b));
        }
        if matches!(&r.value,GraphControlValue::Custom {kind,..} if kind=="event") {
            return self.definitions.selected_event(n).map(|id| {
                format!(
                    "{}  ...",
                    self.definitions
                        .event(id)
                        .map(|e| e.label.as_str())
                        .unwrap_or(id)
                )
            });
        }
        None
    }
    fn connection_active(&self, id: GraphId) -> bool {
        self.base.connection_active(id)
    }
}
enum PickerAction {
    AddNode(Point),
    Event(GraphId),
    Binding(GraphId, GraphId, Vec<GraphEventBinding>),
}
struct DemoPopup {
    picker: GraphPicker,
    action: PickerAction,
}
struct DemoAssets(TheRGBABuffer);
impl GraphAssets for DemoAssets {
    fn image(&self, key: &str) -> Option<&TheRGBABuffer> {
        if key == "office" { Some(&self.0) } else { None }
    }
}

struct Demo {
    definitions: GraphDefinitions,
    samples: BTreeMap<GraphId, GraphEventSample>,
    popup: Option<DemoPopup>,
    shift: bool,
    accel: bool,
    assets: DemoAssets,
    doc: GraphDocument,
    editor: GraphEditor,
    resources: GraphRasterResources,
    context: PreviewContext,
    toolbar_drag: bool,
    undo: Vec<GraphEdit>,
    redo: Vec<GraphEdit>,
}
impl Demo {
    fn open_picker(&mut self, title: &str, items: Vec<GraphPickerItem>, action: PickerAction) {
        self.editor.finish_text(&mut self.doc, true);
        self.collect();
        self.popup = Some(DemoPopup {
            picker: GraphPicker::new(title, [420., 110.], items),
            action,
        });
    }
    fn node_picker(&mut self) {
        let position = self.editor.viewport.to_graph([640., 420.]);
        let items = self
            .definitions
            .nodes()
            .map(|n| GraphPickerItem {
                id: n.id.clone(),
                label: format!("{} / {}", n.category, n.title),
            })
            .collect();
        self.open_picker("Add node", items, PickerAction::AddNode(position));
    }
    fn choose(&mut self, id: &str) {
        let Some(popup) = self.popup.take() else {
            return;
        };
        match popup.action {
            PickerAction::AddNode(position) => {
                if let Ok(node) = self.definitions.instantiate(id, position) {
                    self.editor.selected = Some(node.id);
                    self.doc.nodes.push(node.clone());
                    self.undo.push(GraphEdit::AddNode { node });
                    self.redo.clear();
                }
            }
            PickerAction::Event(node) => {
                if self.definitions.event(id).is_none() {
                    return;
                }
                if let Some(n) = self.doc.nodes.iter_mut().find(|n| n.id == node) {
                    let before = n.clone();
                    if let Some(row) = n
                        .rows
                        .iter_mut()
                        .find(|r| r.key.as_deref() == Some("event"))
                    {
                        row.value = GraphControlValue::Custom {
                            kind: "event".into(),
                            data: id.into(),
                        };
                        self.undo.push(GraphEdit::ReplaceNode {
                            before,
                            after: n.clone(),
                        });
                        self.redo.clear();
                    }
                }
            }
            PickerAction::Binding(node, row, options) => {
                let binding = if id == "literal" {
                    None
                } else {
                    options
                        .into_iter()
                        .find(|b| serde_json::to_string(b).unwrap() == id)
                };
                if id != "literal" && binding.is_none() {
                    return;
                }
                if let Some(n) = self.doc.nodes.iter_mut().find(|n| n.id == node) {
                    let before = n.clone();
                    if let Some(r) = n.rows.iter_mut().find(|r| r.id == row) {
                        r.binding = binding;
                    }
                    self.undo.push(GraphEdit::ReplaceNode {
                        before,
                        after: n.clone(),
                    });
                    self.redo.clear();
                }
            }
        }
        self.refresh_samples();
    }
    fn refresh_samples(&mut self) {
        self.samples.clear();
        for n in &self.doc.nodes {
            if let Some(event) = self.definitions.selected_event(n) {
                // Explicit demonstration payloads, not real engine events.
                let fields = match event {
                    "arrived" => {
                        BTreeMap::from([("destination".into(), GraphDatum::Text("garden".into()))])
                    }
                    "damaged" => BTreeMap::from([
                        ("kind".into(), GraphDatum::Text("physical".into())),
                        ("amount".into(), GraphDatum::Number(3.)),
                        ("attacker".into(), GraphDatum::Entity(42)),
                    ]),
                    "delivery" => BTreeMap::from([
                        ("location".into(), GraphDatum::Text("garden".into())),
                        ("quantity".into(), GraphDatum::Number(5.)),
                    ]),
                    _ => BTreeMap::new(),
                };
                self.samples.insert(
                    n.id,
                    GraphEventSample {
                        event: event.into(),
                        fields,
                    },
                );
            }
        }
    }
    fn parameter_picker(&mut self, screen: Point) -> bool {
        let p = self.editor.viewport.to_graph(screen);
        let metrics = self.doc.metrics();
        let hit = self
            .doc
            .nodes
            .iter()
            .rev()
            .find(|n| n.rect(&metrics).contains(p))
            .and_then(|n| {
                n.rows.iter().enumerate().find_map(|(i, r)| {
                    let rect = n.row_rect(i, &metrics);
                    let bind = GraphRect {
                        origin: [rect.origin[0] + rect.size[0] - 25., rect.origin[1] - 19.],
                        size: [25., 18.],
                    };
                    if r.value_type.is_some()
                        && (bind.contains(p) || r.binding.is_some() && rect.contains(p))
                    {
                        Some((n.id, r.id, false))
                    } else if rect.contains(p)
                        && matches!(&r.value,GraphControlValue::Custom {kind,..} if kind=="event")
                    {
                        Some((n.id, r.id, true))
                    } else {
                        None
                    }
                })
            });
        let Some((node, row, event)) = hit else {
            return false;
        };
        if event {
            let items = self
                .definitions
                .events()
                .map(|e| GraphPickerItem {
                    id: e.id.clone(),
                    label: e.label.clone(),
                })
                .collect();
            self.open_picker("Select event", items, PickerAction::Event(node));
        } else {
            let n = self.doc.nodes.iter().find(|n| n.id == node).unwrap();
            let r = n.rows.iter().find(|r| r.id == row).unwrap();
            let options = self.definitions.binding_options(&self.doc, n, r);
            let mut items = vec![GraphPickerItem {
                id: "literal".into(),
                label: "Literal value (edit directly)".into(),
            }];
            items.extend(options.iter().map(|(label, b)| GraphPickerItem {
                id: serde_json::to_string(b).unwrap(),
                label: label.clone(),
            }));
            self.open_picker(
                "Bind parameter",
                items,
                PickerAction::Binding(node, row, options.into_iter().map(|(_, b)| b).collect()),
            );
        }
        true
    }
    fn collect(&mut self) {
        let edits = self.editor.take_edits();
        if !edits.is_empty() {
            self.redo.clear();
            self.undo.extend(edits);
        }
    }
    fn toolbar(&mut self, x: f32) {
        self.context.minutes = (((x - 25.) / 310.).clamp(0., 1.) * 1439.).round() as u32;
    }
    fn render(&mut self, pixels: &mut [u8], width: usize, height: usize, density: f32) {
        let size = [width as f32 / density, height as f32 / density];
        let mut painter = RasterGraphPainter::new(
            pixels,
            width,
            height,
            density,
            &mut self.resources,
            &self.assets,
        );
        self.editor.paint(
            &self.doc,
            &DefinitionPreview {
                base: &self.context,
                definitions: &self.definitions,
                doc: &self.doc,
                samples: &self.samples,
            },
            &DemoControls,
            &mut painter,
            size,
            &GraphTheme::default(),
        );
        painter.round_rect(
            GraphRect {
                origin: [0., 0.],
                size: [size[0], 100.],
            },
            0.,
            [27, 29, 30, 255],
        );
        painter.text(
            GraphRect {
                origin: [24., 12.],
                size: [1100., 24.],
            },
            "BEHAVIOR GRAPH   /   Framework prototype",
            18.,
            [231, 234, 230, 255],
        );
        painter.round_rect(
            GraphRect {
                origin: [25., 48.],
                size: [310., 30.],
            },
            15.,
            [64, 77, 75, 255],
        );
        painter.round_rect(
            GraphRect {
                origin: [25., 48.],
                size: [310. * self.context.minutes as f32 / 1439., 30.],
            },
            15.,
            [26, 116, 99, 255],
        );
        painter.text(
            GraphRect {
                origin: [38., 52.],
                size: [280., 22.],
            },
            &format!(
                "Preview time   {:02}:{:02}  · drag",
                self.context.minutes / 60,
                self.context.minutes % 60
            ),
            14.,
            [244, 244, 240, 255],
        );
        for (x, label) in [
            (
                360.,
                if self.context.office {
                    "Area: Office"
                } else {
                    "Area: Garden"
                },
            ),
            (
                540.,
                if self.context.trace {
                    "Trace: simulated ON"
                } else {
                    "Trace: OFF"
                },
            ),
            (755., "Add choice"),
            (900., "Undo"),
            (1000., "Redo"),
            (1140., "+ Node"),
        ] {
            painter.round_rect(
                GraphRect {
                    origin: [x, 48.],
                    size: [
                        if x == 540. {
                            195.
                        } else if x == 360. {
                            160.
                        } else if x == 900. {
                            80.
                        } else {
                            120.
                        },
                        30.,
                    ],
                },
                15.,
                [63, 65, 66, 255],
            );
            painter.text(
                GraphRect {
                    origin: [x + 10., 53.],
                    size: [180., 22.],
                },
                label,
                13.,
                [228, 231, 227, 255],
            );
        }
        let help = self.editor.error.as_deref().unwrap_or("Drag background to pan · wheel to zoom · drag ports to connect · select wire + Delete · Esc cancels");
        painter.round_rect(
            GraphRect {
                origin: [0., size[1] - 35.],
                size: [size[0], 35.],
            },
            0.,
            [27, 29, 30, 255],
        );
        painter.text(
            GraphRect {
                origin: [24., size[1] - 28.],
                size: [size[0] - 48., 24.],
            },
            help,
            13.,
            [165, 177, 174, 255],
        );
        if let Some(popup) = &self.popup {
            popup.picker.paint(&mut painter);
        }
    }
}
impl TheTrait for Demo {
    fn new() -> Self {
        let definitions = catalog::definitions();
        let routine = definitions.instantiate("routine", [50., 140.]).unwrap();
        let time = definitions.instantiate("time_range", [400., 140.]).unwrap();
        let walk = definitions
            .instantiate("random_walk", [790., 140.])
            .unwrap();
        let dialogue = definitions.instantiate("dialogue", [50., 950.]).unwrap();
        let arrived = definitions.instantiate("event", [50., 600.]).unwrap();
        let mut equals = definitions
            .instantiate("text_equals", [400., 600.])
            .unwrap();
        let say = definitions.instantiate("say", [790., 650.]).unwrap();
        equals.rows[1].binding = Some(GraphEventBinding {
            node: arrived.id,
            event: "arrived".into(),
            field: "destination".into(),
        });
        let wire = |a: &GraphNode, ap: usize, b: &GraphNode, bp: usize| GraphConnection {
            id: Uuid::new_v4(),
            from: a.ports[ap].id,
            to: b.ports[bp].id,
        };
        let c1 = wire(&routine, 0, &time, 0);
        let c2 = wire(&time, 1, &walk, 0);
        let c3 = wire(&arrived, 0, &equals, 0);
        let c4 = wire(&equals, 1, &say, 0);
        let samples = BTreeMap::from([(
            arrived.id,
            GraphEventSample {
                event: "arrived".into(),
                fields: BTreeMap::from([("destination".into(), GraphDatum::Text("garden".into()))]),
            },
        )]);
        let context = PreviewContext {
            text_node: equals.id,
            minutes: 900,
            office: true,
            trace: false,
            time_node: time.id,
            walk_node: walk.id,
            wire: c2.id,
        };
        let bytes = theframework::Embedded::get("fonts/Roboto-Bold.ttf").unwrap();
        let font = fontdue::Font::from_bytes(bytes.data.as_ref(), fontdue::FontSettings::default())
            .unwrap();
        let mut thumbnail = TheRGBABuffer::new(TheDim::new(0, 0, 120, 36));
        for y in 0..36 {
            for x in 0..120 {
                let wall = x < 4 || x >= 116 || y < 4 || y >= 32;
                let desk = (x > 18 && x < 42 || x > 76 && x < 100) && y > 10 && y < 25;
                let color = if wall {
                    [107, 121, 124, 255]
                } else if desk {
                    [157, 111, 67, 255]
                } else if (x / 6 + y / 6) % 2 == 0 {
                    [55, 65, 66, 255]
                } else {
                    [64, 75, 73, 255]
                };
                thumbnail.set_pixel(x, y, &color);
            }
        }
        Self {
            definitions,
            samples,
            popup: None,
            shift: false,
            accel: false,
            assets: DemoAssets(thumbnail),
            doc: GraphDocument {
                version: 1,
                nodes: vec![routine, time, walk, arrived, equals, say, dialogue],
                connections: vec![c1, c2, c3, c4],
                compact: false,
            },
            editor: GraphEditor::default(),
            resources: GraphRasterResources::new(font),
            context,
            toolbar_drag: false,
            undo: vec![],
            redo: vec![],
        }
    }
    fn window_title(&self) -> String {
        "Eldiron · Behavior Graph Prototype".into()
    }
    fn default_window_size(&self) -> (usize, usize) {
        (1280, 940)
    }
    fn min_window_size(&self) -> (usize, usize) {
        (800, 600)
    }
    fn target_fps(&self) -> f64 {
        60.
    }
    fn native_ui_rendering(&self) -> bool {
        true
    }
    fn draw(&mut self, pixels: &mut [u8], ctx: &mut TheContext) {
        self.render(
            pixels,
            ctx.framebuffer_width,
            ctx.framebuffer_height,
            ctx.ui_render_scale,
        );
    }
    fn touch_down(&mut self, x: f32, y: f32, _: &mut TheContext) -> bool {
        if let Some(popup) = &self.popup {
            if let Some(id) = popup.picker.pick([x, y]) {
                self.choose(&id);
            } else if !popup.picker.contains([x, y]) {
                self.popup = None;
            }
            return true;
        }
        if y < 100. {
            self.editor.finish_text(&mut self.doc, true);
            self.collect();
            if (48. ..78.).contains(&y) {
                if (25. ..335.).contains(&x) {
                    self.toolbar_drag = true;
                    self.toolbar(x);
                } else if (360. ..520.).contains(&x) {
                    self.context.office = !self.context.office;
                } else if (540. ..735.).contains(&x) {
                    self.context.trace = !self.context.trace;
                } else if (755. ..875.).contains(&x) {
                    let n = self
                        .doc
                        .nodes
                        .iter_mut()
                        .find(|n| n.definition.as_deref() == Some("dialogue"))
                        .unwrap();
                    let before = n.clone();
                    let row = GraphRow::new(
                        "Choice",
                        GraphControlValue::Text(format!("Choice {}", n.rows.len() - 1)),
                    );
                    let mut port = GraphPort::new("", PortDirection::Output, PortSide::Right, 0.);
                    port.row = Some(row.id);
                    n.rows.push(row);
                    n.ports.push(port);
                    self.undo.push(GraphEdit::ReplaceNode {
                        before,
                        after: n.clone(),
                    });
                    self.redo.clear();
                } else if (900. ..990.).contains(&x) {
                    self.undo();
                } else if (1000. ..1120.).contains(&x) {
                    self.redo();
                } else if (1140. ..1260.).contains(&x) {
                    self.node_picker();
                }
            }
        } else if !self.parameter_picker([x, y]) {
            self.editor
                .pointer_down(&mut self.doc, [x, y], &DemoControls);
        }
        true
    }
    fn touch_dragged(&mut self, x: f32, y: f32, _: &mut TheContext) -> bool {
        if self.popup.is_some() {
            return true;
        }
        if self.toolbar_drag {
            self.toolbar(x);
        } else {
            self.editor
                .pointer_move(&mut self.doc, [x, y], &DemoControls);
        }
        true
    }
    fn touch_up(&mut self, x: f32, y: f32, _: &mut TheContext) -> bool {
        self.toolbar_drag = false;
        self.editor
            .pointer_up(&mut self.doc, [x, y], &AllowGraphConnections);
        self.collect();
        true
    }
    fn hover(&mut self, x: f32, y: f32, _: &mut TheContext) -> bool {
        self.editor.cursor = [x, y];
        if let Some(popup) = &mut self.popup {
            return popup.picker.hover(Some([x, y]));
        }
        false
    }
    fn mouse_wheel(&mut self, delta: (isize, isize), _: &mut TheContext) -> bool {
        if let Some(popup) = &mut self.popup {
            popup.picker.scroll_by(if delta.1 > 0 { -1 } else { 1 });
            return true;
        }
        self.editor
            .viewport
            .zoom_at(self.editor.cursor, (delta.1 as f32 * 0.002).exp());
        true
    }
    fn modifier_changed(&mut self, shift: bool, ctrl: bool, _: bool, logo: bool) -> bool {
        self.shift = shift;
        self.accel = ctrl || logo;
        false
    }
    fn key_down(&mut self, ch: Option<char>, key: Option<TheKeyCode>, _: &mut TheContext) -> bool {
        if let Some(popup) = &mut self.popup {
            match (ch, key) {
                (_, Some(TheKeyCode::Escape)) => self.popup = None,
                (_, Some(TheKeyCode::Delete)) => popup.picker.backspace(),
                (_, Some(TheKeyCode::Space)) => popup.picker.type_char(' '),
                (_, Some(TheKeyCode::Return)) => {
                    let id = popup.picker.filtered().first().map(|i| i.id.clone());
                    if let Some(id) = id {
                        self.choose(&id);
                    }
                }
                (Some(c), _) => popup.picker.type_char(c),
                _ => {}
            }
            return true;
        }
        if self.editor.text_focus().is_some() {
            let input = match (ch, key) {
                (Some('a' | 'A'), _) if self.accel => Some(GraphTextInput::SelectAll),
                (Some(_), _) if self.accel => None,
                (Some(c), _) => Some(GraphTextInput::Insert(c.to_string())),
                (_, Some(TheKeyCode::Space)) => Some(GraphTextInput::Insert(" ".into())),
                (_, Some(TheKeyCode::Left)) => Some(GraphTextInput::Left { extend: self.shift }),
                (_, Some(TheKeyCode::Right)) => Some(GraphTextInput::Right { extend: self.shift }),
                (_, Some(TheKeyCode::Home)) => Some(GraphTextInput::Home { extend: self.shift }),
                (_, Some(TheKeyCode::End)) => Some(GraphTextInput::End { extend: self.shift }),
                (_, Some(TheKeyCode::Delete)) => Some(GraphTextInput::Backspace),
                (_, Some(TheKeyCode::Return | TheKeyCode::Tab)) => Some(GraphTextInput::Commit),
                (_, Some(TheKeyCode::Escape)) => Some(GraphTextInput::Cancel),
                _ => None,
            };
            if let Some(input) = input {
                self.editor.text_input(&mut self.doc, input);
            }
            self.collect();
            return true;
        }
        match key {
            Some(TheKeyCode::Escape) => self.editor.cancel(&mut self.doc),
            Some(TheKeyCode::Delete) => {
                self.editor.delete_connection(&mut self.doc);
                self.collect();
            }
            _ => {}
        }
        true
    }
    fn undo(&mut self) {
        self.editor.finish_text(&mut self.doc, true);
        self.collect();
        if let Some(e) = self.undo.pop() {
            e.apply(&mut self.doc, false);
            self.redo.push(e);
            self.refresh_samples();
        }
    }
    fn redo(&mut self) {
        self.editor.finish_text(&mut self.doc, true);
        self.collect();
        if let Some(e) = self.redo.pop() {
            e.apply(&mut self.doc, true);
            self.undo.push(e);
            self.refresh_samples();
        }
    }
}
fn main() {
    let mut demo = Demo::new();
    let args: Vec<_> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--snapshot") {
        let path = args.get(i + 1).expect("--snapshot requires a PNG path");
        let option = |name: &str, default: f32| {
            args.iter()
                .position(|a| a == name)
                .and_then(|i| args.get(i + 1))
                .and_then(|a| a.parse::<f32>().ok())
                .unwrap_or(default)
        };
        let density = option("--density", 1.).clamp(1., 3.);
        demo.editor
            .viewport
            .zoom_at([640., 470.], option("--zoom", 1.));
        demo.context.minutes = (option("--hour", 15.).clamp(0., 23.99) * 60.) as u32;
        demo.context.trace = args.iter().any(|a| a == "--trace");
        if let Some(event) = args
            .iter()
            .position(|a| a == "--event")
            .and_then(|i| args.get(i + 1))
        {
            let id = demo
                .doc
                .nodes
                .iter()
                .find(|n| n.definition.as_deref() == Some("event"))
                .unwrap()
                .id;
            demo.open_picker("Event", vec![], PickerAction::Event(id));
            demo.choose(event);
        }
        let width = (1280. * density) as usize;
        let height = (940. * density) as usize;
        let mut pixels = vec![0; width * height * 4];
        demo.render(&mut pixels, width, height, density);
        if args.iter().any(|a| a == "--benchmark") {
            let start = std::time::Instant::now();
            for _ in 0..20 {
                demo.render(&mut pixels, width, height, density);
            }
            println!(
                "20 warm frames at {density}x: {:.2} ms/frame",
                start.elapsed().as_secs_f64() * 1000. / 20.
            );
        }
        if args.iter().any(|a| a == "--picker") {
            demo.node_picker();
            demo.render(&mut pixels, width, height, density);
        }
        if args.iter().any(|a| a == "--focus-text") {
            let metrics = demo.doc.metrics();
            let node = demo
                .doc
                .nodes
                .iter()
                .find(|n| n.id == demo.context.text_node)
                .unwrap();
            let r = node.row_rect(0, &metrics);
            let p = demo
                .editor
                .viewport
                .to_screen([r.origin[0] + 10., r.origin[1] + 10.]);
            demo.editor.pointer_down(&mut demo.doc, p, &DemoControls);
            demo.editor
                .pointer_up(&mut demo.doc, p, &AllowGraphConnections);
            demo.render(&mut pixels, width, height, density);
        }
        let file = std::fs::File::create(path).unwrap();
        let mut encoder = png::Encoder::new(file, width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
    } else {
        TheApp::new().run(Box::new(demo));
    }
}

#[cfg(test)]
mod thegraph_tests {
    use super::*;
    #[test]
    fn preview_changes_do_not_edit_document_or_imply_execution() {
        let mut demo = Demo::new();
        let original = demo.doc.clone();
        let mut ctx = TheContext::new(1280, 940, 1.);
        demo.touch_down(25., 60., &mut ctx);
        demo.touch_up(25., 60., &mut ctx);
        assert_eq!(
            demo.context.observe(&demo.doc.nodes[1]).condition,
            GraphCondition::False
        );
        assert_eq!(
            demo.context.observe(&demo.doc.nodes[2]).execution,
            GraphExecution::Idle
        );
        demo.touch_down(580., 60., &mut ctx);
        demo.touch_up(580., 60., &mut ctx);
        assert_eq!(
            demo.context.observe(&demo.doc.nodes[1]).condition,
            GraphCondition::False
        );
        assert_eq!(
            demo.context.observe(&demo.doc.nodes[2]).execution,
            GraphExecution::Running
        );
        assert_eq!(demo.doc, original);
    }
    #[test]
    fn dynamic_choice_and_port_undo_together() {
        let mut demo = Demo::new();
        let original = demo.doc.clone();
        let mut ctx = TheContext::new(1280, 940, 1.);
        demo.touch_down(780., 60., &mut ctx);
        demo.touch_up(780., 60., &mut ctx);
        let n = demo.doc.nodes.last().unwrap();
        assert_eq!(n.ports.last().unwrap().row, Some(n.rows.last().unwrap().id));
        let after = demo.doc.clone();
        demo.undo();
        assert_eq!(demo.doc, original);
        demo.redo();
        assert_eq!(demo.doc, after);
    }
    #[test]
    fn time_range_handles_end_boundary_and_overnight() {
        let mut demo = Demo::new();
        demo.context.minutes = 960;
        assert_eq!(
            demo.context.observe(&demo.doc.nodes[1]).condition,
            GraphCondition::False
        );
        demo.doc.nodes[1].rows[0].value = GraphControlValue::Custom {
            kind: "time".into(),
            data: 1320.into(),
        };
        demo.doc.nodes[1].rows[1].value = GraphControlValue::Custom {
            kind: "time".into(),
            data: 480.into(),
        };
        demo.context.minutes = 60;
        assert_eq!(
            demo.context.observe(&demo.doc.nodes[1]).condition,
            GraphCondition::True
        );
    }
    #[test]
    fn raster_clips_zoomed_nodes_and_preserves_document_at_high_density() {
        let mut demo = Demo::new();
        let original = demo.doc.clone();
        demo.editor.viewport.zoom_at([0., 0.], 2.3);
        demo.editor.viewport.pan = [-980., -350.];
        let mut pixels = vec![0; 800 * 600 * 4];
        demo.render(&mut pixels, 800, 600, 2.);
        assert!(pixels.chunks_exact(4).all(|p| p[3] == 255));
        assert_eq!(demo.doc, original);
    }
}

#[cfg(test)]
mod text_demo_tests {
    use super::*;
    #[test]
    fn thegraph_arrival_text_uses_focus_and_updates_context() {
        let mut demo = Demo::new();
        let mut ctx = TheContext::new(1280, 940, 1.);
        let node = demo
            .doc
            .nodes
            .iter()
            .find(|n| n.id == demo.context.text_node)
            .unwrap();
        let r = node.row_rect(0);
        let p = demo
            .editor
            .viewport
            .to_screen([r.origin[0] + 10., r.origin[1] + 10.]);
        demo.touch_down(p[0], p[1], &mut ctx);
        demo.touch_up(p[0], p[1], &mut ctx);
        for ch in "office".chars() {
            demo.key_down(Some(ch), None, &mut ctx);
        }
        let n = demo
            .doc
            .nodes
            .iter()
            .find(|n| n.id == demo.context.text_node)
            .unwrap();
        assert_eq!(demo.context.observe(n).condition, GraphCondition::False);
        demo.key_down(None, Some(TheKeyCode::Escape), &mut ctx);
        let n = demo
            .doc
            .nodes
            .iter()
            .find(|n| n.id == demo.context.text_node)
            .unwrap();
        assert_eq!(demo.context.observe(n).condition, GraphCondition::True);
        assert!(demo.editor.text_focus().is_none());
    }
    #[test]
    fn thegraph_focused_long_text_renders_at_zoom_without_mutation() {
        let mut demo = Demo::new();
        let node = demo
            .doc
            .nodes
            .iter()
            .find(|n| n.id == demo.context.text_node)
            .unwrap();
        let r = node.row_rect(0);
        let p = demo
            .editor
            .viewport
            .to_screen([r.origin[0] + 10., r.origin[1] + 10.]);
        demo.editor.pointer_down(&mut demo.doc, p, &DemoControls);
        demo.editor
            .pointer_up(&mut demo.doc, p, &AllowGraphConnections);
        demo.editor.text_input(
            &mut demo.doc,
            GraphTextInput::Insert("garden courtyard é👩‍🚀".repeat(20)),
        );
        let original = demo.doc.clone();
        demo.editor.viewport.zoom_at(p, 1.7);
        let mut pixels = vec![0; 1280 * 940 * 4];
        demo.render(&mut pixels, 1280, 940, 1.);
        assert_eq!(demo.doc, original);
    }
}

#[cfg(test)]
mod definition_tests {
    use super::*;
    fn source(demo: &Demo) -> GraphId {
        demo.doc
            .nodes
            .iter()
            .find(|n| n.definition.as_deref() == Some("event"))
            .unwrap()
            .id
    }
    fn select_event(demo: &mut Demo, event: &str) {
        let id = source(demo);
        demo.open_picker("Event", vec![], PickerAction::Event(id));
        demo.choose(event);
    }
    #[test]
    fn thegraph_generic_event_change_preserves_invalid_binding_and_undo_restores_it() {
        let mut demo = Demo::new();
        let original = demo.doc.clone();
        select_event(&mut demo, "damaged");
        let node = demo
            .doc
            .nodes
            .iter()
            .find(|n| n.definition.as_deref() == Some("text_equals"))
            .unwrap();
        assert_eq!(node.rows[1].binding.as_ref().unwrap().event, "arrived");
        assert!(
            demo.definitions
                .validate_binding(&demo.doc, node, &node.rows[1])
                .is_err()
        );
        let ctx = DefinitionPreview {
            base: &demo.context,
            definitions: &demo.definitions,
            doc: &demo.doc,
            samples: &demo.samples,
        };
        assert!(ctx.diagnostic(node).is_some());
        assert_eq!(ctx.observe(node).execution, GraphExecution::Idle);
        let options = demo
            .definitions
            .binding_options(&demo.doc, node, &node.rows[1]);
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].1.field, "kind");
        demo.undo();
        assert_eq!(demo.doc, original);
        let node = &demo.doc.nodes[4];
        assert_eq!(
            demo.definitions
                .resolve(&demo.doc, node, &node.rows[1], &demo.samples)
                .unwrap(),
            Some(GraphDatum::Text("garden".into()))
        );
    }
    #[test]
    fn thegraph_custom_events_and_literal_bindings_share_the_same_parameter() {
        let mut demo = Demo::new();
        select_event(&mut demo, "delivery");
        let node = demo.doc.nodes[4].clone();
        let row = node.rows[1].id;
        let options = demo
            .definitions
            .binding_options(&demo.doc, &node, &node.rows[1]);
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].1.field, "location");
        let binding = options[0].1.clone();
        demo.open_picker(
            "Bind",
            vec![],
            PickerAction::Binding(node.id, row, vec![binding.clone()]),
        );
        demo.choose(&serde_json::to_string(&binding).unwrap());
        let n = &demo.doc.nodes[4];
        assert_eq!(
            demo.definitions
                .resolve(&demo.doc, n, &n.rows[1], &demo.samples)
                .unwrap(),
            Some(GraphDatum::Text("garden".into()))
        );
        demo.open_picker("Bind", vec![], PickerAction::Binding(node.id, row, vec![]));
        demo.choose("literal");
        assert!(demo.doc.nodes[4].rows[1].binding.is_none());
        demo.doc.nodes[4].rows[1].value = GraphControlValue::Text("office".into());
        let n = &demo.doc.nodes[4];
        assert_eq!(
            demo.definitions
                .resolve(&demo.doc, n, &n.rows[1], &demo.samples)
                .unwrap(),
            Some(GraphDatum::Text("office".into()))
        );
    }
    #[test]
    fn thegraph_disconnected_event_binding_is_rejected_and_missing_sample_is_unknown() {
        let mut demo = Demo::new();
        let node = &demo.doc.nodes[4];
        assert_eq!(
            demo.definitions
                .resolve(&demo.doc, node, &node.rows[1], &BTreeMap::new())
                .unwrap(),
            None
        );
        demo.doc.connections.retain(|c| c.to != node.ports[0].id);
        let node = &demo.doc.nodes[4];
        assert!(
            demo.definitions
                .validate_binding(&demo.doc, node, &node.rows[1])
                .is_err()
        );
    }
    #[test]
    fn thegraph_node_picker_search_adds_definition_instances_with_undo() {
        let mut demo = Demo::new();
        let original = demo.doc.clone();
        let mut context = TheContext::new(1280, 940, 1.);
        demo.touch_down(1170., 60., &mut context);
        demo.touch_up(1170., 60., &mut context);
        assert!(demo.popup.is_some());
        for ch in "say".chars() {
            demo.key_down(Some(ch), None, &mut context);
        }
        demo.key_down(None, Some(TheKeyCode::Return), &mut context);
        assert_eq!(demo.doc.nodes.len(), original.nodes.len() + 1);
        assert_eq!(
            demo.doc.nodes.last().unwrap().definition.as_deref(),
            Some("say")
        );
        demo.undo();
        assert_eq!(demo.doc, original);
        demo.redo();
        assert_eq!(demo.doc.nodes.len(), original.nodes.len() + 1);
    }
    #[test]
    fn thegraph_definition_instances_and_serialized_bindings_keep_stable_keys() {
        let demo = Demo::new();
        let a = demo.definitions.instantiate("event", [0., 0.]).unwrap();
        let b = demo.definitions.instantiate("event", [0., 0.]).unwrap();
        assert_ne!(a.id, b.id);
        assert_ne!(a.rows[0].id, b.rows[0].id);
        assert_ne!(a.ports[0].id, b.ports[0].id);
        assert_eq!(a.rows[0].key, b.rows[0].key);
        assert_eq!(a.ports[0].key, b.ports[0].key);
        let copy: GraphDocument =
            serde_json::from_str(&serde_json::to_string(&demo.doc).unwrap()).unwrap();
        assert_eq!(copy, demo.doc);
    }
    #[test]
    fn thegraph_changed_field_type_and_bad_payload_are_rejected() {
        let mut demo = Demo::new();
        let id = source(&demo);
        demo.samples
            .get_mut(&id)
            .unwrap()
            .fields
            .insert("destination".into(), GraphDatum::Number(3.));
        let n = &demo.doc.nodes[4];
        assert!(
            demo.definitions
                .resolve(&demo.doc, n, &n.rows[1], &demo.samples)
                .is_err()
        );
        let mut definitions = GraphDefinitions::default();
        for n in demo.definitions.nodes() {
            definitions.register_node(n.clone()).unwrap();
        }
        definitions
            .register_event(GraphEventDefinition {
                id: "arrived".into(),
                label: "Renamed event".into(),
                fields: vec![GraphEventField {
                    id: "destination".into(),
                    label: "Renamed field".into(),
                    value_type: GraphValueType::Number,
                }],
            })
            .unwrap();
        assert!(
            definitions
                .validate_binding(&demo.doc, n, &n.rows[1])
                .is_err()
        );
    }
}
