use crate::{
    Cell, CellItem, DebugModule, Grid, GridCtx, ModuleType, cell::CellRole, cellitem::CellItemForm,
};
use serde::de::{self, Deserializer};
use std::collections::VecDeque;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum DragPayload {
    Cell(CellItem),
    Block(BlockPayload),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockPayload {
    pub root: CellItem,
    pub items: Vec<((u32, u32), CellItem)>,
}

fn default_scale() -> f32 {
    1.0
}

fn default_rotation() -> f32 {
    0.0
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Routine {
    pub id: Uuid,
    pub name: String,

    #[serde(
        default = "default_i32_zero",
        deserialize_with = "deserialize_i32_from_any"
    )]
    pub module_offset: i32,
    pub visible: bool,
    pub folded: bool,

    #[serde(default)]
    pub pixelization: i32,

    #[serde(default = "default_scale")]
    pub scale: f32,

    #[serde(default = "default_rotation")]
    pub rotation: f32,

    #[serde(default)]
    pub color_steps: i32,

    pub screen_width: u32,

    #[serde(skip)]
    pub buffer: TheRGBABuffer,

    #[serde(skip)]
    pub shader_background: TheRGBABuffer,

    pub grid: Grid,
}

impl Routine {
    pub fn new(name: &str) -> Self {
        let mut grid = Grid::new();
        grid.insert((0, 0), CellItem::new(Cell::Empty));
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            module_offset: 0,
            visible: false,
            folded: false,
            screen_width: 100,
            buffer: TheRGBABuffer::new(TheDim::sized(100, 100)),
            shader_background: TheRGBABuffer::empty(),
            grid,
            pixelization: 0,
            color_steps: 0,
            scale: default_scale(),
            rotation: default_rotation(),
        }
    }

    pub fn draw(
        &mut self,
        ctx: &TheContext,
        grid_ctx: &GridCtx,
        id: u32,
        debug: Option<&DebugModule>,
        global_header_width: Option<u32>,
        drop_target: Option<((u32, u32), bool, &str)>,
    ) {
        // Size check
        let size = self.grid.size(
            ctx,
            grid_ctx,
            self.folded,
            self.screen_width,
            &self.name,
            id,
            debug,
        );

        let width = size.x.max(self.screen_width);
        let header_width = global_header_width.unwrap_or(width).max(width);
        let height = size.y;

        if self.buffer.dim().width != header_width as i32
            || self.buffer.dim().height != height as i32
        {
            self.buffer = TheRGBABuffer::new(TheDim::sized(header_width as i32, height as i32));
        }

        self.buffer.fill([116, 116, 116, 255]);
        let header_height: i32 = 35;

        // Copy the shader background if available
        if !self.shader_background.is_empty() {
            let render_buffer = &mut self.buffer;
            let dest_width = render_buffer.dim().width;
            let dest_height = render_buffer.dim().height;

            let shader_buffer = &mut self.shader_background;
            let source_width = shader_buffer.dim().width;
            let source_height = shader_buffer.dim().height;

            // Tile the shader_buffer across the entire destination buffer
            for y in (header_height..dest_height).step_by(source_height as usize) {
                for x in (400..dest_width).step_by(source_width as usize) {
                    render_buffer.copy_into(x, y, &*shader_buffer);
                }
            }
        }

        let folded_corners = if !self.folded { 0.0 } else { 12.0 };
        let is_selected = Some(self.id) == grid_ctx.selected_routine;
        let normal_color = CellRole::Event.to_color();
        let text_color = [85, 81, 85, 255];
        let executed_header_color = [255, 243, 163, 255];
        let header_width_i32 = header_width as i32;
        let header_width_usize = header_width as usize;
        let header_executed = debug.is_some_and(|debug| debug.header_was_executed(id, &self.name));
        let stride = self.buffer.dim().width as usize;

        self.buffer.draw_rounded_rect(
            &TheDim::rect(0, 0, header_width_i32, header_height as i32),
            if is_selected {
                &[187, 122, 208, 255]
            } else {
                &normal_color
            },
            &(folded_corners, 12.0, folded_corners, 12.0),
            0.0,
            &WHITE,
        );

        if header_executed {
            ctx.draw.rounded_rect_with_border(
                self.buffer.pixels_mut(),
                &(0, 0, header_width_usize, header_height as usize),
                stride,
                &[0, 0, 0, 0],
                &(folded_corners, 12.0, folded_corners, 12.0),
                &executed_header_color,
                3.0,
            );
        }
        let desc = self.get_description();

        ctx.draw.text_rect_blend(
            self.buffer.pixels_mut(),
            &(20, 0, header_width_usize, header_height as usize),
            stride,
            &format!("{} ({})", self.name, self.grid.count()),
            TheFontSettings {
                size: 15.0,
                ..Default::default()
            },
            &text_color,
            TheHorizontalAlign::Left,
            TheVerticalAlign::Center,
        );
        ctx.draw.text_rect_blend(
            self.buffer.pixels_mut(),
            &(
                200,
                0,
                header_width_usize.saturating_sub(210),
                header_height as usize,
            ),
            stride,
            &desc,
            TheFontSettings {
                size: 13.0,
                ..Default::default()
            },
            &text_color,
            TheHorizontalAlign::Right,
            TheVerticalAlign::Center,
        );

        if !self.folded {
            if let Some(debug) = debug {
                let row_highlight = [255, 243, 163, 72];
                let row_not_taken_highlight = [116, 150, 184, 84];
                for row in self.grid.grid_by_rows() {
                    if let Some((_, (_, y))) = row.first()
                        && debug.row_was_executed(id, &self.name, *y)
                    {
                        let mut min_y: Option<usize> = None;
                        let mut max_y: usize = 0;
                        for (_, coord) in &row {
                            if let Some(rect) = self.grid.grid_rects.get(coord) {
                                let top = rect.y.max(0) as usize;
                                let bottom = (rect.y + rect.height).max(0) as usize;
                                min_y = Some(min_y.map_or(top, |current| current.min(top)));
                                max_y = max_y.max(bottom);
                            }
                        }
                        if let Some(top) = min_y
                            && max_y > top
                        {
                            ctx.draw.blend_rect(
                                self.buffer.pixels_mut(),
                                &(0, top, header_width_usize, max_y - top),
                                stride,
                                if debug.row_was_not_taken(id, &self.name, *y) {
                                    &row_not_taken_highlight
                                } else {
                                    &row_highlight
                                },
                            );
                        }
                    }
                }
            }

            for (coord, cell) in &mut self.grid.grid {
                if let Some(rect) = self.grid.grid_rects.get(coord) {
                    let is_selected = Some(self.id) == grid_ctx.selected_routine
                        && Some(coord.clone()) == grid_ctx.current_cell;
                    cell.draw(
                        &mut self.buffer,
                        &rect,
                        ctx,
                        grid_ctx,
                        is_selected,
                        coord,
                        &self.name,
                        id,
                        debug,
                    );
                }
            }

            if let Some((coord, valid, label)) = drop_target
                && let Some(rect) = self.grid.grid_rects.get(&coord)
            {
                let color = if valid {
                    [120, 214, 137, 255]
                } else {
                    [214, 96, 96, 255]
                };
                ctx.draw.rounded_rect_with_border(
                    self.buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &[0, 0, 0, 0],
                    &(8.0, 8.0, 8.0, 8.0),
                    &color,
                    3.0,
                );

                let badge_w = (ctx
                    .draw
                    .get_text_size(
                        label,
                        &TheFontSettings {
                            size: 11.5,
                            ..Default::default()
                        },
                    )
                    .0 as usize)
                    + 16;
                let badge_x = rect.x.max(0) as usize;
                let badge_y = rect.y.saturating_sub(18).max(0) as usize;
                ctx.draw.rounded_rect(
                    self.buffer.pixels_mut(),
                    &(badge_x, badge_y, badge_w, 16),
                    stride,
                    &color,
                    &(6.0, 6.0, 6.0, 6.0),
                );
                ctx.draw.text_rect_blend(
                    self.buffer.pixels_mut(),
                    &(badge_x, badge_y, badge_w, 16),
                    stride,
                    label,
                    TheFontSettings {
                        size: 11.5,
                        ..Default::default()
                    },
                    &[24, 24, 24, 255],
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }
        }
    }

    /// Sets the screen width.
    pub fn set_screen_width(&mut self, width: u32, ctx: &TheContext, grid_ctx: &GridCtx) {
        self.screen_width = width;
        self.draw(ctx, grid_ctx, 0, None, None, None);
    }

    /// Returns the number of lines in the grid.
    pub fn lines(&self) -> u32 {
        let mut lines = 1;
        for (c, _) in &self.grid.grid {
            if c.1 > lines {
                lines = c.1;
            }
        }
        lines
    }

    pub fn cell_at(&self, loc: Vec2<u32>) -> Option<&CellItem> {
        let header_height = 35;

        if loc.y <= header_height || self.folded {
            return None;
        }

        for (coord, item) in &self.grid.grid {
            if let Some(rect) = self.grid.grid_rects.get(coord)
                && rect.contains(Vec2::new(loc.x as i32, loc.y as i32))
            {
                return Some(item);
            }
        }

        None
    }

    pub fn hit_at(&self, loc: Vec2<u32>) -> Option<((u32, u32), &CellItem)> {
        let header_height = 35;

        if loc.y <= header_height || self.folded {
            return None;
        }

        for (coord, item) in &self.grid.grid {
            if let Some(rect) = self.grid.grid_rects.get(coord)
                && rect.contains(Vec2::new(loc.x as i32, loc.y as i32))
            {
                return Some((*coord, item));
            }
        }

        None
    }

    pub fn drag_payload_at(&self, loc: Vec2<u32>) -> Option<DragPayload> {
        let ((col, row), item) = self.hit_at(loc)?;
        if matches!(item.cell, Cell::Empty) {
            return None;
        }

        let mut root_coord = (col, row);
        let mut root_item = item.clone();
        while let Some(dep_id) = root_item.dependend_on {
            if let Some((coord, parent)) = self
                .grid
                .grid
                .iter()
                .find(|(_, candidate)| candidate.id == dep_id)
                .map(|(coord, candidate)| (*coord, candidate.clone()))
            {
                root_coord = coord;
                root_item = parent;
            } else {
                break;
            }
        }

        if root_item.cell.role() == CellRole::Function {
            let mut queue = VecDeque::from([root_coord]);
            let mut coords = vec![root_coord];
            while let Some(parent_coord) = queue.pop_front() {
                let parent_id = if let Some(parent) = self.grid.grid.get(&parent_coord) {
                    parent.id
                } else {
                    continue;
                };
                let mut children: Vec<(u32, u32)> = self
                    .grid
                    .grid
                    .iter()
                    .filter(|((_, r), item)| *r == row && item.dependend_on == Some(parent_id))
                    .map(|(coord, _)| *coord)
                    .collect();
                children.sort_unstable();
                for child in children {
                    if !coords.contains(&child) {
                        coords.push(child);
                        queue.push_back(child);
                    }
                }
            }

            coords.sort_unstable();
            let mut id_map = FxHashMap::default();
            for coord in &coords {
                if let Some(item) = self.grid.grid.get(coord) {
                    id_map.insert(item.id, Uuid::new_v4());
                }
            }

            let mut root = self.grid.grid.get(&root_coord)?.clone();
            root.id = *id_map.get(&root.id)?;
            root.dependend_on = None;

            let mut items = Vec::new();
            for coord in coords {
                if coord == root_coord {
                    continue;
                }
                if let Some(item) = self.grid.grid.get(&coord) {
                    let mut cloned = item.clone();
                    if let Some(new_id) = id_map.get(&cloned.id) {
                        cloned.id = *new_id;
                    }
                    cloned.dependend_on = cloned
                        .dependend_on
                        .and_then(|dep| id_map.get(&dep).copied());
                    items.push(((coord.0 - root_coord.0, coord.1 - root_coord.1), cloned));
                }
            }

            return Some(DragPayload::Block(BlockPayload { root, items }));
        }

        let mut cell = item.clone();
        cell.id = Uuid::new_v4();
        cell.dependend_on = None;
        Some(DragPayload::Cell(cell))
    }

    fn can_insert_cell(&self, pos: (u32, u32), old_item: &CellItem, item: &CellItem) -> bool {
        let mut insert = true;
        if old_item.cell.role() != item.cell.role() && old_item.cell != Cell::Empty {
            insert = false;
        }
        if old_item.cell.role() == CellRole::Value && item.cell.role() == CellRole::Function {
            insert = true;
        }
        if matches!(item.cell, Cell::Arithmetic(_)) && old_item.cell == Cell::Empty {
            insert = self.grid.is_role_at(pos, -1, CellRole::Value)
                || self.grid.is_role_at(pos, -1, CellRole::Function);
        }
        if (item.cell.role() == CellRole::Value || item.cell.role() == CellRole::Function)
            && old_item.cell == Cell::Empty
        {
            insert = self.grid.is_role_at(pos, -1, CellRole::Operator);
        }
        if item.cell.role() == CellRole::Function && pos.0 == 0 {
            insert = true;
        }
        if item.cell.role() == CellRole::Function && !old_item.description.is_empty() {
            insert = false;
        }
        insert
    }

    pub fn drop_preview_at(
        &self,
        loc: Vec2<u32>,
        drop: &TheDrop,
    ) -> Option<((u32, u32), bool, String)> {
        let header_height = 35;
        if loc.y <= header_height || self.folded {
            return None;
        }

        let mut pos: Option<(u32, u32)> = None;
        let mut old_item: CellItem = CellItem::new(Cell::Empty);

        for (coord, item) in &self.grid.grid {
            if let Some(rect) = self.grid.grid_rects.get(coord)
                && rect.contains(Vec2::new(loc.x as i32, loc.y as i32))
            {
                if item.replaceable {
                    pos = Some(*coord);
                    old_item = item.clone();
                }
                break;
            }
        }

        let pos = pos?;
        let payload: Option<DragPayload> = serde_json::from_str(&drop.data)
            .ok()
            .or_else(|| {
                serde_json::from_str::<CellItem>(&drop.data)
                    .ok()
                    .map(DragPayload::Cell)
            })
            .or_else(|| {
                Cell::from_str(&drop.title).map(|cell| DragPayload::Cell(CellItem::new(cell)))
            });

        let payload = payload?;
        let (valid, label) = match &payload {
            DragPayload::Block(block) => (
                self.can_insert_cell(pos, &old_item, &block.root),
                format!("Copy {}", block.root.cell.to_string()),
            ),
            DragPayload::Cell(item) => (
                self.can_insert_cell(pos, &old_item, item),
                format!("Copy {}", item.cell.to_string()),
            ),
        };

        Some((pos, valid, label))
    }

    /// Handle a click at the given position.
    pub fn drop_at(
        &mut self,
        loc: Vec2<u32>,
        ctx: &mut TheContext,
        grid_ctx: &mut GridCtx,
        drop: &TheDrop,
        module_type: ModuleType,
        palette: &ThePalette,
        settings: &mut Option<TheNodeUI>,
    ) -> bool {
        let mut handled = false;
        let mut pos: Option<(u32, u32)> = None;
        let mut old_item: CellItem = CellItem::new(Cell::Empty);
        let header_height = 35;

        if loc.y > header_height && !self.folded {
            for (coord, item) in self.grid.grid.iter_mut() {
                if let Some(rect) = self.grid.grid_rects.get(coord) {
                    if rect.contains(Vec2::new(loc.x as i32, loc.y as i32)) {
                        if item.replaceable {
                            grid_ctx.selected_routine = Some(self.id);
                            grid_ctx.current_cell = Some(coord.clone());
                            pos = Some(coord.clone());
                            old_item = item.clone();
                        }
                        handled = true;
                        break;
                    }
                }
            }
        }

        if let Some(pos) = pos {
            let payload: Option<DragPayload> = serde_json::from_str(&drop.data)
                .ok()
                .or_else(|| {
                    serde_json::from_str::<CellItem>(&drop.data)
                        .ok()
                        .map(DragPayload::Cell)
                })
                .or_else(|| {
                    Cell::from_str(&drop.title).map(|cell| DragPayload::Cell(CellItem::new(cell)))
                });

            if let Some(payload) = payload {
                match payload {
                    DragPayload::Block(block_payload) => {
                        let mut root = block_payload.root;
                        if self.can_insert_cell(pos, &old_item, &root) {
                            if root.cell.role() == CellRole::Value {
                                root.description = old_item.description.clone();
                                root.replaceable = old_item.replaceable;
                                root.dependend_on = old_item.dependend_on;
                                root.form = old_item.form.clone();
                                root.special_role = old_item.special_role.clone();
                            }
                            self.grid.remove_dependencies_for(old_item.id);
                            self.grid.insert(pos, root);
                            for ((dx, dy), item) in block_payload.items {
                                self.grid.insert((pos.0 + dx, pos.1 + dy), item);
                            }
                        }
                    }
                    DragPayload::Cell(mut item) => {
                        item.id = Uuid::new_v4();
                        item.dependend_on = None;
                        if self.can_insert_cell(pos, &old_item, &item) {
                            if matches!(item.cell, Cell::Arithmetic(_)) {
                                let right_pos = (pos.0 + 1, pos.1);
                                if !self.grid.grid.contains_key(&(pos.0 + 1, pos.1)) {
                                    let value = CellItem::new(Cell::Value("1".to_string()));
                                    value.insert_at(right_pos, &mut self.grid);
                                }
                            }

                            if item.cell.role() == CellRole::Value {
                                item.description = old_item.description.clone();
                                item.replaceable = old_item.replaceable;
                                item.dependend_on = old_item.dependend_on;
                                item.form = old_item.form.clone();
                                item.special_role = old_item.special_role.clone();
                            }

                            self.grid.remove_dependencies_for(old_item.id);
                            item.insert_at(pos, &mut self.grid);
                        }
                    }
                }
            }
        }

        if let Some(pos) = pos {
            if let Some(item) = self.grid.grid.get(&pos) {
                let nodeui: TheNodeUI = item.create_settings(palette, module_type);
                *settings = Some(nodeui);
            }

            self.grid.insert_empty();
            self.draw(ctx, grid_ctx, 0, None, None, None);
        }

        handled
    }

    /// Handle a click at the given position.
    pub fn click_at(
        &mut self,
        loc: Vec2<u32>,
        ctx: &mut TheContext,
        grid_ctx: &mut GridCtx,
        module_type: ModuleType,
        palette: &ThePalette,
        settings: &mut Option<TheNodeUI>,
    ) -> bool {
        let mut handled = false;
        let header_height = 35;

        if loc.y < header_height {
            self.folded = !self.folded;
            grid_ctx.selected_routine = Some(self.id);
            grid_ctx.current_cell = None;
            self.draw(ctx, grid_ctx, 0, None, None, None);
            handled = true;

            ctx.ui.send(TheEvent::Custom(
                TheId::named("ModuleChanged"),
                TheValue::Empty,
            ));
        } else if !self.folded {
            for (coord, cell) in &self.grid.grid {
                if let Some(rect) = self.grid.grid_rects.get(coord) {
                    if rect.contains(Vec2::new(loc.x as i32, loc.y as i32)) {
                        grid_ctx.selected_routine = Some(self.id);
                        if grid_ctx.current_cell != Some(coord.clone()) {
                            grid_ctx.current_cell = Some(coord.clone());

                            let nodeui: TheNodeUI = cell.create_settings(palette, module_type);
                            *settings = Some(nodeui);

                            self.draw(ctx, grid_ctx, 0, None, None, None);
                        }
                        handled = true;
                        break;
                    }
                }
            }
        }

        handled
    }

    /// Handle menu context at the given position.
    pub fn context_at(
        &mut self,
        loc: Vec2<u32>,
        _ctx: &TheContext,
        grid_ctx: &mut GridCtx,
    ) -> Option<TheContextMenu> {
        for (coord, item) in &self.grid.grid {
            if let Some(rect) = self.grid.grid_rects.get(coord) {
                if rect.contains(Vec2::new(loc.x as i32, loc.y as i32)) {
                    grid_ctx.selected_routine = Some(self.id);
                    grid_ctx.current_cell = Some(coord.clone());
                    return Some(item.generate_context());
                }
            }
        }

        None
    }

    pub fn hover_status_at(&self, loc: Vec2<u32>) -> Option<String> {
        let header_height = 35;

        if loc.y < header_height {
            let desc = self.get_description();
            if !desc.is_empty() {
                return Some(desc);
            }
        } else if !self.folded {
            for (coord, item) in &self.grid.grid {
                if let Some(rect) = self.grid.grid_rects.get(coord)
                    && rect.contains(Vec2::new(loc.x as i32, loc.y as i32))
                {
                    let status = item.cell.status();
                    if !status.is_empty() {
                        return Some(status);
                    }
                }
            }
        }

        None
    }

    /// Build the routine into shader source
    pub fn build_shader(&self, out: &mut String, indent: usize) {
        let mut indent = indent;

        *out += "fn shade() {\n";
        indent += 4;

        if self.scale != 1.0 {
            *out += &format!("    uv /= {};\n", self.scale);
        }

        if self.rotation != 0.0 {
            *out += &format!("    uv = rotate2d(uv, {});\n", self.rotation);
        }

        if self.pixelization > 0 {
            *out += &format!(
                "    uv = floor(uv * {}) / {};\n",
                self.pixelization, self.pixelization
            );
        }

        let reserved_vars = vec![
            "color",
            "roughness",
            "metallic",
            "uv",
            "normal",
            "bump",
            "opacity",
        ];

        let rows = self.grid.grid_by_rows();

        let mut var_lookup: FxHashSet<String> = FxHashSet::default();

        for row in rows {
            let mut row_code = String::new();

            let mut is_if = false;
            let mut is_else = false;
            let mut ind = indent;

            for (index, (item, pos)) in row.iter().enumerate() {
                let item_code = item.code();
                if index == 0 {
                    // Add a let for a variable definition
                    if matches!(item.cell, Cell::Variable(_))
                        && !reserved_vars.contains(&item_code.as_str())
                        && !var_lookup.contains(&item_code)
                    {
                        row_code += "let ";
                        var_lookup.insert(item.code());
                    }

                    if matches!(item.cell, Cell::If) {
                        is_if = true;
                    }
                    if matches!(item.cell, Cell::Else) {
                        is_else = true;
                    }

                    if let Some(i) = self.grid.row_indents.get(&pos.1) {
                        ind += *i as usize * 4;
                    }
                }

                row_code += &item_code;
                if item.cell.role() == CellRole::Function && item.form == CellItemForm::Rounded {
                    row_code += ")";
                }

                if !item.description.is_empty() {
                    // Check if we need to insert a "," or ")"
                    if let Some(next) = self.grid.grid.get(&(pos.0 + 1, pos.1)) {
                        if !next.description.is_empty() {
                            row_code += ", ";
                        } else {
                            row_code += ") ";
                        }
                    } else {
                        row_code += ") ";
                    }
                }

                if index == row.len() - 1 {
                    if is_if || is_else {
                        row_code += "{";
                    } else {
                        row_code += " ";
                    }
                } else {
                    row_code += " ";
                }
            }

            row_code += ";";
            *out += &format!("{:ind$}{}\n", "", row_code);
        }

        if self.color_steps > 0 {
            *out += &format!(
                "    color = floor(color * {}) / {};\n",
                self.color_steps, self.color_steps
            );
        }

        *out += "}\n";
    }

    /// Build the routine into source
    pub fn build_source(&self, out: &mut String, indent: usize, debug: bool) {
        let mut indent = indent;

        if self.name != "instantiation" {
            let mut handled = false;
            if self.name.starts_with("intent: ") {
                if let Some(cmd) = self.name.strip_prefix("intent: ") {
                    *out += &format!(
                        "{:indent$}if event == \"intent\" && value == \"{}\" {{\n",
                        "", cmd
                    );
                    handled = true;
                }
            }

            if !handled {
                *out += &format!("{:indent$}if event == \"{}\" {{\n", "", self.name);
            }
            indent += 4;
            if debug {
                *out += &format!("{:indent$}mark_debug_header(\"{}\");\n", "", self.name);
            }
        }

        if self.name == "take_damage" {
            // *out += &format!("{:indent$}amount = value[\"amount\"]\n", "");
            // *out += &format!("{:indent$}from_id = value[\"from\"]\n", "");
            *out += &format!("{:indent$}let from_id = value.subject_id;\n", "");
            *out += &format!("{:indent$}let amount = value.amount;\n", "");
            *out += &format!("{:indent$}let damage_kind = value.string;\n", "");
            *out += &format!(
                "{:indent$}let attacker_name = get_attr_of( from_id,  \"name\");\n",
                ""
            );
        } else if self.name == "intent" {
            // *out += &format!("{:indent$}intent = value[\"intent\"]\n", "");
            // *out += &format!("{:indent$}distance = value[\"distance\"]\n", "");
            // *out += &format!(
            //     "{:indent$}item_id = value[\"item_id\"] if \"item_id\" in value else -1\n",
            //     ""
            // );
            // *out += &format!(
            //     "{:indent$}entity_id = value[\"entity_id\"] if \"entity_id\" in value else -1\n",
            //     ""
            // );
            // *out += &format!(
            //     "{:indent$}target_id = value[\"target_id\"] if \"target_id\" in value else value.get(\"item_id\")\n",
            //     ""
            // );
            *out += &format!("{:indent$}let intent = value.string;\n", "");
            *out += &format!("{:indent$}let distance = value.distance;\n", "");
        } else if self.name == "key_down" || self.name == "key_up" {
            *out += &format!("{:indent$}let key = value;\n", "");
        }

        let rows = self.grid.grid_by_rows();

        // If empty just add a "pass" statement
        // if rows.len() <= 1 {
        //     *out += &format!("{:indent$}pass\n", "");
        // }
        //

        let mut prev_row_indent = indent;
        let mut prev_row_was_if_header = false;

        for row in rows {
            let mut is_if = false;
            let mut is_else = false;
            let mut row_code = String::new();
            let mut assignment_debug: Option<(String, (u32, u32))> = None;

            let mut ind = indent;

            if debug {
                for (_, (item, pos)) in row.iter().enumerate() {
                    // Add debug code
                    if item.cell.role() == CellRole::Function {
                        row_code +=
                            &format!("set_debug_loc(\"{}\", {}, {}); ", self.name, pos.0, pos.1);
                    }
                }
            }

            for (index, (item, pos)) in row.iter().enumerate() {
                if index == 0 {
                    if matches!(item.cell, Cell::If) {
                        is_if = true;
                    }
                    if matches!(item.cell, Cell::Else) {
                        is_else = true;
                    }

                    if row.len() > 1 && matches!(item.cell, Cell::Variable(_)) {
                        row_code += "let ";
                        if let Cell::Variable(name) = &item.cell {
                            assignment_debug = Some((name.clone(), *pos));
                        }
                    }

                    if let Some(i) = self.grid.row_indents.get(&pos.1) {
                        let target_ind = indent + *i as usize * 4;
                        while prev_row_indent > target_ind {
                            prev_row_indent -= 4;
                            *out += &format!("{:ind$}}}\n", "", ind = prev_row_indent);
                        }
                        ind = target_ind;
                    } else if prev_row_indent > ind {
                        while prev_row_indent > ind {
                            prev_row_indent -= 4;
                            *out += &format!("{:ind$}}}\n", "", ind = prev_row_indent);
                        }
                    }
                }

                row_code += &item.code();
                if item.cell.role() == CellRole::Function && item.form == CellItemForm::Rounded {
                    row_code += ")";
                }

                if !item.description.is_empty() {
                    // Check if we need to insert a "," or ")"
                    if let Some(next) = self.grid.grid.get(&(pos.0 + 1, pos.1)) {
                        if !next.description.is_empty() {
                            row_code += ", ";
                        } else {
                            row_code += ") ";
                        }
                    } else {
                        row_code += ") ";
                    }
                }

                if index == row.len() - 1 {
                    if is_if || is_else {
                        row_code += "{";
                    } else {
                        row_code += " ";
                    }
                } else {
                    row_code += " ";
                }
            }

            let mut cleaned = row_code.trim().to_string();

            if !cleaned.is_empty() {
                // If an else follows an if header at the same indent without any body rows,
                // close the empty if block first so output stays syntactically valid.
                if is_else && prev_row_was_if_header && prev_row_indent == ind {
                    *out += &format!("{:ind$}}}\n", "", ind = ind);
                }

                if !cleaned.ends_with("{") {
                    cleaned += ";";
                }

                if debug && is_if && cleaned.ends_with('{') {
                    if let Some(condition) = cleaned
                        .strip_prefix("if ")
                        .and_then(|s| s.strip_suffix('{'))
                        .map(str::trim)
                        && let Some((_, first_pos)) = row.first()
                    {
                        let cond_var = format!("__cgfx_cond_{}_{}", first_pos.0, first_pos.1);
                        *out +=
                            &format!("{:ind$}let {} = {};\n", "", cond_var, condition, ind = ind);
                        *out += &format!(
                            "{:ind$}set_debug_condition(\"{}\", {}, {}, {});\n",
                            "",
                            self.name,
                            first_pos.0,
                            first_pos.1,
                            cond_var,
                            ind = ind
                        );
                        *out += &format!("{:ind$}if {} {{\n", "", cond_var, ind = ind);
                    } else {
                        *out += &format!("{:ind$}{}\n", "", cleaned);
                    }
                } else {
                    *out += &format!("{:ind$}{}\n", "", cleaned);
                    if debug
                        && !is_if
                        && !is_else
                        && let Some((var_name, (x, y))) = &assignment_debug
                    {
                        *out += &format!(
                            "{:ind$}set_debug_value(\"{}\", {}, {}, {});\n",
                            "",
                            self.name,
                            x,
                            y,
                            var_name,
                            ind = ind
                        );
                    }
                }
                prev_row_indent = ind;
                prev_row_was_if_header = is_if && cleaned.ends_with('{');
            }
        }

        while prev_row_indent > indent {
            prev_row_indent -= 4;
            *out += &format!("{:ind$}}}\n", "", ind = prev_row_indent);
        }

        if self.name != "instantiation" {
            indent -= 4;
            *out += &format!("{:indent$}}}\n", "");
        }
    }

    /// Get the description of the event
    fn get_description(&self) -> String {
        if self.name.starts_with("intent: ") {
            if let Some(cmd) = self.name.strip_prefix("intent: ") {
                return format!("Send on '{}' intent", cmd);
            }
        }

        match self.name.as_str() {
            "startup" => "send on startup, 'value' contains the ID".into(),
            "instantiation" => "".into(),
            "proximity_warning" => "'value' is a list of entity IDs in proximity".into(),
            "closed_in" => "`value` is the entity ID".into(),
            "take_damage" => "`amount` is final damage, `from_id` is the ID, `damage_kind` is the type, `attacker_name` resolves the name".into(),
            "death" => "send on death".into(),
            "kill" => "`value` is the killed entity's ID".into(),
            "arrived" => "`value` is the sector name".into(),
            "intent" => "'value' or `intent` is the command.".into(),
            "bumped_by_entity" => "`value` is the entity ID".into(),
            "bumped_into_entity" => "`value` is the entity ID".into(),
            "bumped_into_item" => "`value` is the item ID".into(),
            "active" => "`value` is the active state of the item".into(),
            "goodbye" => "`value` is the entity ID".into(),
            "entered" => "`value` is the sector name".into(),
            "left" => "`value` is the sector name".into(),
            "key_down" => "'key' contains the pressed key string".into(),
            "key_up" => "'key' contains the released key string".into(),

            "shader" | "ceiling_shader" => {
                "Adjust 'color', 'roughness', 'metallic', 'normal' variables".into()
            }

            _ => "custom event".into(),
        }
    }
}

fn default_i32_zero() -> i32 {
    0
}

fn deserialize_i32_from_any<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i as i32)
            } else if let Some(u) = n.as_u64() {
                Ok(u as i32)
            } else if let Some(f) = n.as_f64() {
                Ok(f as i32)
            } else {
                Err(de::Error::custom("invalid number for module_offset"))
            }
        }
        _ => Err(de::Error::custom("expected number for module_offset")),
    }
}
