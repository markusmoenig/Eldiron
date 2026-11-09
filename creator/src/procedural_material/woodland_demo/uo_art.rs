//! A deliberately authored visual benchmark, built entirely from editable MZ parts.
//! Original reference images are never used as textures. Outputs remain ignored.
use super::*;
use denrim_materials::geometry::Document;
fn art_project_file(root: &Path) -> std::path::PathBuf {
    let project = root.join("test_projects/UnendingNight.eldiron");
    if project.exists() { project } else { root.join("generated/uo-art-study/WayfarersRest.eldiron") }
}
fn asset(name: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    std::fs::read_to_string(root.join("crates/shared/examples/iso_2d/uo_art").join(name)).unwrap()
}
fn enable_daylight(p: &mut Project) {
    let mut config: toml::Table = toml::from_str(&p.config).unwrap();
    config
        .entry("simulation")
        .or_insert_with(|| toml::Value::Table(Default::default()))
        .as_table_mut()
        .unwrap()
        .insert("enabled".into(), toml::Value::Boolean(true));
    config.get_mut("simulation").unwrap().as_table_mut().unwrap().insert("night_ambient_strength".into(),toml::Value::Float(0.12));
    config.entry("render").or_insert_with(||toml::Value::Table(Default::default())).as_table_mut().unwrap().insert("point_shadow_softness".into(),toml::Value::Float(0.075));
    p.config = toml::to_string_pretty(&config).unwrap();
}
fn color(p: &Project, hex: &str) -> u16 {
    shared::palette::closest(
        &TheColor::from_hex(&format!("#{hex}")),
        &p.palette
            .colors
            .iter()
            .map(|c| c.clone().unwrap())
            .collect::<Vec<_>>(),
    )
    .unwrap() as u16
}
fn install_material(p: &mut Project, name: &str, colors: &[&str]) -> Uuid {
    let id = Uuid::new_v4();
    let src = asset(&format!("{name}.material"));
    let params = (0..colors.len())
        .map(|i| format!("color{i}"))
        .collect::<Vec<_>>();
    let binding = colors
        .iter()
        .enumerate()
        .map(|(i, c)| ("colors", params[i].as_str(), color(p, c)))
        .collect::<Vec<_>>();
    material_asset(p, id, &src, &binding);
    let mut tile = rusterix::Tile::empty();
    tile.id = id;
    tile.alias = format!("Inn / {name}");
    p.tiles.insert(id, tile);
    id
}
fn tex(id: Uuid, axes: [usize; 2], origin: [f32; 2], span: [f32; 2]) -> Json {
    json!({"type":"texture","source":id.to_string(),"size":128,"axes":axes,"origin":origin,"span":span})
}
fn native_uv_texture(id: Uuid) -> Json {
    let mut op = tex(id, [0, 1], [0., 0.], [1., 1.]);
    op["preserve_uv"] = json!(true);
    op
}
fn cuboid(
    rows: &mut Vec<Json>,
    id: &str,
    size: [f32; 3],
    pos: [f32; 3],
    material: Uuid,
    axes: [usize; 2],
    span: [f32; 2],
) {
    rows.push(row(
        id,
        vec![
            json!({"type":"box","size":size}),
            json!({"type":"transform","translation":pos,"rotation":[0,0,0],"scale":[1,1,1]}),
            tex(material, axes, [0., 0.], span),
        ],
    ));
}
fn result(label: &str, rows: Vec<Json>) -> String {
    let parts = rows
        .iter()
        .filter_map(|r| {
            r["tiles"][0]["op"]["type"]
                .as_str()
                .filter(|t| *t != "material" && *t != "contour")
                .map(|_| r["id"].as_str().unwrap().to_owned())
        })
        .collect();
    finish(label, rows, parts)
}
fn wall(plaster: Uuid, stone: Uuid, wood: Uuid, window: bool) -> String {
    let mut rows = vec![];
    cuboid(
        &mut rows,
        "stone-footing",
        [1., 0.55, 0.26],
        [0., 0.275, 0.],
        stone,
        [0, 1],
        [2., 2.],
    );
    if !window {
        cuboid(
            &mut rows,
            "limewashed-panel",
            [0.91, 1.7, 0.22],
            [0., 1.4, 0.],
            plaster,
            [0, 1],
            [1.8, 2.],
        );
        rows.push(row("diagonal-brace",vec![
            json!({"type":"box","size":[0.075,1.8,0.27]}),
            tex(wood,[0,1],[0.,0.],[1.4,2.]),
            json!({"type":"transform","translation":[0.,1.37,0.],"rotation":[0,0,-24],"scale":[1,1,1]})
        ]));
    } else {
        cuboid(
            &mut rows,
            "lower-stone",
            [1., 0.88, 0.24],
            [0., 0.44, 0.],
            plaster,
            [0, 1],
            [2., 2.],
        );
        cuboid(
            &mut rows,
            "upper-stone",
            [1., 0.35, 0.24],
            [0., 2.075, 0.],
            plaster,
            [0, 1],
            [2., 2.],
        );
        for x in [-0.41, 0.41] {
            cuboid(
                &mut rows,
                &format!("window-pier-{x}"),
                [0.18, 1.02, 0.24],
                [x, 1.39, 0.],
                plaster,
                [0, 1],
                [2., 2.],
            );
        }
        // Leave the opening empty: the scene behind the mullions is visible.
        for (id, size, pos) in [
            ("sill", [0.8, 0.09, 0.34], [0., 0.92, 0.]),
            ("lintel", [0.8, 0.08, 0.3], [0., 1.87, 0.]),
            ("mullion", [0.045, 0.88, 0.29], [0., 1.4, 0.]),
            ("crossbar", [0.69, 0.045, 0.29], [0., 1.4, 0.]),
        ] {
            cuboid(&mut rows, id, size, pos, wood, [0, 1], [0.8, 1.4]);
        }
    }
    for x in [-0.475, 0.475] {
        cuboid(
            &mut rows,
            &format!("corner-post-{x}"),
            [0.07, 2.3, 0.3],
            [x, 1.15, 0.],
            wood,
            [0, 1],
            [0.7, 2.],
        );
    }
    cuboid(
        &mut rows,
        "timber-top",
        [1.04, 0.13, 0.3],
        [0., 2.27, 0.],
        wood,
        [0, 1],
        [1.4, 0.5],
    );
    let mut d: Json = serde_json::from_str(&result(
        if window {
            "Inn / Window bay"
        } else {
            "Inn / Stone wall bay"
        },
        rows,
    ))
    .unwrap();
    let tiles = d["rows"].as_array_mut().unwrap().last_mut().unwrap()["tiles"]
        .as_array_mut()
        .unwrap();
    for (id, x, yaw) in [("join-start", -0.5, -90.), ("join-end", 0.5, 90.)] {
        tiles.push(json!({"id":id,"op":{"type":"connector","family":"wall","translation":[x,0,0],"rotation":[0,yaw,0]}}));
    }
    serde_json::to_string_pretty(&d).unwrap()
}
fn floor(wood: Uuid) -> String {
    let mut rows = vec![];
    for i in 0..12 {
        cuboid(
            &mut rows,
            &format!("floorboard-{i}"),
            [0.488, 0.1, 3.8],
            [-2.75 + i as f32 * 0.5, -0.05, 0.],
            wood,
            [2, 0],
            [2., 0.85],
        );
    }
    result("Inn / Planked floor", rows)
}
fn roof(roof: Uuid, wood: Uuid) -> String {
    let mut rows = vec![];
    for (i, (z, r)) in [(-1.12, -29.), (1.12, 29.)].into_iter().enumerate() {
        let id = format!("roof-slope-{i}");
        rows.push(row(&id,vec![json!({"type":"box","size":[1.06,0.12,2.55]}),tex(roof,[0,2],[0.,0.],[1.9,2.]),json!({"type":"transform","translation":[0.,0.63,z],"rotation":[r,0,0],"scale":[1,1,1]})]));
    }
    cuboid(
        &mut rows,
        "ridge-beam",
        [1.08, 0.13, 0.18],
        [0., 1.29, 0.],
        wood,
        [0, 2],
        [1.3, 0.4],
    );
    // These boards continue through adjoining modules. Only the two outer
    // ends receive a gable, so there are no partitions inside the attic.
    for (i, z) in [-2.24, 2.24].into_iter().enumerate() {
        cuboid(
            &mut rows,
            &format!("eaves-fascia-{i}"),
            [1.06, 0.17, 0.09],
            [0., -0.02, z],
            wood,
            [0, 1],
            [1.9, 0.4],
        );
    }
    result("Inn / Roof module", rows)
}
fn roof_gable(plaster: Uuid, wood: Uuid) -> String {
    let mut rows = vec![row(
        "gable-infill",
        vec![
            json!({"type":"contour","points":[[-2.04,-0.035],[2.04,-0.035],[2.04,0.10],[0.,1.235],[-2.04,0.10]]}),
            json!({"type":"extrude","depth":0.20}),
            json!({"type":"transform","translation":[0,0,0],"rotation":[0,90,0],"scale":[1,1,1]}),
            tex(plaster, [2, 1], [0., 0.], [1.8, 2.]),
        ],
    )];
    cuboid(&mut rows, "gable-tie-beam", [0.26, 0.14, 4.12], [-0.03, 0.01, 0.], wood, [2, 1], [1.4, 0.5]);
    cuboid(&mut rows, "gable-king-post", [0.26, 1.25, 0.13], [-0.03, 0.60, 0.], wood, [2, 1], [0.5, 1.8]);
    for (i, z) in [-1.20, 1.20].into_iter().enumerate() {
        cuboid(&mut rows, &format!("gable-stud-{i}"), [0.25, 0.58, 0.10], [-0.03, 0.29, z], wood, [2, 1], [0.5, 1.2]);
    }
    // Sloping verge boards cover the exposed cut ends of the shingle slabs.
    // The negative-X face is outward; rotate this same part 180° at the far end.
    for (i, (z, pitch)) in [(-1.12, -29.), (1.12, 29.)].into_iter().enumerate() {
        rows.push(row(&format!("gable-verge-{i}"), vec![
            json!({"type":"box","size":[0.18,0.15,2.65]}),
            tex(wood, [2, 1], [0., 0.], [1.8, 0.45]),
            json!({"type":"transform","translation":[-0.09,0.63,z],"rotation":[pitch,0,0],"scale":[1,1,1]}),
        ]));
    }
    // A contour followed by Extrude is rendered geometry, not a footprint row.
    let parts = rows.iter().map(|r| r["id"].as_str().unwrap().into()).collect();
    finish("Inn / Gable end", rows, parts)
}
fn place_roof_gables(map: &mut Map, gable: Uuid) {
    for (x, yaw, label) in [(5.5, 0., "Inn west gable"), (11.5, 180., "Inn east gable")] {
        if map.sectors.iter().any(|s| s.name == label || s.properties.get_str_default("art_study_part", String::new()) == label) { continue; }
        let sid = place_part(map, gable, label, [x, 7.5], [0.13, 2.25], 2.3, yaw, false);
        map.find_sector_mut(sid).unwrap().properties.set("iso_cutaway", Value::Bool(true));
    }
}
fn barrel(wood: Uuid, iron: u16) -> String {
    let mut rows = vec![mat("iron", iron), mat("wood", 19)];
    rows.push(row(
        "barrel",
        vec![
            loft(
                &[
                    ([0., 0., 0.], [0.28, 0.28]),
                    ([0., 0.1, 0.], [0.34, 0.34]),
                    ([0., 0.42, 0.], [0.39, 0.39]),
                    ([0., 0.78, 0.], [0.32, 0.32]),
                    ([0., 0.84, 0.], [0.28, 0.28]),
                ],
                "wood",
                14,
            ),
            tex(wood, [0, 1], [0., 0.], [0.75, 1.2]),
        ],
    ));
    for (i, y) in [0.12, 0.63, 0.77].into_iter().enumerate() {
        rows.push(row(
            &format!("iron-hoop-{i}"),
            vec![loft(
                &[
                    ([0., y - 0.025, 0.], [0.36, 0.36]),
                    ([0., y + 0.025, 0.], [0.36, 0.36]),
                ],
                "iron",
                14,
            )],
        ));
    }
    result("Inn / Coopered barrel", rows)
}
fn crate_mesh(wood: Uuid) -> String {
    let mut rows = vec![];
    cuboid(
        &mut rows,
        "crate-body",
        [0.72, 0.68, 0.72],
        [0., 0.34, 0.],
        wood,
        [0, 1],
        [0.7, 0.7],
    );
    for (i, y) in [0.08, 0.6].into_iter().enumerate() {
        cuboid(
            &mut rows,
            &format!("brace-{i}"),
            [0.79, 0.085, 0.79],
            [0., y, 0.],
            wood,
            [0, 2],
            [0.8, 0.8],
        );
    }
    for (i, x) in [-0.29, 0.29].into_iter().enumerate() {
        cuboid(
            &mut rows,
            &format!("frame-{i}"),
            [0.07, 0.58, 0.78],
            [x, 0.34, 0.],
            wood,
            [0, 1],
            [0.7, 0.7],
        );
    }
    result("Inn / Crate", rows)
}
fn bench(wood: Uuid) -> String {
    let mut rows = vec![];
    cuboid(
        &mut rows,
        "seat",
        [1.65, 0.1, 0.42],
        [0., 0.47, 0.],
        wood,
        [0, 2],
        [1.3, 0.4],
    );
    for (i, x) in [-0.58, 0.58].into_iter().enumerate() {
        cuboid(
            &mut rows,
            &format!("leg-{i}"),
            [0.12, 0.46, 0.35],
            [x, 0.23, 0.],
            wood,
            [0, 1],
            [0.5, 0.6],
        );
    }
    result("Inn / Bench", rows)
}
fn table(wood: Uuid) -> String {
    let mut rows = vec![];
    cuboid(
        &mut rows,
        "tabletop",
        [1.55, 0.12, 0.94],
        [0., 0.87, 0.],
        wood,
        [0, 2],
        [1.4, 0.8],
    );
    for x in [-0.57, 0.57] {
        for z in [-0.3, 0.3] {
            cuboid(
                &mut rows,
                &format!("leg-{x}-{z}"),
                [0.1, 0.86, 0.1],
                [x, 0.43, z],
                wood,
                [0, 1],
                [0.5, 1.],
            );
        }
    }
    result("Inn / Trestle table", rows)
}
fn fence(wood: Uuid) -> String {
    let mut rows = vec![];
    for (i, x) in [0., 1.8].into_iter().enumerate() {
        cuboid(
            &mut rows,
            &format!("post-{i}"),
            [0.13, 1.05, 0.13],
            [x, 0.525, 0.],
            wood,
            [0, 1],
            [0.5, 1.5],
        );
    }
    for (i, y) in [0.38, 0.79].into_iter().enumerate() {
        cuboid(
            &mut rows,
            &format!("rail-{i}"),
            [1.9, 0.1, 0.075],
            [0.9, y, 0.],
            wood,
            [0, 1],
            [1.7, 0.6],
        );
    }
    result("Inn / Split rail fence", rows)
}
fn well(stone: Uuid, wood: Uuid, roof: Uuid, dark: u16) -> String {
    let mut rows = vec![mat("water", dark)];
    rows.push(row("well-bottom",vec![json!({"type":"box","size":[0.9,0.03,0.9]}),json!({"type":"assign_material","row":"water"}),json!({"type":"transform","translation":[0.,0.25,0.],"rotation":[0,0,0],"scale":[1,1,1]})]));
    for (id, size, pos) in [
        ("rim-north", [1.24, 0.62, 0.2], [0., 0.31, -0.51]),
        ("rim-south", [1.24, 0.62, 0.2], [0., 0.31, 0.51]),
        ("rim-west", [0.2, 0.62, 0.82], [-0.51, 0.31, 0.]),
        ("rim-east", [0.2, 0.62, 0.82], [0.51, 0.31, 0.]),
    ] {
        cuboid(&mut rows, id, size, pos, stone, [0, 1], [1.5, 1.5]);
    }
    for (i, x) in [-0.6, 0.6].into_iter().enumerate() {
        cuboid(
            &mut rows,
            &format!("roof-post-{i}"),
            [0.1, 1.9, 0.1],
            [x, 0.95, 0.],
            wood,
            [0, 1],
            [0.5, 1.5],
        );
    }
    cuboid(
        &mut rows,
        "crossbeam",
        [1.43, 0.12, 0.14],
        [0., 1.9, 0.],
        wood,
        [0, 1],
        [1.5, 0.5],
    );
    for (i, (z, angle)) in [(-0.4, -22.), (0.4, 22.)].into_iter().enumerate() {
        rows.push(row(&format!("well-roof-{i}"),vec![json!({"type":"box","size":[1.65,0.1,0.95]}),tex(roof,[0,2],[0.,0.],[1.6,1.6]),json!({"type":"transform","translation":[0.,2.0,z],"rotation":[angle,0,0],"scale":[1,1,1]})]));
    }
    result("Inn / Village well", rows)
}
fn oak_recipe(leaves: Uuid, bark: Uuid, shade: u16, seed: u32) -> String {
    let mut rows = vec![mat("bark", shade)];
    rows.push(row(
        "crooked-trunk",
        vec![
            loft(
                &[
                    ([0., 0., 0.], [0.4, 0.32]),
                    ([0.06, 0.4, 0.], [0.23, 0.19]),
                    ([-0.08, 1.5, 0.04], [0.16, 0.13]),
                    ([0.09, 2.4, 0.], [0.09, 0.08]),
                    ([0.05, 3.1, 0.], [0.025, 0.02]),
                ],
                "bark",
                10,
            ),
            native_uv_texture(bark),
        ],
    ));
    for i in 0..9 {
        let a = i as f32 * 2.4;
        let (s, c) = a.sin_cos();
        let h = 1.1 + (i % 4) as f32 * 0.3;
        rows.push(row(
            &format!("branch-{i}"),
            vec![
                loft(
                    &[
                        ([0., h, 0.], [0.1, 0.08]),
                        ([c * 0.58, h + 0.55, s * 0.58], [0.055, 0.04]),
                        ([c * 1.1, h + 0.85, s * 1.1], [0.008, 0.008]),
                    ],
                    "bark",
                    6,
                ),
                native_uv_texture(bark),
            ],
        ));
    }
    // Overlapping leaf-covered volumes carry an organic canopy in every view.
    // Textured lobes provide depth; small alpha-cut leaves break their outlines.
    rows.push(mat("foliage", shade));
    for i in 0..19 {
        let a = i as f32 * 2.39996 + seed as f32 * 0.7;
        let (sn, cs) = a.sin_cos();
        let ring = if i < 7 {
            0.45
        } else if i < 18 {
            0.88
        } else {
            1.14
        };
        let y = if i < 7 {
            3.15 + ((i * 17 + seed as usize * 7) % 13) as f32 * 0.043
        } else {
            2.25 + ((i * 11 + seed as usize * 3) % 17) as f32 * 0.047
        };
        let r = 0.48 + ((i * 13 + seed as usize) % 11) as f32 * 0.023;
        let x = cs * ring * (0.9 + (seed % 3) as f32 * 0.14) + 0.1;
        let z = sn * ring * 0.89 - 0.12;
        rows.push(row(
            &format!("canopy-lobe-{i}"),
            vec![
                loft(
                    &[
                        ([x, y - 0.37, z], [r * 0.18, r * 0.2]),
                        ([x - 0.04, y - 0.12, z + 0.02], [r * 0.85, r * 0.8]),
                        ([x, y + 0.15, z], [r, r * 0.85]),
                        ([x + 0.04, y + 0.43, z - 0.02], [r * 0.63, r * 0.58]),
                        ([x, y + 0.55, z], [0.03, 0.03]),
                    ],
                    "foliage",
                    9,
                ),
                native_uv_texture(leaves),
            ],
        ));
    }
    // The footprint follows the trunk, not the canopy.
    let points = (0..8)
        .map(|i| {
            let a = i as f32 * std::f32::consts::TAU / 8.;
            [a.cos() * 0.4, a.sin() * 0.32]
        })
        .collect::<Vec<_>>();
    rows.push(row(
        "trunk-contour",
        vec![json!({"type":"contour","points":points})],
    ));
    let mut d: Json = serde_json::from_str(&result("Inn / Broadleaf oak", rows)).unwrap();
    d["rows"].as_array_mut().unwrap().last_mut().unwrap()["tiles"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"ground-contact","op":{"type":"footprint","row":"trunk-contour"}}));
    serde_json::to_string_pretty(&d).unwrap()
}
fn flowers(green: u16, petals: u16) -> String {
    let mut rows = vec![mat("green", green), mat("petals", petals)];
    for i in 0..5 {
        let a = i as f32 * 2.4;
        let x = a.cos() * 0.15;
        let z = a.sin() * 0.15;
        let y = 0.22 + (i % 3) as f32 * 0.05;
        rows.push(row(
            &format!("stem-{i}"),
            vec![loft(
                &[
                    ([x, 0., z], [0.011, 0.011]),
                    ([x + 0.03, y, z], [0.006, 0.006]),
                ],
                "green",
                4,
            )],
        ));
        rows.push(row(
            &format!("bell-{i}"),
            vec![loft(
                &[
                    ([x + 0.03, y - 0.07, z], [0.055, 0.045]),
                    ([x + 0.03, y, z], [0.018, 0.016]),
                    ([x + 0.03, y + 0.015, z], [0.003, 0.003]),
                ],
                "petals",
                6,
            )],
        ));
    }
    result("Inn / Bluebells", rows)
}
fn fern(green: u16) -> String {
    let mut rows = vec![mat("green", green)];
    for i in 0..9 {
        let a = i as f32 * 2.39996;
        let (s, c) = a.sin_cos();
        rows.push(row(
            &format!("frond-{i}"),
            vec![loft(
                &[
                    ([0., 0.02, 0.], [0.012, 0.012]),
                    ([c * 0.12, 0.22, s * 0.12], [0.07, 0.025]),
                    ([c * 0.26, 0.27, s * 0.26], [0.065, 0.018]),
                    ([c * 0.4, 0.29, s * 0.4], [0.005, 0.005]),
                ],
                "green",
                5,
            )],
        ));
    }
    result("Inn / Fern", rows)
}
fn place_part(
    map: &mut Map,
    id: Uuid,
    label: &str,
    at: [f32; 2],
    radius: [f32; 2],
    height: f32,
    yaw: f32,
    follow: bool,
) -> u32 {
    let points = [
        [at[0] - radius[0], at[1] - radius[1]],
        [at[0] + radius[0], at[1] - radius[1]],
        [at[0] + radius[0], at[1] + radius[1]],
        [at[0] - radius[0], at[1] + radius[1]],
    ];
    let sid = sector(map, id, label, &points, height);
    let s = map.find_sector_mut(sid).unwrap();
    s.name.clear();
    s.properties.set("art_study_part", Value::Str(label.into()));
    s.properties.set("iso_yaw", Value::Float(yaw));
    s.properties
        .set("iso_follow_mesh_bounds", Value::Bool(follow));
    sid
}
fn hearth(stone: Uuid, wood: Uuid, black: u16, flame: u16, ember: u16) -> String {
    let mut rows = vec![mat("dark", black)];
    for (id, size, pos) in [
        ("hearth-base", [1.6, 0.16, 0.84], [0., 0.08, 0.]),
        ("left-jamb", [0.23, 1.22, 0.67], [-0.64, 0.69, 0.]),
        ("right-jamb", [0.23, 1.22, 0.67], [0.64, 0.69, 0.]),
        ("mantel", [1.7, 0.18, 0.86], [0., 1.4, 0.]),
        ("chimney", [1.22, 1., 0.56], [0., 1.95, -0.12]),
    ] {
        cuboid(&mut rows, id, size, pos, stone, [0, 1], [1.8, 1.8]);
    }
    rows.push(row("fireback",vec![json!({"type":"box","size":[1.03,1.02,0.04]}),json!({"type":"assign_material","row":"dark"}),json!({"type":"transform","translation":[0.,0.71,-0.3],"rotation":[0,0,0],"scale":[1,1,1]})]));
    for (i, z) in [-0.04, 0.16].into_iter().enumerate() {
        cuboid(
            &mut rows,
            &format!("log-{i}"),
            [0.8, 0.09, 0.12],
            [0., 0.22, z],
            wood,
            [0, 1],
            [0.8, 0.5],
        );
    }
    let mut doc: Json = serde_json::from_str(&result("Inn / Stone hearth", rows)).unwrap();
    doc["rows"].as_array_mut().unwrap().last_mut().unwrap()["tiles"].as_array_mut().unwrap().push(json!({"id":"live-fire","op":{"type":"particle_effect","effect":{"preset":"flame","position":[0,0.36,0.08],"joint":null,"palette":[flame,ember,ember,black],"rate":64,"size":0.1,"lifetime":0.8,"speed":0.55,"turbulence":0.28,"light":0.18,"range":4.5}}}));
    serde_json::to_string_pretty(&doc).unwrap()
}
fn lantern(dark: u16, warm: u16, glass: Uuid) -> String {
    let mut rows = vec![mat("frame", dark), mat("light", warm)];
    for (id, size, pos, material) in [
        ("lit-glass", [0.16, 0.22, 0.16], [0., 0.17, 0.], "light"),
        ("cap", [0.24, 0.045, 0.24], [0., 0.3, 0.], "frame"),
    ] {
        let mut operations = vec![
            json!({"type":"box","size":size}),
            json!({"type":"assign_material","row":material}),
            json!({"type":"transform","translation":pos,"rotation":[0,0,0],"scale":[1,1,1]}),
        ];
        if id == "lit-glass" {
            operations.push(native_uv_texture(glass));
        }
        rows.push(row(id, operations));
    }
    rows.extend(lantern_base_rows());
    for x in [-0.09, 0.09] {
        for z in [-0.09, 0.09] {
            rows.push(row(&format!("frame-{x}-{z}"),vec![json!({"type":"box","size":[0.025,0.27,0.025]}),json!({"type":"assign_material","row":"frame"}),json!({"type":"transform","translation":[x,0.17,z],"rotation":[0,0,0],"scale":[1,1,1]})]));
        }
    }
    result("Inn / Lantern", rows)
}
fn lantern_base_rows() -> Vec<Json> {
    // A solid tray only 11 cm below the point source projects a metre-wide
    // rectangular umbra. Leave the underside open so the lamp lights its feet.
    [
        ("base-left", [0.018,0.018,0.22], [-0.101,0.04,0.]),
        ("base-right", [0.018,0.018,0.22], [0.101,0.04,0.]),
        ("base-back", [0.184,0.018,0.018], [0.,0.04,-0.101]),
        ("base-front", [0.184,0.018,0.018], [0.,0.04,0.101]),
    ].into_iter().map(|(id,size,pos)| row(id,vec![
        json!({"type":"box","size":size}),
        json!({"type":"assign_material","row":"frame"}),
        json!({"type":"transform","translation":pos,"rotation":[0,0,0],"scale":[1,1,1]}),
    ])).collect()
}
fn refresh_lantern_base(p: &mut Project) {
    let tile=p.tiles.iter().find(|(_,t)|t.alias=="Inn / Lantern").map(|(id,_)|*id).unwrap();
    let recipe=p.isometric_bakes[&tile].recipe;
    let mut document:Json=serde_json::from_str(&p.procedural_appearances[&recipe]).unwrap();
    let rows=document["rows"].as_array_mut().unwrap();
    let Some(index)=rows.iter().position(|r|r["id"]=="base") else {return;};
    rows.splice(index..=index,lantern_base_rows());
    for r in rows.iter_mut() {for tile in r["tiles"].as_array_mut().unwrap() {
        if tile["op"]["type"]=="assemble" {
            let parts=tile["op"]["rows"].as_array_mut().unwrap();
            if let Some(i)=parts.iter().position(|id|id=="base") {
                parts.splice(i..=i,[json!("base-left"),json!("base-right"),json!("base-back"),json!("base-front")]);
            }
        }
    }}
    p.procedural_appearances.insert(recipe,serde_json::to_string_pretty(&document).unwrap());
    assert!(shared::isometric::synchronize(p).is_empty());
}
fn palette(p: &mut Project) {
    let colors = shared::palette::parse_colors(&asset("guildhall-256.hex")).unwrap();
    assert_eq!(colors.len(), 256);
    let old = p.palette.clone();
    let mut assigned = vec![None; 256];
    let mut used = HashSet::new();
    // Preserve the original 64 semantic material slots, while assigning every
    // Guildhall swatch exactly once. This also preserves the player materials.
    for i in 0..256 {
        let Some(c) = &old.colors[i] else {
            continue;
        };
        let a = shared::palette::perceptual(c);
        let best = colors
            .iter()
            .enumerate()
            .filter(|(j, _)| !used.contains(j))
            .min_by(|(_, x), (_, y)| {
                let d = |b: &TheColor| {
                    let b = shared::palette::perceptual(b);
                    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>()
                };
                d(x).total_cmp(&d(y))
            })
            .unwrap()
            .0;
        assigned[i] = Some(colors[best].clone());
        used.insert(best);
    }
    let mut remaining = colors
        .iter()
        .enumerate()
        .filter(|(i, _)| !used.contains(i))
        .map(|(_, c)| c.clone());
    for c in &mut assigned {
        if c.is_none() {
            *c = Some(remaining.next().unwrap());
        }
    }
    p.palette = ThePalette::new(assigned);
    p.palette_source_colors = colors;
    p.palette_source_name = "Guildhall 256 — Eric Johnson (Lospec)".into();
    p.ensure_palette_materials_len();
}
fn scene(mut p: Project) -> (Project, Uuid) {
    let character = source_tile(&p, "character");
    let actor_recipe = p.isometric_bakes[&character].recipe;
    let actor_source = p.procedural_appearances[&actor_recipe].clone();
    let actor_bake = p.isometric_bakes[&character].clone();
    p.tiles.retain(|id, _| *id == character);
    p.procedural_appearances.clear();
    p.procedural_appearances.insert(actor_recipe, actor_source);
    p.isometric_bakes.clear();
    p.isometric_bakes.insert(
        character,
        shared::isometric::Bake {
            signature: 0,
            ..actor_bake
        },
    );
    p.procedural_materials.clear();
    p.procedural_material_cache_keys.clear();
    p.material_compositions.clear();
    p.map_material_bindings.clear();
    p.tile_collections.clear();
    p.tile_board_tiles.clear();
    p.tile_board_empty_slots.clear();
    palette(&mut p);
    enable_daylight(&mut p);
    p.name = "The Wayfarer's Rest / 256-color MZ art study".into();
    p.config = super::super::pixel_style::set_density(&p.config, 64).unwrap();
    let grass = install_material(&mut p, "grass", &["262f0d", "3c471b", "54602b", "6e793f"]);
    let earth = install_material(&mut p, "earth", &["29140f", "40261e", "583a2f", "716854"]);
    let stone = install_material(&mut p, "masonry", &["393e47", "646870", "958d73", "bab397"]);
    let cobbles = install_material(&mut p, "cobbles", &["40261e", "646870", "837a63", "a7a084"]);
    let wood = install_material(&mut p, "oak", &["29140f", "583a2f", "896555", "bb957f"]);
    let bark = install_material(&mut p, "bark", &["29140f", "40261e", "704f41", "a27d69"]);
    let leaves = install_material(&mut p, "leaves", &["001003", "17431d", "347423", "6e793f"]);
    let plaster = install_material(&mut p, "plaster", &["4c4334", "837a63", "bab397", "d6ceb3"]);
    let floor_wood = install_material(
        &mut p,
        "floorboards",
        &["29140f", "40261e", "583a2f", "896555"],
    );
    let roofing = install_material(&mut p, "roof", &["29140f", "583a2f", "704f41", "a27d69"]);
    let cloth = install_material(&mut p, "cloth", &["101e39", "203453", "324c6d", "5d7fa0"]);
    let lamp_glass = install_material(&mut p, "lantern-glass", &[]);
    let wall_id = mesh(
        &mut p,
        "Inn / Stone wall bay",
        wall(plaster, stone, wood, false),
        true,
    );
    let window = mesh(
        &mut p,
        "Inn / Window bay",
        wall(plaster, stone, wood, true),
        true,
    );
    let floor_id = mesh(&mut p, "Inn / Planked floor", floor(floor_wood), false);
    let roof_id = mesh(&mut p, "Inn / Roof module", roof(roofing, wood), false);
    let gable_id = mesh(&mut p, "Inn / Gable end", roof_gable(plaster, wood), false);
    let iron = color(&p, "393e47");
    let dark = color(&p, "131822");
    let bark_color = color(&p, "583a2f");
    let barrel = mesh(&mut p, "Inn / Coopered barrel", barrel(wood, iron), true);
    let crate_id = mesh(&mut p, "Inn / Crate", crate_mesh(wood), true);
    let bench = mesh(&mut p, "Inn / Bench", bench(wood), true);
    let table = mesh(&mut p, "Inn / Trestle table", table(wood), true);
    let fence = mesh(&mut p, "Inn / Split rail fence", fence(wood), true);
    let well = mesh(
        &mut p,
        "Inn / Village well",
        well(stone, wood, roofing, dark),
        true,
    );
    let oak = mesh(
        &mut p,
        "Inn / Broadleaf oak",
        oak_recipe(leaves, bark, bark_color, 1),
        true,
    );
    let oak_b = mesh(
        &mut p,
        "Inn / Spreading oak",
        oak_recipe(leaves, bark, bark_color, 7),
        true,
    );
    let oak_c = mesh(
        &mut p,
        "Inn / Young oak",
        oak_recipe(leaves, bark, bark_color, 19),
        true,
    );
    let herb_color = color(&p, "607447");
    let petal_color = color(&p, "9aafd0");
    let flower = mesh(
        &mut p,
        "Inn / Bluebells",
        flowers(herb_color, petal_color),
        false,
    );
    let fern = mesh(&mut p, "Inn / Fern", fern(herb_color), false);
    let rock = mesh(&mut p, "Inn / Field stones", rocks(stone), true);
    let tuft = mesh(&mut p, "Inn / Grass tufts", tuft(), false);
    let mut d = Document::load(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/embedded/meshes/recipes/doorway.mzgeometry.json"
    )))
    .unwrap();
    let denrim_materials::geometry::Operation::Group { controls, .. } = &mut d.rows[0].tiles[0].op
    else {
        panic!()
    };
    for (id, value) in [
        ("width", 2.),
        ("height", 2.25),
        ("depth", 0.26),
        ("thickness", 0.18),
    ] {
        controls.iter_mut().find(|c| c.id == id).unwrap().value = value;
    }
    let mut d: Json = serde_json::from_str(&d.save().unwrap()).unwrap();
    let row_id = d["output"].as_str().unwrap().to_owned();
    d["rows"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["id"] == row_id)
        .unwrap()["tiles"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"oak-surface","op":tex(wood,[0,1],[0.,0.],[1.5,1.5])}));
    let door = mesh(
        &mut p,
        "Inn / Doorway",
        serde_json::to_string_pretty(&d).unwrap(),
        true,
    );
    let mut sign_rows = vec![];
    cuboid(
        &mut sign_rows,
        "sign-board",
        [0.65, 0.72, 0.06],
        [0., 1.75, 0.],
        wood,
        [0, 1],
        [0.7, 0.8],
    );
    cuboid(
        &mut sign_rows,
        "sign-post",
        [0.1, 2.3, 0.1],
        [0.45, 1.15, 0.],
        wood,
        [0, 1],
        [0.5, 1.6],
    );
    cuboid(
        &mut sign_rows,
        "bracket",
        [0.6, 0.08, 0.1],
        [0.2, 2.25, 0.],
        wood,
        [0, 1],
        [0.6, 0.4],
    );
    cuboid(
        &mut sign_rows,
        "blue-painted-sign",
        [0.5, 0.5, 0.008],
        [0., 1.75, -0.035],
        cloth,
        [0, 1],
        [0.7, 0.7],
    );
    let sign = mesh(
        &mut p,
        "Inn / Hanging sign",
        result("Inn / Hanging sign", sign_rows),
        false,
    );
    let flame = color(&p, "f7daaa");
    let ember = color(&p, "e76c00");
    let hearth = mesh(
        &mut p,
        "Inn / Stone hearth",
        hearth(stone, wood, dark, flame, ember),
        true,
    );
    let lamp = mesh(
        &mut p,
        "Inn / Lantern",
        lantern(dark, flame, lamp_glass),
        false,
    );
    let mut collection =
        shared::project::TileCollectionAsset::new("Wayfarer's Rest / MZ parts".into());
    collection.description="An art-quality benchmark: all terrain, surfaces and props are editable MaterialZ sources using Guildhall 256.".into();
    let collection_id = collection.id;
    p.add_tile_collection(collection);
    let ids = p.tiles.keys().copied().collect::<Vec<_>>();
    for (i, id) in ids.iter().enumerate() {
        let src = rusterix::TileSource::SingleTile(*id);
        p.add_source_to_collection(&collection_id, src);
        p.set_collection_tile_board_position(
            &collection_id,
            src,
            Vec2::new((i % 10) as i32, (i / 10) as i32),
        );
    }
    let mut map = Map::default();
    map.name = "Wayfarer's Rest".into();
    map.camera = rusterix::MapCamera::TwoDIso;
    map.grid_size = 32.;
    let ground = sector(
        &mut map,
        grass,
        "Meadow",
        &[[-11., -11.], [32., -11.], [32., 32.], [-11., 32.]],
        0.,
    );
    map.find_sector_mut(ground).unwrap().layer = Some(0);
    map.properties.set("terrain_enabled", Value::Bool(true));
    map.properties.set("terrain_subdivisions", Value::Int(4));
    map.properties.set("terrain_edge_falloff", Value::Float(0.));
    map.properties.set(
        "default_terrain_tile",
        Value::Source(rusterix::PixelSource::TileId(grass)),
    );
    for (name, x, z, height, radius) in [
        ("Wooded hill", 16., 4., 1.6, 6.),
        ("Hill shoulder", 16.5, 7., 0.7, 4.2),
        ("Garden knoll", 3., 15.5, 0.65, 4.),
    ] {
        let mut v = rusterix::Vertex::new(map.find_free_vertex_id().unwrap(), x, z);
        v.z = height;
        v.name = name.into();
        v.properties.set("terrain_control", Value::Bool(true));
        v.properties.set("smoothness", Value::Float(radius / 4.));
        map.vertices.push(v);
    }
    let road = Uuid::new_v4();
    let mut layer = CompositionLayer::split(earth);
    layer.shape = CompositionShape::AutoRoad;
    layer.position = 0.72;
    layer.feather = 0.34;
    layer.wobble = 0.19;
    layer.wobble_scale = 3.;
    p.material_compositions.insert(
        road,
        MaterialComposition {
            base: grass,
            layers: vec![layer],
            tiling: CompositionTiling::default(),
        },
    );
    let mut tile = rusterix::Tile::empty();
    tile.id = road;
    tile.alias = "Inn / Worn trail".into();
    p.tiles.insert(road, tile);
    let cobble = Uuid::new_v4();
    let mut layer = CompositionLayer::split(cobbles);
    layer.shape = CompositionShape::AutoRoad;
    layer.position = 0.97;
    layer.feather = 0.15;
    layer.wobble = 0.07;
    layer.wobble_scale = 4.;
    p.material_compositions.insert(
        cobble,
        MaterialComposition {
            base: grass,
            layers: vec![layer],
            tiling: CompositionTiling::default(),
        },
    );
    let mut tile = rusterix::Tile::empty();
    tile.id = cobble;
    tile.alias = "Inn / Cobbled approach".into();
    p.tiles.insert(cobble, tile);
    let mut paint = std::collections::HashMap::with_hasher(Default::default());
    let trail = [
        14, 14, 15, 16, 16, 16, 15, 14, 13, 12, 12, 12, 12, 12, 13, 14, 14, 15,
    ];
    for (z, &x) in trail.iter().enumerate() {
        for dx in 0..2 {
            paint.insert((x + dx, z as i32), rusterix::PixelSource::TileId(road));
        }
        if z > 0 {
            for fill in trail[z - 1].min(x)..=trail[z - 1].max(x) + 1 {
                paint.insert((fill, z as i32), rusterix::PixelSource::TileId(road));
            }
        }
    }
    for z in 9..14 {
        for x in 7..12 {
            paint.insert((x, z), rusterix::PixelSource::TileId(cobble));
        }
    }
    map.properties.set("tiles", Value::TileOverrides(paint));
    for x in 6..12 {
        for (z, w) in [(5.5, false), (9.5, true)] {
            if w && (x == 8 || x == 9) {
                continue;
            }
            let sid = place_part(
                &mut map,
                if x % 2 == 0 { window } else { wall_id },
                "Inn wall bay",
                [x as f32, z],
                [0.5, 0.12],
                0.,
                0.,
                true,
            );
            map.find_sector_mut(sid)
                .unwrap()
                .properties
                .set("iso_cutaway", Value::Bool(w));
        }
    }
    for z in 6..10 {
        for x in [5.5, 11.5] {
            let sid = place_part(
                &mut map,
                if z % 2 == 0 { window } else { wall_id },
                "Inn side wall",
                [x, z as f32],
                [0.12, 0.5],
                0.,
                90.,
                true,
            );
            map.find_sector_mut(sid)
                .unwrap()
                .properties
                .set("iso_cutaway", Value::Bool(x < 6.));
        }
    }
    let door_sector = place_part(
        &mut map,
        door,
        "Inn doorway",
        [8.5, 9.5],
        [1., 0.14],
        0.,
        0.,
        true,
    );
    map.find_sector_mut(door_sector)
        .unwrap()
        .properties
        .set("iso_cutaway", Value::Bool(true));
    let floor = place_part(
        &mut map,
        floor_id,
        "Inn interior floor",
        [8.5, 7.5],
        [3., 1.9],
        0.09,
        0.,
        false,
    );
    let s = map.find_sector_mut(floor).unwrap();
    s.name = "Inn".into();
    s.properties.set("iso_walkable", Value::Bool(true));
    s.properties.set("iso_interior", Value::Bool(true));
    for x in 6..12 {
        let sid = place_part(
            &mut map,
            roof_id,
            "Inn roof module",
            [x as f32, 7.5],
            [0.5, 2.1],
            2.3,
            0.,
            false,
        );
        map.find_sector_mut(sid)
            .unwrap()
            .properties
            .set("iso_cutaway", Value::Bool(true));
    }
    place_roof_gables(&mut map, gable_id);
    for (id, x, z, yaw, r) in [
        (well, 13.8, 11.7, 0., 0.65),
        (barrel, 11., 10.8, 0., 0.35),
        (barrel, 10.4, 11.3, 30., 0.35),
        (crate_id, 5., 9.4, 0., 0.38),
        (crate_id, 4.6, 10.3, 20., 0.38),
        (bench, 6.3, 10.7, 90., 0.5),
        (sign, 9.6, 10., 0., 0.2),
        (table, 8., 6.8, 0., 0.65),
        (bench, 8., 7.8, 0., 0.55),
        (barrel, 10.7, 6.3, 0., 0.35),
    ] {
        place_part(
            &mut map,
            id,
            "Inn furnishings",
            [x, z],
            [r, r],
            if z < 9. { 0.1 } else { 0. },
            yaw,
            true,
        );
    }
    place_part(
        &mut map,
        hearth,
        "Inn hearth",
        [6.8, 6.],
        [0.8, 0.38],
        0.1,
        0.,
        false,
    );
    place_part(
        &mut map,
        lamp,
        "Door lantern",
        [9.4, 9.73],
        [0.1, 0.1],
        1.3,
        0.,
        false,
    );
    place_part(
        &mut map,
        lamp,
        "Table lantern",
        [8.3, 6.8],
        [0.1, 0.1],
        1.08,
        0.,
        false,
    );
    // Ordinary authored lights: editable with Eldiron's Light tool, shared by cameras.
    for (position, intensity, range, flicker) in [
        ([9.4, 1.47, 9.73], 1.65, 4.5, 0.08),
        ([8.3, 1.25, 6.8], 1.35, 3.6, 0.08),
    ] {
        map.lights.push(
            rusterix::Light::new(rusterix::LightType::Point)
                .with_position(Vec3::from(position))
                .with_color([1., 0.72, 0.38])
                .with_intensity(intensity)
                .with_start_distance(0.5)
                .with_end_distance(range)
                .with_flicker(flicker),
        );
    }
    for (x, z, yaw) in [
        (3.4, 10.5, 90.),
        (3.4, 12.3, 90.),
        (4.3, 14.4, 0.),
        (6.1, 14.4, 0.),
        (14.8, 5.2, 90.),
        (14.8, 7., 90.),
    ] {
        place_part(
            &mut map,
            fence,
            "Paddock fence",
            [x, z],
            [0.2, 0.2],
            0.,
            yaw,
            true,
        );
    }
    for (i, (x, z, yaw)) in [
        (2.2, 4.5, 0.),
        (3.6, 6.8, 60.),
        (4.3, 3., 170.),
        (8.5, 2.5, 260.),
        (12.6, 3.4, 20.),
        (15.9, 6.3, 130.),
        (17.8, 12.7, 250.),
        (1.8, 15.4, 90.),
        (5., 17.6, 0.),
        (18.2, 3.8, 50.),
        (18.8, 6.8, 170.),
    ]
    .into_iter()
    .enumerate()
    {
        place_part(
            &mut map,
            [oak, oak_b, oak_c][i % 3],
            "Mature oak",
            [x, z],
            [0.3, 0.3],
            0.,
            yaw,
            true,
        );
    }
    for (x, z, yaw) in [
        (2.5, 8.9, 0.),
        (14.8, 14.7, 45.),
        (16.2, 9.1, 100.),
        (7.1, 15.3, 240.),
        (10.4, 3.5, 20.),
    ] {
        place_part(
            &mut map,
            rock,
            "Roadside stone",
            [x, z],
            [0.45, 0.35],
            0.,
            yaw,
            true,
        );
    }
    for i in 0..72 {
        let x = 1.2 + (i * 73 % 160) as f32 * 0.1;
        let z = 1.1 + (i * 47 % 155) as f32 * 0.1;
        if (x > 5. && x < 14. && z > 5. && z < 14.) || (x > 11. && x < 14.) {
            continue;
        }
        place_part(
            &mut map,
            tuft,
            "Meadow tuft",
            [x, z],
            [0.1, 0.1],
            0.,
            (i * 37 % 360) as f32,
            false,
        );
    }
    for (cluster, (x, z)) in [
        (3.8, 10.8),
        (5., 13.5),
        (5.3, 5.),
        (12., 4.),
        (17.6, 7.8),
        (15., 14.5),
    ]
    .into_iter()
    .enumerate()
    {
        for i in 0..9 {
            let angle = i as f32 * 2.39996 + cluster as f32;
            let radius = 0.2 + (i % 4) as f32 * 0.19;
            place_part(
                &mut map,
                if i % 3 == 0 { fern } else { flower },
                "Woodland planting",
                [x + angle.cos() * radius, z + angle.sin() * radius],
                [0.08, 0.08],
                0.,
                i as f32 * 37.,
                false,
            );
        }
    }
    map.properties
        .set("iso_brush_follow_mesh_bounds", Value::Bool(true));
    map.properties.set("iso_brush_snap", Value::Bool(true));
    ServerContext::default().center_map_at_grid_pos(
        Vec2::new(1280., 900.),
        Vec2::new(9.4, 9.8),
        &mut map,
    );
    map.changed = map.changed.wrapping_add(1);
    let old_id = p.regions[0].map.id;
    map.id = old_id;
    p.regions[0].map = map;
    for c in p.regions[0].characters.values_mut() {
        c.position = Vec3::new(9., 0., 12.3);
    }
    p.regions[0].name = "Wayfarer's Rest".into();
    p.config = p
        .config
        .replace("Terrain Playground", "Wayfarer's Rest")
        .replace("MZ Iso Room", "Wayfarer's Rest")
        .replace("MZ Clearing", "Wayfarer's Rest");
    (p, character)
}
fn render(
    p: &Project,
    character: Uuid,
    out: &Path,
    name: &str,
    position: Vec3<f32>,
    unit: f32,
    center: Vec2<f32>,
) {
    render_at_hour(p, character, out, name, position, unit, center, 12.);
}
fn render_at_hour(
    p: &Project,
    character: Uuid,
    out: &Path,
    name: &str,
    position: Vec3<f32>,
    unit: f32,
    center: Vec2<f32>,
    hour: f32,
) {
    let mut map = p.regions[0].map.clone();
    map.entities.clear();
    let mut actor = rusterix::Entity::new();
    actor.position = position;
    actor.set_attribute(
        "source",
        Value::Source(rusterix::PixelSource::TileId(character)),
    );
    actor.set_attribute("player", Value::Bool(true));
    map.entities.push(actor);
    let mut assets = rusterix::Assets::default();
    assets.set_tiles(p.tiles.clone());
    assets.palette = p.palette.clone();
    let mut image = TheRGBABuffer::new(TheDim::sized(1280, 900));
    let mut projection = rusterix::isometric::Projection {
        origin: Vec2::zero(),
        unit,
    };
    projection.origin = Vec2::new(640., 450.) - projection.screen(center, 0.);
    let mut settings = rusterix::RenderSettings::default();
    settings.read(&p.config).unwrap();
    assert!(
        settings.simulation.enabled,
        "Art study must use Eldiron sun simulation"
    );
    settings.apply_hour(hour);
    let mut scene = rusterix::SceneHandler::default();
    scene.base_settings = settings.clone();
    scene.settings = settings.clone();
    scene.build_dynamics_2d(&map, 0, &assets, &Default::default());
    let particles = scene.isometric_particles(&map, &assets, 0);
    let lighting =
        rusterix::isometric::lighting::Lighting::new(&settings, scene.vm.vm.lights.values(), 0);
    rusterix::isometric::draw_scene_gpu_cached(
        &map,
        &assets,
        &mut image,
        projection,
        0.,
        None,
        &Default::default(),
        None,
        &lighting,
        &mut scene.vm,
        &particles,
    )
    .unwrap();
    std::fs::write(out.join(format!("{name}.png")), image.to_png().unwrap()).unwrap();
}
fn runtime_review(project: &Project, out: &Path) {
    let mut running = project.clone();
    shared::rusterix_utils::insert_content_into_maps(&mut running);
    let mut runtime = rusterix::Rusterix::new_without_audio();
    runtime.set_tiles_for_maps(running.tiles.clone(), false, [&running.regions[0].map]);
    shared::rusterix_utils::start_server(&mut runtime, &mut running, false).unwrap();
    shared::rusterix_utils::sync_display_templates(&mut runtime.assets, &running, false);
    let commands = shared::rusterix_utils::setup_client(&mut runtime, &mut running);
    runtime.server.process_client_commands(commands);
    fn tick(runtime: &mut rusterix::Rusterix, p: &mut Project) {
        std::thread::sleep(std::time::Duration::from_millis(10));
        shared::rusterix_utils::warmup_runtime(runtime, p, 1);
    }
    for _ in 0..30 {
        tick(&mut runtime, &mut running);
    }
    runtime
        .server
        .local_player_teleport_pos(Vec2::new(8.5, 11.));
    for _ in 0..20 {
        tick(&mut runtime, &mut running);
    }
    for goal in [
        Vec2::new(8.5, 8.8),
        Vec2::new(8.5, 12.3),
        Vec2::new(10.5, 12.5),
        Vec2::new(12.5, 10.5),
        Vec2::new(14.5, 8.5),
        Vec2::new(16.5, 5.5),
        Vec2::new(16.5, 4.5),
        Vec2::new(16.5, 5.5),
        Vec2::new(14.5, 8.5),
        Vec2::new(12.5, 10.5),
        Vec2::new(10.5, 12.5),
    ] {
        runtime
            .server
            .local_player_action(rusterix::EntityAction::TerrainClicked(goal));
        let mut arrived = false;
        let mut last_position = Vec3::zero();
        for _ in 0..700 {
            tick(&mut runtime, &mut running);
            let e = running.regions[0]
                .map
                .entities
                .iter()
                .find(|e| e.is_player())
                .unwrap();
            last_position = e.position;
            if (Vec2::new(e.position.x, e.position.z) - goal).magnitude() < 0.16 {
                if goal.y < 9. && goal.x < 12. {
                    assert!(
                        (e.position.y - 0.09).abs() < 0.01,
                        "Inn floor support: {:?}",
                        e.position
                    );
                }
                if goal.x > 15. {
                    let terrain = rusterix::chunkbuilder::terrain_surface::TerrainSurface::cached(
                        &running.regions[0].map,
                    )
                    .unwrap();
                    let expected = terrain
                        .height_at(Vec2::new(e.position.x, e.position.z))
                        .unwrap();
                    assert!(
                        (e.position.y - expected).abs() < 0.01,
                        "Hill support {:?}, expected {expected}",
                        e.position
                    );
                }
                arrived = true;
                break;
            }
        }
        runtime
            .server
            .local_player_action(rusterix::EntityAction::Off);
        assert!(
            arrived,
            "Could not walk through the art study toward {goal:?}; stopped at {last_position:?}"
        );
    }
    let mut game = rusterix::client::widget::game::GameWidget::new();
    game.set_camera_mode(rusterix::PlayerCamera::D2Iso);
    game.grid_size = 36.;
    game.buffer = TheRGBABuffer::new(TheDim::sized(1100, 800));
    game.graphical_draw(
        &running.regions[0].map,
        &running.time,
        0,
        &runtime.assets,
        &mut runtime.scene_handler,
    );
    std::fs::write(out.join("playing.png"), game.buffer.to_png().unwrap()).unwrap();
    runtime.server.stop();
    eprintln!(
        "Verified live walking into/out of the inn, across the courtyard and up/down the wooded hill."
    );
}

