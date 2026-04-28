use crate::editor::{ACTIONLIST, UNDOMANAGER};
use crate::prelude::*;
use rusterix::{PixelSource, TileRole, TileSource, VertexBlendPreset};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const TILES_TAB_LAYOUT: &str = "Tiles Dock Tabs";
const TILE_VIEW_PREFIX: &str = "Tiles Dock View ";
const TILE_BOARD_COLS: i32 = 12;
const TILE_BOARD_EXTRA_COLS: i32 = 1;
const TILE_BOARD_EXTRA_ROWS: i32 = 1;
const TILE_CELL_BASE: i32 = 40;

#[derive(Clone, Copy)]
struct TileBoardEntry {
    source: TileSource,
    span: Vec2<i32>,
    pos: Option<Vec2<i32>>,
}

#[derive(Clone, Copy)]
struct TileBoardPlacement {
    source: TileSource,
    rect: Vec4<i32>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TileTabKind {
    Project,
    Collection(Uuid),
    Treasury,
}

#[derive(Clone, Copy)]
struct TileTabSpec {
    kind: TileTabKind,
}

impl TileTabSpec {
    fn name(&self, project: &Project) -> String {
        match self.kind {
            TileTabKind::Project => "Project".to_string(),
            TileTabKind::Collection(id) => project
                .tile_collections
                .get(&id)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "Collection".to_string()),
            TileTabKind::Treasury => "Treasury".to_string(),
        }
    }
}

pub struct TilesDock {
    pub filter: String,
    pub filter_role: u8,
    pub zoom: f32,
    pub apply_tile_mode: i32,

    pub curr_tile: Option<Uuid>,
    pub curr_source: Option<TileSource>,
    selected_reserved_slot: Option<(usize, Vec2<i32>)>,

    pub tile_preview_mode: bool,
    pub tile_hover_source: Option<TileSource>,
    treasury_packages: Vec<TreasuryPackageSummary>,
    treasury_error: Option<String>,
    treasury_loaded: bool,

    blend_index: usize,
    active_tab: usize,
    tab_names: Vec<String>,
    tab_offset: Vec<Vec2<i32>>,
    placements: Vec<Vec<TileBoardPlacement>>,
    drag_pan: Option<(usize, Vec2<i32>, Vec2<i32>)>,
    drag_item: Option<(usize, TileSource, Vec2<i32>, Vec2<i32>)>,
    drag_drop_cell: Option<(usize, Vec2<i32>)>,
    drag_hover_coord: Option<(usize, Vec2<i32>)>,
    entered_group: Option<Uuid>,
    entered_group_saved_offset: Option<(usize, Vec2<i32>)>,
    last_group_click: Option<(Uuid, u128)>,
    particle_preview_time: f32,
    particle_preview_last_tick: Instant,
}

