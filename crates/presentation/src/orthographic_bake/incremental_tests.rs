//! Progressive target reuse must preserve original color/depth, immutable
//! published images, and request/projection boundaries while uploads shrink.
use super::tests::{colors, depths, ready};
use super::*;
use rusterix::OrthographicBakeController;
use scenevm::prelude::Vec3;

fn growing(map: Uuid, camera: Camera3D, count: usize) -> (OrthographicBakeController, usize) {
    let mut owner = OrthographicBakeController::default();
    owner.request_render(map, 2);
    let mut n = 0;
    for _ in 0..count {
        let work = owner
            .prepare_work(map, 2048, 2048, camera, Some((-8., 8., -8., 8.)))
            .unwrap()
            .unwrap();
        n = (work.tile_size * work.tile_size) as usize;
        assert!(!owner.finish_sample());
        assert!(owner.finish_sample());
        owner
            .commit_current_tile(
                [40, 80, 120, 255].repeat(n),
                vec![2.; n],
                [40, 80, 120, 255].repeat(n),
                [128, 255, 128, 255].repeat(n),
                [200, 0, 255, 0].repeat(n),
            )
            .unwrap();
    }
    owner
        .prepare_work(map, 2048, 2048, camera, Some((-8., 8., -8., 8.)))
        .unwrap()
        .unwrap();
    owner
        .preview_current_tile(&[160, 32, 20, 255].repeat(n))
        .unwrap();
    (owner, n)
}
fn reference(
    context: &GpuContext,
    output: &OrthographicBakeOutput,
    owner: &OrthographicBakeController,
    map: Uuid,
    camera: &Camera3D,
    light: Option<OrthographicBakeLighting>,
) {
    let expected = owner
        .compose_rgba_lit(map, 128, 128, camera, light)
        .unwrap();
    assert!(
        colors(context, output)
            .iter()
            .zip(expected)
            .all(|(a, b)| a.abs_diff(b) <= 1)
    );
    let expected = owner.compose_depth(map, 128, 128, camera).unwrap();
    assert!(
        depths(context, output)
            .into_iter()
            .zip(expected)
            .all(|(a, b)| a == b || (a - b).abs() < 1e-5)
    );
}

#[test]
#[ignore = "actual GPU progressive private target copy, alpha/depth and reduced transfers"]
fn progressive_pages_copy_completed_targets_and_upload_only_changed_originals() {
    let context = futures::executor::block_on(GpuContext::headless()).unwrap();
    let map = Uuid::new_v4();
    let mut camera = Camera3D::iso();
    camera.ortho_half_h = 10.;
    let (mut owner, n) = growing(map, camera, 9);
    let mut renderer = OrthographicBakeRenderer::new(context.clone()).unwrap();
    let source = owner.presentation_source(map, &camera).unwrap();
    let mut output = ready(&mut renderer, &source, &camera, (128, 128), None);
    assert_eq!(renderer.visible_tiles(), 10);
    reference(&context, &output, &owner, map, &camera, None);
    for rgba in [[180, 40, 30, 255], [0; 4], [20, 30, 40, 128]] {
        let previous = colors(&context, &output);
        owner.preview_current_tile(&rgba.repeat(n)).unwrap();
        let source = owner.presentation_source(map, &camera).unwrap();
        let before = context.transfers();
        let pending = renderer
            .render(&source, &camera, (128, 128), None)
            .unwrap()
            .unwrap();
        assert!(renderer.waiting());
        assert_eq!(pending.source_revision(), output.source_revision());
        assert!(Arc::ptr_eq(pending.image(), output.image()));
        assert_eq!(context.transfers().readback_bytes, before.readback_bytes);
        assert_eq!(
            colors(&context, &output),
            previous,
            "private updates cannot mutate the published source"
        );
        output = ready(&mut renderer, &source, &camera, (128, 128), None);
        assert!(!Arc::ptr_eq(pending.image(), output.image()));
        let uploaded = context.transfers().uploaded_bytes - before.uploaded_bytes;
        assert!(
            (n as u64 * 24..n as u64 * 24 + 4096).contains(&uploaded),
            "one changed tile plus metadata: {uploaded}"
        );
        reference(&context, &output, &owner, map, &camera, None);
    }
    // A completed surface adds depth to a former beauty-only preview, while
    // another tile enters the visible source set under the same frozen request.
    assert!(!owner.finish_sample());
    assert!(owner.finish_sample());
    owner
        .commit_current_tile(
            [90, 60, 30, 255].repeat(n),
            vec![3.; n],
            [90, 60, 30, 255].repeat(n),
            [128, 255, 128, 255].repeat(n),
            [200, 0, 255, 0].repeat(n),
        )
        .unwrap();
    owner
        .prepare_work(map, 2048, 2048, camera, Some((-8., 8., -8., 8.)))
        .unwrap()
        .unwrap();
    owner
        .preview_current_tile(&[100, 120, 140, 255].repeat(n))
        .unwrap();
    let source = owner.presentation_source(map, &camera).unwrap();
    let before = context.transfers();
    output = ready(&mut renderer, &source, &camera, (128, 128), None);
    let uploaded = context.transfers().uploaded_bytes - before.uploaded_bytes;
    assert!(
        (n as u64 * 24 * 2..n as u64 * 24 * 2 + 4096).contains(&uploaded),
        "completed+new original only: {uploaded}"
    );
    reference(&context, &output, &owner, map, &camera, None);
    // Visibility republishes the same originals. No GPU copy/upload is needed.
    owner.toggle_visibility();
    owner.toggle_visibility();
    let source = owner.presentation_source(map, &camera).unwrap();
    let before = context.transfers();
    let same = ready(&mut renderer, &source, &camera, (128, 128), None);
    assert!(Arc::ptr_eq(same.image(), output.image()));
    assert_eq!(context.transfers(), before);
    assert_eq!(same.source_revision(), source.revision());
    // Lighting is part of target identity; unchanged channels must all be
    // shaded again instead of inheriting pixels from the previous light.
    let light = Some(OrthographicBakeLighting {
        sun_direction: Vec3::new(-0.6, -1., -0.7).normalized(),
        sun_color: Vec3::new(1., 0.9, 0.8),
        sun_intensity: 1.2,
        sun_enabled: true,
    });
    let before = context.transfers();
    output = ready(&mut renderer, &source, &camera, (128, 128), light);
    assert!(context.transfers().uploaded_bytes - before.uploaded_bytes >= n as u64 * 24 * 11);
    reference(&context, &output, &owner, map, &camera, light);
    camera.pos += camera.right * 0.07;
    assert!(
        renderer
            .render(&source, &camera, (128, 128), light)
            .unwrap()
            .is_none()
    );
    output = ready(&mut renderer, &source, &camera, (128, 128), light);
    reference(&context, &output, &owner, map, &camera, light);
}

