use crate::Texture;
#[cfg(feature = "gpu")]
use crate::texture::TextureGPU;
use rustc_hash::FxHashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const ATLAS_FRAME_PADDING: u32 = 4;
const SEMANTIC_MATERIAL_MARKER: u8 = 0xFE;
const DEFAULT_MATERIAL_PIXEL: [u8; 4] = [7u8, 15u8, 128u8, 128u8];

#[derive(Debug, Clone, Copy)]
pub struct AtlasEntry {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct AtlasTileMeta {
    pub first_frame: u32,
    pub frame_count: u32,
}

pub struct AtlasGpuTables {
    pub metas: Vec<AtlasTileMeta>,
    pub frames: Vec<AtlasEntry>,
}

#[derive(Debug, Clone)]
pub struct Tile {
    pub w: u32,
    pub h: u32,
    pub frames: Vec<Vec<u8>>,
    pub material_frames: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TileEmissiveSummary {
    pub color_linear: [f32; 3],
    pub strength: f32,
    pub coverage: f32,
    pub hotspot_uv: [f32; 2],
    pub hotspot_strength: f32,
}

pub struct SharedAtlasInner {
    pub tiles_map: FxHashMap<Uuid, Tile>,
    pub tiles_order: Vec<Uuid>,
    pub atlas: Texture,
    pub atlas_material: Texture,
    pub atlas_dirty: bool,
    pub layout_dirty: bool,
    pub layout_version: u64,
    pub tiles_index_map: FxHashMap<Uuid, u32>,
    pub atlas_map: FxHashMap<Uuid, Vec<AtlasEntry>>,
    pub auto_size: bool,
    pub content_version: u64,
}

#[derive(Clone)]
pub struct SharedAtlas {
    inner: Arc<Mutex<SharedAtlasInner>>,
}

impl SharedAtlas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SharedAtlasInner {
                tiles_map: FxHashMap::default(),
                tiles_order: Vec::new(),
                atlas: Texture::new(width, height),
                atlas_material: Texture::new(width, height),
                atlas_dirty: true,
                layout_dirty: true,
                layout_version: 0,
                tiles_index_map: FxHashMap::default(),
                atlas_map: FxHashMap::default(),
                auto_size: true,
                content_version: 0,
            })),
        }
    }

    pub fn dims(&self) -> (u32, u32) {
        let guard = self.inner.lock().unwrap();
        (guard.atlas.width, guard.atlas.height)
    }

    pub fn add_tile(
        &self,
        id: Uuid,
        width: u32,
        height: u32,
        frames: Vec<Vec<u8>>,
        material_frames: Vec<Vec<u8>>,
    ) {
        let mut guard = self.inner.lock().unwrap();
        guard.tiles_map.insert(
            id,
            Tile {
                w: width,
                h: height,
                frames,
                material_frames,
            },
        );
        if !guard.tiles_order.contains(&id) {
            guard.tiles_order.push(id);
        }
        guard.atlas_dirty = true;
        guard.layout_dirty = true;
        guard.content_version = guard.content_version.wrapping_add(1);
    }

    pub fn layout_version(&self) -> u64 {
        let guard = self.inner.lock().unwrap();
        guard.layout_version
    }

    pub fn content_version(&self) -> u64 {
        let guard = self.inner.lock().unwrap();
        guard.content_version
    }

    pub fn tile_emissive_summaries(&self) -> Vec<TileEmissiveSummary> {
        let guard = self.inner.lock().unwrap();
        let mut summaries = Vec::new();
        for id in &guard.tiles_order {
            if !guard.tiles_index_map.contains_key(id) {
                continue;
            }
            let Some(tile) = guard.tiles_map.get(id) else {
                continue;
            };
            summaries.push(summarize_tile_emission(tile));
        }
        summaries
    }

    pub fn tile_index(&self, id: &Uuid) -> Option<u32> {
        let guard = self.inner.lock().unwrap();
        guard.tiles_index_map.get(id).copied()
    }

    /// Returns true if the tile at `tile_index` has any non-opaque texels/material opacity.
    pub fn tile_index_has_translucency(&self, tile_index: u32) -> bool {
        let guard = self.inner.lock().unwrap();
        let Some(id) = guard.tiles_order.get(tile_index as usize) else {
            return false;
        };
        let Some(tile) = guard.tiles_map.get(id) else {
            return false;
        };

        // Color alpha channel.
        if tile
            .frames
            .iter()
            .any(|frame| frame.chunks_exact(4).any(|px| px[3] < 255))
        {
            return true;
        }

        if tile.material_frames.iter().any(|frame| {
            frame
                .chunks_exact(4)
                .any(|px| material_opacity_emissive(px).0 < 0.999)
        }) {
            return true;
        }

        false
    }

    /// Get the raw tile data (width, height, and first frame RGBA pixels)
    pub fn get_tile_data(&self, id: Uuid) -> Option<(u32, u32, Vec<u8>)> {
        let guard = self.inner.lock().unwrap();
        guard.tiles_map.get(&id).map(|tile| {
            let frame_data = if tile.frames.is_empty() {
                vec![]
            } else {
                tile.frames[0].clone()
            };
            (tile.w, tile.h, frame_data)
        })
    }

    /// Sample a tile frame alpha at normalized UV (0..1).
    pub fn sample_tile_alpha(&self, id: &Uuid, anim_frame: u32, uv: [f32; 2]) -> Option<u8> {
        let guard = self.inner.lock().unwrap();
        let tile = guard.tiles_map.get(id)?;
        let frame_count = tile.frames.len();
        if tile.w == 0 || tile.h == 0 || frame_count == 0 {
            return None;
        }
        let frame = &tile.frames[(anim_frame as usize) % frame_count];
        let x = ((uv[0].clamp(0.0, 0.9999)) * tile.w as f32).floor() as usize;
        let y = ((uv[1].clamp(0.0, 0.9999)) * tile.h as f32).floor() as usize;
        let idx = (y * tile.w as usize + x) * 4;
        frame.get(idx + 3).copied()
    }

    /// Returns true if a tile frame pixel matches the frame's top-left color (RGB only).
    pub fn tile_pixel_matches_topleft_rgb(
        &self,
        id: &Uuid,
        anim_frame: u32,
        uv: [f32; 2],
    ) -> Option<bool> {
        let guard = self.inner.lock().unwrap();
        let tile = guard.tiles_map.get(id)?;
        let frame_count = tile.frames.len();
        if tile.w == 0 || tile.h == 0 || frame_count == 0 {
            return None;
        }
        let frame = &tile.frames[(anim_frame as usize) % frame_count];
        if frame.len() < 4 {
            return None;
        }
        let x = ((uv[0].clamp(0.0, 0.9999)) * tile.w as f32).floor() as usize;
        let y = ((uv[1].clamp(0.0, 0.9999)) * tile.h as f32).floor() as usize;
        let idx = (y * tile.w as usize + x) * 4;
        let px = frame.get(idx..idx + 3)?;
        let key = &frame[0..3];
        Some(px == key)
    }

    pub fn gpu_tile_tables(&self) -> AtlasGpuTables {
        let guard = self.inner.lock().unwrap();
        let mut metas: Vec<AtlasTileMeta> = Vec::new();
        let mut frames: Vec<AtlasEntry> = Vec::new();
        for id in &guard.tiles_order {
            if let Some(rects) = guard.atlas_map.get(id) {
                if guard.tiles_index_map.contains_key(id) && !rects.is_empty() {
                    let first = frames.len() as u32;
                    frames.extend(rects.iter().cloned());
                    metas.push(AtlasTileMeta {
                        first_frame: first,
                        frame_count: rects.len() as u32,
                    });
                }
            }
        }
        AtlasGpuTables { metas, frames }
    }

    pub fn remove_tile(&self, id: &Uuid) {
        let mut guard = self.inner.lock().unwrap();
        guard.tiles_map.remove(id);
        guard.tiles_order.retain(|tid| tid != id);
        guard.atlas_map.remove(id);
        guard.atlas_dirty = true;
        guard.layout_dirty = true;
        guard.content_version = guard.content_version.wrapping_add(1);
    }

    pub fn clear(&self) {
        let mut guard = self.inner.lock().unwrap();
        guard.tiles_map.clear();
        guard.tiles_order.clear();
        guard.atlas.data.fill(0);
        fill_default_material_pixels(&mut guard.atlas_material.data);
        guard.atlas_map.clear();
        guard.atlas_dirty = true;
        guard.layout_dirty = true;
        guard.content_version = guard.content_version.wrapping_add(1);
    }

    pub fn with_tile_mut<R>(&self, id: &Uuid, f: impl FnOnce(&mut Tile) -> R) -> Option<R> {
        let mut guard = self.inner.lock().unwrap();
        let tile = guard.tiles_map.get_mut(id)?;
        let out = f(tile);
        guard.atlas_dirty = true;
        guard.content_version = guard.content_version.wrapping_add(1);
        // Pixel/material edits don't require repacking atlas layout.
        Some(out)
    }

    pub fn atlas_pixels(&self) -> Vec<u8> {
        let guard = self.inner.lock().unwrap();
        guard.atlas.data.clone()
    }

    pub fn material_atlas_pixels(&self) -> Vec<u8> {
        let guard = self.inner.lock().unwrap();
        guard.atlas_material.data.clone()
    }

    pub fn copy_atlas_to_slice(&self, dst: &mut [u8], buf_w: u32, buf_h: u32) {
        let guard = self.inner.lock().unwrap();
        guard.atlas.copy_to_slice(dst, buf_w, buf_h);
    }

    pub fn copy_material_atlas_to_slice(&self, dst: &mut [u8], buf_w: u32, buf_h: u32) {
        let guard = self.inner.lock().unwrap();
        guard.atlas_material.copy_to_slice(dst, buf_w, buf_h);
    }

    #[cfg(feature = "gpu")]
    pub fn upload_to_gpu_with(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut guard = self.inner.lock().unwrap();
        if guard.atlas_dirty {
            guard.atlas.upload_to_gpu_with(device, queue);
            guard.atlas_material.upload_to_gpu_with(device, queue);
            guard.atlas_dirty = false;
        }
    }

    #[cfg(feature = "gpu")]
    pub fn download_from_gpu_with(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut guard = self.inner.lock().unwrap();
        guard.atlas.download_from_gpu_with(device, queue);
        guard.atlas_material.download_from_gpu_with(device, queue);
    }

    #[cfg(feature = "gpu")]
    pub fn with_views<R>(&self, f: impl FnOnce(&TextureGPU, &TextureGPU) -> R) -> Option<R> {
        let guard = self.inner.lock().unwrap();
        let a = guard.atlas.gpu.as_ref()?;
        let mat = guard.atlas_material.gpu.as_ref()?;
        Some(f(a, mat))
    }

    #[cfg(feature = "gpu")]
    pub fn texture_views(&self) -> Option<(wgpu::TextureView, wgpu::TextureView)> {
        self.with_views(|a, m| {
            // Sample views must include all mip levels so trilinear/mip filtering can work.
            let atlas_view = a
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mat_view = m
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            (atlas_view, mat_view)
        })
    }

    pub fn frame_rect(&self, id: &Uuid, anim_frame: u32) -> Option<AtlasEntry> {
        let guard = self.inner.lock().unwrap();
        let rects = guard.atlas_map.get(id)?;
        if rects.is_empty() {
            return None;
        }
        let idx = (anim_frame as usize) % rects.len();
        rects.get(idx).cloned()
    }

    pub fn ensure_built(&self) -> bool {
        let mut guard = self.inner.lock().unwrap();
        if guard.layout_dirty {
            if guard.auto_size {
                auto_resize_atlas_inner(&mut guard);
            }
            build_atlas_inner(&mut guard);
            guard.layout_dirty = false;
            return true;
        }
        if guard.atlas_dirty {
            repaint_atlas_pixels_inner(&mut guard);
        }
        false
    }

    /// Return a normalized atlas rect (ofs.xy, scale.xy) for a tile, suitable for SDF data packing.
    pub fn sdf_uv4(&self, id: &Uuid, anim_frame: u32) -> Option<[f32; 4]> {
        let mut guard = self.inner.lock().unwrap();
        if guard.layout_dirty {
            build_atlas_inner(&mut guard);
            guard.layout_dirty = false;
        }

        let rects = guard.atlas_map.get(id)?;
        if rects.is_empty() {
            return None;
        }
        let idx = (anim_frame as usize) % rects.len();
        let rect = rects.get(idx)?;
        let w = guard.atlas.width.max(1) as f32;
        let h = guard.atlas.height.max(1) as f32;
        Some([
            rect.x as f32 / w,
            rect.y as f32 / h,
            rect.w as f32 / w,
            rect.h as f32 / h,
        ])
    }

    pub fn tiles_map_len(&self) -> usize {
        let guard = self.inner.lock().unwrap();
        guard.tiles_map.len()
    }

    pub fn resize(&self, width: u32, height: u32) {
        let mut guard = self.inner.lock().unwrap();
        if guard.atlas.width == width && guard.atlas.height == height {
            return;
        }
        guard.atlas = Texture::new(width, height);
        guard.atlas_material = Texture::new(width, height);
        guard.atlas_dirty = true;
        guard.layout_dirty = true;
        guard.atlas_map.clear();
        guard.tiles_index_map.clear();
        guard.layout_version = guard.layout_version.wrapping_add(1);
        guard.content_version = guard.content_version.wrapping_add(1);
        guard.auto_size = false;
    }

    pub fn inner_arc(&self) -> Arc<Mutex<SharedAtlasInner>> {
        Arc::clone(&self.inner)
    }
}

