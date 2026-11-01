use denrim_materials::{compile, cpu::Evaluator, format, ir::Channel, live::Document, metal};

fn source(name: &str, settings: &str) -> String {
    format!(
        "project 1\nmaterial \"m\" called \"Source\"\n  row \"r\"\n    use \"{name}\"\n{settings}  output base color from row \"r\" xyz\n"
    )
}

#[test]
fn cell_data_is_stable_per_element_and_periodic_across_negative_uvs() {
    for mode in [0, 1, 2, 3, 5] {
        let p = compile(&source("Cell Data", &format!("      set \"cells\" to 12 20\n      set \"seed\" to 37\n      set \"output\" to {mode}\n"))).unwrap();
        let e = Evaluator::new(&p);
        for uv in [[-0.413, -0.271], [0.023, 0.173], [1.127, -1.219]] {
            let a = e.sample("m", Channel::BaseColor, uv).unwrap();
            for delta in [[1., 0.], [0., 1.]] {
                let b = e
                    .sample(
                        "m",
                        Channel::BaseColor,
                        [uv[0] + delta[0], uv[1] + delta[1]],
                    )
                    .unwrap();
                assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 0.00001));
            }
            let shifted = match mode {
                1 => [uv[0] + 0.31, uv[1]],
                2 => [uv[0], uv[1] + 0.31],
                _ => uv,
            };
            if mode == 1 || mode == 2 {
                assert_eq!(a, e.sample("m", Channel::BaseColor, shifted).unwrap());
            }
        }
    }
    let p = compile(&source("Cell Data", "      set \"cells\" to 12 20\n      set \"offset\" to 0.17 0.5\n      set \"output\" to 4\n")).unwrap();
    let local = compile(&source("Cell Data", "      set \"cells\" to 12 20\n      set \"offset\" to 0.17 0.5\n      set \"output\" to 3\n")).unwrap();
    for uv in [[-0.413, -0.271], [0.023, 0.173], [1.127, -1.219]] {
        let center = Evaluator::new(&p)
            .sample("m", Channel::BaseColor, uv)
            .unwrap();
        let local = Evaluator::new(&local)
            .sample("m", Channel::BaseColor, uv)
            .unwrap();
        for (i, count) in [12., 20.].into_iter().enumerate() {
            assert!((center[i] + (local[i] - 0.5) / count - uv[i]).abs() < 0.00001);
        }
    }
}

#[test]
fn grain_has_direction_seed_variation_bounds_and_minification() {
    let text = source(
        "Directional Grain",
        "      set \"scale\" to 3 64\n      set \"bend\" to 0\n      set \"output\" to 3\n",
    );
    let p = compile(&text).unwrap();
    let e = Evaluator::new(&p);
    let mut along = 0.;
    let mut across = 0.;
    for i in 0..61 {
        let uv = [-0.43 + i as f32 * 0.013, 0.137 + i as f32 * 0.0023];
        let a = e.sample_tile("m", "r", 0, uv, 0.00001).unwrap().color;
        let x = e
            .sample_tile("m", "r", 0, [uv[0] + 0.001, uv[1]], 0.00001)
            .unwrap()
            .color;
        let y = e
            .sample_tile("m", "r", 0, [uv[0], uv[1] + 0.001], 0.00001)
            .unwrap()
            .color;
        along += (x[0] - a[0]).abs();
        across += (y[0] - a[0]).abs();
        assert!(a[..3].iter().all(|v| (0.0..=1.0).contains(v)));
    }
    assert!(
        across > along * 4.,
        "Grain must vary primarily across the fibers"
    );
    let different = compile(&text.replace(
        "set \"bend\" to 0",
        "set \"bend\" to 0\n      set \"seed\" to 19",
    ))
    .unwrap();
    assert_ne!(
        e.sample("m", Channel::BaseColor, [0.17, 0.23]).unwrap(),
        Evaluator::new(&different)
            .sample("m", Channel::BaseColor, [0.17, 0.23])
            .unwrap()
    );
    let fine = compile(&source(
        "Directional Grain",
        "      set \"scale\" to 256 1024\n      set \"output\" to 3\n",
    ))
    .unwrap();
    for uv in [[-0.31, 0.17], [0.23, 0.37]] {
        let v = Evaluator::new(&fine)
            .sample_tile("m", "r", 0, uv, 0.125)
            .unwrap()
            .color;
        assert_eq!(v[0], 0.5);
        assert_eq!(v[1], 0.);
        assert!((v[2] - 0.94).abs() < 0.000001);
    }
}

