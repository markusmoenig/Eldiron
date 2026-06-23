use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vert3DPod {
    pub pos: [f32; 3],
    pub organic_enabled: f32,
    pub uv: [f32; 2],
    pub organic_atlas_min: [f32; 2],
    pub tile_index: u32,
    pub tile_index2: u32,
    pub blend_factor: f32,
    pub opacity: f32,
    pub normal: [f32; 3],
    pub organic_uv: [f32; 2],
    pub organic_local_min: [f32; 2],
    pub organic_local_size: [f32; 2],
    pub organic_atlas_size: [f32; 2],
    pub surface_noise: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Line3DPod {
    pub pos: [f32; 3],
    pub _pad0: f32,
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Raster3DUniforms {
    pub cam_pos: [f32; 4],
    pub cam_fwd: [f32; 4],
    pub cam_right: [f32; 4],
    pub cam_up: [f32; 4],
    pub sun_color_intensity: [f32; 4],
    pub sun_dir_enabled: [f32; 4],
    pub ambient_color_strength: [f32; 4],
    pub sky_color: [f32; 4],
    pub fog_color_density: [f32; 4],
    pub shadow_light_right: [f32; 4],
    pub shadow_light_up: [f32; 4],
    pub shadow_light_fwd: [f32; 4],
    pub shadow_light_center: [f32; 4],
    pub shadow_light_extents: [f32; 4],
    pub shadow_params: [f32; 4],
    pub render_params: [f32; 4],
    pub point_light_pos_intensity: [[f32; 4]; 8],
    pub point_light_color_range: [[f32; 4]; 8],
    pub point_light_count: u32,
    pub _pad_light_count: [u32; 3],
    pub _pad_lights: [u32; 4],
    pub fb_size: [f32; 2],
    pub cam_vfov_deg: f32,
    pub cam_ortho_half_h: f32,
    pub cam_near: f32,
    pub cam_far: f32,
    pub cam_kind: u32,
    pub anim_counter: u32,
    pub _pad: [u32; 2],
    pub _pad_post_pre: [u32; 2],
    pub post_params: [f32; 4],
    pub post_color_adjust: [f32; 4],
    pub post_style0: [f32; 4],
    pub post_style1: [f32; 4],
    pub avatar_highlight_params: [f32; 4],
    pub _pad_tail: [u32; 4],
    pub palette: [[f32; 4]; 256],
    pub palette_tile_indices: [[u32; 4]; 64],
    pub organic_params: [u32; 4],
}

// WGSL uniform buffers use strict alignment. Keep this layout exact because
// some backends are less forgiving about scalar/vector packing mismatches.
const RASTER3D_UNIFORM_WGSL_BYTES: usize = 5824;
const _: [(); 0] = [(); std::mem::size_of::<Raster3DUniforms>() % 16];
const _: [(); RASTER3D_UNIFORM_WGSL_BYTES] = [(); std::mem::size_of::<Raster3DUniforms>()];
const _: [(); 512] = [(); std::mem::offset_of!(Raster3DUniforms, point_light_count)];
const _: [(); 528] = [(); std::mem::offset_of!(Raster3DUniforms, _pad_lights)];
const _: [(); 544] = [(); std::mem::offset_of!(Raster3DUniforms, fb_size)];
const _: [(); 592] = [(); std::mem::offset_of!(Raster3DUniforms, post_params)];
const _: [(); 624] = [(); std::mem::offset_of!(Raster3DUniforms, post_style0)];
const _: [(); 656] = [(); std::mem::offset_of!(Raster3DUniforms, avatar_highlight_params)];
const _: [(); 688] = [(); std::mem::offset_of!(Raster3DUniforms, palette)];
const _: [(); 4784] = [(); std::mem::offset_of!(Raster3DUniforms, palette_tile_indices)];
const _: [(); 5808] = [(); std::mem::offset_of!(Raster3DUniforms, organic_params)];

impl super::VMGpu {
    pub(super) fn ensure_raster3d_targets(
        &mut self,
        device: &wgpu::Device,
        fb_w: u32,
        fb_h: u32,
        shadow_res: u32,
        raster_samples: u32,
    ) {
        if self.raster3d_shadow_res != shadow_res || self.raster3d_shadow_tex.is_none() {
            let shadow_tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("vm-raster3d-shadow-depth"),
                size: wgpu::Extent3d {
                    width: shadow_res,
                    height: shadow_res,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let shadow_view = shadow_tex.create_view(&wgpu::TextureViewDescriptor::default());
            self.raster3d_shadow_tex = Some(shadow_tex);
            self.raster3d_shadow_view = Some(shadow_view);
            self.raster3d_shadow_res = shadow_res;
        }

        if self.raster3d_fb_size != (fb_w, fb_h)
            || self.raster3d_sample_count != raster_samples
            || self.raster3d_msaa_color_tex.is_none()
            || self.raster3d_depth_tex.is_none()
        {
            let msaa_color_tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("vm-raster3d-msaa-color"),
                size: wgpu::Extent3d {
                    width: fb_w,
                    height: fb_h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: raster_samples,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let msaa_color_view =
                msaa_color_tex.create_view(&wgpu::TextureViewDescriptor::default());

            let depth_tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("vm-raster3d-depth"),
                size: wgpu::Extent3d {
                    width: fb_w,
                    height: fb_h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: raster_samples,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

            self.raster3d_msaa_color_tex = Some(msaa_color_tex);
            self.raster3d_msaa_color_view = Some(msaa_color_view);
            self.raster3d_depth_tex = Some(depth_tex);
            self.raster3d_depth_view = Some(depth_view);
            self.raster3d_fb_size = (fb_w, fb_h);
            self.raster3d_sample_count = raster_samples;
        }
    }

    pub(super) fn update_or_create_index_buffer(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        slot: &mut Option<wgpu::Buffer>,
        capacity: &mut u64,
        label: &'static str,
        data: &[u32],
    ) {
        let size = std::mem::size_of_val(data) as u64;
        if slot.is_none() || *capacity < size {
            *slot = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: bytemuck::cast_slice(data),
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                }),
            );
            *capacity = size;
        } else if let Some(buffer) = slot.as_ref() {
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(data));
        }
    }
}

#[derive(Default)]
#[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
struct Raster3DDebugTiming {
    frames: u32,
    init_ms: f64,
    prepare_ms: f64,
    geometry_ms: f64,
    visibility_ms: f64,
    upload_ms: f64,
    encode_ms: f64,
    submit_ms: f64,
    total_ms: f64,
    verts: u64,
    indices: u64,
    visible_indices: u64,
    opaque_indices: u64,
    transparent_indices: u64,
    particle_indices: u64,
    geometry_rebuilds: u32,
    shadow_frames: u32,
    msaa_frames: u32,
    shadow_res_sum: u64,
    last_log: Option<instant::Instant>,
}

#[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
static RASTER3D_DEBUG_TIMING: std::sync::OnceLock<std::sync::Mutex<Raster3DDebugTiming>> =
    std::sync::OnceLock::new();

#[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
#[allow(clippy::too_many_arguments)]
pub(super) fn record_raster3d_debug_timing(
    size: (u32, u32),
    init_ms: f64,
    prepare_ms: f64,
    geometry_ms: f64,
    visibility_ms: f64,
    upload_ms: f64,
    encode_ms: f64,
    submit_ms: f64,
    total_ms: f64,
    verts: usize,
    indices: usize,
    visible_indices: usize,
    opaque_indices: usize,
    transparent_indices: usize,
    particle_indices: usize,
    geometry_rebuilt: bool,
    shadow_enabled: bool,
    shadow_res: u32,
    msaa_samples: u32,
    post_enabled: bool,
    bump_strength: f32,
    shadow_distance: f32,
    shadow_strength: f32,
) {
    if !crate::render_debug_enabled() {
        return;
    }

    let mut timing = RASTER3D_DEBUG_TIMING
        .get_or_init(|| std::sync::Mutex::new(Raster3DDebugTiming::default()))
        .lock()
        .unwrap();

    timing.frames = timing.frames.saturating_add(1);
    timing.init_ms += init_ms;
    timing.prepare_ms += prepare_ms;
    timing.geometry_ms += geometry_ms;
    timing.visibility_ms += visibility_ms;
    timing.upload_ms += upload_ms;
    timing.encode_ms += encode_ms;
    timing.submit_ms += submit_ms;
    timing.total_ms += total_ms;
    timing.verts = timing.verts.saturating_add(verts as u64);
    timing.indices = timing.indices.saturating_add(indices as u64);
    timing.visible_indices = timing
        .visible_indices
        .saturating_add(visible_indices as u64);
    timing.opaque_indices = timing.opaque_indices.saturating_add(opaque_indices as u64);
    timing.transparent_indices = timing
        .transparent_indices
        .saturating_add(transparent_indices as u64);
    timing.particle_indices = timing
        .particle_indices
        .saturating_add(particle_indices as u64);
    if geometry_rebuilt {
        timing.geometry_rebuilds = timing.geometry_rebuilds.saturating_add(1);
    }
    if shadow_enabled {
        timing.shadow_frames = timing.shadow_frames.saturating_add(1);
    }
    if msaa_samples > 1 {
        timing.msaa_frames = timing.msaa_frames.saturating_add(1);
    }
    timing.shadow_res_sum = timing.shadow_res_sum.saturating_add(shadow_res as u64);

    let now = instant::Instant::now();
    let should_log = timing
        .last_log
        .map(|last| now.duration_since(last) >= std::time::Duration::from_secs(2))
        .unwrap_or(true);
    if should_log {
        let n = timing.frames.max(1) as f64;
        crate::render_debug_log(&format!(
            "[RenderDebug][Raster3D] size={}x{} frames={} avg_ms init={:.2} prepare={:.2} geometry={:.2} visibility={:.2} upload={:.2} encode={:.2} submit={:.2} total={:.2} avg_counts verts={:.0} indices={:.0} visible={:.0} opaque={:.0} transparent={:.0} particles={:.0} geometry_rebuilds={} shadow_frames={} avg_shadow_res={:.0} msaa_frames={} last_settings shadow={} shadow_distance={:.2} shadow_strength={:.2} bump={:.2} msaa={} post={}",
            size.0,
            size.1,
            timing.frames,
            timing.init_ms / n,
            timing.prepare_ms / n,
            timing.geometry_ms / n,
            timing.visibility_ms / n,
            timing.upload_ms / n,
            timing.encode_ms / n,
            timing.submit_ms / n,
            timing.total_ms / n,
            timing.verts as f64 / n,
            timing.indices as f64 / n,
            timing.visible_indices as f64 / n,
            timing.opaque_indices as f64 / n,
            timing.transparent_indices as f64 / n,
            timing.particle_indices as f64 / n,
            timing.geometry_rebuilds,
            timing.shadow_frames,
            timing.shadow_res_sum as f64 / n,
            timing.msaa_frames,
            shadow_enabled,
            shadow_distance,
            shadow_strength,
            bump_strength,
            msaa_samples,
            post_enabled
        ));
        *timing = Raster3DDebugTiming {
            last_log: Some(now),
            ..Raster3DDebugTiming::default()
        };
    }
}

#[cfg(all(feature = "gpu", target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
pub(super) fn record_raster3d_debug_timing(
    _size: (u32, u32),
    _init_ms: f64,
    _prepare_ms: f64,
    _geometry_ms: f64,
    _visibility_ms: f64,
    _upload_ms: f64,
    _encode_ms: f64,
    _submit_ms: f64,
    _total_ms: f64,
    _verts: usize,
    _indices: usize,
    _visible_indices: usize,
    _opaque_indices: usize,
    _transparent_indices: usize,
    _particle_indices: usize,
    _geometry_rebuilt: bool,
    _shadow_enabled: bool,
    _shadow_res: u32,
    _msaa_samples: u32,
    _post_enabled: bool,
    _bump_strength: f32,
    _shadow_distance: f32,
    _shadow_strength: f32,
) {
}
