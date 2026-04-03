use buildergraph::{BuilderGraph, BuilderScript};
use image::RgbaImage;
use std::env;
use std::fs;
use std::path::Path;

fn save_png(path: &Path, width: u32, height: u32, pixels: Vec<u8>) -> Result<(), String> {
    let image = RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| "failed to create image buffer".to_string())?;
    image
        .save(path)
        .map_err(|err| format!("failed to save {}: {err}", path.display()))
}

fn main() {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: cargo run -p buildergraph -- <graph.buildergraph>");
        std::process::exit(1);
    };

    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("failed to read {path}: {err}");
            std::process::exit(1);
        }
    };

    let (name, output_spec, assembly, preview) = match BuilderScript::from_text(&source) {
        Ok(script) => {
            let assembly = match script.evaluate() {
                Ok(assembly) => assembly,
                Err(err) => {
                    eprintln!("failed to evaluate {path}: {err}");
                    std::process::exit(1);
                }
            };
            (
                script.name.clone(),
                script.output_spec(),
                assembly.clone(),
                script.render_preview(256),
            )
        }
        Err(script_err) => match BuilderGraph::from_text(&source) {
            Ok(graph) => {
                let assembly = match graph.evaluate() {
                    Ok(assembly) => assembly,
                    Err(err) => {
                        eprintln!("failed to evaluate {path}: {err}");
                        std::process::exit(1);
                    }
                };
                (
                    graph.name.clone(),
                    graph.output_spec(),
                    assembly.clone(),
                    graph.render_preview(256),
                )
            }
            Err(graph_err) => {
                eprintln!("failed to parse {path}:");
                eprintln!("  script: {script_err}");
                eprintln!("  graph:  {graph_err}");
                std::process::exit(1);
            }
        },
    };

    println!("graph: {}", name);
    println!(
        "target: {:?} (hosts: {})",
        output_spec.target, output_spec.host_refs
    );
    println!("primitives: {}", assembly.primitives.len());
    println!("anchors: {}", assembly.anchors.len());
    for (index, primitive) in assembly.primitives.iter().enumerate() {
        println!("primitive[{index}]: {:?}", primitive);
    }
    for (index, anchor) in assembly.anchors.iter().enumerate() {
        println!("anchor[{index}]: {} {:?}", anchor.name, anchor.transform);
    }

    let input_path = Path::new(&path);
    let output_file = if let Some(stem) = input_path.file_stem().and_then(|s| s.to_str()) {
        input_path.with_file_name(format!("{stem}.png"))
    } else {
        input_path.with_extension("png")
    };

    if let Err(err) = save_png(&output_file, preview.width, preview.height, preview.pixels) {
        eprintln!("{err}");
        std::process::exit(1);
    }

    println!("preview: {}", output_file.display());
}