impl Dock for TilesDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            filter: String::new(),
            filter_role: 0,
            zoom: 1.5,
            apply_tile_mode: 1,
            curr_tile: None,
            curr_source: None,
            selected_reserved_slot: None,
            tile_preview_mode: false,
            tile_hover_source: None,
            treasury_packages: Vec::new(),
            treasury_error: None,
            treasury_loaded: false,
            blend_index: 0,
            active_tab: 0,
            tab_names: Vec::new(),
            tab_offset: Vec::new(),
            placements: Vec::new(),
            drag_pan: None,
            drag_item: None,
            drag_drop_cell: None,
            drag_hover_coord: None,
            entered_group: None,
            entered_group_saved_offset: None,
            last_group_click: None,
            particle_preview_time: 0.0,
            particle_preview_last_tick: Instant::now(),
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut toolbar_canvas = TheCanvas::default();
        let traybar_widget = TheTraybar::new(TheId::empty());
        toolbar_canvas.set_widget(traybar_widget);
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);

        toolbar_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        toolbar_hlayout.set_padding(3);

        let mut filter_edit = TheTextLineEdit::new(TheId::named("Tiles Dock Filter Edit"));
        filter_edit.set_text(String::new());
        filter_edit.limiter_mut().set_max_size(Vec2::new(120, 18));
        filter_edit.set_font_size(12.5);
        filter_edit.set_status_text(&fl!("status_tiles_filter_edit"));
        filter_edit.set_continuous(true);
        toolbar_hlayout.add_widget(Box::new(filter_edit));

        let mut drop_down = TheDropdownMenu::new(TheId::named("Tiles Dock Filter Role"));
        drop_down.add_option(fl!("all"));
        for dir in TileRole::iterator() {
            drop_down.add_option(dir.to_string().to_string());
        }
        toolbar_hlayout.add_widget(Box::new(drop_down));

        let mut spacer = TheSpacer::new(TheId::empty());
        spacer.limiter_mut().set_max_width(10);
        toolbar_hlayout.add_widget(Box::new(spacer));
        toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));

        let mut add_button = TheTraybarButton::new(TheId::named("Tiles Dock Add"));
        add_button.set_text("New".to_string());
        add_button.set_status_text("Create a new tile or node group.");
        add_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new(
                    "Tile Group".to_string(),
                    TheId::named("Tiles Dock Add Tile Group"),
                ),
                TheContextMenuItem::new(
                    "Node Group".to_string(),
                    TheId::named("Tiles Dock Add Node Group"),
                ),
            ],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(add_button));

        let mut collection_button = TheTraybarButton::new(TheId::named("Tiles Dock Collections"));
        collection_button.set_text("Collections".to_string());
        collection_button
            .set_status_text("Manage collection membership for the selected tile or group.");
        toolbar_hlayout.add_widget(Box::new(collection_button));

        let mut apply_button = TheTraybarButton::new(TheId::named("Tiles Dock Apply Tile"));
        apply_button.set_text(fl!("action_apply_tile"));
        apply_button.set_status_text(&fl!("status_tiles_apply_tile"));
        apply_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new(
                    "Repeat".to_string(),
                    TheId::named("Tiles Dock Apply Tile Repeat"),
                ),
                TheContextMenuItem::new(
                    "Scale".to_string(),
                    TheId::named("Tiles Dock Apply Tile Scale"),
                ),
            ],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(apply_button));

        let mut clear_button = TheTraybarButton::new(TheId::named("Tiles Dock Clear Tile"));
        clear_button.set_text("Clear".to_string());
        clear_button.set_status_text(&fl!("status_tiles_clear_tile"));
        toolbar_hlayout.add_widget(Box::new(clear_button));
        toolbar_hlayout.set_reverse_index(Some(2));

        toolbar_canvas.set_layout(toolbar_hlayout);
        canvas.set_top(toolbar_canvas);

        let mut tab_layout = TheTabLayout::new(TheId::named(TILES_TAB_LAYOUT));
        for tab in 0..2 {
            let mut tab_canvas = TheCanvas::new();
            let render_view = TheRenderView::new(TheId::named(&format!("{TILE_VIEW_PREFIX}{tab}")));
            tab_canvas.set_widget(render_view);
            tab_layout.add_canvas(
                if tab == 0 { "Project" } else { "Treasury" }.to_string(),
                tab_canvas,
            );
        }
        canvas.set_layout(tab_layout);

        let mut bottom_toolbar_canvas = TheCanvas::default();
        let traybar_widget = TheTraybar::new(TheId::empty());
        bottom_toolbar_canvas.set_widget(traybar_widget);
        let mut bottom_toolbar_hlayout = TheHLayout::new(TheId::empty());
        bottom_toolbar_hlayout.set_background_color(None);
        bottom_toolbar_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        bottom_toolbar_hlayout.set_padding(3);

        let size = 24;
        for (index, p) in VertexBlendPreset::ALL.iter().enumerate() {
            let weights = p.weights();
            let buffer = p.preview_vertex_blend(weights, size);
            let rgba = TheRGBABuffer::from(buffer, size as u32, size as u32);
            let mut view = TheIconView::new(TheId::named(&format!("Blend #{}", index)));
            view.set_rgba_tile(TheRGBATile::buffer(rgba));
            if index == 0 {
                view.set_border_color(Some(WHITE));
            }
            bottom_toolbar_hlayout.add_widget(Box::new(view));

            if index == 2 || index == 6 || index == 10 || index == 14 {
                let mut spacer = TheSpacer::new(TheId::empty());
                spacer.limiter_mut().set_max_width(4);
                bottom_toolbar_hlayout.add_widget(Box::new(spacer));
            }
        }

        bottom_toolbar_canvas.set_layout(bottom_toolbar_hlayout);
        canvas.set_bottom(bottom_toolbar_canvas);

        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        self.curr_tile = server_ctx.curr_tile_id;
        self.curr_source = server_ctx.curr_tile_source;
        self.ensure_treasury_loaded();
        self.sync_tabs(ui, ctx, project);
        self.sync_collection_menu(ui, project);
        self.sync_sidebar(ctx, project);
        self.render_views(ui, ctx, project);
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if server_ctx.help_mode {
            let open_tiles_help = match event {
                TheEvent::RenderViewClicked(id, _) => Self::tab_from_view_name(&id.name).is_some(),
                TheEvent::StateChanged(id, state) if *state == TheWidgetState::Clicked => {
                    id.name == "Tiles"
                        || Self::tab_from_view_name(&id.name).is_some()
                        || id.name.starts_with("Blend #")
                }
                TheEvent::MouseDown(coord) => ui
                    .get_widget_at_coord(*coord)
                    .map(|w| {
                        let name = &w.id().name;
                        Self::tab_from_view_name(name).is_some()
                            || name == "Tiles"
                            || name.starts_with("Blend #")
                    })
                    .unwrap_or(false),
                _ => false,
            };
            if open_tiles_help {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Show Help"),
                    TheValue::Text("docs/creator/docks/tile_picker_editor".into()),
                ));
                return true;
            }
        }

        let mut redraw = false;

        match event {
            TheEvent::WidgetResized(id, _) => {
                if Self::tab_from_view_name(&id.name).is_some() {
                    self.render_views(ui, ctx, project);
                    redraw = true;
                }
            }
            TheEvent::IndexChanged(id, index) => {
                if id.name == format!("{TILES_TAB_LAYOUT} Tabbar") {
                    self.active_tab = *index;
                    self.sync_collection_menu(ui, project);
                    self.sync_sidebar(ctx, project);
                    self.render_views(ui, ctx, project);
                    redraw = true;
                }
            }
            TheEvent::ContextMenuSelected(widget_id, item_id) => {
                if widget_id.name == "Tiles Dock Add" {
                    if item_id.name == "Tiles Dock Add Tile Group" {
                        self.create_empty_group(project, ui, ctx, server_ctx, false);
                        self.render_views(ui, ctx, project);
                        redraw = true;
                    } else if item_id.name == "Tiles Dock Add Node Group" {
                        self.create_empty_group(project, ui, ctx, server_ctx, true);
                        self.render_views(ui, ctx, project);
                        redraw = true;
                    }
                } else if widget_id.name == "Tiles Dock Collections" {
                    if item_id.name == "Tiles Dock New Collection" {
                        self.create_collection(project, ui, ctx);
                        self.sync_collection_menu(ui, project);
                        self.sync_sidebar(ctx, project);
                        self.render_views(ui, ctx, project);
                        redraw = true;
                    } else if item_id.name == "Tiles Dock Import Collection" {
                        ctx.ui.open_file_requester(
                            TheId::named("Tiles Dock Import Collection"),
                            "Import Tile Collection".into(),
                            TheFileExtension::new(
                                "Eldiron Tile Collection".into(),
                                vec!["eldiron_tiles".to_string(), "json".to_string()],
                            ),
                        );
                    } else if item_id.name == "Tiles Dock Export Current Collection"
                        && let TileTabKind::Collection(collection_id) =
                            self.current_tab_kind(project)
                    {
                        ctx.ui.save_file_requester(
                            TheId::named_with_id(
                                "Tiles Dock Export Current Collection",
                                collection_id,
                            ),
                            "Export Tile Collection".into(),
                            TheFileExtension::new(
                                "Eldiron Tile Collection".into(),
                                vec!["eldiron_tiles".to_string()],
                            ),
                        );
                    } else if item_id.name == "Tiles Dock Export Current Collection To Treasury"
                        && let TileTabKind::Collection(collection_id) =
                            self.current_tab_kind(project)
                    {
                        match default_treasury_repo_root() {
                            Some(repo_root) => match export_tile_collection_to_treasury_repo(
                                project,
                                collection_id,
                                &repo_root,
                            ) {
                                Ok(package_dir) => {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        widget_id.clone(),
                                        format!(
                                            "Collection exported to Treasury repo at {}",
                                            package_dir.to_string_lossy()
                                        ),
                                    ));
                                }
                                Err(err) => {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        widget_id.clone(),
                                        format!("Treasury export failed: {err}"),
                                    ));
                                }
                            },
                            None => {
                                ctx.ui.send(TheEvent::SetStatusText(
                                    widget_id.clone(),
                                    "Could not resolve the local Treasury repo path.".to_string(),
                                ));
                            }
                        }
                    } else if item_id.name == "Tiles Dock Add To Collection" {
                        if let Some(source) = self.curr_source {
                            project.add_source_to_collection(&item_id.uuid, source);
                            let target_tab = self
                                .tab_specs(project)
                                .iter()
                                .position(|spec| spec.kind == TileTabKind::Collection(item_id.uuid))
                                .unwrap_or(self.active_tab);
                            let span = self.source_span(project, source);
                            let pos = self.find_open_cell_for_span(project, target_tab, span);
                            project.set_collection_tile_board_position(&item_id.uuid, source, pos);
                            self.sync_tabs(ui, ctx, project);
                            self.sync_collection_menu(ui, project);
                            self.sync_sidebar(ctx, project);
                            self.render_views(ui, ctx, project);
                            redraw = true;
                        }
                    } else if item_id.name == "Tiles Dock Remove From Current Collection" {
                        if let Some(source) = self.curr_source
                            && let TileTabKind::Collection(collection_id) =
                                self.current_tab_kind(project)
                            && let Some(collection) =
                                project.tile_collections.get_mut(&collection_id)
                        {
                            collection
                                .entries
                                .retain(|entry| !entry.matches_source(source));
                            match source {
                                TileSource::SingleTile(id) => {
                                    collection.tile_board_tiles.shift_remove(&id);
                                }
                                TileSource::TileGroup(id) => {
                                    collection.tile_board_groups.shift_remove(&id);
                                }
                                _ => {}
                            }
                            self.sync_collection_menu(ui, project);
                            self.sync_sidebar(ctx, project);
                            self.render_views(ui, ctx, project);
                            redraw = true;
                        }
                    } else if item_id.name == "Tiles Dock Delete Current Collection"
                        && let TileTabKind::Collection(collection_id) =
                            self.current_tab_kind(project)
                    {
                        project.tile_collections.shift_remove(&collection_id);
                        self.active_tab = 0;
                        self.sync_tabs(ui, ctx, project);
                        self.sync_collection_menu(ui, project);
                        self.sync_sidebar(ctx, project);
                        self.render_views(ui, ctx, project);
                        redraw = true;
                    }
                } else if widget_id.name == "Tiles Dock Apply Tile" {
                    if item_id.name == "Tiles Dock Apply Tile Repeat" {
                        self.apply_tile_mode = 1;
                        let mut undo_atom: Option<ProjectUndoAtom> = None;
                        let mut needs_scene_redraw = false;
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            let prev = map.clone();
                            let mut changed = false;
                            for sector_id in map.selected_sectors.clone() {
                                if let Some(sector) = map.find_sector_mut(sector_id)
                                    && sector.properties.get_int_default("tile_mode", 1) != 1
                                {
                                    sector.properties.set("tile_mode", Value::Int(1));
                                    changed = true;
                                }
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
                            crate::utils::editor_scene_apply_map_edit_atom(
                                project, server_ctx, undo_atom,
                            );
                        }
                        if let Some(undo_atom) = undo_atom {
                            UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Update Minimap"),
                                TheValue::Empty,
                            ));
                        }
                        ctx.ui.send(TheEvent::SetStatusText(
                            widget_id.clone(),
                            "Apply tile with repeating UVs.".to_string(),
                        ));
                    } else if item_id.name == "Tiles Dock Apply Tile Scale" {
                        self.apply_tile_mode = 0;
                        let mut undo_atom: Option<ProjectUndoAtom> = None;
                        let mut needs_scene_redraw = false;
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            let prev = map.clone();
                            let mut changed = false;
                            for sector_id in map.selected_sectors.clone() {
                                if let Some(sector) = map.find_sector_mut(sector_id)
                                    && sector.properties.get_int_default("tile_mode", 1) != 0
                                {
                                    sector.properties.set("tile_mode", Value::Int(0));
                                    changed = true;
                                }
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
                            crate::utils::editor_scene_apply_map_edit_atom(
                                project, server_ctx, undo_atom,
                            );
                        }
                        if let Some(undo_atom) = undo_atom {
                            UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Update Minimap"),
                                TheValue::Empty,
                            ));
                        }
                        ctx.ui.send(TheEvent::SetStatusText(
                            widget_id.clone(),
                            "Apply tile scaled to fit the sector surface.".to_string(),
                        ));
                    }
                }
            }
            TheEvent::FileRequesterResult(id, paths) => {
                if id.name == "Tiles Dock Import Collection" {
                    let mut last_collection_id = None;
                    for path in paths {
                        match import_tile_collection_package(project, path) {
                            Ok(collection_id) => {
                                last_collection_id = Some(collection_id);
                            }
                            Err(err) => {
                                ctx.ui.send(TheEvent::SetStatusText(
                                    id.clone(),
                                    format!("Collection import failed: {err}"),
                                ));
                            }
                        }
                    }
                    if let Some(collection_id) = last_collection_id {
                        self.active_tab = self
                            .tab_specs(project)
                            .iter()
                            .position(|spec| spec.kind == TileTabKind::Collection(collection_id))
                            .unwrap_or(0);
                        self.sync_tabs(ui, ctx, project);
                        self.sync_collection_menu(ui, project);
                        self.sync_sidebar(ctx, project);
                        self.render_views(ui, ctx, project);
                        redraw = true;
                    }
                } else if id.name == "Tiles Dock Export Current Collection"
                    && let Some(path) = paths.first()
                {
                    match export_tile_collection_package(project, id.uuid, path) {
                        Ok(saved_path) => {
                            ctx.ui.send(TheEvent::SetStatusText(
                                id.clone(),
                                format!("Collection exported to {}", saved_path.to_string_lossy()),
                            ));
                        }
                        Err(err) => {
                            ctx.ui.send(TheEvent::SetStatusText(
                                id.clone(),
                                format!("Collection export failed: {err}"),
                            ));
                        }
                    }
                }
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) => {
                if id.name.starts_with("Blend #") {
                    if let Ok(index) = id.name.strip_prefix("Blend #").unwrap().parse::<usize>() {
                        if let Some(old_icon) =
                            ui.get_icon_view(&format!("Blend #{}", self.blend_index))
                        {
                            old_icon.set_border_color(None);
                        }
                        if let Some(old_icon) = ui.get_icon_view(&format!("Blend #{}", index)) {
                            old_icon.set_border_color(Some(WHITE));
                        }
                        self.blend_index = index;
                        server_ctx.rect_blend_preset = VertexBlendPreset::from_index(index)
                            .unwrap_or(VertexBlendPreset::Solid);
                    }
                } else if id.name == "Tiles Dock Apply Tile" {
                    if let Some(TileSource::Procedural(package_id)) = self.curr_source
                        && let Some(package) = self
                            .treasury_packages
                            .iter()
                            .find(|pkg| pkg.id == package_id)
                    {
                        match install_tile_package(project, package) {
                            Ok(collection_id) => {
                                self.active_tab = self
                                    .tab_specs(project)
                                    .iter()
                                    .position(|spec| {
                                        spec.kind == TileTabKind::Collection(collection_id)
                                    })
                                    .unwrap_or(0);
                                self.sync_tabs(ui, ctx, project);
                                self.sync_collection_menu(ui, project);
                                self.sync_sidebar(ctx, project);
                                self.render_views(ui, ctx, project);
                            }
                            Err(err) => {
                                ctx.ui.send(TheEvent::SetStatusText(
                                    id.clone(),
                                    format!("Treasury install failed: {err}"),
                                ));
                            }
                        }
                        return true;
                    }
                    let builder_selected_source = crate::utils::get_source(ui, server_ctx);
                    let selected_source =
                        crate::utils::get_surface_apply_source(project, server_ctx);
                    if let Some(selected_source) = selected_source {
                        let mut applied_to_action = false;
                        let mut undo_atom: Option<ProjectUndoAtom> = None;
                        let mut needs_scene_redraw = false;

                        if let Some(source) = builder_selected_source
                            && let Some(map) = project.get_map_mut(server_ctx)
                        {
                            let prev = map.clone();
                            if crate::actions::apply_builder_hud_material_to_selection(
                                map,
                                server_ctx,
                                server_ctx.selected_hud_icon_index,
                                Some(source),
                            ) {
                                undo_atom = Some(ProjectUndoAtom::MapEdit(
                                    server_ctx.pc,
                                    Box::new(prev),
                                    Box::new(map.clone()),
                                ));
                                needs_scene_redraw = true;
                                applied_to_action = true;
                            }
                        }

                        if !applied_to_action
                            && server_ctx.get_map_context() == MapContext::Region
                            && let Some(map) = project.get_map(server_ctx)
                            && let Some(action_id) = server_ctx.curr_action_id
                            && let Some(action) =
                                ACTIONLIST.write().unwrap().get_action_by_id_mut(action_id)
                            && action.hud_material_slots(map, server_ctx).is_some()
                            && let crate::utils::SurfaceApplySource::Direct(PixelSource::TileId(
                                tile_id,
                            )) = &selected_source
                            && action.set_hud_material_from_tile(
                                map,
                                server_ctx,
                                server_ctx.selected_hud_icon_index,
                                *tile_id,
                            )
                        {
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Refresh Action Parameters"),
                                TheValue::Empty,
                            ));
                            applied_to_action = true;
                        }

                        if !applied_to_action {
                            if let Some(map) = project.get_map_mut(server_ctx) {
                                let mut changed = false;
                                let prev = map.clone();

                                for sector_id in map.selected_sectors.clone() {
                                    let mut source_key = "source";
                                    if server_ctx.pc.is_screen()
                                        && server_ctx.selected_hud_icon_index == 1
                                    {
                                        source_key = "ceiling_source";
                                    }
                                    changed |= crate::utils::apply_surface_source_to_sector(
                                        map,
                                        sector_id,
                                        source_key,
                                        &selected_source,
                                        Some(self.apply_tile_mode),
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
                            crate::utils::editor_scene_apply_map_edit_atom(
                                project, server_ctx, undo_atom,
                            );
                        }

                        if let Some(undo_atom) = undo_atom {
                            UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Update Minimap"),
                                TheValue::Empty,
                            ));
                        }
                    }
                } else if id.name == "Tiles Dock Clear Tile" {
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
                        && let Some(action) =
                            ACTIONLIST.write().unwrap().get_action_by_id_mut(action_id)
                        && action.hud_material_slots(map, server_ctx).is_some()
                        && action.clear_hud_material_slot(
                            map,
                            server_ctx,
                            server_ctx.selected_hud_icon_index,
                        )
                    {
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Refresh Action Parameters"),
                            TheValue::Empty,
                        ));
                        cleared_action_slot = true;
                    }

                    if !cleared_action_slot {
                        if let Some(map) = project.get_map_mut(server_ctx) {
                            let mut changed = false;
                            let prev = map.clone();

                            for sector_id in map.selected_sectors.clone() {
                                let mut source = "source";
                                if server_ctx.pc.is_screen()
                                    && server_ctx.selected_hud_icon_index == 1
                                {
                                    source = "ceiling_source";
                                }

                                changed |= crate::utils::clear_surface_source_on_sector(
                                    map, sector_id, source,
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
                        crate::utils::editor_scene_apply_map_edit_atom(
                            project, server_ctx, undo_atom,
                        );
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
            TheEvent::Resize => {
                self.render_views(ui, ctx, project);
                redraw = true;
            }
            TheEvent::RenderViewClicked(id, coord) => {
                if let Some(tab) = Self::tab_from_view_name(&id.name) {
                    if self.entered_group.is_none()
                        && let Some(top_level_source) = self.pick_top_level_source(tab, *coord)
                    {
                        let start_cell = self
                            .source_board_cell(project, tab, top_level_source)
                            .unwrap_or_else(|| {
                                self.coord_to_board_cell(tab, *coord)
                                    .unwrap_or(Vec2::zero())
                            });
                        let grab_offset = self
                            .coord_to_board_cell(tab, *coord)
                            .map(|cell| cell - start_cell)
                            .unwrap_or(Vec2::zero());
                        self.drag_item = Some((tab, top_level_source, start_cell, grab_offset));
                        self.drag_drop_cell = self.coord_to_board_cell(tab, *coord).map(|cell| {
                            let cell = cell - grab_offset;
                            (tab, Vec2::new(cell.x.max(0), cell.y.max(0)))
                        });
                        self.drag_hover_coord = Some((tab, *coord));
                        self.drag_pan = None;
                        if let TileSource::TileGroup(group_id) = top_level_source {
                            let now = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .map(|d| d.as_millis())
                                .unwrap_or(0);
                            if let Some((last_id, last_time)) = self.last_group_click
                                && last_id == group_id
                                && now.saturating_sub(last_time) < 400
                            {
                                self.enter_group(group_id, ui, ctx, project);
                            }
                            self.last_group_click = Some((group_id, now));
                        }
                    } else {
                        self.drag_pan = Some((tab, *coord, self.tab_offset[tab]));
                        self.drag_item = None;
                        self.drag_drop_cell = None;
                        self.drag_hover_coord = None;
                    }
                    if let Some(source) = self.pick_source(project, tab, *coord) {
                        self.select_source(project, source, ui, ctx, server_ctx);
                    } else if let Some(cell) = self.coord_to_board_cell(tab, *coord) {
                        if self.is_reserved_slot(project, tab, cell) {
                            self.select_reserved_slot(tab, cell, server_ctx);
                        } else {
                            self.clear_selection(server_ctx);
                        }
                    } else {
                        self.clear_selection(server_ctx);
                    }
                    self.render_views(ui, ctx, project);
                    redraw = true;
                }
            }
            TheEvent::RenderViewDragged(id, coord) => {
                if let Some(tab) = Self::tab_from_view_name(&id.name) {
                    if self.entered_group.is_none()
                        && let Some((drag_tab, _source, _start_cell, grab_offset)) = self.drag_item
                        && drag_tab == tab
                    {
                        self.auto_scroll_drag(ui, tab, &id.name, *coord);
                        self.drag_drop_cell = self.coord_to_board_cell(tab, *coord).map(|cell| {
                            let cell = cell - grab_offset;
                            (tab, Vec2::new(cell.x.max(0), cell.y.max(0)))
                        });
                        self.drag_hover_coord = Some((tab, *coord));
                    } else if let Some((drag_tab, origin, start_offset)) = self.drag_pan
                        && drag_tab == tab
                    {
                        let delta = *coord - origin;
                        self.tab_offset[tab] = Vec2::new(
                            (start_offset.x - delta.x).max(0),
                            (start_offset.y - delta.y).max(0),
                        );
                    }
                    self.render_views(ui, ctx, project);
                    redraw = true;
                }
            }
            TheEvent::RenderViewUp(id, coord) => {
                if Self::tab_from_view_name(&id.name).is_some() {
                    if self.entered_group.is_none()
                        && let Some((tab, source, start_cell, _grab_offset)) = self.drag_item
                    {
                        let before = project.clone();
                        let mut changed = false;
                        if let Some(group_id) =
                            self.pick_group_drop_target(project, tab, *coord, source)
                        {
                            changed =
                                self.try_drop_into_group(project, tab, *coord, source, group_id);
                        } else if let Some((drop_tab, cell)) = self.drag_drop_cell
                            && drop_tab == tab
                        {
                            changed = self
                                .try_move_with_displacement(project, tab, source, start_cell, cell);
                            if !changed {
                                self.set_board_position_for_tab(project, tab, source, start_cell);
                            }
                        } else {
                            self.set_board_position_for_tab(project, tab, source, start_cell);
                        }
                        if changed {
                            let after = project.clone();
                            UNDOMANAGER.write().unwrap().add_undo(
                                ProjectUndoAtom::TilePickerEdit(Box::new(before), Box::new(after)),
                                ctx,
                            );
                        }
                    }
                    self.drag_pan = None;
                    self.drag_item = None;
                    self.drag_drop_cell = None;
                    self.drag_hover_coord = None;
                    self.render_views(ui, ctx, project);
                    redraw = true;
                }
            }
            TheEvent::RenderViewHoverChanged(id, coord) => {
                if let Some(tab) = Self::tab_from_view_name(&id.name) {
                    self.tile_hover_source = self.pick_source(project, tab, *coord);
                    self.tile_preview_mode = self.tile_hover_source.is_some();
                    if let Some(source) = self.tile_hover_source {
                        ctx.ui.send(TheEvent::SetStatusText(
                            id.clone(),
                            self.status_text_for_source(project, source),
                        ));
                    }
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Soft Update Minimap"),
                        TheValue::Empty,
                    ));
                    self.render_views(ui, ctx, project);
                    redraw = true;
                }
            }
            TheEvent::RenderViewLostHover(id) => {
                if Self::tab_from_view_name(&id.name).is_some() {
                    self.drag_pan = None;
                    self.drag_item = None;
                    self.drag_drop_cell = None;
                    self.drag_hover_coord = None;
                    self.tile_preview_mode = false;
                    self.tile_hover_source = None;
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Soft Update Minimap"),
                        TheValue::Empty,
                    ));
                    self.render_views(ui, ctx, project);
                    redraw = true;
                }
            }
            TheEvent::RenderViewScrollBy(id, delta) => {
                if let Some(tab) = Self::tab_from_view_name(&id.name) {
                    if ui.ctrl || ui.logo {
                        let zoom_delta = (delta.y as f32) * 0.05;
                        self.zoom = (self.zoom + zoom_delta).clamp(1.0, 3.0);
                        ui.set_widget_value("Tiles Dock Zoom", ctx, TheValue::Float(self.zoom));
                    } else {
                        self.tab_offset[tab].x = (self.tab_offset[tab].x + delta.x).max(0);
                        self.tab_offset[tab].y = (self.tab_offset[tab].y + delta.y).max(0);
                    }
                    self.render_views(ui, ctx, project);
                    if let Some(render_view) = ui.get_render_view(&id.name) {
                        render_view.set_needs_redraw(true);
                    }
                    ctx.ui.redraw_all = true;
                    redraw = true;
                }
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(key)) => {
                if *key == TheKeyCode::Return {
                    if let Some(TileSource::TileGroup(group_id)) = self.curr_source {
                        self.enter_group(group_id, ui, ctx, project);
                        redraw = true;
                    }
                } else if *key == TheKeyCode::Delete {
                    if self.board_has_active_focus(ctx)
                        && !ui.focus_widget_supports_text_input(ctx)
                        && server_ctx.tile_node_group_id.is_none()
                    {
                        if let Some(source) = self.curr_source {
                            if self.delete_source(project, source, ui, ctx, server_ctx) {
                                redraw = true;
                            }
                        } else if self.delete_reserved_slot(project, ui, ctx, server_ctx) {
                            redraw = true;
                        }
                    }
                } else if *key == TheKeyCode::Escape && self.entered_group.is_some() {
                    self.leave_group(ui, ctx, project);
                    redraw = true;
                }
            }
            TheEvent::Custom(id, _) => {
                if id.name == "Update Tilepicker" {
                    self.curr_tile = server_ctx.curr_tile_id;
                    self.curr_source = server_ctx.curr_tile_source;
                    self.sync_collection_menu(ui, project);
                    self.sync_sidebar(ctx, project);
                    self.render_views(ui, ctx, project);
                    redraw = true;
                } else if id.name == "Reveal Tilepicker Source" {
                    self.curr_tile = server_ctx.curr_tile_id;
                    self.curr_source = server_ctx.curr_tile_source;
                    if let Some(source) = self.curr_source {
                        self.reveal_source(project, ui, ctx, source);
                        self.sync_collection_menu(ui, project);
                        self.sync_sidebar(ctx, project);
                        redraw = true;
                    }
                } else if id.name == "Soft Update Minimap" {
                    let now = Instant::now();
                    let elapsed = now
                        .saturating_duration_since(self.particle_preview_last_tick)
                        .as_secs_f32();
                    self.particle_preview_last_tick = now;
                    self.particle_preview_time += elapsed.clamp(0.0, 0.25);
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Tiles Dock Filter Edit" {
                    if let TheValue::Text(filter) = value {
                        self.filter = filter.to_lowercase();
                        self.render_views(ui, ctx, project);
                        redraw = true;
                    }
                } else if id.name == "Tiles Dock Filter Role" {
                    if let TheValue::Int(filter) = value {
                        self.filter_role = *filter as u8;
                        self.render_views(ui, ctx, project);
                        redraw = true;
                    }
                } else if id.name == "Tiles Dock Zoom"
                    && let TheValue::Float(zoom) = value
                {
                    self.zoom = *zoom;
                    self.render_views(ui, ctx, project);
                    redraw = true;
                }
            }
            _ => {}
        }
        redraw
    }

    fn draw_minimap(
        &self,
        buffer: &mut TheRGBABuffer,
        project: &Project,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) -> bool {
        let Some(source) = self.tile_hover_source else {
            return false;
        };

        self.draw_source_preview(buffer, project, ctx, server_ctx, source);
        true
    }

    fn supports_minimap_animation(&self) -> bool {
        true
    }
}

