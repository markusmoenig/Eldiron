//! Mutable scene targets are private work surfaces. The compositor samples a
//! separate image, updated only after every phase of a scene frame completes.
use super::*;
use renderkit::wgpu;

pub(super) fn publish(
    context: &GpuContext,
    source: &Arc<renderkit::GpuImage>,
    previous: Option<&Arc<renderkit::GpuImage>>,
) -> Result<Arc<renderkit::GpuImage>> {
    let image = match previous {
        Some(image) if image.descriptor == source.descriptor => image.clone(),
        _ => renderkit::GpuImage::new(context, source.descriptor)?,
    };
    let mut encoder = context
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("publish complete scene image"),
        });
    encoder.copy_texture_to_texture(
        source.texture.as_image_copy(),
        image.texture.as_image_copy(),
        wgpu::Extent3d {
            width: source.descriptor.width,
            height: source.descriptor.height,
            depth_or_array_layers: 1,
        },
    );
    context.queue.submit([encoder.finish()]);
    Ok(image)
}
