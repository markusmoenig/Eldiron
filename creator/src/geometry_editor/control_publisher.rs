//! Visual authoring for the existing portable Group controls. Custom mappings
//! survive metadata edits; no generated mesh replaces the source construction.
use super::*;
use denrim_materials::geometry::group::{Control, Modifier};
use denrim_materials::geometry::parameters::{self, Parameter};

const TITLE: &str = "Publish mesh controls";
const LIST: &str = "MZ Published Controls";
const FORM: &str = "MZ Control Form";
const VIEW: &str = "MZ Control Preview";
const SAVE: &str = "MZ Control Save";
const KINDS: &[&str] = &[
    "Width (m)",
    "Height (m)",
    "Depth (m)",
    "Scale X",
    "Scale Y",
    "Scale Z",
    "Offset X (m)",
    "Offset Y (m)",
    "Offset Z (m)",
    "Palette color",
    "Recipe parameter",
    "Input / calculated value",
    "Custom mapping (preserved)",
];

pub(super) struct Publisher {
    asset: Uuid,
    path: Vec<String>,
    edit_path: Vec<String>,
    before: String,
    document: Document,
    operation: String,
    controls: Vec<Control>,
    selected: Option<usize>,
    parts: Vec<(String, String)>,
    bounds: ([f32; 3], [f32; 3]),
    targets: Vec<(String, Vec<String>)>,
    parameters: Vec<Parameter>,
    binding_index: usize,
    parameter_query: String,
    picker_reference: bool,
    error: Option<String>,
}