#[test]
#[ignore = "builds and renders a native 256-color MZ art-quality study under generated/"]
fn build_uo_art_study() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let out = root.join("generated/uo-art-study");
    std::fs::create_dir_all(&out).unwrap();
    let base = shared::project_io::decode_project(
        &std::fs::read(root.join("generated/mz-iso2d-native/MZIso2D.eldiron")).unwrap(),
    )
    .unwrap();
    let (mut project, character) = scene(base);
    let errors = super::super::composition::prepare_project(&mut project);
    assert!(errors.is_empty(), "{errors:?}");
    let (device, queue) = pollster::block_on(async {
        let adapter = wgpu::Instance::default()
            .request_adapter(&Default::default())
            .await
            .unwrap();
        adapter.request_device(&Default::default()).await.unwrap()
    });
    let worker = super::super::gpu::Worker::new(device, queue);
    let sources = project
        .procedural_materials
        .iter()
        .map(|(id, s)| (*id, s.clone()))
        .collect::<Vec<_>>();
    for (id, bundle) in sources {
        eprintln!(
            "Baking material {}",
            project
                .tiles
                .get(&id)
                .map(|t| t.alias.as_str())
                .unwrap_or("composition")
        );
        worker.send(super::super::gpu::Job {
            pixels_per_tile: 64,
            tile: id,
            revision: 0,
            bundle: bundle.clone(),
            continuous: false,
            selection: None,
            preview_aspect: 1.,
        });
        let frame = worker
            .results
            .recv_timeout(std::time::Duration::from_secs(180))
            .unwrap()
            .frame
            .unwrap();
        let tile = project.tiles.get_mut(&id).unwrap();
        if let Some(pages) = frame.surface_pages {
            tile.textures = pages.textures.clone();
            tile.uv_pages = Some(pages.layout.clone());
        } else {
            tile.textures = vec![frame.tile];
            tile.uv_pages = None;
        }
        tile.baked_material_data = tile.textures.iter().map(|t| t.material_bytes()).collect();
        project
            .procedural_material_cache_keys
            .insert(id, super::super::pixel_style::bake_key(&bundle, 64));
    }
    let errors = shared::isometric::synchronize(&mut project);
    assert!(errors.is_empty(), "{errors:?}");
    let bytes = shared::project_io::encode_project(&project).unwrap();
    std::fs::write(out.join("WayfarersRest.eldiron"), &bytes).unwrap();
    let mut reopened = shared::project_io::decode_project(&bytes).unwrap();
    assert!(shared::isometric::synchronize(&mut reopened).is_empty());
    assert_eq!(project.palette.colors.iter().flatten().count(), 256);
    render(
        &project,
        character,
        &out,
        "courtyard",
        Vec3::new(9., 0., 12.3),
        42.,
        Vec2::new(9.5, 9.5),
    );
    render(
        &project,
        character,
        &out,
        "interior",
        Vec3::new(8.5, 0.09, 8.8),
        44.,
        Vec2::new(9., 8.5),
    );
    render(
        &project,
        character,
        &out,
        "wide",
        Vec3::new(9., 0., 12.3),
        36.,
        Vec2::new(10.5, 10.5),
    );
    for (name, hour) in [("morning", 8.), ("evening", 18.5), ("night", 23.)] {
        render_at_hour(
            &project,
            character,
            &out,
            name,
            Vec3::new(9., 0., 12.3),
            42.,
            Vec2::new(9.5, 9.5),
            hour,
        );
    }
    render_at_hour(
        &project,
        character,
        &out,
        "firelight",
        Vec3::new(8.5, 0.09, 8.8),
        50.,
        Vec2::new(8.6, 7.4),
        23.,
    );
    let terrain =
        rusterix::chunkbuilder::terrain_surface::TerrainSurface::from_map(&project.regions[0].map)
            .unwrap();
    let hill = Vec2::new(16.5, 6.5);
    render_at_hour(
        &project,
        character,
        &out,
        "hill",
        Vec3::new(hill.x, terrain.height_at(hill).unwrap(), hill.y),
        42.,
        Vec2::new(15., 6.),
        17.,
    );
    runtime_review(&project, &out);
    eprintln!("Art study: {}", out.join("WayfarersRest.eldiron").display());
}

