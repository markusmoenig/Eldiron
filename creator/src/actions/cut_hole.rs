use crate::prelude::*;
use rusterix::{Surface, Value, Vertex};

pub struct CutHole {
    id: TheId,
    nodeui: TheNodeUI,
}

impl CutHole {
    fn add_vertex_exact(map: &mut Map, x: f32, y: f32) -> Option<u32> {
        if let Some(id) = map.find_vertex_at(x, y) {
            return Some(id);
        }
        let id = map.find_free_vertex_id()?;
        map.vertices.push(Vertex::new(id, x, y));
        Some(id)
    }

    fn sector_ordered_vertex_ids(map: &Map, sector_id: u32) -> Option<Vec<u32>> {
        let sector = map.find_sector(sector_id)?;
        let mut ids = Vec::new();
        for ld_id in &sector.linedefs {
            let ld = map.find_linedef(*ld_id)?;
            if ids.last().copied() == Some(ld.start_vertex) {
                continue;
            }
            ids.push(ld.start_vertex);
        }
        if ids.len() < 3 { None } else { Some(ids) }
    }

    fn sector_points_world(map: &Map, sector_id: u32) -> Option<Vec<Vec3<f32>>> {
        let sector = map.find_sector(sector_id)?;
        let mut points = Vec::new();
        for ld_id in &sector.linedefs {
            let ld = map.find_linedef(*ld_id)?;
            let v = map.get_vertex_3d(ld.start_vertex)?;
            let p = Vec3::new(v.x, v.y, v.z);
            if points
                .last()
                .map(|q: &Vec3<f32>| (*q - p).magnitude() < 0.0001)
                .unwrap_or(false)
            {
                continue;
            }
            if points
                .first()
                .map(|q| (*q - p).magnitude() < 0.0001)
                .unwrap_or(false)
                && points.len() >= 3
            {
                continue;
            }
            points.push(p);
        }
        if points.len() < 3 { None } else { Some(points) }
    }

    fn resolve_host_and_cut(map: &Map, selected_sectors: &[u32]) -> Option<(u32, u32)> {
        if selected_sectors.len() != 2 {
            return None;
        }
        let a = selected_sectors[0];
        let b = selected_sectors[1];

        let a_points_world = Self::sector_points_world(map, a)?;
        let b_points_world = Self::sector_points_world(map, b)?;

        let a_points_xy: Vec<Vec2<f32>> =
            a_points_world.iter().map(|p| Vec2::new(p.x, p.z)).collect();
        let b_points_xy: Vec<Vec2<f32>> =
            b_points_world.iter().map(|p| Vec2::new(p.x, p.z)).collect();

        let a_height =
            a_points_world.iter().map(|p| p.y).sum::<f32>() / a_points_world.len() as f32;
        let b_height =
            b_points_world.iter().map(|p| p.y).sum::<f32>() / b_points_world.len() as f32;
        if (a_height - b_height).abs() > 0.0001 {
            return None;
        }

        let a_sector = map.find_sector(a)?;
        let b_sector = map.find_sector(b)?;
        let a_contains_b = b_points_xy.iter().all(|p| a_sector.is_inside(map, *p));
        let b_contains_a = a_points_xy.iter().all(|p| b_sector.is_inside(map, *p));

        match (a_contains_b, b_contains_a) {
            (true, false) => Some((a, b)),
            (false, true) => Some((b, a)),
            (true, true) => {
                let a_area = a_sector.area(map);
                let b_area = b_sector.area(map);
                if a_area >= b_area {
                    Some((a, b))
                } else {
                    Some((b, a))
                }
            }
            (false, false) => None,
        }
    }

    fn ensure_host_surface_profile(map: &mut Map, host_sector_id: u32) -> Option<(Surface, Uuid)> {
        let mut created_profile = None;
        let profile_id = {
            let surface = if let Some(surface) = map.get_surface_for_sector_id_mut(host_sector_id) {
                surface
            } else {
                let mut surface = Surface::new(host_sector_id);
                surface.calculate_geometry(map);
                let id = surface.id;
                map.surfaces.insert(id, surface);
                map.get_surface_for_sector_id_mut(host_sector_id)?
            };

            if surface.profile.is_none() {
                let profile = Map::default();
                let profile_id = profile.id;
                surface.profile = Some(profile_id);
                created_profile = Some(profile);
            }
            surface.profile?
        };

        if let Some(profile) = created_profile {
            map.profiles.insert(profile.id, profile);
        }

        let surface = map.get_surface_for_sector_id(host_sector_id)?.clone();
        Some((surface, profile_id))
    }

