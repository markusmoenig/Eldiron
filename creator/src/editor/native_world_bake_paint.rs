//! Frozen authoring paint for a private bake scene. Source revision checks do
//! not clone atlases or hash their pixels during every application update.
use super::*;
use shared::iso_paint::{ISO_PAINT_BAKED_CHUNK_SIZE, IsoPaintLayer};
use std::hash::{Hash, Hasher};

const INPUT_BUDGET: u64 = 64 * 1024 * 1024;
#[derive(Clone, PartialEq)]
pub(super) struct Stamp {
    layers: Vec<(Uuid, u64)>,
    bytes: u64,
}
impl Stamp {
    pub(super) fn new(region: &shared::region::Region, project: &Project) -> Self {
        let mut layers = vec![(region.id, &region.iso_paint)];
        let mut seen = FxHashSet::default();
        for instance in &region.map.block_prop_instances {
            if seen.insert(instance.asset_id)
                && let Some(paint) = project.block_prop_paint.get(&instance.asset_id)
            {
                layers.push((instance.asset_id, paint));
            }
        }
        let mut bytes = 0_u64;
        let layers = layers
            .into_iter()
            .map(|(id, layer)| {
                let mut hash = std::collections::hash_map::DefaultHasher::new();
                layer.visible.hash(&mut hash);
                layer.baked_version.hash(&mut hash);
                layer.surface_commit_strokes.hash(&mut hash);
                for (key, chunk) in &layer.baked_chunks {
                    key.hash(&mut hash);
                    chunk.owner.hash(&mut hash);
                    chunk.paint_geo.hash(&mut hash);
                    chunk.origin.hash(&mut hash);
                    chunk.revision.hash(&mut hash);
                }
                for (key, chunk) in &layer.chunks {
                    key.hash(&mut hash);
                    chunk.origin.hash(&mut hash);
                    chunk.revision.hash(&mut hash);
                    chunk.stamp_revision.hash(&mut hash);
                    chunk.strokes.len().hash(&mut hash);
                    chunk.stamps.len().hash(&mut hash);
                    if layer.visible {
                        bytes = bytes.saturating_add(
                            (chunk.stamps.len() as u64).saturating_mul(256 * 256 * 4),
                        );
                    }
                }
                if layer.visible {
                    // Two RGBA planes, with a conservative factor of two for atlas
                    // row packing. Check before cloning/constructing display data.
                    bytes = bytes.saturating_add(
                        (layer.baked_chunks.len() as u64)
                            .saturating_mul(ISO_PAINT_BAKED_CHUNK_SIZE as u64)
                            .saturating_mul(ISO_PAINT_BAKED_CHUNK_SIZE as u64)
                            .saturating_mul(16),
                    );
                }
                (id, hash.finish())
            })
            .collect();
        Self { layers, bytes }
    }
    pub(super) fn supported(&self) -> bool {
        self.bytes <= INPUT_BUDGET
    }
}