fn build_atlas_inner(inner: &mut SharedAtlasInner) {
    inner.atlas.data.fill(0);
    fill_default_material_pixels(&mut inner.atlas_material.data);
    inner.atlas_map.clear();

    // Pre-allocate hash maps with estimated capacity
    let estimated_tiles = inner.tiles_order.len();
    inner.atlas_map.reserve(estimated_tiles);
    inner.tiles_index_map.reserve(estimated_tiles);

    let mut pen_x: u32 = 0;
    let mut pen_y: u32 = 0;
    let mut shelf_h: u32 = 0;

    for id in &inner.tiles_order {
        let Some(tile) = inner.tiles_map.get(id) else {
            continue;
        };
        let w = tile.w;
        let h = tile.h;
        if w == 0 || h == 0 {
            continue;
        }
        let packed_w = w.saturating_add(ATLAS_FRAME_PADDING * 2);
        let packed_h = h.saturating_add(ATLAS_FRAME_PADDING * 2);
        let frames_len = tile.frames.len();
        let mat_len = tile.material_frames.len();
        let mut rects = Vec::with_capacity(frames_len);
        let need_bytes = (w as usize) * (h as usize) * 4;

        for f in 0..frames_len {
            if pen_x + packed_w > inner.atlas.width {
                pen_x = 0;
                pen_y = pen_y.saturating_add(shelf_h);
                shelf_h = 0;
            }
            if pen_y + packed_h > inner.atlas.height {
                break;
            }
            shelf_h = shelf_h.max(packed_h);

            let frame_owned = alpha_bleed_colors(&tile.frames[f], w, h, 4);
            let mat_owned = if f < mat_len {
                tile.material_frames[f].clone()
            } else {
                default_material_frame(need_bytes)
            };
            blit_rgba_into_with_border(
                &mut inner.atlas.data,
                inner.atlas.width,
                &frame_owned,
                w,
                h,
                pen_x,
                pen_y,
                ATLAS_FRAME_PADDING,
            );
            blit_rgba_into_with_border(
                &mut inner.atlas_material.data,
                inner.atlas_material.width,
                &mat_owned,
                w,
                h,
                pen_x,
                pen_y,
                ATLAS_FRAME_PADDING,
            );

            rects.push(AtlasEntry {
                x: pen_x + ATLAS_FRAME_PADDING,
                y: pen_y + ATLAS_FRAME_PADDING,
                w,
                h,
            });
            pen_x = pen_x.saturating_add(packed_w);
        }

        if !rects.is_empty() {
            inner.atlas_map.insert(*id, rects);
        }
    }

    inner.tiles_index_map.clear();
    for id in &inner.tiles_order {
        if inner.atlas_map.contains_key(id) {
            let idx = inner.tiles_index_map.len() as u32;
            inner.tiles_index_map.insert(*id, idx);
        }
    }
    inner.layout_version = inner.layout_version.wrapping_add(1);
}

