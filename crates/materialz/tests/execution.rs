use denrim_materials::{
    compile,
    cpu::Evaluator,
    ir::Channel,
    metal,
    scope::{Scope, classify},
};
const INSTANCE: &str = "project 1\nmaterial \"m\" called \"Scope\"\n  row \"size\"\n    use \"Cell Data\"\n      set \"scope\" to 1\n  row \"shape\"\n    use \"Circle\"\n      set \"radius\" from row \"size\" x mapped 0.2 to 0.4\n  row \"profile\"\n    use \"Distance Profile\"\n      input from row \"shape\" distance\n  row \"repeat\"\n    use \"Grid\"\n      input from row \"profile\" xyz\n  output height from row \"repeat\" x\n";
fn filtered() -> String {
    include_str!("../examples/basketweave.material").replace("  row \"surface\"","  row \"filtered\"\n    use \"Directional Blur\"\n      input from row \"pattern\" xyz\n      set \"samples\" to 9\n  row \"surface\"").replace("input from row \"pattern\" xxx","input from row \"filtered\" xxx")
}
#[test]
fn instance_geometry_keeps_distance_but_uv_geometry_does_not() {
    let p = compile(INSTANCE).unwrap();
    let id = p.tile("m", "size", 0).unwrap();
    assert_eq!(
        classify(p.nodes[id.0].value_dependencies[0]),
        Scope::Instance
    );
    assert!(p.nodes[p.tile("m", "shape", 0).unwrap().0].has_distance);
    assert!(
        Evaluator::new(&p)
            .sample("m", Channel::Height, [0.13, 0.17])
            .unwrap()[0]
            .is_finite()
    );
    assert!(compile(&INSTANCE.replace("set \"scope\" to 1", "set \"scope\" to 0")).is_err());
}
#[test]
fn expensive_filters_partition_and_refresh_only_their_dependencies() {
    let text = filtered();
    let p = compile(&text).unwrap();
    let s = metal::compile(&p, "basketweave", Channel::BaseColor).unwrap();
    assert_eq!(s.passes.len(), 1);
    assert!(s.passes[0].source.contains("kernel void material_cache"));
    assert!(s.source.contains("mz_cache_lookup"));
    assert!(s.layout_json().contains("\"passes\":1"));
    let key = s.pass_key(0).unwrap();
    let changed = compile(&text.replace("0.68 0.24 0.075", "0.5 0.24 0.075")).unwrap();
    let mut refreshed = s.clone();
    refreshed.refresh(&changed).unwrap();
    assert_eq!(refreshed.pass_key(0).unwrap(), key);
    assert_ne!(refreshed.parameter_bytes(), s.parameter_bytes());
    let changed =
        compile(&text.replace("set \"scaleValue\" to 8", "set \"scaleValue\" to 7")).unwrap();
    refreshed.refresh(&changed).unwrap();
    assert_ne!(refreshed.pass_key(0).unwrap(), key);
}