#[test]
fn groove_masks_stay_narrow_and_minify_to_coverage() {
    let base = source(
        "Directional Grain",
        "      set \"scale\" to 3 32\n      set \"bend\" to 0\n      set \"threshold\" to 1\n      set \"softness\" to 0.001\n      set \"output\" to 3\n",
    );
    let p = compile(&base).unwrap();
    let e = Evaluator::new(&p);
    let mean = (0..2048)
        .map(|i| {
            e.sample_tile(
                "m",
                "r",
                0,
                [0.137, (i as f32 + 0.5) / 2048. - 0.5],
                0.00001,
            )
            .unwrap()
            .color[1]
        })
        .sum::<f32>()
        / 2048.;
    assert!(
        (0.12..0.24).contains(&mean),
        "Grooves must occupy narrow fibers, not broad patches: {mean}"
    );
    let value = e
        .sample_tile("m", "r", 0, [0.137, 0.217], 0.5)
        .unwrap()
        .color;
    assert!((value[1] - 0.18).abs() < 0.00001);
    let empty = compile(&base.replace(
        "set \"output\" to 3",
        "set \"output\" to 3\n      set \"grooveWidth\" to 0",
    ))
    .unwrap();
    assert_eq!(
        Evaluator::new(&empty)
            .sample("m", Channel::BaseColor, [0.137, 0.217])
            .unwrap()[1],
        0.
    );
}

#[test]
fn new_sources_preserve_masks_and_refresh_parameters_without_recompilation() {
    for name in ["Cell Data", "Directional Grain", "Height to Normal"] {
        let text = format!(
            "project 1\nmaterial \"m\" called \"Masked\"\n  row \"r\"\n    use \"Circle\"\n    use \"{name}\"\n  output base color from row \"r\" xyz\n"
        );
        let mut document = Document::parse(&text).unwrap();
        let e = Evaluator::new(document.program());
        for uv in [[0.12, 0.17], [0.35, 0.01], [0.49, 0.49]] {
            let a = e.sample_tile("m", "r", 0, uv, 0.01).unwrap();
            let b = e.sample_tile("m", "r", 1, uv, 0.01).unwrap();
            assert_eq!(a.distance, b.distance);
            assert_eq!(a.color[3], b.color[3]);
        }
        let mut shader = metal::compile(document.program(), "m", Channel::BaseColor).unwrap();
        let old = shader.source.clone();
        let params = shader.parameter_bytes();
        let (parameter, value) = if name == "Height to Normal" {
            ("strength", 2.)
        } else {
            ("seed", 123.)
        };
        document
            .set_constant("m", "r", 1, parameter, &[value])
            .unwrap();
        shader.refresh(document.program()).unwrap();
        assert_eq!(old, shader.source);
        assert_ne!(params, shader.parameter_bytes());
    }
}

#[test]
fn height_normals_have_resolution_independent_slopes_and_y_convention() {
    let x = "project 1\nmaterial \"m\" called \"Ramp\"\n  row \"r\"\n    use \"Tex Coord 2D\"\n    use \"Height to Normal\"\n  output normal from row \"r\" xyz\n";
    let p = compile(x).unwrap();
    let expected = [0.1464466, 0.5, 0.8535534];
    for aa in [0.001, 0.01] {
        for uv in [[0., 0.], [-0.21, 0.13]] {
            let value = Evaluator::new(&p).sample_tile("m", "r", 1, uv, aa).unwrap();
            assert!(
                value.color[..3]
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| (a - b).abs() < 0.00001)
            );
        }
    }
    let y = "project 1\nmaterial \"m\" called \"Ramp\"\n  row \"uv\"\n    use \"Tex Coord 2D\"\n  row \"r\"\n    use \"Height to Normal\"\n      input from row \"uv\" yyy\n      set \"flipY\" to true\n  output normal from row \"r\" xyz\n";
    let p = compile(y).unwrap();
    let value = Evaluator::new(&p)
        .sample("m", Channel::Normal, [0.17, -0.23])
        .unwrap();
    assert!(
        value
            .iter()
            .zip([0.5, 0.8535534, 0.8535534])
            .all(|(a, b)| (a - b).abs() < 0.00001)
    );
}