#[cfg(test)]
impl Editor {
    pub(crate) fn log_native_bake_paint_state_for_test(&self) {
        let jobs = &self.native_viewport.world.bake_jobs;
        eprintln!(
            "Paint request state: mode={:?}, context={:?}, status={:?}, active={}, adapter={}, source_ready={:?}",
            self.server_ctx.shared.editor_view_mode,
            self.server_ctx.shared.pc,
            self.server_ctx.bake.read().status(),
            jobs.active.is_some(),
            jobs.legacy.is_some(),
            jobs.active.as_ref().map(|job| job.source.ready())
        );
    }
    pub(crate) fn native_bake_paint_pending_for_test(&self) -> bool {
        self.native_viewport
            .world
            .bake_jobs
            .active
            .as_ref()
            .and_then(|job| job.gpu.as_ref())
            .is_some_and(|job| job.has_pending_capture())
    }
    pub(crate) fn edit_native_bake_paint_for_test(&mut self, ctx: &mut TheContext) {
        let region = self
            .documents
            .project
            .get_region_mut(&self.server_ctx.shared.curr_region)
            .unwrap();
        let before = region.iso_paint.clone();
        let chunk = region.iso_paint.baked_chunks.values_mut().next().unwrap();
        chunk.color_rgba[..4].copy_from_slice(&[220, 40, 30, 255]);
        chunk.revision = chunk.revision.wrapping_add(1);
        let after = region.iso_paint.clone();
        let id = region.id;
        self.server_ctx.history.write().add_undo(
            ProjectUndoAtom::RegionPaintEdit(
                ProjectContext::Region(id),
                id,
                Box::new(before),
                Box::new(after),
            ),
            ctx,
        );
    }
    pub(crate) fn native_bake_paint_retired_for_test(
        &self,
        old: &Option<rusterix::map::OrthographicBakeAsset>,
    ) -> bool {
        assert_eq!(
            &scope::map(&self.documents.project, &self.server_ctx)
                .unwrap()
                .orthographic_bake,
            old,
            "retired paint work cannot replace the previous asset"
        );
        !self.server_ctx.bake.read().is_rendering()
            && self.native_viewport.world.bake_jobs.active.is_none()
    }
    pub(crate) fn stage_native_bake_paint_for_test(&mut self) -> Vec<u8> {
        use shared::iso_paint::{IsoPaintBakedChunk, IsoPaintOwner, IsoPaintPoint};
        let floor = self
            .documents
            .project
            .get_region(&self.server_ctx.shared.curr_region)
            .unwrap()
            .map
            .geometry_objects
            .first()
            .unwrap()
            .id;
        let world = &self.native_viewport.world;
        let source = world.presented.as_ref().unwrap().source.clone();
        let (geo, owner) = source
            .chunks()
            .values()
            .flat_map(|chunk| chunk.polys3d_map.values())
            .flatten()
            .filter_map(|poly| {
                if poly.id != scenevm::GeoId::GeometryObject(floor) {
                    return None;
                }
                let geo = poly.paint_surface_id?;
                let (a, b, c) = poly.indices.first().copied()?;
                let point = |i: usize| {
                    let v = poly.vertices[i];
                    Vec3::new(v[0], v[1], v[2])
                };
                let normal = (point(b) - point(a))
                    .cross(point(c) - point(a))
                    .normalized();
                (normal.y.abs() > 0.9 && !poly.paint_uvs.is_empty()).then_some((
                    geo,
                    poly.id,
                    point(a).y,
                ))
            })
            .max_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(geo, owner, _)| (geo, owner))
            .expect("bake fixture has an authored painted floor");
        eprintln!("Painted fixture floor {owner:?}, surface={geo:?}");
        let owner = match owner {
            scenevm::GeoId::GeometryObject(id) => IsoPaintOwner::GeometryObject(id),
            scenevm::GeoId::Sector(id) => IsoPaintOwner::Sector(id),
            other => panic!("unexpected floor owner {other:?}"),
        };
        let region = self
            .documents
            .project
            .get_region_mut(&self.server_ctx.shared.curr_region)
            .unwrap();
        let paint = &mut region.iso_paint;
        for y in -2..=2 {
            for x in -2..=2 {
                let origin = [
                    x * ISO_PAINT_BAKED_CHUNK_SIZE,
                    y * ISO_PAINT_BAKED_CHUNK_SIZE,
                ];
                let mut chunk = IsoPaintBakedChunk::new(owner.clone(), geo, origin);
                chunk.color_rgba = [24, 190, 72, 192].repeat(chunk.color_rgba.len() / 4);
                chunk.revision = 1;
                paint
                    .baked_chunks
                    .insert(format!("bake-paint:{x}:{y}"), chunk);
            }
        }
        paint.active_brush = "grass".into();
        paint.active_stamp_size_jitter = 0.;
        paint.active_stamp_rotation_jitter = 0.;
        paint.add_stamp(
            IsoPaintPoint::new([300, 220], Some(Vec3::new(0., 0.05, 0.)), Some(owner))
                .with_camera_scale(Some(3.)),
        );
        serde_json::to_vec(paint).unwrap()
    }
    pub(crate) fn check_native_bake_paint_for_test(
        &self,
        authored: &[u8],
        gpu: &Arc<renderkit::GpuContext>,
    ) {
        let region = self
            .documents
            .project
            .get_region(&self.server_ctx.shared.curr_region)
            .unwrap();
        assert_eq!(
            serde_json::to_vec(&region.iso_paint).unwrap(),
            authored,
            "baking can only read the original authored paint layer"
        );
        let world = &self.native_viewport.world;
        let shown = world.presented.as_ref().unwrap();
        let camera = self.native_world_camera().unwrap();
        let size = (shown.binding.size.0 as u32, shown.binding.size.1 as u32);
        let camera_frame = camera.as_scenevm_camera_for_surface(size.0 as f32, size.1 as f32);
        let bridge = self.server_ctx.reference.read();
        let vm = scenevm::SceneVM::new_with_context_and_atlas(
            gpu.clone(),
            1,
            1,
            scenevm::SharedAtlas::new(1024, 1024),
        );
        let mut scene = rusterix::SceneHandler::with_vm(vm);
        eldiron_presentation::configure_scene_assets(&mut scene, &bridge.assets, true).unwrap();
        for chunk in shown.source.chunks().values() {
            scene.build_index.index_chunk(chunk);
            scene.vm.execute(scenevm::Atom::AddChunk {
                id: Uuid::new_v4(),
                chunk: chunk.as_ref().clone(),
            });
        }
        let mut reference = rusterix::Rusterix::with_scene_handler(scene);
        reference.assets = bridge.assets.clone();
        reference.client.camera_d3 = camera;
        reference.client.server_time = bridge.client.server_time;
        reference.client.editor_canvas_colors = bridge.client.editor_canvas_colors;
        reference.client.use_editor_canvas = bridge.client.use_editor_canvas;
        let mut map = region.map.clone();
        map.orthographic_bake = None;
        let mut pixels = vec![0; size.0 as usize * size.1 as usize * 4];
        reference.draw_d3_with_editor_background(
            &map,
            &mut pixels,
            size.0 as usize,
            size.1 as usize,
            true,
        );
        Snapshot::capture(region, &self.documents.project, &bridge.assets).upload(
            &mut reference.scene_handler,
            camera_frame,
            size,
        );
        reference.request_orthographic_bake(map.id, 2);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while reference.orthographic_bake.is_rendering() {
            reference.draw_d3_with_editor_background(
                &map,
                &mut pixels,
                size.0 as usize,
                size.1 as usize,
                true,
            );
            assert!(std::time::Instant::now() < deadline);
        }
        let bake = self.server_ctx.bake.read();
        let actual = bake.presentation_source(map.id, &camera_frame).unwrap();
        let expected = reference
            .orthographic_bake
            .presentation_source(map.id, &camera_frame)
            .unwrap();
        assert_eq!(
            actual.asset().projected_bounds,
            expected.asset().projected_bounds
        );
        let mut painted_pixels = 0;
        for (at, lease) in actual.tiles() {
            let tile = lease.source();
            let mut encoder = png::Encoder::new(
                std::fs::File::create(format!(
                    "/private/tmp/eldiron-painted-albedo-{}-{}.png",
                    at.0, at.1
                ))
                .unwrap(),
                actual.asset().tile_size,
                actual.asset().tile_size,
            );
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(tile.albedo)
                .unwrap();
            let expected_lease = expected.tile(at.0, at.1).unwrap();
            let expected = expected_lease.source();
            for (name, a, b) in [
                ("beauty", tile.color, expected.color),
                ("albedo", tile.albedo, expected.albedo),
                ("normal", tile.normal, expected.normal),
                ("material", tile.material, expected.material),
            ] {
                assert_eq!(a.len(), b.len());
                let delta = a.iter().zip(b).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
                eprintln!("Painted bake {at:?} {name}: max reference delta={delta}");
                assert!(delta <= 1, "painted bake {name} parity");
            }
            painted_pixels += tile
                .albedo
                .chunks_exact(4)
                .filter(|p| p[3] > 0 && p[1] > p[0].saturating_add(25) && p[1] > p[2])
                .count();
            for (a, b) in tile.depth.iter().zip(expected.depth) {
                assert_eq!(a.is_finite(), b.is_finite());
                if a.is_finite() {
                    assert!((a - b).abs() < 1e-5);
                }
            }
        }
        assert!(
            painted_pixels > 100,
            "paint must change actual saved albedo"
        );
        eprintln!(
            "Painted progressive bake: {painted_pixels} green albedo pixels; authored paint unchanged"
        );
    }
}

