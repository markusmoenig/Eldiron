use eldiron_shared::project_io::{decode_project, encode_project};
use rusterix::{GeometryObjectKind, triangulate_geometry_polygon};
use std::time::Instant;

#[test]
fn stonefall_authored_geometry_survives_loading_and_saving() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes = std::fs::read(root.join("test_projects/StonefallDungeon.eldiron")).unwrap();
    assert_eq!(
        bytes,
        std::fs::read(root.join("starters/projects/StonefallDungeon.eldiron")).unwrap()
    );
    let start = Instant::now();
    let project = decode_project(&bytes).expect("Stonefall loads through the real project decoder");
    eprintln!("Stonefall decode: {:?}", start.elapsed());
    let config: toml::Value = toml::from_str(&project.config).unwrap();
    assert_eq!(project.target_fps, 60);
    assert_eq!(config["game"]["target_fps"].as_integer(), Some(60));
    let start_region = config["game"]["start_region"].as_str().unwrap();
    assert!(
        project
            .regions
            .iter()
            .any(|region| region.name == start_region),
        "start_region must resolve through the runtime region names"
    );
    let map = &project.regions[0].map;
    assert!(map.wall_assemblies.is_empty());
    assert!(map.geometry_objects.len() < 350);
    assert!(
        map.geometry_objects
            .iter()
            .map(|o| o.faces.len())
            .sum::<usize>()
            < 2000
    );
    let mut floor_area = 0.0_f32;
    let mut min_floor = f32::MAX;
    let mut max_floor = f32::MIN;
    for object in &map.geometry_objects {
        assert_eq!(object.kind, GeometryObjectKind::Brush);
        assert!(
            !object
                .tags
                .iter()
                .any(|tag| tag == "eldiron_generated_wall")
        );
        for face in &object.faces {
            let points = face
                .indices
                .iter()
                .map(|i| object.vertices[*i])
                .collect::<Vec<_>>();
            let triangles =
                triangulate_geometry_polygon(&points).expect("authored faces triangulate");
            assert!(!triangles.is_empty());
            assert_eq!(face.uvs.len(), face.indices.len());
        }
        if object.group == "Stonefall / Floors" {
            // Only the top polygon: the solid slab's underside must not duplicate coverage.
            let points = object.faces[0]
                .indices
                .iter()
                .map(|i| object.vertices[*i])
                .collect::<Vec<_>>();
            floor_area += triangulate_geometry_polygon(&points)
                .unwrap()
                .iter()
                .map(|&(a, b, c)| {
                    (points[b] - points[a]).cross(points[c] - points[a]).y.abs() * 0.5
                })
                .sum::<f32>();
            for point in points {
                min_floor = min_floor.min(point.y);
                max_floor = max_floor.max(point.y);
            }
        }
    }
    assert!(
        (floor_area - 593.0).abs() < 0.01,
        "walkable area: {floor_area}"
    );
    assert!(
        map.block_prop_instances
            .iter()
            .all(|i| i.host_attachment.is_none())
    );
    assert!((min_floor + 0.4).abs() < 1e-5);
    assert!((max_floor - 0.6).abs() < 1e-5);
    assert_eq!(map.block_prop_instances.len(), 9);
    let gate = project
        .block_props
        .values()
        .find(|asset| asset.name == "Stonefall Bone Gate")
        .unwrap();
    assert_eq!(gate.parts.len(), 2);
    assert_eq!(
        gate.components[0].properties.get_str("required_item"),
        Some("Bone Key")
    );
    let encoded = encode_project(&project).expect("Stonefall saves");
    let restored = decode_project(&encoded).expect("Stonefall reopens");
    assert_eq!(
        map.geometry_objects,
        restored.regions[0].map.geometry_objects
    );
}
