use crate::prelude::*;
use rusterix::{PixelSource, Surface, Value, ValueContainer};
use vek::{Vec2, Vec3};

pub struct BuildRoom {
    id: TheId,
    nodeui: TheNodeUI,
}

impl BuildRoom {
    fn parse_tile_source(text: &str) -> Option<Value> {
        Some(Value::Source(parse_tile_id_pixelsource(text)?))
    }

    fn parse_tile_pixelsource(text: &str) -> Option<PixelSource> {
        match Self::parse_tile_source(text) {
            Some(Value::Source(source)) => Some(source),
            _ => None,
        }
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

    fn base_edge(points: &[Vec3<f32>]) -> Option<(Vec3<f32>, Vec3<f32>, f32, f32)> {
        if points.len() < 3 {
            return None;
        }

        let min_y = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let max_y = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
        let eps = 0.01;

        for i in 0..points.len() {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            if (a.y - min_y).abs() <= eps && (b.y - min_y).abs() <= eps {
                return Some((a, b, min_y, max_y));
            }
        }

        let mut bottom: Vec<Vec3<f32>> = points
            .iter()
            .copied()
            .filter(|p| (p.y - min_y).abs() <= 0.05)
            .collect();
        if bottom.len() < 2 {
            bottom = points.to_vec();
            bottom.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal));
            bottom.truncate(2);
        }
        if bottom.len() < 2 {
            return None;
        }
        Some((bottom[0], bottom[1], min_y, max_y))
    }

    fn local_polygon(kind: i32, width: f32, depth: f32) -> Vec<Vec2<f32>> {
        let w = width.max(0.1);
        let d = depth.max(0.1);
        match kind {
            1 => vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(w, 0.0),
                Vec2::new(w, d),
                Vec2::new(0.0, d),
            ],
            2 => {
                let c = (w.min(d) * 0.18).max(0.05);
                vec![
                    Vec2::new(c, 0.0),
                    Vec2::new(w - c, 0.0),
                    Vec2::new(w, c),
                    Vec2::new(w, d - c),
                    Vec2::new(w - c, d),
                    Vec2::new(c, d),
                    Vec2::new(0.0, d - c),
                    Vec2::new(0.0, c),
                ]
            }
            3 => {
                let c = (w.min(d) * 0.28).max(0.05);
                vec![
                    Vec2::new(c, 0.0),
                    Vec2::new(w - c, 0.0),
                    Vec2::new(w, c),
                    Vec2::new(w, d - c),
                    Vec2::new(w - c, d),
                    Vec2::new(c, d),
                    Vec2::new(0.0, d - c),
                    Vec2::new(0.0, c),
                ]
            }
            _ => vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(w, 0.0),
                Vec2::new(w, d),
                Vec2::new(0.0, d),
            ],
        }
    }

    fn world_from_local(
        origin: Vec3<f32>,
        tangent: Vec2<f32>,
        normal: Vec2<f32>,
        p: Vec2<f32>,
        y: f32,
    ) -> (f32, f32, f32) {
        let x = origin.x + tangent.x * p.x + normal.x * p.y;
        let z = origin.z + tangent.y * p.x + normal.y * p.y;
        (x, z, y)
    }
}

