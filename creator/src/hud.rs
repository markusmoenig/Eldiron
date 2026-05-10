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
    Dungeon,
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
            && action.is_applicable(map, ctx, server_ctx)
            && let Some(slots) = action.hud_material_slots(map, server_ctx)
        {
            return Some(slots);
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
        if v.abs() < 0.0005 { 0.0 } else { v }
    }

    fn grid_step_label(subdivision: usize) -> String {
        if subdivision <= 1 {
            "1".to_string()
        } else {
            format!("1/{subdivision}")
        }
    }

    fn grid_key_label(subdivision: usize) -> String {
        if subdivision == 10 {
            "0".to_string()
        } else {
            subdivision.to_string()
        }
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
        } else if let Some(snapped) = server_ctx
            .hover_cursor_3d
            .map(|v| {
                let mut snapped = server_ctx.snap_world_point_for_edit(map, v);
                snapped.y = v.y;
                snapped
            })
            .or_else(|| Self::selected_geometry_coord(map))
        {
            ctx.draw
                .rect(buffer.pixels_mut(), &(8, 2, 175, 16), stride, &panel_color);
            ctx.draw.text(
                buffer.pixels_mut(),
                &(10, 2),
                stride,
                &format!(
                    "{:.2}, {:.2}, {:.2}",
                    Self::clean_coord(snapped.x),
                    Self::clean_coord(snapped.y),
                    Self::clean_coord(snapped.z)
                ),
                TheFontSettings {
                    size: 13.0,
                    ..Default::default()
                },
                &text_color,
                &bg_color,
            );
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

        let icon_size = 40;
        let mut icons = 0;
        let action_item_slots = self.active_builder_item_slots(map, ctx, server_ctx);
        let action_material_slots = if action_item_slots.is_none() {
            self.active_palette_material_slots(map, ctx, server_ctx)
                .or_else(|| self.active_action_material_slots(map, ctx, server_ctx))
        } else {
            None
        };

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
            || self.mode == HudMode::Dungeon
            || self.mode == HudMode::Rect
            || self.mode == HudMode::Terrain
        {
            icons = 0;
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

            if let Some(id) = id {
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
            }

            if i == self.selected_icon_index {
                ctx.draw.rect_outline(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &sel_text_color,
                );
            }

            self.icon_rects.push(rect);
        }

        // Show Subdivs
        if (map.camera == MapCamera::TwoD
            || server_ctx.get_map_context() == MapContext::Screen
            || server_ctx.editor_view_mode != EditorViewMode::D2)
            && self.mode != HudMode::Terrain
            && self.mode != HudMode::Dungeon
            && self.mode != HudMode::Rect
        {
            let x = 170;
            let size = 20;
            for i in 0..10 {
                let rect = TheDim::rect(x + (i * size), 0, size, size);
                let active = (i + 1) as f32 == map.subdivisions;
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
                        rect.height as usize - 4,
                    ),
                    stride,
                    &inner,
                );
                if active {
                    ctx.draw.rect(
                        buffer.pixels_mut(),
                        &(
                            rect.x as usize + 3,
                            rect.y as usize + 16,
                            rect.width as usize - 6,
                            2,
                        ),
                        stride,
                        &accent_color,
                    );
                }

                let r = rect.to_buffer_utuple();
                let subdivision = (i + 1) as usize;
                let label = Self::grid_key_label(subdivision);
                ctx.draw.text_rect(
                    buffer.pixels_mut(),
                    &(r.0, 1, r.2, 19),
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
                let rect = TheDim::rect(x + (10 * size) + 6, 2, 52, 16);
                ctx.draw
                    .rect(buffer.pixels_mut(), &rect.to_buffer_utuple(), stride, &panel_color);
                ctx.draw.text_rect(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &Self::grid_step_label(map.subdivisions.round().clamp(1.0, 10.0) as usize),
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
                    map.subdivisions = (i + 1) as f32;
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
            if self.mode == HudMode::Sector {
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
        id: u32,
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

        if self.mode == HudMode::Sector {
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