fn kind(c: &Control) -> usize {
    if c.modifiers.is_empty() {
        return 11;
    }
    if !c.modifiers.is_empty()
        && c.modifiers
            .iter()
            .all(|m| matches!(m, Modifier::Parameter { .. }))
    {
        return 10;
    }
    if c.modifiers.len() != 1 {
        return 12;
    }
    match &c.modifiers[0] {
        Modifier::FitAxis { axis } => *axis,
        Modifier::Palette { .. } => 9,
        Modifier::Affine {
            scale,
            translation,
            pivot,
            joints,
            weight,
            frames,
            ..
        } if *pivot == [0.; 3] && joints.is_empty() && weight.is_none() && !frames => {
            for axis in 0..3 {
                let mut unit = [0.; 3];
                unit[axis] = 1.;
                if *scale == unit && *translation == [0.; 3] {
                    return 3 + axis;
                }
                if *translation == unit && *scale == [0.; 3] {
                    return 6 + axis;
                }
            }
            12
        }
        _ => 12,
    }
}
fn targets(c: &Control) -> Option<&Vec<String>> {
    match c.modifiers.first()? {
        Modifier::Palette { parts } | Modifier::Affine { parts, .. } => Some(parts),
        _ => None,
    }
}
fn defaults(mesh: &denrim_materials::geometry::Mesh) -> Vec<Control> {
    let (lo, hi) = mesh.bounds();
    let mut result = Vec::new();
    for axis in 0..3 {
        let value = hi[axis] - lo[axis];
        if value < 0.001 {
            continue;
        }
        result.push(Control {
            expression: None,
            id: Uuid::new_v4().to_string(),
            label: KINDS[axis].into(),
            category: "Dimensions".into(),
            help: "Overall size of this group.".into(),
            order: axis as u16,
            value,
            reference: value,
            min: 0.001,
            max: 100.,
            modifiers: vec![Modifier::FitAxis { axis }],
        });
    }
    let mut colors = std::collections::BTreeMap::<u16, Vec<String>>::new();
    for part in &mesh.parts {
        if let Some(s) = &part.surface {
            colors.entry(s.palette).or_default().push(part.id.clone());
        }
    }
    for (i, (palette, parts)) in colors.into_iter().enumerate() {
        result.push(Control {
            expression: None,
            id: Uuid::new_v4().to_string(),
            label: format!("Color {}", i + 1),
            category: "Appearance".into(),
            help: "Project palette color for these parts.".into(),
            order: i as u16,
            value: palette as f32,
            reference: palette as f32,
            min: 0.,
            max: 255.,
            modifiers: vec![Modifier::Palette { parts }],
        });
    }
    result
}
impl Publisher {
    fn new(editor: &GeometryEditor, project: &Project) -> Result<Self, String> {
        let asset = editor.selected.ok_or("Select a mesh recipe")?;
        let before = project
            .procedural_appearances
            .get(&asset)
            .ok_or("The mesh was removed")?
            .clone();
        let root = Document::parse(&before).map_err(|e| e.to_string())?;
        let mut document = root
            .group_document(&editor.group_path)
            .map_err(|e| e.to_string())?
            .clone();
        let operation = editor
            .selected_operation
            .as_ref()
            .and_then(|id| {
                document
                    .rows
                    .iter()
                    .flat_map(|r| &r.tiles)
                    .find(|t| t.id == *id && matches!(t.op, Operation::Group { .. }))
            })
            .map(|t| t.id.clone());
        let wrapping = operation.is_none() && editor.group_path.is_empty();
        let mut edit_path = editor.group_path.clone();
        let operation = if operation.is_none() && !edit_path.is_empty() {
            let id = edit_path.pop().unwrap();
            document = root
                .group_document(&edit_path)
                .map_err(|e| e.to_string())?
                .clone();
            id
        } else if let Some(id) = operation {
            id
        } else {
            let id = Uuid::new_v4().to_string();
            let construction = document.clone();
            let construction_grip = construction.grip.is_some();
            let row = Uuid::new_v4().to_string();
            document.rows = vec![Row {
                id: row.clone(),
                label: document.label.clone(),
                tiles: vec![Tile {
                    id: id.clone(),
                    op: Operation::Group {
                        label: document.label.clone(),
                        document: Box::new(construction),
                        controls: vec![],
                    },
                }],
            }];
            document.output = row.clone();
            document.grip = construction_grip.then_some(row);
            id
        };
        let (construction, existing) = document
            .rows
            .iter()
            .flat_map(|r| &r.tiles)
            .find_map(|t| {
                if t.id != operation {
                    return None;
                }
                if let Operation::Group {
                    document, controls, ..
                } = &t.op
                {
                    Some((document.as_ref(), controls.clone()))
                } else {
                    None
                }
            })
            .ok_or("Select a Group")?;
        let mesh = shared::appearance_assets::resolve_dependencies(project, construction)?
            .evaluate()
            .map_err(|e| e.to_string())?
            .mesh;
        if mesh.parts.is_empty() {
            return Err("Add mesh geometry before publishing controls".into());
        }
        let controls = if wrapping { defaults(&mesh) } else { existing };
        let parts = mesh
            .parts
            .iter()
            .map(|p| {
                (
                    p.id.clone(),
                    construction
                        .rows
                        .iter()
                        .find(|r| r.id == p.id)
                        .map(|r| r.label.clone())
                        .unwrap_or_else(|| {
                            if p.label.is_empty() {
                                p.id.clone()
                            } else {
                                p.label.clone()
                            }
                        }),
                )
            })
            .collect();
        let parameters = parameters::catalog(construction);
        Ok(Self {
            asset,
            path: editor.group_path.clone(),
            edit_path,
            before,
            document,
            operation,
            selected: (!controls.is_empty()).then_some(0),
            controls,
            parts,
            bounds: mesh.bounds(),
            targets: vec![],
            parameters,
            binding_index: 0,
            parameter_query: String::new(),
            picker_reference: false,
            error: None,
        })
    }
    fn candidate(&self) -> Result<Document, String> {
        let mut doc = self.document.clone();
        let tile = doc
            .rows
            .iter_mut()
            .flat_map(|r| &mut r.tiles)
            .find(|t| t.id == self.operation)
            .ok_or("Group was removed")?;
        let Operation::Group { controls, .. } = &mut tile.op else {
            return Err("Select a group".into());
        };
        if self
            .controls
            .iter()
            .any(|c| c.label.trim().is_empty() || c.category.trim().is_empty())
        {
            return Err("Each control needs a label and section".into());
        }
        *controls = self.controls.clone();
        Ok(doc)
    }
    fn list(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(list) = ui.get_list_layout(LIST) {
            list.clear();
            for (i, c) in self.controls.iter().enumerate() {
                let mut item = TheListItem::new(TheId::named(&format!("MZ Published Control {i}")));
                item.set_text(c.label.clone());
                item.set_sub_text(c.category.clone());
                list.add_item(item, ctx);
            }
            if let Some(i) = self.selected {
                list.select_item_at(i as i32, ctx, false);
            }
        }
    }
    fn form(&mut self, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        self.targets = vec![("Entire group".into(), vec!["*".into()])];
        self.targets.extend(
            self.parts
                .iter()
                .map(|(id, name)| (name.clone(), vec![id.clone()])),
        );
        let Some(c) = self.selected.and_then(|i| self.controls.get(i)) else {
            if let Some(form) = ui.get_text_layout(FORM) {
                form.clear();
            }
            ctx.ui.relayout = true;
            return;
        };
        let binding = kind(c);
        let target = if let Some(parts) = targets(c) {
            self.targets
                .iter()
                .position(|(_, p)| p == parts)
                .unwrap_or_else(|| {
                    self.targets.push((
                        format!("Existing selection ({} parts)", parts.len()),
                        parts.clone(),
                    ));
                    self.targets.len() - 1
                })
        } else {
            0
        };
        if let Some(form) = ui.get_text_layout(FORM) {
            form.clear();
            for (key, label, value) in [
                ("Label", "Label", &c.label),
                ("Category", "Section", &c.category),
                ("Help", "Help text", &c.help),
            ] {
                let mut edit = TheTextLineEdit::new(TheId::named(&format!("MZ Control {key}")));
                edit.limiter_mut().set_max_width(400);
                edit.set_value(TheValue::Text(value.clone()));
                form.add_pair(label.into(), Box::new(edit));
            }
            let mut menu = TheDropdownMenu::new(TheId::named("MZ Control Kind"));
            menu.limiter_mut().set_max_width(380);
            for kind in KINDS.iter().take(if binding == 12 { 13 } else { 12 }) {
                menu.add_option((*kind).into());
            }
            menu.set_selected_index(binding as i32);
            form.add_pair("Behavior".into(), Box::new(menu));
            if binding == 10 {
                self.binding_index = self.binding_index.min(c.modifiers.len().saturating_sub(1));
                let mut menu = TheDropdownMenu::new(TheId::named("MZ Control Binding"));
                menu.limiter_mut().set_max_width(380);
                for i in 0..c.modifiers.len() {
                    menu.add_option(format!("Binding {}", i + 1));
                }
                if c.modifiers.len() < 64 {
                    menu.add_option("+ Add binding…".into());
                }
                menu.set_selected_index(self.binding_index as i32);
                form.add_pair("Binding".into(), Box::new(menu));
                if let Some(Modifier::Parameter {
                    target,
                    factor,
                    offset,
                }) = c.modifiers.get(self.binding_index)
                {
                    let label = self
                        .parameters
                        .iter()
                        .find(|p| p.target == *target)
                        .map(|p| p.label.clone())
                        .unwrap_or_else(|| {
                            format!("Missing: {} / {}", target.operation, target.parameter)
                        });
                    let mut choose =
                        TheTraybarButton::new(TheId::named("MZ Control Choose Parameter"));
                    choose.set_text(format!("{label}…"));
                    choose.set_fixed_size(true);
                    choose.limiter_mut().set_max_size(Vec2::new(380, 20));
                    choose.set_status_text(
                        "Choose a construction input; search by row, operation or field.",
                    );
                    form.add_pair("Recipe input".into(), Box::new(choose));
                    for (key, label, value) in [
                        ("Factor", "Multiply by", *factor),
                        ("Offset", "Then add", *offset),
                    ] {
                        let mut edit =
                            TheTextLineEdit::new(TheId::named(&format!("MZ Control {key}")));
                        edit.set_value(TheValue::Float(value));
                        edit.set_status_text("Input = published value × multiplier + offset; applied before construction.");
                        form.add_pair(label.into(), Box::new(edit));
                    }
                }
                let mut remove = TheTraybarButton::new(TheId::named("MZ Control Remove Binding"));
                remove.set_text("Remove binding".into());
                remove.set_disabled(c.modifiers.len() <= 1);
                form.add_pair(String::new(), Box::new(remove));
            } else {
                let mut menu = TheDropdownMenu::new(TheId::named("MZ Control Target"));
                menu.limiter_mut().set_max_width(380);
                for (name, _) in &self.targets {
                    menu.add_option(name.clone());
                }
                menu.set_selected_index(target as i32);
                menu.set_disabled(binding < 3 || binding >= 11);
                form.add_pair("Applies to".into(), Box::new(menu));
            }
            let mut expression = TheTextLineEdit::new(TheId::named("MZ Control Expression"));
            expression.limiter_mut().set_max_width(400);
            expression.set_value(TheValue::Text(c.expression.clone().unwrap_or_default()));
            expression.set_status_text("Leave blank for an editable input. Use + - * /, parentheses, sin_deg, cos_deg, tan_deg, sqrt, abs, min, max, clamp, floor, ceil, round. References use [control-id].");
            form.add_pair("Formula".into(), Box::new(expression));
            let mut insert = TheTraybarButton::new(TheId::named("MZ Control Insert Reference"));
            insert.set_text("Insert control reference…".into());
            form.add_pair(String::new(), Box::new(insert));
            for (key, label, value) in [
                ("Value", "Current value", c.value),
                ("Default", "Neutral / default", c.reference),
                ("Minimum", "Minimum", c.min),
                ("Maximum", "Maximum", c.max),
            ] {
                if binding == 9 && matches!(key, "Value" | "Default") {
                    let mut picker =
                        ThePaletteIndexPicker::new(TheId::named(&format!("MZ Control {key}")));
                    picker.set_palette(project.palette.clone());
                    picker.set_selected_index(value as i32);
                    form.add_pair(label.into(), Box::new(picker));
                } else {
                    let mut edit = TheTextLineEdit::new(TheId::named(&format!("MZ Control {key}")));
                    edit.set_value(if c.is_integer() && matches!(key, "Value" | "Default") {
                        TheValue::Int(value as i32)
                    } else {
                        TheValue::Float(value)
                    });
                    edit.set_status_text(if key == "Default" {
                        "Reset value; also the neutral point of scale/offset mappings."
                    } else {
                        "Enter a finite number"
                    });
                    edit.set_disabled(
                        (binding == 9 && matches!(key, "Minimum" | "Maximum"))
                            || (c.expression.is_some() && key == "Value"),
                    );
                    form.add_pair(label.into(), Box::new(edit));
                }
            }
        }
        ctx.ui.relayout = true;
    }
    fn parameter_list(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(list) = ui.get_list_layout("MZ Parameter Choices") {
            list.clear();
            if self.picker_reference {
                for (i, c) in self.controls.iter().enumerate().filter(|(i, c)| {
                    Some(*i) != self.selected
                        && format!("{} {}", c.label, c.category)
                            .to_lowercase()
                            .contains(&self.parameter_query.to_lowercase())
                }) {
                    let mut item =
                        TheListItem::new(TheId::named(&format!("MZ Reference Choice {i}")));
                    item.set_text(c.label.clone());
                    item.set_sub_text(format!("{} / [{}]", c.category, c.id));
                    list.add_item(item, ctx);
                }
            } else {
                for (i, p) in self.parameters.iter().enumerate().filter(|(_, p)| {
                    p.label
                        .to_lowercase()
                        .contains(&self.parameter_query.to_lowercase())
                }) {
                    let mut item =
                        TheListItem::new(TheId::named(&format!("MZ Parameter Choice {i}")));
                    let (path, label) =
                        p.label.rsplit_once(" / ").unwrap_or(("", p.label.as_str()));
                    item.set_text(label.into());
                    item.set_sub_text(path.into());
                    list.add_item(item, ctx);
                }
            }
        }
        ctx.ui.relayout = true;
    }
    fn open_parameter_picker(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        let Some(anchor) = ui
            .get_widget(if self.picker_reference {
                "MZ Control Insert Reference"
            } else {
                "MZ Control Choose Parameter"
            })
            .map(|w| *w.dim())
        else {
            return;
        };
        self.parameter_query.clear();
        let mut canvas = TheCanvas::new();
        canvas.limiter_mut().set_max_size(Vec2::new(500, 340));
        let mut search = TheTextLineEdit::new(TheId::named("MZ Parameter Search"));
        search.limiter_mut().set_max_width(450);
        search.set_status_text(if self.picker_reference {
            "Search controls"
        } else {
            "Search recipe inputs"
        });
        let mut bar = TheHLayout::new(TheId::empty());
        bar.limiter_mut().set_max_height(36);
        bar.set_margin(Vec4::new(8, 6, 8, 6));
        let mut label = TheText::new(TheId::empty());
        label.set_text("Find".into());
        bar.add_widget(Box::new(label));
        bar.add_widget(Box::new(search));
        let mut top = TheCanvas::new();
        top.set_layout(bar);
        canvas.set_top(top);
        let mut list = TheListLayout::new(TheId::named("MZ Parameter Choices"));
        list.set_item_size(44);
        canvas.set_layout(list);
        ui.show_popover(TheId::named("MZ Recipe Input Picker"), anchor, canvas, ctx);
        self.parameter_list(ui, ctx);
    }
    fn unused_parameter(&self) -> Option<usize> {
        self.parameters.iter().position(|p| {
            !self
                .controls
                .iter()
                .flat_map(|c| &c.modifiers)
                .any(|m| matches!(m,Modifier::Parameter { target,.. } if *target == p.target))
        })
    }
    fn choose_parameter(&mut self, index: usize, reset: bool) {
        let Some(p) = self.parameters.get(index) else {
            return;
        };
        let Some(c) = self.selected.and_then(|i| self.controls.get_mut(i)) else {
            return;
        };
        let mapping = Modifier::Parameter {
            target: p.target.clone(),
            factor: 1.,
            offset: 0.,
        };
        if reset {
            c.value = p.value;
            c.reference = p.value;
            c.min = p.min;
            c.max = p.max;
            c.modifiers = vec![mapping];
            self.binding_index = 0;
        } else if let Some(m) = c.modifiers.get_mut(self.binding_index) {
            *m = mapping;
        } else if c.modifiers.len() < 64 {
            c.modifiers.push(mapping);
        }
    }
    fn mapping(&mut self, binding: usize, target: usize) {
        if binding == 10 {
            if let Some(i) = self
                .unused_parameter()
                .or_else(|| (!self.parameters.is_empty()).then_some(0))
            {
                self.choose_parameter(i, true);
            }
            return;
        }

        let Some(c) = self.selected.and_then(|i| self.controls.get_mut(i)) else {
            return;
        };
        let parts = self
            .targets
            .get(target)
            .map(|(_, p)| p.clone())
            .unwrap_or_else(|| vec!["*".into()]);
        if binding == 11 {
            c.modifiers.clear();
            return;
        }
        if binding < 3 {
            let value = (self.bounds.1[binding] - self.bounds.0[binding]).max(0.001);
            c.value = value;
            c.reference = value;
            c.min = 0.001;
            c.max = 100.;
            c.modifiers = vec![Modifier::FitAxis { axis: binding }];
        } else if binding < 9 {
            let mut scale = [0.; 3];
            let mut translation = [0.; 3];
            if binding < 6 {
                scale[binding - 3] = 1.;
                c.value = 1.;
                c.reference = 1.;
                c.min = 0.1;
                c.max = 3.;
            } else {
                translation[binding - 6] = 1.;
                c.value = 0.;
                c.reference = 0.;
                c.min = -10.;
                c.max = 10.;
            }
            c.modifiers = vec![Modifier::Affine {
                parts,
                joints: vec![],
                pivot: [0.; 3],
                scale,
                translation,
                weight: None,
                frames: false,
            }];
        } else if binding == 9 {
            c.value = 7.;
            c.reference = 7.;
            c.min = 0.;
            c.max = 255.;
            c.modifiers = vec![Modifier::Palette { parts }];
        }
    }
    fn preview(&mut self, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        if let Some(c) = self.selected.and_then(|i| self.controls.get(i)) {
            if let Some(w) = ui.get_widget("MZ Control Value") {
                w.set_disabled(c.expression.is_some());
            }
            if c.expression.is_some() {
                if let Ok(resolved) =
                    denrim_materials::geometry::expressions::resolve(&self.controls)
                {
                    if let Some(w) = ui.get_widget("MZ Control Value") {
                        w.set_value(TheValue::Float(resolved[self.selected.unwrap()].value));
                    }
                }
            }
        }
        let evaluation = self
            .candidate()
            .and_then(|doc| shared::appearance_assets::resolve_dependencies(project, &doc))
            .and_then(|doc| doc.evaluate().map_err(|e| e.to_string()));
        self.error = evaluation.as_ref().err().cloned();
        if let Some(view) = ui.get_render_view(VIEW) {
            let dim = *view.dim();
            let (w, h) = (dim.width.max(1), dim.height.max(1));
            let buffer = view.render_buffer_mut();
            buffer.resize(w, h);
            buffer.fill([30, 32, 35, 255]);
            if let Ok(e) = evaluation {
                let image = presentation::thumbnail(
                    &Value::Mesh(e.mesh),
                    &project.palette,
                    &TheDarkTheme::new(),
                );
                let side = w.min(h).min(200);
                buffer.copy_into((w - side) / 2, (h - side) / 2, &image.scaled(side, side));
            }
        }
        if let Some(w) = ui.get_widget("MZ Control Error") {
            w.set_value(TheValue::Text(self.error.clone().unwrap_or_else(|| {
                "Changes are applied only when you save.".into()
            })));
        }
        if let Some(w) = ui.get_widget(SAVE) {
            w.set_disabled(self.error.is_some());
        }
        ctx.ui.redraw_all = true;
    }
    fn canvas(&self) -> TheCanvas {
        let mut canvas = TheCanvas::new();
        canvas.limiter_mut().set_max_size(Vec2::new(760, 640));
        let mut left = TheCanvas::new();
        left.limiter_mut().set_max_width(214);
        let mut list = TheListLayout::new(TheId::named(LIST));
        list.limiter_mut().set_max_width(214);
        list.set_item_size(46);
        left.set_layout(list);
        let mut buttons = TheHLayout::new(TheId::empty());
        buttons.limiter_mut().set_max_height(34);
        buttons.set_margin(Vec4::new(4, 4, 4, 4));
        buttons.set_padding(3);
        for (id, text) in [("Add", "+"), ("Remove", "−"), ("Up", "↑"), ("Down", "↓")] {
            let mut b = TheTraybarButton::new(TheId::named(&format!("MZ Control {id}")));
            b.set_text(text.into());
            b.set_status_text(&format!("{id} control"));
            buttons.add_widget(Box::new(b));
        }
        let mut footer = TheCanvas::new();
        footer.set_layout(buttons);
        left.set_bottom(footer);
        canvas.set_left(left);
        let mut right = TheCanvas::new();
        let mut form = TheTextLayout::new(TheId::named(FORM));
        form.set_fixed_text_width(122);
        form.set_padding(6);
        right.set_layout(form);
        let mut preview = TheCanvas::new();
        preview.limiter_mut().set_max_height(166);
        let mut view = TheRenderView::new(TheId::named(VIEW));
        view.limiter_mut().set_max_size(Vec2::new(i32::MAX, 166));
        preview.set_widget(view);
        right.set_top(preview);
        let mut bottom = TheVLayout::new(TheId::empty());
        bottom.limiter_mut().set_max_height(82);
        bottom.set_margin(Vec4::new(8, 4, 8, 4));
        let mut error = TheText::new(TheId::named("MZ Control Error"));
        error.set_text_size(11.);
        error.limiter_mut().set_max_height(28);
        bottom.add_widget(Box::new(error));
        let mut save = TheTraybarButton::new(TheId::named(SAVE));
        save.set_text("Save controls".into());
        bottom.add_widget(Box::new(save));
        let mut sentinel = TheText::new(TheId::named("Dialog Value"));
        sentinel.set_text("Editable MZ Group controls".into());
        sentinel.set_text_size(11.);
        sentinel.limiter_mut().set_max_height(18);
        bottom.add_widget(Box::new(sentinel));
        let mut footer = TheCanvas::new();
        footer.set_layout(bottom);
        right.set_bottom(footer);
        canvas.set_center(right);
        canvas
    }
    fn number(&mut self, key: &str, value: f32) {
        let Some(c) = self.selected.and_then(|i| self.controls.get_mut(i)) else {
            return;
        };
        if matches!(key, "Factor" | "Offset") {
            if let Some(Modifier::Parameter { factor, offset, .. }) =
                c.modifiers.get_mut(self.binding_index)
            {
                if key == "Factor" {
                    *factor = value;
                } else {
                    *offset = value;
                }
            }
            return;
        }
        let value = if c.is_integer() && matches!(key, "Value" | "Default") {
            value.round()
        } else {
            value
        };
        match key {
            "Value" if c.expression.is_none() => c.value = value,
            "Default" => c.reference = value,
            "Minimum" => c.min = value,
            "Maximum" => c.max = value,
            _ => (),
        }
    }
}

