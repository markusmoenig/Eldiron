use super::*;
fn target(size: (usize, usize)) -> CubeTarget {
    CubeTarget {
        view: Uuid::from_u128(803),
        scope: PanelScope {
            document: Uuid::from_u128(801),
            tool: Uuid::from_u128(802),
            revision: 1,
        },
        size,
        visible: true,
    }
}
fn ack(
    owner: &mut ViewCubeFrames,
    frame: &CubeFrame,
    target: CubeTarget,
    camera: &D3OrbitCamera,
) -> bool {
    #[cfg(feature = "native-ui")]
    {
        owner.presented(frame, &[frame.receipt()], target, camera)
    }
    #[cfg(not(feature = "native-ui"))]
    {
        owner.presented_reference(frame, target, camera)
    }
}
#[test]
fn cube_picking_uses_frozen_presented_faces_in_the_original_depth_order() {
    let mut owner = ViewCubeFrames::default();
    for size in [(100, 150), (800, 360)] {
        let target = target(size);
        for kind in 0..13 {
            owner.invalidate();
            let camera = super::super::native_tests::fixture(kind);
            let frame = owner.prepare(target, &camera).unwrap();
            let center = Vec2::new(size.0 as i32 - 52, 102);
            assert_eq!(owner.pick(target, &camera, center), CubePick::Unavailable);
            assert_eq!(
                owner.pick(target, &camera, Vec2::new(2, 2)),
                CubePick::Outside
            );
            assert!(ack(&mut owner, &frame, target, &camera));
            for dy in -41..=41 {
                for dx in -41..=41 {
                    let point = center + Vec2::new(dx, dy);
                    let expected = ViewCube::hit_test(
                        point,
                        TheDim::sized(size.0 as i32, size.1 as i32),
                        &camera,
                    )
                    .map_or(CubePick::Outside, CubePick::Face);
                    assert_eq!(
                        owner.pick(target, &camera, point),
                        expected,
                        "kind {kind}, {dx}/{dy}"
                    );
                }
            }
        }
    }
}
#[test]
fn cube_frames_reject_stale_camera_scope_layout_owner_and_retired_lifetimes() {
    let mut owner = ViewCubeFrames::default();
    let binding = target((800, 360));
    let camera = super::super::native_tests::fixture(1);
    let center = Vec2::new(748, 102);
    let frame = owner.prepare(binding, &camera).unwrap();
    let mut foreign = ViewCubeFrames::default();
    assert!(!ack(&mut foreign, &frame, binding, &camera));
    assert!(ack(&mut owner, &frame, binding, &camera));
    let moved = super::super::native_tests::fixture(2);
    assert_eq!(owner.pick(binding, &moved, center), CubePick::Unavailable);
    let pending = owner.prepare(binding, &moved).unwrap();
    assert!(!ack(&mut owner, &frame, binding, &camera));
    assert!(ack(&mut owner, &pending, binding, &moved));
    for changed in [
        CubeTarget {
            view: Uuid::new_v4(),
            ..binding
        },
        CubeTarget {
            scope: PanelScope {
                revision: 2,
                ..binding.scope
            },
            ..binding
        },
        CubeTarget {
            scope: PanelScope {
                document: Uuid::new_v4(),
                ..binding.scope
            },
            ..binding
        },
        CubeTarget {
            scope: PanelScope {
                tool: Uuid::new_v4(),
                ..binding.scope
            },
            ..binding
        },
        CubeTarget {
            size: (801, 360),
            ..binding
        },
    ] {
        assert_eq!(owner.pick(changed, &moved, center), CubePick::Unavailable);
        assert!(!ack(&mut owner, &pending, changed, &moved));
    }
    let new = owner.prepare(binding, &moved).unwrap();
    owner.invalidate();
    assert!(!ack(&mut owner, &new, binding, &moved));
    assert_eq!(owner.pick(binding, &moved, center), CubePick::Unavailable);
    let fresh = owner.prepare(binding, &moved).unwrap();
    assert!(ack(&mut owner, &fresh, binding, &moved));
    for hidden in [
        CubeTarget {
            visible: false,
            ..binding
        },
        CubeTarget {
            size: (99, 150),
            ..binding
        },
        CubeTarget {
            size: (100, 149),
            ..binding
        },
        CubeTarget {
            size: (0, 0),
            ..binding
        },
    ] {
        assert!(owner.prepare(hidden, &moved).is_none());
        assert!(!ack(&mut owner, &fresh, binding, &moved));
    }
    let mut invalid = moved.clone();
    invalid.azimuth = f32::NAN;
    assert!(owner.prepare(binding, &invalid).is_none());
}
#[test]
#[cfg(feature = "native-ui")]
fn cube_frame_receipts_require_the_matching_successful_root_marker() {
    let mut owner = ViewCubeFrames::default();
    let target = target((800, 360));
    let camera = D3OrbitCamera::new();
    let frame = owner.prepare(target, &camera).unwrap();
    assert!(!owner.presented(&frame, &[], target, &camera));
    let mut foreign = ViewCubeFrames::default();
    let other = foreign.prepare(target, &camera).unwrap();
    assert!(!owner.presented(&frame, &[other.receipt()], target, &camera));
    assert!(!owner.presented(&other, &[other.receipt()], target, &camera));
    assert!(owner.presented(&frame, &[frame.receipt()], target, &camera));
    assert!(!owner.presented(&frame, &[frame.receipt()], target, &camera));
}
