use crate::prelude::*;

/// Generate a ring of “stone” sectors around a door opening (profile UV).
/// - Operates in the active surface's profile map (editing_surface must be Some).
/// - Expects exactly one selected sector **in the profile** (the inner door loop).
pub struct GenerateStoneTrim {
    id: TheId,
    ui: TheNodeUI,
}

impl Action for GenerateStoneTrim {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut ui = TheNodeUI::default();

        // Ring / band
        ui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionTrimCourseWidth".into(),
            "Course width".into(),
            "Thickness of the stone band (UV units).".into(),
            0.25,
            0.01..=2.0,
            false,
        ));
        ui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionTrimJointWidth".into(),
            "Joint width".into(),
            "Gap between stones along the path (visual or geometric).".into(),
            0.0,
            0.0..=0.1,
            false,
        ));
        ui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionTrimClearance".into(),
            "Clearance".into(),
            "Gap from opening to stones (UV units).".into(),
            0.05,
            0.0..=0.5,
            false,
        ));

        // Stone sizing
        ui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionTrimBlockLenMin".into(),
            "Block length min".into(),
            "Minimum block length for straight jamb segments.".into(),
            0.40,
            0.10..=2.0,
            false,
        ));
        ui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionTrimBlockLenMax".into(),
            "Block length max".into(),
            "Maximum block length for straight jamb segments.".into(),
            0.80,
            0.10..=3.0,
            false,
        ));
        ui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionTrimKeystoneWidth".into(),
            "Keystone width".into(),
            "Width of the central arch keystone (UV).".into(),
            0.50,
            0.10..=2.0,
            false,
        ));

        // Finish / detailing
        ui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionTrimBevel".into(),
            "Bevel".into(),
            "Bevel size to apply per stone sector (profile op).".into(),
            0.02,
            0.0..=0.2,
            false,
        ));
        ui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionTrimDepth".into(),
            "Depth".into(),
            "Positive=relief (out), Negative=recess (in).".into(),
            0.03,
            -0.2..=0.2,
            false,
        ));

        // Distribution randomness
        ui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionTrimJitterPct".into(),
            "Jitter %".into(),
            "Random variation percentage for block lengths.".into(),
            0.0,
            0.0..=0.25,
            false,
        ));
        ui.add_item(TheNodeUIItem::IntEditSlider(
            "actionTrimSeed".into(),
            "Seed".into(),
            "Random seed for reproducible results.".into(),
            1337,
            i32::MIN..=i32::MAX,
            false,
        ));

        // Behavior toggles
        ui.add_item(TheNodeUIItem::Checkbox(
            "actionTrimFollowArch".into(),
            "Follow arch curve".into(),
            "Orient arch voussoirs orthogonal to the arch tangent.".into(),
            true,
        ));
        ui.add_item(TheNodeUIItem::Checkbox(
            "actionTrimEmitGaps".into(),
            "Emit geometric joints".into(),
            "If off, keep stones tight and let the material draw joints.".into(),
            false,
        ));

        ui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            "Generate a ring of stone blocks (jamb + arch voussoirs) around the selected **door opening** in the **active surface profile**.\n\
             The tool writes sectors into the profile map; your existing triangulation + relief/bevel ops handle the mesh.".into(),
        ));

        Self {
            id: TheId::named("Generate Stone Trim"),
            ui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> &'static str {
        "Create stone courses around a door opening in the current surface profile."
    }
    fn role(&self) -> ActionRole {
        ActionRole::Geometry
    }
    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        map.selected_sectors.len() == 1 && server_ctx.editor_view_mode == EditorViewMode::D2
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        // Preconditions: must be in 2D profile edit
        if server_ctx.editor_view_mode != EditorViewMode::D2 {
            return None;
        }

        // Snapshot for undo
        let prev = map.clone();

        // Exactly one selected sector (either a main-map sector or a profile sector)
        let selected_sector = {
            if map.selected_sectors.len() != 1 {
                return None;
            }
            map.selected_sectors[0]
        };

        // The current `map` **is** the profile map in D2 profile edit.
        // Use the single selected sector on this map as the inner door loop.
        let inner_sector_id = selected_sector;
        let profile: &mut Map = map;

        // Read params
        let course_w = self
            .ui
            .get_f32_value("actionTrimCourseWidth")
            .unwrap_or(0.25);
        let joint_w = self
            .ui
            .get_f32_value("actionTrimJointWidth")
            .unwrap_or(0.0)
            .max(0.0);
        let len_min = self
            .ui
            .get_f32_value("actionTrimBlockLenMin")
            .unwrap_or(0.40);
        let len_max = self
            .ui
            .get_f32_value("actionTrimBlockLenMax")
            .unwrap_or(0.80)
            .max(len_min);
        let keystone_w = self
            .ui
            .get_f32_value("actionTrimKeystoneWidth")
            .unwrap_or(0.50);
        let bevel = self
            .ui
            .get_f32_value("actionTrimBevel")
            .unwrap_or(0.02)
            .max(0.0);
        let depth = self.ui.get_f32_value("actionTrimDepth").unwrap_or(0.03);
        let jitter_pct = self
            .ui
            .get_f32_value("actionTrimJitterPct")
            .unwrap_or(0.0)
            .clamp(0.0, 0.25);
        let seed = self.ui.get_i32_value("actionTrimSeed").unwrap_or(1337);
        let follow_arch = self
            .ui
            .get_bool_value("actionTrimFollowArch")
            .unwrap_or(true);
        let emit_gaps = self
            .ui
            .get_bool_value("actionTrimEmitGaps")
            .unwrap_or(false);

        // 1) Pull inner loop (door) from the profile map (XY)
        let inner_loop = extract_sector_loop_xy(profile, inner_sector_id)?;
        if inner_loop.len() < 3 {
            return None;
        }

        // 2) Build a **centerline** offset outward by (clearance + half thickness)
        // Stones are standalone rectangles centered on this curve; they do NOT touch the inner loop.
        let clearance = self
            .ui
            .get_f32_value("actionTrimClearance")
            .unwrap_or(0.05)
            .max(0.0);
        let half_thick = 0.5 * course_w;
        let centerline = offset_polyline_miter(&inner_loop, clearance + half_thick);
        if centerline.len() != inner_loop.len() {
            return None;
        }

        // 3) Plan stone rectangles along the centerline
        let rects = plan_stone_rects(
            &inner_loop,
            &centerline,
            len_min,
            len_max,
            joint_w,
            jitter_pct,
            seed,
            half_thick,
        );

        // 4) Emit each rectangle as an independent sector (with a small outward epsilon)
        let mut made_any = false;
        for rect in rects.iter() {
            if let Some(sector_id) = emit_rect_sector(profile, rect) {
                set_block_ops(profile, sector_id, depth, bevel);
                made_any = true;
            }
        }

        if made_any {
            Some(RegionUndoAtom::MapEdit(
                Box::new(prev),
                Box::new(map.clone()),
            ))
        } else {
            None
        }
    }

    fn params(&self) -> TheNodeUI {
        self.ui.clone()
    }
    fn handle_event(&mut self, ev: &TheEvent) -> bool {
        self.ui.handle_event(ev)
    }
}

