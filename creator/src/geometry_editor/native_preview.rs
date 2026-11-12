//! GPU-only geometry preview output. CPU preparation/pose stays in the editor;
//! outputs are immutable leases and native overlays use the shared paint list.
use super::{
    gpu_textures::GpuTextures,
    preview::{Job, Renderer, Request},
    scene::Projection,
};
use denrim_materials::geometry::rig::Pose;
use renderkit::{GpuContext, GpuImage, ImageDescriptor};
use std::sync::Arc;
use theframework::paint::*;
const MAX_TARGETS: usize = 3;
const TARGET_BYTES: u64 = 128 * 1024 * 1024;
pub(super) struct Frame {
    pub epoch: u64,
    pub revision: u64,
    pub image: Arc<GpuImage>,
    pub projection: Projection,
    pub posed: Vec<Pose>,
    pub overlay: PaintList,
    pub logical_size: [f32; 2],
}
impl Frame {
    /// The caller registers this frame's image in its own registry and keeps
    /// that handle/lease alive through presentation and any retained paint.
    pub fn paint(&self, image: ImageHandle) -> PaintList {
        let mut list = PaintList::default();
        list.image(
            image,
            Rect::new(0., 0., self.logical_size[0], self.logical_size[1]),
            Sampling::Nearest,
        );
        list.ops.extend(self.overlay.ops.iter().cloned());
        list
    }
}
pub(super) struct NativePreview {
    context: Arc<GpuContext>,
    mesh: Renderer,
    materials: GpuTextures,
    targets: Vec<Arc<GpuImage>>,
    depth: Option<(u32, u32, wgpu::TextureView)>,
    pending: Option<((u64, u64, u32), Frame)>,
}
impl NativePreview {
    pub fn new(context: Arc<GpuContext>) -> Self {
        Self {
            mesh: Renderer::new(&context.device),
            materials: GpuTextures::new(context.clone()),
            context,
            targets: Vec::new(),
            depth: None,
            pending: None,
        }
    }
    pub fn generation(&self) -> u64 {
        self.context.generation()
    }
    /// None applies backpressure when every bounded output is still leased.
    /// No published image is overwritten, including an unacknowledged frame.
    pub fn render(&mut self, request: &Request, density: f32) -> Result<Option<Frame>, String> {
        let job = &request.job;
        if self.context.is_lost() {
            self.pending = None;
            return Err("Geometry preview device was lost".into());
        }
        let key = (request.epoch, request.job.revision, density.to_bits());
        if self
            .pending
            .as_ref()
            .is_some_and(|(old, _)| old.0 != key.0 || old.2 != key.2)
        {
            self.pending = None;
        }
        if self.pending.is_some() {
            match self.materials.status() {
                Ok(true) => return Ok(self.pending.take().map(|(_, frame)| frame)),
                Ok(false) => return Ok(None),
                Err(error) => {
                    self.pending = None;
                    return Err(error);
                }
            }
        }
        if job.width < 16 || job.height < 16 {
            return Ok(None);
        }
        if !density.is_finite() || density <= 0. || !job.yaw.is_finite() || !job.pitch.is_finite() {
            return Err("Invalid geometry preview scale/camera".into());
        }
        if job.mesh.parts.iter().flat_map(|p| &p.vertices).any(|v| {
            v.position
                .iter()
                .chain(&v.normal)
                .chain(&v.uv)
                .any(|n| !n.is_finite())
        }) || job.framing.is_some_and(|(a, b)| {
            (0..3).any(|i| !a[i].is_finite() || !b[i].is_finite() || a[i] > b[i])
        }) {
            return Err("Invalid geometry preview mesh/bounds".into());
        }
        let physical = [
            job.width as f64 * density as f64,
            job.height as f64 * density as f64,
        ]
        .map(f64::round);
        if physical
            .iter()
            .any(|v| *v < 1. || *v > self.context.device.limits().max_texture_dimension_2d as f64)
        {
            return Err("Geometry preview exceeds GPU texture limits".into());
        }
        let [w, h] = physical.map(|v| v as u32);
        let bytes = w as u64 * h as u64 * 4;
        if bytes * 3 > TARGET_BYTES {
            return Err("Geometry preview exceeds the double-buffer/depth budget".into());
        }
        self.targets.retain(|image| {
            Arc::strong_count(image) > 1
                || (image.descriptor.width == w && image.descriptor.height == h)
        });
        let reusable = self
            .targets
            .iter()
            .find(|image| {
                Arc::strong_count(image) == 1
                    && image.descriptor.width == w
                    && image.descriptor.height == h
            })
            .cloned();
        let image = if let Some(image) = reusable {
            image
        } else {
            let resident: u64 = self
                .targets
                .iter()
                .map(|i| i.descriptor.width as u64 * i.descriptor.height as u64 * 4)
                .sum();
            if self.targets.len() >= MAX_TARGETS || resident + bytes * 2 > TARGET_BYTES {
                return Ok(None);
            }
            let image = GpuImage::new(&self.context, ImageDescriptor::rgba(w, h))
                .map_err(|e| e.to_string())?;
            self.targets.push(image.clone());
            image
        };
        if self.depth.as_ref().is_none_or(|d| d.0 != w || d.1 != h) {
            let depth = self
                .context
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("native geometry depth"),
                    size: wgpu::Extent3d {
                        width: w,
                        height: h,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Depth32Float,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&Default::default());
            self.depth = Some((w, h, depth));
        }
        let mut specs = Vec::new();
        for part in &job.mesh.parts {
            if let Some(texture) = &part.texture {
                if !specs.contains(&texture) {
                    specs.push(texture);
                }
            }
        }
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("native geometry preview"),
                });
        let maps =
            self.materials
                .encode(&mut encoder, &specs, &job.palette, job.surface_seconds)?;
        let bindings = self.mesh.material_bindings(&self.context.device, &maps);
        let prepared = self.mesh.prepare_mesh(
            &self.context.device,
            &self.context.queue,
            job,
            Some(bindings),
        )?;
        self.mesh.encode_mesh(
            &mut encoder,
            &image.view,
            &self.depth.as_ref().unwrap().2,
            &prepared,
            job.background,
        );
        let overlay = overlays(job, &prepared.projection, prepared.extent);
        overlay.validate().map_err(str::to_owned)?;
        self.context.queue.submit([encoder.finish()]);
        self.materials.submitted();
        let frame = Frame {
            epoch: request.epoch,
            revision: job.revision,
            image,
            projection: prepared.projection,
            posed: request.posed.clone(),
            overlay,
            logical_size: [job.width as f32, job.height as f32],
        };
        if self.materials.status()? {
            Ok(Some(frame))
        } else {
            self.pending = Some((key, frame));
            Ok(None)
        }
    }
}
fn overlays(job: &Job, projection: &Projection, extent: f32) -> PaintList {
    let mut list = PaintList::default();
    let mut line = |a, b, color: [u8; 4], width| {
        list.ops.push(PaintOp::Stroke {
            path: Arc::new(Path {
                elements: vec![
                    PathElement::Move(projection.project(a)),
                    PathElement::Line(projection.project(b)),
                ],
            }),
            stroke: Stroke {
                width,
                cap: LineCap::Butt,
                join: LineJoin::Miter,
                brush: Brush::Solid(color.map(|v| v as f32 / 255.)),
            },
        });
    };
    for (a, b) in &job.bones {
        line(*a, *b, job.axes[0], 3.);
    }
    let mut frames = job.attachment_frames.clone();
    if let Some(grip) = &job.grip {
        frames.push(Pose::from_frame(grip));
    }
    for frame in frames {
        for axis in 0..3 {
            let mut p = [0.; 3];
            p[axis] = extent * 0.18;
            line(frame.translation, frame.point(p), job.axes[axis], 1.);
        }
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;
    use denrim_materials::geometry::{Mesh, Part, Surface, Vertex};
    use renderkit::{ImageRegistry, ReadbackTicket};
    use theframework::{prelude::*, render_wgpu::PaintRenderer};
    fn request(textured: bool) -> Request {
        let mesh = Mesh {
            parts: vec![Part {
                id: "triangle".into(),
                label: "Triangle".into(),
                vertices: [[-0.8, -0.6, 0.], [0.8, -0.6, 0.], [0., 0.7, 0.]]
                    .into_iter()
                    .enumerate()
                    .map(|(i, position)| Vertex {
                        position,
                        normal: [0., 0., 1.],
                        uv: [[0., 1.], [1., 1.], [0.5, 0.]][i],
                        skin: None,
                    })
                    .collect(),
                surface: Some(Surface {
                    palette: 7,
                    roughness: 0.5,
                    metallic: 0.,
                }),
                texture: textured.then(|| super::super::gpu_textures::tests::material(16)),
            }],
            ..Default::default()
        };
        let bounds = mesh.bounds();
        Request {
            epoch: 71,
            projection: Projection::new(bounds, 96, 64, 17., -11.),
            posed: vec![],
            job: Job {
                revision: 1,
                surface_seconds: 0.8,
                mesh,
                palette: ThePalette::default(),
                width: 96,
                height: 64,
                yaw: 17.,
                pitch: -11.,
                background: [30, 41, 52, 255],
                grip: None,
                axes: [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]],
                framing: Some(bounds),
                bones: vec![],
                attachment_frames: vec![],
            },
        }
    }
    fn capture(context: &GpuContext, image: Arc<GpuImage>) -> Vec<u8> {
        let mut ticket = ReadbackTicket::capture(context, image).unwrap();
        context
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        ticket
            .try_finish(context)
            .unwrap()
            .unwrap()
            .pixels()
            .to_vec()
    }
    #[test]
    #[ignore = "requires actual native GPU"]
    fn native_geometry_pending_materials_publish_compatible_frames_when_animation_advances() {
        let context = pollster::block_on(GpuContext::headless()).unwrap();
        let mut preview = NativePreview::new(context.clone());
        let mut request = request(true);
        assert!(preview.render(&request, 1.).unwrap().is_none());
        context
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        request.job.revision += 1;
        request.job.yaw = 43.;
        request.job.surface_seconds = 1.5;
        let completed = preview
            .render(&request, 1.)
            .unwrap()
            .expect("a newer animation request must not discard a completed compatible output");
        assert_eq!(completed.revision, 1);
        assert_eq!(
            completed.projection.project([0.3, 0.1, 0.2]),
            request.projection.project([0.3, 0.1, 0.2])
        );
        assert!(preview.render(&request, 1.).unwrap().is_none());
        context
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        let current = preview.render(&request, 1.).unwrap().unwrap();
        assert_eq!(current.revision, 2);
        assert_ne!(
            current.projection.project([0.3, 0.1, 0.2]),
            completed.projection.project([0.3, 0.1, 0.2])
        );
    }
    #[test]
    #[ignore = "requires actual native GPU"]
    fn native_geometry_outputs_match_reference_and_publish_without_framebuffer_readback() {
        let context = pollster::block_on(GpuContext::headless()).unwrap();
        let mut view = NativePreview::new(context.clone());
        let reference = Renderer::new(&context.device);
        for textured in [false, true] {
            let request = request(textured);
            let first = view.render(&request, 1.).unwrap();
            if textured {
                assert!(
                    first.is_none(),
                    "unchecked material status must withhold publication"
                );
                assert!(view.pending.is_some());
            }
            let frame = if let Some(frame) = first {
                frame
            } else {
                context
                    .device
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: Some(std::time::Duration::from_secs(30)),
                    })
                    .unwrap();
                view.render(&request, 1.)
                    .unwrap()
                    .expect("completed diagnostics must release the held output")
            };
            assert_eq!((frame.epoch, frame.revision), (71, 1));
            assert_eq!(
                frame.projection.project([0.; 3]),
                request.projection.project([0.; 3])
            );
            assert!(
                !view.mesh.has_capture_targets(),
                "native rendering must not allocate reference framebuffer staging"
            );
            let actual = capture(&context, frame.image.clone());
            let expected = reference
                .render(&context.device, &context.queue, &request.job)
                .unwrap();
            assert_eq!(actual, expected.pixels(), "textured={textured}");
            // Publish through the actual image registry and render-view paint
            // contract, with no CPU canvas or viewport bitmap involved.
            let mut registry = ImageRegistry::new(context.clone());
            let handle = registry.insert(frame.image.clone()).unwrap();
            let mut widget = TheRenderView::new(TheId::named("Native preview"));
            widget
                .as_render_view()
                .unwrap()
                .set_paint_list(Some(Arc::new(frame.paint(handle))))
                .unwrap();
            let target = GpuImage::new(&context, ImageDescriptor::rgba(144, 96)).unwrap();
            let mut painter = PaintRenderer::new(context.clone()).unwrap();
            let before = context.transfers();
            painter
                .render(
                    &frame.paint(handle),
                    &registry,
                    &target.view,
                    target.descriptor.format,
                    (144, 96),
                    1.5,
                    [0.; 4],
                )
                .unwrap();
            assert_eq!(
                context.transfers(),
                before,
                "GPU preview composition may not upload/read a viewport bitmap"
            );
            registry.remove(handle);
        }
        let mut request = request(false);
        let old = view.render(&request, 1.).unwrap().unwrap();
        let old_pixels = capture(&context, old.image.clone());
        request.job.yaw = 43.;
        request.job.revision += 1;
        let second = view.render(&request, 1.).unwrap().unwrap();
        let third = view.render(&request, 1.).unwrap().unwrap();
        assert!(
            view.render(&request, 1.).unwrap().is_none(),
            "leased outputs apply backpressure instead of overwriting displayed frames"
        );
        assert_eq!(capture(&context, old.image.clone()), old_pixels);
        drop(second);
        drop(third);
        let scaled = view.render(&request, 1.5).unwrap().unwrap();
        assert_eq!(
            (
                scaled.image.descriptor.width,
                scaled.image.descriptor.height
            ),
            (144, 96)
        );
        assert_eq!(
            scaled.projection.project([0.; 3]),
            Projection::new(request.job.framing.unwrap(), 96, 64, 43., -11.).project([0.; 3])
        );
        assert!(view.render(&request, f32::NAN).is_err());
        request.job.width = 0;
        assert!(view.render(&request, 1.).unwrap().is_none());
    }
}
