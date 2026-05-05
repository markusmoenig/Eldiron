use crate::editor::UNDOMANAGER;
use crate::prelude::*;
use rusterix::Surface;
use scenevm::GeoId;
use shared::buildergraph::{
    BuilderCutMask, BuilderCutMode, BuilderDocument, BuilderScriptParameterValue,
};
use std::{
    collections::HashMap,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::{
        Mutex,
        mpsc::{Receiver, channel},
    },
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

const BUILDER_TAB_LAYOUT: &str = "Builder Dock Tabs";
const BUILDER_VIEW_PREFIX: &str = "Builder Dock View ";
const BUILDER_DOCK_REFRESH: &str = "Builder Dock Refresh";
const BUILDER_PARAMS_TOML: &str = "Builder Parameters TOML";
const BUILDER_AUTO_VERTEX_BUTTON: &str = "Builder Dock Auto Vertex";
const BUILDER_PARAMS_WIDTH: i32 = 300;
const BUILDER_CARD_BASE_W: i32 = 240;
const BUILDER_CARD_BASE_H: i32 = 178;
const BUILDER_CARD_GAP: i32 = 12;
const BUILDER_PADDING: i32 = 12;

#[derive(Clone, Copy, PartialEq, Eq)]
enum BuilderTabKind {
    Project,
    Treasury,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum BuilderBuiltinKind {
    Grass,
    Bush,
    Tree,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum BuilderCardKind {
    Builtin(BuilderBuiltinKind),
    Asset(Uuid),
    Treasury(usize),
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

struct BuilderTreasuryItem {
    id: Uuid,
    path: String,
    aliases: String,
    description: String,
    target: String,
    graph_name: String,
    graph_data: String,
}

impl BuilderTreasuryItem {
    fn as_asset(&self) -> BuilderGraphAsset {
        BuilderGraphAsset {
            id: self.id,
            graph_id: self.id,
            graph_name: self.graph_name.clone(),
            graph_data: self.graph_data.clone(),
        }
    }
}

pub struct BuilderDock {
    active_tab: usize,
    selected: Option<Uuid>,
    selected_builtin: Option<BuilderBuiltinKind>,
    builtin_sources: HashMap<(BuilderBuiltinKind, bool), String>,
    selected_treasury: Option<usize>,
    hovered: Option<BuilderCardKind>,
    placements: Vec<Vec<BuilderCardPlacement>>,
    tab_offset: Vec<Vec2<i32>>,
    zoom: f32,
    filter: String,
    preview_cache: HashMap<BuilderCardKind, (u64, TheRGBABuffer)>,
    last_asset_click: Option<(Uuid, u128)>,
    treasury_items: Vec<BuilderTreasuryItem>,
    treasury_error: Option<String>,
    treasury_loaded: bool,
    treasury_loading: bool,
    treasury_load_rx: Option<Mutex<Receiver<Result<Vec<BuilderTreasuryItem>, String>>>>,
}

fn builder_document_hides_host(document: &BuilderDocument) -> bool {
    document
        .evaluate()
        .map(|assembly| {
            assembly.cuts.iter().any(|cut| {
                matches!(
                    cut,
                    BuilderCutMask::Rect {
                        mode: BuilderCutMode::Replace,
                        ..
                    } | BuilderCutMask::Loop {
                        mode: BuilderCutMode::Replace,
                        ..
                    }
                )
            })
        })
        .unwrap_or(false)
}

impl Dock for BuilderDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            active_tab: 0,
            selected: None,
            selected_builtin: None,
            builtin_sources: HashMap::new(),
            selected_treasury: None,
            hovered: None,
            placements: vec![Vec::new(), Vec::new()],
            tab_offset: vec![Vec2::zero(), Vec2::zero()],
            zoom: 1.0,
            filter: String::new(),
            preview_cache: HashMap::new(),
            last_asset_click: None,
            treasury_items: Vec::new(),
            treasury_error: None,
            treasury_loaded: false,
            treasury_loading: false,
            treasury_load_rx: None,
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

        let mut filter_edit = TheTextLineEdit::new(TheId::named("Builder Dock Filter Edit"));
        filter_edit.set_text(String::new());
        filter_edit.limiter_mut().set_min_width(220);
        filter_edit.limiter_mut().set_max_height(18);
        filter_edit.set_font_size(12.5);
        filter_edit.set_status_text("Filter Builder Graphs by name, path, or alias.");
        filter_edit.set_continuous(true);
        toolbar_hlayout.add_widget(Box::new(filter_edit));

        let spacer = TheSpacer::new(TheId::empty());
        toolbar_hlayout.add_widget(Box::new(spacer));

        let mut new_button = TheTraybarButton::new(TheId::named("Builder Dock New"));
        new_button.set_text(fl!("new"));
        new_button.set_status_text(&fl!("status_builder_new"));
        new_button.set_context_menu(Some(TheContextMenu {
            items: vec![TheContextMenuItem::new(
                "Empty".to_string(),
                TheId::named("Builder Dock New Empty"),
            )],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(new_button));

        let mut install_button = TheTraybarButton::new(TheId::named("Builder Dock Install"));
        install_button.set_text("Install".to_string());
        install_button
            .set_status_text("Install the selected Treasury Builder Graph into this project.");
        toolbar_hlayout.add_widget(Box::new(install_button));

        let mut auto_vertex_button =
            TheTraybarButton::new(TheId::named(BUILDER_AUTO_VERTEX_BUTTON));
        auto_vertex_button.set_text("Auto Vertex".to_string());
        auto_vertex_button.set_is_toggle(true);
        auto_vertex_button.set_status_text(
            "When active, clicking in 3D creates a vertex at the hit point and applies the selected vertex Builder Graph.",
        );
        toolbar_hlayout.add_widget(Box::new(auto_vertex_button));

        let mut apply_button = TheTraybarButton::new(TheId::named("Builder Dock Apply Build"));
        apply_button.set_text(fl!("builder_apply_build"));
        apply_button.set_status_text(&fl!("status_builder_apply_build"));
        toolbar_hlayout.add_widget(Box::new(apply_button));

        let mut clear_button = TheTraybarButton::new(TheId::named("Builder Dock Clear Build"));
        clear_button.set_text(fl!("clear"));
        clear_button.set_status_text(&fl!("status_builder_clear_build"));
        toolbar_hlayout.add_widget(Box::new(clear_button));

        toolbar_hlayout.set_reverse_index(Some(3));

        toolbar_canvas.set_layout(toolbar_hlayout);
        canvas.set_top(toolbar_canvas);

        let mut center = TheCanvas::new();

        let mut tab_layout = TheTabLayout::new(TheId::named(BUILDER_TAB_LAYOUT));
        for tab in 0..2 {
            let mut tab_canvas = TheCanvas::new();
            tab_canvas.set_widget(TheRenderView::new(TheId::named(&format!(
                "{BUILDER_VIEW_PREFIX}{tab}"
            ))));
            let label = match tab {
                0 => "Project",
                _ => "Treasury",
            };
            tab_layout.add_canvas(label.to_string(), tab_canvas);
        }
        center.set_layout(tab_layout);

        let mut params_canvas = TheCanvas::new();
        let mut params_edit = TheTextAreaEdit::new(TheId::named(BUILDER_PARAMS_TOML));
        if let Some(bytes) = crate::Embedded::get("parser/TOML.sublime-syntax")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            params_edit.add_syntax_from_string(source);
            params_edit.set_code_type("TOML");
        }
        if let Some(bytes) = crate::Embedded::get("parser/gruvbox-dark.tmTheme")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            params_edit.add_theme_from_string(source);
            params_edit.set_code_theme("Gruvbox Dark");
        }
        params_edit.set_continuous(true);
        params_edit.display_line_number(false);
        params_edit.use_global_statusbar(true);
        params_edit.set_font_size(13.0);
        params_edit
            .limiter_mut()
            .set_min_width(BUILDER_PARAMS_WIDTH);
        params_edit
            .limiter_mut()
            .set_max_width(BUILDER_PARAMS_WIDTH);
        params_canvas.set_widget(params_edit);
        center.set_right(params_canvas);

        canvas.set_center(center);

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
        self.selected_treasury = None;
        ctx.ui.relayout = true;
        self.render_views(ui, ctx, project);
        self.sync_params_ui(ui, project, server_ctx);
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
        redraw |= self.poll_treasury_loader(ui, ctx, project);

        match event {
            TheEvent::Resize => {
                self.render_views(ui, ctx, project);
                redraw = true;
            }
            TheEvent::IndexChanged(id, index)
                if id.name == format!("{BUILDER_TAB_LAYOUT} Tabbar") =>
            {
                self.active_tab = *index;
                if matches!(Self::tab_kind(*index), BuilderTabKind::Treasury) {
                    self.ensure_treasury_loaded(ctx);
                }
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
                            self.selected_builtin = None;
                            self.selected_treasury = None;
                            server_ctx.curr_builder_graph_id = Some(asset_id);
                            self.sync_current_builder_context(project, server_ctx);
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Builder Selection Changed"),
                                TheValue::Id(asset_id),
                            ));
                            self.sync_params_ui(ui, project, server_ctx);
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
                        BuilderCardKind::Builtin(kind) => {
                            self.selected = None;
                            self.selected_builtin = Some(kind);
                            self.selected_treasury = None;
                            server_ctx.curr_builder_graph_id = None;
                            self.sync_current_builder_context(project, server_ctx);
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Builder Selection Changed"),
                                TheValue::Empty,
                            ));
                        }
                        BuilderCardKind::Treasury(index) => {
                            self.selected = None;
                            self.selected_builtin = None;
                            self.selected_treasury = Some(index);
                            server_ctx.curr_builder_graph_id = None;
                            self.sync_current_builder_context(project, server_ctx);
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Builder Selection Changed"),
                                TheValue::Empty,
                            ));
                        }
                    }
                    self.render_views(ui, ctx, project);
                    self.sync_params_ui(ui, project, server_ctx);
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
                            server_ctx.curr_builder_graph_name = None;
                            server_ctx.curr_builder_graph_data = None;
                        }
                        self.selected = None;
                        self.selected_treasury = None;
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
                    self.hovered = self.pick_asset(tab, *coord);
                    match self.hovered {
                        Some(BuilderCardKind::Asset(asset_id)) => {
                            if let Some(asset) = project.builder_graphs.get(&asset_id) {
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
                        }
                        Some(BuilderCardKind::Builtin(kind)) => {
                            let asset = Self::builtin_asset(kind, project, server_ctx);
                            ctx.ui.send(TheEvent::SetStatusText(
                                id.clone(),
                                format!(
                                    "{}",
                                    fl!(
                                        "status_builder_select_asset",
                                        asset_name = asset.graph_name
                                    )
                                ),
                            ));
                        }
                        Some(BuilderCardKind::Treasury(index)) => {
                            if let Some(item) = self.treasury_items.get(index) {
                                ctx.ui.send(TheEvent::SetStatusText(
                                    id.clone(),
                                    format!(
                                        "Treasury Builder Graph: {} ({})",
                                        item.graph_name, item.path
                                    ),
                                ));
                            }
                        }
                        None => {}
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
            TheEvent::RenderViewScrollBy(id, delta) => {
                if let Some(tab) = Self::tab_from_view_name(&id.name) {
                    if ui.ctrl || ui.logo {
                        self.zoom = (self.zoom + (delta.y as f32) * 0.05).clamp(0.75, 2.0);
                    } else if let Some(offset) = self.tab_offset.get_mut(tab) {
                        offset.x = (offset.x + delta.x).max(0);
                        offset.y = (offset.y + delta.y).max(0);
                    }
                    self.render_views(ui, ctx, project);
                    if let Some(render_view) = ui.get_render_view(&id.name) {
                        render_view.set_needs_redraw(true);
                    }
                    ctx.ui.redraw_all = true;
                    redraw = true;
                }
            }
            TheEvent::ContextMenuSelected(id, item) if id.name == "Builder Dock New" => {
                let asset = match item.name.as_str() {
                    "Builder Dock New Empty" => {
                        BuilderGraphAsset::new_empty(Self::next_builder_name(project, "Empty"))
                    }
                    _ => return false,
                };
                let asset_id = asset.id;
                project.add_builder_graph(asset);
                self.selected = Some(asset_id);
                self.selected_builtin = None;
                server_ctx.curr_builder_graph_id = Some(asset_id);
                self.sync_current_builder_context(project, server_ctx);
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Builder Selection Changed"),
                    TheValue::Id(asset_id),
                ));
                self.render_views(ui, ctx, project);
                self.sync_params_ui(ui, project, server_ctx);
                redraw = true;
            }
            TheEvent::ValueChanged(id, TheValue::Text(text))
                if id.name == "Builder Dock Filter Edit" =>
            {
                self.filter = text.to_lowercase();
                self.render_views(ui, ctx, project);
                redraw = true;
            }
            TheEvent::ValueChanged(id, TheValue::Text(text)) if id.name == BUILDER_PARAMS_TOML => {
                let selected_treasury = self.selected_treasury;
                let selected_project = self.selected.or(server_ctx.curr_builder_graph_id);
                let selected_builtin = self.selected_builtin;
                let source = if let Some(kind) = selected_builtin {
                    self.builtin_source(kind, project, server_ctx)
                } else if let Some(index) = selected_treasury {
                    let Some(item) = self.treasury_items.get(index) else {
                        return false;
                    };
                    item.graph_data.clone()
                } else if let Some(builder_id) = selected_project {
                    let Some(asset) = project.builder_graphs.get(&builder_id) else {
                        return false;
                    };
                    asset.graph_data.clone()
                } else {
                    return false;
                };
                let mut nodeui = Self::params_nodeui_for_source(&source);
                if !nodeui.is_empty()
                    && toml::from_str::<toml::Value>(text).is_ok()
                    && apply_toml_to_nodeui(&mut nodeui, text).is_ok()
                {
                    let values = Self::param_replacement_values(&nodeui);
                    let updated = Self::replace_param_value_lines(&source, &values);
                    if updated != source {
                        let graph_name = BuilderDocument::from_text(&updated)
                            .map(|document| document.name().to_string())
                            .ok()
                            .or_else(|| {
                                selected_project.and_then(|builder_id| {
                                    project
                                        .builder_graphs
                                        .get(&builder_id)
                                        .map(|asset| asset.graph_name.clone())
                                })
                            })
                            .or_else(|| {
                                selected_treasury.and_then(|index| {
                                    self.treasury_items
                                        .get(index)
                                        .map(|item| item.graph_name.clone())
                                })
                            })
                            .unwrap_or_else(|| "Builder Script".to_string());
                        if let Some(kind) = selected_builtin {
                            let use_vertex = Self::builtin_uses_vertex(project, server_ctx);
                            self.builtin_sources
                                .insert((kind, use_vertex), updated.clone());
                            self.sync_current_builder_context(project, server_ctx);
                        } else if let Some(builder_id) = selected_project
                            && let Some(asset) = project.builder_graphs.get_mut(&builder_id)
                        {
                            asset.graph_name = graph_name.clone();
                            asset.graph_data = updated.clone();
                            self.sync_current_builder_context(project, server_ctx);
                            Self::update_applied_hosts(
                                project,
                                server_ctx,
                                builder_id,
                                &graph_name,
                                &updated,
                            );
                            crate::utils::editor_scene_full_rebuild(project, server_ctx);
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Builder Graph Updated"),
                                TheValue::Id(builder_id),
                            ));
                        } else if let Some(index) = selected_treasury
                            && let Some(item) = self.treasury_items.get_mut(index)
                        {
                            let item_id = item.id;
                            item.graph_name = graph_name.clone();
                            item.graph_data = updated.clone();
                            self.sync_current_builder_context(project, server_ctx);
                            Self::update_applied_hosts(
                                project,
                                server_ctx,
                                item_id,
                                &graph_name,
                                &updated,
                            );
                            crate::utils::editor_scene_full_rebuild(project, server_ctx);
                        }
                        self.render_views(ui, ctx, project);
                        redraw = true;
                    }
                }
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "Builder Dock Install" =>
            {
                if let Some(index) = self.selected_treasury
                    && let Some(item) = self.treasury_items.get(index)
                {
                    let mut asset = item.as_asset();
                    asset.id = Uuid::new_v4();
                    asset.graph_id = Uuid::new_v4();
                    let asset_id = asset.id;
                    project.add_builder_graph(asset);
                    self.selected = Some(asset_id);
                    self.selected_builtin = None;
                    self.selected_treasury = None;
                    server_ctx.curr_builder_graph_id = Some(asset_id);
                    self.sync_current_builder_context(project, server_ctx);
                    self.render_views(ui, ctx, project);
                    self.sync_params_ui(ui, project, server_ctx);
                    let selection_value = if self.selected_builtin.is_some() {
                        TheValue::Empty
                    } else {
                        TheValue::Id(asset_id)
                    };
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Builder Selection Changed"),
                        selection_value,
                    ));
                    redraw = true;
                }
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "Builder Dock Apply Build" =>
            {
                eprintln!(
                    "[BuilderGraphDebug][click] Apply Build selected={:?} treasury={:?} curr={:?} hud_slot={} map_tool={:?}",
                    self.selected,
                    self.selected_treasury,
                    server_ctx.curr_builder_graph_id,
                    server_ctx.selected_hud_icon_index,
                    server_ctx.curr_map_tool_type
                );
                if let Some(asset) = self.selected_builder_asset(project, server_ctx) {
                    let asset_id = asset.id;
                    let mut applied_to_item_slot = false;
                    if let Some(map) = project.get_map_mut(server_ctx) {
                        applied_to_item_slot = crate::actions::apply_builder_item_to_selection(
                            map,
                            server_ctx,
                            server_ctx.selected_hud_icon_index,
                            &asset,
                        );
                        eprintln!(
                            "[BuilderGraphDebug][click] item-slot apply attempted result={} selected sectors={} linedefs={} vertices={}",
                            applied_to_item_slot,
                            map.selected_sectors.len(),
                            map.selected_linedefs.len(),
                            map.selected_vertices.len()
                        );
                    }
                    if !applied_to_item_slot {
                        eprintln!(
                            "[BuilderGraphDebug][click] activating asset={asset_id} on host selection"
                        );
                        self.activate_builder_asset(&asset, ui, ctx, project, server_ctx);
                    } else {
                        eprintln!(
                            "[BuilderGraphDebug][click] consumed by item-slot apply; host activation skipped"
                        );
                    }
                    crate::utils::editor_scene_full_rebuild(project, server_ctx);
                    self.sync_current_builder_context(project, server_ctx);
                    let selection_value = if self.selected_builtin.is_some() {
                        TheValue::Empty
                    } else {
                        TheValue::Id(asset_id)
                    };
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Builder Selection Changed"),
                        selection_value,
                    ));
                    self.render_views(ui, ctx, project);
                    self.sync_params_ui(ui, project, server_ctx);
                    redraw = true;
                } else {
                    eprintln!(
                        "[BuilderGraphDebug][click] Apply Build ignored: no selected builder graph"
                    );
                }
            }
            TheEvent::StateChanged(id, state) if id.name == BUILDER_AUTO_VERTEX_BUTTON => {
                server_ctx.builder_auto_vertex_mode = *state == TheWidgetState::Selected;
                self.sync_current_builder_context(project, server_ctx);
                self.sync_auto_vertex_button(ui, server_ctx);
                self.sync_params_ui(ui, project, server_ctx);
                self.render_views(ui, ctx, project);
                redraw = true;
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
                crate::utils::editor_scene_full_rebuild(project, server_ctx);
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
                    self.selected_builtin = None;
                    self.selected_treasury = None;
                    server_ctx.curr_builder_graph_id = Some(*builder_id);
                    self.sync_current_builder_context(project, server_ctx);
                } else {
                    if self.selected_builtin.is_none() {
                        self.selected = server_ctx.curr_builder_graph_id;
                    }
                    if self.selected.is_some() && self.selected_builtin.is_none() {
                        self.selected_treasury = None;
                    }
                    self.sync_current_builder_context(project, server_ctx);
                }
                self.render_views(ui, ctx, project);
                self.sync_params_ui(ui, project, server_ctx);
                self.sync_auto_vertex_button(ui, server_ctx);
                redraw = true;
            }
            TheEvent::Custom(id, _) if id.name == BUILDER_DOCK_REFRESH => {
                self.poll_treasury_loader(ui, ctx, project);
                ctx.ui.relayout = true;
                self.render_views(ui, ctx, project);
                self.sync_params_ui(ui, project, server_ctx);
                self.sync_auto_vertex_button(ui, server_ctx);
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
    fn sync_auto_vertex_button(&self, ui: &mut TheUI, server_ctx: &ServerContext) {
        if let Some(widget) = ui.get_widget(BUILDER_AUTO_VERTEX_BUTTON) {
            widget.set_state(if server_ctx.builder_auto_vertex_mode {
                TheWidgetState::Selected
            } else {
                TheWidgetState::None
            });
            widget.set_value(TheValue::Text("Auto Vertex".to_string()));
        }
    }

    fn sync_current_builder_context(&self, project: &Project, server_ctx: &mut ServerContext) {
        let asset = self.selected_builder_asset(project, server_ctx);
        server_ctx.curr_builder_graph_name = asset.as_ref().map(|asset| asset.graph_name.clone());
        server_ctx.curr_builder_graph_data = asset.as_ref().map(|asset| asset.graph_data.clone());
    }

    fn param_replacement_values(nodeui: &TheNodeUI) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for (_, item) in nodeui.list_items() {
            match item {
                TheNodeUIItem::FloatEditSlider(id, _, _, value, _, _)
                | TheNodeUIItem::FloatSlider(id, _, _, value, _, _, _) => {
                    out.push((id.clone(), format!("{value:.4}")));
                }
                TheNodeUIItem::IntEditSlider(id, _, _, value, _, _)
                | TheNodeUIItem::IntSlider(id, _, _, value, _, _, _) => {
                    out.push((id.clone(), value.to_string()));
                }
                TheNodeUIItem::Selector(id, _, _, values, index) => {
                    if let Some(value) = values.get((*index).max(0) as usize) {
                        out.push((id.clone(), value.clone()));
                    }
                }
                TheNodeUIItem::Text(id, _, _, value, _, _) => {
                    out.push((id.clone(), value.clone()));
                }
                _ => {}
            }
        }
        out
    }

    fn params_nodeui_for_source(source: &str) -> TheNodeUI {
        let mut nodeui = TheNodeUI::default();
        if let Ok(document) = BuilderDocument::from_text(source)
            && let Ok(params) = document.parameter_values()
            && !params.is_empty()
        {
            nodeui.add_item(TheNodeUIItem::OpenTree("Parameters".into()));
            for (name, value) in params {
                match value {
                    BuilderScriptParameterValue::Number(value) => {
                        let range = if name.contains("percent") {
                            0.0..=100.0
                        } else if name.contains("chance") || name.contains("damage") {
                            0.0..=1.0
                        } else if name.contains("seed") {
                            0.0..=9999.0
                        } else if name.contains("count") || name.contains("density") {
                            1.0..=128.0
                        } else if name.contains("segment") {
                            3.0..=64.0
                        } else if name.contains("spacing") {
                            0.05..=8.0
                        } else {
                            0.0..=8.0
                        };
                        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                            name.clone(),
                            name.clone(),
                            format!("BuilderGraph parameter '{name}'."),
                            value,
                            range,
                            true,
                        ));
                    }
                    BuilderScriptParameterValue::Ident(value) if name == "placement" => {
                        let values = vec![
                            "relief".to_string(),
                            "attached".to_string(),
                            "structural".to_string(),
                            "freestanding".to_string(),
                        ];
                        let index = values
                            .iter()
                            .position(|candidate| candidate == &value)
                            .unwrap_or(0) as i32;
                        nodeui.add_item(TheNodeUIItem::Selector(
                            name.clone(),
                            name.clone(),
                            "BuilderGraph placement mode.".into(),
                            values,
                            index,
                        ));
                    }
                    BuilderScriptParameterValue::Ident(value) => {
                        nodeui.add_item(TheNodeUIItem::Text(
                            name.clone(),
                            name.clone(),
                            format!("BuilderGraph parameter '{name}'."),
                            value,
                            None,
                            true,
                        ));
                    }
                }
            }
            nodeui.add_item(TheNodeUIItem::CloseTree);
        }
        nodeui
    }

    fn params_toml(&self, project: &Project, server_ctx: &ServerContext) -> String {
        let source = if let Some(kind) = self.selected_builtin {
            Some(self.builtin_source(kind, project, server_ctx))
        } else if let Some(index) = self.selected_treasury {
            self.treasury_items
                .get(index)
                .map(|item| item.graph_data.clone())
        } else {
            self.selected
                .or(server_ctx.curr_builder_graph_id)
                .and_then(|builder_id| project.builder_graphs.get(&builder_id))
                .map(|asset| asset.graph_data.clone())
        };
        let Some(source) = source else {
            return "# Select a Builder Graph to edit parameters.\n".to_string();
        };
        let nodeui = Self::params_nodeui_for_source(&source);
        if nodeui.is_empty() {
            "# No exposed builder parameters.\n# Add lines like: param radius = 0.14;\n".to_string()
        } else {
            let mut text = nodeui_to_toml(&nodeui).trim_end().to_string();
            text.push('\n');
            text
        }
    }

    fn sync_params_ui(&self, ui: &mut TheUI, project: &Project, server_ctx: &ServerContext) {
        if let Some(widget) = ui.get_widget(BUILDER_PARAMS_TOML)
            && let Some(edit) = widget.as_text_area_edit()
        {
            let toml_text = self.params_toml(project, server_ctx);
            if edit.text() != toml_text {
                let previous = edit.get_state();
                edit.set_text(toml_text);
                let mut state = edit.get_state();
                let row_max = state.rows.len().saturating_sub(1);
                let row = previous.cursor.row.min(row_max);
                let col_max = state
                    .rows
                    .get(row)
                    .map(|line| line.chars().count())
                    .unwrap_or(0);
                state.cursor.row = row;
                state.cursor.column = previous.cursor.column.min(col_max);
                state.selection.reset();
                TheTextAreaEditTrait::set_state(edit, state);
            }
        }
    }

    fn replace_param_value_lines(source: &str, values: &[(String, String)]) -> String {
        let mut out = Vec::new();
        for line in source.lines() {
            let trimmed = line.trim_start();
            let indent = &line[..line.len() - trimmed.len()];
            let replacement = values.iter().find_map(|(name, value)| {
                let prefix = format!("param {name}");
                if trimmed.starts_with(&prefix)
                    && trimmed[prefix.len()..].trim_start().starts_with('=')
                {
                    Some(format!("{indent}param {name} = {value};"))
                } else {
                    None
                }
            });
            out.push(replacement.unwrap_or_else(|| line.to_string()));
        }
        let mut text = out.join("\n");
        if source.ends_with('\n') {
            text.push('\n');
        }
        text
    }

    fn update_applied_hosts(
        project: &mut Project,
        server_ctx: &ServerContext,
        builder_id: Uuid,
        graph_name: &str,
        graph_data: &str,
    ) {
        let Ok(document) = BuilderDocument::from_text(graph_data) else {
            return;
        };
        let spec = document.output_spec();
        let builder_hide_host = builder_document_hides_host(&document);
        let Some(map) = project.get_map_mut(server_ctx) else {
            return;
        };

        for sector in &mut map.sectors {
            if matches!(sector.properties.get("builder_graph_id"), Some(Value::Id(id)) if *id == builder_id)
            {
                sector
                    .properties
                    .set("builder_graph_name", Value::Str(graph_name.to_string()));
                sector
                    .properties
                    .set("builder_graph_data", Value::Str(graph_data.to_string()));
                sector
                    .properties
                    .set("builder_graph_target", Value::Str("sector".to_string()));
                sector
                    .properties
                    .set("builder_surface_mode", Value::Str("overlay".to_string()));
                sector
                    .properties
                    .set("builder_hide_host", Value::Bool(builder_hide_host));
                sector
                    .properties
                    .set("builder_graph_host_refs", Value::Int(spec.host_refs as i32));
            }
        }
        for vertex in &mut map.vertices {
            if matches!(vertex.properties.get("builder_graph_id"), Some(Value::Id(id)) if *id == builder_id)
            {
                vertex
                    .properties
                    .set("builder_graph_name", Value::Str(graph_name.to_string()));
                vertex
                    .properties
                    .set("builder_graph_data", Value::Str(graph_data.to_string()));
                vertex.properties.set(
                    "builder_graph_target",
                    Value::Str("vertex_pair".to_string()),
                );
                vertex
                    .properties
                    .set("builder_graph_host_refs", Value::Int(spec.host_refs as i32));
            }
        }
        for linedef in &mut map.linedefs {
            if matches!(linedef.properties.get("builder_graph_id"), Some(Value::Id(id)) if *id == builder_id)
            {
                linedef
                    .properties
                    .set("builder_graph_name", Value::Str(graph_name.to_string()));
                linedef
                    .properties
                    .set("builder_graph_data", Value::Str(graph_data.to_string()));
                linedef
                    .properties
                    .set("builder_graph_target", Value::Str("linedef".to_string()));
                linedef
                    .properties
                    .set("builder_graph_host_refs", Value::Int(spec.host_refs as i32));
            }
        }
    }

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
        for tab in 0..2 {
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
        &mut self,
        buffer: &mut TheRGBABuffer,
        ctx: &mut TheContext,
        project: &Project,
        tab: usize,
    ) -> Vec<BuilderCardPlacement> {
        let stride = buffer.stride();
        let assets = self.assets_for_tab(project, tab);
        let card_w = ((BUILDER_CARD_BASE_W as f32) * self.zoom)
            .round()
            .max(160.0) as i32;
        let card_h = ((BUILDER_CARD_BASE_H as f32) * self.zoom)
            .round()
            .max(132.0) as i32;
        let gap = ((BUILDER_CARD_GAP as f32) * self.zoom).round().max(8.0) as i32;
        let cols = ((buffer.dim().width - BUILDER_PADDING * 2 + gap) / (card_w + gap)).max(1);
        let rows = ((assets.len() as i32 + cols - 1) / cols).max(1);
        let content_height = BUILDER_PADDING * 2 + rows * card_h + (rows - 1).max(0) * gap;
        if let Some(offset) = self.tab_offset.get_mut(tab) {
            offset.x = 0;
            offset.y = offset.y.min((content_height - buffer.dim().height).max(0));
        }
        let offset = self.tab_offset.get(tab).copied().unwrap_or_else(Vec2::zero);

        let mut placements = Vec::new();
        for (index, spec) in assets.iter().enumerate() {
            let col = index as i32 % cols;
            let row = index as i32 / cols;
            let rect = Vec4::new(
                BUILDER_PADDING + col * (card_w + gap) - offset.x,
                BUILDER_PADDING + row * (card_h + gap) - offset.y,
                card_w,
                card_h,
            );
            placements.push(BuilderCardPlacement {
                kind: spec.kind,
                rect,
            });
            if rect.x >= buffer.dim().width
                || rect.x + rect.z <= 0
                || rect.y >= buffer.dim().height
                || rect.y + rect.w <= 0
            {
                continue;
            }

            let hovered = self.hovered == Some(spec.kind);
            let selected = match spec.kind {
                BuilderCardKind::Builtin(kind) => self.selected_builtin == Some(kind),
                BuilderCardKind::Asset(id) => self.selected == Some(id),
                BuilderCardKind::Treasury(index) => self.selected_treasury == Some(index),
            };
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

            let preview_h = (card_h - 58).max(76);
            let preview_rect = Vec4::new(rect.x + 8, rect.y + 8, rect.z - 16, preview_h);
            if let Some(preview) = Self::clip_rect(buffer, preview_rect, 0) {
                ctx.draw
                    .rect(buffer.pixels_mut(), &preview, stride, &[44, 44, 44, 255]);
                ctx.draw
                    .rect_outline(buffer.pixels_mut(), &preview, stride, &[78, 78, 78, 255]);
                self.draw_preview_shape(buffer, preview_rect, spec.preview.as_ref());
            }

            let title_rect = (
                (rect.x + 8).max(0) as usize,
                (rect.y + 8 + preview_h + 8).max(0) as usize,
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
                (rect.y + 8 + preview_h + 26).max(0) as usize,
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
        rect: Vec4<i32>,
        preview: Option<&TheRGBABuffer>,
    ) {
        if let Some(preview) = preview {
            let src_w = preview.dim().width.max(1);
            let src_h = preview.dim().height.max(1);
            let (crop_x, crop_y, crop_w, crop_h) =
                Self::preview_content_bounds(preview).unwrap_or((0, 0, src_w, src_h));
            let draw_w = rect.z.max(1);
            let draw_h = rect.w.max(1);
            let draw_x = rect.x;
            let draw_y = rect.y;

            let clip_x0 = draw_x.max(0).max(rect.x).max(0);
            let clip_y0 = draw_y.max(0).max(rect.y).max(0);
            let clip_x1 = (draw_x + draw_w)
                .min(rect.x + rect.z)
                .min(buffer.dim().width);
            let clip_y1 = (draw_y + draw_h)
                .min(rect.y + rect.w)
                .min(buffer.dim().height);
            if clip_x1 <= clip_x0 || clip_y1 <= clip_y0 {
                return;
            }

            let dst_stride = buffer.stride();
            let dst = buffer.pixels_mut();
            let src = preview.pixels();
            let src_stride = src_w as usize;
            for y in clip_y0..clip_y1 {
                let sy = crop_y as usize
                    + (((y - draw_y) as i64 * crop_h as i64) / draw_h as i64)
                        .clamp(0, crop_h as i64 - 1) as usize;
                for x in clip_x0..clip_x1 {
                    let sx = crop_x as usize
                        + (((x - draw_x) as i64 * crop_w as i64) / draw_w as i64)
                            .clamp(0, crop_w as i64 - 1) as usize;
                    let src_index = (sy * src_stride + sx) * 4;
                    let dst_index = (y as usize * dst_stride + x as usize) * 4;
                    dst[dst_index..dst_index + 4].copy_from_slice(&src[src_index..src_index + 4]);
                }
            }
        }
    }

    fn preview_content_bounds(preview: &TheRGBABuffer) -> Option<(i32, i32, i32, i32)> {
        let width = preview.dim().width.max(1) as usize;
        let height = preview.dim().height.max(1) as usize;
        let pixels = preview.pixels();
        let mut min_x = width;
        let mut min_y = height;
        let mut max_x = 0usize;
        let mut max_y = 0usize;

        for y in 0..height {
            for x in 0..width {
                let index = (y * width + x) * 4;
                if pixels[index + 3] == 0 {
                    continue;
                }
                let r = pixels[index] as i32;
                let g = pixels[index + 1] as i32;
                let b = pixels[index + 2] as i32;
                let background_like =
                    (r - 46).abs() <= 5 && (g - 48).abs() <= 5 && (b - 52).abs() <= 5;
                if background_like {
                    continue;
                }
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }

        if min_x > max_x || min_y > max_y {
            return None;
        }
        let margin = 4usize;
        min_x = min_x.saturating_sub(margin);
        min_y = min_y.saturating_sub(margin);
        max_x = (max_x + margin).min(width - 1);
        max_y = (max_y + margin).min(height - 1);
        Some((
            min_x as i32,
            min_y as i32,
            (max_x - min_x + 1).max(1) as i32,
            (max_y - min_y + 1).max(1) as i32,
        ))
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

    fn assets_for_tab(&mut self, project: &Project, tab: usize) -> Vec<BuilderCardSpec> {
        match Self::tab_kind(tab) {
            BuilderTabKind::Project => {
                let assets: Vec<BuilderGraphAsset> = project
                    .builder_graphs
                    .values()
                    .filter(|asset| self.matches_asset(asset))
                    .cloned()
                    .collect();
                let mut out: Vec<BuilderCardSpec> = Self::builtin_kinds()
                    .iter()
                    .filter_map(|kind| {
                        let asset = Self::builtin_asset_for_host(*kind, false);
                        let preview_asset = Self::builtin_asset_for_host(*kind, true);
                        self.matches_asset(&asset).then(|| {
                            let card_kind = BuilderCardKind::Builtin(*kind);
                            BuilderCardSpec {
                                kind: card_kind,
                                preview: self.preview_for_asset_cached(
                                    card_kind,
                                    &preview_asset,
                                    project,
                                ),
                                label: asset.graph_name.clone(),
                                description: "Built-in organic".to_string(),
                            }
                        })
                    })
                    .collect();
                let mut project_cards: Vec<BuilderCardSpec> = assets
                    .iter()
                    .map(|asset| {
                        let kind = BuilderCardKind::Asset(asset.id);
                        let preview = self.preview_for_asset_cached(kind, asset, project);
                        let description = Self::description_for_asset(asset);
                        BuilderCardSpec {
                            kind,
                            preview,
                            label: asset.graph_name.clone(),
                            description,
                        }
                    })
                    .collect();
                project_cards.sort_by(|a, b| a.label.cmp(&b.label));
                out.extend(project_cards);
                out
            }
            BuilderTabKind::Treasury => {
                if self.treasury_items.is_empty() {
                    return vec![BuilderCardSpec {
                        kind: BuilderCardKind::Treasury(usize::MAX),
                        preview: None,
                        label: "Treasury".to_string(),
                        description: self
                            .treasury_error
                            .clone()
                            .unwrap_or_else(|| "No Builder Graph templates found.".to_string()),
                    }];
                }
                let assets: Vec<(usize, BuilderGraphAsset)> = self
                    .treasury_items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| self.matches_treasury_item(item))
                    .map(|(index, item)| (index, item.as_asset()))
                    .collect();
                assets
                    .iter()
                    .map(|(index, asset)| {
                        let kind = BuilderCardKind::Treasury(*index);
                        let description = self
                            .treasury_items
                            .get(*index)
                            .and_then(|item| {
                                (!item.description.is_empty()).then(|| item.description.clone())
                            })
                            .unwrap_or_else(|| Self::description_for_asset(asset));
                        BuilderCardSpec {
                            kind,
                            preview: self.preview_for_asset_cached(kind, asset, project),
                            label: asset.graph_name.clone(),
                            description,
                        }
                    })
                    .collect()
            }
        }
    }

    fn tab_kind(tab: usize) -> BuilderTabKind {
        match tab {
            1 => BuilderTabKind::Treasury,
            _ => BuilderTabKind::Project,
        }
    }

    fn tab_from_view_name(name: &str) -> Option<usize> {
        name.strip_prefix(BUILDER_VIEW_PREFIX)
            .and_then(|suffix| suffix.parse::<usize>().ok())
    }

    fn pick_asset(&self, tab: usize, coord: Vec2<i32>) -> Option<BuilderCardKind> {
        self.placements.get(tab)?.iter().find_map(|placement| {
            if matches!(placement.kind, BuilderCardKind::Treasury(usize::MAX)) {
                return None;
            }
            let r = placement.rect;
            (coord.x >= r.x && coord.x < r.x + r.z && coord.y >= r.y && coord.y < r.y + r.w)
                .then_some(placement.kind)
        })
    }

    fn poll_treasury_loader(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
    ) -> bool {
        let result = {
            let Some(rx) = self.treasury_load_rx.as_ref() else {
                return false;
            };
            let Ok(rx) = rx.lock() else {
                return false;
            };
            match rx.try_recv() {
                Ok(result) => result,
                Err(std::sync::mpsc::TryRecvError::Empty) => return false,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    Err("Treasury loader stopped before returning a result.".to_string())
                }
            }
        };

        self.treasury_load_rx = None;
        self.treasury_loading = false;
        self.treasury_loaded = true;
        match result {
            Ok(items) => {
                self.treasury_items = items;
                self.treasury_error = None;
            }
            Err(err) => {
                self.treasury_items.clear();
                self.treasury_error = Some(err);
            }
        }
        self.render_views(ui, ctx, project);
        true
    }

    fn ensure_treasury_loaded(&mut self, ctx: &mut TheContext) {
        if self.treasury_loaded || self.treasury_loading {
            return;
        }
        self.treasury_loading = true;
        self.treasury_error = Some("Loading Builder Graph templates from Treasury...".to_string());
        let (tx, rx) = channel();
        let event_sender = ctx.ui.state_events_sender.clone();
        thread::spawn(move || {
            let result = Self::load_treasury_items();
            let _ = tx.send(result);
            if let Some(event_sender) = event_sender {
                let _ = event_sender.send(TheEvent::Custom(
                    TheId::named(BUILDER_DOCK_REFRESH),
                    TheValue::Empty,
                ));
            }
        });
        self.treasury_load_rx = Some(Mutex::new(rx));
    }

    fn load_treasury_items() -> Result<Vec<BuilderTreasuryItem>, String> {
        let mut items = Vec::new();
        for template in crate::treasury::fetch_builder_graph_templates()? {
            let Ok(document) = BuilderDocument::from_text(&template.graph_data) else {
                continue;
            };
            let mut alias_text = Self::treasury_aliases_for_path(
                &template.summary.path,
                &template.summary.display_name(),
            );
            for alias in template
                .summary
                .aliases
                .iter()
                .chain(template.summary.tags.iter())
            {
                alias_text.push(' ');
                alias_text.push_str(&alias.to_lowercase());
            }
            items.push(BuilderTreasuryItem {
                id: template.summary.id,
                aliases: alias_text,
                description: template.summary.description,
                target: template.summary.target,
                path: template.summary.path,
                graph_name: if template.summary.name.is_empty() {
                    document.name().to_string()
                } else {
                    template.summary.name
                },
                graph_data: template.graph_data,
            });
        }
        if items.is_empty() {
            Err("No valid .buildergraph templates found in the Treasury GitHub repo.".to_string())
        } else {
            Ok(items)
        }
    }

    fn treasury_aliases_for_path(path: &str, graph_name: &str) -> String {
        let mut aliases = graph_name.to_lowercase();
        aliases.push(' ');
        aliases.push_str(&path.to_lowercase());
        for token in path
            .split(['/', '\\', '_', '-', '.'])
            .filter(|token| !token.trim().is_empty())
        {
            aliases.push(' ');
            aliases.push_str(&token.to_lowercase());
        }
        aliases
    }

    fn selected_builder_asset(
        &self,
        project: &Project,
        server_ctx: &ServerContext,
    ) -> Option<BuilderGraphAsset> {
        if let Some(kind) = self.selected_builtin {
            let mut asset = Self::builtin_asset(kind, project, server_ctx);
            asset.graph_data = self.builtin_source(kind, project, server_ctx);
            if let Ok(document) = BuilderDocument::from_text(&asset.graph_data) {
                asset.graph_name = document.name().to_string();
            }
            Some(asset)
        } else if let Some(index) = self.selected_treasury {
            self.treasury_items
                .get(index)
                .map(BuilderTreasuryItem::as_asset)
        } else {
            self.selected
                .or(server_ctx.curr_builder_graph_id)
                .and_then(|asset_id| project.builder_graphs.get(&asset_id).cloned())
        }
    }

    fn matches_asset(&self, asset: &BuilderGraphAsset) -> bool {
        self.filter.is_empty() || asset.graph_name.to_lowercase().contains(&self.filter)
    }

    fn builtin_kinds() -> [BuilderBuiltinKind; 3] {
        [
            BuilderBuiltinKind::Grass,
            BuilderBuiltinKind::Bush,
            BuilderBuiltinKind::Tree,
        ]
    }

    fn builtin_asset(
        kind: BuilderBuiltinKind,
        project: &Project,
        server_ctx: &ServerContext,
    ) -> BuilderGraphAsset {
        let use_vertex = Self::builtin_uses_vertex(project, server_ctx);
        Self::builtin_asset_for_host(kind, use_vertex)
    }

    fn builtin_uses_vertex(project: &Project, server_ctx: &ServerContext) -> bool {
        if server_ctx.builder_auto_vertex_mode {
            return true;
        }
        project
            .get_map(server_ctx)
            .map(|map| !map.selected_vertices.is_empty() && map.selected_sectors.is_empty())
            .unwrap_or(false)
    }

    fn builtin_source(
        &self,
        kind: BuilderBuiltinKind,
        project: &Project,
        server_ctx: &ServerContext,
    ) -> String {
        let use_vertex = Self::builtin_uses_vertex(project, server_ctx);
        self.builtin_sources
            .get(&(kind, use_vertex))
            .cloned()
            .unwrap_or_else(|| Self::builtin_asset(kind, project, server_ctx).graph_data)
    }

    fn builtin_asset_for_host(kind: BuilderBuiltinKind, use_vertex: bool) -> BuilderGraphAsset {
        match (kind, use_vertex) {
            (BuilderBuiltinKind::Grass, true) => {
                BuilderGraphAsset::new_grass_vertex("Grass".into())
            }
            (BuilderBuiltinKind::Bush, true) => BuilderGraphAsset::new_bush_vertex("Bush".into()),
            (BuilderBuiltinKind::Tree, true) => BuilderGraphAsset::new_tree_vertex("Tree".into()),
            (BuilderBuiltinKind::Grass, false) => BuilderGraphAsset::new_grass("Grass".into()),
            (BuilderBuiltinKind::Bush, false) => BuilderGraphAsset::new_bush("Bush".into()),
            (BuilderBuiltinKind::Tree, false) => BuilderGraphAsset::new_tree("Tree".into()),
        }
    }

    fn matches_treasury_item(&self, item: &BuilderTreasuryItem) -> bool {
        self.filter.is_empty()
            || item.graph_name.to_lowercase().contains(&self.filter)
            || item.path.to_lowercase().contains(&self.filter)
            || item.description.to_lowercase().contains(&self.filter)
            || item.target.to_lowercase().contains(&self.filter)
            || item.aliases.contains(&self.filter)
    }

    fn preview_for_asset_cached(
        &mut self,
        kind: BuilderCardKind,
        asset: &BuilderGraphAsset,
        project: &Project,
    ) -> Option<TheRGBABuffer> {
        let hash = Self::preview_hash(asset, project);
        if let Some((cached_hash, preview)) = self.preview_cache.get(&kind)
            && *cached_hash == hash
        {
            return Some(preview.clone());
        }
        let preview = Self::preview_for_asset(asset, project)?;
        self.preview_cache.insert(kind, (hash, preview.clone()));
        Some(preview)
    }

    fn preview_hash(asset: &BuilderGraphAsset, project: &Project) -> u64 {
        let mut hasher = DefaultHasher::new();
        asset.graph_data.hash(&mut hasher);
        project.tiles.len().hash(&mut hasher);
        hasher.finish()
    }

    fn preview_for_asset(asset: &BuilderGraphAsset, project: &Project) -> Option<TheRGBABuffer> {
        if let Ok(graph) = shared::buildergraph::BuilderDocument::from_text(&asset.graph_data)
            && let Ok(assembly) = graph.evaluate()
        {
            let mut assets = rusterix::Assets::default();
            assets.tiles = project.tiles.clone();
            let preview = rusterix::builderpreview::render_builder_preview_with_assets(
                &assembly,
                graph.output_spec(),
                &graph.preview_host(),
                rusterix::builderpreview::BuilderPreviewOptions {
                    size: (BUILDER_CARD_BASE_W - 16) as u32,
                    width: Some((BUILDER_CARD_BASE_W - 16) as u32),
                    height: Some((BUILDER_CARD_BASE_H - 58) as u32),
                    scale: Some(1.0),
                    variants: rusterix::builderpreview::PreviewVariants::Single,
                    ..Default::default()
                },
                &assets,
            )
            .ok()?;
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

    fn activate_builder_asset(
        &self,
        asset: &BuilderGraphAsset,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        let asset_builder_id = asset.id;
        let asset_graph_name = asset.graph_name.clone();
        let asset_graph_data = asset.graph_data.clone();
        let graph = match shared::buildergraph::BuilderDocument::from_text(&asset_graph_data) {
            Ok(graph) => graph,
            Err(err) => {
                eprintln!(
                    "[BuilderGraphDebug][apply] asset='{}' parse failed: {} (bytes={} first_bytes={:?})",
                    asset_graph_name,
                    err,
                    asset_graph_data.len(),
                    asset_graph_data
                        .as_bytes()
                        .iter()
                        .take(8)
                        .copied()
                        .collect::<Vec<_>>()
                );
                return;
            }
        };
        let spec = graph.output_spec();
        let builder_hide_host = builder_document_hides_host(&graph);
        let group_id = Uuid::new_v4();

        server_ctx.curr_map_tool_type = match spec.target {
            BuilderOutputTarget::Sector => MapToolType::Sector,
            BuilderOutputTarget::VertexPair => MapToolType::Vertex,
            BuilderOutputTarget::Linedef => MapToolType::Linedef,
        };

        eprintln!(
            "[BuilderGraphDebug][apply] asset='{}' target={:?} selected sectors={} linedefs={} vertices={}",
            asset_graph_name,
            spec.target,
            project
                .get_map(server_ctx)
                .map(|map| map.selected_sectors.len())
                .unwrap_or_default(),
            project
                .get_map(server_ctx)
                .map(|map| map.selected_linedefs.len())
                .unwrap_or_default(),
            project
                .get_map(server_ctx)
                .map(|map| map.selected_vertices.len())
                .unwrap_or_default(),
        );

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
                                .set("builder_hide_host", Value::Bool(builder_hide_host));
                            sector
                                .properties
                                .set("builder_graph_host_refs", Value::Int(spec.host_refs as i32));
                            sector
                                .properties
                                .set("builder_graph_group_id", Value::Id(group_id));
                            sector
                                .properties
                                .set("builder_graph_group_order", Value::Int(group_order as i32));
                            eprintln!(
                                "[BuilderGraphDebug][apply] wrote sector={} graph bytes={} host_refs={}",
                                sector_id,
                                asset_graph_data.len(),
                                spec.host_refs
                            );
                        }
                        if map.get_surface_for_sector_id(sector_id).is_none() {
                            let mut surface = Surface::new(sector_id);
                            surface.calculate_geometry(map);
                            map.surfaces.insert(surface.id, surface);
                            eprintln!(
                                "[BuilderGraphDebug][apply] created surface for sector={sector_id}"
                            );
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
