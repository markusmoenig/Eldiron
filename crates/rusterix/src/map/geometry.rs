use crate::Map;
use theframework::prelude::*;

/// Returns a HashMap<u32, (Vec<[f32; 2]>, Vec<(u32, u32, u32)>)> mapping each linedef ID
/// to a small mesh for that single linedef. Each mesh is a thick "quad" made by offsetting
/// perpendicular to the line direction.
#[allow(clippy::type_complexity)]
pub fn generate_line_segments_d2(
    map: &Map,
    linedef_ids: &[u32],
) -> Option<FxHashMap<u32, (Vec<[f32; 2]>, Vec<(usize, usize, usize)>)>> {
    if linedef_ids.is_empty() {
        return None;
    }

    let mut result = FxHashMap::default();

    for &id in linedef_ids {
        // 1) Lookup the linedef
        let linedef = map.find_linedef(id)?;
        // 2) Lookup the start/end vertices
        let v_start = map.find_vertex(linedef.start_vertex)?;
        let v_end = map.find_vertex(linedef.end_vertex)?;

        // 3) Convert to Vek Vec2
        let start_pos = Vec2::new(v_start.x, v_start.y);
        let end_pos = Vec2::new(v_end.x, v_end.y);

        // 4) Compute direction and perpendicular
        //    (Handle zero-length case if needed, e.g. "normalized_or_zero()")
        let dir = (end_pos - start_pos).normalized();
        let perp = Vec2::new(-dir.y, dir.x); // 90-degree rotation

        // 5) Offset for the thickness

        let half = linedef.properties.get_float_default("wall_width", 0.0) * 0.5;
        let start_top = start_pos + perp * half;
        let end_top = end_pos + perp * half;
        let start_bot = start_pos - perp * half;
        let end_bot = end_pos - perp * half;

        // We'll form a quad by duplicating 4 vertices:
        //    0 ----- 1   (top edge)
        //    |       |
        //    |       |
        //    3 ----- 2   (bottom edge)
        //
        // Two triangles: (0,1,3) and (1,2,3)

        let local_verts = vec![
            [start_top.x, start_top.y], // index 0
            [end_top.x, end_top.y],     // index 1
            [end_bot.x, end_bot.y],     // index 2
            [start_bot.x, start_bot.y], // index 3
        ];

        let local_inds = vec![(0, 1, 3), (1, 2, 3)];

        // 6) Insert into the HashMap keyed by this linedef ID
        result.insert(id, (local_verts, local_inds));
    }

    Some(result)
}
