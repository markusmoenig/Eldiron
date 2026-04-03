use crate::editor::{RUSTERIX, TOOLLIST, UNDOMANAGER};
use crate::prelude::*;
use rusterix::Surface;
use scenevm::GeoId;
use std::time::{SystemTime, UNIX_EPOCH};

const BUILDER_TAB_LAYOUT: &str = "Builder Dock Tabs";
const BUILDER_VIEW_PREFIX: &str = "Builder Dock View ";
const BUILDER_DOCK_REFRESH: &str = "Builder Dock Refresh";
const BUILDER_CARD_W: i32 = 164;
const BUILDER_CARD_H: i32 = 202;
const BUILDER_CARD_GAP: i32 = 12;
const BUILDER_PADDING: i32 = 12;

#[derive(Clone, Copy, PartialEq, Eq)]
enum BuilderTabKind {
    Project,
    Collections,
    Treasury,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BuilderCardKind {
    Asset(Uuid),
    TreasuryPlaceholder,
}

struct BuilderCardSpec {
    kind: BuilderCardKind,
    preview: Option<TheRGBABuffer>,
    label: String,
    description: String,
}

#[derive(Clone, Copy)]
struct BuilderCardPlacement {
    kind: BuilderCardKind,
    rect: Vec4<i32>,
}

pub struct BuilderDock {
    active_tab: usize,
    selected: Option<Uuid>,
    hovered: Option<Uuid>,
    placements: Vec<Vec<BuilderCardPlacement>>,
    last_asset_click: Option<(Uuid, u128)>,
}

impl Dock for BuilderDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            active_tab: 0,
            selected: None,
            hovered: None,
            placements: vec![Vec::new(), Vec::new(), Vec::new()],
            last_asset_click: None,
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        toolbar_hlayout.set_padding(3);

        let mut title = TheText::new(TheId::named("Builder Dock Title"));
        title.set_text(fl!("builder_picker_title"));
        title.set_text_size(12.5);
        toolbar_hlayout.add_widget(Box::new(title));

        let spacer = TheSpacer::new(TheId::empty());
        toolbar_hlayout.add_widget(Box::new(spacer));

