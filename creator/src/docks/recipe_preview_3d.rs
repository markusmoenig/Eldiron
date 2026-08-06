use crate::prelude::*;
use procedural_recipes::{
    GeometryFeature, GeometryOperation, RecipeDocument, RecipePlacement, parse_document,
};
use rusterix::{
    D3Camera, D3IsoCamera, GeometryObject, GeometryObjectKind, Light, LightType, Map, MapCamera,
    PixelSource, Rusterix, SceneManager, SceneManagerResult, Tile, Value,
};
use scenevm::Atom;
use vek::Vec3;

/// An isolated Rusterix scene used by the Recipe editor. It deliberately owns
/// its renderer and SceneVM so a preview never mutates the map editor scene.
pub struct Recipe3DPreview {
    map: Map,
    renderer: Rusterix,
    animated: bool,
}

impl Recipe3DPreview {
    pub fn from_project(project: &Project, asset_id: Uuid) -> Result<Option<Self>, String> {
        let Some((map, tiles, center, scale, azimuth_deg, animated)) =
            build_preview_map(project, asset_id)?
        else {
            return Ok(None);
        };

        let renderer = Rusterix::new_without_audio();
        let mut preview = Self {
            map: Map::new(),
            renderer,
            animated: false,
        };
        preview.install(project, map, tiles, center, scale, azimuth_deg, animated);
        Ok(Some(preview))
    }

    /// Rebuild the scene while retaining the shared GPU renderer and atlas.
    /// Returns false when the edited Recipe is purely 2D.
    pub fn rebuild(&mut self, project: &Project, asset_id: Uuid) -> Result<bool, String> {
        let Some((map, tiles, center, scale, azimuth_deg, animated)) =
            build_preview_map(project, asset_id)?
        else {
            return Ok(false);
        };
        self.install(project, map, tiles, center, scale, azimuth_deg, animated);
        Ok(true)
    }

    fn install(
        &mut self,
        project: &Project,
        map: Map,
        tiles: IndexMap<Uuid, Tile>,
        center: Vec3<f32>,
        scale: f32,
        azimuth_deg: f32,
        animated: bool,
    ) {
        self.renderer.assets.palette = project.art_palette.clone();
        self.renderer.set_tiles(tiles, false);
        self.renderer.set_d3();

        let mut camera = D3IsoCamera::new();
        camera.center = center;
        camera.scale = scale;
        camera.azimuth_deg = azimuth_deg;
        camera.distance = (scale * 6.0).max(5.0);
        camera.height_clearance = scale * 1.5;
        self.renderer.client.set_camera_d3(Box::new(camera));

        install_static_geometry(&mut self.renderer, &map);
        if animated {
            for _ in 0..4 {
                self.renderer.scene_handler.tick_particle_clock_3d();
            }
        }
        self.renderer
            .build_dynamics_3d(&map, self.renderer.client.animation_frame);
        self.map = map;
        self.animated = animated;
    }

    pub fn is_animated(&self) -> bool {
        self.animated
    }

    pub fn draw(&mut self, buffer: &mut TheRGBABuffer) {
        let dim = *buffer.dim();
        if !dim.is_valid() {
            return;
        }
        if self.animated {
            self.renderer.client.inc_animation_frame();
            self.renderer.scene_handler.tick_particle_clock_3d();
            self.renderer
                .build_dynamics_3d(&self.map, self.renderer.client.animation_frame);
        }
        self.renderer.draw_d3_with_editor_background(
            &self.map,
            buffer.pixels_mut(),
            dim.width as usize,
            dim.height as usize,
            true,
        );
    }
}

