use character_generator::*;
fn mannequin() -> CharacterAsset {
    Catalog::parse(include_str!("../examples/anatomy.toml"))
        .unwrap()
        .generate_with_options(
            "anatomy",
            &GenerationOptions {
                style: StylePreset::Natural,
                ..Default::default()
            },
        )
        .unwrap()
}
#[test]
fn walking_has_ground_contact_forward_knees_and_level_feet() {
    let a = mannequin();
    let index = |name: &str| a.joints.iter().position(|j| j.name == name).unwrap();
    let bind = a.bind_pose();
    for frame in 0..64 {
        let t = frame as f32 / 64.0;
        let pose = a.world_pose(&a.local_pose(Motion::Walk, t));
        for (side, shift) in [("left", 0.0), ("right", 0.5)] {
            let foot = index(&format!("{side}_foot"));
            let cycle = (t + shift).fract();
            let y = pose[foot].translation[1] - bind[foot].translation[1];
            if cycle >= 0.5 {
                assert!(y.abs() < a.height * 1e-5, "stance foot floats");
            } else {
                assert!(y >= -a.height * 1e-5);
            }
            assert!(
                pose[foot].rotation[..3].iter().all(|x| x.abs() < 1e-5),
                "foot tilts into ground"
            );
            let hip = pose[index(&format!("{side}_thigh"))].translation;
            let knee = pose[index(&format!("{side}_shin"))].translation;
            assert!(
                knee[2] > (hip[2] + pose[foot].translation[2]) * 0.5,
                "knee points behind leg"
            );
        }
    }
    let start = a.world_pose(&a.local_pose(Motion::Walk, 0.0));
    let end = a.world_pose(&a.local_pose(Motion::Walk, 1.0));
    assert_eq!(
        serde_json::to_vec(&start).unwrap(),
        serde_json::to_vec(&end).unwrap()
    );
    let pose = a.world_pose(&a.local_pose(Motion::Walk, 0.25));
    assert!(
        pose[index("left_foot")].translation[1]
            > bind[index("left_foot")].translation[1] + a.height * 0.025
    );
}
#[test]
fn hip_connections_do_not_spiral_and_neck_keeps_its_width() {
    let a = mannequin();
    let lower = a
        .primitives
        .iter()
        .find(|p| p.name == "lower_body")
        .unwrap();
    let hip = a.bind_pose()[a
        .joints
        .iter()
        .position(|j| j.name == "left_thigh")
        .unwrap()]
    .translation[0];
    let mut checked = 0;
    for tri in lower.indices.as_chunks::<3>().0 {
        let p = tri.map(|i| lower.vertices[i as usize].position);
        if p.iter()
            .all(|p| p[1] >= a.height * 0.390 - 1e-6 && p[1] <= a.height * 0.448 + 1e-6)
        {
            let sign = if p.iter().map(|p| p[0]).sum::<f32>() > 0.0 {
                1.0
            } else {
                -1.0
            };
            let angles = p.map(|p| p[2].atan2(p[0] - sign * hip));
            for (x, y) in [(0, 1), (1, 2), (2, 0)] {
                let delta = (angles[x] - angles[y]).abs();
                let shortest = delta.min(std::f32::consts::TAU - delta);
                assert!(shortest < 1.25, "hip face wraps around thigh axis");
            }
            checked += 1;
        }
    }
    assert!(checked > 20);
    let neck = a.primitives.iter().find(|p| p.name == "neck").unwrap();
    let min = neck
        .vertices
        .iter()
        .map(|v| v.position[1])
        .fold(f32::INFINITY, f32::min);
    let max = neck
        .vertices
        .iter()
        .map(|v| v.position[1])
        .fold(f32::NEG_INFINITY, f32::max);
    let middle = (min + max) * 0.5;
    let width = |y: f32| {
        let x: Vec<_> = neck
            .vertices
            .iter()
            .filter(|v| (v.position[1] - y).abs() < 1e-6)
            .map(|v| v.position[0])
            .collect();
        x.iter().copied().fold(f32::NEG_INFINITY, f32::max)
            - x.iter().copied().fold(f32::INFINITY, f32::min)
    };
    assert!(
        width(middle) > width(min) * 0.90,
        "neck pinches into a triangle"
    );
}

#[test]
fn walking_elbows_flex_forward_and_hair_wraps_the_head() {
    let a = mannequin();
    let index = |name: &str| a.joints.iter().position(|j| j.name == name).unwrap();
    for frame in 0..64 {
        let local = a.local_pose(Motion::Walk, frame as f32 / 64.0);
        for side in ["left", "right"] {
            // A downward forearm must flex toward +Z, the face direction.
            assert!(local[index(&format!("{side}_forearm"))].rotation[0] < 0.0);
        }
    }
    let head = a.primitives.iter().find(|p| p.name == "head").unwrap();
    let hair = a.primitives.iter().find(|p| p.name == "hair").unwrap();
    let lo = head
        .vertices
        .iter()
        .map(|v| v.position[1])
        .fold(f32::INFINITY, f32::min);
    let hi = head
        .vertices
        .iter()
        .map(|v| v.position[1])
        .fold(f32::NEG_INFINITY, f32::max);
    let back = hair
        .vertices
        .iter()
        .filter(|v| v.position[2] < 0.0)
        .map(|v| v.position[1])
        .fold(f32::INFINITY, f32::min);
    assert!(
        back < lo + (hi - lo) * 0.5,
        "hair leaves the back of the head bald"
    );
    let mouth = a.primitives.iter().find(|p| p.name == "mouth").unwrap();
    let mouth_lo = mouth
        .vertices
        .iter()
        .map(|v| v.position[1])
        .fold(f32::INFINITY, f32::min);
    let mouth_hi = mouth
        .vertices
        .iter()
        .map(|v| v.position[1])
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(
        mouth_hi - mouth_lo < (hi - lo) * 0.06,
        "neutral mouth gapes open"
    );
    // The pelvis should stay close to the posterior thigh silhouette, rather
    // than ending in a large isolated peak behind it.
    let lower = a
        .primitives
        .iter()
        .find(|p| p.name == "lower_body")
        .unwrap();
    let rear = lower
        .vertices
        .iter()
        .filter(|v| v.position[1] > a.height * 0.44)
        .map(|v| v.position[2])
        .fold(f32::INFINITY, f32::min);
    assert!(
        rear > -a.height * 0.075,
        "pelvis protrudes excessively backward"
    );
}
