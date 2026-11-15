//! One view owns a frozen pose, scoped surface index and incomplete placement
//! packet. No partial packet mutates the scene's displayed billboard source.
use super::*;
use crate::shared_vec::SharedVec;
use scenevm::vm::{
    PaintSurfaceIndex, PaintSurfaceIndexBuilder, PaintSurfaceIndexSource, PaintSurfaceLookup,
};
use std::collections::HashSet;

struct Scope {
    source: Uuid,
    visible: bool,
    lists: Vec<SharedVec<IsoPaintStamp>>,
    geometry: PaintSurfaceIndexSource,
    basis: [u32; 6],
    height: u32,
}
fn basis(camera: Camera3D) -> [u32; 6] {
    [
        camera.right.x,
        camera.right.y,
        camera.right.z,
        camera.up.x,
        camera.up.y,
        camera.up.z,
    ]
    .map(f32::to_bits)
}
impl Scope {
    fn matches(
        &self,
        set: &StampSpriteSet,
        layer: &IsoPaintLayer,
        vm: &scenevm::VM,
        camera: Camera3D,
        height: u32,
    ) -> bool {
        if self.source != set.id()
            || self.visible != layer.visible
            || self.height != height
            || self.basis != basis(camera)
            || !self.geometry.matches(vm)
        {
            return false;
        }
        if !layer.visible {
            return true;
        }
        let mut chunks = layer
            .chunks
            .values()
            .filter(|chunk| !chunk.stamps.is_empty());
        self.lists.iter().all(|list| {
            chunks
                .next()
                .is_some_and(|chunk| list.same_storage(&chunk.stamps))
        }) && chunks.next().is_none()
    }
}
enum Phase {
    Gather,
    Index(PaintSurfaceIndexBuilder),
    Place,
}
struct Pending {
    scope: Scope,
    set: Arc<StampSpriteSet>,
    camera: Camera3D,
    phase: Phase,
    chunk: usize,
    stamp: usize,
    triangle: usize,
    surfaces: HashSet<[u32; 4]>,
    index_usable: bool,
    instances: Vec<OrganicBillboardInstance>,
    key: StampUploadKey,
    matches_uploaded: bool,
}
impl Pending {
    fn next(&mut self) {
        self.triangle = 0;
        self.stamp += 1;
        if self
            .scope
            .lists
            .get(self.chunk)
            .is_some_and(|list| self.stamp == list.len())
        {
            self.chunk += 1;
            self.stamp = 0;
        }
    }
}
#[derive(Default)]
pub struct StampPlacementOwner {
    index: Option<PaintSurfaceIndex>,
    pending: Option<Pending>,
    ready: Option<Scope>,
    uploaded: Option<StampUploadKey>,
}
impl StampPlacementOwner {
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub fn cancel_pending(&mut self) {
        self.pending = None;
        self.ready = None;
    }
    /// Must refer to base VM0, the same original geometry used by the reference
    /// stamp path. Only immutable lists/artwork and camera values are retained.
    pub fn request(
        &mut self,
        set: Arc<StampSpriteSet>,
        layer: &IsoPaintLayer,
        vm: &scenevm::VM,
        camera: Camera3D,
        height: u32,
    ) -> bool {
        if self
            .ready
            .as_ref()
            .is_some_and(|scope| scope.matches(&set, layer, vm, camera, height))
        {
            return true;
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.scope.matches(&set, layer, vm, camera, height))
        {
            return false;
        }
        self.ready = None;
        if self.index.as_ref().is_some_and(|index| !index.matches(vm)) {
            self.index = None;
        }
        self.pending = Some(Pending {
            scope: Scope {
                source: set.id(),
                visible: layer.visible,
                lists: if layer.visible {
                    layer
                        .chunks
                        .values()
                        .filter(|chunk| !chunk.stamps.is_empty())
                        .map(|chunk| chunk.stamps.clone())
                        .collect()
                } else {
                    Vec::new()
                },
                geometry: vm.paint_surface_index_source(),
                basis: basis(camera),
                height,
            },
            key: StampUploadKey {
                source: set.id(),
                instances: Vec::new(),
            },
            matches_uploaded: self
                .uploaded
                .as_ref()
                .is_some_and(|old| old.source == set.id()),
            set,
            camera,
            phase: Phase::Gather,
            chunk: 0,
            stamp: 0,
            triangle: 0,
            surfaces: HashSet::new(),
            index_usable: self.index.as_ref().is_some_and(|index| index.matches(vm)),
            instances: Vec::new(),
        });
        false
    }
    /// Return readiness and whether the completed packet changed VM data.
    /// Local item/time slices cover recipe and triangle work, including exact
    /// upload-key comparison. The caller schedules another update while pending.
    pub fn advance(&mut self, scene: &mut SceneVM, items: usize) -> Result<(bool, bool), String> {
        let active = scene.active_vm_index();
        scene.set_active_vm(0);
        let result = self.advance_base(scene.active_vm_mut(), items);
        scene.set_active_vm(active);
        result
    }
    pub fn advance_with_budget(
        &mut self,
        scene: &mut SceneVM,
        items: usize,
        budget: &mut scenevm::source_upload::SourceUploadBudget,
    ) -> Result<(bool, bool), String> {
        let active = scene.active_vm_index();
        scene.set_active_vm(0);
        let result = budget.prepare_cpu(|budget| {
            self.advance_base_with(scene.active_vm_mut(), items, || {
                budget.try_prepare_cpu_step()
            })
        });
        scene.set_active_vm(active);
        result
    }
    fn advance_base(&mut self, vm: &mut scenevm::VM, items: usize) -> Result<(bool, bool), String> {
        self.advance_base_with(vm, items, || true)
    }
    fn advance_base_with(
        &mut self,
        vm: &mut scenevm::VM,
        items: usize,
        mut allow_step: impl FnMut() -> bool,
    ) -> Result<(bool, bool), String> {
        let Some(mut pending) = self.pending.take() else {
            return Ok((true, false));
        };
        if !pending.scope.geometry.matches(vm) {
            // Keep original source/camera but restart all geometry-dependent
            // work. No coordinates from the retired generation survive.
            pending.scope.geometry = vm.paint_surface_index_source();
            self.index = None;
            pending.phase = Phase::Gather;
            pending.chunk = 0;
            pending.stamp = 0;
            pending.triangle = 0;
            pending.surfaces.clear();
            pending.instances.clear();
            pending.key.instances.clear();
            pending.index_usable = false;
            pending.matches_uploaded = self
                .uploaded
                .as_ref()
                .is_some_and(|old| old.source == pending.set.id());
        }
        if let Phase::Index(builder) = &mut pending.phase {
            if builder.advance_with(vm, items, &mut allow_step)? {
                let Phase::Index(builder) = std::mem::replace(&mut pending.phase, Phase::Place)
                else {
                    unreachable!()
                };
                self.index = Some(builder.finish()?);
            }
            self.pending = Some(pending);
            return Ok((false, false));
        }
        let started = web_time::Instant::now();
        let mut done = false;
        for _ in 0..items.min(4096) {
            if started.elapsed() >= std::time::Duration::from_millis(2) || !allow_step() {
                break;
            }
            let Some(stamp) = pending
                .scope
                .lists
                .get(pending.chunk)
                .and_then(|list| list.get(pending.stamp))
            else {
                done = true;
                break;
            };
            if matches!(pending.phase, Phase::Gather) {
                if let (Some(geo), Some(_)) = (stamp.paint_geo, stamp.surface_uv) {
                    pending.index_usable &=
                        self.index.as_ref().is_some_and(|index| index.covers(geo));
                    pending.surfaces.insert(geo);
                }
                pending.next();
                continue;
            }
            let world = match (stamp.paint_geo, stamp.surface_uv) {
                (Some(geo), Some(uv)) => {
                    let mut cursor = pending.triangle;
                    let lookup = self
                        .index
                        .as_ref()
                        .ok_or("Missing stamp surface index")?
                        .lookup_step(geo, uv, &mut cursor);
                    pending.triangle = cursor;
                    match lookup {
                        PaintSurfaceLookup::Pending => continue,
                        PaintSurfaceLookup::Complete(world) => {
                            world.map(|(world, _)| world).or(stamp.world)
                        }
                    }
                }
                _ => stamp.world,
            };
            if let Some(world) = world {
                let index = pending
                    .set
                    .find(stamp)
                    .ok_or("Live stamp pixels are not current")?;
                if let Some(raster) = &pending.set.entries[index].raster {
                    let mut instance = placement(
                        stamp,
                        world,
                        pending.camera,
                        pending.scope.height,
                        raster.min,
                        raster.max,
                        raster.anchor,
                    );
                    instance.sprite_index = pending.set.slots[index].unwrap();
                    if !instance.center.iter().all(|value| value.is_finite())
                        || !instance.width.is_finite()
                        || !instance.height.is_finite()
                        || instance.width <= 0.
                        || instance.height <= 0.
                    {
                        return Err("Invalid live stamp placement".into());
                    }
                    let key = instance_key(&instance);
                    pending.matches_uploaded &= self.uploaded.as_ref().is_some_and(|old| {
                        old.instances.get(pending.instances.len()) == Some(&key)
                    });
                    pending.key.instances.push(key);
                    pending.instances.push(instance);
                }
            }
            pending.next();
        }
        if !done {
            self.pending = Some(pending);
            return Ok((false, false));
        }
        if matches!(pending.phase, Phase::Gather) {
            pending.chunk = 0;
            pending.stamp = 0;
            pending.phase = if pending.surfaces.is_empty() {
                self.index = None;
                Phase::Place
            } else if pending.index_usable
                && self.index.as_ref().is_some_and(|index| {
                    index.referenced_surface_count() == Some(pending.surfaces.len())
                })
            {
                Phase::Place
            } else {
                self.index = None;
                Phase::Index(
                    vm.begin_paint_surface_index_for(std::mem::take(&mut pending.surfaces)),
                )
            };
            self.pending = Some(pending);
            return Ok((false, false));
        }
        let unchanged = pending.matches_uploaded
            && self
                .uploaded
                .as_ref()
                .is_some_and(|old| old.instances.len() == pending.instances.len());
        if !unchanged {
            if pending.instances.is_empty() {
                vm.execute(scenevm::Atom::ClearPaintBillboards);
            } else if self
                .uploaded
                .as_ref()
                .is_some_and(|old| old.source == pending.key.source && !old.instances.is_empty())
            {
                vm.execute(scenevm::Atom::SetPaintBillboardInstances {
                    instances: pending.instances,
                });
            } else {
                vm.execute(scenevm::Atom::SetPaintBillboards {
                    sprites: pending.set.sprites(),
                    instances: pending.instances,
                });
            }
            self.uploaded = Some(pending.key);
        }
        self.ready = Some(pending.scope);
        Ok((true, !unchanged))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn drain(owner: &mut StampPlacementOwner, vm: &mut scenevm::VM) -> bool {
        for _ in 0..10000 {
            let (ready, changed) = owner.advance_base(vm, 4096).unwrap();
            if ready {
                return changed;
            }
        }
        panic!("placement preparation starved");
    }
    #[test]
    fn frozen_stamp_placements_yield_restart_geometry_and_compare_exactly() {
        let mut layer = IsoPaintLayer::default();
        layer.active_brush = "grass".into();
        layer.active_size = 1.;
        layer.add_stamp(crate::iso_paint::IsoPaintPoint::new(
            [8, 8],
            Some(Vec3::zero()),
            None,
        ));
        let geo = [1, 2, 3, 4];
        let chunk = layer.chunks.values_mut().next().unwrap();
        let prototype = chunk.stamps[0].clone();
        chunk.stamps = (0..8193)
            .map(|i| {
                let mut stamp = prototype.clone();
                stamp.paint_geo = Some(geo);
                stamp.surface_uv = Some(if i % 2 == 0 { [0.25, 0.25] } else { [5., 5.] });
                stamp.world = Some([i as f32, 2., 3.]);
                stamp
            })
            .collect();
        let frozen = layer.clone();
        let mut prep = StampSpriteRequest::capture(&layer).prepare(None);
        while !prep.advance(1).unwrap() {}
        let set = Arc::new(prep.finish().unwrap());
        let mut vm = scenevm::VM::new(16, 16);
        let mut poly = scenevm::Poly3D::poly(
            scenevm::GeoId::Unknown(4),
            Uuid::nil(),
            vec![[0., 0., 0., 1.], [1., 0., 0., 1.], [0., 1., 0., 1.]],
            Vec::new(),
            vec![(0, 1, 2)],
        );
        poly.paint_surface_id = Some(geo);
        poly.paint_uvs = vec![[0., 0.], [1., 0.], [0., 1.]];
        vm.execute(scenevm::Atom::AddPoly3D { poly });
        let camera = Camera3D::iso();
        let mut owner = StampPlacementOwner::default();
        assert!(!owner.request(set.clone(), &layer, &vm, camera, 128));
        assert_eq!(owner.advance_base(&mut vm, 0).unwrap(), (false, false));
        assert_eq!(
            owner.advance_base(&mut vm, usize::MAX).unwrap(),
            (false, false)
        );
        assert!(owner.uploaded.is_none());
        let captured_chunk = owner.pending.as_ref().unwrap().chunk;
        assert_eq!(
            owner.advance_base_with(&mut vm, 4096, || false).unwrap(),
            (false, false)
        );
        assert_eq!(owner.pending.as_ref().unwrap().chunk, captured_chunk);
        layer.chunks.values_mut().next().unwrap().stamps.clear();
        while !matches!(owner.pending.as_ref().unwrap().phase, Phase::Place) {
            owner.advance_base(&mut vm, 4096).unwrap();
        }
        owner.advance_base(&mut vm, 1).unwrap();
        assert_eq!(owner.pending.as_ref().unwrap().instances.len(), 1);
        vm.execute(scenevm::Atom::SetTransform3D(vek::Mat4::translation_3d(
            Vec3::new(2., 3., 4.),
        )));
        owner.advance_base(&mut vm, 1).unwrap();
        assert!(owner.pending.as_ref().unwrap().instances.is_empty());
        assert!(owner.uploaded.is_none());
        assert!(drain(&mut owner, &mut vm));
        let expected: Vec<_> = frozen
            .chunks
            .values()
            .flat_map(|chunk| &chunk.stamps)
            .map(|stamp| {
                let world = vm
                    .paint_surface_world_at(geo, stamp.surface_uv.unwrap())
                    .map(|(world, _)| world)
                    .or(stamp.world)
                    .unwrap();
                let index = set.find(stamp).unwrap();
                let raster = set.entries[index].raster.as_ref().unwrap();
                let mut instance = placement(
                    stamp,
                    world,
                    camera,
                    128,
                    raster.min,
                    raster.max,
                    raster.anchor,
                );
                instance.sprite_index = set.slots[index].unwrap();
                instance_key(&instance)
            })
            .collect();
        assert_eq!(owner.uploaded.as_ref().unwrap().instances, expected);
        assert!(owner.request(set.clone(), &frozen, &vm, camera, 128));
        assert!(!owner.pending());
        owner.cancel_pending();
        assert!(!owner.request(set.clone(), &frozen, &vm, camera, 128));
        assert!(
            !drain(&mut owner, &mut vm),
            "exact packet reuse must skip upload"
        );
        assert!(!owner.request(set, &layer, &vm, camera, 128));
        assert!(drain(&mut owner, &mut vm));
        assert!(owner.uploaded.as_ref().unwrap().instances.is_empty());
        assert!(owner.index.is_none());
    }
}
