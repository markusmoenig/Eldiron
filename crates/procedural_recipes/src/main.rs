use procedural_recipes::{
    BinaryOperator, ColorSource, Colorize, CoordinateChannel, MaterialOutput, MaterialRecipe,
    Output, PaletteMode, Recipe, RecipeDocument, RecipeRenderer, RenderOptions, RenderSurface,
    RenderSurfaceFrame, RenderSurfaceMapping, ScalarSource, SdfRenderer, UnaryOperator,
    parse_document,
};
use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
    thread,
    time::Duration,
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
        print_error(&error);
        std::process::exit(1);
    }
}

fn print_error(error: &str) {
    if error.starts_with("error[") {
        eprintln!("{error}");
    } else {
        eprintln!("error: {error}");
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
    let document = load_document(&recipe_path)?;
    print_document_warnings(&document);
    match document {
        RecipeDocument::Tile(recipe) => {
            if let Some(alias) = recipe
                .material_map
                .as_ref()
                .map(|map| &map.base)
                .or(recipe.material.as_ref())
            {
                load_referenced_material(&recipe_path, alias)?;
            }
            if let Some(material_map) = &recipe.material_map {
                for layer in &material_map.layers {
                    load_referenced_material(&recipe_path, &layer.material)?;
                }
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
        RecipeDocument::Sdfs(document) => {
            println!(
                "Valid SDF recipe with {} declaration(s): {}",
                document.recipes.len(),
                document
                    .recipes
                    .iter()
                    .map(|recipe| recipe.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    Ok(())
}

fn render_command(arguments: &[String]) -> Result<(), String> {
    if arguments.iter().any(|argument| argument == "--watch") {
        return watch_render_command(arguments);
    }
    render_once(arguments)
}

fn watch_render_command(arguments: &[String]) -> Result<(), String> {
    let recipe_path = arguments
        .first()
        .map(PathBuf::from)
        .ok_or_else(|| "render requires a recipe path".to_string())?;
    let palette_arg = option_value(arguments, "--palette");
    let watch_local_palette = palette_arg.and_then(local_palette_path).is_some();
    let mut cached_palette = None;

    if let Err(error) = render_watched_once(arguments, watch_local_palette, &mut cached_palette) {
        print_error(&error);
    }

    let mut snapshot = watch_snapshot(&recipe_path, palette_arg);
    println!(
        "Watching '{}' for recipe changes; press Ctrl-C to stop",
        recipe_path.display()
    );

    loop {
        thread::sleep(Duration::from_millis(250));
        let next_snapshot = watch_snapshot(&recipe_path, palette_arg);
        if next_snapshot == snapshot {
            continue;
        }
        snapshot = next_snapshot;
        println!("Change detected; rendering...");
        if let Err(error) = render_watched_once(arguments, watch_local_palette, &mut cached_palette)
        {
            print_error(&error);
        }
    }
}

fn render_watched_once(
    arguments: &[String],
    reload_palette: bool,
    cached_palette: &mut Option<RenderPalette>,
) -> Result<(), String> {
    if reload_palette || cached_palette.is_none() {
        *cached_palette = Some(load_render_palette(arguments)?);
    }
    render_with_palette(arguments, cached_palette.as_ref().unwrap())
}

fn render_once(arguments: &[String]) -> Result<(), String> {
    let palette = load_render_palette(arguments)?;
    render_with_palette(arguments, &palette)
}

enum RenderPalette {
    Explicit {
        palette: ThePalette,
        description: String,
    },
    Grayscale,
}

impl RenderPalette {
    fn renderer(&self) -> Result<RecipeRenderer, String> {
        match self {
            Self::Explicit { palette, .. } => {
                RecipeRenderer::new(palette).map_err(|error| error.to_string())
            }
            Self::Grayscale => Ok(RecipeRenderer::grayscale()),
        }
    }

    fn description(&self) -> &str {
        match self {
            Self::Explicit { description, .. } => description,
            Self::Grayscale => "direct grayscale height output",
        }
    }

    fn is_explicit(&self) -> bool {
        matches!(self, Self::Explicit { .. })
    }

    fn material_renderer(&self, material: &MaterialRecipe) -> Result<RecipeRenderer, String> {
        match self {
            Self::Explicit { palette, .. } => {
                RecipeRenderer::new(palette).map_err(|error| error.to_string())
            }
            Self::Grayscale => {
                if material.surface.palette != PaletteMode::BaseOnly {
                    return Err(format!(
                        "material '{}' uses strict palette colorization and requires --palette <lospec:slug|file.hex>",
                        material.id
                    ));
                }
                let mut colors = Vec::new();
                for color in &material.colors {
                    collect_nearest_colors(&color.source, &mut colors);
                }
                collect_nearest_colors(&material.surface.color, &mut colors);
                if let Some(MaterialOutput::Color { source, .. }) = &material.output {
                    collect_nearest_colors(source, &mut colors);
                }
                if colors.is_empty() {
                    Ok(RecipeRenderer::grayscale())
                } else {
                    colors.sort();
                    colors.dedup();
                    let palette = ThePalette::new(
                        colors
                            .into_iter()
                            .map(|color| Some(TheColor::from_u8_array(color)))
                            .collect(),
                    );
                    RecipeRenderer::new(&palette).map_err(|error| error.to_string())
                }
            }
        }
    }
}

fn collect_nearest_colors(source: &ColorSource, colors: &mut Vec<[u8; 4]>) {
    match source {
        ColorSource::Nearest(color) => colors.push(*color),
        ColorSource::Mix { a, b, .. } => {
            collect_nearest_colors(a, colors);
            collect_nearest_colors(b, colors);
        }
        ColorSource::Exact(_) | ColorSource::Reference(_) => {}
    }
}

fn load_render_palette(arguments: &[String]) -> Result<RenderPalette, String> {
    let Some(argument) = option_value(arguments, "--palette") else {
        return Ok(RenderPalette::Grayscale);
    };
    let (palette, description) = load_palette(argument)?;
    Ok(RenderPalette::Explicit {
        palette,
        description,
    })
}

fn render_with_palette(arguments: &[String], palette: &RenderPalette) -> Result<(), String> {
    let recipe_path = arguments
        .first()
        .map(PathBuf::from)
        .ok_or_else(|| "render requires a recipe path".to_string())?;
    let output_path = option_value(arguments, "--output")
        .map(PathBuf::from)
        .unwrap_or_else(|| recipe_path.with_extension("png"));
    let height_output = option_value(arguments, "--height-output").map(PathBuf::from);
    let material_selector = option_value(arguments, "--material");

    let renderer = palette.renderer()?;
    let document = load_document(&recipe_path)?;
    print_document_warnings(&document);
    match document {
        RecipeDocument::Tile(recipe) => {
            if material_selector.is_some() {
                return Err("--material is only used when rendering a material recipe".to_string());
            }
            if recipe.colorize.is_some() && recipe.material.is_none() && !palette.is_explicit() {
                return Err(format!(
                    "tile recipe '{}' declares Colorize and requires --palette <lospec:slug|file.hex>",
                    recipe_path.display()
                ));
            }
            let mut color_description = palette.description().to_string();
            let mut rendered = renderer
                .render(&recipe, &RenderOptions::default())
                .map_err(|error| error.to_string())?;
            let base_material_alias = recipe
                .material_map
                .as_ref()
                .map(|map| map.base.as_str())
                .or(recipe.material.as_deref());
            if let Some(alias) = base_material_alias {
                let material = load_referenced_material(&recipe_path, alias)?;
                let material_renderer = palette.material_renderer(&material)?;
                let mut rendered_material = if let Some(material_map) = &recipe.material_map {
                    material_renderer.render_material_in_space(
                        &material,
                        &rendered,
                        &recipe,
                        &material_map.space,
                        material_map.tiling,
                        &RenderOptions::default(),
                    )
                } else {
                    material_renderer.render_material(
                        &material,
                        &rendered,
                        &RenderOptions::default(),
                    )
                }
                .map_err(|error| error.to_string())?;
                if let Some(material_map) = &recipe.material_map {
                    for layer in &material_map.layers {
                        let layer_material =
                            load_referenced_material(&recipe_path, &layer.material)?;
                        let layer_renderer = palette.material_renderer(&layer_material)?;
                        let layer_rendered = layer_renderer
                            .render_material_in_space(
                                &layer_material,
                                &rendered,
                                &recipe,
                                &layer.space,
                                layer.tiling,
                                &RenderOptions::default(),
                            )
                            .map_err(|error| error.to_string())?;
                        let masks = renderer
                            .render_scalar_field(
                                &recipe,
                                &layer.mask,
                                &procedural_recipes::Domain::Global,
                                &RenderOptions::default(),
                            )
                            .map_err(|error| error.to_string())?;
                        rendered_material
                            .blend_layer(&layer_rendered, &masks)
                            .map_err(|error| error.to_string())?;
                    }
                }
                for (frame, material_frame) in
                    rendered.frames.iter_mut().zip(rendered_material.frames)
                {
                    frame.rgba = material_frame.rgba;
                    frame.palette_indices = material_frame.palette_indices;
                }
                color_description = if let Some(material_map) = &recipe.material_map {
                    format!(
                        "{}-layer MaterialMap based on '{}'",
                        material_map.layers.len() + 1,
                        material.name
                    )
                } else {
                    format!("material '{}'", material.name)
                };
            }

            let outputs = write_frames(&rendered, &output_path)?;
            let height_outputs = height_output
                .as_deref()
                .map(|path| write_height_frames(&rendered, path))
                .transpose()?
                .unwrap_or_default();
            print_rendered_outputs(
                &rendered.name,
                &color_description,
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
                let material_renderer = palette.material_renderer(&material)?;
                let preview = renderer
                    .render(
                        &material_preview_recipe(&material),
                        &RenderOptions::default(),
                    )
                    .map_err(|error| error.to_string())?;
                let rendered = material_renderer
                    .render_material_preview(&material, &preview, &RenderOptions::default())
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
                    if palette.is_explicit() {
                        palette.description()
                    } else {
                        "authored material colors"
                    },
                    &outputs,
                    &normal_outputs,
                );
            }
        }
        RecipeDocument::Sdfs(document) => {
            if material_selector.is_some() {
                return Err("--material is only used when rendering a material recipe".to_string());
            }
            ensure_output_parent(&output_path)?;
            let several = document.recipes.len() > 1;
            let surface = RenderSurface {
                width: 256,
                height: 256,
                mapping: RenderSurfaceMapping::default(),
                fps: 0.0,
                looping: false,
                frames: vec![RenderSurfaceFrame { time: 0.0 }],
            };
            for recipe in document.recipes {
                let rendered =
                    SdfRenderer::render(&recipe, &surface).map_err(|error| error.to_string())?;
                let path = if several {
                    suffixed_output_path(&output_path, &recipe.id)
                } else {
                    output_path.clone()
                };
                image::save_buffer(
                    &path,
                    &rendered.coverage,
                    rendered.width,
                    rendered.height,
                    image::ColorType::L8,
                )
                .map_err(|error| format!("could not write '{}': {error}", path.display()))?;
                println!(
                    "Rendered SDF '{}' coverage to {}",
                    recipe.name,
                    path.display()
                );
            }
        }
    }
    Ok(())
}

#[derive(PartialEq, Eq)]
struct WatchSnapshot {
    files: Vec<(PathBuf, Option<Vec<u8>>)>,
}

fn watch_snapshot(recipe_path: &Path, palette_argument: Option<&str>) -> WatchSnapshot {
    let mut paths = vec![recipe_path.to_path_buf()];
    if let Some(palette_path) = palette_argument.and_then(local_palette_path) {
        paths.push(palette_path);
    }
    if let Ok(RecipeDocument::Tile(recipe)) = load_document(recipe_path) {
        if let Some(alias) = recipe
            .material_map
            .as_ref()
            .map(|map| &map.base)
            .or(recipe.material.as_ref())
            && let Ok(material_path) = referenced_material_path(recipe_path, alias)
        {
            paths.push(material_path);
        }
        if let Some(material_map) = &recipe.material_map {
            for layer in &material_map.layers {
                if let Ok(material_path) = referenced_material_path(recipe_path, &layer.material) {
                    paths.push(material_path);
                }
            }
        }
    }
    paths.sort();
    paths.dedup();

    WatchSnapshot {
        files: paths
            .into_iter()
            .map(|path| {
                let contents = fs::read(&path).ok();
                (path, contents)
            })
            .collect(),
    }
}

fn local_palette_path(argument: &str) -> Option<PathBuf> {
    if argument.starts_with("lospec:") {
        return None;
    }
    let path = PathBuf::from(argument);
    (path.exists()
        || path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("hex")))
    .then_some(path)
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
        colorize: Some(Colorize {
            source: height.clone(),
            ..Colorize::default()
        }),
        output: Output {
            height,
            ..Output::default()
        },
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
    let mut document = parse_document(&source).map_err(|error| {
        error
            .with_source_name(path.display().to_string())
            .to_string()
    })?;
    attach_warning_source_names(&mut document, path);
    Ok(document)
}

fn load_referenced_material(tile_path: &Path, alias: &str) -> Result<MaterialRecipe, String> {
    let (recipe_path, material_id) = referenced_material_target(tile_path, alias)?;
    let source = fs::read_to_string(&recipe_path).map_err(|error| {
        format!(
            "could not read material recipe '{}' referenced by '{}': {error}",
            recipe_path.display(),
            tile_path.display()
        )
    })?;
    let mut document = parse_document(&source).map_err(|error| {
        error
            .with_source_name(recipe_path.display().to_string())
            .to_string()
    })?;
    attach_warning_source_names(&mut document, &recipe_path);
    print_document_warnings(&document);
    let RecipeDocument::Materials(document) = document else {
        return Err(format!(
            "'{}' is a Tile recipe, not a material recipe",
            recipe_path.display()
        ));
    };
    match material_id {
        Some(material_id) => document
            .materials
            .into_iter()
            .find(|material| material.id.eq_ignore_ascii_case(&material_id))
            .ok_or_else(|| {
                format!(
                    "material recipe '{}' has no Material {} declaration",
                    recipe_path.display(),
                    material_id
                )
            }),
        None if document.materials.len() == 1 => Ok(document.materials.into_iter().next().unwrap()),
        None => Err(format!(
            "material recipe '{}' contains {} declarations; reference one as '{alias}/<id>'",
            recipe_path.display(),
            document.materials.len()
        )),
    }
}

fn attach_warning_source_names(document: &mut RecipeDocument, path: &Path) {
    let source_name = path.display().to_string();
    match document {
        RecipeDocument::Tile(recipe) => {
            for pattern in &mut recipe.patterns {
                for warning in &mut pattern.warnings {
                    warning.source_name = Some(source_name.clone());
                }
            }
        }
        RecipeDocument::Materials(document) => {
            for material in &mut document.materials {
                for pattern in &mut material.patterns {
                    for warning in &mut pattern.warnings {
                        warning.source_name = Some(source_name.clone());
                    }
                }
            }
        }
        RecipeDocument::Sdfs(_) => {}
    }
}

fn print_document_warnings(document: &RecipeDocument) {
    match document {
        RecipeDocument::Tile(recipe) => {
            for pattern in &recipe.patterns {
                for warning in &pattern.warnings {
                    eprintln!("{warning}");
                }
            }
        }
        RecipeDocument::Materials(document) => {
            for material in &document.materials {
                for pattern in &material.patterns {
                    for warning in &pattern.warnings {
                        eprintln!("{warning}");
                    }
                }
            }
        }
        RecipeDocument::Sdfs(_) => {}
    }
}

fn referenced_material_path(tile_path: &Path, alias: &str) -> Result<PathBuf, String> {
    referenced_material_target(tile_path, alias).map(|(path, _)| path)
}

fn referenced_material_target(
    tile_path: &Path,
    alias: &str,
) -> Result<(PathBuf, Option<String>), String> {
    let direct_path = referenced_material_path_from_file_alias(tile_path, alias);
    if direct_path.is_file() {
        return Ok((direct_path, None));
    }

    let (file_alias, material_id) = alias.rsplit_once('/').ok_or_else(|| {
        format!(
            "material alias '{alias}' does not resolve to a .recipe file and has no declaration id"
        )
    })?;
    Ok((
        referenced_material_path_from_file_alias(tile_path, file_alias),
        Some(material_id.to_string()),
    ))
}

fn referenced_material_path_from_file_alias(tile_path: &Path, file_alias: &str) -> PathBuf {
    let recipe_root = tile_path
        .ancestors()
        .find(|ancestor| {
            ancestor
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("recipes"))
        })
        .unwrap_or_else(|| tile_path.parent().unwrap_or_else(|| Path::new("")));
    recipe_root.join(file_alias).with_extension("recipe")
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
           procedural-recipes render <recipe> [--palette <lospec:slug|palette.hex>] \
             [--material id] [--output image.png] [--height-output height.png] [--watch]\n\n\
         Without --output, foo.recipe renders beside its source as foo.png.\n\
         A tile without Colorize renders as grayscale and does not need --palette.\n\
         BaseOnly materials can preview authored nearest colors without --palette;\n\
         strict palette materials and inline tile Colorize require --palette.\n\
         A material recipe renders beside its source using the same basename.\n\
         A grouped recipe renders foo-<id>.png for every declaration;\n\
         select one with --material <id> to render exactly foo.png.\n\
         --watch rerenders when the recipe, referenced material, or local palette changes.\n\n\
         Examples:\n\
           procedural-recipes render examples/bricks.recipe --watch\n\
           procedural-recipes render examples/materials/stone.recipe \
             --palette nice31.hex"
    );
}