fn pack_fits(inner: &SharedAtlasInner, atlas_w: u32, atlas_h: u32) -> bool {
    let mut pen_x: u32 = 0;
    let mut pen_y: u32 = 0;
    let mut shelf_h: u32 = 0;

    for id in &inner.tiles_order {
        let Some(tile) = inner.tiles_map.get(id) else {
            continue;
        };
        if tile.w == 0 || tile.h == 0 {
            continue;
        }

        let packed_w = tile.w.saturating_add(ATLAS_FRAME_PADDING * 2);
        let packed_h = tile.h.saturating_add(ATLAS_FRAME_PADDING * 2);
        if packed_w > atlas_w || packed_h > atlas_h {
            return false;
        }

        for _ in 0..tile.frames.len() {
            if pen_x + packed_w > atlas_w {
                pen_x = 0;
                pen_y = pen_y.saturating_add(shelf_h);
                shelf_h = 0;
            }
            if pen_y + packed_h > atlas_h {
                return false;
            }
            shelf_h = shelf_h.max(packed_h);
            pen_x = pen_x.saturating_add(packed_w);
        }
    }

    true
}

fn auto_resize_atlas_inner(inner: &mut SharedAtlasInner) {
    const MIN_ATLAS_SIDE: u32 = 256;
    const MAX_ATLAS_SIDE: u32 = 16384;

    let mut max_dim = 1u32;
    let mut total_area: u64 = 0;
    let mut frame_count: u64 = 0;

    for id in &inner.tiles_order {
        let Some(tile) = inner.tiles_map.get(id) else {
            continue;
        };
        if tile.w == 0 || tile.h == 0 || tile.frames.is_empty() {
            continue;
        }

        let packed_w = tile.w.saturating_add(ATLAS_FRAME_PADDING * 2);
        let packed_h = tile.h.saturating_add(ATLAS_FRAME_PADDING * 2);
        max_dim = max_dim.max(packed_w.max(packed_h));
        total_area = total_area
            .saturating_add((packed_w as u64) * (packed_h as u64) * (tile.frames.len() as u64));
        frame_count = frame_count.saturating_add(tile.frames.len() as u64);
    }

    let mut side = max_dim.max(MIN_ATLAS_SIDE).next_power_of_two();
    if total_area > 0 {
        let estimated = (total_area as f64).sqrt().ceil() as u32;
        side = side.max(estimated.max(MIN_ATLAS_SIDE).next_power_of_two());
    }
    if frame_count == 0 {
        side = MIN_ATLAS_SIDE;
    }

    side = side.min(MAX_ATLAS_SIDE);
    while side < MAX_ATLAS_SIDE && !pack_fits(inner, side, side) {
        side = (side.saturating_mul(2)).min(MAX_ATLAS_SIDE);
    }

    if side != inner.atlas.width || side != inner.atlas.height {
        inner.atlas = Texture::new(side, side);
        inner.atlas_material = Texture::new(side, side);
        inner.atlas_dirty = true;
        inner.layout_dirty = true;
        inner.atlas_map.clear();
        inner.tiles_index_map.clear();
        inner.layout_version = inner.layout_version.wrapping_add(1);
    }
}

