use crate::prelude::*;
use rusterix::Surface;

pub struct CreateSector {
    id: TheId,
    nodeui: TheNodeUI,
}

impl CreateSector {
    /// Order vertex ids into a closed loop by sorting around the centroid (map XY).
    fn order_vertices_clockwise(map: &Map, verts: &[u32]) -> Option<Vec<u32>> {
        if verts.len() < 3 {
            return None;
        }

        // Collect unique positions and guard against duplicates
        let mut pts: Vec<(u32, f32, f32)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for &vid in verts {
            if seen.contains(&vid) {
                continue;
            }
            if let Some(v) = map.get_vertex(vid) {
                // In region maps, XY is the 2D plane; Z is up
                pts.push((vid, v.x, v.y));
                seen.insert(vid);
            }
        }
        if pts.len() < 3 {
            return None;
        }

        // Centroid in XY
        let (mut cx, mut cy) = (0.0f32, 0.0f32);
        for (_, x, y) in &pts {
            cx += *x;
            cy += *y;
        }
        let n = pts.len() as f32;
        cx /= n;
        cy /= n;

        // Sort by angle around centroid; we want CW to keep outer in our editor space
        pts.sort_by(|a, b| {
            let aa = (a.2 - cy).atan2(a.1 - cx);
            let bb = (b.2 - cy).atan2(b.1 - cx);
            bb.partial_cmp(&aa).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Drop nearly duplicate neighbors (epsilon)
        let eps = 1e-5f32;
        let mut ordered: Vec<u32> = Vec::with_capacity(pts.len());
        for (i, (vid, x, y)) in pts.iter().enumerate() {
            let prev = if i == 0 {
                pts.last().unwrap()
            } else {
                &pts[i - 1]
            };
            if (prev.1 - *x).abs() + (prev.2 - *y).abs() < eps {
                continue;
            }
            ordered.push(*vid);
        }
        if ordered.len() < 3 {
            return None;
        }
        Some(ordered)
    }

    /// Assemble an ordered closed loop from selected linedefs by walking adjacency.
    /// Returns ordered vertex ids (without repeating the first at the end).
    fn loop_from_linedefs(map: &Map, lds: &[u32]) -> Option<Vec<u32>> {
        use std::collections::{HashMap, HashSet};
        if lds.len() < 3 {
            return None;
        }

        // Build adjacency: vertex -> list of (linedef_id, other_vertex)
        let mut adj: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
        for &ld_id in lds {
            let ld = map.find_linedef(ld_id)?;
            let a = ld.start_vertex;
            let b = ld.end_vertex;
            adj.entry(a).or_default().push((ld_id, b));
            adj.entry(b).or_default().push((ld_id, a));
        }
        // Single simple cycle requires degree 2 at each vertex
        if adj.values().any(|v| v.len() != 2) {
            return None;
        }

        // Walk the cycle, flipping orientation on the fly
        let start_ld = lds[0];
        let ld0 = map.find_linedef(start_ld)?;
        let start_v = ld0.start_vertex; // arbitrary start
        let mut ordered: Vec<u32> = Vec::with_capacity(lds.len());
        let mut used: HashSet<u32> = HashSet::new();
        let mut curr_v = start_v;

        for _ in 0..lds.len() {
            // pick a next linedef incident to curr_v that is not used
            let next = adj
                .get(&curr_v)?
                .iter()
                .find(|(cand_ld, _)| !used.contains(cand_ld))
                .cloned()?;
            let (next_ld_id, other_v) = next;
            used.insert(next_ld_id);
            ordered.push(curr_v);
            curr_v = other_v;
        }
        // Close check
        if curr_v != ordered[0] {
            return None;
        }
        Some(ordered)
    }
}

impl Action for CreateSector {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();
        let item = TheNodeUIItem::Markdown("desc".into(), fl!("action_create_sector_desc"));
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_create_sector")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_create_sector_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, _server_ctx: &ServerContext) -> bool {
        map.selected_vertices.len() >= 3 || map.selected_linedefs.len() >= 3
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let mut changed = false;

        // Prefer linedefs if available; else use vertices
        let using_linedefs = !map.selected_linedefs.is_empty();
        if !using_linedefs && map.selected_vertices.len() < 3 {
            return None;
        }

        let prev = map.clone();
        map.possible_polygon.clear();

        let sector_id: Option<u32>;

        if using_linedefs {
            // Build an ordered vertex loop from selected linedefs
            let ordered = match Self::loop_from_linedefs(map, &map.selected_linedefs) {
                Some(v) => v,
                None => {
                    return None;
                }
            };
            // Use manual linedef creation to avoid premature/wrong cycle detection
            for i in 0..ordered.len() {
                let a = ordered[i];
                let b = ordered[(i + 1) % ordered.len()];
                if a == b {
                    continue;
                }
                let _ = map.create_linedef_manual(a, b);
            }
            // Now manually close the polygon
            sector_id = map.close_polygon_manual();
        } else {
            // Vertex-based loop (existing ordering)
            let ordered = match Self::order_vertices_clockwise(map, &map.selected_vertices) {
                Some(v) => v,
                None => {
                    return None;
                }
            };
            // Use manual linedef creation to avoid premature/wrong cycle detection
            for i in 0..ordered.len() {
                let a = ordered[i];
                let b = ordered[(i + 1) % ordered.len()];
                if a == b {
                    continue;
                }
                let _ = map.create_linedef_manual(a, b);
            }
            // Now manually close the polygon
            sector_id = map.close_polygon_manual();
        }

        if let Some(sector_id) = sector_id {
            map.selected_sectors.clear();
            map.selected_sectors.push(sector_id);
            map.possible_polygon.clear();

            let mut surface = Surface::new(sector_id);
            surface.calculate_geometry(map);
            map.surfaces.insert(surface.id, surface);

            changed = true;
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
