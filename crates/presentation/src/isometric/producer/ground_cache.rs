//! Preserve deferred terrain before upright sprites and live lighting. Source
//! anchors are weak: cached pixels must not pin oversized material publications.
use super::*;
use rustc_hash::FxHashSet;
use std::sync::Weak;

const GPU_LIMIT: u64 = 32 * 1024 * 1024;
const METADATA_LIMIT: u64 = 64 * 1024 * 1024;
// Bounds signatures, weak control blocks and temporary source-deduplication
// tables together, including the old and replacement key while reusing storage.
const METADATA_PER_TRIANGLE: u64 = 640;
const METADATA_BASE: u64 = 4096;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GroundCacheUsage {
    pub hits: u64,
    pub misses: u64,
    pub gpu_bytes: u64,
    pub reserved_bytes: u64,
}
#[derive(PartialEq, Eq)]
struct SurfaceKey {
    vertices: [[u32; 3]; 3],
    source: usize,
    secondary: Option<(usize, [u32; 4])>,
    paint: Option<usize>,
    kind: SurfaceKind,
    repeat: bool,
    bounds: [u32; 4],
    cell: Option<[i32; 2]>,
}
// Vec growth, weak source control blocks and simultaneous hash-table growth.
// Keep the bound checked when source/signature layouts change on any target.
const _: () = {
    let material = std::mem::size_of::<GpuMaterialSource>();
    let paint = std::mem::size_of::<GpuTerrainPaintSource>();
    let source = if material > paint { material } else { paint };
    let per_source = 2 * std::mem::size_of::<Weak<GpuMaterialSource>>()
        + source
        + 8 * std::mem::size_of::<usize>();
    assert!(
        2 * std::mem::size_of::<SurfaceKey>() + 3 * per_source <= METADATA_PER_TRIANGLE as usize
    );
};
impl SurfaceKey {
    fn of(t: &PreparedSurface) -> Self {
        Self {
            vertices: t.vertices.map(|v| v.map(f32::to_bits)),
            source: Arc::as_ptr(&t.source) as usize,
            secondary: t
                .secondary
                .as_ref()
                .map(|(s, w)| (Arc::as_ptr(s) as usize, w.map(f32::to_bits))),
            paint: t.paint.as_ref().map(|s| Arc::as_ptr(s) as usize),
            kind: t.kind,
            repeat: t.repeat,
            bounds: t.bounds.map(f32::to_bits),
            cell: t.cell,
        }
    }
}
struct Entry {
    canvas: IsometricCanvas,
    projection: [u32; 3],
    surfaces: Vec<SurfaceKey>,
    // Keeping control blocks prevents pointer ABA without retaining GPU data.
    materials: Vec<Weak<GpuMaterialSource>>,
    paints: Vec<Weak<GpuTerrainPaintSource>>,
    lease: renderkit::RetainedCacheLease,
}
#[derive(Default)]
pub(super) struct GroundCache {
    entry: Option<Entry>,
    previous_projection: Option<([u32; 3], (u32, u32))>,
    capture: bool,
    hits: u64,
    misses: u64,
}
fn projection(p: Projection) -> [u32; 3] {
    [p.unit.to_bits(), p.origin.x.to_bits(), p.origin.y.to_bits()]
}
fn eligible(frame: &IsometricPreparedFrame) -> Option<(&[PreparedSurface], u64)> {
    // Other ground command sequences keep their existing ordered renderer.
    let [Command::Surface(surfaces)] = frame.ground.as_slice() else {
        return None;
    };
    if !frame.terrain {
        return None;
    }
    let gpu = u64::from(frame.size.0) * u64::from(frame.size.1) * 24;
    let metadata = (surfaces.len() as u64)
        .checked_mul(METADATA_PER_TRIANGLE)?
        .checked_add(METADATA_BASE)?;
    (gpu <= GPU_LIMIT && metadata <= METADATA_LIMIT).then_some((surfaces, gpu + metadata))
}
fn copy(
    context: &GpuContext,
    source: &IsometricCanvas,
    destination: &IsometricCanvas,
) -> Result<()> {
    if context.is_lost()
        || source.device_generation() != context.generation()
        || destination.device_generation() != context.generation()
        || source.size() != destination.size()
    {
        return Err(RenderError(
            "terrain cache canvas belongs to a different view or device".into(),
        ));
    }
    let pixels = u64::from(source.size().0) * u64::from(source.size().1);
    let mut encoder =
        context
            .device
            .create_command_encoder(&renderkit::wgpu::CommandEncoderDescriptor {
                label: Some("isometric retained terrain copy"),
            });
    for (from, to, stride) in [
        (source.albedo(), destination.albedo(), 4),
        (source.surfaces(), destination.surfaces(), 8),
        (source.owners(), destination.owners(), 4),
        (source.terrain(), destination.terrain(), 8),
    ] {
        encoder.copy_buffer_to_buffer(from, 0, to, 0, pixels * stride);
    }
    context.queue.submit([encoder.finish()]);
    Ok(())
}
impl GroundCache {
    pub fn clear(&mut self) {
        self.entry = None;
        self.previous_projection = None;
        self.capture = false;
    }
    pub fn usage(&self) -> GroundCacheUsage {
        GroundCacheUsage {
            hits: self.hits,
            misses: self.misses,
            gpu_bytes: self.entry.as_ref().map_or(0, |e| {
                u64::from(e.canvas.size().0) * u64::from(e.canvas.size().1) * 24
            }),
            reserved_bytes: self.entry.as_ref().map_or(0, |e| e.lease.bytes()),
        }
    }
    pub fn restore(
        &mut self,
        context: &GpuContext,
        canvas: &IsometricCanvas,
        frame: &IsometricPreparedFrame,
    ) -> Result<bool> {
        let budget = context.retained_cache_usage();
        let Some((surfaces, _)) = eligible(frame).filter(|_| budget.bytes <= budget.byte_limit)
        else {
            self.clear();
            return Ok(false);
        };
        let current = (projection(frame.projection), frame.size);
        self.capture = self.previous_projection == Some(current);
        self.previous_projection = Some(current);
        if self
            .entry
            .as_ref()
            .is_some_and(|entry| (entry.projection, entry.canvas.size()) != current)
        {
            // Moving cameras use the original renderer without paying for a
            // snapshot that the next camera position cannot reuse. Admit a
            // new base only after two render requests share this projection.
            self.entry = None;
        }
        if let Some(entry) = &self.entry {
            if entry.canvas.size() == frame.size
                && entry.projection == projection(frame.projection)
                && entry.surfaces.len() == surfaces.len()
                && entry
                    .surfaces
                    .iter()
                    .zip(surfaces)
                    .all(|(a, b)| *a == SurfaceKey::of(b))
            {
                copy(context, &entry.canvas, canvas)?;
                entry.lease.retain_for_submission(&context.queue);
                self.hits += 1;
                return Ok(true);
            }
        }
        self.misses += 1;
        Ok(false)
    }
    pub fn store(
        &mut self,
        context: &GpuContext,
        canvas: &IsometricCanvas,
        frame: &IsometricPreparedFrame,
    ) -> Result<()> {
        if !self.capture {
            return Ok(());
        }
        let Some((surfaces, cost)) = eligible(frame) else {
            self.clear();
            return Ok(());
        };
        if self
            .entry
            .as_ref()
            .is_some_and(|e| e.canvas.size() != frame.size || e.lease.bytes() < cost)
        {
            self.clear();
        }
        if self.entry.is_none() {
            let Some(lease) = context.try_reserve_retained_cache(cost) else {
                return Ok(());
            };
            self.entry = Some(Entry {
                canvas: IsometricCanvas::new(context, frame.size.0, frame.size.1)?,
                projection: projection(frame.projection),
                surfaces: Vec::new(),
                materials: Vec::new(),
                paints: Vec::new(),
                lease,
            });
        }
        let entry = self.entry.as_mut().unwrap();
        // Drop weak guards only after the current immutable sources exist.
        // Their old control blocks could not have been reused by these inputs.
        entry.surfaces.clear();
        entry.materials.clear();
        entry.paints.clear();
        let mut materials = FxHashSet::default();
        let mut paints = FxHashSet::default();
        for t in surfaces {
            entry.surfaces.push(SurfaceKey::of(t));
            for source in std::iter::once(&t.source).chain(t.secondary.as_ref().map(|s| &s.0)) {
                if materials.insert(Arc::as_ptr(source) as usize) {
                    entry.materials.push(Arc::downgrade(source));
                }
            }
            if let Some(source) = &t.paint {
                if paints.insert(Arc::as_ptr(source) as usize) {
                    entry.paints.push(Arc::downgrade(source));
                }
            }
        }
        entry.projection = projection(frame.projection);
        copy(context, canvas, &entry.canvas)?;
        entry.lease.retain_for_submission(&context.queue);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
