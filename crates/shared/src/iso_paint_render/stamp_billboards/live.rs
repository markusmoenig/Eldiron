//! Immutable live sprite pixels are independent of world/camera placement.
use super::super::stamp_raster::native_sprite;
use super::catalog::{same_visual, visual_hash};
use super::*;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct StampSpriteRequest {
    inputs: Vec<Arc<IsoPaintStamp>>,
}
impl StampSpriteRequest {
    /// Freeze only distinct visual recipes, sharing source image backing. UV
    /// paint, selections, history and camera-dependent placement are not cloned.
    pub fn capture(layer: &IsoPaintLayer) -> Self {
        let mut request = Self::default();
        let mut buckets = HashMap::<u64, Vec<usize>>::new();
        if layer.visible {
            for stamp in layer.chunks.values().flat_map(|chunk| &chunk.stamps) {
                if stamp.world.is_none()
                    && (stamp.paint_geo.is_none() || stamp.surface_uv.is_none())
                {
                    continue;
                }
                let bucket = buckets.entry(visual_hash(stamp)).or_default();
                if bucket
                    .iter()
                    .any(|i| same_visual(&request.inputs[*i], stamp))
                {
                    continue;
                }
                bucket.push(request.inputs.len());
                request.inputs.push(Arc::new(stamp.clone()));
            }
        }
        request
    }

    pub fn same(&self, other: &Self) -> bool {
        self.inputs.len() == other.inputs.len()
            && self
                .inputs
                .iter()
                .zip(&other.inputs)
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
        }
    }
}

struct Entry {
    stamp: Arc<IsoPaintStamp>,
    raster: Option<Raster>,
}
struct Raster {
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
    sprites: Vec<Arc<OrganicBillboardSprite>>,
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
        self.sprites
            .iter()
            .map(|sprite| OrganicBillboardSprite {
                width: sprite.width,
                height: sprite.height,
                rgba: sprite.rgba.clone(),
            })
            .collect()
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
}
impl StampSpritePreparation {
    /// One recipe at a time; the caller owns its worker/cooperative policy.
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
                    let raster = native_sprite(stamp)?.map(|raster| {
                        let bucket = self.pixels.entry(pixel_hash(&raster.sprite)).or_default();
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
            self.entries.push(entry);
            self.cursor += 1;
        }
        Ok(self.cursor == self.request.inputs.len())
    }

    pub fn finish(self) -> Result<StampSpriteSet, String> {
        if self.cursor != self.request.inputs.len() {
            return Err("Live stamp preparation is incomplete".into());
        }
        let mut buckets = HashMap::<u64, Vec<usize>>::new();
        let mut by_pixels = HashMap::new();
        let mut sprites = Vec::new();
        let mut slots = Vec::with_capacity(self.entries.len());
        for (index, entry) in self.entries.iter().enumerate() {
            buckets
                .entry(visual_hash(&entry.stamp))
                .or_default()
                .push(index);
            slots.push(if let Some(raster) = &entry.raster {
                let identity = Arc::as_ptr(&raster.sprite) as usize;
                let slot = if let Some(slot) = by_pixels.get(&identity) {
                    *slot
                } else {
                    let slot =
                        u32::try_from(sprites.len()).map_err(|_| "Too many live stamp sources")?;
                    by_pixels.insert(identity, slot);
                    sprites.push(raster.sprite.clone());
                    slot
                };
                Some(slot)
            } else {
                None
            });
        }
        let mut pixels = HashMap::<u64, Vec<Arc<OrganicBillboardSprite>>>::new();
        for sprite in &sprites {
            pixels
                .entry(pixel_hash(sprite))
                .or_default()
                .push(sprite.clone());
        }
        Ok(StampSpriteSet {
            id: Uuid::new_v4(),
            request: self.request,
            entries: self.entries,
            buckets,
            slots,
            sprites,
            pixels,
        })
    }
}
