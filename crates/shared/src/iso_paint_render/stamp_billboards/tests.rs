use super::*;
use crate::iso_paint::IsoPaintPoint;

fn source(count: usize) -> IsoPaintLayer {
    let mut layer = IsoPaintLayer::default();
    layer.active_brush = "grass".into();
    layer.active_size = 1.;
    layer.active_color = [24, 190, 72, 255];
    layer.add_stamp(IsoPaintPoint::new([8, 8], Some(Vec3::zero()), None));
    let chunk = layer.chunks.values_mut().next().unwrap();
    let prototype = chunk.stamps[0].clone();
    chunk.stamps = (0..count)
        .map(|index| {
            let mut stamp = prototype.clone();
            stamp.id = Uuid::new_v4();
            stamp.order = index as u64;
            stamp.world = Some([index as f32 * 0.01, 0., 0.]);
            stamp
        })
        .collect();
    layer
}
#[test]
fn identical_original_sprites_share_storage_without_merging_ordered_placements() {
    let layer = source(300);
    let before = serde_json::to_vec(&layer).unwrap();
    let originals = layer.chunks.values().next().unwrap().stamps.clone();
    let camera = Camera3D::iso();
    let reference: Vec<_> = originals
        .iter()
        .map(|stamp| stamp_billboard(stamp, stamp.world.unwrap(), camera, 128).unwrap())
        .collect();
    let mut job = StampBillboardPreparation::new(layer.clone(), camera, 128, 1024 * 1024);
    assert!(!job.advance_with(0, |s| s.world).unwrap());
    let mut yields = 0;
    while !job.advance_with(2, |s| s.world).unwrap() {
        yields += 1;
    }
    assert!(yields >= 150);
    let result = job.finish().unwrap();
    assert_eq!(result.sprites.len(), 1);
    assert_eq!(result.instances.len(), 300);
    for (actual, (sprite, expected)) in result.instances.iter().zip(reference) {
        let shared = &result.sprites[actual.sprite_index as usize];
        assert_eq!(shared.rgba, sprite.rgba);
        assert_eq!((shared.width, shared.height), (sprite.width, sprite.height));
        assert_eq!(actual.center, expected.center);
        assert_eq!(actual.width, expected.width);
        assert_eq!(actual.height, expected.height);
        assert_eq!(actual.flags, expected.flags);
    }
    assert!(result.source_bytes < 20000);
    assert_eq!(serde_json::to_vec(&layer).unwrap(), before);
}
#[test]
fn budget_and_invalid_dimensions_never_publish_a_partial_original_set() {
    let camera = Camera3D::iso();
    let mut small = StampBillboardPreparation::new(source(2), camera, 128, 33);
    assert!(small.advance_with(4, |s| s.world).is_err());
    assert!(small.finish().is_err());
    assert!(
        StampBillboardPreparation::new(source(0), camera, 128, 1024)
            .finish()
            .is_err()
    );
    let mut huge = source(1);
    huge.chunks.values_mut().next().unwrap().stamps[0].size = f32::INFINITY;
    let mut job = StampBillboardPreparation::new(huge, camera, 128, 1024 * 1024);
    assert!(matches!(
        job.advance_with(4, |s| s.world),
        Err(StampBillboardError::RasterDimensions(..))
    ));
    assert!(job.advance_with(4, |s| s.world).is_err());
    assert!(job.finish().is_err());
}
#[test]
fn invisible_or_unanchored_stamps_do_not_allocate_original_sprites() {
    for invisible in [true, false] {
        let mut layer = source(2);
        if invisible {
            layer.visible = false;
        } else {
            for stamp in &mut layer.chunks.values_mut().next().unwrap().stamps {
                stamp.world = None;
            }
        }
        let mut job = StampBillboardPreparation::new(layer, Camera3D::iso(), 128, 1024);
        while !job.advance_with(2, |s| s.world).unwrap() {}
        let output = job.finish().unwrap();
        assert!(output.sprites.is_empty() && output.instances.is_empty());
    }
}
