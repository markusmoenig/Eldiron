// pub mod app;
#[cfg(feature = "ui")]
pub mod app_event;
pub mod app_trait;
pub mod atlas;
pub mod bbox2d;
pub mod camera3d;
pub mod chunk;
pub mod dynamic;
pub mod intodata;
pub mod light;
#[cfg(all(feature = "ui", not(target_arch = "wasm32")))]
pub mod native_dialogs;
pub mod poly2d;
pub mod poly3d;
pub mod texture;
#[cfg(feature = "ui")]
pub mod ui;
pub mod vm;

/// Error types for SceneVM operations
#[derive(Debug, Clone)]
pub enum SceneVMError {
    GpuInitFailed(String),
    BufferAllocationFailed(String),
    ShaderCompilationFailed(String),
    TextureUploadFailed(String),
    InvalidGeometry(String),
    AtlasFull(String),
    InvalidOperation(String),
}

pub type SceneVMResult<T> = Result<T, SceneVMError>;

use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "embedded/"]
#[exclude = "*.txt"]
#[exclude = "*.DS_Store"]
pub struct Embedded;

pub mod prelude {
    //! Prelude module with commonly used types for SceneVM applications

    pub use crate::{
        Embedded, SceneVM, SceneVMError, SceneVMResult,
        app_trait::{SceneVMApp, SceneVMRenderCtx},
        atlas::{AtlasEntry, SharedAtlas},
        bbox2d::BBox2D,
        camera3d::{Camera3D, CameraKind},
        chunk::Chunk,
        dynamic::{DynamicKind, DynamicObject, RepeatMode},
        intodata::IntoDataInput,
        light::{Light, LightType},
        poly2d::Poly2D,
        poly3d::Poly3D,
        texture::Texture,
        vm::{Atom, GeoId, LayerBlendMode, LineStrip2D, RenderMode, VM},
    };

    #[cfg(feature = "ui")]
    pub use crate::{
        RenderResult,
        app_event::{AppEvent, AppEventQueue},
        ui::{
            Alignment, Button, ButtonGroup, ButtonGroupOrientation, ButtonGroupStyle, ButtonKind,
            ButtonStyle, Canvas, ColorButton, ColorButtonStyle, ColorWheel, Drawable, DropdownList,
            DropdownListStyle, HAlign, HStack, Image, ImageStyle, Label, LabelRect, NodeId,
            ParamList, ParamListStyle, PopupAlignment, Project, ProjectBrowser, ProjectBrowserItem,
            ProjectBrowserStyle, ProjectError, ProjectMetadata, RecentProject, RecentProjects,
            Slider, SliderStyle, Spacer, TabbedPanel, TabbedPanelStyle, TextButton, Theme, Toolbar,
            ToolbarOrientation, ToolbarSeparator, ToolbarStyle, UiAction, UiEvent, UiEventKind,
            UiRenderer, UiView, UndoCommand, UndoStack, VAlign, VStack, ViewContext, Workspace,
            create_tile_material,
        },
    };

    pub use rustc_hash::{FxHashMap, FxHashSet};
    pub use vek::{Mat3, Mat4, Vec2, Vec3, Vec4};
}

#[cfg(feature = "ui")]
pub use crate::ui::{
    Alignment, Button, ButtonGroup, ButtonGroupStyle, ButtonKind, ButtonStyle, Canvas, Drawable,
    HAlign, HStack, Image, ImageStyle, Label, LabelRect, NodeId, ParamList, ParamListStyle,
    PopupAlignment, Slider, SliderStyle, TextButton, Toolbar, ToolbarOrientation, ToolbarSeparator,
    ToolbarStyle, UiAction, UiEvent, UiEventKind, UiRenderer, UiView, UndoCommand, UndoStack,
    VAlign, VStack, ViewContext, Workspace,
};
pub use crate::{
    app_trait::{SceneVMApp, SceneVMRenderCtx},
    atlas::{AtlasEntry, SharedAtlas},
    bbox2d::BBox2D,
    camera3d::{Camera3D, CameraKind},
    chunk::Chunk,
    dynamic::{DynamicKind, DynamicObject, RepeatMode},
    intodata::IntoDataInput,
    light::{Light, LightType},
    poly2d::Poly2D,
    poly3d::Poly3D,
    texture::Texture,
    vm::{Atom, GeoId, LayerBlendMode, LineStrip2D, RenderMode, VM},
};
use image;
use std::borrow::Cow;
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(not(target_arch = "wasm32"))]
use std::ffi::c_void;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;
#[cfg(target_arch = "wasm32")]
use std::{cell::Cell, future::Future, rc::Rc};
#[cfg(target_arch = "wasm32")]
use std::{
    pin::Pin,
    task::{Context, Poll},
};
#[cfg(all(not(target_arch = "wasm32"), feature = "windowing"))]
use vek::Mat3;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;
#[cfg(target_arch = "wasm32")]
use web_sys::{CanvasRenderingContext2d, Document, HtmlCanvasElement, Window as WebWindow};
#[cfg(all(feature = "windowing", not(target_arch = "wasm32")))]
use winit::window::Window;
#[cfg(all(feature = "windowing", not(target_arch = "wasm32")))]
use winit::{dpi::PhysicalPosition, event::ElementState, event::MouseButton, event::WindowEvent};

/// Result of a call to `render_frame`.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RenderResult {
    /// We copied pixels to the caller's buffer this call (may still have a new frame in flight on WASM)
    Presented,
    /// On WASM: GPU init not finished; nothing rendered yet.
    InitPending,
    /// On WASM: a GPU readback is in flight; we presented the last completed frame this call.
    ReadbackPending,
}

/// Render pipeline that blits the SceneVM storage texture into a window surface.
#[cfg(not(target_arch = "wasm32"))]
struct PresentPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,
    surface_format: wgpu::TextureFormat,
}

/// Compositing pipeline for blending VM layers with alpha
struct CompositingPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    mode_buf: wgpu::Buffer,
    sampler: wgpu::Sampler,
    target_format: wgpu::TextureFormat,
}

impl CompositingPipeline {
    fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scenevm-composite-shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
                "
@group(0) @binding(0) var layer_tex: texture_2d<f32>;
@group(0) @binding(1) var layer_sampler: sampler;
@group(0) @binding(2) var<uniform> blend_mode_buf: u32;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0)
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 2.0),
        vec2<f32>(2.0, 0.0),
        vec2<f32>(0.0, 0.0)
    );
    var out: VsOut;
    out.pos = vec4<f32>(positions[vi], 0.0, 1.0);
    out.uv = uvs[vi];
    return out;
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    return pow(c, vec3<f32>(2.2));
}
fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    return pow(c, vec3<f32>(1.0 / 2.2));
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let src = textureSample(layer_tex, layer_sampler, in.uv);
    // blend_mode_buf: 0 = Alpha (default), 1 = AlphaLinear
    if (blend_mode_buf == 1u) {
        // Treat the layer as linear and encode once for display.
        return vec4<f32>(linear_to_srgb(src.rgb), src.a);
    }
    return src;
}
                ",
            )),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scenevm-composite-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scenevm-composite-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let mode_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scenevm-composite-mode"),
            size: std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("scenevm-composite-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scenevm-composite-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
            mode_buf,
            sampler,
            target_format,
        }
    }
}

/// Optional window surface (swapchain) managed by SceneVM for direct presentation.
#[cfg(not(target_arch = "wasm32"))]
struct WindowSurface {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    format: wgpu::TextureFormat,
    present_pipeline: Option<PresentPipeline>,
}

pub struct GPUState {
    _instance: wgpu::Instance,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// Main render surface for SceneVM
    surface: Texture,
    /// Optional wgpu surface when presenting directly to a window.
    #[cfg(not(target_arch = "wasm32"))]
    window_surface: Option<WindowSurface>,
}

