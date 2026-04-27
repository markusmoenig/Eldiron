use crate::editor::{ACTIONLIST, UNDOMANAGER};
use crate::prelude::*;
use crate::undo::project_helper::{apply_palette, refresh_palette_runtime};
use rusterix::PixelSource;
use shared::project::PaletteMaterial;

const PALETTE_DOCK_PICKER: &str = "Palette Dock Picker";
const PALETTE_DOCK_HEX: &str = "Palette Dock Hex Edit";
const PALETTE_DOCK_ROUGHNESS: &str = "Palette Dock Roughness";
const PALETTE_DOCK_METALLIC: &str = "Palette Dock Metallic";
const PALETTE_DOCK_OPACITY: &str = "Palette Dock Opacity";
const PALETTE_DOCK_EMISSIVE: &str = "Palette Dock Emissive";

pub(crate) struct PaletteDockBoard {
    id: TheId,
    limiter: TheSizeLimiter,
    dim: TheDim,
    is_dirty: bool,
    palette: ThePalette,
    materials: Vec<PaletteMaterial>,
    index: usize,
    drag_index: Option<usize>,
    hovered_index: Option<usize>,
    rectangles: Vec<(usize, TheDim)>,
}

impl PaletteDockBoard {
    fn slot_is_occupied(color: &Option<TheColor>) -> bool {
        color.as_ref().is_some_and(|c| c.a > f32::EPSILON)
    }

    fn new(id: TheId) -> Self {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(i32::MAX, i32::MAX));
        Self {
            id,
            limiter,
            dim: TheDim::zero(),
            is_dirty: true,
            palette: ThePalette::default(),
            materials: vec![PaletteMaterial::default(); 256],
            index: 0,
            drag_index: None,
            hovered_index: None,
            rectangles: Vec::new(),
        }
    }

    pub(crate) fn set_palette(&mut self, palette: ThePalette) {
        self.palette = palette;
        self.is_dirty = true;
    }

    pub(crate) fn set_index(&mut self, index: usize) {
        self.index = index.min(self.palette.colors.len().saturating_sub(1));
        self.is_dirty = true;
    }

    pub(crate) fn set_materials(&mut self, materials: Vec<PaletteMaterial>) {
        self.materials = materials;
        self.is_dirty = true;
    }

    fn visible_count(&self) -> usize {
        self.palette
            .colors
            .iter()
            .rposition(Self::slot_is_occupied)
            .map(|i| i + 1)
            .unwrap_or(1)
    }

    fn calc_layout(&self) -> (i32, i32, i32) {
        const PAD_X: i32 = 10;
        const PAD_Y: i32 = 8;
        const SPACING: i32 = 1;
        const MIN_CELL: i32 = 14;

        let count = self.visible_count();
        let aw = (self.dim.width - PAD_X * 2).max(MIN_CELL);
        let ah = (self.dim.height - PAD_Y * 2).max(MIN_CELL);
        let max_cols = ((aw + SPACING) / (MIN_CELL + SPACING))
            .max(1)
            .min(count as i32);

        let mut best = (1, count as i32, MIN_CELL);
        for cols in 1..=max_cols {
            let rows = (count as i32 + cols - 1) / cols;
            let cell_w = (aw - (cols - 1) * SPACING) / cols;
            let cell_h = (ah - (rows - 1) * SPACING) / rows;
            let cell = cell_w.min(cell_h);
            if cell < MIN_CELL {
                continue;
            }
            if cell > best.2 || (cell == best.2 && cols > best.0) {
                best = (cols, rows, cell);
            }
        }
        best
    }
}

impl TheWidget for PaletteDockBoard {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        Self::new(id)
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

