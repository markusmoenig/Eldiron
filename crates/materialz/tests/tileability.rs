//! Audits unit-period behavior separately from continuity at the exported square.
use denrim_materials::{compile, cpu::Evaluator, metadata::Kernel};
fn error(a: [f32; 4], b: [f32; 4]) -> f32 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| (a - b).abs())
        .fold(0., f32::max)
}
fn measure(source: &str, tile: usize) -> (f32, f32) {
    let p = compile(source).unwrap();
    let e = Evaluator::new(&p);
    let get = |uv| e.sample_tile("m", "r", tile, uv, 0.00001).unwrap().color;
    let mut period = 0f32;
    let mut seam = 0f32;
    for i in 0..19 {
        let t = (i as f32 + 0.381) / 19. - 0.5;
        period = period.max(error(get([t, 0.173]), get([t + 1., 0.173])));
        period = period.max(error(get([0.213, t]), get([0.213, t + 1.])));
        seam = seam.max(error(get([-0.5, t]), get([0.5, t])));
        seam = seam.max(error(get([t, -0.5]), get([t, 0.5])));
    }
    (period, seam)
}
fn material(name: &str, settings: &str, input: Option<&str>) -> (String, usize) {
    (
        format!(
            "project 1\nmaterial \"m\" called \"M\"\n  row \"r\"\n{}    use \"{name}\"\n{settings}",
            input
                .map(|s| format!("    use \"{s}\"\n"))
                .unwrap_or_default()
        ),
        usize::from(input.is_some()),
    )
}
#[test]
fn integer_period_noise_and_fbm_remain_tileable() {
    // Removed Perlin variants are now covered by catalog-removal assertions.
    // Exercise both preserved Organic sources and general fBm on periodic input.
    for (name, settings, input) in [
        ("Organic Noise", "", None),
        ("Organic fBm", "", None),
        ("fBm", "", Some("Organic Noise")),
        ("Checker B/W", "      set \"scale\" to 6\n", None),
    ] {
        let (text, tile) = material(name, settings, input);
        let (period, seam) = measure(&text, tile);
        assert!(
            period < 0.002 && seam < 0.002,
            "{name}: period {period}, seam {seam}"
        );
    }
    let (text, tile) = material("Perlin Noise", "", None);
    assert!(
        measure(&text, tile).0 > 0.01,
        "Ordinary Perlin must not be mistaken for unit-period noise"
    );
}
#[test]
#[ignore = "diagnostic audit across all default tiles"]
fn audit_default_tileability() {
    let mut report = Vec::new();
    for k in Kernel::ALL {
        let m = k.metadata();
        let input = k
            .requires_input()
            .then_some(if m.name == "fBm" || m.name == "Warp" {
                "Perlin Noise"
            } else {
                "Disc"
            });
        let (text, tile) = material(&m.name, "", input);
        let result = std::panic::catch_unwind(|| measure(&text, tile));
        if let Ok((period, seam)) = result {
            println!("{}: period {period:.6}, seam {seam:.6}", m.name);
            report.push(format!("{{\"name\":{:?},\"category\":{:?},\"input\":{},\"unit_period_error\":{period},\"edge_value_error\":{seam}}}",m.name,m.category,input.map(|s|format!("{s:?}")).unwrap_or_else(||"null".into())));
        }
    }
    std::fs::create_dir_all("work").unwrap();
    std::fs::write(
        "work/tileability-audit.json",
        format!("[{}]", report.join(",\n")),
    )
    .unwrap();
}

#[test]
fn period_alignment_depends_on_parameters() {
    for (name, settings, input, should_repeat) in [
        ("Checker B/W", "", None, false),
        (
            "Organic Noise",
            "      set \"scaleValue\" to 10.5\n",
            None,
            false,
        ),
        (
            "Organic fBm",
            "      set \"scaleValue\" to 10.5\n",
            None,
            false,
        ),
        ("Tileable Color Grain", "", None, false),
        (
            "Tileable Color Grain",
            "      set \"globalScale\" to 1\n",
            None,
            true,
        ),
        ("Brick & Tiles (Pattern)", "", Some("Disc"), true),
        (
            "Brick & Tiles (Pattern)",
            "      set \"VCELLST\" to 21\n",
            Some("Disc"),
            false,
        ),
        ("Grid", "", Some("Disc"), true),
        ("Grid", "      set \"rotation\" to 17\n", Some("Disc"), true),
    ] {
        let (text, tile) = material(name, settings, input);
        let (period, seam) = measure(&text, tile);
        println!("{name} {settings:?}: unit period {period:.6}, edges {seam:.6}");
        assert_eq!(
            period < 0.002,
            should_repeat,
            "{name}: period {period}, seam {seam}"
        );
    }
}
