use crate::prelude::*;
use procedural_recipes::{
    MaterialRecipe, Recipe, RecipeDocument, RecipePlacement, RecipeRenderer, RenderOptions,
    RenderSurface, RenderSurfaceFrame, RenderSurfaceMapping, SdfRenderer, parse_document,
};
use rusterix::{Texture, Tile, TileRole};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{LazyLock, RwLock};

#[derive(Clone)]
struct RecipePreviewCacheEntry {
    fingerprint: u64,
    buffer: TheRGBABuffer,
    visual: TheRGBABuffer,
}

static RECIPE_PREVIEW_CACHE: LazyLock<RwLock<FxHashMap<Uuid, RecipePreviewCacheEntry>>> =
    LazyLock::new(|| RwLock::new(FxHashMap::default()));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProceduralRecipeKind {
    Tile,
    Fixture,
    Material,
    Sdf,
}

impl ProceduralRecipeKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tile => "Tile Recipe",
            Self::Fixture => "Fixture Recipe",
            Self::Material => "Material Recipe",
            Self::Sdf => "SDF Recipe",
        }
    }
}

pub fn localized_recipe_kind(kind: ProceduralRecipeKind) -> String {
    match kind {
        ProceduralRecipeKind::Tile => fl!("tile_recipe"),
        ProceduralRecipeKind::Fixture => fl!("fixture_recipe"),
        ProceduralRecipeKind::Material => fl!("material_recipe"),
        ProceduralRecipeKind::Sdf => fl!("sdf_recipe"),
    }
}

/// Converts the parser's multi-line source diagnostic into text suitable for
/// the Creator's single-line status bar.
pub fn compact_recipe_diagnostic(error: &str) -> String {
    let mut lines = error.lines().map(str::trim).filter(|line| !line.is_empty());
    let summary = lines.next().unwrap_or("Invalid Recipe");
    let location = lines
        .find_map(|line| line.strip_prefix("-->").map(str::trim))
        .filter(|line| !line.is_empty());
    let diagnostic = location
        .map(|location| format!("{summary} — {location}"))
        .unwrap_or_else(|| summary.to_string());
    const MAX_CHARS: usize = 180;
    if diagnostic.chars().count() <= MAX_CHARS {
        diagnostic
    } else {
        diagnostic.chars().take(MAX_CHARS - 1).collect::<String>() + "…"
    }
}

pub fn recipe_description(source: &str) -> Result<(String, ProceduralRecipeKind), String> {
    match parse_document(source).map_err(|error| error.to_string())? {
        RecipeDocument::Tile(recipe) => {
            let kind = if recipe.placement == RecipePlacement::Fixture {
                ProceduralRecipeKind::Fixture
            } else {
                ProceduralRecipeKind::Tile
            };
            Ok((recipe.name, kind))
        }
        RecipeDocument::Materials(document) => {
            let first = document
                .materials
                .first()
                .ok_or_else(|| "Material document contains no materials".to_string())?;
            let name = if document.materials.len() == 1 {
                first.name.clone()
            } else {
                format!("{} (+{})", first.name, document.materials.len() - 1)
            };
            Ok((name, ProceduralRecipeKind::Material))
        }
        RecipeDocument::Sdfs(document) => {
            let first = document
                .recipes
                .first()
                .ok_or_else(|| "SDF document contains no recipes".to_string())?;
            let name = if document.recipes.len() == 1 {
                first.name.clone()
            } else {
                format!("{} (+{})", first.name, document.recipes.len() - 1)
            };
            Ok((name, ProceduralRecipeKind::Sdf))
        }
    }
}

pub fn recipe_name(source: &str) -> String {
    recipe_description(source)
        .map(|(name, _)| name)
        .unwrap_or_else(|_| "Invalid Recipe".to_string())
}

