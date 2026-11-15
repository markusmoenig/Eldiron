//! One compute dispatch per compatible disjoint triangle group. Existing waves
//! retain all overlap dependencies. Metadata shares the protection binding, so
//! the pipeline still fits WebGPU/Metal's eight storage-buffer limit.
use super::*;
const WORDS: usize = 44;
const _: () = assert!(std::mem::size_of::<Params>() == WORDS * 4);

pub(super) fn shader_source(batched: bool) -> String {
    let mut source = include_str!("../surface.wgsl").to_owned();
    if batched {
        source = source.replace("@group(0) @binding(0) var<uniform> p:SurfaceParams;", "");
        source = source.replace("fn raster(@builtin(global_invocation_id) id:vec3<u32>) {",
            "fn raster(@builtin(global_invocation_id) id:vec3<u32>) {\n    if(id.z>=batch.count){return;}\n    p=surface_params(protection[batch.first+id.z]);");
        source = source.replace(
            "protection[p.contact.y+previous]",
            "protection[batch.masks+p.contact.y+previous]",
        );
        source.push_str(include_str!("batch.wgsl"));
    }
    source.push_str(include_str!("../material_sample.wgsl"));
    source.push_str(include_str!("../paint_sample.wgsl"));
    source
}
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct Key {
    source: usize,
    secondary: usize,
    paint: usize,
    width: u32,
    height: u32,
}
struct Dispatch {
    triangle: usize,
    first: u32,
    count: u32,
    groups: [u32; 3],
}

pub(super) fn append(
    renderer: &mut IsometricSurfaceRenderer,
    canvas: &IsometricCanvas,
    triangles: &[SurfaceTriangle<'_>],
    params: &[u8],
    masks: &[u32],
    regions: &[[i32; 4]],
    waves: &[Vec<usize>],
) -> Result<bool> {
    let limit = u64::from(
        renderer
            .context
            .device
            .limits()
            .max_storage_buffer_binding_size,
    )
    .min(renderer.context.device.limits().max_buffer_size);
    let active = waves.iter().map(Vec::len).sum::<usize>();
    let words = triangles
        .len()
        .checked_mul(WORDS)
        .and_then(|n| n.checked_add(masks.len()))
        .and_then(|n| n.checked_add(active));
    if words.is_none_or(|n| n as u64 * 4 > limit) {
        return Ok(false);
    }
    if active == 0 {
        return Ok(true);
    }
    let mut storage = Vec::<u32>::with_capacity(words.unwrap());
    for index in 0..triangles.len() {
        let start = index * renderer.stride as usize;
        storage.extend(
            params[start..start + WORDS * 4]
                .chunks_exact(4)
                .map(|b| u32::from_ne_bytes(b.try_into().unwrap())),
        );
    }
    let mask_base = storage.len() as u32;
    storage.extend_from_slice(masks);
    let max_groups = renderer
        .context
        .device
        .limits()
        .max_compute_workgroups_per_dimension;
    let mut dispatches = Vec::new();
    let mut ranges = Vec::with_capacity(waves.len());
    for wave in waves {
        let mut lookup = FxHashMap::default();
        let mut groups = Vec::<(Key, Vec<usize>)>::new();
        for &index in wave {
            let t = &triangles[index];
            let r = regions[index];
            let key = Key {
                source: t.source as *const _ as usize,
                secondary: t.secondary.map_or(t.source, |(s, _)| s) as *const _ as usize,
                paint: t.paint.map_or(0, |p| p as *const _ as usize),
                width: ((r[2] - r[0]) as u32).div_ceil(8).next_power_of_two(),
                height: ((r[3] - r[1]) as u32).div_ceil(8).next_power_of_two(),
            };
            // Bucket dimensions bound unused invocations to less than four
            // times individual padded dispatches, even with mixed extents.
            let slot = *lookup.entry(key).or_insert_with(|| {
                let i = groups.len();
                groups.push((key, Vec::new()));
                i
            });
            groups[slot].1.push(index);
        }
        let start = dispatches.len();
        for (key, indices) in groups {
            for indices in indices.chunks(max_groups as usize) {
                let first = storage.len() as u32;
                storage.extend(indices.iter().map(|i| *i as u32));
                dispatches.push(Dispatch {
                    triangle: indices[0],
                    first,
                    count: indices.len() as u32,
                    groups: [key.width, key.height, indices.len() as u32],
                });
            }
        }
        ranges.push(start..dispatches.len());
    }
    let uniform_bytes = dispatches.len() as u64 * renderer.stride;
    let mut headers = vec![0u8; uniform_bytes as usize];
    for (index, dispatch) in dispatches.iter().enumerate() {
        let at = index * renderer.stride as usize;
        headers[at..at + 16].copy_from_slice(bytemuck::bytes_of(&[
            dispatch.first,
            mask_base,
            dispatch.count,
            0u32,
        ]));
    }
    let metadata = bytemuck::cast_slice(&storage);
    if uniform_bytes > renderer.capacity {
        renderer.params = IsometricSurfaceRenderer::buffer(&renderer.context, uniform_bytes, true)?;
        renderer.capacity = uniform_bytes;
    }
    if metadata.len() as u64 > renderer.mask_capacity {
        renderer.masks =
            IsometricSurfaceRenderer::buffer(&renderer.context, metadata.len() as u64, false)?;
        renderer.mask_capacity = metadata.len() as u64;
    }
    renderer
        .context
        .queue
        .write_buffer(renderer.params.buffer(), 0, &headers);
    renderer.context.record_upload(headers.len() as u64);
    renderer
        .context
        .queue
        .write_buffer(renderer.masks.buffer(), 0, metadata);
    renderer.context.record_upload(metadata.len() as u64);
    let bind = super::bindings::canvas(renderer, canvas);
    let (pairs, paints) = super::bindings::sources(renderer, triangles);
    let mut encoder =
        renderkit::OrderedCompute::new(&renderer.context, "iso-batched-surface-raster");
    {
        // Compute usage scopes are per dispatch, not per pass. WGPU inserts
        // storage barriers before each conflicting dispatch, including when
        // bindings are unchanged. Retain every wave's order without creating
        // hundreds of separate backend compute encoders for that ordering.
        let mut pass = encoder.begin_pass();
        pass.set_pipeline(&renderer.batch_pipeline);
        for index in ranges.into_iter().flatten() {
            let d = &dispatches[index];
            let t = &triangles[d.triangle];
            let other = t.secondary.map_or(t.source, |(s, _)| s);
            let key = (t.source as *const _ as usize, other as *const _ as usize);
            pass.set_bind_group(0, &bind, &[(index as u64 * renderer.stride) as u32]);
            pass.set_bind_group(1, &pairs[&key], &[]);
            pass.set_bind_group(
                2,
                t.paint.map_or(&renderer.empty_paint, |p| {
                    &paints[&(p as *const _ as usize)]
                }),
                &[],
            );
            pass.dispatch_workgroups(d.groups[0], d.groups[1], d.groups[2]);
        }
    }
    encoder.finish();
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("ELDIRON_PROFILE_ISOMETRIC").is_some() {
        eprintln!(
            "Isometric batched surfaces: 1 compute pass, {} dependency waves, {} dispatches, {} active triangles, {} metadata bytes, {} uniform bytes",
            waves.len(),
            dispatches.len(),
            active,
            metadata.len(),
            headers.len()
        );
    }
    Ok(true)
}