fn install_static_geometry(renderer: &mut Rusterix, map: &Map) {
    let mut manager = SceneManager::new();
    manager.set_tile_list(
        renderer.assets.tile_list.clone(),
        renderer.assets.tile_indices.clone(),
    );
    manager.set_map(map.clone());
    while manager.is_busy() {
        manager.tick_batch(16);
    }
    while let Some(result) = manager.receive() {
        match result {
            SceneManagerResult::Clear => {
                renderer.scene_handler.vm.execute(Atom::ClearGeometry);
                renderer.scene_handler.billboards.clear();
                renderer.scene_handler.build_index.clear();
            }
            SceneManagerResult::Chunk(chunk, _, _, billboards) => {
                renderer
                    .scene_handler
                    .build_index
                    .remove_chunk_origin((chunk.origin.x, chunk.origin.y));
                renderer.scene_handler.vm.execute(Atom::RemoveChunkAt {
                    origin: chunk.origin,
                });
                renderer.scene_handler.build_index.index_chunk(&chunk);
                renderer.scene_handler.vm.execute(Atom::AddChunk {
                    id: Uuid::new_v4(),
                    chunk,
                });
                for billboard in billboards {
                    renderer
                        .scene_handler
                        .billboards
                        .insert(billboard.geo_id, billboard);
                }
            }
            SceneManagerResult::Startup | SceneManagerResult::Quit => {}
        }
    }
}

type PreviewMap = (Map, IndexMap<Uuid, Tile>, Vec3<f32>, f32, f32, bool);

#[derive(Clone)]
struct PreviewSubtraction {
    min: Vec3<f32>,
    max: Vec3<f32>,
    surface_id: Uuid,
}

impl PreviewSubtraction {
    fn contains(&self, point: Vec3<f32>) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }
}

