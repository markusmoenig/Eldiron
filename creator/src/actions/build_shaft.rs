use crate::prelude::*;
use rusterix::{Surface, Value, ValueContainer};
use vek::Vec3;

pub struct BuildShaft {
    id: TheId,
    nodeui: TheNodeUI,
}

impl BuildShaft {
    fn create_polygon_sector(
        map: &mut Map,
        points: &[(f32, f32, f32)],
        props: &ValueContainer,
        shader: &Option<Uuid>,
        layer: &Option<u8>,
        name: &str,
    ) -> Option<u32> {
        if points.len() < 3 {
            return None;
        }

        map.possible_polygon.clear();
        let mut vids = Vec::with_capacity(points.len());
        for (x, z, y) in points.iter().copied() {
            vids.push(map.add_vertex_at_3d(x, z, y, false));
        }
        for i in 0..vids.len() {
            let _ = map.create_linedef_manual(vids[i], vids[(i + 1) % vids.len()]);
        }

        let sector_id = map.close_polygon_manual()?;
        if let Some(sector) = map.find_sector_mut(sector_id) {
            sector.properties = props.clone();
            sector.shader = *shader;
            sector.layer = *layer;
            sector.name = name.to_string();
        }

        let mut surface = Surface::new(sector_id);
        surface.calculate_geometry(map);
        map.surfaces.insert(surface.id, surface);
        Some(sector_id)
    }

    fn ordered_world_points(map: &Map, sector_id: u32) -> Option<Vec<Vec3<f32>>> {
        let sector = map.find_sector(sector_id)?;
        let mut points = Vec::new();
        for ld_id in &sector.linedefs {
            let ld = map.find_linedef(*ld_id)?;
            let p = map.get_vertex_3d(ld.start_vertex)?;
            if points
                .last()
                .map(|q: &Vec3<f32>| (*q - p).magnitude() < 0.0001)
                .unwrap_or(false)
            {
                continue;
            }
            points.push(p);
        }
        if points.len() < 3 { None } else { Some(points) }
    }
}

impl Action for BuildShaft {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Selector(
            "actionDirection".into(),
            "".into(),
            "".into(),
            vec!["Down".into(), "Up".into()],
            0,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionDepth".into(),
            "".into(),
            "".into(),
            2.0,
            0.0..=16.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionBottomCap".into(),
            "".into(),
            "".into(),
            true,
        ));
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_build_shaft_desc"),
        ));

        Self {
            id: TheId::named(&fl!("action_build_shaft")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_build_shaft_desc")
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
        map.selected_sectors.len() == 1
            && map
                .selected_sectors
                .first()
                .and_then(|id| map.find_sector(*id))
                .map(|sector| sector.properties.get_bool_default("cutout_handle", false))
                .unwrap_or(false)
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let prev = map.clone();
        let sector_id = *map.selected_sectors.first()?;
        let sector = map.find_sector(sector_id)?.clone();
        let points = Self::ordered_world_points(map, sector_id)?;
        if points.len() < 3 {
            return None;
        }

        let depth = self
            .nodeui
            .get_f32_value("actionDepth")
            .unwrap_or(3.0)
            .max(0.001);
        let direction = self.nodeui.get_i32_value("actionDirection").unwrap_or(0);
        let bottom_cap = self
            .nodeui
            .get_bool_value("actionBottomCap")
            .unwrap_or(true);
        let signed_depth = if direction == 1 { depth } else { -depth };

        let mut wall_props = sector.properties.clone();
        wall_props.set("visible", Value::Bool(true));
        wall_props.set("cutout_handle", Value::Bool(false));
        wall_props.set("generated_by", Value::Str("build_shaft".to_string()));
        wall_props.set("linked_cutout_handle", Value::Int(sector_id as i32));
        if let Some(v) = sector.properties.get("side_source").cloned() {
            wall_props.set("source", v);
        }

        let mut cap_props = wall_props.clone();
        if let Some(v) = sector.properties.get("cap_source").cloned() {
            cap_props.set("source", v);
        }

        let shader = sector.shader;
        let layer = sector.layer;
        let wall_name = if sector.name.is_empty() {
            "Shaft Wall".to_string()
        } else {
            format!("{} Shaft", sector.name)
        };
        let cap_name = if sector.name.is_empty() {
            "Shaft Cap".to_string()
        } else {
            format!("{} Cap", sector.name)
        };

        let mut created = Vec::new();
        let bottom_points: Vec<Vec3<f32>> = points
            .iter()
            .map(|p| Vec3::new(p.x, p.y + signed_depth, p.z))
            .collect();

        for i in 0..points.len() {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            let b_bottom = bottom_points[(i + 1) % bottom_points.len()];
            let a_bottom = bottom_points[i];
            let wall = [
                (a.x, a.z, a.y),
                (b.x, b.z, b.y),
                (b_bottom.x, b_bottom.z, b_bottom.y),
                (a_bottom.x, a_bottom.z, a_bottom.y),
            ];
            if let Some(id) =
                Self::create_polygon_sector(map, &wall, &wall_props, &shader, &layer, &wall_name)
            {
                created.push(id);
            }
        }

        if bottom_cap {
            let cap_points: Vec<(f32, f32, f32)> =
                bottom_points.iter().map(|p| (p.x, p.z, p.y)).collect();
            if let Some(id) = Self::create_polygon_sector(
                map,
                &cap_points,
                &cap_props,
                &shader,
                &layer,
                &cap_name,
            ) {
                created.push(id);
            }
        }

        if created.is_empty() {
            return None;
        }

        for created_id in &created {
            if let Some(created_sector) = map.find_sector_mut(*created_id) {
                created_sector
                    .properties
                    .set("shaft_generated", Value::Bool(true));
                created_sector
                    .properties
                    .set("linked_cutout_handle", Value::Int(sector_id as i32));
            }
        }

        map.selected_sectors = created.clone();
        map.update_surfaces();

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
