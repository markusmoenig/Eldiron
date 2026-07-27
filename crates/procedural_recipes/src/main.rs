use procedural_recipes::{
    BinaryOperator, Colorize, CoordinateChannel, MaterialRecipe, Output, Recipe, RecipeDocument,
    RecipeRenderer, RenderOptions, ScalarSource, UnaryOperator, parse_document,
};
use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use theframework::prelude::{TheColor, ThePalette};

#[derive(Debug, Deserialize)]
struct LospecPalette {
    name: String,
    author: String,
    colors: Vec<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let Some(command) = arguments.first().map(String::as_str) else {
        print_usage();
        return Ok(());
    };
    match command {
        "validate" => validate_command(&arguments[1..]),
        "render" => render_command(&arguments[1..]),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => Err(format!("unknown command '{other}'")),
    }
}

fn validate_command(arguments: &[String]) -> Result<(), String> {
    let recipe_path = arguments
        .first()
        .map(PathBuf::from)
        .ok_or_else(|| "validate requires a recipe path".to_string())?;
    match load_document(&recipe_path)? {
        RecipeDocument::Tile(recipe) => {
            if let Some(alias) = &recipe.material {
                load_referenced_material(&recipe_path, alias)?;
            }
            println!(
                "Valid tile recipe '{}' ({}x{} pixels per tile, {}x{} cells, {} frame(s))",
                recipe.name,
                recipe.size[0],
                recipe.size[1],
                recipe.coverage[0],
                recipe.coverage[1],
                recipe.animation.frames
            );
        }
        RecipeDocument::Materials(document) => {
            println!(
                "Valid material recipe with {} declaration(s): {}",
                document.materials.len(),
                document
                    .materials
                    .iter()
                    .map(|material| material.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    Ok(())
}

fn render_command(arguments: &[String]) -> Result<(), String> {
    let recipe_path = arguments
        .first()
        .map(PathBuf::from)
        .ok_or_else(|| "render requires a recipe path".to_string())?;
    let palette_arg = option_value(arguments, "--palette")
        .ok_or_else(|| "render requires --palette <lospec:slug|file.hex>".to_string())?;
    let output_path = option_value(arguments, "--output")
        .map(PathBuf::from)
        .unwrap_or_else(|| recipe_path.with_extension("png"));
    let height_output = option_value(arguments, "--height-output").map(PathBuf::from);
    let material_selector = option_value(arguments, "--material");

    let (palette, palette_description) = load_palette(palette_arg)?;
    let renderer = RecipeRenderer::new(&palette).map_err(|error| error.to_string())?;
    match load_document(&recipe_path)? {
        RecipeDocument::Tile(recipe) => {
            if material_selector.is_some() {
                return Err("--material is only used when rendering a material recipe".to_string());
            }
            let mut rendered = renderer
                .render(&recipe, &RenderOptions::default())
                .map_err(|error| error.to_string())?;
            if let Some(alias) = &recipe.material {
                let material = load_referenced_material(&recipe_path, alias)?;
                let rendered_material = renderer
                    .render_material(&material, &rendered, &RenderOptions::default())
                    .map_err(|error| error.to_string())?;
                for (frame, material_frame) in
                    rendered.frames.iter_mut().zip(rendered_material.frames)
                {
                    frame.rgba = material_frame.rgba;
                    frame.palette_indices = material_frame.palette_indices;
                }
            }

            let outputs = write_frames(&rendered, &output_path)?;
            let height_outputs = height_output
                .as_deref()
                .map(|path| write_height_frames(&rendered, path))
                .transpose()?
                .unwrap_or_default();
            print_rendered_outputs(
                &rendered.name,
                &palette_description,
                &outputs,
                &height_outputs,
            );
        }
        RecipeDocument::Materials(document) => {
            let materials = if let Some(selector) = material_selector {
                vec![
                    document
                        .materials
                        .into_iter()
                        .find(|material| material.id.eq_ignore_ascii_case(selector))
                        .ok_or_else(|| {
                            format!(
                                "material recipe '{}' has no Material {} declaration",
                                recipe_path.display(),
                                selector
                            )
                        })?,
                ]
            } else {
                document.materials
            };
            let several = materials.len() > 1;
            for material in materials {
                let preview = renderer
                    .render(
                        &material_preview_recipe(&material),
                        &RenderOptions::default(),
                    )
                    .map_err(|error| error.to_string())?;
                let rendered = renderer
                    .render_material(&material, &preview, &RenderOptions::default())
                    .map_err(|error| error.to_string())?;
                let material_output = if several {
                    suffixed_output_path(&output_path, &material.id)
                } else {
                    output_path.clone()
                };
                let outputs = write_material_frames(&rendered, &material_output)?;
                let normal_outputs = height_output
                    .as_deref()
                    .map(|path| {
                        let path = if several {
                            suffixed_output_path(path, &material.id)
                        } else {
                            path.to_path_buf()
                        };
                        write_material_height_frames(&rendered, &path)
                    })
                    .transpose()?
                    .unwrap_or_default();
                print_rendered_outputs(
                    &material.name,
                    &palette_description,
                    &outputs,
                    &normal_outputs,
                );
            }
        }
    }
    Ok(())
}

fn material_preview_recipe(material: &MaterialRecipe) -> Recipe {
    let wave = |channel, cycles: f32, amount: f32| ScalarSource::Binary {
        op: BinaryOperator::Multiply,
        left: Box::new(ScalarSource::Unary {
            op: UnaryOperator::Sin,
            source: Box::new(ScalarSource::Binary {
                op: BinaryOperator::Multiply,
                left: Box::new(ScalarSource::Coordinate(channel)),
                right: Box::new(ScalarSource::Constant(cycles)),
            }),
        }),
        right: Box::new(ScalarSource::Constant(amount)),
    };
    let height = ScalarSource::Clamp {
        source: Box::new(ScalarSource::Binary {
            op: BinaryOperator::Add,
            left: Box::new(ScalarSource::Binary {
                op: BinaryOperator::Add,
                left: Box::new(ScalarSource::Constant(0.5)),
                right: Box::new(wave(CoordinateChannel::U, 1.0, 0.28)),
            }),
            right: Box::new(wave(CoordinateChannel::V, 2.0, 0.16)),
        }),
        min: Box::new(ScalarSource::Constant(0.0)),
        max: Box::new(ScalarSource::Constant(1.0)),
    };
    Recipe {
        name: format!("{} Preview", material.name),
        size: [128, 128],
        wrap: material.wrap,
        seed: material.seed,
        colorize: Colorize {
            source: height.clone(),
            ..Colorize::default()
        },
        output: Output { height },
        ..Recipe::default()
    }
}

fn print_rendered_outputs(
    name: &str,
    palette_description: &str,
    outputs: &[PathBuf],
    auxiliary_outputs: &[PathBuf],
) {
    println!(
        "Rendered '{}' with {} to {} frame file(s)",
        name,
        palette_description,
        outputs.len()
    );
    for output in outputs.iter().chain(auxiliary_outputs) {
        println!("{}", output.display());
    }
}

fn load_document(path: &Path) -> Result<RecipeDocument, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("could not read '{}': {error}", path.display()))?;
    parse_document(&source).map_err(|error| error.to_string())
}

fn load_referenced_material(tile_path: &Path, alias: &str) -> Result<MaterialRecipe, String> {
    let (file_alias, material_id) = alias.rsplit_once('/').ok_or_else(|| {
        format!("material alias '{alias}' must include a recipe file and declaration id")
    })?;
    let recipe_root = tile_path
        .ancestors()
        .find(|ancestor| {
            ancestor
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("recipes"))
        })
        .unwrap_or_else(|| tile_path.parent().unwrap_or_else(|| Path::new("")));
    let recipe_path = recipe_root.join(file_alias).with_extension("recipe");
    let source = fs::read_to_string(&recipe_path).map_err(|error| {
        format!(
            "could not read material recipe '{}' referenced by '{}': {error}",
            recipe_path.display(),
            tile_path.display()
        )
    })?;
    let document = parse_document(&source).map_err(|error| error.to_string())?;
    let RecipeDocument::Materials(document) = document else {
        return Err(format!(
            "'{}' is a Tile recipe, not a material recipe",
            recipe_path.display()
        ));
    };
    document
        .materials
        .into_iter()
        .find(|material| material.id.eq_ignore_ascii_case(material_id))
        .ok_or_else(|| {
            format!(
                "material recipe '{}' has no Material {} declaration",
                recipe_path.display(),
                material_id
            )
        })
}

fn load_palette(argument: &str) -> Result<(ThePalette, String), String> {
    if let Some(slug) = argument.strip_prefix("lospec:") {
        return load_lospec_palette(slug);
    }
    let path = Path::new(argument);
    if path.exists() {
        return load_hex_palette(path);
    }
    load_lospec_palette(argument)
}

fn load_lospec_palette(slug: &str) -> Result<(ThePalette, String), String> {
    let slug = slug.trim();
    if slug.is_empty()
        || !slug
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err("Lospec palette slug contains invalid characters".to_string());
    }
    let url = format!("https://lospec.com/palette-list/{slug}.json");
    let response = ureq::get(&url)
        .call()
        .map_err(|error| format!("could not download Lospec palette '{slug}': {error}"))?;
    let document: LospecPalette = serde_json::from_reader(response.into_reader())
        .map_err(|error| format!("could not decode Lospec palette '{slug}': {error}"))?;
    let colors = document
        .colors
        .iter()
        .map(|color| parse_hex_color(color))
        .collect::<Result<Vec<_>, _>>()?;
    let description = if document.author.trim().is_empty() {
        format!("Lospec palette '{}'", document.name)
    } else {
        format!("Lospec palette '{}' by {}", document.name, document.author)
    };
    Ok((palette_from_colors(colors), description))
}