        let mut new_button = TheTraybarButton::new(TheId::named("Builder Dock New"));
        new_button.set_text(fl!("new"));
        new_button.set_status_text(&fl!("status_builder_new"));
        new_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new(
                    "Empty".to_string(),
                    TheId::named("Builder Dock New Empty"),
                ),
                TheContextMenuItem::new(
                    "Table".to_string(),
                    TheId::named("Builder Dock New Table"),
                ),
                TheContextMenuItem::new(
                    "Wall Torch".to_string(),
                    TheId::named("Builder Dock New Wall Torch"),
                ),
                TheContextMenuItem::new(
                    "Wall Lantern".to_string(),
                    TheId::named("Builder Dock New Wall Lantern"),
                ),
                TheContextMenuItem::new(
                    "Campfire".to_string(),
                    TheId::named("Builder Dock New Campfire"),
                ),
            ],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(new_button));

        let mut collections_button =
            TheTraybarButton::new(TheId::named("Builder Dock Collections"));
        collections_button.set_text(fl!("collections"));
        collections_button.set_status_text(&fl!("status_builder_collections"));
        toolbar_hlayout.add_widget(Box::new(collections_button));

        let mut apply_button = TheTraybarButton::new(TheId::named("Builder Dock Apply Build"));
        apply_button.set_text(fl!("builder_apply_build"));
        apply_button.set_status_text(&fl!("status_builder_apply_build"));
        toolbar_hlayout.add_widget(Box::new(apply_button));

        let mut clear_button = TheTraybarButton::new(TheId::named("Builder Dock Clear Build"));
        clear_button.set_text(fl!("clear"));
        clear_button.set_status_text(&fl!("status_builder_clear_build"));
        toolbar_hlayout.add_widget(Box::new(clear_button));

        toolbar_hlayout.set_reverse_index(Some(2));

        toolbar_canvas.set_layout(toolbar_hlayout);
        canvas.set_top(toolbar_canvas);

        let mut tab_layout = TheTabLayout::new(TheId::named(BUILDER_TAB_LAYOUT));
        for tab in 0..3 {
            let mut tab_canvas = TheCanvas::new();
            tab_canvas.set_widget(TheRenderView::new(TheId::named(&format!(
                "{BUILDER_VIEW_PREFIX}{tab}"
            ))));
            let label = match tab {
                0 => "Project",
                1 => "Collections",
                _ => "Treasury",
            };
            tab_layout.add_canvas(label.to_string(), tab_canvas);
        }
        canvas.set_layout(tab_layout);

        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        self.selected = server_ctx.curr_builder_graph_id;
        ctx.ui.relayout = true;
        self.render_views(ui, ctx, project);
        ctx.ui.send(TheEvent::Custom(
            TheId::named(BUILDER_DOCK_REFRESH),
            TheValue::Empty,
        ));
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::Resize => {
                self.render_views(ui, ctx, project);
                redraw = true;
            }
            TheEvent::IndexChanged(id, index)
                if id.name == format!("{BUILDER_TAB_LAYOUT} Tabbar") =>
            {
                self.active_tab = *index;
                self.render_views(ui, ctx, project);
                redraw = true;
            }
            TheEvent::RenderViewClicked(id, coord) => {
                if let Some(tab) = Self::tab_from_view_name(&id.name)
                    && let Some(kind) = self.pick_asset(tab, *coord)
                {
                    match kind {
                        BuilderCardKind::Asset(asset_id) => {
                            self.selected = Some(asset_id);
                            server_ctx.curr_builder_graph_id = Some(asset_id);
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Builder Selection Changed"),
                                TheValue::Id(asset_id),
                            ));
                            let now = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .map(|d| d.as_millis())
                                .unwrap_or(0);
                            let open_editor = matches!(
                                self.last_asset_click,
                                Some((last_id, last_time))
                                    if last_id == asset_id && now.saturating_sub(last_time) < 400
                            );
                            self.last_asset_click = Some((asset_id, now));
                            if open_editor {
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Open Builder Graph Workflow"),
                                    TheValue::Id(asset_id),
                                ));
                            }
                        }
                        BuilderCardKind::TreasuryPlaceholder => {}
                    }
                    self.render_views(ui, ctx, project);
                    redraw = true;
                }
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(key)) => {
                if *key == TheKeyCode::Return && server_ctx.curr_builder_graph_id.is_some() {
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Open Builder Graph Workflow"),
                        TheValue::Id(server_ctx.curr_builder_graph_id.unwrap()),
                    ));
                    redraw = true;
                } else if *key == TheKeyCode::Delete
                    && !ui.focus_widget_supports_text_input(ctx)
                    && let Some(asset_id) = self.selected
                {
                    let before = project.clone();
                    if project.builder_graphs.shift_remove(&asset_id).is_some() {
                        if server_ctx.curr_builder_graph_id == Some(asset_id) {
                            server_ctx.curr_builder_graph_id = None;
                        }
                        self.selected = None;
                        self.hovered = None;
                        self.last_asset_click = None;
                        let after = project.clone();
                        UNDOMANAGER.write().unwrap().add_undo(
                            ProjectUndoAtom::TilePickerEdit(Box::new(before), Box::new(after)),
                            ctx,
                        );
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Builder Selection Changed"),
                            TheValue::Empty,
                        ));
                        self.render_views(ui, ctx, project);
                        redraw = true;
                    }
                }
            }
            TheEvent::RenderViewHoverChanged(id, coord) => {
                if let Some(tab) = Self::tab_from_view_name(&id.name) {
                    self.hovered = match self.pick_asset(tab, *coord) {
                        Some(BuilderCardKind::Asset(asset_id)) => Some(asset_id),
                        _ => None,
                    };
                    if let Some(asset_id) = self.hovered
                        && let Some(asset) = project.builder_graphs.get(&asset_id)
                    {
                        ctx.ui.send(TheEvent::SetStatusText(
                            id.clone(),
                            format!(
                                "{}",
                                fl!(
                                    "status_builder_select_asset",
                                    asset_name = asset.graph_name.clone()
                                )
                            ),
                        ));
                    }
                    self.render_views(ui, ctx, project);
                    redraw = true;
                }
            }
            TheEvent::RenderViewLostHover(id) => {
                if Self::tab_from_view_name(&id.name).is_some() {
                    self.hovered = None;
                    self.render_views(ui, ctx, project);
                    redraw = true;
                }
            }
            TheEvent::ContextMenuSelected(id, item) if id.name == "Builder Dock New" => {
                let asset = match item.name.as_str() {
                    "Builder Dock New Empty" => {
                        BuilderGraphAsset::new_empty(Self::next_builder_name(project, "Empty"))
                    }
                    "Builder Dock New Table" => {
                        BuilderGraphAsset::new_table(Self::next_builder_name(project, "Table"))
                    }
                    "Builder Dock New Wall Torch" => BuilderGraphAsset::new_wall_torch(
                        Self::next_builder_name(project, "Wall Torch"),
                    ),
                    "Builder Dock New Wall Lantern" => BuilderGraphAsset::new_wall_lantern(
                        Self::next_builder_name(project, "Wall Lantern"),
                    ),
                    "Builder Dock New Campfire" => BuilderGraphAsset::new_campfire(
                        Self::next_builder_name(project, "Campfire"),
                    ),
                    _ => return false,
                };
                let asset_id = asset.id;
                project.add_builder_graph(asset);
                self.selected = Some(asset_id);
                server_ctx.curr_builder_graph_id = Some(asset_id);
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Builder Selection Changed"),
                    TheValue::Id(asset_id),
                ));
                self.render_views(ui, ctx, project);
                redraw = true;
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "Builder Dock Apply Build" =>
            {
                if let Some(asset_id) = self.selected.or(server_ctx.curr_builder_graph_id) {
                    let mut applied_to_item_slot = false;
                    if let Some(asset) = project.builder_graphs.get(&asset_id).cloned()
                        && let Some(map) = project.get_map_mut(server_ctx)
                    {
                        applied_to_item_slot = crate::actions::apply_builder_item_to_selection(
                            map,
                            server_ctx,
                            server_ctx.selected_hud_icon_index,
                            &asset,
                        );
                    }
                    if !applied_to_item_slot {
                        self.activate_asset(asset_id, ui, ctx, project, server_ctx);
                    }
                    RUSTERIX.write().unwrap().set_dirty();
                    crate::utils::scenemanager_render_map(project, server_ctx);
                    TOOLLIST
                        .write()
                        .unwrap()
                        .update_geometry_overlay_3d(project, server_ctx);
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Builder Selection Changed"),
                        TheValue::Id(asset_id),
                    ));
                    self.render_views(ui, ctx, project);
                    redraw = true;
                }
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "Builder Dock Clear Build" =>
            {
                let mut cleared_item_slot = false;
                if let Some(map) = project.get_map_mut(server_ctx) {
                    cleared_item_slot = crate::actions::clear_builder_item_from_selection(
                        map,
                        server_ctx,
                        server_ctx.selected_hud_icon_index,
                    );
                }
                if !cleared_item_slot {
                    self.clear_selected_hosts(project, server_ctx);
                }
                RUSTERIX.write().unwrap().set_dirty();
                crate::utils::scenemanager_render_map(project, server_ctx);
                TOOLLIST
                    .write()
                    .unwrap()
                    .update_geometry_overlay_3d(project, server_ctx);
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Builder Selection Changed"),
                    TheValue::Empty,
                ));
                self.render_views(ui, ctx, project);
                redraw = true;
            }
            TheEvent::Custom(id, value)
                if id.name == "Builder Graph Updated" || id.name == "Builder Selection Changed" =>
            {
                if let TheValue::Id(builder_id) = value {
                    self.selected = Some(*builder_id);
                    server_ctx.curr_builder_graph_id = Some(*builder_id);
                } else {
                    self.selected = server_ctx.curr_builder_graph_id;
                }
                self.render_views(ui, ctx, project);
                redraw = true;
            }
            TheEvent::Custom(id, _) if id.name == BUILDER_DOCK_REFRESH => {
                ctx.ui.relayout = true;
                self.render_views(ui, ctx, project);
                redraw = true;
            }
            _ => {}
        }

        redraw
    }

    fn supports_actions(&self) -> bool {
        false
    }

    fn maximized_state(&self) -> DockMaximizedState {
        DockMaximizedState::Editor
    }
}