impl TilesDock {
    fn particle_preview_time(&self) -> f32 {
        const PREVIEW_FPS: f32 = 15.0;
        (self.particle_preview_time * PREVIEW_FPS).floor() / PREVIEW_FPS
    }

    fn activate_edit_tile_meta_action(server_ctx: &mut ServerContext) {
        if server_ctx.curr_tile_id.is_none() {
            return;
        }

        if let Some(action) = ACTIONLIST
            .read()
            .unwrap()
            .actions
            .iter()
            .find(|action| action.id().name == fl!("action_edit_tile"))
        {
            server_ctx.curr_action_id = Some(action.id().uuid);
        }
    }

    fn ensure_treasury_loaded(&mut self) {
        if self.treasury_loaded {
            return;
        }
        match fetch_tile_packages() {
            Ok(packages) => {
                self.treasury_packages = packages;
                self.treasury_error = None;
            }
            Err(err) => {
                self.treasury_packages.clear();
                self.treasury_error = Some(err);
            }
        }
        self.treasury_loaded = true;
    }

    fn sync_sidebar(&self, ctx: &mut TheContext, project: &Project) {
        match self.current_tab_kind(project) {
            TileTabKind::Collection(collection_id) => {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Show Collection Settings"),
                    TheValue::Id(collection_id),
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Hide Treasury Settings"),
                    TheValue::Empty,
                ));
            }
            TileTabKind::Treasury => {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Hide Collection Settings"),
                    TheValue::Empty,
                ));
                if let Some(TileSource::Procedural(package_id)) = self.curr_source
                    && let Some(package) = self
                        .treasury_packages
                        .iter()
                        .find(|pkg| pkg.id == package_id)
                {
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Show Treasury Settings"),
                        TheValue::List(vec![
                            TheValue::Text(package.slug.clone()),
                            TheValue::Text(package.display_name()),
                            TheValue::Text(package.author.clone()),
                            TheValue::Text(package.version.clone()),
                            TheValue::Text(package.description.clone()),
                        ]),
                    ));
                } else {
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Hide Treasury Settings"),
                        TheValue::Empty,
                    ));
                }
            }
            TileTabKind::Project => {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Hide Collection Settings"),
                    TheValue::Empty,
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Hide Treasury Settings"),
                    TheValue::Empty,
                ));
            }
        }
    }

    fn tab_specs(&self, project: &Project) -> Vec<TileTabSpec> {
        let mut specs = vec![TileTabSpec {
            kind: TileTabKind::Project,
        }];
        for id in project.tile_collections.keys() {
            specs.push(TileTabSpec {
                kind: TileTabKind::Collection(*id),
            });
        }
        specs.push(TileTabSpec {
            kind: TileTabKind::Treasury,
        });
        specs
    }

    fn current_tab_kind(&self, project: &Project) -> TileTabKind {
        self.tab_specs(project)
            .get(self.active_tab)
            .map(|tab| tab.kind)
            .unwrap_or(TileTabKind::Project)
    }

    fn ensure_tab_state(&mut self, count: usize) {
        if self.tab_offset.len() < count {
            self.tab_offset.resize(count, Vec2::zero());
        } else if self.tab_offset.len() > count {
            self.tab_offset.truncate(count);
        }
        if self.placements.len() < count {
            self.placements.resize_with(count, Vec::new);
        } else if self.placements.len() > count {
            self.placements.truncate(count);
        }
        if count == 0 {
            self.active_tab = 0;
        } else {
            self.active_tab = self.active_tab.min(count - 1);
        }
    }

    fn sync_tabs(&mut self, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        let specs = self.tab_specs(project);
        let names: Vec<String> = specs.iter().map(|spec| spec.name(project)).collect();
        self.ensure_tab_state(specs.len());
        if self.tab_names == names {
            if let Some(layout) = ui
                .canvas
                .get_layout(Some(&TILES_TAB_LAYOUT.to_string()), None)
                && let Some(tab_layout) = layout.as_tab_layout()
            {
                tab_layout.set_index(self.active_tab.min(specs.len().saturating_sub(1)));
            }
            return;
        }
        self.tab_names = names.clone();
        if let Some(layout) = ui
            .canvas
            .get_layout(Some(&TILES_TAB_LAYOUT.to_string()), None)
            && let Some(tab_layout) = layout.as_tab_layout()
        {
            tab_layout.clear();
            for (index, name) in names.iter().enumerate() {
                let mut tab_canvas = TheCanvas::new();
                let render_view =
                    TheRenderView::new(TheId::named(&format!("{TILE_VIEW_PREFIX}{index}")));
                tab_canvas.set_widget(render_view);
                tab_layout.add_canvas(name.clone(), tab_canvas);
            }
            tab_layout.set_index(self.active_tab.min(specs.len().saturating_sub(1)));
        }
        ctx.ui.relayout = true;
        ctx.ui.redraw_all = true;
    }

    fn sync_collection_menu(&mut self, ui: &mut TheUI, project: &Project) {
        let mut items = vec![TheContextMenuItem::new(
            "New Collection".to_string(),
            TheId::named("Tiles Dock New Collection"),
        )];
        items.push(TheContextMenuItem::new(
            "Import Collection...".to_string(),
            TheId::named("Tiles Dock Import Collection"),
        ));

        if let Some(source) = self.curr_source
            && !matches!(source, TileSource::Procedural(_))
        {
            for collection in project.tile_collections.values() {
                items.push(TheContextMenuItem::new(
                    format!("Add To {}", collection.name),
                    TheId::named_with_id("Tiles Dock Add To Collection", collection.id),
                ));
            }

            if let TileTabKind::Collection(collection_id) = self.current_tab_kind(project)
                && project.collection_contains_source(&collection_id, source)
            {
                items.push(TheContextMenuItem::new(
                    "Remove From Current".to_string(),
                    TheId::named("Tiles Dock Remove From Current Collection"),
                ));
            }
        }

        if matches!(self.current_tab_kind(project), TileTabKind::Collection(_)) {
            items.push(TheContextMenuItem::new(
                "Export Current...".to_string(),
                TheId::named("Tiles Dock Export Current Collection"),
            ));
            items.push(TheContextMenuItem::new(
                "Export Current To Treasury".to_string(),
                TheId::named("Tiles Dock Export Current Collection To Treasury"),
            ));
            items.push(TheContextMenuItem::new(
                "Delete Current Collection".to_string(),
                TheId::named("Tiles Dock Delete Current Collection"),
            ));
        }

        if let Some(widget) = ui.get_widget("Tiles Dock Collections") {
            widget.set_context_menu(Some(TheContextMenu {
                items,
                ..Default::default()
            }));
        }
    }

    fn enter_group(
        &mut self,
        group_id: Uuid,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
    ) {
        self.entered_group = Some(group_id);
        self.ensure_tab_state(self.tab_specs(project).len());
        if self.entered_group_saved_offset.is_none() && self.active_tab < self.tab_offset.len() {
            self.entered_group_saved_offset =
                Some((self.active_tab, self.tab_offset[self.active_tab]));
        }
        if self.active_tab < self.tab_offset.len() {
            self.tab_offset[self.active_tab] = Vec2::zero();
        }
        self.drag_pan = None;
        self.drag_item = None;
        self.drag_drop_cell = None;
        self.drag_hover_coord = None;
        if project.is_tile_node_group(&group_id) {
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Open Tile Node Group Workflow"),
                TheValue::Id(group_id),
            ));
        } else {
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Close Tile Node Editor Skeleton"),
                TheValue::Empty,
            ));
        }
        self.render_views(ui, ctx, project);
    }

    fn leave_group(&mut self, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        self.entered_group = None;
        if let Some((tab, offset)) = self.entered_group_saved_offset.take()
            && tab < self.tab_offset.len()
        {
            self.tab_offset[tab] = offset;
        }
        self.drag_pan = None;
        self.drag_item = None;
        self.drag_drop_cell = None;
        self.drag_hover_coord = None;
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Close Tile Node Editor Skeleton"),
            TheValue::Empty,
        ));
        self.render_views(ui, ctx, project);
    }

    fn set_active_tab_ui(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        tab: usize,
    ) {
        self.active_tab = tab;
        self.sync_tabs(ui, ctx, project);
        if let Some(layout) = ui
            .canvas
            .get_layout(Some(&TILES_TAB_LAYOUT.to_string()), None)
            && let Some(tab_layout) = layout.as_tab_layout()
        {
            tab_layout.set_index(tab);
        }
    }

    fn reveal_source(
        &mut self,
        project: &Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        source: TileSource,
    ) {
        match source {
            TileSource::TileGroupMember { group_id, .. } => {
                self.set_active_tab_ui(ui, ctx, project, 0);
                if self.entered_group != Some(group_id) {
                    self.enter_group(group_id, ui, ctx, project);
                }
            }
            _ => {
                if self.entered_group.is_some() {
                    self.leave_group(ui, ctx, project);
                }
                self.set_active_tab_ui(ui, ctx, project, 0);
            }
        }

        if let Some(cell) = self.source_board_cell(project, self.active_tab, source) {
            self.scroll_cell_into_view(ui, self.active_tab, cell, Vec2::new(1, 1));
        }
        self.render_views(ui, ctx, project);
    }

    fn create_empty_group(
        &mut self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        node_backed: bool,
    ) -> Uuid {
        let mut group = rusterix::TileGroup::new(2, 2);
        group.name = "New Group".to_string();
        let group_id = group.id;
        let create_tab = match self.current_tab_kind(project) {
            TileTabKind::Treasury => 0,
            _ => self.active_tab,
        };
        let pos = self.find_open_cell_for_span(project, create_tab, Vec2::new(2, 2));
        project.ensure_tile_board_space(pos + Vec2::new(1, 1));
        project.add_tile_group(group);
        if node_backed {
            let palette_colors = project
                .palette
                .colors
                .iter()
                .filter_map(|color| color.clone())
                .collect();
            project.add_tile_node_group(NodeGroupAsset::new(group_id, 2, 2, palette_colors));
        }
        if let TileTabKind::Collection(collection_id) = self.current_tab_kind(project) {
            project.add_source_to_collection(&collection_id, TileSource::TileGroup(group_id));
            project.set_collection_tile_board_position(
                &collection_id,
                TileSource::TileGroup(group_id),
                pos,
            );
        }
        project.set_tile_board_position(TileSource::TileGroup(group_id), pos);
        if self.active_tab != create_tab {
            self.active_tab = create_tab;
            if let Some(layout) = ui
                .canvas
                .get_layout(Some(&TILES_TAB_LAYOUT.to_string()), None)
                && let Some(tab_layout) = layout.as_tab_layout()
            {
                tab_layout.set_index(create_tab);
            }
        }
        self.scroll_cell_into_view(ui, create_tab, pos, Vec2::new(2, 2));
        self.select_source(
            project,
            TileSource::TileGroup(group_id),
            ui,
            ctx,
            server_ctx,
        );
        group_id
    }

    fn create_collection(
        &mut self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> Uuid {
        let index = project.tile_collections.len() + 1;
        let collection = TileCollectionAsset::new(format!("Collection {}", index));
        let collection_id = collection.id;
        project.add_tile_collection(collection);
        let specs = self.tab_specs(project);
        self.active_tab = specs
            .iter()
            .position(|spec| spec.kind == TileTabKind::Collection(collection_id))
            .unwrap_or(0);
        self.sync_tabs(ui, ctx, project);
        collection_id
    }

    fn group_grid_size(&self, project: &Project, group: &rusterix::TileGroup) -> Vec2<i32> {
        let extra = if group.members.is_empty() || project.is_tile_node_group(&group.id) {
            0
        } else {
            1
        };
        Vec2::new(
            group.width.max(1) as i32 + extra,
            group.height.max(1) as i32 + extra,
        )
    }

    fn source_span(&self, project: &Project, source: TileSource) -> Vec2<i32> {
        match source {
            TileSource::SingleTile(_) | TileSource::TileGroupMember { .. } => Vec2::new(1, 1),
            TileSource::TileGroup(group_id) => project
                .tile_groups
                .get(&group_id)
                .map(|group| self.group_grid_size(project, group))
                .unwrap_or(Vec2::new(1, 1)),
            TileSource::Procedural(_) => Vec2::new(1, 1),
        }
    }

    fn clear_selection(&mut self, server_ctx: &mut ServerContext) {
        self.curr_source = None;
        self.curr_tile = None;
        self.selected_reserved_slot = None;
        server_ctx.curr_tile_source = None;
        server_ctx.curr_tile_id = None;
    }

    fn scroll_cell_into_view(
        &mut self,
        ui: &mut TheUI,
        tab: usize,
        cell_pos: Vec2<i32>,
        cell_span: Vec2<i32>,
    ) {
        let Some(render_view) = ui.get_render_view(&format!("{TILE_VIEW_PREFIX}{tab}")) else {
            return;
        };
        let dim = *render_view.dim();
        let cell = ((TILE_CELL_BASE as f32) * self.zoom) as i32;
        let cell = cell.max(TILE_CELL_BASE);
        let margin = cell / 2;
        let x0 = cell_pos.x * cell;
        let y0 = cell_pos.y * cell;
        let x1 = x0 + cell_span.x.max(1) * cell;
        let y1 = y0 + cell_span.y.max(1) * cell;
        let view_w = dim.width.max(cell);
        let view_h = dim.height.max(cell);

        if x0 - margin < self.tab_offset[tab].x {
            self.tab_offset[tab].x = (x0 - margin).max(0);
        } else if x1 + margin > self.tab_offset[tab].x + view_w {
            self.tab_offset[tab].x = (x1 + margin - view_w).max(0);
        }
        if y0 - margin < self.tab_offset[tab].y {
            self.tab_offset[tab].y = (y0 - margin).max(0);
        } else if y1 + margin > self.tab_offset[tab].y + view_h {
            self.tab_offset[tab].y = (y1 + margin - view_h).max(0);
        }
    }

    fn auto_scroll_drag(&mut self, ui: &mut TheUI, tab: usize, view_name: &str, coord: Vec2<i32>) {
        let Some(render_view) = ui.get_render_view(view_name) else {
            return;
        };
        let dim = *render_view.dim();
        let margin = 36;
        let step = 28;

        if coord.x < margin {
            self.tab_offset[tab].x = (self.tab_offset[tab].x - step).max(0);
        } else if coord.x > dim.width - margin {
            self.tab_offset[tab].x += step;
        }
        if coord.y < margin {
            self.tab_offset[tab].y = (self.tab_offset[tab].y - step).max(0);
        } else if coord.y > dim.height - margin {
            self.tab_offset[tab].y += step;
        }
    }

    fn board_has_active_focus(&self, ctx: &TheContext) -> bool {
        ctx.ui
            .focus
            .as_ref()
            .is_some_and(|id| Self::tab_from_view_name(&id.name).is_some())
            || ctx
                .ui
                .hover
                .as_ref()
                .is_some_and(|id| Self::tab_from_view_name(&id.name).is_some())
    }

    fn delete_source(
        &mut self,
        project: &mut Project,
        source: TileSource,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let before = project.clone();
        let deleted_reserved_slot = self
            .board_position_for_tab(project, self.active_tab, source)
            .map(|pos| (self.active_tab, pos));
        match source {
            TileSource::SingleTile(tile_id) => {
                if let Some(pos) = project.tile_board_tiles.get(&tile_id).copied() {
                    project.reserve_tile_board_empty_slot(pos);
                }
                for collection in project.tile_collections.values_mut() {
                    if let Some(pos) = collection.tile_board_tiles.get(&tile_id).copied()
                        && !collection.tile_board_empty_slots.contains(&pos)
                    {
                        collection.tile_board_empty_slots.push(pos);
                    }
                }
                for group in project.tile_groups.values_mut() {
                    group.members.retain(|member| member.tile_id != tile_id);
                }
                project.remove_source_from_collections(TileSource::SingleTile(tile_id));
                project.tile_board_tiles.shift_remove(&tile_id);
                project.remove_tile(&tile_id);
            }
            TileSource::TileGroup(group_id) => {
                let Some(group) = project.tile_groups.get(&group_id).cloned() else {
                    return false;
                };
                if let Some(pos) = project.tile_board_groups.get(&group_id).copied() {
                    project.reserve_tile_board_empty_slot(pos);
                }
                for collection in project.tile_collections.values_mut() {
                    if let Some(pos) = collection.tile_board_groups.get(&group_id).copied()
                        && !collection.tile_board_empty_slots.contains(&pos)
                    {
                        collection.tile_board_empty_slots.push(pos);
                    }
                }
                for member in &group.members {
                    project.tile_board_tiles.shift_remove(&member.tile_id);
                    project.remove_tile(&member.tile_id);
                }
                project.remove_tile_group(&group_id);
                if self.entered_group == Some(group_id) {
                    self.entered_group = None;
                }
            }
            TileSource::TileGroupMember {
                group_id,
                member_index,
            } => {
                let Some(group) = project.tile_groups.get_mut(&group_id) else {
                    return false;
                };
                let Some(member) = group.members.get(member_index as usize).cloned() else {
                    return false;
                };
                group.members.retain(|m| m.tile_id != member.tile_id);
                project.remove_source_from_collections(TileSource::SingleTile(member.tile_id));
                project.tile_board_tiles.shift_remove(&member.tile_id);
                project.remove_tile(&member.tile_id);
            }
            TileSource::Procedural(_) => return false,
        }

        self.clear_selection(server_ctx);
        self.selected_reserved_slot = deleted_reserved_slot;
        self.tile_hover_source = None;
        self.tile_preview_mode = false;
        let after = project.clone();
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::TilePickerEdit(Box::new(before), Box::new(after)),
            ctx,
        );
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tilepicker"),
            TheValue::Empty,
        ));
        self.render_views(ui, ctx, project);
        true
    }

    fn delete_reserved_slot(
        &mut self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let Some((tab, pos)) = self.selected_reserved_slot else {
            return false;
        };
        if !self.is_reserved_slot(project, tab, pos) {
            self.selected_reserved_slot = None;
            return false;
        }

        let before = project.clone();
        self.clear_reserved_slot_for_tab(project, tab, pos);
        self.clear_selection(server_ctx);
        self.tile_hover_source = None;
        self.tile_preview_mode = false;
        let after = project.clone();
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::TilePickerEdit(Box::new(before), Box::new(after)),
            ctx,
        );
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tilepicker"),
            TheValue::Empty,
        ));
        self.render_views(ui, ctx, project);
        true
    }

    fn find_open_cell_for_span(&self, project: &Project, tab: usize, span: Vec2<i32>) -> Vec2<i32> {
        let board_cols = project
            .tile_board_cols
            .max(TILE_BOARD_COLS + TILE_BOARD_EXTRA_COLS);
        let pack_cols = (board_cols - TILE_BOARD_EXTRA_COLS).max(1);
        let cell = ((TILE_CELL_BASE as f32) * self.zoom).max(TILE_CELL_BASE as f32) as i32;
        let start = Vec2::new(
            (self.tab_offset[tab].x / cell).max(0),
            (self.tab_offset[tab].y / cell).max(0),
        );
        let mut occupied: FxHashSet<(i32, i32)> = FxHashSet::default();

        for placement in &self.placements[tab] {
            let cell_x = ((placement.rect.x + self.tab_offset[tab].x) / cell).max(0);
            let cell_y = ((placement.rect.y + self.tab_offset[tab].y) / cell).max(0);
            let span_x = (placement.rect.z / cell).max(1);
            let span_y = (placement.rect.w / cell).max(1);
            for dy in 0..span_y {
                for dx in 0..span_x {
                    occupied.insert((cell_x + dx, cell_y + dy));
                }
            }
        }
        for pos in self.empty_slots_for_tab(project, tab) {
            let reserved_x = pos.x.max(0);
            let reserved_y = pos.y.max(0);
            occupied.insert((reserved_x, reserved_y));
        }

        for y in start.y..(project.tile_board_rows.max(8) + 32) {
            for x in start.x.min((pack_cols - span.x).max(0))..=(pack_cols - span.x).max(0) {
                let mut fits = true;
                for dy in 0..span.y.max(1) {
                    for dx in 0..span.x.max(1) {
                        if occupied.contains(&(x + dx, y + dy)) {
                            fits = false;
                            break;
                        }
                    }
                    if !fits {
                        break;
                    }
                }
                if fits {
                    return Vec2::new(x, y);
                }
            }
        }

        start
    }

    fn crop_texture_rgba(
        src: &[u8],
        src_size: (usize, usize),
        crop: (usize, usize, usize, usize),
    ) -> Vec<u8> {
        let (src_w, _src_h) = src_size;
        let (crop_x, crop_y, crop_w, crop_h) = crop;
        let mut out = vec![0; crop_w * crop_h * 4];

        for y in 0..crop_h {
            let src_start = ((crop_y + y) * src_w + crop_x) * 4;
            let src_end = src_start + crop_w * 4;
            let dst_start = y * crop_w * 4;
            out[dst_start..dst_start + crop_w * 4].copy_from_slice(&src[src_start..src_end]);
        }

        out
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

    fn render_views(&mut self, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        self.sync_tabs(ui, ctx, project);
        let specs = self.tab_specs(project);
        self.ensure_tab_state(specs.len());
        for tab in 0..specs.len() {
            let Some(render_view) = ui.get_render_view(&format!("{TILE_VIEW_PREFIX}{tab}")) else {
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
        &mut self,
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        project: &Project,
        tab: usize,
    ) -> Vec<TileBoardPlacement> {
        let cell = ((TILE_CELL_BASE as f32) * self.zoom) as i32;
        let cell = cell.max(TILE_CELL_BASE);
        let width = buffer.dim().width.max(cell);
        let height = buffer.dim().height.max(cell);
        let board_cols = project
            .tile_board_cols
            .max(TILE_BOARD_COLS + TILE_BOARD_EXTRA_COLS);
        let pack_cols = (board_cols - TILE_BOARD_EXTRA_COLS).max(1);

        let entries = self.entries_for_tab(project, tab);
        let mut placements = self.layout_entries(
            &entries,
            pack_cols,
            board_cols,
            cell,
            self.empty_slots_for_tab(project, tab),
        );
        let board_width = board_cols * cell;
        let content_height = placements
            .iter()
            .map(|placement| placement.rect.y + placement.rect.w)
            .max()
            .unwrap_or(cell);
        let board_rows = project
            .tile_board_rows
            .max(((content_height + cell - 1) / cell).max(1) + TILE_BOARD_EXTRA_ROWS);
        let board_height = board_rows * cell;
        self.tab_offset[tab].x = self.tab_offset[tab].x.min((board_width - width).max(0));
        self.tab_offset[tab].y = self.tab_offset[tab].y.min((board_height - height).max(0));

        let offset = self.tab_offset[tab];
        let stride = buffer.stride();
        let total_rows = board_rows.max(1);
        let visible_rows = ((height + offset.y) / cell + 2).max(1);
        let mut occupied: FxHashSet<(i32, i32)> = FxHashSet::default();
        for pos in self.empty_slots_for_tab(project, tab) {
            let reserved_x = pos.x.max(0);
            let reserved_y = pos.y.max(0);
            occupied.insert((reserved_x, reserved_y));
        }
        for placement in &placements {
            let cell_x = placement.rect.x / cell;
            let cell_y = placement.rect.y / cell;
            let span_x = (placement.rect.z / cell).max(1);
            let span_y = (placement.rect.w / cell).max(1);
            for dy in 0..span_y {
                for dx in 0..span_x {
                    occupied.insert((cell_x + dx, cell_y + dy));
                }
            }
        }

        for row in 0..visible_rows.max(total_rows) {
            for col in 0..board_cols {
                let cell_pos = Vec2::new(col, row);
                let is_reserved = self.is_reserved_slot(project, tab, cell_pos);
                if occupied.contains(&(col, row)) && !is_reserved {
                    continue;
                }
                let x = col * cell - offset.x;
                let y = row * cell - offset.y;
                if x >= width || x + cell <= 0 || y >= height || y + cell <= 0 {
                    continue;
                }
                if let Some(rect) = Self::clip_rect(buffer, Vec4::new(x, y, cell, cell), 2) {
                    if is_reserved {
                        if Some((tab, cell_pos)) == self.selected_reserved_slot {
                            ctx.draw
                                .rect_outline(buffer.pixels_mut(), &rect, stride, &WHITE);
                        }
                    } else {
                        ctx.draw
                            .rect(buffer.pixels_mut(), &rect, stride, &[62, 62, 62, 255]);
                        ctx.draw.rect_outline(
                            buffer.pixels_mut(),
                            &rect,
                            stride,
                            &[74, 74, 74, 255],
                        );
                    }
                }
            }
        }

        for placement in &mut placements {
            let screen_rect = Vec4::new(
                placement.rect.x - offset.x,
                placement.rect.y - offset.y,
                placement.rect.z,
                placement.rect.w,
            );
            if screen_rect.x >= width
                || screen_rect.x + screen_rect.z <= 0
                || screen_rect.y >= height
                || screen_rect.y + screen_rect.w <= 0
            {
                placement.rect = screen_rect;
                continue;
            }

            self.draw_entry(buffer, ctx, project, tab, screen_rect, placement.source);
            placement.rect = screen_rect;
        }

        if tab == self.active_tab
            && let Some((preview_tab, preview_cell)) = self.drag_drop_cell
            && preview_tab == tab
            && let Some((_, preview_source, _, _)) = self.drag_item
        {
            let span = self.source_span(project, preview_source);
            let preview_rect = Vec4::new(
                preview_cell.x * cell - offset.x,
                preview_cell.y * cell - offset.y,
                span.x * cell,
                span.y * cell,
            );
            if let Some(outline_rect) = Self::clip_rect(buffer, preview_rect, 2) {
                ctx.draw.rect_outline(
                    buffer.pixels_mut(),
                    &outline_rect,
                    stride,
                    &[220, 220, 220, 255],
                );
            }
            self.draw_source_alpha(
                buffer,
                ctx,
                project,
                tab,
                preview_rect,
                preview_source,
                0.55,
            );
        }

        placements
    }

    fn entries_for_tab(&self, project: &Project, tab: usize) -> Vec<TileBoardEntry> {
        let tab_kind = self
            .tab_specs(project)
            .get(tab)
            .map(|spec| spec.kind)
            .unwrap_or(TileTabKind::Project);

        if matches!(tab_kind, TileTabKind::Treasury) {
            return self
                .treasury_packages
                .iter()
                .map(|package| TileBoardEntry {
                    source: TileSource::Procedural(package.id),
                    span: Vec2::new(1, 1),
                    pos: None,
                })
                .collect();
        }

        if let Some(entered_group) = self.entered_group {
            return project
                .tile_groups
                .get(&entered_group)
                .filter(|group| self.matches_group(project, group))
                .map(|group| {
                    vec![TileBoardEntry {
                        source: TileSource::TileGroup(entered_group),
                        span: self.group_grid_size(project, group),
                        pos: Some(Vec2::zero()),
                    }]
                })
                .unwrap_or_default();
        }

        let mut entries = Vec::new();
        let grouped_tile_ids = self.grouped_tile_ids(project);
        let in_tab = |source: TileSource| match tab_kind {
            TileTabKind::Project => true,
            TileTabKind::Collection(id) => project.collection_contains_source(&id, source),
            TileTabKind::Treasury => false,
        };

        if !matches!(tab_kind, TileTabKind::Treasury) {
            for (tile_id, tile) in &project.tiles {
                let source = TileSource::SingleTile(*tile_id);
                if !grouped_tile_ids.contains(tile_id) && self.matches_tile(tile) && in_tab(source)
                {
                    entries.push(TileBoardEntry {
                        source,
                        span: Vec2::new(1, 1),
                        pos: self.board_position_for_tab(project, tab, source),
                    });
                }
            }
            for (group_id, group) in &project.tile_groups {
                let source = TileSource::TileGroup(*group_id);
                if self.matches_group(project, group) && in_tab(source) {
                    entries.push(TileBoardEntry {
                        source,
                        span: self.group_grid_size(project, group),
                        pos: self.board_position_for_tab(project, tab, source),
                    });
                }
            }
        }

        entries
    }

    fn layout_entries(
        &self,
        entries: &[TileBoardEntry],
        pack_cols: i32,
        board_cols: i32,
        cell: i32,
        empty_slots: &[Vec2<i32>],
    ) -> Vec<TileBoardPlacement> {
        let mut placements = Vec::with_capacity(entries.len());
        let mut occupied: FxHashSet<(i32, i32)> = FxHashSet::default();
        for pos in empty_slots {
            occupied.insert((pos.x.max(0), pos.y.max(0)));
        }
        let mut ordered_entries = entries.to_vec();
        ordered_entries.sort_by_key(|entry| entry.pos.is_none());

        for entry in &ordered_entries {
            let span_x = entry.span.x.clamp(1, board_cols);
            let span_y = entry.span.y.max(1);
            let mut placed = None;
            if let Some(pos) = entry.pos {
                let x = pos.x.max(0);
                let y = pos.y.max(0);
                if x + span_x <= board_cols {
                    let mut fits = true;
                    for dy in 0..span_y {
                        for dx in 0..span_x {
                            if occupied.contains(&(x + dx, y + dy)) {
                                fits = false;
                                break;
                            }
                        }
                        if !fits {
                            break;
                        }
                    }
                    if fits {
                        for dy in 0..span_y {
                            for dx in 0..span_x {
                                occupied.insert((x + dx, y + dy));
                            }
                        }
                        placed = Some(Vec4::new(x * cell, y * cell, span_x * cell, span_y * cell));
                    }
                }
            }

            let mut y = 0;
            while placed.is_none() {
                for x in 0..=(pack_cols - span_x) {
                    let mut fits = true;
                    for dy in 0..span_y {
                        for dx in 0..span_x {
                            if occupied.contains(&(x + dx, y + dy)) {
                                fits = false;
                                break;
                            }
                        }
                        if !fits {
                            break;
                        }
                    }

                    if fits {
                        for dy in 0..span_y {
                            for dx in 0..span_x {
                                occupied.insert((x + dx, y + dy));
                            }
                        }
                        placed = Some(Vec4::new(x * cell, y * cell, span_x * cell, span_y * cell));
                        break;
                    }
                }
                y += 1;
            }

            if let Some(rect) = placed {
                placements.push(TileBoardPlacement {
                    source: entry.source,
                    rect,
                });
            }
        }

        placements
    }

    fn draw_entry(
        &self,
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        project: &Project,
        tab: usize,
        rect: Vec4<i32>,
        source: TileSource,
    ) {
        self.draw_source_alpha(buffer, ctx, project, tab, rect, source, 1.0);
    }

    fn draw_source_alpha(
        &self,
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        project: &Project,
        tab: usize,
        rect: Vec4<i32>,
        source: TileSource,
        alpha: f32,
    ) {
        let stride = buffer.stride();
        let fill = if Some(source) == self.tile_hover_source {
            [92, 92, 92, 255]
        } else {
            [74, 74, 74, 255]
        };
        let outline = if Some(source) == self.tile_hover_source {
            [210, 210, 210, 255]
        } else if Some(source) == self.current_source() {
            WHITE
        } else {
            [112, 112, 112, 255]
        };
        if matches!(source, TileSource::TileGroup(_))
            && let Some(outer) = Self::clip_rect(buffer, rect, 2)
        {
            let fill = if alpha >= 1.0 {
                fill
            } else {
                [
                    fill[0],
                    fill[1],
                    fill[2],
                    ((fill[3] as f32) * alpha).round().clamp(0.0, 255.0) as u8,
                ]
            };
            ctx.draw.rect(buffer.pixels_mut(), &outer, stride, &fill);
        }

        match source {
            TileSource::SingleTile(tile_id) => {
                if let Some(tile) = project.tiles.get(&tile_id) {
                    self.draw_tile_into_rect(buffer, ctx, tile, rect, 2);
                }
            }
            TileSource::TileGroup(group_id) => {
                if let Some(group) = project.tile_groups.get(&group_id) {
                    let grid = self.group_grid_size(project, group);
                    let grid_w = grid.x;
                    let grid_h = grid.y;
                    let cell_w = (rect.z / grid_w).max(1);
                    let cell_h = (rect.w / grid_h).max(1);
                    let mut occupied = FxHashSet::default();
                    for member in &group.members {
                        occupied.insert((member.x, member.y));
                    }
                    for y in 0..grid_h as u16 {
                        for x in 0..grid_w as u16 {
                            if occupied.contains(&(x, y)) {
                                continue;
                            }
                            if let Some(slot_rect) = Self::clip_rect(
                                buffer,
                                Vec4::new(
                                    rect.x + x as i32 * cell_w,
                                    rect.y + y as i32 * cell_h,
                                    cell_w,
                                    cell_h,
                                ),
                                3,
                            ) {
                                if project.is_tile_node_group(&group_id) {
                                    continue;
                                }
                                let is_drop_target = self
                                    .group_drop_cell(project, tab, group_id)
                                    .map(|(tx, ty)| tx == x && ty == y)
                                    .unwrap_or(false);
                                ctx.draw.rect(
                                    buffer.pixels_mut(),
                                    &slot_rect,
                                    stride,
                                    if is_drop_target {
                                        &[60, 60, 60, 255]
                                    } else {
                                        &[48, 48, 48, 255]
                                    },
                                );
                                ctx.draw.rect_outline(
                                    buffer.pixels_mut(),
                                    &slot_rect,
                                    stride,
                                    if is_drop_target {
                                        &[220, 220, 220, 255]
                                    } else {
                                        &[70, 70, 70, 255]
                                    },
                                );
                            }
                        }
                    }
                    for member in &group.members {
                        if let Some(tile) = project.tiles.get(&member.tile_id) {
                            let member_rect = Vec4::new(
                                rect.x + member.x as i32 * cell_w,
                                rect.y + member.y as i32 * cell_h,
                                cell_w,
                                cell_h,
                            );
                            self.draw_tile_into_rect_alpha(
                                buffer,
                                ctx,
                                tile,
                                member_rect,
                                2,
                                alpha,
                            );
                        }
                    }
                    for (index, member) in group.members.iter().enumerate() {
                        let member_source = TileSource::TileGroupMember {
                            group_id,
                            member_index: index as u16,
                        };
                        if Some(member_source) == self.current_source()
                            || Some(member_source) == self.tile_hover_source
                        {
                            let member_rect = Vec4::new(
                                rect.x + member.x as i32 * cell_w,
                                rect.y + member.y as i32 * cell_h,
                                cell_w,
                                cell_h,
                            );
                            if let Some(outer) = Self::clip_rect(buffer, member_rect, 2) {
                                let member_outline =
                                    if Some(member_source) == self.tile_hover_source {
                                        [210, 210, 210, 255]
                                    } else {
                                        WHITE
                                    };
                                ctx.draw.rect_outline(
                                    buffer.pixels_mut(),
                                    &outer,
                                    stride,
                                    &member_outline,
                                );
                            }
                        }
                    }
                }
            }
            TileSource::TileGroupMember {
                group_id,
                member_index,
            } => {
                if let Some(group) = project.tile_groups.get(&group_id)
                    && let Some(member) = group.members.get(member_index as usize)
                    && let Some(tile) = project.tiles.get(&member.tile_id)
                {
                    self.draw_tile_into_rect(buffer, ctx, tile, rect, 2);
                }
            }
            TileSource::Procedural(package_id) => {
                if let Some(outer) = Self::clip_rect(buffer, rect, 2) {
                    ctx.draw
                        .rect(buffer.pixels_mut(), &outer, stride, &[28, 28, 28, 255]);
                    ctx.draw.rect_outline(
                        buffer.pixels_mut(),
                        &outer,
                        stride,
                        if Some(source) == self.current_source() {
                            &WHITE
                        } else {
                            &[90, 120, 160, 255]
                        },
                    );
                    if let Some(package) = self
                        .treasury_packages
                        .iter()
                        .find(|package| package.id == package_id)
                    {
                        ctx.draw.text_rect_blend(
                            buffer.pixels_mut(),
                            &outer,
                            stride,
                            package.display_name().as_str(),
                            TheFontSettings {
                                size: 11.0,
                                ..Default::default()
                            },
                            &WHITE,
                            TheHorizontalAlign::Center,
                            TheVerticalAlign::Center,
                        );
                    }
                }
            }
        }

        if !matches!(source, TileSource::Procedural(_))
            && ((matches!(source, TileSource::TileGroup(_))
                && (Some(source) == self.current_source()
                    || Some(source) == self.tile_hover_source))
                || (matches!(source, TileSource::SingleTile(_))
                    && (Some(source) == self.current_source()
                        || Some(source) == self.tile_hover_source)))
        {
            let inset = if matches!(source, TileSource::TileGroup(_)) {
                2
            } else {
                1
            };
            if let Some(outer) = Self::clip_rect(buffer, rect, inset) {
                ctx.draw
                    .rect_outline(buffer.pixels_mut(), &outer, stride, &outline);
            }
        }
    }

    fn draw_tile_into_rect(
        &self,
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        tile: &rusterix::Tile,
        rect: Vec4<i32>,
        padding: i32,
    ) {
        self.draw_tile_into_rect_alpha(buffer, ctx, tile, rect, padding, 1.0);
    }

    fn draw_tile_into_rect_alpha(
        &self,
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        tile: &rusterix::Tile,
        rect: Vec4<i32>,
        padding: i32,
        alpha: f32,
    ) {
        if tile.textures.is_empty() {
            return;
        }
        if let Some(emitter) = &tile.particle_emitter {
            let full_rect = Vec4::new(
                rect.x + padding,
                rect.y + padding,
                (rect.z - padding * 2).max(1),
                (rect.w - padding * 2).max(1),
            );
            let Some(draw_rect) = Self::clip_rect(buffer, rect, padding) else {
                return;
            };
            let preview = crate::docks::particle_preview::render_particle_emitter_preview(
                emitter,
                full_rect.z.max(1),
                full_rect.w.max(1),
                0.68,
            );
            let stride = buffer.stride();
            ctx.draw.blend_scale_chunk_alpha(
                buffer.pixels_mut(),
                &draw_rect,
                stride,
                preview.pixels(),
                &(preview.dim().width as usize, preview.dim().height as usize),
                alpha,
            );
            return;
        }
        let tex = &tile.textures[0];
        let stride = buffer.stride();
        let full_rect = Vec4::new(
            rect.x + padding,
            rect.y + padding,
            (rect.z - padding * 2).max(1),
            (rect.w - padding * 2).max(1),
        );
        let Some(draw_rect) = Self::clip_rect(buffer, rect, padding) else {
            return;
        };
        let full_x0 = full_rect.x;
        let full_y0 = full_rect.y;

        let clip_x0 = draw_rect.0 as i32;
        let clip_y0 = draw_rect.1 as i32;
        let clip_x1 = clip_x0 + draw_rect.2 as i32;
        let clip_y1 = clip_y0 + draw_rect.3 as i32;

        let src_w = tex.width.max(1);
        let src_h = tex.height.max(1);

        let u0 = ((clip_x0 - full_x0) as f32 / full_rect.z as f32).clamp(0.0, 1.0);
        let v0 = ((clip_y0 - full_y0) as f32 / full_rect.w as f32).clamp(0.0, 1.0);
        let u1 = ((clip_x1 - full_x0) as f32 / full_rect.z as f32).clamp(0.0, 1.0);
        let v1 = ((clip_y1 - full_y0) as f32 / full_rect.w as f32).clamp(0.0, 1.0);

        let crop_x0 = ((u0 * src_w as f32).round() as usize).min(src_w.saturating_sub(1));
        let crop_y0 = ((v0 * src_h as f32).round() as usize).min(src_h.saturating_sub(1));
        let crop_x1 = ((u1 * src_w as f32).round() as usize).clamp(crop_x0 + 1, src_w);
        let crop_y1 = ((v1 * src_h as f32).round() as usize).clamp(crop_y0 + 1, src_h);
        let crop_w = crop_x1.saturating_sub(crop_x0).max(1);
        let crop_h = crop_y1.saturating_sub(crop_y0).max(1);
        let cropped = Self::crop_texture_rgba(
            &tex.data,
            (tex.width, tex.height),
            (crop_x0, crop_y0, crop_w, crop_h),
        );

        ctx.draw.blend_scale_chunk_alpha(
            buffer.pixels_mut(),
            &draw_rect,
            stride,
            &cropped,
            &(crop_w, crop_h),
            alpha,
        );
    }

    fn draw_source_preview(
        &self,
        buffer: &mut TheRGBABuffer,
        project: &Project,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
        source: TileSource,
    ) {
        buffer.fill(BLACK);
        let stride = buffer.stride();
        let particle_time = self.particle_preview_time();

        match source {
            TileSource::SingleTile(tile_id) => {
                if let Some(tile) = project.tiles.get(&tile_id) {
                    if let Some(emitter) = &tile.particle_emitter {
                        let preview =
                            crate::docks::particle_preview::render_particle_emitter_preview(
                                emitter,
                                buffer.dim().width.max(1),
                                buffer.dim().height.max(1),
                                particle_time,
                            );
                        buffer.pixels_mut().copy_from_slice(preview.pixels());
                    } else if !tile.textures.is_empty() {
                        let index = server_ctx.animation_counter % tile.textures.len();
                        let tex = &tile.textures[index];
                        let src_w = tex.width as f32;
                        let src_h = tex.height as f32;
                        let dst_w = buffer.dim().width as f32;
                        let dst_h = buffer.dim().height as f32;
                        let scale = (dst_w / src_w).min(dst_h / src_h);
                        let draw_w = src_w * scale;
                        let draw_h = src_h * scale;
                        let offset_x = ((dst_w - draw_w) * 0.5).round() as usize;
                        let offset_y = ((dst_h - draw_h) * 0.5).round() as usize;
                        ctx.draw.blend_scale_chunk(
                            buffer.pixels_mut(),
                            &(
                                offset_x,
                                offset_y,
                                draw_w.round() as usize,
                                draw_h.round() as usize,
                            ),
                            stride,
                            &tex.data,
                            &(tex.width, tex.height),
                        );
                    }
                }
            }
            TileSource::TileGroup(group_id) => {
                if let Some(group) = project.tile_groups.get(&group_id) {
                    let grid = self.group_grid_size(project, group);
                    let grid_w = grid.x;
                    let grid_h = grid.y;
                    let cell_w = (buffer.dim().width / grid_w).max(1);
                    let cell_h = (buffer.dim().height / grid_h).max(1);
                    for member in &group.members {
                        if let Some(tile) = project.tiles.get(&member.tile_id) {
                            let cell_rect = (
                                member.x as usize * cell_w as usize + 2,
                                member.y as usize * cell_h as usize + 2,
                                (cell_w - 4).max(1) as usize,
                                (cell_h - 4).max(1) as usize,
                            );
                            if let Some(emitter) = &tile.particle_emitter {
                                let preview =
                                    crate::docks::particle_preview::render_particle_emitter_preview(
                                        emitter,
                                        cell_rect.2.max(1) as i32,
                                        cell_rect.3.max(1) as i32,
                                        particle_time,
                                    );
                                ctx.draw.blend_scale_chunk(
                                    buffer.pixels_mut(),
                                    &cell_rect,
                                    stride,
                                    preview.pixels(),
                                    &(preview.dim().width as usize, preview.dim().height as usize),
                                );
                            } else if !tile.textures.is_empty() {
                                let index = server_ctx.animation_counter % tile.textures.len();
                                let tex = &tile.textures[index];
                                ctx.draw.blend_scale_chunk(
                                    buffer.pixels_mut(),
                                    &cell_rect,
                                    stride,
                                    &tex.data,
                                    &(tex.width, tex.height),
                                );
                            }
                        }
                    }
                }
            }
            TileSource::TileGroupMember {
                group_id,
                member_index,
            } => {
                self.draw_source_preview(
                    buffer,
                    project,
                    ctx,
                    server_ctx,
                    self.group_member_source(project, group_id, member_index),
                );
            }
            TileSource::Procedural(_) => {}
        }
    }

    fn group_member_source(
        &self,
        project: &Project,
        group_id: Uuid,
        member_index: u16,
    ) -> TileSource {
        if let Some(group) = project.tile_groups.get(&group_id)
            && let Some(member) = group.members.get(member_index as usize)
        {
            return TileSource::SingleTile(member.tile_id);
        }
        TileSource::TileGroup(group_id)
    }

    fn current_source(&self) -> Option<TileSource> {
        self.curr_source
    }

    fn grouped_tile_ids(&self, project: &Project) -> FxHashSet<Uuid> {
        let mut ids = FxHashSet::default();
        for group in project.tile_groups.values() {
            for member in &group.members {
                ids.insert(member.tile_id);
            }
        }
        ids
    }

    fn pick_top_level_source(&self, tab: usize, coord: Vec2<i32>) -> Option<TileSource> {
        self.placements[tab]
            .iter()
            .find(|placement| {
                coord.x >= placement.rect.x
                    && coord.x < placement.rect.x + placement.rect.z
                    && coord.y >= placement.rect.y
                    && coord.y < placement.rect.y + placement.rect.w
            })
            .map(|placement| placement.source)
    }

    fn overlapping_top_level_sources(
        &self,
        project: &Project,
        tab: usize,
        source: TileSource,
        target_cell: Vec2<i32>,
    ) -> Vec<TileSource> {
        let target_span = self.source_span(project, source);
        self.placements[tab]
            .iter()
            .filter_map(|placement| {
                if placement.source == source {
                    return None;
                }
                let other_cell = self.source_board_cell(project, tab, placement.source)?;
                let other_span = self.source_span(project, placement.source);
                let intersects = target_cell.x < other_cell.x + other_span.x
                    && target_cell.x + target_span.x > other_cell.x
                    && target_cell.y < other_cell.y + other_span.y
                    && target_cell.y + target_span.y > other_cell.y;
                intersects.then_some(placement.source)
            })
            .collect()
    }

    fn coord_to_board_cell(&self, tab: usize, coord: Vec2<i32>) -> Option<Vec2<i32>> {
        let cell = ((TILE_CELL_BASE as f32) * self.zoom) as i32;
        let cell = cell.max(TILE_CELL_BASE);
        if cell <= 0 {
            return None;
        }
        let board = coord + self.tab_offset[tab];
        Some(Vec2::new((board.x / cell).max(0), (board.y / cell).max(0)))
    }

    fn source_board_cell(
        &self,
        project: &Project,
        tab: usize,
        source: TileSource,
    ) -> Option<Vec2<i32>> {
        let cell = ((TILE_CELL_BASE as f32) * self.zoom) as i32;
        let cell = cell.max(TILE_CELL_BASE);
        self.placements[tab]
            .iter()
            .find(|placement| placement.source == source)
            .map(|placement| {
                Vec2::new(
                    ((placement.rect.x + self.tab_offset[tab].x) / cell).max(0),
                    ((placement.rect.y + self.tab_offset[tab].y) / cell).max(0),
                )
            })
            .or_else(|| self.board_position_for_tab(project, tab, source))
    }

    fn board_position_for_tab(
        &self,
        project: &Project,
        tab: usize,
        source: TileSource,
    ) -> Option<Vec2<i32>> {
        match self
            .tab_specs(project)
            .get(tab)
            .map(|spec| spec.kind)
            .unwrap_or(TileTabKind::Project)
        {
            TileTabKind::Collection(collection_id) => project
                .collection_tile_board_position(&collection_id, source)
                .or_else(|| project.tile_board_position(source)),
            _ => project.tile_board_position(source),
        }
    }

    fn empty_slots_for_tab<'a>(&self, project: &'a Project, tab: usize) -> &'a [Vec2<i32>] {
        match self
            .tab_specs(project)
            .get(tab)
            .map(|spec| spec.kind)
            .unwrap_or(TileTabKind::Project)
        {
            TileTabKind::Collection(collection_id) => project
                .collection_tile_board_empty_slots(&collection_id)
                .unwrap_or(&[]),
            _ => project.tile_board_empty_slots(),
        }
    }

    fn is_reserved_slot(&self, project: &Project, tab: usize, pos: Vec2<i32>) -> bool {
        self.empty_slots_for_tab(project, tab).contains(&pos)
    }

    fn clear_reserved_slot_for_tab(&self, project: &mut Project, tab: usize, pos: Vec2<i32>) {
        match self
            .tab_specs(project)
            .get(tab)
            .map(|spec| spec.kind)
            .unwrap_or(TileTabKind::Project)
        {
            TileTabKind::Collection(collection_id) => {
                if let Some(collection) = project.tile_collections.get_mut(&collection_id)
                    && let Some(index) = collection
                        .tile_board_empty_slots
                        .iter()
                        .position(|p| *p == pos)
                {
                    collection.tile_board_empty_slots.swap_remove(index);
                }
            }
            _ => {
                project.clear_tile_board_empty_slot(pos);
            }
        }
    }

    fn set_board_position_for_tab(
        &self,
        project: &mut Project,
        tab: usize,
        source: TileSource,
        pos: Vec2<i32>,
    ) {
        match self
            .tab_specs(project)
            .get(tab)
            .map(|spec| spec.kind)
            .unwrap_or(TileTabKind::Project)
        {
            TileTabKind::Collection(collection_id) => {
                project.set_collection_tile_board_position(&collection_id, source, pos);
            }
            _ => {
                project.set_tile_board_position(source, pos);
            }
        }
    }

    fn can_place_source_at(
        &self,
        project: &Project,
        tab: usize,
        source: TileSource,
        cell: Vec2<i32>,
        ignore: Option<TileSource>,
    ) -> bool {
        let span = self.source_span(project, source);
        if cell.x < 0 || cell.y < 0 {
            return false;
        }

        for placement in &self.placements[tab] {
            if placement.source == source || Some(placement.source) == ignore {
                continue;
            }
            let other_cell = self
                .source_board_cell(project, tab, placement.source)
                .unwrap_or(Vec2::zero());
            let other_span = self.source_span(project, placement.source);
            let intersects = cell.x < other_cell.x + other_span.x
                && cell.x + span.x > other_cell.x
                && cell.y < other_cell.y + other_span.y
                && cell.y + span.y > other_cell.y;
            if intersects {
                return false;
            }
        }

        true
    }

    fn try_move_with_displacement(
        &self,
        project: &mut Project,
        tab: usize,
        source: TileSource,
        start_cell: Vec2<i32>,
        target_cell: Vec2<i32>,
    ) -> bool {
        let displaced = self.overlapping_top_level_sources(project, tab, source, target_cell);
        if displaced.is_empty() {
            if self.can_place_source_at(project, tab, source, target_cell, None) {
                let target_span = self.source_span(project, source);
                project.ensure_tile_board_space(
                    target_cell + Vec2::new(target_span.x - 1, target_span.y - 1),
                );
                self.set_board_position_for_tab(project, tab, source, target_cell);
                return true;
            }
            return false;
        }

        let mut new_positions: Vec<(TileSource, Vec2<i32>, Vec2<i32>)> = Vec::new();
        for displaced_source in &displaced {
            let Some(old_cell) = self.source_board_cell(project, tab, *displaced_source) else {
                return false;
            };
            let offset = old_cell - target_cell;
            let new_cell = start_cell + offset;
            if new_cell.x < 0 || new_cell.y < 0 {
                return false;
            }
            new_positions.push((*displaced_source, old_cell, new_cell));
        }

        let source_span = self.source_span(project, source);
        for i in 0..new_positions.len() {
            let (src_a, _old_a, cell_a) = new_positions[i];
            let span_a = self.source_span(project, src_a);
            let intersects_source = cell_a.x < target_cell.x + source_span.x
                && cell_a.x + span_a.x > target_cell.x
                && cell_a.y < target_cell.y + source_span.y
                && cell_a.y + span_a.y > target_cell.y;
            if intersects_source {
                return false;
            }
            for j in (i + 1)..new_positions.len() {
                let (src_b, _old_b, cell_b) = new_positions[j];
                let span_b = self.source_span(project, src_b);
                let intersects = cell_a.x < cell_b.x + span_b.x
                    && cell_a.x + span_a.x > cell_b.x
                    && cell_a.y < cell_b.y + span_b.y
                    && cell_a.y + span_a.y > cell_b.y;
                if intersects {
                    return false;
                }
            }
        }

        for placement in &self.placements[tab] {
            if placement.source == source || displaced.contains(&placement.source) {
                continue;
            }
            let Some(other_cell) = self.source_board_cell(project, tab, placement.source) else {
                continue;
            };
            let other_span = self.source_span(project, placement.source);
            let source_intersects = target_cell.x < other_cell.x + other_span.x
                && target_cell.x + source_span.x > other_cell.x
                && target_cell.y < other_cell.y + other_span.y
                && target_cell.y + source_span.y > other_cell.y;
            if source_intersects {
                return false;
            }
            for (displaced_source, _old_cell, new_cell) in &new_positions {
                let displaced_span = self.source_span(project, *displaced_source);
                let intersects = new_cell.x < other_cell.x + other_span.x
                    && new_cell.x + displaced_span.x > other_cell.x
                    && new_cell.y < other_cell.y + other_span.y
                    && new_cell.y + displaced_span.y > other_cell.y;
                if intersects {
                    return false;
                }
            }
        }

        project
            .ensure_tile_board_space(target_cell + Vec2::new(source_span.x - 1, source_span.y - 1));
        self.set_board_position_for_tab(project, tab, source, target_cell);
        for (displaced_source, _old_cell, new_cell) in new_positions {
            let displaced_span = self.source_span(project, displaced_source);
            project.ensure_tile_board_space(
                new_cell + Vec2::new(displaced_span.x - 1, displaced_span.y - 1),
            );
            self.set_board_position_for_tab(project, tab, displaced_source, new_cell);
        }
        true
    }

    fn group_drop_cell(&self, project: &Project, tab: usize, group_id: Uuid) -> Option<(u16, u16)> {
        let Some((hover_tab, coord)) = self.drag_hover_coord else {
            return None;
        };
        if hover_tab != tab {
            return None;
        }
        if project.is_tile_node_group(&group_id) {
            return None;
        }
        let Some(group) = project.tile_groups.get(&group_id) else {
            return None;
        };
        let Some(placement) = self.placements[tab]
            .iter()
            .find(|placement| placement.source == TileSource::TileGroup(group_id))
        else {
            return None;
        };
        let grid = self.group_grid_size(project, group);
        let grid_w = grid.x;
        let grid_h = grid.y;
        let cell_w = (placement.rect.z / grid_w).max(1);
        let cell_h = (placement.rect.w / grid_h).max(1);
        let local_x = ((coord.x - placement.rect.x) / cell_w).clamp(0, grid_w - 1) as u16;
        let local_y = ((coord.y - placement.rect.y) / cell_h).clamp(0, grid_h - 1) as u16;
        Some((local_x, local_y))
    }

    fn pick_group_drop_target(
        &self,
        project: &Project,
        tab: usize,
        coord: Vec2<i32>,
        dragged_source: TileSource,
    ) -> Option<Uuid> {
        self.placements[tab]
            .iter()
            .find(|placement| {
                placement.source != dragged_source
                    && matches!(placement.source, TileSource::TileGroup(_))
                    && coord.x >= placement.rect.x
                    && coord.x < placement.rect.x + placement.rect.z
                    && coord.y >= placement.rect.y
                    && coord.y < placement.rect.y + placement.rect.w
            })
            .and_then(|placement| match placement.source {
                TileSource::TileGroup(group_id) => {
                    if project.is_tile_node_group(&group_id) {
                        None
                    } else {
                        project.tile_groups.get(&group_id).map(|_| group_id)
                    }
                }
                _ => None,
            })
    }

    fn try_drop_into_group(
        &self,
        project: &mut Project,
        tab: usize,
        coord: Vec2<i32>,
        source: TileSource,
        group_id: Uuid,
    ) -> bool {
        let TileSource::SingleTile(tile_id) = source else {
            return false;
        };
        if project.is_tile_node_group(&group_id) {
            return false;
        }
        let Some(group) = project.tile_groups.get_mut(&group_id) else {
            return false;
        };
        let Some(placement) = self.placements[tab]
            .iter()
            .find(|placement| placement.source == TileSource::TileGroup(group_id))
        else {
            return false;
        };
        let extra = if group.members.is_empty() { 0 } else { 1 };
        let grid = Vec2::new(
            group.width.max(1) as i32 + extra,
            group.height.max(1) as i32 + extra,
        );
        let grid_w = grid.x;
        let grid_h = grid.y;
        let cell_w = (placement.rect.z / grid_w).max(1);
        let cell_h = (placement.rect.w / grid_h).max(1);
        let local_x = ((coord.x - placement.rect.x) / cell_w).clamp(0, grid_w - 1);
        let local_y = ((coord.y - placement.rect.y) / cell_h).clamp(0, grid_h - 1);

        if group
            .members
            .iter()
            .any(|member| member.x as i32 == local_x && member.y as i32 == local_y)
        {
            return false;
        }

        group.members.push(rusterix::TileGroupMemberRef {
            tile_id,
            x: local_x as u16,
            y: local_y as u16,
        });
        if local_x >= group.width as i32 {
            group.width = local_x as u16 + 1;
        }
        if local_y >= group.height as i32 {
            group.height = local_y as u16 + 1;
        }
        project.tile_board_tiles.shift_remove(&tile_id);
        true
    }

    fn pick_source(&self, project: &Project, tab: usize, coord: Vec2<i32>) -> Option<TileSource> {
        self.placements[tab]
            .iter()
            .find(|placement| {
                coord.x >= placement.rect.x
                    && coord.x < placement.rect.x + placement.rect.z
                    && coord.y >= placement.rect.y
                    && coord.y < placement.rect.y + placement.rect.w
            })
            .map(|placement| match placement.source {
                TileSource::TileGroup(group_id) => {
                    if project.is_tile_node_group(&group_id) {
                        return placement.source;
                    }
                    if let Some(group) = project.tile_groups.get(&group_id) {
                        let cell_w = (placement.rect.z / group.width.max(1) as i32).max(1);
                        let cell_h = (placement.rect.w / group.height.max(1) as i32).max(1);
                        for (index, member) in group.members.iter().enumerate() {
                            let member_rect = Vec4::new(
                                placement.rect.x + member.x as i32 * cell_w,
                                placement.rect.y + member.y as i32 * cell_h,
                                cell_w,
                                cell_h,
                            );
                            if coord.x >= member_rect.x + 2
                                && coord.x < member_rect.x + member_rect.z - 2
                                && coord.y >= member_rect.y + 2
                                && coord.y < member_rect.y + member_rect.w - 2
                            {
                                return TileSource::TileGroupMember {
                                    group_id,
                                    member_index: index as u16,
                                };
                            }
                        }
                    }
                    placement.source
                }
                _ => placement.source,
            })
    }

    fn select_source(
        &mut self,
        project: &Project,
        source: TileSource,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        self.selected_reserved_slot = None;
        server_ctx.curr_tile_source = Some(source);
        self.curr_source = Some(source);
        match source {
            TileSource::SingleTile(tile_id) => {
                server_ctx.curr_tile_id = Some(tile_id);
                self.curr_tile = Some(tile_id);
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Tile Picked"),
                    TheValue::Id(tile_id),
                ));
            }
            TileSource::TileGroupMember {
                group_id,
                member_index,
            } => {
                if let Some(group) = project.tile_groups.get(&group_id)
                    && let Some(member) = group.members.get(member_index as usize)
                {
                    server_ctx.curr_tile_id = Some(member.tile_id);
                    self.curr_tile = Some(member.tile_id);
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Tile Picked"),
                        TheValue::Id(member.tile_id),
                    ));
                } else {
                    server_ctx.curr_tile_id = None;
                    self.curr_tile = None;
                }
            }
            TileSource::TileGroup(group_id) => {
                let representative = project
                    .tile_groups
                    .get(&group_id)
                    .and_then(|group| group.members.first())
                    .map(|member| member.tile_id);
                server_ctx.curr_tile_id = representative;
                self.curr_tile = representative;
                if let Some(tile_id) = representative {
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Tile Picked"),
                        TheValue::Id(tile_id),
                    ));
                }
            }
            TileSource::Procedural(_) => {
                server_ctx.curr_tile_id = None;
                self.curr_tile = None;
            }
        }
        Self::activate_edit_tile_meta_action(server_ctx);
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Action List"),
            TheValue::Empty,
        ));
        self.sync_collection_menu(_ui, project);
        self.sync_sidebar(ctx, project);
    }

    fn select_reserved_slot(&mut self, tab: usize, pos: Vec2<i32>, server_ctx: &mut ServerContext) {
        self.clear_selection(server_ctx);
        self.selected_reserved_slot = Some((tab, pos));
    }

    fn status_text_for_source(&self, project: &Project, source: TileSource) -> String {
        match source {
            TileSource::SingleTile(tile_id) => {
                if let Some(tile) = project.tiles.get(&tile_id) {
                    if tile.alias.is_empty() {
                        format!(
                            "{}, Blocking: {}",
                            tile.role.to_string(),
                            if tile.blocking { "Yes" } else { "No" },
                        )
                    } else {
                        format!(
                            "{}, Blocking: {}, Alias: \"{}\"",
                            tile.role.to_string(),
                            if tile.blocking { "Yes" } else { "No" },
                            tile.alias
                        )
                    }
                } else {
                    "Tile".to_string()
                }
            }
            TileSource::TileGroup(group_id) => {
                if let Some(group) = project.tile_groups.get(&group_id) {
                    if self.entered_group == Some(group_id) {
                        return format!(
                            "{} {}x{}, {} members. Press Esc to leave group.",
                            if group.name.is_empty() {
                                "Tile Group"
                            } else {
                                group.name.as_str()
                            },
                            group.width,
                            group.height,
                            group.members.len()
                        );
                    }
                    let name = if group.name.is_empty() {
                        "Tile Group"
                    } else {
                        group.name.as_str()
                    };
                    format!(
                        "{} {}x{}, {} members. Double-click or press Return to enter.",
                        name,
                        group.width,
                        group.height,
                        group.members.len()
                    )
                } else {
                    "Tile Group".to_string()
                }
            }
            TileSource::TileGroupMember {
                group_id,
                member_index,
            } => {
                format!("Group Member {} in {}", member_index, group_id)
            }
            TileSource::Procedural(id) => {
                if let Some(package) = self.treasury_packages.iter().find(|pkg| pkg.id == id) {
                    let mut parts = vec![package.display_name()];
                    if !package.author.is_empty() {
                        parts.push(format!("by {}", package.author));
                    }
                    if !package.version.is_empty() {
                        parts.push(format!("v{}", package.version));
                    }
                    if !package.description.is_empty() {
                        parts.push(package.description.clone());
                    }
                    parts.push("Press Apply to install.".to_string());
                    parts.join(" - ")
                } else if let Some(err) = &self.treasury_error {
                    format!("Treasury unavailable: {err}")
                } else {
                    format!("Treasury Package {}", id)
                }
            }
        }
    }

    fn matches_tile(&self, tile: &rusterix::Tile) -> bool {
        tile.alias.to_lowercase().contains(&self.filter)
            && (self.filter_role == 0 || tile.role == TileRole::from_index(self.filter_role - 1))
    }

    fn matches_group(&self, project: &Project, group: &rusterix::TileGroup) -> bool {
        let filter_ok = self.filter.is_empty()
            || group.name.to_lowercase().contains(&self.filter)
            || group.tags.to_lowercase().contains(&self.filter)
            || group.members.iter().any(|member| {
                project
                    .tiles
                    .get(&member.tile_id)
                    .map(|tile| tile.alias.to_lowercase().contains(&self.filter))
                    .unwrap_or(false)
            });

        let role_ok = self.filter_role == 0
            || group.members.iter().any(|member| {
                project
                    .tiles
                    .get(&member.tile_id)
                    .map(|tile| tile.role == TileRole::from_index(self.filter_role - 1))
                    .unwrap_or(false)
            });

        filter_ok && role_ok
    }

    fn tab_from_view_name(name: &str) -> Option<usize> {
        name.strip_prefix(TILE_VIEW_PREFIX)
            .and_then(|index| index.parse::<usize>().ok())
    }
}