#[test]
#[ignore = "refreshes art-study captures and verifies the existing playable project without rebaking surfaces"]
fn review_uo_art_study() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let out = root.join("generated/uo-art-study");
    let file = art_project_file(root);
    let mut project = shared::project_io::decode_project(&std::fs::read(&file).unwrap()).unwrap();
    enable_daylight(&mut project);
    for recipe in project.procedural_appearances.values_mut() {
        let Ok(mut doc) = Document::load(recipe) else {
            continue;
        };
        if doc.label != "Inn / Stone hearth" {
            continue;
        }
        for row in &mut doc.rows {
            for tile in &mut row.tiles {
                if let denrim_materials::geometry::Operation::ParticleEffect { effect } =
                    &mut tile.op
                {
                    effect.light = 0.18;
                }
            }
        }
        *recipe = doc.save().unwrap();
    }

    for s in &mut project.regions[0].map.sectors {
        if s.name == "Inn doorway" || s.properties.get_str_default("art_study_part", String::new()) == "Inn doorway" {
            s.properties.set("iso_cutaway", Value::Bool(true));
        }
    }
    assert!(shared::isometric::synchronize(&mut project).is_empty());
    std::fs::write(file, shared::project_io::encode_project(&project).unwrap()).unwrap();
    let actor = source_tile(&project, "character");
    render(
        &project,
        actor,
        &out,
        "courtyard",
        Vec3::new(9., 0., 12.3),
        42.,
        Vec2::new(9.5, 9.5),
    );
    render(
        &project,
        actor,
        &out,
        "interior",
        Vec3::new(8.5, 0.09, 8.8),
        44.,
        Vec2::new(9., 8.5),
    );
    render(
        &project,
        actor,
        &out,
        "wide",
        Vec3::new(9., 0., 12.3),
        36.,
        Vec2::new(10.5, 10.5),
    );
    for (name, hour) in [("morning", 8.), ("evening", 18.5), ("night", 23.)] {
        render_at_hour(
            &project,
            actor,
            &out,
            name,
            Vec3::new(9., 0., 12.3),
            42.,
            Vec2::new(9.5, 9.5),
            hour,
        );
    }
    runtime_review(&project, &out);
}

