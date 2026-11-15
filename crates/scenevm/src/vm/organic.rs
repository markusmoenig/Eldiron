//! Retained CPU billboard descriptions and segmented compute metadata uploads.
use super::*;

/// The shader's unchanged contiguous layout, retained as independent CPU spans.
/// Source metadata/pixels survive placement updates; GPU owners track each span.
#[derive(Clone)]
pub(super) struct Words {
    instances: Arc<[u32]>,
    sources: Arc<[u32]>,
}

impl Words {
    fn len(&self) -> usize {
        self.instances.len() + self.sources.len()
    }

    fn append_to(&self, words: &mut Vec<u32>) {
        words.extend_from_slice(&self.instances);
        words.extend_from_slice(&self.sources);
    }

    fn write_changes(
        &self,
        queue: &wgpu::Queue,
        buffer: &wgpu::Buffer,
        offset: u64,
        previous: Option<&Self>,
    ) {
        if previous.is_none_or(|old| !Arc::ptr_eq(&old.instances, &self.instances)) {
            queue.write_buffer(buffer, offset, bytemuck::cast_slice(&self.instances));
        }
        // A different instance count moves the retained source span as well.
        if !self.sources.is_empty()
            && previous.is_none_or(|old| {
                old.instances.len() != self.instances.len()
                    || !Arc::ptr_eq(&old.sources, &self.sources)
            })
        {
            queue.write_buffer(
                buffer,
                offset + self.instances.len() as u64 * 4,
                bytemuck::cast_slice(&self.sources),
            );
        }
    }
}

impl VM {
    pub(super) fn invalidate_organic_source(&mut self) {
        self.cached_organic_words = None;
    }
    fn organic_words(&mut self) -> Words {
        if let Some(old) = &self.cached_organic_words {
            if !self.organic_instances_dirty {
                return old.clone();
            }
            let sources = old.sources.clone();
            self.cached_organic_words = Some(Words {
                instances: self.build_organic_instance_words().into(),
                sources,
            });
        } else {
            self.cached_organic_words = Some(Words {
                instances: self.build_organic_instance_words().into(),
                sources: self.build_organic_sprite_words().into(),
            });
        }
        self.organic_instances_dirty = false;
        self.cached_organic_words.as_ref().unwrap().clone()
    }
    pub fn compute_organic_source_bytes(&self) -> u64 {
        let mut sprites = self.organic_billboards.sprites.len() as u64
            + self.paint_billboards.sprites.len() as u64
            + self.paint_billboards.paged_sprites.len() as u64;
        let mut instances = self.organic_billboards.instances.len() as u64
            + self.paint_billboards.instances.len() as u64;
        let mut pixels = self
            .organic_billboards
            .sprites
            .iter()
            .chain(&self.paint_billboards.sprites)
            .map(|sprite| sprite.rgba.len() as u64)
            .sum::<u64>();
        pixels = pixels.saturating_add(
            self.paint_billboards
                .paged_sprites
                .iter()
                .map(|sprite| sprite.alpha_mask.len() as u64 * 4)
                .sum(),
        );
        for chunk in self.chunks_map.values() {
            sprites = sprites.saturating_add(chunk.organic_billboard_sprites.len() as u64);
            instances = instances.saturating_add(chunk.organic_billboard_instances.len() as u64);
            pixels = pixels.saturating_add(
                chunk
                    .organic_billboard_sprites
                    .iter()
                    .map(|sprite| sprite.rgba.len() as u64)
                    .sum(),
            );
        }
        32u64
            .saturating_add(sprites.saturating_mul(16))
            .saturating_add(instances.saturating_mul(32))
            .saturating_add(pixels)
    }

