use crate::prelude::*;
use rusterix::{Linedef, Sector, Surface};

pub const DUPLICATE_ACTION_ID: &str = "1468f85f-ef66-49f9-8c3f-54fbde6e3d9c";

pub struct Duplicate {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for Duplicate {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionDuplicateX".into(),
            "".into(),
            "".into(),
            0.0,
            -1000.0..=1000.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionDuplicateY".into(),
            "".into(),
            "".into(),
            1.0,
            -1000.0..=1000.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionDuplicateZ".into(),
            "".into(),
            "".into(),
            0.0,
            -1000.0..=1000.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::OpenTree("sector".into()));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionSectorConnect".into(),
            "".into(),
            "".into(),
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        Self {
            id: TheId::named_with_id(
                &fl!("action_duplicate"),
                Uuid::parse_str(DUPLICATE_ACTION_ID).unwrap(),
            ),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_duplicate_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, _server_ctx: &ServerContext) -> bool {
        !map.selected_vertices.is_empty()
            || !map.selected_linedefs.is_empty()
            || !map.selected_sectors.is_empty()
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        if map.selected_vertices.is_empty()
            && map.selected_linedefs.is_empty()
            && map.selected_sectors.is_empty()
        {
            return None;
        }

        let prev = map.clone();

        // Match editor XYZ convention: Y maps to vertex.z and Z maps to vertex.y (vertical).
        let offset_x = self.nodeui.get_f32_value("actionDuplicateX").unwrap_or(0.0);
        let offset_y = self.nodeui.get_f32_value("actionDuplicateY").unwrap_or(0.0);
        let offset_z = self.nodeui.get_f32_value("actionDuplicateZ").unwrap_or(1.0);
        let connect_sectors = self
            .nodeui
            .get_bool_value("actionSectorConnect")
            .unwrap_or(false);

        let mut selected_sector_ids = map.selected_sectors.clone();
        selected_sector_ids.sort_unstable();
        let mut selected_linedef_ids = map.selected_linedefs.clone();
        selected_linedef_ids.sort_unstable();
        let mut selected_vertex_ids = map.selected_vertices.clone();
        selected_vertex_ids.sort_unstable();
        selected_vertex_ids.dedup();

        let mut old_linedef_ids: FxHashSet<u32> = FxHashSet::default();
        for linedef_id in &selected_linedef_ids {
            old_linedef_ids.insert(*linedef_id);
        }
        for sector_id in &selected_sector_ids {
            if let Some(sector) = map.find_sector(*sector_id) {
                for linedef_id in &sector.linedefs {
                    old_linedef_ids.insert(*linedef_id);
                }
            }
        }

        let mut old_vertex_ids: FxHashSet<u32> = FxHashSet::default();
        for vertex_id in &selected_vertex_ids {
            old_vertex_ids.insert(*vertex_id);
        }
        for linedef_id in &old_linedef_ids {
            if let Some(linedef) = map.find_linedef(*linedef_id) {
                old_vertex_ids.insert(linedef.start_vertex);
                old_vertex_ids.insert(linedef.end_vertex);
            }
        }

        let mut sorted_vertex_ids: Vec<u32> = old_vertex_ids.into_iter().collect();
        sorted_vertex_ids.sort_unstable();

        let mut sorted_linedef_ids: Vec<u32> = old_linedef_ids.into_iter().collect();
        sorted_linedef_ids.sort_unstable();

        let mut next_vertex_id = map.vertices.iter().map(|v| v.id).max().unwrap_or(0);
        let mut next_linedef_id = map.linedefs.iter().map(|l| l.id).max().unwrap_or(0);
        let mut next_sector_id = map.sectors.iter().map(|s| s.id).max().unwrap_or(0);

        let mut vertex_map: FxHashMap<u32, u32> = FxHashMap::default();
        let mut linedef_map: FxHashMap<u32, u32> = FxHashMap::default();

        let mut new_vertices = Vec::new();
        let mut new_linedefs = Vec::new();
        let mut new_sectors = Vec::new();
        let mut sector_map: FxHashMap<u32, u32> = FxHashMap::default();

        for old_vid in sorted_vertex_ids {
            if let Some(old_vertex) = map.find_vertex(old_vid).cloned() {
                next_vertex_id = next_vertex_id.saturating_add(1);
                let new_id = next_vertex_id;
                let mut new_vertex = old_vertex;
                new_vertex.id = new_id;
                new_vertex.x += offset_x;
                new_vertex.y += offset_z;
                new_vertex.z += offset_y;
                vertex_map.insert(old_vid, new_id);
                new_vertices.push(new_vertex);
            }
        }

        for old_lid in sorted_linedef_ids {
            if let Some(old_linedef) = map.find_linedef(old_lid).cloned()
                && let (Some(&new_start), Some(&new_end)) = (
                    vertex_map.get(&old_linedef.start_vertex),
                    vertex_map.get(&old_linedef.end_vertex),
                )
            {
                next_linedef_id = next_linedef_id.saturating_add(1);
                let new_id = next_linedef_id;
                let mut new_linedef = old_linedef;
                new_linedef.id = new_id;
                new_linedef.start_vertex = new_start;
                new_linedef.end_vertex = new_end;
                new_linedef.sector_ids.clear();
                linedef_map.insert(old_lid, new_id);
                new_linedefs.push(new_linedef);
            }
        }

        for old_sid in &selected_sector_ids {
            if let Some(old_sector) = map.find_sector(*old_sid).cloned() {
                next_sector_id = next_sector_id.saturating_add(1);
                let new_id = next_sector_id;
                let mut new_sector = old_sector;
                new_sector.id = new_id;
                new_sector.linedefs = new_sector
                    .linedefs
                    .iter()
                    .filter_map(|id| linedef_map.get(id).copied())
                    .collect();
                new_sectors.push(new_sector);
                sector_map.insert(*old_sid, new_id);
            }
        }

        if connect_sectors {
            let selected_sector_set: FxHashSet<u32> = selected_sector_ids.iter().copied().collect();
            let mut connector_linedefs = Vec::new();
            let mut connector_sectors = Vec::new();

            for old_sid in &selected_sector_ids {
                let Some(old_sector) = map.find_sector(*old_sid).cloned() else {
                    continue;
                };
                if !sector_map.contains_key(old_sid) {
                    continue;
                }

                for old_linedef_id in old_sector.linedefs {
                    let Some(old_linedef) = map.find_linedef(old_linedef_id) else {
                        continue;
                    };
                    // Skip interior edges when duplicating multiple touching sectors.
                    let is_internal = old_linedef.sector_ids.len() > 1
                        && old_linedef
                            .sector_ids
                            .iter()
                            .all(|sid| selected_sector_set.contains(sid));
                    if is_internal {
                        continue;
                    }

                    let Some(&new_start) = vertex_map.get(&old_linedef.start_vertex) else {
                        continue;
                    };
                    let Some(&new_end) = vertex_map.get(&old_linedef.end_vertex) else {
                        continue;
                    };

                    next_linedef_id = next_linedef_id.saturating_add(1);
                    let bridge_side_a_id = next_linedef_id;
                    let mut bridge_side_a =
                        Linedef::new(bridge_side_a_id, old_linedef.end_vertex, new_end);

                    next_linedef_id = next_linedef_id.saturating_add(1);
                    let bridge_side_b_id = next_linedef_id;
                    let mut bridge_side_b =
                        Linedef::new(bridge_side_b_id, new_start, old_linedef.start_vertex);

                    // Use a dedicated reversed copy of the duplicated top edge so the connector
                    // sector keeps a proper vertex loop order (A -> B -> B' -> A').
                    next_linedef_id = next_linedef_id.saturating_add(1);
                    let bridge_top_id = next_linedef_id;
                    let mut bridge_top = Linedef::new(bridge_top_id, new_end, new_start);

                    next_sector_id = next_sector_id.saturating_add(1);
                    let connector_sector_id = next_sector_id;
                    bridge_side_a.sector_ids.push(connector_sector_id);
                    bridge_side_b.sector_ids.push(connector_sector_id);
                    bridge_top.sector_ids.push(connector_sector_id);

                    let connector_sector = Sector::new(
                        connector_sector_id,
                        vec![
                            old_linedef_id,
                            bridge_side_a_id,
                            bridge_top_id,
                            bridge_side_b_id,
                        ],
                    );

                    connector_linedefs.push(bridge_side_a);
                    connector_linedefs.push(bridge_side_b);
                    connector_linedefs.push(bridge_top);
                    connector_sectors.push(connector_sector);
                }
            }

            new_linedefs.extend(connector_linedefs);
            new_sectors.extend(connector_sectors);
        }

        for new_sector in &new_sectors {
            for new_linedef_id in &new_sector.linedefs {
                if let Some(new_linedef) = new_linedefs.iter_mut().find(|l| l.id == *new_linedef_id)
                    && !new_linedef.sector_ids.contains(&new_sector.id)
                {
                    new_linedef.sector_ids.push(new_sector.id);
                } else if let Some(existing_linedef) = map.find_linedef_mut(*new_linedef_id)
                    && !existing_linedef.sector_ids.contains(&new_sector.id)
                {
                    existing_linedef.sector_ids.push(new_sector.id);
                }
            }
        }

        if new_vertices.is_empty() && new_linedefs.is_empty() && new_sectors.is_empty() {
            return None;
        }

        map.vertices.extend(new_vertices.clone());
        map.linedefs.extend(new_linedefs.clone());
        map.sectors.extend(new_sectors.clone());

        // Ensure duplicated/connector sectors have matching surfaces so they render in 3D.
        for sector in &new_sectors {
            if map.get_surface_for_sector_id(sector.id).is_none() {
                let mut surface = if let Some((&old_sid, _)) = sector_map
                    .iter()
                    .find(|(_, new_sid)| **new_sid == sector.id)
                {
                    if let Some(src_surface) = map.get_surface_for_sector_id(old_sid) {
                        let mut cloned = src_surface.clone();
                        cloned.id = Uuid::new_v4();
                        cloned.sector_id = sector.id;
                        cloned
                    } else {
                        Surface::new(sector.id)
                    }
                } else {
                    Surface::new(sector.id)
                };
                surface.calculate_geometry(map);
                map.surfaces.insert(surface.id, surface);
            }
        }

        map.selected_vertices = selected_vertex_ids
            .iter()
            .filter_map(|id| vertex_map.get(id).copied())
            .collect();

        map.selected_linedefs = selected_linedef_ids
            .iter()
            .filter_map(|id| linedef_map.get(id).copied())
            .collect();

        // For selected sectors we duplicated all their linedefs, so one-to-one order mapping is valid.
        map.selected_sectors = new_sectors.iter().map(|s| s.id).collect();

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