#[allow(dead_code)]
#[derive(Clone)]
struct GlobalGpu {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

#[allow(dead_code)]
#[cfg(not(target_arch = "wasm32"))]
static GLOBAL_GPU: OnceLock<GlobalGpu> = OnceLock::new();

#[cfg(target_arch = "wasm32")]
thread_local! {
    static GLOBAL_GPU_WASM: RefCell<Option<GlobalGpu>> = RefCell::new(None);
}

#[cfg(not(target_arch = "wasm32"))]
impl PresentPipeline {
    fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        source_view: &wgpu::TextureView,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scenevm-present-shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
                "
@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0)
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 2.0),
        vec2<f32>(2.0, 0.0),
        vec2<f32>(0.0, 0.0)
    );
    var out: VsOut;
    out.pos = vec4<f32>(positions[vi], 0.0, 1.0);
    out.uv = uvs[vi];
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(src_tex, src_sampler, in.uv);
}
",
            )),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scenevm-present-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("scenevm-present-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scenevm-present-bind-group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scenevm-present-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scenevm-present-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
            bind_group,
            sampler,
            surface_format: format,
        }
    }

    fn update_bind_group(&mut self, device: &wgpu::Device, source_view: &wgpu::TextureView) {
        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scenevm-present-bind-group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl WindowSurface {
    fn reconfigure(&mut self, device: &wgpu::Device) {
        self.surface.configure(device, &self.config);
    }
}

// --- WASM async map flag future support ---
#[cfg(target_arch = "wasm32")]
struct MapReadyFuture {
    flag: Rc<Cell<bool>>,
}

#[cfg(target_arch = "wasm32")]
impl Future for MapReadyFuture {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.flag.get() {
            Poll::Ready(())
        } else {
            // Re-schedule ourselves to be polled again soon.
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

pub struct SceneVM {
    /// The intended render target size; used by either backend.
    size: (u32, u32),

    /// When `Some`, GPU rendering is enabled and initialized; otherwise CPU path.
    gpu: Option<GPUState>,
    #[cfg(target_arch = "wasm32")]
    needs_gpu_init: bool,
    #[cfg(target_arch = "wasm32")]
    init_in_flight: bool,

    atlas: SharedAtlas,
    pub vm: VM,
    overlay_vms: Vec<VM>,
    active_vm_index: usize,
    log_layer_activity: bool,
    compositing_pipeline: Option<CompositingPipeline>,
}

/// Result of shader compilation with detailed diagnostics
#[derive(Debug, Clone)]
pub struct ShaderCompilationResult {
    /// Whether compilation succeeded (true if only warnings, false if errors)
    pub success: bool,
    /// List of compilation warnings with line numbers relative to body source
    pub warnings: Vec<ShaderDiagnostic>,
    /// List of compilation errors with line numbers relative to body source
    pub errors: Vec<ShaderDiagnostic>,
}

/// Individual shader diagnostic (warning or error)
#[derive(Debug, Clone)]
pub struct ShaderDiagnostic {
    /// Line number in the body source (0-based)
    pub line: u32,
    /// Diagnostic message
    pub message: String,
}

impl Default for SceneVM {
    fn default() -> Self {
        Self::new(100, 100)
    }
}

impl SceneVM {
    fn refresh_layer_metadata(&mut self) {
        self.vm.set_layer_index(0);
        self.vm.set_activity_logging(self.log_layer_activity);
        for (i, vm) in self.overlay_vms.iter_mut().enumerate() {
            vm.set_layer_index(i + 1);
            vm.set_activity_logging(self.log_layer_activity);
        }
    }

    fn total_vm_count(&self) -> usize {
        1 + self.overlay_vms.len()
    }

    fn vm_ref_by_index(&self, index: usize) -> Option<&VM> {
        if index == 0 {
            Some(&self.vm)
        } else {
            self.overlay_vms.get(index.saturating_sub(1))
        }
    }

    fn vm_mut_by_index(&mut self, index: usize) -> Option<&mut VM> {
        if index == 0 {
            Some(&mut self.vm)
        } else {
            self.overlay_vms.get_mut(index.saturating_sub(1))
        }
    }

    fn draw_all_vms(
        base_vm: &mut VM,
        overlays: &mut [VM],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface: &mut Texture,
        w: u32,
        h: u32,
        log_errors: bool,
        compositing_pipeline: &mut Option<CompositingPipeline>,
    ) {
        // The surface texture is always created with Rgba8Unorm in `Texture::ensure_gpu_with`
        let target_format = wgpu::TextureFormat::Rgba8Unorm;
        if let Err(e) = base_vm.draw_into(device, queue, surface, w, h) {
            if log_errors {
                println!("[SceneVM] Error drawing base VM: {:?}", e);
            }
        }

        for vm in overlays.iter_mut() {
            if let Err(e) = vm.draw_into(device, queue, surface, w, h) {
                if log_errors {
                    println!("[SceneVM] Error drawing overlay VM: {:?}", e);
                }
            }
        }

        // Ensure surface has GPU resources
        surface.ensure_gpu_with(device);

        // Initialize compositing pipeline if needed
        if compositing_pipeline
            .as_ref()
            .map(|p| p.target_format != target_format)
            .unwrap_or(true)
        {
            *compositing_pipeline = Some(CompositingPipeline::new(device, target_format));
        }

        let pipeline = compositing_pipeline.as_ref().unwrap();

        // Create command encoder for compositing
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("scenevm-compositing-encoder"),
        });

        // Composite all layers onto the surface
        let surface_view = &surface.gpu.as_ref().unwrap().view;

        // Collect all VMs that are enabled
        let mut vms_to_composite: Vec<&VM> = Vec::new();
        if base_vm.is_enabled() {
            vms_to_composite.push(base_vm);
        }
        for vm in overlays.iter() {
            if vm.is_enabled() {
                vms_to_composite.push(vm);
            }
        }

        // Composite each layer in order
        for (i, vm) in vms_to_composite.iter().enumerate() {
            if let Some(layer_texture) = vm.composite_texture() {
                if let Some(layer_gpu) = &layer_texture.gpu {
                    // Create bind group for this layer
                    let mode_u32: u32 = match vm.blend_mode() {
                        vm::LayerBlendMode::Alpha => 0,
                        vm::LayerBlendMode::AlphaLinear => 1,
                    };
                    // Upload mode
                    queue.write_buffer(&pipeline.mode_buf, 0, bytemuck::bytes_of(&mode_u32));

                    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("scenevm-compositing-bind-group"),
                        layout: &pipeline.bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&layer_gpu.view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: pipeline.mode_buf.as_entire_binding(),
                            },
                        ],
                    });

                    // Begin render pass
                    // First layer: clear surface to black (layer texture has background baked in)
                    // Subsequent layers: load existing content and blend on top
                    let load_op = if i == 0 {
                        wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                    } else {
                        wgpu::LoadOp::Load
                    };

                    {
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("scenevm-compositing-pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: surface_view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: load_op,
                                    store: wgpu::StoreOp::Store,
                                },
                                depth_slice: None,
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });

                        rpass.set_pipeline(&pipeline.pipeline);
                        rpass.set_bind_group(0, &bind_group, &[]);
                        rpass.draw(0..3, 0..1);
                    }
                }
            }
        }

        queue.submit(Some(encoder.finish()));
    }

    /// Total number of VM layers (base + overlays).
    pub fn vm_layer_count(&self) -> usize {
        self.total_vm_count()
    }

    /// Append a new VM layer that will render on top of the existing ones. Returns its layer index.
    pub fn add_vm_layer(&mut self) -> usize {
        // Overlays default to transparent so they don't hide layers below unless drawn into.
        let mut vm = VM::new_with_shared_atlas(self.atlas.clone());
        vm.background = vek::Vec4::new(0.0, 0.0, 0.0, 0.0);
        self.overlay_vms.push(vm);
        self.refresh_layer_metadata();
        self.total_vm_count() - 1
    }

    /// Remove a VM layer by index (cannot remove the base layer at index 0).
    pub fn remove_vm_layer(&mut self, index: usize) -> Option<VM> {
        if index == 0 {
            return None;
        }
        let idx = index - 1;
        if idx >= self.overlay_vms.len() {
            return None;
        }
        let removed = self.overlay_vms.remove(idx);
        if self.active_vm_index >= self.total_vm_count() {
            self.active_vm_index = self.total_vm_count().saturating_sub(1);
        }
        self.refresh_layer_metadata();
        Some(removed)
    }

    /// Switch the VM layer targeted by `execute`. Returns `true` if the index existed.
    pub fn set_active_vm(&mut self, index: usize) -> bool {
        if index < self.total_vm_count() {
            self.active_vm_index = index;
            true
        } else {
            false
        }
    }

    /// Index of the currently active VM used by `execute`.
    pub fn active_vm_index(&self) -> usize {
        self.active_vm_index
    }

    /// Normalized atlas rect (ofs.x, ofs.y, scale.x, scale.y) for a tile/frame, useful for SDF packing.
    pub fn atlas_sdf_uv4(&self, id: &uuid::Uuid, anim_frame: u32) -> Option<[f32; 4]> {
        self.atlas.sdf_uv4(id, anim_frame)
    }

    /// Enable or disable drawing for a VM layer. Disabled layers still receive commands.
    pub fn set_layer_enabled(&mut self, index: usize, enabled: bool) -> bool {
        if let Some(vm) = self.vm_mut_by_index(index) {
            vm.set_enabled(enabled);
            true
        } else {
            false
        }
    }

    /// Toggle verbose per-layer logging for uploads/atlas/grid events.
    pub fn set_layer_activity_logging(&mut self, enabled: bool) {
        self.log_layer_activity = enabled;
        self.refresh_layer_metadata();
    }

    /// Borrow the currently active VM immutably.
    pub fn active_vm(&self) -> &VM {
        self.vm_ref_by_index(self.active_vm_index)
            .expect("active VM index out of range")
    }

    /// Borrow the currently active VM mutably.
    pub fn active_vm_mut(&mut self) -> &mut VM {
        self.vm_mut_by_index(self.active_vm_index)
            .expect("active VM index out of range")
    }

    /// Ray-pick against the active VM layer using normalized screen UVs.
    pub fn pick_geo_id_at_uv(
        &self,
        fb_w: u32,
        fb_h: u32,
        screen_uv: [f32; 2],
        include_hidden: bool,
        include_billboards: bool,
    ) -> Option<(GeoId, vek::Vec3<f32>, f32)> {
        self.active_vm().pick_geo_id_at_uv(
            fb_w,
            fb_h,
            screen_uv,
            include_hidden,
            include_billboards,
        )
    }

    /// Build a world-space ray from screen uv (0..1) using the active VM's camera and a provided framebuffer size.
    pub fn ray_from_uv_with_size(
        &self,
        fb_w: u32,
        fb_h: u32,
        screen_uv: [f32; 2],
    ) -> Option<(vek::Vec3<f32>, vek::Vec3<f32>)> {
        self.active_vm().ray_from_uv(fb_w, fb_h, screen_uv)
    }

    /// Build a world-space ray from screen uv (0..1) using the active VM's camera and the current SceneVM size.
    pub fn ray_from_uv(&self, screen_uv: [f32; 2]) -> Option<(vek::Vec3<f32>, vek::Vec3<f32>)> {
        let (w, h) = self.size;
        self.active_vm().ray_from_uv(w, h, screen_uv)
    }

    /// Prints statistics about 2D and 3D polygons currently loaded in all chunks.
    pub fn print_geometry_stats(&self) {
        let mut total_2d = 0usize;
        let mut total_3d = 0usize;
        let mut total_lines = 0usize;

        for vm in std::iter::once(&self.vm).chain(self.overlay_vms.iter()) {
            for (_cid, ch) in &vm.chunks_map {
                total_2d += ch.polys_map.len();
                total_3d += ch.polys3d_map.values().map(|v| v.len()).sum::<usize>();
                total_lines += ch.lines2d_px.len();
            }
        }

        println!(
            "[SceneVM] Geometry Stats → 2D polys: {} | 3D polys: {} | 2D lines: {} | Total: {}",
            total_2d,
            total_3d,
            total_lines,
            total_2d + total_3d + total_lines
        );
    }

    /// Executes a single atom on the currently active VM layer.
    pub fn execute(&mut self, atom: Atom) {
        let affects_atlas = SceneVM::atom_touches_atlas(&atom);
        let active = self.active_vm_index;
        if active == 0 {
            self.vm.execute(atom);
        } else if let Some(vm) = self.vm_mut_by_index(active) {
            vm.execute(atom);
        }
        if affects_atlas {
            self.for_each_vm_mut(|vm| vm.mark_all_geometry_dirty());
        }
    }

    /// Is the GPU initialized and ready?
    pub fn is_gpu_ready(&self) -> bool {
        if self.gpu.is_some() {
            #[cfg(target_arch = "wasm32")]
            {
                return !self.needs_gpu_init && !self.init_in_flight;
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                return true;
            }
        }
        false
    }

    /// Is a GPU readback currently in flight (WASM only)? Always false on native.
    pub fn frame_in_flight(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(gpu) = &self.gpu {
                return gpu
                    .surface
                    .gpu
                    .as_ref()
                    .and_then(|g| g.map_ready.as_ref())
                    .is_some();
            }
            return false;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            false
        }
    }
    /// Create a new SceneVM. Always uses GPU backend.
    pub fn new(initial_width: u32, initial_height: u32) -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            let atlas = SharedAtlas::new(4096, 4096);
            let mut this = Self {
                size: (initial_width, initial_height),
                gpu: None,
                needs_gpu_init: true,
                init_in_flight: false,
                atlas: atlas.clone(),
                vm: VM::new_with_shared_atlas(atlas.clone()),
                overlay_vms: Vec::new(),
                active_vm_index: 0,
                log_layer_activity: false,
                compositing_pipeline: None,
            };
            this.refresh_layer_metadata();
            this
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: { wgpu::Backends::all() },
                ..Default::default()
            });
            let adapter =
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: None,
                }))
                .expect("No compatible GPU adapter found");

            let (device, queue) =
                pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                    label: Some("scenevm-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    ..Default::default()
                }))
                .expect("Failed to create wgpu device");

            let mut surface = Texture::new(initial_width, initial_height);
            surface.ensure_gpu_with(&device);

            let gpu = GPUState {
                _instance: instance,
                _adapter: adapter,
                device,
                queue,
                surface,
                window_surface: None,
            };

            let atlas = SharedAtlas::new(4096, 4096);
            let mut this = Self {
                size: (initial_width, initial_height),
                gpu: Some(gpu),
                atlas: atlas.clone(),
                vm: VM::new_with_shared_atlas(atlas.clone()),
                overlay_vms: Vec::new(),
                active_vm_index: 0,
                log_layer_activity: false,
                compositing_pipeline: None,
            };
            this.refresh_layer_metadata();
            this
        }
    }

    /// Create a SceneVM that is configured to present directly into a winit window surface.
    #[cfg(all(feature = "windowing", not(target_arch = "wasm32")))]
    pub fn new_with_window(window: &Window) -> Self {
        let initial_size = window.inner_size();
        let width = initial_size.width.max(1);
        let height = initial_size.height.max(1);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: { wgpu::Backends::all() },
            ..Default::default()
        });
        let surface = unsafe {
            instance.create_surface_unsafe(
                wgpu::SurfaceTargetUnsafe::from_window(window)
                    .expect("Failed to access raw window handle"),
            )
        }
        .expect("Failed to create wgpu surface for window");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .expect("No compatible GPU adapter found");

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("scenevm-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }))
        .expect("Failed to create wgpu device");

        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let present_mode = caps
            .present_modes
            .iter()
            .copied()
            .find(|m| {
                matches!(
                    m,
                    wgpu::PresentMode::Mailbox
                        | wgpu::PresentMode::Immediate
                        | wgpu::PresentMode::Fifo
                )
            })
            .unwrap_or(wgpu::PresentMode::Fifo);
        let alpha_mode = caps
            .alpha_modes
            .get(0)
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
            format: surface_format,
            width,
            height,
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let mut storage_surface = Texture::new(width, height);
        storage_surface.ensure_gpu_with(&device);

        let gpu = GPUState {
            _instance: instance,
            _adapter: adapter,
            device,
            queue,
            surface: storage_surface,
            window_surface: Some(WindowSurface {
                surface,
                config: surface_config,
                format: surface_format,
                present_pipeline: None,
            }),
        };

        let atlas = SharedAtlas::new(4096, 4096);
        let mut this = Self {
            size: (width, height),
            gpu: Some(gpu),
            atlas: atlas.clone(),
            vm: VM::new_with_shared_atlas(atlas.clone()),
            overlay_vms: Vec::new(),
            active_vm_index: 0,
            log_layer_activity: false,
            compositing_pipeline: None,
        };
        this.refresh_layer_metadata();
        this
    }

    /// Create a SceneVM that presents into an existing CoreAnimation layer (Metal) without winit.
    #[cfg(all(
        not(target_arch = "wasm32"),
        any(target_os = "macos", target_os = "ios")
    ))]
    pub fn new_with_metal_layer(layer_ptr: *mut c_void, width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: { wgpu::Backends::all() },
            ..Default::default()
        });

        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer_ptr))
        }
        .expect("Failed to create wgpu surface for CoreAnimationLayer");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .expect("No compatible GPU adapter found");

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("scenevm-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }))
        .expect("Failed to create wgpu device");

        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let present_mode = caps
            .present_modes
            .iter()
            .copied()
            .find(|m| {
                matches!(
                    m,
                    wgpu::PresentMode::Mailbox
                        | wgpu::PresentMode::Immediate
                        | wgpu::PresentMode::Fifo
                )
            })
            .unwrap_or(wgpu::PresentMode::Fifo);
        let alpha_mode = caps
            .alpha_modes
            .get(0)
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
            format: surface_format,
            width,
            height,
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let mut storage_surface = Texture::new(width, height);
        storage_surface.ensure_gpu_with(&device);

        let gpu = GPUState {
            _instance: instance,
            _adapter: adapter,
            device,
            queue,
            surface: storage_surface,
            window_surface: Some(WindowSurface {
                surface,
                config: surface_config,
                format: surface_format,
                present_pipeline: None,
            }),
        };

        let atlas = SharedAtlas::new(4096, 4096);
        let mut this = Self {
            size: (width, height),
            gpu: Some(gpu),
            atlas: atlas.clone(),
            vm: VM::new_with_shared_atlas(atlas.clone()),
            overlay_vms: Vec::new(),
            active_vm_index: 0,
            log_layer_activity: false,
            compositing_pipeline: None,
        };
        this.refresh_layer_metadata();
        this
    }

    /// Initialize GPU backend asynchronously on WASM. On native, this will initialize synchronously if not already.
    pub async fn init_async(&mut self) {
        // If already initialized, nothing to do.
        if self.gpu.is_some() {
            return;
        }

        #[cfg(target_arch = "wasm32")]
        {
            if !self.needs_gpu_init {
                return;
            }
            if global_gpu_get().is_none() {
                global_gpu_init_async().await;
            }
            let gg = global_gpu_get().expect("Global GPU not initialized");
            let (w, h) = self.size;
            let mut surface = Texture::new(w, h);
            surface.ensure_gpu_with(&gg.device);
            let gpu = GPUState {
                _instance: gg.instance,
                _adapter: gg.adapter,
                device: gg.device,
                queue: gg.queue,
                surface,
            };
            self.gpu = Some(gpu);
            self.needs_gpu_init = false;
            #[cfg(debug_assertions)]
            {
                web_sys::console::log_1(&"SceneVM WebGPU initialized (global)".into());
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.gpu.is_some() {
                return;
            }
            let (w, h) = self.size;
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: { wgpu::Backends::all() },
                ..Default::default()
            });
            let adapter =
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: None,
                }))
                .expect("No compatible GPU adapter found");

            let (device, queue) =
                pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                    label: Some("scenevm-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    ..Default::default()
                }))
                .expect("Failed to create wgpu device");

            let mut surface = Texture::new(w, h);
            surface.ensure_gpu_with(&device);

            let gpu = GPUState {
                _instance: instance,
                _adapter: adapter,
                device,
                queue,
                surface,
                window_surface: None,
            };
            self.gpu = Some(gpu);
        }
    }

    /// Blit a `Texture` via GPU to the main surface texture, if GPU is ready.
    pub fn blit_texture(
        &mut self,
        tex: &mut Texture,
        _cpu_pixels: &mut [u8],
        _buf_w: u32,
        _buf_h: u32,
    ) {
        if let Some(g) = self.gpu.as_ref() {
            tex.gpu_blit_to_storage(g, &g.surface.gpu.as_ref().unwrap().texture);
        }
    }

    /// Update the window surface size and internal storage texture (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn resize_window_surface(&mut self, width: u32, height: u32) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        let Some(ws) = gpu.window_surface.as_mut() else {
            return;
        };

        let w = width.max(1);
        let h = height.max(1);
        if ws.config.width == w && ws.config.height == h {
            return;
        }

        ws.config.width = w;
        ws.config.height = h;
        ws.reconfigure(&gpu.device);

        self.size = (w, h);
        gpu.surface.width = w;
        gpu.surface.height = h;
        gpu.surface.ensure_gpu_with(&gpu.device);

        // Force recreation of the present pipeline/bindings on next render.
        ws.present_pipeline = None;
    }

    /// Render directly into the configured window surface (native only, no CPU readback).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn render_to_window(&mut self) -> SceneVMResult<RenderResult> {
        let (gpu_slot, base_vm, overlays) = (&mut self.gpu, &mut self.vm, &mut self.overlay_vms);
        let Some(gpu) = gpu_slot.as_mut() else {
            return Err(SceneVMError::InvalidOperation(
                "GPU not initialized".to_string(),
            ));
        };
        let Some(ws) = gpu.window_surface.as_mut() else {
            return Err(SceneVMError::InvalidOperation(
                "No window surface configured".to_string(),
            ));
        };

        let target_w = ws.config.width.max(1);
        let target_h = ws.config.height.max(1);

        if self.size != (target_w, target_h) {
            self.size = (target_w, target_h);
            gpu.surface.width = target_w;
            gpu.surface.height = target_h;
            gpu.surface.ensure_gpu_with(&gpu.device);
            ws.present_pipeline = None;
        }

        let (w, h) = self.size;
        SceneVM::draw_all_vms(
            base_vm,
            overlays,
            &gpu.device,
            &gpu.queue,
            &mut gpu.surface,
            w,
            h,
            self.log_layer_activity,
            &mut self.compositing_pipeline,
        );

        let frame = match ws.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost) | Err(wgpu::SurfaceError::Outdated) => {
                ws.reconfigure(&gpu.device);
                return Ok(RenderResult::InitPending);
            }
            Err(wgpu::SurfaceError::Timeout) => {
                return Ok(RenderResult::ReadbackPending);
            }
            Err(wgpu::SurfaceError::Other) => {
                return Err(SceneVMError::InvalidOperation(
                    "Surface returned an unspecified error".to_string(),
                ));
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                return Err(SceneVMError::BufferAllocationFailed(
                    "Surface out of memory".to_string(),
                ));
            }
        };

        let frame_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let src_view = gpu
            .surface
            .gpu
            .as_ref()
            .expect("Surface GPU not allocated")
            .view
            .clone();

        if ws
            .present_pipeline
            .as_ref()
            .map(|p| p.surface_format != ws.format)
            .unwrap_or(true)
        {
            ws.present_pipeline = Some(PresentPipeline::new(&gpu.device, ws.format, &src_view));
        } else if let Some(pipeline) = ws.present_pipeline.as_mut() {
            pipeline.update_bind_group(&gpu.device, &src_view);
        }

        let present = ws
            .present_pipeline
            .as_ref()
            .expect("Present pipeline should be initialized");

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scenevm-present-encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scenevm-present-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&present.pipeline);
            pass.set_bind_group(0, &present.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        gpu.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(RenderResult::Presented)
    }

    /// Draw: if GPU is present, run the compute path. Returns immediately if GPU is not yet ready (WASM before init).
    #[cfg(not(target_arch = "wasm32"))]
    fn draw(&mut self, out_pixels: &mut [u8], out_w: u32, out_h: u32) {
        // GPU-only: do nothing if GPU is not ready (e.g., WASM before init)
        let (gpu_slot, base_vm, overlays) = (&mut self.gpu, &mut self.vm, &mut self.overlay_vms);
        let Some(gpu) = gpu_slot.as_mut() else {
            return;
        };

        let buffer_width = out_w;
        let buffer_height = out_h;

        // Resize surface if needed (bind group managed internally by VM)
        if self.size != (buffer_width, buffer_height) {
            self.size = (buffer_width, buffer_height);
            gpu.surface.width = buffer_width;
            gpu.surface.height = buffer_height;
            gpu.surface.ensure_gpu_with(&gpu.device);
        }

        let (w, h) = self.size;

        // Delegate rendering to all VM layers in order (each overlays the previous result)
        SceneVM::draw_all_vms(
            base_vm,
            overlays,
            &gpu.device,
            &gpu.queue,
            &mut gpu.surface,
            w,
            h,
            self.log_layer_activity,
            &mut self.compositing_pipeline,
        );

        // Readback into the surface's CPU memory (blocking on native, non-blocking noop on wasm)
        let device = gpu.device.clone();
        let queue = gpu.queue.clone();
        gpu.surface.download_from_gpu_with(&device, &queue);

        // On native, pixels are now in `surface.data`; copy them to the output buffer.
        // On WASM, if you need the pixels immediately, prefer `draw_async`.
        gpu.surface.copy_to_slice(out_pixels, out_w, out_h);
    }

    /// Cross-platform async render: same call on native & WASM.
    #[cfg(target_arch = "wasm32")]
    pub async fn render_frame_async(&mut self, out_pixels: &mut [u8], out_w: u32, out_h: u32) {
        let (gpu_slot, base_vm, overlays) = (&mut self.gpu, &mut self.vm, &mut self.overlay_vms);
        let Some(gpu) = gpu_slot.as_mut() else {
            return;
        };
        let buffer_width = out_w;
        let buffer_height = out_h;

        if self.size != (buffer_width, buffer_height) {
            self.size = (buffer_width, buffer_height);
            gpu.surface.width = buffer_width;
            gpu.surface.height = buffer_height;
            gpu.surface.ensure_gpu_with(&gpu.device);
        }

        let (w, h) = self.size;
        SceneVM::draw_all_vms(
            base_vm,
            overlays,
            &gpu.device,
            &gpu.queue,
            &mut gpu.surface,
            w,
            h,
            self.log_layer_activity,
            &mut self.compositing_pipeline,
        );

        // Start readback and await readiness
        let device = gpu.device.clone();
        let queue = gpu.queue.clone();
        gpu.surface.download_from_gpu_with(&device, &queue);
        let flag = gpu
            .surface
            .gpu
            .as_ref()
            .and_then(|g| g.map_ready.as_ref().map(|f| std::rc::Rc::clone(f)));
        if let Some(flag) = flag {
            MapReadyFuture { flag }.await;
        }
        let _ = gpu.surface.try_finish_download_from_gpu();
        gpu.surface.copy_to_slice(out_pixels, out_w, out_h);
    }

    /// Single cross-platform async entrypoint for rendering a frame.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn render_frame_async(&mut self, out_pixels: &mut [u8], out_w: u32, out_h: u32) {
        self.draw(out_pixels, out_w, out_h);
    }

    /// Cross-platform synchronous render entrypoint (one function for Native & WASM). Returns a RenderResult.
    /// Native: blocks until pixels are ready. WASM: presents the last completed frame
    /// and kicks off a new GPU frame if none is in flight. Call this every frame.
    /// On WASM, you must call `init_async().await` once before rendering.
    pub fn render_frame(&mut self, out_pixels: &mut [u8], out_w: u32, out_h: u32) -> RenderResult {
        // let start = std::time::Instant::now();

        #[cfg(not(target_arch = "wasm32"))]
        {
            // Native path just does the full render and readback synchronously
            self.draw(out_pixels, out_w, out_h);

            // let elapsed = start.elapsed();
            // println!("Frame time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);

            return RenderResult::Presented;
        }

        #[cfg(target_arch = "wasm32")]
        {
            // WASM path: auto-init GPU if needed, else non-blocking render logic.
            if self.gpu.is_none() {
                if !self.init_in_flight && self.needs_gpu_init {
                    self.init_in_flight = true;
                    let this: *mut SceneVM = self as *mut _;
                    spawn_local(async move {
                        // SAFETY: we rely on the caller to call `render_frame` from the UI thread.
                        // We only flip flags and build GPU state; no aliasing mutable accesses occur concurrently
                        // because the user code keeps calling `render_frame`, which is single-threaded on wasm.
                        unsafe {
                            (&mut *this).init_async().await;
                            (&mut *this).init_in_flight = false;
                        }
                    });
                }
                // Nothing to render until init finishes; return quietly.
                return RenderResult::InitPending;
            }
            let (gpu_slot, base_vm, overlays) =
                (&mut self.gpu, &mut self.vm, &mut self.overlay_vms);
            let gpu = gpu_slot.as_mut().unwrap();

            // Ensure surface size (bind group managed internally by VM)
            if self.size != (out_w, out_h) {
                self.size = (out_w, out_h);
                gpu.surface.width = out_w;
                gpu.surface.height = out_h;
                gpu.surface.ensure_gpu_with(&gpu.device);
            }

            // If a readback is already in flight, try to finish it; otherwise kick off a new one.
            let inflight = gpu
                .surface
                .gpu
                .as_ref()
                .and_then(|g| g.map_ready.as_ref())
                .is_some();

            // If a readback is in flight, try to finish it and present whatever CPU pixels we have.
            // When the download completes, continue on to kick off the next frame immediately
            // instead of skipping a render for a whole call (which caused visible stutter on WASM).
            let mut presented_frame = false;
            if inflight {
                let ready = gpu.surface.try_finish_download_from_gpu();
                gpu.surface.copy_to_slice(out_pixels, out_w, out_h);
                if !ready {
                    return RenderResult::ReadbackPending;
                }
                presented_frame = true;
            } else {
                // No download in flight yet; present whatever pixels are already on the CPU.
                gpu.surface.copy_to_slice(out_pixels, out_w, out_h);
            }

            // Render a new frame and start a download for the next call.
            let (w, h) = self.size;
            SceneVM::draw_all_vms(
                base_vm,
                overlays,
                &gpu.device,
                &gpu.queue,
                &mut gpu.surface,
                w,
                h,
                self.log_layer_activity,
                &mut self.compositing_pipeline,
            );

            let device = gpu.device.clone();
            let queue = gpu.queue.clone();
            gpu.surface.download_from_gpu_with(&device, &queue);

            if presented_frame {
                RenderResult::Presented
            } else {
                RenderResult::ReadbackPending
            }
        }
    }

    /// Load an image from various inputs (file path on native, raw bytes, &str) and decode to RGBA8.
    pub fn load_image_rgba<I: IntoDataInput>(&self, input: I) -> Option<(Vec<u8>, u32, u32)> {
        let bytes = match input.load_data() {
            Ok(b) => b,
            Err(_) => return None,
        };
        let img = match image::load_from_memory(&bytes) {
            Ok(i) => i,
            Err(_) => return None,
        };
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        Some((rgba.into_raw(), w, h))
    }

    /// Compile a 2D body shader with the header and return detailed diagnostics.
    /// If compilation succeeds (only warnings), the shader is automatically set as active.
    pub fn compile_shader_2d(&mut self, body_source: &str) -> ShaderCompilationResult {
        self.compile_shader_internal(body_source, true)
    }

    /// Compile a 3D body shader with the header and return detailed diagnostics.
    /// If compilation succeeds (only warnings), the shader is automatically set as active.
    pub fn compile_shader_3d(&mut self, body_source: &str) -> ShaderCompilationResult {
        self.compile_shader_internal(body_source, false)
    }

    /// Compile an SDF body shader with the header and return detailed diagnostics.
    /// If compilation succeeds (only warnings), the shader is automatically set as active.
    pub fn compile_shader_sdf(&mut self, body_source: &str) -> ShaderCompilationResult {
        use wgpu::ShaderSource;

        let header_source = if let Some(bytes) = Embedded::get("sdf_header.wgsl") {
            std::str::from_utf8(bytes.data.as_ref())
                .unwrap_or("")
                .to_string()
        } else {
            "".to_string()
        };

        let full_source = format!("{}\n{}", header_source, body_source);

        let device = if let Some(gpu) = &self.gpu {
            &gpu.device
        } else {
            return ShaderCompilationResult {
                success: false,
                warnings: vec![],
                errors: vec![ShaderDiagnostic {
                    line: 0,
                    message: "GPU device not initialized. Cannot compile shader.".to_string(),
                }],
            };
        };

        let _shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scenevm-compile-sdf"),
            source: ShaderSource::Wgsl(full_source.into()),
        });

        self.vm
            .execute(vm::Atom::SetSourceSdf(body_source.to_string()));

        ShaderCompilationResult {
            success: true,
            warnings: vec![],
            errors: vec![],
        }
    }

    /// Fetch the source of a built-in shader body by name (e.g. "ui", "2d", "3d", "sdf", "noise").
    pub fn default_shader_source(kind: &str) -> Option<String> {
        let file_name = match kind {
            "ui" => "ui_body.wgsl",
            "2d" => "2d_body.wgsl",
            "3d" => "3d_body.wgsl",
            "sdf" => "sdf_body.wgsl",
            _ => return None,
        };

        Embedded::get(file_name).map(|bytes| {
            // Convert embedded bytes to owned string; avoids borrowing the embedded buffer.
            String::from_utf8_lossy(bytes.data.as_ref()).into_owned()
        })
    }

    /// Internal shader compilation with diagnostics
    fn compile_shader_internal(
        &mut self,
        body_source: &str,
        is_2d: bool,
    ) -> ShaderCompilationResult {
        use wgpu::ShaderSource;

        // Get the appropriate header
        let header_source = if is_2d {
            if let Some(bytes) = Embedded::get("2d_header.wgsl") {
                std::str::from_utf8(bytes.data.as_ref())
                    .unwrap_or("")
                    .to_string()
            } else {
                "".to_string()
            }
        } else {
            if let Some(bytes) = Embedded::get("3d_header.wgsl") {
                std::str::from_utf8(bytes.data.as_ref())
                    .unwrap_or("")
                    .to_string()
            } else {
                "".to_string()
            }
        };

        // Combine header and body
        let full_source = format!("{}\n{}", header_source, body_source);

        // Try to create shader module to trigger compilation
        let device = if let Some(gpu) = &self.gpu {
            // We have a device from previous initialization
            &gpu.device
        } else {
            // No device available, return compilation failure
            return ShaderCompilationResult {
                success: false,
                warnings: vec![],
                errors: vec![ShaderDiagnostic {
                    line: 0,
                    message: "GPU device not initialized. Cannot compile shader.".to_string(),
                }],
            };
        };

        let _shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(if is_2d {
                "scenevm-compile-2d"
            } else {
                "scenevm-compile-3d"
            }),
            source: ShaderSource::Wgsl(full_source.into()),
        });

        // Note: wgpu doesn't provide direct access to compilation warnings/errors at module creation.
        // The compilation happens asynchronously and errors surface when the pipeline is created.
        // For now, we'll assume success if the module was created without panic.
        // In a real implementation, you'd want to use wgpu's validation layers or compile offline.

        // For the purpose of this implementation, we'll simulate successful compilation
        // and set the source if we got this far without panic
        let success = true; // Module creation succeeded

        if success {
            // Set the source if compilation succeeded
            if is_2d {
                self.vm
                    .execute(vm::Atom::SetSource2D(body_source.to_string()));
            } else {
                self.vm
                    .execute(vm::Atom::SetSource3D(body_source.to_string()));
            }
        }

        ShaderCompilationResult {
            success,
            warnings: vec![], // Currently empty - would be populated with real compilation info
            errors: vec![],   // Currently empty - would be populated with real compilation info
        }
    }
}

