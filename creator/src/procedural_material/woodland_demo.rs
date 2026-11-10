//! Native, source-editable art experiment. Explicitly run the ignored builder;
//! all projects and renders it writes belong under /generated.
use super::*;
use denrim_materials::integration::{InstanceRecord, PaletteBinding, ParameterKey};
use serde_json::{Value as Json, json};
use shared::material_composition::{
    CompositionLayer, CompositionShape, CompositionTiling, MaterialComposition,
};
use std::{collections::HashSet, path::Path};

mod recipe_playground;
mod terrain_art;
pub(super) mod uo_art;

const GRASS: &str = include_str!("../../../crates/shared/examples/iso_2d/woodland/grass.material");
const EARTH: &str = include_str!("../../../crates/shared/examples/iso_2d/woodland/soil.material");
const STONE: &str = include_str!("../../../crates/shared/examples/iso_2d/woodland/stone.material");

fn source_tile(project: &Project, name: &str) -> Uuid {
    project
        .tiles
        .iter()
        .find(|(_, t)| t.alias == format!("MZ Iso / {name}"))
        .unwrap()
        .0
        .to_owned()
}
fn source_recipe(project: &Project, name: &str) -> Uuid {
    project.isometric_bakes[&source_tile(project, name)].recipe
}
fn material_asset(project: &mut Project, id: Uuid, source: &str, bindings: &[(&str, &str, u16)]) {
    let mut asset = EditableMaterialAsset::parse(&id.to_string(), source, "surface").unwrap();
    let mut record = InstanceRecord::new("eldiron-palette");
    for &(tile, param, index) in bindings {
        record.palette_bindings.push(PaletteBinding {
            key: ParameterKey::new(&id.to_string(), tile, param),
            index: index.into(),
        });
    }
    asset.instances.push(record);
    rusterix::ruleset_material::resolve_palette(&mut asset, &project.palette).unwrap();
    project
        .procedural_materials
        .insert(id, asset.save().unwrap());
}
fn ramp(project: &mut Project, id: Uuid, source: &str, slots: [u16; 4]) {
    material_asset(
        project,
        id,
        source,
        &[
            ("colors", "color0", slots[0]),
            ("colors", "color1", slots[1]),
            ("colors", "color2", slots[2]),
            ("colors", "color3", slots[3]),
        ],
    );
}
fn row(id: &str, operations: Vec<Json>) -> Json {
    json!({"id":id,"label":id.replace('-'," "),"tiles":operations.into_iter().enumerate().map(|(i,op)|json!({"id":format!("{id}-op-{i}"),"op":op})).collect::<Vec<_>>()})
}
fn finish(label: &str, mut rows: Vec<Json>, parts: Vec<String>) -> String {
    rows.push(row("output", vec![json!({"type":"assemble","rows":parts})]));
    serde_json::to_string_pretty(
        &json!({"version":1,"id":label,"label":label,"rows":rows,"output":"output"}),
    )
    .unwrap()
}
fn mat(id: &str, index: u16) -> Json {
    row(
        id,
        vec![json!({"type":"material","palette":index,"roughness":0.95,"metallic":0})],
    )
}
fn loft(sections: &[([f32; 3], [f32; 2])], material: &str, segments: u32) -> Json {
    json!({"type":"loft","segments":segments,"material":material,"sections":sections.iter().map(|(c,r)|json!({"center":c,"radius":r})).collect::<Vec<_>>()})
}
fn texture(source: Uuid, axes: [usize; 2], span: [f32; 2]) -> Json {
    json!({"type":"texture","source":source.to_string(),"size":64,"axes":axes,"origin":[0,0],"span":span})
}
fn tree(foliage: Uuid, bark: Uuid) -> String {
    let mut rows = vec![mat("bark", 19), mat("leaves", 36)];
    let mut parts = vec!["crooked-trunk".into()];
    rows.push(row(
        "crooked-trunk",
        vec![
            loft(
                &[
                    ([0., 0., 0.], [0.26, 0.22]),
                    ([0.08, 0.35, 0.01], [0.17, 0.15]),
                    ([-0.06, 1.1, 0.02], [0.13, 0.12]),
                    ([0.05, 1.9, -0.04], [0.07, 0.07]),
                    ([0.01, 2.45, 0.01], [0.025, 0.025]),
                ],
                "bark",
                9,
            ),
            texture(bark, [0, 1], [0.5, 1.5]),
        ],
    ));
    // Overlapping, asymmetrical foliage clusters give a broken silhouette and
    // leaf-sized light/shadow groups instead of one smooth conical canopy.
    for (i, (x, y, z, r)) in [
        (-0.55, 1.85, 0.15, 0.47),
        (0.47, 1.95, 0.33, 0.48),
        (-0.1, 2.05, 0.57, 0.44),
        (-0.62, 2.25, -0.16, 0.5),
        (0.5, 2.32, -0.25, 0.49),
        (0.04, 2.55, 0.23, 0.56),
        (-0.25, 2.65, -0.32, 0.49),
        (0.2, 2.94, -0.05, 0.43),
        (-0.53, 2.65, 0.19, 0.34),
        (0.64, 2.54, 0.24, 0.31),
        (-0.26, 3.03, 0.16, 0.32),
        (0.06, 2.28, -0.56, 0.4),
        (-0.75, 2.07, 0.35, 0.25),
        (0.71, 2.12, 0.05, 0.27),
    ]
    .into_iter()
    .enumerate()
    {
        let id = format!("leaf-cluster-{i}");
        rows.push(row(
            &id,
            vec![
                loft(
                    &[
                        ([x, y - 0.23, z], [r * 0.35, r * 0.32]),
                        ([x - 0.04, y, z], [r, r * 0.86]),
                        ([x + 0.04, y + 0.27, z - 0.03], [r * 0.72, r * 0.65]),
                        ([x, y + 0.41, z], [0.06, 0.05]),
                    ],
                    "leaves",
                    7,
                ),
                texture(foliage, [0, 1], [0.7, 0.7]),
            ],
        ));
        parts.push(id);
    }
    finish("Woodland Oak", rows, parts)
}
fn shrub(foliage: Uuid) -> String {
    let mut rows = vec![mat("leaves", 36)];
    let mut parts = vec![];
    for (i, (x, z, r, h)) in [
        (-0.22, 0.05, 0.28, 0.45),
        (0.2, -0.06, 0.3, 0.58),
        (0.02, 0.2, 0.24, 0.4),
    ]
    .into_iter()
    .enumerate()
    {
        let id = format!("bush-{i}");
        rows.push(row(
            &id,
            vec![
                loft(
                    &[
                        ([x, 0.02, z], [r * 0.7, r * 0.65]),
                        ([x, h * 0.55, z], [r, r * 0.9]),
                        ([x + 0.03, h, z], [0.06, 0.05]),
                    ],
                    "leaves",
                    7,
                ),
                texture(foliage, [0, 1], [0.6, 0.6]),
            ],
        ));
        parts.push(id);
    }
    finish("Woodland Shrub", rows, parts)
}
fn rocks(stone: Uuid) -> String {
    let mut rows = vec![mat("stone", 5)];
    let mut parts = vec![];
    for (i, (x, z, r, h)) in [
        (0., 0., 0.33, 0.32),
        (-0.29, 0.18, 0.17, 0.16),
        (0.27, 0.04, 0.14, 0.11),
    ]
    .into_iter()
    .enumerate()
    {
        let id = format!("stone-{i}");
        rows.push(row(
            &id,
            vec![
                loft(
                    &[
                        ([x, 0., z], [r * 0.85, r * 0.8]),
                        ([x - 0.04, h * 0.5, z], [r, r * 0.8]),
                        ([x + 0.05, h, z], [r * 0.35, r * 0.4]),
                    ],
                    "stone",
                    6,
                ),
                texture(stone, [0, 2], [0.8, 0.8]),
            ],
        ));
        parts.push(id);
    }
    finish("Woodland Stones", rows, parts)
}
fn tuft() -> String {
    let mut rows = vec![mat("green", 35), mat("dry", 34)];
    let mut parts = vec![];
    for i in 0..7 {
        let a = i as f32 * 2.39996;
        let (s, c) = a.sin_cos();
        let h = 0.19 + (i % 3) as f32 * 0.055;
        let id = format!("blade-{i}");
        rows.push(row(
            &id,
            vec![loft(
                &[
                    ([c * 0.05, 0., s * 0.05], [0.025, 0.035]),
                    ([c * 0.09, h * 0.65, s * 0.09], [0.02, 0.026]),
                    ([c * 0.14, h, s * 0.14], [0.002, 0.003]),
                ],
                if i == 3 { "dry" } else { "green" },
                5,
            )],
        ));
        parts.push(id);
    }
    finish("Woodland Grass Tuft", rows, parts)
}
fn mesh(project: &mut Project, label: &str, source: String, blocking: bool) -> Uuid {
    let id = Uuid::new_v4();
    project.procedural_appearances.insert(id, source);
    let mut tile = rusterix::Tile::empty();
    tile.id = id;
    tile.alias = label.into();
    tile.blocking = blocking;
    project.tiles.insert(id, tile);
    project.isometric_bakes.insert(
        id,
        shared::isometric::Bake {
            recipe: id,
            ground: false,
            yaw: 0.,
            directions: 4,
            motion: None,
            frames: 1,
            signature: 0,
        },
    );
    id
}
fn sector(map: &mut Map, source: Uuid, label: &str, points: &[[f32; 2]], height: f32) -> u32 {
    map.possible_polygon.clear();
    let vertices: Vec<_> = points
        .iter()
        .map(|p| {
            let id = map.find_free_vertex_id().unwrap();
            map.vertices.push(rusterix::Vertex::new(id, p[0], p[1]));
            id
        })
        .collect();
    for i in 0..vertices.len() {
        map.create_linedef_manual(vertices[i], vertices[(i + 1) % vertices.len()]);
    }
    let id = map.close_polygon_manual().unwrap();
    let s = map.find_sector_mut(id).unwrap();
    s.name = label.into();
    s.layer = Some(1);
    s.properties.set(
        "source",
        Value::Source(rusterix::PixelSource::TileId(source)),
    );
    s.properties.set("floor_height", Value::Float(height));
    id
}
fn place(
    map: &mut Map,
    source: Uuid,
    label: &str,
    x: f32,
    z: f32,
    radius: f32,
    height: f32,
    yaw: f32,
) {
    let points: Vec<_> = (0..8)
        .map(|i| {
            let a = i as f32 * std::f32::consts::TAU / 8.;
            [x + a.cos() * radius, z + a.sin() * radius]
        })
        .collect();
    let id = sector(map, source, label, &points, height);
    map.find_sector_mut(id)
        .unwrap()
        .properties
        .set("iso_yaw", Value::Float(yaw));
}

