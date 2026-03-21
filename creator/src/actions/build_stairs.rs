use crate::prelude::*;
use rusterix::{PixelSource, Surface, Value, ValueContainer};
use vek::Vec3;

pub struct BuildStairs {
    id: TheId,
    nodeui: TheNodeUI,
}

impl BuildStairs {
    fn parse_tile_source(text: &str) -> Option<Value> {
        Some(Value::Source(parse_tile_id_pixelsource(text)?))
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

    fn linedef_endpoints(map: &Map, linedef_id: u32) -> Option<(Vec3<f32>, Vec3<f32>)> {
        let ld = map.find_linedef(linedef_id)?;
        Some((
            map.get_vertex_3d(ld.start_vertex)?,
            map.get_vertex_3d(ld.end_vertex)?,
        ))
    }

    fn lerp(a: Vec3<f32>, b: Vec3<f32>, t: f32) -> Vec3<f32> {
        a + (b - a) * t
    }

    fn edges_match(
        a0: Vec3<f32>,
        a1: Vec3<f32>,
        b0: Vec3<f32>,
        b1: Vec3<f32>,
        tolerance: f32,
    ) -> bool {
        ((a0 - b0).magnitude() <= tolerance && (a1 - b1).magnitude() <= tolerance)
            || ((a0 - b1).magnitude() <= tolerance && (a1 - b0).magnitude() <= tolerance)
    }

    fn align_edges(
        bottom: (Vec3<f32>, Vec3<f32>),
        top: (Vec3<f32>, Vec3<f32>),
    ) -> ((Vec3<f32>, Vec3<f32>), (Vec3<f32>, Vec3<f32>)) {
        let direct = (bottom.0 - top.0).magnitude() + (bottom.1 - top.1).magnitude();
        let flipped = (bottom.0 - top.1).magnitude() + (bottom.1 - top.0).magnitude();
        if flipped < direct {
            (bottom, (top.1, top.0))
        } else {
            (bottom, top)
        }
    }

    fn parse_tile_pixelsource(text: &str) -> Option<PixelSource> {
        match Self::parse_tile_source(text) {
            Some(Value::Source(source)) => Some(source),
            _ => None,
        }
    }
}

impl Action for BuildStairs {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::IntEditSlider(
            "actionSteps".into(),
            "".into(),
            "".into(),
            8,
            1..=64,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionSideWalls".into(),
            "".into(),
            "".into(),
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("material".into()));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionStairsTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionStairsTreadTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionStairsRiserTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionStairsSideTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_build_stairs_desc"),
        ));

        Self {
            id: TheId::named(&fl!("action_build_stairs")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_build_stairs_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn load_params(&mut self, map: &Map) {
        let selected = map.selected_linedefs.clone();
        let host_sector = selected
            .first()
            .and_then(|id| map.find_linedef(*id))
            .and_then(|ld| ld.sector_ids.first().copied())
            .or_else(|| {
                selected
                    .get(1)
                    .and_then(|id| map.find_linedef(*id))
                    .and_then(|ld| ld.sector_ids.first().copied())
            })
            .and_then(|sid| map.find_sector(sid));
        let Some(sector) = host_sector else {
            return;
        };

        self.nodeui.set_bool_value(
            "actionSideWalls",
            sector
                .properties
                .get_bool_default("stairs_fill_sides", false),
        );
        self.nodeui.set_text_value(
            "actionStairsTileId",
            source_to_text(sector.properties.get("stairs_tile_source")),
        );
        self.nodeui.set_text_value(
            "actionStairsTreadTileId",
            source_to_text(sector.properties.get("stairs_tread_source")),
        );
        self.nodeui.set_text_value(
            "actionStairsRiserTileId",
            source_to_text(sector.properties.get("stairs_riser_source")),
        );
        self.nodeui.set_text_value(
            "actionStairsSideTileId",
            source_to_text(sector.properties.get("stairs_side_source")),
        );
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        if server_ctx.editor_view_mode == EditorViewMode::D2
            || server_ctx.geometry_edit_mode == GeometryEditMode::Detail
        {
            return false;
        }
        map.selected_linedefs.len() == 2 && map.selected_sectors.is_empty()
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let prev = map.clone();
        let selected = map.selected_linedefs.clone();
        let a = *selected.first()?;
        let b = *selected.get(1)?;
        let (a0, a1) = Self::linedef_endpoints(map, a)?;
        let (b0, b1) = Self::linedef_endpoints(map, b)?;

        let a_avg_y = (a0.y + a1.y) * 0.5;
        let b_avg_y = (b0.y + b1.y) * 0.5;
        let (bottom, top) = if a_avg_y <= b_avg_y {
            ((a0, a1), (b0, b1))
        } else {
            ((b0, b1), (a0, a1))
        };
        let ((b0, b1), (t0, t1)) = Self::align_edges(bottom, top);

        let dir_bottom = b1 - b0;
        let dir_top = t1 - t0;
        let len_bottom = dir_bottom.magnitude();
        let len_top = dir_top.magnitude();
        if len_bottom < 0.01 || len_top < 0.01 {
            return None;
        }
        let cos_parallel = dir_bottom.normalized().dot(dir_top.normalized()).abs();
        if cos_parallel < 0.9 {
            return None;
        }

        let steps = self.nodeui.get_i32_value("actionSteps").unwrap_or(8).max(1) as usize;
        let side_walls = self
            .nodeui
            .get_bool_value("actionSideWalls")
            .unwrap_or(false);

        let source_sector = map
            .find_linedef(a)
            .and_then(|ld| ld.sector_ids.first().copied())
            .or_else(|| {
                map.find_linedef(b)
                    .and_then(|ld| ld.sector_ids.first().copied())
            })
            .and_then(|sid| map.find_sector(sid).cloned());

        let material_sector = source_sector
            .as_ref()
            .and_then(|sector| {
                if sector.properties.get_bool_default("cutout_handle", false) {
                    sector
                        .properties
                        .get_int("host_sector")
                        .and_then(|id| map.find_sector(id as u32).cloned())
                } else {
                    None
                }
            })
            .or_else(|| source_sector.clone());

        let props = material_sector
            .as_ref()
            .map(|s| s.properties.clone())
            .unwrap_or_default();
        let shader = material_sector.as_ref().and_then(|s| s.shader);
        let layer = material_sector.as_ref().and_then(|s| s.layer);
        let base_name = material_sector
            .as_ref()
            .map(|s| {
                if s.name.is_empty() {
                    "Stairs".to_string()
                } else {
                    format!("{} Stairs", s.name)
                }
            })
            .unwrap_or_else(|| "Stairs".to_string());

        let mut tread_props = props.clone();
        tread_props.set("generated_by", Value::Str("build_stairs".to_string()));
        tread_props.set("stairs_generated", Value::Bool(true));
        tread_props.set("stairs_part", Value::Str("tread".to_string()));
        tread_props.set("visible", Value::Bool(true));
        tread_props.set("stairs_source_a", Value::Int(a as i32));
        tread_props.set("stairs_source_b", Value::Int(b as i32));
        tread_props.set("stairs_steps", Value::Int(steps as i32));
        tread_props.set("stairs_fill_sides", Value::Bool(side_walls));
        if let Some(v) = Self::parse_tile_source(
            &self
                .nodeui
                .get_text_value("actionStairsTileId")
                .unwrap_or_default(),
        ) {
            tread_props.set("stairs_tile_source", v.clone());
            tread_props.set("source", v);
        }
        if let Some(v) = Self::parse_tile_source(
            &self
                .nodeui
                .get_text_value("actionStairsTreadTileId")
                .unwrap_or_default(),
        ) {
            tread_props.set("stairs_tread_source", v.clone());
            tread_props.set("source", v);
        } else if let Some(v) = tread_props.get("stairs_tread_source").cloned() {
            tread_props.set("source", v);
        } else if let Some(v) = tread_props.get("stairs_tile_source").cloned() {
            tread_props.set("source", v);
        }

        let mut riser_props = props.clone();
        riser_props.set("generated_by", Value::Str("build_stairs".to_string()));
        riser_props.set("stairs_generated", Value::Bool(true));
        riser_props.set("stairs_part", Value::Str("riser".to_string()));
        riser_props.set("visible", Value::Bool(true));
        riser_props.set("stairs_source_a", Value::Int(a as i32));
        riser_props.set("stairs_source_b", Value::Int(b as i32));
        riser_props.set("stairs_steps", Value::Int(steps as i32));
        riser_props.set("stairs_fill_sides", Value::Bool(side_walls));
        if let Some(v) = Self::parse_tile_source(
            &self
                .nodeui
                .get_text_value("actionStairsTileId")
                .unwrap_or_default(),
        ) {
            riser_props.set("stairs_tile_source", v.clone());
            riser_props.set("source", v);
        }
        if let Some(v) = Self::parse_tile_source(
            &self
                .nodeui
                .get_text_value("actionStairsRiserTileId")
                .unwrap_or_default(),
        ) {
            riser_props.set("stairs_riser_source", v.clone());
            riser_props.set("source", v);
        } else if let Some(v) = riser_props.get("stairs_riser_source").cloned() {
            riser_props.set("source", v);
        } else if let Some(v) = riser_props.get("stairs_tile_source").cloned() {
            riser_props.set("source", v);
        }

        let mut side_props = props.clone();
        side_props.set("generated_by", Value::Str("build_stairs".to_string()));
        side_props.set("stairs_generated", Value::Bool(true));
        side_props.set("stairs_part", Value::Str("side".to_string()));
        side_props.set("visible", Value::Bool(true));
        side_props.set("stairs_source_a", Value::Int(a as i32));
        side_props.set("stairs_source_b", Value::Int(b as i32));
        side_props.set("stairs_steps", Value::Int(steps as i32));
        side_props.set("stairs_fill_sides", Value::Bool(side_walls));
        if let Some(v) = Self::parse_tile_source(
            &self
                .nodeui
                .get_text_value("actionStairsTileId")
                .unwrap_or_default(),
        ) {
            side_props.set("stairs_tile_source", v.clone());
            side_props.set("source", v);
        }
        if let Some(v) = Self::parse_tile_source(
            &self
                .nodeui
                .get_text_value("actionStairsSideTileId")
                .unwrap_or_default(),
        ) {
            side_props.set("stairs_side_source", v.clone());
            side_props.set("source", v);
        } else if let Some(v) = side_props.get("stairs_side_source").cloned() {
            side_props.set("source", v);
        } else if let Some(v) = side_props.get("stairs_tile_source").cloned() {
            side_props.set("source", v);
        }

        let bottom_y = a_avg_y.min(b_avg_y);
        let top_y = a_avg_y.max(b_avg_y);
        let rise = (top_y - bottom_y) / steps as f32;

        let mut created = Vec::new();
        let mut left_profile = Vec::with_capacity(steps + 1);
        let mut right_profile = Vec::with_capacity(steps + 1);

        for i in 0..steps {
            let t0f = i as f32 / steps as f32;
            let t1f = (i + 1) as f32 / steps as f32;

            let front_left_base = Self::lerp(b0, t0, t0f);
            let front_right_base = Self::lerp(b1, t1, t0f);
            let back_left_base = Self::lerp(b0, t0, t1f);
            let back_right_base = Self::lerp(b1, t1, t1f);

            let tread_y = bottom_y + (i + 1) as f32 * rise;
            let lower_y = bottom_y + i as f32 * rise;

            let front_left = Vec3::new(front_left_base.x, tread_y, front_left_base.z);
            let front_right = Vec3::new(front_right_base.x, tread_y, front_right_base.z);
            let back_left = Vec3::new(back_left_base.x, tread_y, back_left_base.z);
            let back_right = Vec3::new(back_right_base.x, tread_y, back_right_base.z);

            let tread = [
                (front_left.x, front_left.z, front_left.y),
                (front_right.x, front_right.z, front_right.y),
                (back_right.x, back_right.z, back_right.y),
                (back_left.x, back_left.z, back_left.y),
            ];
            if let Some(id) = Self::create_polygon_sector(
                map,
                &tread,
                &tread_props,
                &shader,
                &layer,
                &format!("{base_name} Tread {}", i + 1),
            ) {
                created.push(id);
            }

            let riser_front_left = Vec3::new(front_left_base.x, lower_y, front_left_base.z);
            let riser_front_right = Vec3::new(front_right_base.x, lower_y, front_right_base.z);
            let riser = [
                (riser_front_left.x, riser_front_left.z, riser_front_left.y),
                (
                    riser_front_right.x,
                    riser_front_right.z,
                    riser_front_right.y,
                ),
                (front_right.x, front_right.z, front_right.y),
                (front_left.x, front_left.z, front_left.y),
            ];
            if let Some(id) = Self::create_polygon_sector(
                map,
                &riser,
                &riser_props,
                &shader,
                &layer,
                &format!("{base_name} Riser {}", i + 1),
            ) {
                created.push(id);
            }

            if i == 0 {
                left_profile.push(Vec3::new(front_left_base.x, lower_y, front_left_base.z));
                right_profile.push(Vec3::new(front_right_base.x, lower_y, front_right_base.z));
            }
            left_profile.push(back_left);
            right_profile.push(back_right);
        }

        if side_walls {
            for i in 0..steps {
                let a0p = left_profile[i];
                let a1p = left_profile[i + 1];
                let left_wall = [
                    (a0p.x, a0p.z, a0p.y),
                    (a1p.x, a1p.z, a1p.y),
                    (a1p.x, a1p.z, bottom_y),
                    (a0p.x, a0p.z, bottom_y),
                ];
                if let Some(id) = Self::create_polygon_sector(
                    map,
                    &left_wall,
                    &side_props,
                    &shader,
                    &layer,
                    &format!("{base_name} Left {}", i + 1),
                ) {
                    created.push(id);
                }

                let b0p = right_profile[i];
                let b1p = right_profile[i + 1];
                let right_wall = [
                    (b1p.x, b1p.z, b1p.y),
                    (b0p.x, b0p.z, b0p.y),
                    (b0p.x, b0p.z, bottom_y),
                    (b1p.x, b1p.z, bottom_y),
                ];
                if let Some(id) = Self::create_polygon_sector(
                    map,
                    &right_wall,
                    &side_props,
                    &shader,
                    &layer,
                    &format!("{base_name} Right {}", i + 1),
                ) {
                    created.push(id);
                }
            }
        }

        if created.is_empty() {
            None
        } else {
            let mut shaft_openings = Vec::new();
            let open_edges = [(t0, t1)];
            for sector in &map.sectors {
                if !sector.properties.get_bool_default("shaft_generated", false) {
                    continue;
                }
                let mut matched = false;
                for &linedef_id in &sector.linedefs {
                    let Some((s0, s1)) = Self::linedef_endpoints(map, linedef_id) else {
                        continue;
                    };
                    if open_edges
                        .iter()
                        .any(|(e0, e1)| Self::edges_match(s0, s1, *e0, *e1, 0.05))
                    {
                        matched = true;
                        break;
                    }
                }
                if matched {
                    shaft_openings.push(sector.id);
                }
            }

            for sector_id in shaft_openings {
                if let Some(sector) = map.find_sector_mut(sector_id) {
                    sector.properties.set("visible", Value::Bool(false));
                    sector.properties.set("shaft_opening", Value::Bool(true));
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
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn hud_material_slots(
        &self,
        _map: &Map,
        _server_ctx: &ServerContext,
    ) -> Option<Vec<ActionMaterialSlot>> {
        let all = self
            .nodeui
            .get_text_value("actionStairsTileId")
            .unwrap_or_default();
        let tread = self
            .nodeui
            .get_text_value("actionStairsTreadTileId")
            .unwrap_or_default();
        let riser = self
            .nodeui
            .get_text_value("actionStairsRiserTileId")
            .unwrap_or_default();
        let side = self
            .nodeui
            .get_text_value("actionStairsSideTileId")
            .unwrap_or_default();
        let all_source = Self::parse_tile_pixelsource(&all);
        Some(vec![
            ActionMaterialSlot {
                label: "STAIR".to_string(),
                source: all_source.clone(),
            },
            ActionMaterialSlot {
                label: "TREAD".to_string(),
                source: Self::parse_tile_pixelsource(&tread).or(all_source.clone()),
            },
            ActionMaterialSlot {
                label: "RISER".to_string(),
                source: Self::parse_tile_pixelsource(&riser).or(all_source.clone()),
            },
            ActionMaterialSlot {
                label: "SIDE".to_string(),
                source: Self::parse_tile_pixelsource(&side).or(all_source),
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
            0 => self.nodeui.set_text_value("actionStairsTileId", value),
            1 => self.nodeui.set_text_value("actionStairsTreadTileId", value),
            2 => self.nodeui.set_text_value("actionStairsRiserTileId", value),
            3 => self.nodeui.set_text_value("actionStairsSideTileId", value),
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
                .set_text_value("actionStairsTileId", String::new()),
            1 => self
                .nodeui
                .set_text_value("actionStairsTreadTileId", String::new()),
            2 => self
                .nodeui
                .set_text_value("actionStairsRiserTileId", String::new()),
            3 => self
                .nodeui
                .set_text_value("actionStairsSideTileId", String::new()),
            _ => return false,
        }
        true
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