    fn build_organic_instance_words(&self) -> Vec<u32> {
        let chunk_sprite_count: usize = self
            .chunks_map
            .values()
            .map(|chunk| chunk.organic_billboard_sprites.len())
            .sum();
        let chunk_instance_count: usize = self
            .chunks_map
            .values()
            .map(|chunk| chunk.organic_billboard_instances.len())
            .sum();
        let sprite_count = (self.organic_billboards.sprites.len()
            + chunk_sprite_count
            + self.paint_billboards.sprites.len()
            + self.paint_billboards.paged_sprites.len())
        .min(u32::MAX as usize) as u32;
        let instance_count = (self.organic_billboards.instances.len()
            + chunk_instance_count
            + self.paint_billboards.instances.len())
        .min(u32::MAX as usize) as u32;
        let instance_offset_words = 8u32;
        let sprite_meta_offset_words = instance_offset_words + instance_count * 8;
        let pixel_offset_words = sprite_meta_offset_words + sprite_count * 4;
        let mut words = vec![
            0x4F42_494Cu32,
            sprite_count,
            instance_count,
            instance_offset_words,
            sprite_meta_offset_words,
            pixel_offset_words,
            0,
            0,
        ];

        let mut sprite_base = self.organic_billboards.sprites.len() as u32;
        for instance in self.organic_billboards.instances.iter() {
            words.extend_from_slice(&[
                instance.center[0].to_bits(),
                instance.center[1].to_bits(),
                instance.center[2].to_bits(),
                instance.width.to_bits(),
                instance.height.to_bits(),
                instance.sprite_index,
                instance.flags,
                0,
            ]);
        }
        for chunk in self.chunks_map.values() {
            for instance in &chunk.organic_billboard_instances {
                words.extend_from_slice(&[
                    instance.center[0].to_bits(),
                    instance.center[1].to_bits(),
                    instance.center[2].to_bits(),
                    instance.width.to_bits(),
                    instance.height.to_bits(),
                    sprite_base.saturating_add(instance.sprite_index),
                    instance.flags,
                    0,
                ]);
            }
            sprite_base = sprite_base.saturating_add(chunk.organic_billboard_sprites.len() as u32);
        }
        for instance in &self.paint_billboards.instances {
            words.extend_from_slice(&[
                instance.center[0].to_bits(),
                instance.center[1].to_bits(),
                instance.center[2].to_bits(),
                instance.width.to_bits(),
                instance.height.to_bits(),
                sprite_base.saturating_add(instance.sprite_index),
                instance.flags,
                0,
            ]);
        }

        words
    }

    fn build_organic_sprite_words(&self) -> Vec<u32> {
        let mut words = Vec::new();
        let mut pixel_words = Vec::new();
        for sprite in &self.organic_billboards.sprites {
            let offset_pixels = pixel_words.len() as u32;
            words.extend_from_slice(&[offset_pixels, sprite.width, sprite.height, 0]);
            for px in sprite.rgba.chunks_exact(4) {
                pixel_words.push(u32::from_le_bytes([px[0], px[1], px[2], px[3]]));
            }
        }
        for chunk in self.chunks_map.values() {
            for sprite in &chunk.organic_billboard_sprites {
                let offset_pixels = pixel_words.len() as u32;
                words.extend_from_slice(&[offset_pixels, sprite.width, sprite.height, 0]);
                for px in sprite.rgba.chunks_exact(4) {
                    pixel_words.push(u32::from_le_bytes([px[0], px[1], px[2], px[3]]));
                }
            }
        }
        for sprite in &self.paint_billboards.sprites {
            let offset_pixels = pixel_words.len() as u32;
            words.extend_from_slice(&[offset_pixels, sprite.width, sprite.height, 0]);
            for px in sprite.rgba.chunks_exact(4) {
                pixel_words.push(u32::from_le_bytes([px[0], px[1], px[2], px[3]]));
            }
        }
        for sprite in &self.paint_billboards.paged_sprites {
            let mask_offset = pixel_words.len() as u32;
            words.extend_from_slice(&[
                sprite.source_id,
                sprite.width,
                sprite.height,
                mask_offset | 0x8000_0000,
            ]);
            pixel_words.extend_from_slice(&sprite.alpha_mask);
        }
        words.extend_from_slice(&pixel_words);

        words
    }