fn repaint_atlas_pixels_inner(inner: &mut SharedAtlasInner) {
    inner.atlas.data.fill(0);
    fill_default_material_pixels(&mut inner.atlas_material.data);

    for id in &inner.tiles_order {
        let Some(tile) = inner.tiles_map.get(id) else {
            continue;
        };
        let Some(rects) = inner.atlas_map.get(id) else {
            continue;
        };
        if tile.w == 0 || tile.h == 0 {
            continue;
        }
        let need_bytes = (tile.w as usize) * (tile.h as usize) * 4;
        for (f, rect) in rects.iter().enumerate() {
            if f >= tile.frames.len() {
                break;
            }
            let frame_owned = alpha_bleed_colors(&tile.frames[f], rect.w, rect.h, 4);
            let mat_owned = if f < tile.material_frames.len() {
                tile.material_frames[f].clone()
            } else {
                default_material_frame(need_bytes)
            };
            blit_rgba_into_with_border(
                &mut inner.atlas.data,
                inner.atlas.width,
                &frame_owned,
                rect.w,
                rect.h,
                rect.x.saturating_sub(ATLAS_FRAME_PADDING),
                rect.y.saturating_sub(ATLAS_FRAME_PADDING),
                ATLAS_FRAME_PADDING,
            );
            blit_rgba_into_with_border(
                &mut inner.atlas_material.data,
                inner.atlas_material.width,
                &mat_owned,
                rect.w,
                rect.h,
                rect.x.saturating_sub(ATLAS_FRAME_PADDING),
                rect.y.saturating_sub(ATLAS_FRAME_PADDING),
                ATLAS_FRAME_PADDING,
            );
        }
    }
}