fn build_preview_map(project: &Project, asset_id: Uuid) -> Result<Option<PreviewMap>, String> {
    let asset = project
        .procedural_recipes
        .get(&asset_id)
        .ok_or_else(|| "Recipe asset was not found".to_string())?;
    let RecipeDocument::Tile(recipe) =
        parse_document(&asset.source).map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    if recipe.geometry.is_empty() && recipe.lights.is_empty() && recipe.particles.is_empty() {
        return Ok(None);
    }

    let tile_id = asset
        .tile_id
        .ok_or_else(|| "Recipe preview tile was not baked".to_string())?;
    let recipe_tile = project
        .tiles
        .get(&tile_id)
        .cloned()
        .ok_or_else(|| "Recipe preview tile was not found".to_string())?;
    let host_tile_id = if recipe.placement == RecipePlacement::Fixture {
        resolve_fixture_preview_host(project, tile_id, &recipe_tile).unwrap_or(tile_id)
    } else {
        tile_id
    };
    let mut tiles = IndexMap::new();
    tiles.insert(tile_id, recipe_tile.clone());
    if host_tile_id != tile_id
        && let Some(host) = project.tiles.get(&host_tile_id)
    {
        tiles.insert(host_tile_id, host.clone());
    }

    let mut map = Map::new();
    map.name = format!("{} Preview", recipe.name);
    map.camera = MapCamera::ThreeDIso;
    let mut min = Vec3::broadcast(f32::INFINITY);
    let mut max = Vec3::broadcast(f32::NEG_INFINITY);
    // Recipe geometry uses X/Y for wall-local features and X/Z for
    // floor/ceiling-local features. Keep the authored extents separate from
    // the camera bounds so we can add the correct contextual tile surface.
    let mut authored_min = Vec3::zero();
    let mut authored_max = Vec3::zero();
    let mut subtractions = Vec::new();

    for feature in &recipe.geometry {
        let GeometryFeature::Box(geometry_box) = feature;
        let surface_id = resolve_surface_tile(project, &geometry_box.surface).ok_or_else(|| {
            format!(
                "Geometry '{}' references unknown surface tile '{}'",
                geometry_box.name, geometry_box.surface
            )
        })?;
        if let Some(tile) = project.tiles.get(&surface_id) {
            tiles.entry(surface_id).or_insert_with(|| tile.clone());
        }
        for rx in 0..geometry_box.repeat[0] {
            for ry in 0..geometry_box.repeat[1] {
                for rz in 0..geometry_box.repeat[2] {
                    let position = Vec3::from(geometry_box.position)
                        + Vec3::new(
                            rx as f32 * geometry_box.spacing[0],
                            ry as f32 * geometry_box.spacing[1],
                            rz as f32 * geometry_box.spacing[2],
                        );
                    let size = Vec3::from(geometry_box.size);
                    let box_min = position;
                    let box_max = position + size;
                    include_bounds(&mut authored_min, &mut authored_max, box_min);
                    include_bounds(&mut authored_min, &mut authored_max, box_max);
                    match geometry_box.operation {
                        GeometryOperation::Add => {
                            let mut object = GeometryObject::box_from_bounds(
                                geometry_box.name.clone(),
                                box_min,
                                box_max,
                            );
                            for face in &mut object.faces {
                                face.tile = Some(PixelSource::TileId(surface_id));
                            }
                            map.geometry_objects.push(object);
                            include_bounds(&mut min, &mut max, box_min);
                            include_bounds(&mut min, &mut max, box_max);
                        }
                        GeometryOperation::Subtract => {
                            subtractions.push(PreviewSubtraction {
                                min: box_min,
                                max: box_max,
                                surface_id,
                            });
                        }
                    }
                }
            }
        }
    }
    for attachment in &recipe.attachments {
        include_bounds(
            &mut authored_min,
            &mut authored_max,
            Vec3::from(attachment.position),
        );
    }
    if !subtractions.is_empty() {
        add_subtracted_preview_host(&mut map, host_tile_id, &subtractions, &mut min, &mut max);
    } else {
        add_contextual_preview_host(
            &mut map,
            host_tile_id,
            authored_min,
            authored_max,
            &mut min,
            &mut max,
        );
    }

    let attachment = |name: &str| {
        recipe
            .attachments
            .iter()
            .find(|attachment| attachment.name.eq_ignore_ascii_case(name))
    };
    for light in &recipe.lights {
        let Some(anchor) = attachment(&light.attachment) else {
            continue;
        };
        let position = Vec3::from(anchor.position) + Vec3::new(0.0, light.lift, 0.0);
        let range = light.range.max(0.1);
        map.lights.push(
            Light::new(LightType::Point)
                .with_position(position)
                .with_color([
                    light.color[0] as f32 / 255.0,
                    light.color[1] as f32 / 255.0,
                    light.color[2] as f32 / 255.0,
                ])
                .with_intensity(light.intensity.max(0.0))
                .with_start_distance((range * 0.15).min(0.75))
                .with_end_distance(range)
                .with_flicker(light.flicker.max(0.0)),
        );
        include_bounds(&mut min, &mut max, position);
    }
    for particles in &recipe.particles {
        let Some(anchor) = attachment(&particles.attachment) else {
            continue;
        };
        let position = Vec3::from(anchor.position);
        let direction = Vec3::from(particles.direction)
            .try_normalized()
            .unwrap_or(Vec3::unit_y());
        let epsilon = 0.002;
        let mut object = GeometryObject::box_from_bounds(
            format!("recipe_effect_{}", particles.name),
            position - Vec3::broadcast(epsilon),
            position + Vec3::broadcast(epsilon),
        );
        object.kind = GeometryObjectKind::Generated;
        object.solid = false;
        object.group = "eldiron-recipe-effects".to_string();
        for face in &mut object.faces {
            face.tile = Some(PixelSource::Off);
        }
        object.properties.set(
            "recipe_effect_source",
            Value::Source(PixelSource::TileId(tile_id)),
        );
        object
            .properties
            .set("recipe_particle_effect", Value::Str(particles.name.clone()));
        object.properties.set(
            "recipe_effect_origin",
            Value::Vec3([position.x, position.y, position.z]),
        );
        object.properties.set(
            "recipe_effect_direction",
            Value::Vec3([direction.x, direction.y, direction.z]),
        );
        map.geometry_objects.push(object);
        include_bounds(&mut min, &mut max, position);
        let travel = particles.speed[1].max(0.0) * particles.lifetime[1].max(0.0)
            + particles.radius[1].max(0.0) * 2.0;
        include_bounds(&mut min, &mut max, position + direction * travel);
    }

    if !min.x.is_finite() {
        min = Vec3::broadcast(-0.5);
        max = Vec3::broadcast(0.5);
    }
    let center = (min + max) * 0.5;
    let span = max - min;
    let scale = (span.x.max(span.y).max(span.z) * 0.72).max(0.55);
    // Wall-local recipes protrude toward negative Z. View those from their
    // authored outward side instead of through the contextual wall slab.
    let azimuth_deg = if preview_host_is_horizontal(authored_min, authored_max) {
        135.0
    } else {
        225.0
    };
    let animated = !recipe.particles.is_empty() || recipe.animation.frames > 1;
    Ok(Some((map, tiles, center, scale, azimuth_deg, animated)))
}

