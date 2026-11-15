//! Cooperatively append lazy stamp pages to the complete frozen key table.
use super::*;
use crate::iso_paint_render::StampRecipe;
use std::sync::Arc;

pub struct StampPageIndexBuilder {
    index: PaintPageIndex,
    recipes: std::iter::Enumerate<std::vec::IntoIter<Arc<StampRecipe>>>,
    current: Option<(u32, Arc<StampRecipe>, [u32; 2])>,
    originals: usize,
    next_original: usize,
    table_len: usize,
    done: bool,
    failure: Option<String>,
}
impl StampPageIndexBuilder {
    pub fn new(
        mut index: PaintPageIndex,
        recipes: Vec<Arc<StampRecipe>>,
        table_byte_limit: u64,
    ) -> Result<Self, String> {
        let total = recipes
            .iter()
            .try_fold(index.pages.len(), |count, recipe| {
                let size = recipe.size();
                count.checked_add((size[0].div_ceil(SIZE) * size[1].div_ceil(SIZE)) as usize)
            })
            .ok_or("Stamp page count overflow")?;
        let table_len = total
            .checked_mul(2)
            .and_then(usize::checked_next_power_of_two)
            .filter(|length| *length <= u32::MAX as usize)
            .ok_or("Stamp page lookup exceeds u32")?
            .max(2);
        if table_len as u64 * size_of::<Raster3DSurfacePaintEntry>() as u64 > table_byte_limit {
            return Err("Combined paint/stamp lookup exceeds metadata budget".into());
        }
        index.entries = Arc::new(Vec::new());
        index.sources_by_slot.clear();
        Arc::get_mut(&mut index.entries)
            .unwrap()
            .try_reserve_exact(table_len)
            .map_err(|e| e.to_string())?;
        index
            .sources_by_slot
            .try_reserve_exact(table_len)
            .map_err(|e| e.to_string())?;
        index
            .pages
            .try_reserve_exact(total - index.pages.len())
            .map_err(|e| e.to_string())?;
        let originals = index.pages.len();
        Ok(Self {
            index,
            recipes: recipes.into_iter().enumerate(),
            current: None,
            originals,
            next_original: 0,
            table_len,
            done: false,
            failure: None,
        })
    }
    pub fn advance(&mut self, items: usize) -> Result<bool, String> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        let result = self.advance_inner(items);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
    fn advance_inner(&mut self, items: usize) -> Result<bool, String> {
        let end = self
            .index
            .entries
            .len()
            .saturating_add(items.saturating_mul(64))
            .min(self.table_len);
        Arc::get_mut(&mut self.index.entries).unwrap().resize(
            end,
            Raster3DSurfacePaintEntry {
                geo: [0; 4],
                uv_origin: [0; 2],
                uv_size: [0; 2],
                atlas_rect: [0; 4],
            },
        );
        self.index.sources_by_slot.resize(end, None);
        if end < self.table_len {
            return Ok(false);
        }
        for _ in 0..items {
            if self.next_original < self.originals {
                self.insert(self.next_original)?;
                self.next_original += 1;
                continue;
            }
            if self.current.is_none() {
                let Some((id, recipe)) = self.recipes.next() else {
                    self.done = true;
                    break;
                };
                self.current = Some((
                    u32::try_from(id).map_err(|_| "Stamp recipe index exceeds u32")?,
                    recipe,
                    [0; 2],
                ));
            }
            let (id, recipe, origin) = self.current.as_ref().unwrap();
            let source = self.index.pages.len();
            self.index.pages.push(PaintPageSource {
                geo: [scenevm::core::PAGED_STAMP_GEO, *id, 0, 0],
                uv_origin: [origin[0] as i32, origin[1] as i32],
                revision: 0,
                color: Default::default(),
                material: Default::default(),
            });
            self.index.generated.insert(source, recipe.clone(), *origin);
            self.insert(source)?;
            let (_, recipe, origin) = self.current.as_mut().unwrap();
            origin[0] += SIZE;
            if origin[0] >= recipe.size()[0] {
                origin[0] = 0;
                origin[1] += SIZE;
                if origin[1] >= recipe.size()[1] {
                    self.current = None;
                }
            }
        }
        Ok(self.done)
    }
    fn insert(&mut self, source: usize) -> Result<(), String> {
        let page = &self.index.pages[source];
        let mut slot = Builder::surface_paint_entry_hash(page.geo, page.uv_origin) as usize
            & (self.table_len - 1);
        while self.index.entries[slot].uv_size != [0; 2] {
            if self.index.entries[slot].geo == page.geo
                && self.index.entries[slot].uv_origin == page.uv_origin
            {
                return Err("Duplicate authored/private stamp source key".into());
            }
            slot = (slot + 1) & (self.table_len - 1);
        }
        Arc::get_mut(&mut self.index.entries).unwrap()[slot] = Raster3DSurfacePaintEntry {
            geo: page.geo,
            uv_origin: page.uv_origin,
            uv_size: [SIZE; 2],
            atlas_rect: [0; 4],
        };
        self.index.sources_by_slot[slot] = Some(source);
        Ok(())
    }
    pub fn finish(self) -> Result<PaintPageIndex, String> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        if !self.done {
            return Err("Stamp page index is incomplete".into());
        }
        Ok(self.index)
    }
}
