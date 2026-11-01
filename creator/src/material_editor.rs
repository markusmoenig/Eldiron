//! Native MaterialZ row-editor presentation. The host owns the validated document,
//! material evaluation, persistence and undo. This widget only emits editor commands.
use crate::prelude::*;
use theframework::thegraph::{
    GraphAssets, GraphPainter, GraphRasterResources, GraphRect, GraphTheme, RasterGraphPainter,
};

pub const MATERIAL_EDITOR_COMMANDS: &str = "Material Editor Commands";

/// Resolve all procedural-editor surfaces through the active application theme.
pub fn material_editor_theme(theme: &dyn TheTheme) -> GraphTheme {
    GraphTheme {
        background: *theme.color(MaterialCanvas),
        grid: *theme.color(MaterialGrid),
        body: *theme.color(MaterialPanel),
        control: *theme.color(MaterialControl),
        text: *theme.color(MaterialText),
        muted: *theme.color(MaterialMuted),
        wire: *theme.color(MaterialWire),
        active: *theme.color(SelectedTextEditBorder1),
        execution: *theme.color(TextEditLineNumberDebugColor),
        yes: *theme.color(Green),
        no: *theme.color(Red),
    }
}

#[derive(Clone)]
struct MaterialEditorChrome {
    dropdown: RGBA,
    source: RGBA,
    popup: RGBA,
    hover: RGBA,
    selection: RGBA,
    shadow: RGBA,
    checker: [RGBA; 2],
    categories: [RGBA; 6],
}
impl MaterialEditorChrome {
    fn from_theme(theme: &dyn TheTheme) -> Self {
        Self {
            dropdown: *theme.color(MaterialDropdown),
            source: *theme.color(MaterialSource),
            popup: *theme.color(MaterialPopup),
            hover: *theme.color(MaterialPopupHover),
            selection: *theme.color(MaterialPopupSelection),
            shadow: *theme.color(MaterialShadow),
            checker: [
                *theme.color(MaterialCheckerDark),
                *theme.color(MaterialCheckerLight),
            ],
            categories: std::array::from_fn(|i| {
                theme.palette_color(TheThemePalettes::MaterialCategories, i)
            }),
        }
    }
    fn category(&self, category: &str) -> RGBA {
        self.categories[match category.to_lowercase().as_str() {
            "noise" => 0,
            "shape" | "shapes" | "sdf" => 1,
            "color" | "style" => 2,
            "transform" | "coordinates" => 3,
            "math" | "operator" | "operators" => 4,
            _ => 5,
        }]
    }
}