impl Action for BuildRoom {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Selector(
            "actionRoomType".into(),
            "".into(),
            "".into(),
            vec![
                "Rect".into(),
                "Corridor".into(),
                "Chamfered".into(),
                "Octagon".into(),
            ],
            0,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionRoomDepth".into(),
            "".into(),
            "".into(),
            6.0,
            1.0..=64.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionRoomHeight".into(),
            "".into(),
            "".into(),
            3.0,
            1.0..=16.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Selector(
            "actionWidthMode".into(),
            "".into(),
            "".into(),
            vec!["Match Wall".into(), "Expand".into(), "Custom".into()],
            0,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionRoomWidth".into(),
            "".into(),
            "".into(),
            6.0,
            1.0..=64.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Selector(
            "actionCeilingMode".into(),
            "".into(),
            "".into(),
            vec!["Flat".into(), "None".into()],
            0,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionKeepOriginalWall".into(),
            "".into(),
            "".into(),
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionCloseFrontLip".into(),
            "".into(),
            "".into(),
            false,
        ));
        nodeui.add_item(TheNodeUIItem::OpenTree("material".into()));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionRoomTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionRoomFloorTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionRoomWallTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionRoomCeilingTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_build_room_desc"),
        ));

        Self {
            id: TheId::named(&fl!("action_build_room")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_build_room_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn load_params(&mut self, map: &Map) {
        let Some(sector_id) = map.selected_sectors.first().copied() else {
            return;
        };
        let Some(points) = Self::ordered_world_points(map, sector_id) else {
            return;
        };
        let Some((a, b, floor_y, ceil_y)) = Self::base_edge(&points) else {
            return;
        };
        let width = Vec2::new(b.x - a.x, b.z - a.z).magnitude();
        let height = (ceil_y - floor_y).abs().max(0.1);
        self.nodeui.set_f32_value("actionRoomWidth", width);
        self.nodeui.set_f32_value("actionRoomHeight", height);
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        if server_ctx.editor_view_mode == EditorViewMode::D2
            || server_ctx.geometry_edit_mode == GeometryEditMode::Detail
        {
            return false;
        }
        if map.selected_sectors.len() != 1 || !map.selected_linedefs.is_empty() {
            return false;
        }
        let Some(sector_id) = map.selected_sectors.first().copied() else {
            return false;
        };
        map.get_surface_for_sector_id(sector_id)
            .map(|surface| surface.plane.normal.y.abs() <= 0.25)
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
        let source_sector_id = *map.selected_sectors.first()?;
        let source_sector = map.find_sector(source_sector_id)?.clone();
        let source_surface = map.get_surface_for_sector_id(source_sector_id)?.clone();
        let points = Self::ordered_world_points(map, source_sector_id)?;
        let (a, b, floor_y, wall_top_y) = Self::base_edge(&points)?;

        let source_width = Vec2::new(b.x - a.x, b.z - a.z).magnitude().max(0.1);
        let source_height = (wall_top_y - floor_y).abs().max(0.1);

        let room_type = self.nodeui.get_i32_value("actionRoomType").unwrap_or(0);
        let depth = self
            .nodeui
            .get_f32_value("actionRoomDepth")
            .unwrap_or(6.0)
            .max(0.1);
        let room_height = self
            .nodeui
            .get_f32_value("actionRoomHeight")
            .unwrap_or(source_height)
            .max(0.1);
        let width_mode = self.nodeui.get_i32_value("actionWidthMode").unwrap_or(0);
        let room_width = match width_mode {
            1 => {
                source_width
                    + self
                        .nodeui
                        .get_f32_value("actionRoomWidth")
                        .unwrap_or(source_width)
            }
            2 => self
                .nodeui
                .get_f32_value("actionRoomWidth")
                .unwrap_or(source_width),
            _ => source_width,
        }
        .max(0.1);
        let ceiling_mode = self.nodeui.get_i32_value("actionCeilingMode").unwrap_or(0);
        let keep_original_wall = self
            .nodeui
            .get_bool_value("actionKeepOriginalWall")
            .unwrap_or(false);
        let close_front_lip = self
            .nodeui
            .get_bool_value("actionCloseFrontLip")
            .unwrap_or(false);

        let tangent = Vec2::new(b.x - a.x, b.z - a.z).normalized();
        let mut normal = Vec2::new(source_surface.plane.normal.x, source_surface.plane.normal.z);
        if normal.magnitude_squared() < 1e-6 {
            normal = Vec2::new(-tangent.y, tangent.x);
        } else {
            normal = normal.normalized();
        }

        let local_poly = Self::local_polygon(room_type, room_width, depth);
        if local_poly.len() < 3 {
            return None;
        }

        let shader = source_sector.shader;
        let layer = source_sector.layer;

        let mut floor_props = source_sector.properties.clone();
        floor_props.set("generated_by", Value::Str("build_room".to_string()));
        floor_props.set("room_generated", Value::Bool(true));
        floor_props.set("source_sector", Value::Int(source_sector_id as i32));
        floor_props.set("room_kind", Value::Int(room_type));
        floor_props.set("room_part", Value::Str("floor".to_string()));
        floor_props.set("visible", Value::Bool(true));

        let mut wall_props = floor_props.clone();
        wall_props.set("room_part", Value::Str("wall".to_string()));

        let mut ceil_props = floor_props.clone();
        ceil_props.set("room_part", Value::Str("ceiling".to_string()));

        if let Some(v) = Self::parse_tile_source(
            &self
                .nodeui
                .get_text_value("actionRoomTileId")
                .unwrap_or_default(),
        ) {
            floor_props.set("source", v.clone());
            wall_props.set("source", v.clone());
            ceil_props.set("source", v);
        }

        if let Some(v) = Self::parse_tile_source(
            &self
                .nodeui
                .get_text_value("actionRoomFloorTileId")
                .unwrap_or_default(),
        ) {
            floor_props.set("source", v);
        }
        if let Some(v) = Self::parse_tile_source(
            &self
                .nodeui
                .get_text_value("actionRoomWallTileId")
                .unwrap_or_default(),
        ) {
            wall_props.set("source", v);
        }
        if let Some(v) = Self::parse_tile_source(
            &self
                .nodeui
                .get_text_value("actionRoomCeilingTileId")
                .unwrap_or_default(),
        ) {
            ceil_props.set("source", v);
        }

        let mut lip_props = wall_props.clone();
        lip_props.set("room_part", Value::Str("front_lip".to_string()));

        let floor_points: Vec<(f32, f32, f32)> = local_poly
            .iter()
            .map(|p| Self::world_from_local(a, tangent, normal, *p, floor_y))
            .collect();
        let ceiling_y = floor_y + room_height;
        let ceiling_points: Vec<(f32, f32, f32)> = local_poly
            .iter()
            .map(|p| Self::world_from_local(a, tangent, normal, *p, ceiling_y))
            .collect();

        let mut created = Vec::new();
        let floor_name = if source_sector.name.is_empty() {
            "Room Floor".to_string()
        } else {
            format!("{} Room Floor", source_sector.name)
        };
        if let Some(id) = Self::create_polygon_sector(
            map,
            &floor_points,
            &floor_props,
            &shader,
            &layer,
            &floor_name,
        ) {
            created.push(id);
        }

        for i in 0..local_poly.len() {
            let lp0 = local_poly[i];
            let lp1 = local_poly[(i + 1) % local_poly.len()];
            if !keep_original_wall && lp0.y.abs() <= 0.001 && lp1.y.abs() <= 0.001 {
                continue;
            }

            let p0 = Self::world_from_local(a, tangent, normal, lp0, floor_y);
            let p1 = Self::world_from_local(a, tangent, normal, lp1, floor_y);
            let p2 = Self::world_from_local(a, tangent, normal, lp1, ceiling_y);
            let p3 = Self::world_from_local(a, tangent, normal, lp0, ceiling_y);
            let wall = [p0, p1, p2, p3];
            let wall_name = if source_sector.name.is_empty() {
                "Room Wall".to_string()
            } else {
                format!("{} Room Wall", source_sector.name)
            };
            if let Some(id) =
                Self::create_polygon_sector(map, &wall, &wall_props, &shader, &layer, &wall_name)
            {
                created.push(id);
            }
        }

        if !keep_original_wall && close_front_lip && wall_top_y > ceiling_y + 0.001 {
            let lip = [
                (a.x, a.z, ceiling_y),
                (b.x, b.z, ceiling_y),
                (b.x, b.z, wall_top_y),
                (a.x, a.z, wall_top_y),
            ];
            let lip_name = if source_sector.name.is_empty() {
                "Room Front Lip".to_string()
            } else {
                format!("{} Room Front Lip", source_sector.name)
            };
            if let Some(id) =
                Self::create_polygon_sector(map, &lip, &lip_props, &shader, &layer, &lip_name)
            {
                created.push(id);
            }
        }

        if ceiling_mode == 0 {
            let ceil_name = if source_sector.name.is_empty() {
                "Room Ceiling".to_string()
            } else {
                format!("{} Room Ceiling", source_sector.name)
            };
            if let Some(id) = Self::create_polygon_sector(
                map,
                &ceiling_points,
                &ceil_props,
                &shader,
                &layer,
                &ceil_name,
            ) {
                created.push(id);
            }
        }

        if created.is_empty() {
            return None;
        }

        if !keep_original_wall {
            if let Some(sector) = map.find_sector_mut(source_sector_id) {
                sector.properties.set("visible", Value::Bool(false));
                sector
                    .properties
                    .set("room_entry_hidden", Value::Bool(true));
            }
        }

        map.selected_sectors = created;
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

    fn hud_material_slots(
        &self,
        _map: &Map,
        _server_ctx: &ServerContext,
    ) -> Option<Vec<ActionMaterialSlot>> {
        let room_text = self
            .nodeui
            .get_text_value("actionRoomTileId")
            .unwrap_or_default();
        let floor_text = self
            .nodeui
            .get_text_value("actionRoomFloorTileId")
            .unwrap_or_default();
        let wall_text = self
            .nodeui
            .get_text_value("actionRoomWallTileId")
            .unwrap_or_default();
        let ceiling_text = self
            .nodeui
            .get_text_value("actionRoomCeilingTileId")
            .unwrap_or_default();

        let room_source = Self::parse_tile_pixelsource(&room_text);

        Some(vec![
            ActionMaterialSlot {
                label: "ROOM".to_string(),
                source: room_source.clone(),
            },
            ActionMaterialSlot {
                label: "FLOOR".to_string(),
                source: Self::parse_tile_pixelsource(&floor_text).or(room_source.clone()),
            },
            ActionMaterialSlot {
                label: "WALL".to_string(),
                source: Self::parse_tile_pixelsource(&wall_text).or(room_source.clone()),
            },
            ActionMaterialSlot {
                label: "CEIL".to_string(),
                source: Self::parse_tile_pixelsource(&ceiling_text).or(room_source),
            },
        ])
    }

    fn set_hud_material_from_tile(
        &mut self,
        _map: &Map,
        _server_ctx: &ServerContext,
        slot_index: i32,
        tile_id: Uuid,
    ) -> bool {
        let value = tile_id.to_string();
        match slot_index {
            0 => self.nodeui.set_text_value("actionRoomTileId", value),
            1 => self.nodeui.set_text_value("actionRoomFloorTileId", value),
            2 => self.nodeui.set_text_value("actionRoomWallTileId", value),
            3 => self.nodeui.set_text_value("actionRoomCeilingTileId", value),
            _ => return false,
        }
        true
    }

    fn clear_hud_material_slot(
        &mut self,
        _map: &Map,
        _server_ctx: &ServerContext,
        slot_index: i32,
    ) -> bool {
        match slot_index {
            0 => self
                .nodeui
                .set_text_value("actionRoomTileId", String::new()),
            1 => self
                .nodeui
                .set_text_value("actionRoomFloorTileId", String::new()),
            2 => self
                .nodeui
                .set_text_value("actionRoomWallTileId", String::new()),
            3 => self
                .nodeui
                .set_text_value("actionRoomCeilingTileId", String::new()),
            _ => return false,
        }
        true
    }
}
