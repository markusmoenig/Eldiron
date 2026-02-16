use crate::prelude::*;
use rusterix::Surface;
use vek::Vec3;

pub struct ExtrudeLinedef {
    id: TheId,
    nodeui: TheNodeUI,
}

impl ExtrudeLinedef {
    pub fn extrude_linedef(
        &self,
        map: &mut Map,
        ld_id: u32,
        distance: f32,
        angle_deg: f32,
    ) -> Option<u32> {
        let ld = map.find_linedef(ld_id)?;
        let v0 = ld.start_vertex;
        let v1 = ld.end_vertex;

        let p0v = map.find_vertex(v0)?;
        let p1v = map.find_vertex(v1)?;
        let p0 = Vec3::new(p0v.x, p0v.y, p0v.z);
        let p1 = Vec3::new(p1v.x, p1v.y, p1v.z);

        // Rotate around the linedef axis (its tangent) by `angle` degrees.
        // Base direction is world +Z (map up). We first project it to be perpendicular to the axis
        // so rotation never "slides" along the edge.
        let axis = {
            let mut a = p1 - p0; // linedef tangent
            let len = a.magnitude();
            if len > 1e-6 {
                a /= len;
            } else {
                a = Vec3::new(1.0, 0.0, 0.0);
            }
            a
        };
        let mut base = Vec3::new(0.0, 0.0, 1.0); // world up (Z)
        // Make base perpendicular to axis
        base = base - axis * base.dot(axis);
        let blen = base.magnitude();
        if blen <= 1e-6 || !blen.is_finite() {
            // If the edge is parallel to +Z, pick +X as base and reproject
            base = Vec3::new(1.0, 0.0, 0.0) - axis * axis.dot(Vec3::new(1.0, 0.0, 0.0));
        }
        base = base.normalized();
        let ortho = axis.cross(base); // also perpendicular to axis, 90° from base

        let angle = angle_deg.to_radians();
        let dir = base * angle.cos() - ortho * angle.sin();

        let offset = dir * distance;
        let p1_top = p1 + offset;
        let p0_top = p0 + offset;

        // Create (or reuse) the new top vertices
        let v2 = map.add_vertex_at_3d(p1_top.x, p1_top.y, p1_top.z, false);
        let v3 = map.add_vertex_at_3d(p0_top.x, p0_top.y, p0_top.z, false);

        // Use manual linedef creation to avoid premature sector detection
        // (auto-detection can find wrong cycles when vertices are reused)
        map.possible_polygon = vec![];
        let _ = map.create_linedef_manual(v0, v1); // bottom
        let _ = map.create_linedef_manual(v1, v2); // side
        let _ = map.create_linedef_manual(v2, v3); // top
        let _ = map.create_linedef_manual(v3, v0); // side

        map.close_polygon_manual()
    }
}

impl Action for ExtrudeLinedef {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::FloatEditSlider(
            "actionDistance".into(),
            "".into(),
            "".into(),
            2.0,
            0.0..=0.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionAngle".into(),
            "".into(),
            "".into(),
            0.0,
            0.0..=360.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Markdown("desc".into(), "".into());
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_extrude_linedef")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_extrude_linedef_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        Some(TheAccelerator::new(TheAcceleratorKey::ALT, 'e'))
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        if server_ctx.editor_view_mode == EditorViewMode::D2 && server_ctx.editing_surface.is_some()
        {
            return false;
        }

        map.selected_sectors.is_empty() && !map.selected_linedefs.is_empty()
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let mut changed = false;
        let prev = map.clone();

        let distance = self.nodeui.get_f32_value("actionDistance").unwrap_or(2.0);
        let angle = self.nodeui.get_f32_value("actionAngle").unwrap_or(0.0);

        for linedef_id in &map.selected_linedefs.clone() {
            if let Some(sector_id) = self.extrude_linedef(map, *linedef_id, distance, angle) {
                let mut surface = Surface::new(sector_id);
                surface.calculate_geometry(map);
                map.surfaces.insert(surface.id, surface);

                changed = true;
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
