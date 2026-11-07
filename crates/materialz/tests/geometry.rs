#![cfg(feature = "integration")]
use denrim_materials::geometry::*;
fn sword() -> Document {
    Document::load(denrim_materials::geometry::SWORD_EXAMPLE).unwrap()
}
#[test]
fn loft_surface_uvs_run_from_the_top_of_a_garment_downwards() {
    let mut doc = sword();
    doc.rows = vec![Row {
        id: "cloth".into(),
        label: "Cloth".into(),
        tiles: vec![Tile {
            id: "loft".into(),
            op: Operation::Loft {
                sections: vec![
                    Section {
                        center: [0., 0., 0.],
                        radius: [1., 1.],
                    },
                    Section {
                        center: [0., 1., 0.],
                        radius: [1., 1.],
                    },
                ],
                segments: 8,
                material: None,
                skin: None,
            },
        }],
    }];
    doc.output = "cloth".into();
    doc.grip = None;
    let mesh = doc.evaluate().unwrap().mesh;
    for vertex in &mesh.parts[0].vertices {
        if vertex.normal[1].abs() < 0.1 {
            assert_eq!(vertex.uv[1], 1. - vertex.position[1]);
        }
    }
}
#[test]
fn sword_roundtrips_and_preserves_part_material_and_grip_identity() {
    let doc = sword();
    assert_eq!(Document::load(&doc.save().unwrap()).unwrap(), doc);
    let result = doc.evaluate().unwrap();
    assert_eq!(result.mesh.parts.len(), 4);
    assert_eq!(result.grip.unwrap().translation, [0., -0.125, 0.]);
    assert_eq!(result.mesh.parts[0].id, "blade");
    assert_eq!(result.mesh.parts[0].surface.as_ref().unwrap().palette, 7);
    assert!(result.mesh.parts.iter().flat_map(|p| &p.vertices).all(|v| {
        v.position
            .iter()
            .chain(&v.normal)
            .chain(&v.uv)
            .all(|v| v.is_finite())
    }));
}
#[test]
fn editable_contours_change_geometry_without_affecting_other_parts() {
    let doc = sword();
    let before = doc.evaluate().unwrap();
    let mut changed = doc.clone();
    let Operation::Contour { points } = &mut changed.rows[3].tiles[0].op else {
        panic!()
    };
    points[3][1] = 1.2;
    let after = changed.evaluate().unwrap();
    assert!(after.mesh.bounds().1[1] > before.mesh.bounds().1[1]);
    assert_eq!(before.mesh.parts[1..], after.mesh.parts[1..]);
}
#[test]
fn forward_links_and_wrong_port_types_are_rejected() {
    let mut doc = sword();
    doc.rows.swap(0, 3);
    assert!(doc.evaluate().unwrap_err().to_string().contains("forward"));
    let mut doc = sword();
    doc.rows[7].tiles[0].op = Operation::Assemble {
        rows: vec!["steel".into()],
    };
    assert!(doc.evaluate().is_err());
    let mut doc = sword();
    doc.grip = Some("blade".into());
    assert!(doc.evaluate().is_err());
}
#[test]
fn contour_winding_concavity_and_invalid_shapes_are_handled() {
    let mut doc = sword();
    let points = vec![[-0.2, 0.], [0.2, 0.], [0.2, 0.2], [0., 0.1], [-0.2, 0.2]];
    doc.rows[3].tiles[0].op = Operation::Contour {
        points: points.clone(),
    };
    let first = doc.evaluate().unwrap().mesh;
    doc.rows[3].tiles[0].op = Operation::Contour {
        points: points.into_iter().rev().collect(),
    };
    assert_eq!(first.bounds(), doc.evaluate().unwrap().mesh.bounds());
    for points in [
        vec![[0., 0.], [1., 1.], [0., 1.], [1., 0.]],
        vec![[0., 0.], [0., 0.], [1., 1.]],
        vec![[0., 0.], [1., 0.], [2., 0.]],
    ] {
        doc.rows[3].tiles[0].op = Operation::Contour { points };
        assert!(doc.evaluate().is_err());
    }
}
#[test]
fn user_can_add_a_different_item_without_a_new_generator() {
    let mut doc = sword();
    doc.label = "Axe".into();
    doc.rows[3].tiles[0].op = Operation::Contour {
        points: vec![
            [-0.16, 0.],
            [0.2, -0.08],
            [0.28, 0.12],
            [0.2, 0.3],
            [-0.16, 0.2],
        ],
    };
    doc.rows[3].tiles[1].op = Operation::Extrude { depth: 0.04 };
    doc.output = "blade".into();
    assert_eq!(
        Document::load(&doc.save().unwrap())
            .unwrap()
            .evaluate()
            .unwrap()
            .mesh
            .parts
            .len(),
        1
    );
}