#[test]
fn wicker_has_paired_ribs_alternating_broad_strips_and_a_clean_repeat() {
    let text = include_str!("../examples/wicker.material");
    let ast = format::parse(text).unwrap();
    assert_eq!(ast, format::parse(&format::write(&ast)).unwrap());
    let p = compile(text).unwrap();
    let e = Evaluator::new(&p);
    let tile = |row: &str, index, uv| {
        e.sample_tile("weave", row, index, uv, 0.00001)
            .unwrap()
            .color[0]
    };
    for x in [-0.145, 0.145] {
        assert!(tile("rib-profile", 1, [x / 12., 0.5 / 20.]) > 0.9);
        for j in -2_i32..=2 {
            let uv = [x / 12., j as f32 / 20.];
            let strip = tile("strip-body", 0, uv);
            let rib = tile("rib-height", 0, uv);
            assert!(
                if j % 2 == 0 { strip > rib } else { rib > strip },
                "Crossing {x} {j}: strip {strip}, rib {rib}"
            );
        }
    }
    for (uv, expected) in [
        ([0.5 / 12., 0.5 / 20.], 0.),
        ([0.5 / 12., 0.], 1.),
        ([0.145 / 12., 0.5 / 20.], 1.),
    ] {
        assert_eq!(
            e.sample("weave", Channel::Opacity, uv).unwrap()[0],
            expected
        );
    }
    assert_eq!(tile("rib-profile", 1, [0., 0.5 / 20.]), 0.);
    assert_eq!(tile("rib-profile", 1, [0.5 / 12., 0.5 / 20.]), 0.);
    for i in 0..13 {
        let uv = [-0.413 + i as f32 * 0.067, 0.173];
        for channel in [
            Channel::BaseColor,
            Channel::Height,
            Channel::Roughness,
            Channel::Normal,
            Channel::Opacity,
        ] {
            let a = e.sample("weave", channel, uv).unwrap();
            for shift in [[1., 0.], [0., 1.]] {
                let b = e
                    .sample("weave", channel, [uv[0] + shift[0], uv[1] + shift[1]])
                    .unwrap();
                assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 0.001));
            }
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires a Metal device"]
fn cell_grain_and_wicker_match_metal() {
    use std::{fs, process::Command};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("work/wicker-metal");
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
    cases.push(("project 1\nmaterial \"m\" called \"Ramp\"\n  row \"r\"\n    use \"Tex Coord 2D\"\n    use \"Height to Normal\"\n  output normal from row \"r\" xyz\n".into(),"m",Channel::Normal));
    for mode in 0..6 {
        cases.push((source("Cell Data",&format!("      set \"output\" to {mode}\n      set \"cells\" to 12 20\n      set \"seed\" to -137\n")),"m",Channel::BaseColor));
    }
    for direction in [0, 33, 90] {
        for output in 0..4 {
            cases.push((source("Directional Grain",&format!("      set \"direction\" to {direction}\n      set \"output\" to {output}\n      set \"scale\" to 3 9\n      set \"seed\" to 123\n")),"m",Channel::BaseColor));
        }
    }
    for channel in [
        Channel::BaseColor,
        Channel::Height,
        Channel::Roughness,
        Channel::Normal,
        Channel::Opacity,
    ] {
        cases.push((
            include_str!("../examples/wicker.material").into(),
            "weave",
            channel,
        ));
    }
    for (text, material, channel) in cases {
        let p = compile(&text).unwrap();
        let shader = metal::compile(&p, material, channel).unwrap();
        let pixels = Evaluator::new(&p)
            .render(material, channel, 43, 37)
            .unwrap();
        let bytes: Vec<_> = pixels
            .into_iter()
            .flat_map(|v| {
                if v.len() == 1 {
                    [v[0], v[0], v[0], 1.]
                } else {
                    [v[0], v[1], v[2], 1.]
                }
            })
            .flat_map(f32::to_le_bytes)
            .collect();
        fs::write(dir.join("shader.metal"), &shader.source).unwrap();
        fs::write(dir.join("params.bin"), shader.parameter_bytes()).unwrap();
        fs::write(dir.join("cpu.bin"), bytes).unwrap();
        let result = Command::new(&harness)
            .arg(dir.join("shader.metal"))
            .arg(dir.join("cpu.bin"))
            .args(["43", "37"])
            .arg(dir.join("params.bin"))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{material} {channel:?}: {}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        println!(
            "{material} {channel:?}: {}",
            String::from_utf8_lossy(&result.stdout).trim()
        );
    }
}
