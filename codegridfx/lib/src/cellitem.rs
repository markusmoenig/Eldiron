use crate::{Cell, GridCtx};
use theframework::prelude::*;

use Cell::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CellItem {
    pub id: Uuid,
    pub cell: Cell,
    pub has_error: bool,
}

impl CellItem {
    pub fn new(cell: Cell) -> Self {
        Self {
            id: Uuid::new_v4(),
            cell,
            has_error: false,
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
        _pos: &(u32, u32),
    ) {
        let stride = buffer.dim().width as usize;
        let color = if is_selected {
            &grid_ctx.selection_color
        } else {
            &grid_ctx.normal_color
        };
        let zoom = 5.0;
        match &self.cell {
            Cell::Variable(name) => {
                if let Some(font) = &ctx.ui.font {
                    ctx.draw.rounded_rect(
                        buffer.pixels_mut(),
                        &rect.to_buffer_utuple(),
                        stride,
                        &color,
                        &(2.0 * zoom, 2.0 * zoom, 2.0 * zoom, 2.0 * zoom),
                    );

                    ctx.draw.text_rect_blend(
                        buffer.pixels_mut(),
                        &rect.to_buffer_utuple(),
                        stride,
                        font,
                        grid_ctx.font_size,
                        name,
                        &grid_ctx.text_color,
                        TheHorizontalAlign::Center,
                        TheVerticalAlign::Center,
                    );
                }
            }
            Cell::Assignment => {
                if let Some(font) = &ctx.ui.font {
                    ctx.draw.text_rect_blend(
                        buffer.pixels_mut(),
                        &rect.to_buffer_utuple(),
                        stride,
                        font,
                        grid_ctx.font_size + 10.0 * grid_ctx.zoom,
                        "=",
                        color,
                        TheHorizontalAlign::Center,
                        TheVerticalAlign::Center,
                    );
                }
            }
            Cell::Value(value) => {
                if let Some(font) = &ctx.ui.font {
                    ctx.draw.rounded_rect(
                        buffer.pixels_mut(),
                        &rect.to_buffer_utuple(),
                        stride,
                        &color,
                        &(2.0 * zoom, 2.0 * zoom, 2.0 * zoom, 2.0 * zoom),
                    );

                    ctx.draw.text_rect_blend(
                        buffer.pixels_mut(),
                        &rect.to_buffer_utuple(),
                        stride,
                        font,
                        grid_ctx.font_size,
                        value,
                        &grid_ctx.text_color,
                        TheHorizontalAlign::Center,
                        TheVerticalAlign::Center,
                    );
                }
            }
            _ => {
                buffer.draw_rect_outline(
                    rect,
                    if is_selected {
                        &grid_ctx.selection_color
                    } else {
                        &grid_ctx.dark_color
                    },
                );
            }
        }
    }

    /// Returns the size of the cell
    pub fn size(&self, ctx: &TheContext, grid_ctx: &GridCtx) -> Vec2<u32> {
        let mut size = Vec2::new(100, 60);
        match &self.cell {
            Variable(name) | Value(name) => {
                if let Some(font) = &ctx.ui.font {
                    size.x = ctx.draw.get_text_size(font, grid_ctx.font_size, name).0 as u32 + 20;
                }
            }
            Assignment => {
                if let Some(font) = &ctx.ui.font {
                    size.x = ctx.draw.get_text_size(font, grid_ctx.font_size, "=").0 as u32 + 20;
                }
            }
            _ => {}
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
                    "Variable Name".into(),
                    "Set the name of the variable".into(),
                    name.clone(),
                    None,
                    false,
                );
                nodeui.add_item(item);
            }
            Value(value) => {
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
                }
            }
            Value(value_name) => {
                if let Some(v) = value.to_string()
                    && name == "cgfxValue"
                {
                    if Self::is_valid_python_number(&v) {
                        *value_name = v;
                    }
                }
            }
            _ => {}
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
