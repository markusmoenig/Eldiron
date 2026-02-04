use crate::{
    AssignmentOp, Cell, DebugModule, Grid, GridCtx, ModuleType,
    cell::{ArithmeticOp, CellRole, ComparisonOp},
};
use Cell::*;
use rusteria::PatternKind;
use strum::IntoEnumIterator;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub enum CellItemForm {
    Box,
    #[default]
    Rounded,
    LeftRounded,
    RightRounded,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub enum CellItemSpecialRole {
    #[default]
    None,
    DealDamageValue,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CellItem {
    pub id: Uuid,
    pub cell: Cell,
    pub has_error: bool,

    pub dependend_on: Option<Uuid>,
    pub replaceable: bool,
    pub description: String,

    pub form: CellItemForm,

    #[serde(default)]
    pub option: i32,

    #[serde(default)]
    pub special_role: CellItemSpecialRole,
}

impl CellItem {
    pub fn new(cell: Cell) -> Self {
        Self {
            id: Uuid::new_v4(),
            cell,
            has_error: false,

            dependend_on: None,
            replaceable: true,
            description: String::new(),
            form: CellItemForm::default(),

            option: 0,
            special_role: CellItemSpecialRole::None,
        }
    }

    pub fn new_dependency(
        cell: Cell,
        dependend_on: Uuid,
        replaceable: bool,
        description: &str,
        form: CellItemForm,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            cell,
            has_error: false,

            dependend_on: Some(dependend_on),
            replaceable,
            description: description.to_string(),
            form,

            option: 0,
            special_role: CellItemSpecialRole::None,
        }
    }

    /// Draw the cell
    pub fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        rect: &TheDim,
        ctx: &TheContext,
        grid_ctx: &GridCtx,
        is_selected: bool,
        pos: &(u32, u32),
        event: &str,
        id: u32,
        debug: Option<&DebugModule>,
    ) {
        let font_size = 12.5;
        let large_font_size = 14.0;
        let background_color = [116, 116, 116, 255];
        // let normal_color = [174, 174, 174, 255];
        // let dark_color = [74, 74, 74, 255];
        let selection_color = [187, 122, 208, 255];
        let text_color = [85, 81, 85, 255];
        let highlight_text_color = [242, 242, 242, 255];
        let error_color = [209, 42, 42, 255];

        let stride = buffer.dim().width as usize;
        let color = if self.has_error {
            &error_color
        } else if is_selected {
            &selection_color
        } else {
            // &grid_ctx.normal_color
            &self.cell.role().to_color()
        };
        let zoom = 5.0;
        let rounding = 2.0 * zoom;
        match &self.cell {
            Cell::Variable(name) => {
                let text = match self.option {
                    1 => {
                        format!("First({})", name)
                    }
                    2 => {
                        format!("Length({})", name)
                    }
                    _ => self.cell.to_string(),
                };
                ctx.draw.rounded_rect(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &color,
                    &self.rounding(rounding),
                );

                let r = rect.to_buffer_utuple();
                ctx.draw.text_rect_blend(
                    buffer.pixels_mut(),
                    &(
                        r.0,
                        r.1,
                        r.2,
                        r.3 - if self.description.is_empty() { 0 } else { 10 },
                    ),
                    stride,
                    &text,
                    TheFontSettings {
                        size: font_size,
                        ..Default::default()
                    },
                    &text_color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );

                if !self.description.is_empty() {
                    let r = rect.to_buffer_utuple();
                    ctx.draw.text_rect_blend(
                        buffer.pixels_mut(),
                        &(r.0, r.1 + 15, r.2, r.3),
                        stride,
                        &self.description,
                        TheFontSettings {
                            size: font_size,
                            ..Default::default()
                        },
                        &highlight_text_color,
                        TheHorizontalAlign::Center,
                        TheVerticalAlign::Center,
                    );
                }
            }
            Cell::Comparison(_) | Cell::If | Arithmetic(_) | Cell::Else => {
                ctx.draw.text_rect_blend(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &self.cell.to_string(),
                    TheFontSettings {
                        size: font_size * 2.0,
                        ..Default::default()
                    },
                    color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }
            Cell::Assignment => {
                let text = if let Some(op) = AssignmentOp::from_index(self.option) {
                    op.describe().to_string()
                } else {
                    "=".to_string()
                };

                ctx.draw.text_rect_blend(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &text,
                    TheFontSettings {
                        size: font_size * 2.0,
                        ..Default::default()
                    },
                    color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }
            Cell::LeftParent | Cell::RightParent => {
                ctx.draw.text_rect_blend(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &self.cell.to_string(),
                    TheFontSettings {
                        size: font_size * 2.0 + 10.0 * grid_ctx.zoom,
                        ..Default::default()
                    },
                    color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }
            Cell::Integer(_)
            | Cell::Float(_)
            | Cell::Str(_)
            | Cell::Boolean(_)
            | Cell::Textures(_)
            | PaletteColor(_)
            | Value(_) => {
                ctx.draw.rounded_rect(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &color,
                    &self.rounding(rounding),
                );

                let r = rect.to_buffer_utuple();
                ctx.draw.text_rect_blend(
                    buffer.pixels_mut(),
                    &(
                        r.0,
                        r.1,
                        r.2,
                        r.3 - if self.description.is_empty() { 0 } else { 10 },
                    ),
                    stride,
                    &self.cell.to_string(),
                    TheFontSettings {
                        size: font_size,
                        ..Default::default()
                    },
                    &text_color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );

                if !self.description.is_empty() {
                    let r = rect.to_buffer_utuple();
                    ctx.draw.text_rect_blend(
                        buffer.pixels_mut(),
                        &(r.0, r.1 + 15, r.2, r.3),
                        stride,
                        &self.description,
                        TheFontSettings {
                            size: font_size,
                            ..Default::default()
                        },
                        &highlight_text_color,
                        TheHorizontalAlign::Center,
                        TheVerticalAlign::Center,
                    );
                }
            }
            Empty => {
                let mut shrinker = TheDimShrinker::zero();
                shrinker.shrink(4);
                ctx.draw.rounded_rect_with_border(
                    buffer.pixels_mut(),
                    &rect.to_buffer_shrunk_utuple(&shrinker),
                    stride,
                    &background_color,
                    &self.rounding(rounding),
                    &color,
                    1.5,
                );
            }
            _ => {
                // Function Header

                ctx.draw.rounded_rect(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &color,
                    &self.rounding(rounding),
                );

                let r = rect.to_buffer_utuple();
                let mut has_debug = false;
                if let Some(debug) = debug {
                    if let Some(value) = debug.get_value(id, event, pos.0, pos.1) {
                        let color = if debug.has_error(id, event, pos.0, pos.1) {
                            &error_color
                        } else {
                            &highlight_text_color
                        };
                        has_debug = true;
                        ctx.draw.text_rect_blend(
                            buffer.pixels_mut(),
                            &(r.0, r.1 + 15, r.2, r.3),
                            stride,
                            &value.describe(),
                            TheFontSettings {
                                size: font_size,
                                ..Default::default()
                            },
                            color,
                            TheHorizontalAlign::Center,
                            TheVerticalAlign::Center,
                        );
                    }
                }

                ctx.draw.text_rect_blend(
                    buffer.pixels_mut(),
                    &(r.0, r.1, r.2, r.3 - if !has_debug { 0 } else { 10 }),
                    stride,
                    &self.cell.to_string(),
                    TheFontSettings {
                        size: large_font_size,
                        ..Default::default()
                    },
                    &text_color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            } // _ => {
              //     buffer.draw_rect_outline(
              //         rect,
              //         if is_selected {
              //             &grid_ctx.selection_color
              //         } else {
              //             &grid_ctx.dark_color
              //         },
              //     );
              // }
        }
    }

    /// Returns the size of the cell
    pub fn size(
        &self,
        ctx: &TheContext,
        _grid_ctx: &GridCtx,
        pos: &(u32, u32),
        event: &str,
        id: u32,
        debug: Option<&DebugModule>,
    ) -> Vec2<u32> {
        let font_size = 12.5;
        let large_font_size = 14.0;
        let mut size = Vec2::new(30, 50);
        match &self.cell {
            Variable(_) | Integer(_) | Float(_) | Str(_) | Boolean(_) | Textures(_)
            | PaletteColor(_) | Value(_) => {
                let text = match self.option {
                    1 => {
                        format!("First({})", self.cell.to_string())
                    }
                    2 => {
                        format!("Length({})", self.cell.to_string())
                    }
                    _ => self.cell.to_string(),
                };
                size.x = ctx
                    .draw
                    .get_text_size(
                        &text,
                        &TheFontSettings {
                            size: font_size,
                            ..Default::default()
                        },
                    )
                    .0 as u32
                    + 20;
                size.x = size.x.min(200);

                if !self.description.is_empty() {
                    let desc = ctx
                        .draw
                        .get_text_size(
                            &self.description,
                            &TheFontSettings {
                                size: font_size,
                                ..Default::default()
                            },
                        )
                        .0 as u32
                        + 20;
                    size.x = size.x.max(desc);
                }
            }
            Assignment => {
                if let Some(op) = AssignmentOp::from_index(self.option) {
                    size.x = ctx
                        .draw
                        .get_text_size(
                            op.describe(),
                            &TheFontSettings {
                                size: font_size,
                                ..Default::default()
                            },
                        )
                        .0 as u32
                        + 20;
                } else {
                    size.x = ctx
                        .draw
                        .get_text_size(
                            "=",
                            &TheFontSettings {
                                size: font_size,
                                ..Default::default()
                            },
                        )
                        .0 as u32
                        + 20;
                }
            }
            If | Else | Comparison(_) => {
                size.x = ctx
                    .draw
                    .get_text_size(
                        &self.cell.to_string(),
                        &TheFontSettings {
                            size: font_size * 2.0,
                            ..Default::default()
                        },
                    )
                    .0 as u32
                    + 10;
                if matches!(self.cell, Cell::Else) {
                    size.y = 30;
                }
            }
            LeftParent | RightParent => {
                size.x = ctx
                    .draw
                    .get_text_size(
                        &self.cell.to_string(),
                        &TheFontSettings {
                            size: font_size * 2.0,
                            ..Default::default()
                        },
                    )
                    .0 as u32
                    + 10;
            }
            Empty => {}
            _ => {
                size.x = ctx
                    .draw
                    .get_text_size(
                        &self.cell.to_string(),
                        &TheFontSettings {
                            size: large_font_size,
                            ..Default::default()
                        },
                    )
                    .0 as u32
                    + 20;

                if let Some(debug) = debug {
                    if let Some(value) = debug.get_value(id, event, pos.0, pos.1) {
                        let desc = ctx
                            .draw
                            .get_text_size(
                                &value.describe(),
                                &TheFontSettings {
                                    size: font_size,
                                    ..Default::default()
                                },
                            )
                            .0 as u32
                            + 20;
                        size.x = size.x.max(desc);
                    }
                }
            }
        }
        size
    }

    /// Creates the settings for the cell
    pub fn create_settings(&self, palette: &ThePalette, _module_type: ModuleType) -> TheNodeUI {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        match &self.cell {
            Variable(name) => {
                let item = TheNodeUIItem::Text(
                    "cgfxVariableName".into(),
                    "Name".into(),
                    "Set the name of the variable".into(),
                    name.clone(),
                    None,
                    false,
                );
                nodeui.add_item(item);

                /*
                if !module_type.is_shader() {
                    let item = TheNodeUIItem::Selector(
                        "cgfxVariableOption".into(),
                        "Access".into(),
                        "Select the access mode of the variable".into(),
                        vec![
                            "Self".to_string(),
                            "List: First Item".to_string(),
                            "List: Length".to_string(),
                        ],
                        self.option,
                    );
                    nodeui.add_item(item);
                }*/
            }
            Integer(value) | Float(value) => {
                let item = TheNodeUIItem::Text(
                    "cgfxValue".into(),
                    "Value".into(),
                    "Set the value".into(),
                    value.clone(),
                    None,
                    false,
                );
                nodeui.add_item(item);
            }
            Str(value) | Value(value) => {
                let item = TheNodeUIItem::Text(
                    "cgfxValue".into(),
                    "Value".into(),
                    "Set the value".into(),
                    value.clone(),
                    None,
                    false,
                );
                nodeui.add_item(item);
            }
            &Boolean(value) => {
                let item = TheNodeUIItem::Selector(
                    "cgfxValue".into(),
                    "Value".into(),
                    "Select the boolean value".into(),
                    vec!["True".to_string(), "False".to_string()],
                    if value { 0 } else { 1 },
                );
                nodeui.add_item(item);
            }
            &PaletteColor(value) => {
                let item = TheNodeUIItem::PaletteSlider(
                    "cgfxValue".into(),
                    "Value".into(),
                    "Set the value".into(),
                    value as i32,
                    palette.clone(),
                    false,
                );
                nodeui.add_item(item);
            }

            Comparison(op) => {
                let item = TheNodeUIItem::Selector(
                    "cgfxComparisonOp".into(),
                    "Operator".into(),
                    "Select the comparison operator".into(),
                    vec![
                        "==".to_string(),
                        "!=".to_string(),
                        "<=".to_string(),
                        ">=".to_string(),
                        "<".to_string(),
                        ">".to_string(),
                    ],
                    op.to_index() as i32,
                );
                nodeui.add_item(item);
            }
            Arithmetic(op) => {
                let item = TheNodeUIItem::Selector(
                    "cgfxArithmeticOp".into(),
                    "Operator".into(),
                    "Select the arithmetic operator".into(),
                    vec![
                        "+".to_string(),
                        "-".to_string(),
                        "*".to_string(),
                        "/".to_string(),
                    ],
                    op.to_index() as i32,
                );
                nodeui.add_item(item);
            }
            Assignment => {
                let item = TheNodeUIItem::Selector(
                    "cgfxAssignmentOp".into(),
                    "Operator".into(),
                    "Select the arithmetic operator".into(),
                    vec![
                        "=".to_string(),
                        "+=".to_string(),
                        "-=".to_string(),
                        "*=".to_string(),
                        "/=".to_string(),
                    ],
                    self.option,
                );
                nodeui.add_item(item);
            }
            Textures(name) => {
                let mut index = 0;
                if let Some(kind) = PatternKind::from_name(name) {
                    index = kind.to_index();
                }

                // building your selector:
                let options: Vec<String> = PatternKind::iter()
                    .map(|k| <&'static str>::from(k).to_string())
                    .collect();

                let item = TheNodeUIItem::Selector(
                    "cgfxPatternKind".into(),
                    "Pattern".into(),
                    "Select the precomputed pattern".into(),
                    options,
                    index as i32,
                );
                nodeui.add_item(item);
            }
            _ => {}
        }

        nodeui
    }

    /// Creates the settings for the cell
    pub fn apply_value(&mut self, name: &str, value: &TheValue, module_type: ModuleType) -> bool {
        match &mut self.cell {
            Variable(var_name) => {
                if let Some(n) = value.to_string()
                    && name == "cgfxVariableName"
                {
                    if !module_type.is_shader() {
                        if Self::is_valid_code_variable(&n) {
                            *var_name = n;
                        }
                    } else {
                        *var_name = n;
                    }
                } else if let Some(v) = value.to_i32()
                    && name == "cgfxVariableOption"
                {
                    self.option = v;
                }
            }
            PaletteColor(index) => {
                if let Some(v) = value.to_i32()
                    && name == "cgfxValue"
                    && v < 256
                {
                    *index = v as u8;
                    return false;
                }
            }
            Integer(value_name) => {
                if let Some(v) = value.to_string()
                    && name == "cgfxValue"
                {
                    self.has_error = !Self::is_valid_integer(&v);
                    *value_name = v;
                }
            }
            Float(value_name) => {
                if let Some(v) = value.to_string()
                    && name == "cgfxValue"
                {
                    self.has_error = !Self::is_valid_float(&v);
                    *value_name = v;
                }
            }
            Str(value_name) | Value(value_name) => {
                if let Some(v) = value.to_string()
                    && name == "cgfxValue"
                {
                    *value_name = v.clone();
                }
            }
            Boolean(v) => {
                if let Some(val) = value.to_i32() {
                    *v = if val == 0 { true } else { false };
                }
            }
            Comparison(op) => {
                if let Some(val) = value.to_i32() {
                    if let Some(o) = ComparisonOp::from_index(val as usize) {
                        *op = o;
                    }
                }
            }
            Arithmetic(op) => {
                if let Some(val) = value.to_i32() {
                    if let Some(o) = ArithmeticOp::from_index(val as usize) {
                        *op = o;
                    }
                }
            }
            Assignment => {
                if let Some(val) = value.to_i32() {
                    self.option = val;
                }
            }
            Textures(str) => {
                if let Some(val) = value.to_i32() {
                    if let Some(kind) = PatternKind::from_index(val as usize) {
                        *str = kind.display_name().to_string();
                    }
                }
            }
            _ => {}
        }
        true
    }

    /// Inserts the item at the given position.
    pub fn insert_at(mut self, pos: (u32, u32), grid: &mut Grid) {
        match &self.cell {
            Cell::ConstructAssignBlock => {
                if pos.0 == 0 {
                    grid.insert((pos.0, pos.1), CellItem::new(Cell::Variable("var".into())));
                    grid.insert((pos.0 + 1, pos.1), CellItem::new(Cell::Assignment));
                    grid.insert((pos.0 + 2, pos.1), CellItem::new(Cell::Integer("0".into())));
                }
            }
            Cell::ConstructColorAssignBlock => {
                if pos.0 == 0 {
                    grid.insert(
                        (pos.0, pos.1),
                        CellItem::new(Cell::Variable("color".into())),
                    );
                    grid.insert((pos.0 + 1, pos.1), CellItem::new(Cell::Assignment));
                    grid.insert((pos.0 + 2, pos.1), CellItem::new(Cell::Value("1".into())));
                }
            }
            Cell::ConstructIfBlock => {
                if pos.0 == 0 {
                    grid.insert((pos.0, pos.1), CellItem::new(Cell::If));
                    grid.insert(
                        (pos.0 + 1, pos.1),
                        CellItem::new(Cell::Variable("var".into())),
                    );
                    grid.insert(
                        (pos.0 + 2, pos.1),
                        CellItem::new(Cell::Comparison(ComparisonOp::Equal)),
                    );
                    grid.insert((pos.0 + 3, pos.1), CellItem::new(Cell::Integer("0".into())));

                    grid.move_down_from(pos.1 + 2);
                    grid.insert((0, pos.1 + 1), CellItem::new(Cell::Empty));

                    let mut indent = 1;
                    if let Some(ind) = grid.row_indents.get(&pos.1) {
                        indent += *ind;
                    }

                    if !grid.grid.contains_key(&(0, pos.1 + 1)) {
                        grid.row_indents.insert(pos.1 + 1, indent);
                    }
                    grid.insert_empty();
                }
            }
            Cell::Else => {
                if pos.0 == 0 {
                    grid.insert((pos.0, pos.1), CellItem::new(Cell::Else));

                    grid.move_down_from(pos.1 + 2);
                    grid.insert((0, pos.1 + 1), CellItem::new(Cell::Empty));

                    let mut indent = 1;
                    if let Some(ind) = grid.row_indents.get(&pos.1) {
                        indent += *ind;
                    }

                    if !grid.grid.contains_key(&(0, pos.1 + 1)) {
                        grid.row_indents.insert(pos.1 + 1, indent);
                    }
                    grid.insert_empty();
                }
            }
            Cell::Action => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("".into()),
                        self.id,
                        true,
                        "Action Cmd",
                        CellItemForm::RightRounded,
                    ),
                );

                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::Intent => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("".into()),
                        self.id,
                        true,
                        "Intent Cmd",
                        CellItemForm::RightRounded,
                    ),
                );

                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::AddItem => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("".into()),
                        self.id,
                        false,
                        "Item Name",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::BlockEvents => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Integer("1".into()),
                        self.id,
                        true,
                        "Minutes",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("".into()),
                        self.id,
                        true,
                        "Event",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::CloseIn => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("".into()),
                        self.id,
                        true,
                        "Target ID",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Float("1.5".into()),
                        self.id,
                        true,
                        "Radius",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 3, pos.1),
                    CellItem::new_dependency(
                        Cell::Float("1.0".into()),
                        self.id,
                        true,
                        "Speed",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::DealDamage => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("".into()),
                        self.id,
                        true,
                        "Entity ID",
                        CellItemForm::Box,
                    ),
                );
                let mut item = CellItem::new_dependency(
                    Cell::Integer("0".into()),
                    self.id,
                    true,
                    "Damage",
                    CellItemForm::RightRounded,
                );
                item.special_role = CellItemSpecialRole::DealDamageValue;
                grid.insert((pos.0 + 2, pos.1), item);
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::Drop => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Integer("0".into()),
                        self.id,
                        true,
                        "Item ID",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::DropItems => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("".into()),
                        self.id,
                        false,
                        "Filter",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::EntitiesInRadius => {
                self.form = CellItemForm::Rounded;
                grid.insert(pos, self)
            }
            Cell::Equip => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Integer("0".into()),
                        self.id,
                        true,
                        "Item ID",
                        CellItemForm::RightRounded,
                    ),
                );

                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::GetAttr => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("attr".into()),
                        self.id,
                        false,
                        "Attribute Name",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::GetAttrOf => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("".into()),
                        self.id,
                        true,
                        "Entity/Item ID",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("attr".into()),
                        self.id,
                        false,
                        "Attribute Name",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::Goto => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("".into()),
                        self.id,
                        true,
                        "Sector Name",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Float("1.0".into()),
                        self.id,
                        true,
                        "Speed",
                        CellItemForm::RightRounded,
                    ),
                );

                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::Id => {
                self.form = CellItemForm::Rounded;
                grid.insert(pos, self)
            }
            Cell::InventoryItems => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("".into()),
                        self.id,
                        true,
                        "Filter",
                        CellItemForm::RightRounded,
                    ),
                );

                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::InventoryItemsOf => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("".into()),
                        self.id,
                        true,
                        "Entity ID",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("".into()),
                        self.id,
                        true,
                        "Filter",
                        CellItemForm::RightRounded,
                    ),
                );

                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::Message => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("".into()),
                        self.id,
                        true,
                        "Receiver ID",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("".into()),
                        self.id,
                        true,
                        "Message",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 3, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("".into()),
                        self.id,
                        true,
                        "Category",
                        CellItemForm::RightRounded,
                    ),
                );

                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::NotifyIn => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Integer("1".into()),
                        self.id,
                        true,
                        "In-Game Minutes",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("event".into()),
                        self.id,
                        true,
                        "Event Name",
                        CellItemForm::RightRounded,
                    ),
                );

                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::OfferInventory => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("".into()),
                        self.id,
                        true,
                        "Entity ID",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("".into()),
                        self.id,
                        true,
                        "Filter",
                        CellItemForm::RightRounded,
                    ),
                );

                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::Random => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Integer("1".into()),
                        self.id,
                        true,
                        "From",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Integer("5".into()),
                        self.id,
                        true,
                        "To",
                        CellItemForm::RightRounded,
                    ),
                );

                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::RandomWalkInSector | Cell::RandomWalk => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Float("1.0".into()),
                        self.id,
                        true,
                        "Distance",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Float("1.0".into()),
                        self.id,
                        true,
                        "Speed",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 3, pos.1),
                    CellItem::new_dependency(
                        Cell::Integer("1".into()),
                        self.id,
                        true,
                        "Max Sleep",
                        CellItemForm::RightRounded,
                    ),
                );

                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::SetAttr => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("attr".into()),
                        self.id,
                        false,
                        "Attribute Name",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Integer("0".into()),
                        self.id,
                        true,
                        "Value",
                        CellItemForm::RightRounded,
                    ),
                );

                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::SetEmitLight => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Boolean(false),
                        self.id,
                        true,
                        "Emission State",
                        CellItemForm::RightRounded,
                    ),
                );

                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::SetPlayerCamera => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("firstp".into()),
                        self.id,
                        true,
                        "Camera",
                        CellItemForm::RightRounded,
                    ),
                );

                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::SetProximityTracking => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Boolean(true),
                        self.id,
                        true,
                        "On / Off",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Float("5.0".into()),
                        self.id,
                        true,
                        "Distance",
                        CellItemForm::RightRounded,
                    ),
                );

                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::SetTile => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("".into()),
                        self.id,
                        true,
                        "Tile ID",
                        CellItemForm::RightRounded,
                    ),
                );

                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::Take => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("".into()),
                        self.id,
                        true,
                        "Item ID",
                        CellItemForm::RightRounded,
                    ),
                );

                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::Teleport => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("".into()),
                        self.id,
                        true,
                        "Sector Name",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("".into()),
                        self.id,
                        true,
                        "Region Name",
                        CellItemForm::RightRounded,
                    ),
                );

                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::ToggleAttr => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Str("attr".into()),
                        self.id,
                        false,
                        "Attribute Name",
                        CellItemForm::Box,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::TookDamage => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Integer("0".into()),
                        self.id,
                        true,
                        "ID",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Integer("0".into()),
                        self.id,
                        true,
                        "Amount",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Abs => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "X",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Atan => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "X",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Atan2 => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("y".into()),
                        self.id,
                        true,
                        "Y",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "X",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Ceil => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "X",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Clamp => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "X",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("min".into()),
                        self.id,
                        true,
                        "Min",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 3, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("max".into()),
                        self.id,
                        true,
                        "Max",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Cos => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "Radians",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Cross => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("a".into()),
                        self.id,
                        true,
                        "A",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("b".into()),
                        self.id,
                        true,
                        "B",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Degrees => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "Radians",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Dot => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("a".into()),
                        self.id,
                        true,
                        "A",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("b".into()),
                        self.id,
                        true,
                        "B",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Exp => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "X",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Floor => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "X",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Fract => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "X",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Length => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "X",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Log => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "X",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Max => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("a".into()),
                        self.id,
                        true,
                        "A",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("b".into()),
                        self.id,
                        true,
                        "B",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Min => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("a".into()),
                        self.id,
                        true,
                        "A",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("b".into()),
                        self.id,
                        true,
                        "B",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Mix => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("a".into()),
                        self.id,
                        true,
                        "A",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("b".into()),
                        self.id,
                        true,
                        "B",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 3, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("t".into()),
                        self.id,
                        true,
                        "T",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Mod => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "X",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("y".into()),
                        self.id,
                        true,
                        "Y",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Normalize => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "X",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Pow => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "Base X",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("y".into()),
                        self.id,
                        true,
                        "Exponent Y",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Radians => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("degrees".into()),
                        self.id,
                        true,
                        "Degrees",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Rand => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("uv".into()),
                        self.id,
                        true,
                        "UV",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Rotate2d => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("uv".into()),
                        self.id,
                        true,
                        "UV",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Float("0.0".into()),
                        self.id,
                        true,
                        "Angle (rad)",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Sign => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "X",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Sin => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "Radians",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Smoothstep => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("edge0".into()),
                        self.id,
                        true,
                        "Edge 0",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("edge1".into()),
                        self.id,
                        true,
                        "Edge 1",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 3, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "X",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Sqrt => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "X",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Step => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("edge".into()),
                        self.id,
                        true,
                        "Edge",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "X",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }

            Cell::Tan => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("x".into()),
                        self.id,
                        true,
                        "Radians",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            Cell::Sample | Cell::SampleNormal => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Variable("uv".into()),
                        self.id,
                        true,
                        "UV",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Textures("value".into()),
                        self.id,
                        true,
                        "Texture",
                        CellItemForm::RightRounded,
                    ),
                );
                self.form = CellItemForm::LeftRounded;
                grid.insert(pos, self)
            }
            _ => grid.insert(pos, self),
        }
    }

    /// Generates code for the item.
    pub fn code(&self) -> String {
        if self.cell.role() == CellRole::Function {
            return self.cell.to_string() + "(";
        }

        // if self.special_role == CellItemSpecialRole::DealDamageValue {
        // deal_damage(value, {"from": id(), "amount": random(1, 3)})
        // return format!("{{\"from\": id(), \"amount\": {}}}", self.cell.to_string());
        // }

        match self.cell {
            Variable(_) => match self.option {
                1 => {
                    format!("{}[0]", self.cell.to_string())
                }
                2 => {
                    format!("len({})", self.cell.to_string())
                }
                _ => self.cell.to_string(),
            },
            Assignment => {
                if let Some(op) = AssignmentOp::from_index(self.option) {
                    op.describe().to_string()
                } else {
                    "=".to_string()
                }
            }
            _ => self.cell.to_string(),
        }
    }

    /// Checks if the string is a valid variable name
    pub fn is_valid_code_variable(name: &str) -> bool {
        // General identifier: non-empty, does not start with a digit or '.', and
        // may contain letters, digits, underscores, or dots (for swizzles/paths).
        let mut chars = name.chars();
        match chars.next() {
            Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
            _ => return false,
        }

        name.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    }

    /// Checks if the string is a valid number
    pub fn is_valid_number(s: &str) -> bool {
        // Try to parse as integer
        if s.parse::<i64>().is_ok() {
            return true;
        }
        // Try to parse as float (Python allows scientific notation, etc.)
        if s.parse::<f64>().is_ok() {
            return true;
        }
        false
    }

    /// Checks if the string is a valid integer
    pub fn is_valid_integer(s: &str) -> bool {
        // Try to parse as integer
        if s.parse::<i64>().is_ok() {
            return true;
        }
        false
    }

    /// Checks if the string is a valid float
    pub fn is_valid_float(s: &str) -> bool {
        // Try to parse as float
        if s.parse::<f64>().is_ok() {
            return true;
        }
        false
    }

    /// Rounding based on the form
    pub fn rounding(&self, rounding: f32) -> (f32, f32, f32, f32) {
        match &self.form {
            CellItemForm::Box => (0.0, 0.0, 0.0, 0.0),
            CellItemForm::Rounded => (rounding, rounding, rounding, rounding),
            CellItemForm::LeftRounded => (0.0, 0.0, rounding, rounding),
            CellItemForm::RightRounded => (rounding, rounding, 0.0, 0.0),
        }
    }

    pub fn generate_context(&self) -> TheContextMenu {
        let mut context_menu = TheContextMenu::named(str!("CGFContext"));

        match &self.cell {
            _ => {
                context_menu.add(TheContextMenuItem::new(
                    str!("Assignment"),
                    TheId::named("CGFAssignment"),
                ));
            }
        }

        context_menu
    }
}
