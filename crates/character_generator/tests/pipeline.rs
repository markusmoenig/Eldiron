use character_generator::*;
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "eldiron-character-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
const SOURCE: &str = include_str!("../examples/villager.toml");
fn asset() -> CharacterAsset {
    Catalog::parse(SOURCE)
        .unwrap()
        .generate("villager")
        .unwrap()
}
#[test]
fn rejects_unresolved_and_ambiguous_definitions() {
    for s in [
        SOURCE.replace("height = 1.72", "height = -2.0"),
        SOURCE.replace(
            "linen_shirt\", \"work_trousers",
            "missing\", \"work_trousers",
        ),
        SOURCE.replace("outfit = [", "outfit = [\"linen_shirt\", "),
        SOURCE.replace("generator = \"sword\"", "generator = \"spear\""),
        SOURCE.replace("height = 1.72", "heigth = 1.72"),
    ] {
        assert!(Catalog::parse(&s).is_err(), "{s}");
    }
    assert!(Catalog::parse(SOURCE).unwrap().generate("missing").is_err());
}
#[test]
fn skinning_preserves_bind_mesh_and_moves_weapon_with_hand() {
    let a = asset();
    let bind = a.bind_pose();
    let walk = a.world_pose(&a.local_pose(Motion::Walk, 0.25));
    for p in &a.primitives {
        for v in &p.vertices {
            let (pos, n) = a.posed_vertex(v, &bind, &bind);
            for i in 0..3 {
                assert!((pos[i] - v.position[i]).abs() < 1e-5);
                assert!((n[i] - v.normal[i]).abs() < 1e-5);
            }
            assert!((v.weights.iter().sum::<f32>() - 1.0).abs() < 1e-6);
            assert!(v.joints.iter().all(|j| (*j as usize) < a.joints.len()));
        }
    }
    let blade = a
        .primitives
        .iter()
        .find(|p| p.name == "iron_sword_blade")
        .unwrap();
    let hand = a
        .sockets
        .iter()
        .find(|s| s.name == "main_hand")
        .unwrap()
        .joint;
    let v = &blade.vertices[0];
    assert_eq!(v.joints[0] as usize, hand);
    let distance = |p: [f32; 3], q: [f32; 3]| (0..3).map(|i| (p[i] - q[i]).powi(2)).sum::<f32>();
    let (pos, _) = a.posed_vertex(v, &bind, &walk);
    assert!(
        (distance(pos, walk[hand].translation) - distance(v.position, bind[hand].translation))
            .abs()
            < 1e-5
    );
    assert!(distance(pos, v.position) > 0.001);
    for motion in [Motion::Idle, Motion::Walk] {
        for (s, e) in a
            .local_pose(motion, 0.0)
            .iter()
            .zip(a.local_pose(motion, motion.duration()))
        {
            assert_eq!(s.translation, e.translation);
            assert_eq!(s.rotation, e.rotation);
        }
    }
}
#[test]
fn glb_container_accessors_and_animation_are_consistent() {
    let dir = Temp::new();
    let a = asset();
    let path = dir.0.join("test.glb");
    export_glb(&a, &path).unwrap();
    let bytes = std::fs::read(path).unwrap();
    let u32_at = |o| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap()) as usize;
    assert_eq!(&bytes[..4], b"glTF");
    assert_eq!(u32_at(4), 2);
    assert_eq!(u32_at(8), bytes.len());
    let len = u32_at(12);
    assert_eq!(u32_at(16), 0x4e4f534a);
    let doc: Value = serde_json::from_slice(&bytes[20..20 + len]).unwrap();
    let bin = 20 + len;
    assert_eq!(u32_at(bin + 4), 0x004e4942);
    assert_eq!(bin + 8 + u32_at(bin), bytes.len());
    let views = doc["bufferViews"].as_array().unwrap();
    let accessors = doc["accessors"].as_array().unwrap();
    for accessor in accessors {
        let view = &views[accessor["bufferView"].as_u64().unwrap() as usize];
        let offset = view["byteOffset"].as_u64().unwrap() as usize;
        let len = view["byteLength"].as_u64().unwrap() as usize;
        assert_eq!(offset % 4, 0);
        assert!(offset + len <= doc["buffers"][0]["byteLength"].as_u64().unwrap() as usize);
        let components = match accessor["type"].as_str().unwrap() {
            "SCALAR" => 1,
            "VEC2" => 2,
            "VEC3" => 3,
            "VEC4" => 4,
            "MAT4" => 16,
            _ => panic!(),
        };
        let width = if accessor["componentType"] == 5123 {
            2
        } else {
            4
        };
        assert_eq!(
            len,
            accessor["count"].as_u64().unwrap() as usize * components * width
        );
    }
    assert_eq!(doc["animations"].as_array().unwrap().len(), a.motions.len());
    for animation in doc["animations"].as_array().unwrap() {
        let motion = a
            .motions
            .iter()
            .find(|m| m.name() == animation["name"].as_str().unwrap())
            .unwrap();
        assert_eq!(animation["extras"]["loop"], motion.looping());
    }
    assert_eq!(
        doc["skins"][0]["joints"].as_array().unwrap().len(),
        a.joints.len()
    );
    let inverse = &accessors[doc["skins"][0]["inverseBindMatrices"].as_u64().unwrap() as usize];
    let view = &views[inverse["bufferView"].as_u64().unwrap() as usize];
    let base = bin + 8 + view["byteOffset"].as_u64().unwrap() as usize;
    for (j, t) in a.bind_pose().iter().enumerate() {
        for i in 0..3 {
            let off = base + j * 64 + (12 + i) * 4;
            let v = f32::from_le_bytes(bytes[off..off + 4].try_into().unwrap());
            assert!((v + t.translation[i]).abs() < 1e-5);
        }
    }
}
#[test]
fn atlas_is_deterministic_has_all_views_masks_and_uncropped_pixels() {
    let a = asset();
    let one = Temp::new();
    let two = Temp::new();
    let options = BakeOptions {
        size: 48,
        frames: 4,
    };
    let meta = bake_atlas(&a, options, &one.0).unwrap();
    bake_atlas(&a, options, &two.0).unwrap();
    assert_eq!(meta.frames.len(), a.motions.len() * 8 * 4);
    assert_eq!(meta.size, [192, 48 * 8 * a.motions.len() as u32]);
    for file in ["atlas.png", "channels.png", "atlas.json", "preview.html"] {
        assert_eq!(
            std::fs::read(one.0.join(file)).unwrap(),
            std::fs::read(two.0.join(file)).unwrap()
        );
    }
    for motion in &a.motions {
        assert_eq!(meta.looping[motion.name()], motion.looping());
        let last = meta
            .frames
            .iter()
            .rfind(|f| f.motion == motion.name() && f.direction == "front")
            .unwrap();
        if motion.looping() {
            assert!(last.time < motion.duration());
        } else {
            assert_eq!(last.time, motion.duration());
        }
    }
    let rgba = image::open(one.0.join("atlas.png")).unwrap().to_rgba8();
    let mask = image::open(one.0.join("channels.png")).unwrap().to_luma8();
    for f in meta.frames {
        assert_eq!(f.sockets.len(), 2);
        let [ox, oy, w, h] = f.rect;
        let mut count = 0;
        for y in 0..h {
            for x in 0..w {
                let visible = rgba.get_pixel(ox + x, oy + y)[3] > 0;
                assert_eq!(visible, mask.get_pixel(ox + x, oy + y)[0] > 0);
                if visible {
                    count += 1;
                    assert!(x > 0 && y > 0 && x < w - 1 && y < h - 1);
                }
            }
        }
        assert!(count > 40);
    }
    assert!(bake_atlas(&a, BakeOptions { size: 0, frames: 4 }, &one.0).is_err());
    assert!(
        bake_atlas(
            &a,
            BakeOptions {
                size: 512,
                frames: 32
            },
            &one.0
        )
        .is_err()
    );
}
#[test]
fn actions_hold_final_poses_and_have_distinct_useful_silhouettes() {
    let a = asset();
    let index = |name: &str| a.joints.iter().position(|j| j.name == name).unwrap();
    let bind = a.bind_pose();
    for motion in [
        Motion::Cast,
        Motion::Use,
        Motion::Sit,
        Motion::Death,
        Motion::Attack,
        Motion::Parry,
    ] {
        assert!(!motion.looping());
        for (end, held) in a
            .local_pose(motion, motion.duration())
            .iter()
            .zip(a.local_pose(motion, 3.0 * motion.duration()))
        {
            assert_eq!(end.translation, held.translation);
            assert_eq!(end.rotation, held.rotation);
        }
        for frame in 0..33 {
            let pose = a.world_pose(&a.local_pose(motion, frame as f32 / 32.0 * motion.duration()));
            for p in &a.primitives {
                for v in &p.vertices {
                    let (pos, n) = a.posed_vertex(v, &bind, &pose);
                    assert!(pos.iter().all(|x| x.is_finite()));
                    assert!((n.iter().map(|x| x * x).sum::<f32>() - 1.0).abs() < 1e-5);
                }
            }
        }
    }
    let cast = a.world_pose(&a.local_pose(Motion::Cast, Motion::Cast.duration() * 0.5));
    for hand in ["left_hand", "right_hand"] {
        assert!(
            cast[index(hand)].translation[1] > bind[index(hand)].translation[1] + a.height * 0.15
        );
    }
    let use_pose = a.world_pose(&a.local_pose(Motion::Use, 0.5));
    assert!(use_pose[index("right_hand")].translation[2] > a.height * 0.15);
    let seated = a.world_pose(&a.local_pose(Motion::Sit, Motion::Sit.duration()));
    assert!(
        seated[index("root")].translation[1] < bind[index("root")].translation[1] - a.height * 0.2
    );
    for side in ["left", "right"] {
        let foot = index(&format!("{side}_foot"));
        assert!((seated[foot].translation[1] - bind[foot].translation[1]).abs() < 1e-5);
    }
    let dead = a.world_pose(&a.local_pose(Motion::Death, Motion::Death.duration()));
    assert!(
        dead[index("head")].translation[2] < dead[index("root")].translation[2] - a.height * 0.25
    );
    assert!((dead[index("head")].translation[1] - dead[index("root")].translation[1]).abs() < 1e-5);
}
#[test]
fn attack_swings_main_hand_and_parry_brings_shield_forward() {
    let a = Catalog::parse(include_str!("../examples/guard.toml"))
        .unwrap()
        .generate("guard")
        .unwrap();
    let bind = a.bind_pose();
    let wind = a.world_pose(&a.local_pose(Motion::Attack, 0.32));
    let strike = a.world_pose(&a.local_pose(Motion::Attack, 0.52));
    let blade = a
        .primitives
        .iter()
        .find(|p| p.name == "iron_sword_blade")
        .unwrap();
    let tip = blade
        .vertices
        .iter()
        .max_by(|a, b| a.position[2].total_cmp(&b.position[2]))
        .unwrap();
    let (raised, _) = a.posed_vertex(tip, &bind, &wind);
    let (struck, _) = a.posed_vertex(tip, &bind, &strike);
    assert!(raised[1] > struck[1] + a.height * 0.3);
    let guard = a.world_pose(&a.local_pose(Motion::Parry, Motion::Parry.duration() * 0.35));
    let shield = a
        .primitives
        .iter()
        .find(|p| p.name == "round_shield")
        .unwrap();
    let front = shield.vertices.iter().find(|v| v.normal[2] > 0.99).unwrap();
    let (pos, n) = a.posed_vertex(front, &bind, &guard);
    assert!(n[2] > 0.95);
    assert!(pos[2] > front.position[2] + a.height * 0.10);
    for motion in [Motion::Attack, Motion::Parry] {
        for (s, e) in a
            .local_pose(motion, 0.0)
            .iter()
            .zip(a.local_pose(motion, motion.duration()))
        {
            assert_eq!(s.translation, e.translation);
            assert_eq!(s.rotation, e.rotation);
        }
    }
}
#[test]
fn action_weight_shifts_bend_knees_without_sliding_or_tilting_feet() {
    let catalog = Catalog::parse(include_str!("../examples/guard.toml")).unwrap();
    for style in [
        StylePreset::Blocky,
        StylePreset::Stylized,
        StylePreset::Natural,
    ] {
        let a = catalog
            .generate_with_options(
                "guard",
                &GenerationOptions {
                    style,
                    ..Default::default()
                },
            )
            .unwrap();
        let bind = a.bind_pose();
        let index = |name: &str| a.joints.iter().position(|j| j.name == name).unwrap();
        for motion in [Motion::Attack, Motion::Parry, Motion::Cast, Motion::Use] {
            let mid = a.local_pose(motion, motion.duration() * 0.4);
            assert!(
                mid[index("root")].translation[1]
                    < a.joints[index("root")].translation[1] - a.height * 0.01
            );
            assert!(mid[index("left_shin")].rotation[0] > 0.10);
            assert!(mid[index("right_shin")].rotation[0] > 0.10);
            for frame in 0..65 {
                let pose =
                    a.world_pose(&a.local_pose(motion, frame as f32 / 64.0 * motion.duration()));
                for side in ["left", "right"] {
                    let foot = index(&format!("{side}_foot"));
                    for (p, b) in pose[foot].translation.iter().zip(bind[foot].translation) {
                        if motion == Motion::Attack && side == "right" {
                            continue;
                        }
                        assert!((p - b).abs() < 1e-5, "{motion:?} frame {frame}: foot drift");
                    }
                    for (r, b) in pose[foot].rotation.iter().zip(bind[foot].rotation) {
                        assert!((r - b).abs() < 1e-5, "{motion:?} frame {frame}: foot tilt");
                    }
                }
            }
        }
    }
}

