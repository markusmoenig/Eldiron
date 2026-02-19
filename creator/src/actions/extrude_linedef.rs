use crate::prelude::*;
use rusterix::Surface;
use vek::Vec3;

pub struct ExtrudeLinedef {
    id: TheId,
    nodeui: TheNodeUI,
}

impl ExtrudeLinedef {
    fn hash01(mut x: u32) -> f32 {
        // Small deterministic hash for repeatable "random" profiles.
        x ^= x >> 16;
        x = x.wrapping_mul(0x7feb352d);
        x ^= x >> 15;
        x = x.wrapping_mul(0x846ca68b);
        x ^= x >> 16;
        (x as f32) / (u32::MAX as f32)
    }

    fn segment_height(style: i32, seg: u32, seg_count: u32, variation: f32, seed: u32) -> f32 {
        if seg == 0 || seg + 1 == seg_count {
            return 0.0;
        }
        match style {
            // Crenelated: alternating high/low battlements.
            1 => {
                if seg % 2 == 0 {
                    0.0
                } else {
                    variation
                }
            }
            // Random/broken: irregular dips.
            3 => variation * Self::hash01(seed ^ seg.wrapping_mul(1664525)),
            _ => 0.0,
        }
    }

    fn build_top_profile(
        p1_top: Vec3<f32>,
        p0_top: Vec3<f32>,
        offset: Vec3<f32>,
        style: i32,
        segment_size: f32,
        variation: f32,
        seed: u32,
    ) -> Vec<Vec3<f32>> {
        let dir_vec = p0_top - p1_top;
        let len = dir_vec.magnitude();
        if len <= 1e-5 {
            return vec![p1_top, p0_top];
        }
        if style == 0 {
            return vec![p1_top, p0_top];
        }

        let dir = dir_vec / len;
        let seg_size = segment_size.max(0.05);
        let seg_count = ((len / seg_size).ceil() as u32).max(2);
        let step = len / seg_count as f32;

        let up = if offset.magnitude() > 1e-5 {
            offset.normalized()
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        };
        let down = -up;

        let mut points = Vec::new();
        points.push(p1_top);

        if style == 2 {
            // Palisade: triangular spikes.
            for seg in 0..seg_count {
                let start_t = seg as f32 * step;
                let mid_t = start_t + step * 0.5;
                let end_t = (seg + 1) as f32 * step;
                let spike = variation.max(0.0);
                points.push(p1_top + dir * mid_t + up * spike);
                points.push(p1_top + dir * end_t);
            }
            return points;
        }

        // Crenelated/random: step profile with vertical jumps at segment boundaries.
        let mut curr_h = Self::segment_height(style, 0, seg_count, variation.max(0.0), seed);
        for b in 1..seg_count {
            let t = b as f32 * step;
            let boundary = p1_top + dir * t;
            let next_h = Self::segment_height(style, b, seg_count, variation.max(0.0), seed);

            points.push(boundary + down * curr_h);
            if (next_h - curr_h).abs() > 1e-5 {
                points.push(boundary + down * next_h);
            }
            curr_h = next_h;
        }
        points.push(p0_top + down * curr_h);
        if curr_h > 1e-5 {
            points.push(p0_top);
        }

        points
    }

    pub fn extrude_linedef(
        &self,
        map: &mut Map,
        ld_id: u32,
        distance: f32,
        angle_deg: f32,
        top_style: i32,
        segment_size: f32,
        top_variation: f32,
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

        let top_points = Self::build_top_profile(
            p1_top,
            p0_top,
            offset,
            top_style,
            segment_size,
            top_variation,
            ld_id,
        );

        // Build on duplicated base vertices so generated profile geometry never shares
        // the original host linedef. This allows safe replace/delete of generated sectors.
        let v0_base = map.add_vertex_at_3d(p0.x, p0.y, p0.z, false);
        let v1_base = map.add_vertex_at_3d(p1.x, p1.y, p1.z, false);

        // Use manual linedef creation to avoid premature sector detection
        // (auto-detection can find wrong cycles when vertices are reused)
        map.possible_polygon = vec![];
        let _ = map.create_linedef_manual(v0_base, v1_base); // bottom
        let mut prev = v1_base;
        for p in top_points {
            let v = map.add_vertex_at_3d(p.x, p.y, p.z, false);
            let _ = map.create_linedef_manual(prev, v);
            prev = v;
        }
        let _ = map.create_linedef_manual(prev, v0_base); // close side

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
        nodeui.add_item(TheNodeUIItem::OpenTree("top".into()));
        nodeui.add_item(TheNodeUIItem::Selector(
            "actionTopStyle".into(),
            "".into(),
            "".into(),
            vec![
                "flat".into(),
                "crenelated".into(),
                "palisade".into(),
                "random".into(),
            ],
            0,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionTopSegmentSize".into(),
            "".into(),
            "".into(),
            1.0,
            0.1..=8.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionTopVariation".into(),
            "".into(),
            "".into(),
            0.5,
            0.0..=4.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

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
        let top_style = self.nodeui.get_i32_value("actionTopStyle").unwrap_or(0);
        let segment_size = self
            .nodeui
            .get_f32_value("actionTopSegmentSize")
            .unwrap_or(1.0);
        let top_variation = self
            .nodeui
            .get_f32_value("actionTopVariation")
            .unwrap_or(0.5);

        for linedef_id in &map.selected_linedefs.clone() {
            if let Some(sector_id) = self.extrude_linedef(
                map,
                *linedef_id,
                distance,
                angle,
                top_style,
                segment_size,
                top_variation,
            ) {
                let mut surface = Surface::new(sector_id);
                surface.calculate_geometry(map);
                map.surfaces.insert(surface.id, surface);
                if let Some(sector) = map.find_sector_mut(sector_id) {
                    sector
                        .properties
                        .set("generated_profile", Value::Bool(true));
                    sector.properties.set(
                        "generated_profile_host_linedef",
                        Value::Int(*linedef_id as i32),
                    );
                }

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
