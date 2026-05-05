use crate::editor::DOCKMANAGER;
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;
use rusterix::Surface;
use scenevm::GeoId;
use shared::buildergraph::BuilderDocument;
use std::str::FromStr;

fn resolve_creation_surface_side(
    hit_pos: Vec3<f32>,
    surface_normal: Vec3<f32>,
    surface_origin: Vec3<f32>,
    hover_ray_dir: Option<Vec3<f32>>,
) -> Vec3<f32> {
    if let Some(ray_dir) = hover_ray_dir.and_then(|dir| dir.try_normalized()) {
        if surface_normal.dot(-ray_dir) >= 0.0 {
            surface_normal
        } else {
            -surface_normal
        }
    } else {
        let signed_dist = (hit_pos - surface_origin).dot(surface_normal);
        if signed_dist >= 0.0 {
            surface_normal
        } else {
            -surface_normal
        }
    }
}

pub struct BuilderTool {
    id: TheId,
    previous_dock: Option<String>,
}

impl BuilderTool {
    fn detail_surface_at_point(map: &Map, point: Vec3<f32>) -> Option<Surface> {
        let mut best_surface: Option<(Surface, f32)> = None;
        for surface in map.surfaces.values() {
            let loop_uv = match surface.sector_loop_uv(map) {
                Some(loop_uv) if !loop_uv.is_empty() => loop_uv,
                _ => continue,
            };
            let uv = surface.world_to_uv(point);
            let mut min = loop_uv[0];
            let mut max = loop_uv[0];
            for p in loop_uv.iter().skip(1) {
                min.x = min.x.min(p.x);
                min.y = min.y.min(p.y);
                max.x = max.x.max(p.x);
                max.y = max.y.max(p.y);
            }
            let eps = 0.01;
            if uv.x < min.x - eps || uv.x > max.x + eps || uv.y < min.y - eps || uv.y > max.y + eps
            {
                continue;
            }
            let n = surface.plane.normal;
            let n_len = n.magnitude();
            if n_len <= 1e-6 {
                continue;
            }
            let dist = ((point - surface.plane.origin).dot(n / n_len)).abs();
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

    fn creation_host_surface(map: &Map, server_ctx: &ServerContext) -> Option<Surface> {
        let hit_pos = server_ctx
            .hover_surface_hit_pos
            .or(server_ctx.editing_surface_hit_pos)
            .unwrap_or(server_ctx.geo_hit_pos);
        if let Some(surface) = Self::detail_surface_at_point(map, hit_pos) {
            return Some(surface);
        }
        if let Some(surface) = server_ctx
            .hover_surface
            .as_ref()
            .or(server_ctx.editing_surface.as_ref())
        {
            return Some(surface.clone());
        }
        let sector_id = match server_ctx.geo_hit {
            Some(GeoId::Sector(id)) => Some(id),
            _ => server_ctx
                .hover_surface
                .as_ref()
                .map(|surface| surface.sector_id)
                .or_else(|| {
                    server_ctx
                        .editing_surface
                        .as_ref()
                        .map(|surface| surface.sector_id)
                }),
        }?;
        if let Some(surface) = map.get_surface_for_sector_id(sector_id) {
            return Some(surface.clone());
        }
        let mut surface = Surface::new(sector_id);
        surface.calculate_geometry(map);
        surface.is_valid().then_some(surface)
    }

    fn creation_host_sector_id(map: &Map, server_ctx: &ServerContext) -> Option<u32> {
        Self::creation_host_surface(map, server_ctx)
            .map(|surface| surface.sector_id)
            .or(match server_ctx.geo_hit {
                Some(GeoId::Sector(id)) => Some(id),
                _ => None,
            })
    }

    fn selected_builder_document(
        server_ctx: &ServerContext,
    ) -> Option<(String, String, BuilderDocument)> {
        let graph_data = server_ctx.curr_builder_graph_data.clone()?;
        let graph_name = server_ctx
            .curr_builder_graph_name
            .clone()
            .unwrap_or_else(|| "Builder Script".to_string());
        let document = BuilderDocument::from_text(&graph_data).ok()?;
        Some((graph_name, graph_data, document))
    }

    fn apply_vertex_builder_to_vertex(
        map: &mut Map,
        vertex_id: u32,
        builder_id: Uuid,
        graph_name: &str,
        graph_data: &str,
        document: &BuilderDocument,
    ) {
        let spec = document.output_spec();
        if let Some(vertex) = map.find_vertex_mut(vertex_id) {
            vertex
                .properties
                .set("builder_graph_id", Value::Id(builder_id));
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

    fn selected_host_tool(project: &Project, server_ctx: &ServerContext) -> Option<MapToolType> {
        let map = project.get_map(server_ctx)?;
        if !map.selected_sectors.is_empty() {
            Some(MapToolType::Sector)
        } else if !map.selected_linedefs.is_empty() {
            Some(MapToolType::Linedef)
        } else if !map.selected_vertices.is_empty() {
            Some(MapToolType::Vertex)
        } else {
            None
        }
    }

    fn selected_builder_tool(project: &Project, server_ctx: &ServerContext) -> Option<MapToolType> {
        let builder_id = server_ctx.curr_builder_graph_id?;
        project
            .builder_graphs
            .get(&builder_id)
            .and_then(|asset| {
                shared::buildergraph::BuilderDocument::from_text(&asset.graph_data).ok()
            })
            .map(|graph| match graph.output_spec().target {
                BuilderOutputTarget::Sector => MapToolType::Sector,
                BuilderOutputTarget::VertexPair => MapToolType::Vertex,
                BuilderOutputTarget::Linedef => MapToolType::Linedef,
            })
    }
}

impl Tool for BuilderTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Builder Tool"),
            previous_dock: None,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("tool_builder")
    }

    fn icon_name(&self) -> String {
        "package".to_string()
    }

    fn accel(&self) -> Option<char> {
        Some('B')
    }

    fn help_url(&self) -> Option<String> {
        Some("docs/creator/tools/builder".to_string())
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                server_ctx.builder_tool_active = true;
                if let Some(builder_id) = server_ctx.curr_builder_graph_id
                    && let Some(asset) = project.builder_graphs.get(&builder_id)
                {
                    server_ctx.curr_builder_graph_name = Some(asset.graph_name.clone());
                    server_ctx.curr_builder_graph_data = Some(asset.graph_data.clone());
                }
                server_ctx.curr_map_tool_type = Self::selected_host_tool(project, server_ctx)
                    .or_else(|| Self::selected_builder_tool(project, server_ctx))
                    .unwrap_or(MapToolType::Sector);
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;

                let current_dock = DOCKMANAGER.read().unwrap().dock.clone();
                if current_dock != "Builder" {
                    self.previous_dock = if current_dock.is_empty() {
                        None
                    } else {
                        Some(current_dock)
                    };
                }
                DOCKMANAGER.write().unwrap().set_dock(
                    "Builder".into(),
                    ui,
                    ctx,
                    project,
                    server_ctx,
                );
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                true
            }
            DeActivate => {
                server_ctx.builder_tool_active = false;
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;
                if DOCKMANAGER.read().unwrap().dock == "Builder"
                    && let Some(prev) = self.previous_dock.take()
                {
                    DOCKMANAGER
                        .write()
                        .unwrap()
                        .set_dock(prev, ui, ctx, project, server_ctx);
                }
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                true
            }
            _ => false,
        }
    }

