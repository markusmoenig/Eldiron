use super::*;

#[test]
fn shared_catalogue_preparation_matches_current_selection_reference() {
    let mut layer = IsoPaintLayer::default();
    let geo = [7, 0, 0, 1];
    let owner = IsoPaintOwner::Sector(7);
    for x in [-64, 0] {
        let mut page = IsoPaintBakedChunk::new(owner.clone(), geo, [x, 0]);
        for pixel in page.color_rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[80, 150, 220, if x < 0 { 160 } else { 255 }]);
        }
        if x < 0 {
            page.material_rgba[..4].copy_from_slice(&[254, 20, 128, 255]);
        }
        layer.baked_chunks.insert(x.to_string(), page);
    }
    let mut selection = IsoPaintSelection::new(owner.clone());
    selection.set_pixel(owner.clone(), geo, [1, 1]);
    selection.set_pixel(owner, geo, [-65, 4]);
    layer.set_active_selection(selection);
    layer.surface_instance_owners.push(IsoPaintOwner::Sector(9));
    let frozen = layer.presentation_source(true);
    let request = SurfacePaintRequest::capture(&frozen);
    assert!(request.chunks.same_storage(&layer.baked_chunks));
    assert!(request.owners.same_storage(&layer.surface_instance_owners));
    assert!(
        request
            .selection
            .as_ref()
            .unwrap()
            .chunks
            .same_storage(&layer.active_selection().unwrap().chunks)
    );
    let display = IsoPaintRenderer::surface_chunks_with_selection(&frozen);
    let (width, height, color, material, entries) =
        IsoPaintRenderer::build_surface_paint_atlas(&display).unwrap();
    let mut prep = request.prepare(1024 * 1024);
    assert!(
        prep.display.is_empty(),
        "constructor must not copy the catalogue"
    );
    assert!(!prep.advance(0).unwrap());
    assert!(!prep.advance(1).unwrap());
    assert_eq!(prep.display.len(), 1, "one unit copies one descriptor");
    // Pending work owns the original map and bytes even after another edit.
    layer.baked_chunks.values_mut().next().unwrap().color_rgba[0] = 255;
    layer
        .selection_masks
        .get_mut(&layer.active_selection_id.unwrap())
        .unwrap()
        .chunks
        .values_mut()
        .next()
        .unwrap()
        .mask[0] = 255;
    let mut advances = 1;
    while !prep.advance(1).unwrap() {
        advances += 1;
        assert!(advances < 2000);
    }
    let publication = prep.finish().unwrap();
    let source = publication.source().unwrap();
    assert_eq!([source.width, source.height], [width, height]);
    assert_eq!(
        source.alpha_geo_ids.as_slice(),
        IsoPaintRenderer::surface_paint_alpha_geo_ids(&frozen)
    );
    assert_eq!(source.entries.as_ref(), entries);
    for page in &source.pages {
        for row in 0..ISO_PAINT_BAKED_CHUNK_SIZE as usize {
            let start = ((page.atlas_origin[1] as usize + row) * width as usize
                + page.atlas_origin[0] as usize)
                * 4;
            let local = row * ISO_PAINT_BAKED_CHUNK_SIZE as usize * 4;
            assert_eq!(
                &page.color_rgba[local..local + 256],
                &color[start..start + 256]
            );
            assert_eq!(
                &page.material_rgba[local..local + 256],
                &material[start..start + 256]
            );
        }
    }
    assert!(publication.matches(&SurfacePaintRequest::capture(&frozen)));
    assert!(!publication.matches(&SurfacePaintRequest::capture(
        &layer.presentation_source(true)
    )));
}