#[test]
fn named_fields_are_independent_of_display_mode_and_share_resolved_nodes() {
    let text = "project 1\nmaterial \"m\" called \"Fields\"\n  row \"cells\"\n    use \"Voronoi\"\n      set \"output\" to 4\n      set \"jitter\" to 0\n  row \"copy\"\n    use \"Multiply\"\n      input from row \"cells\" nearest.xxx\n      set \"localValue\" from row \"cells\" nearest.xxx\n  output base color from row \"copy\" xyz\n  output height from row \"cells\" border.x\n";
    let p = compile(text).unwrap();
    let n = &p.nodes[p.tile("m", "copy", 0).unwrap().0];
    let denrim_materials::ir::Binding::From(other) = n.parameter("localValue") else {
        panic!()
    };
    assert_eq!(n.input.as_ref().unwrap().node, other.node);
    let e = Evaluator::new(&p);
    let v = e.sample("m", Channel::BaseColor, [0.013, 0.017]).unwrap();
    assert!((v[0] - (0.37 * 0.37 + 0.33 * 0.33)).abs() < 0.00003);
    assert!((e.sample("m", Channel::Height, [0.013, 0.017]).unwrap()[0] - 0.13).abs() < 0.00003);
    assert!(compile(&text.replace("border.x", "key.w")).is_err());
    assert!(compile(&text.replace("border.x", "missing.x")).is_err());
}
#[test]
fn xyz_coordinates_reach_volume_sources_and_polar_round_trips() {
    let text = "project 1\nmaterial \"m\" called \"Coordinates\"\n  row \"domain\"\n    use \"Coordinates\"\n      set \"offset\" to 0 0 0.37\n  row \"n\"\n    coordinates from row \"domain\" xyz\n    use \"Perlin Noise\"\n      set \"dimensions\" to 1\n  output base color from row \"n\" xyz\n";
    let direct = text
        .replace("    coordinates from row \"domain\" xyz\n", "")
        .replace(
            "      set \"dimensions\" to 1",
            "      set \"dimensions\" to 1\n      set \"slice\" to 0.37",
        );
    for uv in [[0.137, 0.213], [-0.17, 0.23]] {
        let a = Evaluator::new(&compile(text).unwrap())
            .sample("m", Channel::BaseColor, uv)
            .unwrap();
        let b = Evaluator::new(&compile(&direct).unwrap())
            .sample("m", Channel::BaseColor, uv)
            .unwrap();
        assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 0.000001));
    }
    let polar = "project 1\nmaterial \"m\" called \"Polar\"\n  row \"r\"\n    use \"Coordinates\"\n      set \"mapping\" to 1\n    use \"Coordinates\"\n      input from row \"polar\" xyz\n      set \"source\" to 2\n      set \"mapping\" to 2\n  row \"polar\"\n    use \"Coordinates\"\n      set \"mapping\" to 1\n  output base color from row \"r\" xyz\n";
    let p = compile(polar).unwrap();
    let v = Evaluator::new(&p)
        .sample("m", Channel::BaseColor, [0.13, -0.23])
        .unwrap();
    assert!((v[0] - 0.13).abs() < 0.00001 && (v[1] + 0.23).abs() < 0.00001);
}
#[test]
fn irregular_intervals_preserve_opaque_and_partial_coverage_at_shared_boundaries() {
    for alpha in [1., 0.25] {
        let text = format!(
            "project 1\nmaterial \"m\" called \"P\"\n  row \"r\"\n    use \"Color\"\n      set \"color\" to 0.8 0.2 0.1 {alpha}\n    use \"Partition\"\n      set \"gap\" to 0\n      set \"jitter\" to 0.95\n      set \"seed\" to 13\n"
        );
        let p = compile(&text).unwrap();
        let e = Evaluator::new(&p);
        for i in 0..57 {
            let v = e
                .sample_tile("m", "r", 1, [i as f32 / 57. - 0.5, 0.13], 0.002)
                .unwrap();
            assert!((v.color[3] - alpha).abs() < 0.0001, "{:?}", v.color);
        }
    }
}
#[test]
fn scatter_height_blends_and_winner_fields_have_distinct_semantics() {
    let text = "project 1\nmaterial \"m\" called \"S\"\n  row \"r\"\n    use \"Color\"\n      set \"color\" to 0.5 0.5 0.5 1\n    use \"Scatter\"\n      set \"cells\" to 2 2\n      set \"size\" to 3 3\n      set \"variation\" to 0\n      set \"intensity\" to 0\n      set \"amount\" to 2\n      set \"blend\" to 2\n  output height from row \"r\" x\n";
    let p = compile(text).unwrap();
    let maximum = Evaluator::new(&p)
        .sample("m", Channel::Height, [0.13, 0.17])
        .unwrap()[0];
    assert!((maximum - 0.5).abs() < 0.00001);
    let p = compile(&text.replace("set \"blend\" to 2", "set \"blend\" to 1")).unwrap();
    assert!(
        Evaluator::new(&p)
            .sample("m", Channel::Height, [0.13, 0.17])
            .unwrap()[0]
            > maximum
    );
    let p = compile(&text.replace(
        "  output height from row \"r\" x",
        "  output base color from row \"r\" key.xyz",
    ))
    .unwrap();
    assert!(
        Evaluator::new(&p)
            .sample("m", Channel::BaseColor, [0.13, 0.17])
            .unwrap()
            .iter()
            .all(|x| x.is_finite())
    );
}