pub fn default_material_frame(bytes: usize) -> Vec<u8> {
    if bytes == 0 {
        return Vec::new();
    }
    let mut v = Vec::with_capacity(bytes);
    let pixels = bytes / 4;
    for _ in 0..pixels {
        v.extend_from_slice(&DEFAULT_MATERIAL_PIXEL);
    }
    if v.len() < bytes {
        v.extend_from_slice(&DEFAULT_MATERIAL_PIXEL[..bytes - v.len()]);
    }
    v
}

pub fn normalize_material_frame(mut frame: Vec<u8>, bytes: usize) -> Vec<u8> {
    if frame.len() > bytes {
        frame.truncate(bytes);
    }
    while frame.len() < bytes {
        let byte_offset = frame.len() % DEFAULT_MATERIAL_PIXEL.len();
        frame.push(DEFAULT_MATERIAL_PIXEL[byte_offset]);
    }
    frame
}

fn fill_default_material_pixels(data: &mut [u8]) {
    for px in data.chunks_exact_mut(4) {
        px.copy_from_slice(&DEFAULT_MATERIAL_PIXEL);
    }
    let rem = data.len() % 4;
    if rem != 0 {
        let start = data.len() - rem;
        data[start..].copy_from_slice(&DEFAULT_MATERIAL_PIXEL[..rem]);
    }
}

