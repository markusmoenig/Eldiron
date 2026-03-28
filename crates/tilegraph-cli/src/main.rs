use image::RgbaImage;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use theframework::prelude::*;
use tilegraph::{RenderedTileGraph, TileGraphDocument, TileGraphRenderer, TileNodeGraphExchange};

fn save_png(path: &Path, width: u32, height: u32, pixels: Vec<u8>) -> Result<(), String> {
    let image = RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| format!("Failed to create image buffer for {}", path.display()))?;
    image.save(path).map_err(|e| e.to_string())
}

fn parse_palette_arg(value: &str) -> Vec<TheColor> {
    value
        .split(',')
        .filter_map(|entry| {
            let trimmed = entry.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(TheColor::from_hex(trimmed))
            }
        })
        .collect()
}

// CC-29 palette: https://lospec.com/palette-list/cc-29
fn default_cc29_palette() -> Vec<TheColor> {
    [
        "#f2f0e5", "#b8b5b9", "#868188", "#646365", "#45444f", "#3a3858", "#212123", "#352b42",
        "#43436a", "#4b80ca", "#68c2d3", "#a2dcc7", "#ede19e", "#d3a068", "#b45252", "#6a536e",
        "#4b4158", "#80493a", "#a77b5b", "#e5ceb4", "#c2d368", "#8ab060", "#567b79", "#4e584a",
        "#7b7243", "#b2b47e", "#edc8c4", "#cf8acb", "#5f556a",
    ]
    .into_iter()
    .map(TheColor::from_hex)
    .collect()
}

fn print_usage() {
    eprintln!(
        "Usage: tilegraph-cli <graph.eldiron_graph|graph.tilegraph> [output_dir] [--palette #RRGGBB,#RRGGBB,...]"
    );
}

fn write_rendered_graph(output_dir: &Path, rendered: RenderedTileGraph) -> Result<(), String> {
    fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;

    let sheet_width = (rendered.tile_width * rendered.grid_width) as u32;
    let sheet_height = (rendered.tile_height * rendered.grid_height) as u32;
    save_png(
        &output_dir.join("sheet_color.png"),
        sheet_width,
        sheet_height,
        rendered.sheet_color,
    )?;
    save_png(
        &output_dir.join("sheet_material.png"),
        sheet_width,
        sheet_height,
        rendered.sheet_material,
    )?;

    for cell_y in 0..rendered.grid_height {
        for cell_x in 0..rendered.grid_width {
            let index = cell_y * rendered.grid_width + cell_x;
            save_png(
                &output_dir.join(format!("tile_{}_{}.png", cell_x, cell_y)),
                rendered.tile_width as u32,
                rendered.tile_height as u32,
                rendered.tiles_color[index].clone(),
            )?;
            save_png(
                &output_dir.join(format!("tile_{}_{}_material.png", cell_x, cell_y)),
                rendered.tile_width as u32,
                rendered.tile_height as u32,
                rendered.tiles_material[index].clone(),
            )?;
        }
    }
    Ok(())
}

fn write_rendered_sheet_png(output_file: &Path, rendered: RenderedTileGraph) -> Result<(), String> {
    if let Some(parent) = output_file.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let sheet_width = (rendered.tile_width * rendered.grid_width) as u32;
    let sheet_height = (rendered.tile_height * rendered.grid_height) as u32;
    save_png(output_file, sheet_width, sheet_height, rendered.sheet_color)
}

fn load_graph_from_text(input_path: &Path, text: &str) -> Result<TileNodeGraphExchange, String> {
    let is_tilegraph = input_path
        .file_name()
        .and_then(|f| f.to_str())
        .map(|f| f.ends_with(".tilegraph") || f.ends_with(".tilegraph.toml"))
        .unwrap_or(false);
    if is_tilegraph {
        return TileGraphDocument::from_toml_str(text)
            .and_then(|doc| doc.to_exchange())
            .map_err(|e| e.to_string());
    }
    serde_json::from_str(text).map_err(|e| e.to_string())
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
    let mut output_path: Option<String> = None;

    let mut palette = Vec::new();
    while let Some(arg) = args.next() {
        if arg == "--palette" {
            let Some(value) = args.next() else {
                return Err("Missing value for --palette.".to_string());
            };
            palette = parse_palette_arg(&value);
        } else if output_path.is_none() {
            output_path = Some(arg);
        } else {
            return Err(format!("Unknown argument: {arg}"));
        }
    }

    let input_path = PathBuf::from(&input);
    let text = fs::read_to_string(&input_path).map_err(|e| e.to_string())?;
    let mut graph = load_graph_from_text(&input_path, &text)?;
    graph.graph_state.ensure_root();
    if palette.is_empty() && !graph.palette_colors.is_empty() {
        palette = graph.palette_colors.clone();
    }
    if palette.is_empty() {
        palette = default_cc29_palette();
    }

    let renderer = TileGraphRenderer::new(palette);
    let rendered = renderer.render_graph(&graph);
    let output_path = output_path.map(PathBuf::from);

    if let Some(path) = output_path {
        write_rendered_graph(&path, rendered)?;
        println!(
            "Rendered '{}' to {}",
            if graph.graph_name.is_empty() {
                input.as_str()
            } else {
                graph.graph_name.as_str()
            },
            path.display()
        );
    } else {
        let output_file = if input.ends_with(".tilegraph") || input.ends_with(".tilegraph.toml") {
            let file_name = input_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("graph.tilegraph");
            let stem = file_name
                .trim_end_matches(".tilegraph.toml")
                .trim_end_matches(".tilegraph");
            input_path.with_file_name(format!("{stem}.png"))
        } else {
            input_path.with_extension("png")
        };
        write_rendered_sheet_png(&output_file, rendered)?;
        println!(
            "Rendered '{}' to {}",
            if graph.graph_name.is_empty() {
                input.as_str()
            } else {
                graph.graph_name.as_str()
            },
            output_file.display()
        );
    }
    Ok(())
}
