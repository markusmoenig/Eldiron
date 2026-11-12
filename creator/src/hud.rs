use crate::editor::{ACTIONLIST, RUSTERIX};
use crate::editor_overlay::procedural::{ColorNoise, ProceduralIcon};
use crate::editor_overlay::{EditorOverlay, OverlaySources};
use crate::prelude::*;
use rusterix::prelude::*;
use theframework::prelude::*;
mod frame;
pub(crate) use frame::{HudAction, HudEffects, HudFrame, HudHit, HudScope};

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct HudColors {
    pub(crate) background: RGBA,
    pub(crate) panel: RGBA,
    pub(crate) text: RGBA,
    pub(crate) selected_text: RGBA,
    pub(crate) accent: RGBA,
    pub(crate) border: RGBA,
    pub(crate) hover: RGBA,
    pub(crate) selected: RGBA,
}

static HUD_COLORS: LazyLock<RwLock<HudColors>> = LazyLock::new(|| {
    RwLock::new(HudColors {
        background: [48, 48, 50, 235],
        panel: [34, 34, 36, 240],
        text: [168, 168, 172, 255],
        selected_text: [240, 240, 242, 255],
        accent: [210, 176, 92, 255],
        border: [76, 78, 84, 255],
        hover: [54, 54, 58, 255],
        selected: [68, 68, 72, 255],
    })
});

pub(crate) fn set_theme(theme: &dyn TheTheme) {
    let mut colors = HUD_COLORS.write().unwrap();
    *colors = HudColors {
        background: *theme.color(DefaultWidgetBackground),
        panel: *theme.color(DefaultWidgetDarkBackground),
        text: *theme.color(MaterialMuted),
        selected_text: *theme.color(SectionbarNormalTextColor),
        accent: *theme.color(SelectedTextEditBorder1),
        border: *theme.color(DefaultWidgetBorder),
        hover: *theme.color(ToolbarButtonHover),
        selected: *theme.color(DefaultSelection),
    };
    colors.background[3] = 235;
    colors.panel[3] = 240;
}

pub(crate) fn colors() -> HudColors {
    *HUD_COLORS.read().unwrap()
}

#[derive(Clone, PartialEq)]
pub(crate) struct HudStyle {
    pub(crate) colors: HudColors,
    pub(crate) geometry_labels: [String; 2],
}
impl HudStyle {
    pub(crate) fn current() -> Self {
        Self {
            colors: colors(),
            geometry_labels: [fl!("hud_geometry_op_move"), fl!("hud_geometry_op_size")],
        }
    }
}

/// Borrowed CPU icon source; never a destination or GPU resource.
#[derive(Clone, Copy)]
pub(crate) struct HudImageSource<'a> {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) pixels: &'a [u8],
}
/// Resolved authoring inputs: no action callbacks, UI context or renderer borrow.
#[derive(Clone, Copy)]
pub(crate) struct HudInputs<'a> {
    pub(crate) map: &'a Map,
    pub(crate) server: &'a ServerContext,
    pub(crate) assets: &'a Assets,
    pub(crate) selected_id: Option<u32>,
    pub(crate) material_slots: Option<&'a [ActionMaterialSlot]>,
    pub(crate) light: Option<HudImageSource<'a>>,
}
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum HudMode {
    Selection,
    Prefab,
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
    native_input: bool,

    icon_rects: Vec<TheDim>,

    pub selected_icon_index: i32,

    subdiv_rects: Vec<TheDim>,

    mouse_pos: Vec2<i32>,

    sources: OverlaySources,
    frame_owner: Uuid,
    serial: u64,
    last_presented: u64,
    invalidated_after: u64,
    prepared_source: Option<std::sync::Arc<frame::Source>>,
    displayed_source: Option<std::sync::Arc<frame::Source>>,
    #[cfg(feature = "native-ui")]
    receipt_owner: u64,

    edit_mode_rects: Vec<TheDim>,
}

impl Hud {
    const GRID_SUBDIVISIONS: [usize; 6] = [1, 2, 4, 8, 16, 32];
    const SHORTCUT_GUIDANCE_HEIGHT: usize = 24;

