//! Immutable live sprite pixels are independent of world/camera placement.
use super::super::stamp_raster::{NativeSprite, StampWork, native_sprite_work};
use super::catalog::{same_visual, visual_hash};
use super::*;
use scenevm::vm::{PaintBillboardSourceBuilder, PaintBillboardSources};
use std::sync::Arc;
mod capture;
pub use capture::StampRequestCapture;
mod placements;
pub use placements::StampPlacementOwner;

#[derive(Clone, Default)]
pub struct StampSpriteRequest {
    inputs: Arc<Vec<Arc<IsoPaintStamp>>>,
    visible: bool,
    sources: Arc<Vec<crate::shared_vec::SharedVec<IsoPaintStamp>>>,
    visual_identity: Arc<()>,
}
impl StampSpriteRequest {
    /// Freeze distinct visual recipes and retain their immutable source lists.
    /// UV paint, selections, history and source image pixels are not cloned.
    pub fn capture(layer: &IsoPaintLayer) -> Self {
        // Explicit synchronous/reference callers can drain the same exact
        // builder; live owners use begin_capture and retain its bounded cursor.
        let mut capture = Self::begin_capture(layer);
        while !capture.advance(4096) {}
        capture.finish().expect("completed stamp request capture")
    }

    /// Exact unchanged recipe backing can be recognized without cloning/hashing
    /// every recipe. Placement changes detach the list and still reuse artwork.
    pub fn same_source(&self, layer: &IsoPaintLayer) -> bool {
        if self.visible != layer.visible {
            return false;
        }
        if !self.visible {
            return true;
        }
        let mut chunks = layer
            .chunks
            .values()
            .filter(|chunk| !chunk.stamps.is_empty());
        self.sources.iter().all(|source| {
            chunks
                .next()
                .is_some_and(|chunk| source.same_storage(&chunk.stamps))
        }) && chunks.next().is_none()
    }
    pub fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.visual_identity, &other.visual_identity)
            || Arc::ptr_eq(&self.inputs, &other.inputs)
            || self.inputs.len() == other.inputs.len()
                && self
                    .inputs
                    .iter()
                    .zip(other.inputs.iter())
                    .all(|(a, b)| same_visual(a, b))
    }

    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty()
    }

    pub fn prepare(self, previous: Option<Arc<StampSpriteSet>>) -> StampSpritePreparation {
        let pixels = previous
            .as_ref()
            .map(|set| set.pixels.clone())
            .unwrap_or_default();
        StampSpritePreparation {
            request: self,
            previous,
            entries: Vec::new(),
            cursor: 0,
            pixels,
            pending: None,
            buckets: HashMap::new(),
            slots: Vec::new(),
            source_builder: PaintBillboardSourceBuilder::default(),
            by_pixels: HashMap::new(),
            current_pixels: HashMap::new(),
        }
    }
}

struct Entry {
    stamp: Arc<IsoPaintStamp>,
    raster: Option<Raster>,
}
struct Raster {
    hash: u64,
    sprite: Arc<OrganicBillboardSprite>,
    min: [i32; 2],
    max: [i32; 2],
    anchor: [i32; 2],
}
fn pixel_hash(sprite: &OrganicBillboardSprite) -> u64 {
    let mut hash = DefaultHasher::new();
    sprite.width.hash(&mut hash);
    sprite.height.hash(&mut hash);
    sprite.rgba.hash(&mut hash);
    hash.finish()
}

pub struct StampSpriteSet {
    id: Uuid,
    request: StampSpriteRequest,
    entries: Vec<Arc<Entry>>,
    buckets: HashMap<u64, Vec<usize>>,
    slots: Vec<Option<u32>>,
    sources: PaintBillboardSources,
    pixels: HashMap<u64, Vec<Arc<OrganicBillboardSprite>>>,
}
impl StampSpriteSet {
    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn matches(&self, request: &StampSpriteRequest) -> bool {
        self.request.same(request)
    }

    fn find(&self, stamp: &IsoPaintStamp) -> Option<usize> {
        self.buckets
            .get(&visual_hash(stamp))?
            .iter()
            .copied()
            .find(|i| same_visual(&self.entries[*i].stamp, stamp))
    }

    pub fn sprites(&self) -> Vec<OrganicBillboardSprite> {
        self.sources.sprites().to_vec()
    }

    /// Resolve the current view's surface/world placement after preparation.
    /// Ordered authored instances remain distinct even when their pixels share.
    pub fn instances(
        &self,
        layer: &IsoPaintLayer,
        vm: &SceneVM,
        camera: Camera3D,
        height: u32,
    ) -> Result<Vec<OrganicBillboardInstance>, String> {
        let mut instances = Vec::new();
        if !layer.visible {
            return Ok(instances);
        }
        for stamp in layer.chunks.values().flat_map(|chunk| &chunk.stamps) {
            let world = match (stamp.paint_geo, stamp.surface_uv) {
                (Some(geo), Some(uv)) => vm
                    .paint_surface_world_at(geo, uv)
                    .map(|(world, _)| world)
                    .or(stamp.world),
                _ => stamp.world,
            };
            let Some(world) = world else {
                continue;
            };
            let index = self
                .find(stamp)
                .ok_or("Live stamp pixels are not current")?;
            let Some(raster) = &self.entries[index].raster else {
                continue;
            };
            let mut instance = placement(
                stamp,
                world,
                camera,
                height,
                raster.min,
                raster.max,
                raster.anchor,
            );
            instance.sprite_index = self.slots[index].unwrap();
            if !instance.center.iter().all(|v| v.is_finite())
                || !instance.width.is_finite()
                || !instance.height.is_finite()
                || instance.width <= 0.
                || instance.height <= 0.
            {
                return Err("Invalid live stamp placement".into());
            }
            instances.push(instance);
        }
        Ok(instances)
    }
}

