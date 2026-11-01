use denrim_materials::{
    compile,
    cpu::{Evaluator, PreviewMode},
    ir::Channel,
    metadata::Kernel,
    metal,
};

fn fixture(source: &str, settings: &str, tail: &str) -> String {
    format!(
        "project 1\nmaterial \"m\" called \"M\"\n  row \"r\"\n    use \"{source}\"\n{settings}{tail}  output base color from row \"r\" xyz\n"
    )
}
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 0.00003, "{a} != {b}");
}
const GRID: &str = "    use \"Grid\"\n      set \"cells\" to 4 4\n      set \"gap\" to 0.2 0.2\n";
const NESTED: &str = "project 1\nmaterial \"m\" called \"M\"\n  row \"parity\"\n    use \"Cell Data\"\n      set \"scope\" to 1\n      set \"output\" to 7\n  row \"brick\"\n    use \"Rounded Rectangle\"\n      set \"halfSize\" to 0.48 0.48\n      set \"radius\" to 0.02\n    use \"Color\"\n      set \"color\" to 0.8 0.2 0.1 0.5\n  row \"pair\"\n    use \"Grid\"\n      input from row \"brick\" xyz\n      set \"cells\" to 1 2\n      set \"gap\" to 0 0.1\n    use \"Transform\"\n      set \"rot\" from row \"parity\" x mapped 0 to 90\n  row \"r\"\n    use \"Grid\"\n      input from row \"pair\" xyz\n      set \"cells\" to 2 2\n      set \"gap\" to 0 0\n  output base color from row \"r\" xyz\n";

