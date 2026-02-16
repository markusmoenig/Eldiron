use crate::prelude::*;

pub struct AddArch {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for AddArch {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::FloatEditSlider(
            "actionArchHeight".into(),
            "".into(),
            "".into(),
            1.0,
            0.1..=2.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::IntEditSlider(
            "actionArchSegments".into(),
            "".into(),
            "".into(),
            12,
            4..=64,
            false,
        );
        nodeui.add_item(item);

        nodeui.add_item(TheNodeUIItem::Markdown("desc".into(), "".into()));

        Self {
            id: TheId::named(&fl!("action_add_arch")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_add_arch_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        !map.selected_linedefs.is_empty() && server_ctx.editor_view_mode == EditorViewMode::D2
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        if map.selected_linedefs.is_empty() {
            return None;
        }
        let prev = map.clone();
        let mut changed = false;

        // read UI params
        let height = self.nodeui.get_f32_value("actionArchHeight").unwrap_or(1.0) as f32;
        let segments = self
            .nodeui
            .get_i32_value("actionArchSegments")
            .unwrap_or(12)
            .clamp(4, 64) as u32;

        for ld_id in map.selected_linedefs.clone() {
            if let Some(ld) = map.find_linedef(ld_id).cloned() {
                // Remember original sector bindings and properties
                let orig_sector_ids = ld.sector_ids.clone();
                let orig_props = ld.properties.clone();

                // Fetch endpoints in 3D (Y up)
                if let (Some(a), Some(b)) = (
                    map.find_vertex(ld.start_vertex).cloned(),
                    map.find_vertex(ld.end_vertex).cloned(),
                ) {
                    // Work in XY plane only; do not displace Z
                    let ax = a.x;
                    let ay = a.y;
                    let az = a.z;
                    let bx = b.x;
                    let by = b.y;
                    let bz = b.z;

                    // 2D tangent and perpendicular normal in XY
                    let dx = bx - ax;
                    let dy = by - ay;
                    let mut nx = -dy;
                    let mut ny = dx;
                    let len = (nx * nx + ny * ny).sqrt();
                    if len > 1e-6 {
                        nx /= len;
                        ny /= len;
                    } else {
                        nx = 0.0;
                        ny = 1.0;
                    }

                    // Midpoint of the segment (in XY)
                    let midx = (ax + bx) * 0.5;
                    let midy = (ay + by) * 0.5;

                    // --- Flip normal OUTWARD if we can infer an inside sector ---
                    // Prefer a sector that is currently selected; otherwise use the first attached sector.
                    let mut ref_sector_id: Option<u32> = None;
                    if let Some(sel) = ld
                        .sector_ids
                        .iter()
                        .find(|sid| map.selected_sectors.contains(sid))
                    {
                        ref_sector_id = Some(*sel);
                    } else if let Some(first) = ld.sector_ids.first() {
                        ref_sector_id = Some(*first);
                    }

                    if let Some(sec_id) = ref_sector_id {
                        if let Some(sector) = map.find_sector(sec_id) {
                            // Compute a lightweight centroid in XY from the sector's vertex loop
                            let mut cx_sum = 0.0f32;
                            let mut cy_sum = 0.0f32;
                            let mut cnt = 0usize;
                            for &edge_id in &sector.linedefs {
                                if let Some(edge) = map.find_linedef(edge_id) {
                                    if let Some(v) = map.find_vertex(edge.start_vertex) {
                                        cx_sum += v.x;
                                        cy_sum += v.y;
                                        cnt += 1;
                                    }
                                }
                            }
                            if cnt > 0 {
                                let scx = cx_sum / (cnt as f32);
                                let scy = cy_sum / (cnt as f32);
                                // Vector from mid to sector centroid
                                let vx = scx - midx;
                                let vy = scy - midy;
                                // If normal points TOWARD the sector (dot > 0), flip to point outward
                                if nx * vx + ny * vy > 0.0 {
                                    nx = -nx;
                                    ny = -ny;
                                }
                            }
                        }
                    }

                    // Quadratic Bezier control point in XY (midpoint + n*height)
                    let cx = midx + nx * height;
                    let cy = midy + ny * height;

                    // Create interior points along the Bezier in XY; Z is lerped only (no displacement)
                    let step = 1.0_f32 / (segments as f32);
                    let mut new_vertex_ids: Vec<u32> = Vec::new();
                    for i in 1..segments {
                        // interior only
                        let t = step * (i as f32);
                        let one_t = 1.0 - t;
                        let px = ax * (one_t * one_t) + cx * (2.0 * one_t * t) + bx * (t * t);
                        let py = ay * (one_t * one_t) + cy * (2.0 * one_t * t) + by * (t * t);
                        let pz = az + (bz - az) * t; // maintain continuity; no Z bulge
                        let vid = map.add_vertex_at_3d(px, py, pz, false);
                        new_vertex_ids.push(vid);
                    }

                    // Build the new vertex chain including endpoints
                    let mut chain: Vec<u32> = Vec::with_capacity(segments as usize + 1);
                    chain.push(ld.start_vertex);
                    chain.extend(new_vertex_ids.iter().copied());
                    chain.push(ld.end_vertex);

                    // Phase 1: create/reuse linedefs for each consecutive pair in the chain via Map API (no sector borrow yet)
                    // Ensure standalone creation; don't chain with prior edges
                    map.possible_polygon.clear();

                    let mut new_ids: Vec<u32> = Vec::with_capacity(segments as usize);
                    for w in chain.windows(2) {
                        // Use manual creation to avoid unwanted sector auto-detection
                        let new_ld_id = map.create_linedef_manual(w[0], w[1]);
                        // Copy properties & bind sectors like the original linedef
                        if let Some(nld) = map.find_linedef_mut(new_ld_id) {
                            nld.properties = orig_props.clone();
                            nld.sector_ids = orig_sector_ids.clone();
                        }
                        new_ids.push(new_ld_id);
                    }
                    // Clear possible_polygon since we don't want to create a sector
                    map.possible_polygon.clear();

                    // Phase 2: splice the new chain into every sector that referenced the old linedef
                    let mut touched_sectors: Vec<u32> = Vec::new();
                    for sector in map.sectors.iter_mut() {
                        if let Some(pos) = sector.linedefs.iter().position(|&id| id == ld_id) {
                            sector.linedefs.splice(pos..=pos, new_ids.iter().copied());
                            if !touched_sectors.contains(&sector.id) {
                                touched_sectors.push(sector.id);
                            }
                        }
                    }

                    // Update new linedefs with sector memberships
                    let mut new_sector_ids = orig_sector_ids.clone();
                    for sid in touched_sectors {
                        if !new_sector_ids.contains(&sid) {
                            new_sector_ids.push(sid);
                        }
                    }
                    for nid in &new_ids {
                        if let Some(nld) = map.find_linedef_mut(*nid) {
                            nld.sector_ids = new_sector_ids.clone();
                        }
                    }

                    // Update selection once (remove old id, add new chain) after sector updates
                    if let Some(pos_sel) = map.selected_linedefs.iter().position(|&id| id == ld_id)
                    {
                        map.selected_linedefs.remove(pos_sel);
                    }
                    for nid in &new_ids {
                        if !map.selected_linedefs.contains(nid) {
                            map.selected_linedefs.push(*nid);
                        }
                    }

                    // Remove old id from any sectors just in case
                    for sector in map.sectors.iter_mut() {
                        if let Some(pos) = sector.linedefs.iter().position(|&id| id == ld_id) {
                            sector.linedefs.remove(pos);
                        }
                    }

                    // Also drop the old linedef from the map's linedef list to avoid drawing it standalone
                    map.linedefs.retain(|l| l.id != ld_id);

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