#[test]
#[ignore = "finishes the existing local art-study roof and captures native GPU views under generated/"]
fn finish_uo_roof_ends() {
    use denrim_materials::geometry::Operation;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let out = root.join("generated/uo-art-study");
    let file = art_project_file(root);
    let mut project = shared::project_io::decode_project(&std::fs::read(&file).unwrap()).unwrap();
    let material_sources = project.procedural_materials.clone();
    let tile_id = |alias: &str| *project.tiles.iter().find(|(_, t)| t.alias == alias).unwrap().0;
    let (roof_id, roofing, plaster, wood) = (tile_id("Inn / Roof module"), tile_id("Inn / roof"), tile_id("Inn / plaster"), tile_id("Inn / oak"));
    let recipe = project.isometric_bakes[&roof_id].recipe;
    let mut doc = Document::load(&project.procedural_appearances[&recipe]).unwrap();
    let additions = Document::load(&roof(roofing, wood)).unwrap().rows.into_iter()
        .filter(|r| r.id.starts_with("eaves-fascia-") && !doc.rows.iter().any(|existing| existing.id == r.id))
        .collect::<Vec<_>>();
    let added_ids = additions.iter().map(|r| r.id.clone()).collect::<Vec<_>>();
    let output_index = doc.rows.iter().position(|r| r.id == doc.output).unwrap();
    let output = &mut doc.rows[output_index];
    let Operation::Assemble { rows } = &mut output.tiles.iter_mut().find(|t| matches!(t.op, Operation::Assemble { .. })).unwrap().op else { unreachable!() };
    rows.extend(added_ids);
    doc.rows.splice(output_index..output_index, additions);
    project.procedural_appearances.insert(recipe, doc.save().unwrap());
    let gable = project.tiles.iter().find(|(_, t)| t.alias == "Inn / Gable end").map(|(id, _)| *id)
        .unwrap_or_else(|| mesh(&mut project, "Inn / Gable end", roof_gable(plaster, wood), false));
    let collection = *project.tile_collections.iter().find(|(_, c)| c.name == "Wayfarer's Rest / MZ parts").unwrap().0;
    let source = rusterix::TileSource::SingleTile(gable);
    if !project.collection_contains_source(&collection, source) {
        let next_row = project.tile_collections[&collection].tile_board_tiles.values().map(|p| p.y).max().unwrap_or(-1) + 1;
        project.add_source_to_collection(&collection, source);
        project.set_collection_tile_board_position(&collection, source, Vec2::new(0, next_row));
    }
    place_roof_gables(&mut project.regions[0].map, gable);
    assert!(shared::isometric::synchronize(&mut project).is_empty());
    assert_eq!(project.procedural_materials, material_sources, "Finishing the roof must retain the existing material recipes");
    std::fs::write(&file, shared::project_io::encode_project(&project).unwrap()).unwrap();
    let project = shared::project_io::decode_project(&std::fs::read(file).unwrap()).unwrap();
    let actor = source_tile(&project, "character");
    for (name, hour) in [("courtyard", 12.), ("night", 23.)] {
        render_at_hour(&project, actor, &out, name, Vec3::new(9., 0., 12.3), 42., Vec2::new(9.5, 9.5), hour);
    }
    render(&project, actor, &out, "interior", Vec3::new(8.5, 0.09, 8.8), 44., Vec2::new(9., 8.5));
    runtime_review(&project, &out);
}

