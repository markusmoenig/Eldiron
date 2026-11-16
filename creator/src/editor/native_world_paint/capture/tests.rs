use super::*;

#[test]
fn admitted_paint_snapshot_finishes_before_capturing_newer_edits() {
    let mut map = Map::default();
    map.block_prop_instances.push(rusterix::BlockPropInstance::new(Uuid::new_v4()));
    let scope = (Uuid::new_v4(), ProjectContext::Region(map.id), map.id, None);
    let assets = rusterix::Assets::default();
    let catalog = indexmap::IndexMap::new();
    let mut base = IsoPaintLayer::default();
    base.baked_version = ISO_PAINT_BAKE_VERSION;
    base.baked_chunks.insert("page".into(), IsoPaintBakedChunk::new(
        IsoPaintOwner::Terrain { x: 0, z: 0 }, [1, 2, 3, 4], [0; 2]));
    let mut capture = Capture::default();
    assert!(capture.request_source(scope, Some(&base), false, Some(&map),
        &assets, &catalog, 0).is_none());
    assert!(capture.pending());
    let before = base.baked_chunks["page"].color_rgba[0];
    base.baked_chunks.get_mut("page").unwrap().color_rgba[0] = 77;
    let admitted = capture.advance_pinned(&assets).unwrap();
    assert_eq!(admitted.baked_chunks["page"].color_rgba[0], before);
    let latest = capture.request_source(scope, Some(&base), false, Some(&map),
        &assets, &catalog, 4096).unwrap();
    assert_eq!(latest.baked_chunks["page"].color_rgba[0], 77);
    assert!(!Arc::ptr_eq(&admitted, &latest));
}

fn drain(
    capture: &mut Capture,
    scope: Scope,
    base: &IsoPaintLayer,
    map: &Map,
    assets: &rusterix::Assets,
    catalog: &indexmap::IndexMap<Uuid, IsoPaintLayer>,
    selection: bool,
) -> Arc<IsoPaintLayer> {
    for _ in 0..10000 {
        if let Some(ready) = capture.request_source(
            scope,
            Some(base),
            selection,
            Some(map),
            assets,
            catalog,
            4096,
        ) {
            return ready;
        }
    }
    panic!("merged paint capture starved");
}
fn expected(
    base: &IsoPaintLayer,
    map: &Map,
    assets: &rusterix::Assets,
    catalog: &indexmap::IndexMap<Uuid, IsoPaintLayer>,
    selection: bool,
) -> IsoPaintLayer {
    let mut expected = base.presentation_source(selection);
    crate::block_props::merge_prefab_paint_source_for_map(
        &mut expected,
        map,
        &assets.block_props,
        catalog,
        false,
    );
    expected.selection_visible = selection;
    expected
}

#[test]
fn merged_paint_capture_reuses_sources_and_matches_original_visibility_and_order() {
    let document = Uuid::new_v4();
    let mut map = Map::default();
    let scope = (document, ProjectContext::Region(map.id), map.id, None);
    let object =
        rusterix::GeometryObject::box_from_bounds("Painted", Vec3::zero(), Vec3::broadcast(1.));
    let first = rusterix::BlockPropAsset::new_authored("First", vec![object.clone()]);
    let second = rusterix::BlockPropAsset::new_authored("Second", vec![object]);
    let first_id = first.id;
    let second_id = second.id;
    let mut assets = rusterix::Assets::default();
    assets.set_block_props(
        [(first_id, first), (second_id, second)]
            .into_iter()
            .collect(),
    );
    map.block_prop_instances = (0..8193)
        .map(|_| rusterix::BlockPropInstance::new(first_id))
        .collect();
    map.block_prop_instances
        .push(rusterix::BlockPropInstance::new(second_id));
    let mut base = IsoPaintLayer::default();
    base.baked_version = ISO_PAINT_BAKE_VERSION;
    let page = IsoPaintBakedChunk::new(
        IsoPaintOwner::GeometryObject(Uuid::new_v4()),
        [1, 2, 3, 4],
        [0; 2],
    );
    base.baked_chunks.insert("overlap".into(), page.clone());
    let mut catalog = indexmap::IndexMap::new();
    for (id, value) in [(first_id, 80), (second_id, 180)] {
        let mut paint = IsoPaintLayer::default();
        paint.baked_version = ISO_PAINT_BAKE_VERSION;
        let mut page = page.clone();
        page.color_rgba = vec![value; page.color_rgba.len()].into();
        paint.baked_chunks.insert("overlap".into(), page);
        catalog.insert(id, paint);
    }
    let mut capture = Capture::default();
    assert!(
        capture
            .request_source(scope, Some(&base), false, Some(&map), &assets, &catalog, 0)
            .is_none()
    );
    assert!(capture.pending());
    assert!(
        capture
            .request_source(
                scope,
                Some(&base),
                false,
                Some(&map),
                &assets,
                &catalog,
                usize::MAX
            )
            .is_none()
    );
    let ready = drain(&mut capture, scope, &base, &map, &assets, &catalog, false);
    assert_eq!(
        ready.as_ref(),
        &expected(&base, &map, &assets, &catalog, false)
    );
    assert_eq!(ready.surface_instance_owners.len(), 8194);
    let same = capture
        .request_source(scope, Some(&base), false, Some(&map), &assets, &catalog, 0)
        .unwrap();
    assert!(Arc::ptr_eq(&ready, &same));
    assert!(ready.baked_chunks.same_storage(&same.baked_chunks));
    assert!(
        ready
            .surface_instance_owners
            .same_storage(&same.surface_instance_owners)
    );
    map.block_prop_instances[0].world_transform[3][0] = 8.;
    assert!(Arc::ptr_eq(
        &ready,
        &capture
            .request_source(scope, Some(&base), false, Some(&map), &assets, &catalog, 0)
            .unwrap()
    ));
    // Changing geometry source identity must cancel the partial owner list.
    let mut changed = assets.block_props.clone();
    changed.get_mut(&first_id).unwrap().parts[0]
        .geometry_source
        .geometry_objects_mut()[0]
        .id = Uuid::new_v4();
    assets.set_block_props(changed);
    assert!(
        capture
            .request_source(scope, Some(&base), false, Some(&map), &assets, &catalog, 1)
            .is_none()
    );
    // Replacement paint must be observed before that partial merge publishes.
    catalog.get_mut(&first_id).unwrap().visible = false;
    let replacement = drain(&mut capture, scope, &base, &map, &assets, &catalog, false);
    assert_eq!(
        replacement.as_ref(),
        &expected(&base, &map, &assets, &catalog, false)
    );
    assert_eq!(replacement.surface_instance_owners.len(), 1);
    assert!(!Arc::ptr_eq(&ready, &replacement));
    base.visible = false;
    catalog.get_mut(&second_id).unwrap().baked_version = 0;
    let hidden = drain(&mut capture, scope, &base, &map, &assets, &catalog, true);
    assert_eq!(
        hidden.as_ref(),
        &expected(&base, &map, &assets, &catalog, true)
    );
    assert!(!hidden.visible);
    assert!(hidden.baked_chunks.is_empty());
    assert_eq!(hidden.surface_instance_owners.len(), 1);
    capture.clear();
    assert!(!capture.pending());
    map.block_prop_instances.clear();
    let empty = capture
        .request_source(scope, Some(&base), true, Some(&map), &assets, &catalog, 0)
        .unwrap();
    assert_eq!(
        empty.as_ref(),
        &expected(&base, &map, &assets, &catalog, true)
    );
}
