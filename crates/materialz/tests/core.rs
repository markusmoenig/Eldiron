use denrim_materials::{
    compile,
    cpu::{Evaluator, perlin},
    format,
    ir::{Channel, Limits, Program},
    metadata::{Kernel, ValueType},
    metal,
};

const DEMO: &str = include_str!("../examples/warped-disc.material");

#[test]
fn row_labels_are_optional_and_do_not_affect_reference_identity() {
    let source = "project 1\nmaterial \"m\" called \"M\"\n  row \"result\"\n    use \"Multiply\"\n      input from row \"noise\" xyz\n  row \"noise\"\n    use \"Perlin Noise\"\n  output base color from row \"result\" xyz\n";
    let ast = format::parse(source).unwrap();
    assert!(ast.materials[0].rows.iter().all(|r| r.label.is_empty()));
    assert_eq!(format::parse(&format::write(&ast)).unwrap(), ast);
    let p = compile(source).unwrap();
    let value = Evaluator::new(&p)
        .sample("m", Channel::BaseColor, [0.1, 0.2])
        .unwrap();
    let named = source.replace("row \"noise\"\n", "row \"noise\" called \"Named\"\n");
    assert_eq!(
        Evaluator::new(&compile(&named).unwrap())
            .sample("m", Channel::BaseColor, [0.1, 0.2])
            .unwrap(),
        value
    );
}

#[test]
fn tile_tab_sources_are_scoped_to_the_selected_definition() {
    let p = compile(DEMO).unwrap();
    let source = metal::tile_source(&p, "demo", "noise", 0).unwrap();
    assert!(source.contains("MaterialZ Tile Language") && source.contains("Perlin Noise"));
    assert!(!source.contains("kernel void material_preview"));
    let p = compile(include_str!("../examples/icon.material")).unwrap();
    let source = metal::tile_source(&p, "icon", "styled", 1).unwrap();
    assert!(source.contains("primitive border") && source.contains("Tile Language"));
    assert!(!source.contains("kernel void material_preview"));
}
fn project(rows: &str, output: &str) -> String {
    format!("project 1\nmaterial \"m\" called \"Test\"\n{rows}  output {output}\n")
}
fn check_error(source: &str, message: &str) {
    let e = compile(source).unwrap_err().to_string();
    assert!(e.contains(message), "{e:?} should contain {message:?}");
}

