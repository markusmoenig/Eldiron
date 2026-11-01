use denrim_materials::{compile, cpu::Evaluator, format, ir::Channel, live::Document, metal};

fn profile_source(mode: usize, width: f32, frequency: f32, antialias: bool) -> String {
    format!(
        "project 1\nmaterial \"m\" called \"Profile\"\n  row \"p\"\n    use \"Tex Coord 2D\"\n    use \"Periodic Profile\"\n      set \"profile\" to {mode}\n      set \"width\" to {width}\n      set \"frequency\" to {frequency}\n      set \"phase\" to 0.13\n      set \"antialias\" to {antialias}\n  output base color from row \"p\" xyz\n"
    )
}

// Independent double-precision point function, integrated numerically below.
fn point(mode: usize, width: f64, phase: f64) -> f64 {
    if width == 0. {
        return 0.;
    }
    let distance = (phase + 0.5).rem_euclid(1.) - 0.5;
    if distance.abs() >= width / 2. {
        return 0.;
    }
    let r = 2. * distance / width;
    match mode {
        0 => 0.5 + 0.5 * (std::f64::consts::PI * r).cos(),
        1 => 1. - r * r,
        2 => 1. - r.abs(),
        _ => 1.,
    }
}

#[test]
fn profiles_match_independent_numerical_integrals() {
    for mode in 0..4 {
        for (width, frequency, aa, u) in [
            (0., 16., 0.01, -0.217),
            (0.07, -16., 0.019, -0.217),
            (0.8, 3., 0.08, 0.073),
            (1., 24., 0.067, 0.231),
            (0.4, 0., 0.01, 0.271),
            (0.6, 1., 0.0001, 0.137),
            (0.0002, 1., 0.0005, -0.1298),
        ] {
            let p = compile(&profile_source(mode, width, frequency, true)).unwrap();
            let value = Evaluator::new(&p)
                .sample_tile("m", "p", 1, [u, 0.19], aa)
                .unwrap()
                .color[0];
            let phase = f64::from(u) * f64::from(frequency) + f64::from(0.13_f32);
            let span = f64::from(aa * frequency.abs());
            let expected = if span < 0.00001 {
                point(mode, f64::from(width), phase)
            } else {
                (0..32768)
                    .map(|i| {
                        point(
                            mode,
                            f64::from(width),
                            phase + ((i as f64 + 0.5) / 32768. - 0.5) * span,
                        )
                    })
                    .sum::<f64>()
                    / 32768.
            };
            assert!(
                (f64::from(value) - expected).abs() < 0.00015,
                "mode {mode}, width {width}, frequency {frequency}: {value} != {expected}"
            );
        }
    }
}

#[test]
fn unresolved_profiles_conserve_area_and_unfiltered_profiles_keep_peaks() {
    for (mode, mean) in [(0, 0.5), (1, 2. / 3.), (2, 0.5), (3, 1.)] {
        let p = compile(&profile_source(mode, 0.37, 4096., true)).unwrap();
        let e = Evaluator::new(&p);
        for u in [-0.27, 0.01, 0.173] {
            let value = e.sample_tile("m", "p", 1, [u, 0.], 0.125).unwrap();
            assert!((value.color[0] - 0.37 * mean).abs() < 0.00001);
        }
        let p = compile(&profile_source(mode, 0.37, -16., false)).unwrap();
        let center = 0.13 / 16.;
        let value = Evaluator::new(&p)
            .sample_tile("m", "p", 1, [center, 0.], 0.125)
            .unwrap();
        assert!((value.color[0] - 1.).abs() < 0.00001);
    }
}

fn comparison_source(mode: usize, options: &str) -> String {
    format!(
        "project 1\nmaterial \"m\" called \"Compare\"\n  row \"p\"\n    use \"Color\"\n      set \"color\" to 0.25 0.5 0.75 0.37\n    use \"Compare/Select\"\n      set \"comparison\" to {mode}\n{options}  output base color from row \"p\" xyz\n"
    )
}

#[test]
fn comparisons_are_componentwise_with_explicit_ties_and_select_values() {
    let cases = [
        [0., 0., 1.],
        [1., 0., 0.],
        [0., 1., 1.],
        [1., 1., 0.],
        [0., 1., 0.],
        [1., 0., 1.],
    ];
    for (mode, expected) in cases.into_iter().enumerate() {
        let p = compile(&comparison_source(mode, "")).unwrap();
        let value = Evaluator::new(&p)
            .sample_tile("m", "p", 1, [0., 0.], 0.001)
            .unwrap();
        assert_eq!(value.color[..3], expected);
        assert_eq!(value.color[3], 0.37);
    }
    let p = compile(&comparison_source(0, "      set \"output\" to 1\n      set \"falseValue\" to -2 10 20\n      set \"trueValue\" to 4 30 60\n      set \"softness\" to 0.5\n")).unwrap();
    let actual = Evaluator::new(&p)
        .sample("m", Channel::BaseColor, [0., 0.])
        .unwrap();
    for (a, b) in actual.iter().zip([-1.0625, 20., 53.75]) {
        assert!((a - b).abs() < 0.00001);
    }
    let p = compile(&comparison_source(
        4,
        "      set \"tolerance\" to 0.1\n      set \"softness\" to 0.2\n",
    ))
    .unwrap();
    let actual = Evaluator::new(&p)
        .sample("m", Channel::BaseColor, [0., 0.])
        .unwrap();
    for (a, b) in actual.iter().zip([0.15625, 1., 0.15625]) {
        assert!((a - b).abs() < 0.00001);
    }
}

