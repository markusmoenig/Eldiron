//! Logical isometric pixels are composed once on the application GPU, then
//! scaled as one image. No CPU destination, readback, or second device exists.
use super::*;
use renderkit::{
    AlphaMode, ColorSpace, GpuImage, ImageDescriptor, ImageHandle, ImageRegistry, wgpu,
};
use theframework::{
    paint::{Brush, DisplayCompositing},
    render_wgpu::PaintRenderer,
};

#[path = "native_world_pixel_alpha.rs"]
mod alpha;
#[cfg(test)]
#[path = "native_world_pixel_guides.rs"]
mod guide_tests;

#[derive(PartialEq)]
enum SourceOp {
    Clear(Rect, [f32; 4]),
    Sprite(ImageHandle, [f32; 2], [f32; 2], [f32; 4]),
    Line([f32; 2], [f32; 2], [f32; 4]),
}
impl SourceOp {
    fn from(op: &PaintOp) -> Result<Self, String> {
        Ok(match op {
            PaintOp::Rect {
                rect,
                radius,
                brush: Brush::Solid(color),
            } if *radius == 0. => Self::Clear(*rect, *color),
            PaintOp::SpriteImage {
                image,
                origin,
                scale,
                tint,
            } => Self::Sprite(*image, *origin, *scale, *tint),
            PaintOp::SampledLine { from, to, color } => Self::Line(*from, *to, *color),
            _ => return Err("unsupported logical isometric source paint".into()),
        })
    }
}
struct Cached {
    output: ViewImage,
    size: (usize, usize),
    ops: Vec<SourceOp>,
    image: ImageHandle,
    density: u32,
    #[cfg(test)]
    source_draws: usize,
}
#[derive(Default)]
pub(super) struct Gpu {
    renderer: Option<PaintRenderer>,
    cached: Option<Cached>,
    alpha: Option<alpha::Composer>,
}
impl Gpu {
    #[cfg(test)]
    fn guide_proof(&self) -> Option<(ImageHandle, usize, usize)> {
        let cached = self.cached.as_ref()?;
        Some((
            cached.image,
            cached
                .ops
                .iter()
                .filter(|op| matches!(op, SourceOp::Line(..)))
                .count(),
            cached.source_draws,
        ))
    }
    pub(super) fn clear(&mut self, images: &mut ImageRegistry) {
        if let Some(old) = self.cached.take() {
            images.remove(old.image);
        }
        self.renderer = None;
        self.alpha = None;
    }
    pub(super) fn compose(
        &mut self,
        mut list: PaintList,
        output: ViewImage,
        size: (usize, usize),
        density: f32,
        images: &mut ImageRegistry,
    ) -> Result<PaintList, String> {
        let ops = list
            .ops
            .iter()
            .map(SourceOp::from)
            .collect::<Result<Vec<_>, _>>()?;
        let image = if let Some(cached) = &self.cached
            && cached.output == output
            && cached.size == size
            && cached.density == density.to_bits()
            && cached.ops == ops
            && images.get(cached.image).is_some()
        {
            cached.image
        } else {
            let descriptor = ImageDescriptor {
                width: u32::try_from(size.0).map_err(|_| "logical isometric width overflow")?,
                height: u32::try_from(size.1).map_err(|_| "logical isometric height overflow")?,
                format: wgpu::TextureFormat::Rgba8Unorm,
                color_space: ColorSpace::Srgb,
                alpha_mode: AlphaMode::Straight,
            };
            let image =
                GpuImage::render_target(images.context(), descriptor).map_err(|e| e.to_string())?;
            if self.renderer.is_none() {
                self.renderer =
                    Some(PaintRenderer::new(images.context().clone()).map_err(|e| e.to_string())?);
            }
            list.ops.insert(
                0,
                PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
            );
            // The CPU guide routine blends stored RGB regardless of its prior
            // alpha, and makes touched pixels opaque. Preserve hidden source
            // RGB until guides are painted, then flatten the finished source.
            let source = images
                .get(output.image)
                .ok_or("isometric alpha source missing")?;
            let opaque = images
                .insert(source.with_alpha_mode(AlphaMode::Opaque))
                .map_err(|e| e.to_string())?;
            for op in &mut list.ops {
                if let PaintOp::SpriteImage { image, .. } = op
                    && *image == output.image
                {
                    *image = opaque;
                }
            }
            let result = self
                .renderer
                .as_mut()
                .unwrap()
                .render(
                    &list,
                    images,
                    &image.view,
                    descriptor.format,
                    (descriptor.width, descriptor.height),
                    1.,
                    [0., 0., 0., 1.],
                )
                .map_err(|e| e.to_string());
            images.remove(opaque);
            result?;
            #[cfg(test)]
            let source_draws = self.renderer.as_ref().unwrap().frame_stats().draw_calls;
            if self.alpha.is_none() {
                self.alpha = Some(
                    alpha::Composer::new(images.context().clone()).map_err(|e| e.to_string())?,
                );
            }
            let image = self
                .alpha
                .as_mut()
                .unwrap()
                .compose(&list, &source, image, images)?;
            // The reference first expands the logical RenderView into its local
            // canvas, then copies that canvas at a rounded parent origin. At
            // fractional DPI those two integer grids may differ by one pixel.
            // Preserve both left-edge lookups entirely on GPU. Integral DPI is
            // equivalent to a single lookup and needs no intermediate target.
            let image = if density.fract() != 0. {
                let physical = ImageDescriptor {
                    width: (size.0 as f64 * f64::from(density)).round() as u32,
                    height: (size.1 as f64 * f64::from(density)).round() as u32,
                    ..descriptor
                };
                let source = images.insert(image).map_err(|e| e.to_string())?;
                let result = (|| {
                    let target = GpuImage::render_target(images.context(), physical)
                        .map_err(|e| e.to_string())?;
                    let scale = PaintList {
                        ops: vec![
                            PaintOp::DisplayCompositing(DisplayCompositing::LegacyOpaque),
                            PaintOp::PixelImage {
                                image: source,
                                rect: Rect::new(0., 0., size.0 as f32, size.1 as f32),
                                tint: [1.; 4],
                            },
                        ],
                    };
                    self.renderer
                        .as_mut()
                        .unwrap()
                        .render(
                            &scale,
                            images,
                            &target.view,
                            physical.format,
                            (physical.width, physical.height),
                            density,
                            [0., 0., 0., 1.],
                        )
                        .map_err(|e| e.to_string())?;
                    Ok::<_, String>(target)
                })();
                images.remove(source);
                result?
            } else {
                image
            };
            let image = images.insert(image).map_err(|e| e.to_string())?;
            if let Some(old) = self.cached.replace(Cached {
                output,
                size,
                ops,
                image,
                density: density.to_bits(),
                #[cfg(test)]
                source_draws,
            }) {
                images.remove(old.image);
            }
            image
        };
        Ok(PaintList {
            ops: vec![PaintOp::PixelImage {
                image,
                rect: Rect::new(0., 0., size.0 as f32, size.1 as f32),
                tint: [1.; 4],
            }],
        })
    }
}