fn load_hex_palette(path: &Path) -> Result<(ThePalette, String), String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("could not read palette '{}': {error}", path.display()))?;
    let colors = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with(';'))
        .map(parse_hex_color)
        .collect::<Result<Vec<_>, _>>()?;
    if colors.is_empty() {
        return Err(format!("palette '{}' contains no colors", path.display()));
    }
    Ok((
        palette_from_colors(colors),
        format!("palette '{}'", path.display()),
    ))
}

fn parse_hex_color(value: &str) -> Result<TheColor, String> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return Err(format!("palette color '{value}' must have six hex digits"));
    }
    let red = u8::from_str_radix(&hex[0..2], 16)
        .map_err(|_| format!("invalid palette color '{value}'"))?;
    let green = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|_| format!("invalid palette color '{value}'"))?;
    let blue = u8::from_str_radix(&hex[4..6], 16)
        .map_err(|_| format!("invalid palette color '{value}'"))?;
    Ok(TheColor::from_u8(red, green, blue, 255))
}

fn palette_from_colors(colors: Vec<TheColor>) -> ThePalette {
    ThePalette::new(colors.into_iter().map(Some).collect())
}

fn write_frames(
    rendered: &procedural_recipes::RenderedRecipe,
    output: &Path,
) -> Result<Vec<PathBuf>, String> {
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create '{}': {error}", parent.display()))?;
    }
    let mut outputs = Vec::with_capacity(rendered.frames.len());
    for (index, frame) in rendered.frames.iter().enumerate() {
        let frame_path = if rendered.frames.len() == 1 {
            output.to_path_buf()
        } else {
            numbered_frame_path(output, index)
        };
        image::save_buffer(
            &frame_path,
            &frame.rgba,
            rendered.width,
            rendered.height,
            image::ColorType::Rgba8,
        )
        .map_err(|error| format!("could not write '{}': {error}", frame_path.display()))?;
        outputs.push(frame_path);
    }
    Ok(outputs)
}

