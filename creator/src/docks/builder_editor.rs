use crate::prelude::*;
use rusterix::Surface;
use rusterix::builderpreview::{BuilderPreviewOptions, PreviewVariants, render_builder_preview};
use scenevm::GeoId;
use shared::buildergraph::{BuilderDocument, BuilderGraph};

const BUILDER_SCRIPT_EDITOR: &str = "Builder Script Editor";
const BUILDER_SCRIPT_PREVIEW: &str = "Builder Script Preview";
const BUILDER_SCRIPT_TITLE: &str = "Builder Script Title";
const BUILDER_EDITOR_GRAPH_BUTTON: &str = "Builder Editor Graph";
const BUILDER_EDITOR_RESET_BUTTON: &str = "Builder Editor Reset";

pub struct BuilderEditorDock {
    source: String,
    active_builder_id: Option<Uuid>,
}

impl BuilderEditorDock {
    fn default_source() -> String {
        BuilderGraph::preset_table_script_named("Table".to_string())
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

    fn load_state_from_project(&mut self, project: &Project, server_ctx: &ServerContext) {
        self.active_builder_id = server_ctx.curr_builder_graph_id;
        self.source = self
            .active_builder_id
            .and_then(|builder_id| project.builder_graphs.get(&builder_id))
            .map(|asset| asset.graph_data.clone())
            .unwrap_or_else(Self::default_source);
    }

    fn sync_editor(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        ui.set_widget_value(
            BUILDER_SCRIPT_EDITOR,
            ctx,
            TheValue::Text(self.source.clone()),
        );
    }

    fn refresh_ui(
        &self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &ServerContext,
    ) {
        self.sync_editor(ui, ctx);
        self.render_preview(ui, ctx);
        self.sync_title(ui, ctx, project, server_ctx);
    }

    fn sync_title(
        &self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &ServerContext,
    ) {
        let title = server_ctx
            .curr_builder_graph_id
            .and_then(|id| project.builder_graphs.get(&id))
            .map(|asset| asset.graph_name.clone())
            .unwrap_or_else(|| "Builder Script".to_string());
        ui.set_widget_value(BUILDER_SCRIPT_TITLE, ctx, TheValue::Text(title));
    }

    fn render_preview(&self, ui: &mut TheUI, _ctx: &mut TheContext) {
        let mut buffer = TheRGBABuffer::new(TheDim::sized(384, 384));
        for pixel in buffer.pixels_mut().chunks_exact_mut(4) {
            pixel.copy_from_slice(&[30, 32, 36, 255]);
        }

        if let Ok(document) = BuilderDocument::from_text(&self.source)
            && let Ok(assembly) = document.evaluate()
        {
            let preview = render_builder_preview(
                &assembly,
                document.output_spec(),
                &document.preview_host(),
                BuilderPreviewOptions {
                    size: 640,
                    variants: PreviewVariants::Single,
                    ..Default::default()
                },
            );
            if let Ok(preview) = preview {
                buffer =
                    TheRGBABuffer::new(TheDim::sized(preview.width as i32, preview.height as i32));
                buffer.pixels_mut().copy_from_slice(&preview.pixels);
            }
        }

        if let Some(render_view) = ui.get_render_view(BUILDER_SCRIPT_PREVIEW) {
            *render_view.render_buffer_mut() = buffer;
        }
    }

    fn save_state_to_project(
        &self,
        project: &mut Project,
        server_ctx: &ServerContext,
        ctx: &mut TheContext,
    ) {
        let Some(builder_id) = self.active_builder_id.or(server_ctx.curr_builder_graph_id) else {
            return;
        };

        let parsed = BuilderDocument::from_text(&self.source).ok();
        let graph_name = parsed
            .as_ref()
            .map(|doc| doc.name().to_string())
            .or_else(|| {
                project
                    .builder_graphs
                    .get(&builder_id)
                    .map(|asset| asset.graph_name.clone())
            })
            .unwrap_or_else(|| "Builder Script".to_string());
        let spec = parsed.as_ref().map(|doc| doc.output_spec());

        if let Some(asset) = project.builder_graphs.get_mut(&builder_id) {
            asset.graph_name = graph_name.clone();
            asset.graph_data = self.source.clone();
        }

        if let Some(map) = project.get_map_mut(server_ctx) {
            if let Some(spec) = spec {
                for sector_id in map.selected_sectors.clone() {
                    if let Some(sector) = map.find_sector_mut(sector_id) {
                        let matches_builder = matches!(
                            sector.properties.get("builder_graph_id"),
                            Some(Value::Id(id)) if *id == builder_id
                        );
                        if matches_builder {
                            sector
                                .properties
                                .set("builder_graph_name", Value::Str(graph_name.clone()));
                            sector
                                .properties
                                .set("builder_graph_data", Value::Str(self.source.clone()));
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
                        }
                    }
                    if map.get_surface_for_sector_id(sector_id).is_none() {
                        let mut surface = Surface::new(sector_id);
                        surface.calculate_geometry(map);
                        map.surfaces.insert(surface.id, surface);
                    }
                }
                for vertex_id in map.selected_vertices.clone() {
                    if let Some(vertex) = map.find_vertex_mut(vertex_id) {
                        let matches_builder = matches!(
                            vertex.properties.get("builder_graph_id"),
                            Some(Value::Id(id)) if *id == builder_id
                        );
                        if matches_builder {
                            vertex
                                .properties
                                .set("builder_graph_name", Value::Str(graph_name.clone()));
                            vertex
                                .properties
                                .set("builder_graph_data", Value::Str(self.source.clone()));
                            vertex.properties.set(
                                "builder_graph_target",
                                Value::Str("vertex_pair".to_string()),
                            );
                            vertex
                                .properties
                                .set("builder_graph_host_refs", Value::Int(spec.host_refs as i32));
                        }
                    }
                }
                for linedef_id in map.selected_linedefs.clone() {
                    let wall_side = Self::linedef_builder_wall_side(map, server_ctx, linedef_id);
                    let wall_outward = Self::linedef_builder_outward(map, server_ctx, linedef_id);
                    let wall_face_origin =
                        Self::linedef_builder_face_origin(map, server_ctx, linedef_id);
                    if let Some(linedef) = map.find_linedef_mut(linedef_id) {
                        let matches_builder = matches!(
                            linedef.properties.get("builder_graph_id"),
                            Some(Value::Id(id)) if *id == builder_id
                        );
                        if matches_builder {
                            linedef
                                .properties
                                .set("builder_graph_name", Value::Str(graph_name.clone()));
                            linedef
                                .properties
                                .set("builder_graph_data", Value::Str(self.source.clone()));
                            linedef
                                .properties
                                .set("builder_graph_target", Value::Str("linedef".to_string()));
                            linedef
                                .properties
                                .set("builder_graph_host_refs", Value::Int(spec.host_refs as i32));
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
            } else {
                for sector_id in map.selected_sectors.clone() {
                    if let Some(sector) = map.find_sector_mut(sector_id)
                        && matches!(
                            sector.properties.get("builder_graph_id"),
                            Some(Value::Id(id)) if *id == builder_id
                        )
                    {
                        sector
                            .properties
                            .set("builder_graph_data", Value::Str(self.source.clone()));
                    }
                }
                for vertex_id in map.selected_vertices.clone() {
                    if let Some(vertex) = map.find_vertex_mut(vertex_id)
                        && matches!(
                            vertex.properties.get("builder_graph_id"),
                            Some(Value::Id(id)) if *id == builder_id
                        )
                    {
                        vertex
                            .properties
                            .set("builder_graph_data", Value::Str(self.source.clone()));
                    }
                }
                for linedef_id in map.selected_linedefs.clone() {
                    if let Some(linedef) = map.find_linedef_mut(linedef_id)
                        && matches!(
                            linedef.properties.get("builder_graph_id"),
                            Some(Value::Id(id)) if *id == builder_id
                        )
                    {
                        linedef
                            .properties
                            .set("builder_graph_data", Value::Str(self.source.clone()));
                    }
                }
            }
        }

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Builder Graph Updated"),
            TheValue::Id(builder_id),
        ));
    }

    fn import_graph_file(&mut self, path: &std::path::Path) -> Result<(), String> {
        let source = std::fs::read_to_string(path)
            .map_err(|err| format!("failed to read {}: {err}", path.to_string_lossy()))?;
        BuilderDocument::from_text(&source)
            .map_err(|err| format!("failed to parse {}: {err}", path.to_string_lossy()))?;
        self.source = source;
        Ok(())
    }

    fn export_graph_file(&self, path: &std::path::Path) -> Result<std::path::PathBuf, String> {
        let mut output_path = path.to_path_buf();
        if output_path.extension().is_none() {
            output_path.set_extension("buildergraph");
        }
        std::fs::write(&output_path, &self.source)
            .map_err(|err| format!("failed to write {}: {err}", output_path.to_string_lossy()))?;
        Ok(output_path)
    }
}

impl Dock for BuilderEditorDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            source: Self::default_source(),
            active_builder_id: None,
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
        toolbar_hlayout.set_reverse_index(Some(2));

        let mut title = TheText::new(TheId::named(BUILDER_SCRIPT_TITLE));
        title.set_text("Builder Script".to_string());
        title.set_text_size(12.0);
        toolbar_hlayout.add_widget(Box::new(title));

        let spacer = TheSpacer::new(TheId::empty());
        toolbar_hlayout.add_widget(Box::new(spacer));

        let mut graph_button = TheTraybarButton::new(TheId::named(BUILDER_EDITOR_GRAPH_BUTTON));
        graph_button.set_text("Graph".to_string());
        graph_button.set_status_text("Import or export the current builder script.");
        let mut graph_menu = TheContextMenu::default();
        graph_menu.add(TheContextMenuItem::new(
            "Import Script...".to_string(),
            TheId::named("Builder Import Graph"),
        ));
        graph_menu.add(TheContextMenuItem::new(
            "Export Script...".to_string(),
            TheId::named("Builder Export Graph"),
        ));
        graph_button.set_context_menu(Some(graph_menu));
        toolbar_hlayout.add_widget(Box::new(graph_button));

        let mut reset_button = TheTraybarButton::new(TheId::named(BUILDER_EDITOR_RESET_BUTTON));
        reset_button.set_text("Reset".to_string());
        reset_button.set_status_text("Reset to the current default builder script preset.");
        toolbar_hlayout.add_widget(Box::new(reset_button));

        toolbar_canvas.set_layout(toolbar_hlayout);
        canvas.set_top(toolbar_canvas);

        let mut center = TheCanvas::new();

        let mut script_canvas = TheCanvas::new();
        let mut textedit = TheTextAreaEdit::new(TheId::named(BUILDER_SCRIPT_EDITOR));
        if let Some(bytes) = crate::Embedded::get("parser/buildergraph.sublime-syntax")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            textedit.add_syntax_from_string(source);
            textedit.set_code_type("BuilderGraph");
        }
        if let Some(bytes) = crate::Embedded::get("parser/gruvbox-dark.tmTheme")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            textedit.add_theme_from_string(source);
            textedit.set_code_theme("Gruvbox Dark");
        }
        textedit.set_continuous(true);
        textedit.display_line_number(true);
        textedit.use_global_statusbar(true);
        textedit.set_font_size(14.0);
        textedit.set_supports_undo(false);
        script_canvas.set_widget(textedit);
        center.set_center(script_canvas);

        let mut preview_canvas = TheCanvas::new();
        let mut preview_view = TheRenderView::new(TheId::named(BUILDER_SCRIPT_PREVIEW));
        preview_view
            .limiter_mut()
            .set_max_size(Vec2::new(i32::MAX, i32::MAX));
        preview_view.limiter_mut().set_max_width(560);
        preview_view.limiter_mut().set_min_width(360);
        preview_view.limiter_mut().set_min_height(360);
        preview_canvas.set_widget(preview_view);
        center.set_right(preview_canvas);

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
        self.load_state_from_project(project, server_ctx);
        self.refresh_ui(ui, ctx, project, server_ctx);
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match event {
            TheEvent::ValueChanged(id, TheValue::Text(text))
                if id.name == BUILDER_SCRIPT_EDITOR =>
            {
                self.source = text.clone();
                self.render_preview(ui, ctx);
                self.save_state_to_project(project, server_ctx, ctx);
                true
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == BUILDER_EDITOR_RESET_BUTTON =>
            {
                self.source = server_ctx
                    .curr_builder_graph_id
                    .and_then(|id| project.builder_graphs.get(&id))
                    .map(|asset| asset.graph_name.clone())
                    .map(|name| {
                        if name.starts_with("Wall Torch") {
                            BuilderGraph::preset_wall_torch_script_named(name)
                        } else if name.starts_with("Empty") {
                            BuilderGraph::empty_script_named(name)
                        } else {
                            BuilderGraph::preset_table_script_named(name)
                        }
                    })
                    .unwrap_or_else(Self::default_source);
                self.refresh_ui(ui, ctx, project, server_ctx);
                self.save_state_to_project(project, server_ctx, ctx);
                true
            }
            TheEvent::ContextMenuSelected(id, item) if id.name == BUILDER_EDITOR_GRAPH_BUTTON => {
                if item.name == "Builder Import Graph" {
                    ctx.ui.open_file_requester(
                        TheId::named("Builder Import Graph File"),
                        "Import Builder Script".into(),
                        TheFileExtension::new(
                            "Eldiron Builder Graph".into(),
                            vec!["buildergraph".to_string(), "json".to_string()],
                        ),
                    );
                    true
                } else if item.name == "Builder Export Graph" {
                    ctx.ui.save_file_requester(
                        TheId::named("Builder Export Graph File"),
                        "Export Builder Script".into(),
                        TheFileExtension::new(
                            "Eldiron Builder Graph".into(),
                            vec!["buildergraph".to_string()],
                        ),
                    );
                    true
                } else {
                    false
                }
            }
            TheEvent::FileRequesterResult(id, paths)
                if id.name == "Builder Import Graph File" && !paths.is_empty() =>
            {
                if let Some(path) = paths.first() {
                    match self.import_graph_file(path) {
                        Ok(()) => {
                            self.refresh_ui(ui, ctx, project, server_ctx);
                            self.save_state_to_project(project, server_ctx, ctx);
                            ctx.ui.send(TheEvent::SetStatusText(
                                id.clone(),
                                format!("Imported builder script from {}", path.to_string_lossy()),
                            ));
                        }
                        Err(err) => {
                            ctx.ui.send(TheEvent::SetStatusText(
                                id.clone(),
                                format!("Builder import failed: {err}"),
                            ));
                        }
                    }
                }
                true
            }
            TheEvent::FileRequesterResult(id, paths)
                if id.name == "Builder Export Graph File" && !paths.is_empty() =>
            {
                if let Some(path) = paths.first() {
                    match self.export_graph_file(path) {
                        Ok(saved_path) => {
                            ctx.ui.send(TheEvent::SetStatusText(
                                id.clone(),
                                format!(
                                    "Exported builder script to {}",
                                    saved_path.to_string_lossy()
                                ),
                            ));
                        }
                        Err(err) => {
                            ctx.ui.send(TheEvent::SetStatusText(
                                id.clone(),
                                format!("Builder export failed: {err}"),
                            ));
                        }
                    }
                }
                true
            }
            TheEvent::Custom(id, _) if id.name == "Builder Selection Changed" => {
                self.load_state_from_project(project, server_ctx);
                self.refresh_ui(ui, ctx, project, server_ctx);
                true
            }
            _ => false,
        }
    }
}