fn build(mut project: Project) -> Project {
    let colors = shared::palette::parse_colors(include_str!(
        "../../../crates/shared/examples/iso_2d/woodland/steam-lords.hex"
    ))
    .unwrap();
    let plan = shared::palette::PaletteSwitch::prepare(
        &project,
        colors,
        &[vec![37, 36, 35, 34, 33], vec![17, 18, 19, 20, 21, 22]],
    )
    .unwrap();
    plan.after.apply(&mut project);
    project.palette_source_name = "Steam Lords — Slynyrd (Lospec)".into();
    // Preserve the ruleset's material slot identities, including the humanoid.
    for (slot, hex) in [
        (1, "170e19"),
        (4, "433a60"),
        (5, "4f5277"),
        (6, "65738c"),
        (7, "7c94a1"),
        (17, "3b2137"),
        (18, "603b3a"),
        (19, "775c4f"),
        (20, "77744f"),
        (21, "a19f7c"),
        (22, "a19f7c"),
        (33, "a19f7c"),
        (34, "77744f"),
        (35, "4f7754"),
        (36, "3a604a"),
        (37, "213b25"),
    ] {
        project.palette.colors[slot] = Some(TheColor::from_hex(&format!("#{hex}")));
    }
    let swatches: HashSet<_> = project
        .palette_source_colors
        .iter()
        .map(TheColor::to_u8_array)
        .collect();
    assert!(
        project
            .palette
            .colors
            .iter()
            .flatten()
            .all(|c| swatches.contains(&c.to_u8_array())),
        "Every assigned swatch must belong to Steam Lords"
    );
    project.name = "MZ Woodland / Steam Lords".into();
    project.config = super::pixel_style::set_density(&project.config, 24).unwrap();
    let grass = source_recipe(&project, "grass");
    let path = source_recipe(&project, "path");
    let wall_recipe = source_recipe(&project, "wall-x");
    let doc: denrim_materials::geometry::Document =
        serde_json::from_str(&project.procedural_appearances[&wall_recipe]).unwrap();
    let wall = doc
        .rows
        .iter()
        .flat_map(|r| &r.tiles)
        .find_map(|t| match &t.op {
            denrim_materials::geometry::Operation::Texture { source, .. } => {
                Uuid::parse_str(source).ok()
            }
            _ => None,
        })
        .unwrap();
    ramp(&mut project, grass, GRASS, [37, 37, 36, 36]);
    ramp(&mut project, path, EARTH, [17, 18, 19, 20]);
    material_asset(
        &mut project,
        wall,
        &include_str!("../../embedded/materials/05-walls.material")
            .replace("called \"Walls\"", "called \"Woodland / Old Masonry\""),
        &[
            ("wall-colors", "color0", 17),
            ("wall-colors", "color1", 4),
            ("wall-colors", "color2", 5),
            ("wall-colors", "color3", 6),
        ],
    );
    let stone = Uuid::new_v4();
    let foliage = Uuid::new_v4();
    let bark = Uuid::new_v4();
    let bank = Uuid::new_v4();
    ramp(&mut project, stone, STONE, [4, 5, 6, 20]);
    ramp(
        &mut project,
        foliage,
        &GRASS
            .replace("Moss and Grass", "Oak Leaves")
            .replace("to 8", "to 1.1")
            .replace("  tiling raw", "  tiling seamless"),
        [36, 35, 35, 33],
    );
    ramp(
        &mut project,
        bark,
        &EARTH.replace("Worn Earth", "Oak Bark"),
        [18, 19, 20, 21],
    );
    ramp(
        &mut project,
        bank,
        &EARTH.replace("Worn Earth", "Mossy Bank"),
        [37, 18, 19, 36],
    );
    let stone_step = source_recipe(&project, "stone-step");
    let grass_step = source_recipe(&project, "grass-step");
    for (recipe, from, to) in [(stone_step, path, stone)] {
        let old = project.procedural_appearances[&recipe].clone();
        project
            .procedural_appearances
            .insert(recipe, old.replace(&from.to_string(), &to.to_string()));
    }
    let mut hill: Json =
        serde_json::from_str(&project.procedural_appearances[&grass_step]).unwrap();
    hill["rows"][2]["tiles"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"bank-texture","op":texture(bank,[0,1],[1.,0.6])}));
    project
        .procedural_appearances
        .insert(grass_step, serde_json::to_string_pretty(&hill).unwrap());
    let tree_recipe = source_recipe(&project, "tree");
    project
        .procedural_appearances
        .insert(tree_recipe, tree(foliage, bark));
    let tree_tile = source_tile(&project, "tree");
    project
        .isometric_bakes
        .get_mut(&tree_tile)
        .unwrap()
        .directions = 4;
    let shrub = mesh(&mut project, "Woodland / Shrub", shrub(foliage), false);
    let rocks = mesh(&mut project, "Woodland / Stones", rocks(stone), true);
    let tuft = mesh(&mut project, "Woodland / Grass Tuft", tuft(), false);
    project.ensure_material_tiles();
    let grass_tile = source_tile(&project, "grass");
    let map = &mut project.regions[0].map;
    let old: Vec<_> = map
        .sectors
        .iter()
        .filter(|s| s.name == "Grass hill")
        .map(|s| s.id)
        .collect();
    let lines: Vec<_> = map
        .sectors
        .iter()
        .filter(|s| old.contains(&s.id))
        .flat_map(|s| s.linedefs.clone())
        .collect();
    let vertices: Vec<_> = map
        .linedefs
        .iter()
        .filter(|l| lines.contains(&l.id))
        .flat_map(|l| [l.start_vertex, l.end_vertex])
        .collect();
    map.delete_elements(&vertices, &lines, &old);
    for s in &mut map.sectors {
        if s.layer == Some(0) {
            s.properties.set(
                "source",
                Value::Source(rusterix::PixelSource::TileId(grass_tile)),
            );
        }
    }
    // The same terrain controls and subdivision settings used by the 3D engine.
    map.properties.set("terrain_enabled", Value::Bool(true));
    map.properties.set("terrain_subdivisions", Value::Int(4));
    map.properties.set("terrain_edge_falloff", Value::Float(0.));
    map.properties.set(
        "default_terrain_tile",
        Value::Source(rusterix::PixelSource::TileId(grass)),
    );
    for (name, x, z, height, radius) in [
        ("Hill summit", 6.5, 13.5, 1.4, 4.4),
        ("Hill shoulder", 8., 14.5, 0.65, 2.8),
    ] {
        let mut v = rusterix::Vertex::new(map.find_free_vertex_id().unwrap(), x, z);
        v.z = height;
        v.name = name.into();
        v.properties.set("terrain_control", Value::Bool(true));
        v.properties.set("smoothness", Value::Float(radius / 4.));
        map.vertices.push(v);
    }
    // MZ's actual Auto road + uneven-edge + feather controls, shared with the UI.
    let road = Uuid::new_v4();
    let mut road_layer = CompositionLayer::split(path);
    road_layer.shape = CompositionShape::AutoRoad;
    road_layer.position = 0.6;
    road_layer.feather = 0.22;
    road_layer.wobble = 0.08;
    road_layer.wobble_scale = 1.7;
    project.material_compositions.insert(
        road,
        MaterialComposition {
            base: grass,
            layers: vec![road_layer],
            tiling: CompositionTiling::default(),
        },
    );
    let mut t = rusterix::Tile::empty();
    t.id = road;
    t.alias = "Woodland / Winding Earth Path".into();
    project.tiles.insert(road, t);
    let cells: HashSet<_> = [
        (5, 7),
        (5, 8),
        (5, 9),
        (6, 9),
        (7, 9),
        (8, 9),
        (9, 9),
        (9, 8),
        (9, 7),
        (9, 6),
        (8, 6),
        (8, 5),
        (8, 4),
        (4, 8),
        (3, 8),
        (2, 8),
        (1, 8),
        (7, 10),
        (7, 11),
        (6, 11),
        (6, 12),
        (6, 13),
        (7, 13),
        (7, 14),
        (8, 14),
        (8, 15),
        (9, 15),
    ]
    .into_iter()
    .collect();
    project.regions[0].map.properties.set(
        "tiles",
        Value::TileOverrides(
            cells
                .iter()
                .map(|cell| (*cell, rusterix::PixelSource::TileId(road)))
                .collect(),
        ),
    );
    let map = &mut project.regions[0].map;
    for (i, (x, z)) in [
        (1.4, 2.2),
        (1.8, 11.5),
        (1.4, 15.4),
        (10.7, 14.8),
        (13.1, 12.8),
        (18.8, 12.9),
        (19.3, 9.5),
        (15.2, 16.1),
        (13.8, 1.2),
    ]
    .into_iter()
    .enumerate()
    {
        place(
            map,
            tree_tile,
            "Woodland oak",
            x,
            z,
            0.22,
            0.,
            (i % 4) as f32 * 90.,
        );
    }
    for (i, (x, z)) in [
        (2.2, 2.6),
        (1.8, 3.3),
        (2.5, 11.1),
        (1.5, 12.8),
        (2.1, 15.8),
        (9.9, 14.6),
        (11.5, 15.4),
        (12.4, 12.1),
        (13.9, 13.2),
        (18.2, 12.4),
        (19.8, 10.3),
        (15.9, 15.6),
        (3.2, 6.8),
        (7.9, 7.7),
    ]
    .into_iter()
    .enumerate()
    {
        place(
            map,
            shrub,
            "Woodland undergrowth",
            x,
            z,
            0.12,
            0.,
            (i % 4) as f32 * 90.,
        );
    }
    let elevations = map.as_mini(&Default::default()).elevation;
    for (i, (x, z)) in [
        (1.1, 5.9),
        (2.3, 10.4),
        (8.8, 16.4),
        (10.8, 13.4),
        (12.3, 9.2),
        (15.1, 12.7),
        (19.8, 8.7),
        (3.5, 16.4),
        (17.8, 5.8),
        (20.2, 2.8),
    ]
    .into_iter()
    .enumerate()
    {
        let height = elevations.height_at(Vec2::new(x, z));
        place(
            map,
            rocks,
            "Weathered stones",
            x,
            z,
            0.12,
            height,
            (i % 4) as f32 * 90.,
        );
    }
    for i in 0..72 {
        let x = 0.8 + ((i * 37) % 201) as f32 * 0.1;
        let z = 0.8 + ((i * 53) % 161) as f32 * 0.1;
        // Keep the room, stair route, and doorway free of decorative clutter.
        if x > 3. && x < 8.5 && z > 3. && z < 10. {
            continue;
        }
        if x > 8. && z < 7.5 {
            continue;
        }
        let height = elevations.height_at(Vec2::new(x, z));
        place(
            map,
            tuft,
            "Grass tuft",
            x,
            z,
            0.05,
            height,
            (i % 4) as f32 * 90.,
        );
    }
    map.changed = map.changed.wrapping_add(1);
    project
}

