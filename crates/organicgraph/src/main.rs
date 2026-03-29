use image::RgbaImage;
use organicgraph::OrganicBrushGraph;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn save_png(path: &Path, width: u32, height: u32, pixels: Vec<u8>) -> Result<(), String> {
    let image = RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| format!("Failed to create image buffer for {}", path.display()))?;
    image.save(path).map_err(|e| e.to_string())
}

fn print_usage() {
    eprintln!("Usage: organicgraph <graph.organicgraph> [output.png]");
    eprintln!("The input graph can be JSON or TOML.");
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(input) = args.next() else {
        print_usage();
        return Err("Missing graph input path.".to_string());
    };
    let output = args.next();

    let input_path = PathBuf::from(&input);
    let text = fs::read_to_string(&input_path).map_err(|e| e.to_string())?;
    let graph = OrganicBrushGraph::from_text(&text)?;
    let preview = graph.render_preview(256);

    let output_file = output
        .map(PathBuf::from)
        .unwrap_or_else(|| input_path.with_extension("png"));
    save_png(&output_file, preview.width, preview.height, preview.pixels)?;
    println!("Rendered '{}' to {}", graph.name, output_file.display());
    Ok(())
}