    fn add_profile_cutout(map: &mut Map, host_sector_id: u32, cut_sector_id: u32) -> Option<u32> {
        let cut_points_world = Self::sector_points_world(map, cut_sector_id)?;
        let (surface, profile_id) = Self::ensure_host_surface_profile(map, host_sector_id)?;
        let profile_map = map.profiles.get_mut(&profile_id)?;

        profile_map.clear_temp();

        let mut vids = Vec::with_capacity(cut_points_world.len());
        for p in cut_points_world {
            let uv_world = surface.world_to_uv(p);
            let uv = Vec2::new(uv_world.x, -uv_world.y);
            vids.push(Self::add_vertex_exact(profile_map, uv.x, uv.y)?);
        }

        for i in 0..vids.len() {
            let _ = profile_map.create_linedef_manual(vids[i], vids[(i + 1) % vids.len()]);
        }

        let sector_id = profile_map.close_polygon_manual()?;
        if let Some(sector) = profile_map.find_sector_mut(sector_id) {
            sector
                .properties
                .set("generated_by", Value::Str("cut_hole".to_string()));
            sector.properties.set("cut_hole", Value::Bool(true));
            sector
                .properties
                .set("host_sector", Value::Int(host_sector_id as i32));
        }
        Some(sector_id)
    }

    fn sync_cut_sector_from_profile(
        map: &mut Map,
        host_sector_id: u32,
        cut_sector_id: u32,
        profile_sector_id: u32,
    ) -> bool {
        let Some((surface, profile_id)) = Self::ensure_host_surface_profile(map, host_sector_id)
        else {
            return false;
        };
        let Some(profile_map) = map.profiles.get(&profile_id) else {
            return false;
        };
        let Some(profile_vertex_ids) =
            Self::sector_ordered_vertex_ids(profile_map, profile_sector_id)
        else {
            return false;
        };

        let mut world_points = Vec::with_capacity(profile_vertex_ids.len());
        for vertex_id in profile_vertex_ids {
            let Some(v) = profile_map.find_vertex(vertex_id) else {
                return false;
            };
            let uv = Vec2::new(v.x, -v.y);
            world_points.push(surface.uv_to_world(uv));
        }

        let Some(cut_vertex_ids) = Self::sector_ordered_vertex_ids(map, cut_sector_id) else {
            return false;
        };
        if cut_vertex_ids.len() != world_points.len() {
            return false;
        }

        for (vertex_id, world) in cut_vertex_ids.into_iter().zip(world_points.into_iter()) {
            if let Some(v) = map.find_vertex_mut(vertex_id) {
                v.x = world.x;
                v.y = world.z;
                v.z = world.y;
            }
        }
        true
    }
}

impl Action for CutHole {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_cut_hole_desc"),
        ));

        Self {
            id: TheId::named(&fl!("action_cut_hole")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_cut_hole_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        if server_ctx.editor_view_mode == EditorViewMode::D2
            || server_ctx.geometry_edit_mode == GeometryEditMode::Detail
        {
            return false;
        }
        map.selected_sectors.len() == 2
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let prev = map.clone();
        let selected = map.selected_sectors.clone();

        let (host_sector_id, cut_sector_id) = Self::resolve_host_and_cut(map, &selected)?;
        let profile_sector_id = Self::add_profile_cutout(map, host_sector_id, cut_sector_id)?;
        let _ = Self::sync_cut_sector_from_profile(
            map,
            host_sector_id,
            cut_sector_id,
            profile_sector_id,
        );

        if let Some(cut_sector) = map.find_sector_mut(cut_sector_id) {
            cut_sector.properties.set("visible", Value::Bool(false));
            cut_sector
                .properties
                .set("cutout_handle", Value::Bool(true));
            cut_sector
                .properties
                .set("host_sector", Value::Int(host_sector_id as i32));
            cut_sector.properties.set(
                "linked_profile_sector",
                Value::Int(profile_sector_id as i32),
            );
        }
        if let Some(host_sector) = map.find_sector_mut(host_sector_id) {
            host_sector
                .properties
                .set("linked_cutout_handle", Value::Int(cut_sector_id as i32));
            host_sector.properties.set(
                "linked_profile_sector",
                Value::Int(profile_sector_id as i32),
            );
        }
        map.selected_sectors.clear();
        map.selected_sectors.push(cut_sector_id);
        map.update_surfaces();

        if let Some(surface) = map.get_surface_for_sector_id(host_sector_id).cloned()
            && let Some(profile_id) = surface.profile
        {
            if let Some(host_sector) = map.find_sector_mut(host_sector_id) {
                host_sector
                    .properties
                    .set("linked_surface", Value::Id(surface.id));
                host_sector
                    .properties
                    .set("linked_profile", Value::Id(profile_id));
            }
            if let Some(cut_sector) = map.find_sector_mut(cut_sector_id) {
                cut_sector
                    .properties
                    .set("linked_surface", Value::Id(surface.id));
                cut_sector
                    .properties
                    .set("linked_profile", Value::Id(profile_id));
            }
            if let Some(profile_map) = map.profiles.get_mut(&profile_id) {
                profile_map.selected_sectors.clear();
                profile_map.selected_sectors.push(profile_sector_id);
                if let Some(profile_sector) = profile_map.find_sector_mut(profile_sector_id) {
                    profile_sector
                        .properties
                        .set("linked_geom_sector", Value::Int(cut_sector_id as i32));
                    profile_sector
                        .properties
                        .set("linked_surface", Value::Id(surface.id));
                    profile_sector
                        .properties
                        .set("linked_profile", Value::Id(profile_id));
                }
            }
        }

        Some(ProjectUndoAtom::MapEdit(
            server_ctx.pc,
            Box::new(prev),
            Box::new(map.clone()),
        ))
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _project: &mut Project,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        self.nodeui.handle_event(event)
    }
}
