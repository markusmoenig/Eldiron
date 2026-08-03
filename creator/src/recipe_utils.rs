use crate::prelude::*;
use procedural_recipes::{
    MaterialRecipe, Recipe, RecipeDocument, RecipeRenderer, RenderOptions, RenderSurface,
    RenderSurfaceFrame, RenderSurfaceMapping, SdfRenderer, parse_document,
};
use rusterix::{Texture, Tile, TileRole};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{LazyLock, RwLock};

#[derive(Clone)]
struct RecipePreviewCacheEntry {
    fingerprint: u64,
    buffer: TheRGBABuffer,
}

static RECIPE_PREVIEW_CACHE: LazyLock<RwLock<FxHashMap<Uuid, RecipePreviewCacheEntry>>> =
    LazyLock::new(|| RwLock::new(FxHashMap::default()));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProceduralRecipeKind {
    Tile,
    Material,
    Sdf,
}

impl ProceduralRecipeKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tile => "Tile Recipe",
            Self::Material => "Material Recipe",
            Self::Sdf => "SDF Recipe",
        }
    }
}

pub fn localized_recipe_kind(kind: ProceduralRecipeKind) -> String {
    match kind {
        ProceduralRecipeKind::Tile => fl!("tile_recipe"),
        ProceduralRecipeKind::Material => fl!("material_recipe"),
        ProceduralRecipeKind::Sdf => fl!("sdf_recipe"),
    }
}

pub fn recipe_description(source: &str) -> Result<(String, ProceduralRecipeKind), String> {
    match parse_document(source).map_err(|error| error.to_string())? {
        RecipeDocument::Tile(recipe) => Ok((recipe.name, ProceduralRecipeKind::Tile)),
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

/// A cheap revision used by list views to avoid rebuilding unchanged rows.
pub fn recipe_catalog_fingerprint(project: &Project) -> u64 {
    let mut hasher = DefaultHasher::new();
    hash_art_palette(project, &mut hasher);
    for (id, asset) in &project.procedural_recipes {
        id.hash(&mut hasher);
        asset.alias.hash(&mut hasher);
        asset.source.hash(&mut hasher);
        asset.tile_id.hash(&mut hasher);
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
            },
        );
        return Ok(buffer);
    }

    let buffer = render_recipe_preview_uncached(project, asset_id)?;
    RECIPE_PREVIEW_CACHE.write().unwrap().insert(
        asset_id,
        RecipePreviewCacheEntry {
            fingerprint,
            buffer: buffer.clone(),
        },
    );
    Ok(buffer)
}

fn render_recipe_preview_uncached(
    project: &Project,
    asset_id: Uuid,
) -> Result<TheRGBABuffer, String> {
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
            let mut rgba = rendered
                .frames
                .first()
                .map(|frame| frame.rgba.clone())
                .ok_or_else(|| "Recipe rendered no frames".to_string())?;
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
            Ok(TheRGBABuffer::from(rgba, rendered.width, rendered.height))
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
            Ok(TheRGBABuffer::from(
                frame.rgba.clone(),
                rendered.width,
                rendered.height,
            ))
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
            Ok(TheRGBABuffer::from(rgba, rendered.width, rendered.height))
        }
    }
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
        tile.blocking = recipe.blocking;
        tile.procedural.coverage = recipe.coverage;
        tile_id
    } else {
        let mut tile = Tile::from_texture(texture);
        tile.alias = asset.alias.clone();
        tile.role = TileRole::ManMade;
        tile.blocking = recipe.blocking;
        tile.procedural.coverage = recipe.coverage;
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
