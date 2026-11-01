use denrim_materials::{compile, cpu::Evaluator, format, ir::Channel, ir::Program, metal};

const SOURCE: &str = "project 1\nmaterial \"m\" called \"Material\"\n  mode pbr\n  row \"result\"\n    row input \"palette-edge\" from row \"palette\" xyzw mapped -1 to 2\n    use \"Perlin Noise\" id \"noise\"\n  row \"palette\"\n    use \"Color\" id \"color\"\n  output base color from row \"result\" xyz\n";

#[test]
fn retained_connections_round_trip_without_changing_the_render() {
    let project = format::parse(SOURCE).unwrap();
    assert_eq!(project.materials[0].rows[0].incoming[0].id, "palette-edge");
    let written = format::write(&project);
    assert_eq!(format::parse(&written).unwrap(), project);
    let mut without = project.clone();
    without.materials[0].rows[0].incoming.clear();
    let program = Program::lower(&project).unwrap();
    let plain = Program::lower(&without).unwrap();
    assert_eq!(program.nodes.len(), plain.nodes.len());
    for uv in [[0.1, 0.2], [-0.8, 3.1]] {
        assert_eq!(
            Evaluator::new(&program)
                .sample("m", Channel::BaseColor, uv)
                .unwrap(),
            Evaluator::new(&plain)
                .sample("m", Channel::BaseColor, uv)
                .unwrap()
        );
    }
}

#[test]
fn dormant_cycles_are_not_computational_cycles() {
    let source = SOURCE.replace(
        "    use \"Color\" id \"color\"",
        "    row input \"back\" from row \"result\" xyz\n    use \"Color\" id \"color\"",
    );
    compile(&source).unwrap();
    let active = source
        .replace(
            "    use \"Perlin Noise\" id \"noise\"",
            "    coordinates from row \"palette\" xy\n    use \"Perlin Noise\" id \"noise\"",
        )
        .replace(
            "    use \"Color\" id \"color\"",
            "    coordinates from row \"result\" xy\n    use \"Color\" id \"color\"",
        );
    assert!(compile(&active).unwrap_err().to_string().contains("cycle"));
}

#[test]
fn empty_receiver_keeps_its_connections_and_has_a_white_gpu_output() {
    let source = SOURCE.replace("    use \"Perlin Noise\" id \"noise\"\n", "");
    let project = format::parse(&source).unwrap();
    assert!(project.materials[0].rows[0].tiles.is_empty());
    let program = Program::lower(&project).unwrap();
    assert_eq!(
        Evaluator::new(&program)
            .sample("m", Channel::BaseColor, [0.3, 0.7])
            .unwrap(),
        vec![1.; 3]
    );
    metal::compile(&program, "m", Channel::BaseColor).unwrap();
    assert_eq!(format::parse(&format::write(&project)).unwrap(), project);
}

#[test]
fn source_deletion_and_invalid_retained_connections_are_diagnosed() {
    let missing = SOURCE.replace("from row \"palette\" xyzw", "from row \"missing\" xyzw");
    assert!(
        compile(&missing)
            .unwrap_err()
            .to_string()
            .contains("Unknown row")
    );
    let duplicate = SOURCE.replace(
        "    use \"Perlin Noise\" id \"noise\"",
        "    row input \"palette-edge\" from row \"palette\" x\n    use \"Perlin Noise\" id \"noise\"",
    );
    assert!(
        compile(&duplicate)
            .unwrap_err()
            .to_string()
            .contains("duplicate row input ID")
    );
    let invalid = SOURCE.replace("xyzw mapped", "abc mapped");
    assert!(
        compile(&invalid)
            .unwrap_err()
            .to_string()
            .contains("Invalid component")
    );
    let mut project = format::parse(SOURCE).unwrap();
    project.materials[0].rows[0].incoming[0].source.mapping = Some((0., f32::INFINITY));
    assert!(
        Program::lower(&project)
            .unwrap_err()
            .to_string()
            .contains("finite")
    );
}

#[cfg(feature = "integration")]
#[test]
fn native_bundle_keeps_dormant_edges_and_empty_receiver_rows() {
    use denrim_materials::integration::EditableMaterialAsset;
    let source = SOURCE.replace("    use \"Perlin Noise\" id \"noise\"\n", "");
    let asset = EditableMaterialAsset::parse("asset", &source, "m").unwrap();
    asset.validate().unwrap();
    let saved = asset.save().unwrap();
    let reopened = EditableMaterialAsset::reopen(&saved).unwrap();
    assert_eq!(asset.root.project, reopened.root.project);
    assert_eq!(reopened.save().unwrap(), saved);
}
