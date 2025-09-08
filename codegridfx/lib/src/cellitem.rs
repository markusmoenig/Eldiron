use crate::{
    Cell, Grid, GridCtx,
    cell::{ArithmeticOp, CellRole, ComparisonOp},
};
use rusterix::Debug;
use theframework::prelude::*;

use Cell::*;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub enum CellItemForm {
    Box,
    #[default]
    Rounded,
    LeftRounded,
    RightRounded,
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
        debug: Option<&Debug>,
    ) {
        let stride = buffer.dim().width as usize;
        let color = if self.has_error {
            &grid_ctx.error_color
        } else if is_selected {
            &grid_ctx.selection_color
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
                if let Some(font) = &ctx.ui.font {
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
                        font,
                        grid_ctx.font_size,
                        &text,
                        &grid_ctx.text_color,
                        TheHorizontalAlign::Center,
                        TheVerticalAlign::Center,
                    );

                    if !self.description.is_empty() {
                        let r = rect.to_buffer_utuple();
                        ctx.draw.text_rect_blend(
                            buffer.pixels_mut(),
                            &(r.0, r.1 + 15, r.2, r.3),
                            stride,
                            font,
                            grid_ctx.font_size,
                            &self.description,
                            &grid_ctx.highlight_text_color,
                            TheHorizontalAlign::Center,
                            TheVerticalAlign::Center,
                        );
                    }
                }
            }
            Cell::Assignment | Cell::Comparison(_) | Cell::If | Arithmetic(_) => {
                if let Some(font) = &ctx.ui.font {
                    ctx.draw.text_rect_blend(
                        buffer.pixels_mut(),
                        &rect.to_buffer_utuple(),
                        stride,
                        font,
                        grid_ctx.font_size * 2.0,
                        &self.cell.to_string(),
                        color,
                        TheHorizontalAlign::Center,
                        TheVerticalAlign::Center,
                    );
                }
            }
            Cell::LeftParent | Cell::RightParent => {
                if let Some(font) = &ctx.ui.font {
                    ctx.draw.text_rect_blend(
                        buffer.pixels_mut(),
                        &rect.to_buffer_utuple(),
                        stride,
                        font,
                        grid_ctx.font_size * 2.0 + 10.0 * grid_ctx.zoom,
                        &self.cell.to_string(),
                        color,
                        TheHorizontalAlign::Center,
                        TheVerticalAlign::Center,
                    );
                }
            }
            Cell::Integer(_) | Cell::Float(_) | Cell::Str(_) | Cell::Boolean(_) => {
                if let Some(font) = &ctx.ui.font {
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
                        font,
                        grid_ctx.font_size,
                        &self.cell.to_string(),
                        &grid_ctx.text_color,
                        TheHorizontalAlign::Center,
                        TheVerticalAlign::Center,
                    );

                    if !self.description.is_empty() {
                        let r = rect.to_buffer_utuple();
                        ctx.draw.text_rect_blend(
                            buffer.pixels_mut(),
                            &(r.0, r.1 + 15, r.2, r.3),
                            stride,
                            font,
                            grid_ctx.font_size,
                            &self.description,
                            &grid_ctx.highlight_text_color,
                            TheHorizontalAlign::Center,
                            TheVerticalAlign::Center,
                        );
                    }
                }
            }
            Empty => {
                let mut shrinker = TheDimShrinker::zero();
                shrinker.shrink(4);
                ctx.draw.rounded_rect_with_border(
                    buffer.pixels_mut(),
                    &rect.to_buffer_shrunk_utuple(&shrinker),
                    stride,
                    &grid_ctx.background_color,
                    &self.rounding(rounding),
                    &color,
                    1.5,
                );
            }
            _ => {
                // Function Header

                if let Some(font) = &ctx.ui.font {
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
                                &grid_ctx.error_color
                            } else {
                                &grid_ctx.highlight_text_color
                            };
                            has_debug = true;
                            ctx.draw.text_rect_blend(
                                buffer.pixels_mut(),
                                &(r.0, r.1 + 15, r.2, r.3),
                                stride,
                                font,
                                grid_ctx.font_size,
                                &value.to_string(),
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
                        font,
                        grid_ctx.large_font_size,
                        &self.cell.to_string(),
                        &grid_ctx.text_color,
                        TheHorizontalAlign::Center,
                        TheVerticalAlign::Center,
                    );
                }
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
        grid_ctx: &GridCtx,
        pos: &(u32, u32),
        event: &str,
        id: u32,
        debug: Option<&Debug>,
    ) -> Vec2<u32> {
        let mut size = Vec2::new(30, 50);
        match &self.cell {
            Variable(_) | Integer(_) | Float(_) | Str(_) | Boolean(_) => {
                let text = match self.option {
                    1 => {
                        format!("First({})", self.cell.to_string())
                    }
                    2 => {
                        format!("Length({})", self.cell.to_string())
                    }
                    _ => self.cell.to_string(),
                };
                if let Some(font) = &ctx.ui.font {
                    size.x = ctx.draw.get_text_size(font, grid_ctx.font_size, &text).0 as u32 + 20;

                    if !self.description.is_empty() {
                        let desc = ctx
                            .draw
                            .get_text_size(font, grid_ctx.font_size, &self.description)
                            .0 as u32
                            + 20;
                        size.x = size.x.max(desc);
                    }
                }
            }
            Assignment => {
                if let Some(font) = &ctx.ui.font {
                    size.x = ctx.draw.get_text_size(font, grid_ctx.font_size, "=").0 as u32 + 20;
                }
            }
            If | Comparison(_) => {
                if let Some(font) = &ctx.ui.font {
                    size.x = ctx
                        .draw
                        .get_text_size(font, grid_ctx.font_size * 2.0, &self.cell.to_string())
                        .0 as u32
                        + 10;
                }
            }
            LeftParent | RightParent => {
                if let Some(font) = &ctx.ui.font {
                    size.x = ctx
                        .draw
                        .get_text_size(font, grid_ctx.font_size * 2.0, &self.cell.to_string())
                        .0 as u32
                        + 10;
                }
            }
            Empty => {}
            _ => {
                if let Some(font) = &ctx.ui.font {
                    size.x = ctx
                        .draw
                        .get_text_size(font, grid_ctx.large_font_size, &self.cell.to_string())
                        .0 as u32
                        + 20;

                    if let Some(debug) = debug {
                        if let Some(value) = debug.get_value(id, event, pos.0, pos.1) {
                            let desc = ctx
                                .draw
                                .get_text_size(font, grid_ctx.font_size, &value.to_string())
                                .0 as u32
                                + 20;
                            size.x = size.x.max(desc);
                        }
                    }
                }
            }
        }
        size
    }

    /// Creates the settings for the cell
    pub fn create_settings(&self) -> TheNodeUI {
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
            Str(value) => {
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
            Comparison(op) => {
                let item = TheNodeUIItem::Selector(
                    "cgfxComparisonOp".into(),
                    "Operator".into(),
                    "Select the comparison operator".into(),
                    vec![
                        "==".to_string(),
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
            _ => {}
        }

        nodeui
    }

    /// Creates the settings for the cell
    pub fn apply_value(&mut self, name: &str, value: &TheValue) {
        match &mut self.cell {
            Variable(var_name) => {
                if let Some(n) = value.to_string()
                    && name == "cgfxVariableName"
                {
                    if Self::is_valid_python_variable(&n) {
                        *var_name = n;
                    }
                } else if let Some(v) = value.to_i32()
                    && name == "cgfxVariableOption"
                {
                    self.option = v;
                }
            }
            Integer(value_name) => {
                if let Some(v) = value.to_string()
                    && name == "cgfxValue"
                {
                    self.has_error = !Self::is_valid_python_integer(&v);
                    *value_name = v;
                }
            }
            Float(value_name) => {
                if let Some(v) = value.to_string()
                    && name == "cgfxValue"
                {
                    self.has_error = !Self::is_valid_python_float(&v);
                    *value_name = v;
                }
            }
            Str(value_name) => {
                if let Some(v) = value.to_string()
                    && name == "cgfxValue"
                {
                    *value_name = v.replace("\"", "");
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
            _ => {}
        }
    }

    /// Inserts the item at the given position.
    pub fn insert_at(mut self, pos: (u32, u32), grid: &mut Grid, _old_item: CellItem) {
        match &self.cell {
            Cell::Assignment => {
                if pos.0 == 0 {
                    grid.insert((pos.0, pos.1), CellItem::new(Cell::Variable("dest".into())));
                    grid.insert((pos.0 + 1, pos.1), self);
                    grid.insert((pos.0 + 2, pos.1), CellItem::new(Cell::Integer("0".into())));
                }
            }
            Cell::Comparison(_) => {
                if pos.0 == 0 {
                    grid.insert((pos.0, pos.1), CellItem::new(Cell::If));
                    grid.insert(
                        (pos.0 + 1, pos.1),
                        CellItem::new(Cell::Variable("variable".into())),
                    );
                    grid.insert((pos.0 + 2, pos.1), self);
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

                    // } else {
                    // grid.insert((pos.0, pos.1 + 2), CellItem::new(Cell::Empty));
                    // grid.row_indents.insert(pos.1 + 2, 0);
                    // }
                    // grid.insert_empty();
                    // if indent == 2 {
                    //     if !grid.grid.contains_key(&(0, pos.1 + 2)) {
                    //         grid.row_indents.insert(pos.1 + 2, 1);
                    //     }

                    //     if !grid.grid.contains_key(&(0, pos.1 + 3)) {
                    //         grid.insert((pos.0, pos.1 + 3), CellItem::new(Cell::Empty));
                    //         grid.row_indents.insert(pos.1 + 3, 1);
                    //     }

                    //     if !grid.grid.contains_key(&(0, pos.1 + 4)) {
                    //         grid.insert((pos.0, pos.1 + 4), CellItem::new(Cell::Empty));
                    //         grid.row_indents.insert(pos.1 + 4, 0);
                    //     }
                    // } else {
                    //     grid.insert((pos.0, pos.1 + 2), CellItem::new(Cell::Empty));
                    //     grid.row_indents.insert(pos.1 + 2, 0);
                    // }
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
                        Cell::Integer("0".into()),
                        self.id,
                        true,
                        "Target ID",
                        CellItemForm::Box,
                    ),
                );
                grid.insert(
                    (pos.0 + 2, pos.1),
                    CellItem::new_dependency(
                        Cell::Float("5.0".into()),
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
                        Cell::Integer("0".into()),
                        self.id,
                        true,
                        "Entity",
                        CellItemForm::Box,
                    ),
                );
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
            Cell::GetEntityAttr => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Integer("0".into()),
                        self.id,
                        true,
                        "Entity ID",
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
            Cell::GetItemAttr => {
                grid.insert(
                    (pos.0 + 1, pos.1),
                    CellItem::new_dependency(
                        Cell::Integer("0".into()),
                        self.id,
                        true,
                        "Item ID",
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
                self.form = CellItemForm::Box;
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
                        Cell::Integer("0".into()),
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
                        Cell::Integer("0".into()),
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
                        Cell::Integer("0".into()),
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
                // TODO
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
            _ => grid.insert(pos, self),
        }
    }

    /// Generates code for the item.
    pub fn code(&self) -> String {
        if self.cell.role() == CellRole::Function {
            return self.cell.to_string() + "(";
        }

        match self.cell {
            Variable(_) => match self.option {
                1 => {
                    format!("{}[0]", self.cell.to_string())
                }
                2 => {
                    format!("length({})", self.cell.to_string())
                }
                _ => self.cell.to_string(),
            },
            _ => self.cell.to_string(),
        }
    }

    /// Checks if the string is a valid python variable name
    pub fn is_valid_python_variable(name: &str) -> bool {
        // Must not be empty, must start with a letter or underscore, and only contain letters, digits, or underscores
        let mut chars = name.chars();
        match chars.next() {
            Some(c) if c.is_ascii_alphabetic() || c == '_' => (),
            _ => return false,
        }
        if name.is_empty() {
            return false;
        }
        if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            // Python keywords are not valid variable names
            const PYTHON_KEYWORDS: &[&str] = &[
                "False", "None", "True", "and", "as", "assert", "break", "class", "continue",
                "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if",
                "import", "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return",
                "try", "while", "with", "yield",
            ];
            !PYTHON_KEYWORDS.contains(&name)
        } else {
            false
        }
    }

    /// Checks if the string is a valid python number
    pub fn is_valid_python_number(s: &str) -> bool {
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

    /// Checks if the string is a valid python integer
    pub fn is_valid_python_integer(s: &str) -> bool {
        // Try to parse as integer
        if s.parse::<i64>().is_ok() {
            return true;
        }
        false
    }

    /// Checks if the string is a valid python float
    pub fn is_valid_python_float(s: &str) -> bool {
        // Try to parse as float (Python allows scientific notation, etc.)
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