    fn set_dim(&mut self, dim: TheDim, _ctx: &mut TheContext) {
        if self.dim != dim {
            self.dim = dim;
            self.is_dirty = true;
        }
    }

    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }

    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
    }

    fn needs_redraw(&mut self) -> bool {
        self.is_dirty
    }

    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        match event {
            TheEvent::MouseDown(coord) => {
                for (index, rect) in &self.rectangles {
                    if rect.contains(*coord) {
                        ctx.ui.set_focus(self.id());
                        self.drag_index = Some(*index);
                        self.index = *index;
                        ctx.ui.send(TheEvent::PaletteIndexChanged(
                            self.id.clone(),
                            *index as u16,
                        ));
                        self.is_dirty = true;
                        return true;
                    }
                }
            }
            TheEvent::Hover(coord) => {
                let mut hovered = None;
                for (index, rect) in &self.rectangles {
                    if rect.contains(*coord) {
                        hovered = Some(*index);
                        break;
                    }
                }
                if hovered != self.hovered_index {
                    self.hovered_index = hovered;
                    let text = hovered
                        .map(|index| {
                            crate::undo::project_helper::palette_status_text(
                                index,
                                self.palette.colors.get(index).and_then(|c| c.as_ref()),
                                self.materials.get(index),
                            )
                        })
                        .unwrap_or_default();
                    ctx.ui.send(TheEvent::SetStatusText(self.id.clone(), text));
                    self.is_dirty = true;
                    return true;
                }
            }
            TheEvent::MouseUp(coord) => {
                if let Some(from) = self.drag_index.take() {
                    for (to, rect) in &self.rectangles {
                        if rect.contains(*coord) {
                            if from != *to {
                                ctx.ui.send(TheEvent::PaletteEntriesSwapped(
                                    self.id.clone(),
                                    from as u16,
                                    *to as u16,
                                ));
                            }
                            break;
                        }
                    }
                }
            }
            TheEvent::LostHover(_id) => {
                self.hovered_index = None;
                ctx.ui
                    .send(TheEvent::SetStatusText(self.id.clone(), String::new()));
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Delete)) => {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Palette Dock Delete Entry"),
                    TheValue::Int(self.index as i32),
                ));
                return true;
            }
            _ => {}
        }
        false
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

        let utuple = self.dim.to_buffer_utuple();
        let stride = buffer.stride();
        ctx.draw.rect(
            buffer.pixels_mut(),
            &utuple,
            stride,
            style.theme().color(ListLayoutBackground),
        );

        let (columns, rows, item_width) = self.calc_layout();
        let item_spacing = 1usize;
        let mut index = 0usize;
        let count = self.visible_count();
        self.rectangles.clear();

        let mut y_off = 8usize;
        for _ in 0..rows {
            let mut x_off = 10usize;
            for _ in 0..columns {
                if index >= count {
                    break;
                }
                let outer_rect = (
                    utuple.0 + x_off,
                    utuple.1 + y_off,
                    item_width as usize,
                    item_width as usize,
                );
                let inner_border_rect = (
                    utuple.0 + x_off + 1,
                    utuple.1 + y_off + 1,
                    (item_width as usize).saturating_sub(2),
                    (item_width as usize).saturating_sub(2),
                );
                let fill_rect = (
                    utuple.0 + x_off + 2,
                    utuple.1 + y_off + 2,
                    (item_width as usize).saturating_sub(4),
                    (item_width as usize).saturating_sub(4),
                );
                if self.index == index {
                    ctx.draw
                        .rect_outline(buffer.pixels_mut(), &outer_rect, stride, &WHITE);
                }
                ctx.draw
                    .rect_outline(buffer.pixels_mut(), &inner_border_rect, stride, &BLACK);
                if let Some(Some(color)) = self.palette.colors.get(index) {
                    ctx.draw.rect(
                        buffer.pixels_mut(),
                        &fill_rect,
                        stride,
                        &color.to_u8_array(),
                    );
                }
                self.rectangles.push((
                    index,
                    TheDim::new(x_off as i32, y_off as i32, item_width, item_width),
                ));
                index += 1;
                x_off += item_width as usize + item_spacing;
            }
            y_off += item_width as usize + item_spacing;
            if index >= count {
                break;
            }
        }
        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub struct PaletteDock {
    nodeui: TheNodeUI,
}

impl PaletteDock {
    fn make_opaque(color: &mut Option<TheColor>) {
        if let Some(color) = color {
            color.a = 1.0;
        }
    }

    fn normalize_palette(project: &mut Project) -> bool {
        let mut changed = false;
        project.ensure_palette_materials_len();
        for (index, color) in project.palette.colors.iter_mut().enumerate() {
            if !PaletteDockBoard::slot_is_occupied(color) && color.is_some() {
                *color = None;
                if let Some(material) = project.palette_materials.get_mut(index) {
                    *material = PaletteMaterial::default();
                }
                changed = true;
            }
        }
        changed
    }

