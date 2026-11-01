use denrim_materials::{compile, cpu::Evaluator, format, ir::Channel, metal};
fn project(kind: &str, settings: &str) -> String {
    format!(
        r#"project 1
material "result" called "Result"
  mix "{kind}"
  mixing input 1 from material "a"
  mixing input 2 from material "b"
{settings}
material "a" called "A"
  default base color to 1 0 0
  default metallic to 0.9
  default roughness to 0.2
  default height to 0.1
  default transmission to 0.8
  default coat to 0.7
material "b" called "B"
  default base color to 0 0 1
  default metallic to 0.1
  default roughness to 0.8
  default height to 0.3
  default transmission to 0.2
  default coat to 0.1
"#
    )
}
fn sample(text: &str, c: Channel, uv: [f32; 2]) -> Vec<f32> {
    Evaluator::new(&compile(text).unwrap())
        .sample("result", c, uv)
        .unwrap()
}
#[test]
fn mask_blends_all_channels_and_preserves_endpoints() {
    let source = project("Mask", "  mixing mask to 0.25 0.5 0.75");
    let color = sample(&source, Channel::BaseColor, [0.; 2]);
    assert_eq!(color, vec![0.75, 0., 0.75]);
    assert!((sample(&source, Channel::Coat, [0.; 2])[0] - 0.55).abs() < 1e-6);
    assert!((sample(&source, Channel::Transmission, [0.; 2])[0] - 0.65).abs() < 1e-6);
    for (mask, material) in [(0, "a"), (1, "b")] {
        let text = project("Mask", &format!("  mixing mask to {mask} {mask} {mask}"));
        let p = compile(&text).unwrap();
        let e = Evaluator::new(&p);
        for c in Channel::SURFACE {
            assert_eq!(
                e.sample("result", *c, [0.; 2]).unwrap(),
                e.sample(material, *c, [0.; 2]).unwrap(),
                "{}",
                c.name()
            );
        }
    }
    let off = project("Mask", "  mixing blend false\n  mixing mask to 0.5 0.5 0.5");
    assert_eq!(sample(&off, Channel::Opacity, [0.; 2]), vec![0.]);
}
#[test]
fn height_overlay_and_uv_follow_original_rules() {
    assert_eq!(
        sample(&project("Bump Height", ""), Channel::BaseColor, [0.; 2]),
        vec![0., 0., 1.]
    );
    let overlay = project("Overlay", "  mixing levels 0.2 0.5 0.75");
    assert_eq!(
        sample(&overlay, Channel::BaseColor, [0.; 2]),
        vec![0., 0., 1.]
    );
    assert!((sample(&overlay, Channel::Height, [0.; 2])[0] - 0.4).abs() < 1e-6);
    let text = project(
        "UV Position",
        "  mixing input 3 from material \"a\"\n  mixing input 4 from material \"b\"",
    );
    for (x, color) in [
        (-0.4, vec![1., 0., 0.]),
        (-0.1, vec![0., 0., 1.]),
        (0.1, vec![1., 0., 0.]),
        (0.4, vec![0., 0., 1.]),
    ] {
        assert_eq!(sample(&text, Channel::BaseColor, [x, 0.]), color);
    }
}
#[test]
fn mixing_round_trips_reuses_materials_and_rejects_cycles() {
    let mut text = project("Mask", "  mixing mask to 0.5 0.5 0.5");
    text.push_str("material \"nested\" called \"Nested\"\n  mix \"Bump Height\"\n  mixing input 1 from material \"result\"\n  mixing input 2 from material \"a\"\n");
    let ast = format::parse(&text).unwrap();
    let pinned = text.replace("project 1", "project 1\nresult from material \"nested\"");
    let pinned_ast = format::parse(&pinned).unwrap();
    assert_eq!(
        format::parse(&format::write(&pinned_ast)).unwrap(),
        pinned_ast
    );
    assert!(
        compile(&pinned.replace(
            "result from material \"nested\"",
            "result from material \"unknown\""
        ))
        .is_err()
    );
    assert_eq!(format::parse(&format::write(&ast)).unwrap(), ast);
    let p = compile(&text).unwrap();
    assert_eq!(
        Evaluator::new(&p)
            .sample("nested", Channel::Coat, [0.; 2])
            .unwrap(),
        Evaluator::new(&p)
            .sample("result", Channel::Coat, [0.; 2])
            .unwrap()
    );
    assert!(
        denrim_materials::live::Document::parse(&text)
            .unwrap()
            .json()
            .contains("\"mixing\":{\"kind\":\"Mask\"")
    );
    let cycle = text.replace(
        "material \"a\" called \"A\"",
        "material \"a\" called \"A\"\n  mix \"Mask\"\n  mixing input 1 from material \"nested\"",
    );
    assert!(
        compile(&cycle)
            .unwrap_err()
            .to_string()
            .contains("Material dependency cycle")
    );
    assert!(compile(&text.replace("from material \"a\"", "from material \"missing\"")).is_err());
    let mut shader = metal::compile(&p, "nested", Channel::Roughness).unwrap();
    let changed = compile(&text.replace("roughness to 0.2", "roughness to 0.4")).unwrap();
    let before = shader.parameter_bytes();
    shader.refresh(&changed).unwrap();
    assert_ne!(before, shader.parameter_bytes());
}
#[test]
#[ignore = "requires a macOS Metal device"]
fn full_material_mix_matches_metal() {
    use std::{fs, process::Command};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let work = root.join("work/material-mix-metal");
    fs::create_dir_all(&work).unwrap();
    let harness = work.join("compare");
    assert!(
        Command::new("xcrun")
            .args(["swiftc", "-module-cache-path"])
            .arg(work.join("module-cache"))
            .arg(root.join("tests/metal_compare.swift"))
            .arg("-o")
            .arg(&harness)
            .status()
            .unwrap()
            .success()
    );
    for kind in ["Mask", "Bump Height", "Overlay", "UV Position"] {
        let settings = if kind == "Mask" {
            "  mixing mask from row \"mask\" xyz\n  row \"mask\"\n    use \"Perlin Noise\""
        } else {
            ""
        };
        let p = compile(&project(kind, settings)).unwrap();
        for channel in Channel::SURFACE {
            let path = work.join("test.metal");
            fs::write(&path, metal::generate(&p, "result", *channel).unwrap()).unwrap();
            let pixels = Evaluator::new(&p).render("result", *channel, 7, 5).unwrap();
            let expected = work.join("expected.bin");
            let bytes = pixels
                .iter()
                .flat_map(|p| {
                    let v = if p.len() == 1 {
                        [p[0], p[0], p[0], 1.]
                    } else {
                        [p[0], p[1], p[2], 1.]
                    };
                    v.into_iter().flat_map(f32::to_le_bytes)
                })
                .collect::<Vec<_>>();
            fs::write(&expected, bytes).unwrap();
            let output = Command::new(&harness)
                .arg(&path)
                .arg(&expected)
                .args(["7", "5"])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{kind} {}: {} {}",
                channel.name(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

#[test]
fn stack_slots_persist_and_combine_material_channels() {
    let source = project("Mask", "  stack slot 3\n  mixing mask to 0.5 0.5 0.5");
    let ast = format::parse(&source).unwrap();
    assert_eq!(ast.materials[0].stack_slot, Some(2));
    assert_eq!(format::parse(&format::write(&ast)).unwrap(), ast);
    assert_eq!(
        sample(&source, Channel::BaseColor, [0.; 2]),
        vec![0.5, 0., 0.5]
    );
    for setting in [
        "stack slot 0",
        "stack slot 5",
        "stack slot 1\n  stack slot 2",
    ] {
        assert!(format::parse(&project("Mask", &format!("  {setting}"))).is_err());
    }
    assert!(format::parse("project 1\nmaterial \"a\" called \"A\"\n  stack slot 1\n").is_err());
    let duplicate = format!(
        "{}\nmaterial \"other\" called \"Other\"\n  mix \"Mask\"\n  stack slot 3\n",
        source
    );
    assert!(format::parse(&duplicate).is_err());
}