fn material_opacity_emissive(px: &[u8]) -> (f32, f32) {
    if px.len() < 2 {
        return (1.0, 0.0);
    }
    if px[0] == SEMANTIC_MATERIAL_MARKER {
        let family = px[1] / 4;
        let opacity = match family {
            5 => 0.35,
            6 => 0.55,
            _ => 1.0,
        };
        let emissive = if family == 8 { 1.0 } else { 0.0 };
        return (opacity, emissive);
    }
    let packed_oe = px[1];
    (
        (packed_oe & 0x0F) as f32 / 15.0,
        ((packed_oe >> 4) & 0x0F) as f32 / 15.0,
    )
}

fn summarize_tile_emission(tile: &Tile) -> TileEmissiveSummary {
    if tile.w == 0 || tile.h == 0 || tile.frames.is_empty() {
        return TileEmissiveSummary::default();
    }

    let pixels_per_frame = (tile.w as usize).saturating_mul(tile.h as usize);
    if pixels_per_frame == 0 {
        return TileEmissiveSummary::default();
    }

    let mut weighted_color = [0.0f32; 3];
    let mut emissive_sum = 0.0f32;
    let mut covered_sum = 0.0f32;
    let mut total_texels = 0usize;
    let mut hotspot_uv = [0.5f32, 0.5f32];
    let mut hotspot_strength = 0.0f32;

    for (frame_index, frame) in tile.frames.iter().enumerate() {
        let material_frame = tile
            .material_frames
            .get(frame_index)
            .or_else(|| tile.material_frames.first());
        let Some(material_frame) = material_frame else {
            continue;
        };
        let frame_texels = pixels_per_frame
            .min(frame.len() / 4)
            .min(material_frame.len() / 4);
        total_texels += frame_texels;

        for texel in 0..frame_texels {
            let offset = texel * 4;
            let color_alpha = frame[offset + 3] as f32 / 255.0;
            let (opacity, emissive) =
                material_opacity_emissive(&material_frame[offset..offset + 4]);
            let strength = emissive * opacity * color_alpha;
            if strength <= 0.0 {
                continue;
            }

            let r = srgb_u8_to_linear(frame[offset]);
            let g = srgb_u8_to_linear(frame[offset + 1]);
            let b = srgb_u8_to_linear(frame[offset + 2]);
            weighted_color[0] += r * strength;
            weighted_color[1] += g * strength;
            weighted_color[2] += b * strength;
            emissive_sum += strength;
            covered_sum += (opacity * color_alpha).min(1.0);

            if strength > hotspot_strength {
                let x = (texel % tile.w as usize) as f32;
                let y = (texel / tile.w as usize) as f32;
                hotspot_uv = [
                    (x + 0.5) / tile.w.max(1) as f32,
                    (y + 0.5) / tile.h.max(1) as f32,
                ];
                hotspot_strength = strength;
            }
        }
    }

    if emissive_sum <= 0.0 || total_texels == 0 {
        return TileEmissiveSummary::default();
    }

    let inv = 1.0 / emissive_sum;
    TileEmissiveSummary {
        color_linear: [
            weighted_color[0] * inv,
            weighted_color[1] * inv,
            weighted_color[2] * inv,
        ],
        strength: emissive_sum / total_texels as f32,
        coverage: covered_sum / total_texels as f32,
        hotspot_uv,
        hotspot_strength,
    }
}