    fn append_index(project: &Project) -> Option<usize> {
        let end = project
            .palette
            .colors
            .iter()
            .rposition(PaletteDockBoard::slot_is_occupied)
            .map(|i| i + 1)
            .unwrap_or(0);
        (end < project.palette.colors.len()).then_some(end)
    }

    fn build_nodeui() -> TheNodeUI {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Text(
            PALETTE_DOCK_HEX.into(),
            "Hex".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            PALETTE_DOCK_ROUGHNESS.into(),
            fl!("roughness").into(),
            "".into(),
            0.5,
            0.0..=1.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            PALETTE_DOCK_METALLIC.into(),
            fl!("metallic").into(),
            "".into(),
            0.0,
            0.0..=1.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            PALETTE_DOCK_OPACITY.into(),
            fl!("opacity").into(),
            "".into(),
            1.0,
            0.0..=1.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            PALETTE_DOCK_EMISSIVE.into(),
            fl!("emissive").into(),
            "".into(),
            0.0,
            0.0..=1.0,
            false,
        ));
        nodeui
    }

    fn palette_undo_atom(
        prev_palette: ThePalette,
        prev_materials: Vec<PaletteMaterial>,
        project: &Project,
    ) -> ProjectUndoAtom {
        ProjectUndoAtom::PaletteEdit(
            prev_palette,
            prev_materials,
            project.palette.clone(),
            project.palette_materials.clone(),
        )
    }

    fn sync_widgets(&mut self, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        let index = project.palette.current_index as usize;
        if let Some(widget) = ui.get_widget(PALETTE_DOCK_PICKER)
            && let Some(board) = widget.as_any().downcast_mut::<PaletteDockBoard>()
        {
            board.set_palette(project.palette.clone());
            board.set_materials(project.palette_materials.clone());
            board.set_index(index);
        }
        let text = project.palette[index]
            .as_ref()
            .map(TheColor::to_hex)
            .unwrap_or_default();
        self.nodeui.set_text_value(PALETTE_DOCK_HEX, text);
        let material = project
            .palette_materials
            .get(index)
            .cloned()
            .unwrap_or_default();
        self.nodeui
            .set_f32_value(PALETTE_DOCK_ROUGHNESS, material.roughness);
        self.nodeui
            .set_f32_value(PALETTE_DOCK_METALLIC, material.metallic);
        self.nodeui
            .set_f32_value(PALETTE_DOCK_OPACITY, material.opacity);
        self.nodeui
            .set_f32_value(PALETTE_DOCK_EMISSIVE, material.emissive);

        if let Some(layout) = ui.get_text_layout("Palette Dock Inspector Layout") {
            self.nodeui.apply_to_text_layout(layout);
            ctx.ui.relayout = true;
        }
    }

    fn current_selection_tool_type(project: &Project, server_ctx: &ServerContext) -> MapToolType {
        if let Some(map) = project.get_map(server_ctx) {
            if !map.selected_vertices.is_empty() {
                MapToolType::Vertex
            } else if !map.selected_linedefs.is_empty() {
                MapToolType::Linedef
            } else if !map.selected_sectors.is_empty() {
                MapToolType::Sector
            } else {
                MapToolType::Sector
            }
        } else {
            MapToolType::Sector
        }
    }

    fn apply_current_palette_color(
        &self,
        project: &mut Project,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        let source = PixelSource::PaletteIndex(project.palette.current_index);
        server_ctx.curr_map_tool_type = Self::current_selection_tool_type(project, server_ctx);

        let mut undo_atom: Option<ProjectUndoAtom> = None;
        let mut needs_scene_redraw = false;

        if let Some(map) = project.get_map_mut(server_ctx) {
            let prev = map.clone();
            if crate::actions::apply_builder_hud_material_to_selection(
                map,
                server_ctx,
                server_ctx.selected_hud_icon_index,
                Some(source.clone()),
            ) {
                undo_atom = Some(ProjectUndoAtom::MapEdit(
                    server_ctx.pc,
                    Box::new(prev),
                    Box::new(map.clone()),
                ));
                needs_scene_redraw = true;
            } else {
                let mut changed = false;
                for sector_id in map.selected_sectors.clone() {
                    let mut source_key = "source";
                    if server_ctx.pc.is_screen() && server_ctx.selected_hud_icon_index == 1 {
                        source_key = "ceiling_source";
                    }
                    changed |= crate::utils::apply_surface_source_to_sector(
                        map,
                        sector_id,
                        source_key,
                        &crate::utils::SurfaceApplySource::Direct(source.clone()),
                        None,
                    );
                }
                if changed {
                    map.update_surfaces();
                    undo_atom = Some(ProjectUndoAtom::MapEdit(
                        server_ctx.pc,
                        Box::new(prev),
                        Box::new(map.clone()),
                    ));
                    needs_scene_redraw = true;
                }
            }
        }

        if needs_scene_redraw && let Some(undo_atom) = &undo_atom {
            crate::utils::editor_scene_apply_map_edit_atom(project, server_ctx, undo_atom);
        }
        if let Some(undo_atom) = undo_atom {
            UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Update Minimap"),
                TheValue::Empty,
            ));
        }
    }

    fn clear_current_palette_color(
        &self,
        project: &mut Project,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        server_ctx.curr_map_tool_type = Self::current_selection_tool_type(project, server_ctx);

        let mut cleared_action_slot = false;
        let mut undo_atom: Option<ProjectUndoAtom> = None;
        let mut needs_scene_redraw = false;

        if let Some(map) = project.get_map_mut(server_ctx) {
            let prev = map.clone();
            if crate::actions::apply_builder_hud_material_to_selection(
                map,
                server_ctx,
                server_ctx.selected_hud_icon_index,
                None,
            ) {
                undo_atom = Some(ProjectUndoAtom::MapEdit(
                    server_ctx.pc,
                    Box::new(prev),
                    Box::new(map.clone()),
                ));
                needs_scene_redraw = true;
                cleared_action_slot = true;
            }
        }

        if !cleared_action_slot
            && server_ctx.get_map_context() == MapContext::Region
            && let Some(map) = project.get_map(server_ctx)
            && let Some(action_id) = server_ctx.curr_action_id
            && let Some(action) = ACTIONLIST.write().unwrap().get_action_by_id_mut(action_id)
            && action.hud_material_slots(map, server_ctx).is_some()
            && action.clear_hud_material_slot(map, server_ctx, server_ctx.selected_hud_icon_index)
        {
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Refresh Action Parameters"),
                TheValue::Empty,
            ));
            cleared_action_slot = true;
        }

        if !cleared_action_slot && let Some(map) = project.get_map_mut(server_ctx) {
            let mut changed = false;
            let prev = map.clone();
            for sector_id in map.selected_sectors.clone() {
                let mut source_key = "source";
                if server_ctx.pc.is_screen() && server_ctx.selected_hud_icon_index == 1 {
                    source_key = "ceiling_source";
                }
                changed |= crate::utils::clear_surface_source_on_sector(map, sector_id, source_key);
            }

            if changed {
                map.update_surfaces();
                undo_atom = Some(ProjectUndoAtom::MapEdit(
                    server_ctx.pc,
                    Box::new(prev),
                    Box::new(map.clone()),
                ));
                needs_scene_redraw = true;
            }
        }

        if needs_scene_redraw && let Some(undo_atom) = &undo_atom {
            crate::utils::editor_scene_apply_map_edit_atom(project, server_ctx, undo_atom);
        }
        if let Some(undo_atom) = undo_atom {
            UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Update Minimap"),
                TheValue::Empty,
            ));
        }
    }
}