#[cfg(test)]
impl Editor {
    pub(crate) fn native_world_logical_guides_for_test(
        &self,
    ) -> Option<(ImageHandle, usize, usize)> {
        self.native_viewport
            .world
            .gpu
            .as_ref()?
            .pixels
            .guide_proof()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "native GPU; logical isometric source ownership"]
    fn logical_isometric_layer_reuses_exact_source_and_keeps_old_gpu_pixels_immutable() {
        let context = futures::executor::block_on(renderkit::GpuContext::headless()).unwrap();
        let mut images = ImageRegistry::new(context.clone());
        let source = images
            .upload_linear(
                &renderkit::ImageAsset::rgba(17, 11, [40, 70, 100, 255].repeat(17 * 11)).unwrap(),
            )
            .unwrap();
        let descriptor = images.get(source).unwrap().descriptor;
        let mut output = ViewImage {
            view: ViewId::new(),
            image: source,
            descriptor,
            revision: 1,
            content_revision: 1,
        };
        let paint = |green: bool| {
            let mut list = PaintList::default();
            list.ops.push(PaintOp::SpriteImage {
                image: source,
                origin: [0.; 2],
                scale: [1.; 2],
                tint: [1.; 4],
            });
            list.ops.push(PaintOp::SampledLine {
                from: [1., 2.],
                to: [14., 8.],
                color: if green {
                    [0., 1., 0., 1.]
                } else {
                    [1., 0., 0., 1.]
                },
            });
            list
        };
        let mut layer = Gpu::default();
        let first = layer
            .compose(paint(false), output, (17, 11), 1.5, &mut images)
            .unwrap();
        let handle = first.image_handles().next().unwrap();
        let old = images.get(handle).unwrap();
        let transfers = context.transfers();
        for _ in 0..30 {
            assert_eq!(
                layer
                    .compose(paint(false), output, (17, 11), 1.5, &mut images)
                    .unwrap()
                    .image_handles()
                    .next(),
                Some(handle)
            );
        }
        assert_eq!(context.transfers(), transfers);
        let changed = layer
            .compose(paint(true), output, (17, 11), 1.5, &mut images)
            .unwrap()
            .image_handles()
            .next()
            .unwrap();
        assert_ne!(handle, changed);
        assert!(images.get(handle).is_none());
        let current = images.get(changed).unwrap();
        output.revision += 1;
        let rerendered = layer
            .compose(paint(true), output, (17, 11), 1.5, &mut images)
            .unwrap()
            .image_handles()
            .next()
            .unwrap();
        assert_ne!(
            changed, rerendered,
            "producer revision matters even with stable image handle and guide commands"
        );
        assert_eq!(context.transfers().readback_bytes, 0);
        let mut before = renderkit::ReadbackTicket::capture(&context, old).unwrap();
        let mut after = renderkit::ReadbackTicket::capture(&context, current).unwrap();
        context
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        let before = before.try_finish(&context).unwrap().unwrap();
        let after = after.try_finish(&context).unwrap().unwrap();
        assert!(
            before
                .pixels()
                .chunks_exact(4)
                .any(|p| p == [255, 0, 0, 255])
        );
        assert!(
            after
                .pixels()
                .chunks_exact(4)
                .any(|p| p == [0, 255, 0, 255])
        );
        assert!(
            !before
                .pixels()
                .chunks_exact(4)
                .any(|p| p == [0, 255, 0, 255])
        );
        layer.clear(&mut images);
        assert!(images.get(rerendered).is_none());
        let resized = layer
            .compose(paint(true), output, (17, 11), 2., &mut images)
            .unwrap()
            .image_handles()
            .next()
            .unwrap();
        let image = images.get(resized).unwrap();
        assert_eq!(
            (image.descriptor.width, image.descriptor.height),
            (17, 11),
            "integer density uses equivalent single-stage logical pixels"
        );
        let fractional = layer
            .compose(paint(true), output, (17, 11), 1.25, &mut images)
            .unwrap()
            .image_handles()
            .next()
            .unwrap();
        assert_ne!(resized, fractional);
        let image = images.get(fractional).unwrap();
        assert_eq!((image.descriptor.width, image.descriptor.height), (21, 14));
        layer.clear(&mut images);
        assert!(images.get(resized).is_none() && images.get(fractional).is_none());
        assert!(images.get(source).is_some());
        assert!(layer.cached.is_none() && layer.renderer.is_none());
    }
}
