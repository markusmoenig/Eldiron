use denrim_materials::{compile, cpu::Evaluator, ir::Channel, metadata::Kernel, metal};
#[test]
fn every_library_tile_compiles_and_samples() {
    let mut failures = Vec::new();
    let mut names = std::collections::HashSet::new();
    for kernel in Kernel::ALL {
        let m = kernel.metadata();
        assert!(
            names.insert(m.name.clone()),
            "Duplicate source name {}",
            m.name
        );
        if !matches!(kernel, Kernel::Library(_)) {
            continue;
        }
        let input = if kernel.requires_input() {
            "    use \"Color\"\n"
        } else {
            ""
        };
        let text = format!(
            "project 1\nmaterial \"m\" called \"Library Check\"\n  row \"r\"\n{input}    use \"{}\"\n  output base color from row \"r\" xyz\n",
            m.name
        );
        match compile(&text) {
            Err(e) => failures.push(format!("{}: {e}", m.name)),
            Ok(p) => {
                if let Err(e) = metal::compile(&p, "m", Channel::BaseColor) {
                    failures.push(format!("{} Metal: {e}", m.name));
                }
                for uv in [[0.13, 0.17], [-0.31, 0.29], [0., 0.]] {
                    match Evaluator::new(&p).sample("m", Channel::BaseColor, uv) {
                        Ok(v) if v.iter().all(|x| x.is_finite()) => {}
                        value => failures.push(format!("{} at {uv:?}: {value:?}", m.name)),
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn fbm_resamples_its_input_and_loop_bounds_are_enforced() {
    let text = "project 1\nmaterial \"m\" called \"fBm\"\n  row \"n\"\n    use \"Perlin Noise\"\n    use \"fBm\"\n      set \"layers\" to 3\n  output roughness from row \"n\" x\n";
    let p = compile(text).unwrap();
    for uv in [[0.13, 0.17], [-0.23, 0.29]] {
        let weighted_sum = denrim_materials::cpu::perlin(uv, 1., [1., 1.])
            + 0.5 * denrim_materials::cpu::perlin(uv.map(|x| x * 2.), 1., [1., 1.])
            + 0.25 * denrim_materials::cpu::perlin(uv.map(|x| x * 4.), 1., [1., 1.]);
        // fBm now normalizes by the octave weights in both dimensions.
        // For these three octaves, the exact denominator is 1 + 0.5 + 0.25.
        let expected = weighted_sum / (1.0 + 0.5 + 0.25);
        let got = Evaluator::new(&p)
            .sample("m", Channel::Roughness, uv)
            .unwrap()[0];
        assert!(
            (got - expected).abs() < 0.00001,
            "Expected {expected}, got {got}"
        );
    }
    let k = Kernel::find("Gabor Noise").unwrap();
    let mut algorithm = denrim_materials::tilelang::source(k).to_owned();
    algorithm = algorithm.replace("repeat 201 times", "repeat 1000000 times");
    assert!(denrim_materials::tilelang::Algorithm::compile(&algorithm, &k.metadata(), k).is_err());
    let explosive=denrim_materials::tilelang::source(Kernel::Color).replace("image = vec4(color.rgb, clamp(color.a, 0, 1) * clamp(input.a, 0, 1))","state s = 0\nrepeat 1024 times 1024 while 1\nrepeat 1024 times 1024 while 1\nrepeat 1024 times 1024 while 1\nupdate s = s + 1\nend repeat\nend repeat\nend repeat\nimage = vec4(s)");
    let body = explosive
        .lines()
        .map(|s| format!("        {s}\n"))
        .collect::<String>();
    let project = format!(
        "project 1\nmaterial \"m\" called \"M\"\n  row \"r\"\n    use \"Color\"\n      source\n{body}      end source\n"
    );
    assert!(
        compile(&project)
            .unwrap_err()
            .to_string()
            .contains("complexity budget")
    );
}

#[test]
#[ignore = "requires a macOS Metal device"]
#[cfg(target_os = "macos")]
fn expanded_library_runs_on_metal() {
    use std::{fs, process::Command};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("work/library-metal");
    fs::create_dir_all(&dir).unwrap();
    let harness = dir.join("compare");
    let result = Command::new("xcrun")
        .args(["swiftc", "-module-cache-path"])
        .arg(dir.join("module-cache"))
        .arg(root.join("tests/metal_compare.swift"))
        .arg("-o")
        .arg(&harness)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let mut failures = Vec::new();
    for kernel in Kernel::ALL {
        let m = kernel.metadata();
        if !matches!(kernel, Kernel::Library(_)) {
            continue;
        }
        let input = if kernel.requires_input() {
            if m.category.contains("Mixer") || m.category.contains("Operator") {
                "    use \"Color\"\n"
            } else {
                "    use \"Perlin Noise\"\n"
            }
        } else {
            ""
        };
        let text = format!(
            "project 1\nmaterial \"m\" called \"Library Check\"\n  row \"r\"\n{input}    use \"{}\"\n  output base color from row \"r\" xyz\n",
            m.name
        );
        let p = compile(&text).unwrap();
        let shader = dir.join("program.metal");
        let reference = dir.join("cpu.rgba32f");
        fs::write(
            &shader,
            metal::generate(&p, "m", Channel::BaseColor).unwrap(),
        )
        .unwrap();
        let pixels = Evaluator::new(&p)
            .render("m", Channel::BaseColor, 7, 5)
            .unwrap();
        let mut bytes = Vec::new();
        for pixel in pixels {
            for x in [pixel[0], pixel[1], pixel[2], 1.] {
                bytes.extend(x.to_le_bytes());
            }
        }
        fs::write(&reference, bytes).unwrap();
        let r = Command::new(&harness)
            .arg(&shader)
            .arg(&reference)
            .args(["7", "5"])
            .output()
            .unwrap();
        if !r.status.success() {
            failures.push(format!(
                "{}: {}{}",
                m.name,
                String::from_utf8_lossy(&r.stdout),
                String::from_utf8_lossy(&r.stderr)
            ));
        } else {
            println!("{}: {}", m.name, String::from_utf8_lossy(&r.stdout).trim());
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
