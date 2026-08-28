use rustc_hash::FxHashMap;
use rusterix::{
    Assets, Chunk, ChunkBuilder, GeometryFace, GeometryObject, GeometryObjectBuilder, Map,
    triangulate_geometry_polygon,
};
use scenevm::GeoId;
use uuid::Uuid;
use vek::{Vec2, Vec3};

fn face(indices: Vec<usize>, smoothing_group: u32) -> GeometryFace {
    GeometryFace {
        id: Uuid::new_v4(),
        paint_surface_id: None,
        indices,
        uvs: Vec::new(),
        paint_uvs: Vec::new(),
        auto_uv: true,
        texture_offset: Vec2::zero(),
        texture_scale: Vec2::one(),
        texture_rotation: 0.0,
        tile: None,
        tiles: FxHashMap::default(),
        surface_points: Vec::new(),
        surface_segments: Vec::new(),
        smoothing_group,
    }
}

#[test]
fn smoothing_groups_are_backward_compatible_in_serialized_faces() {
    let legacy_value = serde_json::to_value(face(vec![0, 1, 2], 0)).unwrap();
    assert!(legacy_value.get("smoothing_group").is_none());
    let legacy_face = serde_json::from_value::<GeometryFace>(legacy_value).unwrap();
    assert_eq!(legacy_face.smoothing_group, 0);

    let smooth_value = serde_json::to_value(face(vec![0, 1, 2], 9)).unwrap();
    assert_eq!(smooth_value["smoothing_group"], 9);
}

#[test]
fn concave_face_triangulation_covers_only_the_authored_polygon() {
    let points = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(3.0, 0.0, 0.0),
        Vec3::new(3.0, 0.0, 3.0),
        Vec3::new(2.0, 0.0, 3.0),
        Vec3::new(2.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 3.0),
        Vec3::new(0.0, 0.0, 3.0),
    ];
    let triangles = triangulate_geometry_polygon(&points).unwrap();
    let area = triangles
        .iter()
        .map(|&(a, b, c)| {
            let a = points[a];
            let b = points[b];
            let c = points[c];
            ((b.x - a.x) * (c.z - a.z) - (b.z - a.z) * (c.x - a.x)).abs() * 0.5
        })
        .sum::<f32>();

    assert_eq!(triangles.len(), 6);
    assert!((area - 7.0).abs() < 1e-5);
}

#[test]
fn a_collinear_corner_remains_a_renderable_polygon() {
    let points = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 2.0),
        Vec3::new(0.0, 0.0, 2.0),
    ];
    let triangles = triangulate_geometry_polygon(&points).unwrap();
    let area = triangles
        .iter()
        .map(|&(a, b, c)| {
            let a = points[a];
            let b = points[b];
            let c = points[c];
            ((b.x - a.x) * (c.z - a.z) - (b.z - a.z) * (c.x - a.x)).abs() * 0.5
        })
        .sum::<f32>();
    assert!((area - 4.0).abs() < 1e-5);
}

#[test]
fn matching_smoothing_groups_share_render_normals_across_a_corner() {
    let mut object = GeometryObject::new("Rounded corner");
    object.vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
    ];
    object.faces = vec![face(vec![0, 1, 2, 3], 1), face(vec![1, 4, 5, 2], 1)];
    let object_id = object.id;
    let mut map = Map::default();
    map.geometry_objects.push(object);
    let mut chunk = Chunk::new(Vec2::zero(), 16);
    let mut vmchunk = scenevm::Chunk::new(Vec2::zero(), 16);
    let mut builder = GeometryObjectBuilder;
    builder.build(&map, &Assets::default(), &mut chunk, &mut vmchunk);

    let polygons = &vmchunk.polys3d_map[&GeoId::GeometryObject(object_id)];
    assert_eq!(polygons.len(), 2);
    assert_eq!(polygons[0].normals[1], polygons[1].normals[0]);
    assert!(polygons[0].normals[1][0] < -0.6);
    assert!(polygons[0].normals[1][2] > 0.6);
}