fn write_material_frames(
    rendered: &procedural_recipes::RenderedMaterial,
    output: &Path,
) -> Result<Vec<PathBuf>, String> {
    ensure_output_parent(output)?;
    let mut outputs = Vec::with_capacity(rendered.frames.len());
    for (index, frame) in rendered.frames.iter().enumerate() {
        let frame_path = frame_output_path(output, rendered.frames.len(), index);
        image::save_buffer(
            &frame_path,
            &frame.rgba,
            rendered.width,
            rendered.height,
            image::ColorType::Rgba8,
        )
        .map_err(|error| format!("could not write '{}': {error}", frame_path.display()))?;
        outputs.push(frame_path);
    }
    Ok(outputs)
}

fn write_material_height_frames(
    rendered: &procedural_recipes::RenderedMaterial,
    output: &Path,
) -> Result<Vec<PathBuf>, String> {
    ensure_output_parent(output)?;
    let mut outputs = Vec::with_capacity(rendered.frames.len());
    for (index, frame) in rendered.frames.iter().enumerate() {
        let frame_path = frame_output_path(output, rendered.frames.len(), index);
        image::save_buffer(
            &frame_path,
            &frame.normal_height,
            rendered.width,
            rendered.height,
            image::ColorType::L8,
        )
        .map_err(|error| format!("could not write '{}': {error}", frame_path.display()))?;
        outputs.push(frame_path);
    }
    Ok(outputs)
}