#[test]
fn metadata_preserves_original_controls() {
    for k in Kernel::ALL {
        let m = k.metadata();
        assert!(!m.description.is_empty());
        if !matches!(k, Kernel::Library(_)) {
            assert!(!m.parameters.is_empty());
        }
    }
    let p = Kernel::Perlin.metadata();
    assert_eq!(p.parameters[0].id, "globalScale");
    // The cleaned scale control excludes zero; the evaluator still rejects zero divisors.
    assert_eq!(p.parameters[0].range, Some((0.001, 10.)));
    assert_eq!(p.parameters[1].kind, ValueType::Vec2);
    assert_eq!(p.parameters[1].default, vec![1., 1.]);
    let d = Kernel::Disc.metadata();
    assert_eq!(d.parameters.len(), 6);
    assert!(!d.groups[1].expanded);
    assert_eq!(d.parameters[0].tooltip.as_deref(), Some("Beveling."));
    assert_eq!(
        Kernel::ColorBlend.metadata().parameters[1].default,
        vec![1., 1., 1., 1.]
    );
}
#[test]
fn canonical_format_round_trip() {
    let p = format::parse(DEMO).unwrap();
    let written = format::write(&p);
    assert_eq!(format::parse(&written).unwrap(), p);
    assert_eq!(format::write(&format::parse(&written).unwrap()), written);
    let escaped = project(
        "  row \"a\\\"b\" called \"A \\\\ B\"\n    use \"Disc\"\n",
        "height from row \"a\\\"b\" x",
    );
    let ast = format::parse(&escaped).unwrap();
    assert_eq!(format::parse(&format::write(&ast)).unwrap(), ast);
}
#[test]
fn forward_references_material_scopes_and_labels() {
    let p = compile(DEMO).unwrap();
    assert_eq!(p.materials.len(), 2);
    assert_eq!(p.nodes.len(), 6);
    let renamed = DEMO.replace("called \"Curated Perlin\"", "called \"Renamed Noise\"");
    let q = compile(&renamed).unwrap();
    assert_eq!(
        Evaluator::new(&p)
            .sample("demo", Channel::Height, [0.2, -0.1])
            .unwrap(),
        Evaluator::new(&q)
            .sample("demo", Channel::Height, [0.2, -0.1])
            .unwrap()
    );
}
#[test]
fn perlin_period_and_lattice_values() {
    for uv in [[0., 0.], [0.1, 0.2], [-0.2, -0.3]] {
        assert!((perlin(uv, 1., [1., 1.]) - 0.5).abs() < 1e-6);
    }
    for uv in [[0.123, 0.234], [-0.312, -0.123], [0.02, -0.7]] {
        let a = perlin(uv, 1., [1., 1.]);
        let b = perlin([uv[0] + 7.1, uv[1]], 1., [1., 1.]);
        assert!((a - b).abs() < 0.00003, "{a} {b}");
    }
}
#[test]
fn operators_and_transformed_function_sampling() {
    let source = project(
        "  row \"shape\" called \"Shape\"\n    use \"Disc\"\n      set \"inBEVEL\" to 0.5\n    use \"Multiply\"\n      set \"globalValue\" to 0.4\n      set \"localValue\" to 1 0.5 0.25\n    use \"Mix\"\n      set \"factor\" to 1 1 1\n      set \"color1\" to 0 0 0 1\n      set \"color2\" to 1 1 1 1\n  row \"moved\" called \"Moved\"\n    use \"Transform\"\n      input from row \"shape\" xyz\n      set \"offset\" to 0.8 0\n",
        "base color from row \"moved\" xyz",
    );
    let p = compile(&source).unwrap();
    let e = Evaluator::new(&p);
    assert_eq!(
        e.sample("m", Channel::BaseColor, [0.8, 0.]).unwrap(),
        vec![0.4, 0.2, 0.1]
    );
    assert_eq!(
        e.sample("m", Channel::BaseColor, [0., 0.]).unwrap(),
        vec![0., 0., 0.]
    );
}
#[test]
fn row_coordinates_wrap_entire_sequence() {
    let source = project(
        "  row \"coords\" called \"Coords\"\n    use \"Disc\"\n    use \"Multiply\"\n      set \"globalValue\" to 0\n  row \"sample\" called \"Sample\"\n    coordinates from row \"coords\" xy\n    use \"Disc\"\n    use \"Transform\"\n      set \"offset\" to 0.8 0\n",
        "height from row \"sample\" x",
    );
    // Coordinate source always returns zero. Transform then samples Disc at -0.8.
    // Applying the coordinate binding to the first tile would incorrectly return 1.
    let p = compile(&source).unwrap();
    assert_eq!(
        Evaluator::new(&p)
            .sample("m", Channel::Height, [0.2, 0.1])
            .unwrap(),
        vec![0.]
    );
}
#[test]
fn connected_parameters_change_result_and_safe_dynamic_divisors() {
    let source = project(
        "  row \"control\" called \"Control\"\n    use \"Disc\"\n    use \"Multiply\"\n      set \"globalValue\" to 0\n  row \"shape\" called \"Shape\"\n    use \"Disc\"\n      set \"globalScale\" from row \"control\" x\n",
        "height from row \"shape\" x",
    );
    let p = compile(&source).unwrap();
    let e = Evaluator::new(&p);
    assert_eq!(e.sample("m", Channel::Height, [0., 0.]).unwrap(), vec![1.]);
    assert_eq!(e.sample("m", Channel::Height, [0.1, 0.]).unwrap(), vec![0.]);
}
#[test]
fn rejects_cycles_types_unknown_names_and_invalid_values() {
    check_error(
        &DEMO.replace(
            "input from row \"shape\" xyz",
            "input from row \"surface\" xyz",
        ),
        "cycle",
    );
    check_error(
        &DEMO.replace(
            "from row \"noise\" x mapped -80",
            "from row \"noise\" xy mapped -80",
        ),
        "Expected 1 component",
    );
    check_error(
        &DEMO.replace("from row \"shape\" xyz", "from row \"absent\" xyz"),
        "Unknown row",
    );
    check_error(
        &DEMO.replace("set \"inBEVEL\" to 0.25", "set \"inBEVEL\" to 0"),
        "magnitude",
    );
    check_error(
        &DEMO.replace("set \"inBEVEL\" to 0.25", "set \"typo\" to 0.25"),
        "Unknown parameter",
    );
    check_error(
        &DEMO.replace("set \"inBEVEL\" to 0.25", "set \"inBEVEL\" to NaN"),
        "finite number",
    );
    check_error(
        &DEMO.replace("use \"Disc\"", "use \"Unknown\""),
        "Unknown source",
    );
    check_error(
        &DEMO.replace("output height", "output nonexistent"),
        "Unknown material channel",
    );
    check_error("project 2\n", "project 1");
}
#[test]
fn rejects_duplicate_ids_and_missing_input() {
    check_error(
        &DEMO.replace("row \"shape\" called", "row \"noise\" called"),
        "duplicate row",
    );
    check_error(
        &project(
            "  row \"a\" called \"A\"\n    use \"Multiply\"\n",
            "height from row \"a\" x",
        ),
        "needs input",
    );
}

