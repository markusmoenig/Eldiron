#![cfg(target_os = "macos")]
use denrim_materials::{
    compile,
    cpu::{Evaluator, PreviewMode},
    ir::Channel,
    metal,
};
use std::{fs, path::Path, process::Command};
#[test]
#[ignore = "requires a Metal device; run cargo test --test sdf_metal_runtime -- --ignored --nocapture"]
fn icon_outputs_and_all_tile_preview_modes_match_cpu() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("work/sdf-metal-validation");
    fs::create_dir_all(&dir).unwrap();
    let harness = dir.join("compare");
    let status = Command::new("xcrun")
        .args(["swiftc", "-module-cache-path"])
        .arg(dir.join("cache"))
        .arg(root.join("tests/metal_compare.swift"))
        .arg("-o")
        .arg(&harness)
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let p = compile(include_str!("../examples/icon.material")).unwrap();
    let e = Evaluator::new(&p);
    let run = |shader: metal::Shader, pixels: Vec<[f32; 4]>| {
        fs::write(dir.join("icon.metal"), &shader.source).unwrap();
        fs::write(dir.join("icon.params"), shader.parameter_bytes()).unwrap();
        fs::write(
            dir.join("cpu.rgba32f"),
            pixels
                .into_iter()
                .flatten()
                .flat_map(f32::to_le_bytes)
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let result = Command::new(&harness)
            .arg(dir.join("icon.metal"))
            .arg(dir.join("cpu.rgba32f"))
            .args(["73", "61"])
            .arg(dir.join("icon.params"))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        println!("{}", String::from_utf8_lossy(&result.stdout).trim());
    };
    for channel in [Channel::BaseColor, Channel::Opacity, Channel::Distance] {
        let pixels = e
            .render("icon", channel, 73, 61)
            .unwrap()
            .into_iter()
            .map(|v| {
                if v.len() == 1 {
                    [v[0], v[0], v[0], 1.]
                } else {
                    [v[0], v[1], v[2], 1.]
                }
            })
            .collect();
        run(metal::compile(&p, "icon", channel).unwrap(), pixels);
    }
    for (material, row, tile) in [
        ("icon", "silhouette", 0),
        ("icon", "silhouette", 1),
        ("icon", "styled", 1),
        ("icon", "textured", 2),
        ("combinations", "union", 1),
        ("combinations", "smooth", 1),
        ("combinations", "intersection", 1),
    ] {
        for mode in [
            PreviewMode::Image,
            PreviewMode::Shape,
            PreviewMode::Distance,
        ] {
            println!("{material}/{row}/{tile} {}", mode.name());
            run(
                metal::compile_tile_preview(&p, material, row, tile, mode).unwrap(),
                e.render_tile(material, row, tile, mode, 73, 61).unwrap(),
            );
        }
    }
    // Uniform transform keeps an SDF and adjusts antialias footprint consistently.
    let source=include_str!("../examples/icon.material").replace("  row \"styled\"", "    use \"Transform\"\n      set \"globalScale\" to 0.7\n      set \"rot\" to 20\n  row \"styled\"");
    let p = compile(&source).unwrap();
    let e = Evaluator::new(&p);
    run(
        metal::compile_tile_preview(&p, "icon", "styled", 1, PreviewMode::Image).unwrap(),
        e.render_tile("icon", "styled", 1, PreviewMode::Image, 73, 61)
            .unwrap(),
    );
    // The same Border tile works on image alpha without an advertised distance.
    let p = compile(include_str!("../examples/raster-border.material")).unwrap();
    let e = Evaluator::new(&p);
    run(
        metal::compile_tile_preview(&p, "image", "icon", 3, PreviewMode::Image).unwrap(),
        e.render_tile("image", "icon", 3, PreviewMode::Image, 73, 61)
            .unwrap(),
    );
    // A normal Color is also a standalone RGBA source.
    let p=compile("project 1\nmaterial \"m\" called \"M\"\n  row \"c\" called \"Color\"\n    use \"Color\"\n      set \"color\" to 0.1 0.2 0.3 0.4\n  output base color from row \"c\" xyz\n").unwrap();
    run(
        metal::compile_tile_preview(&p, "m", "c", 0, PreviewMode::Image).unwrap(),
        Evaluator::new(&p)
            .render_tile("m", "c", 0, PreviewMode::Image, 73, 61)
            .unwrap(),
    );
}
