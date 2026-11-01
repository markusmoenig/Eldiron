use denrim_materials::{
    compile,
    cpu::Evaluator,
    format,
    ir::{Channel, Program},
    metadata::Kernel,
    metal,
    tilelang::{self, Algorithm},
};

fn color_project(body: &str) -> String {
    let code = tilelang::source(Kernel::Color).replace(
        "image = vec4(color.rgb, clamp(color.a, 0, 1) * clamp(input.a, 0, 1))",
        body,
    );
    let block = code
        .lines()
        .map(|line| format!("        {line}\n"))
        .collect::<String>();
    format!(
        "project 1\nmaterial \"m\" called \"M\"\n  mode art\n  row \"c\"\n    use \"Color\"\n      source\n{block}      end source\n  output base color from row \"c\" xyz\n  output opacity from row \"c\" w\n"
    )
}
#[test]
fn source_controls_preserve_curated_metadata() {
    let mut errors = Vec::new();
    for k in Kernel::ALL {
        if k == Kernel::Perlin {
            // The family intentionally has new names/controls; its two original
            // scale defaults remain intact and the numeric reference test below
            // still checks the original 2D algorithm independently.
            let original = denrim_materials::metadata::parse(k.original()).unwrap();
            for i in 0..2 {
                assert_eq!(
                    k.metadata().parameters[i].default,
                    original.parameters[i].default
                );
            }
            assert_eq!(
                k.metadata()
                    .parameters
                    .iter()
                    .find(|p| p.id == "dimensions")
                    .unwrap()
                    .choices,
                ["2D", "3D"]
            );
        } else if matches!(
            k,
            Kernel::PeriodicProfile
                | Kernel::CompareSelect
                | Kernel::CellData
                | Kernel::DirectionalGrain
                | Kernel::HeightToNormal
        ) {
            assert_eq!(k.metadata(), tilelang::definition(k.original()).unwrap());
        } else if k == Kernel::ColorBlend {
            assert_eq!(k.metadata().name, "Mix");
            assert!(k.metadata().parameters.iter().any(|p| p.id == "factor"));
        } else if !matches!(k, Kernel::Library(_)) {
            assert_eq!(
                k.metadata(),
                denrim_materials::metadata::parse(k.original()).unwrap()
            );
        }
        if let Err(e) = Algorithm::compile(tilelang::source(k), &k.metadata(), k) {
            errors.push(format!("{}: {e}", k.metadata().name));
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
#[test]
fn custom_source_executes_and_round_trips_in_project() {
    let text = color_project(
        "let mask = clamp(uv.x + 0.5, 0, 1)\nimage = vec4(mix(vec3(0), color.rgb, mask), 0.25)",
    );
    let ast = format::parse(&text).unwrap();
    assert_eq!(format::parse(&format::write(&ast)).unwrap(), ast);
    let p = Program::lower(&ast).unwrap();
    let e = Evaluator::new(&p);
    assert_eq!(
        e.sample("m", Channel::BaseColor, [-0.5, 0.]).unwrap(),
        vec![0.; 3]
    );
    let a = e.sample("m", Channel::BaseColor, [0.5, 0.]).unwrap();
    assert_eq!(a, vec![0.2, 0.7, 0.9]);
    assert_eq!(
        e.sample("m", Channel::Opacity, [0., 0.]).unwrap(),
        vec![0.25]
    );
    let shader = metal::compile(&p, "m", Channel::BaseColor).unwrap();
    assert!(shader.source.contains("mix("));
    let mut changed = shader.clone();
    let new = compile(&text.replace("0.25)", "0.5)")).unwrap();
    assert!(
        changed.refresh(&new).is_err(),
        "Algorithm edits must invalidate pipelines"
    );
}
#[test]
fn types_scopes_and_work_are_bounded() {
    let cases = [
        "image = uv",
        "let x = missing\nimage = vec4(1)",
        "let x = 1\nlet x = 2\nimage = vec4(x)",
        "image = vec4(uv.xyz,1)",
        "image = vec4(mix(vec2(1), vec3(0), 0.5),1)",
        "while true",
        "image = vec4(foo(1))",
    ];
    for body in cases {
        assert!(compile(&color_project(body)).is_err(), "{body}");
    }
    assert!(
        compile(&color_project(&format!(
            "image = vec4({}1{})",
            "(".repeat(40),
            ")".repeat(40)
        )))
        .is_err()
    );
    assert!(compile(&color_project("image = vec4(1e100)")).is_err());
    assert!(compile(&color_project("image = vec4(1)").replace("      end source\n", "")).is_err());
}
#[test]
fn perlin_language_matches_curated_reference() {
    let p=compile("project 1\nmaterial \"m\" called \"M\"\n  row \"n\"\n    use \"Perlin Noise\"\n      set \"globalScale\" to 0.55\n      set \"localScale\" to 1 2.5\n  output roughness from row \"n\" x\n").unwrap();
    for i in -20..20 {
        for j in -10..10 {
            let uv = [i as f32 * 0.031, j as f32 * 0.027];
            let actual = Evaluator::new(&p)
                .sample("m", Channel::Roughness, uv)
                .unwrap()[0];
            let expected = denrim_materials::cpu::perlin(uv, 0.55, [1., 2.5]);
            assert!(
                (actual - expected).abs() < 0.000002,
                "{uv:?}: {actual} vs {expected}"
            );
        }
    }
}
#[test]
fn bsdf_channel_contract_and_modes() {
    assert_eq!(Channel::SURFACE.len(), 40);
    for channel in Channel::SURFACE {
        assert_eq!(Channel::parse(channel.name()).unwrap(), *channel);
        let components = if channel.width() == 3 { "xyz" } else { "x" };
        let text = format!(
            "project 1\nmaterial \"m\" called \"M\"\n  mode bsdf\n  row \"r\"\n    use \"Color\"\n  output {} from row \"r\" {components}\n",
            channel.name()
        );
        let p = compile(&text).unwrap();
        assert_eq!(
            Evaluator::new(&p)
                .sample("m", *channel, [0., 0.])
                .unwrap()
                .len(),
            channel.width()
        );
        metal::compile(&p, "m", *channel).unwrap();
    }
    let art = color_project("image = color");
    let parsed = format::parse(&art).unwrap();
    assert_eq!(parsed.materials[0].mode, "art");
    assert!(compile(&art.replace("mode art", "mode invalid")).is_err());
    assert!(compile(&art.replace("mode art", "mode art\n  mode bsdf")).is_err());
}