/// One connected loop visits each summit; the spur ends in a flat brush clearing.
const PLAYGROUND_LOOP: &[(i32, i32)] = &[
    (9, 15),
    (10, 15),
    (10, 16),
    (12, 16),
    (12, 17),
    (14, 17),
    (14, 16),
    (17, 16),
    (17, 15),
    (18, 15),
    (18, 16),
    (20, 16),
    (20, 17),
    (22, 17),
    (22, 18),
    (24, 18),
    (24, 17),
    (26, 17),
    (26, 16),
    (28, 16),
    (29, 16),
    (29, 17),
    (30, 17),
    (30, 19),
    (31, 19),
    (31, 20),
    (30, 20),
    (30, 21),
    (29, 21),
    (29, 22),
    (28, 22),
    (28, 23),
    (26, 23),
    (26, 24),
    (24, 24),
    (24, 25),
    (22, 25),
    (21, 25),
    (21, 24),
    (20, 24),
    (20, 23),
    (19, 23),
    (19, 22),
    (17, 22),
    (17, 21),
    (16, 21),
    (16, 20),
    (14, 20),
    (14, 19),
    (12, 19),
    (12, 17),
];
const BRUSH_SPUR: &[(i32, i32)] = &[
    (12, 19),
    (11, 19),
    (11, 20),
    (10, 20),
    (10, 21),
    (9, 21),
    (9, 23),
    (10, 23),
    (10, 24),
];
const BRUSH_HILLS: &[(&str, f32, f32, f32, f32)] = &[
    ("East ridge", 17.5, 15.5, 1.55, 4.4),
    ("High meadow", 28.5, 16.5, 2., 5.),
    ("South knoll", 22.5, 25.5, 1.15, 4.2),
    ("Rolling shoulder", 24.5, 22.5, 0.65, 3.5),
];