/// A Fixture never owns its host. For an isolated preview, prefer the largest
/// ordinary surface Tile using the same authored material; this lets fixtures
/// such as Stonefall's torch appear on the real continuous wall Tile. The
/// recipe carrier Tile is only a final fallback when no context is available.
fn resolve_fixture_preview_host(
    project: &Project,
    fixture_tile_id: Uuid,
    fixture_tile: &Tile,
) -> Option<Uuid> {
    let material_alias = fixture_tile.material_alias.trim();
    let candidates = || {
        project.tiles.iter().filter(|(id, tile)| {
            **id != fixture_tile_id
                && tile.recipe_placement == rusterix::TileRecipePlacement::Surface
                && tile.geometry.is_empty()
        })
    };
    let coverage =
        |tile: &Tile| tile.procedural.coverage[0].max(1) * tile.procedural.coverage[1].max(1);

    if !material_alias.is_empty()
        && let Some((id, _)) = candidates()
            .filter(|(_, tile)| tile.material_alias.eq_ignore_ascii_case(material_alias))
            .max_by_key(|(_, tile)| coverage(tile))
    {
        return Some(*id);
    }

    // Material graphs do not always expose the same base alias as their final
    // surface Tile. Prefer the largest compatible solid surface as a neutral
    // project context (Stonefall's 4x4 wall is selected this way).
    candidates()
        .filter(|(_, tile)| tile.blocking == fixture_tile.blocking)
        .max_by_key(|(_, tile)| coverage(tile))
        .map(|(id, _)| *id)
}

fn preview_host_is_horizontal(authored_min: Vec3<f32>, authored_max: Vec3<f32>) -> bool {
    let y_extent = authored_min.y.abs().max(authored_max.y.abs());
    let z_extent = authored_min.z.abs().max(authored_max.z.abs());
    z_extent > y_extent
}

/// Adds a contextual host surface behind additive geometry, lights, and
/// particles. Surface recipes use their baked Tile; fixtures prefer an
/// existing compatible project surface. In recipe-local coordinates X is the shared
/// horizontal axis: wall features extend primarily along Y and protrude on Z,
/// while floor/ceiling features extend primarily along Z and protrude on Y.
fn add_contextual_preview_host(
    map: &mut Map,
    base_tile_id: Uuid,
    authored_min: Vec3<f32>,
    authored_max: Vec3<f32>,
    preview_min: &mut Vec3<f32>,
    preview_max: &mut Vec3<f32>,
) {
    let thickness = 0.08;

    let (base_min, base_max) = if preview_host_is_horizontal(authored_min, authored_max) {
        // Horizontal surface (floor/ceiling): keep its visible face at Y = 0
        // and put the thin slab opposite the authored feature.
        let (min_y, max_y) = if authored_min.y.abs() > authored_max.y.abs() {
            (0.0, thickness)
        } else {
            (-thickness, 0.0)
        };
        (
            Vec3::new(authored_min.x.min(0.0), min_y, authored_min.z.min(0.0)),
            Vec3::new(authored_max.x.max(1.0), max_y, authored_max.z.max(1.0)),
        )
    } else {
        // Vertical wall surface: wall recipes use Z = 0 as their attachment
        // plane. A standard wall-cell height gives the tile enough context for
        // fixtures such as torches without dwarfing them.
        let (min_z, max_z) = if authored_min.z.abs() > authored_max.z.abs() {
            (0.0, thickness)
        } else {
            (-thickness, 0.0)
        };
        (
            Vec3::new(authored_min.x.min(0.0), authored_min.y.min(0.0), min_z),
            Vec3::new(authored_max.x.max(1.0), authored_max.y.max(2.4), max_z),
        )
    };

    let mut object = GeometryObject::box_from_bounds("Recipe Surface", base_min, base_max);
    for face in &mut object.faces {
        face.tile = Some(PixelSource::TileId(base_tile_id));
    }
    map.geometry_objects.insert(0, object);
    include_bounds(preview_min, preview_max, base_min);
    include_bounds(preview_min, preview_max, base_max);
}

