//! An outgoing view packet may be revoked before its replacement is prepared.
//! Keep only its unique immutable source leases across that handoff, without
//! retaining stale input targets, triangles, rendering commands or CPU pixels.
use super::*;
use rustc_hash::FxHashMap;

#[derive(Default)]
pub(crate) struct IsometricFrameSources {
    materials: FxHashMap<usize, Arc<GpuMaterialSource>>,
    paints: FxHashMap<usize, Arc<GpuTerrainPaintSource>>,
    sprites: FxHashMap<usize, Arc<GpuSpriteSource>>,
}
impl IsometricPreparedFrame {
    pub(crate) fn source_leases(&self) -> IsometricFrameSources {
        fn keep<T>(sources: &mut FxHashMap<usize, Arc<T>>, source: &Arc<T>) {
            sources
                .entry(Arc::as_ptr(source) as usize)
                .or_insert_with(|| source.clone());
        }
        let mut sources = IsometricFrameSources::default();
        for command in self
            .ground
            .iter()
            .chain(self.upright.iter().map(|(_, command)| command))
        {
            match command {
                Command::Surface(surfaces) => {
                    for surface in surfaces {
                        keep(&mut sources.materials, &surface.source);
                        if let Some((source, _)) = &surface.secondary {
                            keep(&mut sources.materials, source);
                        }
                        if let Some(source) = &surface.paint {
                            keep(&mut sources.paints, source);
                        }
                    }
                }
                Command::Sprite(sprite) => keep(&mut sources.sprites, &sprite.source),
            }
        }
        sources
    }
}