// ---------- Helpers & planning stubs (MVP) ----------
use vek::Vec2 as V2;

/// Extract the sector’s ordered XY loop from the profile map.
fn extract_sector_loop_xy(profile: &Map, sector_id: u32) -> Option<Vec<V2<f32>>> {
    let sector = profile.find_sector(sector_id)?;
    if sector.linedefs.is_empty() {
        return None;
    }
    let mut loop_xy = Vec::with_capacity(sector.linedefs.len());
    for &ld_id in &sector.linedefs {
        let ld = profile.find_linedef(ld_id)?;
        let v = profile.find_vertex(ld.start_vertex)?;
        loop_xy.push(V2::new(v.x, v.y));
    }
    Some(loop_xy)
}

#[inline]
fn v2_normalized(v: V2<f32>) -> V2<f32> {
    let l = (v.x * v.x + v.y * v.y).sqrt();
    if l > 1e-6 {
        V2::new(v.x / l, v.y / l)
    } else {
        V2::new(0.0, 0.0)
    }
}

#[inline]
fn v2_perp(v: V2<f32>) -> V2<f32> {
    V2::new(-v.y, v.x)
}

/// Signed area (shoelace). >0 => CCW, <0 => CW.
fn polygon_signed_area(poly: &[V2<f32>]) -> f32 {
    let n = poly.len();
    if n < 3 {
        return 0.0;
    }
    let mut a = 0.0f32;
    for i in 0..n {
        let p = poly[i];
        let q = poly[(i + 1) % n];
        a += p.x * q.y - q.x * p.y;
    }
    0.5 * a
}

/// Mitered offset of a closed polyline by `d`, **outward** relative to polygon interior.
/// If the inner loop is CCW, outward is to the right of each edge; if CW, outward is to the left.
fn offset_polyline_miter(inner: &[V2<f32>], d: f32) -> Vec<V2<f32>> {
    let n = inner.len();
    if n < 3 {
        return Vec::new();
    }

    let ccw = polygon_signed_area(inner) > 0.0;
    // Helper: outward normal for a unit tangent depending on winding
    let outward = |t: V2<f32>| -> V2<f32> {
        if ccw {
            V2::new(t.y, -t.x)
        } else {
            V2::new(-t.y, t.x)
        }
    };

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let p0 = inner[(i + n - 1) % n];
        let p1 = inner[i];
        let p2 = inner[(i + 1) % n];

        let t0 = v2_normalized(p1 - p0);
        let t1 = v2_normalized(p2 - p1);
        let n0 = outward(t0);
        let n1 = outward(t1);

        // Average the two outward normals and normalize to avoid spikes at near-180°
        let mut n_avg = n0 + n1;
        let len = (n_avg.x * n_avg.x + n_avg.y * n_avg.y).sqrt();
        if len > 1e-6 {
            n_avg /= len;
        } else {
            n_avg = n1;
        }

        out.push(p1 + n_avg * d);
    }
    out
}

