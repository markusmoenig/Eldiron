//! Bounded original-channel decoding. Leases remain charged after cache
//! eviction/clear, so a consumer cannot bypass the budget by retaining tiles.
use super::*;
use std::{
    ops::Deref,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

mod decode;
use decode::{DecodeTask, DecodeWork, WorkerSlot};

pub(super) const DECODE_BUDGET: u64 = 32 * 1024 * 1024;

#[derive(Debug, Default)]
struct Budget(AtomicU64);
#[derive(Debug)]
struct ChargedTile {
    pixels: DecodedBakeTile,
    _charge: Charge,
}
impl Drop for ChargedTile {
    fn drop(&mut self) {
        // Free backing allocations before another consumer can reserve their
        // charge; Rust normally drops fields after this Drop body returns.
        self.pixels = DecodedBakeTile::default();
    }
}

#[derive(Debug)]
struct Charge {
    bytes: u64,
    budget: Arc<Budget>,
}
impl Charge {
    fn shrink(&mut self, bytes: u64) {
        assert!(bytes <= self.bytes);
        self.budget
            .0
            .fetch_sub(self.bytes - bytes, Ordering::Relaxed);
        self.bytes = bytes;
    }
}
impl Drop for Charge {
    fn drop(&mut self) {
        self.budget.0.fetch_sub(self.bytes, Ordering::Relaxed);
    }
}
fn pixel_bytes(pixels: &DecodedBakeTile) -> u64 {
    (pixels.color.capacity()
        + pixels.albedo.capacity()
        + pixels.normal.capacity()
        + pixels.material.capacity()) as u64
        + pixels.depth.capacity() as u64 * 4
}

/// A nonblocking original-source lookup. Pending must be retried by the host;
/// it never denotes a missing tile and must not publish a partial projection.
#[derive(Debug)]
pub enum OrthographicBakeTilePoll {
    Ready(OrthographicBakeTileLease),
    Pending,
    Missing,
}

/// Immutable original channels; no viewport pixels or renderer ownership.
#[derive(Clone, Debug)]
pub struct OrthographicBakeTileLease(Arc<ChargedTile>);
impl Deref for OrthographicBakeTileLease {
    type Target = DecodedBakeTile;
    fn deref(&self) -> &Self::Target {
        &self.0.pixels
    }
}
impl OrthographicBakeTileLease {
    pub fn source(&self) -> OrthographicBakeTileSource<'_> {
        (&**self).into()
    }
}

#[derive(Debug, Default)]
struct State {
    entries: HashMap<(i32, i32), (OrthographicBakeTileLease, u64)>,
    clock: u64,
    failed: std::collections::HashSet<(i32, i32)>,
    usable: Option<bool>,
    pending: Option<DecodeWork>,
}
#[derive(Debug, Default)]
pub(super) struct TileCache {
    state: Mutex<State>,
    budget: Arc<Budget>,
    index: HashMap<(i32, i32), usize>,
    revisions: HashMap<(i32, i32), Uuid>,
}
impl TileCache {
    pub(super) fn clear(&mut self) {
        let state = self.state.get_mut().unwrap();
        state.pending = None;
        state.entries.clear();
        state.failed.clear();
        state.usable = None;
        self.index.clear();
        self.revisions.clear();
    }
    pub(super) fn reset(&mut self, asset: Option<&OrthographicBakeAsset>) {
        self.clear();
        if let Some(asset) = asset {
            for (i, tile) in asset.tiles.iter().enumerate() {
                let at = (tile.x, tile.y);
                self.index.insert(at, i);
                self.revisions.insert(at, Uuid::new_v4());
            }
        }
    }
    pub(super) fn positions(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.index.keys().copied()
    }
    pub(super) fn has_tiles(&self, asset: &OrthographicBakeAsset) -> bool {
        if let Some(usable) = self.state.lock().unwrap().usable {
            return usable;
        }
        // Preserve the normal geometry fallback for missing/corrupt sources.
        // Probe only until one usable original exists, once per installation.
        for at in self.positions() {
            match self.get(asset, at) {
                Ok(Some(_)) => return true,
                // Let the producer report temporary lease pressure explicitly.
                Err(_) => return true,
                Ok(None) => (),
            }
        }
        self.state.lock().unwrap().usable = Some(false);
        false
    }
    pub(super) fn usage(&self) -> u64 {
        self.budget.0.load(Ordering::Relaxed)
    }

