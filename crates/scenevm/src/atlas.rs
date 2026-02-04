use crate::{Texture, texture::TextureGPU};
use rustc_hash::FxHashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

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
    }

    pub fn layout_version(&self) -> u64 {
        let guard = self.inner.lock().unwrap();
        guard.layout_version
    }

    pub fn tile_index(&self, id: &Uuid) -> Option<u32> {
        let guard = self.inner.lock().unwrap();
        guard.tiles_index_map.get(id).copied()
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
    }

    pub fn clear(&self) {
        let mut guard = self.inner.lock().unwrap();
        guard.tiles_map.clear();
        guard.tiles_order.clear();
        guard.atlas.data.fill(0);
        guard.atlas_material.data.fill(0);
        guard.atlas_map.clear();
        guard.atlas_dirty = true;
        guard.layout_dirty = true;
    }

    pub fn with_tile_mut<R>(&self, id: &Uuid, f: impl FnOnce(&mut Tile) -> R) -> Option<R> {
        let mut guard = self.inner.lock().unwrap();
        let tile = guard.tiles_map.get_mut(id)?;
        let out = f(tile);
        guard.atlas_dirty = true;
        guard.layout_dirty = true;
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

    pub fn upload_to_gpu_with(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut guard = self.inner.lock().unwrap();
        if guard.atlas_dirty {
            guard.atlas.upload_to_gpu_with(device, queue);
            guard.atlas_material.upload_to_gpu_with(device, queue);
            guard.atlas_dirty = false;
        }
    }

    pub fn download_from_gpu_with(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut guard = self.inner.lock().unwrap();
        guard.atlas.download_from_gpu_with(device, queue);
        guard.atlas_material.download_from_gpu_with(device, queue);
    }

    pub fn with_views<R>(&self, f: impl FnOnce(&TextureGPU, &TextureGPU) -> R) -> Option<R> {
        let guard = self.inner.lock().unwrap();
        let a = guard.atlas.gpu.as_ref()?;
        let mat = guard.atlas_material.gpu.as_ref()?;
        Some(f(a, mat))
    }

    pub fn texture_views(&self) -> Option<(wgpu::TextureView, wgpu::TextureView)> {
        self.with_views(|a, m| (a.view.clone(), m.view.clone()))
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
        if !guard.layout_dirty {
            return false;
        }
        build_atlas_inner(&mut guard);
        guard.layout_dirty = false;
        true
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
    }

    pub fn inner_arc(&self) -> Arc<Mutex<SharedAtlasInner>> {
        Arc::clone(&self.inner)
    }
}

fn build_atlas_inner(inner: &mut SharedAtlasInner) {
    inner.atlas.data.fill(0);
    inner.atlas_material.data.fill(0);
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
        let frames_len = tile.frames.len();
        let mat_len = tile.material_frames.len();
        let mut rects = Vec::with_capacity(frames_len);
        let need_bytes = (w as usize) * (h as usize) * 4;

        for f in 0..frames_len {
            if pen_x + w > inner.atlas.width {
                pen_x = 0;
                pen_y = pen_y.saturating_add(shelf_h);
                shelf_h = 0;
            }
            if pen_y + h > inner.atlas.height {
                break;
            }
            shelf_h = shelf_h.max(h);

            let frame_owned = tile.frames[f].clone();
            let mat_owned = if f < mat_len {
                tile.material_frames[f].clone()
            } else {
                default_material_frame(need_bytes)
            };
            blit_rgba_into(
                &mut inner.atlas.data,
                inner.atlas.width,
                &frame_owned,
                w,
                h,
                pen_x,
                pen_y,
            );
            blit_rgba_into(
                &mut inner.atlas_material.data,
                inner.atlas_material.width,
                &mat_owned,
                w,
                h,
                pen_x,
                pen_y,
            );

            rects.push(AtlasEntry {
                x: pen_x,
                y: pen_y,
                w,
                h,
            });
            pen_x = pen_x.saturating_add(w);
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

pub fn default_material_frame(bytes: usize) -> Vec<u8> {
    if bytes == 0 {
        return Vec::new();
    }
    let mut v = Vec::with_capacity(bytes);
    let pixels = bytes / 4;
    // Default: roughness=0.5 (7/15), metallic=0.0 (0/15), opacity=1.0 (15/15), emissive=0.0 (0/15)
    // Packed as: byte0 = r|(m<<4) = 7|0 = 7, byte1 = o|(e<<4) = 15|0 = 15
    // Normal defaults to 0.0: 128 = (0.0*0.5+0.5)*255
    for _ in 0..pixels {
        v.extend_from_slice(&[7u8, 15u8, 128u8, 128u8]);
    }
    if v.len() < bytes {
        v.resize(bytes, 0);
    }
    v
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
