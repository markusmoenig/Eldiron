use procedural_recipes::{
    MaterialRecipe, RecipeDocument, RecipeRenderer, RenderOptions, WrapMode, parse_document,
};
use rusterix::{
    GeometryObject, GeometryObjectKind, Light, LightType, Map, MapCamera, PixelSource, Sector,
    Texture, Tile, TileRole, Value, ValueContainer, map::tile::TileLightEmitter,
};
use serde::Deserialize;
use shared::prelude::{Asset, AssetBuffer, Character, IndexMap, Item, Project, Region, Screen};
use std::path::{Path, PathBuf};
use std::{collections::BTreeMap, fs};
use uuid::Uuid;
use vek::{Vec2, Vec3};

#[derive(Debug, Deserialize)]
struct ProjectToml {
    #[serde(default)]
    project: ProjectSection,
    #[serde(default)]
    source: SourceSection,
    #[serde(default)]
    game: GameSection,
    #[serde(default)]
    viewport: ViewportSection,
    #[serde(default)]
    terminal: TerminalSection,
    #[serde(default)]
    build: BuildSection,
}

#[derive(Debug, Default, Deserialize)]
struct ProjectSection {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct SourceSection {
    #[serde(default = "default_main_source")]
    main: String,
    #[serde(default)]
    tile_dirs: Vec<String>,
    #[serde(default)]
    tiles: Vec<SourceTileAsset>,
    #[serde(default)]
    tile_animations: Vec<SourceTileAnimation>,
}

impl Default for SourceSection {
    fn default() -> Self {
        Self {
            main: default_main_source(),
            tile_dirs: Vec::new(),
            tiles: Vec::new(),
            tile_animations: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct SourceTileAsset {
    alias: String,
    path: String,
    #[serde(default = "default_source_tile_role")]
    role: String,
    #[serde(default)]
    blocking: bool,
    #[serde(default = "default_source_tile_scale")]
    scale: f32,
    #[serde(default)]
    light_color: String,
    #[serde(default)]
    light_intensity: f32,
    #[serde(default)]
    light_range: f32,
    #[serde(default)]
    light_flicker: f32,
    #[serde(default)]
    light_lift: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct SourceTileAnimation {
    alias: String,
    frames: Vec<String>,
    #[serde(default = "default_source_tile_role")]
    role: String,
    #[serde(default)]
    blocking: bool,
    #[serde(default = "default_source_tile_scale")]
    scale: f32,
    #[serde(default)]
    light_color: String,
    #[serde(default)]
    light_intensity: f32,
    #[serde(default)]
    light_range: f32,
    #[serde(default)]
    light_flicker: f32,
    #[serde(default)]
    light_lift: f32,
}

#[derive(Debug, Deserialize)]
struct GameSection {
    #[serde(default)]
    start_region: String,
    #[serde(default)]
    start_screen: String,
    #[serde(default = "default_client_mode")]
    client_mode: String,
    #[serde(default = "default_terminal_mode")]
    terminal_mode: String,
    #[serde(default = "default_simulation_mode")]
    simulation_mode: String,
    #[serde(default = "default_game_tick_ms")]
    game_tick_ms: u32,
    #[serde(default = "default_turn_timeout_ms")]
    turn_timeout_ms: u32,
    #[serde(default = "default_movement_units_per_sec")]
    movement_units_per_sec: u32,
    #[serde(default = "default_turn_speed_deg_per_sec")]
    turn_speed_deg_per_sec: u32,
    #[serde(default = "default_collision_mode")]
    collision_mode: String,
    #[serde(default)]
    auto_create_player: bool,
    #[serde(default = "default_player")]
    player: String,
}

impl Default for GameSection {
    fn default() -> Self {
        Self {
            start_region: String::new(),
            start_screen: String::new(),
            client_mode: default_client_mode(),
            terminal_mode: default_terminal_mode(),
            simulation_mode: default_simulation_mode(),
            game_tick_ms: default_game_tick_ms(),
            turn_timeout_ms: default_turn_timeout_ms(),
            movement_units_per_sec: default_movement_units_per_sec(),
            turn_speed_deg_per_sec: default_turn_speed_deg_per_sec(),
            collision_mode: default_collision_mode(),
            auto_create_player: true,
            player: default_player(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ViewportSection {
    #[serde(default = "default_viewport_width")]
    width: u32,
    #[serde(default = "default_viewport_height")]
    height: u32,
    #[serde(default = "default_viewport_grid_size")]
    grid_size: u32,
    #[serde(default = "default_viewport_unit")]
    unit: String,
    #[serde(default = "default_viewport_resize")]
    resize: String,
    #[serde(default)]
    cursor: String,
    #[serde(default)]
    cursor_id: String,
}

impl Default for ViewportSection {
    fn default() -> Self {
        Self {
            width: default_viewport_width(),
            height: default_viewport_height(),
            grid_size: default_viewport_grid_size(),
            unit: default_viewport_unit(),
            resize: default_viewport_resize(),
            cursor: String::new(),
            cursor_id: String::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct TerminalSection {
    #[serde(default = "default_terminal_text_updates")]
    text_updates: bool,
}

impl Default for TerminalSection {
    fn default() -> Self {
        Self {
            text_updates: default_terminal_text_updates(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct BuildSection {
    #[serde(default = "default_output")]
    output: String,
}

impl Default for BuildSection {
    fn default() -> Self {
        Self {
            output: default_output(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceCharacter {
    id: String,
    name: String,
    glyph: Option<char>,
    data: String,
    script: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceItem {
    id: String,
    name: String,
    glyph: Option<char>,
    data: String,
    script: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceTileSymbol {
    tile: String,
    material: SourceMaterial,
    blocking: Option<bool>,
    wall_feature: bool,
    ceiling: Option<String>,
}

impl SourceTileSymbol {
    fn tile_only(tile: String) -> Self {
        Self {
            tile,
            material: SourceMaterial::default(),
            blocking: None,
            wall_feature: false,
            ceiling: None,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct SourceMaterial {
    preset: Option<String>,
    finish: Option<String>,
}

impl SourceMaterial {
    fn is_empty(&self) -> bool {
        self.preset.as_deref().unwrap_or_default().trim().is_empty()
            && self.finish.as_deref().unwrap_or_default().trim().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceRegion {
    id: String,
    name: String,
    default: String,
    floor: String,
    ceiling: String,
    tile_symbols: IndexMap<char, SourceTileSymbol>,
    terrain: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceScreen {
    id: String,
    name: String,
    widgets: Vec<SourceWidget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceWidget {
    name: String,
    role: String,
    source: Option<String>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    data: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct SourceDocument {
    tile_symbols: IndexMap<char, SourceTileSymbol>,
    characters: Vec<SourceCharacter>,
    items: Vec<SourceItem>,
    regions: Vec<SourceRegion>,
    screens: Vec<SourceScreen>,
}

impl SourceDocument {
    fn extend(&mut self, other: SourceDocument) {
        self.tile_symbols.extend(other.tile_symbols);
        self.characters.extend(other.characters);
        self.items.extend(other.items);
        self.regions.extend(other.regions);
        self.screens.extend(other.screens);
    }
}

#[derive(Debug, Default)]
struct SourceTileLookup {
    aliases: IndexMap<String, Uuid>,
    leaf_aliases: IndexMap<String, Option<Uuid>>,
    light_emitters: IndexMap<Uuid, TileLightEmitter>,
    coverage: IndexMap<Uuid, [u32; 2]>,
}

#[derive(Debug, Default, Clone)]
struct ResolvedSourceTiles {
    explicit: IndexMap<char, ResolvedTileSymbol>,
    wall: Option<ResolvedTileSymbol>,
    floor: Option<ResolvedTileSymbol>,
    ceiling: Option<ResolvedTileSymbol>,
}

#[derive(Debug, Clone)]
struct ResolvedTileSymbol {
    source: PixelSource,
    coverage: [u32; 2],
    material: SourceMaterial,
    blocking: Option<bool>,
    wall_feature: bool,
    ceiling: Option<PixelSource>,
    ceiling_coverage: [u32; 2],
    light_emitter: Option<TileLightEmitter>,
}

impl ResolvedTileSymbol {
    fn tile_only(source: PixelSource) -> Self {
        Self::tile_with_coverage(source, [1, 1])
    }

    fn tile_with_coverage(source: PixelSource, coverage: [u32; 2]) -> Self {
        Self {
            source,
            coverage,
            material: SourceMaterial::default(),
            blocking: None,
            wall_feature: false,
            ceiling: None,
            ceiling_coverage: [1, 1],
            light_emitter: None,
        }
    }
}

#[derive(Debug, Clone)]
struct Spawn {
    kind: SpawnKind,
    x: usize,
    y: usize,
}

#[derive(Debug, Clone)]
enum SpawnKind {
    Player,
    Character(char),
    Item(char),
}

pub fn build_project(project_dir: &Path) -> Result<PathBuf, String> {
    let config_path = project_dir.join("eldiron.toml");
    let config_text = fs::read_to_string(&config_path)
        .map_err(|err| format!("failed to read {}: {err}", config_path.display()))?;
    let config: ProjectToml = toml::from_str(&config_text)
        .map_err(|err| format!("failed to parse {}: {err}", config_path.display()))?;
    let passthrough_config = project_config_passthrough(&config_text).map_err(|err| {
        format!(
            "failed to parse {} for runtime config: {err}",
            config_path.display()
        )
    })?;

    let source_path = project_dir.join(&config.source.main);
    let source_text = fs::read_to_string(&source_path)
        .map_err(|err| format!("failed to read {}: {err}", source_path.display()))?;
    let mut source = parse_source(&source_text)
        .map_err(|err| format!("failed to parse {}: {err}", source_path.display()))?;
    source.extend(load_source_dir(project_dir, "characters")?);
    source.extend(load_source_dir(project_dir, "items")?);
    source.extend(load_source_dir(project_dir, "regions")?);
    source.extend(load_source_dir(project_dir, "screens")?);

    let project =
        compile_project_with_project_dir(&config, source, project_dir, &passthrough_config)?;
    let output_path = project_dir.join(&config.build.output);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(&project)
        .map_err(|err| format!("failed to serialize project: {err}"))?;
    fs::write(&output_path, json)
        .map_err(|err| format!("failed to write {}: {err}", output_path.display()))?;
    Ok(output_path)
}

fn load_source_dir(project_dir: &Path, name: &str) -> Result<SourceDocument, String> {
    let dir = project_dir.join(name);
    if !dir.exists() {
        return Ok(SourceDocument::default());
    }
    if !dir.is_dir() {
        return Err(format!("{} exists but is not a directory", dir.display()));
    }

    let mut paths = fs::read_dir(&dir)
        .map_err(|err| format!("failed to read {}: {err}", dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read {}: {err}", dir.display()))?;
    paths.retain(|path| path.extension().is_some_and(|extension| extension == "els"));
    paths.sort();

    let mut document = SourceDocument::default();
    for path in paths {
        let source_text = fs::read_to_string(&path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        let parsed = parse_source(&source_text)
            .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
        document.extend(parsed);
    }
    Ok(document)
}

fn load_project_directory_assets(
    project: &mut Project,
    project_dir: &Path,
    source: &SourceSection,
) -> Result<(), String> {
    let tile_roots = source
        .tile_dirs
        .iter()
        .map(|tile_dir| tile_dir.trim())
        .filter(|tile_dir| !tile_dir.is_empty())
        .map(|tile_dir| project_dir.join(tile_dir))
        .collect::<Vec<_>>();
    let animation_frames = source
        .tile_animations
        .iter()
        .flat_map(|animation| animation.frames.iter())
        .map(|frame| project_dir.join(frame))
        .collect::<Vec<_>>();
    let declared_tile_files = source
        .tiles
        .iter()
        .map(|tile| project_dir.join(&tile.path))
        .chain(animation_frames)
        .collect::<Vec<_>>();
    load_generic_assets_dir(
        project,
        project_dir,
        "assets",
        &tile_roots,
        &declared_tile_files,
    )?;
    load_tile_image_dir(project, project_dir, "tiles")?;
    load_tile_image_dir(project, project_dir, "images")?;
    load_procedural_recipe_dir(project, project_dir, "recipes")?;
    for tile_dir in &source.tile_dirs {
        let tile_dir = tile_dir.trim();
        if !tile_dir.is_empty() {
            load_tile_image_root(project, &project_dir.join(tile_dir), TileRole::Dungeon)?;
        }
    }
    for tile in &source.tiles {
        load_source_tile_asset(project, project_dir, tile)?;
    }
    for animation in &source.tile_animations {
        load_source_tile_animation(project, project_dir, animation)?;
    }
    Ok(())
}

fn load_generic_assets_dir(
    project: &mut Project,
    project_dir: &Path,
    dir_name: &str,
    excluded_roots: &[PathBuf],
    excluded_files: &[PathBuf],
) -> Result<(), String> {
    let root = project_dir.join(dir_name);
    if !root.exists() {
        return Ok(());
    }
    if !root.is_dir() {
        return Err(format!("{} must be a directory", root.display()));
    }

    for path in collect_files_recursive(&root)? {
        if excluded_roots
            .iter()
            .any(|excluded| path.starts_with(excluded))
            || excluded_files.contains(&path)
        {
            continue;
        }
        let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        let ext = ext.to_ascii_lowercase();
        let Some(buffer) = (match ext.as_str() {
            "ttf" | "otf" => Some(AssetBuffer::Font(read_bytes(&path)?)),
            "wav" | "ogg" | "mp3" | "flac" => Some(AssetBuffer::Audio(read_bytes(&path)?)),
            "png" | "jpg" | "jpeg" => {
                let texture = Texture::from_image_safe(path.as_path())
                    .ok_or_else(|| format!("failed to decode image asset {}", path.display()))?;
                Some(AssetBuffer::Image(texture.to_rgba()))
            }
            _ => None,
        }) else {
            continue;
        };

        let mut asset = Asset::new();
        asset.name = asset_name_from_path(&root, &path);
        asset.buffer = buffer;
        project.assets.insert(asset.id, asset);
    }

    Ok(())
}

fn load_tile_image_dir(
    project: &mut Project,
    project_dir: &Path,
    dir_name: &str,
) -> Result<(), String> {
    let root = project_dir.join(dir_name);
    if !root.exists() {
        return Ok(());
    }
    load_tile_image_root(project, &root, TileRole::ManMade)
}

fn load_tile_image_root(project: &mut Project, root: &Path, role: TileRole) -> Result<(), String> {
    if !root.exists() {
        return Err(format!("tile directory {} does not exist", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("{} must be a directory", root.display()));
    }

    for path in collect_files_recursive(root)? {
        let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        if !matches!(ext.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg") {
            continue;
        }

        let texture = Texture::from_image_safe(path.as_path())
            .ok_or_else(|| format!("failed to decode tile image {}", path.display()))?;
        let mut tile = Tile::from_texture(texture);
        tile.role = role;
        tile.alias = asset_name_from_path(root, &path);
        project.tiles.insert(tile.id, tile);
    }

    Ok(())
}

fn load_procedural_recipe_dir(
    project: &mut Project,
    project_dir: &Path,
    dir_name: &str,
) -> Result<(), String> {
    let root = project_dir.join(dir_name);
    if !root.exists() {
        return Ok(());
    }
    if !root.is_dir() {
        return Err(format!("{} must be a directory", root.display()));
    }

    let mut documents = Vec::new();
    for path in collect_files_recursive(&root)? {
        let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
            continue;
        };
        if !extension.eq_ignore_ascii_case("recipe") {
            continue;
        }

        let source = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read recipe {}: {error}", path.display()))?;
        let document = parse_document(&source)
            .map_err(|error| format!("failed to parse recipe {}: {error}", path.display()))?;
        documents.push((path, document));
    }

    let mut materials = BTreeMap::<String, MaterialRecipe>::new();
    for (path, document) in &documents {
        let RecipeDocument::Materials(document) = document else {
            continue;
        };
        let file_alias = asset_name_from_path(&root, path);
        for material in &document.materials {
            let alias = format!("{file_alias}/{}", material.id).to_ascii_lowercase();
            if materials.insert(alias.clone(), material.clone()).is_some() {
                return Err(format!("duplicate procedural material alias '{alias}'"));
            }
        }
    }

    let renderer = RecipeRenderer::new(&project.art_palette).map_err(|error| {
        format!(
            "failed to create procedural recipe renderer for {}: {error}",
            root.display()
        )
    })?;
    for (path, document) in documents {
        let RecipeDocument::Tile(recipe) = document else {
            continue;
        };
        let rendered = renderer
            .render(&recipe, &RenderOptions::default())
            .map_err(|error| format!("failed to render recipe {}: {error}", path.display()))?;
        let resolved_material = recipe
            .material
            .as_ref()
            .map(|alias| {
                materials.get(&alias.to_ascii_lowercase()).ok_or_else(|| {
                    format!(
                        "recipe {} references unknown material '{}'",
                        path.display(),
                        alias
                    )
                })
            })
            .transpose()?;
        let rendered_material = resolved_material
            .map(|material| {
                renderer
                    .render_material(material, &rendered, &RenderOptions::default())
                    .map_err(|error| {
                        format!(
                            "failed to render material '{}' for recipe {}: {error}",
                            material.id,
                            path.display()
                        )
                    })
            })
            .transpose()?;
        let wrap_normals = matches!(
            resolved_material.map_or(recipe.wrap, |material| material.wrap),
            WrapMode::Repeat | WrapMode::Mirror
        );
        let mut textures = Vec::with_capacity(rendered.frames.len());
        for (frame_index, frame) in rendered.frames.iter().enumerate() {
            let material_frame = rendered_material
                .as_ref()
                .and_then(|material| material.frames.get(frame_index));
            let mut texture = Texture::new(
                material_frame
                    .map(|frame| frame.rgba.clone())
                    .unwrap_or_else(|| frame.rgba.clone()),
                rendered.width as usize,
                rendered.height as usize,
            );
            if let (Some(material), Some(material_frame)) =
                (rendered_material.as_ref(), material_frame)
            {
                for (index, [roughness, metallic, opacity, emissive]) in
                    material_frame.material.iter().copied().enumerate()
                {
                    texture.set_materials(
                        index as u32 % rendered.width,
                        index as u32 / rendered.width,
                        roughness,
                        metallic,
                        opacity,
                        emissive,
                    );
                }
                texture.generate_normals_from_height_with_strength(
                    &material_frame.normal_height,
                    wrap_normals,
                    material.normal_strength,
                );
            } else {
                texture.generate_normals_from_height(&frame.height, wrap_normals);
            }
            textures.push(texture);
        }
        let mut tile = Tile::from_textures(textures);
        tile.role = TileRole::ManMade;
        tile.alias = asset_name_from_path(&root, &path);
        tile.procedural.coverage = [rendered.grid_width, rendered.grid_height];
        if let Some(alias) = &recipe.material {
            tile.material_alias = alias.clone();
            tile.baked_material_data = tile.textures.iter().map(Texture::material_bytes).collect();
        }
        project.tiles.insert(tile.id, tile);
    }

    Ok(())
}

fn load_source_tile_asset(
    project: &mut Project,
    project_dir: &Path,
    source_tile: &SourceTileAsset,
) -> Result<(), String> {
    let alias = source_tile.alias.trim();
    if alias.is_empty() {
        return Err("[[source.tiles]] requires a non-empty alias".to_string());
    }
    let path = project_dir.join(source_tile.path.trim());
    let texture = Texture::from_image_safe(path.as_path()).ok_or_else(|| {
        format!(
            "failed to decode '{}' for source tile '{}'",
            path.display(),
            alias
        )
    })?;
    let mut tile = Tile::from_texture(texture);
    tile.alias = alias.to_string();
    tile.role = parse_source_tile_role(&source_tile.role)?;
    tile.blocking = source_tile.blocking;
    tile.scale = source_tile.scale.max(0.01);
    tile.light_emitter = source_tile_light_emitter(
        &source_tile.light_color,
        source_tile.light_intensity,
        source_tile.light_range,
        source_tile.light_flicker,
        source_tile.light_lift,
    )?;
    project.tiles.insert(tile.id, tile);
    Ok(())
}

fn load_source_tile_animation(
    project: &mut Project,
    project_dir: &Path,
    animation: &SourceTileAnimation,
) -> Result<(), String> {
    let alias = animation.alias.trim();
    if alias.is_empty() {
        return Err("[[source.tile_animations]] requires a non-empty alias".to_string());
    }
    if animation.frames.is_empty() {
        return Err(format!(
            "[[source.tile_animations]] '{}' requires at least one frame",
            alias
        ));
    }

    let mut textures = Vec::with_capacity(animation.frames.len());
    for frame in &animation.frames {
        let path = project_dir.join(frame);
        let texture = Texture::from_image_safe(path.as_path()).ok_or_else(|| {
            format!(
                "failed to decode frame '{}' for source tile animation '{}'",
                path.display(),
                alias
            )
        })?;
        textures.push(texture);
    }

    let mut tile = Tile::from_textures(textures);
    tile.alias = alias.to_string();
    tile.role = parse_source_tile_role(&animation.role)?;
    tile.blocking = animation.blocking;
    tile.scale = animation.scale.max(0.01);
    tile.light_emitter = source_tile_light_emitter(
        &animation.light_color,
        animation.light_intensity,
        animation.light_range,
        animation.light_flicker,
        animation.light_lift,
    )?;
    project.tiles.insert(tile.id, tile);
    Ok(())
}

fn source_tile_light_emitter(
    color: &str,
    intensity: f32,
    range: f32,
    flicker: f32,
    lift: f32,
) -> Result<Option<TileLightEmitter>, String> {
    if intensity <= 0.0 && range <= 0.0 {
        return Ok(None);
    }
    Ok(Some(TileLightEmitter {
        color: parse_source_light_color(color)?,
        intensity: intensity.max(0.0),
        range: range.max(0.0),
        flicker: flicker.max(0.0),
        lift,
    }))
}

fn parse_source_tile_role(role: &str) -> Result<TileRole, String> {
    match role
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-'], "_")
        .as_str()
    {
        "character" => Ok(TileRole::Character),
        "nature" => Ok(TileRole::Nature),
        "mountain" => Ok(TileRole::Mountain),
        "road" => Ok(TileRole::Road),
        "water" => Ok(TileRole::Water),
        "man_made" | "manmade" => Ok(TileRole::ManMade),
        "dungeon" => Ok(TileRole::Dungeon),
        "effect" => Ok(TileRole::Effect),
        "icon" => Ok(TileRole::Icon),
        "ui" => Ok(TileRole::UI),
        other => Err(format!(
            "unknown source tile role '{}'; expected character, nature, mountain, road, water, man_made, dungeon, effect, icon, or ui",
            other
        )),
    }
}

fn parse_source_light_color(color: &str) -> Result<[u8; 4], String> {
    let color = color.trim();
    if color.is_empty() {
        return Ok([255, 196, 96, 255]);
    }
    let hex = color.strip_prefix('#').unwrap_or(color);
    if !matches!(hex.len(), 6 | 8) {
        return Err(format!(
            "source tile animation light_color '{}' must be #RRGGBB or #RRGGBBAA",
            color
        ));
    }
    let parse = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&hex[range], 16).map_err(|_| {
            format!(
                "source tile animation light_color '{}' contains invalid hexadecimal digits",
                color
            )
        })
    };
    Ok([
        parse(0..2)?,
        parse(2..4)?,
        parse(4..6)?,
        if hex.len() == 8 { parse(6..8)? } else { 255 },
    ])
}

fn collect_files_recursive(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_files_recursive_into(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_files_recursive_into(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|err| format!("failed to read asset directory {}: {err}", dir.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|err| format!("failed to read asset directory {}: {err}", dir.display()))?
            .path();
        if path.is_dir() {
            collect_files_recursive_into(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

fn asset_name_from_path(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let without_ext = relative.with_extension("");
    without_ext
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

impl SourceTileLookup {
    fn from_project(project: &Project) -> Self {
        let mut lookup = Self::default();
        for tile in project.tiles.values() {
            lookup.coverage.insert(
                tile.id,
                [
                    tile.procedural.coverage[0].max(1),
                    tile.procedural.coverage[1].max(1),
                ],
            );
            if let Some(light) = &tile.light_emitter {
                lookup.light_emitters.insert(tile.id, light.clone());
            }
            let alias = tile.alias.trim();
            if alias.is_empty() {
                continue;
            }
            lookup.aliases.insert(alias.to_string(), tile.id);
            if let Some(leaf) = alias.rsplit('/').next()
                && !leaf.is_empty()
            {
                match lookup.leaf_aliases.get_mut(leaf) {
                    Some(existing) => {
                        if *existing != Some(tile.id) {
                            *existing = None;
                        }
                    }
                    None => {
                        lookup.leaf_aliases.insert(leaf.to_string(), Some(tile.id));
                    }
                }
            }
        }
        lookup
    }

    fn source_for(&self, name: &str) -> Option<PixelSource> {
        let name = name.trim().trim_matches('"');
        if name.is_empty() {
            return None;
        }
        if let Ok(id) = Uuid::parse_str(name) {
            return Some(PixelSource::TileId(id));
        }
        for candidate in tile_alias_candidates(name) {
            if let Some(id) = self.aliases.get(&candidate) {
                return Some(PixelSource::TileId(*id));
            }
            if let Some(Some(id)) = self.leaf_aliases.get(&candidate) {
                return Some(PixelSource::TileId(*id));
            }
        }
        None
    }

    fn tile_only_for(&self, name: &str) -> Option<ResolvedTileSymbol> {
        let source = self.source_for(name)?;
        let light_emitter = match &source {
            PixelSource::TileId(id) => self.light_emitters.get(id).cloned(),
            _ => None,
        };
        let coverage = self.coverage_for(&source);
        Some(ResolvedTileSymbol {
            source,
            coverage,
            material: SourceMaterial::default(),
            blocking: None,
            wall_feature: false,
            ceiling: None,
            ceiling_coverage: [1, 1],
            light_emitter,
        })
    }

    fn coverage_for(&self, source: &PixelSource) -> [u32; 2] {
        match source {
            PixelSource::TileId(id) => self.coverage.get(id).copied().unwrap_or([1, 1]),
            _ => [1, 1],
        }
    }
}

impl ResolvedSourceTiles {
    fn source_for_glyph(&self, glyph: char) -> Option<ResolvedTileSymbol> {
        self.explicit
            .get(&glyph)
            .cloned()
            .or_else(|| {
                self.glyph_blocks(glyph)
                    .then(|| self.wall.clone())
                    .flatten()
            })
            .or_else(|| {
                (!self.glyph_blocks(glyph))
                    .then(|| self.floor.clone())
                    .flatten()
            })
    }

    fn glyph_blocks(&self, glyph: char) -> bool {
        self.explicit
            .get(&glyph)
            .and_then(|symbol| symbol.blocking)
            .unwrap_or_else(|| source_glyph_blocks_by_default(glyph))
    }
}

fn tile_alias_candidates(name: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    push_unique_candidate(&mut candidates, name.to_string());
    push_unique_candidate(&mut candidates, name.replace('.', "/"));
    push_unique_candidate(&mut candidates, name.replace('.', "_"));
    if let Some(leaf) = name.rsplit('.').next() {
        push_unique_candidate(&mut candidates, leaf.to_string());
    }
    if let Some(leaf) = name.rsplit('/').next() {
        push_unique_candidate(&mut candidates, leaf.to_string());
    }
    candidates
}

fn push_unique_candidate(candidates: &mut Vec<String>, candidate: String) {
    if !candidate.is_empty() && !candidates.contains(&candidate) {
        candidates.push(candidate);
    }
}

fn resolve_source_tiles(
    lookup: &SourceTileLookup,
    global_tile_symbols: &IndexMap<char, SourceTileSymbol>,
    source_region: &SourceRegion,
) -> Result<ResolvedSourceTiles, String> {
    let mut symbols = global_tile_symbols.clone();
    symbols.extend(source_region.tile_symbols.clone());

    let mut explicit = IndexMap::default();
    for (glyph, symbol) in symbols {
        let Some(source) = lookup.source_for(&symbol.tile) else {
            return Err(format!(
                "Region '{}' maps '{}' to tile '{}', but no loaded tile with that alias/name exists",
                source_region.id, glyph, symbol.tile
            ));
        };
        let ceiling = match symbol.ceiling.as_deref() {
            Some(name) => Some(lookup.source_for(name).ok_or_else(|| {
                format!(
                    "Region '{}' maps '{}' to ceiling tile '{}', but no loaded tile with that alias/name exists",
                    source_region.id, glyph, name
                )
            })?),
            None => None,
        };
        let coverage = lookup.coverage_for(&source);
        let ceiling_coverage = ceiling
            .as_ref()
            .map(|source| lookup.coverage_for(source))
            .unwrap_or([1, 1]);
        explicit.insert(
            glyph,
            ResolvedTileSymbol {
                light_emitter: match &source {
                    PixelSource::TileId(id) => lookup.light_emitters.get(id).cloned(),
                    _ => None,
                },
                source,
                coverage,
                material: symbol.material,
                blocking: symbol.blocking,
                wall_feature: symbol.wall_feature,
                ceiling,
                ceiling_coverage,
            },
        );
    }

    Ok(ResolvedSourceTiles {
        explicit,
        wall: lookup
            .tile_only_for(&source_region.default)
            .or_else(|| lookup.tile_only_for("wall")),
        floor: lookup
            .tile_only_for(&source_region.floor)
            .or_else(|| lookup.tile_only_for("floor"))
            .or_else(|| lookup.tile_only_for("floor.stone")),
        ceiling: lookup
            .tile_only_for(&source_region.ceiling)
            .or_else(|| lookup.tile_only_for("ceiling"))
            .or_else(|| lookup.tile_only_for("ceiling.stone")),
    })
}

fn write_source_map_metadata(
    properties: &mut ValueContainer,
    source_region: &SourceRegion,
    source_tiles: &ResolvedSourceTiles,
) {
    properties.set(
        "eldiron_source_terrain",
        Value::Str(source_region.terrain.join("\n")),
    );
    properties.set(
        "eldiron_source_default",
        Value::Str(source_region.default.clone()),
    );
    let tile_map = source_tile_metadata(source_tiles);
    if !tile_map.is_empty() {
        properties.set("eldiron_source_tiles", Value::Str(tile_map));
    }
}

fn source_tile_metadata(source_tiles: &ResolvedSourceTiles) -> String {
    let mut lines = Vec::new();
    if let Some(ResolvedTileSymbol {
        source: PixelSource::TileId(id),
        ..
    }) = &source_tiles.wall
    {
        lines.push(format!("wall = \"{}\"", id));
    }
    if let Some(ResolvedTileSymbol {
        source: PixelSource::TileId(id),
        ..
    }) = &source_tiles.floor
    {
        lines.push(format!("floor = \"{}\"", id));
    }
    if let Some(ResolvedTileSymbol {
        source: PixelSource::TileId(id),
        ..
    }) = &source_tiles.ceiling
    {
        lines.push(format!("ceiling = \"{}\"", id));
    }
    for (glyph, tile) in &source_tiles.explicit {
        if let PixelSource::TileId(id) = &tile.source {
            lines.push(format!(
                "\"{}\" = \"{}\"",
                escape_toml_string(&glyph.to_string()),
                id
            ));
        }
    }
    lines.join("\n")
}

#[cfg(test)]
fn compile_project(config: &ProjectToml, source: SourceDocument) -> Result<Project, String> {
    compile_project_inner(config, source, None, "")
}

fn compile_project_with_project_dir(
    config: &ProjectToml,
    source: SourceDocument,
    project_dir: &Path,
    passthrough_config: &str,
) -> Result<Project, String> {
    compile_project_inner(config, source, Some(project_dir), passthrough_config)
}

fn compile_project_inner(
    config: &ProjectToml,
    source: SourceDocument,
    project_dir: Option<&Path>,
    passthrough_config: &str,
) -> Result<Project, String> {
    let mut project = Project::new();
    project.name = if config.project.name.trim().is_empty() {
        "Eldiron Source Project".to_string()
    } else {
        config.project.name.trim().to_string()
    };
    project.regions.clear();
    project.characters.clear();
    project.items.clear();
    project.screens.clear();
    project.config = project_config(
        &config.game,
        &config.viewport,
        &config.terminal,
        None,
        passthrough_config,
    );
    project.migrate_default_ruleset();
    project.authoring = "[startup]\nshow = \"room\"\n".to_string();
    project.sync_ruleset_items()?;
    if let Some(project_dir) = project_dir {
        load_project_directory_assets(&mut project, project_dir, &config.source)?;
    }
    let tile_lookup = SourceTileLookup::from_project(&project);
    project.config = project_config(
        &config.game,
        &config.viewport,
        &config.terminal,
        viewport_cursor_id(&config.viewport, &tile_lookup),
        passthrough_config,
    );
    let global_tile_symbols = source.tile_symbols.clone();

    let mut character_templates = IndexMap::default();
    let mut item_templates = IndexMap::default();
    register_ruleset_item_glyphs(&project.items, &mut item_templates);
    let player_camera = source_player_camera(&config.game);
    for source_character in source.characters {
        let mut character = Character::new();
        character.name = source_character.name;
        character.source = source_character.script;
        character.source_debug = character.source.clone();
        character.data = if source_character.data.trim().is_empty() {
            character_data(&source_character.id, &config.game.player, player_camera)
        } else {
            let camera = if source_character.id == config.game.player {
                player_camera
            } else {
                None
            };
            ensure_source_player_camera(&source_character.data, camera)
        };
        character.authoring = character_authoring(source_character.glyph);
        character.module = module_shell(
            "CharacterTemplate",
            &character.name,
            source_character.id == config.game.player,
        );
        let template_id = character.id;
        character_templates.insert(source_character.id.clone(), template_id);
        if let Some(glyph) = source_character.glyph {
            character_templates.insert(glyph.to_string(), template_id);
        }
        project.characters.insert(template_id, character);
    }

    for source_item in source.items {
        let mut item = Item::new();
        item.name = source_item.name;
        item.source = source_item.script;
        item.source_debug = item.source.clone();
        item.data = if source_item.data.trim().is_empty() {
            item_data(&source_item.id, source_item.glyph)
        } else {
            source_item.data
        };
        item.authoring = item_authoring(source_item.glyph);
        item.module = module_shell("ItemTemplate", &item.name, false);
        let template_id = item.id;
        item_templates.insert(source_item.id.clone(), template_id);
        if let Some(glyph) = source_item.glyph {
            item_templates.insert(glyph.to_string(), template_id);
        }
        project.items.insert(template_id, item);
    }

    ensure_player_template(
        &mut project,
        &mut character_templates,
        &config.game.player,
        player_camera,
    );

    for source_region in source.regions {
        project.regions.push(compile_region(
            source_region,
            &character_templates,
            &item_templates,
            &project.items,
            &config.game.player,
            player_camera,
            game_client_mode_is_3d(&config.game),
            &tile_lookup,
            &global_tile_symbols,
        )?);
    }

    for source_screen in source.screens {
        project.add_screen(compile_screen(
            source_screen,
            &config.viewport,
            &tile_lookup,
        )?);
    }

    if project.regions.is_empty() {
        return Err("source project does not define any Region blocks".to_string());
    }
    if !config.game.start_region.trim().is_empty()
        && !project
            .regions
            .iter()
            .any(|region| region.map.name == config.game.start_region)
    {
        return Err(format!(
            "[game].start_region '{}' does not match any Region",
            config.game.start_region
        ));
    }
    if !config.game.start_screen.trim().is_empty()
        && !project
            .screens
            .values()
            .any(|screen| screen.map.name == config.game.start_screen)
    {
        return Err(format!(
            "[game].start_screen '{}' does not match any Screen",
            config.game.start_screen
        ));
    }

    normalize_project_modules(&mut project);

    Ok(project)
}

fn compile_region(
    source_region: SourceRegion,
    character_templates: &IndexMap<String, Uuid>,
    item_templates: &IndexMap<String, Uuid>,
    item_template_data: &IndexMap<Uuid, Item>,
    player_id: &str,
    player_camera: Option<&str>,
    mode_3d: bool,
    tile_lookup: &SourceTileLookup,
    global_tile_symbols: &IndexMap<char, SourceTileSymbol>,
) -> Result<Region, String> {
    if source_region.terrain.is_empty() {
        return Err(format!(
            "Region '{}' is missing a terrain block",
            source_region.id
        ));
    }

    let mut region = Region::new();
    region.id = Uuid::new_v4();
    region.name = source_region.name.clone();
    region.module = module_shell("Region", &region.name, false);
    let source_tiles = resolve_source_tiles(tile_lookup, global_tile_symbols, &source_region)?;
    region.map = build_map(&source_region, mode_3d, &source_tiles)?;
    region.characters.clear();
    region.items.clear();

    let spawns = collect_spawns(&source_region.terrain);
    for spawn in spawns {
        match spawn.kind {
            SpawnKind::Player => {
                let template_id = character_templates
                    .get(player_id)
                    .copied()
                    .ok_or_else(|| format!("player template '{}' was not generated", player_id))?;
                let instance = character_instance(
                    player_id,
                    template_id,
                    spawn.x,
                    spawn.y,
                    true,
                    player_camera,
                );
                region.characters.insert(instance.id, instance);
            }
            SpawnKind::Character(glyph) => {
                let id = glyph.to_string();
                if let Some(template_id) = character_templates.get(&id).copied() {
                    let instance = character_instance(
                        &id,
                        template_id,
                        spawn.x,
                        spawn.y,
                        false,
                        player_camera,
                    );
                    region.characters.insert(instance.id, instance);
                }
            }
            SpawnKind::Item(glyph) => {
                let id = glyph.to_string();
                let item = if let Some(template_id) = item_templates.get(&id).copied() {
                    item_instance(template_id, item_template_data, spawn.x, spawn.y)
                } else {
                    default_item_instance(glyph, spawn.x, spawn.y)
                };
                region.items.insert(item.id, item);
            }
        }
    }

    Ok(region)
}

fn build_map(
    source_region: &SourceRegion,
    mode_3d: bool,
    source_tiles: &ResolvedSourceTiles,
) -> Result<Map, String> {
    let width = source_region
        .terrain
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let height = source_region.terrain.len();
    if width < 2 || height < 2 {
        return Err(format!(
            "Region '{}' terrain must be at least 2x2",
            source_region.id
        ));
    }

    let mut map = Map::default();
    map.name = source_region.id.clone();
    map.camera = if mode_3d {
        MapCamera::ThreeDFirstPerson
    } else {
        MapCamera::TwoD
    };
    map.vertices.clear();
    map.linedefs.clear();
    map.sectors.clear();
    map.geometry_objects.clear();
    map.entities.clear();
    map.items.clear();

    if mode_3d {
        write_source_map_metadata(&mut map.properties, source_region, source_tiles);
        build_3d_blocks_from_source_terrain(&mut map, source_region, source_tiles)?;
        return Ok(map);
    }

    let x0 = 0.0;
    let y0 = 0.0;
    let x1 = width as f32;
    let y1 = height as f32;
    let v0 = map.add_vertex_at(x0, y0);
    let v1 = map.add_vertex_at(x1, y0);
    let v2 = map.add_vertex_at(x1, y1);
    let v3 = map.add_vertex_at(x0, y1);
    map.create_linedef_manual(v0, v1);
    map.create_linedef_manual(v1, v2);
    map.create_linedef_manual(v2, v3);
    map.create_linedef_manual(v3, v0);
    let sector_id = map
        .close_polygon_manual()
        .ok_or_else(|| format!("failed to create sector for Region '{}'", source_region.id))?;
    if let Some(sector) = map.find_sector_mut(sector_id) {
        sector.name = source_region.name.clone();
        sector.properties.set(
            "data",
            Value::Str(format!(
                "title = \"{}\"\n",
                escape_toml_string(&source_region.name)
            )),
        );
        write_source_map_metadata(&mut sector.properties, source_region, source_tiles);
        if let Some(tile) = source_tiles.floor.clone() {
            sector.properties.set("source", Value::Source(tile.source));
        } else {
            sector
                .properties
                .set("source", Value::Source(PixelSource::Off));
        }
    }

    Ok(map)
}

fn build_3d_blocks_from_source_terrain(
    map: &mut Map,
    source_region: &SourceRegion,
    source_tiles: &ResolvedSourceTiles,
) -> Result<(), String> {
    map.vertices.clear();
    map.linedefs.clear();
    map.sectors.clear();
    map.surfaces.clear();
    map.geometry_objects.clear();
    map.lights.clear();

    let mut cells = 0usize;
    for (y, row) in source_region.terrain.iter().enumerate() {
        for (x, glyph) in row.chars().enumerate() {
            if source_tiles.glyph_blocks(glyph) {
                continue;
            }
            let glyph_tile = source_tiles.source_for_glyph(glyph);
            add_source_block(
                map,
                format!("floor_{x}_{y}"),
                x as f32,
                -0.1,
                y as f32,
                x as f32 + 1.0,
                0.0,
                y as f32 + 1.0,
                glyph_tile
                    .clone()
                    .or_else(|| source_tiles.floor.clone())
                    .unwrap_or_else(|| ResolvedTileSymbol::tile_only(PixelSource::PaletteIndex(8))),
                true,
                glyph == '@',
            );
            if glyph == '@' {
                add_source_entrance_sector(map, x as f32, y as f32)?;
            }
            add_source_block(
                map,
                format!("ceiling_{x}_{y}"),
                x as f32,
                3.0,
                y as f32,
                x as f32 + 1.0,
                3.1,
                y as f32 + 1.0,
                glyph_tile
                    .and_then(|tile| {
                        tile.ceiling.map(|source| {
                            ResolvedTileSymbol::tile_with_coverage(source, tile.ceiling_coverage)
                        })
                    })
                    .or_else(|| source_tiles.ceiling.clone())
                    .or_else(|| source_tiles.floor.clone())
                    .unwrap_or_else(|| ResolvedTileSymbol::tile_only(PixelSource::PaletteIndex(7))),
                false,
                false,
            );
            cells += 1;
        }
    }

    if cells == 0 {
        return Err(format!(
            "Region '{}' has no walkable cells for 3d mode",
            source_region.id
        ));
    }

    for (y, row) in source_region.terrain.iter().enumerate() {
        for (x, glyph) in row.chars().enumerate() {
            if !source_tiles.glyph_blocks(glyph)
                || !source_block_has_walkable_neighbor(
                    &source_region.terrain,
                    x as isize,
                    y as isize,
                    source_tiles,
                )
            {
                continue;
            }
            let symbol_tile = source_tiles
                .source_for_glyph(glyph)
                .or_else(|| source_tiles.wall.clone())
                .unwrap_or_else(|| ResolvedTileSymbol::tile_only(PixelSource::PaletteIndex(12)));
            let light_emitter = symbol_tile.light_emitter.clone();
            let wall_feature = symbol_tile.wall_feature;
            let wall_tile = if wall_feature {
                source_tiles
                    .wall
                    .clone()
                    .unwrap_or_else(|| ResolvedTileSymbol::tile_only(PixelSource::PaletteIndex(12)))
            } else {
                symbol_tile.clone()
            };
            add_source_block(
                map,
                format!("wall_{x}_{y}"),
                x as f32,
                0.0,
                y as f32,
                x as f32 + 1.0,
                3.0,
                y as f32 + 1.0,
                wall_tile,
                true,
                false,
            );
            if wall_feature {
                add_source_wall_feature(
                    map,
                    &source_region.terrain,
                    x,
                    y,
                    source_tiles,
                    symbol_tile,
                );
            }
            if let Some(light_emitter) = light_emitter {
                add_source_wall_light(
                    map,
                    &source_region.terrain,
                    x,
                    y,
                    source_tiles,
                    &light_emitter,
                );
            }
        }
    }

    Ok(())
}

fn add_source_wall_feature(
    map: &mut Map,
    terrain: &[String],
    x: usize,
    y: usize,
    source_tiles: &ResolvedSourceTiles,
    feature: ResolvedTileSymbol,
) {
    let Some((offset_x, offset_z, outward_face)) = [
        (0_isize, -1_isize, 0_usize),
        (1, 0, 3),
        (0, 1, 1),
        (-1, 0, 2),
    ]
    .into_iter()
    .find(|(dx, dz, _)| {
        !source_cell_blocks(terrain, x as isize + dx, y as isize + dz, source_tiles)
    }) else {
        return;
    };

    let inset = 0.015;
    let (min_x, max_x, min_z, max_z) = match (offset_x, offset_z) {
        (0, -1) => (x as f32, x as f32 + 1.0, y as f32 - inset, y as f32),
        (1, 0) => (
            x as f32 + 1.0,
            x as f32 + 1.0 + inset,
            y as f32,
            y as f32 + 1.0,
        ),
        (0, 1) => (
            x as f32,
            x as f32 + 1.0,
            y as f32 + 1.0,
            y as f32 + 1.0 + inset,
        ),
        (-1, 0) => (x as f32 - inset, x as f32, y as f32, y as f32 + 1.0),
        _ => return,
    };
    let mut object = GeometryObject::box_from_bounds(
        format!("wall_feature_{x}_{y}"),
        Vec3::new(min_x, 1.0, min_z),
        Vec3::new(max_x, 2.0, max_z),
    );
    object.kind = GeometryObjectKind::Generated;
    object.solid = false;
    object.group = "eldiron-source-features".to_string();
    apply_source_material(&mut object.properties, &feature.material);
    for face in &mut object.faces {
        face.tile = Some(PixelSource::Off);
    }
    if let Some(face) = object.faces.get_mut(outward_face) {
        face.auto_uv = false;
        face.uvs = vec![
            vek::Vec2::new(0.0, 1.0),
            vek::Vec2::new(1.0, 1.0),
            vek::Vec2::new(1.0, 0.0),
            vek::Vec2::new(0.0, 0.0),
        ];
        face.tile = Some(feature.source);
    }
    object.ensure_face_paint_data();
    map.geometry_objects.push(object);
}

fn add_source_wall_light(
    map: &mut Map,
    terrain: &[String],
    x: usize,
    y: usize,
    source_tiles: &ResolvedSourceTiles,
    emitter: &TileLightEmitter,
) {
    let (offset_x, offset_z) = [(0_isize, -1_isize), (1, 0), (0, 1), (-1, 0)]
        .into_iter()
        .find(|(dx, dz)| {
            !source_cell_blocks(terrain, x as isize + dx, y as isize + dz, source_tiles)
        })
        .map(|(dx, dz)| (dx as f32 * 0.55, dz as f32 * 0.55))
        .unwrap_or((0.0, 0.0));
    let color = [
        emitter.color[0] as f32 / 255.0,
        emitter.color[1] as f32 / 255.0,
        emitter.color[2] as f32 / 255.0,
    ];
    let end_distance = emitter.range.max(0.1);
    map.lights.push(
        Light::new(LightType::Point)
            .with_position(Vec3::new(
                x as f32 + 0.5 + offset_x,
                1.45 + emitter.lift,
                y as f32 + 0.5 + offset_z,
            ))
            .with_color(color)
            .with_intensity(emitter.intensity.max(0.0))
            .with_start_distance((end_distance * 0.15).min(0.75))
            .with_end_distance(end_distance)
            .with_flicker(emitter.flicker.max(0.0)),
    );
}

fn add_source_entrance_sector(map: &mut Map, x: f32, y: f32) -> Result<(), String> {
    let v0 = map.add_vertex_at(x, y);
    let v1 = map.add_vertex_at(x + 1.0, y);
    let v2 = map.add_vertex_at(x + 1.0, y + 1.0);
    let v3 = map.add_vertex_at(x, y + 1.0);
    map.possible_polygon.clear();
    let _ = map.create_linedef_manual(v0, v1);
    let _ = map.create_linedef_manual(v1, v2);
    let _ = map.create_linedef_manual(v2, v3);
    let _ = map.create_linedef_manual(v3, v0);
    let sector_id = map
        .close_polygon_manual()
        .ok_or_else(|| "failed to create source entrance marker".to_string())?;
    if let Some(sector) = map.find_sector_mut(sector_id) {
        sector.name = "entrance".to_string();
        sector.properties.set("visible", Value::Bool(false));
        sector
            .properties
            .set("procedural_kind", Value::Str("entrance".to_string()));
        sector
            .properties
            .set("source", Value::Source(PixelSource::Off));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn add_source_block(
    map: &mut Map,
    name: String,
    min_x: f32,
    min_y: f32,
    min_z: f32,
    max_x: f32,
    max_y: f32,
    max_z: f32,
    tile: ResolvedTileSymbol,
    solid: bool,
    entrance: bool,
) {
    let mut object = GeometryObject::box_from_bounds(
        name,
        Vec3::new(min_x, min_y, min_z),
        Vec3::new(max_x, max_y, max_z),
    );
    object.kind = GeometryObjectKind::Generated;
    object.solid = solid;
    object.group = "eldiron-source".to_string();
    if entrance {
        object.name = "entrance".to_string();
        object.properties.set("area", Value::Bool(true));
        object
            .properties
            .set("procedural_kind", Value::Str("entrance".to_string()));
    }
    apply_source_material(&mut object.properties, &tile.material);
    for face in &mut object.faces {
        face.tile = Some(tile.source.clone());
    }
    apply_source_recipe_uvs(&mut object, tile.coverage);
    map.geometry_objects.push(object);
}

fn apply_source_recipe_uvs(object: &mut GeometryObject, coverage: [u32; 2]) {
    let scale = Vec2::new(coverage[0].max(1) as f32, coverage[1].max(1) as f32);
    if scale == Vec2::broadcast(1.0) {
        return;
    }

    for face in &mut object.faces {
        let points = face
            .indices
            .iter()
            .filter_map(|index| object.vertices.get(*index).copied())
            .collect::<Vec<_>>();
        if points.len() != face.indices.len() || points.len() < 3 {
            continue;
        }

        let mut normal = Vec3::<f32>::zero();
        for index in 1..points.len() - 1 {
            normal += (points[index] - points[0]).cross(points[index + 1] - points[0]);
        }
        let abs = Vec3::new(normal.x.abs(), normal.y.abs(), normal.z.abs());
        face.uvs = points
            .iter()
            .map(|point| {
                let projected = if abs.y >= abs.x && abs.y >= abs.z {
                    Vec2::new(point.x, point.z)
                } else if abs.x >= abs.z {
                    Vec2::new(point.z, point.y)
                } else {
                    Vec2::new(point.x, point.y)
                };
                Vec2::new(projected.x / scale.x, projected.y / scale.y)
            })
            .collect();
        face.auto_uv = false;
    }
}

fn apply_source_material(properties: &mut ValueContainer, material: &SourceMaterial) {
    if material.is_empty() {
        return;
    }
    if let Some(preset) = material
        .preset
        .as_deref()
        .map(str::trim)
        .filter(|preset| !preset.is_empty())
    {
        properties.set("material_preset", Value::Str(preset.to_string()));
    }
    if let Some(finish) = material
        .finish
        .as_deref()
        .map(str::trim)
        .filter(|finish| !finish.is_empty())
    {
        properties.set("material_finish", Value::Str(finish.to_string()));
    }
}

fn source_block_has_walkable_neighbor(
    lines: &[String],
    x: isize,
    y: isize,
    source_tiles: &ResolvedSourceTiles,
) -> bool {
    !source_cell_blocks(lines, x, y - 1, source_tiles)
        || !source_cell_blocks(lines, x + 1, y, source_tiles)
        || !source_cell_blocks(lines, x, y + 1, source_tiles)
        || !source_cell_blocks(lines, x - 1, y, source_tiles)
}

fn source_cell_blocks(
    lines: &[String],
    x: isize,
    y: isize,
    source_tiles: &ResolvedSourceTiles,
) -> bool {
    if x < 0 || y < 0 {
        return true;
    }
    lines
        .get(y as usize)
        .and_then(|line| line.chars().nth(x as usize))
        .map(|glyph| source_tiles.glyph_blocks(glyph))
        .unwrap_or(true)
}

fn source_glyph_blocks_by_default(glyph: char) -> bool {
    glyph == '#' || glyph == ' '
}

fn compile_screen(
    source_screen: SourceScreen,
    viewport: &ViewportSection,
    tile_lookup: &SourceTileLookup,
) -> Result<Screen, String> {
    if source_screen.widgets.is_empty() {
        return Err(format!(
            "Screen '{}' does not define any widgets",
            source_screen.id
        ));
    }

    let mut screen = Screen::new();
    screen.name = source_screen.name.clone();
    screen.map = build_screen_map(&source_screen, viewport, tile_lookup)?;
    Ok(screen)
}

fn build_screen_map(
    source_screen: &SourceScreen,
    viewport: &ViewportSection,
    tile_lookup: &SourceTileLookup,
) -> Result<Map, String> {
    let mut map = Map::default();
    map.name = source_screen.id.clone();
    map.camera = MapCamera::TwoD;
    map.grid_size = viewport.grid_size.max(1) as f32;
    map.vertices.clear();
    map.linedefs.clear();
    map.sectors.clear();
    map.geometry_objects.clear();
    map.entities.clear();
    map.items.clear();

    let start_x = -(viewport.width as f32) / 2.0;
    let start_y = -(viewport.height as f32) / 2.0;
    for widget in &source_screen.widgets {
        add_screen_widget_sector(&mut map, widget, start_x, start_y, tile_lookup)?;
    }
    Ok(map)
}

fn add_screen_widget_sector(
    map: &mut Map,
    widget: &SourceWidget,
    start_x: f32,
    start_y: f32,
    tile_lookup: &SourceTileLookup,
) -> Result<(), String> {
    if widget.width <= 0 || widget.height <= 0 {
        return Err(format!(
            "Widget '{}' must have positive width and height",
            widget.name
        ));
    }

    let x0 = start_x + widget.x as f32;
    let y0 = start_y + widget.y as f32;
    let x1 = x0 + widget.width as f32;
    let y1 = y0 + widget.height as f32;
    let v0 = map.add_vertex_at(x0, y0);
    let v1 = map.add_vertex_at(x1, y0);
    let v2 = map.add_vertex_at(x1, y1);
    let v3 = map.add_vertex_at(x0, y1);
    map.possible_polygon.clear();
    let linedefs = vec![
        map.create_linedef_manual(v0, v1),
        map.create_linedef_manual(v1, v2),
        map.create_linedef_manual(v2, v3),
        map.create_linedef_manual(v3, v0),
    ];
    map.possible_polygon.clear();
    let sector_id = map
        .find_free_sector_id()
        .ok_or_else(|| format!("failed to allocate screen widget '{}'", widget.name))?;
    for linedef_id in &linedefs {
        if let Some(linedef) = map
            .linedefs
            .iter_mut()
            .find(|linedef| linedef.id == *linedef_id)
            && !linedef.sector_ids.contains(&sector_id)
        {
            linedef.sector_ids.push(sector_id);
        }
    }
    map.sectors.push(Sector::new(sector_id, linedefs));
    if let Some(sector) = map.find_sector_mut(sector_id) {
        sector.name = widget.name.clone();
        sector
            .properties
            .set("data", Value::Str(widget_data(&widget.role, &widget.data)));
        let source = if let Some(source) = widget.source.as_deref() {
            tile_lookup.source_for(source).ok_or_else(|| {
                format!(
                    "Widget '{}' references tile/source '{}', but no loaded tile with that alias/name exists",
                    widget.name, source
                )
            })?
        } else {
            PixelSource::Off
        };
        sector.properties.set("source", Value::Source(source));
    }
    Ok(())
}

fn widget_data(role: &str, data: &str) -> String {
    let data = data.trim();
    if data.is_empty() {
        return format!("[ui]\nrole = \"{}\"\n", escape_toml_string(role));
    }
    let Ok(mut table) = data.parse::<toml::Table>() else {
        return data.to_string();
    };
    let ui = table
        .entry("ui")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    if let Some(ui) = ui.as_table_mut() {
        ui.entry("role")
            .or_insert_with(|| toml::Value::String(role.to_string()));
    }
    toml::to_string(&table).unwrap_or_else(|_| data.to_string())
}

fn parse_source(src: &str) -> Result<SourceDocument, String> {
    let mut document = SourceDocument::default();
    document
        .tile_symbols
        .extend(parse_top_level_tile_symbol_blocks(src)?);
    for block in find_named_blocks(src, "Character")? {
        document.characters.push(parse_character(&block)?);
    }
    for block in find_named_blocks(src, "Item")? {
        document.items.push(parse_item(&block)?);
    }
    for block in find_named_blocks(src, "Region")? {
        document.regions.push(parse_region(&block)?);
    }
    for block in find_named_blocks(src, "Screen")? {
        document.screens.push(parse_screen(&block)?);
    }
    Ok(document)
}

fn parse_character(block: &NamedBlock) -> Result<SourceCharacter, String> {
    let name = string_field(&block.body, "name").unwrap_or_else(|| title_case_id(&block.name));
    let glyph = string_field(&block.body, "glyph").and_then(|value| value.chars().next());
    let data = brace_block(&block.body, "data")
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    let script = brace_block(&block.body, "script")
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    Ok(SourceCharacter {
        id: block.name.clone(),
        name,
        glyph,
        data,
        script,
    })
}

fn parse_item(block: &NamedBlock) -> Result<SourceItem, String> {
    let name = string_field(&block.body, "name").unwrap_or_else(|| title_case_id(&block.name));
    let glyph = string_field(&block.body, "glyph").and_then(|value| value.chars().next());
    let data = brace_block(&block.body, "data")
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    let script = brace_block(&block.body, "script")
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    Ok(SourceItem {
        id: block.name.clone(),
        name,
        glyph,
        data,
        script,
    })
}

fn parse_region(block: &NamedBlock) -> Result<SourceRegion, String> {
    let name = string_field(&block.body, "name").unwrap_or_else(|| title_case_id(&block.name));
    let default = bare_field(&block.body, "default").unwrap_or_else(|| "wall.stone".to_string());
    let floor = bare_field(&block.body, "floor").unwrap_or_else(|| "floor".to_string());
    let ceiling = bare_field(&block.body, "ceiling").unwrap_or_else(|| "ceiling".to_string());
    let tile_symbols = parse_tile_symbol_blocks(&block.body)?;
    let terrain = triple_string_field(&block.body, "terrain")
        .ok_or_else(|| format!("Region '{}' is missing terrain \"\"\"...\"\"\"", block.name))?;
    let lines = normalize_terrain_lines(&terrain);
    Ok(SourceRegion {
        id: block.name.clone(),
        name,
        default,
        floor,
        ceiling,
        tile_symbols,
        terrain: lines,
    })
}

fn parse_top_level_tile_symbol_blocks(
    src: &str,
) -> Result<IndexMap<char, SourceTileSymbol>, String> {
    let mut symbols = IndexMap::default();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut index = 0usize;
    while index < src.len() {
        if !in_string && src[index..].starts_with("\"\"\"") {
            index += 3;
            let end = src[index..]
                .find("\"\"\"")
                .ok_or_else(|| "unterminated triple-quoted string".to_string())?;
            index += end + 3;
            continue;
        }
        if !in_string
            && depth == 0
            && src[index..].starts_with("tiles")
            && is_boundary(src, index, "tiles".len())
        {
            let cursor = skip_ws(src, index + "tiles".len());
            if src[cursor..].chars().next() == Some('{') {
                let end = find_matching_brace(src, cursor).ok_or_else(|| {
                    format!("tiles block at byte {index} has an unterminated body")
                })?;
                symbols.extend(parse_tile_symbol_assignments(&src[cursor + 1..end])?);
                index = end + 1;
                continue;
            }
        }
        let Some(ch) = src[index..].chars().next() else {
            break;
        };
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
        } else {
            match ch {
                '"' => in_string = true,
                '{' => depth += 1,
                '}' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        index += ch.len_utf8();
    }
    Ok(symbols)
}

fn parse_tile_symbol_blocks(src: &str) -> Result<IndexMap<char, SourceTileSymbol>, String> {
    let mut symbols = IndexMap::default();
    let mut search_from = 0;
    while let Some(relative) = src[search_from..].find("tiles") {
        let pos = search_from + relative;
        if !is_boundary(src, pos, "tiles".len()) {
            search_from = pos + "tiles".len();
            continue;
        }
        let cursor = skip_ws(src, pos + "tiles".len());
        if src[cursor..].chars().next() != Some('{') {
            search_from = cursor;
            continue;
        }
        let end = find_matching_brace(src, cursor)
            .ok_or_else(|| format!("tiles block at byte {pos} has an unterminated body"))?;
        symbols.extend(parse_tile_symbol_assignments(&src[cursor + 1..end])?);
        search_from = end + 1;
    }
    Ok(symbols)
}

fn parse_tile_symbol_assignments(src: &str) -> Result<IndexMap<char, SourceTileSymbol>, String> {
    let mut symbols = IndexMap::default();
    for line in src.lines() {
        let line = strip_line_comment(line).trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("invalid tiles entry '{}'", line));
        };
        let key = key.trim().trim_matches('"');
        let mut chars = key.chars();
        let Some(glyph) = chars.next() else {
            return Err("tiles entry has an empty glyph".to_string());
        };
        if chars.next().is_some() {
            return Err(format!(
                "tiles entry '{}' must use a single-character glyph",
                key
            ));
        }
        let value = parse_tile_symbol_value(value)?;
        if value.tile.is_empty() {
            return Err(format!("tiles entry '{}' has an empty tile name", key));
        }
        symbols.insert(glyph, value);
    }
    Ok(symbols)
}

fn parse_tile_symbol_value(value: &str) -> Result<SourceTileSymbol, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(SourceTileSymbol::tile_only(String::new()));
    }
    if value.starts_with('{') {
        return parse_tile_symbol_inline_table(value);
    }

    let mut tile = String::new();
    let mut material = SourceMaterial::default();
    let mut blocking = None;
    let mut wall_feature = false;
    let mut ceiling = None;
    for token in value.replace(',', " ").split_whitespace() {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if let Some((key, raw_value)) = token.split_once('=') {
            let key = key.trim();
            let raw_value = clean_tile_symbol_atom(raw_value);
            match key {
                "material" | "preset" | "material_preset" => material.preset = Some(raw_value),
                "finish" | "material_finish" => material.finish = Some(raw_value),
                "tile" | "source" => tile = raw_value,
                "blocking" | "solid" => blocking = Some(parse_tile_symbol_bool(key, &raw_value)?),
                "feature" | "wall_feature" => {
                    wall_feature = parse_tile_symbol_bool(key, &raw_value)?
                }
                "ceiling" => ceiling = Some(raw_value),
                _ => {
                    return Err(format!(
                        "unknown tiles entry option '{}'; expected tile, material/preset, finish, blocking/solid, feature/wall_feature, or ceiling",
                        key
                    ));
                }
            }
        } else if tile.is_empty() {
            tile = clean_tile_symbol_atom(token);
        } else {
            return Err(format!(
                "unexpected tiles entry token '{}'; use key=value options after the tile name",
                token
            ));
        }
    }

    Ok(SourceTileSymbol {
        tile,
        material,
        blocking,
        wall_feature,
        ceiling,
    })
}

fn parse_tile_symbol_inline_table(value: &str) -> Result<SourceTileSymbol, String> {
    let table: toml::Table = format!("symbol = {value}")
        .parse()
        .map_err(|err| format!("invalid tiles inline table '{}': {err}", value))?;
    let Some(toml::Value::Table(symbol)) = table.get("symbol") else {
        return Err(format!("invalid tiles inline table '{}'", value));
    };
    let tile = inline_string_value(symbol, "tile")
        .or_else(|| inline_string_value(symbol, "source"))
        .ok_or_else(|| format!("tiles inline table '{}' is missing tile/source", value))?;
    let material = SourceMaterial {
        preset: inline_string_value(symbol, "material")
            .or_else(|| inline_string_value(symbol, "preset"))
            .or_else(|| inline_string_value(symbol, "material_preset")),
        finish: inline_string_value(symbol, "finish")
            .or_else(|| inline_string_value(symbol, "material_finish")),
    };
    let blocking = inline_bool_value(symbol, "blocking")
        .or_else(|| inline_bool_value(symbol, "solid"))
        .transpose()?;
    let wall_feature = inline_bool_value(symbol, "wall_feature")
        .or_else(|| inline_bool_value(symbol, "feature"))
        .transpose()?
        .unwrap_or(false);
    let ceiling = inline_string_value(symbol, "ceiling");
    Ok(SourceTileSymbol {
        tile,
        material,
        blocking,
        wall_feature,
        ceiling,
    })
}

fn parse_tile_symbol_bool(key: &str, value: &str) -> Result<bool, String> {
    value.parse::<bool>().map_err(|_| {
        format!(
            "tiles entry option '{}={}' must use true or false",
            key, value
        )
    })
}

fn inline_bool_value(table: &toml::Table, key: &str) -> Option<Result<bool, String>> {
    table.get(key).map(|value| match value {
        toml::Value::Boolean(value) => Ok(*value),
        toml::Value::String(value) => parse_tile_symbol_bool(key, value),
        _ => Err(format!(
            "tiles inline option '{}' must use true or false",
            key
        )),
    })
}

fn inline_string_value(table: &toml::Table, key: &str) -> Option<String> {
    table.get(key).and_then(|value| match value {
        toml::Value::String(value) => Some(value.trim().to_string()),
        toml::Value::Integer(value) => Some(value.to_string()),
        toml::Value::Float(value) => Some(value.to_string()),
        toml::Value::Boolean(value) => Some(value.to_string()),
        _ => None,
    })
}

fn clean_tile_symbol_atom(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn strip_line_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in line.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
        } else if ch == '"' {
            in_string = true;
        } else if ch == '#' {
            return &line[..index];
        }
    }
    line
}

fn parse_screen(block: &NamedBlock) -> Result<SourceScreen, String> {
    let name = string_field(&block.body, "name").unwrap_or_else(|| title_case_id(&block.name));
    let mut widgets = Vec::new();
    for widget in find_named_blocks(&block.body, "widget")? {
        widgets.push(parse_widget(&widget)?);
    }
    for widget in find_named_blocks(&block.body, "Widget")? {
        widgets.push(parse_widget(&widget)?);
    }
    Ok(SourceScreen {
        id: block.name.clone(),
        name,
        widgets,
    })
}

fn parse_widget(block: &NamedBlock) -> Result<SourceWidget, String> {
    let role = string_field(&block.body, "role").unwrap_or_else(|| "none".to_string());
    let source = string_field(&block.body, "source")
        .or_else(|| string_field(&block.body, "tile"))
        .or_else(|| bare_field(&block.body, "source"))
        .or_else(|| bare_field(&block.body, "tile"))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let x = int_field(&block.body, "x").unwrap_or(0);
    let y = int_field(&block.body, "y").unwrap_or(0);
    let width = int_field(&block.body, "width")
        .or_else(|| int_field(&block.body, "w"))
        .ok_or_else(|| format!("widget '{}' is missing width", block.name))?;
    let height = int_field(&block.body, "height")
        .or_else(|| int_field(&block.body, "h"))
        .ok_or_else(|| format!("widget '{}' is missing height", block.name))?;
    let data = brace_block(&block.body, "data")
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    Ok(SourceWidget {
        name: block.name.clone(),
        role,
        source,
        x,
        y,
        width,
        height,
        data,
    })
}

#[derive(Debug)]
struct NamedBlock {
    name: String,
    body: String,
}

fn find_named_blocks(src: &str, keyword: &str) -> Result<Vec<NamedBlock>, String> {
    let mut blocks = Vec::new();
    let mut index = 0;
    while let Some(relative) = src[index..].find(keyword) {
        let start = index + relative;
        if !is_boundary(src, start, keyword.len()) {
            index = start + keyword.len();
            continue;
        }
        let mut cursor = start + keyword.len();
        cursor = skip_ws(src, cursor);
        let (name, after_name) = parse_quoted(src, cursor)
            .ok_or_else(|| format!("{keyword} block at byte {start} is missing a quoted name"))?;
        cursor = skip_ws(src, after_name);
        if src[cursor..].chars().next() != Some('{') {
            return Err(format!("{keyword} \"{name}\" is missing '{{'"));
        }
        let end = find_matching_brace(src, cursor)
            .ok_or_else(|| format!("{keyword} \"{name}\" has an unterminated body"))?;
        blocks.push(NamedBlock {
            name,
            body: src[cursor + 1..end].to_string(),
        });
        index = end + 1;
    }
    Ok(blocks)
}

fn is_boundary(src: &str, start: usize, len: usize) -> bool {
    let before = src[..start].chars().next_back();
    let after = src[start + len..].chars().next();
    !before.is_some_and(is_ident_char) && !after.is_some_and(is_ident_char)
}

fn is_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn skip_ws(src: &str, mut index: usize) -> usize {
    while let Some(ch) = src[index..].chars().next() {
        if !ch.is_whitespace() {
            break;
        }
        index += ch.len_utf8();
    }
    index
}

fn parse_quoted(src: &str, start: usize) -> Option<(String, usize)> {
    if src[start..].chars().next()? != '"' {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    let mut index = start + 1;
    for ch in src[index..].chars() {
        index += ch.len_utf8();
        if escaped {
            out.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some((out, index));
        } else {
            out.push(ch);
        }
    }
    None
}

fn find_matching_brace(src: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut index = open;
    while index < src.len() {
        if !in_string && src[index..].starts_with("\"\"\"") {
            index += 3;
            let end = src[index..].find("\"\"\"")?;
            index += end + 3;
            continue;
        }
        let ch = src[index..].chars().next()?;
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            index += ch.len_utf8();
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += ch.len_utf8();
    }
    None
}

fn string_field(src: &str, key: &str) -> Option<String> {
    let pos = find_field_pos(src, key)?;
    let mut cursor = skip_ws(src, pos + key.len());
    if src[cursor..].chars().next()? != '=' {
        return None;
    }
    cursor = skip_ws(src, cursor + 1);
    parse_quoted(src, cursor).map(|(value, _)| value)
}

fn bare_field(src: &str, key: &str) -> Option<String> {
    let pos = find_field_pos(src, key)?;
    let mut cursor = skip_ws(src, pos + key.len());
    if src[cursor..].chars().next()? != '=' {
        return None;
    }
    cursor = skip_ws(src, cursor + 1);
    let rest = &src[cursor..];
    let end = rest
        .find(|ch: char| ch == '\n' || ch == '\r' || ch == '}')
        .unwrap_or(rest.len());
    Some(rest[..end].trim().trim_matches('"').to_string())
}

fn int_field(src: &str, key: &str) -> Option<i32> {
    bare_field(src, key)?.parse().ok()
}

fn triple_string_field(src: &str, key: &str) -> Option<String> {
    let pos = find_field_pos(src, key)?;
    let mut cursor = skip_ws(src, pos + key.len());
    let at_pos = src[cursor..].starts_with("at ");
    if at_pos {
        let rest = &src[cursor..];
        cursor += rest.find("\"\"\"")?;
    }
    if !src[cursor..].starts_with("\"\"\"") {
        cursor = skip_ws(src, cursor);
        if !src[cursor..].starts_with("\"\"\"") {
            return None;
        }
    }
    let content_start = cursor + 3;
    let content_end = src[content_start..].find("\"\"\"")? + content_start;
    Some(
        src[content_start..content_end]
            .trim_matches('\n')
            .to_string(),
    )
}

fn brace_block<'a>(src: &'a str, key: &str) -> Option<&'a str> {
    let pos = find_field_pos(src, key)?;
    let cursor = skip_ws(src, pos + key.len());
    if src[cursor..].chars().next()? != '{' {
        return None;
    }
    let end = find_matching_brace(src, cursor)?;
    Some(&src[cursor + 1..end])
}

fn find_field_pos(src: &str, key: &str) -> Option<usize> {
    let mut search_from = 0;
    while let Some(relative) = src[search_from..].find(key) {
        let pos = search_from + relative;
        if is_boundary(src, pos, key.len()) {
            return Some(pos);
        }
        search_from = pos + key.len();
    }
    None
}

fn normalize_terrain_lines(src: &str) -> Vec<String> {
    let lines: Vec<&str> = src
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .collect();
    let indent = lines
        .iter()
        .map(|line| line.chars().take_while(|ch| ch.is_whitespace()).count())
        .min()
        .unwrap_or(0);
    lines
        .into_iter()
        .map(|line| line.chars().skip(indent).collect())
        .collect()
}

fn collect_spawns(lines: &[String]) -> Vec<Spawn> {
    let mut spawns = Vec::new();
    for (y, line) in lines.iter().enumerate() {
        for (x, ch) in line.chars().enumerate() {
            let kind = if ch == '@' {
                Some(SpawnKind::Player)
            } else if ch.is_ascii_uppercase() {
                Some(SpawnKind::Character(ch))
            } else if ch.is_ascii_lowercase() {
                Some(SpawnKind::Item(ch))
            } else {
                None
            };
            if let Some(kind) = kind {
                spawns.push(Spawn { kind, x, y });
            }
        }
    }
    spawns
}

fn ensure_player_template(
    project: &mut Project,
    character_templates: &mut IndexMap<String, Uuid>,
    player_id: &str,
    player_camera: Option<&str>,
) {
    if character_templates.contains_key(player_id) {
        return;
    }
    let mut character = Character::new();
    character.name = title_case_id(player_id);
    character.data = character_data(player_id, player_id, player_camera);
    character.authoring = character_authoring(Some('@'));
    character.module = module_shell("CharacterTemplate", &character.name, true);
    let template_id = character.id;
    character_templates.insert(player_id.to_string(), template_id);
    project.characters.insert(template_id, character);
}

fn character_instance(
    id: &str,
    template_id: Uuid,
    x: usize,
    y: usize,
    player: bool,
    player_camera: Option<&str>,
) -> Character {
    let mut character = Character::new();
    character.name = title_case_id(id);
    character.position = grid_position(x, y);
    character.character_id = template_id;
    character.data = character_data(
        id,
        if player { id } else { "" },
        player.then_some(player_camera).flatten(),
    );
    character.module = module_shell("CharacterTemplate", &character.name, player);
    character
}

fn item_instance(
    template_id: Uuid,
    item_template_data: &IndexMap<Uuid, Item>,
    x: usize,
    y: usize,
) -> Item {
    let mut item = Item::new();
    if let Some(template) = item_template_data.get(&template_id) {
        item.name = template.name.clone();
        item.data = template.data.clone();
        item.source = template.source.clone();
        item.source_debug = template.source_debug.clone();
        item.authoring = template.authoring.clone();
        item.module = template.module.clone();
    }
    item.position = grid_position(x, y);
    item.item_id = template_id;
    if item.module.is_null() {
        item.module = module_shell("ItemTemplate", &item.name, false);
    }
    item
}

fn default_item_instance(glyph: char, x: usize, y: usize) -> Item {
    let mut item = Item::new();
    item.name = default_item_name(glyph);
    item.position = grid_position(x, y);
    item.data = item_data(&glyph.to_string(), Some(glyph));
    item.authoring = item_authoring(Some(glyph));
    item.module = module_shell("ItemTemplate", &item.name, false);
    item
}

fn module_shell(module_type: &str, name: &str, player: bool) -> serde_json::Value {
    serde_json::json!({
        "filter_text": "",
        "id": Uuid::nil().to_string(),
        "module_type": module_type,
        "name": name,
        "player": player,
        "routines": {},
        "view_name": "",
    })
}

fn normalize_project_modules(project: &mut Project) {
    if project.world_module.is_null() {
        project.world_module = module_shell("World", "", false);
    }

    for item in project.items.values_mut() {
        if item.module.is_null() {
            item.module = module_shell("ItemTemplate", &item.name, false);
        }
    }
    for character in project.characters.values_mut() {
        if character.module.is_null() {
            character.module = module_shell(
                "CharacterTemplate",
                &character.name,
                character
                    .data
                    .parse::<toml::Table>()
                    .ok()
                    .and_then(|data| {
                        data.get("attributes")
                            .and_then(toml::Value::as_table)
                            .cloned()
                    })
                    .and_then(|attributes| attributes.get("player").and_then(toml::Value::as_bool))
                    .unwrap_or(false),
            );
        }
    }

    for region in &mut project.regions {
        if region.module.is_null() {
            region.module = module_shell("Region", &region.name, false);
        }
        for item in region.items.values_mut() {
            if item.module.is_null() {
                item.module = module_shell("ItemTemplate", &item.name, false);
            }
        }
        for character in region.characters.values_mut() {
            if character.module.is_null() {
                character.module = module_shell(
                    "CharacterTemplate",
                    &character.name,
                    character.data.contains("player = true"),
                );
            }
        }
    }
}

fn register_ruleset_item_glyphs(
    items: &IndexMap<Uuid, Item>,
    item_templates: &mut IndexMap<String, Uuid>,
) {
    for (glyph, ruleset_id) in [('h', "wild_herb"), ('b', "blessed_herb")] {
        if let Some(template_id) = ruleset_item_template_id(items, ruleset_id) {
            item_templates.insert(glyph.to_string(), template_id);
        }
    }
}

fn ruleset_item_template_id(items: &IndexMap<Uuid, Item>, ruleset_id: &str) -> Option<Uuid> {
    items.iter().find_map(|(id, item)| {
        item.data
            .parse::<toml::Table>()
            .ok()
            .and_then(|data| {
                data.get("attributes")
                    .and_then(toml::Value::as_table)
                    .cloned()
            })
            .and_then(|attributes| {
                let id_matches = attributes
                    .get("ruleset_id")
                    .and_then(toml::Value::as_str)
                    .is_some_and(|value| value == ruleset_id);
                id_matches.then_some(*id)
            })
    })
}

fn grid_position(x: usize, y: usize) -> Vec3<f32> {
    Vec3::new(x as f32 + 0.5, 1.0, y as f32 + 0.5)
}

fn viewport_cursor_id(viewport: &ViewportSection, lookup: &SourceTileLookup) -> Option<Uuid> {
    let explicit = viewport.cursor_id.trim();
    if let Ok(id) = Uuid::parse_str(explicit) {
        return Some(id);
    }
    if let Some(PixelSource::TileId(id)) = lookup.source_for(explicit) {
        return Some(id);
    }
    if let Some(PixelSource::TileId(id)) = lookup.source_for(&viewport.cursor) {
        return Some(id);
    }
    if let Some(PixelSource::TileId(id)) = lookup.source_for("cursor") {
        return Some(id);
    }
    None
}

fn project_config(
    game: &GameSection,
    viewport: &ViewportSection,
    terminal: &TerminalSection,
    cursor_id: Option<Uuid>,
    passthrough_config: &str,
) -> String {
    let cursor_line = cursor_id
        .map(|id| format!("cursor_id = \"{}\"\n", id))
        .unwrap_or_default();
    let mut config = format!(
        "[game]\nstart_region = \"{}\"\nstart_screen = \"{}\"\nclient_mode = \"{}\"\nterminal_mode = \"{}\"\nsimulation_mode = \"{}\"\ngame_tick_ms = {}\nturn_timeout_ms = {}\nmovement_units_per_sec = {}\nturn_speed_deg_per_sec = {}\nauto_create_player = {}\ncollision_mode = \"{}\"\n\n[viewport]\nwidth = {}\nheight = {}\ngrid_size = {}\nunit = \"{}\"\nresize = \"{}\"\n{}\n[terminal]\ntext_updates = {}\n",
        escape_toml_string(&game.start_region),
        escape_toml_string(&game.start_screen),
        escape_toml_string(&game.client_mode),
        escape_toml_string(&game.terminal_mode),
        escape_toml_string(&game.simulation_mode),
        game.game_tick_ms,
        game.turn_timeout_ms,
        game.movement_units_per_sec,
        game.turn_speed_deg_per_sec,
        game.auto_create_player,
        escape_toml_string(&game.collision_mode),
        viewport.width,
        viewport.height,
        viewport.grid_size,
        escape_toml_string(&viewport.unit),
        escape_toml_string(&viewport.resize),
        cursor_line,
        terminal.text_updates
    );
    let passthrough_config = passthrough_config.trim();
    if !passthrough_config.is_empty() {
        config.push('\n');
        config.push_str(passthrough_config);
        config.push('\n');
    }
    config
}

fn project_config_passthrough(config_text: &str) -> Result<String, String> {
    const SOURCE_OWNED_SECTIONS: &[&str] =
        &["project", "source", "game", "viewport", "terminal", "build"];

    let mut config: toml::Table = toml::from_str(config_text).map_err(|err| err.to_string())?;
    for section in SOURCE_OWNED_SECTIONS {
        config.remove(*section);
    }
    if config.is_empty() {
        Ok(String::new())
    } else {
        toml::to_string(&config).map_err(|err| err.to_string())
    }
}

fn character_data(id: &str, player_id: &str, player_camera: Option<&str>) -> String {
    let is_player = !player_id.is_empty() && id == player_id;
    let uses_3d_player_camera = player_camera.is_some();
    let mut data = format!(
        "[attributes]\nplayer = {}\nsource_id = \"{}\"\n",
        is_player,
        escape_toml_string(id)
    );
    if let Some(player_camera) = player_camera
        && is_player
    {
        data.push_str(&format!(
            "player_camera = \"{}\"\n",
            escape_toml_string(player_camera)
        ));
    }
    if is_player && uses_3d_player_camera {
        data.push_str("radius = 0.35\n");
    }
    if is_player {
        data.push_str(
            "\n[input]\nw = \"control.forward\"\na = \"control.left\"\ns = \"control.backward\"\nd = \"control.right\"\nup = \"control.forward\"\nleft = \"control.left\"\ndown = \"control.backward\"\nright = \"control.right\"\n",
        );
        data.push_str("g = \"intent(take)\"\n");
    }
    data
}

fn ensure_source_player_camera(data: &str, player_camera: Option<&str>) -> String {
    let Some(player_camera) = player_camera else {
        return data.to_string();
    };
    let Ok(mut table) = data.parse::<toml::Table>() else {
        return data.to_string();
    };
    let attributes = table
        .entry("attributes".to_string())
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    let Some(attributes) = attributes.as_table_mut() else {
        return data.to_string();
    };
    attributes
        .entry("player_camera".to_string())
        .or_insert_with(|| toml::Value::String(player_camera.to_string()));
    attributes
        .entry("radius".to_string())
        .or_insert_with(|| toml::Value::Float(0.35));
    toml::to_string(&table).unwrap_or_else(|_| data.to_string())
}

fn game_client_mode_is_3d(game: &GameSection) -> bool {
    matches!(
        game.client_mode.trim().to_ascii_lowercase().as_str(),
        "3d" | "firstp" | "firstp_grid" | "dungeon3d" | "dungeon_3d"
    )
}

fn source_player_camera(game: &GameSection) -> Option<&'static str> {
    game_client_mode_is_3d(game).then_some("firstp_grid")
}

fn character_authoring(glyph: Option<char>) -> String {
    match glyph {
        Some(glyph) => format!(
            "[terminal]\nglyph = \"{}\"\n",
            escape_toml_string(&glyph.to_string())
        ),
        None => String::new(),
    }
}

fn item_data(id: &str, glyph: Option<char>) -> String {
    let glyph = glyph
        .map(|glyph| glyph.to_string())
        .unwrap_or_else(|| id.to_string());
    format!(
        "[attributes]\nsource_id = \"{}\"\nsource_symbol = \"{}\"\nterminal_glyph = \"{}\"\non_take = \"take\"\n",
        escape_toml_string(id),
        escape_toml_string(&glyph),
        escape_toml_string(&glyph)
    )
}

fn item_authoring(glyph: Option<char>) -> String {
    match glyph {
        Some(glyph) => format!(
            "[terminal]\nglyph = \"{}\"\n",
            escape_toml_string(&glyph.to_string())
        ),
        None => String::new(),
    }
}

fn default_item_name(glyph: char) -> String {
    match glyph {
        'h' => "Herb".to_string(),
        'b' => "Blessed Herb".to_string(),
        _ => format!("Item {glyph}"),
    }
}

fn title_case_id(id: &str) -> String {
    id.split(['_', '-', '.'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn escape_toml_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn default_main_source() -> String {
    "main.els".to_string()
}

fn default_source_tile_role() -> String {
    "dungeon".to_string()
}

fn default_source_tile_scale() -> f32 {
    1.0
}

fn default_player() -> String {
    "player".to_string()
}

fn default_client_mode() -> String {
    "terminal".to_string()
}

fn default_terminal_mode() -> String {
    "roguelike".to_string()
}

fn default_terminal_text_updates() -> bool {
    true
}

fn default_simulation_mode() -> String {
    "hybrid".to_string()
}

fn default_game_tick_ms() -> u32 {
    250
}

fn default_turn_timeout_ms() -> u32 {
    600
}

fn default_movement_units_per_sec() -> u32 {
    4
}

fn default_turn_speed_deg_per_sec() -> u32 {
    120
}

fn default_collision_mode() -> String {
    "tile".to_string()
}

fn default_viewport_width() -> u32 {
    80
}

fn default_viewport_height() -> u32 {
    24
}

fn default_viewport_grid_size() -> u32 {
    40
}

fn default_viewport_unit() -> String {
    "cell".to_string()
}

fn default_viewport_resize() -> String {
    "fit".to_string()
}

fn default_output() -> String {
    "build/game.eldiron".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusterix::ChunkBuilder;
    use vek::Vec2;

    #[test]
    fn parses_minimal_source() {
        let source = r##"
Character "player" {
  name = "Player"
  glyph = "@"
  script {
    on_interact {
      say("hello");
    }
  }
}

Item "herb" {
  name = "Herb"
  glyph = "h"
}

Region "cellar" {
  default = wall.stone
  floor = floor.cracked
  ceiling = ceiling.vault
  tiles {
    "#" = wall material=stone finish=matte blocking=true wall_feature=true ceiling=ceiling.vault
    "." = floor
  }
  terrain """
  ####
  #@.#
  ####
  """
}

Screen "play" {
  widget "Game" {
    role = "game"
    width = 80
    height = 20
  }
}
"##;
        let parsed = parse_source(source).expect("source parses");
        assert_eq!(parsed.characters.len(), 1);
        assert_eq!(parsed.characters[0].id, "player");
        assert!(parsed.characters[0].script.contains("on_interact"));
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(parsed.items[0].id, "herb");
        assert_eq!(
            parsed.regions[0]
                .tile_symbols
                .get(&'#')
                .map(|symbol| symbol.tile.as_str()),
            Some("wall")
        );
        assert_eq!(
            parsed.regions[0]
                .tile_symbols
                .get(&'#')
                .and_then(|symbol| symbol.material.preset.as_deref()),
            Some("stone")
        );
        assert_eq!(
            parsed.regions[0]
                .tile_symbols
                .get(&'#')
                .and_then(|symbol| symbol.material.finish.as_deref()),
            Some("matte")
        );
        assert_eq!(
            parsed.regions[0]
                .tile_symbols
                .get(&'#')
                .and_then(|symbol| symbol.blocking),
            Some(true)
        );
        assert!(
            parsed.regions[0]
                .tile_symbols
                .get(&'#')
                .is_some_and(|symbol| symbol.wall_feature)
        );
        assert_eq!(
            parsed.regions[0]
                .tile_symbols
                .get(&'#')
                .and_then(|symbol| symbol.ceiling.as_deref()),
            Some("ceiling.vault")
        );
        assert_eq!(parsed.regions[0].floor, "floor.cracked");
        assert_eq!(parsed.regions[0].ceiling, "ceiling.vault");
        assert_eq!(
            parsed.regions[0]
                .tile_symbols
                .get(&'.')
                .map(|symbol| symbol.tile.as_str()),
            Some("floor")
        );
        assert_eq!(parsed.regions.len(), 1);
        assert_eq!(parsed.regions[0].terrain.len(), 3);
        assert_eq!(parsed.screens.len(), 1);
        assert_eq!(parsed.screens[0].widgets.len(), 1);
        assert_eq!(parsed.screens[0].widgets[0].role, "game");
    }

    #[test]
    fn explicit_tile_blocking_controls_source_terrain_geometry() {
        let mut tiles = ResolvedSourceTiles::default();
        tiles.explicit.insert(
            '%',
            ResolvedTileSymbol {
                source: PixelSource::PaletteIndex(1),
                coverage: [1, 1],
                material: SourceMaterial::default(),
                blocking: Some(true),
                wall_feature: false,
                ceiling: None,
                ceiling_coverage: [1, 1],
                light_emitter: None,
            },
        );
        tiles.explicit.insert(
            '-',
            ResolvedTileSymbol {
                source: PixelSource::PaletteIndex(2),
                coverage: [1, 1],
                material: SourceMaterial::default(),
                blocking: Some(false),
                wall_feature: false,
                ceiling: None,
                ceiling_coverage: [1, 1],
                light_emitter: None,
            },
        );

        assert!(tiles.glyph_blocks('%'));
        assert!(!tiles.glyph_blocks('-'));
        assert!(tiles.glyph_blocks('#'));
        assert!(!tiles.glyph_blocks('.'));
    }

    #[test]
    fn wall_features_render_once_over_the_base_wall() {
        let base_wall = ResolvedTileSymbol {
            source: PixelSource::PaletteIndex(1),
            coverage: [1, 1],
            material: SourceMaterial::default(),
            blocking: Some(true),
            wall_feature: false,
            ceiling: None,
            ceiling_coverage: [1, 1],
            light_emitter: None,
        };
        let mut source_tiles = ResolvedSourceTiles {
            wall: Some(base_wall.clone()),
            floor: Some(ResolvedTileSymbol::tile_only(PixelSource::PaletteIndex(3))),
            ceiling: Some(ResolvedTileSymbol::tile_only(PixelSource::PaletteIndex(4))),
            ..Default::default()
        };
        source_tiles.explicit.insert(
            '!',
            ResolvedTileSymbol {
                source: PixelSource::PaletteIndex(2),
                coverage: [1, 1],
                material: SourceMaterial::default(),
                blocking: Some(true),
                wall_feature: true,
                ceiling: None,
                ceiling_coverage: [1, 1],
                light_emitter: None,
            },
        );
        let region = SourceRegion {
            id: "feature-test".to_string(),
            name: "Feature Test".to_string(),
            default: "wall".to_string(),
            floor: "floor".to_string(),
            ceiling: "ceiling".to_string(),
            tile_symbols: IndexMap::default(),
            terrain: vec!["#!#".to_string(), "#.#".to_string(), "###".to_string()],
        };
        let mut map = Map::default();

        build_3d_blocks_from_source_terrain(&mut map, &region, &source_tiles)
            .expect("feature map builds");

        let wall = map
            .geometry_objects
            .iter()
            .find(|object| object.name == "wall_1_0")
            .expect("base wall exists");
        assert!(
            wall.faces
                .iter()
                .all(|face| face.tile == Some(base_wall.source.clone()))
        );
        let feature = map
            .geometry_objects
            .iter()
            .find(|object| object.name == "wall_feature_1_0")
            .expect("wall feature exists");
        assert!(!feature.solid);
        assert_eq!(feature.faces[1].tile, Some(PixelSource::PaletteIndex(2)));
        assert!(
            feature
                .faces
                .iter()
                .enumerate()
                .filter(|(_, face)| face.tile == Some(PixelSource::PaletteIndex(2)))
                .map(|(index, _)| index)
                .eq([1])
        );
    }

    #[test]
    fn procedural_coverage_uses_continuous_world_aligned_uvs() {
        let tile = ResolvedTileSymbol::tile_with_coverage(PixelSource::PaletteIndex(1), [4, 4]);
        let mut map = Map::default();
        add_source_block(
            &mut map,
            "first".to_string(),
            4.0,
            0.0,
            8.0,
            5.0,
            3.0,
            9.0,
            tile.clone(),
            true,
            false,
        );
        add_source_block(
            &mut map,
            "next".to_string(),
            5.0,
            0.0,
            8.0,
            6.0,
            3.0,
            9.0,
            tile,
            true,
            false,
        );

        let first_front = &map.geometry_objects[0].faces[0];
        let next_front = &map.geometry_objects[1].faces[0];
        assert!(!first_front.auto_uv);
        assert!(!next_front.auto_uv);
        assert_eq!(
            first_front.uvs,
            vec![
                Vec2::new(1.0, 0.0),
                Vec2::new(1.25, 0.0),
                Vec2::new(1.25, 0.75),
                Vec2::new(1.0, 0.75),
            ]
        );
        assert_eq!(first_front.uvs[1], next_front.uvs[0]);
        assert_eq!(first_front.uvs[2], next_front.uvs[3]);
    }

    #[test]
    fn compiles_minimal_project() {
        let config = ProjectToml {
            project: ProjectSection {
                name: "Test".to_string(),
            },
            source: SourceSection::default(),
            game: GameSection {
                start_region: "cellar".to_string(),
                start_screen: String::new(),
                client_mode: "terminal".to_string(),
                terminal_mode: "text".to_string(),
                simulation_mode: "hybrid".to_string(),
                game_tick_ms: 250,
                turn_timeout_ms: 600,
                movement_units_per_sec: 4,
                turn_speed_deg_per_sec: 120,
                collision_mode: "tile".to_string(),
                auto_create_player: true,
                player: "player".to_string(),
            },
            viewport: ViewportSection::default(),
            terminal: TerminalSection::default(),
            build: BuildSection::default(),
        };
        let source = SourceDocument {
            tile_symbols: IndexMap::default(),
            characters: Vec::new(),
            items: Vec::new(),
            regions: vec![SourceRegion {
                id: "cellar".to_string(),
                name: "Cellar".to_string(),
                default: "wall.stone".to_string(),
                floor: "floor".to_string(),
                ceiling: "ceiling".to_string(),
                tile_symbols: IndexMap::default(),
                terrain: vec!["###".to_string(), "#@#".to_string(), "###".to_string()],
            }],
            screens: Vec::new(),
        };
        let project = compile_project(&config, source).expect("project compiles");
        assert_eq!(project.regions.len(), 1);
        assert_eq!(project.regions[0].map.name, "cellar");
        assert_eq!(project.characters.len(), 1);
        assert_eq!(project.regions[0].characters.len(), 1);
    }

    #[test]
    fn compiles_3d_project() {
        let config = ProjectToml {
            project: ProjectSection {
                name: "Dungeon".to_string(),
            },
            source: SourceSection::default(),
            game: GameSection {
                start_region: "cellar".to_string(),
                start_screen: String::new(),
                client_mode: "3d".to_string(),
                terminal_mode: "roguelike".to_string(),
                simulation_mode: "hybrid".to_string(),
                game_tick_ms: 250,
                turn_timeout_ms: 600,
                movement_units_per_sec: 4,
                turn_speed_deg_per_sec: 120,
                collision_mode: "tile".to_string(),
                auto_create_player: true,
                player: "player".to_string(),
            },
            viewport: ViewportSection::default(),
            terminal: TerminalSection::default(),
            build: BuildSection::default(),
        };
        let source = SourceDocument {
            tile_symbols: IndexMap::default(),
            characters: Vec::new(),
            items: Vec::new(),
            regions: vec![SourceRegion {
                id: "cellar".to_string(),
                name: "Cellar".to_string(),
                default: "wall.stone".to_string(),
                floor: "floor".to_string(),
                ceiling: "ceiling".to_string(),
                tile_symbols: IndexMap::default(),
                terrain: vec![
                    "#####".to_string(),
                    "#@..#".to_string(),
                    "#...#".to_string(),
                    "#####".to_string(),
                ],
            }],
            screens: Vec::new(),
        };
        let project = compile_project(&config, source).expect("project compiles");
        let map = &project.regions[0].map;
        assert_eq!(map.camera, MapCamera::ThreeDFirstPerson);
        assert!(
            map.geometry_objects
                .iter()
                .any(|object| object.name == "entrance")
        );
        assert!(
            map.geometry_objects
                .iter()
                .any(|object| object.name.starts_with("wall_"))
        );
        let player = project
            .characters
            .values()
            .find(|character| character.name == "Player")
            .expect("player template exists");
        assert!(player.data.contains("player_camera = \"firstp_grid\""));
    }

    #[test]
    fn loads_standard_asset_directories() {
        let root = std::env::temp_dir().join(format!("eldiron-source-assets-{}", Uuid::new_v4()));
        let assets_dir = root.join("assets/fonts");
        let tiles_dir = root.join("tiles/dungeon");
        fs::create_dir_all(&assets_dir).expect("asset dir created");
        fs::create_dir_all(&tiles_dir).expect("tile dir created");
        fs::write(
            assets_dir.join("Roboto-Bold.ttf"),
            include_bytes!("../../theframework/embedded/fonts/Roboto-Bold.ttf"),
        )
        .expect("font copied");
        fs::write(
            tiles_dir.join("stone.png"),
            include_bytes!("../../rusterix/embedded/icons/character_on.png"),
        )
        .expect("tile image written");

        let mut project = Project::new();
        load_project_directory_assets(&mut project, &root, &SourceSection::default())
            .expect("assets load");

        assert!(project.assets.values().any(|asset| {
            asset.name == "fonts/Roboto-Bold" && matches!(asset.buffer, AssetBuffer::Font(_))
        }));
        assert!(
            project
                .tiles
                .values()
                .any(|tile| tile.alias == "dungeon/stone" && !tile.textures.is_empty())
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn loads_procedural_recipes_as_regular_tiles() {
        let root = std::env::temp_dir().join(format!("eldiron-source-recipes-{}", Uuid::new_v4()));
        let recipes_dir = root.join("recipes/dungeon");
        fs::create_dir_all(&recipes_dir).expect("recipe dir created");
        fs::write(
            root.join("recipes/dungeon.recipe"),
            r#"
Material wall
    Noise Grain
        type = Gradient
        fractal = Ridged
        scale = F2(3.0, 2.0)

    Colorize
        source = Input.height
        base = DarkBrown

    MaterialData
        roughness = 0.8
        metallic = 0.2
        opacity = 1.0
        emissive = 0.1

    Normal
        source = Input.height + Grain * 0.05
        strength = 0.5

Material unused
    Colorize
        source = Input.height
        base = Gray
"#,
        )
        .expect("material written");
        fs::write(
            recipes_dir.join("wall.recipe"),
            r#"
Tile
    name = "Test Wall"
    material = dungeon/wall
    size = I2(8, 6)
    coverage = I2(2, 1)
    wrap = Repeat

    Pattern Wall
        Bricks
            columns = 3
            rows = 2
            gap = 0.08
            rounding = 0.1

    Height Surface
        source = Wall.height

    Animation
        frames = 3
        fps = 6
        looping = true

    Output
        height = Surface
"#,
        )
        .expect("recipe written");

        let mut project = Project::new();
        load_project_directory_assets(&mut project, &root, &SourceSection::default())
            .expect("procedural recipes load");

        let wall = project
            .tiles
            .values()
            .find(|tile| tile.alias == "dungeon/wall")
            .expect("procedural wall tile loaded");
        assert_eq!(wall.role, TileRole::ManMade);
        assert_eq!(wall.material_alias, "dungeon/wall");
        assert_eq!(wall.baked_material_data.len(), 3);
        assert_eq!(wall.procedural.coverage, [2, 1]);
        assert_eq!(wall.textures.len(), 3);
        assert!(
            wall.textures
                .iter()
                .all(|texture| texture.width == 16 && texture.height == 6)
        );
        assert!(
            wall.textures
                .iter()
                .all(|texture| texture.data_ext.is_some())
        );
        assert!(wall.textures.iter().any(|texture| {
            (0..texture.height).any(|y| {
                (0..texture.width).any(|x| texture.get_normal(x as u32, y as u32) != (0.0, 0.0))
            })
        }));
        let material_frames = wall.to_material_array();
        let mut material_texture = Texture::from_color([0, 0, 0, 255]);
        material_texture.width = wall.textures[0].width;
        material_texture.height = wall.textures[0].height;
        material_texture.data = vec![0; material_texture.width * material_texture.height * 4];
        material_texture.data_ext = Some(material_frames[0].clone());
        let (roughness, metallic, opacity, emissive) = material_texture.get_materials(0, 0);
        assert!((roughness - 0.8).abs() < 0.07);
        assert!((metallic - 0.2).abs() < 0.07);
        assert_eq!(opacity, 1.0);
        assert!((emissive - 0.1).abs() < 0.07);

        let lookup = SourceTileLookup::from_project(&project);
        assert_eq!(
            lookup.source_for("dungeon.wall"),
            Some(PixelSource::TileId(wall.id))
        );
        assert_eq!(
            lookup
                .tile_only_for("dungeon.wall")
                .map(|tile| tile.coverage),
            Some([2, 1])
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reports_procedural_recipe_path_when_parsing_fails() {
        let root =
            std::env::temp_dir().join(format!("eldiron-source-bad-recipe-{}", Uuid::new_v4()));
        let recipes_dir = root.join("recipes");
        fs::create_dir_all(&recipes_dir).expect("recipe dir created");
        let recipe_path = recipes_dir.join("broken.recipe");
        fs::write(&recipe_path, "Tile\n    nope = true\n").expect("recipe written");

        let mut project = Project::new();
        let error = load_project_directory_assets(&mut project, &root, &SourceSection::default())
            .expect_err("invalid recipe is rejected");
        assert!(error.contains(&recipe_path.display().to_string()));
        assert!(error.contains("failed to parse recipe"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn loads_external_tile_roots_and_animated_tiles() {
        let base =
            std::env::temp_dir().join(format!("eldiron-source-external-tiles-{}", Uuid::new_v4()));
        let root = base.join("project");
        let external_dir = base.join("shared-dungeon-tiles");
        let frames_dir = root.join("animation");
        fs::create_dir_all(&external_dir).expect("external tile dir created");
        fs::create_dir_all(&frames_dir).expect("animation frame dir created");
        let png = include_bytes!("../../rusterix/embedded/icons/character_on.png");
        fs::write(external_dir.join("wall.png"), png).expect("external tile written");
        fs::write(frames_dir.join("frame-1.png"), png).expect("first frame written");
        fs::write(frames_dir.join("frame-2.png"), png).expect("second frame written");

        let source = SourceSection {
            main: default_main_source(),
            tile_dirs: vec!["../shared-dungeon-tiles".to_string()],
            tiles: Vec::new(),
            tile_animations: vec![SourceTileAnimation {
                alias: "dungeon/torch".to_string(),
                frames: vec![
                    "animation/frame-1.png".to_string(),
                    "animation/frame-2.png".to_string(),
                ],
                role: "dungeon".to_string(),
                blocking: true,
                scale: 1.0,
                light_color: "#ff8844".to_string(),
                light_intensity: 1.0,
                light_range: 4.0,
                light_flicker: 0.1,
                light_lift: 0.5,
            }],
        };
        let mut project = Project::new();

        load_project_directory_assets(&mut project, &root, &source)
            .expect("external and animated tiles load");

        let wall = project
            .tiles
            .values()
            .find(|tile| tile.alias == "wall")
            .expect("external wall loaded");
        assert_eq!(wall.role, TileRole::Dungeon);
        let torch = project
            .tiles
            .values()
            .find(|tile| tile.alias == "dungeon/torch")
            .expect("animated torch loaded");
        assert_eq!(torch.textures.len(), 2);
        assert!(torch.blocking);
        assert_eq!(
            torch.light_emitter.as_ref().map(|light| light.color),
            Some([255, 136, 68, 255])
        );

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn local_asset_tiles_are_not_duplicated_as_generic_images() {
        let root = std::env::temp_dir().join(format!(
            "eldiron-source-local-asset-tiles-{}",
            Uuid::new_v4()
        ));
        let tiles_dir = root.join("assets/environment/tiles");
        let features_dir = root.join("assets/environment/features");
        fs::create_dir_all(&tiles_dir).expect("local tile dir created");
        fs::create_dir_all(&features_dir).expect("feature dir created");
        let png = include_bytes!("../../rusterix/embedded/icons/character_on.png");
        fs::write(tiles_dir.join("wall.png"), png).expect("local tile written");
        fs::write(features_dir.join("torch.png"), png).expect("feature image written");

        let source = SourceSection {
            main: default_main_source(),
            tile_dirs: vec!["assets/environment/tiles".to_string()],
            tiles: vec![SourceTileAsset {
                alias: "environment/torch".to_string(),
                path: "assets/environment/features/torch.png".to_string(),
                role: "dungeon".to_string(),
                blocking: true,
                scale: 1.0,
                light_color: "#ff8844".to_string(),
                light_intensity: 1.0,
                light_range: 4.0,
                light_flicker: 0.1,
                light_lift: 0.5,
            }],
            tile_animations: Vec::new(),
        };
        let mut project = Project::new();

        load_project_directory_assets(&mut project, &root, &source)
            .expect("local asset tiles load");

        assert!(project.tiles.values().any(|tile| tile.alias == "wall"));
        let torch = project
            .tiles
            .values()
            .find(|tile| tile.alias == "environment/torch")
            .expect("static feature tile loaded");
        assert_eq!(torch.textures.len(), 1);
        assert_eq!(
            torch.light_emitter.as_ref().map(|light| light.color),
            Some([255, 136, 68, 255])
        );
        assert!(
            !project.assets.values().any(|asset| {
                asset.name == "environment/tiles/wall" || asset.name == "environment/features/torch"
            }),
            "tile textures declared under assets should only be embedded as tiles"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn build_preserves_runtime_toml_sections() {
        let root = std::env::temp_dir().join(format!("eldiron-source-config-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("project dir created");
        fs::write(
            root.join("eldiron.toml"),
            r#"[project]
name = "Runtime Config"

[source]
main = "main.els"

[game]
start_region = "cellar"
start_screen = "play"
client_mode = "3d"

[viewport]
width = 320
height = 200
grid_size = 1

[renderer]
backend_3d = "raster"
style = "retro"

[render]
sun_enabled = false
fog_density = 5.0

[post]
enabled = true
posterize = 0.25

[build]
output = "build/game.eldiron"
"#,
        )
        .expect("toml written");
        fs::write(
            root.join("main.els"),
            r##"Region "cellar" {
  default = wall.stone
  terrain """
  ###
  #@#
  ###
  """
}

Screen "play" {
  widget "Game" {
    role = "game"
    x = 0
    y = 0
    width = 320
    height = 200
  }
}
"##,
        )
        .expect("source written");

        let output = build_project(&root).expect("project builds");
        let project: Project =
            serde_json::from_str(&fs::read_to_string(&output).expect("compiled project readable"))
                .expect("compiled project parses");

        assert!(project.config.contains("[renderer]"));
        assert!(project.config.contains("backend_3d = \"raster\""));
        assert!(project.config.contains("[render]"));
        assert!(project.config.contains("sun_enabled = false"));
        assert!(project.config.contains("[post]"));
        assert!(project.config.contains("posterize = 0.25"));
        assert!(!project.config.contains("[source]"));
        assert!(!project.config.contains("[build]"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn build_loads_regions_and_screens_dirs() {
        let root = std::env::temp_dir().join(format!("eldiron-source-split-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join("regions")).expect("regions dir created");
        fs::create_dir_all(root.join("screens")).expect("screens dir created");
        fs::write(
            root.join("eldiron.toml"),
            r#"[project]
name = "Split Source"

[source]
main = "main.els"

[game]
start_region = "cellar"
start_screen = "play"

[build]
output = "build/game.eldiron"
"#,
        )
        .expect("toml written");
        fs::write(root.join("main.els"), "tiles {\n}\n").expect("main source written");
        fs::write(
            root.join("regions/cellar.els"),
            r##"Region "cellar" {
  name = "Cellar"
  default = wall.stone

  terrain """
  ###
  #@#
  ###
  """
}
"##,
        )
        .expect("region source written");
        fs::write(
            root.join("screens/play.els"),
            r##"Screen "play" {
  name = "Play"

  widget "Game" {
    role = "game"
    x = 0
    y = 0
    width = 80
    height = 24
  }
}
"##,
        )
        .expect("screen source written");

        let output = build_project(&root).expect("project builds");
        let project: Project =
            serde_json::from_str(&fs::read_to_string(&output).expect("compiled project readable"))
                .expect("compiled project parses");

        assert!(
            project
                .regions
                .iter()
                .any(|region| region.map.name == "cellar")
        );
        assert!(
            project
                .screens
                .values()
                .any(|screen| screen.map.name == "play")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn source_tile_symbols_resolve_to_project_tiles() {
        let root = std::env::temp_dir().join(format!("eldiron-source-tiles-{}", Uuid::new_v4()));
        let tiles_dir = root.join("tiles");
        fs::create_dir_all(&tiles_dir).expect("tile dir created");
        fs::write(
            tiles_dir.join("floor.png"),
            include_bytes!("../../rusterix/embedded/icons/character_on.png"),
        )
        .expect("floor tile written");
        fs::write(
            tiles_dir.join("wall.png"),
            include_bytes!("../../rusterix/embedded/icons/character_on.png"),
        )
        .expect("wall tile written");
        fs::write(
            root.join("eldiron.toml"),
            r#"[project]
name = "Tile Symbols"

[source]
main = "main.els"

[game]
start_region = "cellar"
client_mode = "3d"

[build]
output = "build/game.eldiron"
"#,
        )
        .expect("toml written");
        fs::write(
            root.join("main.els"),
            r##"tiles {
  "#" = wall material=stone finish=wet
  "." = floor material=stone finish=polished
  "@" = floor
}

Region "cellar" {
  default = wall.stone
  terrain """
  ####
  #@.#
  ####
  """
}
"##,
        )
        .expect("source written");

        let output = build_project(&root).expect("project builds");
        let project: Project =
            serde_json::from_str(&fs::read_to_string(&output).expect("compiled project readable"))
                .expect("compiled project parses");
        let floor_id = project
            .tiles
            .values()
            .find(|tile| tile.alias == "floor")
            .expect("floor tile loaded")
            .id;
        let wall_id = project
            .tiles
            .values()
            .find(|tile| tile.alias == "wall")
            .expect("wall tile loaded")
            .id;
        let map = &project.regions[0].map;
        assert!(
            map.geometry_objects
                .iter()
                .filter(|object| object.name.starts_with("floor_"))
                .flat_map(|object| &object.faces)
                .any(|face| face.tile == Some(PixelSource::TileId(floor_id)))
        );
        assert!(
            map.geometry_objects
                .iter()
                .filter(|object| object.name.starts_with("wall_"))
                .flat_map(|object| &object.faces)
                .any(|face| face.tile == Some(PixelSource::TileId(wall_id)))
        );
        let wall = map
            .geometry_objects
            .iter()
            .find(|object| object.name.starts_with("wall_"))
            .expect("generated wall exists");
        assert_eq!(
            wall.properties.get_str("material_preset").as_deref(),
            Some("stone")
        );
        assert_eq!(
            wall.properties.get_str("material_finish").as_deref(),
            Some("wet")
        );
        let floor = map
            .geometry_objects
            .iter()
            .find(|object| object.name.starts_with("floor_"))
            .expect("generated floor exists");
        assert_eq!(
            floor.properties.get_str("material_preset").as_deref(),
            Some("stone")
        );
        assert_eq!(
            floor.properties.get_str("material_finish").as_deref(),
            Some("polished")
        );
        assert!(
            map.properties
                .get_str("eldiron_source_tiles")
                .is_some_and(|tiles| tiles.contains(&floor_id.to_string()))
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn source_3d_one_tile_corridor_supports_firstp_direct_movement() {
        let config = ProjectToml {
            project: ProjectSection {
                name: "Dungeon".to_string(),
            },
            source: SourceSection::default(),
            game: GameSection {
                start_region: "cellar".to_string(),
                start_screen: String::new(),
                client_mode: "3d".to_string(),
                terminal_mode: "roguelike".to_string(),
                simulation_mode: "hybrid".to_string(),
                game_tick_ms: 250,
                turn_timeout_ms: 600,
                movement_units_per_sec: 4,
                turn_speed_deg_per_sec: 120,
                collision_mode: "mesh".to_string(),
                auto_create_player: true,
                player: "player".to_string(),
            },
            viewport: ViewportSection::default(),
            terminal: TerminalSection::default(),
            build: BuildSection::default(),
        };
        let source = SourceDocument {
            tile_symbols: IndexMap::default(),
            characters: Vec::new(),
            items: Vec::new(),
            regions: vec![SourceRegion {
                id: "cellar".to_string(),
                name: "Cellar".to_string(),
                default: "wall.stone".to_string(),
                floor: "floor".to_string(),
                ceiling: "ceiling".to_string(),
                tile_symbols: IndexMap::default(),
                terrain: vec![
                    "#####".to_string(),
                    "#@..#".to_string(),
                    "#...#".to_string(),
                    "#####".to_string(),
                ],
            }],
            screens: Vec::new(),
        };
        let project = compile_project(&config, source).expect("project compiles");
        let region = &project.regions[0];
        let map = &region.map;
        let player = region
            .characters
            .values()
            .find(|character| character.name == "Player")
            .expect("player instance exists");
        let start = Vec2::new(player.position.x, player.position.z);
        let radius = 0.34;

        let mut world = rusterix::CollisionWorld::new(10);
        let mut builder = rusterix::D3ChunkBuilder::new();
        let assets = rusterix::Assets::default();
        let bbox = map.bbox();
        let min_chunk = Vec2::new(
            (bbox.min.x / 10.0).floor() as i32,
            (bbox.min.y / 10.0).floor() as i32,
        );
        let max_chunk = Vec2::new(
            (bbox.max.x / 10.0).floor() as i32,
            (bbox.max.y / 10.0).floor() as i32,
        );
        for cy in min_chunk.y..=max_chunk.y {
            for cx in min_chunk.x..=max_chunk.x {
                let chunk_origin = Vec2::new(cx, cy);
                let collision = builder.build_collision(map, &assets, chunk_origin, 10);
                world.update_chunk(chunk_origin, collision);
            }
        }

        let (east, arrived_east) = world
            .move_towards_on_floors_direct(
                start,
                start + Vec2::new(0.4, 0.0),
                0.4,
                radius,
                1.0,
                0.0,
            )
            .expect("source floor should support firstp direct movement");
        assert!(
            east.x > start.x + 0.1,
            "player should be able to move east from source spawn, start={start:?}, end={east:?}, arrived={arrived_east}"
        );

        let (north, arrived_north) = world
            .move_towards_on_floors_direct(
                start,
                start + Vec2::new(0.0, -0.8),
                0.8,
                radius,
                1.0,
                0.0,
            )
            .expect("source floor should have collision context");
        assert!(
            !arrived_north && north.z > start.y - 0.7,
            "north wall should block firstp direct movement, start={start:?}, end={north:?}"
        );
    }
}
