//! Isolated, CPU-baked MZ / Canvas 2D isometric experiment. No runtime 3D renderer.
//! cargo run -p eldiron-shared --example mz_iso_2d -- generated/mz-iso2d
use base64::{Engine as _, engine::general_purpose::STANDARD};
use denrim_materials::geometry::{Document, Operation, Value};
use eldiron_shared::{
    isometric::{Baker, Sprite, plane},
    project::Project,
    rulesets,
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use theframework::prelude::*;

const HALF_W: f32 = 32.;
const HALF_H: f32 = 16.;
const VERTICAL: f32 = 39.1918359;
const WALK_FRAMES: usize = 16;

#[derive(Serialize)]
struct Manifest {
    images: BTreeMap<String, String>,
    sprites: BTreeMap<String, Sprite>,
    character: Vec<Vec<Sprite>>,
    frames: usize,
    walk_seconds: f32,
    projection: [f32; 3],
    provenance: Vec<String>,
}
fn save_image(
    out: &Path,
    name: &str,
    rgba: Vec<u8>,
    width: usize,
    height: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    let png = TheRGBABuffer::from(rgba, width as u32, height as u32).to_png()?;
    std::fs::write(out.join(format!("{name}.png")), &png)?;
    Ok(format!("data:image/png;base64,{}", STANDARD.encode(png)))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or("generated/mz-iso2d".into()),
    );
    std::fs::create_dir_all(&out)?;
    let project = Project::default();
    let mut baker = Baker {
        project: &project,
        textures: BTreeMap::new(),
    };
    baker.material(
        "experiment:grass",
        include_str!("../../../creator/embedded/materials/01-grass.material"),
    )?;
    baker.material(
        "experiment:wall",
        include_str!("../../../creator/embedded/materials/05-walls.material"),
    )?;
    baker.material(
        "experiment:path",
        include_str!("../../../creator/embedded/materials/04-street.material"),
    )?;
    let mut manifest = Manifest {
        images: BTreeMap::new(),
        sprites: BTreeMap::new(),
        character: vec![],
        frames: WALK_FRAMES,
        walk_seconds: 1.,
        projection: [HALF_W, HALF_H, VERTICAL],
        provenance: vec![
            "Ground/path: MZ procedural material recipes".into(),
            "Wall/tree: editable MZ geometry baked once into 2D sprites".into(),
            "Character: official humanoid MZ body, textures and walk rig".into(),
            "Runtime: Canvas 2D sprite blits; no meshes or WebGL".into(),
        ],
    };
    for (key, mesh, yaw) in [
        ("grass", plane("experiment:grass"), 0.),
        ("path", plane("experiment:path"), 0.),
        (
            "wall-x",
            Document::load(include_str!("iso_2d/wall.mzgeometry.json"))?
                .evaluate()?
                .mesh,
            0.,
        ),
        (
            "wall-z",
            Document::load(include_str!("iso_2d/wall.mzgeometry.json"))?
                .evaluate()?
                .mesh,
            90.,
        ),
        (
            "corner-0",
            Document::load(include_str!("iso_2d/corner.mzgeometry.json"))?
                .evaluate()?
                .mesh,
            0.,
        ),
        (
            "corner-90",
            Document::load(include_str!("iso_2d/corner.mzgeometry.json"))?
                .evaluate()?
                .mesh,
            90.,
        ),
        (
            "corner-180",
            Document::load(include_str!("iso_2d/corner.mzgeometry.json"))?
                .evaluate()?
                .mesh,
            180.,
        ),
        (
            "corner-270",
            Document::load(include_str!("iso_2d/corner.mzgeometry.json"))?
                .evaluate()?
                .mesh,
            270.,
        ),
        (
            "tree",
            Document::load(include_str!("iso_2d/tree.mzgeometry.json"))?
                .evaluate()?
                .mesh,
            0.,
        ),
    ] {
        let baked = baker.bake(key, &mesh, yaw, false)?;
        manifest.images.insert(
            key.into(),
            save_image(&out, key, baked.rgba, baked.meta.w, baked.meta.h)?,
        );
        manifest.sprites.insert(key.into(), baked.meta);
    }
    eprintln!("Baked MZ ground, path, straight walls, four corners and tree sprites.");
    let source = rulesets::bundled_appearance_assets_for_ruleset(
        rulesets::OFFICIAL_RULESET_ID,
        rulesets::OFFICIAL_RULESET_VERSION,
    )
    .into_iter()
    .find(|a| a.id == "humanoid")
    .ok_or("Humanoid recipe missing")?;
    let mut doc = Document::load(source.source)?;
    // Use the production body/rig/materials. Equipment is unnecessary for this
    // walking test; preserve the human row so the existing motion rig stays valid.
    doc.rows
        .retain(|r| ["body", "human", "idle", "walk"].contains(&r.id.as_str()));
    if let Operation::Assemble { rows } =
        &mut doc.rows.iter_mut().find(|r| r.id == "human").unwrap().tiles[0].op
    {
        *rows = vec!["body".into()];
    }
    let evaluation = doc.evaluate()?;
    let Some(Value::Motion(walk)) = evaluation.rows.get("walk") else {
        return Err("Walk motion missing".into());
    };
    manifest.walk_seconds = walk.duration;
    let cols = WALK_FRAMES + 1;
    let width = cols * 128;
    let height = 8 * 128;
    let mut atlas = vec![0; width * height * 4];
    for direction in 0..8 {
        let mut frames = vec![];
        for frame in 0..cols {
            let motion = if frame == WALK_FRAMES {
                None
            } else {
                Some(walk)
            };
            let mesh = evaluation
                .mesh
                .posed(motion, frame as f32 / WALK_FRAMES as f32 * walk.duration)?
                .mesh;
            let mut baked = baker.bake("character", &mesh, direction as f32 * 45., true)?;
            for y in 0..128 {
                let target = ((direction * 128 + y) * width + frame * 128) * 4;
                atlas[target..target + 512].copy_from_slice(&baked.rgba[y * 512..y * 512 + 512]);
            }
            baked.meta.x = frame * 128;
            baked.meta.y = direction * 128;
            frames.push(baked.meta);
        }
        manifest.character.push(frames);
    }
    // The walk cycle must have actual changing poses, not repeated stills.
    assert!(manifest.character.len() == 8);
    let first = (0..128)
        .flat_map(|y| atlas[y * width * 4..y * width * 4 + 512].to_vec())
        .collect::<Vec<_>>();
    let middle = (0..128)
        .flat_map(|y| {
            atlas[y * width * 4 + 4 * 128 * 4..y * width * 4 + 4 * 128 * 4 + 512].to_vec()
        })
        .collect::<Vec<_>>();
    assert_ne!(first, middle, "Walk atlas contains no animation");
    manifest.images.insert(
        "character".into(),
        save_image(&out, "character", atlas, width, height)?,
    );
    let data = serde_json::to_string(&manifest)?;
    std::fs::write(
        out.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    let html = include_str!("iso_2d/demo.html")
        .replace("/*__WORLD__*/", include_str!("iso_2d/world.js"))
        .replace("/*__ASSETS__*/", &data);
    std::fs::write(out.join("index.html"), html)?;
    std::fs::write(
        out.join("README.txt"),
        "Open index.html directly in a browser. All images are embedded; no server or network is required. Click to walk, use WASD/arrows, or enable Auto walk. Debug footprints show anchors and collision. This is an isolated Canvas 2D experiment, not a new Eldiron map mode.\n",
    )?;
    eprintln!(
        "Baked 8 directions × 16 walking frames + idle. Open {}/index.html",
        out.display()
    );
    Ok(())
}
