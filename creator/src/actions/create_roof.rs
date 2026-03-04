use crate::prelude::*;
use rusterix::PixelSource;
use rusterix::Surface;
use std::collections::{BTreeSet, HashSet};
use std::str::FromStr;

pub const CREATE_ROOF_ACTION_ID: &str = "9f4b34ad-2f43-4c31-9f41-9f5664c6d5e3";

pub struct CreateRoof {
    id: TheId,
    nodeui: TheNodeUI,
}

impl CreateRoof {
    fn parse_tile_source(text: &str) -> Option<Value> {
        let id = Uuid::parse_str(text.trim()).ok()?;
        Some(Value::Source(PixelSource::TileId(id)))
    }

    fn apply_sector_roof(&self, map: &mut Map, sector_id: u32) -> bool {
        let roof_name = self
            .nodeui
            .get_text_value("actionRoofName")
            .unwrap_or_else(|| "Roof".to_string());
        let roof_style = self
            .nodeui
            .get_i32_value("actionRoofStyle")
            .unwrap_or(1)
            .clamp(0, 2);
        let roof_height = self
            .nodeui
            .get_f32_value("actionRoofHeight")
            .unwrap_or(1.0)
            .max(0.0);
        let roof_overhang = self
            .nodeui
            .get_f32_value("actionRoofOverhang")
            .unwrap_or(0.0)
            .max(0.0);

        let tile_id_text = self
            .nodeui
            .get_text_value("actionRoofTileId")
            .unwrap_or_default();
        let side_tile_id_text = self
            .nodeui
            .get_text_value("actionRoofSideTileId")
            .unwrap_or_default();

        let Some(sector) = map.find_sector_mut(sector_id) else {
            return false;
        };

        if roof_height <= 0.0 {
            sector
                .properties
                .set("sector_feature", Value::Str("None".to_string()));
            sector.properties.remove("roof_name");
            sector.properties.remove("roof_style");
            sector.properties.remove("roof_height");
            sector.properties.remove("roof_overhang");
            sector.properties.remove("roof_tile_source");
            sector.properties.remove("roof_side_source");
            return true;
        }

        sector
            .properties
            .set("sector_feature", Value::Str("Roof".to_string()));
        sector.properties.set("roof_name", Value::Str(roof_name));
        sector.properties.set("roof_style", Value::Int(roof_style));
        sector
            .properties
            .set("roof_height", Value::Float(roof_height));
        sector
            .properties
            .set("roof_overhang", Value::Float(roof_overhang));

        if let Some(src) = Self::parse_tile_source(&tile_id_text) {
            sector.properties.set("roof_tile_source", src);
        } else {
            sector.properties.remove("roof_tile_source");
        }
        if let Some(src) = Self::parse_tile_source(&side_tile_id_text) {
            sector.properties.set("roof_side_source", src);
        } else {
            sector.properties.remove("roof_side_source");
        }

        true
    }

    fn clear_sector_roof(map: &mut Map, sector_id: u32) -> bool {
        let Some(sector) = map.find_sector_mut(sector_id) else {
            return false;
        };
        let had_roof = sector
            .properties
            .get_str_default("sector_feature", "None".to_string())
            == "Roof";
        if had_roof {
            sector
                .properties
                .set("sector_feature", Value::Str("None".to_string()));
            sector.properties.remove("roof_name");
            sector.properties.remove("roof_style");
            sector.properties.remove("roof_height");
            sector.properties.remove("roof_overhang");
            sector.properties.remove("roof_tile_source");
            sector.properties.remove("roof_side_source");
        }
        had_roof
    }

    fn sector_has_horizontal_loop(map: &Map, sector_id: u32) -> bool {
        for surface in map.surfaces.values() {
            if surface.sector_id != sector_id {
                continue;
            }
            if surface.plane.normal.y.abs() <= 0.7 {
                continue;
            }
            if let Some(loop_uv) = surface.sector_loop_uv(map)
                && loop_uv.len() >= 3
            {
                return true;
            }
        }
        false
    }

    fn sector_bbox_area(map: &Map, sector_id: u32) -> f32 {
        if let Some(sector) = map.find_sector(sector_id) {
            let bbox = sector.bounding_box(map);
            let sx = (bbox.max.x - bbox.min.x).abs();
            let sy = (bbox.max.y - bbox.min.y).abs();
            sx * sy
        } else {
            0.0
        }
    }

