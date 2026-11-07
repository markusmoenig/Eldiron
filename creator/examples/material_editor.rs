//! Interactive native UI fixture; no MaterialZ evaluation or project data writes.
//! cargo run -p eldiron-creator --example material_editor
//! Append -- --snapshot /tmp/material-editor.png --density 2 for a headless preview.
use eldiron_creator::material_editor::*;
use theframework::prelude::*;

fn sample() -> MaterialEditorView {
    let operation = |id: &str, label: &str| {
        MaterialOperationView {
        id: id.into(),
        label: label.into(),
        thumbnail: String::new(),
        palette: None,
        properties: vec![
            MaterialPropertyView{id:"scale".into(),label:"Scale".into(),group:"Scale".into(),kind:MaterialPropertyKind::Number,components:vec!["scale".into()],choices:vec![],tooltip:"Pattern scale".into()},
            MaterialPropertyView{id:"stretch".into(),label:"Stretch X / Y".into(),group:"Scale".into(),kind:MaterialPropertyKind::Vector,components:vec!["stretch.0".into(),"stretch.1".into()],choices:vec![],tooltip:String::new()},
            MaterialPropertyView{id:"dimensions".into(),label:"Dimensions".into(),group:"Coordinates".into(),kind:MaterialPropertyKind::Choice,components:vec!["dimensions".into()],choices:vec!["2D".into(),"3D".into()],tooltip:"Noise coordinate dimensions".into()},
            MaterialPropertyView{id:"normalize".into(),label:"Normalize".into(),group:"Coordinates".into(),kind:MaterialPropertyKind::Boolean,components:vec!["normalize".into()],choices:vec![],tooltip:String::new()},
        ],
        combine: None,
        source: Some(MaterialSourceCodeView{code:"// Perlin noise\nfloat evaluate(float2 uv) {\n    return perlin(uv * scale);\n}".into(),help:"Continuous Perlin noise with adjustable scale and stretch.\n\nChoose 2D for a surface or 3D for a volume slice.".into(),references:"Ken Perlin · Improved Noise".into()}),
        parameters: [ ("scale","Scale",0.55,0.,2.),("stretch.0","X",1.,0.01,10.),("stretch.1","Y",2.5,0.01,10.),("dimensions","Dimensions",0.,0.,1.),("normalize","Normalize",1.,0.,1.) ].into_iter().map(|(id,label,value,min,max)|MaterialNumberView{id:id.into(),label:label.into(),value,min,max}).collect(),
    }
    };
    MaterialEditorView {
        geometry: false,
        row_types: vec![],
        animation: Default::default(),
        title: "Material Study".into(),
        image_sources: vec![],
        image_bindings: vec![],
        palette: vec![],
        palette_bindings: vec![],
        tiling: MaterialTilingView::default(),
        defaults: vec![],
        rows: vec![
            MaterialRowView {
                id: "grain".into(),
                label: "Grain".into(),
                inputs: vec![],
                operations: vec![
                    operation("noise", "Perlin Noise"),
                    operation("multiply", "Multiply"),
                ],
            },
            MaterialRowView {
                id: "color".into(),
                label: "Base Color".into(),
                inputs: vec![MaterialInputView {
                    target: MaterialInputTarget::Image {
                        operation: "mix".into(),
                    },
                    label: "Image · Mix".into(),
                    binding: MaterialInputBinding::Row(MaterialReferenceView {
                        row: "grain".into(),
                        components: "xyz".into(),
                        mapping: None,
                    }),
                    components: "xyz".into(),
                }],
                operations: vec![operation("mix", "Mix")],
            },
            MaterialRowView {
                id: "rough".into(),
                label: "Roughness".into(),
                inputs: vec![],
                operations: vec![operation("rough-noise", "Perlin Noise")],
            },
        ],
        outputs: [
            ("base color", "Color", Some("color"), ""),
            ("roughness", "Roughness", Some("rough"), ""),
            ("metallic", "Metallic", Some("grain"), ""),
            ("opacity", "Opacity", None, "1.0"),
            ("emission", "Emission", None, "Black"),
        ]
        .into_iter()
        .map(|(id, label, row, fallback)| MaterialOutputView {
            id: id.into(),
            label: label.into(),
            row: row.map(str::to_owned),
            fallback: fallback.into(),
        })
        .collect(),
        sources: [
            ("perlin", "Perlin Noise", "Noise"),
            ("simplex", "Simplex Noise", "Noise"),
            ("mix", "Mix", "Color"),
            ("multiply", "Multiply", "Math"),
            ("transform", "Transform", "Coordinates"),
            ("disc", "Disc", "Shapes"),
        ]
        .into_iter()
        .map(|(id, label, category)| MaterialSourceView {
            id: id.into(),
            label: label.into(),
            category: category.into(),
        })
        .collect(),
    }
}
struct Demo {
    widget: MaterialEditorWidget,
    view: MaterialEditorView,
    pointer: [f32; 2],
    next_id: usize,
}
impl Demo {
    fn apply(&mut self) {
        let mut changed = false;
        for command in self.widget.take_commands() {
            match command {
                MaterialEditorCommand::AddSource { source, row, after } => {
                    let Some(source) = self.view.sources.iter().find(|s| s.id == source) else {
                        continue;
                    };
                    self.next_id += 1;
                    let op = MaterialOperationView {
                        id: format!("example-{}", self.next_id),
                        label: source.label.clone(),
                        thumbnail: String::new(),
                        palette: None,
                        properties: vec![],
                        combine: None,
                        source: None,
                        parameters: Vec::new(),
                    };
                    if let Some(id) = row {
                        if let Some(row) = self.view.rows.iter_mut().find(|r| r.id == id) {
                            let index = after
                                .as_ref()
                                .and_then(|id| row.operations.iter().position(|op| &op.id == id))
                                .map_or(0, |i| i + 1);
                            row.operations.insert(index, op);
                        }
                    } else {
                        self.view.rows.push(MaterialRowView {
                            id: format!("row-{}", self.next_id),
                            label: "New Row".into(),
                            inputs: vec![],
                            operations: vec![op],
                        });
                    }
                    changed = true;
                }
                MaterialEditorCommand::MoveOperation {
                    operation,
                    row,
                    after,
                } => {
                    let operation = self.view.rows.iter_mut().find_map(|source| {
                        source
                            .operations
                            .iter()
                            .position(|op| op.id == operation)
                            .map(|i| source.operations.remove(i))
                    });
                    if let Some(operation) = operation {
                        if let Some(target) = self.view.rows.iter_mut().find(|r| r.id == row) {
                            let index = after
                                .as_ref()
                                .and_then(|id| target.operations.iter().position(|op| &op.id == id))
                                .map_or(0, |i| i + 1);
                            target.operations.insert(index, operation);
                            changed = true;
                        }
                    }
                }
                MaterialEditorCommand::ConnectOutput { output, row } => {
                    if let Some(output) = self.view.outputs.iter_mut().find(|o| o.id == output) {
                        output.row = Some(row);
                        changed = true;
                    }
                }
                MaterialEditorCommand::DisconnectOutput(id) => {
                    if let Some(output) = self.view.outputs.iter_mut().find(|o| o.id == id) {
                        output.row = None;
                        changed = true;
                    }
                }
                MaterialEditorCommand::SetNumber {
                    operation,
                    parameter,
                    value,
                } => {
                    if let Some(op) = self
                        .view
                        .rows
                        .iter_mut()
                        .flat_map(|r| &mut r.operations)
                        .find(|o| o.id == operation)
                    {
                        if let Some(p) = op.parameters.iter_mut().find(|p| p.id == parameter) {
                            p.value = value;
                            changed = true;
                        }
                    }
                }
                MaterialEditorCommand::SetAnimation { frames, duration } => {
                    self.view.animation.frames = frames;
                    self.view.animation.duration = duration;
                    changed = true;
                }
                MaterialEditorCommand::SetTiling {
                    mode,
                    period,
                    blend,
                } => {
                    self.view.tiling = MaterialTilingView {
                        mode,
                        period,
                        blend,
                    };
                    changed = true;
                }
                MaterialEditorCommand::SetCombine { operation, mode } => {
                    if let Some(op) = self
                        .view
                        .rows
                        .iter_mut()
                        .flat_map(|row| &mut row.operations)
                        .find(|op| op.id == operation)
                    {
                        op.combine = Some(mode);
                        changed = true;
                    }
                }
                MaterialEditorCommand::SetSource { operation, code } => {
                    if let Some(source) = self
                        .view
                        .rows
                        .iter_mut()
                        .flat_map(|row| &mut row.operations)
                        .find(|op| op.id == operation)
                        .and_then(|op| op.source.as_mut())
                    {
                        source.code = code;
                        changed = true;
                    }
                }
                MaterialEditorCommand::SetInput { target, binding } => {
                    for row in &mut self.view.rows {
                        if let Some(input) =
                            row.inputs.iter_mut().find(|input| input.target == target)
                        {
                            input.binding = binding.clone();
                            changed = true;
                        }
                    }
                }
                MaterialEditorCommand::RemoveOperation(id) => {
                    for row in &mut self.view.rows {
                        row.operations.retain(|op| op.id != id);
                    }
                    changed = true;
                }
                _ => {}
            }
        }
        if changed {
            self.widget.set_view(self.view.clone());
        }
    }
}
impl TheTrait for Demo {
    fn new() -> Self {
        let view = sample();
        let mut widget = MaterialEditorWidget::new(TheId::named("Material Editor"));
        widget.set_view(view.clone());
        Self {
            widget,
            view,
            pointer: [0., 0.],
            next_id: 0,
        }
    }
    fn default_window_size(&self) -> (usize, usize) {
        (480, 850)
    }
    fn min_window_size(&self) -> (usize, usize) {
        (320, 600)
    }
    fn window_title(&self) -> String {
        "Eldiron · Material Editor UI Prototype".into()
    }
    fn native_ui_rendering(&self) -> bool {
        true
    }
    fn draw(&mut self, pixels: &mut [u8], ctx: &mut TheContext) {
        self.widget
            .set_dim(TheDim::new(0, 0, ctx.width as i32, ctx.height as i32), ctx);
        let mut buffer = TheRGBABuffer::new(TheDim::new(0, 0, ctx.width as i32, ctx.height as i32));
        buffer.set_render_scale(ctx.ui_render_scale);
        self.widget.paint(
            &mut buffer,
            &material_editor_theme(&TheBlackBlueTheme::new()),
        );
        if buffer.pixels().len() == pixels.len() {
            pixels.copy_from_slice(buffer.pixels());
        }
    }
    fn touch_down(&mut self, x: f32, y: f32, _: &mut TheContext) -> bool {
        self.widget.press([x, y]);
        self.apply();
        true
    }
    fn touch_dragged(&mut self, x: f32, y: f32, _: &mut TheContext) -> bool {
        self.widget.drag([x, y]);
        true
    }
    fn touch_up(&mut self, x: f32, y: f32, _: &mut TheContext) -> bool {
        self.widget.release([x, y]);
        self.apply();
        true
    }
    fn hover(&mut self, x: f32, y: f32, _: &mut TheContext) -> bool {
        self.pointer = [x, y];
        false
    }
    fn mouse_wheel(&mut self, delta: (isize, isize), _: &mut TheContext) -> bool {
        self.widget
            .scroll(self.pointer, [-delta.0 as f32, -delta.1 as f32]);
        true
    }
    fn key_down(
        &mut self,
        ch: Option<char>,
        key: Option<TheKeyCode>,
        ctx: &mut TheContext,
    ) -> bool {
        if let Some(ch) = ch {
            self.widget
                .on_event(&TheEvent::KeyDown(TheValue::Char(ch)), ctx);
        }
        if let Some(key) = key {
            self.widget
                .on_event(&TheEvent::KeyCodeDown(TheValue::KeyCode(key)), ctx);
        }
        self.apply();
        true
    }
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if let Some(index) = args.iter().position(|a| a == "--snapshot") {
        let option = |key: &str, default: f32| {
            args.iter()
                .position(|a| a == key)
                .and_then(|i| args.get(i + 1))
                .and_then(|v| v.parse().ok())
                .unwrap_or(default)
        };
        let (width, height) = (
            option("--width", 380.) as i32,
            option("--height", 800.) as i32,
        );
        let density = option("--density", 2.).clamp(1., 3.);
        let mut ctx = TheContext::new(width as usize, height as usize, 1.);
        let mut demo = Demo::new();
        demo.widget
            .set_dim(TheDim::new(0, 0, width, height), &mut ctx);
        demo.widget.press([145., 120.]);
        demo.widget.release([145., 120.]);
        if args.iter().any(|arg| arg == "--color") {
            let op = &mut demo.view.rows[0].operations[0];
            for (i, value) in [0.1, 0.4, 0.7, 0.5].into_iter().enumerate() {
                op.parameters.push(MaterialNumberView {
                    id: format!("tint.{i}"),
                    label: ["R", "G", "B", "A"][i].into(),
                    value,
                    min: 0.,
                    max: 1.,
                });
            }
            op.properties.push(MaterialPropertyView {
                id: "tint".into(),
                label: "Tint RGBA".into(),
                group: "Color".into(),
                kind: MaterialPropertyKind::Color,
                components: (0..4).map(|i| format!("tint.{i}")).collect(),
                choices: vec![],
                tooltip: "Material tint".into(),
            });
            demo.widget.set_view(demo.view.clone());
        }
        if args.iter().any(|arg| arg == "--row-input") {
            demo.widget.press([91., 114.]);
        }
        let editor_bottom = (height as f32 - 314.).max(216.).min(height as f32 * 0.6);
        if args
            .iter()
            .any(|arg| arg == "--typed" || arg == "--source" || arg == "--color")
        {
            demo.widget.press([120., 24.]);
            demo.widget.release([120., 24.]);
        }
        if args.iter().any(|arg| arg == "--source") {
            demo.widget
                .press([10. + 3.5 * (width as f32 - 20.) / 4., editor_bottom + 12.]);
        }
        if args.iter().any(|arg| arg == "--tiling") {
            demo.view.tiling.mode = "seamless".into();
            demo.widget.set_view(demo.view.clone());
            demo.widget.press([20., editor_bottom + 12.]);
            demo.widget.press([20., editor_bottom + 34.]);
        }
        if args
            .iter()
            .any(|arg| arg == "--sources" || arg == "--category")
        {
            demo.widget
                .press([10. + 1.5 * (width as f32 - 20.) / 4., editor_bottom + 12.]);
        }
        if args.iter().any(|arg| arg == "--category") {
            demo.widget.press([width as f32 - 40., editor_bottom + 38.]);
        }
        if args.iter().any(|arg| arg == "--color") {
            demo.widget
                .press([20., editor_bottom + 52. + 7. * 24. + 10.]);
        }
        demo.widget.take_commands();
        let mut buffer = TheRGBABuffer::new(TheDim::new(0, 0, width, height));
        buffer.set_render_scale(density);
        ctx.ui_render_scale = density;
        let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::new());
        style.set_theme(if args.iter().any(|arg| arg == "--brown") {
            Box::new(TheBrownTheme::new())
        } else if args.iter().any(|arg| arg == "--light") {
            Box::new(TheLightTheme::new())
        } else {
            Box::new(TheBlackBlueTheme::new())
        });
        demo.widget.draw(&mut buffer, &mut style, &mut ctx);
        std::fs::write(&args[index + 1], buffer.to_png().unwrap()).unwrap();
    } else {
        TheApp::new().run(Box::new(Demo::new()));
    }
}