    pub(super) fn upload_organic_billboard_ssbo(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        use wgpu::util::DeviceExt;

        let organic_words = self.organic_words();
        let organic_byte_len = organic_words.len() * 4;
        let g = self.gpu.as_mut().unwrap();
        if g.organic_billboard_ssbo.is_none() || g.organic_billboard_ssbo_size != organic_byte_len {
            let mut words = Vec::with_capacity(organic_words.len());
            organic_words.append_to(&mut words);
            g.organic_billboard_ssbo = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("vm-organic-billboards"),
                    contents: bytemuck::cast_slice(&words),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                },
            ));
        } else {
            organic_words.write_changes(
                queue,
                g.organic_billboard_ssbo.as_ref().unwrap(),
                0,
                g.organic_billboard_words.as_ref(),
            );
        }
        g.organic_billboard_ssbo_size = organic_byte_len;
        g.organic_billboard_count = organic_words.instances[2];
        g.organic_billboard_words = Some(organic_words);
    }
    pub(super) fn upload_compute3d_aux_ssbo(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> crate::SceneVMResult<()> {
        use wgpu::util::DeviceExt;
        let organic = self.organic_words();
        let dummy = Raster3DSurfacePaintEntry::zeroed();
        let (surface, surface_changed) = {
            let current = self
                .paint_entries()
                .filter(|entries| !entries.is_empty())
                .unwrap_or(std::slice::from_ref(&dummy));
            match self
                .gpu
                .as_ref()
                .and_then(|gpu| gpu.compute3d_aux_surface.as_ref())
            {
                Some(old) if old.as_ref() == current => (old.clone(), false),
                _ => (Arc::<[Raster3DSurfacePaintEntry]>::from(current), true),
            }
        };
        let surface_words: &[u32] = bytemuck::cast_slice(surface.as_ref());
        let organic_offset = 4usize.checked_add(surface_words.len()).ok_or_else(|| {
            crate::SceneVMError::BufferAllocationFailed("compute metadata offset overflow".into())
        })?;
        let word_len = organic_offset
            .checked_add(organic.len())
            .and_then(|len| len.checked_add(1))
            .ok_or_else(|| {
                crate::SceneVMError::BufferAllocationFailed("compute metadata size overflow".into())
            })?;
        let byte_len = word_len.checked_mul(4).ok_or_else(|| {
            crate::SceneVMError::BufferAllocationFailed("compute metadata size overflow".into())
        })?;
        if byte_len as u64 > u64::from(device.limits().max_storage_buffer_binding_size)
            || byte_len as u64 > device.limits().max_buffer_size
        {
            return Err(crate::SceneVMError::BufferAllocationFailed(
                "compute paint/organic metadata exceeds device binding limit".into(),
            ));
        }
        let header = [
            u32::MAX,
            4,
            surface.len() as u32,
            organic_offset as u32,
            organic.len() as u32,
        ];
        let gpu = self.gpu.as_mut().unwrap();
        let recreate = gpu.compute3d_aux_ssbo.is_none() || gpu.compute3d_aux_ssbo_size != byte_len;
        let organic_moved = gpu
            .compute3d_aux_surface
            .as_ref()
            .is_none_or(|old| old.len() != surface.len());
        if recreate {
            let mut words = Vec::with_capacity(word_len);
            words.extend_from_slice(&header);
            words.extend_from_slice(surface_words);
            organic.append_to(&mut words);
            gpu.compute3d_aux_ssbo = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("vm-compute3d-aux"),
                    contents: bytemuck::cast_slice(&words),
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST
                        | wgpu::BufferUsages::COPY_SRC,
                },
            ));
            gpu.compute3d_aux_ssbo_size = byte_len;
        } else {
            let buffer = gpu.compute3d_aux_ssbo.as_ref().unwrap();
            // Reset the dependency prefix for each ordered dispatch. Ordinary
            // scene frames leave both retained payload segments untouched.
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&header));
            if surface_changed {
                queue.write_buffer(buffer, 20, bytemuck::cast_slice(surface_words));
            }
            organic.write_changes(
                queue,
                buffer,
                (1 + organic_offset) as u64 * 4,
                if organic_moved {
                    None
                } else {
                    gpu.compute3d_aux_organic.as_ref()
                },
            );
        }
        gpu.compute3d_aux_surface = Some(surface);
        gpu.compute3d_aux_organic = Some(organic);
        Ok(())
    }
}