impl Dock for PaletteDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            nodeui: Self::build_nodeui(),
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut top_canvas = TheCanvas::default();
        top_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut top_layout = TheHLayout::new(TheId::empty());
        top_layout.set_background_color(None);
        top_layout.set_margin(Vec4::new(10, 1, 5, 1));
        top_layout.set_padding(3);

        let mut apply = TheTraybarButton::new(TheId::named("Palette Dock Apply Color"));
        apply.set_text(fl!("palette_apply_color"));
        apply.set_status_text(&fl!("status_palette_apply_color"));
        let mut new_button = TheTraybarButton::new(TheId::named("Palette Dock New"));
        new_button.set_text(fl!("new"));
        top_layout.add_widget(Box::new(new_button));

        let mut clone_button = TheTraybarButton::new(TheId::named("Palette Dock Clone"));
        clone_button.set_text(fl!("action_duplicate"));
        top_layout.add_widget(Box::new(clone_button));

        top_layout.add_widget(Box::new(apply));

        let mut clear = TheTraybarButton::new(TheId::named("Palette Dock Clear Color"));
        clear.set_text(fl!("clear"));
        clear.set_status_text(&fl!("status_tiles_clear_tile"));
        top_layout.add_widget(Box::new(clear));
        top_layout.set_reverse_index(Some(2));
        top_canvas.set_layout(top_layout);
        canvas.set_top(top_canvas);

        let mut center = TheCanvas::new();

        let mut picker_canvas = TheCanvas::new();
        picker_canvas.set_widget(PaletteDockBoard::new(TheId::named(PALETTE_DOCK_PICKER)));
        center.set_center(picker_canvas);

        let mut inspector_canvas = TheCanvas::new();
        inspector_canvas.limiter_mut().set_min_width(300);
        inspector_canvas.limiter_mut().set_max_width(300);
        let mut inspector = TheTextLayout::new(TheId::named("Palette Dock Inspector Layout"));
        inspector.limiter_mut().set_min_width(300);
        inspector.limiter_mut().set_max_width(300);
        inspector.set_text_margin(20);
        inspector.set_text_align(TheHorizontalAlign::Right);
        inspector_canvas.set_layout(inspector);
        center.set_right(inspector_canvas);

        canvas.set_center(center);
        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        _server_ctx: &mut ServerContext,
    ) {
        self.sync_widgets(ui, ctx, project);
    }

    fn supports_actions(&self) -> bool {
        true
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if Self::normalize_palette(project) {
            apply_palette(ui, ctx, server_ctx, project);
            refresh_palette_runtime(project);
        }
        match event {
            TheEvent::PaletteEntriesSwapped(id, from, to) if id.name == PALETTE_DOCK_PICKER => {
                let from = *from as usize;
                let to = *to as usize;
                if from < project.palette.colors.len()
                    && to < project.palette.colors.len()
                    && from != to
                {
                    let prev = project.palette.clone();
                    let prev_materials = project.palette_materials.clone();
                    project.palette.colors.swap(from, to);
                    project.palette_materials.swap(from, to);
                    Self::make_opaque(&mut project.palette.colors[from]);
                    Self::make_opaque(&mut project.palette.colors[to]);
                    project.palette.current_index = to as u16;
                    apply_palette(ui, ctx, server_ctx, project);
                    refresh_palette_runtime(project);
                    self.sync_widgets(ui, ctx, project);
                    UNDOMANAGER
                        .write()
                        .unwrap()
                        .add_undo(Self::palette_undo_atom(prev, prev_materials, project), ctx);
                }
                true
            }
            TheEvent::PaletteIndexChanged(id, index) if id.name == PALETTE_DOCK_PICKER => {
                project.palette.current_index = *index;
                project.ensure_palette_materials_len();
                apply_palette(ui, ctx, server_ctx, project);
                self.sync_widgets(ui, ctx, project);
                true
            }
            TheEvent::ValueChanged(id, TheValue::Text(text)) if id.name == PALETTE_DOCK_HEX => {
                let color = TheColor::from_hex(text);
                let index = project.palette.current_index as usize;
                project.ensure_palette_materials_len();
                if project.palette[index] != Some(color.clone()) {
                    let prev = project.palette.clone();
                    let prev_materials = project.palette_materials.clone();
                    project.palette[index] = Some(color);
                    apply_palette(ui, ctx, server_ctx, project);
                    refresh_palette_runtime(project);
                    self.sync_widgets(ui, ctx, project);
                    UNDOMANAGER
                        .write()
                        .unwrap()
                        .add_undo(Self::palette_undo_atom(prev, prev_materials, project), ctx);
                }
                true
            }
            TheEvent::ValueChanged(id, value)
                if matches!(
                    id.name.as_str(),
                    PALETTE_DOCK_ROUGHNESS
                        | PALETTE_DOCK_METALLIC
                        | PALETTE_DOCK_OPACITY
                        | PALETTE_DOCK_EMISSIVE
                ) =>
            {
                let index = project.palette.current_index as usize;
                project.ensure_palette_materials_len();
                let prev = project.palette.clone();
                let prev_materials = project.palette_materials.clone();
                let material = &mut project.palette_materials[index];
                let Some(value) = (match value {
                    TheValue::Float(v) => Some(*v),
                    TheValue::FloatRange(v, _) => Some(*v),
                    TheValue::Text(text) => text.parse::<f32>().ok(),
                    _ => None,
                }) else {
                    return false;
                };
                let value = value.clamp(0.0, 1.0);
                let changed = match id.name.as_str() {
                    PALETTE_DOCK_ROUGHNESS => {
                        let changed = (material.roughness - value).abs() > f32::EPSILON;
                        material.roughness = value;
                        changed
                    }
                    PALETTE_DOCK_METALLIC => {
                        let changed = (material.metallic - value).abs() > f32::EPSILON;
                        material.metallic = value;
                        changed
                    }
                    PALETTE_DOCK_OPACITY => {
                        let changed = (material.opacity - value).abs() > f32::EPSILON;
                        material.opacity = value;
                        changed
                    }
                    PALETTE_DOCK_EMISSIVE => {
                        let changed = (material.emissive - value).abs() > f32::EPSILON;
                        material.emissive = value;
                        changed
                    }
                    _ => false,
                };
                if changed {
                    refresh_palette_runtime(project);
                    UNDOMANAGER
                        .write()
                        .unwrap()
                        .add_undo(Self::palette_undo_atom(prev, prev_materials, project), ctx);
                }
                true
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) => {
                if id.name == "Palette Dock New" {
                    if let Some(index) = Self::append_index(project) {
                        let prev = project.palette.clone();
                        let prev_materials = project.palette_materials.clone();
                        project.ensure_palette_materials_len();
                        project.palette.colors[index] = Some(TheColor::from_u8(0, 0, 0, 255));
                        if let Some(material) = project.palette_materials.get_mut(index) {
                            *material = PaletteMaterial::default();
                        }
                        project.palette.current_index = index as u16;
                        apply_palette(ui, ctx, server_ctx, project);
                        refresh_palette_runtime(project);
                        self.sync_widgets(ui, ctx, project);
                        UNDOMANAGER
                            .write()
                            .unwrap()
                            .add_undo(Self::palette_undo_atom(prev, prev_materials, project), ctx);
                    }
                    true
                } else if id.name == "Palette Dock Clone" {
                    if let Some(index) = Self::append_index(project) {
                        let prev = project.palette.clone();
                        let prev_materials = project.palette_materials.clone();
                        let src = project.palette.current_index as usize;
                        project.ensure_palette_materials_len();
                        project.palette.colors[index] = project.palette.colors[src].clone();
                        Self::make_opaque(&mut project.palette.colors[index]);
                        if let Some(src_material) = project.palette_materials.get(src).cloned()
                            && let Some(dst_material) = project.palette_materials.get_mut(index)
                        {
                            *dst_material = src_material;
                        }
                        project.palette.current_index = index as u16;
                        apply_palette(ui, ctx, server_ctx, project);
                        refresh_palette_runtime(project);
                        self.sync_widgets(ui, ctx, project);
                        UNDOMANAGER
                            .write()
                            .unwrap()
                            .add_undo(Self::palette_undo_atom(prev, prev_materials, project), ctx);
                    }
                    true
                } else if id.name == "Palette Dock Apply Color" {
                    self.apply_current_palette_color(project, ctx, server_ctx);
                    true
                } else if id.name == "Palette Dock Clear Color" {
                    self.clear_current_palette_color(project, ctx, server_ctx);
                    true
                } else {
                    false
                }
            }
            TheEvent::Custom(id, TheValue::Int(index))
                if id.name == "Palette Dock Delete Entry" =>
            {
                let index = *index as usize;
                if index < project.palette.colors.len() {
                    let prev = project.palette.clone();
                    let prev_materials = project.palette_materials.clone();
                    project.ensure_palette_materials_len();
                    project.palette.colors[index] = None;
                    if let Some(material) = project.palette_materials.get_mut(index) {
                        *material = PaletteMaterial::default();
                    }
                    if project.palette.current_index as usize == index {
                        project.palette.current_index = index.saturating_sub(1) as u16;
                    }
                    apply_palette(ui, ctx, server_ctx, project);
                    refresh_palette_runtime(project);
                    self.sync_widgets(ui, ctx, project);
                    UNDOMANAGER
                        .write()
                        .unwrap()
                        .add_undo(Self::palette_undo_atom(prev, prev_materials, project), ctx);
                }
                true
            }
            _ => false,
        }
    }
}