fn path_cells(points: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let mut result = vec![points[0]];
    for &(x, z) in points.iter().skip(1) {
        let (mut a, mut b) = *result.last().unwrap();
        while a != x || b != z {
            if a != x {
                a += (x - a).signum();
            } else {
                b += (z - b).signum();
            }
            result.push((a, b));
        }
    }
    result
}

fn clear_path_decorations(map: &mut Map, tree: Uuid, stones: Uuid) {
    let cells: Vec<_> = match map.properties.get("tiles") {
        Some(Value::TileOverrides(cells)) => cells
            .keys()
            .map(|&(x, z)| {
                rusterix::BBox::new(
                    Vec2::new(x as f32, z as f32),
                    Vec2::new(x as f32 + 1., z as f32 + 1.),
                )
            })
            .collect(),
        _ => Vec::new(),
    };
    let ids: Vec<_> = map.sectors.iter().filter(|s| matches!(s.properties.get_default_source(),Some(rusterix::PixelSource::TileId(id)) if *id==tree || *id==stones)).map(|s|s.id).collect();
    let mut offsets: Vec<_> = (-4_i32..=4)
        .flat_map(|x| (-4_i32..=4).map(move |z| (x, z)))
        .collect();
    offsets.sort_by_key(|&(x, z)| (x * x + z * z, z, x));
    for id in ids {
        let bounds = map.find_sector(id).unwrap().bounding_box(map);
        if !cells.iter().any(|cell| cell.intersects(&bounds)) {
            continue;
        }
        let delta = offsets
            .iter()
            .find_map(|&(x, z)| {
                let delta = Vec2::new(x as f32, z as f32);
                let shifted = rusterix::BBox::new(bounds.min + delta, bounds.max + delta);
                let room = rusterix::BBox::new(Vec2::new(3., 3.), Vec2::new(8.5, 10.));
                let stairs = rusterix::BBox::new(Vec2::new(8., 1.), Vec2::new(22., 7.5));
                (!shifted.intersects(&room)
                    && !shifted.intersects(&stairs)
                    && !cells.iter().any(|cell| cell.intersects(&shifted)))
                .then_some(delta)
            })
            .expect("decorations must have a clear position beside the path");
        let lines = map.find_sector(id).unwrap().linedefs.clone();
        let vertices: HashSet<_> = lines
            .iter()
            .filter_map(|id| map.find_linedef(*id))
            .flat_map(|l| [l.start_vertex, l.end_vertex])
            .collect();
        for vertex in vertices {
            let v = map.find_vertex_mut(vertex).unwrap();
            v.x += delta.x;
            v.y += delta.y;
        }
        let s = map.find_sector_mut(id).unwrap();
        if let Some(anchor) = s.properties.get_vec2("iso_anchor") {
            s.properties.set(
                "iso_anchor",
                Value::Vec2([anchor[0] + delta.x, anchor[1] + delta.y]),
            );
        }
        s.properties.set("floor_height", Value::Float(0.));
    }
}