    fn shortcut_guidance_rect(
        width: usize,
        height: usize,
        help: &str,
    ) -> Option<(usize, usize, usize, usize)> {
        if help.trim().is_empty() || width < 20 || height < Self::SHORTCUT_GUIDANCE_HEIGHT {
            return None;
        }

        Some((
            0,
            height - Self::SHORTCUT_GUIDANCE_HEIGHT,
            width,
            Self::SHORTCUT_GUIDANCE_HEIGHT,
        ))
    }

    pub(crate) fn draw_shortcut_guidance(
        buffer: &mut TheRGBABuffer,
        map: &Map,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) {
        let help = crate::mapeditor::geometry_selection_status_text(map, server_ctx);
        let size = (
            buffer.dim().width.max(0) as usize,
            buffer.dim().height.max(0) as usize,
        );
        Self::prepare_guidance(size, help.as_deref(), colors()).draw_reference(buffer, ctx);
    }
    /// Informational footer, sharing the tool overlay command stream and text
    /// service. It has no hit regions or independent renderer ownership.
    pub(crate) fn prepare_guidance(
        size: (usize, usize),
        help: Option<&str>,
        colors: HudColors,
    ) -> EditorOverlay {
        let mut paint = EditorOverlay::new(size);
        let Some(help) = help else {
            return paint;
        };
        let Some(bar) = Self::shortcut_guidance_rect(size.0, size.1, help) else {
            return paint;
        };
        paint.rect(&bar, &colors.panel);
        paint.rect(&(bar.0, bar.1, bar.2, 1), &colors.border);
        paint.text_rect_blend(
            &(bar.0 + 9, bar.1 + 1, bar.2 - 18, bar.3 - 2),
            help,
            TheFontSettings {
                size: 12.5,
                ..Default::default()
            },
            &colors.selected_text,
            TheHorizontalAlign::Left,
            TheVerticalAlign::Center,
        );
        paint
    }

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

