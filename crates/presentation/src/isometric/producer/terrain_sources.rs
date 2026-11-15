//! Source selection is constant within a terrain cell for one preparation.
//! Geometry/order remain per triangle; no cache survives a new input/frame.
use super::*;
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;

#[derive(Default)]
pub(super) struct CellSources(FxHashMap<[i32; 2], Option<PreparedSurface>>);
impl CellSources {
    pub fn prepare(
        &mut self,
        renderer: &mut IsometricSceneRenderer,
        input: &IsometricSceneInput<'_>,
        policy: &SurfaceMaterials<'_>,
        frame: usize,
        cell: [i32; 2],
        vertices: [[f32; 3]; 3],
    ) -> Result<Option<PreparedSurface>> {
        let template = match self.0.entry(cell) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let source = policy
                    .cell((cell[0], cell[1]))
                    .map(|selection| {
                        renderer.prepare_surface(
                            input,
                            selection,
                            frame,
                            cell,
                            vertices,
                            SurfaceKind::Terrain,
                            true,
                            [0.; 4],
                            false,
                            true,
                        )
                    })
                    .transpose()?;
                entry.insert(source)
            }
        };
        Ok(template.as_ref().map(|template| {
            let mut surface = template.clone();
            surface.vertices = vertices;
            surface
        }))
    }
}
