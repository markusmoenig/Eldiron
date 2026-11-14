//! Cancelable owned region work. Native workers and cooperative browser work
//! use the exact same incremental original-channel decoder and completion rule.
use super::*;
use crate::orthographic_bake::region_decode::{DepthRegionDecoder, PngRegionDecoder};
use theframework_core::SharedText;

struct EncodedOwners {
    sources: [Option<SharedText>; 5],
    bytes: u64,
    budget: Arc<Budget>,
}
impl Drop for EncodedOwners {
    fn drop(&mut self) {
        self.sources = Default::default();
        self.budget.1.fetch_sub(self.bytes, Ordering::Relaxed);
    }
}
pub(super) struct RegionTask {
    // Decoders/inputs/pixels drop before their charges and shared worker slot.
    png: Option<PngRegionDecoder<SharedText>>,
    depth: Option<DepthRegionDecoder<SharedText>>,
    input: EncodedOwners,
    pixels: DecodedBakeTile,
    charge: Charge,
    _slot: WorkerSlot,
    size: u32,
    region: OriginalRegion,
    stage: usize,
    valid: bool,
}
impl RegionTask {
    pub fn new(
        tile: &OrthographicBakeTile,
        size: u32,
        region: OriginalRegion,
        charge: Charge,
        slot: WorkerSlot,
    ) -> Self {
        let sources = [
            Some(tile.color_png_base64.clone()),
            tile.depth_base64.clone(),
            tile.albedo_png_base64.clone(),
            tile.normal_png_base64.clone(),
            tile.material_png_base64.clone(),
        ];
        let bytes = sources
            .iter()
            .flatten()
            .map(|source| source.len() as u64)
            .sum();
        charge.budget.1.fetch_add(bytes, Ordering::Relaxed);
        Self {
            png: None,
            depth: None,
            input: EncodedOwners {
                sources,
                bytes,
                budget: charge.budget.clone(),
            },
            pixels: Default::default(),
            charge,
            _slot: slot,
            size,
            region,
            stage: 0,
            valid: true,
        }
    }
    fn step(&mut self) -> bool {
        if self.stage >= 5 || !self.valid {
            return true;
        }
        let result = self.advance_channel();
        match result {
            Ok(false) => return false,
            Ok(true) => (),
            Err(_) if self.stage == 0 => self.valid = false,
            Err(_) => (), // Optional malformed channels match full-tile decoding.
        }
        self.png = None;
        self.depth = None;
        self.input.sources[self.stage] = None;
        self.stage += 1;
        self.stage >= 5 || !self.valid
    }
    fn advance_channel(&mut self) -> Result<bool, String> {
        let Some(input) = self.input.sources[self.stage].as_ref() else {
            return if self.stage == 0 {
                Err("Missing original region color".into())
            } else {
                Ok(true)
            };
        };
        if self.stage == 1 {
            if self.depth.is_none() {
                self.depth = Some(DepthRegionDecoder::new(
                    input.clone(),
                    self.size,
                    self.region,
                )?);
            }
            if !self.depth.as_mut().unwrap().advance(32)? {
                return Ok(false);
            }
            self.pixels.depth = self.depth.take().unwrap().finish()?;
        } else {
            if self.png.is_none() {
                self.png = Some(PngRegionDecoder::new(
                    input.clone(),
                    self.size,
                    self.region,
                )?);
            }
            if !self.png.as_mut().unwrap().advance(32)? {
                return Ok(false);
            }
            let channel = self.png.take().unwrap().finish()?;
            match self.stage {
                0 => self.pixels.color = channel,
                2 => self.pixels.albedo = channel,
                3 => self.pixels.normal = channel,
                4 => self.pixels.material = channel,
                _ => unreachable!(),
            }
        }
        Ok(true)
    }
    fn finish(mut self) -> Option<OrthographicBakeTileLease> {
        if !self.valid || self.stage != 5 {
            return None;
        }
        self.png = None;
        self.depth = None;
        drop(self.input);
        let bytes = pixel_bytes(&self.pixels);
        if bytes > self.charge.bytes {
            return None;
        }
        self.charge.shrink(bytes);
        Some(OrthographicBakeTileLease(Arc::new(ChargedTile {
            pixels: self.pixels,
            _charge: self.charge,
        })))
    }
}

#[cfg(not(any(target_arch = "wasm32", feature = "single_thread")))]
#[derive(Debug)]
pub(super) struct RegionWork {
    key: Key,
    result: std::sync::mpsc::Receiver<Option<OrthographicBakeTileLease>>,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
}
#[cfg(not(any(target_arch = "wasm32", feature = "single_thread")))]
impl RegionWork {
    pub fn start(key: Key, mut task: RegionTask) -> Result<Self, String> {
        let (sender, result) = std::sync::mpsc::sync_channel(1);
        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop = cancelled.clone();
        std::thread::Builder::new()
            .name("bake-region-decode".into())
            .spawn(move || {
                loop {
                    if stop.load(Ordering::Acquire) {
                        return;
                    }
                    if task.step() {
                        break;
                    }
                }
                if !stop.load(Ordering::Acquire) {
                    let _ = sender.send(task.finish());
                }
            })
            .map_err(|e| format!("Cannot start original-region worker: {e}"))?;
        Ok(Self {
            key,
            result,
            cancelled,
        })
    }
    pub fn key(&self) -> Key {
        self.key
    }
    pub fn poll(&mut self) -> Result<OrthographicBakeTilePoll, String> {
        use std::sync::mpsc::TryRecvError;
        match self.result.try_recv() {
            Ok(Some(lease)) => Ok(OrthographicBakeTilePoll::Ready(lease)),
            Ok(None) => Ok(OrthographicBakeTilePoll::Missing),
            Err(TryRecvError::Empty) => Ok(OrthographicBakeTilePoll::Pending),
            Err(TryRecvError::Disconnected) => Err("Original-region decode worker stopped".into()),
        }
    }
}
#[cfg(not(any(target_arch = "wasm32", feature = "single_thread")))]
impl Drop for RegionWork {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

#[cfg(any(target_arch = "wasm32", feature = "single_thread"))]
pub(super) struct RegionWork {
    key: Key,
    task: Option<RegionTask>,
}
#[cfg(any(target_arch = "wasm32", feature = "single_thread"))]
impl std::fmt::Debug for RegionWork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegionWork")
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}
#[cfg(any(target_arch = "wasm32", feature = "single_thread"))]
impl RegionWork {
    pub fn start(key: Key, task: RegionTask) -> Result<Self, String> {
        Ok(Self {
            key,
            task: Some(task),
        })
    }
    pub fn key(&self) -> Key {
        self.key
    }
    pub fn poll(&mut self) -> Result<OrthographicBakeTilePoll, String> {
        // No whole encoded input transfer/duplication. A host advances only one
        // bounded row/chunk batch per preparation, then yields to RAF/input.
        if self.task.as_mut().unwrap().step() {
            Ok(self.task.take().unwrap().finish().map_or(
                OrthographicBakeTilePoll::Missing,
                OrthographicBakeTilePoll::Ready,
            ))
        } else {
            Ok(OrthographicBakeTilePoll::Pending)
        }
    }
}