impl BuilderDock {
    fn linedef_builder_host_sector_id(map: &Map, linedef_id: u32) -> Option<u32> {
        let linedef = map.find_linedef(linedef_id)?;
        let host_sector = linedef
            .properties
            .get_int("host_sector")
            .map(|id| id as u32);
        host_sector
            .filter(|sector_id| map.find_sector(*sector_id).is_some())
            .or_else(|| linedef.sector_ids.first().copied())
    }

    fn linedef_builder_stored_outward(map: &Map, linedef_id: u32) -> Option<Vec3<f32>> {
        let linedef = map.find_linedef(linedef_id)?;
        let outward = Vec3::new(
            linedef.properties.get_float("host_outward_x")?,
            linedef.properties.get_float("host_outward_y")?,
            linedef.properties.get_float("host_outward_z")?,
        );
        outward.try_normalized()
    }

    fn linedef_builder_stored_face_origin(map: &Map, linedef_id: u32) -> Option<Vec3<f32>> {
        let linedef = map.find_linedef(linedef_id)?;
        Some(Vec3::new(
            linedef.properties.get_float("host_surface_origin_x")?,
            linedef.properties.get_float("host_surface_origin_y")?,
            linedef.properties.get_float("host_surface_origin_z")?,
        ))
    }

    fn builder_surface_for_sector(map: &Map, sector_id: u32) -> Option<Surface> {
        if let Some(surface) = map.get_surface_for_sector_id(sector_id) {
            return Some(surface.clone());
        }
        let mut surface = Surface::new(sector_id);
        surface.calculate_geometry(map);
        surface.is_valid().then_some(surface)
    }

