//! Explicit opt-in: needs macOS, Swift and a functioning Metal device.
#![cfg(target_os = "macos")]
use denrim_materials::{compile, cpu::Evaluator, ir::Channel, metal};
use std::{fs, process::Command};

#[test]
#[ignore = "requires a Metal device; run cargo test --test metal_runtime -- --ignored --nocapture"]
fn cpu_and_metal_agree_on_real_device() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("work/metal-validation");
    fs::create_dir_all(&dir).unwrap();
    let harness = dir.join("compare");
    let result = Command::new("xcrun")
        .args(["swiftc", "-module-cache-path"])
        .arg(dir.join("module-cache"))
        .arg(root.join("tests/metal_compare.swift"))
        .arg("-o")
        .arg(&harness)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let base = include_str!("../examples/warped-disc.material");
    let gradient=base.replace("set \"inBEVEL\" to 0.25", "set \"inBEVEL\" to 0.25\n      set \"GRAD\" to true\n      set \"inStrengthGradient\" to 0.2");
    let coordinates = base.replace(
        "row \"shape\" called \"Disc\"",
        "row \"shape\" called \"Disc\"\n    coordinates from row \"noise\" xy mapped -0.5 to 0.5",
    );
    let negative = base.replace(
        "set \"localScale\" to 1 1.3",
        "set \"localScale\" to -1 1.3",
    );
    for (name, source) in [
        ("demo", base),
        ("gradient", &gradient),
        ("coordinates", &coordinates),
        ("negative", &negative),
    ] {
        let p = compile(source).unwrap();
        let e = Evaluator::new(&p);
        for (material, channel) in [
            ("demo", Channel::BaseColor),
            ("demo", Channel::Height),
            ("demo", Channel::Roughness),
            ("noise-study", Channel::BaseColor),
        ] {
            let shader = dir.join("program.metal");
            let reference = dir.join("cpu.rgba32f");
            fs::write(&shader, metal::generate(&p, material, channel).unwrap()).unwrap();
            let pixels = e.render(material, channel, 73, 61).unwrap();
            let mut bytes = vec![];
            for p in pixels {
                let rgb = if p.len() == 1 {
                    [p[0]; 3]
                } else {
                    [p[0], p[1], p[2]]
                };
                for v in [rgb[0], rgb[1], rgb[2], 1.] {
                    bytes.extend(v.to_le_bytes());
                }
            }
            fs::write(&reference, bytes).unwrap();
            let result = Command::new(&harness)
                .arg(&shader)
                .arg(&reference)
                .args(["73", "61"])
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{name} {material} {channel:?}: {}{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            println!(
                "{name} {material} {channel:?}: {}",
                String::from_utf8_lossy(&result.stdout).trim()
            );
        }
    }
    // User-authored expressions exercise broadcast calls, scalar swizzles and
    // additional BSDF channel widths through the same CPU/Metal IR.
    let text = "project 1\nmaterial \"custom\" called \"Custom\"\n  row \"c\"\n    use \"Color\"\n  output normal from row \"c\" xyz\n  output ior from row \"c\" x\n  output emission strength from row \"c\" y\n";
    let mut ast = denrim_materials::format::parse(text).unwrap();
    ast.materials[0].rows[0].tiles[0].code=Some(denrim_materials::tilelang::source(denrim_materials::metadata::Kernel::Color).replace(
        "image = vec4(color.rgb, clamp(color.a, 0, 1) * clamp(input.a, 0, 1))",
        "let mask = clamp(uv.x + 0.5, 0, 1)\nlet scalar = 0.25\nimage = vec4(mix(scalar.xxx, color.rgb, mask), 1)"));
    let custom = denrim_materials::ir::Program::lower(&ast).unwrap();
    for channel in [Channel::Normal, Channel::Ior, Channel::EmissionStrength] {
        let shader = dir.join("custom.metal");
        let reference = dir.join("custom.rgba32f");
        fs::write(
            &shader,
            metal::generate(&custom, "custom", channel).unwrap(),
        )
        .unwrap();
        let pixels = Evaluator::new(&custom)
            .render("custom", channel, 73, 61)
            .unwrap();
        let bytes = pixels
            .into_iter()
            .flat_map(|v| {
                if v.len() == 1 {
                    [v[0], v[0], v[0], 1.]
                } else {
                    [v[0], v[1], v[2], 1.]
                }
            })
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>();
        fs::write(&reference, bytes).unwrap();
        let result = Command::new(&harness)
            .arg(&shader)
            .arg(&reference)
            .args(["73", "61"])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "custom {channel:?}: {}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        println!(
            "custom {channel:?}: {}",
            String::from_utf8_lossy(&result.stdout).trim()
        );
    }
    let defaults=compile("project 1\nmaterial \"d\" called \"Defaults\"\n  default base color to 0.1 0.3 0.7\n  default roughness to 0.37\n  default metallic to 0.8\n  row \"r\"\n    use \"Color\"\n").unwrap();
    for channel in [
        Channel::BaseColor,
        Channel::Roughness,
        Channel::Metallic,
        Channel::EmissionStrength,
    ] {
        let compiled = metal::compile(&defaults, "d", channel).unwrap();
        fs::write(dir.join("default.metal"), &compiled.source).unwrap();
        fs::write(dir.join("default.params"), compiled.parameter_bytes()).unwrap();
        let pixels = Evaluator::new(&defaults)
            .render("d", channel, 73, 61)
            .unwrap();
        let bytes = pixels
            .into_iter()
            .flat_map(|v| {
                if v.len() == 1 {
                    [v[0], v[0], v[0], 1.]
                } else {
                    [v[0], v[1], v[2], 1.]
                }
            })
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>();
        fs::write(dir.join("default.rgba32f"), bytes).unwrap();
        let result = Command::new(&harness)
            .arg(dir.join("default.metal"))
            .arg(dir.join("default.rgba32f"))
            .args(["73", "61"])
            .arg(dir.join("default.params"))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "default {channel:?}: {}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        println!(
            "default {channel:?}: {}",
            String::from_utf8_lossy(&result.stdout).trim()
        );
    }
}
