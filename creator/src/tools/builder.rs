use crate::docks::builder::BuilderDock;
use crate::editor::DOCKMANAGER;
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;
use rusterix::Surface;
use scenevm::GeoId;
use shared::buildergraph::BuilderDocument;

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
    fn geometry_face_normal(
        object: &rusterix::GeometryObject,
        face: &rusterix::GeometryFace,
    ) -> Option<(Vec3<f32>, Vec3<f32>)> {
        let first_index = *face.indices.first()?;
        let first = object.transform_point(*object.vertices.get(first_index)?);
        for i in 1..face.indices.len().saturating_sub(1) {
            let b = object.transform_point(*object.vertices.get(face.indices[i])?);
            let c = object.transform_point(*object.vertices.get(face.indices[i + 1])?);
            if let Some(normal) = (b - first).cross(c - first).try_normalized() {
                return Some((normal, first));
            }
        }
        None
    }

    fn geometry_face_along(
        object: &rusterix::GeometryObject,
        face: &rusterix::GeometryFace,
        normal: Vec3<f32>,
    ) -> Vec3<f32> {
        let mut best = None;
        let mut best_len_sq = 0.0;
        for i in 0..face.indices.len() {
            let Some(a) = face
                .indices
                .get(i)
                .and_then(|index| object.vertices.get(*index))
                .map(|point| object.transform_point(*point))
            else {
                continue;
            };
            let Some(b) = face
                .indices
                .get((i + 1) % face.indices.len())
                .and_then(|index| object.vertices.get(*index))
                .map(|point| object.transform_point(*point))
            else {
                continue;
            };
            let edge = b - a;
            let horizontal = Vec3::new(edge.x, 0.0, edge.z);
            let len_sq = horizontal.magnitude_squared();
            if len_sq > best_len_sq {
                best = horizontal.try_normalized();
                best_len_sq = len_sq;
            }
        }

        let mut along = best
            .or_else(|| Vec3::new(0.0, 1.0, 0.0).cross(normal).try_normalized())
            .unwrap_or_else(|| Vec3::new(1.0, 0.0, 0.0));
        let ax = along.x.abs();
        let az = along.z.abs();
        if (ax >= az && along.x < 0.0) || (az > ax && along.z < 0.0) {
            along = -along;
        }
        along
    }

    fn geometry_object_hit_frame(
        map: &Map,
        server_ctx: &ServerContext,
        hit_pos: Vec3<f32>,
    ) -> Option<(Vec3<f32>, Vec3<f32>)> {
        let Some(GeoId::GeometryObject(object_id)) = server_ctx.geo_hit else {
            return None;
        };
        let object = map
            .geometry_objects
            .iter()
            .find(|object| object.id == object_id)?;
        let (face, normal, origin) = object
            .faces
            .iter()
            .filter_map(|face| {
                let (normal, origin) = Self::geometry_face_normal(object, face)?;
                let dist = (hit_pos - origin).dot(normal).abs();
                Some((face, normal, origin, dist))
            })
            .min_by(|a, b| a.3.total_cmp(&b.3))
            .map(|(face, normal, origin, _)| (face, normal, origin))?;
        let out = resolve_creation_surface_side(
            hit_pos,
            normal,
            origin,
            server_ctx
                .hover_ray_dir_3d
                .and_then(|dir| dir.try_normalized()),
        );
        let along = Self::geometry_face_along(object, face, out);
        Some((out, along))
    }

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
                if DOCKMANAGER.read().unwrap().dock == "Builder" {
                    let mut dockmanager = DOCKMANAGER.write().unwrap();
                    dockmanager.minimize_for_tool_switch(ui, ctx);
                    if let Some(prev) = self.previous_dock.take() {
                        dockmanager.set_dock(prev, ui, ctx, project, server_ctx);
                    }
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
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            return None;
        }

        let (graph_name, graph_data, _document) = Self::selected_builder_document(server_ctx)?;
        let hit_pos = server_ctx
            .hover_surface_hit_pos
            .or_else(|| server_ctx.geo_hit.map(|_| server_ctx.geo_hit_pos))
            .or(server_ctx.hover_cursor_3d)?;

        let snapped = server_ctx.snap_world_point_for_edit(map, hit_pos);
        let geometry_frame = Self::geometry_object_hit_frame(map, server_ctx, hit_pos);
        let host_surface = if geometry_frame.is_none() {
            Self::creation_host_surface(map, server_ctx)
        } else {
            None
        };
        let host_hit_pos = server_ctx.hover_surface_hit_pos.unwrap_or(hit_pos);
        let out = geometry_frame
            .as_ref()
            .map(|(out, _)| *out)
            .or_else(|| {
                host_surface.as_ref().and_then(|surface| {
                    let normal = surface.plane.normal.try_normalized()?;
                    Some(resolve_creation_surface_side(
                        host_hit_pos,
                        normal,
                        surface.plane.origin,
                        server_ctx
                            .hover_ray_dir_3d
                            .and_then(|dir| dir.try_normalized()),
                    ))
                })
            })
            .unwrap_or_else(|| Vec3::new(0.0, 0.0, 1.0));
        let along = geometry_frame
            .as_ref()
            .map(|(_, along)| *along)
            .or_else(|| {
                host_surface.as_ref().and_then(|surface| {
                    let mut along = Vec3::new(surface.frame.right.x, 0.0, surface.frame.right.z)
                        .try_normalized()?;
                    let ax = along.x.abs();
                    let az = along.z.abs();
                    if (ax >= az && along.x < 0.0) || (az > ax && along.z < 0.0) {
                        along = -along;
                    }
                    Some(along)
                })
            })
            .unwrap_or_else(|| Vec3::new(1.0, 0.0, 0.0));
        let origin = if out.y.abs() < 0.75 {
            snapped - out * (snapped - host_hit_pos).dot(out)
        } else {
            snapped
        };

        let undo = BuilderDock::bake_builder_graph_at_point(
            map,
            server_ctx,
            server_ctx
                .curr_builder_graph_id
                .unwrap_or_else(Uuid::new_v4),
            &graph_name,
            &graph_data,
            origin,
            along,
            out,
        )?;
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Map Selection Changed"),
            TheValue::Empty,
        ));
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Geometry Overlay 3D"),
            TheValue::Empty,
        ));
        Some(undo)
    }
}
