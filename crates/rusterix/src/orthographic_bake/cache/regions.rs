//! Original-region cache sharing the full-tile budget and worker admission.
//! Encoded originals are immutable shared owners, not whole-payload copies.
use super::*;
use crate::orthographic_bake::region_decode::OriginalRegion;
mod work;
use work::{RegionTask, RegionWork};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Key {
    at: (i32, i32),
    region: OriginalRegion,
}
#[derive(Debug, Default)]
pub(super) struct Regions {
    entries: HashMap<Key, (OrthographicBakeTileLease, u64)>,
    pending: Option<RegionWork>,
}
impl Regions {
    pub fn invalidate(&mut self, at: (i32, i32)) {
        self.entries.retain(|key, _| key.at != at);
        if self
            .pending
            .as_ref()
            .is_some_and(|work| work.key().at == at)
        {
            self.pending = None;
        }
    }
    pub fn evict(&mut self) -> bool {
        let victim = self
            .entries
            .iter()
            .filter(|(_, (lease, _))| Arc::strong_count(&lease.0) == 1)
            .min_by_key(|(_, (_, age))| *age)
            .map(|(key, _)| *key);
        victim.is_some_and(|key| self.entries.remove(&key).is_some())
    }
}
impl TileCache {
    pub(in crate::orthographic_bake) fn region_retained_encoded_bytes(&self) -> u64 {
        self.budget.1.load(Ordering::Relaxed)
    }
    pub(super) fn advance_region(&self, state: &mut State) -> Result<(), String> {
        let Some(mut work) = state.regions.pending.take() else {
            return Ok(());
        };
        let key = work.key();
        match work.poll()? {
            OrthographicBakeTilePoll::Pending => state.regions.pending = Some(work),
            OrthographicBakeTilePoll::Missing => {
                state.failed.insert(key.at);
            }
            OrthographicBakeTilePoll::Ready(lease) => {
                state.clock = state.clock.wrapping_add(1);
                state.usable = Some(true);
                state.failed.remove(&key.at);
                state.regions.entries.insert(key, (lease, state.clock));
            }
        }
        Ok(())
    }
    pub(in crate::orthographic_bake) fn poll_region(
        &self,
        asset: &OrthographicBakeAsset,
        at: (i32, i32),
        region: OriginalRegion,
    ) -> Result<OrthographicBakeTilePoll, String> {
        use OrthographicBakeTilePoll::*;
        let count = region.validate(asset.tile_size)?;
        let key = Key { at, region };
        let mut state = self.state.lock().unwrap();
        state.clock = state.clock.wrapping_add(1);
        let age = state.clock;
        if let Some((lease, last)) = state.regions.entries.get_mut(&key) {
            *last = age;
            return Ok(Ready(lease.clone()));
        }
        if state.failed.contains(&at) {
            return Ok(Missing);
        }
        // Bound metadata as well as decoded storage. Referenced leases continue
        // to own their budget charge after cache eviction or controller clear.
        while state.regions.entries.len() >= 128 {
            if !state.regions.evict() {
                return Ok(Pending);
            }
        }
        if let Some((original, _)) = state.entries.get(&at) {
            let original = original.clone();
            let bytes = count as u64 * 20;
            if self.make_room(&mut state, bytes).is_err() {
                return Ok(Pending);
            }
            let pixels = crop(&original, asset.tile_size, region);
            let bytes = pixel_bytes(&pixels);
            self.budget.0.fetch_add(bytes, Ordering::Relaxed);
            let lease = OrthographicBakeTileLease(Arc::new(ChargedTile {
                pixels,
                _charge: Charge {
                    bytes,
                    budget: self.budget.clone(),
                },
            }));
            state.regions.entries.insert(key, (lease.clone(), age));
            return Ok(Ready(lease));
        }
        let Some(tile) = self
            .index
            .get(&at)
            .and_then(|index| asset.tiles.get(*index))
        else {
            return Ok(Missing);
        };
        if state.regions.pending.is_some() {
            return Ok(Pending);
        }
        let Some(slot) = WorkerSlot::acquire() else {
            return Ok(Pending);
        };
        // One live PNG codec plus completed region channels. Keep independent
        // codec scratch charged until the worker actually retires.
        let bytes = count as u64 * 20 + 16 * 1024 * 1024;
        if self.make_room(&mut state, bytes).is_err() {
            return Ok(Pending);
        }
        self.budget.0.fetch_add(bytes, Ordering::Relaxed);
        let task = RegionTask::new(
            tile,
            asset.tile_size,
            region,
            Charge {
                bytes,
                budget: self.budget.clone(),
            },
            slot,
        );
        state.regions.pending = Some(RegionWork::start(key, task)?);
        Ok(Pending)
    }
}

fn crop(original: &DecodedBakeTile, size: u32, region: OriginalRegion) -> DecodedBakeTile {
    fn rows<T: Copy>(source: &[T], size: u32, region: OriginalRegion, channels: usize) -> Vec<T> {
        let expected = (size as usize)
            .checked_mul(size as usize)
            .and_then(|pixels| pixels.checked_mul(channels));
        if expected != Some(source.len()) {
            return Vec::new();
        }
        let mut output =
            Vec::with_capacity(region.size[0] as usize * region.size[1] as usize * channels);
        for y in region.origin[1]..region.origin[1] + region.size[1] {
            let start = (y as usize * size as usize + region.origin[0] as usize) * channels;
            output.extend_from_slice(&source[start..start + region.size[0] as usize * channels]);
        }
        output
    }
    DecodedBakeTile {
        color: rows(&original.color, size, region, 4),
        depth: rows(&original.depth, size, region, 1),
        albedo: rows(&original.albedo, size, region, 4),
        normal: rows(&original.normal, size, region, 4),
        material: rows(&original.material, size, region, 4),
    }
}