#[inline]
fn srgb_u8_to_linear(value: u8) -> f32 {
    (value as f32 / 255.0).powf(2.2)
}

fn blit_rgba_into(
    dst: &mut [u8],
    atlas_w: u32,
    src: &[u8],
    src_w: u32,
    src_h: u32,
    dst_x: u32,
    dst_y: u32,
) {
    if src.is_empty() {
        return;
    }
    let atlas_w = atlas_w as usize;
    let src_w = src_w as usize;
    let src_h = src_h as usize;
    let dx = dst_x as usize;
    let dy = dst_y as usize;
    for row in 0..src_h {
        let src_off = row * src_w * 4;
        let dst_off = ((dy + row) * atlas_w + dx) * 4;
        let src_slice = &src[src_off..src_off + src_w * 4];
        let dst_slice = &mut dst[dst_off..dst_off + src_w * 4];
        dst_slice.copy_from_slice(src_slice);
    }
}

fn alpha_bleed_colors(src: &[u8], w: u32, h: u32, iterations: u32) -> Vec<u8> {
    if src.is_empty() || w == 0 || h == 0 {
        return src.to_vec();
    }
    let mut out = src.to_vec();
    let mut prev = out.clone();
    let w_us = w as usize;
    let h_us = h as usize;
    let iters = iterations.max(1);

    for _ in 0..iters {
        prev.copy_from_slice(&out);
        for y in 0..h_us {
            for x in 0..w_us {
                let i = (y * w_us + x) * 4;
                if prev[i + 3] != 0 {
                    continue;
                }

                let mut rs: u32 = 0;
                let mut gs: u32 = 0;
                let mut bs: u32 = 0;
                let mut n: u32 = 0;

                let y0 = y.saturating_sub(1);
                let y1 = (y + 1).min(h_us - 1);
                let x0 = x.saturating_sub(1);
                let x1 = (x + 1).min(w_us - 1);
                for ny in y0..=y1 {
                    for nx in x0..=x1 {
                        if nx == x && ny == y {
                            continue;
                        }
                        let ni = (ny * w_us + nx) * 4;
                        if prev[ni + 3] > 0 {
                            rs += prev[ni] as u32;
                            gs += prev[ni + 1] as u32;
                            bs += prev[ni + 2] as u32;
                            n += 1;
                        }
                    }
                }

                if n > 0 {
                    out[i] = (rs / n) as u8;
                    out[i + 1] = (gs / n) as u8;
                    out[i + 2] = (bs / n) as u8;
                    // Keep alpha at 0; we only bleed color into transparent pixels.
                }
            }
        }
    }

    out
}

