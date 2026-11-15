use super::*;
use crate::iso_paint::{IsoPaintBakedChunk, IsoPaintOwner};

fn layer(count: usize) -> IsoPaintLayer {
    let mut layer = IsoPaintLayer::default();
    let mut prototype = IsoPaintBakedChunk::new(IsoPaintOwner::Sector(7), [7, 0, 0, 1], [0, 0]);
    prototype.color_rgba[..4].copy_from_slice(&[23, 141, 61, 192]);
    for index in 0..count {
        let mut chunk = prototype.clone();
        chunk.origin = [(index as i32 - count as i32 / 2) * SIZE as i32, -64];
        chunk.revision = index as u64;
        layer.baked_chunks.insert(index.to_string(), chunk);
    }
    layer
}

#[test]
fn complete_authored_keys_keep_shader_slots_and_share_frozen_original_planes() {
    let mut layer = layer(37);
    let index = PaintPageIndex::from_layer(&layer, 1024 * 1024).unwrap();
    assert_eq!(index.source_count(), 37);
    let expected = Builder::surface_paint_entry_table(
        layer
            .baked_chunks
            .values()
            .map(|chunk| Raster3DSurfacePaintEntry {
                geo: chunk.paint_geo,
                uv_origin: chunk.origin,
                uv_size: [SIZE; 2],
                atlas_rect: [0; 4],
            })
            .collect(),
    );
    assert_eq!(index.unloaded_entries(), expected);
    for source in layer.baked_chunks.values() {
        let slot = index.slot(source.paint_geo, source.origin).unwrap();
        let frozen = index.page(slot).unwrap();
        assert_eq!(frozen.revision, source.revision);
        assert_eq!(frozen.color.as_ptr(), source.color_rgba.as_ptr());
        assert_eq!(frozen.material.as_ptr(), source.material_rgba.as_ptr());
    }
    assert!(index.slot([9; 4], [0, 0]).is_none());
    let first = layer.baked_chunks.values_mut().next().unwrap();
    let slot = index.slot(first.paint_geo, first.origin).unwrap();
    first.color_rgba[0] = 99;
    first.material_rgba[1] = 42;
    first.origin = [123, 456];
    drop(layer);
    let original = index.page(slot).unwrap();
    assert_eq!(&original.color[..4], &[23, 141, 61, 192]);
    assert_eq!(&original.material[..4], &[254, 0, 0, 0]);
    assert_eq!(original.uv_origin, [-18 * 64, -64]);
}

#[test]
fn large_logical_sources_need_only_complete_metadata_and_selected_residency() {
    // Repeated immutable planes keep this fixture small. Real original pages
    // may be distinct; indexing never clones their bytes in either case.
    let layer = layer(4097);
    let index = PaintPageIndex::from_layer(&layer, 2 * 1024 * 1024).unwrap();
    assert!(index.logical_source_bytes() > 128 * 1024 * 1024);
    assert!(index.table_bytes() < 1024 * 1024);
    assert_eq!(index.source_count(), layer.baked_chunks.len());
    assert!(
        index
            .unloaded_entries()
            .iter()
            .all(|entry| entry.atlas_rect == [0; 4])
    );
    let selected: Vec<_> = layer
        .baked_chunks
        .values()
        .take(2)
        .enumerate()
        .map(|(at, chunk)| {
            (
                index.slot(chunk.paint_geo, chunk.origin).unwrap(),
                [at as u32 * SIZE, 0],
            )
        })
        .collect();
    let entries = index.resident_entries([128, 64], &selected).unwrap();
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry.atlas_rect[2] != 0)
            .count(),
        2
    );
    for chunk in layer.baked_chunks.values() {
        let slot = index.slot(chunk.paint_geo, chunk.origin).unwrap();
        assert_eq!(
            index.page(slot).unwrap().color.as_ptr(),
            chunk.color_rgba.as_ptr()
        );
        assert_eq!(entries[slot as usize].uv_size, [64; 2]);
    }
    // Evicting all pages cannot remove original keys or change source slots.
    assert_eq!(
        index.resident_entries([64, 64], &[]).unwrap(),
        index.unloaded_entries()
    );
    assert!(PaintPageIndex::from_layer(&layer, index.table_bytes() - 1).is_err());
}

#[test]
fn malformed_pages_and_ambiguous_keys_fail_without_partial_source_publication() {
    let mut layer = layer(1);
    let first = layer.baked_chunks.values().next().unwrap().clone();
    layer.baked_chunks.insert("duplicate".into(), first);
    assert!(
        PaintPageIndex::from_layer(&layer, 1024)
            .err()
            .unwrap()
            .contains("duplicate")
    );
    layer.baked_chunks.shift_remove("duplicate");
    layer.baked_chunks.values_mut().next().unwrap().origin[0] = -1;
    assert!(
        PaintPageIndex::from_layer(&layer, 1024)
            .err()
            .unwrap()
            .contains("unaligned")
    );
    let first = layer.baked_chunks.values_mut().next().unwrap();
    first.origin[0] = -64;
    first.material_rgba = vec![0; 4].into();
    assert!(
        PaintPageIndex::from_layer(&layer, 1024)
            .err()
            .unwrap()
            .contains("dimensions")
    );
}