fn expand_playground(mut project: Project) -> Project {
    use rusterix::chunkbuilder::terrain_sculpt::{SculptMode, TerrainSculpt};
    let road = *project
        .tiles
        .iter()
        .find(|(_, t)| t.alias == "Woodland / Winding Earth Path")
        .unwrap()
        .0;
    let tree = source_tile(&project, "tree");
    let shrub = *project
        .tiles
        .iter()
        .find(|(_, t)| t.alias == "Woodland / Shrub")
        .unwrap()
        .0;
    let stones = *project
        .tiles
        .iter()
        .find(|(_, t)| t.alias == "Woodland / Stones")
        .unwrap()
        .0;
    project.name = "MZ Terrain Playground / Steam Lords".into();
    project.regions[0].name = "Terrain Playground".into();
    let mut config: toml::Table = project.config.parse().unwrap();
    config
        .get_mut("game")
        .unwrap()
        .as_table_mut()
        .unwrap()
        .insert("start_region".into(), "Terrain Playground".into());
    project.config = toml::to_string_pretty(&config).unwrap();
    let region = &mut project.regions[0];
    let map = &mut region.map;
    map.name = region.name.clone();
    // The original clearing has four invisible collision edges. Extend them
    // with the playable landscape so the new route is reachable on foot.
    let boundary: Vec<_> = map
        .linedefs
        .iter()
        .filter(|line| {
            line.sector_ids.is_empty()
                && (line.properties.get_float_default("wall_width", 0.) - 0.05).abs() < 0.001
        })
        .flat_map(|line| [line.start_vertex, line.end_vertex])
        .collect();
    assert_eq!(boundary.len(), 8, "expected four clearing boundary edges");
    for id in boundary {
        let vertex = map.find_vertex_mut(id).unwrap();
        if (vertex.x - 21.9).abs() < 0.001 {
            vertex.x = 35.9;
        }
        if (vertex.y - 17.9).abs() < 0.001 {
            vertex.y = 30.9;
        }
    }
    for &(_, x, z, height, radius) in BRUSH_HILLS {
        assert!(TerrainSculpt::dab(
            map,
            Vec2::new(x, z),
            radius,
            height,
            SculptMode::Raise
        ));
    }
    let mut cells = match map.properties.get("tiles") {
        Some(Value::TileOverrides(cells)) => cells.clone(),
        _ => Default::default(),
    };
    for cell in path_cells(PLAYGROUND_LOOP)
        .into_iter()
        .chain(path_cells(BRUSH_SPUR))
    {
        cells.insert(cell, rusterix::PixelSource::TileId(road));
    }
    map.properties.set("tiles", Value::TileOverrides(cells));
    // Use the same palette-bound MZ parts around the new route, with generous
    // clearances for the player and an uncluttered practice clearing at 10.5,24.5.
    for (i, (x, z)) in [
        (23.5, 12.5),
        (32., 14.5),
        (33., 20.5),
        (29.5, 26.5),
        (19., 27.5),
        (16.5, 25.5),
        (8., 27.5),
        (6.5, 21.5),
    ]
    .into_iter()
    .enumerate()
    {
        place(
            map,
            tree,
            "Meadow oak",
            x,
            z,
            0.22,
            0.,
            (i % 4) as f32 * 90.,
        );
    }
    for (i, (x, z)) in [
        (21., 14.),
        (25., 14.),
        (32., 17.),
        (32., 23.),
        (25., 27.5),
        (20., 26.5),
        (14., 24.5),
        (7., 25.),
    ]
    .into_iter()
    .enumerate()
    {
        place(
            map,
            shrub,
            "Meadow shrubs",
            x,
            z,
            0.12,
            0.,
            (i % 4) as f32 * 90.,
        );
    }
    for (i, (x, z)) in [
        (20.5, 13.5),
        (27., 13.),
        (32.5, 22.5),
        (27.5, 27.),
        (18., 25.5),
        (7.5, 23.),
    ]
    .into_iter()
    .enumerate()
    {
        place(
            map,
            stones,
            "Meadow stones",
            x,
            z,
            0.12,
            0.,
            (i % 4) as f32 * 90.,
        );
    }
    clear_path_decorations(map, tree, stones);
    let height = rusterix::chunkbuilder::terrain_surface::TerrainSurface::from_map(map)
        .unwrap()
        .height_at(Vec2::new(12.5, 17.5))
        .unwrap();
    for instance in region.characters.values_mut() {
        instance.position = Vec3::new(12.5, height, 17.5);
    }
    map.grid_size = 22.;
    ServerContext::default().center_map_at_grid_pos(
        Vec2::new(1200., 850.),
        Vec2::new(17., 16.),
        map,
    );
    map.changed = map.changed.wrapping_add(1);
    project
}