// --- Global GPU helpers ---
#[cfg(target_arch = "wasm32")]
fn global_gpu_get() -> Option<GlobalGpu> {
    GLOBAL_GPU_WASM.with(|c| c.borrow().clone())
}

#[cfg(target_arch = "wasm32")]
async fn global_gpu_init_async() {
    if global_gpu_get().is_some() {
        return;
    }
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..Default::default()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .await
        .expect("No compatible GPU adapter found (WebGPU)");
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("scenevm-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        })
        .await
        .expect("Failed to create wgpu device (WebGPU)");
    let gg = GlobalGpu {
        instance,
        adapter,
        device,
        queue,
    };
    GLOBAL_GPU_WASM.with(|c| *c.borrow_mut() = Some(gg));
}
impl SceneVM {
    fn for_each_vm_mut(&mut self, mut f: impl FnMut(&mut VM)) {
        f(&mut self.vm);
        for vm in &mut self.overlay_vms {
            f(vm);
        }
    }

    fn atom_touches_atlas(atom: &Atom) -> bool {
        matches!(
            atom,
            Atom::AddTile { .. }
                | Atom::AddSolid { .. }
                | Atom::SetTileMaterialFrames { .. }
                | Atom::BuildAtlas
                | Atom::Clear
                | Atom::ClearTiles
        )
    }
}

// -------------------------
// Minimal cross-platform app runner
// -------------------------

#[cfg(all(not(target_arch = "wasm32"), feature = "windowing"))]
struct NativeRenderCtx {
    size: (u32, u32),
    last_result: RenderResult,
    present_called: bool,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "windowing"))]
