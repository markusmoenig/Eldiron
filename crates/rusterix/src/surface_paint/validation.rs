//! Cooperative validation keeps the exact immutable publication alive. Callers
//! cannot switch the pages/table underneath a partially checked source.
use super::*;

fn dimensions(source: &SurfacePaint) -> Result<(), String> {
    if source.width == 0
        || source.height == 0
        || source.pages.is_empty()
        || source.width % PAGE_SIZE != 0
        || source.height % PAGE_SIZE != 0
        || source.entries.len() < 2
        || !source.entries.len().is_power_of_two()
    {
        return Err("Invalid surface paint page dimensions/table".into());
    }
    Ok(())
}
#[derive(Default)]
struct Cursor {
    pages: usize,
    entries: usize,
    origins: HashSet<[u32; 2]>,
}
impl Cursor {
    fn advance(&mut self, source: &SurfacePaint, units: usize) -> Result<bool, String> {
        for _ in 0..units {
            if let Some(page) = source.pages.get(self.pages) {
                let bytes = (PAGE_SIZE * PAGE_SIZE * 4) as usize;
                if page.color_rgba.len() != bytes
                    || page.material_rgba.len() != bytes
                    || !self.origins.insert(page.atlas_origin)
                    || !(0..2).all(|axis| {
                        page.atlas_origin[axis] % PAGE_SIZE == 0
                            && page.atlas_origin[axis]
                                .checked_add(PAGE_SIZE)
                                .is_some_and(|end| end <= [source.width, source.height][axis])
                    })
                {
                    return Err("Invalid, overlapping or out-of-bounds surface paint page".into());
                }
                self.pages += 1;
            } else if let Some(entry) = source.entries.get(self.entries) {
                if entry.uv_size != [0; 2]
                    && (entry.uv_size != [PAGE_SIZE; 2]
                        || entry.atlas_rect[2..] != [PAGE_SIZE; 2]
                        || !self
                            .origins
                            .contains(&[entry.atlas_rect[0], entry.atlas_rect[1]]))
                {
                    return Err("Surface paint lookup refers to an absent page".into());
                }
                self.entries += 1;
            } else {
                return Ok(true);
            }
        }
        Ok(self.pages == source.pages.len() && self.entries == source.entries.len())
    }
}
pub struct SurfacePaintValidation {
    source: Arc<SurfacePaint>,
    cursor: Cursor,
    failure: Option<String>,
}
impl SurfacePaintValidation {
    pub fn new(source: Arc<SurfacePaint>) -> Result<Self, String> {
        dimensions(&source)?;
        Ok(Self {
            source,
            cursor: Default::default(),
            failure: None,
        })
    }
    /// One unit checks one original page or one lookup slot, including empties.
    pub fn advance(&mut self, units: usize) -> Result<bool, String> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        match self.cursor.advance(&self.source, units) {
            Ok(ready) => Ok(ready),
            Err(error) => {
                self.failure = Some(error.clone());
                Err(error)
            }
        }
    }
    pub fn finish(self) -> Result<Arc<SurfacePaint>, String> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        if self.cursor.pages != self.source.pages.len()
            || self.cursor.entries != self.source.entries.len()
        {
            return Err("Surface paint validation is incomplete".into());
        }
        Ok(self.source)
    }
}
impl SurfacePaint {
    /// Explicit synchronous consumers use the same validation kernel.
    pub fn validate(&self) -> Result<(), String> {
        dimensions(self)?;
        Cursor::default().advance(self, usize::MAX).map(|_| ())
    }
}
