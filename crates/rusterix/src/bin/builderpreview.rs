use buildergraph::{BuilderDocument, BuilderGraph, BuilderScript};
use image::RgbaImage;
use rusterix::builderpreview::{BuilderPreviewOptions, PreviewVariants, render_builder_preview};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn save_png(path: &Path, width: u32, height: u32, pixels: Vec<u8>) -> Result<(), String> {
    let image = RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| "failed to create image buffer".to_string())?;
    image
        .save(path)
        .map_err(|err| format!("failed to save {}: {err}", path.display()))
}

fn main() {
    let (path, options) = match parse_args(env::args().skip(1).collect()) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("{err}");
            eprintln!(
                "usage: cargo run -p rusterix --bin builderpreview -- <file.buildergraph> [--size N] [--azimuth DEG] [--elevation DEG] [--scale HALF_HEIGHT] [--variants single|all]"
            );
            std::process::exit(1);
        }
    };

    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("failed to read {}: {err}", path.display());
            std::process::exit(1);
        }
    };

    let document = match BuilderDocument::from_text(&source) {
        Ok(document) => document,
        Err(err) => {
            let script_err = BuilderScript::from_text(&source).err().unwrap_or_default();
            let graph_err = BuilderGraph::from_text(&source).err().unwrap_or_default();
            eprintln!("failed to parse {}:", path.display());
            eprintln!("  document: {err}");
            eprintln!("  script:   {script_err}");
            eprintln!("  graph:    {graph_err}");
            std::process::exit(1);
        }
    };

    let assembly = match document.evaluate() {
        Ok(assembly) => assembly,
        Err(err) => {
            eprintln!("failed to evaluate {}: {err}", path.display());
            std::process::exit(1);
        }
    };

    let preview = match render_builder_preview(
        &assembly,
        document.output_spec(),
        &document.preview_host(),
        options,
    ) {
        Ok(preview) => preview,
        Err(err) => {
            eprintln!("failed to render {}: {err}", path.display());
            std::process::exit(1);
        }
    };

    let output = path.with_extension("png");
    if let Err(err) = save_png(&output, preview.width, preview.height, preview.pixels) {
        eprintln!("{err}");
        std::process::exit(1);
    }

    println!("graph: {}", document.name());
    println!("target: {:?}", document.output_spec().target);
    println!("primitives: {}", assembly.primitives.len());
    println!("surface details: {}", assembly.surface_details.len());
    println!("preview: {}", output.display());
}

fn parse_args(args: Vec<String>) -> Result<(PathBuf, BuilderPreviewOptions), String> {
    let mut options = BuilderPreviewOptions::default();
    let mut path = None;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--size" => {
                index += 1;
                options.size = args
                    .get(index)
                    .ok_or_else(|| "missing value for --size".to_string())?
                    .parse::<u32>()
                    .map_err(|err| format!("invalid --size: {err}"))?
                    .max(64);
            }
            "--azimuth" => {
                index += 1;
                options.azimuth_deg = args
                    .get(index)
                    .ok_or_else(|| "missing value for --azimuth".to_string())?
                    .parse::<f32>()
                    .map_err(|err| format!("invalid --azimuth: {err}"))?;
            }
            "--elevation" => {
                index += 1;
                options.elevation_deg = args
                    .get(index)
                    .ok_or_else(|| "missing value for --elevation".to_string())?
                    .parse::<f32>()
                    .map_err(|err| format!("invalid --elevation: {err}"))?;
            }
            "--scale" => {
                index += 1;
                options.scale = Some(
                    args.get(index)
                        .ok_or_else(|| "missing value for --scale".to_string())?
                        .parse::<f32>()
                        .map_err(|err| format!("invalid --scale: {err}"))?,
                );
            }
            "--variants" => {
                index += 1;
                options.variants = match args
                    .get(index)
                    .ok_or_else(|| "missing value for --variants".to_string())?
                    .as_str()
                {
                    "single" => PreviewVariants::Single,
                    "all" => PreviewVariants::AllLineDirections,
                    other => return Err(format!("invalid --variants '{other}'")),
                };
            }
            value if value.starts_with("--") => {
                return Err(format!("unknown option '{value}'"));
            }
            value => {
                if path.is_some() {
                    return Err(format!("unexpected extra argument '{value}'"));
                }
                path = Some(PathBuf::from(value));
            }
        }
        index += 1;
    }
    let path = path.ok_or_else(|| "missing input file".to_string())?;
    Ok((path, options))
}