    fn active_palette_material_slots(
        &self,
        map: &Map,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<Vec<ActionMaterialSlot>> {
        if !server_ctx.palette_tool_active {
            return None;
        }
        crate::actions::named_geometry_hud_material_slots_for_selection(map)
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
        if matches!(server_ctx.curr_map_tool_type, MapToolType::Linedef)
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
        let frame_owner = Uuid::new_v4();
        Self {
            mode,
            native_input: false,

            icon_rects: vec![],
            selected_icon_index: 0,

            subdiv_rects: vec![],

            mouse_pos: Vec2::zero(),

            sources: OverlaySources::new(frame_owner),
            frame_owner,
            serial: 0,
            last_presented: 0,
            invalidated_after: 0,
            prepared_source: None,
            displayed_source: None,
            #[cfg(feature = "native-ui")]
            receipt_owner: theframework::paint::new_receipt_owner(),

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
        assets: &Assets,
    ) {
        let size = (
            buffer.dim().width.max(0) as usize,
            buffer.dim().height.max(0) as usize,
        );
        let scope = HudScope {
            document: map.id,
            tool: self.frame_owner,
            revision: map.changed as u64,
        };
        let style = HudStyle::current();
        let frame =
            self.prepare_reference_frame(scope, size, map, ctx, server_ctx, id, assets, &style);
        frame.commands.draw_reference(buffer, ctx);
        self.presented_frame(&frame, scope, size, map, server_ctx);
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_reference_frame(
        &mut self,
        scope: HudScope,
        size: (usize, usize),
        map: &Map,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        id: Option<u32>,
        assets: &Assets,
        style: &HudStyle,
    ) -> HudFrame {
        let material_slots = crate::actions::named_geometry_hud_material_slots_for_selection(map)
            .or_else(|| crate::actions::prefab_hud_material_slots_for_selected_instances(map))
            .or_else(|| self.active_action_material_slots(map, ctx, server_ctx))
            .or_else(|| self.active_palette_material_slots(map, ctx, server_ctx));
        let light = ctx.ui.icon("light_small").map(|icon| HudImageSource {
            width: icon.pixel_width(),
            height: icon.pixel_height(),
            pixels: icon.pixels(),
        });
        self.prepare_frame(
            scope,
            size,
            HudInputs {
                map,
                server: server_ctx,
                assets,
                selected_id: id,
                material_slots: material_slots.as_deref(),
                light,
            },
            style,
        )
    }
    pub(crate) fn prepare_frame(
        &mut self,
        scope: HudScope,
        size: (usize, usize),
        input: HudInputs<'_>,
        style: &HudStyle,
    ) -> HudFrame {
        let HudInputs {
            map,
            server: server_ctx,
            assets,
            selected_id: id,
            material_slots: action_material_slots,
            light,
        } = input;
        let colors = style.colors;
        let (width, height) = size;
        let mut paint = EditorOverlay::new(size);
        self.sources.begin();
        if width < 20 || height < 20 {
            self.sources.finish();
            return self.stage(
                paint,
                frame::Source::new(scope, size, map, server_ctx, frame::HitLayout::default()),
            );
        }
        let info_height = 20;
        let bg_color = colors.background;
        let panel_color = colors.panel;
        let text_color = colors.text;
        let sel_text_color = colors.selected_text;
        let accent_color = colors.accent;

        let mut layout = frame::HitLayout {
            header: true,
            ..Default::default()
        };

        let icon_size = 40;
        let mut icons = 0;

        if server_ctx.get_map_context() == MapContext::Region {
            icons = if let Some(slots) = &action_material_slots {
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
            || self.mode == HudMode::Prefab
        {
            icons = 0;
        }

        paint.rect(&(0, 0, width, info_height), &bg_color);
        paint.rect(&(0, info_height - 1, width, 1), &colors.border);

        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            if let Some(v) = server_ctx.hover_cursor {
                paint.rect(&(8, 2, 150, 16), &panel_color);
                paint.text(
                    &(10, 2),
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
                paint.rect(&(8, 2, panel_width, 16), &panel_color);
                paint.text(
                    &(10, 2),
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
                paint.rect(&(x, 2, panel_width, 16), &panel_color);
                paint.text(
                    &(x + 2, 2),
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
            paint.text(
                &(550, 2),
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

        let selected_icon_index = if icons > 0 {
            server_ctx.selected_hud_icon_index.clamp(0, icons - 1)
        } else {
            0
        };
        layout.selected = selected_icon_index;
        layout.selected_server = (icons > 0).then_some(selected_icon_index);

        layout.icons.clear();
        let x = width as i32 - (icon_size * icons) - 1;
        for i in 0..icons {
            let rect = TheDim::rect(x + (i * icon_size), 20, icon_size, icon_size);
            paint.rect(&rect.to_buffer_utuple(), &bg_color);

            let r = rect.to_buffer_utuple();
            paint.text_rect(
                &(r.0, 1, r.2, 19),
                &self.get_icon_text(i, map, server_ctx, action_material_slots.as_deref()),
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
            paint.rect(&(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2), &panel_color);

            let (source, has_light) =
                self.icon_source(i, map, id, action_material_slots.as_deref());
            if let Some((source, noise)) = source {
                if let Some(description) = ProceduralIcon::new(&source, noise, icon_size as u32) {
                    paint.procedural(description, rect.to_buffer_utuple());
                } else if let Some(image) =
                    self.sources
                        .pixel_source(&format!("icon:{i}"), &source, assets)
                {
                    paint.image(image, rect.to_buffer_utuple());
                }
            }
            if has_light
                && let Some(light) = &light
                && let Some(image) =
                    self.sources
                        .image("light", light.width, light.height, light.pixels)
            {
                paint.image(
                    image,
                    (
                        rect.x as usize + 1,
                        rect.y as usize + 1,
                        light.width,
                        light.height,
                    ),
                );
            }

            if i == selected_icon_index {
                paint.rect_outline(&rect.to_buffer_utuple(), &sel_text_color);
            }

            layout
                .icons
                .push(TheDim::rect(rect.x, 0, rect.width, rect.y + rect.height));
        }

        // Show Subdivs
        if (matches!(map.camera, MapCamera::TwoD | MapCamera::TwoDIso)
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
                    colors.selected
                } else if hovered {
                    colors.hover
                } else {
                    panel_color
                };

                paint.rect(&rect.to_buffer_utuple(), &bg_color);
                paint.rect(
                    &(
                        rect.x as usize + 1,
                        rect.y as usize + 2,
                        rect.width as usize - 2,
                        rect.height as usize - 3,
                    ),
                    &inner,
                );
                if active {
                    paint.rect(
                        &(
                            rect.x as usize + 3,
                            rect.y as usize + rect.height as usize - 4,
                            rect.width as usize - 6,
                            2,
                        ),
                        &accent_color,
                    );
                }

                let r = rect.to_buffer_utuple();
                let label = Self::grid_key_label(i);
                paint.text_rect(
                    &(r.0, 1, r.2, rect.height as usize - 1),
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
                layout.subdivisions.push(rect);
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
                paint.rect(&rect.to_buffer_utuple(), &panel_color);
                paint.text_rect(
                    &rect.to_buffer_utuple(),
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

        layout.edit_modes.clear();
        if self.mode == HudMode::Selection
            && server_ctx.editor_view_mode != EditorViewMode::D2
            && server_ctx.get_map_context() == MapContext::Region
        {
            let labels = &style.geometry_labels;
            let modes = [GeometryGizmoOp::Move, GeometryGizmoOp::Resize];
            let start_x = 432;
            let button_w = 52;

            for i in 0..2 {
                let rect = TheDim::rect(start_x + (i as i32 * (button_w + 4)), 0, button_w, 20);
                let active = server_ctx.geometry_gizmo_op == modes[i];
                let fg = if active { sel_text_color } else { text_color };
                let inner = if active { colors.selected } else { panel_color };

                paint.rect(&rect.to_buffer_utuple(), &bg_color);
                paint.rect(
                    &(
                        rect.x as usize + 1,
                        rect.y as usize + 2,
                        rect.width as usize - 2,
                        rect.height as usize - 4,
                    ),
                    &inner,
                );
                paint.text_rect(
                    &rect.to_buffer_utuple(),
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
                    paint.rect_outline(&rect.to_buffer_utuple(), &sel_text_color);
                    paint.rect(
                        &(
                            rect.x as usize + 4,
                            rect.y as usize + 16,
                            rect.width as usize - 8,
                            2,
                        ),
                        &accent_color,
                    );
                }
                layout.edit_modes.push(rect);
            }
        }

        // Terrain: Height
        if self.mode == HudMode::Terrain {
            if let Some(v) = server_ctx.hover_height {
                paint.text(
                    &(150, 2),
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
                if let Some(image) =
                    self.sources
                        .image("shape", texture.width, texture.height, &texture.data)
                {
                    paint.image(image, preview_rect.to_buffer_utuple());
                }
            }
        }

        self.sources.finish();
        self.stage(
            paint,
            frame::Source::new(scope, size, map, server_ctx, layout),
        )
    }

    pub(crate) fn uses_native_input(&self) -> bool {
        self.native_input
    }
    pub(crate) fn set_native_input(&mut self, native: bool) {
        if self.native_input != native {
            self.invalidate_frame();
            self.native_input = native;
        }
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
        if self.native_input {
            return false;
        }
        if !self.source_input_current(map, server_ctx) {
            // Compatibility input consumes stale painted controls without
            // applying them to a new selection/document or leaking the click
            // into world authoring. Native owners use explicit HudHit states.
            let consumed = self.source_pointer_action(Vec2::new(x, y), map).is_some()
                || (map.camera == MapCamera::TwoD && y >= 0 && y < 20);
            self.invalidate_frame();
            return consumed;
        }
        if server_ctx.get_map_context() != MapContext::Region
            && server_ctx.get_map_context() != MapContext::Screen
            && server_ctx.get_map_context() != MapContext::Character
            && server_ctx.get_map_context() != MapContext::Item
        {
            return false;
        }

        let Some(action) = self.source_pointer_action(Vec2::new(x, y), map) else {
            return false;
        };
        self.apply_action(action, map, server_ctx)
            .apply_reference(ctx);
        true
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

    pub(crate) fn update_pointer_position(&mut self, position: Vec2<i32>) -> bool {
        let changed = self.mouse_pos != position;
        self.mouse_pos = position;
        changed
    }
    pub(crate) fn shortcut_action(key: char) -> Option<HudAction> {
        matches!(key, '1'..='6').then(|| {
            HudAction::Subdivision(Self::GRID_SUBDIVISIONS[(key as u8 - b'1') as usize])
        })
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

    /// Routes HUD input before a specialized geometry tool delegates the remaining event to the
    /// shared object-editing engine. The visible HUD belongs to the specialized tool, so its hit
    /// rectangles must be consulted before delegation.
    pub fn handle_map_event_before_delegate(
        &mut self,
        event: &MapEvent,
        map: &mut Map,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if self.native_input {
            return false;
        }
        match *event {
            MapEvent::MapClicked(coord) => self.clicked(coord.x, coord.y, map, ui, ctx, server_ctx),
            MapEvent::MapDragged(coord) => self.dragged(coord.x, coord.y, map, ui, ctx, server_ctx),
            MapEvent::MapHover(coord) => {
                self.hovered(coord.x, coord.y, map, ui, ctx, server_ctx);
                false
            }
            MapEvent::MapKey(key) if matches!(key, '1'..='6') => {
                let subdivision = Self::GRID_SUBDIVISIONS[(key as u8 - b'1') as usize];
                self.apply_action(HudAction::Subdivision(subdivision), map, server_ctx)
                    .apply_reference(ctx);
                true
            }
            _ => false,
        }
    }

    #[allow(clippy::collapsible_if)]
    pub fn get_icon_text(
        &self,
        index: i32,
        map: &Map,
        server_ctx: &ServerContext,
        action_material_slots: Option<&[ActionMaterialSlot]>,
    ) -> String {
        let mut text: String = "".into();

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

    /// Source selection only. Native preparation never calls to_tile or clones
    /// an asset's texture/material arrays merely to display its first frame.
    fn icon_source(
        &self,
        index: i32,
        map: &Map,
        id: Option<u32>,
        slots: Option<&[ActionMaterialSlot]>,
    ) -> (Option<(PixelSource, ColorNoise)>, bool) {
        if let Some(slot) = slots.and_then(|slots| slots.get(index as usize)) {
            return (
                slot.source
                    .clone()
                    .map(|source| (source, ColorNoise::default())),
                false,
            );
        }
        if index == 0
            && Self::has_selected_geometry_surface(map)
            && let Some(source) = Self::selected_geometry_source(map)
        {
            return (Some((source, ColorNoise::default())), false);
        }
        if self.mode == HudMode::Sector
            && let Some(sector) = id.and_then(|id| map.find_sector(id))
        {
            let (source, light) = match index {
                0 => (
                    sector.properties.get_default_source().cloned(),
                    sector.properties.get("floor_light").is_some(),
                ),
                1 => (
                    match sector.properties.get("ceiling_source") {
                        Some(Value::Source(source)) => Some(source.clone()),
                        _ => None,
                    },
                    sector.properties.get("ceiling_light").is_some(),
                ),
                _ => (None, false),
            };
            return (
                source.map(|source| (source, ColorNoise::from_values(&sector.properties))),
                light,
            );
        }
        (None, false)
    }
    /// Explicit legacy pixel consumer, retained for reference/export validation.
    pub fn get_icon_reference(
        &self,
        index: i32,
        map: &Map,
        id: Option<u32>,
        icon_size: usize,
        slots: Option<&[ActionMaterialSlot]>,
        assets: &Assets,
    ) -> (Option<rusterix::Tile>, bool) {
        let (source, light) = self.icon_source(index, map, id, slots);
        (
            source.and_then(|(source, noise)| {
                source.to_tile(assets, icon_size, &noise.values(), map)
            }),
            light,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_icons_resolve_only_the_supplied_asset_owner_and_preserve_source_maps() {
        let id = Uuid::new_v4();
        let hud = Hud::new(HudMode::Sector);
        let slots = [ActionMaterialSlot {
            label: "Surface".into(),
            source: Some(PixelSource::TileId(id)),
        }];
        let (mut geometry, object) = box_map();
        geometry.geometry_objects[0].faces[0].tile = Some(PixelSource::TileId(id));
        geometry.selected_geometry_faces.push((object, 0));
        let mut sectors = Map::default();
        let mut sector = rusterix::Sector::new(42, vec![]);
        sector
            .properties
            .set("source", Value::Source(PixelSource::TileId(id)));
        sector
            .properties
            .set("ceiling_source", Value::Source(PixelSource::TileId(id)));
        sector.properties.set("floor_light", Value::Bool(true));
        sector.properties.set("ceiling_light", Value::Bool(true));
        sectors.sectors.push(sector);
        let authored_geometry = serde_json::to_vec(&geometry).unwrap();
        let authored_sectors = serde_json::to_vec(&sectors).unwrap();
        let colors = [[210, 30, 50, 255], [30, 180, 210, 90]];
        // Two documents may bind the same asset ID to different editable pixels.
        // Resolving a HUD source must not consult the global renderer's assets.
        for color in colors {
            let mut assets = Assets::default();
            let mut tile = rusterix::Tile::from_texture(rusterix::Texture::from_color(color));
            tile.id = id;
            assets.tiles.insert(id, tile);
            for (map, index, selected, slots, light) in [
                (&sectors, 0, None, Some(slots.as_slice()), false),
                (&geometry, 0, None, None, false),
                (&sectors, 0, Some(42), None, true),
                (&sectors, 1, Some(42), None, true),
            ] {
                let (tile, has_light) =
                    hud.get_icon_reference(index, map, selected, 40, slots, &assets);
                let tile = tile.expect("the supplied owner contains this tile");
                assert_eq!(tile.textures[0].data.as_slice(), &color);
                assert_eq!(has_light, light);
                let (missing, has_light) =
                    hud.get_icon_reference(index, map, selected, 40, slots, &Assets::default());
                assert!(
                    missing.is_none(),
                    "a missing source cannot fall through to another owner"
                );
                assert_eq!(has_light, light);
            }
        }
        assert_eq!(serde_json::to_vec(&geometry).unwrap(), authored_geometry);
        assert_eq!(serde_json::to_vec(&sectors).unwrap(), authored_sectors);
    }

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
    #[ignore = "native HUD drawing and pointer dispatch"]
    fn iso_geometry_tools_show_and_apply_the_shared_subdivision_toolbar() {
        let mut map = Map::default();
        map.camera = MapCamera::TwoDIso;
        let mut server = ServerContext::default();
        server.editor_view_mode = EditorViewMode::D2;
        let mut ctx = TheContext::new(900, 500, 1.);
        let mut ui = TheUI::new();
        let mut buffer = TheRGBABuffer::new(TheDim::sized(900, 500));
        for mode in [HudMode::Vertex, HudMode::Linedef, HudMode::Sector] {
            let mut hud = Hud::new(mode);
            hud.draw(
                &mut buffer,
                &mut map,
                &mut ctx,
                &mut server,
                None,
                &rusterix::Assets::default(),
            );
            assert_eq!(hud.subdiv_rects.len(), Hud::GRID_SUBDIVISIONS.len());
            let at = Vec2::new(hud.subdiv_rects[2].x + 5, 5);
            assert!(hud.handle_map_event_before_delegate(
                &MapEvent::MapClicked(at),
                &mut map,
                &mut ui,
                &mut ctx,
                &mut server
            ));
            assert_eq!(map.subdivisions, 4.);
        }
    }

    #[test]
    fn coord_precision_matches_grid_subdivision() {
        assert_eq!(Hud::coord_precision(1.0), 3);
        assert_eq!(Hud::coord_precision(8.0), 3);
        assert_eq!(Hud::coord_precision(16.0), 4);
        assert_eq!(Hud::coord_precision(32.0), 5);
    }

    #[test]
    fn shortcut_guidance_fills_and_touches_viewport_bottom() {
        let rect = Hud::shortcut_guidance_rect(640, 480, "Shortcuts").unwrap();
        assert_eq!(rect, (0, 456, 640, 24));
        assert_eq!(rect.0 + rect.2, 640);
        assert_eq!(rect.1 + rect.3, 480);
    }

    #[test]
    fn shortcut_guidance_is_hidden_without_content() {
        assert_eq!(Hud::shortcut_guidance_rect(640, 480, ""), None);
        assert_eq!(Hud::shortcut_guidance_rect(640, 480, "   "), None);
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

#[cfg(test)]
#[path = "hud/frame_tests.rs"]
mod frame_tests;
