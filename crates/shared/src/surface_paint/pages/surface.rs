//! Convert a current immutable live publication into complete original keys.
//! Work advances by metadata units and shares authored pixel backing. The source
//! may exceed GPU atlas dimensions; residency chooses its own physical layout.
use super::*;
use rusterix::surface_paint::{SurfacePaint, SurfacePaintValidation};
use std::{collections::HashMap, sync::Arc};

pub struct SurfacePaintIndexBuilder {
    source: Arc<SurfacePaint>,
    validation: Option<SurfacePaintValidation>,
    index: PaintPageIndex,
    origins: HashMap<[u32; 2], usize>,
    page: usize,
    entry: usize,
    alpha: usize,
    failure: Option<String>,
}
impl SurfacePaintIndexBuilder {
    pub fn new(source: Arc<SurfacePaint>, table_byte_limit: u64) -> Result<Self, String> {
        let bytes = size_of_val(source.entries.as_ref()) as u64;
        if bytes > table_byte_limit || source.entries.len() > u32::MAX as usize {
            return Err("Live paint source exceeds its metadata byte limit".into());
        }
        let validation = SurfacePaintValidation::new(source.clone())?;
        let mut index = PaintPageIndex {
            pages: Vec::new(),
            #[cfg(feature = "graphics")]
            generated: Default::default(),
            entries: Vec::new(),
            sources_by_slot: Vec::new(),
            alpha_geo_ids: Vec::new(),
        };
        index
            .entries
            .try_reserve_exact(source.entries.len())
            .map_err(|e| e.to_string())?;
        index
            .sources_by_slot
            .try_reserve_exact(source.entries.len())
            .map_err(|e| e.to_string())?;
        index
            .alpha_geo_ids
            .try_reserve_exact(source.alpha_geo_ids.len())
            .map_err(|e| e.to_string())?;
        Ok(Self {
            source,
            validation: Some(validation),
            index,
            origins: HashMap::new(),
            page: 0,
            entry: 0,
            alpha: 0,
            failure: None,
        })
    }
    /// One unit validates or indexes one original page, key slot (including
    /// empty slots), or alpha owner. No partially built index can escape.
    pub fn advance(&mut self, units: usize) -> Result<bool, String> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        let result = self.advance_inner(units);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
    fn advance_inner(&mut self, units: usize) -> Result<bool, String> {
        if let Some(validation) = &mut self.validation {
            if validation.advance(units)? {
                self.validation.take().unwrap().finish()?;
            }
            return Ok(false);
        }
        for _ in 0..units {
            if let Some(page) = self.source.pages.get(self.page) {
                self.origins.try_reserve(1).map_err(|e| e.to_string())?;
                self.origins.insert(page.atlas_origin, self.page);
                self.page += 1;
            } else if let Some(entry) = self.source.entries.get(self.entry) {
                let mut key = *entry;
                let source = if key.uv_size != [0; 2] {
                    let at = self
                        .origins
                        .get(&[key.atlas_rect[0], key.atlas_rect[1]])
                        .ok_or("Live paint key refers to an absent original page")?;
                    let page = &self.source.pages[*at];
                    let source = self.index.pages.len();
                    self.index.pages.try_reserve(1).map_err(|e| e.to_string())?;
                    self.index.pages.push(PaintPageSource {
                        geo: key.geo,
                        uv_origin: key.uv_origin,
                        revision: self.source.key,
                        color: page.color_rgba.clone(),
                        material: page.material_rgba.clone(),
                    });
                    // Preserve slot/key/UV identity while removing the original
                    // atlas address. Missing remains distinct from absent.
                    key.atlas_rect = [0; 4];
                    Some(source)
                } else {
                    None
                };
                self.index.entries.push(key);
                self.index.sources_by_slot.push(source);
                self.entry += 1;
            } else if let Some(alpha) = self.source.alpha_geo_ids.get(self.alpha) {
                self.index.alpha_geo_ids.push(*alpha);
                self.alpha += 1;
            } else {
                break;
            }
        }
        Ok(self.ready())
    }
    fn ready(&self) -> bool {
        self.validation.is_none()
            && self.page == self.source.pages.len()
            && self.entry == self.source.entries.len()
            && self.alpha == self.source.alpha_geo_ids.len()
    }
    pub fn finish(self) -> Result<PaintPageIndex, String> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.ready() {
            return Err("Live paint source index is incomplete".into());
        }
        Ok(self.index)
    }
}
