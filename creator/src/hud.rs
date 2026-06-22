use crate::editor::{ACTIONLIST, RUSTERIX};
use crate::prelude::*;
use rusterix::prelude::*;
use theframework::prelude::*;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum HudMode {
    Selection,
    Vertex,
    Linedef,
    Sector,
    Effects,
    Rect,
    Terrain,
    Entity,
}

pub struct Hud {
    mode: HudMode,

    icon_rects: Vec<TheDim>,

    pub selected_icon_index: i32,

    subdiv_rects: Vec<TheDim>,

    mouse_pos: Vec2<i32>,

    light_icon: Option<TheRGBABuffer>,

    edit_mode_rects: Vec<TheDim>,
}

impl Hud {
    const GRID_SUBDIVISIONS: [usize; 6] = [1, 2, 4, 8, 16, 32];

    fn active_action_material_slots(
        &self,
        map: &Map,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<Vec<ActionMaterialSlot>> {
        if server_ctx.get_map_context() != MapContext::Region {
            return None;
        }

        if let Some(action_id) = server_ctx.curr_action_id
            && let Some(action) = ACTIONLIST.read().unwrap().get_action_by_id(action_id)
        {
            if let Some(slots) = action.hud_material_slots(map, server_ctx) {
                return Some(slots);
            }
            if action.is_applicable(map, ctx, server_ctx)
                && let Some(slots) = action.hud_material_slots(map, server_ctx)
            {
                return Some(slots);
            }
        }
        None
    }

    fn active_builder_item_slots(
        &self,
        map: &Map,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<Vec<ActionItemSlot>> {
        if !server_ctx.builder_tool_active {
            return None;
        }
        match server_ctx.curr_map_tool_type {
            MapToolType::Sector => crate::actions::builder_hud_item_slots_for_selected_sector(map),
            MapToolType::Linedef => {
                crate::actions::builder_hud_item_slots_for_selected_linedef(map)
            }
            MapToolType::Vertex => crate::actions::builder_hud_item_slots_for_selected_vertex(map),
            _ => None,
        }
    }

    fn active_palette_material_slots(
        &self,
        map: &Map,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<Vec<ActionMaterialSlot>> {
        if !server_ctx.palette_tool_active {
            return None;
        }
        if let Some(slots) = crate::actions::builder_hud_material_slots_for_selected_geometry(map) {
            return Some(slots);
        }
        match server_ctx.curr_map_tool_type {
            MapToolType::Sector => {
                crate::actions::builder_hud_material_slots_for_selected_sector(map)
            }
            MapToolType::Linedef => {
                crate::actions::builder_hud_material_slots_for_selected_linedef(map)
            }
            MapToolType::Vertex => {
                crate::actions::builder_hud_material_slots_for_selected_vertex(map)
            }
            _ => None,
        }
    }

    fn clean_coord(v: f32) -> f32 {
        if v.abs() < 0.000005 { 0.0 } else { v }
    }

    fn grid_step_label(subdivision: usize) -> String {
        if subdivision <= 1 {
            "1".to_string()
        } else {
            format!("1/{subdivision}")
        }
    }

    fn grid_key_label(index: usize) -> String {
        (index + 1).to_string()
    }

    fn geometry_face_source(face: &rusterix::GeometryFace) -> Option<PixelSource> {
        face.tile
            .clone()
            .or_else(|| face.tiles.values().next().cloned())
    }

    fn selected_geometry_source(map: &Map) -> Option<PixelSource> {
        if !map.selected_geometry_faces.is_empty() {
            for (object_id, face_index) in &map.selected_geometry_faces {
                if let Some(object) = map
                    .geometry_objects
                    .iter()
                    .find(|object| object.id == *object_id)
                    && let Some(face) = object.faces.get(*face_index)
                    && let Some(source) = Self::geometry_face_source(face)
                {
                    return Some(source);
                }
            }
            return None;
        }

        for object_id in &map.selected_geometry_objects {
            if let Some(object) = map
                .geometry_objects
                .iter()
                .find(|object| object.id == *object_id)
            {
                for face in &object.faces {
                    if let Some(source) = Self::geometry_face_source(face) {
                        return Some(source);
                    }
                }
            }
        }

        None
    }

    fn has_selected_geometry_surface(map: &Map) -> bool {
        !map.selected_geometry_faces.is_empty() || !map.selected_geometry_objects.is_empty()
    }

    fn active_grid_subdivision(subdivisions: f32) -> usize {
        let step = ServerContext::edit_grid_step(subdivisions);
        ((1.0 / step).round() as usize).clamp(1, 32)
    }

    fn coord_precision(subdivisions: f32) -> usize {
        match Self::active_grid_subdivision(subdivisions) {
            subdivision if subdivision >= 32 => 5,
            subdivision if subdivision >= 16 => 4,
            _ => 3,
        }
    }

    fn format_coord(v: f32, precision: usize) -> String {
        format!("{:.*}", precision, Self::clean_coord(v))
    }

    fn selected_geometry_coord(map: &Map) -> Option<Vec3<f32>> {
        let mut sum = Vec3::zero();
        let mut count = 0usize;

        let mut add_point = |point: Vec3<f32>| {
            if point.x.is_finite() && point.y.is_finite() && point.z.is_finite() {
                sum += point;
                count += 1;
            }
        };

        for (object_id, face_index, point_index) in &map.selected_geometry_surface_points {
            if let Some(object) = map
                .geometry_objects
                .iter()
                .find(|object| object.id == *object_id)
                && let Some(face) = object.faces.get(*face_index)
                && let Some(point) = face.surface_points.get(*point_index)
            {
                add_point(object.transform_point(point.position));
            }
        }

        for (object_id, face_index, segment_index) in &map.selected_geometry_surface_segments {
            if let Some(object) = map
                .geometry_objects
                .iter()
                .find(|object| object.id == *object_id)
                && let Some(face) = object.faces.get(*face_index)
                && let Some(segment) = face.surface_segments.get(*segment_index)
            {
                if let Some(point) = face.surface_points.get(segment.start) {
                    add_point(object.transform_point(point.position));
                }
                if let Some(point) = face.surface_points.get(segment.end) {
                    add_point(object.transform_point(point.position));
                }
            }
        }

        for (object_id, vertex_index) in &map.selected_geometry_vertices {
            if let Some(object) = map
                .geometry_objects
                .iter()
                .find(|object| object.id == *object_id)
                && let Some(vertex) = object.vertices.get(*vertex_index)
            {
                add_point(object.transform_point(*vertex));
            }
        }

        for (object_id, face_index) in &map.selected_geometry_faces {
            if let Some(object) = map
                .geometry_objects
                .iter()
                .find(|object| object.id == *object_id)
                && let Some(face) = object.faces.get(*face_index)
            {
                for index in &face.indices {
                    if let Some(vertex) = object.vertices.get(*index) {
                        add_point(object.transform_point(*vertex));
                    }
                }
            }
        }

        for object_id in &map.selected_geometry_objects {
            if let Some(object) = map
                .geometry_objects
                .iter()
                .find(|object| object.id == *object_id)
            {
                for vertex in &object.vertices {
                    add_point(object.transform_point(*vertex));
                }
            }
        }

        (count > 0).then(|| sum / count as f32)
    }

    fn geometry_edge_length(map: &Map) -> Option<f32> {
        let mut edges = std::collections::BTreeSet::new();
        for object in &map.geometry_objects {
            let selected = map
                .selected_geometry_vertices
                .iter()
                .filter_map(|(object_id, vertex_index)| {
                    (*object_id == object.id).then_some(*vertex_index)
                })
                .collect::<std::collections::BTreeSet<_>>();
            if selected.len() < 2 {
                continue;
            }
            for face in &object.faces {
                if face.indices.len() < 2 {
                    continue;
                }
                for index in 0..face.indices.len() {
                    let a = face.indices[index];
                    let b = face.indices[(index + 1) % face.indices.len()];
                    if selected.contains(&a) && selected.contains(&b) {
                        edges.insert((object.id, a.min(b), a.max(b)));
                    }
                }
            }
        }

        let mut length = 0.0f32;
        for (object_id, a_index, b_index) in edges {
            if let Some(object) = map
                .geometry_objects
                .iter()
                .find(|object| object.id == object_id)
                && let Some(a) = object.vertices.get(a_index)
                && let Some(b) = object.vertices.get(b_index)
            {
                length += (object.transform_point(*b) - object.transform_point(*a)).magnitude();
            }
        }
        (length > 0.0).then_some(length)
    }

    fn surface_segment_length(map: &Map) -> Option<f32> {
        let mut length = 0.0f32;
        for (object_id, face_index, segment_index) in &map.selected_geometry_surface_segments {
            if let Some(object) = map
                .geometry_objects
                .iter()
                .find(|object| object.id == *object_id)
                && let Some(face) = object.faces.get(*face_index)
                && let Some(segment) = face.surface_segments.get(*segment_index)
                && let Some(start) = face.surface_points.get(segment.start)
                && let Some(end) = face.surface_points.get(segment.end)
            {
                length += (object.transform_point(end.position)
                    - object.transform_point(start.position))
                .magnitude();
            }
        }
        (length > 0.0).then_some(length)
    }

    fn geometry_length_readout(map: &Map, server_ctx: &ServerContext) -> Option<f32> {
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            return None;
        }
        if server_ctx.curr_map_tool_type == MapToolType::Linedef
            && let Some(start) = map.curr_grid_pos_3d
            && let Some(end) = server_ctx.hover_cursor_3d
        {
            let length = (end - start).magnitude();
            if length > 0.0 {
                return Some(length);
            }
        }
        Self::surface_segment_length(map).or_else(|| Self::geometry_edge_length(map))
    }

    pub fn new(mode: HudMode) -> Self {
        Self {
            mode,

            icon_rects: vec![],
            selected_icon_index: 0,

            subdiv_rects: vec![],

            mouse_pos: Vec2::zero(),

            light_icon: None,

            edit_mode_rects: vec![],
        }
    }

    pub fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        map: &mut Map,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        id: Option<u32>,
        _assets: &Assets,
    ) {
        if (self.mode == HudMode::Linedef || self.mode == HudMode::Sector)
            && self.light_icon.is_none()
        {
            if let Some(li) = ctx.ui.icon("light_small") {
                self.light_icon = Some(li.clone());
            }
        }

        let width = buffer.dim().width as usize;
        let height = buffer.dim().height as usize;
        let stride = buffer.stride();

        let info_height = 20;
        let bg_color = [48, 48, 50, 235];
        let panel_color = [34, 34, 36, 240];
        let text_color = [168, 168, 172, 255];
        let sel_text_color = [240, 240, 242, 255];
        let accent_color = [210, 176, 92, 255];

        self.subdiv_rects = vec![];

        let icon_size = 40;
        let action_item_slots = self.active_builder_item_slots(map, ctx, server_ctx);
        let action_material_slots = if action_item_slots.is_none() {
            crate::actions::builder_hud_material_slots_for_selected_geometry(map)
                .or_else(|| self.active_action_material_slots(map, ctx, server_ctx))
                .or_else(|| self.active_palette_material_slots(map, ctx, server_ctx))
        } else {
            None
        };
        let mut icons = 0;

        if server_ctx.get_map_context() == MapContext::Region {
            icons = if let Some(slots) = &action_item_slots {
                slots.len() as i32
            } else if let Some(slots) = &action_material_slots {
                slots.len() as i32
            } else if self.mode == HudMode::Vertex {
                0
            } else if self.mode == HudMode::Linedef {
                0
            } else {
                1
            };
        } else if server_ctx.get_map_context() == MapContext::Screen {
            icons = if self.mode == HudMode::Sector { 2 } else { 0 };
        }

        if self.mode == HudMode::Effects
            || self.mode == HudMode::Rect
            || self.mode == HudMode::Terrain
        {
            icons = 0;
        }

        ctx.draw.rect(
            buffer.pixels_mut(),
            &(0, 0, width, info_height),
            stride,
            &bg_color,
        );
        ctx.draw.rect(
            buffer.pixels_mut(),
            &(0, info_height - 1, width, 1),
            stride,
            &[96, 96, 100, 255],
        );

        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            if let Some(v) = server_ctx.hover_cursor {
                ctx.draw
                    .rect(buffer.pixels_mut(), &(8, 2, 150, 16), stride, &panel_color);
                ctx.draw.text(
                    buffer.pixels_mut(),
                    &(10, 2),
                    stride,
                    &format!("{:.2}, {:.2}", v.x, v.y),
                    TheFontSettings {
                        size: 13.0,
                        ..Default::default()
                    },
                    &text_color,
                    &bg_color,
                );
            }
        } else {
            if let Some((snapped, coord_color)) = Self::selected_geometry_coord(map)
                .map(|coord| (coord, sel_text_color))
                .or_else(|| {
                    server_ctx.hover_cursor_3d.map(|v| {
                        let mut snapped = server_ctx.snap_world_point_for_edit(map, v);
                        snapped.y = v.y;
                        (snapped, text_color)
                    })
                })
            {
                let precision = Self::coord_precision(map.subdivisions);
                let panel_width = if precision >= 5 {
                    250
                } else if precision >= 4 {
                    225
                } else {
                    200
                };
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &(8, 2, panel_width, 16),
                    stride,
                    &panel_color,
                );
                ctx.draw.text(
                    buffer.pixels_mut(),
                    &(10, 2),
                    stride,
                    &format!(
                        "{}, {}, {}",
                        Self::format_coord(snapped.x, precision),
                        Self::format_coord(snapped.y, precision),
                        Self::format_coord(snapped.z, precision)
                    ),
                    TheFontSettings {
                        size: 13.0,
                        ..Default::default()
                    },
                    &coord_color,
                    &bg_color,
                );
            }

            if icons == 0
                && let Some(length) = Self::geometry_length_readout(map, server_ctx)
            {
                let precision = Self::coord_precision(map.subdivisions);
                let panel_width = if precision >= 5 { 120 } else { 100 };
                let x = width.saturating_sub(panel_width + 8);
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &(x, 2, panel_width, 16),
                    stride,
                    &panel_color,
                );
                ctx.draw.text(
                    buffer.pixels_mut(),
                    &(x + 2, 2),
                    stride,
                    &format!("Len {}", Self::format_coord(length, precision)),
                    TheFontSettings {
                        size: 13.0,
                        ..Default::default()
                    },
                    &sel_text_color,
                    &bg_color,
                );
            }
        }

        if let Some(v) = &server_ctx.background_progress {
            ctx.draw.text(
                buffer.pixels_mut(),
                &(550, 2),
                stride,
                v,
                TheFontSettings {
                    size: 13.0,
                    ..Default::default()
                },
                &text_color,
                &bg_color,
            );
        }

        // Icons

        if icons > 0 {
            server_ctx.selected_hud_icon_index =
                server_ctx.selected_hud_icon_index.clamp(0, icons - 1);
            self.selected_icon_index = server_ctx.selected_hud_icon_index;
        } else {
            self.selected_icon_index = 0;
        }

        self.icon_rects.clear();
        let x = width as i32 - (icon_size * icons) - 1;
        for i in 0..icons {
            let rect = TheDim::rect(x + (i * icon_size), 20, icon_size, icon_size);
            ctx.draw.rect(
                buffer.pixels_mut(),
                &rect.to_buffer_utuple(),
                stride,
                &bg_color,
            );

            let r = rect.to_buffer_utuple();
            ctx.draw.text_rect(
                buffer.pixels_mut(),
                &(r.0, 1, r.2, 19),
                stride,
                &self.get_icon_text(
                    i,
                    map,
                    server_ctx,
                    action_item_slots.as_deref(),
                    action_material_slots.as_deref(),
                ),
                TheFontSettings {
                    size: 10.0,
                    ..Default::default()
                },
                &text_color,
                &bg_color,
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );

            let r = &rect.to_buffer_utuple();
            ctx.draw.rect(
                buffer.pixels_mut(),
                &(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2),
                stride,
                &[30, 30, 30, 255],
            );

            let (tile, has_light) = self.get_icon(
                i,
                map,
                id,
                icon_size as usize,
                action_item_slots.as_deref(),
                action_material_slots.as_deref(),
            );
            if let Some(tile) = tile {
                let texture = tile.textures[0].resized(icon_size as usize, icon_size as usize);
                ctx.draw.blend_slice(
                    buffer.pixels_mut(),
                    &texture.data,
                    &rect.to_buffer_utuple(),
                    stride,
                );
            }
            if has_light {
                if let Some(light_icon) = &self.light_icon {
                    ctx.draw.blend_slice(
                        buffer.pixels_mut(),
                        light_icon.pixels(),
                        &(
                            rect.x as usize + 1,
                            rect.y as usize + 1,
                            light_icon.dim().width as usize,
                            light_icon.dim().height as usize,
                        ),
                        stride,
                    );
                }
            }

            if i == server_ctx.selected_hud_icon_index {
                ctx.draw.rect_outline(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &sel_text_color,
                );
            }

            self.icon_rects
                .push(TheDim::rect(rect.x, 0, rect.width, rect.y + rect.height));
        }

        // Show Subdivs
        if (map.camera == MapCamera::TwoD
            || server_ctx.get_map_context() == MapContext::Screen
            || server_ctx.editor_view_mode != EditorViewMode::D2)
            && self.mode != HudMode::Terrain
            && self.mode != HudMode::Rect
        {
            let x = if server_ctx.editor_view_mode == EditorViewMode::D2 {
                185
            } else {
                215
            };
            let size = 20i32;
            let button_height = info_height as i32 - 1;
            let active_subdivision = Self::active_grid_subdivision(map.subdivisions);
            for (i, subdivision) in Self::GRID_SUBDIVISIONS.iter().copied().enumerate() {
                let rect = TheDim::rect(x + (i as i32 * size), 0, size, button_height);
                let active = active_subdivision == subdivision;
                let hovered = rect.contains(self.mouse_pos);
                let inner = if active {
                    [68, 68, 72, 255]
                } else if hovered {
                    [54, 54, 58, 255]
                } else {
                    panel_color
                };

                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &bg_color,
                );
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &(
                        rect.x as usize + 1,
                        rect.y as usize + 2,
                        rect.width as usize - 2,
                        rect.height as usize - 3,
                    ),
                    stride,
                    &inner,
                );
                if active {
                    ctx.draw.rect(
                        buffer.pixels_mut(),
                        &(
                            rect.x as usize + 3,
                            rect.y as usize + rect.height as usize - 4,
                            rect.width as usize - 6,
                            2,
                        ),
                        stride,
                        &accent_color,
                    );
                }

                let r = rect.to_buffer_utuple();
                let label = Self::grid_key_label(i);
                ctx.draw.text_rect(
                    buffer.pixels_mut(),
                    &(r.0, 1, r.2, rect.height as usize - 1),
                    stride,
                    &label,
                    TheFontSettings {
                        size: 12.5,
                        ..Default::default()
                    },
                    &if active || hovered {
                        sel_text_color
                    } else {
                        text_color
                    },
                    &bg_color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
                self.subdiv_rects.push(rect);
            }

            if server_ctx.editor_view_mode != EditorViewMode::D2
                && server_ctx.get_map_context() == MapContext::Region
            {
                let rect = TheDim::rect(
                    x + (Self::GRID_SUBDIVISIONS.len() as i32 * size) + 6,
                    2,
                    52,
                    16,
                );
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &panel_color,
                );
                ctx.draw.text_rect(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &Self::grid_step_label(active_subdivision),
                    TheFontSettings {
                        size: 11.0,
                        ..Default::default()
                    },
                    &sel_text_color,
                    &bg_color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }
        }

        self.edit_mode_rects.clear();
        if self.mode == HudMode::Selection
            && server_ctx.editor_view_mode != EditorViewMode::D2
            && server_ctx.get_map_context() == MapContext::Region
        {
            let labels = [fl!("hud_geometry_op_move"), fl!("hud_geometry_op_size")];
            let modes = [GeometryGizmoOp::Move, GeometryGizmoOp::Resize];
            let start_x = 432;
            let button_w = 52;

            for i in 0..2 {
                let rect = TheDim::rect(start_x + (i as i32 * (button_w + 4)), 0, button_w, 20);
                let active = server_ctx.geometry_gizmo_op == modes[i];
                let fg = if active { sel_text_color } else { text_color };
                let inner = if active {
                    [60, 60, 64, 255]
                } else {
                    panel_color
                };

                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &bg_color,
                );
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &(
                        rect.x as usize + 1,
                        rect.y as usize + 2,
                        rect.width as usize - 2,
                        rect.height as usize - 4,
                    ),
                    stride,
                    &inner,
                );
                ctx.draw.text_rect(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &labels[i],
                    TheFontSettings {
                        size: 11.0,
                        ..Default::default()
                    },
                    &fg,
                    &inner,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
                if active {
                    ctx.draw.rect_outline(
                        buffer.pixels_mut(),
                        &rect.to_buffer_utuple(),
                        stride,
                        &sel_text_color,
                    );
                    ctx.draw.rect(
                        buffer.pixels_mut(),
                        &(
                            rect.x as usize + 4,
                            rect.y as usize + 16,
                            rect.width as usize - 8,
                            2,
                        ),
                        stride,
                        &accent_color,
                    );
                }
                self.edit_mode_rects.push(rect);
            }
        }

        // Terrain: Height
        if self.mode == HudMode::Terrain {
            if let Some(v) = server_ctx.hover_height {
                ctx.draw.text(
                    buffer.pixels_mut(),
                    &(150, 2),
                    stride,
                    &format!("Elevation {v:.2}"),
                    TheFontSettings {
                        size: 13.0,
                        ..Default::default()
                    },
                    &text_color,
                    &bg_color,
                );
            }
        }

        // Preview

        if server_ctx.get_map_context() == MapContext::Character
            || server_ctx.get_map_context() == MapContext::Item
        {
            if let Some(Value::Texture(texture)) = map.properties.get("shape") {
                let w = texture.width as i32;
                let h = texture.height as i32;
                let preview_rect = TheDim::rect(width as i32 - w - 1, height as i32 - h - 1, w, h);
                ctx.draw.blend_slice(
                    buffer.pixels_mut(),
                    &texture.data,
                    &preview_rect.to_buffer_utuple(),
                    stride,
                );
            }
        } else if server_ctx.get_map_context() == MapContext::Screen {
            //let mut stack = ShapeStack::new(Vec2::new(-5.0, -5.0), Vec2::new(5.0, 5.0));
            //stack.render_geometry(&mut texture, map, assets, false, &FxHashMap::default());

            /*
            if let Some(Value::Texture(texture)) = map.properties.get("shape") {
                let w = texture.width as i32;
                let h = texture.height as i32;
                let preview_rect = TheDim::rect(width as i32 - w - 1, height as i32 - h - 1, w, h);
                ctx.draw.copy_slice(
                    buffer.pixels_mut(),
                    &texture.data,
                    &preview_rect.to_buffer_utuple(),
                    stride,
                );
            }*/
        }

        let _ = (width, height, stride);
    }

    pub fn clicked(
        &mut self,
        x: i32,
        y: i32,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if server_ctx.get_map_context() != MapContext::Region
            && server_ctx.get_map_context() != MapContext::Screen
            && server_ctx.get_map_context() != MapContext::Character
            && server_ctx.get_map_context() != MapContext::Item
        {
            return false;
        }

        for (i, rect) in self.edit_mode_rects.iter().enumerate() {
            if rect.contains(Vec2::new(x, y)) {
                server_ctx.geometry_gizmo_op = if i == 1 {
                    GeometryGizmoOp::Resize
                } else {
                    GeometryGizmoOp::Move
                };
                RUSTERIX.write().unwrap().set_overlay_dirty();
                return true;
            }
        }

        for (i, rect) in self.icon_rects.iter().enumerate() {
            if rect.contains(Vec2::new(x, y)) {
                self.selected_icon_index = i as i32;
                server_ctx.selected_hud_icon_index = i as i32;
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Minimap"),
                    TheValue::Empty,
                ));
                ctx.ui.redraw_all = true;
                if self.mode == HudMode::Linedef {
                    server_ctx.selected_wall_row = Some(i as i32);
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                }
                return true;
            }
        }
        if self.mode != HudMode::Rect {
            for (i, rect) in self.subdiv_rects.iter().enumerate() {
                if rect.contains(Vec2::new(x, y)) {
                    if let Some(subdivision) = Self::GRID_SUBDIVISIONS.get(i) {
                        map.subdivisions = *subdivision as f32;
                    }
                    {
                        let mut rusterix = RUSTERIX.write().unwrap();
                        rusterix.set_dirty();
                        rusterix.set_overlay_dirty();
                    }
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Tool Changed"),
                        TheValue::Empty,
                    ));
                    ctx.ui.redraw_all = true;
                    return true;
                }
            }
        }

        if map.camera == MapCamera::TwoD && y < 20 {
            return true;
        }

        false
    }

    pub fn dragged(
        &mut self,
        _x: i32,
        _y: i32,
        _map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        /*
        if self.timeline_rect.contains(Vec2::new(x, y)) {
            let offset = x - self.timeline_rect.x;
            let progress = offset as f32 / self.timeline_rect.width as f32;
            map.animation.transition_progress = progress;
            return true;
        }*/

        false
    }

    pub fn hovered(
        &mut self,
        x: i32,
        y: i32,
        _map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        self.mouse_pos = Vec2::new(x, y);

        /*
        if self.rect_geo_rect.contains(self.mouse_pos) {
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                "Show or hide geometry created with the Rect tool.".to_string(),
            ));
        } else {
            ctx.ui
                .send(TheEvent::SetStatusText(TheId::empty(), "".into()));
        }*/
        false
    }

    #[allow(clippy::collapsible_if)]
    pub fn get_icon_text(
        &self,
        index: i32,
        map: &Map,
        server_ctx: &mut ServerContext,
        action_item_slots: Option<&[ActionItemSlot]>,
        action_material_slots: Option<&[ActionMaterialSlot]>,
    ) -> String {
        let mut text: String = "".into();
        if let Some(slots) = action_item_slots
            && let Some(slot) = slots.get(index as usize)
        {
            return slot.label.clone();
        }
        if let Some(slots) = action_material_slots {
            if let Some(slot) = slots.get(index as usize) {
                return slot.label.clone();
            }
        }
        if server_ctx.get_map_context() == MapContext::Region {
            if self.mode == HudMode::Sector || Self::has_selected_geometry_surface(map) {
                if index == 0 {
                    text = "TILE".into();
                }
            }
        } else if server_ctx.get_map_context() == MapContext::Screen {
            if index == 0 {
                text = "NORM".into();
            } else if index == 1 {
                text = "ACTIVE".into();
            }
        }

        text
    }

    #[allow(clippy::collapsible_if)]
    pub fn get_icon(
        &self,
        index: i32,
        map: &Map,
        id: Option<u32>,
        icon_size: usize,
        action_item_slots: Option<&[ActionItemSlot]>,
        action_material_slots: Option<&[ActionMaterialSlot]>,
    ) -> (Option<rusterix::Tile>, bool) {
        if let Some(slots) = action_item_slots
            && slots.get(index as usize).is_some()
        {
            return (None, false);
        }
        if let Some(slots) = action_material_slots
            && let Some(slot) = slots.get(index as usize)
        {
            if let Some(pixelsource) = &slot.source {
                let props = ValueContainer::default();
                if let Some(tile) =
                    pixelsource.to_tile(&RUSTERIX.read().unwrap().assets, icon_size, &props, map)
                {
                    return (Some(tile), false);
                }
            }
            return (None, false);
        }

        if index == 0
            && Self::has_selected_geometry_surface(map)
            && let Some(pixelsource) = Self::selected_geometry_source(map)
        {
            let props = ValueContainer::default();
            if let Some(tile) =
                pixelsource.to_tile(&RUSTERIX.read().unwrap().assets, icon_size, &props, map)
            {
                return (Some(tile), false);
            }
            return (None, false);
        }

        if self.mode == HudMode::Sector {
            let Some(id) = id else {
                return (None, false);
            };
            if let Some(sector) = map.find_sector(id) {
                if index == 0 {
                    let has_light = sector.properties.get("floor_light").is_some();
                    if let Some(pixelsource) = sector.properties.get_default_source() {
                        if let Some(tile) = pixelsource.to_tile(
                            &RUSTERIX.read().unwrap().assets,
                            icon_size,
                            &sector.properties,
                            map,
                        ) {
                            return (Some(tile), has_light);
                        }
                    }
                    return (None, has_light);
                } else if index == 1 {
                    let has_light = sector.properties.get("ceiling_light").is_some();
                    if let Some(Value::Source(pixelsource)) =
                        &sector.properties.get("ceiling_source")
                    {
                        if let Some(tile) = pixelsource.to_tile(
                            &RUSTERIX.read().unwrap().assets,
                            icon_size,
                            &sector.properties,
                            map,
                        ) {
                            return (Some(tile), has_light);
                        }
                    }
                    return (None, has_light);
                }
            }
        }

        (None, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_map() -> (Map, Uuid) {
        let mut map = Map::new();
        let object = rusterix::GeometryObject::box_from_bounds(
            "box",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 2.0, 2.0),
        );
        let object_id = object.id;
        map.geometry_objects.push(object);
        (map, object_id)
    }

    #[test]
    fn coord_precision_matches_grid_subdivision() {
        assert_eq!(Hud::coord_precision(1.0), 3);
        assert_eq!(Hud::coord_precision(8.0), 3);
        assert_eq!(Hud::coord_precision(16.0), 4);
        assert_eq!(Hud::coord_precision(32.0), 5);
    }

    #[test]
    fn geometry_edge_length_sums_selected_edges() {
        let (mut map, object_id) = box_map();
        map.selected_geometry_vertices = vec![(object_id, 0), (object_id, 1)];

        assert_eq!(Hud::geometry_edge_length(&map), Some(2.0));
    }

    #[test]
    fn selected_geometry_source_reads_selected_face_tile() {
        let (mut map, object_id) = box_map();
        let source = PixelSource::PaletteIndex(3);
        map.geometry_objects[0].faces[0].tile = Some(source.clone());
        map.selected_geometry_faces.push((object_id, 0));

        assert_eq!(Hud::selected_geometry_source(&map), Some(source));
    }

    #[test]
    fn selected_geometry_source_reads_selected_object_tile() {
        let (mut map, object_id) = box_map();
        let source = PixelSource::PaletteIndex(7);
        map.geometry_objects[0].faces[0].tile = Some(source.clone());
        map.selected_geometry_objects.push(object_id);

        assert_eq!(Hud::selected_geometry_source(&map), Some(source));
    }

    #[test]
    fn geometry_length_readout_uses_active_3d_surface_line() {
        let mut map = Map::new();
        map.curr_grid_pos_3d = Some(Vec3::new(0.0, 0.0, 0.0));
        let mut server_ctx = ServerContext::new();
        server_ctx.editor_view_mode = EditorViewMode::Orbit;
        server_ctx.curr_map_tool_type = MapToolType::Linedef;
        server_ctx.hover_cursor_3d = Some(Vec3::new(3.0, 4.0, 0.0));

        assert_eq!(Hud::geometry_length_readout(&map, &server_ctx), Some(5.0));
    }

    #[test]
    fn geometry_length_readout_is_hidden_in_2d_mode() {
        let (mut map, object_id) = box_map();
        map.selected_geometry_vertices = vec![(object_id, 0), (object_id, 1)];
        let mut server_ctx = ServerContext::new();
        server_ctx.editor_view_mode = EditorViewMode::D2;

        assert_eq!(Hud::geometry_length_readout(&map, &server_ctx), None);
    }
}