#[test]
fn filter_taps_preserve_center_distance_and_alpha_and_edits_stay_buffered() {
    let source = "project 1\nmaterial \"m\" called \"Mask\"\n  row \"p\"\n    use \"Circle\"\n    use \"Periodic Profile\"\n    use \"Compare/Select\"\n";
    let p = compile(source).unwrap();
    let e = Evaluator::new(&p);
    for uv in [[0.12, 0.17], [0.35, 0.01], [0.49, 0.49]] {
        let original = e.sample_tile("m", "p", 0, uv, 0.01).unwrap();
        for tile in [1, 2] {
            let value = e.sample_tile("m", "p", tile, uv, 0.01).unwrap();
            assert_eq!(value.distance, original.distance);
            assert_eq!(value.color[3], original.color[3]);
        }
    }
    let mut d = Document::parse(&profile_source(0, 0.8, 16., true)).unwrap();
    let mut shader = metal::compile(d.program(), "m", Channel::BaseColor).unwrap();
    let old_source = shader.source.clone();
    let old_params = shader.parameter_bytes();
    d.set_constant("m", "p", 1, "width", &[0.4]).unwrap();
    d.set_constant("m", "p", 1, "profile", &[3.]).unwrap();
    shader.refresh(d.program()).unwrap();
    assert_eq!(old_source, shader.source);
    assert_ne!(old_params, shader.parameter_bytes());
}

#[test]
fn weave_round_trips_repeats_and_retains_dye_variation() {
    let source = include_str!("../examples/weave-primitives.material");
    let ast = format::parse(source).unwrap();
    assert_eq!(ast, format::parse(&format::write(&ast)).unwrap());
    assert!(ast.materials[0].rows.iter().all(|r| r.label.len() <= 16));
    let p = compile(source).unwrap();
    let e = Evaluator::new(&p);
    let mut colors = vec![];
    for i in 0..17 {
        let uv = [-0.4 + i as f32 * 0.047, 0.173];
        for channel in [Channel::BaseColor, Channel::Height, Channel::Roughness] {
            let a = e.sample("weave", channel, uv).unwrap();
            for shift in [[1., 0.], [0., 1.]] {
                let b = e
                    .sample("weave", channel, [uv[0] + shift[0], uv[1] + shift[1]])
                    .unwrap();
                for (a, b) in a.iter().zip(b) {
                    assert!((a - b).abs() < 0.0002);
                }
            }
        }
        colors.push(e.sample("weave", Channel::BaseColor, uv).unwrap());
    }
    assert!(colors.windows(2).any(|v| (v[0][0] - v[1][0]).abs() > 0.03));
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires a Metal device"]
fn profiles_comparisons_and_weave_match_metal() {
    use std::{fs, process::Command};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("work/profiles-selection-metal");
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
    for mode in 0..4 {
        cases.push((
            profile_source(mode, 0.17, -128., true),
            "m",
            Channel::BaseColor,
        ));
        cases.push((
            profile_source(mode, 0.8, 3., false),
            "m",
            Channel::BaseColor,
        ));
        let narrow = profile_source(mode, 0.0002, 1., true)
            .replace(
                "use \"Tex Coord 2D\"",
                "use \"Tex Coord 2D\"\n      set \"scale\" to 0.003 0.003",
            )
            .replace("phase\" to 0.13", "phase\" to 0");
        cases.push((narrow, "m", Channel::BaseColor));
    }
    for mode in 0..6 {
        cases.push((comparison_source(mode, ""), "m", Channel::BaseColor));
        cases.push((comparison_source(mode, "      set \"output\" to 1\n      set \"softness\" to 0.2\n      set \"tolerance\" to 0.1\n"), "m", Channel::BaseColor));
    }
    for channel in [Channel::BaseColor, Channel::Height, Channel::Roughness] {
        cases.push((
            include_str!("../examples/weave-primitives.material").into(),
            "weave",
            channel,
        ));
    }
    let masked = "project 1\nmaterial \"m\" called \"Masked\"\n  row \"p\"\n    use \"Circle\"\n    use \"Periodic Profile\"\n    use \"Compare/Select\"\n  output opacity from row \"p\" w\n  output distance from row \"p\" distance\n";
    for channel in [Channel::Opacity, Channel::Distance] {
        cases.push((masked.into(), "m", channel));
    }
    for (text, material, channel) in cases {
        let p = compile(&text).unwrap();
        let shader = metal::compile(&p, material, channel).unwrap();
        let pixels = Evaluator::new(&p)
            .render(material, channel, 31, 29)
            .unwrap();
        let reference: Vec<_> = pixels
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
        fs::write(dir.join("cpu.bin"), reference).unwrap();
        let result = Command::new(&harness)
            .arg(dir.join("shader.metal"))
            .arg(dir.join("cpu.bin"))
            .args(["31", "29"])
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
