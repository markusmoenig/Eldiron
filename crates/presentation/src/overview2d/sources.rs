use super::*;
struct Entry {
    source: Arc<Source>,
    used: u64,
}
pub(super) struct Source {
    pub view: wgpu::TextureView,
    pub sizes: wgpu::Buffer,
    pub layers: u32,
    bytes: usize,
}
pub(super) struct Sources {
    context: Arc<GpuContext>,
    entries: HashMap<(Uuid, u64, Uuid, usize, usize), Entry>,
    clock: u64,
    bytes: usize,
}
impl Sources {
    pub fn new(context: Arc<GpuContext>) -> Self {
        Self {
            context,
            entries: HashMap::new(),
            clock: 0,
            bytes: 0,
        }
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
    pub fn dummy(&mut self) -> Result<Arc<Source>> {
        let tile = Tile::from_texture(rusterix::Texture::new(vec![0; 4], 1, 1));
        self.prepare(&tile, Uuid::nil(), 0, 0, 1)
    }
    pub fn prepare(
        &mut self,
        tile: &Tile,
        namespace: Uuid,
        revision: u64,
        base: usize,
        count: usize,
    ) -> Result<Arc<Source>> {
        self.clock = self.clock.wrapping_add(1);
        let key = (namespace, revision, tile.id, base, count);
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.used = self.clock;
            return Ok(entry.source.clone());
        }
        let frames = tile.textures.get(base..).unwrap_or(&[]);
        let frames = &frames[..frames.len().min(count)];
        let width = frames.iter().map(|t| t.width).max().unwrap_or(0);
        let height = frames.iter().map(|t| t.height).max().unwrap_or(0);
        let layers = u32::try_from(frames.len())
            .map_err(|_| RenderError("too many overview source layers".into()))?;
        let limits = self.context.device.limits();
        if width == 0
            || height == 0
            || width > limits.max_texture_dimension_2d as usize
            || height > limits.max_texture_dimension_2d as usize
            || layers > limits.max_texture_array_layers
        {
            return Err(RenderError("overview source exceeds texture limits".into()));
        }
        let bytes = width
            .checked_mul(height)
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| n.checked_mul(layers as usize))
            .and_then(|n| n.checked_add(layers as usize * 16))
            .ok_or_else(|| RenderError("overview source extent overflow".into()))?;
        let texture = self
            .context
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("overview source frames"),
                size: wgpu::Extent3d {
                    width: width as u32,
                    height: height as u32,
                    depth_or_array_layers: layers,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Uint,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
        let mut sizes = Vec::new();
        for (layer, source) in frames.iter().enumerate() {
            if source.width == 0
                || source.height == 0
                || source
                    .width
                    .checked_mul(source.height)
                    .and_then(|n| n.checked_mul(4))
                    != Some(source.data.len())
            {
                return Err(RenderError("invalid overview source pixels".into()));
            }
            self.context.record_upload(source.data.len() as u64);
            self.context.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &source.data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(source.width as u32 * 4),
                    rows_per_image: Some(source.height as u32),
                },
                wgpu::Extent3d {
                    width: source.width as u32,
                    height: source.height as u32,
                    depth_or_array_layers: 1,
                },
            );
            sizes.push([source.width as u32, source.height as u32, 0, 0]);
        }
        self.context.record_upload((sizes.len() * 16) as u64);
        let sizes = self
            .context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("overview source sizes"),
                contents: bytemuck::cast_slice(&sizes),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let source = Arc::new(Source {
            view: texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            }),
            sizes,
            layers,
            bytes,
        });
        if bytes <= 64 * 1024 * 1024 {
            self.bytes += bytes;
            self.entries.insert(
                key,
                Entry {
                    source: source.clone(),
                    used: self.clock,
                },
            );
            while self.bytes > 64 * 1024 * 1024 || self.entries.len() > 512 {
                let key = self
                    .entries
                    .iter()
                    .min_by_key(|(_, v)| v.used)
                    .map(|(k, _)| *k)
                    .unwrap();
                self.bytes -= self.entries.remove(&key).unwrap().source.bytes;
            }
        }
        Ok(source)
    }
}
