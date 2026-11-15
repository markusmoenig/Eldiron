//! Frozen stamp recipes and ordered placement, without retained sprite pixels.
use super::*;
use crate::iso_paint_render::StampRecipe;
use scenevm::PagedBillboardSprite;
use std::sync::Arc;

pub struct StampCatalog {
    pub recipes: Vec<Arc<StampRecipe>>,
    pub sprites: Vec<PagedBillboardSprite>,
    pub instances: Vec<OrganicBillboardInstance>,
}
impl StampCatalog {
    pub fn install(self, vm: &mut SceneVM) -> Vec<Arc<StampRecipe>> {
        vm.execute(Atom::SetPagedPaintBillboards {
            sprites: self.sprites,
            instances: self.instances,
        });
        self.recipes
    }
}
pub struct StampCatalogBuilder {
    chunks: indexmap::map::IntoValues<String, IsoPaintChunk>,
    current: std::vec::IntoIter<IsoPaintStamp>,
    output: StampCatalog,
    by_hash: HashMap<u64, Vec<u32>>,
    camera: Camera3D,
    height: u32,
    metadata_limit: u64,
    metadata_bytes: u64,
    done: bool,
    failure: Option<String>,
}
impl StampCatalogBuilder {
    pub fn new(
        mut source: IsoPaintLayer,
        camera: Camera3D,
        height: u32,
        metadata_limit: u64,
    ) -> Self {
        let chunks = if source.visible {
            std::mem::take(&mut source.chunks)
        } else {
            Default::default()
        };
        Self {
            chunks: chunks.into_values(),
            current: Vec::new().into_iter(),
            output: StampCatalog {
                recipes: Vec::new(),
                sprites: Vec::new(),
                instances: Vec::new(),
            },
            by_hash: HashMap::new(),
            camera,
            height,
            metadata_limit,
            metadata_bytes: 32,
            done: false,
            failure: None,
        }
    }
    pub fn advance(&mut self, vm: &SceneVM, items: usize) -> Result<bool, String> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        let result = self.advance_inner(vm, items);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
    fn advance_inner(&mut self, vm: &SceneVM, items: usize) -> Result<bool, String> {
        for _ in 0..items {
            if let Some(stamp) = self.current.next() {
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
                let hash = visual_hash(&stamp);
                let existing = self.by_hash.get(&hash).and_then(|slots| {
                    slots.iter().copied().find(|slot| {
                        same_visual(&self.output.recipes[*slot as usize].stamp, &stamp)
                    })
                });
                let slot = if let Some(slot) = existing {
                    slot
                } else {
                    let slot = u32::try_from(self.output.recipes.len())
                        .map_err(|_| "Stamp recipe count exceeds u32")?;
                    let Some(recipe) = StampRecipe::prepare(Arc::new(stamp.clone()))? else {
                        continue;
                    };
                    let size = recipe.size();
                    let alpha_mask = recipe.alpha_mask();
                    self.metadata_bytes = self
                        .metadata_bytes
                        .saturating_add(16 + alpha_mask.len() as u64 * 4);
                    self.output.sprites.push(PagedBillboardSprite {
                        width: size[0],
                        height: size[1],
                        source_id: slot,
                        alpha_mask,
                    });
                    self.output.recipes.push(Arc::new(recipe));
                    self.by_hash.entry(hash).or_default().push(slot);
                    slot
                };
                let (min, max, anchor) = self.output.recipes[slot as usize].crop();
                let mut instance =
                    placement(&stamp, world, self.camera, self.height, min, max, anchor);
                instance.sprite_index = slot;
                if !instance.width.is_finite()
                    || instance.width <= 0.
                    || !instance.height.is_finite()
                    || instance.height <= 0.
                    || !instance.center.iter().all(|value| value.is_finite())
                {
                    return Err("Invalid stamp world placement".into());
                }
                self.output.instances.push(instance);
                self.metadata_bytes = self.metadata_bytes.saturating_add(32);
                if self.metadata_bytes > self.metadata_limit {
                    return Err(
                        "Stamp placement metadata exceeds the storage-binding budget".into(),
                    );
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
    pub fn finish(self) -> Result<StampCatalog, String> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        if !self.done {
            return Err("Stamp recipe catalogue is incomplete".into());
        }
        Ok(self.output)
    }
}
pub(super) fn visual_hash(stamp: &IsoPaintStamp) -> u64 {
    let mut hash = DefaultHasher::new();
    stamp.kind.hash(&mut hash);
    stamp.variant.hash(&mut hash);
    stamp.size.to_bits().hash(&mut hash);
    stamp.rotation.to_bits().hash(&mut hash);
    stamp.opacity.to_bits().hash(&mut hash);
    stamp.variation.hash(&mut hash);
    stamp.material_id.hash(&mut hash);
    stamp.color.hash(&mut hash);
    stamp.palette_colors.hash(&mut hash);
    stamp.source_material_pass.hash(&mut hash);
    stamp
        .source_image
        .as_ref()
        .map(|image| Arc::as_ptr(image) as usize)
        .hash(&mut hash);
    hash.finish()
}
pub(super) fn same_visual(a: &IsoPaintStamp, b: &IsoPaintStamp) -> bool {
    a.kind == b.kind
        && a.variant == b.variant
        && a.size.to_bits() == b.size.to_bits()
        && a.rotation.to_bits() == b.rotation.to_bits()
        && a.opacity.to_bits() == b.opacity.to_bits()
        && a.variation == b.variation
        && a.material_id == b.material_id
        && a.color == b.color
        && a.palette_colors == b.palette_colors
        && a.source_material_pass == b.source_material_pass
        && match (&a.source_image, &b.source_image) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
}