#[derive(Clone)]
struct StoneRect {
    // CCW rectangle: p0->p1->p2->p3
    rect: [V2<f32>; 4],
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Plan rectangles centered on the outward offset curve. Each rect has length along tangent and thickness across normal.
fn plan_stone_rects(
    inner: &[V2<f32>],
    center: &[V2<f32>],
    len_min: f32,
    len_max: f32,
    joint_w: f32,
    jitter_pct: f32,
    seed: i32,
    half_thick: f32,
) -> Vec<StoneRect> {
    use rand::{Rng, SeedableRng, rngs::StdRng};
    let mut rng = StdRng::seed_from_u64(seed as u64);
    let mut rects = Vec::new();

    let n = center.len();
    if n < 3 {
        return rects;
    }

    let target = 0.5 * (len_min + len_max);
    let ccw = polygon_signed_area(inner) > 0.0;

    for i in 0..n {
        let ia = i;
        let ib = (i + 1) % n;
        let ca = center[ia];
        let cb = center[ib];

        let seg = cb - ca;
        let seg_len = (seg.x * seg.x + seg.y * seg.y).sqrt().max(1e-6);
        let t_hat = V2::new(seg.x / seg_len, seg.y / seg_len);
        let n_hat = if ccw {
            V2::new(t_hat.y, -t_hat.x)
        } else {
            V2::new(-t_hat.y, t_hat.x)
        };

        // Per-segment target with jitter
        let jitter = if jitter_pct > 0.0 {
            1.0 + rng.gen_range(-jitter_pct..=jitter_pct)
        } else {
            1.0
        };
        let mut block_len = (target * jitter).clamp(len_min, len_max);
        let mut count = (seg_len / block_len).round().max(1.0) as usize;
        block_len = seg_len / (count as f32);

        let half_len = 0.5 * (block_len - joint_w).max(0.0);

        let mut t0 = 0.0f32;
        for _ in 0..count {
            let t_c = (t0 + half_len / seg_len).min(1.0 - 1e-6);
            let c = V2::new(lerp(ca.x, cb.x, t_c), lerp(ca.y, cb.y, t_c));

            // Rectangle corners CCW: c -t -n, c +t -n, c +t +n, c -t +n
            let p0 = c - t_hat * half_len - n_hat * half_thick;
            let p1 = c + t_hat * half_len - n_hat * half_thick;
            let p2 = c + t_hat * half_len + n_hat * half_thick;
            let p3 = c - t_hat * half_len + n_hat * half_thick;

            rects.push(StoneRect {
                rect: [p0, p1, p2, p3],
            });
            t0 += block_len / seg_len;
        }
    }

    rects
}

/// Emit a rectangle as a standalone sector (independent vertices; clears polygon chain).
fn emit_rect_sector(profile: &mut Map, r: &StoneRect) -> Option<u32> {
    profile.possible_polygon.clear();
    let v_ids: Vec<u32> = r
        .rect
        .iter()
        .map(|p| profile.add_vertex_at_3d(p.x, p.y, 0.0, false))
        .collect();
    let mut created_sector: Option<u32> = None;
    for w in [
        (v_ids[0], v_ids[1]),
        (v_ids[1], v_ids[2]),
        (v_ids[2], v_ids[3]),
        (v_ids[3], v_ids[0]),
    ] {
        let (_ld, maybe) = profile.create_linedef(w.0, w.1);
        if let Some(s) = maybe {
            created_sector = Some(s);
        }
    }
    created_sector
}

/// Attach per-block relief/bevel properties (profile ops) to the sector.
fn set_block_ops(profile: &mut Map, sector_id: u32, depth: f32, bevel: f32) {
    if let Some(sec) = profile.find_sector_mut(sector_id) {
        // Generic properties — adapt names to your builder’s expectations if needed
        if depth > 0.0 {
            sec.properties
                .set("profile_op", Value::Str("relief".into()));
            sec.properties.set("relief_depth", Value::Float(depth));
        } else if depth < 0.0 {
            sec.properties
                .set("profile_op", Value::Str("recess".into()));
            sec.properties.set("recess_depth", Value::Float(-depth));
        } else {
            sec.properties.set("profile_op", Value::Str("inset".into()));
        }
        if bevel > 0.0 {
            sec.properties.set("bevel", Value::Float(bevel));
        }
        sec.properties.set("stone", Value::Bool(true));
    }
}
