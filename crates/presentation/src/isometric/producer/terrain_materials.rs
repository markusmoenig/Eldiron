//! Material pages used by retained terrain. Positions still sample the same
//! world-space page table and the shared nearest-texel shader.
use std::collections::BTreeSet;

use super::super::material_sources::{material_page_headers, material_page_texels};

use rusterix::Tile;

pub(super) const BANK_BIT: u32 = 1 << 31;
pub(super) const NO_MATERIAL: u32 = u32::MAX;

pub(super) struct TerrainMaterial<'a> {
    tile: &'a Tile,
    pages: BTreeSet<usize>,
}

impl<'a> TerrainMaterial<'a> {
    pub(super) fn new(tile: &'a Tile) -> Self {
        Self {
            tile,
            pages: BTreeSet::new(),
        }
    }

    pub(super) fn include_cell(&mut self, [x, z]: [i32; 2]) {
        let Some(layout) = self.tile.uv_pages.as_ref().filter(|layout| {
            !layout.size.contains(&0)
                && !layout
                    .repeat_period
                    .iter()
                    .all(|v| v.is_finite() && *v > 0.)
        }) else {
            self.pages.insert(0);
            return;
        };
        // Include adjacent pages at both endpoints. Raster barycentrics can
        // put a triangle's world coordinate just across its integer cell edge.
        let units = i64::from(layout.units_per_page.max(1));
        let page = |coordinate: i64, axis: usize| {
            (coordinate.div_euclid(units) - i64::from(layout.origin[axis]))
                .clamp(0, i64::from(layout.size[axis]) - 1) as usize
        };
        let width = layout.size[0] as usize;
        for row in page(i64::from(z) - 1, 1)..=page(i64::from(z) + 1, 1) {
            let start = row * width;
            self.pages
                .extend(start + page(i64::from(x) - 1, 0)..=start + page(i64::from(x) + 1, 0));
        }
    }

    // Headers and phase addressing match IsometricMaterialSources. Missing
    // entries remain zero, and offsets within each phase stay relative to it.
    fn layout(&self) -> Option<([[u32; 4]; 3], usize)> {
        let Some(pages) = &self.tile.uv_pages else {
            return Some((
                material_page_headers(self.tile, 1),
                self.tile.textures.len().max(1),
            ));
        };
        let count = pages.size[0]
            .checked_mul(pages.size[1])
            .filter(|&count| count > 0)?;
        Some((
            material_page_headers(self.tile, count),
            pages.animation_frames.max(1) as usize,
        ))
    }

    fn word_count(&self, pages: usize, phases: usize, capacity: usize) -> Option<usize> {
        let mut words = phases.div_ceil(4).checked_add(1)?;
        words = words.checked_add(pages.checked_add(3)?.checked_mul(phases)?)?;
        if words > capacity {
            return None;
        }
        for phase in 0..phases {
            for &page in &self.pages {
                if let Some(texture) = self
                    .tile
                    .textures
                    .get(phase.checked_mul(pages)?.checked_add(page)?)
                {
                    words = words.checked_add(texture.width.checked_mul(texture.height)?)?;
                    if words > capacity {
                        return None;
                    }
                }
            }
        }
        Some(words)
    }

    fn append_to(
        &self,
        bank: &mut Vec<[u32; 4]>,
        header: [[u32; 4]; 3],
        phases: usize,
    ) -> Option<u32> {
        let directory = bank.len();
        bank.resize(directory + 1 + phases.div_ceil(4), [0; 4]);
        bank[directory][0] = u32::try_from(phases).ok()?;
        let count = header[0][0] as usize;
        for phase in 0..phases {
            let start = bank.len();
            bank[directory + 1 + phase / 4][phase % 4] = start as u32;
            bank.extend(header);
            bank.resize(start + 3 + count, [0; 4]);
            for &page in &self.pages {
                let index = phase * count + page;
                let Some(texture) = self.tile.textures.get(index) else {
                    continue;
                };
                bank[start + 3 + page] = [
                    (bank.len() - start) as u32,
                    u32::try_from(texture.width).ok()?,
                    u32::try_from(texture.height).ok()?,
                    0,
                ];
                bank.extend(material_page_texels(self.tile, index)?);
            }
        }
        Some(directory as u32)
    }
}

pub(super) struct TerrainAtlas {
    pub(super) banks: [Vec<[u32; 4]>; 2],
    pub(super) addresses: Vec<u32>,
}

impl TerrainAtlas {
    pub(super) fn prepare(materials: &[TerrainMaterial<'_>], limit: u64) -> Option<Self> {
        const BYTE_BUDGET: u64 = 160 * 1024 * 1024;
        let capacity = (limit.min(BYTE_BUDGET) / 16) as usize;
        let mut layouts = materials
            .iter()
            .enumerate()
            .map(|(index, material)| {
                let (header, phases) = material.layout()?;
                let size = material.word_count(header[0][0] as usize, phases, capacity)?;
                Some((index, size, header, phases))
            })
            .collect::<Option<Vec<_>>>()?;
        layouts.sort_unstable_by_key(|(_, size, _, _)| std::cmp::Reverse(*size));
        let mut counts = [0; 2];
        let mut placements = Vec::with_capacity(layouts.len());
        for (_, size, _, _) in &layouts {
            let bank = usize::from(counts[1] < counts[0]);
            counts[bank] += *size;
            if counts[bank] > capacity {
                return None;
            }
            placements.push(bank);
        }
        if (counts[0] + counts[1]) as u64 * 16 > BYTE_BUDGET {
            return None;
        }
        let mut banks = counts.map(|count| Vec::with_capacity(count.max(1)));
        let mut addresses = vec![0; materials.len()];
        for ((index, _, header, phases), bank) in layouts.into_iter().zip(placements) {
            let offset = materials[index].append_to(&mut banks[bank], header, phases)?;
            addresses[index] = offset | if bank == 0 { 0 } else { BANK_BIT };
        }
        // An empty bank still has a legal storage binding and no references.
        for bank in &mut banks {
            if bank.is_empty() {
                bank.push([0; 4]);
            }
        }
        Some(Self { banks, addresses })
    }
}
