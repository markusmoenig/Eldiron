//! Retained CPU surface coordinates, independent of GPU generations/cameras.
//! Original hash traversal and triangle order define overlapping first matches.
use super::*;

#[derive(Clone)]
pub struct PaintSurfaceIndexSource {
    generation: Arc<()>,
    transform: [u32; 16],
}
impl PaintSurfaceIndexSource {
    pub fn matches(&self, vm: &VM) -> bool {
        Arc::ptr_eq(&self.generation, &vm.paint_surface_generation)
            && self.transform == vm.transform3d.into_col_array().map(f32::to_bits)
    }
}
impl VM {
    pub fn paint_surface_index_source(&self) -> PaintSurfaceIndexSource {
        PaintSurfaceIndexSource {
            generation: self.paint_surface_generation.clone(),
            transform: self.transform3d.into_col_array().map(f32::to_bits),
        }
    }
    /// Snapshot only chunk keys initially. Geometry is read a triangle at a time
    /// under the generation guard; no reference into a mutable VM escapes.
    pub fn begin_paint_surface_index(&self) -> PaintSurfaceIndexBuilder {
        self.begin_paint_surface_index_filtered(None)
    }
    /// Canonical placement indexes retain only surfaces referenced by the
    /// frozen stamp source; unrelated geometry never duplicates CPU triangles.
    pub fn begin_paint_surface_index_for(
        &self,
        surfaces: std::collections::HashSet<[u32; 4]>,
    ) -> PaintSurfaceIndexBuilder {
        self.begin_paint_surface_index_filtered(Some(surfaces))
    }
    fn begin_paint_surface_index_filtered(
        &self,
        surfaces: Option<std::collections::HashSet<[u32; 4]>>,
    ) -> PaintSurfaceIndexBuilder {
        PaintSurfaceIndexBuilder {
            index: PaintSurfaceIndex {
                source: self.paint_surface_index_source(),
                triangles: FxHashMap::default(),
                surfaces,
            },
            chunks: self.chunks_map.keys().copied().collect(),
            chunk: 0,
            groups: None,
            group: 0,
            poly: 0,
            triangle: 0,
        }
    }
}
struct Triangle {
    uv: [Vec2<f32>; 3],
    world: [Vec3<f32>; 3],
    normal: [f32; 3],
}
pub struct PaintSurfaceIndex {
    source: PaintSurfaceIndexSource,
    triangles: FxHashMap<[u32; 4], Vec<Triangle>>,
    surfaces: Option<std::collections::HashSet<[u32; 4]>>,
}
pub enum PaintSurfaceLookup {
    Pending,
    Complete(Option<([f32; 3], [f32; 3])>),
}
impl PaintSurfaceIndex {
    pub fn referenced_surface_count(&self) -> Option<usize> {
        self.surfaces.as_ref().map(|surfaces| surfaces.len())
    }
    pub fn covers(&self, geo: [u32; 4]) -> bool {
        self.surfaces
            .as_ref()
            .is_none_or(|surfaces| surfaces.contains(&geo))
    }
    pub fn matches(&self, vm: &VM) -> bool {
        self.source.matches(vm)
    }
    /// One original triangle candidate per call, including the original UV
    /// tolerance, homogeneous transform and degenerate-normal rejection policy.
    pub fn lookup_step(
        &self,
        geo: [u32; 4],
        uv: [f32; 2],
        cursor: &mut usize,
    ) -> PaintSurfaceLookup {
        let Some(triangle) = self.triangles.get(&geo).and_then(|rows| rows.get(*cursor)) else {
            return PaintSurfaceLookup::Complete(None);
        };
        *cursor += 1;
        let world = VM::world_position_from_triangle_uv(
            Vec2::new(uv[0], uv[1]),
            triangle.uv[0],
            triangle.uv[1],
            triangle.uv[2],
            triangle.world[0],
            triangle.world[1],
            triangle.world[2],
        );
        match world {
            Some(world) => {
                PaintSurfaceLookup::Complete(Some((world.into_array(), triangle.normal)))
            }
            None => PaintSurfaceLookup::Pending,
        }
    }
}
pub struct PaintSurfaceIndexBuilder {
    index: PaintSurfaceIndex,
    chunks: Vec<Uuid>,
    chunk: usize,
    groups: Option<Vec<GeoId>>,
    group: usize,
    poly: usize,
    triangle: usize,
}
impl PaintSurfaceIndexBuilder {
    pub fn matches(&self, vm: &VM) -> bool {
        self.index.matches(vm)
    }
    pub fn advance(&mut self, vm: &VM, items: usize) -> Result<bool, String> {
        self.advance_with(vm, items, || true)
    }
    pub fn advance_with(
        &mut self,
        vm: &VM,
        items: usize,
        mut allow_step: impl FnMut() -> bool,
    ) -> Result<bool, String> {
        if !self.matches(vm) {
            return Err("Surface geometry changed during index preparation".into());
        }
        let started = instant::Instant::now();
        for _ in 0..items.min(4096) {
            if started.elapsed().as_millis() >= 2 || !allow_step() {
                break;
            }
            let Some(chunk_id) = self.chunks.get(self.chunk) else {
                return Ok(true);
            };
            let chunk = vm
                .chunks_map
                .get(chunk_id)
                .ok_or("Surface chunk retired during preparation")?;
            let groups = self
                .groups
                .get_or_insert_with(|| chunk.polys3d_map.keys().copied().collect());
            let Some(group) = groups.get(self.group) else {
                self.chunk += 1;
                self.groups = None;
                self.group = 0;
                continue;
            };
            let polys = &chunk.polys3d_map[group];
            let Some(poly) = polys.get(self.poly) else {
                self.group += 1;
                self.poly = 0;
                continue;
            };
            let indices = poly
                .paint_surface_id
                .filter(|geo| self.index.covers(*geo))
                .filter(|_| poly.paint_uvs.len() == poly.vertices.len())
                .and_then(|geo| {
                    poly.indices
                        .get(self.triangle)
                        .map(|indices| (geo, *indices))
                });
            let Some((geo, (ia, ib, ic))) = indices else {
                self.poly += 1;
                self.triangle = 0;
                continue;
            };
            self.triangle += 1;
            let (Some(a), Some(b), Some(c), Some(ua), Some(ub), Some(uc)) = (
                poly.vertices.get(ia),
                poly.vertices.get(ib),
                poly.vertices.get(ic),
                poly.paint_uvs.get(ia),
                poly.paint_uvs.get(ib),
                poly.paint_uvs.get(ic),
            ) else {
                continue;
            };
            let world_point = |vertex: &[f32; 4]| {
                let point = vm.transform3d * Vec4::from(*vertex);
                let w = if point.w.abs() > 1e-8 { point.w } else { 1. };
                Vec3::new(point.x / w, point.y / w, point.z / w)
            };
            let world = [world_point(a), world_point(b), world_point(c)];
            let Some(normal) = (world[1] - world[0])
                .cross(world[2] - world[0])
                .try_normalized()
            else {
                continue;
            };
            self.index.triangles.entry(geo).or_default().push(Triangle {
                uv: [Vec2::from(*ua), Vec2::from(*ub), Vec2::from(*uc)],
                world,
                normal: normal.into_array(),
            });
        }
        Ok(self.chunk == self.chunks.len())
    }
    pub fn finish(self) -> Result<PaintSurfaceIndex, String> {
        if self.chunk != self.chunks.len() {
            return Err("Surface index preparation is incomplete".into());
        }
        Ok(self.index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_surface_index_preserves_lookup_order_and_rejects_edits() {
        let mut vm = VM::new(16, 16);
        let geo = [1, 2, 3, 4];
        for x in [0., 3.] {
            let mut poly = Poly3D::poly(
                GeoId::Unknown(7),
                Uuid::nil(),
                vec![[x, 0., 0., 1.], [x + 1., 0., 0., 1.], [x, 1., 0., 1.]],
                Vec::new(),
                vec![(0, 1, 2); 4097],
            );
            poly.paint_surface_id = Some(geo);
            poly.paint_uvs = vec![[0., 0.], [1., 0.], [0., 1.]];
            let id = Uuid::new_v4();
            vm.execute(Atom::NewChunk { id });
            vm.execute(Atom::SetCurrentChunk { id });
            vm.execute(Atom::AddPoly3D { poly });
        }
        let mut builder = vm.begin_paint_surface_index();
        assert!(!builder.advance(&vm, 0).unwrap());
        assert!(!builder.advance(&vm, usize::MAX).unwrap());
        assert!(vm.begin_paint_surface_index().finish().is_err());
        while !builder.advance(&vm, 4096).unwrap() {}
        let index = builder.finish().unwrap();
        for uv in [[0.25, 0.5], [-0.02, 0.5], [5., 5.]] {
            let mut cursor = 0;
            let result = loop {
                if let PaintSurfaceLookup::Complete(result) =
                    index.lookup_step(geo, uv, &mut cursor)
                {
                    break result;
                }
            };
            assert_eq!(result, vm.paint_surface_world_at(geo, uv));
        }
        let mut stale = vm.begin_paint_surface_index();
        vm.execute(Atom::SetTransform3D(Mat4::translation_3d(Vec3::new(
            2., 3., 4.,
        ))));
        assert!(!index.matches(&vm));
        assert!(stale.advance(&vm, 1).is_err());
        let mut next = vm.begin_paint_surface_index();
        while !next.advance(&vm, 4096).unwrap() {}
        let next = next.finish().unwrap();
        let mut cursor = 0;
        match next.lookup_step(geo, [0.25, 0.5], &mut cursor) {
            PaintSurfaceLookup::Complete(result) => {
                assert_eq!(result, vm.paint_surface_world_at(geo, [0.25, 0.5]))
            }
            PaintSurfaceLookup::Pending => panic!("expected first triangle hit"),
        }
        vm.mark_all_geometry_dirty();
        assert!(!next.matches(&vm));
    }
}