    fn map_event(
        &mut self,
        map_event: MapEvent,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let MapClicked(_coord) = map_event else {
            return None;
        };
        if !server_ctx.builder_auto_vertex_mode || server_ctx.editor_view_mode == EditorViewMode::D2
        {
            return None;
        }

        let (graph_name, graph_data, document) = Self::selected_builder_document(server_ctx)?;
        if document.output_spec().target != BuilderOutputTarget::VertexPair {
            return None;
        }

        let pt = server_ctx
            .hover_cursor_3d
            .or_else(|| server_ctx.geo_hit.map(|_| server_ctx.geo_hit_pos))?;
        let prev = map.clone();
        let snapped = server_ctx.snap_world_point_for_edit(map, pt);
        let vertex_id = map.add_vertex_at_3d(snapped.x, snapped.z, snapped.y, false);

        if let Some(host_sector_id) = Self::creation_host_sector_id(map, server_ctx) {
            let host_surface = Self::creation_host_surface(map, server_ctx);
            let host_hit_pos = server_ctx
                .hover_surface_hit_pos
                .or(server_ctx.editing_surface_hit_pos)
                .unwrap_or(server_ctx.geo_hit_pos);
            let host_outward = host_surface.as_ref().and_then(|surface| {
                let normal = surface.plane.normal.try_normalized()?;
                Some(resolve_creation_surface_side(
                    host_hit_pos,
                    normal,
                    surface.plane.origin,
                    server_ctx
                        .hover_ray_dir_3d
                        .and_then(|dir| dir.try_normalized()),
                ))
            });
            let host_along = host_surface.as_ref().and_then(|surface| {
                let mut along = Vec3::new(surface.frame.right.x, 0.0, surface.frame.right.z)
                    .try_normalized()?;
                let ax = along.x.abs();
                let az = along.z.abs();
                if (ax >= az && along.x < 0.0) || (az > ax && along.z < 0.0) {
                    along = -along;
                }
                Some(along)
            });
            let host_face_origin = host_outward
                .map(|outward| snapped - outward * (snapped - host_hit_pos).dot(outward));
            if let Some(vertex) = map.find_vertex_mut(vertex_id) {
                vertex
                    .properties
                    .set("host_sector", Value::Int(host_sector_id as i32));
                if let Some(outward) = host_outward {
                    vertex
                        .properties
                        .set("host_outward_x", Value::Float(outward.x));
                    vertex
                        .properties
                        .set("host_outward_y", Value::Float(outward.y));
                    vertex
                        .properties
                        .set("host_outward_z", Value::Float(outward.z));
                }
                if let Some(along) = host_along {
                    vertex.properties.set("host_along_x", Value::Float(along.x));
                    vertex.properties.set("host_along_y", Value::Float(along.y));
                    vertex.properties.set("host_along_z", Value::Float(along.z));
                }
                let origin = host_face_origin.unwrap_or(snapped);
                vertex
                    .properties
                    .set("host_surface_origin_x", Value::Float(origin.x));
                vertex
                    .properties
                    .set("host_surface_origin_y", Value::Float(origin.y));
                vertex
                    .properties
                    .set("host_surface_origin_z", Value::Float(origin.z));
            }
        }

        Self::apply_vertex_builder_to_vertex(
            map,
            vertex_id,
            server_ctx
                .curr_builder_graph_id
                .unwrap_or_else(Uuid::new_v4),
            &graph_name,
            &graph_data,
            &document,
        );
        map.selected_vertices = vec![vertex_id];
        map.selected_linedefs.clear();
        map.selected_sectors.clear();
        server_ctx.curr_map_tool_type = MapToolType::Vertex;
        server_ctx.curr_action_id =
            Some(Uuid::from_str(crate::actions::edit_vertex::EDIT_VERTEX_ACTION_ID).unwrap());
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Map Selection Changed"),
            TheValue::Empty,
        ));
        crate::editor::RUSTERIX.write().unwrap().set_dirty();

        Some(ProjectUndoAtom::MapEdit(
            server_ctx.pc,
            Box::new(prev),
            Box::new(map.clone()),
        ))
    }
}