pub fn unique_recipe_alias(project: &Project, requested: &str) -> String {
    let base = requested
        .trim()
        .trim_end_matches(".recipe")
        .to_ascii_lowercase()
        .replace([' ', '_'], "-");
    let base = if base.is_empty() { "recipe" } else { &base };
    if !project
        .procedural_recipes
        .values()
        .any(|asset| asset.alias.eq_ignore_ascii_case(base))
    {
        return base.to_string();
    }
    for suffix in 2.. {
        let candidate = format!("{base}-{suffix}");
        if !project
            .procedural_recipes
            .values()
            .any(|asset| asset.alias.eq_ignore_ascii_case(&candidate))
        {
            return candidate;
        }
    }
    unreachable!()
}

pub fn duplicate_recipe_source(source: &str) -> String {
    let current_name = recipe_name(source);
    let replacement = format!("name = \"{} Copy\"", current_name.replace('"', "\\\""));
    let mut changed = false;
    source
        .lines()
        .map(|line| {
            if !changed && line.trim_start().starts_with("name =") {
                changed = true;
                let indent = &line[..line.len() - line.trim_start().len()];
                format!("{indent}{replacement}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + if source.ends_with('\n') { "\n" } else { "" }
}

fn material_source_from_project<'a>(project: &'a Project, alias: &str) -> Option<&'a str> {
    let alias = alias.to_ascii_lowercase();
    project
        .procedural_materials
        .get(&alias)
        .map(String::as_str)
        .or_else(|| {
            project
                .procedural_recipes
                .values()
                .find(|asset| asset.alias.eq_ignore_ascii_case(&alias))
                .map(|asset| asset.source.as_str())
        })
}

fn material_from_project(project: &Project, alias: &str) -> Option<MaterialRecipe> {
    let source = material_source_from_project(project, alias)?;
    let RecipeDocument::Materials(document) = parse_document(source).ok()? else {
        return None;
    };
    document
        .materials
        .iter()
        .find(|material| {
            alias.ends_with(&format!("/{}", material.id.to_ascii_lowercase()))
                || document.materials.len() == 1
        })
        .cloned()
}

fn flat_preview_recipe(name: &str) -> Recipe {
    Recipe {
        name: format!("{name} Preview"),
        size: [128, 128],
        ..Recipe::default()
    }
}

fn hash_art_palette(project: &Project, hasher: &mut DefaultHasher) {
    for color in &project.art_palette.colors {
        match color {
            Some(color) => {
                1_u8.hash(hasher);
                color.r.to_bits().hash(hasher);
                color.g.to_bits().hash(hasher);
                color.b.to_bits().hash(hasher);
                color.a.to_bits().hash(hasher);
                color.name.hash(hasher);
            }
            None => 0_u8.hash(hasher),
        }
    }
}

fn recipe_preview_fingerprint(project: &Project, asset: &ProceduralRecipeAsset) -> u64 {
    let mut hasher = DefaultHasher::new();
    asset.id.hash(&mut hasher);
    asset.alias.hash(&mut hasher);
    asset.source.hash(&mut hasher);
    hash_art_palette(project, &mut hasher);

    // Tile previews can depend on a separate Material Recipe. Include only
    // that source so editing one Recipe does not invalidate every thumbnail.
    if let Ok(RecipeDocument::Tile(recipe)) = parse_document(&asset.source)
        && let Some(alias) = recipe
            .material_map
            .as_ref()
            .map(|map| map.base.as_str())
            .or(recipe.material.as_deref())
        && let Some(source) = material_source_from_project(project, alias)
    {
        alias.hash(&mut hasher);
        source.hash(&mut hasher);
    }
    hasher.finish()
}

pub fn clear_recipe_preview_cache() {
    RECIPE_PREVIEW_CACHE.write().unwrap().clear();
}

pub fn render_recipe_preview(project: &Project, asset_id: Uuid) -> Result<TheRGBABuffer, String> {
    let asset = project
        .procedural_recipes
        .get(&asset_id)
        .ok_or_else(|| "Recipe asset was not found".to_string())?;
    let fingerprint = recipe_preview_fingerprint(project, asset);
    let (cached, has_older_entry) = {
        let cache = RECIPE_PREVIEW_CACHE.read().unwrap();
        (
            cache
                .get(&asset_id)
                .filter(|entry| entry.fingerprint == fingerprint)
                .map(|entry| entry.buffer.clone()),
            cache.contains_key(&asset_id),
        )
    };
    if let Some(cached) = cached {
        return Ok(cached);
    }

    // Source projects and saved Creator projects already contain the baked
    // Tile texture. Use it to seed the first preview without re-running the
    // procedural renderer. A changed source has an older cache entry and must
    // be rendered normally before the baked Tile is updated.
    if !has_older_entry
        && let Some(tile_id) = asset.tile_id
        && let Some(texture) = project
            .tiles
            .get(&tile_id)
            .and_then(|tile| tile.textures.first())
        && texture.data.len() == texture.width * texture.height * 4
    {
        let buffer = TheRGBABuffer::from(
            texture.data.clone(),
            texture.width as u32,
            texture.height as u32,
        );
        RECIPE_PREVIEW_CACHE.write().unwrap().insert(
            asset_id,
            RecipePreviewCacheEntry {
                fingerprint,
                buffer: buffer.clone(),
                visual: buffer.clone(),
            },
        );
        return Ok(buffer);
    }

    let (buffer, visual) = render_recipe_preview_uncached(project, asset_id)?;
    RECIPE_PREVIEW_CACHE.write().unwrap().insert(
        asset_id,
        RecipePreviewCacheEntry {
            fingerprint,
            buffer: buffer.clone(),
            visual,
        },
    );
    Ok(buffer)
}

/// Returns the editor-facing visualization for a Recipe. For Tile recipes this
/// is the recipe's own rendered output (height, or its Colorize output), not
/// the referenced material. A material is allowed to stay constant while the
/// recipe's noise/geometry changes, so using it here makes successful edits
/// appear as if the preview never refreshed.
pub fn render_recipe_visual_preview(
    project: &Project,
    asset_id: Uuid,
) -> Result<TheRGBABuffer, String> {
    let _ = render_recipe_preview(project, asset_id)?;
    let asset = project
        .procedural_recipes
        .get(&asset_id)
        .ok_or_else(|| "Recipe asset was not found".to_string())?;
    let fingerprint = recipe_preview_fingerprint(project, asset);
    RECIPE_PREVIEW_CACHE
        .read()
        .unwrap()
        .get(&asset_id)
        .filter(|entry| entry.fingerprint == fingerprint)
        .map(|entry| entry.visual.clone())
        .ok_or_else(|| "Recipe preview cache was not populated".to_string())
}

/// Return an already-rendered editor preview without doing any rendering on
/// the UI thread. Selection uses this for an immediate image when available,
/// then queues a fresh revision through the preview worker.
pub fn cached_recipe_visual_preview(project: &Project, asset_id: Uuid) -> Option<TheRGBABuffer> {
    let asset = project.procedural_recipes.get(&asset_id)?;
    let fingerprint = recipe_preview_fingerprint(project, asset);
    RECIPE_PREVIEW_CACHE
        .read()
        .unwrap()
        .get(&asset_id)
        .filter(|entry| entry.fingerprint == fingerprint)
        .map(|entry| entry.visual.clone())
}

fn render_recipe_preview_uncached(
    project: &Project,
    asset_id: Uuid,
) -> Result<(TheRGBABuffer, TheRGBABuffer), String> {
    let asset = project
        .procedural_recipes
        .get(&asset_id)
        .ok_or_else(|| "Recipe asset was not found".to_string())?;
    let document = parse_document(&asset.source).map_err(|error| error.to_string())?;
    let renderer =
        RecipeRenderer::new(&project.art_palette).unwrap_or_else(|_| RecipeRenderer::grayscale());
    let options = RenderOptions::default();

    match document {
        RecipeDocument::Tile(recipe) => {
            let rendered = renderer
                .render(&recipe, &options)
                .map_err(|error| error.to_string())?;
            let frame = rendered
                .frames
                .first()
                .ok_or_else(|| "Recipe rendered no frames".to_string())?;
            let mut rgba = frame.rgba.clone();
            let visual = rgba.clone();
            if let Some(alias) = recipe
                .material_map
                .as_ref()
                .map(|map| map.base.as_str())
                .or(recipe.material.as_deref())
                && let Some(material) = material_from_project(project, alias)
                && let Ok(material) = renderer.render_material(&material, &rendered, &options)
                && let Some(frame) = material.frames.first()
            {
                rgba = frame.rgba.clone();
            }
            Ok((
                TheRGBABuffer::from(rgba, rendered.width, rendered.height),
                TheRGBABuffer::from(visual, rendered.width, rendered.height),
            ))
        }
        RecipeDocument::Materials(document) => {
            let material = document
                .materials
                .first()
                .ok_or_else(|| "Material document contains no materials".to_string())?;
            let base = renderer
                .render(&flat_preview_recipe(&material.name), &options)
                .map_err(|error| error.to_string())?;
            let rendered = renderer
                .render_material_preview(material, &base, &options)
                .map_err(|error| error.to_string())?;
            let frame = rendered
                .frames
                .first()
                .ok_or_else(|| "Material rendered no frames".to_string())?;
            let buffer = TheRGBABuffer::from(frame.rgba.clone(), rendered.width, rendered.height);
            Ok((buffer.clone(), buffer))
        }
        RecipeDocument::Sdfs(document) => {
            let recipe = document
                .recipes
                .first()
                .ok_or_else(|| "SDF document contains no recipes".to_string())?;
            let surface = RenderSurface {
                width: 128,
                height: 128,
                mapping: RenderSurfaceMapping::default(),
                fps: 0.0,
                looping: true,
                frames: vec![RenderSurfaceFrame { time: 0.0 }],
            };
            let rendered =
                SdfRenderer::render(recipe, &surface).map_err(|error| error.to_string())?;
            let mut rgba = Vec::with_capacity(rendered.coverage.len() * 4);
            for alpha in rendered.coverage {
                rgba.extend_from_slice(&[216, 201, 167, alpha]);
            }
            let buffer = TheRGBABuffer::from(rgba, rendered.width, rendered.height);
            Ok((buffer.clone(), buffer))
        }
    }
}

/// Fully render a preview without consulting or mutating the shared cache.
/// RecipeEditor uses this on its worker thread so typing never blocks the UI.
pub(crate) fn render_recipe_preview_fresh(
    project: &Project,
    asset_id: Uuid,
) -> Result<(TheRGBABuffer, TheRGBABuffer), String> {
    render_recipe_preview_uncached(project, asset_id)
}

/// Install a completed worker render only after its generation has been
/// accepted by the editor. This prevents an older, slower render from
/// replacing the cache entry for newer source.
pub(crate) fn cache_recipe_preview_result(
    project: &Project,
    asset_id: Uuid,
    buffer: TheRGBABuffer,
    visual: TheRGBABuffer,
) -> Result<(), String> {
    let asset = project
        .procedural_recipes
        .get(&asset_id)
        .ok_or_else(|| "Recipe asset was not found".to_string())?;
    RECIPE_PREVIEW_CACHE.write().unwrap().insert(
        asset_id,
        RecipePreviewCacheEntry {
            fingerprint: recipe_preview_fingerprint(project, asset),
            buffer,
            visual,
        },
    );
    Ok(())
}

/// Rebuild the compatibility material/SDF lookup maps after canonical Recipe edits.
pub fn sync_recipe_compatibility_catalogs(project: &mut Project) {
    project.procedural_materials.clear();
    project.procedural_sdfs.clear();
    for asset in project.procedural_recipes.values() {
        let Ok(document) = parse_document(&asset.source) else {
            continue;
        };
        match document {
            RecipeDocument::Materials(document) => {
                if document.materials.len() == 1 {
                    project
                        .procedural_materials
                        .insert(asset.alias.to_ascii_lowercase(), asset.source.clone());
                }
                for material in document.materials {
                    project.procedural_materials.insert(
                        format!("{}/{}", asset.alias, material.id).to_ascii_lowercase(),
                        asset.source.clone(),
                    );
                }
            }
            RecipeDocument::Sdfs(document) => {
                if document.recipes.len() == 1 {
                    project
                        .procedural_sdfs
                        .insert(asset.alias.to_ascii_lowercase(), asset.source.clone());
                }
                for recipe in document.recipes {
                    project.procedural_sdfs.insert(
                        format!("{}/{}", asset.alias, recipe.id).to_ascii_lowercase(),
                        asset.source.clone(),
                    );
                }
            }
            RecipeDocument::Tile(_) => {}
        }
    }
}

/// Bake a Tile Recipe into the ordinary tile catalog used by maps and the tile
/// picker. Material and SDF documents remain reusable catalog-only assets.
pub fn rebake_tile_recipe(project: &mut Project, asset_id: Uuid) -> Result<(), String> {
    let preview = render_recipe_preview(project, asset_id)?;
    rebake_tile_recipe_with_preview(project, asset_id, &preview)
}

pub fn rebake_tile_recipe_with_preview(
    project: &mut Project,
    asset_id: Uuid,
    preview: &TheRGBABuffer,
) -> Result<(), String> {
    let asset = project
        .procedural_recipes
        .get(&asset_id)
        .cloned()
        .ok_or_else(|| "Recipe asset was not found".to_string())?;
    let RecipeDocument::Tile(recipe) =
        parse_document(&asset.source).map_err(|error| error.to_string())?
    else {
        return Ok(());
    };
    let texture = Texture::new(
        preview.pixels().to_vec(),
        preview.dim().width as usize,
        preview.dim().height as usize,
    );
    let tile_id = if let Some(tile_id) = asset.tile_id
        && let Some(tile) = project.tiles.get_mut(&tile_id)
    {
        tile.textures = vec![texture];
        tile.alias = asset.alias.clone();
        tile.apply_procedural_recipe_metadata(&recipe);
        tile_id
    } else {
        let mut tile = Tile::from_texture(texture);
        tile.alias = asset.alias.clone();
        tile.role = TileRole::ManMade;
        tile.apply_procedural_recipe_metadata(&recipe);
        let tile_id = tile.id;
        project.tiles.insert(tile_id, tile);
        tile_id
    };
    if let Some(asset) = project.procedural_recipes.get_mut(&asset_id) {
        asset.tile_id = Some(tile_id);
    }
    Ok(())
}

/// Upgrade projects created while only the runtime material/SDF maps were
/// persisted. Identical multi-declaration sources become one editable asset.
pub fn migrate_legacy_recipe_catalog(project: &mut Project) -> bool {
    let mut changed = false;
    let legacy = project
        .procedural_materials
        .iter()
        .chain(project.procedural_sdfs.iter())
        .map(|(alias, source)| (alias.clone(), source.clone()))
        .collect::<Vec<_>>();
    for (alias, source) in legacy {
        if project
            .procedural_recipes
            .values()
            .any(|asset| asset.source == source)
        {
            continue;
        }
        let alias = unique_recipe_alias(project, &alias);
        let asset = ProceduralRecipeAsset::new(alias, source);
        project.procedural_recipes.insert(asset.id, asset);
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_asset_name_comes_from_canonical_source() {
        let asset = ProceduralRecipeAsset::default();
        assert_eq!(recipe_name(&asset.source), "Untitled Tile");
        assert_eq!(
            recipe_description(&asset.source).unwrap().1,
            ProceduralRecipeKind::Tile
        );
    }

    #[test]
    fn parser_diagnostics_are_compacted_for_the_single_line_statusbar() {
        let diagnostic = compact_recipe_diagnostic(
            "error[PR0006]: field assignments require a name and value\n --> line 20:5\n 20 | seed =\n    |     ^",
        );
        assert_eq!(
            diagnostic,
            "error[PR0006]: field assignments require a name and value — line 20:5"
        );
        assert!(!diagnostic.contains('\n'));
    }

    #[test]
    fn duplicate_changes_the_source_name_and_uses_a_unique_alias() {
        let mut project = Project::new();
        let asset = ProceduralRecipeAsset::default();
        project.procedural_recipes.insert(asset.id, asset.clone());
        assert_eq!(
            unique_recipe_alias(&project, "untitled-tile"),
            "untitled-tile-2"
        );
        let source = duplicate_recipe_source(&asset.source);
        assert_eq!(recipe_name(&source), "Untitled Tile Copy");
    }

    #[test]
    fn legacy_alias_maps_migrate_without_duplicating_shared_sources() {
        let mut project = Project::new();
        let source = "Material cloth\n    Surface\n        color = #777777\n".to_string();
        project
            .procedural_materials
            .insert("cloth".into(), source.clone());
        project
            .procedural_materials
            .insert("cloth/cloth".into(), source);
        assert!(migrate_legacy_recipe_catalog(&mut project));
        assert_eq!(project.procedural_recipes.len(), 1);
        assert!(!migrate_legacy_recipe_catalog(&mut project));
    }

    #[test]
    fn tile_recipe_rebakes_into_the_tile_catalog() {
        let mut project = Project::new();
        let asset = ProceduralRecipeAsset::default();
        let id = asset.id;
        project.procedural_recipes.insert(id, asset);
        rebake_tile_recipe(&mut project, id).unwrap();
        let tile_id = project.procedural_recipes[&id].tile_id.unwrap();
        let tile = &project.tiles[&tile_id];
        assert_eq!(tile.alias, "untitled-tile");
        assert_eq!(tile.procedural.coverage, [1, 1]);
        assert_eq!(tile.textures.len(), 1);
    }

    #[test]
    fn hideout2d_meadow_grass_rebakes_as_a_paintable_surface_tile() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test_projects/Hideout2D.eldiron");
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read Hideout2D fixture '{}': {error}", path.display()));
        let mut project: Project = serde_json::from_str(&contents)
            .unwrap_or_else(|error| panic!("load Hideout2D fixture: {error}"));
        let recipe_id = project
            .procedural_recipes
            .values()
            .find(|asset| asset.alias == "meadow-grass")
            .map(|asset| asset.id)
            .expect("Hideout2D contains the meadow-grass recipe");
        let original_tile_id = project.procedural_recipes[&recipe_id]
            .tile_id
            .expect("meadow-grass has a baked tile");

        rebake_tile_recipe(&mut project, recipe_id).expect("rebake meadow grass");

        let tile_id = project.procedural_recipes[&recipe_id].tile_id.unwrap();
        let tile = &project.tiles[&tile_id];
        assert_eq!(tile_id, original_tile_id);
        assert_eq!(tile.alias, "meadow-grass");
        assert_eq!(tile.role, TileRole::Nature);
        assert_eq!(
            tile.recipe_placement,
            rusterix::TileRecipePlacement::Surface
        );
        assert_eq!(tile.procedural.coverage, [1, 1]);
        assert_eq!(tile.textures.len(), 1);
        assert_eq!((tile.textures[0].width, tile.textures[0].height), (64, 64));

        let pixels = &tile.textures[0].data;
        let green_pixels = pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[1] > pixel[0] && pixel[1] > pixel[2])
            .count();
        assert!(
            green_pixels * 4 > pixels.len() / 4 * 3,
            "expected the rebaked tile to be predominantly green"
        );
    }

    #[test]
    fn gate_brick_wall_rebakes_as_a_rect_paintable_surface_tile() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_projects/Gate.eldiron");
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read Gate fixture '{}': {error}", path.display()));
        let mut project: Project = serde_json::from_str(&contents)
            .unwrap_or_else(|error| panic!("load Gate fixture: {error}"));
        let recipe_id = project
            .procedural_recipes
            .values()
            .find(|asset| asset.alias == "gate-brick-wall")
            .map(|asset| asset.id)
            .expect("Gate contains the brick-wall recipe");
        assert!(
            project
                .procedural_recipes
                .values()
                .any(|asset| asset.alias == "gate-brick-material")
        );

        let original_tile_id = project.procedural_recipes[&recipe_id]
            .tile_id
            .expect("gate-brick-wall has a baked tile");
        let saved_pixels = project.tiles[&original_tile_id].textures[0].data.clone();
        let (fresh_preview, _) =
            render_recipe_preview_fresh(&project, recipe_id).expect("render Gate brick wall");
        rebake_tile_recipe_with_preview(&mut project, recipe_id, &fresh_preview)
            .expect("rebake Gate brick wall");

        let tile_id = project.procedural_recipes[&recipe_id].tile_id.unwrap();
        let tile = &project.tiles[&tile_id];
        assert_eq!(tile_id, original_tile_id);
        assert_eq!(tile.alias, "gate-brick-wall");
        assert_eq!(tile.role, TileRole::ManMade);
        assert!(tile.blocking);
        assert_eq!(
            tile.recipe_placement,
            rusterix::TileRecipePlacement::Surface
        );
        assert_eq!(tile.procedural.coverage, [1, 1]);
        assert_eq!(tile.textures.len(), 1);
        assert_eq!((tile.textures[0].width, tile.textures[0].height), (48, 48));
        assert_eq!(tile.textures[0].data, saved_pixels);

        let red_brick_pixels = saved_pixels
            .chunks_exact(4)
            .filter(|pixel| {
                pixel[0] > pixel[1].saturating_add(20) && pixel[0] > pixel[2].saturating_add(20)
            })
            .count();
        assert!(
            red_brick_pixels > 48 * 48 / 2,
            "expected most of the tile to be red brick"
        );
    }

    #[test]
    fn fixture_recipe_rebakes_with_shared_fixture_metadata() {
        let mut project = Project::new();
        let asset = ProceduralRecipeAsset::new(
            "fixture",
            r#"Tile
    name = "Fixture"
    placement = Fixture
    Output
        height = 0.0
"#,
        );
        let id = asset.id;
        project.procedural_recipes.insert(id, asset);
        rebake_tile_recipe(&mut project, id).unwrap();
        let tile_id = project.procedural_recipes[&id].tile_id.unwrap();
        assert_eq!(
            project.tiles[&tile_id].recipe_placement,
            rusterix::TileRecipePlacement::Fixture
        );
        assert_eq!(
            recipe_description(&project.procedural_recipes[&id].source)
                .unwrap()
                .1,
            ProceduralRecipeKind::Fixture
        );
    }

    #[test]
    fn preview_cache_hits_until_the_source_changes() {
        let mut project = Project::new();
        let asset = ProceduralRecipeAsset::default();
        let id = asset.id;
        project.procedural_recipes.insert(id, asset);

        let first = render_recipe_preview(&project, id).unwrap();
        let first_fingerprint = RECIPE_PREVIEW_CACHE.read().unwrap()[&id].fingerprint;
        let second = render_recipe_preview(&project, id).unwrap();
        assert_eq!(first.pixels(), second.pixels());
        assert_eq!(
            RECIPE_PREVIEW_CACHE.read().unwrap()[&id].fingerprint,
            first_fingerprint
        );

        project.procedural_recipes.get_mut(&id).unwrap().source =
            duplicate_recipe_source(&project.procedural_recipes[&id].source);
        assert_ne!(
            recipe_preview_fingerprint(&project, &project.procedural_recipes[&id]),
            first_fingerprint
        );
        render_recipe_preview(&project, id).unwrap();
        assert_ne!(
            RECIPE_PREVIEW_CACHE.read().unwrap()[&id].fingerprint,
            first_fingerprint
        );
    }

    #[test]
    fn tile_visual_preview_changes_when_an_external_material_hides_height_changes() {
        let mut project = Project::new();
        let material = ProceduralRecipeAsset::new(
            "material",
            "Material stone\n    name = \"Stone\"\n\n    Color Base\n        nearest = #555555\n\n    Surface\n        color = Base\n",
        );
        let tile = ProceduralRecipeAsset::new(
            "stone",
            "Tile\n    name = \"Stone\"\n    material = material/stone\n    size = I2(16, 16)\n\n    Noise Broad\n        scale = I2(3, 3)\n        seed = 17\n\n    Height Surface\n        source = Broad\n\n    Output\n        height = Surface\n",
        );
        let tile_id = tile.id;
        project.procedural_recipes.insert(material.id, material);
        project.procedural_recipes.insert(tile.id, tile);
        sync_recipe_compatibility_catalogs(&mut project);

        let first_baked = render_recipe_preview(&project, tile_id).unwrap();
        let first = render_recipe_visual_preview(&project, tile_id).unwrap();
        let changed_source =
            project.procedural_recipes[&tile_id]
                .source
                .replacen("seed = 17", "seed = 18", 1);
        project.procedural_recipes.get_mut(&tile_id).unwrap().source = changed_source;
        let second_baked = render_recipe_preview(&project, tile_id).unwrap();
        let second = render_recipe_visual_preview(&project, tile_id).unwrap();

        assert_eq!(first_baked.pixels(), second_baked.pixels());
        assert_ne!(first.pixels(), second.pixels());
    }

    #[test]
    fn stonefall_ceiling_visual_preview_changes_after_a_noise_seed_edit() {
        let mut project = Project::new();
        let material = ProceduralRecipeAsset::new(
            "dungeon",
            include_str!("../../source_projects/stonefall-dungeon/recipes/dungeon.recipe"),
        );
        let tile = ProceduralRecipeAsset::new(
            "ceiling-stone",
            include_str!("../../source_projects/stonefall-dungeon/recipes/ceiling-stone.recipe"),
        );
        let tile_id = tile.id;
        project.procedural_recipes.insert(material.id, material);
        project.procedural_recipes.insert(tile.id, tile);
        sync_recipe_compatibility_catalogs(&mut project);

        let first = render_recipe_visual_preview(&project, tile_id).unwrap();
        let source = &project.procedural_recipes[&tile_id].source;
        let value_start = source.find("seed = ").unwrap() + "seed = ".len();
        let value_end = source[value_start..]
            .find('\n')
            .map(|offset| value_start + offset)
            .unwrap_or(source.len());
        let mut changed_source = source.clone();
        changed_source.replace_range(value_start..value_end, "987654");
        project.procedural_recipes.get_mut(&tile_id).unwrap().source = changed_source;
        let second = render_recipe_visual_preview(&project, tile_id).unwrap();

        let changed_channels = first
            .pixels()
            .iter()
            .zip(second.pixels())
            .filter(|(a, b)| a != b)
            .count();
        assert!(changed_channels > first.pixels().len() / 20);
    }

    #[test]
    fn baked_tile_seeds_the_first_preview_without_parsing_again() {
        let mut project = Project::new();
        let texture = Texture::new(vec![12, 34, 56, 255], 1, 1);
        let tile = Tile::from_texture(texture);
        let tile_id = tile.id;
        project.tiles.insert(tile_id, tile);

        let mut asset = ProceduralRecipeAsset::new("baked", "not a valid recipe");
        asset.tile_id = Some(tile_id);
        let id = asset.id;
        project.procedural_recipes.insert(id, asset);
        RECIPE_PREVIEW_CACHE.write().unwrap().remove(&id);

        let preview = render_recipe_preview(&project, id).unwrap();
        assert_eq!(preview.pixels(), &[12, 34, 56, 255]);
    }
}