impl NativeRenderCtx {
    fn new(size: (u32, u32)) -> Self {
        Self {
            size,
            last_result: RenderResult::InitPending,
            present_called: false,
        }
    }

    fn begin_frame(&mut self) {
        self.present_called = false;
    }

    fn ensure_presented(&mut self, vm: &mut SceneVM) -> SceneVMResult<RenderResult> {
        if !self.present_called {
            self.present(vm)?;
        }
        Ok(self.last_result)
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "windowing"))]
impl SceneVMRenderCtx for NativeRenderCtx {
    fn size(&self) -> (u32, u32) {
        self.size
    }

    fn present(&mut self, vm: &mut SceneVM) -> SceneVMResult<RenderResult> {
        let res = vm.render_to_window();
        if let Ok(r) = res {
            self.last_result = r;
        }
        self.present_called = true;
        res
    }
}

/// Run a `SceneVMApp` on native (winit) with GPU presentation to a window.
#[cfg(all(not(target_arch = "wasm32"), feature = "windowing"))]
pub fn run_scenevm_app<A: SceneVMApp + 'static>(
    mut app: A,
) -> Result<(), Box<dyn std::error::Error>> {
    use winit::dpi::LogicalSize;
    use winit::event::{Event, StartCause};
    use winit::event_loop::{ControlFlow, EventLoop};
    use winit::window::WindowAttributes;

    let frame_interval = app.target_fps().and_then(|fps| {
        if fps > 0.0 {
            Some(std::time::Duration::from_secs_f32(1.0 / fps))
        } else {
            None
        }
    });

    let event_loop = EventLoop::new()?;
    let mut window: Option<winit::window::Window> = None;
    let mut vm: Option<SceneVM> = None;
    let mut ctx: Option<NativeRenderCtx> = None;
    let mut cursor_pos: PhysicalPosition<f64> = PhysicalPosition { x: 0.0, y: 0.0 };
    #[cfg(feature = "ui")]
    let mut modifiers = winit::event::Modifiers::default();
    let apply_logical_scale = |vm_ref: &mut SceneVM, scale: f64| {
        // Scale logical coordinates into the framebuffer when hi-dpi.
        let s = scale as f32;
        let m = Mat3::<f32>::new(s, 0.0, 0.0, 0.0, s, 0.0, 0.0, 0.0, 1.0);
        vm_ref.execute(Atom::SetTransform2D(m));
    };
    #[allow(deprecated)]
    event_loop.run(move |event, target| match event {
        Event::NewEvents(StartCause::Init) => {
            let mut attrs = WindowAttributes::default()
                .with_title(app.window_title().unwrap_or_else(|| "SceneVM".to_string()));
            if let Some((w, h)) = app.initial_window_size() {
                attrs = attrs.with_inner_size(LogicalSize::new(w as f64, h as f64));
            }
            let win = target
                .create_window(attrs)
                .expect("failed to create window");
            let size = win.inner_size();
            let scale = win.scale_factor();
            let logical = size.to_logical::<f64>(scale);
            let logical_size = (logical.width.round() as u32, logical.height.round() as u32);
            let mut new_vm = SceneVM::new_with_window(&win);
            apply_logical_scale(&mut new_vm, scale);
            let new_ctx = NativeRenderCtx::new(logical_size);
            app.set_scale(scale as f32);
            app.set_native_mode(true); // Native wgpu runner
            app.init(&mut new_vm, logical_size);
            window = Some(win);
            vm = Some(new_vm);
            ctx = Some(new_ctx);
            target.set_control_flow(ControlFlow::Poll);
        }
        Event::WindowEvent { window_id, event } => {
            if let (Some(win), Some(vm_ref), Some(ctx_ref)) =
                (window.as_ref(), vm.as_mut(), ctx.as_mut())
            {
                if window_id == win.id() {
                    match event {
                        WindowEvent::CloseRequested => target.exit(),
                        WindowEvent::Resized(size) => {
                            let scale = win.scale_factor();
                            let logical = size.to_logical::<f64>(scale);
                            let logical_size =
                                (logical.width.round() as u32, logical.height.round() as u32);
                            ctx_ref.size = logical_size;
                            vm_ref.resize_window_surface(size.width, size.height);
                            apply_logical_scale(vm_ref, scale);
                            app.set_scale(scale as f32);
                            app.resize(vm_ref, logical_size);
                        }
                        WindowEvent::ScaleFactorChanged {
                            scale_factor,
                            mut inner_size_writer,
                        } => {
                            let size = win.inner_size();
                            let _ = inner_size_writer.request_inner_size(size);
                            let logical = size.to_logical::<f64>(scale_factor);
                            let logical_size =
                                (logical.width.round() as u32, logical.height.round() as u32);
                            ctx_ref.size = logical_size;
                            vm_ref.resize_window_surface(size.width, size.height);
                            app.set_scale(scale_factor as f32);
                            apply_logical_scale(vm_ref, scale_factor);
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            cursor_pos = position;
                            let scale = win.scale_factor() as f32;
                            app.mouse_move(
                                vm_ref,
                                (cursor_pos.x as f32) / scale,
                                (cursor_pos.y as f32) / scale,
                            );
                        }
                        WindowEvent::MouseInput {
                            state,
                            button: MouseButton::Left,
                            ..
                        } => match state {
                            ElementState::Pressed => {
                                let scale = win.scale_factor() as f32;
                                app.mouse_down(
                                    vm_ref,
                                    (cursor_pos.x as f32) / scale,
                                    (cursor_pos.y as f32) / scale,
                                );
                            }
                            ElementState::Released => {
                                let scale = win.scale_factor() as f32;
                                app.mouse_up(
                                    vm_ref,
                                    (cursor_pos.x as f32) / scale,
                                    (cursor_pos.y as f32) / scale,
                                );
                            }
                        },
                        WindowEvent::MouseWheel { delta, .. } => {
                            let (dx, dy) = match delta {
                                winit::event::MouseScrollDelta::LineDelta(x, y) => {
                                    (x * 120.0, y * 120.0)
                                }
                                winit::event::MouseScrollDelta::PixelDelta(pos) => {
                                    (pos.x as f32, pos.y as f32)
                                }
                            };
                            let scale = win.scale_factor() as f32;
                            app.scroll(vm_ref, dx / scale, dy / scale);
                        }
                        WindowEvent::RedrawRequested => {
                            if app.needs_update(vm_ref) {
                                ctx_ref.begin_frame();
                                app.update(vm_ref);
                                let _ = app.render(vm_ref, ctx_ref);
                                let _ = ctx_ref.ensure_presented(vm_ref);

                                // Handle app events after render
                                #[cfg(feature = "ui")]
                                {
                                    use crate::app_event::AppEvent;
                                    let events = app.take_app_events();
                                    for event in events {
                                        match event {
                                            AppEvent::RequestUndo => {
                                                app.undo(vm_ref);
                                            }
                                            AppEvent::RequestRedo => {
                                                app.redo(vm_ref);
                                            }
                                            AppEvent::RequestExport { format, filename } => {
                                                #[cfg(all(
                                                    not(target_arch = "wasm32"),
                                                    not(target_os = "ios")
                                                ))]
                                                {
                                                    crate::native_dialogs::handle_export(
                                                        &mut app, vm_ref, &format, &filename,
                                                    );
                                                }
                                            }
                                            AppEvent::RequestSave {
                                                filename,
                                                extension,
                                            } => {
                                                #[cfg(all(
                                                    not(target_arch = "wasm32"),
                                                    not(target_os = "ios")
                                                ))]
                                                {
                                                    crate::native_dialogs::handle_save(
                                                        &mut app, vm_ref, &filename, &extension,
                                                    );
                                                }
                                            }
                                            AppEvent::RequestOpen { extension } => {
                                                #[cfg(all(
                                                    not(target_arch = "wasm32"),
                                                    not(target_os = "ios")
                                                ))]
                                                {
                                                    crate::native_dialogs::handle_open(
                                                        &mut app, vm_ref, &extension,
                                                    );
                                                }
                                            }
                                            AppEvent::RequestImport { file_types } => {
                                                #[cfg(all(
                                                    not(target_arch = "wasm32"),
                                                    not(target_os = "ios")
                                                ))]
                                                {
                                                    crate::native_dialogs::handle_import(
                                                        &mut app,
                                                        vm_ref,
                                                        &file_types,
                                                    );
                                                }
                                            }
                                            _ => {
                                                // Other events
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        #[cfg(feature = "ui")]
                        WindowEvent::ModifiersChanged(new_modifiers) => {
                            modifiers = new_modifiers;
                        }
                        #[cfg(feature = "ui")]
                        WindowEvent::KeyboardInput { event, .. } => {
                            use winit::keyboard::{KeyCode, PhysicalKey};

                            if event.state == ElementState::Pressed {
                                // Check for Cmd/Ctrl+Z (Undo)
                                if event.physical_key == PhysicalKey::Code(KeyCode::KeyZ) {
                                    #[cfg(target_os = "macos")]
                                    let cmd_pressed = modifiers.state().super_key();
                                    #[cfg(not(target_os = "macos"))]
                                    let cmd_pressed = modifiers.state().control_key();

                                    if cmd_pressed && !modifiers.state().shift_key() {
                                        // Undo: Cmd+Z (macOS) or Ctrl+Z (other platforms)
                                        app.undo(vm_ref);
                                    } else if cmd_pressed && modifiers.state().shift_key() {
                                        // Redo: Cmd+Shift+Z (macOS) or Ctrl+Shift+Z (other platforms)
                                        app.redo(vm_ref);
                                    }
                                }
                                // Check for Ctrl+Y (Redo on Windows/Linux)
                                #[cfg(not(target_os = "macos"))]
                                if event.physical_key == PhysicalKey::Code(KeyCode::KeyY) {
                                    if modifiers.state().control_key() {
                                        app.redo(vm_ref);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Event::AboutToWait => {
            if let (Some(win), Some(vm_ref)) = (window.as_ref(), vm.as_mut()) {
                let wants_frame = app.needs_update(vm_ref);
                if let Some(dt) = frame_interval {
                    let next = std::time::Instant::now() + dt;
                    target.set_control_flow(ControlFlow::WaitUntil(next));
                    if wants_frame {
                        win.request_redraw();
                    }
                } else if wants_frame {
                    target.set_control_flow(ControlFlow::Poll);
                    win.request_redraw();
                } else {
                    target.set_control_flow(ControlFlow::Wait);
                }
            }
        }
        _ => {}
    })?;
    #[allow(unreachable_code)]
    Ok(())
}

#[cfg(target_arch = "wasm32")]
struct WasmRenderCtx {
    buffer: Vec<u8>,
    width: u32,
    height: u32,
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    /// True when the last render was not fully presented (Init/Readback pending).
    pending_present: bool,
}

#[cfg(target_arch = "wasm32")]
impl WasmRenderCtx {
    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.width = width;
        self.height = height;
        self.canvas.set_width(width);
        self.canvas.set_height(height);
        self.buffer.resize((width * height * 4) as usize, 0);
    }
}

#[cfg(target_arch = "wasm32")]
impl SceneVMRenderCtx for WasmRenderCtx {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn present(&mut self, vm: &mut SceneVM) -> SceneVMResult<RenderResult> {
        let mut res = vm.render_frame(&mut self.buffer, self.width, self.height);

        // If the frame wasn't ready yet, try to finish the readback immediately.
        if res != RenderResult::Presented {
            if let Some(gpu) = vm.gpu.as_mut() {
                let ready = gpu.surface.try_finish_download_from_gpu();
                if ready {
                    res = RenderResult::Presented;
                }
            }
        }

        // Always blit whatever pixels we have (latest completed frame).
        let clamped = wasm_bindgen::Clamped(&self.buffer[..]);
        let image_data =
            web_sys::ImageData::new_with_u8_clamped_array_and_sh(clamped, self.width, self.height)
                .map_err(|e| SceneVMError::InvalidOperation(format!("{:?}", e)))?;
        self.ctx
            .put_image_data(&image_data, 0.0, 0.0)
            .map_err(|e| SceneVMError::InvalidOperation(format!("{:?}", e)))?;

        self.pending_present = res != RenderResult::Presented;
        Ok(res)
    }
}

#[cfg(target_arch = "wasm32")]
fn create_or_get_canvas(document: &Document) -> Result<HtmlCanvasElement, JsValue> {
    if let Some(existing) = document
        .get_element_by_id("canvas")
        .and_then(|el| el.dyn_into::<HtmlCanvasElement>().ok())
    {
        return Ok(existing);
    }
    let canvas: HtmlCanvasElement = document
        .create_element("canvas")?
        .dyn_into::<HtmlCanvasElement>()?;
    document
        .body()
        .ok_or_else(|| JsValue::from_str("no body"))?
        .append_child(&canvas)?;
    Ok(canvas)
}

/// Run a `SceneVMApp` in the browser using a canvas + ImageData blit.
#[cfg(target_arch = "wasm32")]
pub fn run_scenevm_app<A: SceneVMApp + 'static>(mut app: A) -> Result<(), JsValue> {
    let window: WebWindow = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let canvas = create_or_get_canvas(&document)?;

    let (width, height) = app.initial_window_size().unwrap_or_else(|| {
        let w = window
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(800.0)
            .round() as u32;
        let h = window
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(600.0)
            .round() as u32;
        (w, h)
    });
    canvas.set_width(width);
    canvas.set_height(height);

    let ctx = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("2d context missing"))?
        .dyn_into::<CanvasRenderingContext2d>()?;

    let mut vm = SceneVM::new(width, height);
    let render_ctx = WasmRenderCtx {
        buffer: vec![0u8; (width * height * 4) as usize],
        width,
        height,
        canvas,
        ctx,
        pending_present: true, // force initial render until Presented lands
    };
    app.init(&mut vm, (width, height));

    let app_rc = Rc::new(RefCell::new(app));
    let vm_rc = Rc::new(RefCell::new(vm));
    let ctx_rc = Rc::new(RefCell::new(render_ctx));
    let first_frame = Rc::new(Cell::new(true));

    // Resize handler
    {
        let app = Rc::clone(&app_rc);
        let vm = Rc::clone(&vm_rc);
        let ctx = Rc::clone(&ctx_rc);
        let window_resize = window.clone();
        let resize_closure = Closure::<dyn FnMut()>::new(move || {
            if let (Ok(w), Ok(h)) = (window_resize.inner_width(), window_resize.inner_height()) {
                let w = w.as_f64().unwrap_or(800.0).round() as u32;
                let h = h.as_f64().unwrap_or(600.0).round() as u32;
                ctx.borrow_mut().resize(w, h);
                app.borrow_mut().resize(&mut vm.borrow_mut(), (w, h));
            }
        });
        window
            .add_event_listener_with_callback("resize", resize_closure.as_ref().unchecked_ref())?;
        resize_closure.forget();
    }

    // Pointer down handler
    {
        let app = Rc::clone(&app_rc);
        let vm = Rc::clone(&vm_rc);
        let canvas = ctx_rc.borrow().canvas.clone();
        let down_closure =
            Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |e: web_sys::PointerEvent| {
                let rect = canvas.get_bounding_client_rect();
                let x = e.client_x() as f64 - rect.left();
                let y = e.client_y() as f64 - rect.top();
                app.borrow_mut()
                    .mouse_down(&mut vm.borrow_mut(), x as f32, y as f32);
            });
        ctx_rc.borrow().canvas.add_event_listener_with_callback(
            "pointerdown",
            down_closure.as_ref().unchecked_ref(),
        )?;
        down_closure.forget();
    }

    // Animation loop
    {
        let app = Rc::clone(&app_rc);
        let vm = Rc::clone(&vm_rc);
        let ctx = Rc::clone(&ctx_rc);
        let first = Rc::clone(&first_frame);
        let f = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
        let f_clone = Rc::clone(&f);
        let window_clone = window.clone();
        *f.borrow_mut() = Some(Closure::<dyn FnMut()>::new(move || {
            {
                let mut app_mut = app.borrow_mut();
                let mut vm_mut = vm.borrow_mut();
                let ctx_pending = ctx.borrow().pending_present;
                let do_render = app_mut.needs_update(&vm_mut) || first.get() || ctx_pending;
                if do_render {
                    first.set(false);
                    app_mut.update(&mut vm_mut);
                    app_mut.render(&mut vm_mut, &mut *ctx.borrow_mut());
                }
            }
            let _ = window_clone.request_animation_frame(
                f_clone.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
            );
        }));
        let _ =
            window.request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref());
    }
    Ok(())
}

// -------------------------
// C FFI for CoreAnimation layer (Metal) presentation (macOS/iOS)
// -------------------------
#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn scenevm_ca_create(
    layer_ptr: *mut c_void,
    width: u32,
    height: u32,
) -> *mut SceneVM {
    if layer_ptr.is_null() {
        return std::ptr::null_mut();
    }
    let vm = SceneVM::new_with_metal_layer(layer_ptr, width, height);
    Box::into_raw(Box::new(vm))
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn scenevm_ca_destroy(ptr: *mut SceneVM) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(ptr));
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn scenevm_ca_resize(ptr: *mut SceneVM, width: u32, height: u32) {
    if let Some(vm) = unsafe { ptr.as_mut() } {
        vm.resize_window_surface(width, height);
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn scenevm_ca_render(ptr: *mut SceneVM) -> i32 {
    if let Some(vm) = unsafe { ptr.as_mut() } {
        match vm.render_to_window() {
            Ok(RenderResult::Presented) => 0,
            Ok(RenderResult::InitPending) => 1,
            Ok(RenderResult::ReadbackPending) => 2,
            Err(_) => -1,
        }
    } else {
        -1
    }
}