    fn selected_roof_sector_ids(&self, map: &Map) -> Vec<u32> {
        let selected: HashSet<u32> = map.selected_linedefs.iter().copied().collect();
        if selected.is_empty() {
            return vec![];
        }

        // Exact match first: when user selects the enclosing linedef loop,
        // prefer the sector that is built from exactly that loop.
        let mut exact: Vec<(u32, bool, f32)> = Vec::new(); // (sector_id, has_roof_feature, area)
        for sector in &map.sectors {
            if sector.linedefs.len() != selected.len() {
                continue;
            }
            if !sector.linedefs.iter().all(|id| selected.contains(id)) {
                continue;
            }
            if !Self::sector_has_horizontal_loop(map, sector.id) {
                continue;
            }
            let has_roof_feature = sector
                .properties
                .get_str_default("sector_feature", "None".to_string())
                == "Roof";
            exact.push((
                sector.id,
                has_roof_feature,
                Self::sector_bbox_area(map, sector.id),
            ));
        }
        if !exact.is_empty() {
            exact.sort_by(|a, b| {
                b.1.cmp(&a.1)
                    .then_with(|| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal))
            });
            return vec![exact[0].0];
        }

        // Prefer sectors that are clearly enclosed by the selected linedef set.
        let mut scored: Vec<(u32, bool, usize, usize, f32)> = Vec::new(); // (sector_id, has_roof_feature, hits, total, area)
        for sector in &map.sectors {
            let total = sector.linedefs.len();
            if total < 3 {
                continue;
            }
            let hits = sector
                .linedefs
                .iter()
                .filter(|id| selected.contains(id))
                .count();
            if hits >= 3 && Self::sector_has_horizontal_loop(map, sector.id) {
                let area = Self::sector_bbox_area(map, sector.id);
                let has_roof_feature = sector
                    .properties
                    .get_str_default("sector_feature", "None".to_string())
                    == "Roof";
                scored.push((sector.id, has_roof_feature, hits, total, area));
            }
        }

        if !scored.is_empty() {
            // Highest linedef hit-count first, then larger area, then fewer edges.
            scored.sort_by(|a, b| {
                b.2.cmp(&a.2)
                    .then_with(|| b.1.cmp(&a.1))
                    .then_with(|| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal))
                    .then(a.3.cmp(&b.3))
            });
            let best_hits = scored[0].2;
            let mut best: Option<(u32, bool, f32, usize)> = None; // (sector_id, has_roof_feature, area, total)
            for (sector_id, has_roof_feature, hits, total, area) in scored {
                if hits != best_hits {
                    continue;
                }
                match best {
                    None => best = Some((sector_id, has_roof_feature, area, total)),
                    Some((_id, best_roof, best_area, best_total)) => {
                        if (has_roof_feature && !best_roof)
                            || (has_roof_feature == best_roof
                                && (area > best_area || (area == best_area && total < best_total)))
                        {
                            best = Some((sector_id, has_roof_feature, area, total));
                        }
                    }
                }
            }
            if let Some((sector_id, _best_roof, _area, _total)) = best {
                return vec![sector_id];
            }
        }

        // Fallback: direct adjacency from selected linedefs.
        let mut ids: BTreeSet<u32> = BTreeSet::new();
        for linedef_id in &map.selected_linedefs {
            if let Some(linedef) = map.find_linedef(*linedef_id) {
                for sector_id in &linedef.sector_ids {
                    if Self::sector_has_horizontal_loop(map, *sector_id) {
                        ids.insert(*sector_id);
                    }
                }
            }
        }
        if ids.is_empty() {
            vec![]
        } else {
            // Keep fallback deterministic: prefer largest area horizontal sector.
            let mut best = 0u32;
            let mut best_area = f32::NEG_INFINITY;
            for id in ids {
                let area = Self::sector_bbox_area(map, id);
                if area > best_area {
                    best_area = area;
                    best = id;
                }
            }
            vec![best]
        }
    }

    fn create_sector_from_selected_linedefs(map: &mut Map) -> Option<u32> {
        if map.selected_linedefs.len() < 3 {
            return None;
        }

        // Build a directed chain from selected linedefs using their existing winding.
        let mut remaining: Vec<u32> = map.selected_linedefs.clone();
        let first_id = *remaining.first()?;
        let first = map.find_linedef(first_id)?;
        let start_vertex = first.start_vertex;
        let mut current_end = first.end_vertex;
        let mut ordered = vec![first_id];
        remaining.remove(0);

        while !remaining.is_empty() {
            let mut found_idx: Option<usize> = None;
            for (idx, id) in remaining.iter().enumerate() {
                if let Some(ld) = map.find_linedef(*id)
                    && ld.start_vertex == current_end
                {
                    found_idx = Some(idx);
                    current_end = ld.end_vertex;
                    ordered.push(*id);
                    break;
                }
            }
            if let Some(idx) = found_idx {
                remaining.remove(idx);
            } else {
                // Could not build a directed closed chain.
                return None;
            }
        }

        if current_end != start_vertex {
            return None;
        }

        map.possible_polygon = ordered;
        let sector_id = map.create_sector_from_polygon()?;

        // Ensure the newly enclosed sector has a generated surface so roof builder
        // can find a horizontal base loop immediately.
        let mut surface = Surface::new(sector_id);
        surface.calculate_geometry(map);
        map.surfaces.insert(surface.id, surface);

        Some(sector_id)
    }
}