fn add_subtracted_preview_host(
    map: &mut Map,
    base_tile_id: Uuid,
    subtractions: &[PreviewSubtraction],
    preview_min: &mut Vec3<f32>,
    preview_max: &mut Vec3<f32>,
) {
    let base_min = Vec3::zero();
    let base_max = Vec3::new(
        subtractions
            .iter()
            .map(|volume| volume.max.x)
            .fold(1.0_f32, f32::max),
        subtractions
            .iter()
            .map(|volume| volume.max.y + 0.25)
            .fold(2.4_f32, f32::max),
        subtractions
            .iter()
            .map(|volume| volume.max.z)
            .fold(0.5_f32, f32::max),
    );
    let mut x_cuts = vec![base_min.x, base_max.x];
    let mut y_cuts = vec![base_min.y, base_max.y];
    let mut z_cuts = vec![base_min.z, base_max.z];
    for volume in subtractions {
        x_cuts.extend([
            volume.min.x.clamp(base_min.x, base_max.x),
            volume.max.x.clamp(base_min.x, base_max.x),
        ]);
        y_cuts.extend([
            volume.min.y.clamp(base_min.y, base_max.y),
            volume.max.y.clamp(base_min.y, base_max.y),
        ]);
        z_cuts.extend([
            volume.min.z.clamp(base_min.z, base_max.z),
            volume.max.z.clamp(base_min.z, base_max.z),
        ]);
    }
    sort_cuts(&mut x_cuts);
    sort_cuts(&mut y_cuts);
    sort_cuts(&mut z_cuts);

    let mut part = 0;
    for x in x_cuts.windows(2) {
        for y in y_cuts.windows(2) {
            for z in z_cuts.windows(2) {
                let min = Vec3::new(x[0], y[0], z[0]);
                let max = Vec3::new(x[1], y[1], z[1]);
                let center = (min + max) * 0.5;
                if subtractions.iter().any(|volume| volume.contains(center)) {
                    continue;
                }
                let mut object =
                    GeometryObject::box_from_bounds(format!("Recipe Host {part}"), min, max);
                for face in &mut object.faces {
                    face.tile = Some(PixelSource::TileId(base_tile_id));
                }
                let epsilon = 0.0001;
                let samples = [
                    Vec3::new(center.x, center.y, min.z - epsilon),
                    Vec3::new(center.x, center.y, max.z + epsilon),
                    Vec3::new(min.x - epsilon, center.y, center.z),
                    Vec3::new(max.x + epsilon, center.y, center.z),
                    Vec3::new(center.x, max.y + epsilon, center.z),
                    Vec3::new(center.x, min.y - epsilon, center.z),
                ];
                for (face_index, sample) in samples.into_iter().enumerate() {
                    if let Some(volume) = subtractions.iter().find(|volume| volume.contains(sample))
                        && let Some(face) = object.faces.get_mut(face_index)
                    {
                        face.tile = Some(PixelSource::TileId(volume.surface_id));
                    }
                }
                map.geometry_objects.push(object);
                part += 1;
            }
        }
    }
    include_bounds(preview_min, preview_max, base_min);
    include_bounds(preview_min, preview_max, base_max);
}

fn sort_cuts(cuts: &mut Vec<f32>) {
    cuts.sort_by(|a, b| a.total_cmp(b));
    cuts.dedup_by(|a, b| (*a - *b).abs() <= 0.0001);
}

fn resolve_surface_tile(project: &Project, requested: &str) -> Option<Uuid> {
    let requested = requested.trim().trim_matches('"').to_ascii_lowercase();
    let mut candidates = vec![requested.clone(), requested.replace('.', "/")];
    if let Some(leaf) = requested.rsplit(['/', '.']).next() {
        candidates.push(leaf.to_string());
    }
    project.tiles.iter().find_map(|(id, tile)| {
        let alias = tile.alias.trim().to_ascii_lowercase();
        let leaf = alias.rsplit('/').next().unwrap_or(alias.as_str());
        candidates
            .iter()
            .any(|candidate| alias == *candidate || leaf == candidate)
            .then_some(*id)
    })
}