#[test]
#[ignore = "captures the persisted GPU art project without rerunning the walking test"]
fn capture_uo_gpu_lighting() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let out = root.join("generated/uo-art-study");
    let mut p = shared::project_io::decode_project(
        &std::fs::read(art_project_file(root)).unwrap(),
    )
    .unwrap();
    refresh_lantern_base(&mut p);
    for light in &mut p.regions[0].map.lights {
        let position=light.position();
        if (position.x-9.4).abs()<0.001 && (position.z-9.73).abs()<0.001 {
            light.set_intensity(1.65);light.set_end_distance(4.5);light.set_flicker(0.08);
        } else if (position.x-8.3).abs()<0.001 && (position.z-6.8).abs()<0.001 {
            light.set_intensity(1.35);light.set_end_distance(3.6);light.set_flicker(0.08);
        }
    }
    enable_daylight(&mut p);
    std::fs::write(art_project_file(root),shared::project_io::encode_project(&p).unwrap()).unwrap();
    let hearth = p
        .tiles
        .values()
        .find(|t| t.alias == "Inn / Stone hearth")
        .unwrap();
    assert_eq!(
        hearth.isometric.as_ref().unwrap().surfaces[0].effects[0].light,
        0.18
    );
    let actor = source_tile(&p, "character");
    for (name, hour) in [
        ("courtyard", 12.),
        ("morning", 8.),
        ("evening", 18.5),
        ("night", 23.),
    ] {
        render_at_hour(
            &p,
            actor,
            &out,
            name,
            Vec3::new(9., 0., 12.3),
            42.,
            Vec2::new(9.5, 9.5),
            hour,
        );
    }
    render_at_hour(
        &p,
        actor,
        &out,
        "interior",
        Vec3::new(8.5, 0.09, 8.8),
        44.,
        Vec2::new(9., 8.5),
        12.,
    );
    render_at_hour(&p,actor,&out,"firelight",Vec3::new(8.5,0.09,8.8),50.,Vec2::new(8.6,7.4),23.);
}


