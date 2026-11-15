//! Camera-independent current UV/selection publication. CPU preparation owns
//! shared authored pages and emits no renderer, history or stitched atlas.
use super::*;
use crate::surface_paint::Builder;
use rusterix::surface_paint::{SurfacePaint, SurfacePaintPage, SurfacePaintValidation};
use std::sync::Arc;

#[derive(Clone, PartialEq)]
pub struct SurfacePaintRequest {
    visible: bool,
    chunks: IndexMap<String, IsoPaintBakedChunk>,
    selection: Option<IsoPaintSelection>,
    owners: Vec<IsoPaintOwner>,
}
impl SurfacePaintRequest {
    pub fn capture(layer: &IsoPaintLayer) -> Self {
        let selection = layer
            .selection_visible
            .then(|| layer.active_selection())
            .flatten()
            .cloned();
        let current =
            layer.baked_version == ISO_PAINT_BAKE_VERSION && (layer.visible || selection.is_some());
        Self {
            visible: layer.visible,
            chunks: if current {
                layer.baked_chunks.clone()
            } else {
                Default::default()
            },
            selection,
            owners: if current {
                layer.surface_instance_owners.clone()
            } else {
                Vec::new()
            },
        }
    }
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty() && self.selection.is_none()
    }
    pub fn prepare(self, table_limit: u64) -> SurfacePaintPreparation {
        let selection = self.selection.clone().map(Arc::new);
        let empty = self.is_empty();
        let invalid_selection = selection.as_ref().is_some_and(|selection| {
            selection
                .chunks
                .values()
                .any(|chunk| chunk.mask.len() != (ISO_PAINT_BAKED_CHUNK_SIZE as usize).pow(2))
        });

        SurfacePaintPreparation {
            display: if self.visible {
                self.chunks.clone()
            } else {
                Default::default()
            },
            original: self.chunks.clone().into_values(),
            owners: self.owners.clone().into_iter(),
            selections: selection
                .as_ref()
                .map(|s| s.chunks.values().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
                .into_iter(),
            selection,
            request: self,
            stage: if empty { Stage::Done } else { Stage::Original },
            current_selection: None,
            alpha: Vec::new(),
            alpha_seen: HashSet::new(),
            has_alpha: false,
            packed: IndexMap::<String, IsoPaintBakedChunk>::new().into_values(),
            entries: Vec::new(),
            pages: Vec::new(),
            columns: 1,
            size: [0; 2],
            table_len: 2,
            table_limit,
            hash: DefaultHasher::new(),
            validation: None,
            source: None,
            failure: invalid_selection.then(|| "Invalid selection mask size".into()),
        }
    }
}