fn ensure_output_parent(output: &Path) -> Result<(), String> {
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create '{}': {error}", parent.display()))?;
    }
    Ok(())
}

fn frame_output_path(output: &Path, frame_count: usize, index: usize) -> PathBuf {
    if frame_count == 1 {
        output.to_path_buf()
    } else {
        numbered_frame_path(output, index)
    }
}

fn suffixed_output_path(path: &Path, suffix: &str) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("recipe");
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("png");
    parent.join(format!("{stem}-{suffix}.{extension}"))
}

fn numbered_frame_path(path: &Path, index: usize) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("recipe");
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("png");
    parent.join(format!("{stem}-{index:03}.{extension}"))
}

fn write_height_frames(
    rendered: &procedural_recipes::RenderedRecipe,
    output: &Path,
) -> Result<Vec<PathBuf>, String> {
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create '{}': {error}", parent.display()))?;
    }
    let mut outputs = Vec::with_capacity(rendered.frames.len());
    for (index, frame) in rendered.frames.iter().enumerate() {
        let frame_path = if rendered.frames.len() == 1 {
            output.to_path_buf()
        } else {
            numbered_frame_path(output, index)
        };
        image::save_buffer(
            &frame_path,
            &frame.height,
            rendered.width,
            rendered.height,
            image::ColorType::L8,
        )
        .map_err(|error| format!("could not write '{}': {error}", frame_path.display()))?;
        outputs.push(frame_path);
    }
    Ok(outputs)
}

fn option_value<'a>(arguments: &'a [String], name: &str) -> Option<&'a str> {
    arguments
        .windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].as_str())
}

fn print_usage() {
    println!(
        "Procedural Recipes\n\n\
         Usage:\n\
           procedural-recipes validate <recipe>\n\
           procedural-recipes render <recipe> --palette <lospec:slug|palette.hex> \
             [--material id] [--output image.png] [--height-output height.png]\n\n\
         Without --output, foo.recipe renders beside its source as foo.png.\n\
         A multi-material recipe renders foo-<id>.png for every declaration;\n\
         select one with --material <id> to render exactly foo.png.\n\n\
         Examples:\n\
           procedural-recipes render examples/bricks.recipe --palette lospec:31\n\
           procedural-recipes render examples/materials.recipe --material stone \
             --palette nice31.hex"
    );
}
