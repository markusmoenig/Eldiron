//! Cooperative CPU original stamp preparation, independent of GPU publication.
use super::*;
mod catalog;
mod live;
pub use catalog::{StampCatalog, StampCatalogBuilder};
pub use live::{StampSpritePreparation, StampSpriteRequest, StampSpriteSet};
use crate::iso_paint::{IsoPaintChunk, IsoPaintLayer, IsoPaintStamp};
use scenevm::{OrganicBillboardInstance, OrganicBillboardSprite};

pub(super) fn stamp_billboard(
    stamp: &IsoPaintStamp,
    world: [f32; 3],
    camera: Camera3D,
    height: u32,
) -> Option<(OrganicBillboardSprite, OrganicBillboardInstance)> {
    let (source, anchor) = IsoPaintRenderer::iso_paint_native_stamp_raster(stamp);
    let source_width = source.dim().width.max(0) as usize;
    let source_height = source.dim().height.max(0) as usize;
    if source_width == 0 || source_height == 0 {
        return None;
    }
    let pixels = source.pixels();
    let mut min_x = source_width;
    let mut min_y = source_height;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    let mut found = false;
    for y in 0..source_height {
        for x in 0..source_width {
            if pixels[(y * source_width + x) * 4 + 3] == 0 {
                continue;
            }
            found = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    if !found {
        return None;
    }
    let crop_width = max_x - min_x + 1;
    let crop_height = max_y - min_y + 1;
    let sprite_width = crop_width.min(256);
    let sprite_height = crop_height.min(256);
    let mut rgba = vec![0u8; sprite_width * sprite_height * 4];
    for y in 0..sprite_height {
        let source_y = min_y + y * crop_height / sprite_height;
        for x in 0..sprite_width {
            let source_x = min_x + x * crop_width / sprite_width;
            let source_index = (source_y * source_width + source_x) * 4;
            let target_index = (y * sprite_width + x) * 4;
            rgba[target_index..target_index + 4]
                .copy_from_slice(&pixels[source_index..source_index + 4]);
        }
    }

    let sprite = OrganicBillboardSprite {
        width: sprite_width as u32,
        height: sprite_height as u32,
        rgba: rgba.into(),
    };
    let instance = placement(
        stamp,
        world,
        camera,
        height,
        [min_x as i32, min_y as i32],
        [max_x as i32, max_y as i32],
        anchor,
    );
    Some((sprite, instance))
}

pub(super) fn bounded_stamp_billboard(
    stamp: &IsoPaintStamp,
    world: [f32; 3],
    camera: Camera3D,
    height: u32,
) -> Result<Option<(OrganicBillboardSprite, OrganicBillboardInstance)>, StampBillboardError> {
    let Some(raster) =
        super::stamp_raster::native_sprite(stamp).map_err(StampBillboardError::RasterDimensions)?
    else {
        return Ok(None);
    };
    let instance = placement(
        stamp,
        world,
        camera,
        height,
        raster.min,
        raster.max,
        raster.anchor,
    );
    Ok(Some((raster.sprite, instance)))
}

fn placement(
    stamp: &IsoPaintStamp,
    world: [f32; 3],
    camera: Camera3D,
    height: u32,
    min: [i32; 2],
    max: [i32; 2],
    anchor: [i32; 2],
) -> OrganicBillboardInstance {
    let authored_height = stamp
        .viewport_size
        .map(|size| size[1])
        .filter(|height| *height > 0)
        .unwrap_or(height.max(1) as i32) as f32;
    let world_per_pixel = 2.0 * stamp.camera_scale.unwrap_or(6.0).max(0.001) / authored_height;
    let crop_center_x = (min[0] as f32 + max[0] as f32) * 0.5;
    let crop_center_y = (min[1] as f32 + max[1] as f32) * 0.5;
    let offset_x = (crop_center_x - anchor[0] as f32) * world_per_pixel;
    let offset_y = (crop_center_y - anchor[1] as f32) * world_per_pixel;
    let anchor_world = Vec3::new(world[0], world[1], world[2]);
    let center = anchor_world + camera.right * offset_x - camera.up * offset_y;
    let instance = scenevm::OrganicBillboardInstance {
        center: [center.x, center.y, center.z],
        width: (max[0] - min[0] + 1) as f32 * world_per_pixel,
        height: (max[1] - min[1] + 1) as f32 * world_per_pixel,
        sprite_index: 0,
        // The HDR billboard pass also needs the recipe's emission channel.
        flags: 1
            | (1 << 7)
            | (((stamp
                .source_image
                .as_ref()
                .map(|tile| {
                    tile.sample_paint_surface([0.5, 0.5], stamp.variation as usize)
                        .1[3]
                })
                .unwrap_or_else(|| {
                    rusterix::material_library::MaterialDefinition::from_id(stamp.material_id)
                        .emissive
                })
                .clamp(0., 1.)
                * 255.)
                .round() as u32)
                << 8),
    };
    instance
}

#[derive(Clone, Debug)]
pub enum StampBillboardError {
    SourceBudget { bytes: u64, limit: u64 },
    RasterDimensions(String),
    Incomplete,
}
impl std::fmt::Display for StampBillboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceBudget { bytes, limit } => write!(
                f,
                "Stamp originals require {bytes} bytes; source limit is {limit}"
            ),
            Self::RasterDimensions(error) => f.write_str(error),
            Self::Incomplete => f.write_str("Stamp source preparation is incomplete"),
        }
    }
}
impl std::error::Error for StampBillboardError {}