pub struct SurfacePaintPublication {
    request: SurfacePaintRequest,
    source: Option<Arc<SurfacePaint>>,
}
impl SurfacePaintPublication {
    pub fn matches(&self, request: &SurfacePaintRequest) -> bool {
        &self.request == request
    }
    pub fn source(&self) -> Option<&Arc<SurfacePaint>> {
        self.source.as_ref()
    }
}
enum Stage {
    Original,
    Owners,
    Selection,
    Table,
    Pages,
    Validate,
    Done,
}
struct SelectionWork {
    mask: IsoPaintSelectionChunk,
    key: String,
    pixel: usize,
}
pub struct SurfacePaintPreparation {
    request: SurfacePaintRequest,
    original: indexmap::map::IntoValues<String, IsoPaintBakedChunk>,
    owners: std::vec::IntoIter<IsoPaintOwner>,
    selection: Option<Arc<IsoPaintSelection>>,
    selections: std::vec::IntoIter<IsoPaintSelectionChunk>,
    current_selection: Option<SelectionWork>,
    display: IndexMap<String, IsoPaintBakedChunk>,
    packed: indexmap::map::IntoValues<String, IsoPaintBakedChunk>,
    stage: Stage,
    alpha: Vec<scenevm::GeoId>,
    alpha_seen: HashSet<scenevm::GeoId>,
    has_alpha: bool,
    entries: Vec<scenevm::Raster3DSurfacePaintEntry>,
    pages: Vec<SurfacePaintPage>,
    columns: usize,
    size: [u32; 2],
    table_len: usize,
    table_limit: u64,
    hash: DefaultHasher,
    validation: Option<SurfacePaintValidation>,
    source: Option<Arc<SurfacePaint>>,
    failure: Option<String>,
}
impl SurfacePaintPreparation {
    /// One unit scans a fixed original page, 64 selection pixels, 256 empty
    /// table slots or one page's metadata. Elapsed checks separate units.
    pub fn advance(&mut self, units: usize) -> Result<bool, String> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        let started = web_time::Instant::now();
        for _ in 0..units {
            if let Err(error) = self.step() {
                self.failure = Some(error.clone());
                return Err(error);
            }
            if matches!(self.stage, Stage::Done) {
                return Ok(true);
            }
            if started.elapsed() >= web_time::Duration::from_millis(2) {
                break;
            }
        }
        Ok(matches!(self.stage, Stage::Done))
    }
    fn alpha(&mut self, owner: &IsoPaintOwner) {
        let id = Builder::iso_paint_owner_geo_id(owner);
        if self.alpha_seen.insert(id) {
            self.alpha.push(id);
        }
    }
    fn step(&mut self) -> Result<(), String> {
        let side = ISO_PAINT_BAKED_CHUNK_SIZE as usize;
        match self.stage {
            Stage::Original => {
                if let Some(chunk) = self.original.next() {
                    if chunk.color_rgba.len() != side * side * 4
                        || chunk.material_rgba.len() != side * side * 4
                    {
                        return Err("Invalid current paint plane size".into());
                    }
                    if Builder::surface_paint_chunk_needs_alpha(&chunk) {
                        self.has_alpha = true;
                        self.alpha(&chunk.owner);
                    }
                } else {
                    self.stage = Stage::Owners;
                }
            }
            Stage::Owners => {
                if let Some(owner) = self.owners.next() {
                    if self.has_alpha {
                        self.alpha(&owner);
                    }
                } else {
                    self.stage = Stage::Selection;
                }
            }
            Stage::Selection => {
                if self.current_selection.is_none() {
                    if let Some(mask) = self.selections.next() {
                        if mask.mask.len() != side * side {
                            return Err("Invalid selection mask size".into());
                        }
                        let key =
                            IsoPaintRenderer::selection_chunk_target(&mut self.display, &mask);
                        self.current_selection = Some(SelectionWork {
                            mask,
                            key,
                            pixel: 0,
                        });
                    } else {
                        if self.display.is_empty() {
                            self.stage = Stage::Done;
                            return Ok(());
                        }
                        let count = self.display.len();
                        self.columns = (count as f32).sqrt().ceil().max(1.) as usize;
                        let dimension = |cells: usize| {
                            cells
                                .checked_mul(side)
                                .and_then(|v| u32::try_from(v).ok())
                                .ok_or("Paint atlas dimensions exceed u32")
                        };
                        self.size = [
                            dimension(self.columns)?,
                            dimension(count.div_ceil(self.columns))?,
                        ];
                        self.table_len = count
                            .checked_mul(2)
                            .and_then(usize::checked_next_power_of_two)
                            .filter(|n| *n <= u32::MAX as usize)
                            .ok_or("Paint metadata exceeds u32")?
                            .max(2);
                        if self.table_len as u64
                            * std::mem::size_of::<scenevm::Raster3DSurfacePaintEntry>() as u64
                            + 20
                            > self.table_limit
                        {
                            return Err("Paint metadata exceeds this device's binding limit".into());
                        }
                        self.entries
                            .try_reserve_exact(self.table_len)
                            .map_err(|e| e.to_string())?;
                        self.pages
                            .try_reserve_exact(count)
                            .map_err(|e| e.to_string())?;
                        self.packed = std::mem::take(&mut self.display).into_values();
                        self.stage = Stage::Table;
                        return Ok(());
                    }
                }
                let current = self.current_selection.as_mut().unwrap();
                let chunk = self.display.get_mut(&current.key).unwrap();
                let end = (current.pixel + 64).min(side * side);
                for index in current.pixel..end {
                    IsoPaintRenderer::selection_chunk_pixel(
                        self.selection.as_ref().unwrap(),
                        &current.mask,
                        chunk,
                        index,
                    );
                }
                current.pixel = end;
                if end == side * side {
                    chunk.revision = chunk.revision.wrapping_add(1);
                    self.current_selection = None;
                }
            }
            Stage::Table => {
                self.entries.resize(
                    (self.entries.len() + 256).min(self.table_len),
                    scenevm::Raster3DSurfacePaintEntry {
                        geo: [0; 4],
                        uv_origin: [0; 2],
                        uv_size: [0; 2],
                        atlas_rect: [0; 4],
                    },
                );
                if self.entries.len() == self.table_len {
                    self.stage = Stage::Pages;
                }
            }
            Stage::Pages => {
                let Some(chunk) = self.packed.next() else {
                    if self.pages.is_empty() {
                        self.stage = Stage::Done;
                        return Ok(());
                    }
                    let source = Arc::new(SurfacePaint {
                        key: self.hash.finish(),
                        width: self.size[0],
                        height: self.size[1],
                        pages: std::mem::take(&mut self.pages),
                        entries: std::mem::take(&mut self.entries).into(),
                        alpha_geo_ids: Arc::new(std::mem::take(&mut self.alpha)),
                    });
                    self.validation = Some(SurfacePaintValidation::new(source)?);
                    self.stage = Stage::Validate;
                    return Ok(());
                };
                let index = self.pages.len();
                let origin = [
                    (index % self.columns * side) as u32,
                    (index / self.columns * side) as u32,
                ];
                let mut slot = Builder::surface_paint_entry_hash(chunk.paint_geo, chunk.origin)
                    as usize
                    & (self.table_len - 1);
                while self.entries[slot].uv_size != [0; 2] {
                    if self.entries[slot].geo == chunk.paint_geo
                        && self.entries[slot].uv_origin == chunk.origin
                    {
                        return Err("Duplicate UV paint source key".into());
                    }
                    slot = (slot + 1) & (self.table_len - 1);
                }
                self.entries[slot] = scenevm::Raster3DSurfacePaintEntry {
                    geo: chunk.paint_geo,
                    uv_origin: chunk.origin,
                    uv_size: [side as u32; 2],
                    atlas_rect: [origin[0], origin[1], side as u32, side as u32],
                };
                chunk.paint_geo.hash(&mut self.hash);
                chunk.origin.hash(&mut self.hash);
                chunk.color_rgba.hash(&mut self.hash);
                chunk.material_rgba.hash(&mut self.hash);
                self.pages.push(SurfacePaintPage {
                    atlas_origin: origin,
                    color_rgba: chunk.color_rgba,
                    material_rgba: chunk.material_rgba,
                });
            }
            Stage::Validate => {
                if self.validation.as_mut().unwrap().advance(64)? {
                    self.source = Some(self.validation.take().unwrap().finish()?);
                    self.stage = Stage::Done;
                }
            }
            Stage::Done => (),
        }
        Ok(())
    }
    pub fn finish(self) -> Result<SurfacePaintPublication, String> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        if !matches!(self.stage, Stage::Done) {
            return Err("UV paint publication is incomplete".into());
        }
        Ok(SurfacePaintPublication {
            request: self.request,
            source: self.source,
        })
    }
}