#[test]
fn attack_steps_right_foot_forward_plants_for_strike_and_returns() {
    let a = Catalog::parse(include_str!("../examples/guard.toml"))
        .unwrap()
        .generate("guard")
        .unwrap();
    let bind = a.bind_pose();
    let index = |name: &str| a.joints.iter().position(|j| j.name == name).unwrap();
    let right = index("right_foot");
    let left = index("left_foot");
    let pose = |t| a.world_pose(&a.local_pose(Motion::Attack, t));
    let lifting = pose(0.27);
    assert!(lifting[right].translation[1] > bind[right].translation[1] + a.height * 0.02);
    let planted = pose(0.50);
    let impact = pose(0.60);
    assert!(planted[right].translation[2] > bind[right].translation[2] + a.height * 0.11);
    for (p, i) in planted[right]
        .translation
        .iter()
        .zip(impact[right].translation)
    {
        assert!(
            (p - i).abs() < 1e-5,
            "foot must hold its planted position during the strike"
        );
    }
    assert!((planted[right].translation[1] - bind[right].translation[1]).abs() < 1e-5);
    assert!(pose(0.79)[right].translation[1] > bind[right].translation[1] + a.height * 0.015);
    for (end, rest) in pose(1.0)[right]
        .translation
        .iter()
        .zip(bind[right].translation)
    {
        assert!((end - rest).abs() < 1e-5);
    }
    for frame in 0..101 {
        let p = pose(frame as f32 / 100.0);
        assert!(p[right].translation[1] >= bind[right].translation[1] - 1e-5);
        for (p, rest) in p[left].translation.iter().zip(bind[left].translation) {
            assert!((p - rest).abs() < 1e-5, "support foot must stay planted");
        }
    }
}