#[test]
fn grid_repeats_rgba_with_filtered_gaps_and_clears_metric_distance() {
    let p = compile(&fixture(
        "Color",
        "      set \"color\" to 0.8 0.2 0.1 0.25\n",
        GRID,
    ))
    .unwrap();
    let e = Evaluator::new(&p);
    let get = |uv| e.sample_tile("m", "r", 1, uv, 0.00001).unwrap();
    let v = get([-0.375, -0.375]);
    for (a, b) in v.color.into_iter().zip([0.8, 0.2, 0.1, 0.25]) {
        close(a, b)
    }
    assert_eq!(get([-0.25, -0.375]).color, [0.; 4]);
    let edge = e
        .sample_tile("m", "r", 1, [-0.275, -0.375], 0.002)
        .unwrap()
        .color;
    close(edge[3], 0.125);
    close(edge[0], 0.8);
    let p = compile(&fixture("Circle", "", GRID)).unwrap();
    assert_eq!(
        Evaluator::new(&p)
            .sample_tile("m", "r", 1, [0.1, 0.1], 0.001)
            .unwrap()
            .distance,
        None
    );
}
#[test]
fn touching_cells_have_no_alpha_seams_and_stagger_is_periodic() {
    let tail = "    use \"Grid\"\n      set \"cells\" to 4 4\n      set \"gap\" to 0 0\n      set \"rowOffset\" to 0.5\n";
    let p = compile(&fixture(
        "Color",
        "      set \"color\" to 0.8 0.2 0.1 0.25\n",
        tail,
    ))
    .unwrap();
    let e = Evaluator::new(&p);
    for uv in [[0., 0.], [-0.125, 0.], [0.25, 0.125], [0.173, 0.271]] {
        let a = e.sample_tile("m", "r", 1, uv, 0.001).unwrap().color;
        for (a, b) in a.into_iter().zip([0.8, 0.2, 0.1, 0.25]) {
            close(a, b)
        }
        let b = e
            .sample_tile("m", "r", 1, [uv[0] + 1., uv[1] + 1.], 0.001)
            .unwrap()
            .color;
        for (a, b) in a.into_iter().zip(b) {
            close(a, b)
        }
    }
}
#[test]
fn nested_grid_rotation_uses_parent_parity_and_keeps_alpha() {
    let p = compile(NESTED).unwrap();
    let e = Evaluator::new(&p);
    let alpha = |uv| e.sample_tile("m", "r", 0, uv, 0.00001).unwrap().color[3];
    // Even parent: horizontal separator. Odd parent: vertical separator.
    close(alpha([-0.15, -0.25]), 0.);
    close(alpha([-0.25, -0.15]), 0.5);
    close(alpha([0.15, -0.25]), 0.5);
    close(alpha([0.25, -0.15]), 0.);
    for uv in [[-0.35, -0.37], [0.13, -0.17], [0.31, 0.35]] {
        let a = e.sample_tile("m", "r", 0, uv, 0.00001).unwrap().color;
        for delta in [[1., 0.], [0., 1.]] {
            let b = e
                .sample_tile("m", "r", 0, [uv[0] + delta[0], uv[1] + delta[1]], 0.00001)
                .unwrap()
                .color;
            for (a, b) in a.into_iter().zip(b) {
                close(a, b)
            }
        }
    }
}
#[test]
fn instance_and_group_keys_are_distinct_and_stable_through_parameter_rows() {
    for output in [0, 8] {
        let text = NESTED.replace(
            "    use \"Color\"\n      set \"color\" to 0.8 0.2 0.1 0.5",
            &format!(
                "    use \"Cell Data\"\n      set \"scope\" to 1\n      set \"output\" to {output}"
            ),
        );
        let p = compile(&text).unwrap();
        let e = Evaluator::new(&p);
        let get = |uv| e.sample("m", Channel::BaseColor, uv).unwrap()[0];
        let a = get([-0.25, -0.375]);
        let b = get([-0.25, -0.125]);
        let c = get([0.125, -0.25]);
        if output == 0 {
            assert!((a - b).abs() > 0.0001)
        } else {
            close(a, b)
        }
        assert!((a - c).abs() > 0.0001);
    }
}
#[test]
fn shape_distance_broadcast_and_general_profiles_are_numeric() {
    let text = "project 1\nmaterial \"m\" called \"M\"\n  row \"s\"\n    use \"Circle\"\n      set \"radius\" to 0.3\n  row \"r\"\n    use \"Distance Profile\"\n      input from row \"s\" distance\n      set \"width\" to 0.1\n      set \"profile\" to 0\n  output base color from row \"r\" xyz\n";
    let p = compile(text).unwrap();
    let v = Evaluator::new(&p)
        .sample("m", Channel::BaseColor, [0.25, 0.])
        .unwrap();
    for v in v {
        close(v, 0.5)
    }
    for (source, settings, expected) in [
        (
            "Levels",
            "      set \"inputLow\" to 2\n      set \"inputHigh\" to 4\n      set \"outputLow\" to -1\n      set \"outputHigh\" to 1\n",
            [0., 0., 0.],
        ),
        ("Curve", "", [1., 1., 1.]),
        (
            "Vector Math",
            "      set \"operation\" to 2\n      set \"other\" to 1 2 3\n",
            [18., 18., 18.],
        ),
        ("Divide", "      set \"divisor\" to 3 3 3\n", [1., 1., 1.]),
    ] {
        let text = fixture(
            "Value",
            "      set \"globalValue\" to 3\n",
            &format!("    use \"{source}\"\n{settings}"),
        );
        let p = compile(&text).unwrap();
        let v = Evaluator::new(&p)
            .sample("m", Channel::BaseColor, [0., 0.])
            .unwrap();
        for (a, b) in v.into_iter().zip(expected) {
            close(a, b)
        }
    }
    let p = compile(&fixture(
        "Color",
        "      set \"color\" to 0.5 0.5 1 1\n",
        "    use \"Normal Combine\"\n",
    ))
    .unwrap();
    let v = Evaluator::new(&p)
        .sample("m", Channel::BaseColor, [0., 0.])
        .unwrap();
    for (a, b) in v.into_iter().zip([0.5, 0.5, 1.]) {
        close(a, b)
    }
}
#[test]
fn voronoi_metrics_and_exact_border_have_defined_geometry() {
    for dimensions in [0, 1] {
        for (metric, expected) in [
            (
                0,
                if dimensions == 0 {
                    (0.37_f32 * 0.37 + 0.33 * 0.33).sqrt()
                } else {
                    (0.37_f32 * 0.37 + 0.33 * 0.33 + 0.27 * 0.27).sqrt()
                },
            ),
            (1, if dimensions == 0 { 0.7 } else { 0.97 }),
            (2, 0.37),
        ] {
            let settings = format!(
                "      set \"jitter\" to 0\n      set \"dimensions\" to {dimensions}\n      set \"slice\" to 0.023\n      set \"metric\" to {metric}\n"
            );
            let p = compile(&fixture("Voronoi", &settings, "")).unwrap();
            let v = Evaluator::new(&p)
                .sample("m", Channel::BaseColor, [0.013, 0.017])
                .unwrap();
            close(v[1], expected);
            let p = compile(&fixture(
                "Voronoi",
                &(settings + "      set \"output\" to 1\n"),
                "",
            ))
            .unwrap();
            let v = Evaluator::new(&p)
                .sample("m", Channel::BaseColor, [0.013, 0.017])
                .unwrap();
            for v in v {
                close(v, 0.13)
            }
        }
    }
}
#[test]
fn retired_duplicates_have_no_catalog_aliases() {
    for name in [
        "Split",
        "Utility Node",
        "Parameter",
        "Add (Mixer)",
        "Multiply (Mixer)",
        "Color Blend",
        "Smooth Mix",
        "Step",
        "Mask (Operator)",
        "Rotate 2D (Mixer)",
        "Manhattan Voronoi Distance",
        "Normal Based Mix",
        "Displace",
        "Tiler",
    ] {
        assert!(Kernel::find(name).is_none(), "{name}")
    }
    for name in [
        "Mix",
        "Add",
        "Multiply",
        "Power",
        "Grid",
        "Compose Channels",
        "Levels",
        "Curve",
        "Distance Profile",
        "Normal Combine",
        "Vector Math",
        "Color Ramp",
    ] {
        assert!(Kernel::find(name).is_some(), "{name}")
    }
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires a Metal device"]
fn new_core_modes_and_nested_rgba_match_metal() {
    use std::{fs, process::Command};
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("work/grid-core-metal");
    fs::create_dir_all(&dir).unwrap();
    let harness = dir.join("compare");
    let output = Command::new("xcrun")
        .args(["swiftc", "-module-cache-path"])
        .arg(dir.join("cache"))
        .arg("tests/metal_compare.swift")
        .arg("-o")
        .arg(&harness)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut cases = vec![
        (NESTED.to_owned(), "r", 0),
        (
            fixture("Color", "      set \"color\" to 0.8 0.2 0.1 0.25\n", GRID),
            "r",
            1,
        ),
    ];
    for operation in 0..6 {
        cases.push((fixture("Value","      set \"localValue\" to 0.1 0.3 0.6\n",&format!("    use \"Vector Math\"\n      set \"operation\" to {operation}\n      set \"angle\" to 37\n")),"r",1));
    }
    for name in [
        "Curve",
        "Levels",
        "Normal Combine",
        "Color Ramp",
        "Directional Blur",
    ] {
        cases.push((
            fixture("Circle", "", &format!("    use \"{name}\"\n")),
            "r",
            1,
        ));
    }
    for dimension in [0, 1] {
        for metric in [0, 1, 2] {
            cases.push((fixture("Voronoi",&format!("      set \"dimensions\" to {dimension}\n      set \"metric\" to {metric}\n      set \"output\" to 1\n"),""),"r",0));
        }
    }
    let scoped_border=NESTED.replace("  row \"brick\"", "  row \"seed\"\n    use \"Cell Data\"\n      set \"scope\" to 1\n  row \"brick\"").replace("      set \"color\" to 0.8 0.2 0.1 0.5", "      set \"color\" from row \"seed\" xxxx\n    use \"Transform\"\n      set \"localScale\" to 0.9 0.8\n    use \"Border\"\n      set \"width\" to 0.03");
    let scoped_border = scoped_border.replace("    use \"Grid\"\n      input from row \"brick\" xyz\n      set \"cells\" to 1 2\n      set \"gap\" to 0 0.1\n", "    use \"Transform\"\n      input from row \"brick\" xyz\n");
    let scoped_border = scoped_border.replace(
        "input from row \"pair\" xyz",
        "input from row \"brick\" xyz",
    );
    cases.push((scoped_border, "r", 0));
    cases.push((
        include_str!("../examples/basketweave.material").to_owned(),
        "pattern",
        0,
    ));
    for blend in 0..3 {
        cases.push((fixture("Rounded Rectangle","",&format!("    use \"Scatter\"\n      set \"blend\" to {blend}\n      set \"amount\" to 2\n      set \"size\" to 1.5 1.2\n")),"r",1));
    }
    cases.push((
        fixture(
            "Color",
            "      set \"color\" to 0.8 0.2 0.1 0.25\n",
            "    use \"Partition\"\n      set \"gap\" to 0\n",
        ),
        "r",
        1,
    ));
    cases.push((
        fixture(
            "Rounded Rectangle",
            "",
            "    use \"Grid\"\n      set \"layout\" to 1\n      set \"rowOffset\" to 0.5\n",
        ),
        "r",
        1,
    ));
    for mapping in 0..5 {
        cases.push((
            fixture(
                "Coordinates",
                &format!(
                    "      set \"mapping\" to {mapping}\n      set \"rotation\" to 13 29 47\n"
                ),
                "",
            ),
            "r",
            0,
        ));
    }
    for dimensions in 0..2 {
        for output in 2..5 {
            cases.push((fixture("Voronoi",&format!("      set \"dimensions\" to {dimensions}\n      set \"output\" to {output}\n"),""),"r",0));
        }
    }
    let named = "project 1\nmaterial \"m\" called \"Fields\"\n  row \"cells\"\n    use \"Voronoi\"\n  row \"r\"\n    use \"Levels\"\n      input from row \"cells\" border.xxx\n      set \"inputHigh\" to 0.1\n  output base color from row \"r\" xyz\n";
    cases.push((named.into(), "r", 0));
    cases.push((fixture("Cell Data","      set \"scope\" to 1\n      set \"output\" to 12\n","    use \"Transform\"\n      set \"globalScale\" to 0.5\n    use \"Grid\"\n      set \"cells\" to 4 8\n"),"r",2));
    for (i, (text, row, tile)) in cases.into_iter().enumerate() {
        let p = compile(&text).unwrap();
        let material = if row == "pattern" { "basketweave" } else { "m" };
        let shader =
            metal::compile_tile_preview(&p, material, row, tile, PreviewMode::Image).unwrap();
        let pixels = Evaluator::new(&p)
            .render_tile(material, row, tile, PreviewMode::Image, 11, 9)
            .unwrap();
        fs::write(dir.join("source.metal"), &shader.source).unwrap();
        fs::write(dir.join("params.bin"), shader.parameter_bytes()).unwrap();
        fs::write(
            dir.join("cpu.bin"),
            pixels
                .into_iter()
                .flatten()
                .flat_map(f32::to_le_bytes)
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let result = Command::new(&harness)
            .arg(dir.join("source.metal"))
            .arg(dir.join("cpu.bin"))
            .args(["11", "9"])
            .arg(dir.join("params.bin"))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "case {i}: {}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        println!(
            "case {i}: {}",
            String::from_utf8_lossy(&result.stdout).trim()
        );
    }
}
