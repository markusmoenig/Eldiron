use denrim_materials::{compile, cpu::Evaluator, ir::Channel, metadata::Kernel, metal};
fn polygon(settings: &str) -> String {
    format!(
        "project 1\nmaterial \"m\" called \"Polygon\"\n  row \"p\"\n    use \"Polygon\"\n{settings}  output height from row \"p\" distance\n"
    )
}
fn field(quadratic: bool, after: &str) -> String {
    let control = if quadratic {
        "      set \"other\" from row \"uv\" xyz\n"
    } else {
        "      set \"other\" to 2 -3 0\n"
    };
    format!(
        "project 1\nmaterial \"m\" called \"Derivatives\"\n  row \"uv\"\n    use \"Coordinates\"\n  row \"height\"\n    use \"Vector Math\"\n      input from row \"uv\" xyz\n      set \"operation\" to 2\n{control}  row \"result\"\n{after}  output base color from row \"result\" xyz\n"
    )
}
fn packed() -> &'static str {
    "project 1\nmaterial \"m\" called \"Packed Shape\"\n  row \"p\"\n    use \"Polygon\"\n      set \"sides\" to 4\n      set \"radius\" to 0.4\n      set \"rotation\" to 45\n  row \"pack\"\n    use \"Compose Channels\"\n      set \"x\" from row \"p\" distance mapped 0.5 to 1\n      set \"y\" from row \"p\" x\n      set \"z\" from row \"p\" x mapped 0.2 to 0.8\n      set \"alpha\" from row \"p\" w\n  output base color from row \"pack\" xyz\n"
}
#[test]
fn packed_parameters_share_a_sample_without_losing_selection_or_mapping() {
    let p = compile(packed()).unwrap();
    let shape = p.tile("m", "p", 0).unwrap();
    let pack = p.tile("m", "pack", 0).unwrap();
    assert_eq!(p.nodes[pack.0].dependencies(), vec![shape]);
    assert_eq!(
        p.costs[pack.0].expanded_calls,
        p.costs[shape.0].expanded_calls + 1
    );
    let shader = metal::compile(&p, "m", Channel::BaseColor).unwrap();
    assert_eq!(
        shader
            .source
            .matches(&format!("MzSample mz_parameter_{} =", shape.0))
            .count(),
        1
    );
    let e = Evaluator::new(&p);
    let half = 0.4_f32 / 2_f32.sqrt();
    for (uv, expected) in [
        ([0., 0.], [0.5 - half * 0.5, 1., 0.8, 1.]),
        ([0.6, 0.1], [0.5 + (0.6 - half) * 0.5, 1., 0.8, 0.]),
    ] {
        let sample = e.sample_tile("m", "pack", 0, uv, 0.001).unwrap();
        for (a, b) in sample.color.into_iter().zip(expected) {
            assert!((a - b).abs() < 0.00001);
        }
    }
}
#[test]
fn polygons_have_exact_signed_edge_distance_and_instance_uniform_geometry() {
    let text = polygon(
        "      set \"sides\" to 4\n      set \"radius\" to 0.4\n      set \"rotation\" to 45\n",
    );
    let p = compile(&text).unwrap();
    let e = Evaluator::new(&p);
    let half = 0.4_f32 / 2_f32.sqrt();
    for (uv, expected) in [
        ([0., 0.], -half),
        ([0.6, 0.1], 0.6 - half),
        ([0.6, 0.6], 2_f32.sqrt() * (0.6 - half)),
    ] {
        let d = e.sample("m", Channel::Height, uv).unwrap()[0];
        assert!((d - expected).abs() < 0.00001, "{d} vs {expected}");
    }
    let irregular = polygon(
        "      set \"sides\" to 9\n      set \"irregularity\" to 0.6\n      set \"seed\" to 73\n",
    );
    let p = compile(&irregular).unwrap();
    let e = Evaluator::new(&p);
    for i in 0..40 {
        let x = i as f32 / 40. - 0.5;
        let a = e.sample("m", Channel::Height, [x, 0.13]).unwrap()[0];
        let b = e.sample("m", Channel::Height, [x + 0.003, 0.134]).unwrap()[0];
        assert!((a - b).abs() <= 0.00501);
    }
    let instance = "project 1\nmaterial \"m\" called \"Instance\"\n  row \"key\"\n    use \"Cell Data\"\n      set \"scope\" to 1\n  row \"p\"\n    use \"Polygon\"\n      set \"seed\" from row \"key\" x mapped 0 to 100\n      set \"irregularity\" to 0.4\n  output height from row \"p\" distance\n";
    assert!(compile(instance).is_ok());
    assert!(compile(&instance.replace("set \"scope\" to 1", "set \"scope\" to 0")).is_err());
    assert!(Kernel::find("Polygon").unwrap().is_shape());
}
#[test]
fn height_gradient_measures_slopes_and_curvature_in_uv_units() {
    for radius in [0.01, 0.02] {
        let text = field(
            false,
            &format!(
                "    use \"Height Gradient\"\n      input from row \"height\" xxx\n      set \"radius\" to {radius}\n      set \"units\" to 1\n"
            ),
        );
        let p = compile(&text).unwrap();
        let e = Evaluator::new(&p);
        let v = e.sample("m", Channel::BaseColor, [0.11, 0.17]).unwrap();
        assert!(
            (v[0] - 2.).abs() < 0.0001 && (v[1] + 3.).abs() < 0.0001 && v[2].abs() < 0.002,
            "{v:?}"
        );
    }
    let text = field(
        true,
        "    use \"Height Gradient\"\n      input from row \"height\" xxx\n      set \"radius\" to 0.02\n      set \"units\" to 1\n",
    );
    let p = compile(&text).unwrap();
    let v = Evaluator::new(&p)
        .sample("m", Channel::BaseColor, [0.11, 0.17])
        .unwrap();
    for (a, b) in v.iter().zip([0.22, 0.34, 4.]) {
        assert!((a - b).abs() < 0.003, "{v:?}");
    }
    let named = text.replace("row \"result\" xyz", "row \"result\" curvature.xxx");
    let p = compile(&named).unwrap();
    assert!(
        (Evaluator::new(&p)
            .sample("m", Channel::BaseColor, [0.11, 0.17])
            .unwrap()[0]
            - 4.)
            .abs()
            < 0.003
    );
}
#[test]
fn vector_filters_have_bounded_extrema_symmetric_means_and_no_alpha_fringe() {
    let base = 2. * 0.11 - 3. * 0.17;
    for (mode, vector, centered, expected) in [
        (0, "-1 0 0", 0, base - 0.2),
        (1, "1 0 0", 0, base + 0.2),
        (2, "1 0 0", 0, base + 0.1),
        (2, "1 0 0", 1, base),
        (1, "0 0 0", 0, base),
    ] {
        let text = field(
            false,
            &format!(
                "    use \"Vector Filter\"\n      input from row \"height\" xyz\n      set \"radius\" to 0.1\n      set \"units\" to 1\n      set \"samples\" to 4\n      set \"mode\" to {mode}\n      set \"vector\" to {vector}\n      set \"centered\" to {centered}\n"
            ),
        );
        let p = compile(&text).unwrap();
        let v = Evaluator::new(&p)
            .sample("m", Channel::BaseColor, [0.11, 0.17])
            .unwrap();
        assert!((v[0] - expected).abs() < 0.00001, "{v:?} vs {expected}");
    }
    let text = "project 1\nmaterial \"m\" called \"Alpha\"\n  row \"r\"\n    use \"Circle\"\n      set \"radius\" to 0.3\n    use \"Color\"\n      set \"color\" to 0.8 0.2 0.1 0.25\n    use \"Vector Filter\"\n      set \"radius\" to 0.1\n      set \"units\" to 1\n      set \"mode\" to 2\n      set \"samples\" to 8\n  output base color from row \"r\" xyz\n";
    let p = compile(text).unwrap();
    let v = Evaluator::new(&p)
        .sample_tile("m", "r", 2, [0.28, 0.], 0.002)
        .unwrap();
    assert!(v.color[3] > 0. && v.color[3] < 0.25);
    for (a, b) in v.color[..3].iter().zip([0.8, 0.2, 0.1]) {
        assert!((a - b).abs() < 0.00001);
    }
}
#[test]
fn rock_recipe_compiles_through_chained_cached_filters_and_normals() {
    let text = include_str!("../examples/fractured-rock.material");
    let p = compile(text).unwrap();
    for name in ["Rocks", "Disc", "Tile", "Random"] {
        assert!(!text.contains(&format!("use \"{name}\"")));
    }
    let color = metal::compile(&p, "fractured-rock", Channel::BaseColor).unwrap();
    let normal = metal::compile(&p, "fractured-rock", Channel::Normal).unwrap();
    assert!(!color.passes.is_empty());
    assert!(normal.pixel_padding > color.pixel_padding);
    let preview = metal::compile_tile_preview(
        &p,
        "fractured-rock",
        "normal",
        0,
        denrim_materials::cpu::PreviewMode::Image,
    )
    .unwrap();
    assert!(!preview.passes.is_empty());
    let e = Evaluator::new(&p);
    for uv in [[0.13, 0.17], [-0.21, 0.32], [0., 0.]] {
        let v = e.sample("fractured-rock", Channel::Height, uv).unwrap();
        assert!(v[0].is_finite() && v[0] >= 0. && v[0] <= 0.08);
    }
}
#[test]
#[ignore = "requires a macOS Metal device"]
#[cfg(target_os = "macos")]
fn rock_primitives_match_metal() {
    use std::{fs, process::Command};
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("work/rock-core-metal");
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
    let mut cases = vec![packed().to_owned()];
    for sides in [3, 4, 7, 16] {
        for irregularity in [0., 0.6] {
            cases.push(polygon(&format!("      set \"sides\" to {sides}\n      set \"irregularity\" to {irregularity}\n      set \"seed\" to 73\n")).replace("output height from row \"p\" distance","output base color from row \"p\" distance"));
        }
    }
    for output in 0..3 {
        cases.push(field(true,&format!("    use \"Height Gradient\"\n      input from row \"height\" xxx\n      set \"radius\" to 0.05\n      set \"units\" to 1\n      set \"output\" to {output}\n")));
    }
    for mode in 0..3 {
        for centered in 0..2 {
            cases.push(field(false,&format!("    use \"Vector Filter\"\n      input from row \"height\" xyz\n      set \"radius\" to 0.1\n      set \"units\" to 1\n      set \"mode\" to {mode}\n      set \"centered\" to {centered}\n")));
        }
    }
    for (i, text) in cases.into_iter().enumerate() {
        let p = compile(&text).unwrap();
        let shader = dir.join("program.metal");
        let reference = dir.join("reference.rgba32f");
        fs::write(
            &shader,
            metal::generate(&p, "m", Channel::BaseColor).unwrap(),
        )
        .unwrap();
        let bytes: Vec<u8> = Evaluator::new(&p)
            .render("m", Channel::BaseColor, 13, 11)
            .unwrap()
            .iter()
            .flat_map(|v| {
                v.iter()
                    .chain(std::iter::once(&1.))
                    .flat_map(|f| f.to_le_bytes())
            })
            .collect();
        fs::write(&reference, bytes).unwrap();
        let result = Command::new(&harness)
            .arg(&shader)
            .arg(&reference)
            .args(["13", "11"])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "case {i}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        println!(
            "case {i}: {}",
            String::from_utf8_lossy(&result.stdout).trim()
        );
    }
}