#[test]
fn resident_rectangles_cannot_alias_sources_or_escape_the_atlas() {
    let layer = layer(2);
    let index = PaintPageIndex::from_layer(&layer, 1024).unwrap();
    let mut source = layer.baked_chunks.values();
    let a = source.next().unwrap();
    let b = source.next().unwrap();
    let a = index.slot(a.paint_geo, a.origin).unwrap();
    let b = index.slot(b.paint_geo, b.origin).unwrap();
    for residents in [
        vec![(a, [0, 0]), (b, [0, 0])],
        vec![(a, [0, 0]), (a, [64, 0])],
        vec![(a, [1, 0])],
        vec![(a, [128, 0])],
        vec![(u32::MAX, [0, 0])],
        vec![(a, [u32::MAX - 63, 0])],
    ] {
        assert!(index.resident_entries([128, 64], &residents).is_err());
    }
    assert_eq!(
        index
            .resident_entries([128, 64], &[(a, [0, 0]), (b, [64, 0])])
            .unwrap()[b as usize]
            .atlas_rect,
        [64, 0, 64, 64]
    );
}

#[test]
fn invisible_or_obsolete_paint_is_ignored_without_rewriting_the_layer() {
    let mut layer = layer(1);
    for obsolete in [false, true] {
        layer.visible = obsolete;
        if obsolete {
            layer.baked_version = ISO_PAINT_BAKE_VERSION.wrapping_sub(1);
        }
        let before = serde_json::to_vec(&layer).unwrap();
        let index = PaintPageIndex::from_layer(&layer, 1024).unwrap();
        assert_eq!(index.source_count(), 0);
        assert!(index.alpha_geo_ids().is_empty());
        assert!(
            index
                .unloaded_entries()
                .iter()
                .all(|entry| entry.uv_size == [0; 2])
        );
        assert_eq!(serde_json::to_vec(&layer).unwrap(), before);
    }
}

#[test]
fn unloaded_material_pages_retain_transparency_and_instance_classification() {
    let mut layer = layer(2);
    layer
        .baked_chunks
        .values_mut()
        .next()
        .unwrap()
        .material_rgba[..4]
        .copy_from_slice(&[254, 20, 128, 255]);
    layer.surface_instance_owners = vec![IsoPaintOwner::Sector(9), IsoPaintOwner::Sector(7)].into();
    let index = PaintPageIndex::from_layer(&layer, 1024).unwrap();
    assert_eq!(index.alpha_geo_ids(), &[GeoId::Sector(7), GeoId::Sector(9)]);
    assert!(
        index
            .unloaded_entries()
            .iter()
            .all(|entry| entry.atlas_rect == [0; 4])
    );
    // Neither temporary eviction nor dropping the authored owner may reclassify
    // geometry as opaque before its actual material page has been requested.
    drop(layer);
    let _entries = index.resident_entries([64, 64], &[]).unwrap();
    assert_eq!(index.alpha_geo_ids(), &[GeoId::Sector(7), GeoId::Sector(9)]);
}

#[test]
fn cooperative_preparation_matches_all_slots_alpha_and_shared_planes() {
    let mut source = layer(2049);
    source
        .baked_chunks
        .values_mut()
        .last()
        .unwrap()
        .material_rgba[..4]
        .copy_from_slice(&[254, 20, 128, 255]);
    source.surface_instance_owners = (8..55).map(IsoPaintOwner::Sector).collect();
    let expected_alpha = Builder::surface_paint_alpha_geo_ids(&source);
    let expected = PaintPageIndex::from_layer(&source, 1024 * 1024).unwrap();
    let original = source.baked_chunks.clone();
    let mut preparation = PaintPageIndexBuilder::new(source, 1024 * 1024).unwrap();
    assert!(!preparation.advance(0).unwrap());
    let mut advances = 1;
    while !preparation.advance(16).unwrap() {
        advances += 1;
    }
    assert!(
        advances > 128,
        "preparation must yield across original pages"
    );
    let (index, residual) = preparation.finish().unwrap();
    assert!(residual.baked_chunks.is_empty());
    assert_eq!(index.unloaded_entries(), expected.unloaded_entries());
    assert_eq!(index.alpha_geo_ids(), expected_alpha);
    for chunk in original.values() {
        let slot = index.slot(chunk.paint_geo, chunk.origin).unwrap();
        assert_eq!(
            index.page(slot).unwrap().color.as_ptr(),
            chunk.color_rgba.as_ptr()
        );
        assert_eq!(
            index.page(slot).unwrap().material.as_ptr(),
            chunk.material_rgba.as_ptr()
        );
    }
}

#[test]
fn unfinished_or_failed_preparation_cannot_publish_an_index() {
    assert!(
        PaintPageIndexBuilder::new(layer(2), 1024)
            .unwrap()
            .finish()
            .is_err()
    );
    let mut source = layer(2);
    source
        .baked_chunks
        .values_mut()
        .last()
        .unwrap()
        .material_rgba = vec![0; 4].into();
    let mut preparation = PaintPageIndexBuilder::new(source, 1024).unwrap();
    let error = preparation.advance(16).unwrap_err();
    assert!(error.contains("dimensions"));
    assert_eq!(preparation.advance(16).unwrap_err(), error);
    assert!(preparation.finish().is_err());
}
