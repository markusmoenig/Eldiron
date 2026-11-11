use crate::{
    paint::Rect,
    text::{GlyphBitmap, GlyphKey},
};
use renderkit::{
    AlphaMode, ColorSpace, GpuContext, GpuImage, ImageAsset, ImageDescriptor, RenderError, Result,
};
use std::{collections::HashMap, sync::Arc};
#[derive(Clone)]
pub struct Entry {
    pub image: Arc<GpuImage>,
    pub uv: Rect,
    pub bitmap: Arc<GlyphBitmap>,
}
struct Page {
    image: Arc<GpuImage>,
    x: u32,
    y: u32,
    row: u32,
}
pub struct GlyphAtlas {
    entries: HashMap<GlyphKey, Entry>,
    pages: Vec<Page>,
}
impl Default for GlyphAtlas {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            pages: vec![],
        }
    }
}
impl GlyphAtlas {
    pub fn get(&self, key: &GlyphKey) -> Option<Entry> {
        self.entries.get(key).cloned()
    }
    pub fn insert(
        &mut self,
        context: &GpuContext,
        key: GlyphKey,
        bitmap: Arc<GlyphBitmap>,
    ) -> Result<Entry> {
        const SIZE: u32 = 1024;
        const LIMIT: usize = 4;
        let (w, h) = (bitmap.width, bitmap.height);
        if w + 2 > SIZE || h + 2 > SIZE {
            return Err(RenderError("glyph exceeds atlas dimensions".into()));
        }
        let fits = |p: &Page| {
            let y = if p.x + w + 2 > SIZE { p.y + p.row } else { p.y };
            y + h + 2 <= SIZE
        };
        if self.pages.last().is_none_or(|p| !fits(p)) {
            // Existing prepared frames retain Arc leases to old pages.
            if self.pages.len() >= LIMIT {
                self.pages.clear();
                self.entries.clear();
            }
            self.pages.push(Page {
                image: GpuImage::new(
                    context,
                    ImageDescriptor {
                        width: SIZE,
                        height: SIZE,
                        format: renderkit::wgpu::TextureFormat::Rgba16Float,
                        color_space: ColorSpace::Linear,
                        alpha_mode: AlphaMode::Premultiplied,
                    },
                )?,
                x: 1,
                y: 1,
                row: 0,
            });
        }
        let page = self.pages.last_mut().unwrap();
        if page.x + w + 1 > SIZE {
            page.x = 1;
            page.y += page.row;
            page.row = 0;
        }
        let (x, y) = (page.x, page.y);
        page.x += w + 2;
        page.row = page.row.max(h + 2);
        let pixels = if bitmap.color {
            bitmap.pixels.clone()
        } else {
            bitmap
                .pixels
                .iter()
                .flat_map(|&a| [255, 255, 255, a])
                .collect()
        };
        let mut source = ImageAsset::rgba(w, h, pixels).map_err(|e| RenderError(e.to_string()))?;
        source.alpha_mode = bitmap.alpha_mode;
        let pixels = renderkit::linear_premultiplied_pixels(&source);
        context.record_upload(pixels.len() as u64);
        context.queue.write_texture(
            renderkit::wgpu::TexelCopyTextureInfo {
                texture: &page.image.texture,
                mip_level: 0,
                origin: renderkit::wgpu::Origin3d { x, y, z: 0 },
                aspect: renderkit::wgpu::TextureAspect::All,
            },
            &pixels,
            renderkit::wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 8),
                rows_per_image: Some(h),
            },
            renderkit::wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        let entry = Entry {
            image: page.image.clone(),
            uv: Rect::new(
                x as f32 / SIZE as f32,
                y as f32 / SIZE as f32,
                w as f32 / SIZE as f32,
                h as f32 / SIZE as f32,
            ),
            bitmap,
        };
        self.entries.insert(key, entry.clone());
        Ok(entry)
    }
}