pub(super) struct Snapshot(IsoPaintLayer);

#[cfg(test)]
#[test]
fn frozen_paint_source_owns_current_pixels_without_editor_history() {
    use shared::iso_paint::{IsoPaintOwner, IsoPaintPoint, IsoPaintSelection};
    use vek::Vec2;
    let mut project = Project::new();
    let mut region = shared::region::Region::new();
    let paint = &mut region.iso_paint;
    let owner = IsoPaintOwner::Sector(7);
    let geo = [7, 0, 0, 1];
    let mut mask = IsoPaintSelection::new(owner.clone());
    for y in 0..16 {
        for x in 0..16 {
            mask.set_pixel(owner.clone(), geo, [x, y]);
        }
    }
    paint.set_active_selection(mask);
    paint.active_size = 3.;
    paint.active_color = [24, 190, 72, 192];
    paint.begin_stroke(
        IsoPaintPoint::new([8, 8], None, Some(owner.clone()))
            .with_surface_uv(Some(Vec2::new(0.125, 0.125)))
            .with_paint_geo(Some(geo)),
    );
    paint.active_brush = "grass".into();
    paint.add_stamp(IsoPaintPoint::new(
        [10, 10],
        Some(Vec3::zero()),
        Some(owner.clone()),
    ));
    paint.ensure_screen_chunk([0, 0]);
    paint.selection_visible = true;
    paint.surface_commit_strokes.push(Uuid::new_v4());
    // Unreferenced selection data must never enter the frozen source.
    paint
        .selection_masks
        .insert(Uuid::new_v4(), IsoPaintSelection::new(owner));
    let asset = Uuid::new_v4();
    region
        .map
        .block_prop_instances
        .push(rusterix::BlockPropInstance::new(asset));
    project.block_prop_paint.insert(asset, paint.clone());
    let assets = rusterix::Assets::default();
    let before = serde_json::to_vec(&region.iso_paint).unwrap();
    let mut expected = region.iso_paint.clone();
    crate::block_props::merge_prefab_paint_for_map(
        &mut expected,
        &region.map,
        &assets.block_props,
        &project.block_prop_paint,
    );
    let snapshot = Snapshot::capture(&region, &project, &assets).0;
    assert_eq!(snapshot.baked_chunks, expected.baked_chunks);
    assert!(
        snapshot
            .baked_chunks
            .values()
            .any(|chunk| chunk.color_rgba.chunks_exact(4).any(|p| p[3] > 0))
    );
    assert_eq!(
        snapshot
            .chunks
            .values()
            .flat_map(|c| &c.stamps)
            .collect::<Vec<_>>(),
        expected
            .chunks
            .values()
            .flat_map(|c| &c.stamps)
            .collect::<Vec<_>>()
    );
    assert!(snapshot.chunks.values().all(|c| c.strokes.is_empty()));
    assert!(snapshot.selection_masks.is_empty() && snapshot.screen_chunks.is_empty());
    assert!(snapshot.source_images.is_empty() && snapshot.surface_commit_strokes.is_empty());
    assert!(snapshot.active_selection_id.is_none() && !snapshot.selection_visible);
    assert_eq!(serde_json::to_vec(&region.iso_paint).unwrap(), before);
    // Retiring the document's history/selection cannot invalidate frozen pixels.
    region.iso_paint = Default::default();
    project.block_prop_paint.clear();
    assert_eq!(snapshot.baked_chunks, expected.baked_chunks);
}

