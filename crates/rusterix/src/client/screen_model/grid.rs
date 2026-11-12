//! Visible square grid iteration in widened coordinates, independent of layout policy.
use crate::Rect;
pub(super) fn visible_square_grid(
    origin: [f64; 2],
    columns_count: usize,
    count: usize,
    size: f64,
    step: f64,
    clip: Rect,
    offset: [f64; 2],
) -> impl Iterator<Item = (usize, Rect)> {
    let valid = [clip.x, clip.y, clip.width, clip.height]
        .into_iter()
        .all(f32::is_finite)
        && offset.into_iter().all(f64::is_finite)
        && columns_count > 0
        && size.is_finite()
        && size > 0.
        && step.is_finite()
        && step > 0.
        && clip.width > 0.
        && clip.height > 0.;
    let range = |start: f64, extent: f32, origin: f64, limit: usize| {
        if !valid {
            return 0..0;
        }
        let first = (((start - origin - size) / step).floor() + 1.).max(0.) as usize;
        let end = (((start + extent as f64 - origin) / step).ceil().max(0.) as usize).min(limit);
        first.min(end)..end
    };
    let columns = range(
        clip.x as f64 + offset[0],
        clip.width,
        origin[0],
        columns_count,
    );
    let rows = range(
        clip.y as f64 + offset[1],
        clip.height,
        origin[1],
        count.div_ceil(columns_count.max(1)),
    );
    rows.flat_map(move |row| {
        columns.clone().filter_map(move |column| {
            let index = row.checked_mul(columns_count)?.checked_add(column)?;
            if index >= count {
                return None;
            }
            Some((
                index,
                Rect::new(
                    (origin[0] + column as f64 * step - offset[0]) as f32,
                    (origin[1] + row as f64 * step - offset[1]) as f32,
                    size as f32,
                    size as f32,
                ),
            ))
        })
    })
}
