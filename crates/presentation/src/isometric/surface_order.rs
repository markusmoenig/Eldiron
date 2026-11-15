//! Dependency waves for per-pixel surface operations. Disjoint screen regions
//! commute; every overlapping pair retains its original order across dispatches.
// Fine buckets reduce false dependencies for dense/subpixel terrain, especially
// fitted overviews. Planning still touches bounded rectangle metadata only;
// large footprints retain the conservative global fence below.
const TILE: i32 = 2;
const MAX_TILES_PER_REGION: usize = 512;

pub(super) fn waves(regions: &[[i32; 4]], size: (u32, u32)) -> Vec<Vec<usize>> {
    let columns = size.0.div_ceil(TILE as u32) as usize;
    let rows = size.1.div_ceil(TILE as u32) as usize;
    let mut previous = vec![0usize; columns * rows];
    let mut result = Vec::<Vec<usize>>::new();
    let mut fence = 0;
    let mut highest = 0;
    for (index, &[x0, y0, x1, y1]) in regions.iter().enumerate() {
        if x0 >= x1 || y0 >= y1 {
            continue;
        }
        // Renderer bounds are already clamped to this canvas. Inclusive tile
        // endpoints cover every half-open dispatch pixel, without rasterizing
        // destination pixels on the CPU.
        let (left, right) = (
            x0 as usize / TILE as usize,
            (x1 - 1) as usize / TILE as usize,
        );
        let (top, bottom) = (
            y0 as usize / TILE as usize,
            (y1 - 1) as usize / TILE as usize,
        );
        let area = (right - left + 1) * (bottom - top + 1);
        let wave = if area > MAX_TILES_PER_REGION {
            // A large footprint is a conservative global fence, keeping CPU
            // planning bounded rather than visiting a full canvas per region.
            fence = highest + 1;
            fence
        } else {
            let mut dependency = fence;
            for y in top..=bottom {
                for x in left..=right {
                    dependency = dependency.max(previous[y * columns + x]);
                }
            }
            let wave = dependency + 1;
            for y in top..=bottom {
                for x in left..=right {
                    previous[y * columns + x] = wave;
                }
            }
            wave
        };
        highest = highest.max(wave);
        result.resize_with(highest, Vec::new);
        result[wave - 1].push(index);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn region_waves_retain_all_overlap_dependencies_and_share_disjoint_work() {
        let mut seed = 7u32;
        let mut next = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            seed
        };
        let mut regions = Vec::new();
        for index in 0..512 {
            let x = (next() % 230) as i32;
            let y = (next() % 230) as i32;
            regions.push(if index % 31 == 0 {
                [0, 0, 0, 0]
            } else if index % 23 == 0 {
                [0, 0, 256, 256]
            } else {
                [
                    x,
                    y,
                    x + 1 + (next() % 20) as i32,
                    y + 1 + (next() % 20) as i32,
                ]
            });
        }
        let result = waves(&regions, (256, 256));
        let mut assigned = vec![None; regions.len()];
        for (wave, entries) in result.iter().enumerate() {
            for &index in entries {
                assert!(assigned[index].replace(wave).is_none());
            }
        }
        for (a, first) in regions.iter().enumerate() {
            let empty = first[0] >= first[2] || first[1] >= first[3];
            assert_eq!(assigned[a].is_none(), empty);
            for (b, second) in regions.iter().enumerate().skip(a + 1) {
                let overlap = first[0] < second[2]
                    && second[0] < first[2]
                    && first[1] < second[3]
                    && second[1] < first[3]
                    && second[0] < second[2]
                    && second[1] < second[3];
                if !empty && overlap {
                    assert!(assigned[a].unwrap() < assigned[b].unwrap());
                }
            }
        }
        let disjoint = (0..1024)
            .map(|i| {
                let x = i % 32 * 8;
                let y = i / 32 * 8;
                [x, y, x + 4, y + 4]
            })
            .collect::<Vec<_>>();
        assert_eq!(waves(&disjoint, (256, 256)).len(), 1);
    }
}