    fn make_room(&self, state: &mut State, bytes: u64) -> Result<(), String> {
        if bytes > DECODE_BUDGET {
            return Err("Bake tile exceeds decoded source budget.".into());
        }
        while self.usage().saturating_add(bytes) > DECODE_BUDGET {
            let victim = state
                .entries
                .iter()
                .filter(|(_, (lease, _))| Arc::strong_count(&lease.0) == 1)
                .min_by_key(|(_, (_, age))| *age)
                .map(|(at, _)| *at);
            let Some(victim) = victim else {
                return Err("Decoded bake source budget is pinned by active consumers.".into());
            };
            state.entries.remove(&victim);
        }
        Ok(())
    }
    fn insert_locked(
        &self,
        state: &mut State,
        at: (i32, i32),
        pixels: DecodedBakeTile,
    ) -> Result<OrthographicBakeTileLease, String> {
        let bytes = pixel_bytes(&pixels);
        state.entries.remove(&at);
        self.make_room(state, bytes)?;
        self.budget.0.fetch_add(bytes, Ordering::Relaxed);
        let lease = OrthographicBakeTileLease(Arc::new(ChargedTile {
            pixels,
            _charge: Charge {
                bytes,
                budget: self.budget.clone(),
            },
        }));
        state.clock = state.clock.wrapping_add(1);
        state.failed.remove(&at);
        state.usable = Some(true);
        state.entries.insert(at, (lease.clone(), state.clock));
        Ok(lease)
    }
    pub(super) fn insert(
        &mut self,
        at: (i32, i32),
        index: Option<usize>,
        pixels: DecodedBakeTile,
    ) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if state.pending.as_ref().is_some_and(|work| work.at() == at) {
            // A newer producer sample supersedes an encoded decode in flight.
            state.pending = None;
        }
        self.insert_locked(&mut state, at, pixels)?;
        self.revisions.insert(at, Uuid::new_v4());
        if let Some(index) = index {
            self.index.insert(at, index);
        }
        Ok(())
    }
    pub(super) fn get(
        &self,
        asset: &OrthographicBakeAsset,
        at: (i32, i32),
    ) -> Result<Option<OrthographicBakeTileLease>, String> {
        let mut state = self.state.lock().unwrap();
        state.clock = state.clock.wrapping_add(1);
        let age = state.clock;
        if let Some((lease, last)) = state.entries.get_mut(&at) {
            *last = age;
            return Ok(Some(lease.clone()));
        }
        if state.failed.contains(&at) {
            return Ok(None);
        }
        let Some(tile) = self.index.get(&at).and_then(|i| asset.tiles.get(*i)) else {
            return Ok(None);
        };
        let bytes = u64::from(asset.tile_size)
            .checked_mul(u64::from(asset.tile_size))
            .and_then(|n| n.checked_mul(20))
            .ok_or("Invalid decoded bake tile size.")?;
        // Reserve room before invoking image decoding, even for missing optional
        // channels. A single tile is the only additional decode scratch input.
        self.make_room(&mut state, bytes)?;
        let Some(pixels) = decode_tile(tile, asset.tile_size) else {
            state.failed.insert(at);
            return Ok(None);
        };
        self.insert_locked(&mut state, at, pixels).map(Some)
    }
    pub(super) fn may_have_tiles(&self) -> bool {
        let state = self.state.lock().unwrap();
        !state.entries.is_empty() || self.index.len() > state.failed.len()
    }
    pub(super) fn advance(&self) -> Result<(), String> {
        use OrthographicBakeTilePoll::*;
        let mut state = self.state.lock().unwrap();
        if let Some(mut work) = state.pending.take() {
            let work_at = work.at();
            match work.poll()? {
                Pending => state.pending = Some(work),
                Missing => {
                    state.failed.insert(work_at);
                }
                Ready(lease) => {
                    state.clock = state.clock.wrapping_add(1);
                    let age = state.clock;
                    state.failed.remove(&work_at);
                    state.usable = Some(true);
                    state.entries.insert(work_at, (lease, age));
                }
            }
        }
        Ok(())
    }
    pub(super) fn poll(
        &self,
        asset: &OrthographicBakeAsset,
        at: (i32, i32),
    ) -> Result<OrthographicBakeTilePoll, String> {
        use OrthographicBakeTilePoll::*;
        let mut state = self.state.lock().unwrap();
        state.clock = state.clock.wrapping_add(1);
        let age = state.clock;
        if let Some((lease, last)) = state.entries.get_mut(&at) {
            *last = age;
            return Ok(Ready(lease.clone()));
        }
        if state.failed.contains(&at) {
            return Ok(Missing);
        }
        let Some(tile) = self.index.get(&at).and_then(|i| asset.tiles.get(*i)) else {
            return Ok(Missing);
        };
        if state.pending.is_some() {
            return Ok(Pending);
        }
        let Some(slot) = WorkerSlot::acquire() else {
            return Ok(Pending);
        };
        let inputs = [
            Some(tile.color_png_base64.as_str()),
            tile.depth_base64.as_deref(),
            tile.albedo_png_base64.as_deref(),
            tile.normal_png_base64.as_deref(),
            tile.material_png_base64.as_deref(),
        ];
        let decoded = u64::from(asset.tile_size)
            .checked_mul(u64::from(asset.tile_size))
            .and_then(|n| n.checked_mul(20))
            .ok_or("Invalid decoded bake tile size.")?;
        let bytes = inputs
            .iter()
            .flatten()
            .try_fold(decoded, |sum, s| sum.checked_add(s.len() as u64))
            .ok_or("Invalid encoded bake tile size.")?;
        // Charge owned encoded inputs and maximum output before cloning or
        // starting work. PNG/depth codec scratch has separate bounded limits.
        if bytes > DECODE_BUDGET {
            return Err("Bake tile and owned decode inputs exceed source budget.".into());
        }
        if self.make_room(&mut state, bytes).is_err() {
            // A cancelled worker or externally retained original still owns
            // this charge. Retry without blocking or failing the document.
            return Ok(Pending);
        }
        self.budget.0.fetch_add(bytes, Ordering::Relaxed);
        let charge = Charge {
            bytes,
            budget: self.budget.clone(),
        };
        let task = DecodeTask::new(inputs, asset.tile_size, charge, slot);
        state.pending = Some(DecodeWork::start(at, task)?);
        Ok(Pending)
    }
    pub(super) fn revision(&self, at: (i32, i32)) -> Option<Uuid> {
        self.revisions.get(&at).copied()
    }
    pub(super) fn all_positions(&self) -> Vec<(i32, i32)> {
        self.revisions.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leases_remain_charged_after_clear_and_prevent_budget_bypass() {
        let asset = OrthographicBakeAsset {
            version: 5,
            tile_size: 1,
            pixels_per_world_unit: 1.,
            projected_bounds: [0.; 4],
            camera_forward: [0., 0., 1.],
            camera_right: [1., 0., 0.],
            camera_up: [0., 1., 0.],
            camera_forward_origin: 0.,
            samples: 1,
            tiles: Vec::new(),
        };
        let mut cache = TileCache::default();
        let pixels = || DecodedBakeTile {
            color: vec![7; (DECODE_BUDGET / 2) as usize],
            ..Default::default()
        };
        cache.insert((0, 0), None, pixels()).unwrap();
        let first = cache.get(&asset, (0, 0)).unwrap().unwrap();
        cache.insert((1, 0), None, pixels()).unwrap();
        let second = cache.get(&asset, (1, 0)).unwrap().unwrap();
        assert_eq!(cache.usage(), DECODE_BUDGET);
        assert!(
            cache
                .insert((2, 0), None, pixels())
                .unwrap_err()
                .contains("pinned")
        );
        cache.clear();
        assert_eq!(cache.usage(), DECODE_BUDGET);
        assert_eq!(first.color[0], 7);
        drop(first);
        assert_eq!(cache.usage(), DECODE_BUDGET / 2);
        drop(second);
        assert_eq!(cache.usage(), 0);
    }
}