#[derive(Clone, Debug, Default)]
pub struct MaterialEditorView {
    pub title: String,
    /// Material constants, edited when no operation is selected.
    pub defaults: Vec<MaterialNumberView>,
    pub rows: Vec<MaterialRowView>,
    pub outputs: Vec<MaterialOutputView>,
    pub sources: Vec<MaterialSourceView>,
    pub palette: Vec<MaterialPaletteEntry>,
    pub palette_bindings: Vec<MaterialPaletteBindingView>,
    pub tiling: MaterialTilingView,
    pub animation: MaterialAnimationView,
}
#[derive(Clone, Debug)]
pub struct MaterialPaletteBindingView {
    pub operation: String,
    pub parameter: String,
    pub index: u16,
}
#[derive(Clone, Debug)]
pub struct MaterialAnimationView {
    pub frames: u32,
    pub duration: f32,
}
impl Default for MaterialAnimationView {
    fn default() -> Self {
        Self {
            frames: 1,
            duration: 2.,
        }
    }
}
#[derive(Clone, Debug)]
pub struct MaterialRowView {
    pub id: String,
    pub label: String,
    pub operations: Vec<MaterialOperationView>,
    pub inputs: Vec<MaterialInputView>,
}
#[derive(Clone, Debug)]
pub struct MaterialOperationView {
    pub id: String,
    pub label: String,
    /// Host-supplied bitmap asset key; the widget never evaluates a material.
    pub thumbnail: String,
    pub parameters: Vec<MaterialNumberView>,
    pub palette: Option<MaterialPaletteView>,
    pub properties: Vec<MaterialPropertyView>,
    pub combine: Option<String>,
    pub source: Option<MaterialSourceCodeView>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialTilingView {
    pub mode: String,
    pub period: [f32; 2],
    pub blend: f32,
}
impl Default for MaterialTilingView {
    fn default() -> Self {
        Self {
            mode: "raw".into(),
            period: [1., 1.],
            blend: 0.15,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MaterialPropertyKind {
    Number,
    Boolean,
    Choice,
    Vector,
    Color,
}
#[derive(Clone, Debug)]
pub struct MaterialPropertyView {
    pub id: String,
    pub label: String,
    pub group: String,
    pub kind: MaterialPropertyKind,
    /// IDs of the flattened, ranged parameters backing this control.
    pub components: Vec<String>,
    pub choices: Vec<String>,
    pub tooltip: String,
}
#[derive(Clone, Debug)]
pub struct MaterialSourceCodeView {
    pub code: String,
    pub help: String,
    pub references: String,
}
#[derive(Clone, Debug)]
pub struct MaterialNumberView {
    pub id: String,
    pub label: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
}
#[derive(Clone, Debug)]
pub struct MaterialOutputView {
    pub id: String,
    pub label: String,
    pub row: Option<String>,
    pub fallback: String,
}
#[derive(Clone, Debug)]
pub struct MaterialSourceView {
    pub id: String,
    pub label: String,
    pub category: String,
}

#[derive(Clone, Debug)]
pub struct MaterialPaletteEntry {
    pub index: u16,
    pub label: String,
    pub color: [u8; 4],
}
#[derive(Clone, Debug)]
pub struct MaterialPaletteView {
    pub index: Option<u16>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialReferenceView {
    pub row: String,
    pub components: String,
    pub mapping: Option<[f32; 2]>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MaterialInputTarget {
    RowLink {
        row: String,
        connection: String,
    },
    Coordinates {
        row: String,
    },
    Image {
        operation: String,
    },
    Parameter {
        operation: String,
        parameter: String,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub enum MaterialInputBinding {
    Default,
    Disconnected,
    Row(MaterialReferenceView),
}
#[derive(Clone, Debug)]
pub struct MaterialInputView {
    pub target: MaterialInputTarget,
    pub label: String,
    pub binding: MaterialInputBinding,
    pub components: String,
}

/// Commands carry host identities, never row/tile array indices. The host validates
/// candidate changes in MaterialZ and supplies a fresh view after a successful edit.
#[derive(Clone, Debug, PartialEq)]
pub enum MaterialEditorCommand {
    InspectOperation(String),
    InspectSurface,
    InspectSource(String),
    InspectRow(String),
    InspectInput {
        target: MaterialInputTarget,
        binding: MaterialInputBinding,
    },
    AssignRowInput {
        row: String,
        connection: String,
        target: MaterialInputTarget,
    },
    SetAnimation {
        frames: u32,
        duration: f32,
    },
    SetTiling {
        mode: String,
        period: [f32; 2],
        blend: f32,
    },
    SetCombine {
        operation: String,
        mode: String,
    },
    SetSource {
        operation: String,
        code: String,
    },
    SetInput {
        target: MaterialInputTarget,
        binding: MaterialInputBinding,
    },
    SetPaletteParameter {
        operation: String,
        parameter: String,
        index: u16,
    },
    SetPalette {
        operation: String,
        index: u16,
    },
    AddSource {
        source: String,
        row: Option<String>,
        after: Option<String>,
    },
    MoveOperation {
        operation: String,
        row: String,
        after: Option<String>,
    },
    ConnectOutput {
        output: String,
        row: String,
    },
    DisconnectOutput(String),
    SetNumber {
        operation: String,
        parameter: String,
        value: f32,
    },
    RemoveOperation(String),
}

#[derive(Clone, Debug)]
enum Gesture {
    Connection {
        row: String,
        start: [f32; 2],
    },
    Source {
        id: String,
        start: [f32; 2],
        point: [f32; 2],
    },
    Operation {
        id: String,
        start: [f32; 2],
        point: [f32; 2],
    },
    RowWire {
        row: String,
        point: [f32; 2],
    },
    OutputWire {
        output: String,
        start: [f32; 2],
        point: [f32; 2],
    },
    Number {
        operation: String,
        parameter: String,
        original: f32,
        value: f32,
        min: f32,
        max: f32,
        start: [f32; 2],
    },
}
#[derive(Clone, Debug, PartialEq)]
enum Hit {
    Pane(MaterialPane),
    RowInput(String),
    InputControl(usize),
    PopupOption(usize),
    Palette(usize),
    Property(usize),
    SourceAction(usize),
    Surface,
    Search,
    SourceCategory,
    Source(String),
    Operation(String),
    RowWire(String),
    Connection(String),
    Output(String),
    Number(String, String),
    Drop {
        row: Option<String>,
        after: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum MaterialPane {
    Properties,
    RowInput,
    Tile,
    #[default]
    Sources,
}

const HEADER: f32 = 0.;
const RAIL: f32 = 88.;
const GUTTER: f32 = 32.;
const CARD_LEFT: f32 = RAIL + GUTTER + 8.;
const COLUMNS: usize = 3;
const CARD: f32 = 76.;
const LINE: f32 = 86.;
const SOURCE_STEP: f32 = 32.;
const PARAMETER_STEP: f32 = 24.;
#[derive(Clone)]
struct Layout {
    width: f32,
    height: f32,
    editor_bottom: f32,
    properties_bottom: f32,
    library_top: f32,
    rows: Vec<(f32, f32)>,
    row_lengths: Vec<usize>,
    content_bottom: f32,
}
impl Layout {
    fn new(size: [f32; 2], parameters: usize, rows: &[MaterialRowView]) -> Self {
        let _ = parameters;
        let properties = 124.;
        let editor_bottom = (size[1] - properties - 190.).max(216.).min(size[1] * 0.6);
        let properties_bottom = size[1];
        let mut top = 8.;
        let row_lengths = rows.iter().map(|row| row.operations.len()).collect();
        let rows = rows
            .iter()
            .map(|row| {
                let count = row.operations.len();
                let height = 6. + (count + 1).div_ceil(COLUMNS) as f32 * LINE;
                let result = (top, height);
                top += height + 4.;
                result
            })
            .collect();
        Self {
            width: size[0],
            height: size[1],
            editor_bottom,
            properties_bottom,
            library_top: editor_bottom + 54.,
            rows,
            row_lengths,
            content_bottom: top + CARD + 8.,
        }
    }
    fn row(&self, index: usize, pan: [f32; 2]) -> GraphRect {
        let (top, height) = self.rows[index];
        rect(
            CARD_LEFT - 4.,
            top - pan[1],
            self.width - CARD_LEFT - 6.,
            height,
        )
    }
    fn card(&self, row: usize, column: usize, pan: [f32; 2]) -> GraphRect {
        let r = self.row(row, pan);
        rect(
            CARD_LEFT + (column % COLUMNS) as f32 * (self.card_width() + 8.),
            r.origin[1] + 4. + (column / COLUMNS) as f32 * LINE,
            self.card_width(),
            CARD,
        )
    }
    fn card_width(&self) -> f32 {
        ((self.width - CARD_LEFT - 24. - (COLUMNS - 1) as f32 * 8.) / COLUMNS as f32)
            .clamp(20., CARD)
    }
    fn row_input(&self, index: usize, pan: [f32; 2]) -> GraphRect {
        let row = self.row(index, pan);
        rect(CARD_LEFT - 14., row.origin[1] + 20., 10., 22.)
    }
    fn input_control(&self, index: usize) -> GraphRect {
        rect(
            10.,
            self.editor_bottom
                + 30.
                + (if index == 7 {
                    2
                } else if index >= 2 {
                    index + 1
                } else {
                    index
                }) as f32
                    * 28.,
            self.width - 20.,
            24.,
        )
    }
    fn output(&self, index: usize) -> GraphRect {
        rect(RAIL - 16., 14. + index as f32 * 38., 10., 10.)
    }
    fn row_port(&self, index: usize, pan: [f32; 2]) -> GraphRect {
        let r = self.row(index, pan);
        let line = self.row_lengths[index].saturating_sub(1) / COLUMNS;
        rect(
            self.width - 14.,
            r.origin[1] + 27. + line as f32 * LINE,
            10.,
            10.,
        )
    }
    fn new_row(&self, _rows: usize, pan: [f32; 2]) -> GraphRect {
        rect(
            CARD_LEFT,
            self.content_bottom - CARD - 8. - pan[1],
            self.card_width(),
            CARD,
        )
    }
    fn pane_tab(&self, index: usize) -> GraphRect {
        let width = (self.width - 20.) / 4.;
        rect(
            10. + index as f32 * width,
            self.editor_bottom + 2.,
            width,
            22.,
        )
    }
    fn search(&self) -> GraphRect {
        rect(10., self.editor_bottom + 28., (self.width - 24.) * 0.6, 22.)
    }
    fn category(&self) -> GraphRect {
        let search = self.search();
        rect(
            search.origin[0] + search.size[0] + 4.,
            search.origin[1],
            (self.width - 24.) * 0.4,
            22.,
        )
    }
    fn parameter(&self, index: usize) -> GraphRect {
        rect(
            10.,
            self.editor_bottom + 52. + index as f32 * PARAMETER_STEP,
            self.width - 20.,
            20.,
        )
    }
}

struct ConnectionPath {
    source: usize,
    target: Option<usize>,
    points: Vec<[f32; 2]>,
}
impl ConnectionPath {
    /// Clip individual segments so a connection remains visible when its source
    /// row is offscreen, without drawing over the fixed lower pane.
    fn segments(&self, bottom: f32) -> Vec<[[f32; 2]; 2]> {
        self.points
            .windows(2)
            .filter_map(|pair| {
                let [mut a, mut b] = [pair[0], pair[1]];
                let top = HEADER + 1.;
                let bottom = bottom - 1.;
                if a[1].max(b[1]) < top || a[1].min(b[1]) > bottom {
                    return None;
                }
                let dy = b[1] - a[1];
                if dy.abs() > f32::EPSILON {
                    let from = ((top - a[1]) / dy).min((bottom - a[1]) / dy).max(0.);
                    let to = ((top - a[1]) / dy).max((bottom - a[1]) / dy).min(1.);
                    let original = a;
                    let dx = b[0] - a[0];
                    a = [original[0] + dx * from, original[1] + dy * from];
                    b = [original[0] + dx * to, original[1] + dy * to];
                }
                Some([a, b])
            })
            .collect()
    }
    fn distance(&self, point: [f32; 2], bottom: f32) -> f32 {
        self.segments(bottom)
            .into_iter()
            .map(|[a, b]| {
                let delta = [b[0] - a[0], b[1] - a[1]];
                let length = delta[0] * delta[0] + delta[1] * delta[1];
                let t = if length > 0. {
                    (((point[0] - a[0]) * delta[0] + (point[1] - a[1]) * delta[1]) / length)
                        .clamp(0., 1.)
                } else {
                    0.
                };
                (point[0] - a[0] - delta[0] * t).powi(2) + (point[1] - a[1] - delta[1] * t).powi(2)
            })
            .fold(f32::INFINITY, f32::min)
    }
}

fn linear_to_srgb(value: f32) -> f32 {
    if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1. / 2.4) - 0.055
    }
}
fn rect(x: f32, y: f32, w: f32, h: f32) -> GraphRect {
    GraphRect {
        origin: [x, y],
        size: [w.max(0.), h.max(0.)],
    }
}
fn center(r: GraphRect) -> [f32; 2] {
    [r.origin[0] + r.size[0] / 2., r.origin[1] + r.size[1] / 2.]
}

fn centered_text(
    p: &mut impl GraphPainter,
    area: GraphRect,
    label: &str,
    size: f32,
    color: [u8; 4],
) {
    let mut visible = label.to_owned();
    if p.text_width(&visible, size) > area.size[0] {
        let ellipsis = "…";
        if p.text_width(ellipsis, size) > area.size[0] {
            return;
        }
        while !visible.is_empty()
            && p.text_width(&format!("{visible}{ellipsis}"), size) > area.size[0]
        {
            visible.pop();
        }
        visible.push_str(ellipsis);
    }
    let width = p.text_width(&visible, size);
    p.text(
        rect(
            area.origin[0] + (area.size[0] - width) / 2.,
            area.origin[1] + (area.size[1] - size) / 2.,
            width,
            size + 2.,
        ),
        &visible,
        size,
        color,
    );
}
fn empty_tile(p: &mut impl GraphPainter, area: GraphRect, theme: &GraphTheme) {
    p.round_rect(area, 4., theme.control);
    p.round_rect(
        rect(
            area.origin[0] + 1.,
            area.origin[1] + 1.,
            area.size[0] - 2.,
            area.size[1] - 2.,
        ),
        3.,
        theme.background,
    );
}

#[derive(Clone)]
enum PropertyLine {
    Section(String),
    Bound {
        label: String,
        source: String,
        target: MaterialInputTarget,
    },
    Values {
        owner: String,
        label: String,
        values: Vec<MaterialNumberView>,
        color: Option<String>,
        tooltip: String,
    },
    Choice {
        owner: String,
        parameter: String,
        label: String,
        value: usize,
        choices: Vec<String>,
        tooltip: String,
    },
    Boolean {
        owner: String,
        parameter: String,
        label: String,
        value: bool,
        tooltip: String,
    },
}
impl PropertyLine {
    fn tooltip(&self) -> &str {
        match self {
            Self::Values { tooltip, .. }
            | Self::Choice { tooltip, .. }
            | Self::Boolean { tooltip, .. } => tooltip,
            Self::Section(_) => "",
            Self::Bound { .. } => "Controlled by a row input. Click to edit the connection.",
        }
    }
}

pub struct MaterialEditorWidget {
    chrome: MaterialEditorChrome,
    id: TheId,
    dim: TheDim,
    limiter: TheSizeLimiter,
    view: MaterialEditorView,
    selected: Option<String>,
    selected_source: Option<String>,
    pane: MaterialPane,
    gesture: Option<Gesture>,
    commands: Vec<MaterialEditorCommand>,
    pan: [f32; 2],
    library_scroll: usize,
    parameter_scroll: usize,
    library_fraction: f32,
    parameter_fraction: f32,
    query: String,
    source_category: Option<String>,
    search_focused: bool,
    pointer: [f32; 2],
    dirty: bool,
    raster: GraphRasterResources,
    thumbnails: std::collections::HashMap<String, TheRGBABuffer>,
    number_edits: Vec<(String, String, TheTextLineEdit)>,
    number_focus: Option<usize>,
    number_drag: bool,
    selected_row: Option<String>,
    selected_input: usize,
    input_popup: Option<usize>,
    palette_scroll: usize,
    popup_scroll: usize,
    source_edit: Option<(String, TheTextAreaEdit)>,
    source_focus: bool,
    source_help: bool,
    source_scroll_fraction: [f32; 2],
    color_property: Option<usize>,
}
impl MaterialEditorWidget {
    pub fn set_view(&mut self, view: MaterialEditorView) {
        self.gesture = None;
        self.source_edit = None;
        self.source_focus = false;
        self.color_property = None;
        self.number_edits.clear();
        self.number_focus = None;
        self.commands.clear();
        self.input_popup = None;
        if !view
            .rows
            .iter()
            .any(|row| Some(&row.id) == self.selected_row.as_ref())
        {
            self.selected_row = view.rows.first().map(|r| r.id.clone());
            self.selected_input = 0;
        }
        if !view
            .rows
            .iter()
            .flat_map(|r| &r.operations)
            .any(|op| Some(&op.id) == self.selected.as_ref())
        {
            self.selected = None;
        }
        if !view
            .sources
            .iter()
            .any(|source| Some(&source.id) == self.selected_source.as_ref())
        {
            self.selected_source = None;
        }
        self.thumbnails.retain(|key, _| {
            view.rows
                .iter()
                .flat_map(|row| &row.operations)
                .any(|op| &op.thumbnail == key)
        });
        self.view = view;
        self.clamp_scroll();
        self.dirty = true;
    }
    fn input_row(&self) -> Option<&MaterialRowView> {
        self.view
            .rows
            .iter()
            .find(|row| Some(&row.id) == self.selected_row.as_ref())
    }
    fn active_input(&self) -> Option<&MaterialInputView> {
        self.input_row()?.inputs.get(self.selected_input)
    }
    fn input_owner(input: &MaterialInputView) -> String {
        format!("@row-input:{:?}", input.target)
    }
    fn input_options(&self, control: usize) -> Vec<String> {
        if control >= 1000 {
            return self.property_options(control - 1000);
        }
        match control {
            8 => {
                let mut categories: Vec<_> = self
                    .view
                    .sources
                    .iter()
                    .map(|source| source.category.clone())
                    .collect();
                categories.sort();
                categories.dedup();
                categories.insert(0, "All".into());
                categories
            }
            3 => ["x", "y", "z", "w", "xy", "xyz", "xyzw", "distance"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            7 => self
                .input_row()
                .map(|r| {
                    r.inputs
                        .iter()
                        .filter(|i| !matches!(i.target, MaterialInputTarget::RowLink { .. }))
                        .map(|i| i.label.clone())
                        .collect()
                })
                .unwrap_or_default(),
            0 => self.view.rows.iter().map(|r| r.label.clone()).collect(),
            1 => self
                .input_row()
                .map(|r| r.inputs.iter().map(|i| i.label.clone()).collect())
                .unwrap_or_default(),
            2 => {
                let mut options = vec!["Default".into()];
                if matches!(
                    self.active_input().map(|i| &i.target),
                    Some(MaterialInputTarget::Image { .. })
                ) {
                    options.push("Disconnected".into());
                }
                options.extend(self.view.rows.iter().map(|r| r.label.clone()));
                options
            }
            _ => vec![],
        }
    }
    fn choose_input_option(&mut self, control: usize, index: usize) {
        if control >= 1000 {
            match self.property_lines().get(control - 1000) {
                Some(PropertyLine::Choice {
                    owner, parameter, ..
                }) => {
                    self.commands
                        .push(self.property_number_command(owner, parameter, index as f32))
                }
                Some(PropertyLine::Values {
                    owner,
                    color: Some(parameter),
                    ..
                }) => {
                    if let Some(entry) = self.view.palette.get(index) {
                        self.commands
                            .push(MaterialEditorCommand::SetPaletteParameter {
                                operation: owner.clone(),
                                parameter: parameter.clone(),
                                index: entry.index,
                            });
                    }
                }
                _ => {}
            }
            self.input_popup = None;
            self.dirty = true;
            return;
        }
        match control {
            3 => {
                if let Some(input) = self.active_input() {
                    if let MaterialInputBinding::Row(mut reference) = input.binding.clone() {
                        if let Some(components) = self.input_options(3).get(index) {
                            reference.components = components.clone();
                            self.commands.push(MaterialEditorCommand::SetInput {
                                target: input.target.clone(),
                                binding: MaterialInputBinding::Row(reference),
                            });
                        }
                    }
                }
            }
            8 => {
                self.source_category = if index == 0 {
                    None
                } else {
                    self.input_options(8).get(index).cloned()
                };
                self.library_scroll = 0;
            }
            7 => {
                if let Some(input) = self.active_input() {
                    if let MaterialInputTarget::RowLink { row, connection } = &input.target {
                        if let Some(target) = self.input_row().and_then(|r| {
                            r.inputs
                                .iter()
                                .filter(|i| {
                                    !matches!(i.target, MaterialInputTarget::RowLink { .. })
                                })
                                .nth(index)
                        }) {
                            self.commands.push(MaterialEditorCommand::AssignRowInput {
                                row: row.clone(),
                                connection: connection.clone(),
                                target: target.target.clone(),
                            });
                        }
                    }
                }
            }
            0 => {
                if let Some(row) = self.view.rows.get(index) {
                    self.selected_row = Some(row.id.clone());
                    self.selected_input = 0;
                    self.commands
                        .push(MaterialEditorCommand::InspectRow(row.id.clone()));
                }
            }
            1 => {
                self.selected_input = index;
            }
            2 => {
                if let Some(input) = self.active_input() {
                    let target = input.target.clone();
                    let is_image = matches!(target, MaterialInputTarget::Image { .. });
                    let binding = if index == 0 {
                        MaterialInputBinding::Default
                    } else if is_image && index == 1 {
                        MaterialInputBinding::Disconnected
                    } else {
                        let row_index = index - 1 - usize::from(is_image);
                        let Some(row) = self.view.rows.get(row_index) else {
                            return;
                        };
                        let (components, mapping) = match &input.binding {
                            MaterialInputBinding::Row(reference) => {
                                (reference.components.clone(), reference.mapping)
                            }
                            _ => (input.components.clone(), None),
                        };
                        MaterialInputBinding::Row(MaterialReferenceView {
                            row: row.id.clone(),
                            components,
                            mapping,
                        })
                    };
                    self.commands
                        .push(MaterialEditorCommand::SetInput { target, binding });
                }
            }
            _ => {}
        }
        if control <= 1 {
            self.inspect_active_input();
        }
        self.number_edits.clear();
        self.number_focus = None;
        self.input_popup = None;
        self.dirty = true;
    }
    fn inspect_active_input(&mut self) {
        if let Some(input) = self.active_input() {
            self.commands.push(MaterialEditorCommand::InspectInput {
                target: input.target.clone(),
                binding: input.binding.clone(),
            });
        }
    }
    pub fn selected_input_preview(&self) -> Option<(MaterialInputTarget, MaterialReferenceView)> {
        if self.pane != MaterialPane::RowInput {
            return None;
        }
        let input = self.active_input()?;
        let MaterialInputBinding::Row(reference) = &input.binding else {
            return None;
        };
        Some((input.target.clone(), reference.clone()))
    }
    fn input_control_label(&self, index: usize) -> String {
        match index {
            0 => format!(
                "Row: {}",
                self.input_row()
                    .map_or("Choose a row", |r| r.label.as_str())
            ),
            1 => format!(
                "Target: {}",
                self.active_input()
                    .map_or("No inputs available", |i| i.label.as_str())
            ),
            2 => format!(
                "Source: {}",
                self.active_input()
                    .map_or("Default".into(), |i| match &i.binding {
                        MaterialInputBinding::Default => "Default".into(),
                        MaterialInputBinding::Disconnected => "Disconnected".into(),
                        MaterialInputBinding::Row(reference) => self
                            .view
                            .rows
                            .iter()
                            .find(|r| r.id == reference.row)
                            .map_or(reference.row.clone(), |r| r.label.clone()),
                    })
            ),
            3 => "Channels".into(),
            4 => format!(
                "Remap: {}",
                if matches!(
                    self.active_input().map(|i| &i.binding),
                    Some(MaterialInputBinding::Row(MaterialReferenceView {
                        mapping: Some(_),
                        ..
                    }))
                ) {
                    "On"
                } else {
                    "Off"
                }
            ),
            5 => "Remap low".into(),
            6 => "Remap high".into(),
            7 => "Use as: Choose an input…".into(),
            _ => String::new(),
        }
    }
    fn sync_input_edits(&mut self, ctx: &mut TheContext) {
        let Some(input) = self.active_input() else {
            self.number_edits.clear();
            self.number_focus = None;
            return;
        };
        let MaterialInputBinding::Row(reference) = &input.binding else {
            self.number_edits.clear();
            self.number_focus = None;
            return;
        };
        let owner = Self::input_owner(input);
        let layout = self.layout();
        let mut fields = vec![(
            "components".to_string(),
            TheValue::Text(reference.components.clone()),
            None,
            layout.input_control(3),
        )];
        if let Some([low, high]) = reference.mapping {
            fields.push((
                "low".into(),
                TheValue::Float(low),
                Some(-1000.0..=1000.0),
                layout.input_control(5),
            ));
            fields.push((
                "high".into(),
                TheValue::Float(high),
                Some(-1000.0..=1000.0),
                layout.input_control(6),
            ));
        }
        let same = self.number_edits.len() == fields.len()
            && self
                .number_edits
                .iter()
                .zip(&fields)
                .all(|((o, p, _), (name, _, _, _))| o == &owner && p == name);
        if !same {
            self.number_focus = None;
            self.number_edits = fields
                .iter()
                .map(|(name, value, range, _)| {
                    let mut edit = TheTextLineEdit::new(TheId::named(&format!(
                        "Procedural Input {owner}/{name}"
                    )));
                    edit.set_embedded(true);
                    edit.set_font_size(11.);
                    edit.set_status_text(if name=="components" {
                        "Source channels: x, y, z, w, xy, xyz, xyzw or distance. A single channel expands to fit the input."
                    } else {"Remap the source's 0–1 range to these endpoints. Drag or type a value and press Return."});
                    if let Some(range) = range {
                        edit.set_range(TheValue::RangeF32(range.clone()));
                    }
                    edit.set_value(value.clone());
                    (owner.clone(), name.clone(), edit)
                })
                .collect();
        }
        for (index, ((_, _, _, area), (_, _, edit))) in
            fields.iter().zip(&mut self.number_edits).enumerate()
        {
            edit.set_parent_id(if self.number_focus == Some(index) {
                self.id.clone()
            } else {
                TheId::empty()
            });
            edit.set_dim(
                TheDim::rect(
                    (area.origin[0] + area.size[0] * 0.45) as i32,
                    area.origin[1] as i32,
                    (area.size[0] * 0.55) as i32,
                    area.size[1] as i32,
                ),
                ctx,
            );
        }
    }
    fn palette_columns(&self) -> usize {
        ((self.dim.width as f32 - 20.) / 30.).floor().max(1.) as usize
    }
    fn palette_rect(&self, index: usize) -> GraphRect {
        let columns = self.palette_columns();
        rect(
            10. + (index % columns) as f32 * 30.,
            self.layout().editor_bottom + 54. + (index / columns) as f32 * 30.,
            26.,
            26.,
        )
    }
    fn popup_area(&self) -> Option<GraphRect> {
        let control = self.input_popup?;
        let l = self.layout();
        let count = self.input_options(control).len().min(8);
        Some(rect(
            10.,
            ((if control == 8 {
                l.category()
            } else if control >= 1000 {
                l.parameter((control - 1000).saturating_sub(self.parameter_scroll))
            } else {
                l.input_control(control)
            })
            .origin[1]
                + 24.)
                .min(l.height - count as f32 * 22. - 6.)
                .max(l.editor_bottom + 2.),
            l.width - 20.,
            count as f32 * 22.,
        ))
    }
    fn connection_paths(&self, l: &Layout) -> Vec<ConnectionPath> {
        let mut wires = vec![];
        for output in &self.view.outputs {
            if let Some(source) = output
                .row
                .as_ref()
                .and_then(|id| self.view.rows.iter().position(|r| &r.id == id))
            {
                let index = self
                    .view
                    .outputs
                    .iter()
                    .position(|o| o.id == output.id)
                    .unwrap();
                wires.push((source, None, center(l.output(index))));
            }
        }
        for (target, row) in self.view.rows.iter().enumerate() {
            for input in &row.inputs {
                if let MaterialInputBinding::Row(reference) = &input.binding {
                    if let Some(source) = self.view.rows.iter().position(|r| r.id == reference.row)
                    {
                        wires.push((source, Some(target), center(l.row_input(target, self.pan))));
                    }
                }
            }
        }
        let count = wires.len().max(1);
        wires
            .into_iter()
            .enumerate()
            .map(|(index, (source, target, end))| {
                let start = center(l.row_port(source, self.pan));
                let row = l.row(source, self.pan);
                let y = row.origin[1] + row.size[1] - 3. - (index % 3) as f32 * 1.25;
                let lane = RAIL + 4. + index as f32 * (GUTTER - 10.) / count as f32;
                let tail = if target.is_some() {
                    end
                } else {
                    [end[0], end[1]]
                };
                ConnectionPath {
                    source,
                    target,
                    points: vec![start, [start[0], y], [lane, y], [lane, tail[1]], tail],
                }
            })
            .collect()
    }
    fn connection_selected(&self, path: &ConnectionPath) -> bool {
        self.selected_row.as_ref().is_some_and(|selected| {
            self.view.rows[path.source].id == *selected
                || path
                    .target
                    .is_some_and(|target| self.view.rows[target].id == *selected)
        })
    }
    fn connection_style(&self, path: &ConnectionPath, theme: &GraphTheme) -> (RGBA, f32) {
        if self.connection_selected(path) {
            return (theme.active, 1.8);
        }
        if self.selected_row.is_none() {
            return (theme.wire, 0.8);
        }
        let color = std::array::from_fn(|i| {
            if i == 3 {
                255
            } else {
                (theme.wire[i] as f32 * 0.8 + theme.muted[i] as f32 * 0.2) as u8
            }
        });
        (color, 1.)
    }
    fn focus_connection_source(&mut self, row: &str) {
        let Some(index) = self.view.rows.iter().position(|r| r.id == row) else {
            return;
        };
        let first = self.view.rows[index]
            .operations
            .first()
            .map(|op| op.id.clone());
        self.selected_row = Some(row.to_owned());
        self.selected = first.clone();
        self.selected_input = 0;
        self.parameter_scroll = 0;
        self.parameter_fraction = 0.;
        self.input_popup = None;
        self.number_focus = None;
        self.number_edits.clear();
        self.source_edit = None;
        self.source_focus = false;
        self.color_property = None;
        self.pane = if first.is_some() {
            MaterialPane::Properties
        } else {
            MaterialPane::RowInput
        };
        let l = self.layout();
        let card = l.card(index, 0, self.pan);
        if card.origin[1] < HEADER || card.origin[1] + card.size[1] > l.editor_bottom {
            self.pan[1] = l.rows[index].0 - 8.;
        }
        self.clamp_scroll();
        self.commands.push(
            first
                .map(MaterialEditorCommand::InspectOperation)
                .unwrap_or_else(|| MaterialEditorCommand::InspectRow(row.to_owned())),
        );
    }
    pub fn set_thumbnail(&mut self, key: String, image: TheRGBABuffer) {
        self.thumbnails.insert(key, image);
        self.dirty = true;
    }
    pub fn take_commands(&mut self) -> Vec<MaterialEditorCommand> {
        std::mem::take(&mut self.commands)
    }
    pub fn number_command(&self, id: &TheId, value: &TheValue) -> Option<MaterialEditorCommand> {
        let (operation, parameter, _) = self
            .number_edits
            .iter()
            .find(|(_, _, edit)| edit.id() == id)?;
        if operation.starts_with("@row-input:") {
            let input = self.active_input()?;
            if Self::input_owner(input) != *operation {
                return None;
            }
            let MaterialInputBinding::Row(mut reference) = input.binding.clone() else {
                return None;
            };
            match parameter.as_str() {
                "components" => reference.components = value.to_string()?.trim().to_owned(),
                "low" => reference.mapping.as_mut()?[0] = value.to_f32().unwrap_or(f32::NAN),
                "high" => reference.mapping.as_mut()?[1] = value.to_f32().unwrap_or(f32::NAN),
                _ => return None,
            }
            return Some(MaterialEditorCommand::SetInput {
                target: input.target.clone(),
                binding: MaterialInputBinding::Row(reference),
            });
        }
        Some(self.property_number_command(operation, parameter, value.to_f32().unwrap_or(f32::NAN)))
    }
    fn sync_number_edits(&mut self, ctx: &mut TheContext) {
        if self.pane == MaterialPane::RowInput {
            self.sync_input_edits(ctx);
            return;
        }
        if self.pane != MaterialPane::Properties || self.active_palette().is_some() {
            self.number_edits.clear();
            self.number_focus = None;
            return;
        }
        let layout = self.layout();
        let mut parameters = Vec::new();
        for (i, line) in self
            .property_lines()
            .iter()
            .skip(self.parameter_scroll)
            .enumerate()
        {
            let area = layout.parameter(i);
            if area.origin[1] + area.size[1] > layout.properties_bottom {
                break;
            }
            if let PropertyLine::Values {
                owner,
                values,
                color: None,
                ..
            } = line
            {
                let width = area.size[0] * 0.55 / values.len().max(1) as f32;
                for (component, p) in values.iter().enumerate() {
                    parameters.push((
                        owner.clone(),
                        p.clone(),
                        rect(
                            area.origin[0] + area.size[0] * 0.45 + component as f32 * width,
                            area.origin[1],
                            width - 3.,
                            area.size[1],
                        ),
                    ));
                }
            }
        }
        let unchanged = parameters.len() == self.number_edits.len()
            && parameters.iter().zip(&self.number_edits).all(
                |((owner, p, _), (old_owner, old_id, _))| owner == old_owner && &p.id == old_id,
            );
        if !unchanged {
            self.number_focus = None;
            self.number_edits = parameters
                .iter()
                .map(|(owner, p, _)| {
                    let mut edit = TheTextLineEdit::new(TheId::named(&format!(
                        "Procedural Parameter {owner}/{}",
                        p.id
                    )));
                    edit.set_embedded(true);
                    edit.set_range(TheValue::RangeF32(p.min..=p.max));
                    edit.set_value(TheValue::Float(p.value));
                    edit.set_font_size(11.);
                    (owner.clone(), p.id.clone(), edit)
                })
                .collect();
        }
        for (index, ((_, _, area), (_, _, edit))) in
            parameters.iter().zip(&mut self.number_edits).enumerate()
        {
            edit.set_parent_id(if self.number_focus == Some(index) {
                self.id.clone()
            } else {
                TheId::empty()
            });
            let x = area.origin[0] as i32;
            let y = area.origin[1] as i32;
            let mut dim = TheDim::new(x, y, area.size[0] as i32, area.size[1] as i32);
            dim.buffer_x = x;
            dim.buffer_y = y;
            edit.set_dim(dim, ctx);
        }
    }
    fn number_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        if self.input_popup.is_some() {
            return false;
        }
        if let TheEvent::MouseDown(p) = event {
            self.sync_number_edits(ctx);
            let input_number = if self.pane == MaterialPane::RowInput {
                self.number_edits
                    .iter()
                    .position(|(_, _, edit)| edit.dim().contains(*p))
            } else {
                None
            };
            if let Some(index) = input_number {
                self.number_focus = Some(index);
                self.number_drag = true;
                ctx.ui.set_focus(&self.id);
            } else if let Some(Hit::Number(owner, parameter)) = self.hit([p.x as f32, p.y as f32]) {
                self.number_focus = self
                    .number_edits
                    .iter()
                    .position(|(o, p, _)| *o == owner && *p == parameter);
                self.number_drag = true;
                ctx.ui.set_focus(&self.id);
            } else {
                self.number_focus = None;
                self.number_drag = false;
                return false;
            }
        }
        let Some(index) = self.number_focus else {
            return false;
        };
        let Some((_, _, edit)) = self.number_edits.get_mut(index) else {
            return false;
        };
        let local = |p: &Vec2<i32>| Vec2::new(p.x - edit.dim().x, p.y - edit.dim().y);
        let event = match event {
            TheEvent::MouseDown(p) => TheEvent::MouseDown(local(p)),
            TheEvent::MouseDragged(p) if self.number_drag => TheEvent::MouseDragged(local(p)),
            TheEvent::MouseUp(p) if self.number_drag => {
                self.number_drag = false;
                TheEvent::MouseUp(local(p))
            }
            TheEvent::KeyDown(_)
            | TheEvent::KeyCodeDown(_)
            | TheEvent::ModifierChanged(..)
            | TheEvent::Paste(..)
            | TheEvent::Copy
            | TheEvent::Cut
            | TheEvent::Undo
            | TheEvent::Redo
            | TheEvent::LostFocus(_) => event.clone(),
            _ => return false,
        };
        edit.on_event(&event, ctx);
        self.dirty = true;
        true
    }
    pub fn selected_operation(&self) -> Option<&str> {
        self.selected.as_deref()
    }
    pub fn selected_source(&self) -> Option<&str> {
        if self.pane == MaterialPane::Sources {
            self.selected_source.as_deref()
        } else {
            None
        }
    }
    pub fn set_source_filter(&mut self, query: &str) {
        self.query = query.to_owned();
        self.library_scroll = 0;
        self.library_fraction = 0.;
        self.dirty = true;
    }
    fn selected(&self) -> Option<&MaterialOperationView> {
        self.view
            .rows
            .iter()
            .flat_map(|row| &row.operations)
            .find(|op| Some(&op.id) == self.selected.as_ref())
    }
    fn active_palette(&self) -> Option<(String, String, Option<u16>)> {
        let (owner, parameter) = if let Some(index) = self.color_property {
            match self.property_lines().get(index)? {
                PropertyLine::Values {
                    owner,
                    color: Some(parameter),
                    ..
                } => (owner.clone(), parameter.clone()),
                _ => return None,
            }
        } else {
            let op = self.selected()?;
            op.palette.as_ref()?;
            (op.id.clone(), "color".into())
        };
        let index = self
            .view
            .palette_bindings
            .iter()
            .find(|b| b.operation == owner && b.parameter == parameter)
            .map(|b| b.index)
            .or_else(|| {
                self.selected()
                    .filter(|op| op.id == owner)
                    .and_then(|op| op.palette.as_ref())
                    .and_then(|p| p.index)
            });
        Some((owner, parameter, index))
    }
    fn parameters(&self) -> (&str, &str, &[MaterialNumberView]) {
        self.selected()
            .map(|op| {
                (
                    op.id.as_str(),
                    op.label.as_str(),
                    if op.palette.is_some() {
                        &[][..]
                    } else {
                        op.parameters.as_slice()
                    },
                )
            })
            .unwrap_or(("@surface", "Material", &self.view.defaults))
    }
    fn property_lines(&self) -> Vec<PropertyLine> {
        let mut lines = Vec::new();
        let (owner, _, numbers) = self.parameters();
        let value = |p: &MaterialNumberView| PropertyLine::Values {
            owner: owner.into(),
            label: p.label.clone(),
            values: vec![p.clone()],
            color: None,
            tooltip: String::new(),
        };
        if let Some(op) = self.selected() {
            if op.palette.is_some() {
                return lines;
            }
            if let Some(mode) = &op.combine {
                let modes = ["replace", "union", "subtract", "intersect", "smooth union"];
                lines.push(PropertyLine::Choice {
                    owner: op.id.clone(),
                    parameter: "@combine".into(),
                    label: "Combine".into(),
                    value: modes
                        .iter()
                        .position(|m| *m == mode.replace('_', " "))
                        .unwrap_or(0),
                    choices: ["Replace", "Union", "Subtract", "Intersect", "Smooth Union"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    tooltip: "How this shape combines with the incoming shape.".into(),
                });
            }
            if op.properties.is_empty() {
                lines.extend(numbers.iter().map(value));
            }
            let mut group = String::new();
            for property in &op.properties {
                if !property.group.is_empty() && group != property.group {
                    group = property.group.clone();
                    lines.push(PropertyLine::Section(group.clone()));
                }
                let components: Vec<_> = property
                    .components
                    .iter()
                    .filter_map(|id| numbers.iter().find(|n| &n.id == id).cloned())
                    .collect();
                let Some(first) = components.first() else {
                    continue;
                };
                if let Some(input)=self.input_row().and_then(|row|row.inputs.iter().find(|input|matches!(&input.target,MaterialInputTarget::Parameter{operation,parameter} if operation==&op.id && parameter==&property.id) && matches!(input.binding,MaterialInputBinding::Row(_)))) {
                    if let MaterialInputBinding::Row(reference)=&input.binding {
                        let source=self.view.rows.iter().find(|r|r.id==reference.row).map_or(reference.row.clone(),|r|r.label.clone());
                        lines.push(PropertyLine::Bound {label:property.label.clone(),source,target:input.target.clone()});continue;
                    }
                }
                match property.kind {
                    MaterialPropertyKind::Choice => lines.push(PropertyLine::Choice {
                        owner: owner.into(),
                        parameter: first.id.clone(),
                        label: property.label.clone(),
                        value: first.value.round().max(0.) as usize,
                        choices: property.choices.clone(),
                        tooltip: property.tooltip.clone(),
                    }),
                    MaterialPropertyKind::Boolean => lines.push(PropertyLine::Boolean {
                        owner: owner.into(),
                        parameter: first.id.clone(),
                        label: property.label.clone(),
                        value: first.value != 0.,
                        tooltip: property.tooltip.clone(),
                    }),
                    _ => lines.push(PropertyLine::Values {
                        owner: owner.into(),
                        label: property.label.clone(),
                        values: components,
                        color: (property.kind == MaterialPropertyKind::Color)
                            .then(|| property.id.clone()),
                        tooltip: property.tooltip.clone(),
                    }),
                }
            }
        } else {
            for p in numbers
                .iter()
                .filter(|p| !p.id.starts_with("base color:") && !p.id.starts_with("emission:"))
            {
                lines.push(value(p));
            }
            for name in ["base color", "emission"].into_iter().filter(|name| {
                numbers
                    .iter()
                    .any(|p| p.id.starts_with(&format!("{name}:")))
            }) {
                lines.push(PropertyLine::Values {
                    owner: "@surface".into(),
                    label: name.into(),
                    values: numbers
                        .iter()
                        .filter(|p| p.id.starts_with(&format!("{name}:")))
                        .cloned()
                        .collect(),
                    color: Some(name.into()),
                    tooltip: "Choose a project palette color".into(),
                });
            }
            lines.push(PropertyLine::Section("Tiling".into()));
            let modes = ["raw", "repeat", "seamless"];
            lines.push(PropertyLine::Choice {owner:"@tiling".into(),parameter:"mode".into(),label:"Edges".into(),value:modes.iter().position(|m| *m==self.view.tiling.mode).unwrap_or(0),choices:vec!["Continuous".into(),"Repeat".into(),"Seamless".into()], tooltip: "Raw uses continuous coordinates. Repeat wraps the pattern. Seamless blends shifted copies at its edges.".into()});
            if self.view.tiling.mode != "raw" {
                lines.push(PropertyLine::Values {
                    owner: "@tiling".into(),
                    label: "Period X / Y".into(),
                    values: (0..2)
                        .map(|i| MaterialNumberView {
                            id: format!("period{i}"),
                            label: ["X", "Y"][i].into(),
                            value: self.view.tiling.period[i],
                            min: 0.0001,
                            max: 1_000_000.,
                        })
                        .collect(),
                    color: None,
                    tooltip: "Pattern period along each UV axis.".into(),
                });
            }
            if self.view.tiling.mode == "seamless" {
                lines.push(PropertyLine::Values {
                    owner: "@tiling".into(),
                    label: "Edge Blend".into(),
                    values: vec![MaterialNumberView {
                        id: "blend".into(),
                        label: "Edge Blend".into(),
                        value: self.view.tiling.blend,
                        min: 0.01,
                        max: 0.5,
                    }],
                    color: None,
                    tooltip: "Width of the seamless edge blend.".into(),
                });
            }
            lines.push(PropertyLine::Section("Animation".into()));
            lines.push(PropertyLine::Values {
                owner: "@animation".into(), label: "Loop Frames".into(),
                values: vec![
                    MaterialNumberView { id:"frames".into(), label:"Frames".into(), value:self.view.animation.frames as f32, min:1., max:32. },
                ], color:None, tooltip:"One frame is static. Animated tiles advance one frame per server tick, like normal tiles. At the default 250 ms tick, 16 frames take four seconds.".into(),
            });
        }
        lines
    }
    fn property_number_command(
        &self,
        owner: &str,
        parameter: &str,
        value: f32,
    ) -> MaterialEditorCommand {
        if owner == "@animation" {
            MaterialEditorCommand::SetAnimation {
                frames: if parameter == "frames" {
                    value.round().clamp(1., 32.) as u32
                } else {
                    self.view.animation.frames
                },
                duration: if parameter == "duration" {
                    value
                } else {
                    self.view.animation.duration
                },
            }
        } else if owner == "@tiling" {
            let mut tiling = self.view.tiling.clone();
            match parameter {
                "mode" => {
                    tiling.mode =
                        ["raw", "repeat", "seamless"][value.round().clamp(0., 2.) as usize].into()
                }
                "period0" => tiling.period[0] = value,
                "period1" => tiling.period[1] = value,
                "blend" => tiling.blend = value,
                _ => {}
            }
            MaterialEditorCommand::SetTiling {
                mode: tiling.mode,
                period: tiling.period,
                blend: tiling.blend,
            }
        } else if parameter == "@combine" {
            MaterialEditorCommand::SetCombine {
                operation: owner.into(),
                mode: ["replace", "union", "subtract", "intersect", "smooth union"]
                    [value.round().clamp(0., 4.) as usize]
                    .into(),
            }
        } else {
            MaterialEditorCommand::SetNumber {
                operation: owner.into(),
                parameter: parameter.into(),
                value,
            }
        }
    }
    fn property_options(&self, index: usize) -> Vec<String> {
        match self.property_lines().get(index) {
            Some(PropertyLine::Choice { choices, .. }) => choices.clone(),
            Some(PropertyLine::Values { color: Some(_), .. }) => self
                .view
                .palette
                .iter()
                .map(|entry| format!("{} · {}", entry.index, entry.label))
                .collect(),
            _ => vec![],
        }
    }
    fn layout(&self) -> Layout {
        Layout::new(
            [self.dim.width as f32, self.dim.height as f32],
            self.parameters().2.len(),
            &self.view.rows,
        )
    }
    fn sources(&self) -> Vec<&MaterialSourceView> {
        let query = self.query.to_lowercase();
        self.view
            .sources
            .iter()
            .filter(|s| {
                self.source_category
                    .as_ref()
                    .is_none_or(|category| category == &s.category)
                    && (s.label.to_lowercase().contains(&query)
                        || s.category.to_lowercase().contains(&query))
            })
            .collect()
    }
    fn clamp_scroll(&mut self) {
        let layout = self.layout();
        let count = ((layout.properties_bottom - layout.editor_bottom - 52.) / PARAMETER_STEP)
            .floor()
            .max(1.) as usize;
        self.parameter_scroll = self
            .parameter_scroll
            .min(self.property_lines().len().saturating_sub(count));
        let visible = ((layout.height - layout.library_top) / SOURCE_STEP)
            .floor()
            .max(1.) as usize;
        self.library_scroll = self
            .library_scroll
            .min(self.sources().len().saturating_sub(visible));
        self.pan[1] = self.pan[1].clamp(0., (layout.content_bottom - layout.editor_bottom).max(0.));
        self.pan[0] = 0.;
        if self.library_scroll == 0 {
            self.library_fraction = self.library_fraction.max(0.);
        }
        if self.parameter_scroll == 0 {
            self.parameter_fraction = self.parameter_fraction.max(0.);
        }
    }
    fn hit(&self, point: [f32; 2]) -> Option<Hit> {
        let l = self.layout();
        if !rect(0., 0., l.width, l.height).contains(point) {
            return None;
        }
        if let Some(area) = self.popup_area() {
            if area.contains(point) {
                return Some(Hit::PopupOption(
                    ((point[1] - area.origin[1]) / 22.).floor() as usize + self.popup_scroll,
                ));
            }
        }
        for (index, pane) in [
            MaterialPane::Properties,
            MaterialPane::Sources,
            MaterialPane::RowInput,
            MaterialPane::Tile,
        ]
        .into_iter()
        .enumerate()
        {
            if l.pane_tab(index).contains(point) {
                return Some(Hit::Pane(pane));
            }
        }
        if self.pane == MaterialPane::Properties
            && rect(0., l.editor_bottom + 26., l.width, 24.).contains(point)
        {
            return Some(Hit::Surface);
        }
        if point[1] >= HEADER && point[1] < l.editor_bottom {
            for (i, output) in self.view.outputs.iter().enumerate() {
                let port = l.output(i);
                if rect(0., port.origin[1] - 4., RAIL, 22.).contains(point) {
                    return Some(Hit::Output(output.id.clone()));
                }
            }
            if point[0] > RAIL {
                for (i, row) in self.view.rows.iter().enumerate() {
                    if l.row_input(i, self.pan).contains(point) {
                        return Some(Hit::RowInput(row.id.clone()));
                    }
                    if l.row_port(i, self.pan).contains(point) {
                        return Some(Hit::RowWire(row.id.clone()));
                    }
                    for (j, op) in row.operations.iter().enumerate() {
                        if l.card(i, j, self.pan).contains(point) {
                            return Some(Hit::Operation(op.id.clone()));
                        }
                    }
                }
                if let Some(path) = self
                    .connection_paths(&l)
                    .into_iter()
                    .filter(|path| path.distance(point, l.editor_bottom) <= 9.)
                    .min_by(|a, b| {
                        a.distance(point, l.editor_bottom)
                            .total_cmp(&b.distance(point, l.editor_bottom))
                            .then_with(|| {
                                self.connection_selected(b)
                                    .cmp(&self.connection_selected(a))
                            })
                    })
                {
                    return Some(Hit::Connection(self.view.rows[path.source].id.clone()));
                }
                for (i, row) in self.view.rows.iter().enumerate() {
                    if l.row(i, self.pan).contains(point) {
                        return Some(Hit::Drop {
                            row: Some(row.id.clone()),
                            after: row.operations.last().map(|op| op.id.clone()),
                        });
                    }
                }
                if l.new_row(self.view.rows.len(), self.pan).contains(point) {
                    return Some(Hit::Drop {
                        row: None,
                        after: None,
                    });
                }
            }
        }
        if self.pane == MaterialPane::RowInput {
            for control in 0..8 {
                if control == 7
                    && !matches!(
                        self.active_input().map(|i| &i.target),
                        Some(MaterialInputTarget::RowLink { .. })
                    )
                {
                    continue;
                }
                if l.input_control(control).contains(point) {
                    return Some(Hit::InputControl(control));
                }
            }
        }
        if self.pane == MaterialPane::Properties {
            if self.active_palette().is_some() {
                let columns = self.palette_columns();
                for (i, _) in self
                    .view
                    .palette
                    .iter()
                    .skip(self.palette_scroll * columns)
                    .enumerate()
                {
                    if self.palette_rect(i).contains(point) {
                        return Some(Hit::Palette(i + self.palette_scroll * columns));
                    }
                }
                return None;
            }
            for (i, line) in self
                .property_lines()
                .iter()
                .skip(self.parameter_scroll)
                .enumerate()
            {
                let area = l.parameter(i);
                if area.contains(point) && point[1] < l.properties_bottom {
                    return match line {
                        PropertyLine::Section(_) => None,
                        PropertyLine::Values {
                            owner,
                            values,
                            color,
                            ..
                        } => {
                            if color.is_some() && point[0] < area.origin[0] + area.size[0] * 0.45 {
                                Some(Hit::Property(i + self.parameter_scroll))
                            } else {
                                let component =
                                    (((point[0] - area.origin[0] - area.size[0] * 0.45)
                                        / (area.size[0] * 0.55))
                                        * values.len() as f32)
                                        .floor()
                                        .max(0.) as usize;
                                values
                                    .get(component.min(values.len().saturating_sub(1)))
                                    .map(|p| Hit::Number(owner.clone(), p.id.clone()))
                            }
                        }
                        _ => Some(Hit::Property(i + self.parameter_scroll)),
                    };
                }
            }
        }
        if self.pane == MaterialPane::Tile {
            for action in 0..4 {
                if self.source_action_rect(action).contains(point) {
                    return Some(Hit::SourceAction(action));
                }
            }
        }
        if self.pane == MaterialPane::Sources && l.category().contains(point) {
            return Some(Hit::SourceCategory);
        }
        if self.pane == MaterialPane::Sources && l.search().contains(point) {
            return Some(Hit::Search);
        }
        if self.pane == MaterialPane::Sources && point[1] >= l.library_top {
            let index =
                ((point[1] - l.library_top) / SOURCE_STEP).floor() as usize + self.library_scroll;
            return self.sources().get(index).map(|s| Hit::Source(s.id.clone()));
        }
        None
    }
    pub fn press(&mut self, point: [f32; 2]) {
        self.pointer = point;
        self.search_focused = false;
        self.gesture = None;
        let hit = self.hit(point);
        if self.input_popup.is_some() && !matches!(hit, Some(Hit::PopupOption(_))) {
            self.input_popup = None;
        }
        match hit {
            Some(Hit::PopupOption(index)) => {
                if let Some(control) = self.input_popup {
                    self.choose_input_option(control, index);
                }
            }
            Some(Hit::RowInput(row)) => {
                self.selected_row = Some(row.clone());
                self.selected_input = 0;
                self.pane = MaterialPane::RowInput;
                self.number_edits.clear();
                self.number_focus = None;
                self.commands.push(MaterialEditorCommand::InspectRow(row));
                self.inspect_active_input();
            }
            Some(Hit::InputControl(control)) => {
                if control < 3 || control == 3 || control == 7 {
                    self.input_popup = Some(control);
                    self.popup_scroll = 0;
                } else if control == 4 {
                    if let Some(input) = self.active_input() {
                        if let MaterialInputBinding::Row(mut reference) = input.binding.clone() {
                            reference.mapping = if reference.mapping.is_some() {
                                None
                            } else {
                                Some([0., 1.])
                            };
                            self.commands.push(MaterialEditorCommand::SetInput {
                                target: input.target.clone(),
                                binding: MaterialInputBinding::Row(reference),
                            });
                        }
                    }
                }
            }
            Some(Hit::Property(index)) => match self.property_lines().get(index) {
                Some(PropertyLine::Bound { target, .. }) => {
                    if let Some(index) = self
                        .input_row()
                        .and_then(|r| r.inputs.iter().position(|i| &i.target == target))
                    {
                        self.selected_input = index;
                        self.pane = MaterialPane::RowInput;
                        self.inspect_active_input();
                    }
                }
                Some(PropertyLine::Boolean {
                    owner,
                    parameter,
                    value,
                    ..
                }) => self.commands.push(self.property_number_command(
                    owner,
                    parameter,
                    if *value { 0. } else { 1. },
                )),
                Some(PropertyLine::Values { color: Some(_), .. }) => {
                    self.color_property = Some(index);
                    self.input_popup = None;
                }
                Some(PropertyLine::Choice { .. }) => {
                    self.input_popup = Some(1000 + index);
                    self.popup_scroll = 0;
                }
                _ => {}
            },
            Some(Hit::SourceAction(action)) => self.source_action(action),
            Some(Hit::Palette(index)) => {
                if let (Some((operation, parameter, _)), Some(entry)) =
                    (self.active_palette(), self.view.palette.get(index))
                {
                    self.commands.push(if parameter == "color" {
                        MaterialEditorCommand::SetPalette {
                            operation,
                            index: entry.index,
                        }
                    } else {
                        MaterialEditorCommand::SetPaletteParameter {
                            operation,
                            parameter,
                            index: entry.index,
                        }
                    });
                    self.color_property = None;
                }
            }
            Some(Hit::Pane(pane)) => {
                self.color_property = None;
                self.pane = pane;
                if pane == MaterialPane::Properties {
                    self.commands.push(
                        self.selected
                            .as_ref()
                            .map(|id| MaterialEditorCommand::InspectOperation(id.clone()))
                            .unwrap_or(MaterialEditorCommand::InspectSurface),
                    );
                } else if pane == MaterialPane::RowInput {
                    if let Some(row) = &self.selected_row {
                        self.commands
                            .push(MaterialEditorCommand::InspectRow(row.clone()));
                    }
                    self.inspect_active_input();
                } else if pane == MaterialPane::Sources
                    && let Some(source) = &self.selected_source
                {
                    self.commands
                        .push(MaterialEditorCommand::InspectSource(source.clone()));
                }
                self.clamp_scroll();
            }
            Some(Hit::Surface) => {
                self.commands.push(MaterialEditorCommand::InspectSurface);
                self.selected = None;
                self.selected_row = None;
                self.parameter_scroll = 0;
            }
            Some(Hit::SourceCategory) => {
                self.input_popup = Some(8);
                self.popup_scroll = 0;
            }
            Some(Hit::Search) => self.search_focused = true,
            Some(Hit::Source(id)) => {
                self.gesture = Some(Gesture::Source {
                    id,
                    start: point,
                    point,
                })
            }
            Some(Hit::Operation(id)) => {
                self.pane = MaterialPane::Properties;
                if self.selected.as_ref() != Some(&id) {
                    self.parameter_scroll = 0;
                }
                self.selected = Some(id.clone());
                self.selected_row = self
                    .view
                    .rows
                    .iter()
                    .find(|r| r.operations.iter().any(|op| op.id == id))
                    .map(|r| r.id.clone());
                self.selected_input = 0;
                self.commands
                    .push(MaterialEditorCommand::InspectOperation(id.clone()));
                self.gesture = Some(Gesture::Operation {
                    id,
                    start: point,
                    point,
                });
            }
            Some(Hit::Connection(row)) => {
                self.gesture = Some(Gesture::Connection { row, start: point })
            }
            Some(Hit::RowWire(row)) => self.gesture = Some(Gesture::RowWire { row, point }),
            Some(Hit::Output(output)) => {
                self.gesture = Some(Gesture::OutputWire {
                    output,
                    start: point,
                    point,
                })
            }
            Some(Hit::Number(operation, parameter)) => {
                let numbers: Vec<_> = self
                    .property_lines()
                    .into_iter()
                    .flat_map(|line| match line {
                        PropertyLine::Values { values, .. } => values,
                        _ => vec![],
                    })
                    .collect();
                if let Some(p) = numbers.iter().find(|p| p.id == parameter) {
                    if p.value.is_finite()
                        && p.min.is_finite()
                        && p.max.is_finite()
                        && p.max > p.min
                    {
                        self.gesture = Some(Gesture::Number {
                            operation,
                            parameter,
                            original: p.value,
                            value: p.value,
                            min: p.min,
                            max: p.max,
                            start: point,
                        });
                    }
                }
            }
            _ => {}
        }
        self.dirty = true;
    }
    pub fn drag(&mut self, point: [f32; 2]) {
        if !point.iter().all(|v| v.is_finite()) {
            return;
        }
        self.pointer = point;
        match &mut self.gesture {
            Some(
                Gesture::Source { point: p, .. }
                | Gesture::Operation { point: p, .. }
                | Gesture::RowWire { point: p, .. }
                | Gesture::OutputWire { point: p, .. },
            ) => *p = point,
            Some(Gesture::Number {
                original,
                value,
                min,
                max,
                start,
                ..
            }) => {
                *value =
                    (*original + (point[0] - start[0]) / 200. * (*max - *min)).clamp(*min, *max);
            }
            _ => {}
        }
        self.dirty = true;
    }
    pub fn release(&mut self, point: [f32; 2]) {
        self.drag(point);
        let hit = self.hit(point);
        let destination = match &hit {
            Some(Hit::Drop { row, after }) => Some((row.clone(), after.clone())),
            Some(Hit::Operation(id)) => self
                .view
                .rows
                .iter()
                .find(|r| r.operations.iter().any(|op| &op.id == id))
                .map(|r| (Some(r.id.clone()), Some(id.clone()))),
            _ => None,
        };
        match self.gesture.take() {
            Some(Gesture::Connection { row, start })
                if (point[0] - start[0]).abs() + (point[1] - start[1]).abs() <= 4. =>
            {
                self.focus_connection_source(&row);
            }
            Some(Gesture::Source { id, start, .. }) => {
                if let Some((row, after)) = destination {
                    self.commands.push(MaterialEditorCommand::AddSource {
                        source: id,
                        row,
                        after,
                    });
                } else if matches!(hit,Some(Hit::Source(ref source)) if source==&id)
                    && (point[0] - start[0]).abs() + (point[1] - start[1]).abs() <= 4.
                {
                    self.selected_source = Some(id.clone());
                    self.commands.push(MaterialEditorCommand::InspectSource(id));
                }
            }
            Some(Gesture::Operation { id, start, .. })
                if (point[0] - start[0]).abs() + (point[1] - start[1]).abs() > 6. =>
            {
                if let Some((Some(row), after)) = destination {
                    if after.as_ref() != Some(&id) {
                        self.commands.push(MaterialEditorCommand::MoveOperation {
                            operation: id,
                            row,
                            after,
                        });
                    }
                }
            }
            Some(Gesture::RowWire { row, .. }) => {
                if let Some(Hit::RowInput(target_row)) = &hit {
                    if let Some(target) = self
                        .view
                        .rows
                        .iter()
                        .find(|r| &r.id == target_row)
                        .and_then(|r| {
                            r.inputs
                                .iter()
                                .find(|i| matches!(i.target, MaterialInputTarget::Image { .. }))
                                .or_else(|| r.inputs.first())
                        })
                    {
                        self.commands.push(MaterialEditorCommand::SetInput {
                            target: target.target.clone(),
                            binding: MaterialInputBinding::Row(MaterialReferenceView {
                                row: row.clone(),
                                components: target.components.clone(),
                                mapping: None,
                            }),
                        });
                        let target_id = target.target.clone();
                        self.selected_row = Some(target_row.clone());
                        self.selected_input = self
                            .view
                            .rows
                            .iter()
                            .find(|r| &r.id == target_row)
                            .and_then(|r| r.inputs.iter().position(|i| i.target == target_id))
                            .unwrap_or(0);
                        self.pane = MaterialPane::RowInput;
                    }
                }
                if let Some(Hit::Output(output)) = hit {
                    self.commands
                        .push(MaterialEditorCommand::ConnectOutput { output, row });
                }
            }
            Some(Gesture::OutputWire { output, start, .. }) => {
                let row = match &hit {
                    Some(Hit::RowWire(row)) => Some(row.clone()),
                    _ => destination.as_ref().and_then(|(row, _)| row.clone()),
                };
                if let Some(row) = row {
                    self.commands
                        .push(MaterialEditorCommand::ConnectOutput { output, row });
                } else if matches!(hit,Some(Hit::Output(ref id)) if id==&output)
                    && (point[0] - start[0]).abs() + (point[1] - start[1]).abs() <= 4.
                    && self
                        .view
                        .outputs
                        .iter()
                        .any(|port| port.id == output && port.row.is_some())
                {
                    self.commands
                        .push(MaterialEditorCommand::DisconnectOutput(output));
                }
            }
            Some(Gesture::Number {
                operation,
                parameter,
                original,
                value,
                ..
            }) if value != original => self
                .commands
                .push(self.property_number_command(&operation, &parameter, value)),
            _ => {}
        }
        self.dirty = true;
    }
    pub fn cancel_gesture(&mut self) {
        self.gesture = None;
        self.dirty = true;
    }
    pub fn scroll(&mut self, point: [f32; 2], delta: [f32; 2]) {
        if !delta.iter().chain(point.iter()).all(|v| v.is_finite()) {
            return;
        }
        if let Some(control) = self.input_popup {
            let amount = self.library_fraction + delta[1] / 10.;
            let steps = amount.trunc();
            self.library_fraction = amount - steps;
            self.popup_scroll = self
                .popup_scroll
                .saturating_add_signed(steps as isize)
                .min(self.input_options(control).len().saturating_sub(8));
            self.dirty = true;
            return;
        }
        if self.pane == MaterialPane::Properties
            && self.active_palette().is_some()
            && point[1] > self.layout().editor_bottom + 26.
        {
            let amount = self.parameter_fraction + delta[1] / 10.;
            let steps = amount.trunc();
            self.parameter_fraction = amount - steps;
            let columns = self.palette_columns();
            let visible = ((self.layout().height - self.layout().editor_bottom - 54.) / 30.)
                .floor()
                .max(1.) as usize;
            self.palette_scroll = self
                .palette_scroll
                .saturating_add_signed(steps as isize)
                .min(
                    self.view
                        .palette
                        .len()
                        .div_ceil(columns)
                        .saturating_sub(visible),
                );
            self.dirty = true;
            return;
        }
        if self.pane == MaterialPane::Sources && point[1] >= self.layout().library_top {
            let amount = self.library_fraction + delta[1] / 10.;
            let steps = amount.trunc();
            self.library_fraction = amount - steps;
            self.library_scroll = self.library_scroll.saturating_add_signed(steps as isize);
        } else if self.pane == MaterialPane::Properties
            && point[1] >= self.layout().editor_bottom + 26.
            && point[1] < self.layout().properties_bottom
        {
            let amount = self.parameter_fraction + delta[1] / 10.;
            let steps = amount.trunc();
            self.parameter_fraction = amount - steps;
            self.parameter_scroll = self.parameter_scroll.saturating_add_signed(steps as isize);
        } else if point[1] >= HEADER && point[1] < self.layout().editor_bottom {
            self.pan[1] += delta[1];
        }
        self.clamp_scroll();
        self.dirty = true;
    }
    fn source_area(&self) -> GraphRect {
        let l = self.layout();
        rect(
            10.,
            l.editor_bottom + 52.,
            l.width - 20.,
            (l.height - l.editor_bottom - 88.).max(20.),
        )
    }
    fn source_action_rect(&self, index: usize) -> GraphRect {
        let l = self.layout();
        rect(
            10. + index as f32 * (l.width - 20.) / 4.,
            l.height - 30.,
            (l.width - 20.) / 4. - 4.,
            22.,
        )
    }
    fn source_action(&mut self, action: usize) {
        let Some(operation) = self.selected() else {
            return;
        };
        let id = operation.id.clone();
        match action {
            0 => {
                if let Some((owner, edit)) = &self.source_edit {
                    if owner == &id && !self.source_help {
                        self.commands.push(MaterialEditorCommand::SetSource {
                            operation: id,
                            code: edit.text(),
                        });
                    }
                }
            }
            1 => {
                self.source_edit = None;
                self.source_help = false;
            }
            2 => self.commands.push(MaterialEditorCommand::SetSource {
                operation: id,
                code: String::new(),
            }),
            3 => {
                self.source_help = !self.source_help;
                self.source_edit = None;
            }
            _ => {}
        }
        self.source_focus = false;
        self.dirty = true;
    }
    fn sync_source_edit(&mut self, ctx: &mut TheContext) {
        if self.pane != MaterialPane::Tile {
            self.source_focus = false;
            return;
        }
        let Some(operation) = self.selected() else {
            self.source_edit = None;
            return;
        };
        let Some(source) = &operation.source else {
            self.source_edit = None;
            return;
        };
        let owner = operation.id.clone();
        if !self
            .source_edit
            .as_ref()
            .is_some_and(|(id, _)| id == &owner)
        {
            let mut edit = TheTextAreaEdit::new(self.id.clone());
            TheTextAreaEditTrait::set_embedded(&mut edit, true);
            edit.set_font_size(11.);
            edit.set_continuous(false);
            edit.set_supports_undo(true);
            edit.use_statusbar(false);
            edit.use_global_statusbar(false);
            edit.set_word_wrap(self.source_help);
            edit.readonly(self.source_help);
            if self.source_help {
                edit.set_text(format!("{}\n\n{}", source.help, source.references));
            } else {
                edit.set_text(source.code.clone());
                edit.display_line_number(true);
            }
            self.source_edit = Some((owner, edit));
        }
        let area = self.source_area();
        if let Some((_, edit)) = &mut self.source_edit {
            edit.set_dim(
                TheDim::rect(
                    area.origin[0] as i32,
                    area.origin[1] as i32,
                    area.size[0] as i32,
                    area.size[1] as i32,
                ),
                ctx,
            );
        }
    }
    fn source_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        if self.pane != MaterialPane::Tile {
            return false;
        }
        if let TheEvent::Hover(p) = event {
            self.pointer = [p.x as f32, p.y as f32];
        }
        self.sync_source_edit(ctx);
        let Some((_, edit)) = &mut self.source_edit else {
            return false;
        };
        let local = |p: &Vec2<i32>| Vec2::new(p.x - edit.dim().x, p.y - edit.dim().y);
        let event = match event {
            TheEvent::MouseDown(p) if edit.dim().contains(*p) => {
                self.source_focus = true;
                ctx.ui.set_focus(&self.id);
                TheEvent::MouseDown(local(p))
            }
            TheEvent::MouseDown(_) => {
                self.source_focus = false;
                return false;
            }
            TheEvent::MouseDragged(p) if self.source_focus => TheEvent::MouseDragged(local(p)),
            TheEvent::MouseUp(p) if self.source_focus => TheEvent::MouseUp(local(p)),
            TheEvent::Hover(p) if edit.dim().contains(*p) => TheEvent::Hover(local(p)),
            TheEvent::PreciseScroll(delta)
                if edit
                    .dim()
                    .contains(Vec2::new(self.pointer[0] as i32, self.pointer[1] as i32)) =>
            {
                self.source_scroll_fraction[0] += delta.x;
                self.source_scroll_fraction[1] += delta.y;
                let steps = [
                    self.source_scroll_fraction[0].trunc(),
                    self.source_scroll_fraction[1].trunc(),
                ];
                self.source_scroll_fraction[0] -= steps[0];
                self.source_scroll_fraction[1] -= steps[1];
                TheEvent::MouseWheel(Vec2::new(steps[0] as i32, steps[1] as i32))
            }
            TheEvent::MouseWheel(_)
                if edit
                    .dim()
                    .contains(Vec2::new(self.pointer[0] as i32, self.pointer[1] as i32)) =>
            {
                event.clone()
            }
            TheEvent::KeyDown(_)
            | TheEvent::KeyCodeDown(_)
            | TheEvent::ModifierChanged(..)
            | TheEvent::Paste(..)
            | TheEvent::Copy
            | TheEvent::Cut
            | TheEvent::Undo
            | TheEvent::Redo
                if self.source_focus =>
            {
                event.clone()
            }
            _ => return false,
        };
        edit.on_event(&event, ctx);
        self.dirty = true;
        true
    }
    /// Render directly for a panel host or snapshot, at the buffer's device density.
    pub fn paint(&mut self, buffer: &mut TheRGBABuffer, theme: &GraphTheme) {
        struct Images<'a>(&'a std::collections::HashMap<String, TheRGBABuffer>);
        impl GraphAssets for Images<'_> {
            fn image(&self, key: &str) -> Option<&TheRGBABuffer> {
                self.0.get(key)
            }
        }
        let l = Layout::new(
            [buffer.dim().width as f32, buffer.dim().height as f32],
            self.parameters().2.len(),
            &self.view.rows,
        );
        let (_, label, values) = self.parameters();
        let parameters = Some((label.to_owned(), values.to_vec()));
        let property_lines = if self.active_palette().is_some() {
            vec![]
        } else {
            self.property_lines()
        };
        let source_title = self
            .selected()
            .map_or("Select a tile to view its source".to_owned(), |op| {
                op.label.clone()
            });
        let source_document = self.selected().and_then(|op| op.source.clone());
        let source_area = self.source_area();
        let source_actions: Vec<_> = (0..4).map(|i| self.source_action_rect(i)).collect();
        let active_input = self.active_input().cloned();
        let input_labels: Vec<_> = (0..8).map(|i| self.input_control_label(i)).collect();
        let active_palette = self.active_palette();
        let palette_entries: Vec<_> = self
            .view
            .palette
            .iter()
            .skip(self.palette_scroll * self.palette_columns())
            .enumerate()
            .map(|(i, entry)| (self.palette_rect(i), entry.clone()))
            .collect();
        let mut connection_paths = self.connection_paths(&l);
        connection_paths.sort_by_key(|path| self.connection_selected(path));
        let connection_paths: Vec<_> = connection_paths
            .into_iter()
            .map(|path| {
                let style = self.connection_style(&path, theme);
                (path, style)
            })
            .collect();
        let popup = self.input_popup.and_then(|control| {
            self.popup_area()
                .map(|area| (area, self.input_options(control)))
        });
        let chrome = self.chrome.clone();
        let assets = Images(&self.thumbnails);
        let (width, height, density) = (
            buffer.pixel_width(),
            buffer.pixel_height(),
            buffer.render_scale(),
        );
        let mut p = RasterGraphPainter::new(
            buffer.pixels_mut(),
            width,
            height,
            density,
            &mut self.raster,
            &assets,
        );
        p.round_rect(rect(0., 0., l.width, l.height), 0., theme.background);
        for (i, row) in self.view.rows.iter().enumerate() {
            let r = l.row(i, self.pan);
            if r.origin[1] + r.size[1] < HEADER || r.origin[1] > l.editor_bottom {
                continue;
            }
            let input = l.row_input(i, self.pan);
            p.round_rect(
                input,
                3.,
                if self.pane == MaterialPane::RowInput
                    && self.selected_row.as_ref() == Some(&row.id)
                {
                    theme.active
                } else {
                    theme.control
                },
            );
            for offset in [6., 10., 14.] {
                p.round_rect(
                    rect(input.origin[0] + 2., input.origin[1] + offset, 6., 1.),
                    0.,
                    theme.text,
                );
            }
            for (j, op) in row.operations.iter().enumerate() {
                let c = l.card(i, j, self.pan);
                if c.origin[1] + c.size[1] < HEADER
                    || c.origin[1] > l.editor_bottom
                    || c.origin[0] + c.size[0] > l.width - 18.
                {
                    continue;
                }
                p.round_rect(
                    c,
                    5.,
                    if self.selected.as_ref() == Some(&op.id) {
                        theme.active
                    } else {
                        theme.control
                    },
                );
                p.round_rect(
                    rect(
                        c.origin[0] + 1.,
                        c.origin[1] + 1.,
                        c.size[0] - 2.,
                        c.size[1] - 2.,
                    ),
                    4.,
                    theme.body,
                );
                let category = self
                    .view
                    .sources
                    .iter()
                    .find(|source| source.label == op.label)
                    .map_or("", |source| source.category.as_str());
                p.round_rect(
                    rect(c.origin[0] + 5., c.origin[1] + 2., c.size[0] - 10., 2.),
                    1.,
                    chrome.category(category),
                );
                if self.thumbnails.contains_key(&op.thumbnail) {
                    p.preview(
                        rect(
                            c.origin[0] + 3.,
                            c.origin[1] + 3.,
                            c.size[0] - 6.,
                            CARD - 25.,
                        ),
                        &op.thumbnail,
                    );
                } else {
                    p.text(
                        rect(c.origin[0] + 7., c.origin[1] + 8., CARD - 14., 24.),
                        "MZ",
                        11.,
                        theme.muted,
                    );
                }
                centered_text(
                    &mut p,
                    rect(
                        c.origin[0] + 3.,
                        c.origin[1] + CARD - 22.,
                        c.size[0] - 6.,
                        20.,
                    ),
                    &op.label,
                    10.5,
                    theme.text,
                );
            }
            let count = row.operations.len();
            let slots = if count % COLUMNS == 0 {
                count + 1
            } else {
                count.div_ceil(COLUMNS) * COLUMNS
            };
            for column in row.operations.len()..slots {
                let area = l.card(i, column, self.pan);
                if area.origin[1] + CARD >= HEADER
                    && area.origin[1] < l.editor_bottom
                    && area.origin[0] + area.size[0] <= l.width - 18.
                {
                    empty_tile(&mut p, area, theme);
                }
            }
            let terminal = l.row_port(i, self.pan);
            p.round_rect(terminal, 5., theme.active);
        }
        let new_row = l.new_row(self.view.rows.len(), self.pan);
        if new_row.origin[1] < l.editor_bottom {
            empty_tile(&mut p, new_row, theme);
        }
        // Pinned output rail is drawn after the scrollable rows.
        p.round_rect(
            rect(0., HEADER, RAIL, l.editor_bottom - HEADER),
            0.,
            theme.body,
        );
        for (i, output) in self.view.outputs.iter().enumerate() {
            let terminal = l.output(i);
            p.text(
                rect(5., terminal.origin[1] - 2., RAIL - 23., 17.),
                &output.label,
                11.,
                theme.text,
            );
            p.round_rect(
                terminal,
                5.,
                if output.row.is_some() {
                    theme.active
                } else {
                    theme.muted
                },
            );
            if output.row.is_none() {
                p.round_rect(
                    rect(terminal.origin[0] + 2., terminal.origin[1] + 2., 6., 6.),
                    3.,
                    theme.body,
                );
                p.text(
                    rect(5., terminal.origin[1] + 16., RAIL - 12., 14.),
                    &output.fallback,
                    10.5,
                    theme.muted,
                );
            }
        }
        // Draw the selected row's incoming/outgoing connections last so they
        // remain readable at crossings with dimmed unrelated wires.
        for (path, (color, width)) in connection_paths {
            for [a, b] in path.segments(l.editor_bottom) {
                p.curve([a, a, b, b], width, color);
            }
        }
        match &self.gesture {
            Some(Gesture::RowWire { row, point }) => {
                if let Some(i) = self.view.rows.iter().position(|r| &r.id == row) {
                    let a = center(l.row_port(i, self.pan));
                    p.curve(
                        [a, [a[0] - 32., a[1]], [point[0] + 32., point[1]], *point],
                        2.,
                        theme.active,
                    );
                }
            }
            Some(Gesture::OutputWire { output, point, .. }) => {
                if let Some(i) = self.view.outputs.iter().position(|o| &o.id == output) {
                    let a = center(l.output(i));
                    p.curve(
                        [a, [a[0] + 32., a[1]], [point[0] - 32., point[1]], *point],
                        2.,
                        theme.active,
                    );
                }
            }
            _ => {}
        }
        p.round_rect(
            rect(0., l.editor_bottom, l.width, l.height - l.editor_bottom),
            0.,
            theme.body,
        );
        p.round_rect(rect(0., l.editor_bottom, l.width, 1.), 0., theme.control);
        for (index, label) in ["Props", "Sources", "Row Input", "Tile"]
            .into_iter()
            .enumerate()
        {
            let selected = (index == 0 && self.pane == MaterialPane::Properties)
                || (index == 1 && self.pane == MaterialPane::Sources)
                || (index == 2 && self.pane == MaterialPane::RowInput)
                || (index == 3 && self.pane == MaterialPane::Tile);
            let area = l.pane_tab(index);
            p.round_rect(area, 4., if selected { theme.control } else { theme.body });
            if selected {
                p.round_rect(
                    rect(
                        area.origin[0] + 8.,
                        area.origin[1] + area.size[1] - 2.,
                        area.size[0] - 16.,
                        2.,
                    ),
                    1.,
                    theme.active,
                );
            }
            centered_text(
                &mut p,
                area,
                label,
                11.,
                if selected { theme.text } else { theme.muted },
            );
        }
        if self.pane == MaterialPane::Properties {
            p.text(
                rect(10., l.editor_bottom + 30., l.width - 20., 18.),
                parameters.as_ref().map_or("Material", |(label, _)| label),
                11.,
                theme.muted,
            );
            if let Some(palette) = &active_palette {
                if palette_entries.is_empty() {
                    p.text(
                        rect(16., l.editor_bottom + 58., l.width - 32., 36.),
                        "Add colors in the project Palette to use this tile.",
                        11.,
                        theme.muted,
                    );
                }
                for (area, entry) in &palette_entries {
                    let area = *area;
                    if area.origin[1] + area.size[1] > l.height {
                        break;
                    }
                    p.round_rect(
                        area,
                        3.,
                        if palette.2 == Some(entry.index) {
                            theme.active
                        } else {
                            theme.control
                        },
                    );
                    for y in 0..4 {
                        for x in 0..4 {
                            let checker = chrome.checker[(x + y) % 2];
                            let alpha = entry.color[3] as f32 / 255.;
                            let color = [
                                (entry.color[0] as f32 * alpha + checker[0] as f32 * (1. - alpha))
                                    as u8,
                                (entry.color[1] as f32 * alpha + checker[1] as f32 * (1. - alpha))
                                    as u8,
                                (entry.color[2] as f32 * alpha + checker[2] as f32 * (1. - alpha))
                                    as u8,
                                255,
                            ];
                            p.round_rect(
                                rect(
                                    area.origin[0] + 2. + x as f32 * 5.5,
                                    area.origin[1] + 2. + y as f32 * 5.5,
                                    5.5,
                                    5.5,
                                ),
                                0.,
                                color,
                            );
                        }
                    }
                }
            }
            for (i, line) in property_lines
                .iter()
                .skip(self.parameter_scroll)
                .enumerate()
            {
                let area = l.parameter(i);
                if area.origin[1] + area.size[1] > l.properties_bottom {
                    break;
                }
                match line {
                    PropertyLine::Section(label) => {
                        p.round_rect(
                            rect(area.origin[0], area.origin[1] + 5., 2., 12.),
                            1.,
                            theme.active,
                        );
                        p.text(
                            rect(
                                area.origin[0] + 4.,
                                area.origin[1] + 3.,
                                area.size[0] - 8.,
                                16.,
                            ),
                            &label.to_uppercase(),
                            8.,
                            theme.active,
                        );
                    }
                    PropertyLine::Values {
                        label,
                        values,
                        color,
                        owner,
                        ..
                    } => {
                        p.round_rect(area, 3., theme.control);
                        let swatch = if color.is_some() && values.len() >= 3 {
                            let rgba = [
                                linear_to_srgb(values[0].value),
                                linear_to_srgb(values[1].value),
                                linear_to_srgb(values[2].value),
                                values.get(3).map_or(1., |v| v.value),
                            ];
                            Some(rgba.map(|v| (v.clamp(0., 1.) * 255.) as u8))
                        } else {
                            None
                        };
                        if let Some(color) = swatch {
                            p.round_rect(
                                rect(area.origin[0] + 4., area.origin[1] + 3., 14., 14.),
                                2.,
                                color,
                            );
                        }
                        p.text(
                            rect(
                                area.origin[0] + if swatch.is_some() { 22. } else { 8. },
                                area.origin[1] + 3.,
                                area.size[0] * 0.44 - 10.,
                                16.,
                            ),
                            label,
                            10.5,
                            theme.text,
                        );
                        let width = area.size[0] * 0.55 / values.len().max(1) as f32;
                        if let Some(parameter) = color {
                            let text = self
                                .view
                                .palette_bindings
                                .iter()
                                .find(|b| &b.operation == owner && &b.parameter == parameter)
                                .map_or("Choose palette color".into(), |b| {
                                    format!("Palette {}", b.index)
                                });
                            p.text(
                                rect(
                                    area.origin[0] + area.size[0] * 0.45,
                                    area.origin[1] + 3.,
                                    area.size[0] * 0.55,
                                    16.,
                                ),
                                &text,
                                10.5,
                                theme.text,
                            );
                        }
                        for (j, value) in values.iter().enumerate().filter(|_| color.is_none()) {
                            p.text(
                                rect(
                                    area.origin[0] + area.size[0] * 0.45 + j as f32 * width + 2.,
                                    area.origin[1] + 3.,
                                    width - 5.,
                                    16.,
                                ),
                                &format!("{:.3}", value.value),
                                10.5,
                                theme.text,
                            );
                        }
                    }
                    PropertyLine::Choice {
                        label,
                        value,
                        choices,
                        ..
                    } => {
                        p.round_rect(area, 3., theme.control);
                        p.text(
                            rect(
                                area.origin[0] + 8.,
                                area.origin[1] + 3.,
                                area.size[0] * 0.44,
                                16.,
                            ),
                            label,
                            10.5,
                            theme.text,
                        );
                        p.text(
                            rect(
                                area.origin[0] + area.size[0] * 0.45,
                                area.origin[1] + 3.,
                                area.size[0] * 0.5 - 12.,
                                16.,
                            ),
                            choices.get(*value).map_or("Choose…", |s| s.as_str()),
                            10.5,
                            theme.text,
                        );
                        p.text(
                            rect(
                                area.origin[0] + area.size[0] - 14.,
                                area.origin[1] + 3.,
                                11.,
                                16.,
                            ),
                            "v",
                            10.5,
                            theme.muted,
                        );
                    }
                    PropertyLine::Bound { label, source, .. } => {
                        p.round_rect(area, 3., theme.control);
                        p.text(
                            rect(
                                area.origin[0] + 8.,
                                area.origin[1] + 3.,
                                area.size[0] * 0.44,
                                16.,
                            ),
                            label,
                            10.5,
                            theme.text,
                        );
                        p.text(
                            rect(
                                area.origin[0] + area.size[0] * 0.45,
                                area.origin[1] + 3.,
                                area.size[0] * 0.55 - 8.,
                                16.,
                            ),
                            &format!("← {source}"),
                            10.5,
                            theme.active,
                        );
                    }
                    PropertyLine::Boolean { label, value, .. } => {
                        p.round_rect(area, 3., theme.control);
                        p.text(
                            rect(
                                area.origin[0] + 8.,
                                area.origin[1] + 3.,
                                area.size[0] * 0.75,
                                16.,
                            ),
                            label,
                            10.5,
                            theme.text,
                        );
                        let x = area.origin[0] + area.size[0] - 39.;
                        p.round_rect(
                            rect(x, area.origin[1] + 3., 30., 14.),
                            7.,
                            if *value { theme.active } else { theme.body },
                        );
                        p.round_rect(
                            rect(
                                x + if *value { 17. } else { 2. },
                                area.origin[1] + 5.,
                                11.,
                                11.,
                            ),
                            5.,
                            theme.text,
                        );
                    }
                }
            }
        } else if self.pane == MaterialPane::RowInput {
            for control in 0..8 {
                if control == 7
                    && !matches!(
                        active_input.as_ref().map(|i| &i.target),
                        Some(MaterialInputTarget::RowLink { .. })
                    )
                {
                    continue;
                }
                let visible = control < 3
                    || control == 7
                    || matches!(
                        active_input.as_ref().map(|i| &i.binding),
                        Some(MaterialInputBinding::Row(_))
                    ) && (control < 5
                        || matches!(
                            active_input.as_ref().map(|i| &i.binding),
                            Some(MaterialInputBinding::Row(MaterialReferenceView {
                                mapping: Some(_),
                                ..
                            }))
                        ));
                if !visible {
                    continue;
                }
                let area = l.input_control(control);
                if area.origin[1] + area.size[1] > l.height {
                    continue;
                }
                p.round_rect(area, 3., theme.control);
                p.text(
                    rect(
                        area.origin[0] + 6.,
                        area.origin[1] + 5.,
                        area.size[0] - 18.,
                        16.,
                    ),
                    &input_labels[control],
                    10.5,
                    theme.text,
                );
                if control < 3 || control == 7 {
                    p.text(
                        rect(
                            area.origin[0] + area.size[0] - 14.,
                            area.origin[1] + 5.,
                            11.,
                            16.,
                        ),
                        "v",
                        10.5,
                        theme.muted,
                    );
                }
                if control == 3 {
                    if let Some(MaterialInputBinding::Row(reference)) =
                        active_input.as_ref().map(|i| &i.binding)
                    {
                        p.text(
                            rect(
                                area.origin[0] + area.size[0] * 0.5,
                                area.origin[1] + 4.,
                                area.size[0] * 0.45,
                                16.,
                            ),
                            &reference.components,
                            11.,
                            theme.text,
                        );
                    }
                }
            }
            if active_input.is_none() {
                p.text(
                    rect(16., l.editor_bottom + 122., l.width - 32., 36.),
                    "Select a row or tile to inspect its inputs.",
                    11.,
                    theme.muted,
                );
            }
        } else if self.pane == MaterialPane::Tile {
            p.text(
                rect(10., l.editor_bottom + 30., l.width - 20., 18.),
                &source_title,
                11.,
                theme.muted,
            );
            p.round_rect(source_area, 3., theme.body);
            for (i, label) in [
                "Apply",
                "Revert",
                "Reset",
                if self.source_help { "Source" } else { "Help" },
            ]
            .into_iter()
            .enumerate()
            {
                let area = source_actions[i];
                p.round_rect(area, 3., theme.control);
                centered_text(&mut p, area, label, 10.5, theme.text);
            }
            if self.source_edit.is_none() {
                if let Some(source) = &source_document {
                    let text = if self.source_help {
                        source.help.as_str()
                    } else {
                        source.code.as_str()
                    };
                    let area = source_area;
                    for (i, line) in text
                        .lines()
                        .take((area.size[1] / 15.).max(0.) as usize)
                        .enumerate()
                    {
                        p.text(
                            rect(
                                area.origin[0] + 5.,
                                area.origin[1] + i as f32 * 15. + 3.,
                                area.size[0] - 10.,
                                15.,
                            ),
                            line,
                            10.5,
                            theme.text,
                        );
                    }
                }
            }
        } else {
            let search = l.search();
            let category = l.category();
            let category_color =
                chrome.category(self.source_category.as_deref().unwrap_or("Coordinates"));
            p.round_rect(category, 4., category_color);
            p.round_rect(
                rect(
                    category.origin[0] + 1.,
                    category.origin[1] + 1.,
                    category.size[0] - 2.,
                    category.size[1] - 2.,
                ),
                3.,
                chrome.dropdown,
            );
            p.text(
                rect(
                    category.origin[0] + 6.,
                    category.origin[1] + 4.,
                    category.size[0] - 18.,
                    16.,
                ),
                self.source_category.as_deref().unwrap_or("All"),
                10.5,
                category_color,
            );
            p.text(
                rect(
                    category.origin[0] + category.size[0] - 13.,
                    category.origin[1] + 4.,
                    11.,
                    16.,
                ),
                "v",
                10.5,
                category_color,
            );
            p.round_rect(
                search,
                4.,
                if self.search_focused {
                    theme.active
                } else {
                    theme.control
                },
            );
            p.text(
                rect(16., search.origin[1] + 2., search.size[0] - 12., 18.),
                if self.query.is_empty() {
                    "Find a source…"
                } else {
                    &self.query
                },
                10.5,
                theme.text,
            );
            let query = self.query.to_lowercase();
            let sources: Vec<_> = self
                .view
                .sources
                .iter()
                .filter(|s| {
                    self.source_category
                        .as_ref()
                        .is_none_or(|category| category == &s.category)
                        && (s.label.to_lowercase().contains(&query)
                            || s.category.to_lowercase().contains(&query))
                })
                .collect();
            for (i, source) in sources.iter().skip(self.library_scroll).enumerate() {
                let y = l.library_top + i as f32 * SOURCE_STEP;
                if y + 30. > l.height {
                    break;
                }
                let area = rect(10., y, l.width - 20., 30.);
                p.round_rect(
                    area,
                    4.,
                    if self.selected_source.as_ref() == Some(&source.id) {
                        theme.active
                    } else {
                        chrome.source
                    },
                );
                if self.selected_source.as_ref() == Some(&source.id) {
                    p.round_rect(rect(11., y + 1., l.width - 22., 28.), 3., theme.body);
                }
                let accent = chrome.category(&source.category);
                p.round_rect(rect(10., y + 4., 3., 22.), 1., accent);
                p.text(
                    rect(16., y + 1., l.width - 32., 16.),
                    &source.label,
                    10.5,
                    theme.text,
                );
                p.text(
                    rect(16., y + 16., l.width - 32., 12.),
                    &source.category,
                    9.5,
                    accent,
                );
            }
            if sources.is_empty() {
                p.text(
                    rect(20., l.library_top + 12., l.width - 40., 32.),
                    "No matching sources",
                    11.,
                    theme.muted,
                );
            }
        }
        if let Some((area, options)) = popup {
            p.round_rect(
                rect(
                    area.origin[0] + 3.,
                    area.origin[1] + 4.,
                    area.size[0],
                    area.size[1],
                ),
                4.,
                chrome.shadow,
            );
            p.round_rect(area, 3., theme.active);
            p.round_rect(
                rect(
                    area.origin[0] + 1.,
                    area.origin[1] + 1.,
                    area.size[0] - 2.,
                    area.size[1] - 2.,
                ),
                2.,
                chrome.popup,
            );
            for (index, option) in options.iter().skip(self.popup_scroll).take(8).enumerate() {
                let item = rect(
                    area.origin[0] + 1.,
                    area.origin[1] + index as f32 * 22. + 1.,
                    area.size[0] - 2.,
                    20.,
                );
                let selected = self.input_popup == Some(8)
                    && option == self.source_category.as_deref().unwrap_or("All");
                let hovered = item.contains(self.pointer);
                p.round_rect(
                    item,
                    0.,
                    if selected {
                        chrome.selection
                    } else if hovered {
                        chrome.hover
                    } else {
                        chrome.popup
                    },
                );
                let accent = if self.input_popup == Some(8) {
                    chrome.category(option)
                } else {
                    theme.active
                };
                if self.input_popup == Some(8) {
                    p.round_rect(
                        rect(item.origin[0] + 5., item.origin[1] + 7., 3., 7.),
                        1.,
                        accent,
                    );
                }
                p.text(
                    rect(
                        item.origin[0] + 13.,
                        item.origin[1] + 4.,
                        item.size[0] - 19.,
                        16.,
                    ),
                    option,
                    10.5,
                    if selected { accent } else { theme.text },
                );
            }
        }
        if let Some(Gesture::Source { id, start, point }) = &self.gesture
            && (point[0] - start[0]).abs() + (point[1] - start[1]).abs() > 4.
        {
            if let Some(source) = self.view.sources.iter().find(|s| &s.id == id) {
                p.round_rect(
                    rect(point[0] + 8., point[1] + 8., 116., 27.),
                    4.,
                    theme.active,
                );
                p.text(
                    rect(point[0] + 16., point[1] + 10., 100., 23.),
                    &source.label,
                    11.,
                    theme.text,
                );
            }
        }
        self.dirty = false;
    }
}
impl TheWidget for MaterialEditorWidget {
    fn new(id: TheId) -> Self {
        let font = theframework::Embedded::get("fonts/Roboto-Bold.ttf").expect("framework UI font");
        let font = fontdue::Font::from_bytes(font.data.as_ref(), fontdue::FontSettings::default())
            .expect("valid UI font");
        Self {
            chrome: MaterialEditorChrome::from_theme(&TheDarkTheme::new()),
            id,
            dim: TheDim::zero(),
            limiter: TheSizeLimiter::new(),
            view: MaterialEditorView::default(),
            selected: None,
            selected_source: None,
            pane: MaterialPane::Sources,
            gesture: None,
            commands: Vec::new(),
            pan: [0., 0.],
            library_scroll: 0,
            parameter_scroll: 0,
            library_fraction: 0.,
            parameter_fraction: 0.,
            query: String::new(),
            source_category: None,
            search_focused: false,
            pointer: [0., 0.],
            dirty: true,
            raster: GraphRasterResources::new(font),
            thumbnails: Default::default(),
            number_edits: Vec::new(),
            number_focus: None,
            number_drag: false,
            selected_row: None,
            selected_input: 0,
            input_popup: None,
            palette_scroll: 0,
            popup_scroll: 0,
            source_edit: None,
            source_focus: false,
            source_help: false,
            source_scroll_fraction: [0., 0.],
            color_property: None,
        }
    }
    fn id(&self) -> &TheId {
        &self.id
    }
    fn dim(&self) -> &TheDim {
        &self.dim
    }
    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }
    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }
    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
    }
    fn set_dim(&mut self, dim: TheDim, _ctx: &mut TheContext) {
        self.dim = dim;
        self.clamp_scroll();
        self.dirty = true;
    }
    fn needs_redraw(&mut self) -> bool {
        self.dirty
    }
    fn set_needs_redraw(&mut self, value: bool) {
        self.dirty = value;
    }
    fn supports_hover(&mut self) -> bool {
        true
    }
    fn supports_precise_scroll(&self) -> bool {
        true
    }
    fn supports_text_input(&self) -> bool {
        self.search_focused || self.number_focus.is_some() || self.source_focus
    }
    fn supports_clipboard(&mut self) -> bool {
        self.number_focus.is_some() || self.source_focus
    }
    fn supports_undo_redo(&mut self) -> bool {
        self.number_focus.is_some() || self.source_focus
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        if self.source_event(event, ctx) {
            return true;
        }
        if self.number_event(event, ctx) {
            return true;
        }
        let before = self.commands.len();
        match event {
            TheEvent::MouseDown(p) => {
                ctx.ui.set_focus(&self.id);
                self.press([p.x as f32, p.y as f32]);
            }
            TheEvent::MouseDragged(p) => self.drag([p.x as f32, p.y as f32]),
            TheEvent::MouseUp(p) => self.release([p.x as f32, p.y as f32]),
            TheEvent::Hover(p) => {
                self.pointer = [p.x as f32, p.y as f32];
                ctx.ui.set_hover(&self.id);
                match self.hit(self.pointer) {
                    Some(Hit::Property(index))=>{if let Some(line)=self.property_lines().get(index){ctx.ui.send(TheEvent::SetStatusText(self.id.clone(),line.tooltip().to_owned()));}},
                    Some(Hit::Number(_,parameter))=>{if let Some(line)=self.property_lines().iter().find(|line|matches!(line,PropertyLine::Values{values,..} if values.iter().any(|v|v.id==parameter))){ctx.ui.send(TheEvent::SetStatusText(self.id.clone(),line.tooltip().to_owned()));}},
                    Some(Hit::Palette(index)) => {
                        if let Some(entry) = self.view.palette.get(index) {
                            ctx.ui.send(TheEvent::SetStatusText(
                                self.id.clone(),
                                format!("{} · Palette {}", entry.label, entry.index),
                            ));
                        }
                    }
                    Some(Hit::RowInput(id)) => {
                        if let Some(row) = self.view.rows.iter().find(|row| row.id == id) {
                            ctx.ui.send(TheEvent::SetStatusText(
                                self.id.clone(),
                                format!("Row Input · {}", row.label),
                            ));
                        }
                    }
                    _ => {}
                }
            }
            TheEvent::MouseWheel(delta) => {
                self.scroll(self.pointer, [-delta.x as f32, -delta.y as f32])
            }
            TheEvent::PreciseScroll(delta) => self.scroll(self.pointer, [-delta.x, -delta.y]),
            TheEvent::KeyDown(TheValue::Char(c)) if self.search_focused => {
                self.query.push(*c);
                self.library_scroll = 0;
                self.dirty = true;
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Delete)) if self.search_focused => {
                self.query.pop();
                self.library_scroll = 0;
                self.dirty = true;
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Delete)) => {
                if self.pane == MaterialPane::Properties
                    && let Some(id) = &self.selected
                {
                    self.commands
                        .push(MaterialEditorCommand::RemoveOperation(id.clone()));
                }
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Escape)) => {
                self.cancel_gesture();
                self.search_focused = false;
                self.input_popup = None;
            }
            TheEvent::LostFocus(_) => {
                self.cancel_gesture();
                self.search_focused = false;
            }
            TheEvent::Context(p) => {
                if let Some(Hit::Output(id)) = self.hit([p.x as f32, p.y as f32]) {
                    self.commands
                        .push(MaterialEditorCommand::DisconnectOutput(id));
                }
            }
            _ => return false,
        }
        if self.commands.len() > before {
            ctx.ui.send(TheEvent::Custom(
                TheId::named(MATERIAL_EDITOR_COMMANDS),
                TheValue::Id(self.id.uuid),
            ));
        }
        self.dirty = true;
        true
    }
    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim.is_valid() {
            return;
        }
        let mut local = TheRGBABuffer::new(TheDim::new(0, 0, self.dim.width, self.dim.height));
        local.set_render_scale(ctx.ui_render_scale);
        self.chrome = MaterialEditorChrome::from_theme(style.theme().as_ref());
        let theme = material_editor_theme(style.theme().as_ref());
        self.paint(&mut local, &theme);
        self.sync_number_edits(ctx);
        if self.input_popup.is_none() {
            for (_, _, edit) in &mut self.number_edits {
                edit.draw(&mut local, style, ctx);
            }
        }
        self.sync_source_edit(ctx);
        if self.pane == MaterialPane::Tile {
            if let Some((_, edit)) = &mut self.source_edit {
                edit.draw(&mut local, style, ctx);
            }
        }
        buffer.copy_into(self.dim.buffer_x, self.dim.buffer_y, &local);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_visible_pbr_output_connects_by_label_and_by_row_drop() {
        let mut w = widget();
        w.view.outputs = [
            ("base color", "Color"),
            ("roughness", "Roughness"),
            ("metallic", "Metallic"),
            ("opacity", "Opacity"),
            ("emission", "Emission"),
        ]
        .into_iter()
        .map(|(id, label)| MaterialOutputView {
            id: id.into(),
            label: label.into(),
            row: None,
            fallback: "Default".into(),
        })
        .collect();
        let row = w.view.rows[0].id.clone();
        for index in 0..w.view.outputs.len() {
            let label = [16., w.layout().output(index).origin[1] + 2.];
            let port = center(w.layout().row_port(0, w.pan));
            let expected = MaterialEditorCommand::ConnectOutput {
                output: w.view.outputs[index].id.clone(),
                row: row.clone(),
            };
            w.press(port);
            w.release(label);
            assert_eq!(w.take_commands(), vec![expected.clone()]);
            w.press(label);
            w.release(center(w.layout().card(0, 0, w.pan)));
            assert_eq!(w.take_commands(), vec![expected]);
        }
    }
    #[test]
    fn unconnected_outputs_remain_visible_and_canvas_has_no_static_grid() {
        let mut w = widget();
        w.view.outputs = ["Color", "Roughness", "Metallic", "Opacity", "Emission"]
            .into_iter()
            .map(|label| MaterialOutputView {
                id: label.into(),
                label: label.into(),
                row: None,
                fallback: "Default".into(),
            })
            .collect();
        let theme = material_editor_theme(&TheBrownTheme::new());
        let mut buffer = TheRGBABuffer::new(TheDim::sized(380, 800));
        w.paint(&mut buffer, &theme);
        let at = |x: f32, y: f32| {
            let i = (y as usize * 380 + x as usize) * 4;
            &buffer.pixels()[i..i + 4]
        };
        for i in 0..5 {
            let port = w.layout().output(i);
            let edge = at(port.origin[0], port.origin[1] + 5.);
            assert!(
                edge[..3]
                    .iter()
                    .zip(theme.muted)
                    .all(|(actual, expected)| actual.abs_diff(expected) <= 5)
            );
            assert_eq!(at(port.origin[0] + 5., port.origin[1] + 5.), theme.body);
        }
        // A blank strip between columns used to contain fixed lines every 24px.
        let card = w.layout().card(0, 0, w.pan);
        let gap = card.origin[0] + card.size[0] + 4.;
        for y in 14..70 {
            assert_eq!(at(gap, y as f32), theme.background);
        }
    }
    #[test]
    fn native_render_uses_theme_overrides_for_material_controls_and_categories() {
        let mut widget = widget();
        let mut ctx = TheContext::new(380, 700, 1.);
        widget.set_dim(TheDim::new(0, 0, 380, 700), &mut ctx);
        widget.pane = MaterialPane::Sources;
        let mut skin = TheBlackBlueTheme::new();
        let category_color = [71, 214, 129, 255];
        skin.set_palette_color(TheThemePalettes::MaterialCategories, 3, category_color);
        let popup_color = [23, 31, 42, 255];
        skin.set_color(MaterialPopup, popup_color);
        let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::new());
        style.set_theme(Box::new(skin));
        let mut buffer = TheRGBABuffer::new(TheDim::new(0, 0, 380, 700));
        widget.draw(&mut buffer, &mut style, &mut ctx);
        let category = widget.layout().category();
        let at = ((category.origin[1] as usize + 12) * 380 + category.origin[0] as usize) * 4;
        let pixel = &buffer.pixels()[at..at + 4];
        // The thin rounded border is antialiased against the dropdown surface.
        assert!(pixel[1] > pixel[0] * 2 && pixel[1] > pixel[2], "{pixel:?}");
        widget.press(center(category));
        widget.draw(&mut buffer, &mut style, &mut ctx);
        let popup = widget.popup_area().unwrap();
        // Second option, away from its text/marker and pointer hover.
        let at = ((popup.origin[1] as usize + 30) * 380 + popup.origin[0] as usize + 90) * 4;
        assert_eq!(&buffer.pixels()[at..at + 4], &popup_color);
    }
    fn widget() -> MaterialEditorWidget {
        let mut w = MaterialEditorWidget::new(TheId::named("material"));
        w.dim = TheDim::new(0, 0, 380, 800);
        let operation = |id: &str| MaterialOperationView {
            id: id.into(),
            label: "Noise".into(),
            thumbnail: String::new(),
            palette: None,
            properties: vec![],
            combine: None,
            source: None,
            parameters: vec![MaterialNumberView {
                id: "scale".into(),
                label: "Scale".into(),
                value: 0.5,
                min: 0.,
                max: 1.,
            }],
        };
        w.set_view(MaterialEditorView {
            title: "Material".into(),
            palette: vec![],
            palette_bindings: vec![],
            tiling: MaterialTilingView::default(),
            animation: MaterialAnimationView::default(),
            defaults: vec![],
            rows: vec![
                MaterialRowView {
                    id: "grain".into(),
                    label: "Grain".into(),
                    operations: vec![operation("a")],
                    inputs: vec![],
                },
                MaterialRowView {
                    id: "color".into(),
                    label: "Color".into(),
                    operations: vec![operation("b")],
                    inputs: vec![],
                },
            ],
            outputs: vec![MaterialOutputView {
                id: "base color".into(),
                label: "Color".into(),
                row: None,
                fallback: "0.8".into(),
            }],
            sources: vec![
                MaterialSourceView {
                    id: "perlin".into(),
                    label: "Perlin Noise".into(),
                    category: "Noise".into(),
                },
                MaterialSourceView {
                    id: "mix".into(),
                    label: "Mix".into(),
                    category: "Color".into(),
                },
            ],
        });
        w
    }
    #[test]
    fn source_drop_emits_id_based_command_without_mutating_document() {
        let mut w = widget();
        let source = [30., w.layout().library_top + 12.];
        let target = center(w.layout().card(1, 0, w.pan));
        w.press(source);
        w.drag(target);
        w.release(target);
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::AddSource {
                source: "perlin".into(),
                row: Some("color".into()),
                after: Some("b".into())
            }]
        );
        assert_eq!(w.view.rows[1].operations.len(), 1);
        assert_eq!(w.view.outputs[0].row, None);
    }
    #[test]
    fn filtering_and_cancelled_source_drag_do_not_retarget_sources() {
        let mut w = widget();
        w.set_source_filter("mix");
        let source = [30., w.layout().library_top + 12.];
        let target = center(w.layout().new_row(w.view.rows.len(), w.pan));
        w.press(source);
        w.cancel_gesture();
        w.release(target);
        assert!(w.take_commands().is_empty());
        w.press(source);
        w.release(target);
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::AddSource {
                source: "mix".into(),
                row: None,
                after: None
            }]
        );
    }
    #[test]
    fn connections_use_host_identity_after_row_reordering() {
        let mut w = widget();
        let mut view = w.view.clone();
        view.rows.swap(0, 1);
        w.set_view(view);
        let start = center(w.layout().row_port(0, w.pan));
        let target = center(w.layout().output(0));
        w.press(start);
        w.release(target);
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::ConnectOutput {
                output: "base color".into(),
                row: "color".into()
            }]
        );
        assert_eq!(w.view.outputs[0].row, None);
    }
    #[test]
    fn numeric_gesture_commits_once_and_cancellation_discards_preview() {
        let mut w = widget();
        let card = center(w.layout().card(0, 0, w.pan));
        w.press(card);
        w.release(card);
        w.take_commands();
        let start = center(w.layout().parameter(0));
        w.press(start);
        w.drag([start[0] + 500., start[1]]);
        w.drag([start[0] + 400., start[1]]);
        assert!(w.commands.is_empty());
        w.release([start[0] + 400., start[1]]);
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::SetNumber {
                operation: "a".into(),
                parameter: "scale".into(),
                value: 1.
            }]
        );
        assert_eq!(w.view.rows[0].operations[0].parameters[0].value, 0.5);
        w.press(start);
        w.drag([start[0] + 10., start[1]]);
        w.cancel_gesture();
        w.release(start);
        assert!(w.take_commands().is_empty());
    }
    #[test]
    fn replacing_host_document_cancels_in_flight_gestures() {
        let mut w = widget();
        w.press([30., w.layout().library_top + 12.]);
        let mut next = w.view.clone();
        next.rows.reverse();
        w.set_view(next);
        let target = center(w.layout().card(0, 0, w.pan));
        w.release(target);
        assert!(w.take_commands().is_empty());
    }
    #[test]
    fn panel_renders_at_native_density_and_handles_small_clipped_surfaces() {
        let mut w = widget();
        for (width, height, density) in [(380, 800, 1.), (380, 800, 2.), (42, 35, 2.)] {
            let mut buffer = TheRGBABuffer::new(TheDim::new(0, 0, width, height));
            buffer.set_render_scale(density);
            w.paint(&mut buffer, &GraphTheme::default());
            assert_eq!(
                buffer.pixels().len(),
                buffer.pixel_width() * buffer.pixel_height() * 4
            );
            assert!(buffer.pixels().chunks_exact(4).all(|p| p[3] == 255));
        }
    }
    #[test]
    fn scrolled_properties_commit_the_visible_parameter_identity() {
        let mut w = widget();
        w.pane = MaterialPane::Properties;
        w.view.defaults = (0..30)
            .map(|i| MaterialNumberView {
                id: format!("constant-{i}"),
                label: format!("Constant {i}"),
                value: 0.5,
                min: 0.,
                max: 1.,
            })
            .collect();
        w.scroll([30., w.layout().editor_bottom + 60.], [0., 40.]);
        assert_eq!(w.parameter_scroll, 4);
        let point = center(w.layout().parameter(0));
        w.press(point);
        w.drag([point[0] + 20., point[1]]);
        w.release([point[0] + 20., point[1]]);
        assert!(
            matches!(w.take_commands().as_slice(),[MaterialEditorCommand::SetNumber {operation,parameter,..}] if operation=="@surface" && parameter=="constant-4")
        );
        w.press([15., w.layout().editor_bottom + 32.]);
        assert_eq!(w.parameter_scroll, 0);
    }
    #[test]
    fn three_column_rows_wrap_operations_without_splitting_the_material_row() {
        let mut w = widget();
        let sample = w.view.rows[0].operations[0].clone();
        w.view.rows[0].operations = (0..8)
            .map(|i| {
                let mut op = sample.clone();
                op.id = format!("op-{i}");
                op
            })
            .collect();
        let layout = w.layout();
        let first = layout.card(0, 0, w.pan);
        let third = layout.card(0, 2, w.pan);
        let fourth = layout.card(0, 3, w.pan);
        assert_eq!(first.origin[1], third.origin[1]);
        assert_eq!(first.origin[0], fourth.origin[0]);
        assert!(fourth.origin[1] > first.origin[1]);
        assert!(third.origin[0] + third.size[0] < layout.width - 24.);
        assert!(layout.row(1, w.pan).origin[1] > layout.card(0, 7, w.pan).origin[1] + CARD);
        assert_eq!(w.view.rows[0].operations.len(), 8);
    }
    #[test]
    fn touchpad_scroll_routes_to_sidebar_under_pointer_and_retains_fractional_deltas() {
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(800, 600, 1.);
        ui.init(&mut ctx);
        let mut sidebar = widget();
        sidebar.limiter_mut().set_max_width(380);
        sidebar.view.sources = (0..50)
            .map(|i| MaterialSourceView {
                id: format!("source-{i}"),
                label: format!("Source {i}"),
                category: "Noise".into(),
            })
            .collect();
        let mut right = TheCanvas::default();
        right.set_widget(sidebar);
        ui.canvas.set_right(right);
        let mut center = TheCanvas::default();
        center.set_widget(TheRenderView::new(TheId::named("map")));
        ui.canvas.set_center(center);
        let mut pixels = vec![0; 800 * 600 * 4];
        ui.draw(&mut pixels, &mut ctx);
        ui.hover(10., 10., &mut ctx);
        ctx.ui.set_focus(&TheId::named("map"));
        let (origin, top) = {
            let w = ui
                .get_widget("material")
                .unwrap()
                .as_any()
                .downcast_mut::<MaterialEditorWidget>()
                .unwrap();
            (*w.dim(), w.layout().library_top)
        };
        let (tx, rx) = std::sync::mpsc::channel();
        ctx.ui.state_events_sender = Some(tx);
        ui.mouse_coord = Vec2::new(origin.x + 20, origin.y + top as i32 + 12);
        for _ in 0..80 {
            assert!(ui.precise_scroll((0., -0.25), &mut ctx));
        }
        let w = ui
            .get_widget("material")
            .unwrap()
            .as_any()
            .downcast_mut::<MaterialEditorWidget>()
            .unwrap();
        assert!(w.library_scroll >= 1, "subpixel deltas must accumulate");
        assert_eq!(ctx.ui.hover.as_ref().unwrap().name, "material");
        assert_eq!(
            ctx.ui.focus.as_ref().unwrap().name,
            "map",
            "scrolling must not steal keyboard focus"
        );
        assert!(!rx.try_iter().any(|event| matches!(
            event,
            TheEvent::RenderViewPreciseScrollBy(..) | TheEvent::RenderViewScrollBy(..)
        )));
        let w = ui
            .get_widget("material")
            .unwrap()
            .as_any()
            .downcast_mut::<MaterialEditorWidget>()
            .unwrap();
        let template = w.view.rows[0].clone();
        w.view.rows = (0..10)
            .map(|i| {
                let mut row = template.clone();
                row.id = format!("row-{i}");
                row
            })
            .collect();
        ui.mouse_coord = Vec2::new(origin.x + 100, origin.y + 100);
        ui.precise_scroll((0., -25.), &mut ctx);
        let w = ui
            .get_widget("material")
            .unwrap()
            .as_any()
            .downcast_mut::<MaterialEditorWidget>()
            .unwrap();
        assert!(w.pan[1] > 0.);
    }
    #[test]
    fn input_terminal_click_disconnects_but_drag_connects_and_cancellation_is_safe() {
        let mut w = widget();
        w.view.outputs[0].row = Some("color".into());
        let input = center(w.layout().output(0));
        let output = center(w.layout().row_port(0, w.pan));
        w.press(input);
        w.release(input);
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::DisconnectOutput("base color".into())]
        );
        w.press(input);
        w.drag(output);
        w.release(output);
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::ConnectOutput {
                output: "base color".into(),
                row: "grain".into()
            }]
        );
        w.press(input);
        w.cancel_gesture();
        w.release(input);
        assert!(w.take_commands().is_empty());
        w.view.outputs[0].row = None;
        w.press(input);
        w.release(input);
        assert!(w.take_commands().is_empty());
    }
    #[test]
    fn source_click_selects_for_preview_while_drag_still_adds_source() {
        let mut w = widget();
        let point = [30., w.layout().library_top + 12.];
        w.press(point);
        w.release(point);
        assert_eq!(w.selected_source.as_deref(), Some("perlin"));
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::InspectSource("perlin".into())]
        );
        let properties = center(w.layout().pane_tab(0));
        w.press(properties);
        w.release(properties);
        assert_eq!(w.pane, MaterialPane::Properties);
        w.take_commands();
        let sources = center(w.layout().pane_tab(1));
        w.press(sources);
        w.release(sources);
        assert_eq!(w.pane, MaterialPane::Sources);
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::InspectSource("perlin".into())]
        );
        let target = center(w.layout().new_row(w.view.rows.len(), w.pan));
        w.press(point);
        w.drag(target);
        w.release(target);
        assert!(
            matches!(w.take_commands().as_slice(),[MaterialEditorCommand::AddSource {source,row:None,..}] if source=="perlin")
        );
    }
    #[test]
    fn empty_slots_and_new_row_are_visible_tile_sized_drop_targets() {
        let mut w = widget();
        let layout = w.layout();
        let empty = layout.card(0, 1, w.pan);
        let new = layout.new_row(w.view.rows.len(), w.pan);
        assert_eq!(new.size, [layout.card_width(), CARD]);
        assert!(matches!(
            w.hit(center(empty)),
            Some(Hit::Drop { row: Some(_), .. })
        ));
        assert!(matches!(
            w.hit(center(new)),
            Some(Hit::Drop { row: None, .. })
        ));
        let theme = GraphTheme::default();
        let mut buffer = TheRGBABuffer::new(TheDim::new(0, 0, 380, 800));
        w.paint(&mut buffer, &theme);
        for area in [empty, new] {
            let at = |x: f32, y: f32| {
                let i = (y as usize * 380 + x as usize) * 4;
                &buffer.pixels()[i..i + 4]
            };
            assert_eq!(at(area.origin[0] + 6., area.origin[1]), theme.control);
            assert_eq!(
                at(area.origin[0] + CARD / 2., area.origin[1] + CARD / 2.),
                theme.background
            );
        }
    }
    #[test]
    fn a_full_line_keeps_an_outlined_append_slot_in_the_same_material_row() {
        let mut w = widget();
        let operation = w.view.rows[0].operations[0].clone();
        w.view.rows[0].operations = (0..COLUMNS)
            .map(|i| {
                let mut operation = operation.clone();
                operation.id = format!("op-{i}");
                operation
            })
            .collect();
        let target = center(w.layout().card(0, COLUMNS, w.pan));
        assert!(
            matches!(w.hit(target),Some(Hit::Drop {row:Some(ref row),after:Some(ref after)}) if row=="grain" && after=="op-2")
        );
        let point = [30., w.layout().library_top + 12.];
        w.press(point);
        w.drag(target);
        w.release(target);
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::AddSource {
                source: "perlin".into(),
                row: Some("grain".into()),
                after: Some("op-2".into())
            }]
        );
    }
    #[test]
    fn native_ranged_property_fields_support_drag_and_manual_entry() {
        let mut w = widget();
        w.selected = Some("a".into());
        w.pane = MaterialPane::Properties;
        let mut ctx = TheContext::new(380, 800, 1.);
        let mut pixels = TheRGBABuffer::new(TheDim::new(0, 0, 380, 800));
        let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::new());
        w.draw(&mut pixels, &mut style, &mut ctx);
        let dim = *w.number_edits[0].2.dim();
        let start = Vec2::new(dim.x + dim.width / 2, dim.y + 5);
        w.on_event(&TheEvent::MouseDown(start), &mut ctx);
        w.on_event(
            &TheEvent::MouseDragged(Vec2::new(dim.x + dim.width, dim.y + 5)),
            &mut ctx,
        );
        w.on_event(&TheEvent::MouseUp(start), &mut ctx);
        assert!(w.supports_text_input());
        assert_eq!(w.number_edits[0].2.value().to_f32(), Some(1.));
        w.on_event(
            &TheEvent::ModifierChanged(false, true, false, false),
            &mut ctx,
        );
        w.on_event(&TheEvent::KeyDown(TheValue::Char('a')), &mut ctx);
        w.on_event(
            &TheEvent::ModifierChanged(false, false, false, false),
            &mut ctx,
        );
        for c in "0.375".chars() {
            w.on_event(&TheEvent::KeyDown(TheValue::Char(c)), &mut ctx);
        }
        w.draw(&mut pixels, &mut style, &mut ctx);
        w.on_event(
            &TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Return)),
            &mut ctx,
        );
        let (owner, parameter, edit) = &w.number_edits[0];
        assert_eq!((owner.as_str(), parameter.as_str()), ("a", "scale"));
        assert_eq!(
            w.number_command(edit.id(), &edit.value()),
            Some(MaterialEditorCommand::SetNumber {
                operation: "a".into(),
                parameter: "scale".into(),
                value: 0.375
            })
        );
        assert!(
            w.take_commands().is_empty(),
            "Typing must not delete graph operations"
        );
        w.parameter_scroll = 1;
        w.sync_number_edits(&mut ctx);
        assert!(w.number_edits.is_empty());
    }
    fn with_inputs() -> MaterialEditorWidget {
        let mut w = widget();
        w.view.rows[1].inputs = vec![
            MaterialInputView {
                target: MaterialInputTarget::Coordinates {
                    row: "color".into(),
                },
                label: "Coordinates (UV / XYZ)".into(),
                binding: MaterialInputBinding::Default,
                components: "xy".into(),
            },
            MaterialInputView {
                target: MaterialInputTarget::Image {
                    operation: "b".into(),
                },
                label: "Image · Mix".into(),
                binding: MaterialInputBinding::Row(MaterialReferenceView {
                    row: "grain".into(),
                    components: "xyz".into(),
                    mapping: None,
                }),
                components: "xyz".into(),
            },
            MaterialInputView {
                target: MaterialInputTarget::Parameter {
                    operation: "b".into(),
                    parameter: "factor".into(),
                },
                label: "Factor · Mix".into(),
                binding: MaterialInputBinding::Default,
                components: "xyz".into(),
            },
        ];
        w
    }
    #[test]
    fn row_input_tab_edits_stable_targets_components_and_remap_without_optimistic_mutation() {
        let mut w = with_inputs();
        let input = center(w.layout().row_input(1, w.pan));
        w.press(input);
        assert_eq!(w.pane, MaterialPane::RowInput);
        assert_eq!(w.selected_row.as_deref(), Some("color"));
        assert_eq!(
            w.take_commands(),
            vec![
                MaterialEditorCommand::InspectRow("color".into()),
                MaterialEditorCommand::InspectInput {
                    target: MaterialInputTarget::Coordinates {
                        row: "color".into()
                    },
                    binding: MaterialInputBinding::Default
                }
            ]
        );
        w.choose_input_option(1, 1);
        assert!(matches!(
            w.take_commands().as_slice(),
            [MaterialEditorCommand::InspectInput {
                target: MaterialInputTarget::Image { .. },
                binding: MaterialInputBinding::Row(_)
            }]
        ));
        w.choose_input_option(2, 2);
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::SetInput {
                target: MaterialInputTarget::Image {
                    operation: "b".into()
                },
                binding: MaterialInputBinding::Row(MaterialReferenceView {
                    row: "grain".into(),
                    components: "xyz".into(),
                    mapping: None
                })
            }]
        );
        w.press(center(w.layout().input_control(4)));
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::SetInput {
                target: MaterialInputTarget::Image {
                    operation: "b".into()
                },
                binding: MaterialInputBinding::Row(MaterialReferenceView {
                    row: "grain".into(),
                    components: "xyz".into(),
                    mapping: Some([0., 1.])
                })
            }]
        );
        assert_eq!(
            w.active_input().unwrap().binding,
            MaterialInputBinding::Row(MaterialReferenceView {
                row: "grain".into(),
                components: "xyz".into(),
                mapping: None
            })
        );
        w.view.rows[1].inputs[1].binding = MaterialInputBinding::Row(MaterialReferenceView {
            row: "grain".into(),
            components: "xyz".into(),
            mapping: Some([0., 1.]),
        });
        let mut ctx = TheContext::new(380, 800, 1.);
        w.sync_number_edits(&mut ctx);
        assert_eq!(w.number_edits.len(), 3);
        let (_, _, edit) = &w.number_edits[0];
        let command = w
            .number_command(edit.id(), &TheValue::Text("distance".into()))
            .unwrap();
        assert_eq!(
            command,
            MaterialEditorCommand::SetInput {
                target: MaterialInputTarget::Image {
                    operation: "b".into()
                },
                binding: MaterialInputBinding::Row(MaterialReferenceView {
                    row: "grain".into(),
                    components: "distance".into(),
                    mapping: Some([0., 1.])
                })
            }
        );
        let (_, _, edit) = &w.number_edits[1];
        assert_eq!(
            w.number_command(edit.id(), &TheValue::Float(-2.)),
            Some(MaterialEditorCommand::SetInput {
                target: MaterialInputTarget::Image {
                    operation: "b".into()
                },
                binding: MaterialInputBinding::Row(MaterialReferenceView {
                    row: "grain".into(),
                    components: "xyz".into(),
                    mapping: Some([-2., 1.])
                })
            })
        );
    }
    #[test]
    fn selected_row_highlights_incoming_and_outgoing_connections_and_dims_others() {
        let mut w = with_inputs();
        w.view.outputs[0].row = Some("color".into());
        let mut unrelated = w.view.outputs[0].clone();
        unrelated.id = "roughness".into();
        unrelated.row = Some("grain".into());
        w.view.outputs.push(unrelated);
        w.selected_row = Some("color".into());
        let paths = w.connection_paths(&w.layout());
        let theme = material_editor_theme(&TheBrownTheme::new());
        assert_eq!(paths.len(), 3);
        assert_eq!(w.connection_style(&paths[0], &theme), (theme.active, 1.8));
        assert_eq!(w.connection_style(&paths[2], &theme), (theme.active, 1.8));
        let (dimmed, width) = w.connection_style(&paths[1], &theme);
        assert_eq!(width, 1.);
        assert_ne!(dimmed, theme.active);
        for i in 0..3 {
            assert!(
                dimmed[i].abs_diff(theme.background[i])
                    >= theme.wire[i].abs_diff(theme.background[i])
            );
        }
        w.selected_row = None;
        assert_eq!(w.connection_style(&paths[1], &theme), (theme.wire, 0.8));
    }
    #[test]
    fn clicking_row_connection_selects_first_source_tile_and_updates_preview() {
        let mut w = with_inputs();
        let first = w.view.rows[0].operations[0].clone();
        let mut second = first.clone();
        second.id = "second".into();
        w.view.rows[0].operations.push(second);
        w.selected = Some("b".into());
        w.selected_row = Some("color".into());
        let l = w.layout();
        let paths = w.connection_paths(&l);
        let point = [
            paths[0].points[2][0],
            (paths[0].points[2][1] + paths[0].points[3][1]) / 2.,
        ];
        assert_eq!(w.hit(point), Some(Hit::Connection("grain".into())));
        w.press(point);
        w.release(point);
        assert_eq!(w.selected_row.as_deref(), Some("grain"));
        assert_eq!(w.selected_operation(), Some(first.id.as_str()));
        assert_eq!(w.pane, MaterialPane::Properties);
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::InspectOperation(first.id)]
        );
    }
    #[test]
    fn output_connection_click_scrolls_to_offscreen_source_without_changing_recipe() {
        let mut w = with_inputs();
        let template = w.view.rows[0].clone();
        for index in 0..10 {
            let mut row = template.clone();
            row.id = format!("row-{index}");
            row.operations[0].id = format!("tile-{index}");
            w.view.rows.push(row);
        }
        w.view.outputs[0].row = Some("row-9".into());
        w.selected_row = Some("color".into());
        let l = w.layout();
        let paths = w.connection_paths(&l);
        let output_path = &paths[0];
        assert!(output_path.points[0][1] > l.editor_bottom);
        let segments = output_path.segments(l.editor_bottom);
        assert!(!segments.is_empty());
        assert!(
            segments
                .iter()
                .flatten()
                .all(|point| point[1] >= HEADER && point[1] < l.editor_bottom)
        );
        let point = [output_path.points[2][0], 100.];
        assert_eq!(w.hit(point), Some(Hit::Connection("row-9".into())));
        w.press(point);
        w.release(point);
        assert_eq!(w.selected_operation(), Some("tile-9"));
        assert!(w.pan[1] > 0.);
        let l = w.layout();
        let card = l.card(w.view.rows.len() - 1, 0, w.pan);
        assert!(card.origin[1] >= HEADER && card.origin[1] + CARD <= l.editor_bottom);
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::InspectOperation("tile-9".into())]
        );
        assert_eq!(w.view.outputs[0].row.as_deref(), Some("row-9"));
    }
    #[test]
    fn row_connections_use_reserved_left_gutter_and_all_three_cards_fit() {
        let mut w = with_inputs();
        w.view.outputs[0].row = Some("color".into());
        for width in [350, 380, 480] {
            w.dim.width = width;
            let l = w.layout();
            let last = l.card(1, COLUMNS - 1, w.pan);
            assert!(last.origin[0] + last.size[0] <= width as f32 - 18.);
            let paths = w.connection_paths(&l);
            assert_eq!(paths.len(), 2);
            assert_ne!(paths[0].points[2][0], paths[1].points[2][0]);
            for path in &paths {
                assert!(path.points[2][0] > RAIL && path.points[2][0] < CARD_LEFT);
                assert!(path.points[3][0] > RAIL && path.points[3][0] < CARD_LEFT);
            }
        }
        let l = w.layout();
        w.press(center(l.row_port(0, w.pan)));
        w.release(center(l.row_input(1, w.pan)));
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::SetInput {
                target: MaterialInputTarget::Image {
                    operation: "b".into()
                },
                binding: MaterialInputBinding::Row(MaterialReferenceView {
                    row: "grain".into(),
                    components: "xyz".into(),
                    mapping: None
                })
            }]
        );
        assert_eq!(w.selected_input, 1);
    }
    #[test]
    fn palette_swatches_keep_sparse_real_indices_and_never_show_raw_component_sliders() {
        let mut w = widget();
        w.view.rows[0].operations[0].palette = Some(MaterialPaletteView { index: Some(240) });
        w.view.palette = vec![
            MaterialPaletteEntry {
                index: 7,
                label: "Grass".into(),
                color: [10, 120, 40, 255],
            },
            MaterialPaletteEntry {
                index: 240,
                label: "Water".into(),
                color: [20, 40, 100, 128],
            },
        ];
        w.selected = Some("a".into());
        w.pane = MaterialPane::Properties;
        assert!(w.parameters().2.is_empty());
        w.press(center(w.palette_rect(1)));
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::SetPalette {
                operation: "a".into(),
                index: 240
            }]
        );
        let mut buffer = TheRGBABuffer::new(TheDim::new(0, 0, 380, 800));
        w.paint(&mut buffer, &GraphTheme::default());
        assert!(buffer.pixels().chunks_exact(4).all(|p| p[3] == 255));
    }
    #[test]
    fn row_input_native_controls_render_and_open_popups_in_small_sidebar() {
        let mut w = with_inputs();
        w.press(center(w.layout().row_input(1, w.pan)));
        w.selected_input = 1;
        let mut ctx = TheContext::new(380, 800, 1.);
        let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::new());
        let mut buffer = TheRGBABuffer::new(TheDim::new(0, 0, 380, 800));
        w.draw(&mut buffer, &mut style, &mut ctx);
        assert_eq!(w.number_edits.len(), 1);
        let d = *w.number_edits[0].2.dim();
        assert!(d.x > 0 && d.x + d.width <= 380);
        assert!(d.y >= w.layout().editor_bottom as i32);
        w.press(center(w.layout().input_control(2)));
        assert_eq!(w.input_popup, Some(2));
        w.draw(&mut buffer, &mut style, &mut ctx);
        let area = w.popup_area().unwrap();
        assert!(area.origin[1] + area.size[1] <= 800.);
        w.press([area.origin[0] + 5., area.origin[1] + 2. * 22. + 10.]);
        assert!(w.input_popup.is_none());
        assert!(
            w.take_commands()
                .iter()
                .any(|command| matches!(command, MaterialEditorCommand::SetInput { .. }))
        );
    }
    #[test]
    fn metadata_controls_use_choices_toggles_grouped_vectors_and_palette_colors() {
        let mut w = widget();
        w.selected = Some("a".into());
        w.pane = MaterialPane::Properties;
        let op = &mut w.view.rows[0].operations[0];
        op.parameters = (0..8)
            .map(|i| MaterialNumberView {
                id: format!("p{i}"),
                label: format!("P{i}"),
                value: 0.25,
                min: 0.,
                max: 1.,
            })
            .collect();
        op.properties = vec![
            MaterialPropertyView {
                id: "mode".into(),
                label: "Mode".into(),
                group: "Mix".into(),
                kind: MaterialPropertyKind::Choice,
                components: vec!["p0".into()],
                choices: vec!["Multiply".into(), "Overlay".into()],
                tooltip: "Blend mode".into(),
            },
            MaterialPropertyView {
                id: "enabled".into(),
                label: "Enabled".into(),
                group: "Mix".into(),
                kind: MaterialPropertyKind::Boolean,
                components: vec!["p1".into()],
                choices: vec![],
                tooltip: "Enable effect".into(),
            },
            MaterialPropertyView {
                id: "stretch".into(),
                label: "Stretch XY".into(),
                group: "Coordinates".into(),
                kind: MaterialPropertyKind::Vector,
                components: vec!["p2".into(), "p3".into()],
                choices: vec![],
                tooltip: String::new(),
            },
            MaterialPropertyView {
                id: "color".into(),
                label: "Tint".into(),
                group: "Color".into(),
                kind: MaterialPropertyKind::Color,
                components: (4..8).map(|i| format!("p{i}")).collect(),
                choices: vec![],
                tooltip: String::new(),
            },
        ];
        w.view.palette = vec![MaterialPaletteEntry {
            index: 7,
            label: "Red".into(),
            color: [255, 0, 0, 128],
        }];
        assert!(matches!(&w.property_lines()[0],PropertyLine::Section(group) if group=="Mix"));
        w.press(center(w.layout().parameter(1)));
        assert_eq!(w.input_popup, Some(1001));
        assert_eq!(w.input_options(1001), vec!["Multiply", "Overlay"]);
        w.choose_input_option(1001, 1);
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::SetNumber {
                operation: "a".into(),
                parameter: "p0".into(),
                value: 1.
            }]
        );
        w.press(center(w.layout().parameter(2)));
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::SetNumber {
                operation: "a".into(),
                parameter: "p1".into(),
                value: 0.
            }]
        );
        let mut ctx = TheContext::new(380, 800, 1.);
        w.sync_number_edits(&mut ctx);
        assert_eq!(
            w.number_edits.len(),
            2,
            "Choice, bool and palette colors must never become numeric slider fields"
        );
        assert_eq!(w.number_edits[0].1, "p2");
        assert_eq!(w.number_edits[1].1, "p3");
        assert_eq!(
            w.number_edits[0].2.dim().y,
            w.number_edits[1].2.dim().y,
            "Vector components are grouped horizontally"
        );
        w.choose_input_option(1006, 0);
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::SetPaletteParameter {
                operation: "a".into(),
                parameter: "color".into(),
                index: 7
            }]
        );
        w.color_property = Some(6);
        assert_eq!(
            w.active_palette()
                .map(|(owner, parameter, _)| (owner, parameter)),
            Some(("a".into(), "color".into()))
        );
        w.press(center(w.palette_rect(0)));
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::SetPalette {
                operation: "a".into(),
                index: 7
            }]
        );
    }
    #[test]
    fn tiling_and_shape_combine_commands_preserve_document_until_host_validation() {
        let mut w = widget();
        w.pane = MaterialPane::Properties;
        w.choose_input_option(1001, 2);
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::SetTiling {
                mode: "seamless".into(),
                period: [1., 1.],
                blend: 0.15
            }]
        );
        assert_eq!(w.view.tiling.mode, "raw");
        w.view.tiling.mode = "seamless".into();
        let mut ctx = TheContext::new(380, 800, 1.);
        w.sync_number_edits(&mut ctx);
        let (_, _, edit) = &w.number_edits[0];
        assert_eq!(
            w.number_command(edit.id(), &TheValue::Float(2.)),
            Some(MaterialEditorCommand::SetTiling {
                mode: "seamless".into(),
                period: [2., 1.],
                blend: 0.15
            })
        );
        w.view.rows[0].operations[0].combine = Some("replace".into());
        w.selected = Some("a".into());
        w.choose_input_option(1000, 4);
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::SetCombine {
                operation: "a".into(),
                mode: "smooth union".into()
            }]
        );
    }
    #[test]
    fn retained_row_links_can_preview_edit_channels_and_be_assigned_to_a_new_receiver() {
        let mut w = with_inputs();
        let reference = MaterialReferenceView {
            row: "grain".into(),
            components: "xyz".into(),
            mapping: None,
        };
        w.view.rows[1].inputs.push(MaterialInputView {
            target: MaterialInputTarget::RowLink {
                row: "color".into(),
                connection: "wire-palette".into(),
            },
            label: "Unassigned · Grain".into(),
            binding: MaterialInputBinding::Row(reference.clone()),
            components: "xyz".into(),
        });
        w.selected_row = Some("color".into());
        w.pane = MaterialPane::RowInput;
        w.selected_input = 3;
        assert_eq!(
            w.selected_input_preview(),
            Some((
                MaterialInputTarget::RowLink {
                    row: "color".into(),
                    connection: "wire-palette".into()
                },
                reference
            ))
        );
        assert_eq!(w.input_options(7).len(), 3);
        w.choose_input_option(7, 2);
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::AssignRowInput {
                row: "color".into(),
                connection: "wire-palette".into(),
                target: MaterialInputTarget::Parameter {
                    operation: "b".into(),
                    parameter: "factor".into()
                }
            }]
        );
        w.choose_input_option(3, 7);
        assert!(
            matches!(w.take_commands().as_slice(),[MaterialEditorCommand::SetInput{target:MaterialInputTarget::RowLink{..},binding:MaterialInputBinding::Row(reference)}] if reference.components=="distance")
        );
    }
    #[test]
    fn native_source_editor_applies_draft_and_exposes_help_without_mutating_graph() {
        let mut w = widget();
        w.selected = Some("a".into());
        w.pane = MaterialPane::Tile;
        w.view.rows[0].operations[0].source = Some(MaterialSourceCodeView {
            code: "return uv.x;".into(),
            help: "Noise help".into(),
            references: "Reference text".into(),
        });
        let mut ctx = TheContext::new(380, 800, 1.);
        let mut pixels = TheRGBABuffer::new(TheDim::new(0, 0, 380, 800));
        let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::new());
        w.draw(&mut pixels, &mut style, &mut ctx);
        let dim = *w.source_edit.as_ref().unwrap().1.dim();
        w.on_event(
            &TheEvent::MouseDown(Vec2::new(dim.x + 5, dim.y + 5)),
            &mut ctx,
        );
        assert!(w.supports_text_input());
        assert_eq!(ctx.ui.focus, Some(w.id.clone()));
        w.source_edit
            .as_mut()
            .unwrap()
            .1
            .set_text("return uv.y;".into());
        w.press(center(w.source_action_rect(0)));
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::SetSource {
                operation: "a".into(),
                code: "return uv.y;".into()
            }]
        );
        assert_eq!(
            w.view.rows[0].operations[0].source.as_ref().unwrap().code,
            "return uv.x;"
        );
        w.source_action(3);
        w.sync_source_edit(&mut ctx);
        assert!(
            w.source_edit
                .as_ref()
                .unwrap()
                .1
                .text()
                .contains("Noise help")
        );
        w.source_action(2);
        assert_eq!(
            w.take_commands(),
            vec![MaterialEditorCommand::SetSource {
                operation: "a".into(),
                code: String::new()
            }]
        );
    }
    #[test]
    fn source_categories_are_derived_from_catalog_and_combine_with_search() {
        let mut w = widget();
        assert_eq!(w.input_options(8), vec!["All", "Color", "Noise"]);
        w.press(center(w.layout().category()));
        assert_eq!(w.input_popup, Some(8));
        w.choose_input_option(8, 2);
        assert_eq!(w.sources().len(), 1);
        assert_eq!(w.sources()[0].id, "perlin");
        w.set_source_filter("Mix");
        assert!(w.sources().is_empty());
        w.choose_input_option(8, 0);
        assert_eq!(w.sources()[0].id, "mix");
    }
}