    fn resolve_builder_surface_side(
        signed_dist: f32,
        surface_normal: Vec3<f32>,
        hover_ray_dir: Option<Vec3<f32>>,
    ) -> bool {
        if signed_dist.abs() > 0.01 {
            signed_dist >= 0.0
        } else if let Some(ray_dir) = hover_ray_dir {
            surface_normal.dot(-ray_dir) >= 0.0
        } else {
            true
        }
    }

    fn linedef_builder_face_origin(
        map: &Map,
        server_ctx: &ServerContext,
        linedef_id: u32,
    ) -> Option<Vec3<f32>> {
        if let Some(face_origin) = Self::linedef_builder_stored_face_origin(map, linedef_id) {
            return Some(face_origin);
        }
        let surface = Self::linedef_builder_surface(map, server_ctx, linedef_id)?;
        let outward = Self::linedef_builder_outward(map, server_ctx, linedef_id)?;
        let normal = surface.plane.normal.try_normalized()?;
        let (front_offset, back_offset) = surface.extrusion.offsets();
        let selected_offset = if outward.dot(normal) >= 0.0 {
            front_offset.max(back_offset)
        } else {
            front_offset.min(back_offset)
        };
        Some(surface.plane.origin + normal * selected_offset)
    }

    fn linedef_builder_surface(
        map: &Map,
        server_ctx: &ServerContext,
        linedef_id: u32,
    ) -> Option<Surface> {
        let linedef = map.find_linedef(linedef_id)?;
        let host_sector_id = Self::linedef_builder_host_sector_id(map, linedef_id);

        let preferred = server_ctx
            .active_detail_surface
            .as_ref()
            .or(server_ctx.hover_surface.as_ref())
            .or(server_ctx.editing_surface.as_ref());
        if let Some(surface) = preferred
            && host_sector_id
                .map(|sector_id| surface.sector_id == sector_id)
                .unwrap_or_else(|| linedef.sector_ids.contains(&surface.sector_id))
            && surface.plane.normal.y.abs() <= 0.25
        {
            return Some(surface.clone());
        }

        if let Some(host_sector_id) = host_sector_id
            && let Some(surface) = Self::builder_surface_for_sector(map, host_sector_id)
            && surface.plane.normal.y.abs() <= 0.25
        {
            return Some(surface);
        }

        let hit_pos = server_ctx
            .editing_surface_hit_pos
            .or(server_ctx.hover_cursor_3d)
            .unwrap_or(server_ctx.geo_hit_pos);
        let ray_dir = server_ctx
            .hover_ray_dir_3d
            .and_then(|dir| dir.try_normalized());
        let mut best_surface: Option<(Surface, f32)> = None;
        for surface in map.surfaces.values() {
            let surface_matches_host = host_sector_id
                .map(|sector_id| surface.sector_id == sector_id)
                .unwrap_or_else(|| linedef.sector_ids.contains(&surface.sector_id));
            if !surface_matches_host || surface.plane.normal.y.abs() > 0.25 {
                continue;
            }
            let Some(normal) = surface.plane.normal.try_normalized() else {
                continue;
            };
            if let Some(ray_dir) = ray_dir
                && normal.dot(ray_dir) >= -1e-4
            {
                continue;
            }
            let dist = (hit_pos - surface.plane.origin).dot(normal).abs();
            if best_surface
                .as_ref()
                .map(|(_, best_dist)| dist < *best_dist)
                .unwrap_or(true)
            {
                best_surface = Some((surface.clone(), dist));
            }
        }
        best_surface.map(|(surface, _)| surface)
    }

    fn linedef_builder_outward(
        map: &Map,
        server_ctx: &ServerContext,
        linedef_id: u32,
    ) -> Option<Vec3<f32>> {
        if let Some(outward) = Self::linedef_builder_stored_outward(map, linedef_id) {
            return Some(outward);
        }
        let hit = server_ctx.hover_cursor_3d.unwrap_or(server_ctx.geo_hit_pos);
        let surface = Self::linedef_builder_surface(map, server_ctx, linedef_id)?;
        let normal = surface.plane.normal.try_normalized()?;
        let signed_dist = (hit - surface.plane.origin).dot(normal);
        let grow_positive = Self::resolve_builder_surface_side(
            signed_dist,
            normal,
            server_ctx
                .hover_ray_dir_3d
                .and_then(|dir| dir.try_normalized()),
        );
        Some(if grow_positive { normal } else { -normal })
    }