impl Snapshot {
    pub(super) fn capture(
        region: &shared::region::Region,
        project: &Project,
        assets: &rusterix::Assets,
    ) -> Self {
        use shared::iso_paint::{ISO_PAINT_BAKE_VERSION, IsoPaintChunk};
        let source = &region.iso_paint;
        let mut paint = IsoPaintLayer {
            visible: source.visible,
            ..Default::default()
        };
        if source.visible {
            // Obsolete experimental UV versions are outside the compatibility
            // contract. Never replay their history or use their derived pixels.
            if source.baked_version == ISO_PAINT_BAKE_VERSION {
                paint.baked_chunks = source.baked_chunks.clone();
            }
            for (key, chunk) in &source.chunks {
                if chunk.stamps.is_empty() {
                    continue;
                }
                paint.chunks.insert(
                    key.clone(),
                    IsoPaintChunk {
                        origin: chunk.origin,
                        revision: chunk.revision,
                        stamp_revision: chunk.stamp_revision,
                        strokes: Vec::new(),
                        stamps: chunk.stamps.clone(),
                    },
                );
            }
        }
        crate::block_props::merge_prefab_paint_source_for_map(
            &mut paint,
            &region.map,
            &assets.block_props,
            &project.block_prop_paint,
            false,
        );
        Self(paint)
    }
    pub(super) fn upload(
        mut self,
        scene: &mut rusterix::SceneHandler,
        camera: scenevm::Camera3D,
        viewport: (u32, u32),
    ) {
        shared::iso_paint_render::IsoPaintRenderer::upload_sources_cached(
            &mut Default::default(),
            &mut self.0,
            &mut scene.vm,
            camera,
            viewport,
        );
    }
}