#[test]
fn placement_frames_compose_and_drive_profiles_without_parent_cycles() {
    let text = "project 1\nmaterial \"m\" called \"Frame\"\n  row \"data\"\n    use \"Cell Data\"\n      set \"scope\" to 1\n      set \"output\" to 12\n  row \"r\"\n    use \"Grid\"\n      input from row \"data\" xyz\n      set \"cells\" to 4 8\n      set \"gap\" to 0.2 0.4\n  output base color from row \"r\" xyz\n";
    let p = compile(text).unwrap();
    let v = Evaluator::new(&p)
        .sample("m", Channel::BaseColor, [-0.375, -0.4375])
        .unwrap();
    for (a, b) in v.into_iter().zip([0.2, 0.075, 1.]) {
        assert!((a - b).abs() < 0.00001, "{a} != {b}")
    }
    let text = "project 1\nmaterial \"m\" called \"Bevel\"\n  row \"data\"\n    use \"Cell Data\"\n      set \"scope\" to 1\n  row \"width\"\n    use \"Value\"\n      set \"globalValue\" to 0.01\n    use \"Divide\"\n      set \"divisor\" from row \"data\" size.xxx\n  row \"shape\"\n    use \"Circle\"\n      set \"radius\" to 0.4\n  row \"profile\"\n    use \"Distance Profile\"\n      input from row \"shape\" distance\n      set \"profile\" to 0\n      set \"width\" from row \"width\" x\n  row \"r\"\n    use \"Grid\"\n      input from row \"profile\" xyz\n      set \"cells\" to 2 2\n      set \"gap\" to 0 0\n  output base color from row \"r\" xyz\n";
    let p = compile(text).unwrap();
    let v = Evaluator::new(&p)
        .sample("m", Channel::BaseColor, [-0.055, -0.25])
        .unwrap();
    assert!((v[0] - 0.5).abs() < 0.00005, "{v:?}");
}
#[test]
fn affine_domains_cover_cached_queries_and_unsafe_contexts_are_diagnosed() {
    let text=filtered().replace("  row \"surface\"","  row \"moved\"\n    use \"Transform\"\n      input from row \"filtered\" xyz\n      set \"globalScale\" to 0.5\n      set \"offset\" to 0.2 0.1\n  row \"surface\"").replace("input from row \"filtered\" xxx","input from row \"moved\" xxx");
    let p = compile(&text).unwrap();
    let s = metal::compile(&p, "basketweave", Channel::BaseColor).unwrap();
    assert_eq!(s.domains, vec![[2., 0., 0., 2., -0.4, -0.2]]);
    let warped=text.replace("    use \"Transform\"\n      input from row \"filtered\" xyz\n      set \"globalScale\" to 0.5\n      set \"offset\" to 0.2 0.1","    use \"Warp\"\n      input from row \"filtered\" xyz");
    let p = compile(&warped).unwrap();
    assert!(
        metal::compile(&p, "basketweave", Channel::BaseColor)
            .unwrap_err()
            .to_string()
            .contains("spatially varying domain")
    );
}

#[test]
fn loop_control_and_custom_sources_cannot_hide_pixel_dependent_geometry() {
    use denrim_materials::{metadata::Kernel, tilelang};
    let kernel = Kernel::find("Value").unwrap();
    let prefix = tilelang::source(kernel).split("let v0 =").next().unwrap();
    let code = format!(
        "{prefix}state varying = 0\nrepeat 1 times 1 while less(uv.x, 0)\nupdate varying = 0.3\nend repeat\nimage = vec4(0.3, varying, varying, 1)\n"
    );
    let body = code
        .lines()
        .map(|l| format!("        {l}\n"))
        .collect::<String>();
    let text = format!(
        "project 1\nmaterial \"m\" called \"Scopes\"\n  row \"data\"\n    use \"Value\"\n      source\n{body}      end source\n  row \"shape\"\n    use \"Circle\"\n      set \"radius\" from row \"data\" x\n  row \"profile\"\n    use \"Distance Profile\"\n      input from row \"shape\" distance\n"
    );
    assert!(compile(&text).is_ok());
    assert!(compile(&text.replace("from row \"data\" x", "from row \"data\" y")).is_err());
}

#[test]
fn cached_filters_keep_repeat_and_seamless_domains_for_outputs_and_previews() {
    use denrim_materials::cpu::PreviewMode;
    for mode in ["repeat", "seamless"] {
        let text = filtered().replace("tiling raw", &format!("tiling {mode}"));
        let p = compile(&text).unwrap();
        let s = metal::compile(&p, "basketweave", Channel::BaseColor).unwrap();
        assert_eq!(s.passes.len(), 1);
        assert!(!s.fixed_domains.is_empty());
        assert!(s.domain_fixed.iter().all(|x| *x));
        let s = metal::compile_tile_preview(&p, "basketweave", "surface", 0, PreviewMode::Image)
            .unwrap();
        assert_eq!(s.passes.len(), 1);
        assert!(!s.fixed_domains.is_empty());
    }
}