#[test]
#[ignore = "builds the native woodland demo and renders it on a native GPU; outputs stay in generated/"]
fn build_woodland_iso_demo() {
    build_native_demo(false);
}

#[test]
#[ignore = "builds the walkable terrain-brush playground using native MZ bakes; outputs stay in generated/"]
fn build_iso_terrain_playground() {
    build_native_demo(true);
}

fn build_native_demo(playground: bool) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let input = root.join("generated/mz-iso2d-native/MZIso2D.eldiron");
    let output = root.join(if playground {
        "generated/mz-iso2d-terrain-playground"
    } else {
        "generated/mz-iso2d-woodland"
    });
    let filename = if playground {
        "MZTerrainPlayground.eldiron"
    } else {
        "MZWoodland.eldiron"
    };
    std::fs::create_dir_all(&output).unwrap();
    let mut project =
        build(shared::project_io::decode_project(&std::fs::read(input).unwrap()).unwrap());
    if playground {
        project = expand_playground(project);
    }
    let errors = super::composition::prepare_project(&mut project);
    assert!(errors.is_empty(), "{errors:?}");
    let (device, queue) = pollster::block_on(async {
        let adapter = wgpu::Instance::default()
            .request_adapter(&Default::default())
            .await
            .unwrap();
        adapter.request_device(&Default::default()).await.unwrap()
    });
    let worker = super::gpu::Worker::new(device, queue);
    let density = super::pixel_style::density(&project.config);
    let sources: Vec<_> = project
        .procedural_materials
        .iter()
        .filter(|(id, _)| !shared::ruleset_materials::is_library_tile(&project.config, id))
        .map(|(id, b)| (*id, b.clone()))
        .collect();
    for (id, bundle) in sources {
        eprintln!("Baking source material {}", project.tiles[&id].alias);
        worker.send(super::gpu::Job {
            pixels_per_tile: density,
            tile: id,
            revision: 0,
            bundle: bundle.clone(),
            continuous: false,
            selection: None,
            preview_aspect: 1.,
        });
        let done = worker
            .results
            .recv_timeout(std::time::Duration::from_secs(180))
            .unwrap();
        let frame = done.frame.unwrap();
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
            .insert(id, super::pixel_style::bake_key(&bundle, density));
    }
    let errors = shared::isometric::synchronize(&mut project);
    assert!(errors.is_empty(), "{errors:?}");
    let mini = project.regions[0].map.as_mini(
        &project
            .tiles
            .iter()
            .filter(|(_, t)| t.blocking)
            .map(|(id, _)| *id)
            .collect(),
    );
    let (top, blocked) =
        mini.elevation
            .move_distance(Vec3::new(8.5, 0., 4.5), Vec2::new(10., 0.), 0.22, 0.35, 0.5);
    assert!(!blocked && top == Vec3::new(18.5, 2., 4.5));
    let (hill, blocked) = mini.elevation.move_distance(
        Vec3::new(6.5, mini.elevation.height_at(Vec2::new(6.5, 10.5)), 10.5),
        Vec2::new(0., 3.),
        0.22,
        0.35,
        0.5,
    );
    assert!(
        !blocked && (hill.y - 1.4).abs() < 0.001,
        "Hill: {hill:?}, blocked={blocked}"
    );
    if playground {
        for &(name, x, z, height, _) in BRUSH_HILLS {
            let actual = mini.elevation.height_at(Vec2::new(x, z));
            assert!(actual >= height - 0.001, "{name}: {actual}");
        }
        // Exercise the user's next edit too: raising the flat practice clearing
        // bends its existing MZ path without changing the painted sources.
        let mut edited = project.regions[0].map.clone();
        let paint = edited.properties.get("tiles").cloned();
        assert!(rusterix::chunkbuilder::terrain_sculpt::TerrainSculpt::dab(
            &mut edited,
            Vec2::new(10.5, 24.5),
            2.5,
            0.7,
            rusterix::chunkbuilder::terrain_sculpt::SculptMode::Raise
        ));
        assert_eq!(edited.properties.get("tiles"), paint.as_ref());
        let field = edited.as_mini(&Default::default()).elevation;
        let (top, blocked) = field.move_distance(
            Vec3::new(10.5, 0., 21.5),
            Vec2::new(0., 3.),
            0.22,
            0.35,
            0.5,
        );
        assert!(!blocked && (top.y - 0.7).abs() < 0.001);
        let (back, blocked) = field.move_distance(top, Vec2::new(0., -3.), 0.22, 0.35, 0.5);
        assert!(!blocked && back.y < 0.001);
    }
    let bytes = shared::project_io::encode_project(&project).unwrap();
    let mut reopened = shared::project_io::decode_project(&bytes).unwrap();
    assert_eq!(
        project.material_compositions,
        reopened.material_compositions
    );
    let errors = shared::isometric::synchronize(&mut reopened);
    assert!(errors.is_empty(), "{errors:?}");
    assert!(
        project.procedural_material_cache_keys == reopened.procedural_material_cache_keys,
        "Material bake keys changed on reopen"
    );
    for (id, bake) in &project.isometric_bakes {
        assert_eq!(
            bake.signature, reopened.isometric_bakes[id].signature,
            "Iso bake invalidated on reopen: {id}"
        );
        assert!(
            project.tiles[id].isometric == reopened.tiles[id].isometric,
            "Iso frames changed on reopen: {id}"
        );
    }
    std::fs::write(output.join(filename), &bytes).unwrap();
    // Exercise the actual server's click and keyboard paths, not just the height
    // sampler. The character traverses the painted winding route both ways.
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
        .local_player_teleport_pos(Vec2::new(7.5, 9.5));
    for _ in 0..30 {
        tick(&mut runtime, &mut running);
    }
    let route = [
        (7.5, 9.5),
        (7.5, 10.5),
        (7.5, 11.5),
        (6.5, 11.5),
        (6.5, 12.5),
        (6.5, 13.5),
        (7.5, 13.5),
        (7.5, 14.5),
        (8.5, 14.5),
        (8.5, 15.5),
        (9.5, 15.5),
    ];
    let mut highest = 0_f32;
    for &(x, z) in route.iter().skip(1).chain(route.iter().rev().skip(1)) {
        runtime
            .server
            .local_player_action(rusterix::EntityAction::TerrainClicked(Vec2::new(x, z)));
        let mut arrived = false;
        let mut actual = Vec3::zero();
        for _ in 0..250 {
            tick(&mut runtime, &mut running);
            actual = running.regions[0]
                .map
                .entities
                .iter()
                .find(|e| e.is_player())
                .unwrap()
                .position;
            let expected = mini.elevation.height_at(Vec2::new(actual.x, actual.z));
            assert!(
                (actual.y - expected).abs() < 0.003,
                "Feet left terrain: {actual:?}, ground={expected}"
            );
            highest = highest.max(actual.y);
            if (Vec2::new(actual.x, actual.z) - Vec2::new(x, z)).magnitude() < 0.16 {
                arrived = true;
                break;
            }
        }
        runtime
            .server
            .local_player_action(rusterix::EntityAction::Off);
        assert!(
            arrived,
            "Server winding route failed at {x},{z}: {actual:?}"
        );
    }
    assert!(highest > 1.3);
    for (action, z) in [
        (rusterix::EntityAction::Right, 12.5),
        (rusterix::EntityAction::Left, 9.5),
    ] {
        runtime.server.local_player_action(action);
        let mut arrived = false;
        for _ in 0..300 {
            tick(&mut runtime, &mut running);
            let pos = running.regions[0]
                .map
                .entities
                .iter()
                .find(|e| e.is_player())
                .unwrap()
                .position;
            assert!((pos.y - mini.elevation.height_at(Vec2::new(pos.x, pos.z))).abs() < 0.003);
            if (pos.z - z).abs() < 0.15 {
                arrived = true;
                break;
            }
        }
        runtime
            .server
            .local_player_action(rusterix::EntityAction::Off);
        assert!(arrived, "Server keyboard hill traversal failed");
    }
    if playground {
        for (label, waypoints) in [
            ("hill loop", PLAYGROUND_LOOP),
            ("brush clearing", BRUSH_SPUR),
        ] {
            let cells = path_cells(waypoints);
            let first = cells[0];
            runtime
                .server
                .local_player_teleport_pos(Vec2::new(first.0 as f32 + 0.5, first.1 as f32 + 0.5));
            for _ in 0..20 {
                tick(&mut runtime, &mut running);
            }
            for &(x, z) in cells.iter().skip(1).chain(cells.iter().rev().skip(1)) {
                let goal = Vec2::new(x as f32 + 0.5, z as f32 + 0.5);
                runtime
                    .server
                    .local_player_action(rusterix::EntityAction::TerrainClicked(goal));
                let mut arrived = false;
                let mut actual = Vec3::zero();
                for _ in 0..250 {
                    tick(&mut runtime, &mut running);
                    actual = running.regions[0]
                        .map
                        .entities
                        .iter()
                        .find(|e| e.is_player())
                        .unwrap()
                        .position;
                    assert!(
                        (actual.y - mini.elevation.height_at(Vec2::new(actual.x, actual.z))).abs()
                            < 0.003,
                        "{label}: feet left the visible hill at {actual:?}"
                    );
                    if (Vec2::new(actual.x, actual.z) - goal).magnitude() < 0.16 {
                        arrived = true;
                        break;
                    }
                }
                runtime
                    .server
                    .local_player_action(rusterix::EntityAction::Off);
                assert!(
                    arrived,
                    "{label}: player failed to reach {goal:?}, stopped at {actual:?}"
                );
            }
            eprintln!("Walked {label} both ways ({} cells)", cells.len());
        }
    }
    runtime.server.local_player_teleport_pos(if playground {
        Vec2::new(28.5, 16.5)
    } else {
        Vec2::new(6.5, 13.5)
    });
    for _ in 0..30 {
        tick(&mut runtime, &mut running);
    }
    let mut game = rusterix::client::widget::game::GameWidget::new();
    game.set_camera_mode(rusterix::PlayerCamera::D2Iso);
    game.grid_size = 40.;
    game.buffer = TheRGBABuffer::new(TheDim::sized(1000, 700));
    game.graphical_draw(
        &running.regions[0].map,
        &running.time,
        0,
        &runtime.assets,
        &mut runtime.scene_handler,
    );
    std::fs::write(
        output.join("playing-hill.png"),
        game.buffer.to_png().unwrap(),
    )
    .unwrap();
    let mut assets = rusterix::Assets::default();
    assets.palette = project.palette.clone();
    assets.set_tiles(project.tiles.clone());
    let mut map = project.regions[0].map.clone();
    map.entities.clear();
    let mut actor = rusterix::Entity::new();
    actor.set_attribute(
        "source",
        Value::Source(rusterix::PixelSource::TileId(source_tile(
            &project,
            "character",
        ))),
    );
    actor.set_attribute("player", Value::Bool(true));
    map.entities.push(actor);
    let mut views = vec![
        ("outside", 5.5, 8.7, 40.),
        ("hill", 6.5, 13.5, 48.),
        ("stairs", 13.5, 4.5, 40.),
        ("overview", 11., 9., 24.),
    ];
    if playground {
        views.extend([
            ("east-ridge", 17.5, 15.5, 40.),
            ("high-meadow", 28.5, 16.5, 40.),
            ("south-knoll", 22.5, 25.5, 40.),
            ("brush-clearing", 10.5, 24.5, 40.),
            ("playground-overview", 17., 16., 18.),
        ]);
    }
    for (name, x, z, unit) in views {
        let h = mini.elevation.height_at(Vec2::new(x, z));
        map.entities[0].position = Vec3::new(x, h, z);
        let mut p = rusterix::isometric::Projection {
            origin: Vec2::zero(),
            unit,
        };
        p.origin = Vec2::new(600., 470.) - p.screen(Vec2::new(x, z), h);
        let mut image = TheRGBABuffer::new(TheDim::sized(1200, 850));
        rusterix::isometric::draw(&map, &assets, &mut image, p, 0.);
        std::fs::write(output.join(format!("{name}.png")), image.to_png().unwrap()).unwrap();
    }
    eprintln!("Woodland scene: {}", output.join(filename).display());
}
