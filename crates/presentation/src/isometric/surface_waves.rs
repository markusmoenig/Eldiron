//! Equal-height floor cells address disjoint destination pixels: the shader
//! derives one world cell from each pixel before sampling or touching storage.
//! Triangles within a cell keep their order, with a pass barrier between waves.
use super::{SurfaceKind, SurfaceTriangle};
use rustc_hash::FxHashMap;

pub(super) fn floor_waves(
    triangles: &[SurfaceTriangle<'_>],
    regions: &[[i32; 4]],
) -> Option<Vec<Vec<usize>>> {
    let height = triangles.first()?.vertices[0][1].to_bits();
    if triangles.iter().any(|triangle| {
        triangle.kind != SurfaceKind::Floor
            || triangle.clip_cell.is_none()
            || triangle.vertices[0][1].to_bits() != height
    }) {
        return None;
    }
    Some(assign(triangles.iter().enumerate().filter_map(
        |(index, triangle)| {
            let region = regions[index];
            (region[0] < region[2] && region[1] < region[3])
                .then(|| (index, triangle.clip_cell.unwrap()))
        },
    )))
}
fn assign(cells: impl Iterator<Item = (usize, [i32; 2])>) -> Vec<Vec<usize>> {
    let mut counts = FxHashMap::<[i32; 2], usize>::default();
    let mut waves = Vec::<Vec<usize>>::new();
    for (index, cell) in cells {
        let next = counts.entry(cell).or_default();
        if *next == waves.len() {
            waves.push(Vec::new());
        }
        waves[*next].push(index);
        *next += 1;
    }
    waves
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn floor_waves_preserve_each_pixels_triangle_order_and_partition_disjoint_cells() {
        let cells = (0..1024)
            .flat_map(|x| (0..13).map(move |_| [x, -x]))
            .collect::<Vec<_>>();
        let waves = assign(cells.iter().copied().enumerate());
        assert_eq!(waves.len(), 13, "many cells must share passes");
        let mut observed = FxHashMap::<[i32; 2], Vec<usize>>::default();
        let mut total = 0;
        for wave in waves {
            let mut occupied = std::collections::HashSet::new();
            for index in wave {
                assert!(
                    occupied.insert(cells[index]),
                    "a pass cannot write one cell twice"
                );
                observed.entry(cells[index]).or_default().push(index);
                total += 1;
            }
        }
        assert_eq!(total, cells.len());
        for (cell, indices) in observed {
            assert_eq!(
                indices,
                cells
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| (*c == cell).then_some(i))
                    .collect::<Vec<_>>()
            );
        }
    }
}
