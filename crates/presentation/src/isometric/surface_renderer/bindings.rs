//! Shared GPU bindings for individual and batched surface dispatches.
use super::*;
pub(super) fn canvas(
    renderer: &IsometricSurfaceRenderer,
    canvas: &IsometricCanvas,
) -> wgpu::BindGroup {
    let bind = renderer
        .context
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("iso-surface-canvas"),
            layout: &renderer.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: renderer.params.buffer(),
                        offset: 0,
                        size: NonZeroU64::new(std::mem::size_of::<Params>() as u64),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: canvas.albedo().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: canvas.surfaces().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: canvas.owners().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: canvas.terrain().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: renderer.masks.buffer().as_entire_binding(),
                },
            ],
        });

    bind
}
pub(super) fn sources(
    renderer: &IsometricSurfaceRenderer,
    triangles: &[SurfaceTriangle<'_>],
) -> (
    FxHashMap<(usize, usize), wgpu::BindGroup>,
    FxHashMap<usize, wgpu::BindGroup>,
) {
    let mut pairs = FxHashMap::default();
    for t in triangles {
        let other = t.secondary.map_or(t.source, |(source, _)| source);
        let key = (t.source as *const _ as usize, other as *const _ as usize);
        pairs.entry(key).or_insert_with(|| {
            renderer
                .context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("iso-surface-sources"),
                    layout: &renderer.source_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: t.source.buffer().as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: other.buffer().as_entire_binding(),
                        },
                    ],
                })
        });
    }
    let mut paints = FxHashMap::default();
    for source in triangles.iter().filter_map(|t| t.paint) {
        paints
            .entry(source as *const _ as usize)
            .or_insert_with(|| {
                renderer
                    .context
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("iso-surface-paint"),
                        layout: &renderer.paint_layout,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: source.buffer().as_entire_binding(),
                        }],
                    })
            });
    }

    (pairs, paints)
}