fn include_bounds(min: &mut Vec3<f32>, max: &mut Vec3<f32>, point: Vec3<f32>) {
    min.x = min.x.min(point.x);
    min.y = min.y.min(point.y);
    min.z = min.z.min(point.z);
    max.x = max.x.max(point.x);
    max.y = max.y.max(point.y);
    max.z = max.z.max(point.z);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe_utils::rebake_tile_recipe;

    #[test]
    fn geometry_particle_recipe_builds_isolated_preview_map() {
        let mut project = Project::new();
        let surface = Tile::from_texture(rusterix::Texture::from_color([110, 90, 70, 255]));
        let surface_id = surface.id;
        let mut surface = surface;
        surface.alias = "surface".to_string();
        project.tiles.insert(surface_id, surface);
        let asset = ProceduralRecipeAsset::new(
            "preview",
            r#"Tile
    name = "Preview"
    size = I2(8, 8)
    Geometry
        Box Body
            operation = Add
            surface = surface
            position = F3(0.0, 0.0, 0.0)
            size = F3(1.0, 1.0, 1.0)
    Attachment Flame
        position = F3(0.5, 1.1, 0.5)
        direction = F3(0.0, 1.0, 0.0)
    Particles Flame
        attach = Flame
        direction = F3(0.0, 1.0, 0.0)
        rate = 10.0
        color = #ff8844
    Output
        height = 0.5
"#,
        );
        let asset_id = asset.id;
        project.procedural_recipes.insert(asset_id, asset);
        rebake_tile_recipe(&mut project, asset_id).unwrap();
        let recipe_tile_id = project.procedural_recipes[&asset_id].tile_id.unwrap();
        let recipe_tile = &project.tiles[&recipe_tile_id];
        assert_eq!(recipe_tile.geometry.len(), 1);
        assert_eq!(recipe_tile.attachments.len(), 1);
        assert_eq!(recipe_tile.particle_effects.len(), 1);

        let (map, tiles, _, _, _, animated) = build_preview_map(&project, asset_id)
            .unwrap()
            .expect("3D preview");
        assert!(animated);
        assert_eq!(map.lights.len(), 0);
        assert_eq!(map.geometry_objects.len(), 3);
        assert!(tiles.contains_key(&surface_id));
        assert!(map.geometry_objects.iter().any(|object| {
            object
                .faces
                .iter()
                .all(|face| face.tile == Some(PixelSource::TileId(recipe_tile_id)))
        }));
        assert!(map.geometry_objects.iter().any(|object| {
            object.properties.get_str("recipe_particle_effect") == Some("Flame")
        }));
    }

    #[test]
    fn ceiling_geometry_gets_a_horizontal_recipe_surface() {
        let mut project = Project::new();
        let mut beam = Tile::from_texture(rusterix::Texture::from_color([90, 60, 40, 255]));
        beam.alias = "beam".to_string();
        let beam_id = beam.id;
        project.tiles.insert(beam_id, beam);
        let asset = ProceduralRecipeAsset::new(
            "ceiling",
            r#"Tile
    name = "Ceiling"
    size = I2(8, 8)
    Geometry
        Box Beam
            operation = Add
            surface = beam
            position = F3(0.4, 0.0, 0.0)
            size = F3(0.2, 0.12, 1.0)
    Output
        height = 0.5
"#,
        );
        let asset_id = asset.id;
        project.procedural_recipes.insert(asset_id, asset);
        rebake_tile_recipe(&mut project, asset_id).unwrap();
        let recipe_tile_id = project.procedural_recipes[&asset_id].tile_id.unwrap();

        let (map, _, _, _, _, _) = build_preview_map(&project, asset_id)
            .unwrap()
            .expect("ceiling 3D preview");
        let surface = map
            .geometry_objects
            .iter()
            .find(|object| {
                object
                    .faces
                    .iter()
                    .all(|face| face.tile == Some(PixelSource::TileId(recipe_tile_id)))
            })
            .expect("recipe surface");
        let surface_min_y = surface
            .vertices
            .iter()
            .map(|vertex| vertex.y)
            .fold(f32::INFINITY, f32::min);
        let surface_max_y = surface
            .vertices
            .iter()
            .map(|vertex| vertex.y)
            .fold(f32::NEG_INFINITY, f32::max);
        assert_eq!(surface_min_y, -0.08);
        assert_eq!(surface_max_y, 0.0);
    }

    #[test]
    fn wall_fixture_uses_a_compatible_project_surface_as_preview_context() {
        let mut project = Project::new();
        let mut wall = Tile::from_texture(rusterix::Texture::from_color([65, 65, 70, 255]));
        wall.alias = "wall".to_string();
        wall.material_alias = "stone".to_string();
        wall.procedural.coverage = [4, 4];
        let wall_id = wall.id;
        project.tiles.insert(wall_id, wall);
        let mut fixture = Tile::from_texture(rusterix::Texture::from_color([80, 50, 30, 255]));
        fixture.alias = "fixture".to_string();
        let fixture_id = fixture.id;
        project.tiles.insert(fixture_id, fixture);
        let asset = ProceduralRecipeAsset::new(
            "wall",
            r#"Tile
    name = "Wall Fixture"
    placement = Fixture
    size = I2(8, 8)
    Geometry
        Box Fixture
            operation = Add
            surface = fixture
            position = F3(0.4, 1.1, -0.4)
            size = F3(0.2, 0.4, 0.4)
    Output
        height = 0.5
"#,
        );
        let asset_id = asset.id;
        project.procedural_recipes.insert(asset_id, asset);
        rebake_tile_recipe(&mut project, asset_id).unwrap();
        let recipe_tile_id = project.procedural_recipes[&asset_id].tile_id.unwrap();
        project
            .tiles
            .get_mut(&recipe_tile_id)
            .unwrap()
            .material_alias = "stone".to_string();

        let (map, tiles, _, _, azimuth_deg, _) = build_preview_map(&project, asset_id)
            .unwrap()
            .expect("wall 3D preview");
        assert!(tiles.contains_key(&wall_id));
        let surface = map
            .geometry_objects
            .iter()
            .find(|object| {
                object
                    .faces
                    .iter()
                    .all(|face| face.tile == Some(PixelSource::TileId(wall_id)))
            })
            .expect("fixture context surface");
        assert!(map.geometry_objects.iter().all(|object| {
            !object
                .faces
                .iter()
                .all(|face| face.tile == Some(PixelSource::TileId(recipe_tile_id)))
        }));
        let surface_min_z = surface
            .vertices
            .iter()
            .map(|vertex| vertex.z)
            .fold(f32::INFINITY, f32::min);
        let surface_max_z = surface
            .vertices
            .iter()
            .map(|vertex| vertex.z)
            .fold(f32::NEG_INFINITY, f32::max);
        assert_eq!(surface_min_z, 0.0);
        assert_eq!(surface_max_z, 0.08);
        assert_eq!(azimuth_deg, 225.0);
    }

    #[test]
    fn subtract_recipe_builds_a_carved_preview_host() {
        let mut project = Project::new();
        let mut cavity = Tile::from_texture(rusterix::Texture::from_color([80, 70, 60, 255]));
        cavity.alias = "cavity".to_string();
        let cavity_id = cavity.id;
        project.tiles.insert(cavity_id, cavity);
        let asset = ProceduralRecipeAsset::new(
            "carved",
            r#"Tile
    name = "Carved"
    size = I2(8, 8)
    Geometry
        Box Recess
            operation = Subtract
            surface = cavity
            position = F3(0.2, 0.6, 0.0)
            size = F3(0.6, 1.0, 0.4)
    Output
        height = 0.5
"#,
        );
        let asset_id = asset.id;
        project.procedural_recipes.insert(asset_id, asset);
        rebake_tile_recipe(&mut project, asset_id).unwrap();

        let (map, _, _, _, _, _) = build_preview_map(&project, asset_id)
            .unwrap()
            .expect("carved 3D preview");
        assert!(map.geometry_objects.len() > 1);
        assert!(map.geometry_objects.iter().any(|object| {
            object
                .faces
                .iter()
                .any(|face| face.tile == Some(PixelSource::TileId(cavity_id)))
        }));
    }
}