impl GeometryEditor {
    pub(super) fn control_publisher_event(
        &mut self,
        event: &TheEvent,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> bool {
        if matches!(event,TheEvent::IndexChanged(id,1) if id.name=="MZ Geometry Actions") {
            layout::set_choice(ui, "MZ Geometry Actions", 0);
            self.finish_drag(project, ctx, false);
            match Publisher::new(self, project) {
                Ok(mut p) => {
                    ui.show_dialog(TITLE, p.canvas(), vec![TheDialogButtonRole::Reject], ctx);
                    p.list(ui, ctx);
                    p.form(ui, ctx, project);
                    p.preview(ui, ctx, project);
                    self.control_publisher = Some(p);
                }
                Err(error) => Self::status(ctx, error),
            }
            return true;
        }
        if ui.dialog_text != TITLE || ui.dialog.is_none() {
            self.control_publisher = None;
            return false;
        }
        let Some(mut p) = self.control_publisher.take() else {
            return false;
        };
        let mut handled = true;
        match event {
            TheEvent::WidgetResized(id, _) if id.name == VIEW => {}
            TheEvent::NewListItemSelected(id, list) if list.name == LIST => {
                p.selected = id
                    .name
                    .strip_prefix("MZ Published Control ")
                    .and_then(|s| s.parse().ok())
                    .filter(|i: &usize| *i < p.controls.len());
                if let (Some(list), Some(i)) = (ui.get_list_layout(LIST), p.selected) {
                    list.select_item_at(i as i32, ctx, false);
                }
                p.binding_index = 0;
                p.form(ui, ctx, project);
            }
            TheEvent::ValueChanged(id, value) if id.name.starts_with("MZ Control ") => {
                let key = id.name.trim_start_matches("MZ Control ");
                if let Some(c) = p.selected.and_then(|i| p.controls.get_mut(i)) {
                    match key {
                        "Label" => c.label = value.to_string().unwrap_or_default(),
                        "Category" => c.category = value.to_string().unwrap_or_default(),
                        "Help" => c.help = value.to_string().unwrap_or_default(),
                        "Expression" => {
                            let text = value.to_string().unwrap_or_default();
                            c.expression = (!text.trim().is_empty()).then_some(text);
                            if c.expression.is_none() {
                                if let Some(w) = ui.get_widget("MZ Control Value") {
                                    w.set_value(TheValue::Float(c.value));
                                }
                            }
                        }
                        _ => (),
                    }
                }
                if let Some(v) = value.to_f32().or_else(|| value.to_i32().map(|v| v as f32)) {
                    p.number(key, v);
                }
                if matches!(key, "Label" | "Category") {
                    p.list(ui, ctx);
                }
            }
            TheEvent::PaletteIndexChanged(id, index) if id.name.starts_with("MZ Control ") => {
                p.number(id.name.trim_start_matches("MZ Control "), *index as f32)
            }
            TheEvent::IndexChanged(id, index) if id.name == "MZ Control Kind" => {
                let target = ui
                    .get_widget_value("MZ Control Target")
                    .and_then(|v| v.to_i32())
                    .unwrap_or(0)
                    .max(0) as usize;
                p.mapping((*index).max(0) as usize, target);
                p.form(ui, ctx, project);
            }
            TheEvent::IndexChanged(id, index) if id.name == "MZ Control Binding" => {
                let len = p
                    .selected
                    .and_then(|i| p.controls.get(i))
                    .map_or(0, |c| c.modifiers.len());
                let index = (*index).max(0) as usize;
                if index < len {
                    p.binding_index = index;
                } else if len < 64 {
                    if let Some(i) = p.unused_parameter() {
                        p.binding_index = len;
                        p.choose_parameter(i, false);
                    }
                }
                p.form(ui, ctx, project);
            }
            TheEvent::NewListItemSelected(id, list) if list.name == "MZ Parameter Choices" => {
                if let Some(reference) = id
                    .name
                    .strip_prefix("MZ Reference Choice ")
                    .and_then(|s| s.parse::<usize>().ok())
                    .and_then(|i| p.controls.get(i))
                    .map(|c| format!("[{}]", c.id))
                {
                    if let Some(c) = p.selected.and_then(|i| p.controls.get_mut(i)) {
                        c.expression
                            .get_or_insert_with(String::new)
                            .push_str(&reference);
                    }
                    ui.clear_popover(ctx);
                    p.form(ui, ctx, project);
                } else if let Some(index) = id
                    .name
                    .strip_prefix("MZ Parameter Choice ")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    let reset = p
                        .selected
                        .and_then(|i| p.controls.get(i))
                        .is_some_and(|c| c.modifiers.len() == 1);
                    p.choose_parameter(index, reset);
                    ui.clear_popover(ctx);
                    p.form(ui, ctx, project);
                }
            }
            TheEvent::ValueChanged(id, value) if id.name == "MZ Parameter Search" => {
                p.parameter_query = value.to_string().unwrap_or_default();
                p.parameter_list(ui, ctx);
            }
            TheEvent::IndexChanged(id, index) if id.name == "MZ Control Target" => {
                if let (Some(c), Some((_, parts))) = (
                    p.selected.and_then(|i| p.controls.get_mut(i)),
                    p.targets.get((*index).max(0) as usize),
                ) {
                    if let Some(
                        Modifier::Affine { parts: targets, .. }
                        | Modifier::Palette { parts: targets },
                    ) = c.modifiers.first_mut()
                    {
                        *targets = parts.clone();
                    }
                }
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) if id.name == SAVE => {
                let result = (|| {
                    if self.selected != Some(p.asset)
                        || self.group_path != p.path
                        || project.procedural_appearances.get(&p.asset) != Some(&p.before)
                    {
                        return Err("The recipe changed while this dialog was open. Reopen Publish controls.".to_string());
                    }
                    let candidate = p.candidate()?;
                    let scope = std::mem::replace(&mut self.group_path, p.edit_path.clone());
                    let result = self.commit(candidate, project, ctx);
                    self.group_path = scope;
                    result?;
                    if self.group_path == p.edit_path {
                        self.selected_operation = Some(p.operation.clone());
                    }
                    self.key = None;
                    Ok(())
                })();
                match result {
                    Ok(()) => {
                        ui.clear_dialog();
                        ctx.ui.relayout = true;
                        return true;
                    }
                    Err(error) => {
                        p.error = Some(error.clone());
                        if let Some(w) = ui.get_widget("MZ Control Error") {
                            w.set_value(TheValue::Text(error));
                        }
                        self.control_publisher = Some(p);
                        return true;
                    }
                }
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name.starts_with("MZ Control ") =>
            {
                match id.name.trim_start_matches("MZ Control ") {
                    "Add" if p.controls.len() < 64 => {
                        p.binding_index = 0;
                        let value = (p.bounds.1[0] - p.bounds.0[0]).max(0.001);
                        p.controls.push(Control {
                            expression: None,
                            id: Uuid::new_v4().to_string(),
                            label: "Width (m)".into(),
                            category: "Dimensions".into(),
                            help: String::new(),
                            order: p.controls.len() as u16,
                            value,
                            reference: value,
                            min: 0.001,
                            max: 100.,
                            modifiers: vec![Modifier::FitAxis { axis: 0 }],
                        });
                        p.selected = Some(p.controls.len() - 1);
                    }
                    "Choose Parameter" => {
                        p.picker_reference = false;
                        p.open_parameter_picker(ui, ctx);
                    }
                    "Insert Reference" => {
                        p.picker_reference = true;
                        p.open_parameter_picker(ui, ctx);
                    }
                    "Remove Binding" => {
                        if let Some(c) = p.selected.and_then(|i| p.controls.get_mut(i)) {
                            if c.modifiers.len() > 1 {
                                c.modifiers.remove(p.binding_index);
                                p.binding_index = p.binding_index.min(c.modifiers.len() - 1);
                            }
                        }
                    }
                    "Remove" => {
                        if let Some(i) = p.selected {
                            p.controls.remove(i);
                            p.selected = (!p.controls.is_empty())
                                .then_some(i.min(p.controls.len().saturating_sub(1)));
                        }
                    }
                    "Up" => {
                        if let Some(i) = p.selected.filter(|i| *i > 0) {
                            p.controls.swap(i, i - 1);
                            p.selected = Some(i - 1);
                        }
                    }
                    "Down" => {
                        if let Some(i) = p.selected.filter(|i| *i + 1 < p.controls.len()) {
                            p.controls.swap(i, i + 1);
                            p.selected = Some(i + 1);
                        }
                    }
                    _ => handled = false,
                }
                if matches!(
                    id.name.as_str(),
                    "MZ Control Add" | "MZ Control Remove" | "MZ Control Up" | "MZ Control Down"
                ) {
                    for (i, c) in p.controls.iter_mut().enumerate() {
                        c.order = i as u16;
                    }
                }
                p.list(ui, ctx);
                p.form(ui, ctx, project);
            }
            _ => handled = false,
        }
        if handled {
            p.preview(ui, ctx, project);
        }
        self.control_publisher = Some(p);
        handled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(doc: Document) -> (GeometryEditor, Project, Uuid) {
        let id = Uuid::new_v4();
        let mut p = Project::default();
        p.procedural_appearances.insert(id, doc.save().unwrap());
        let mut editor = GeometryEditor::default();
        editor.selected = Some(id);
        editor.selected_operation = doc
            .rows
            .iter()
            .flat_map(|r| &r.tiles)
            .find(|t| matches!(t.op, Operation::Group { .. }))
            .map(|t| t.id.clone());
        (editor, p, id)
    }
    #[test]
    fn visual_control_publisher_wraps_existing_construction_without_losing_grip_or_geometry() {
        let original = Document::load(denrim_materials::geometry::SWORD_EXAMPLE).unwrap();
        let evaluation = original.evaluate().unwrap();
        let (editor, project, id) = fixture(original.clone());
        let publisher = Publisher::new(&editor, &project).unwrap();
        let next = publisher.candidate().unwrap().evaluate().unwrap();
        assert_eq!(next.grip, evaluation.grip);
        assert_eq!(next.mesh.bounds(), evaluation.mesh.bounds());
        assert_eq!(next.mesh.vertex_count(), evaluation.mesh.vertex_count());
        assert_eq!(
            project.procedural_appearances[&id],
            original.save().unwrap()
        );
        assert!(publisher.controls.len() >= 4);
    }
    #[test]
    fn visual_control_publisher_preserves_custom_mappings_and_validates_ranges() {
        let original = Document::load(denrim_materials::geometry::HUMAN_EXAMPLE).unwrap();
        let (editor, project, _) = fixture(original);
        let mut publisher = Publisher::new(&editor, &project).unwrap();
        let index = publisher
            .controls
            .iter()
            .position(|c| kind(c) == 12)
            .expect("human has advanced controls");
        let modifiers = publisher.controls[index].modifiers.clone();
        publisher.selected = Some(index);
        publisher.controls[index].label = "Body proportions".into();
        publisher.controls[index].help = "Published visually".into();
        publisher.candidate().unwrap().evaluate().unwrap();
        assert_eq!(publisher.controls[index].modifiers, modifiers);
        publisher.number("Minimum", publisher.controls[index].max + 1.);
        assert!(publisher.candidate().unwrap().evaluate().is_err());
    }
    #[test]
    fn visual_control_publisher_saves_one_validated_recipe_edit_and_cancel_is_non_destructive() {
        let original = super::super::recipe_browser::bundled().remove(1).document;
        let (mut editor, mut project, id) = fixture(original);
        let before = project.procedural_appearances[&id].clone();
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(1000, 760, 1.);
        ui.init(&mut ctx);
        editor.control_publisher_event(
            &TheEvent::IndexChanged(TheId::named("MZ Geometry Actions"), 1),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        assert_eq!(ui.dialog_text, TITLE);
        editor.control_publisher_event(
            &TheEvent::ValueChanged(
                TheId::named("MZ Control Label"),
                TheValue::Text("Length (m)".into()),
            ),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        editor.control_publisher_event(
            &TheEvent::ValueChanged(TheId::named("MZ Control Value"), TheValue::Float(2.)),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        assert_eq!(project.procedural_appearances[&id], before);
        // Invalid values remain in the dialog without altering the project.
        editor.control_publisher_event(
            &TheEvent::ValueChanged(TheId::named("MZ Control Minimum"), TheValue::Float(20.)),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        editor.control_publisher_event(
            &TheEvent::StateChanged(TheId::named(SAVE), TheWidgetState::Clicked),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        assert!(ui.dialog.is_some());
        assert_eq!(project.procedural_appearances[&id], before);
        editor.control_publisher_event(
            &TheEvent::ValueChanged(TheId::named("MZ Control Minimum"), TheValue::Float(0.02)),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        editor.control_publisher_event(
            &TheEvent::StateChanged(TheId::named(SAVE), TheWidgetState::Clicked),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        assert!(ui.dialog.is_none());
        let after = project.procedural_appearances[&id].clone();
        assert_ne!(after, before);
        let doc = Document::load(&after).unwrap();
        let c = super::super::recipe_browser::controls(&doc).unwrap();
        assert_eq!(c[0].label, "Length (m)");
        assert_eq!(c[0].value, 2.);
        let mut server = ServerContext::default();
        let undo = ProjectUndoAtom::ProceduralAppearanceEdit(
            id,
            Some(before.clone()),
            Some(after.clone()),
        );
        undo.undo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(project.procedural_appearances[&id], before);
        undo.redo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(project.procedural_appearances[&id], after);
        editor.control_publisher_event(
            &TheEvent::IndexChanged(TheId::named("MZ Geometry Actions"), 1),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        editor.control_publisher_event(
            &TheEvent::ValueChanged(TheId::named("MZ Control Value"), TheValue::Float(3.)),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        ui.clear_dialog();
        assert_eq!(project.procedural_appearances[&id], after);
    }
    #[test]
    fn visual_control_publisher_edits_nested_groups_and_rejects_stale_sources() {
        let original = super::super::recipe_browser::bundled().remove(1).document;
        let outer = original.rows[0].tiles[0].id.clone();
        let outer_controls = super::super::recipe_browser::controls(&original)
            .unwrap()
            .clone();
        let (mut editor, mut project, id) = fixture(original);
        editor.group_path = vec![outer.clone()];
        editor.selected_operation = None;
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(1000, 760, 1.);
        ui.init(&mut ctx);
        let open = TheEvent::IndexChanged(TheId::named("MZ Geometry Actions"), 1);
        let save = TheEvent::StateChanged(TheId::named(SAVE), TheWidgetState::Clicked);
        editor.control_publisher_event(&open, &mut project, &mut ui, &mut ctx);
        editor.control_publisher.as_mut().unwrap().controls[0].label = "Published width".into();
        editor.control_publisher_event(&save, &mut project, &mut ui, &mut ctx);
        assert!(ui.dialog.is_none());
        let saved = Document::parse(&project.procedural_appearances[&id]).unwrap();
        let updated = super::super::recipe_browser::controls(&saved).unwrap();
        assert_eq!(updated[0].label, "Published width");
        assert_eq!(updated[0].modifiers, outer_controls[0].modifiers);
        let inner = saved.group_document(&[outer]).unwrap();
        assert!(super::super::recipe_browser::controls(inner).is_none());
        assert!(saved.evaluate().is_ok());
        editor.control_publisher_event(&open, &mut project, &mut ui, &mut ctx);
        let mut external = saved.clone();
        external.label = "Changed elsewhere".into();
        let external_source = external.save().unwrap();
        project
            .procedural_appearances
            .insert(id, external_source.clone());
        editor.control_publisher_event(&save, &mut project, &mut ui, &mut ctx);
        assert!(ui.dialog.is_some());
        assert!(
            editor
                .control_publisher
                .as_ref()
                .unwrap()
                .error
                .as_ref()
                .unwrap()
                .contains("changed")
        );
        assert_eq!(project.procedural_appearances[&id], external_source);
    }
    #[test]
    fn visual_parameter_publishing_authors_multiple_inputs_and_persists_recipe_bindings() {
        let mut original = super::super::recipe_browser::bundled().remove(0).document;
        let Operation::Group { controls, .. } = &mut original.rows[0].tiles[0].op else {
            panic!()
        };
        controls.truncate(1);
        controls[0].modifiers = vec![Modifier::FitAxis { axis: 1 }];
        let (mut editor, mut project, id) = fixture(original);
        let before = project.procedural_appearances[&id].clone();
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(1100, 800, 1.);
        ui.init(&mut ctx);
        let send = |e: TheEvent,
                    editor: &mut GeometryEditor,
                    project: &mut Project,
                    ui: &mut TheUI,
                    ctx: &mut TheContext| {
            assert!(editor.control_publisher_event(&e, project, ui, ctx));
        };
        send(
            TheEvent::IndexChanged(TheId::named("MZ Geometry Actions"), 1),
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
        );
        send(
            TheEvent::IndexChanged(TheId::named("MZ Control Kind"), 10),
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
        );
        let height = editor
            .control_publisher
            .as_ref()
            .unwrap()
            .parameters
            .iter()
            .position(|p| p.target.operation == "solid-0" && p.target.parameter == "size1")
            .unwrap();
        let position = editor
            .control_publisher
            .as_ref()
            .unwrap()
            .parameters
            .iter()
            .position(|p| p.target.operation == "solid-2" && p.target.parameter == "position1")
            .unwrap();
        send(
            TheEvent::NewListItemSelected(
                TheId::named(&format!("MZ Parameter Choice {height}")),
                TheId::named("MZ Parameter Choices"),
            ),
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
        );
        send(
            TheEvent::IndexChanged(TheId::named("MZ Control Binding"), 1),
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
        );
        send(
            TheEvent::NewListItemSelected(
                TheId::named(&format!("MZ Parameter Choice {position}")),
                TheId::named("MZ Parameter Choices"),
            ),
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
        );
        send(
            TheEvent::ValueChanged(TheId::named("MZ Control Factor"), TheValue::Float(0.5)),
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
        );
        send(
            TheEvent::ValueChanged(TheId::named("MZ Control Value"), TheValue::Float(3.)),
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
        );
        assert_eq!(project.procedural_appearances[&id], before);
        assert!(editor.control_publisher.as_ref().unwrap().error.is_none());
        send(
            TheEvent::StateChanged(TheId::named(SAVE), TheWidgetState::Clicked),
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
        );
        assert!(ui.dialog.is_none());
        let saved = Document::load(&project.procedural_appearances[&id]).unwrap();
        let Operation::Group {
            document, controls, ..
        } = &saved.rows[0].tiles[0].op
        else {
            panic!()
        };
        assert_eq!(controls[0].modifiers.len(), 2);
        assert_eq!(kind(&controls[0]), 10);
        let Operation::Box { size } = document.rows[1].tiles[0].op else {
            panic!()
        };
        assert_eq!(size, [1.; 3]);
        let (lo, hi) = saved.evaluate().unwrap().mesh.bounds();
        assert!(lo[1].abs() < 0.00001);
        assert!((hi[1] - 3.).abs() < 0.00001);
        project.regions[0].map.camera = rusterix::MapCamera::TwoDIso;
        assert!(shared::isometric::synchronize(&mut project).is_empty());
        assert!(project.tiles[&id].isometric.is_some());
    }
    #[test]
    fn visual_formulas_insert_references_preview_validate_and_save_in_the_recipe() {
        let original = Document::load(include_str!(
            "../../embedded/meshes/recipes/fence.mzgeometry.json"
        ))
        .unwrap();
        let (mut editor, mut project, id) = fixture(original);
        let before = project.procedural_appearances[&id].clone();
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(1100, 800, 1.);
        ui.init(&mut ctx);
        let send = |e,
                    editor: &mut GeometryEditor,
                    project: &mut Project,
                    ui: &mut TheUI,
                    ctx: &mut TheContext| {
            assert!(editor.control_publisher_event(&e, project, ui, ctx));
        };
        send(
            TheEvent::IndexChanged(TheId::named("MZ Geometry Actions"), 1),
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
        );
        let length = editor
            .control_publisher
            .as_ref()
            .unwrap()
            .controls
            .iter()
            .position(|c| c.id == "length")
            .unwrap();
        send(
            TheEvent::NewListItemSelected(
                TheId::named(&format!("MZ Published Control {length}")),
                TheId::named(LIST),
            ),
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
        );
        send(
            TheEvent::ValueChanged(
                TheId::named("MZ Control Expression"),
                TheValue::Text(String::new()),
            ),
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
        );
        send(
            TheEvent::StateChanged(
                TheId::named("MZ Control Insert Reference"),
                TheWidgetState::Clicked,
            ),
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
        );
        assert!(ui.get_widget("MZ Parameter Search").is_some());
        send(
            TheEvent::NewListItemSelected(
                TheId::named("MZ Reference Choice 0"),
                TheId::named("MZ Parameter Choices"),
            ),
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
        );
        assert_eq!(
            editor.control_publisher.as_ref().unwrap().controls[length]
                .expression
                .as_deref(),
            Some("[count]")
        );
        send(
            TheEvent::ValueChanged(
                TheId::named("MZ Control Expression"),
                TheValue::Text("[count] * [spacing]".into()),
            ),
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
        );
        assert!(editor.control_publisher.as_ref().unwrap().error.is_none());
        assert!(
            (ui.get_widget_value("MZ Control Value")
                .unwrap()
                .to_f32()
                .unwrap()
                - 2.1)
                .abs()
                < 1e-5
        );
        assert_eq!(project.procedural_appearances[&id], before);
        // Typed/cyclic expressions leave the source and last valid project untouched.
        send(
            TheEvent::ValueChanged(
                TheId::named("MZ Control Expression"),
                TheValue::Text("length + 1".into()),
            ),
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
        );
        assert!(
            editor
                .control_publisher
                .as_ref()
                .unwrap()
                .error
                .as_ref()
                .unwrap()
                .contains("Cyclic")
        );
        send(
            TheEvent::StateChanged(TheId::named(SAVE), TheWidgetState::Clicked),
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
        );
        assert_eq!(project.procedural_appearances[&id], before);
        assert!(ui.dialog.is_some());
        send(
            TheEvent::ValueChanged(
                TheId::named("MZ Control Expression"),
                TheValue::Text("count * spacing".into()),
            ),
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
        );
        send(
            TheEvent::StateChanged(TheId::named(SAVE), TheWidgetState::Clicked),
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
        );
        assert!(ui.dialog.is_none());
        let saved = Document::load(&project.procedural_appearances[&id]).unwrap();
        let mut controls = super::super::recipe_browser::controls(&saved)
            .unwrap()
            .clone();
        assert_eq!(
            controls[length].expression.as_deref(),
            Some("count * spacing")
        );
        controls[0].value = 8.;
        let selection = rigging::Selection::default();
        let (numbers, _) = groups::parameters(&controls, &selection);
        let n = numbers.iter().find(|n| n.id == "control:length").unwrap();
        assert!((n.value - 2.8).abs() < 1e-5);
        assert_eq!(n.min, n.max);
        assert!(groups::edit(&mut controls, "control:length", 5., &selection).is_err());
        groups::edit(&mut controls, "group_reset", 2., &selection).unwrap();
        assert!(
            (denrim_materials::geometry::expressions::resolve(&controls).unwrap()[length].value
                - 2.1)
                .abs()
                < 1e-5
        );
    }
}
