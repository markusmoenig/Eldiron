#![cfg(feature = "wgpu")]
use denrim_materials::{compile, ir::Channel, metadata::Kernel, wgsl};

#[test]
fn pbr_authoring_maps_extend_the_runtime_profile_without_changing_it() {
    use denrim_materials::gpu::{output_layout, preview_output_layout};
    let runtime = output_layout();
    let preview = preview_output_layout();
    assert_eq!(runtime.len(), 5);
    assert_eq!(preview.len(), 7);
    assert_eq!(preview[5].channel, Channel::Height);
    assert_eq!(preview[5].format, wgpu::TextureFormat::R32Float);
    assert_eq!(preview[6].channel, Channel::Normal);
    assert_eq!(preview[6].format, wgpu::TextureFormat::Rgba16Float);
    assert_eq!(preview[6].color_space, "linear vector");
    for (a, b) in runtime.iter().zip(&preview) {
        assert_eq!(a.channel, b.channel);
        assert_eq!(a.format, b.format);
    }
    let source = "project 1\nmaterial \"m\" called \"PBR\"\n  mode pbr\n  default height to 0.65\n  default normal to 0.25 0.75 1\n";
    let program = compile(source).unwrap();
    for layout in preview {
        wgsl::compile(&program, "m", layout.channel).unwrap();
    }
}

#[test]
fn every_builtin_source_lowers_to_valid_wgsl() {
    for k in Kernel::ALL {
        let source = format!(
            "project 1\nmaterial \"m\" called \"M\"\n  tiling raw\n  row \"r\"\n    use \"Color\"\n    use \"{}\"\n  output base color from row \"r\" xyz\n",
            k.metadata().name
        );
        let program = compile(&source).unwrap();
        wgsl::compile(&program, "m", Channel::BaseColor)
            .unwrap_or_else(|e| panic!("{}: {e}", k.metadata().name));
    }
}
#[test]
fn examples_include_continuous_periodic_mixed_and_cached_materials() {
    for path in [
        "examples/warped-disc.material",
        "examples/wicker.material",
        "examples/fractured-rock.material",
        "examples/material-mixing.material",
    ] {
        let source = std::fs::read_to_string(path).unwrap();
        let program = compile(&source).unwrap();
        for m in &program.materials {
            wgsl::compile(&program, &m.id, Channel::BaseColor)
                .unwrap_or_else(|e| panic!("{path}/{}: {e}", m.id));
        }
    }
}