#[test]
#[ignore = "requires native GPU and the local art-study project"]
fn lantern_open_base_lights_ground_and_preserves_nearby_occlusion() {
    let root=Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let p=shared::project_io::decode_project(&std::fs::read(art_project_file(root)).unwrap()).unwrap();
    let glass=*p.tiles.iter().find(|(_,t)|t.alias=="Inn / lantern-glass").unwrap().0;
    let doc=Document::load(&lantern(color(&p,"131822"),color(&p,"f7daaa"),glass)).unwrap();
    let mesh=doc.evaluate().unwrap().mesh;
    let mut baker=shared::isometric::Baker{project:&p,textures:Default::default()};
    let frame=baker.bake("open-base-lantern",&mesh,0.,false).unwrap();
    let fixture=frame.surface.shadow_mesh.unwrap();
    let mut open=scenevm::isometric::ShadowGeometry{revision:1,..Default::default()};
    open.append(&fixture,[0.,1.3,0.],1.);
    let mut closed=open.clone();closed.revision=2;
    for triangle in [
        [[-0.11,1.36,-0.11],[0.11,1.36,-0.11],[-0.11,1.36,0.11]],
        [[0.11,1.36,-0.11],[0.11,1.36,0.11],[-0.11,1.36,0.11]],
    ] {closed.triangle(triangle);}
    let mut scene=scenevm::SceneVM::new(1,1);
    let light=scenevm::isometric::PointLight{position:[0.,1.47,0.,0.],color:[1.,0.72,0.38,0.],parameters:[3.,4.,0.5,0.]};
    let mut uniforms=scenevm::isometric::Uniforms{projection:[0.5,0.5,1.,1.],counts:[1,1,1,0],shadow:[10.,1.,0.,1.],..Default::default()};
    let surfaces=[rusterix::isometric::surface::packed(Vec3::unit_y(),0.,0.95,0.,0.)];
    let mut render=|mesh:&scenevm::isometric::ShadowGeometry,u:&scenevm::isometric::Uniforms| {
        let mut pixels=[160,160,160,255];
        scene.render_isometric_geometry(u,&mut pixels,&surfaces,&[light],&[],&[],&[],Some(mesh)).unwrap();pixels
    };
    let blocked=render(&closed,&uniforms);
    let lit=render(&open,&uniforms);
    assert!(blocked[0]<20,"solid tray reproduces the rectangular shadow: {blocked:?}");
    assert!(lit[0]>100&&lit[0]>lit[2],"open recipe transmits warm light to the ground: {lit:?}");
    uniforms.projection=[0.1,0.7,1.,1.];
    let beside=render(&open,&uniforms);
    let mut wall=open.clone();wall.revision=3;
    wall.triangle([[0.2,0.,-1.],[0.2,1.,-1.],[0.2,0.,1.]]);
    wall.triangle([[0.2,1.,-1.],[0.2,1.,1.],[0.2,0.,1.]]);
    let occluded=render(&wall,&uniforms);
    assert!(beside[0]>100&&occluded[0]<beside[0]/2,"a nearby wall must still cast a real shadow: {beside:?} vs {occluded:?}");
}