#[test]
#[ignore = "actual GPU missing original clears inherited pixels and depth exactly"]
fn missing_changed_original_clears_its_inherited_tile_without_neighbor_damage() {
    let context = futures::executor::block_on(GpuContext::headless()).unwrap();
    let map = Uuid::new_v4();
    let mut camera = Camera3D::iso();
    camera.ortho_half_h = 10.;
    let (mut owner, n) = growing(map, camera, 28);
    let mut renderer = OrthographicBakeRenderer::new(context.clone()).unwrap();
    ready(
        &mut renderer,
        &owner.presentation_source(map, &camera).unwrap(),
        &camera,
        (128, 128),
        None,
    );
    // Ensure a visible published preview before deliberately evicting its next
    // unencoded sample through genuine bounded original-source pressure.
    owner
        .preview_current_tile(&[180, 40, 30, 255].repeat(n))
        .unwrap();
    let source = owner.presentation_source(map, &camera).unwrap();
    let previous = ready(&mut renderer, &source, &camera, (128, 128), None);
    let old_ids: HashMap<_, _> = source
        .positions()
        .map(|at| (at, source.tile_revision(at.0, at.1)))
        .collect();
    owner
        .preview_current_tile(&[20, 90, 160, 255].repeat(n))
        .unwrap();
    let source = owner.presentation_source(map, &camera).unwrap();
    let changed: Vec<_> = source
        .positions()
        .filter(|at| old_ids.get(at).copied().flatten() != source.tile_revision(at.0, at.1))
        .collect();
    assert_eq!(changed.len(), 1);
    for tile in &source.asset().tiles {
        drop(source.try_tile(tile.x, tile.y).unwrap().unwrap());
    }
    assert!(matches!(
        source.poll_tile(changed[0].0, changed[0].1).unwrap(),
        rusterix::orthographic_bake::OrthographicBakeTilePoll::Missing
    ));
    let before = context.transfers();
    let output = ready(&mut renderer, &source, &camera, (128, 128), None);
    assert!(!Arc::ptr_eq(previous.image(), output.image()));
    assert!(
        context.transfers().uploaded_bytes - before.uploaded_bytes < 4096,
        "missing original requires metadata only"
    );
    assert_eq!(context.transfers().readback_bytes, before.readback_bytes);
    reference(&context, &output, &owner, map, &camera, None);
}
