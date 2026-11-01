use denrim_materials::{compile, cpu::Evaluator, format, ir::Channel, metadata::Kernel, metal};

fn source(name: &str, settings: &str, after: &str) -> String {
    format!(
        "project 1\nmaterial \"m\" called \"Noise\"\n  row \"n\"\n    use \"{name}\"\n{settings}{after}  output base color from row \"n\" xyz\n"
    )
}
#[test]
fn catalog_has_exactly_the_agreed_noise_families_and_no_old_aliases() {
    let mut actual: Vec<_> = Kernel::ALL
        .iter()
        .map(|k| k.metadata())
        .filter(|m| m.category == "Noise" || m.category == "Fractal" || m.name == "Warp")
        .map(|m| m.name)
        .collect();
    actual.sort();
    let mut expected = vec![
        "Organic Noise",
        "Organic fBm",
        "Perlin Noise",
        "Simplex Noise",
        "Voronoi",
        "White Noise",
        "Spots",
        "Directional Grain",
        "Gabor Noise",
        "Wave Noise",
        "fBm",
        "Warp",
    ];
    expected.sort();
    assert_eq!(actual, expected);
    for name in [
        "Brians Perlin Noise",
        "Daves Tileable Value Noise",
        "Daves Tileable Value fBM",
        "Adrians Value Noise (2D Noise)",
        "Adrians Value Noise (3D Noise)",
        "Brians Cellular Noise",
        "Brians Value Noise (2D Noise)",
        "Brians Value Noise (3D Noise)",
        "IQs Value Noise (2D Noise)",
        "IQs Value Noise (3D Noise)",
        "Ashimas Perlin Noise",
        "Fabrices Tileable Perlin Noise",
        "Fabrices Tileable Perlin fBM",
        "Gradient Noise (2D Noise)",
        "Gradient Noise (3D Noise)",
        "Domain Warping",
        "Terrain fBm",
        "Turbulence (2D fBm)",
        "Turbulence (3D fBm)",
        "Offset (2D fBm)",
        "Offset (3D fBm)",
    ] {
        assert!(
            Kernel::find(name).is_none(),
            "Removed source remains registered: {name}"
        );
    }
}
#[test]
fn normalized_fbm_samples_both_coordinate_spaces_without_pairing_rules() {
    for name in ["Perlin Noise", "Simplex Noise", "Voronoi"] {
        for dimension in [0, 1] {
            let settings =
                format!("      set \"dimensions\" to {dimension}\n      set \"slice\" to 0.37\n");
            let plain = compile(&source(name, &settings, "")).unwrap();
            // Broadcasting an explicit component keeps all packed fields independent.
            let text = format!(
                "project 1\nmaterial \"m\" called \"Fractal\"\n  row \"source\"\n    use \"{name}\"\n{settings}  row \"fractal\"\n    use \"fBm\"\n      input from row \"source\" yyy\n      set \"layers\" to 3\n  output base color from row \"fractal\" xyz\n"
            );
            let fractal = compile(&text).unwrap();
            for uv in [[0.137, 0.213], [-0.291, 0.173]] {
                let sample = |frequency: f32| {
                    Evaluator::new(&plain)
                        .sample("m", Channel::BaseColor, uv.map(|v| v * frequency))
                        .unwrap()[1]
                };
                let expected = (sample(1.) + 0.5 * sample(2.) + 0.25 * sample(4.)) / 1.75;
                let actual = Evaluator::new(&fractal)
                    .sample("m", Channel::BaseColor, uv)
                    .unwrap();
                assert!(
                    actual.iter().all(|v| (v - expected).abs() < 0.00001),
                    "{name} dimension {dimension}: {actual:?} vs {expected}"
                );
            }
        }
    }
}
#[test]
fn voronoi_exposes_bounded_cell_values_and_two_ordered_distances() {
    let m = Kernel::find("Voronoi").unwrap().metadata();
    assert_eq!(m.default_channel.as_deref(), Some("y"));
    assert_eq!(m.output_channels.len(), 3);
    for dimension in [0, 1] {
        let p = compile(&source(
            "Voronoi",
            &format!("      set \"dimensions\" to {dimension}\n"),
            "",
        ))
        .unwrap();
        for i in 0..31 {
            let v = Evaluator::new(&p)
                .sample("m", Channel::BaseColor, [-0.41 + i as f32 * 0.031, 0.173])
                .unwrap();
            assert!((0.0..=1.0).contains(&v[0]) && v[1] >= 0. && v[2] >= v[1]);
        }
    }
}
#[test]
fn noise_sources_keep_optional_shape_coverage_and_white_noise_is_seeded() {
    for name in [
        "Organic Noise",
        "Organic fBm",
        "Perlin Noise",
        "Simplex Noise",
        "Voronoi",
        "White Noise",
        "Spots",
        "Directional Grain",
        "Gabor Noise",
        "Wave Noise",
    ] {
        let text = format!(
            "project 1\nmaterial \"m\" called \"Mask\"\n  row \"n\"\n    use \"Color\"\n      set \"color\" to 1 1 1 0.25\n    use \"{name}\"\n"
        );
        let p = compile(&text).unwrap();
        assert_eq!(
            Evaluator::new(&p)
                .sample_tile("m", "n", 1, [0.137, 0.213], 0.00001)
                .unwrap()
                .color[3],
            0.25,
            "{name}"
        );
    }
    let a = compile(&source("White Noise", "", "")).unwrap();
    let b = compile(&source("White Noise", "      set \"seed\" to 71\n", "")).unwrap();
    assert_ne!(
        Evaluator::new(&a)
            .sample("m", Channel::BaseColor, [0.137, 0.213])
            .unwrap(),
        Evaluator::new(&b)
            .sample("m", Channel::BaseColor, [0.137, 0.213])
            .unwrap()
    );
}
#[test]
fn every_shipped_example_round_trips_compiles_and_samples_all_connected_outputs() {
    for directory in ["examples"] {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "material") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            let ast = format::parse(&text).unwrap();
            assert_eq!(ast, format::parse(&format::write(&ast)).unwrap());
            let p = compile(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            for m in &ast.materials {
                for output in &m.outputs {
                    let c = Channel::parse(&output.channel).unwrap();
                    metal::compile(&p, &m.id, c).unwrap();
                    let v = Evaluator::new(&p).sample(&m.id, c, [0.137, 0.213]).unwrap();
                    assert!(
                        v.iter().all(|v| v.is_finite()),
                        "{} {}",
                        path.display(),
                        output.channel
                    );
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires a Metal device"]
fn curated_noise_modes_match_metal() {
    use std::{fs, process::Command};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("work/noise-core-metal");
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
    let mut cases = vec![];
    for name in [
        "Organic Noise",
        "Organic fBm",
        "White Noise",
        "Spots",
        "Gabor Noise",
        "Wave Noise",
        "Directional Grain",
    ] {
        cases.push((name.to_string(), source(name, "", "")));
    }
    cases.push((
        "White Noise color".into(),
        source(
            "White Noise",
            "      set \"output\" to 1\n      set \"seed\" to 71\n",
            "",
        ),
    ));
    for name in ["Perlin Noise", "Simplex Noise", "Voronoi"] {
        for dimension in [0, 1] {
            let settings =
                format!("      set \"dimensions\" to {dimension}\n      set \"slice\" to 0.37\n");
            cases.push((
                format!("{name} dimension {dimension}"),
                source(name, &settings, ""),
            ));
            cases.push((
                format!("{name} fBm dimension {dimension}"),
                source(
                    name,
                    &settings,
                    "    use \"fBm\"\n      set \"layers\" to 3\n",
                ),
            ));
        }
    }
    cases.push((
        "Warp self".into(),
        source("Perlin Noise", "", "    use \"Warp\"\n"),
    ));
    cases.push(("Warp vector".into(),source("Perlin Noise","      set \"dimensions\" to 1\n","    use \"Warp\"\n      set \"mode\" to 1\n      set \"displacement\" to 0.3 -0.1 0.2\n")));
    for (label, text) in cases {
        let program = compile(&text).unwrap();
        let shader = metal::compile(&program, "m", Channel::BaseColor).unwrap();
        let pixels = Evaluator::new(&program)
            .render("m", Channel::BaseColor, 23, 19)
            .unwrap();
        let bytes: Vec<_> = pixels
            .into_iter()
            .flat_map(|p| [p[0], p[1], p[2], 1.])
            .flat_map(f32::to_le_bytes)
            .collect();
        fs::write(dir.join("source.metal"), &shader.source).unwrap();
        fs::write(dir.join("params.bin"), shader.parameter_bytes()).unwrap();
        fs::write(dir.join("cpu.bin"), bytes).unwrap();
        let output = Command::new(&harness)
            .arg(dir.join("source.metal"))
            .arg(dir.join("cpu.bin"))
            .args(["23", "19"])
            .arg(dir.join("params.bin"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{label}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        println!(
            "{label}: {}",
            String::from_utf8_lossy(&output.stdout).trim()
        );
    }
}