    fn linedef_builder_wall_side(map: &Map, server_ctx: &ServerContext, linedef_id: u32) -> f32 {
        if let Some(linedef) = map.find_linedef(linedef_id) {
            let hit_pos = server_ctx
                .editing_surface_hit_pos
                .or(server_ctx.hover_cursor_3d)
                .unwrap_or(server_ctx.geo_hit_pos);
            if matches!(server_ctx.geo_hit, Some(GeoId::Linedef(id)) if id == linedef_id)
                && let Some(dist) = linedef.signed_distance(map, Vec2::new(hit_pos.x, hit_pos.z))
                && dist.abs() > 1e-5
            {
                return if dist >= 0.0 { -1.0 } else { 1.0 };
            }

            let preferred_sector = map
                .selected_sectors
                .iter()
                .copied()
                .find(|sid| linedef.sector_ids.contains(sid))
                .or_else(|| Self::linedef_builder_host_sector_id(map, linedef_id));
            if let Some(sector_id) = preferred_sector
                && let Some(sector) = map.find_sector(sector_id)
                && let Some(center) = sector.center(map)
                && let Some(dist) = linedef.signed_distance(map, center)
                && dist.abs() > 1e-5
            {
                return if dist >= 0.0 { 1.0 } else { -1.0 };
            }
        }
        1.0
    }

    fn render_views(&mut self, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        for tab in 0..3 {
            let Some(render_view) = ui.get_render_view(&format!("{BUILDER_VIEW_PREFIX}{tab}"))
            else {
                continue;
            };
            let dim = *render_view.dim();
            if dim.width <= 0 || dim.height <= 0 {
                continue;
            }

            *render_view.render_buffer_mut() =
                TheRGBABuffer::new(TheDim::new(0, 0, dim.width, dim.height));
            let buffer = render_view.render_buffer_mut();
            buffer.fill(BLACK);
            self.placements[tab] = self.draw_tab(buffer, ctx, project, tab);
            render_view.set_needs_redraw(true);
        }
        ctx.ui.redraw_all = true;
    }