#[test]
fn an_attached_rig_keeps_child_skinning_and_keyed_branch_visibility() {
    let child = serde_json::json!({"version":1,"id":"mechanism","label":"Mechanism","rows":[
        {"id":"rig","label":"Rig","tiles":[{"id":"arm","op":{"type":"armature","joints":[
            {"id":"weapon-root","label":"Root","parent":null,"translation":[0,0,0],"rotation":[0,0,0]},
            {"id":"bolt","label":"Bolt","parent":"weapon-root","translation":[0,0.5,0],"rotation":[0,0,0]}
        ]}}]},
        {"id":"mesh","label":"Bolt mesh","tiles":[
            {"id":"box","op":{"type":"box","size":[0.1,0.1,0.1]}},
            {"id":"place","op":{"type":"transform","translation":[0,0.5,0],"rotation":[0,0,0],"scale":[1,1,1]}},
            {"id":"skin","op":{"type":"skin","rig":"rig","joints":["bolt"],"blend":1}}
        ]},
        {"id":"grip","label":"Grip","tiles":[{"id":"grip-frame","op":{"type":"grip","translation":[0,0,0],"rotation":[0,0,0]}}]}
    ],"output":"mesh","grip":"grip"});
    let root = serde_json::json!({"version":1,"id":"host","label":"Host","rows":[
        {"id":"rig","label":"Host rig","tiles":[{"id":"arm","op":{"type":"armature","joints":[
            {"id":"body","label":"Body","parent":null,"translation":[0,0,0],"rotation":[0,0,0]},
            {"id":"hand","label":"Hand","parent":"body","translation":[0.3,1,0],"rotation":[0,0,0]}
        ]}}]},
        {"id":"body","label":"Body","tiles":[{"id":"body-mesh","op":{"type":"profiles","rig":"rig","parts":[]}}]},
        {"id":"item","label":"Attached mechanism","tiles":[
            {"id":"source","op":{"type":"asset","source":"project:mechanism","document":child}},
            {"id":"attach","op":{"type":"attach","enabled":true,"rig":"rig","joint":"hand","translation":[0,0,0],"rotation":[0,0,0]}}
        ]},
        {"id":"all","label":"Combined","tiles":[{"id":"assemble","op":{"type":"assemble","rows":["body","item"]}}]},
        {"id":"shot","label":"Shot","tiles":[{"id":"shot-clip","op":{"type":"motion","rig":"all","duration":1,"looping":false,"tracks":[
            {"joint":"bolt","keys":[
                {"time":0,"translation":[0,0,0],"rotation":[0,0,0],"visible":true},
                {"time":0.5,"translation":[0,0.2,0],"rotation":[0,0,0],"visible":false},
                {"time":1,"translation":[0,0,0],"rotation":[0,0,0],"visible":true}
            ]}
        ]}}]}
    ],"output":"all","grip":null});
    let doc = Document::load(&root.to_string()).unwrap();
    assert_eq!(doc, Document::load(&doc.save().unwrap()).unwrap());
    let evaluation = doc.evaluate().unwrap();
    let mesh = evaluation.mesh;
    assert_eq!(mesh.rig.as_ref().unwrap().joints.len(), 4);
    let Value::Motion(motion) = &evaluation.rows["shot"] else {
        panic!()
    };
    let start = mesh.posed(Some(motion), 0.).unwrap();
    let firing = mesh.posed(Some(motion), 0.25).unwrap();
    let hidden = mesh.posed(Some(motion), 0.6).unwrap();
    let reloaded = mesh.posed(Some(motion), 1.).unwrap();
    assert!(hidden.mesh.parts[0].vertices.is_empty());
    assert_eq!(hidden.mesh.parts[0].id, start.mesh.parts[0].id);
    assert_eq!(
        start.mesh.parts[0].vertices,
        reloaded.mesh.parts[0].vertices
    );
    assert!(
        (firing.mesh.parts[0].vertices[0].position[1]
            - start.mesh.parts[0].vertices[0].position[1]
            - 0.1)
            .abs()
            < 0.00001
    );
    assert_eq!(firing.attachments["item"].translation, [0.3, 1., 0.]);
    let mut combined = mesh.clone();
    let Value::Mesh(body) = evaluation.rows["body"].clone() else {
        panic!()
    };
    combined.replace_parts(body).unwrap();
    assert_eq!(combined.rig, mesh.rig);
    assert_eq!(combined.attachments, mesh.attachments);
}