/// Original sprites and placement are separate: byte-identical sprites share
/// one storage rectangle while all original ordered instances remain intact.
pub struct StampBillboards {
    pub sprites: Vec<OrganicBillboardSprite>,
    pub instances: Vec<OrganicBillboardInstance>,
    pub source_bytes: u64,
}
impl StampBillboards {
    pub fn upload(self, vm: &mut SceneVM) {
        if self.instances.is_empty() {
            vm.execute(Atom::ClearPaintBillboards);
        } else {
            vm.execute(Atom::SetPaintBillboards {
                sprites: self.sprites,
                instances: self.instances,
            });
        }
    }
}

pub struct StampBillboardPreparation {
    chunks: indexmap::map::IntoValues<String, IsoPaintChunk>,
    current: std::vec::IntoIter<IsoPaintStamp>,
    camera: Camera3D,
    height: u32,
    output: StampBillboards,
    by_hash: HashMap<u64, Vec<u32>>,
    limit: u64,
    done: bool,
    failed: Option<StampBillboardError>,
}
impl StampBillboardPreparation {
    pub fn new(
        mut frozen: IsoPaintLayer,
        camera: Camera3D,
        height: u32,
        source_byte_limit: u64,
    ) -> Self {
        let chunks = if frozen.visible {
            std::mem::take(&mut frozen.chunks)
        } else {
            Default::default()
        };
        Self {
            chunks: chunks.into_values(),
            current: Vec::new().into_iter(),
            camera,
            height,
            output: StampBillboards {
                sprites: Vec::new(),
                instances: Vec::new(),
                source_bytes: 32,
            },
            by_hash: HashMap::new(),
            limit: source_byte_limit,
            done: false,
            failed: if source_byte_limit < 32 {
                Some(StampBillboardError::SourceBudget {
                    bytes: 32,
                    limit: source_byte_limit,
                })
            } else {
                None
            },
        }
    }
    /// At most `items` chunks/stamps are visited; no partial GPU publication.
    /// Native virtual-raster scratch is independent of the original canvas area.
    pub fn advance(&mut self, vm: &SceneVM, items: usize) -> Result<bool, StampBillboardError> {
        self.advance_with(items, |stamp| match (stamp.paint_geo, stamp.surface_uv) {
            (Some(geo), Some(uv)) => vm
                .paint_surface_world_at(geo, uv)
                .map(|(world, _)| world)
                .or(stamp.world),
            _ => stamp.world,
        })
    }
    fn advance_with(
        &mut self,
        items: usize,
        mut resolve: impl FnMut(&IsoPaintStamp) -> Option<[f32; 3]>,
    ) -> Result<bool, StampBillboardError> {
        if let Some(error) = &self.failed {
            return Err(error.clone());
        }
        for _ in 0..items {
            if let Some(stamp) = self.current.next() {
                let world = resolve(&stamp);
                if let Err(error) = self.insert(stamp, world) {
                    self.failed = Some(error.clone());
                    return Err(error);
                }
            } else if let Some(chunk) = self.chunks.next() {
                self.current = chunk.stamps.into_iter();
            } else {
                self.done = true;
                break;
            }
        }
        Ok(self.done)
    }
    fn insert(
        &mut self,
        stamp: IsoPaintStamp,
        world: Option<[f32; 3]>,
    ) -> Result<(), StampBillboardError> {
        let Some(world) = world else {
            return Ok(());
        };
        let Some((sprite, mut instance)) =
            bounded_stamp_billboard(&stamp, world, self.camera, self.height)?
        else {
            return Ok(());
        };
        let mut hash = DefaultHasher::new();
        sprite.width.hash(&mut hash);
        sprite.height.hash(&mut hash);
        sprite.rgba.hash(&mut hash);
        let hash = hash.finish();
        // Hashes are candidate buckets only; byte comparison protects identity.
        let existing = self.by_hash.get(&hash).and_then(|slots| {
            slots.iter().copied().find(|slot| {
                let old = &self.output.sprites[*slot as usize];
                old.width == sprite.width && old.height == sprite.height && old.rgba == sprite.rgba
            })
        });
        let added = 32
            + if existing.is_some() {
                0
            } else {
                16 + sprite.rgba.len() as u64
            };
        let bytes = self.output.source_bytes.saturating_add(added);
        if bytes > self.limit {
            return Err(StampBillboardError::SourceBudget {
                bytes,
                limit: self.limit,
            });
        }
        instance.sprite_index = existing.unwrap_or(self.output.sprites.len() as u32);
        if existing.is_none() {
            self.by_hash
                .entry(hash)
                .or_default()
                .push(instance.sprite_index);
            self.output.sprites.push(sprite);
        }
        self.output.instances.push(instance);
        self.output.source_bytes = bytes;
        Ok(())
    }
    pub fn finish(self) -> Result<StampBillboards, StampBillboardError> {
        if let Some(error) = self.failed {
            return Err(error);
        }
        if !self.done {
            return Err(StampBillboardError::Incomplete);
        }
        Ok(self.output)
    }
}

#[cfg(test)]
mod tests;