impl Action for CreateRoof {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();

        nodeui.add_item(TheNodeUIItem::OpenTree("roof".into()));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionRoofName".into(),
            "".into(),
            "".into(),
            "Roof".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Selector(
            "actionRoofStyle".into(),
            "".into(),
            "".into(),
            vec!["flat".into(), "pyramid".into(), "gable".into()],
            1,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionRoofHeight".into(),
            "".into(),
            "".into(),
            1.0,
            0.0..=16.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionRoofOverhang".into(),
            "".into(),
            "".into(),
            0.0,
            0.0..=4.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("material".into()));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionRoofTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionRoofSideTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::Markdown("desc".into(), "".into()));

        Self {
            id: TheId::named_with_id(
                "Create Roof",
                Uuid::from_str(CREATE_ROOF_ACTION_ID).unwrap(),
            ),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        "Configure a non-destructive roof on sectors touched by selected linedefs.".to_string()
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            return false;
        }
        map.selected_sectors.is_empty() && !map.selected_linedefs.is_empty()
    }

    fn load_params(&mut self, map: &Map) {
        let sector_ids = self.selected_roof_sector_ids(map);
        let Some(sector_id) = sector_ids.first().copied() else {
            return;
        };
        let Some(sector) = map.find_sector(sector_id) else {
            return;
        };

        self.nodeui.set_text_value(
            "actionRoofName",
            sector
                .properties
                .get_str_default("roof_name", "Roof".to_string()),
        );
        self.nodeui.set_i32_value(
            "actionRoofStyle",
            sector
                .properties
                .get_int_default("roof_style", 1)
                .clamp(0, 2),
        );
        self.nodeui.set_f32_value(
            "actionRoofHeight",
            sector.properties.get_float_default("roof_height", 1.0),
        );
        self.nodeui.set_f32_value(
            "actionRoofOverhang",
            sector.properties.get_float_default("roof_overhang", 0.0),
        );

        let tile_id_text = match sector.properties.get("roof_tile_source") {
            Some(Value::Source(PixelSource::TileId(id))) => id.to_string(),
            _ => String::new(),
        };
        let side_tile_id_text = match sector.properties.get("roof_side_source") {
            Some(Value::Source(PixelSource::TileId(id))) => id.to_string(),
            _ => String::new(),
        };
        self.nodeui.set_text_value("actionRoofTileId", tile_id_text);
        self.nodeui
            .set_text_value("actionRoofSideTileId", side_tile_id_text);
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let prev = map.clone();
        let mut changed = false;

        let mut sector_ids = self.selected_roof_sector_ids(map);
        if sector_ids.is_empty() {
            if let Some(created) = Self::create_sector_from_selected_linedefs(map) {
                sector_ids = vec![created];
            }
        }
        for sector_id in &sector_ids {
            changed |= self.apply_sector_roof(map, *sector_id);
        }

        // Cleanup stale roof features on sibling sectors touched by the same selected linedefs.
        // This avoids stacked/mixed roofs after earlier mis-targeted applies.
        if !map.selected_linedefs.is_empty() && !sector_ids.is_empty() {
            let selected_set: HashSet<u32> = sector_ids.iter().copied().collect();
            let mut touched: BTreeSet<u32> = BTreeSet::new();
            for linedef_id in &map.selected_linedefs {
                if let Some(linedef) = map.find_linedef(*linedef_id) {
                    for sid in &linedef.sector_ids {
                        touched.insert(*sid);
                    }
                }
            }
            for sid in touched {
                if selected_set.contains(&sid) {
                    continue;
                }
                if Self::clear_sector_roof(map, sid) {
                    changed = true;
                }
            }
        }

        if changed {
            Some(ProjectUndoAtom::MapEdit(
                server_ctx.pc,
                Box::new(prev),
                Box::new(map.clone()),
            ))
        } else {
            None
        }
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