#[test]
fn parser_rejects_statements_outside_their_scope() {
    let after_output = project(
        "  row \"a\" called \"A\"\n    use \"Disc\"\n",
        "height from row \"a\" x",
    ) + "    use \"Disc\"\n";
    check_error(&after_output, "outside row");
    let after_coordinates = project(
        "  row \"a\" called \"A\"\n    use \"Disc\"\n    coordinates from row \"a\" xy\n      set \"inBEVEL\" to 0.2\n",
        "height from row \"a\" x",
    );
    check_error(&after_coordinates, "outside tile");
    let tab = DEMO.replace("  row", "\trow");
    check_error(&tab, "spaces, not tabs");
    let duplicate = DEMO.replace(
        "set \"inBEVEL\" to 0.25",
        "set \"inBEVEL\" to 0.25\n      set \"inBEVEL\" to 0.4",
    );
    check_error(&duplicate, "Duplicate parameter");
    let unclosed = "project 1\nmaterial \"m called Test\n";
    check_error(unclosed, "line 2: Unterminated");
}
#[test]
fn expansion_budget_counts_shared_dependencies_at_each_call_site() {
    let ast = format::parse(DEMO).unwrap();
    assert!(
        Program::lower_with_limits(
            &ast,
            Limits {
                nodes: 3,
                ..Limits::default()
            }
        )
        .unwrap_err()
        .to_string()
        .contains("Node budget")
    );
    assert!(
        Program::lower_with_limits(
            &ast,
            Limits {
                expanded_calls: 2,
                ..Limits::default()
            }
        )
        .unwrap_err()
        .to_string()
        .contains("Complexity budget")
    );
    assert!(
        Program::lower_with_limits(
            &ast,
            Limits {
                depth: 2,
                ..Limits::default()
            }
        )
        .is_err()
    );
}
#[test]
fn rendering_and_metal_generation() {
    let p = compile(DEMO).unwrap();
    let pixels = Evaluator::new(&p)
        .render("demo", Channel::BaseColor, 32, 24)
        .unwrap();
    assert_eq!(pixels.len(), 768);
    assert!(pixels.iter().flatten().all(|v| v.is_finite()));
    assert!(pixels.windows(2).any(|w| w[0] != w[1]));
    assert!(
        Evaluator::new(&p)
            .render("demo", Channel::Height, 0, 1)
            .is_err()
    );
    let source = metal::generate(&p, "demo", Channel::BaseColor).unwrap();
    assert!(source.contains("kernel void material_preview"));
    assert!(
        !source.contains("mz_node_5("),
        "Unreachable second material must not be emitted"
    );
}

#[test]
fn material_can_disconnect_its_last_output_and_keep_editing_tiles() {
    let text = "project 1\nmaterial \"m\" called \"Unconnected\"\n  row \"c\"\n    use \"Color\"\n";
    let p = compile(text).unwrap();
    assert!(p.materials[0].outputs.is_empty());
    let sample = Evaluator::new(&p)
        .sample_tile("m", "c", 0, [0., 0.], 0.01)
        .unwrap();
    assert_eq!(sample.color, [0.2, 0.7, 0.9, 1.]);
    metal::compile_tile_preview(&p, "m", "c", 0, denrim_materials::cpu::PreviewMode::Image)
        .unwrap();
    let project = denrim_materials::format::parse(text).unwrap();
    assert_eq!(
        denrim_materials::format::parse(&denrim_materials::format::write(&project)).unwrap(),
        project
    );
}

#[test]
fn canvas_positions_round_trip_without_changing_material_values() {
    let base = format::parse(DEMO).unwrap();
    assert!(base.materials.iter().all(|m| m.position.is_none()));
    let mut placed = base.clone();
    placed.materials[0].position = Some([245.25, 708.5]);
    let text = format::write(&placed);
    assert!(text.contains("  position 245.25 708.5"));
    assert_eq!(format::parse(&text).unwrap(), placed);
    let document = denrim_materials::live::Document::parse(&text).unwrap();
    assert!(document.json().contains("\"position\":[245.25,708.5]"));
    let id = &base.materials[0].id;
    assert_eq!(
        Evaluator::new(&compile(DEMO).unwrap())
            .sample(id, Channel::BaseColor, [0.3, 0.4])
            .unwrap(),
        Evaluator::new(document.program())
            .sample(id, Channel::BaseColor, [0.3, 0.4])
            .unwrap()
    );
    for bad in [
        "position -1 2",
        "position 1 nan",
        "position 1",
        "position 1 2 3",
        "position 1 2\n  position 3 4",
    ] {
        let text = text.replace("position 245.25 708.5", bad);
        assert!(format::parse(&text).is_err(), "accepted {bad}");
    }
}
