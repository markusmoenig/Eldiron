use denrim_materials::{
    compile,
    cpu::{Evaluator, PreviewMode},
    format,
    ir::Channel,
    metal,
};
fn text(mode: &str, source: &str) -> String {
    format!(
        "project 1\nmaterial \"m\" called \"M\"\n  tiling {mode}\n  row \"r\"\n    use \"{source}\"\n  output base color from row \"r\" xyz\n  output height from row \"r\" x\n"
    )
}
fn value(p: &denrim_materials::ir::Program, uv: [f32; 2]) -> f32 {
    Evaluator::new(p).sample("m", Channel::Height, uv).unwrap()[0]
}
#[test]
fn seamless_matches_edges_slopes_and_preserves_the_center() {
    let raw = compile(&text("raw", "Tex Coord 2D")).unwrap();
    let repeat = compile(&text("repeat", "Tex Coord 2D")).unwrap();
    let smooth = compile(&text("seamless", "Tex Coord 2D")).unwrap();
    let e = 0.0001;
    assert!(
        (value(&repeat, [-0.5 + e, 0.]) - value(&repeat, [0.5 - e, 0.])).abs() > 0.9,
        "Repeat alone must not claim smooth seams"
    );
    for y in [-0.43, -0.12, 0.16, 0.46] {
        assert!((value(&smooth, [-0.5 + e, y]) - value(&smooth, [0.5 - e, y])).abs() < 0.001);
        let left = (value(&smooth, [-0.5 + 2. * e, y]) - value(&smooth, [-0.5 + e, y])) / e;
        let right = (value(&smooth, [0.5 - e, y]) - value(&smooth, [0.5 - 2. * e, y])) / e;
        assert!((left - right).abs() < 0.02, "Slopes {left} {right}");
    }
    for uv in [[0.13, -0.21], [-0.3, 0.1]] {
        assert!((value(&smooth, uv) - value(&raw, uv)).abs() < 1e-6);
    }
    let source = text("seamless", "Perlin Noise")
        .replace("  tiling seamless", "  tiling seamless\n  period 2 3");
    let p = compile(&source).unwrap();
    for uv in [[0.93, 1.4], [-0.96, -1.47], [0.13, -0.2]] {
        assert!((value(&p, uv) - value(&p, [uv[0] + 2., uv[1]])).abs() < 0.0001);
        assert!((value(&p, uv) - value(&p, [uv[0], uv[1] + 3.])).abs() < 0.0001);
    }
    let ast = format::parse(&source).unwrap();
    assert_eq!(ast, format::parse(&format::write(&ast)).unwrap());
    assert!(compile(&source.replace("period 2 3", "period 0 3")).is_err());
    assert!(compile(&source.replace("period 2 3", "seam blend 0")).is_err());
}
#[test]
fn live_tiling_and_alpha_previews_share_the_ir() {
    let source = text("seamless", "Circle");
    let p = compile(&source).unwrap();
    let mut shader = metal::compile(&p, "m", Channel::BaseColor).unwrap();
    let initial = shader.parameter_bytes();
    let changed = compile(&source.replace(
        "tiling seamless",
        "tiling seamless\n  period 2 2\n  seam blend 0.25",
    ))
    .unwrap();
    shader.refresh(&changed).unwrap();
    assert_ne!(initial, shader.parameter_bytes());
    let pixels = Evaluator::new(&p)
        .render_tile("m", "r", 0, PreviewMode::Image, 17, 13)
        .unwrap();
    assert!(pixels.iter().any(|p| p[3] == 0.) && pixels.iter().any(|p| p[3] > 0.9));
    let raw = compile(&text("raw", "Circle")).unwrap();
    assert_eq!(
        Evaluator::new(&p)
            .sample_tile("m", "r", 0, [0., 0.], 0.001)
            .unwrap()
            .distance,
        Evaluator::new(&raw)
            .sample_tile("m", "r", 0, [0., 0.], 0.001)
            .unwrap()
            .distance
    );
    let sdf = source.replace(
        "output height from row \"r\" x",
        "output height from row \"r\" distance",
    );
    assert!(value(&compile(&sdf).unwrap(), [0.49, 0.49]).is_finite());
}
#[test]
fn mixed_materials_respect_input_periodicity() {
    let source = text("seamless", "Perlin Noise")
        + "material \"constant\" called \"Constant\"\nmaterial \"mixed\" called \"Mixed\"\n  mix \"Mask\"\n  mixing input 1 from material \"m\"\n  mixing input 2 from material \"constant\"\n";
    let p = compile(&source).unwrap();
    let e = Evaluator::new(&p);
    assert_eq!(
        e.sample("mixed", Channel::BaseColor, [-0.5, 0.173])
            .unwrap(),
        e.sample("mixed", Channel::BaseColor, [0.5, 0.173]).unwrap()
    );
}
#[test]
#[ignore = "requires a macOS Metal device"]
fn seamless_materials_and_alpha_match_metal() {
    use std::{fs, process::Command};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("work/seamless-metal");
    fs::create_dir_all(&dir).unwrap();
    let harness = dir.join("compare");
    assert!(
        Command::new("xcrun")
            .args(["swiftc", "-module-cache-path"])
            .arg(dir.join("module-cache"))
            .arg(root.join("tests/metal_compare.swift"))
            .arg("-o")
            .arg(&harness)
            .status()
            .unwrap()
            .success()
    );
    for mode in ["repeat", "seamless"] {
        for source in ["Perlin Noise", "Circle"] {
            for preview in [false, true] {
                let p = compile(&text(mode, source)).unwrap();
                let shader = if preview {
                    metal::compile_tile_preview(&p, "m", "r", 0, PreviewMode::Image).unwrap()
                } else {
                    metal::compile(&p, "m", Channel::BaseColor).unwrap()
                };
                let pixels = if preview {
                    Evaluator::new(&p)
                        .render_tile("m", "r", 0, PreviewMode::Image, 19, 17)
                        .unwrap()
                } else {
                    Evaluator::new(&p)
                        .render("m", Channel::BaseColor, 19, 17)
                        .unwrap()
                        .iter()
                        .map(|v| [v[0], v[1], v[2], 1.])
                        .collect()
                };
                let path = dir.join("shader.metal");
                let expected = dir.join("expected.bin");
                let params = dir.join("params.bin");
                fs::write(&path, &shader.source).unwrap();
                fs::write(&params, shader.parameter_bytes()).unwrap();
                fs::write(
                    &expected,
                    pixels
                        .into_iter()
                        .flatten()
                        .flat_map(f32::to_le_bytes)
                        .collect::<Vec<_>>(),
                )
                .unwrap();
                let output = Command::new(&harness)
                    .arg(path)
                    .arg(expected)
                    .args(["19", "17"])
                    .arg(params)
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "{mode} {source} preview={preview}: {} {}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }
}