    fn draw_tab(
        &self,
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        project: &Project,
        tab: usize,
    ) -> Vec<BuilderCardPlacement> {
        let stride = buffer.stride();
        let assets = self.assets_for_tab(project, tab);
        let cols = ((buffer.dim().width - BUILDER_PADDING * 2 + BUILDER_CARD_GAP)
            / (BUILDER_CARD_W + BUILDER_CARD_GAP))
            .max(1);

        let mut placements = Vec::new();
        for (index, spec) in assets.iter().enumerate() {
            let col = index as i32 % cols;
            let row = index as i32 / cols;
            let rect = Vec4::new(
                BUILDER_PADDING + col * (BUILDER_CARD_W + BUILDER_CARD_GAP),
                BUILDER_PADDING + row * (BUILDER_CARD_H + BUILDER_CARD_GAP),
                BUILDER_CARD_W,
                BUILDER_CARD_H,
            );
            placements.push(BuilderCardPlacement {
                kind: spec.kind,
                rect,
            });

            let hovered =
                matches!(spec.kind, BuilderCardKind::Asset(id) if self.hovered == Some(id));
            let selected =
                matches!(spec.kind, BuilderCardKind::Asset(id) if self.selected == Some(id));
            let fill = if hovered {
                [84, 84, 84, 255]
            } else {
                [66, 66, 66, 255]
            };
            let outline = if selected {
                WHITE
            } else if hovered {
                [210, 210, 210, 255]
            } else {
                [104, 104, 104, 255]
            };

            if let Some(card) = Self::clip_rect(buffer, rect, 0) {
                ctx.draw.rect(buffer.pixels_mut(), &card, stride, &fill);
                ctx.draw
                    .rect_outline(buffer.pixels_mut(), &card, stride, &outline);
            }

            let preview_size = rect.z - 16;
            let preview_rect = Vec4::new(rect.x + 8, rect.y + 8, preview_size, preview_size);
            if let Some(preview) = Self::clip_rect(buffer, preview_rect, 0) {
                ctx.draw
                    .rect(buffer.pixels_mut(), &preview, stride, &[44, 44, 44, 255]);
                ctx.draw
                    .rect_outline(buffer.pixels_mut(), &preview, stride, &[78, 78, 78, 255]);
                self.draw_preview_shape(buffer, ctx, preview_rect, spec.preview.as_ref());
            }

            let title_rect = (
                (rect.x + 8).max(0) as usize,
                (rect.y + 8 + preview_size + 8).max(0) as usize,
                (rect.z - 16).max(1) as usize,
                18usize,
            );
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &title_rect,
                stride,
                &spec.label,
                TheFontSettings {
                    size: 12.5,
                    ..Default::default()
                },
                &WHITE,
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );

            let body_rect = (
                (rect.x + 8).max(0) as usize,
                (rect.y + 8 + preview_size + 26).max(0) as usize,
                (rect.z - 16).max(1) as usize,
                28usize,
            );
            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &body_rect,
                stride,
                &spec.description,
                TheFontSettings {
                    size: 10.5,
                    ..Default::default()
                },
                &[210, 210, 210, 255],
                TheHorizontalAlign::Left,
                TheVerticalAlign::Top,
            );
        }

        placements
    }

    fn draw_preview_shape(
        &self,
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        rect: Vec4<i32>,
        preview: Option<&TheRGBABuffer>,
    ) {
        if let Some(preview) = preview {
            if let Some(view) = Self::clip_rect(buffer, rect, 0) {
                let stride = buffer.stride();
                let view_w = view.2 as i32;
                let view_h = view.3 as i32;
                let src_w = preview.dim().width.max(1);
                let src_h = preview.dim().height.max(1);
                let fit = ((view_w as f32) / (src_w as f32))
                    .min((view_h as f32) / (src_h as f32))
                    .max(0.01);
                let draw_w = ((src_w as f32) * fit).round().max(1.0) as i32;
                let draw_h = ((src_h as f32) * fit).round().max(1.0) as i32;
                let centered = (
                    (view.0 as i32 + (view_w - draw_w) / 2).max(0) as usize,
                    (view.1 as i32 + (view_h - draw_h) / 2).max(0) as usize,
                    draw_w.max(1) as usize,
                    draw_h.max(1) as usize,
                );
                ctx.draw.scale_chunk(
                    buffer.pixels_mut(),
                    &centered,
                    stride,
                    preview.pixels(),
                    &(preview.dim().width as usize, preview.dim().height as usize),
                    1.0,
                );
            }
        }
    }

    fn clip_rect(
        buffer: &TheRGBABuffer,
        rect: Vec4<i32>,
        inset: i32,
    ) -> Option<(usize, usize, usize, usize)> {
        let x0 = (rect.x + inset).clamp(0, buffer.dim().width);
        let y0 = (rect.y + inset).clamp(0, buffer.dim().height);
        let x1 = (rect.x + rect.z - inset).clamp(0, buffer.dim().width);
        let y1 = (rect.y + rect.w - inset).clamp(0, buffer.dim().height);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        Some((
            x0 as usize,
            y0 as usize,
            (x1 - x0) as usize,
            (y1 - y0) as usize,
        ))
    }

    fn assets_for_tab(&self, project: &Project, tab: usize) -> Vec<BuilderCardSpec> {
        match Self::tab_kind(tab) {
            BuilderTabKind::Project | BuilderTabKind::Collections => {
                let mut out: Vec<BuilderCardSpec> = project
                    .builder_graphs
                    .values()
                    .map(|asset| {
                        let preview = Self::preview_for_asset(asset);
                        let description = Self::description_for_asset(asset);
                        BuilderCardSpec {
                            kind: BuilderCardKind::Asset(asset.id),
                            preview,
                            label: asset.graph_name.clone(),
                            description,
                        }
                    })
                    .collect();
                out.sort_by(|a, b| a.label.cmp(&b.label));
                out
            }
            BuilderTabKind::Treasury => vec![BuilderCardSpec {
                kind: BuilderCardKind::TreasuryPlaceholder,
                preview: None,
                label: "Treasury".to_string(),
                description: "Shared builder assets will appear here once builder packages are wired into Treasury.".to_string(),
            }],
        }
    }

    fn tab_kind(tab: usize) -> BuilderTabKind {
        match tab {
            1 => BuilderTabKind::Collections,
            2 => BuilderTabKind::Treasury,
            _ => BuilderTabKind::Project,
        }
    }

    fn tab_from_view_name(name: &str) -> Option<usize> {
        name.strip_prefix(BUILDER_VIEW_PREFIX)
            .and_then(|suffix| suffix.parse::<usize>().ok())
    }

    fn pick_asset(&self, tab: usize, coord: Vec2<i32>) -> Option<BuilderCardKind> {
        self.placements.get(tab)?.iter().find_map(|placement| {
            let r = placement.rect;
            (coord.x >= r.x && coord.x < r.x + r.z && coord.y >= r.y && coord.y < r.y + r.w)
                .then_some(placement.kind)
        })
    }

    fn preview_for_asset(asset: &BuilderGraphAsset) -> Option<TheRGBABuffer> {
        if let Ok(graph) = shared::buildergraph::BuilderDocument::from_text(&asset.graph_data)
            && let Ok(assembly) = graph.evaluate()
            && let Ok(preview) = rusterix::builderpreview::render_builder_preview(
                &assembly,
                graph.output_spec(),
                &graph.preview_host(),
                rusterix::builderpreview::BuilderPreviewOptions {
                    size: (BUILDER_CARD_W - 16) as u32,
                    scale: Some(1.0),
                    variants: rusterix::builderpreview::PreviewVariants::Single,
                    ..Default::default()
                },
            )
        {
            let mut buffer =
                TheRGBABuffer::new(TheDim::sized(preview.width as i32, preview.height as i32));
            buffer.pixels_mut().copy_from_slice(&preview.pixels);
            Some(buffer)
        } else {
            None
        }
    }

    fn description_for_asset(asset: &BuilderGraphAsset) -> String {
        if let Ok(graph) = shared::buildergraph::BuilderDocument::from_text(&asset.graph_data) {
            let spec = graph.output_spec();
            let target = match spec.target {
                BuilderOutputTarget::Sector => "Sector",
                BuilderOutputTarget::VertexPair => "Vertex",
                BuilderOutputTarget::Linedef => "Linedef",
            };
            if spec.host_refs > 1 {
                format!("{target} x{}", spec.host_refs)
            } else {
                target.to_string()
            }
        } else {
            "Invalid builder graph.".to_string()
        }
    }

    fn next_builder_name(project: &Project, base: &str) -> String {
        let base = base.to_string();
        if !project
            .builder_graphs
            .values()
            .any(|a| a.graph_name == base)
        {
            return base;
        }
        let mut index = 2;
        loop {
            let candidate = format!("{base} {index}");
            if !project
                .builder_graphs
                .values()
                .any(|asset| asset.graph_name == candidate)
            {
                return candidate;
            }
            index += 1;
        }
    }

    fn activate_asset(
        &self,
        asset_id: Uuid,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        let Some(asset) = project.builder_graphs.get(&asset_id) else {
            return;
        };
        let asset_builder_id = asset.id;
        let asset_graph_name = asset.graph_name.clone();
        let asset_graph_data = asset.graph_data.clone();
        let Ok(graph) = shared::buildergraph::BuilderDocument::from_text(&asset_graph_data) else {
            return;
        };
        let spec = graph.output_spec();
        let group_id = Uuid::new_v4();

        server_ctx.curr_map_tool_type = match spec.target {
            BuilderOutputTarget::Sector => MapToolType::Sector,
            BuilderOutputTarget::VertexPair => MapToolType::Vertex,
            BuilderOutputTarget::Linedef => MapToolType::Linedef,
        };

        if let Some(map) = project.get_map_mut(server_ctx) {
            match spec.target {
                BuilderOutputTarget::Sector => {
                    for (group_order, sector_id) in
                        map.selected_sectors.clone().into_iter().enumerate()
                    {
                        if let Some(sector) = map.find_sector_mut(sector_id) {
                            sector
                                .properties
                                .set("builder_graph_id", Value::Id(asset_builder_id));
                            sector
                                .properties
                                .set("builder_graph_name", Value::Str(asset_graph_name.clone()));
                            sector
                                .properties
                                .set("builder_graph_data", Value::Str(asset_graph_data.clone()));
                            sector
                                .properties
                                .set("builder_graph_target", Value::Str("sector".to_string()));
                            sector
                                .properties
                                .set("builder_surface_mode", Value::Str("overlay".to_string()));
                            sector
                                .properties
                                .set("builder_hide_host", Value::Bool(true));
                            sector
                                .properties
                                .set("builder_graph_host_refs", Value::Int(spec.host_refs as i32));
                            sector
                                .properties
                                .set("builder_graph_group_id", Value::Id(group_id));
                            sector
                                .properties
                                .set("builder_graph_group_order", Value::Int(group_order as i32));
                        }
                        if map.get_surface_for_sector_id(sector_id).is_none() {
                            let mut surface = Surface::new(sector_id);
                            surface.calculate_geometry(map);
                            map.surfaces.insert(surface.id, surface);
                        }
                    }
                }
                BuilderOutputTarget::VertexPair => {
                    for (group_order, vertex_id) in
                        map.selected_vertices.clone().into_iter().enumerate()
                    {
                        if let Some(vertex) = map.find_vertex_mut(vertex_id) {
                            vertex
                                .properties
                                .set("builder_graph_id", Value::Id(asset_builder_id));
                            vertex
                                .properties
                                .set("builder_graph_name", Value::Str(asset_graph_name.clone()));
                            vertex
                                .properties
                                .set("builder_graph_data", Value::Str(asset_graph_data.clone()));
                            vertex.properties.set(
                                "builder_graph_target",
                                Value::Str("vertex_pair".to_string()),
                            );
                            vertex
                                .properties
                                .set("builder_graph_host_refs", Value::Int(spec.host_refs as i32));
                            vertex
                                .properties
                                .set("builder_graph_group_id", Value::Id(group_id));
                            vertex
                                .properties
                                .set("builder_graph_group_order", Value::Int(group_order as i32));
                        }
                    }
                }
                BuilderOutputTarget::Linedef => {
                    for (group_order, linedef_id) in
                        map.selected_linedefs.clone().into_iter().enumerate()
                    {
                        let wall_side =
                            Self::linedef_builder_wall_side(map, server_ctx, linedef_id);
                        let wall_outward =
                            Self::linedef_builder_outward(map, server_ctx, linedef_id);
                        let wall_face_origin =
                            Self::linedef_builder_face_origin(map, server_ctx, linedef_id);
                        if let Some(linedef) = map.find_linedef_mut(linedef_id) {
                            linedef
                                .properties
                                .set("builder_graph_id", Value::Id(asset_builder_id));
                            linedef
                                .properties
                                .set("builder_graph_name", Value::Str(asset_graph_name.clone()));
                            linedef
                                .properties
                                .set("builder_graph_data", Value::Str(asset_graph_data.clone()));
                            linedef
                                .properties
                                .set("builder_graph_target", Value::Str("linedef".to_string()));
                            linedef
                                .properties
                                .set("builder_graph_host_refs", Value::Int(spec.host_refs as i32));
                            linedef
                                .properties
                                .set("builder_graph_group_id", Value::Id(group_id));
                            linedef
                                .properties
                                .set("builder_graph_group_order", Value::Int(group_order as i32));
                            linedef
                                .properties
                                .set("builder_graph_wall_side", Value::Float(wall_side));
                            if let Some(outward) = wall_outward {
                                linedef
                                    .properties
                                    .set("builder_graph_outward_x", Value::Float(outward.x));
                                linedef
                                    .properties
                                    .set("builder_graph_outward_y", Value::Float(outward.y));
                                linedef
                                    .properties
                                    .set("builder_graph_outward_z", Value::Float(outward.z));
                            }
                            if let Some(face_origin) = wall_face_origin {
                                linedef.properties.set(
                                    "builder_graph_surface_origin_x",
                                    Value::Float(face_origin.x),
                                );
                                linedef.properties.set(
                                    "builder_graph_surface_origin_y",
                                    Value::Float(face_origin.y),
                                );
                                linedef.properties.set(
                                    "builder_graph_surface_origin_z",
                                    Value::Float(face_origin.z),
                                );
                            }
                            linedef.properties.remove("builder_graph_face_offset");
                        }
                    }
                }
            }
        }
    }

    fn clear_selected_hosts(&self, project: &mut Project, server_ctx: &mut ServerContext) {
        let Some(map) = project.get_map_mut(server_ctx) else {
            return;
        };

        for sector_id in map.selected_sectors.clone() {
            if let Some(sector) = map.find_sector_mut(sector_id) {
                for key in [
                    "builder_graph_id",
                    "builder_graph_name",
                    "builder_graph_data",
                    "builder_graph_target",
                    "builder_surface_mode",
                    "builder_hide_host",
                    "builder_graph_host_refs",
                    "builder_graph_group_id",
                    "builder_graph_group_order",
                ] {
                    sector.properties.remove(key);
                }
            }
        }

        for vertex_id in map.selected_vertices.clone() {
            if let Some(vertex) = map.find_vertex_mut(vertex_id) {
                for key in [
                    "builder_graph_id",
                    "builder_graph_name",
                    "builder_graph_data",
                    "builder_graph_target",
                    "builder_graph_host_refs",
                    "builder_graph_wall_side",
                    "builder_graph_outward_x",
                    "builder_graph_outward_y",
                    "builder_graph_outward_z",
                    "builder_graph_group_id",
                    "builder_graph_group_order",
                ] {
                    vertex.properties.remove(key);
                }
            }
        }

        for linedef_id in map.selected_linedefs.clone() {
            if let Some(linedef) = map.find_linedef_mut(linedef_id) {
                for key in [
                    "builder_graph_id",
                    "builder_graph_name",
                    "builder_graph_data",
                    "builder_graph_target",
                    "builder_graph_host_refs",
                    "builder_graph_wall_side",
                    "builder_graph_outward_x",
                    "builder_graph_outward_y",
                    "builder_graph_outward_z",
                    "builder_graph_surface_origin_x",
                    "builder_graph_surface_origin_y",
                    "builder_graph_surface_origin_z",
                    "builder_graph_face_offset",
                    "builder_graph_group_id",
                    "builder_graph_group_order",
                ] {
                    linedef.properties.remove(key);
                }
            }
        }
    }
}