pub struct StampSpritePreparation {
    request: StampSpriteRequest,
    previous: Option<Arc<StampSpriteSet>>,
    entries: Vec<Arc<Entry>>,
    cursor: usize,
    pixels: HashMap<u64, Vec<Arc<OrganicBillboardSprite>>>,
    pending: Option<StampWork<Option<NativeSprite>>>,
    buckets: HashMap<u64, Vec<usize>>,
    slots: Vec<Option<u32>>,
    source_builder: PaintBillboardSourceBuilder,
    by_pixels: HashMap<usize, u32>,
    current_pixels: HashMap<u64, Vec<Arc<OrganicBillboardSprite>>>,
}
impl StampSpritePreparation {
    /// At most one inner kernel slice per requested item. The caller owns its
    /// worker/cooperative policy; dropping preparation cancels a partial recipe.
    pub fn advance(&mut self, items: usize) -> Result<bool, String> {
        for _ in 0..items {
            let Some(stamp) = self.request.inputs.get(self.cursor) else {
                return Ok(true);
            };
            let cached = self
                .previous
                .as_ref()
                .and_then(|set| set.find(stamp).map(|index| set.entries[index].clone()));
            let entry = match cached {
                Some(entry) => entry,
                None => {
                    let pending = self
                        .pending
                        .get_or_insert_with(|| native_sprite_work(stamp.clone()));
                    let Some(raster) = pending.advance() else {
                        return Ok(false);
                    };
                    self.pending = None;
                    let raster = raster?.map(|raster| {
                        let hash = pixel_hash(&raster.sprite);
                        let bucket = self.pixels.entry(hash).or_default();
                        let same = bucket
                            .iter()
                            .find(|old| {
                                old.width == raster.sprite.width
                                    && old.height == raster.sprite.height
                                    && old.rgba == raster.sprite.rgba
                            })
                            .cloned();
                        let sprite = same.unwrap_or_else(|| {
                            let sprite = Arc::new(raster.sprite);
                            bucket.push(sprite.clone());
                            sprite
                        });
                        Raster {
                            hash,
                            sprite,
                            min: raster.min,
                            max: raster.max,
                            anchor: raster.anchor,
                        }
                    });
                    Arc::new(Entry {
                        stamp: stamp.clone(),
                        raster,
                    })
                }
            };
            self.buckets
                .entry(visual_hash(&entry.stamp))
                .or_default()
                .push(self.entries.len());
            self.slots.push(if let Some(raster) = &entry.raster {
                let identity = Arc::as_ptr(&raster.sprite) as usize;
                let slot = if let Some(slot) = self.by_pixels.get(&identity) {
                    *slot
                } else {
                    let slot = self.source_builder.push((*raster.sprite).clone())?;
                    self.by_pixels.insert(identity, slot);
                    self.current_pixels
                        .entry(raster.hash)
                        .or_default()
                        .push(raster.sprite.clone());
                    slot
                };
                Some(slot)
            } else {
                None
            });
            self.entries.push(entry);
            self.cursor += 1;
        }
        Ok(self.cursor == self.request.inputs.len())
    }

    pub fn finish(self) -> Result<StampSpriteSet, String> {
        if self.cursor != self.request.inputs.len() {
            return Err("Live stamp preparation is incomplete".into());
        }
        let mut request = self.request;
        // Artwork needs distinct visual inputs, not every original placement.
        // The live request owner retains its own latest source identity.
        request.sources = Arc::default();
        Ok(StampSpriteSet {
            id: Uuid::new_v4(),
            request,
            entries: self.entries,
            buckets: self.buckets,
            slots: self.slots,
            sources: self.source_builder.finish(),
            pixels: self.current_pixels,
        })
    }
}

#[derive(PartialEq, Eq)]
pub struct StampUploadKey {
    source: Uuid,
    instances: Vec<[u32; 7]>,
}
fn instance_key(i: &OrganicBillboardInstance) -> [u32; 7] {
    [
        i.center[0].to_bits(),
        i.center[1].to_bits(),
        i.center[2].to_bits(),
        i.width.to_bits(),
        i.height.to_bits(),
        i.sprite_index,
        i.flags,
    ]
}
/// Upload camera-independent pixels only for a new source. Placements retain
/// authored order and update independently, with exact float-bit cache keys.
pub fn upload_sprite_set(
    set: &StampSpriteSet,
    layer: &IsoPaintLayer,
    vm: &mut SceneVM,
    camera: Camera3D,
    height: u32,
    uploaded: &mut Option<StampUploadKey>,
) -> Result<bool, String> {
    let active = vm.active_vm_index();
    vm.set_active_vm(0);
    let result = (|| {
        let instances = set.instances(layer, vm, camera, height)?;
        let key = StampUploadKey {
            source: set.id(),
            instances: instances.iter().map(instance_key).collect(),
        };
        if uploaded.as_ref() == Some(&key) {
            return Ok(false);
        }
        if instances.is_empty() {
            vm.execute(scenevm::Atom::ClearPaintBillboards);
        } else if uploaded
            .as_ref()
            .is_some_and(|old| old.source == key.source && !old.instances.is_empty())
        {
            vm.execute(scenevm::Atom::SetPaintBillboardInstances { instances });
        } else {
            vm.execute(scenevm::Atom::SetPaintBillboards {
                sprites: set.sprites(),
                instances,
            });
        }
        *uploaded = Some(key);
        Ok(true)
    })();
    vm.set_active_vm(active);
    result
}