fn blit_rgba_into_with_border(
    dst: &mut [u8],
    atlas_w: u32,
    src: &[u8],
    src_w: u32,
    src_h: u32,
    dst_x: u32,
    dst_y: u32,
    border: u32,
) {
    if src.is_empty() || src_w == 0 || src_h == 0 {
        return;
    }

    blit_rgba_into(
        dst,
        atlas_w,
        src,
        src_w,
        src_h,
        dst_x + border,
        dst_y + border,
    );

    if border == 0 {
        return;
    }

    let atlas_w_us = atlas_w as usize;
    let src_w_us = src_w as usize;
    let src_h_us = src_h as usize;
    let border_us = border as usize;
    let base_x = dst_x as usize;
    let base_y = dst_y as usize;

    for row in 0..src_h_us {
        let src_row_off = row * src_w_us * 4;
        let first_px = &src[src_row_off..src_row_off + 4];
        let last_px = &src[src_row_off + (src_w_us - 1) * 4..src_row_off + src_w_us * 4];
        let y = base_y + border_us + row;

        for b in 0..border_us {
            let lx = base_x + b;
            let rx = base_x + border_us + src_w_us + b;
            let l_off = (y * atlas_w_us + lx) * 4;
            let r_off = (y * atlas_w_us + rx) * 4;
            dst[l_off..l_off + 4].copy_from_slice(first_px);
            dst[r_off..r_off + 4].copy_from_slice(last_px);
        }
    }

    let padded_w = src_w_us + border_us * 2;
    for b in 0..border_us {
        let src_top_y = base_y + border_us;
        let src_bot_y = base_y + border_us + src_h_us - 1;
        let top_y = base_y + b;
        let bot_y = base_y + border_us + src_h_us + b;
        for x in 0..padded_w {
            let sx = base_x + x;
            let top_src_off = (src_top_y * atlas_w_us + sx) * 4;
            let bot_src_off = (src_bot_y * atlas_w_us + sx) * 4;
            let top_dst_off = (top_y * atlas_w_us + sx) * 4;
            let bot_dst_off = (bot_y * atlas_w_us + sx) * 4;
            let top_px = [
                dst[top_src_off],
                dst[top_src_off + 1],
                dst[top_src_off + 2],
                dst[top_src_off + 3],
            ];
            let bot_px = [
                dst[bot_src_off],
                dst[bot_src_off + 1],
                dst[bot_src_off + 2],
                dst[bot_src_off + 3],
            ];
            dst[top_dst_off..top_dst_off + 4].copy_from_slice(&top_px);
            dst[bot_dst_off..bot_dst_off + 4].copy_from_slice(&bot_px);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_opaque_material_is_not_translucent() {
        let atlas = SharedAtlas::new(16, 16);
        atlas.add_tile(
            Uuid::nil(),
            1,
            1,
            vec![vec![255, 128, 64, 255]],
            vec![vec![SEMANTIC_MATERIAL_MARKER, 4, 128, 128]],
        );

        assert!(!atlas.tile_index_has_translucency(0));
    }

    #[test]
    fn semantic_translucent_material_is_translucent() {
        let atlas = SharedAtlas::new(16, 16);
        atlas.add_tile(
            Uuid::nil(),
            1,
            1,
            vec![vec![255, 128, 64, 255]],
            vec![vec![SEMANTIC_MATERIAL_MARKER, 20, 128, 128]],
        );

        assert!(atlas.tile_index_has_translucency(0));
    }

    #[test]
    fn normalizing_short_material_frame_preserves_default_byte_positions() {
        assert_eq!(
            normalize_material_frame(vec![SEMANTIC_MATERIAL_MARKER, 4], 8),
            vec![SEMANTIC_MATERIAL_MARKER, 4, 128, 128, 7, 15, 128, 128]
        );
    }

    #[test]
    fn default_material_frame_is_opaque() {
        let frame = default_material_frame(8);
        assert_eq!(frame, vec![7, 15, 128, 128, 7, 15, 128, 128]);
        assert_eq!(material_opacity_emissive(&frame[0..4]), (1.0, 0.0));
    }
}
