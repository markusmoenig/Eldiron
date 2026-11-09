//! MZ assets baked with a shared, foot-anchored 2:1 isometric projection.
pub mod placement;
pub mod snapping;
use crate::{project::Project, ruleset_materials};
use denrim_materials::{
    geometry::{Document, Mesh, Part, Value as GeoValue, Vertex},
    integration::EditableMaterialAsset,
};
use rusterix::Texture;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use theframework::prelude::*;
const HALF_W: f32 = 32.;
const HALF_H: f32 = 16.;
const VERTICAL: f32 = 39.1918359;

pub fn draw_preview(buffer: &mut TheRGBABuffer, image: &Texture, checker: [[u8; 4]; 2]) {
    let width = buffer.dim().width.max(0) as usize;
    let height = buffer.dim().height.max(0) as usize;
    if width == 0 || height == 0 || image.width == 0 || image.height == 0 {
        return;
    }
    let scale = (width as f32 / (image.width as f32 + 16.))
        .min(height as f32 / (image.height as f32 + 16.));
    let size = Vec2::new(image.width as f32, image.height as f32) * scale;
    let origin = (Vec2::new(width as f32, height as f32) - size) * 0.5;
    for y in 0..height {
        for x in 0..width {
            let mut color = checker[(x / 12 + y / 12) % 2];
            let sx = (x as f32 - origin.x) / scale;
            let sy = (y as f32 - origin.y) / scale;
            if sx >= 0. && sy >= 0. && sx < image.width as f32 && sy < image.height as f32 {
                let i = (sy as usize * image.width + sx as usize) * 4;
                let a = image.data[i + 3] as u32;
                for k in 0..3 {
                    color[k] = ((image.data[i + k] as u32 * a + color[k] as u32 * (255 - a) + 127)
                        / 255) as u8;
                }
            }
            let i = (y * width + x) * 4;
            buffer.pixels_mut()[i..i + 4].copy_from_slice(&color);
        }
    }
}

/// A reusable tile bake, referencing a project-owned MZ recipe. Generated pixels
/// live in Project::tiles; the source and settings remain editable and portable.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bake {
    pub recipe: Uuid,
    pub ground: bool,
    pub yaw: f32,
    pub directions: usize,
    pub motion: Option<String>,
    pub frames: usize,
    #[serde(default)]
    pub signature: u64,
}

pub fn synchronize(project: &mut Project) -> Vec<String> {
    let iso = project
        .regions
        .iter()
        .any(|r| r.map.camera == rusterix::MapCamera::TwoDIso);
    if iso {
        discover_models(project);
    }
    let mut errors = Vec::new();
    let bindings = project.isometric_bakes.clone();
    for (id, settings) in bindings {
        let result = (|| -> Result<(), String> {
            let recipe = if settings.ground {
                project.procedural_materials.get(&settings.recipe)
            } else {
                project.procedural_appearances.get(&settings.recipe)
            }
            .ok_or("MZ recipe missing")?;
            let mut signature = rusterix::procedural_avatar::fingerprint(&(
                if settings.ground { 4u32 } else { 14u32 },
                if settings.ground {
                    recipe.clone()
                } else {
                    crate::appearance_assets::resolve_dependencies(
                        project,
                        &Document::load(recipe).map_err(|e| e.to_string())?,
                    )?
                    .save()
                    .map_err(|e| e.to_string())?
                },
                serde_json::to_string(&project.palette.colors).map_err(|e| e.to_string())?,
                material_inputs(project, recipe, settings.ground)?,
                settings.yaw.to_bits(),
                settings.directions,
                &settings.motion,
                settings.frames,
                settings.ground,
            ));
            // Continuous materials own world-coordinate pages at the source
            // UUID. Their projected output must follow that cache, rather than
            // repeating one diamond over every ground cell.
            let pages = settings
                .ground
                .then(|| project.tiles.get(&settings.recipe))
                .flatten()
                .filter(|tile| tile.uv_pages.is_some() && !tile.textures.is_empty());
            if let Some(tile) = pages {
                signature = rusterix::procedural_avatar::fingerprint(&(
                    signature,
                    5u32,
                    project.procedural_material_cache_keys.get(&settings.recipe),
                    serde_json::to_string(&tile.uv_pages).map_err(|e| e.to_string())?,
                ));
            }
            if signature == settings.signature
                && project
                    .tiles
                    .get(&id)
                    .is_some_and(|t| t.isometric.is_some() && !t.textures.is_empty())
            {
                return Ok(());
            }
            let mut generated = if let Some(source) = pages {
                let mut tile = source.clone();
                tile.isometric = Some(rusterix::isometric::SpriteLayout {
                    ground: true,
                    anchor: [32, 16],
                    pixels_per_unit: HALF_W,
                    directions: 1,
                    frames_per_direction: 1,
                    frames: vec![rusterix::isometric::ground_sprite(&source.textures[0])],
                    ..Default::default()
                });
                tile
            } else {
                bake(project, &settings)?
            };
            if let Some(previous) = project.tiles.get(&id) {
                generated.id = id;
                generated.alias = previous.alias.clone();
                generated.role = previous.role;
                generated.blocking = previous.blocking;
                generated.gameplay_tags = previous.gameplay_tags.clone();
            } else {
                generated.id = id;
            }
            project.tiles.insert(id, generated);
            project.isometric_bakes.get_mut(&id).unwrap().signature = signature;
            Ok(())
        })();
        if let Err(error) = result {
            errors.push(format!("Isometric tile {id}: {error}"));
        }
    }
    if iso {
        project_material_tiles(project);
        // Store anchors relative to editable footprints so dragging a placement
        // moves the art and its collision together.
        for region in &mut project.regions {
            let anchors = region
                .map
                .sectors
                .iter()
                .filter_map(|s| {
                    let anchor = s.properties.get_vec2("iso_anchor")?;
                    let center = s.center(&region.map)?;
                    Some((s.id, [anchor[0] - center.x, anchor[1] - center.y]))
                })
                .collect::<Vec<_>>();
            for (id, offset) in anchors {
                if let Some(sector) = region.map.find_sector_mut(id) {
                    sector
                        .properties
                        .set("iso_anchor_offset", rusterix::Value::Vec2(offset));
                    sector.properties.remove("iso_anchor");
                }
            }
        }
    }
    placement::synchronize(project);
    errors
}

/// Authored recipe behind a tile. Rotated outputs can share one source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileRecipe {
    Material(Uuid),
    Geometry(Uuid),
}

pub fn tile_recipe(project: &Project, tile: Uuid) -> Option<TileRecipe> {
    if let Some(binding) = project.isometric_bakes.get(&tile) {
        return if binding.ground {
            project
                .procedural_materials
                .contains_key(&binding.recipe)
                .then_some(TileRecipe::Material(binding.recipe))
        } else {
            project
                .procedural_appearances
                .contains_key(&binding.recipe)
                .then_some(TileRecipe::Geometry(binding.recipe))
        };
    }
    project
        .procedural_materials
        .contains_key(&tile)
        .then_some(TileRecipe::Material(tile))
}

pub fn is_material_source(project: &Project, recipe: Uuid) -> bool {
    project
        .isometric_bakes
        .values()
        .any(|binding| binding.ground && binding.recipe == recipe)
}

fn material_inputs(
    project: &Project,
    source: &str,
    ground: bool,
) -> Result<Vec<(String, String)>, String> {
    if ground {
        return Ok(Vec::new());
    } // Ground source is already in the bake key.
    fn collect(doc: &Document, sources: &mut std::collections::BTreeSet<String>) {
        use denrim_materials::geometry::Operation;
        for tile in doc.rows.iter().flat_map(|r| &r.tiles) {
            match &tile.op {
                Operation::Texture { source, .. } => {
                    sources.insert(source.clone());
                }
                Operation::Asset { document, .. }
                | Operation::Group { document, .. }
                | Operation::Input {
                    preview: document, ..
                } => collect(document, sources),
                _ => {}
            }
        }
    }
    let doc = crate::appearance_assets::resolve_dependencies(
        project,
        &Document::load(source).map_err(|e| e.to_string())?,
    )?;
    let mut sources = std::collections::BTreeSet::new();
    collect(&doc, &mut sources);
    let defs = ruleset_materials::definitions(&project.config);
    Ok(sources
        .into_iter()
        .map(|source| {
            let id = source.strip_prefix("project:material/").unwrap_or(&source);
            let id = Uuid::parse_str(id).ok().or_else(|| {
                source
                    .strip_prefix("ruleset:material/")
                    .map(crate::rulesets::material_asset_id)
            });
            let bundle = id
                .and_then(|id| project.procedural_materials.get(&id))
                .cloned()
                .unwrap_or_else(|| {
                    source
                        .strip_prefix("ruleset:material/")
                        .and_then(|name| defs.iter().find(|d| d.id == name))
                        .map(|d| format!("{}:{}", d.source, d.palette))
                        .unwrap_or_default()
                });
            (source, bundle)
        })
        .collect())
}

fn discover_models(project: &mut Project) {
    let selection = crate::rulesets::selected_ruleset_config(&project.config);
    let library =
        crate::rulesets::bundled_appearance_assets_for_ruleset(&selection.id, &selection.version)
            .into_iter()
            .filter_map(|a| Uuid::parse_str(a.asset_id).ok())
            .collect::<std::collections::HashSet<_>>();
    let models = project
        .procedural_appearances
        .iter()
        .filter(|(id, _)| {
            !library.contains(id) && !project.isometric_bakes.values().any(|b| b.recipe == **id)
        })
        .filter_map(|(id, source)| {
            let doc = Document::load(source).ok()?;
            if crate::appearance_assets::resolve_dependencies(project, &doc)
                .ok()?
                .evaluate()
                .ok()?
                .mesh
                .parts
                .is_empty()
            {
                return None;
            }
            let motion = doc
                .rows
                .iter()
                .find(|r| {
                    r.id.eq_ignore_ascii_case("walk")
                        && r.tiles.iter().any(|t| {
                            matches!(t.op, denrim_materials::geometry::Operation::Motion { .. })
                        })
                })
                .map(|r| r.id.clone());
            Some((*id, doc.label, motion))
        })
        .collect::<Vec<_>>();
    for (id, label, motion) in models {
        project.isometric_bakes.insert(
            id,
            Bake {
                recipe: id,
                ground: false,
                yaw: 0.,
                directions: if motion.is_some() { 8 } else { 4 },
                frames: 16,
                motion,

                signature: 0,
            },
        );
        if !project.tiles.contains_key(&id) {
            let mut tile = rusterix::Tile::empty();
            tile.id = id;
            tile.alias = label;
            project.tiles.insert(id, tile);
            project.place_tile_at_picker_selection(id, None);
        }
    }
}

/// Cache projected material tiles without replacing the flat textures used by
/// MZ itself, top-down maps or live 3D surfaces in the same project.
fn project_material_tiles(project: &mut Project) {
    for id in project.procedural_materials.keys() {
        if project.isometric_bakes.contains_key(id) {
            continue;
        }
        let Some(tile) = project.tiles.get_mut(id) else {
            continue;
        };
        if tile.textures.is_empty() {
            continue;
        }
        let signature = rusterix::procedural_avatar::fingerprint(&(
            2u32,
            tile.textures
                .iter()
                .map(|t| (t.width, t.height, &t.data))
                .collect::<Vec<_>>(),
        ));
        if tile
            .isometric
            .as_ref()
            .is_some_and(|s| s.source_signature == signature)
        {
            continue;
        }
        let count = tile
            .uv_pages
            .as_ref()
            .map_or(tile.textures.len(), |p| p.animation_frames as usize)
            .max(1);
        let frames = (0..count)
            .filter_map(|f| tile.textures.get(tile.frame_index(f)))
            .map(rusterix::isometric::ground_sprite)
            .collect::<Vec<_>>();
        tile.isometric = Some(rusterix::isometric::SpriteLayout {
            anchor: [32, 16],
            pixels_per_unit: 32.,
            ground: true,
            directions: 1,
            frames_per_direction: frames.len(),
            walk_frames: frames.len(),
            frames_per_second: 4.,
            frames,
            source_signature: signature,
            bitmap:false,prelit:false,
            surfaces: Vec::new(),
            top_material: None,
            placement_bounds: Vec::new(),
            placement_contours: Vec::new(),
            placement_shapes: Vec::new(),
            placement_connectors: Vec::new(),
        });
    }
}

pub fn bake(project: &Project, settings: &Bake) -> Result<rusterix::Tile, String> {
    if !settings.yaw.is_finite()
        || !(1..=8).contains(&settings.directions)
        || !(1..=64).contains(&settings.frames)
    {
        return Err("Isometric bake requires finite yaw, 1–8 directions and 1–64 frames".into());
    }
    let mut baker = Baker {
        project,
        textures: BTreeMap::new(),
    };
    let mut generated = Vec::new();
    let mut surfaces = Vec::new();
    let mut anchors = Vec::new();
    let mut duration = 1.;
    let mut top_material = None;
    let mut placement_bounds = Vec::new();
    let mut placement_contours = Vec::new();
    let mut placement_shapes = Vec::new();
    let mut placement_connectors = Vec::new();
    if settings.ground {
        let key = format!("project:material/{}", settings.recipe);
        let image = baker.bake("ground", &plane(&key), settings.yaw, false)?;
        anchors.push(image.meta.anchor);
        generated.push(Texture::new(image.rgba, image.meta.w, image.meta.h));
    } else {
        let source = project
            .procedural_appearances
            .get(&settings.recipe)
            .ok_or("MZ geometry recipe missing")?;
        let doc = crate::appearance_assets::resolve_dependencies(
            project,
            &Document::load(source).map_err(|e| e.to_string())?,
        )?;
        let result = doc.evaluate().map_err(|e| e.to_string())?;
        if settings.motion.is_none() {
            top_material = unit_top_material(&result.mesh);
        }
        let motion = match &settings.motion {
            Some(name) => match result.rows.get(name) {
                Some(GeoValue::Motion(m)) => Some(m),
                _ => return Err(format!("MZ motion {name} missing")),
            },
            None => None,
        };
        if let Some(m) = motion {
            duration = m.duration;
        }
        for direction in 0..settings.directions {
            if motion.is_none() {
                let yaw = settings.yaw + direction as f32 * 360. / settings.directions as f32;
                placement_connectors.push(
                    result
                        .mesh
                        .connectors
                        .iter()
                        .cloned()
                        .map(|mut c| {
                            c.position = rotate(c.position, yaw);
                            c.direction = rotate(c.direction, yaw);
                            c
                        })
                        .collect(),
                );
                if let Some(shape) = &result.mesh.footprint {
                    let yaw = settings.yaw + direction as f32 * 360. / settings.directions as f32;
                    let shape = shape.map(|p| {
                        let p = rotate(*p, yaw);
                        [p[0], p[2]]
                    });
                    denrim_materials::geometry::footprint::validate(&shape)
                        .map_err(|e| e.to_string())?;
                    if shape.regions.len() == 1 && shape.regions[0].holes.is_empty() {
                        placement_contours.push(shape.regions[0].outer.clone());
                    }
                    placement_shapes.push(shape);
                }
                if let Some(bounds) = placement::mesh_bounds(
                    &result.mesh,
                    settings.yaw + direction as f32 * 360. / settings.directions as f32,
                ) {
                    placement_bounds.push(bounds);
                }
            }
            let count = if motion.is_some() {
                settings.frames + 1
            } else {
                1
            };
            for frame in 0..count {
                let mesh = if frame < settings.frames && motion.is_some() {
                    result
                        .mesh
                        .posed(motion, frame as f32 / settings.frames as f32 * duration)
                        .map_err(|e| e.to_string())?
                        .mesh
                } else {
                    result.mesh.clone()
                };
                let image = baker.bake(
                    "sprite",
                    &mesh,
                    settings.yaw + direction as f32 * 360. / settings.directions as f32,
                    motion.is_some(),
                )?;
                anchors.push(image.meta.anchor);
                surfaces.push(image.surface);
                generated.push(Texture::new(image.rgba, image.meta.w, image.meta.h));
            }
        }
    }
    // All views/poses share a canvas and foot anchor, including tall static props.
    let left = anchors.iter().map(|a| a[0]).max().unwrap_or(0);
    let top = anchors.iter().map(|a| a[1]).max().unwrap_or(0);
    let right = generated
        .iter()
        .zip(&anchors)
        .map(|(t, a)| t.width as i32 - a[0])
        .max()
        .unwrap_or(0);
    let bottom = generated
        .iter()
        .zip(&anchors)
        .map(|(t, a)| t.height as i32 - a[1])
        .max()
        .unwrap_or(0);
    let (w, h) = (
        (left + right).max(1) as usize,
        (top + bottom).max(1) as usize,
    );
    for (i, (texture, a)) in generated.iter_mut().zip(anchors).enumerate() {
        if texture.width == w && texture.height == h && a == [left, top] {
            continue;
        }
        let mut pixels = vec![0; w * h * 4];
        for y in 0..texture.height {
            let at = ((top - a[1]) as usize + y) * w * 4 + (left - a[0]) as usize * 4;
            pixels[at..at + texture.width * 4]
                .copy_from_slice(&texture.data[y * texture.width * 4..(y + 1) * texture.width * 4]);
        }
        if let Some(surface) = surfaces.get_mut(i) {
            surface.pad(
                [texture.width, texture.height],
                [w, h],
                [(left - a[0]) as usize, (top - a[1]) as usize],
            );
        }
        *texture = Texture::new(pixels, w, h);
    }
    if placement_bounds.len() != settings.directions {
        placement_bounds.clear();
    }
    let anchor = [left, top];
    let mut tile = rusterix::Tile::from_textures(generated);
    let animated = settings.motion.is_some() && !settings.ground;
    tile.isometric = Some(rusterix::isometric::SpriteLayout {
        anchor,
        pixels_per_unit: HALF_W,
        ground: settings.ground,
        directions: if settings.ground {
            1
        } else {
            settings.directions
        },
        frames_per_direction: if animated { settings.frames + 1 } else { 1 },
        walk_frames: if animated { settings.frames } else { 0 },
        frames_per_second: settings.frames as f32 / duration.max(0.001),
        frames: Vec::new(),
        source_signature: 0,
        bitmap:false,prelit:false,
        surfaces,
        top_material,
        placement_bounds,
        placement_contours,
        placement_shapes,
        placement_connectors,
    });
    Ok(tile)
}

/// A complete horizontal unit cap, with the ordinary ground UV mapping. This
/// deliberately excludes partial tops, custom UVs, slopes and upright props.
fn unit_top_material(mesh: &Mesh) -> Option<Uuid> {
    let mut areas = BTreeMap::<Uuid, f32>::new();
    for part in &mesh.parts {
        let Some(id) = part.texture.as_ref().and_then(|t| {
            Uuid::parse_str(
                t.source
                    .strip_prefix("project:material/")
                    .unwrap_or(&t.source),
            )
            .ok()
        }) else {
            continue;
        };
        for tri in part.vertices.chunks_exact(3) {
            if tri.iter().any(|v| {
                v.position[1].abs() > 0.0001
                    || v.normal[1] < 0.99
                    || (v.uv[0] - v.position[0] - 0.5).abs() > 0.0001
                    || ((v.uv[1] - v.position[2] - 0.5).abs() > 0.0001
                        && (v.uv[1] + v.position[2] - 0.5).abs() > 0.0001)
                    || v.position[0].abs() > 0.5001
                    || v.position[2].abs() > 0.5001
            }) {
                continue;
            }
            let a = tri[0].position;
            let b = tri[1].position;
            let c = tri[2].position;
            *areas.entry(id).or_default() +=
                ((b[0] - a[0]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[0] - a[0])).abs() * 0.5;
        }
    }
    let mut full = areas
        .into_iter()
        .filter(|(_, area)| (*area - 1.).abs() < 0.0001);
    let first = full.next()?.0;
    full.next().is_none().then_some(first)
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Sprite {
    pub image: String,
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub anchor: [i32; 2],
}
pub struct Baked {
    pub meta: Sprite,
    pub rgba: Vec<u8>,
    pub surface: rusterix::isometric::surface::SurfaceFrame,
}
pub struct Baker<'a> {
    pub project: &'a Project,
    pub textures: BTreeMap<String, Texture>,
}

fn project(p: [f32; 3]) -> [f32; 3] {
    [
        (p[0] + p[2]) * HALF_W,
        (p[2] - p[0]) * HALF_H - p[1] * VERTICAL,
        (p[2] - p[0]) * 0.612372436 + p[1] * 0.5,
    ]
}
fn rotate(p: [f32; 3], yaw: f32) -> [f32; 3] {
    let (s, c) = yaw.to_radians().sin_cos();
    [c * p[0] + s * p[2], p[1], -s * p[0] + c * p[2]]
}
fn edge(a: [f32; 3], b: [f32; 3], x: f32, y: f32) -> f32 {
    (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
}

impl Baker<'_> {
    pub fn material(&mut self, key: &str, source: &str) -> Result<(), Box<dyn std::error::Error>> {
        let asset = EditableMaterialAsset::parse(key, source, "surface")?;
        self.textures.insert(
            key.into(),
            ruleset_materials::bake_asset(&asset, &self.project.palette, 64)?,
        );
        Ok(())
    }
    fn texture(&mut self, spec: &denrim_materials::geometry::Texture) -> Result<Texture, String> {
        let size = spec.size.clamp(16, 512);
        let cache_key = format!(
            "{}:{size}:{}",
            spec.source,
            rusterix::procedural_avatar::fingerprint(&spec.bundle)
        );
        if let Some(texture) = self.textures.get(&cache_key).or_else(|| {
            (size == 64)
                .then(|| self.textures.get(&spec.source))
                .flatten()
        }) {
            return Ok(texture.clone());
        }
        let texture = if let Ok(id) = Uuid::parse_str(
            spec.source
                .strip_prefix("project:material/")
                .unwrap_or(&spec.source),
        ) {
            let source = self
                .project
                .procedural_materials
                .get(&id)
                .ok_or("Project MZ material missing")?;
            let asset = EditableMaterialAsset::reopen(source)
                .or_else(|_| EditableMaterialAsset::parse(&spec.source, source, "surface"))
                .map_err(|e| e.to_string())?;
            ruleset_materials::bake_asset(&asset, &self.project.palette, size)?
        } else if let Some(bundle) = &spec.bundle {
            ruleset_materials::bake_asset(
                &EditableMaterialAsset::reopen(bundle).map_err(|e| e.to_string())?,
                &self.project.palette,
                size,
            )?
        } else if let Some(name) = spec.source.strip_prefix("ruleset:material/") {
            ruleset_materials::bake_asset(
                &ruleset_materials::editable(self.project, name)?,
                &self.project.palette,
                size,
            )?
        } else {
            return Err(format!("Unsupported experiment texture: {}", spec.source));
        };
        self.textures.insert(cache_key, texture.clone());
        Ok(texture)
    }
    fn shadow_grid(
        &mut self,
        mesh: &Mesh,
        yaw: f32,
    ) -> Result<rusterix::isometric::surface::ShadowGrid, String> {
        use rusterix::isometric::surface::ShadowGrid;
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for v in mesh.parts.iter().flat_map(|p| &p.vertices) {
            let p = rotate(v.position, yaw);
            for a in 0..3 {
                min[a] = min[a].min(p[a]);
                max[a] = max[a].max(p[a]);
            }
        }
        // A derived geometric acceleration structure, independent of every light.
        let width = ((max[0] - min[0]) * 24.).ceil().clamp(1., 128.) as usize;
        let height = ((max[2] - min[2]) * 24.).ceil().clamp(1., 128.) as usize;
        let span = [(max[0] - min[0]).max(0.001), (max[2] - min[2]).max(0.001)];
        let mut spans = vec![Vec::<[f32; 2]>::new(); width * height];
        for part in &mesh.parts {
            let mut intervals = vec![[f32::INFINITY, f32::NEG_INFINITY]; width * height];
            let texture = part
                .texture
                .as_ref()
                .map(|spec| self.texture(spec))
                .transpose()?;
            for tri in part.vertices.chunks_exact(3) {
                let p = std::array::from_fn::<_, 3, _>(|i| {
                    let v = rotate(tri[i].position, yaw);
                    [
                        (v[0] - min[0]) / span[0] * width as f32,
                        (v[2] - min[2]) / span[1] * height as f32,
                        v[1],
                    ]
                });
                let area = edge(p[0], p[1], p[2][0], p[2][1]);
                if area.abs() < 1e-6 {
                    continue;
                }
                let lo = [
                    p.iter()
                        .map(|v| v[0])
                        .fold(f32::INFINITY, f32::min)
                        .floor()
                        .max(0.) as usize,
                    p.iter()
                        .map(|v| v[1])
                        .fold(f32::INFINITY, f32::min)
                        .floor()
                        .max(0.) as usize,
                ];
                let hi = [
                    p.iter()
                        .map(|v| v[0])
                        .fold(f32::NEG_INFINITY, f32::max)
                        .ceil()
                        .min(width as f32) as usize,
                    p.iter()
                        .map(|v| v[1])
                        .fold(f32::NEG_INFINITY, f32::max)
                        .ceil()
                        .min(height as f32) as usize,
                ];
                for z in lo[1]..hi[1] {
                    for x in lo[0]..hi[0] {
                        let w0 = edge(p[1], p[2], x as f32 + 0.5, z as f32 + 0.5) / area;
                        let w1 = edge(p[2], p[0], x as f32 + 0.5, z as f32 + 0.5) / area;
                        let w = [w0, w1, 1. - w0 - w1];
                        if w.iter().any(|v| *v < -1e-5) {
                            continue;
                        }
                        if let Some(t) = &texture {
                            let uv = std::array::from_fn::<_, 2, _>(|k| {
                                (0..3).map(|i| tri[i].uv[k] * w[i]).sum::<f32>()
                            });
                            let tx =
                                ((uv[0].rem_euclid(1.) * t.width as f32) as usize).min(t.width - 1);
                            let ty = ((uv[1].rem_euclid(1.) * t.height as f32) as usize)
                                .min(t.height - 1);
                            if t.data[(ty * t.width + tx) * 4 + 3] < 128 {
                                continue;
                            }
                        }
                        let y = (0..3).map(|i| p[i][2] * w[i]).sum::<f32>();
                        let cell = &mut intervals[z * width + x];
                        cell[0] = cell[0].min(y);
                        cell[1] = cell[1].max(y);
                    }
                }
            }
            for (target, interval) in spans.iter_mut().zip(intervals) {
                if interval[0] <= interval[1] {
                    target.push(interval);
                }
            }
        }
        for column in &mut spans {
            column.sort_by(|a, b| a[0].total_cmp(&b[0]));
            let mut merged: Vec<[f32; 2]> = Vec::new();
            for span in column.drain(..) {
                if let Some(last) = merged.last_mut().filter(|last| span[0] <= last[1] + 0.005) {
                    last[1] = last[1].max(span[1]);
                } else {
                    merged.push(span);
                }
            }
            // Eight disjoint occupied spans preserve windows and roof cavities.
            // Extremely fragmented columns conservatively merge the tail.
            if merged.len() > 8 {
                let end = merged.last().unwrap()[1];
                merged.truncate(8);
                merged[7][1] = end;
            }
            *column = merged;
        }
        let layers = spans.iter().map(Vec::len).max().unwrap_or(1).max(1);
        let data = spans
            .into_iter()
            .flat_map(|mut column| {
                column.resize(layers, [f32::INFINITY, f32::NEG_INFINITY]);
                column
            })
            .flat_map(|v| v.into_iter().flat_map(f32::to_le_bytes))
            .collect();
        Ok(ShadowGrid {
            layers,
            emissive: false,
            min,
            max,
            width,
            height,
            data,
        })
    }
    pub fn bake(&mut self, key: &str, mesh: &Mesh, yaw: f32, fixed: bool) -> Result<Baked, String> {
        let positions = mesh
            .parts
            .iter()
            .flat_map(|p| p.vertices.iter().map(|v| project(rotate(v.position, yaw))))
            .collect::<Vec<_>>();
        if positions.is_empty() {
            return Err(format!("{key}: empty mesh"));
        }
        if !yaw.is_finite()
            || positions
                .iter()
                .flatten()
                .any(|p| !p.is_finite() || p.abs() > 1_000_000.)
        {
            return Err(format!("{key}: invalid projected bounds"));
        }
        let min_x = positions
            .iter()
            .map(|p| p[0])
            .fold(f32::INFINITY, f32::min)
            .floor() as i32
            - 2;
        let min_y = positions
            .iter()
            .map(|p| p[1])
            .fold(f32::INFINITY, f32::min)
            .floor() as i32
            - 2;
        let max_x = positions
            .iter()
            .map(|p| p[0])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil() as i32
            + 2;
        let max_y = positions
            .iter()
            .map(|p| p[1])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil() as i32
            + 2;
        let (width, height, anchor) = if fixed {
            (128, 128, [64, 100])
        } else {
            (
                (max_x - min_x + 1) as usize,
                (max_y - min_y + 1) as usize,
                [-min_x, -min_y],
            )
        };
        if width >= 1024 || height >= 1024 {
            return Err(format!("{key}: sprite exceeds 1024 pixels"));
        }
        if fixed
            && positions.iter().any(|p| {
                p[0] + 64. < 1. || p[0] + 64. > 126. || p[1] + 100. < 1. || p[1] + 100. > 126.
            })
        {
            return Err(format!("{key}: directional frame exceeds 128×128 canvas"));
        }
        let mut rgba = vec![0u8; width * height * 4];
        let mut depth = vec![f32::NEG_INFINITY; width * height];
        let mut surface = rusterix::isometric::surface::SurfaceFrame::new(width * height);
        for part in &mesh.parts {
            let texture = match &part.texture {
                Some(spec) => Some(self.texture(spec)?),
                None => None,
            };
            let color = part
                .surface
                .as_ref()
                .and_then(|s| self.project.palette.colors.get(s.palette as usize))
                .and_then(Option::as_ref)
                .map(|c| c.to_u8_array())
                .unwrap_or([150, 150, 150, 255]);
            for tri in part.vertices.chunks_exact(3) {
                let p = std::array::from_fn::<_, 3, _>(|i| {
                    let mut p = project(rotate(tri[i].position, yaw));
                    p[0] += anchor[0] as f32;
                    p[1] += anchor[1] as f32;
                    p
                });
                let area = edge(p[0], p[1], p[2][0], p[2][1]);
                if area.abs() < 1e-6 {
                    continue;
                }
                let lo_x = p
                    .iter()
                    .map(|p| p[0])
                    .fold(f32::INFINITY, f32::min)
                    .floor()
                    .max(0.) as usize;
                let hi_x = p
                    .iter()
                    .map(|p| p[0])
                    .fold(f32::NEG_INFINITY, f32::max)
                    .ceil()
                    .min(width as f32) as usize;
                let lo_y = p
                    .iter()
                    .map(|p| p[1])
                    .fold(f32::INFINITY, f32::min)
                    .floor()
                    .max(0.) as usize;
                let hi_y = p
                    .iter()
                    .map(|p| p[1])
                    .fold(f32::NEG_INFINITY, f32::max)
                    .ceil()
                    .min(height as f32) as usize;
                for y in lo_y..hi_y {
                    for x in lo_x..hi_x {
                        let w0 = edge(p[1], p[2], x as f32 + 0.5, y as f32 + 0.5) / area;
                        let w1 = edge(p[2], p[0], x as f32 + 0.5, y as f32 + 0.5) / area;
                        let w = [w0, w1, 1. - w0 - w1];
                        if w.iter().any(|v| *v < -1e-5) {
                            continue;
                        }
                        let z = (0..3).map(|i| p[i][2] * w[i]).sum::<f32>();
                        let at = y * width + x;
                        if z <= depth[at] {
                            continue;
                        }
                        let mut materials = part
                            .surface
                            .as_ref()
                            .map(|s| (s.roughness, s.metallic, 1., 0.))
                            .unwrap_or((0.9, 0., 1., 0.));
                        let c = if let Some(texture) = &texture {
                            let uv = std::array::from_fn::<_, 2, _>(|k| {
                                (0..3).map(|i| tri[i].uv[k] * w[i]).sum::<f32>()
                            });
                            let tx = ((uv[0].rem_euclid(1.) * texture.width as f32) as usize)
                                .min(texture.width - 1);
                            let ty = ((uv[1].rem_euclid(1.) * texture.height as f32) as usize)
                                .min(texture.height - 1);
                            materials = texture.get_materials(tx as u32, ty as u32);
                            texture.data
                                [(ty * texture.width + tx) * 4..(ty * texture.width + tx) * 4 + 4]
                                .try_into()
                                .unwrap()
                        } else {
                            color
                        };
                        if c[3] < 128 {
                            continue;
                        }
                        let n = rotate(
                            std::array::from_fn(|k| {
                                (0..3).map(|i| tri[i].normal[k] * w[i]).sum::<f32>()
                            }),
                            yaw,
                        );
                        let height = (0..3).map(|i| tri[i].position[1] * w[i]).sum::<f32>();
                        surface.set(
                            at,
                            Vec3::from(n),
                            height,
                            materials.0,
                            materials.1,
                            materials.3,
                        );
                        rgba[at * 4..at * 4 + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
                        depth[at] = z;
                    }
                }
            }
        }
        assert!(
            rgba.chunks_exact(4).any(|p| p[3] > 0),
            "{key}: blank sprite"
        );
        if fixed {
            assert!(
                (0..width)
                    .all(|x| rgba[x * 4 + 3] == 0 && rgba[((height - 1) * width + x) * 4 + 3] == 0),
                "Actor clipped vertically"
            );
        }
        let mut shadow_mesh = scenevm::isometric::ShadowGeometry::default();
        for part in &mesh.parts {
            let texture = part.texture.as_ref().map(|spec| self.texture(spec)).transpose()?;
            let offset = shadow_mesh.alpha.len() as u32;
            let mut mask = 0;
            if let Some(t) = &texture {
                // Fully opaque surfaces need no mask. Preserve the same alpha
                // threshold and repeating UVs as the visible MZ rasterization.
                if t.data.chunks_exact(4).any(|p| p[3] < 128) {
                    mask = t.width as u32 | ((t.height as u32) << 16);
                    let mut words = vec![0u32; (t.width * t.height).div_ceil(32)];
                    for (i, pixel) in t.data.chunks_exact(4).enumerate() {
                        if pixel[3] >= 128 { words[i / 32] |= 1 << (i % 32); }
                    }
                    shadow_mesh.alpha.extend(words);
                }
            }
            let emissive = texture.as_ref().is_some_and(|t| {
                (0..t.height).any(|y| (0..t.width).any(|x| t.get_materials(x as u32, y as u32).3 > 0.))
            });
            shadow_mesh.vertices.extend(part.vertices.iter().map(|v| {
                let p = rotate(v.position, yaw);
                scenevm::isometric::ShadowVertex {
                    position: [p[0], p[1], p[2], if emissive { 1. } else { 0. }],
                    uv_alpha: [v.uv[0], v.uv[1], f32::from_bits(offset), f32::from_bits(mask)],
                }
            }));
        }
        surface.shadow_mesh = Some(shadow_mesh);
        let mut shadow = self.shadow_grid(mesh, yaw)?;
        shadow.emissive = surface.data.chunks_exact(8).any(|p| p[7] > 0);
        surface.shadow = Some(shadow);
        surface.effects = mesh
            .effects
            .iter()
            .map(|e| {
                let mut e = e.definition.clone();
                e.position = rotate(e.position, yaw);
                e
            })
            .collect();
        Ok(Baked {
            meta: Sprite {
                image: key.into(),
                x: 0,
                y: 0,
                w: width,
                h: height,
                anchor,
            },
            rgba,
            surface,
        })
    }
}

pub fn plane(texture: &str) -> Mesh {
    let points = [
        [-0.5, 0., -0.5],
        [0.5, 0., -0.5],
        [0.5, 0., 0.5],
        [-0.5, 0., 0.5],
    ];
    let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    Mesh {
        parts: vec![Part {
            id: "ground".into(),
            label: "Ground".into(),
            vertices: [0, 1, 2, 0, 2, 3]
                .into_iter()
                .map(|i| Vertex {
                    position: points[i],
                    normal: [0., 1., 0.],
                    uv: uv[i],
                    skin: None,
                })
                .collect(),
            surface: None,
            texture: Some(denrim_materials::geometry::Texture {
                source: texture.into(),
                bundle: None,
                size: 64,
            }),
        }],
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shadow_columns_keep_the_window_open_between_separate_stone_parts() {
        let doc=Document::load(r#"{"version":1,"id":"window","label":"Window","rows":[{"id":"base","label":"Base","tiles":[{"id":"base-0","op":{"type":"box","size":[1,0.5,0.25]}},{"id":"base-1","op":{"type":"transform","translation":[0,0.25,0],"rotation":[0,0,0],"scale":[1,1,1]}}]},{"id":"lintel","label":"Lintel","tiles":[{"id":"lintel-0","op":{"type":"box","size":[1,0.5,0.25]}},{"id":"lintel-1","op":{"type":"transform","translation":[0,1.75,0],"rotation":[0,0,0],"scale":[1,1,1]}}]},{"id":"out","label":"Window","tiles":[{"id":"out-0","op":{"type":"assemble","rows":["base","lintel"]}}]}],"output":"out"}"#).unwrap();
        let mesh = doc.evaluate().unwrap().mesh;
        let project = Project::default();
        let mut baker = Baker {
            project: &project,
            textures: BTreeMap::new(),
        };
        let baked=baker.bake("window-triangles",&mesh,90.,false).unwrap();
        let exact=baked.surface.shadow_mesh.as_ref().unwrap();
        let original=mesh.parts.iter().flat_map(|p|p.vertices.iter()).collect::<Vec<_>>();
        assert_eq!(exact.vertices.len(),original.len());
        for (v,source) in exact.vertices.iter().zip(original) {
            assert_eq!(&v.position[..3],&rotate(source.position,90.));
            assert_eq!(&v.uv_alpha[..2],&source.uv);
        }
        let restored:rusterix::isometric::surface::SurfaceFrame=serde_json::from_str(&serde_json::to_string(&baked.surface).unwrap()).unwrap();
        assert_eq!(restored,baked.surface,"exact geometry persists with the unlit sprite cache");
        let grid = baker.shadow_grid(&mesh, 0.).unwrap();
        assert_eq!(grid.layers, 2);
        let x = grid.width / 2;
        let z = grid.height / 2;
        assert_eq!(grid.interval_layer(x, z, 0).unwrap(), [0., 0.5]);
        let upper = grid.interval_layer(x, z, 1).unwrap();
        assert!((upper[0] - 1.5).abs() < 1e-5 && (upper[1] - 2.).abs() < 1e-5);
        let encoded = serde_json::to_string(&grid).unwrap();
        let restored: rusterix::isometric::surface::ShadowGrid =
            serde_json::from_str(&encoded).unwrap();
        assert_eq!(grid, restored);
    }
    #[test]
    fn mesh_texture_resolution_and_source_snapshots_have_separate_cache_entries() {
        let mut project = Project::default();
        let source = r#"project 1
result from material "surface"
material "surface" called "Texture resolution study"
  row "shape" called "Cutout"
    use "Circle"
      set "radius" to 0.3
    use "Color"
      set "color" to 0.2 0.3 0.1 1
  output base color from row "shape" xyz
  output opacity from row "shape" w
"#;
        let id = Uuid::new_v4();
        let bundle = EditableMaterialAsset::parse(&id.to_string(), source, "surface")
            .unwrap()
            .save()
            .unwrap();
        project.procedural_materials.insert(id, bundle.clone());
        let mut baker = Baker {
            project: &project,
            textures: BTreeMap::new(),
        };
        let mut spec = denrim_materials::geometry::Texture {
            source: id.to_string(),
            bundle: None,
            size: 16,
        };
        let coarse = baker.texture(&spec).unwrap();
        spec.size = 128;
        let fine = baker.texture(&spec).unwrap();
        assert_eq!((coarse.width, coarse.height), (16, 16));
        assert_eq!((fine.width, fine.height), (128, 128));
        assert!(coarse.data.chunks_exact(4).any(|p| p[3] == 0));
        assert!(fine.data.chunks_exact(4).any(|p| p[3] > 127));
        spec.size = 16;
        assert_eq!(baker.texture(&spec).unwrap(), coarse);
        spec.source = "embedded:study".into();
        spec.bundle = Some(bundle);
        let first = baker.texture(&spec).unwrap();
        let changed = source.replace("0.2 0.3 0.1 1", "0.8 0.1 0.1 1");
        spec.bundle = Some(
            EditableMaterialAsset::parse("other", &changed, "surface")
                .unwrap()
                .save()
                .unwrap(),
        );
        assert_ne!(baker.texture(&spec).unwrap().data, first.data);
    }

    #[test]
    fn mz_unit_caps_expose_their_continuous_material_without_matching_custom_uvs() {
        let id = Uuid::new_v4();
        let recipe = include_str!("../examples/iso_2d/grass-step.mzgeometry.json")
            .replace("experiment:grass", &id.to_string());
        let mesh = Document::load(&recipe).unwrap().evaluate().unwrap().mesh;
        assert_eq!(unit_top_material(&mesh), Some(id));
        let mut custom = mesh.clone();
        for vertex in custom.parts.iter_mut().flat_map(|p| &mut p.vertices) {
            vertex.uv[0] += 0.2;
        }
        assert_eq!(
            unit_top_material(&custom),
            None,
            "Custom UV layouts must keep their authored mapping"
        );
    }
    #[test]
    fn continuous_iso_ground_keeps_world_pages_and_follows_cache_and_coverage_changes() {
        let mut p = Project::default();
        let recipe = Uuid::new_v4();
        let output = Uuid::new_v4();
        let asset=EditableMaterialAsset::parse(&recipe.to_string(),"project 1\nmaterial \"surface\" called \"Ground\"\n  mode pbr\n  tiling raw\n  default base color to 0.1 0.2 0.1\n","surface").unwrap();
        p.procedural_materials.insert(recipe, asset.save().unwrap());
        let mut source = rusterix::Tile::empty();
        source.id = recipe;
        source.textures = vec![
            Texture::from_color([40, 70, 30, 255]),
            Texture::from_color([70, 90, 40, 255]),
        ];
        source.uv_pages = Some(rusterix::TileUvPages {
            origin: [-1, 0],
            size: [2, 1],
            animation_frames: 1,
            repeat_period: [0.; 2],
            axes: [[0.; 3]; 2],
            units_per_page: 1,
        });
        p.tiles.insert(recipe, source);
        p.procedural_material_cache_keys.insert(recipe, 1);
        let mut tile = rusterix::Tile::empty();
        tile.id = output;
        tile.alias = "Continuous iso grass".into();
        p.tiles.insert(output, tile);
        p.isometric_bakes.insert(
            output,
            Bake {
                recipe,
                ground: true,
                yaw: 0.,
                directions: 1,
                motion: None,
                frames: 1,
                signature: 0,
            },
        );
        assert!(synchronize(&mut p).is_empty());
        assert_eq!(p.tiles[&output].uv_pages, p.tiles[&recipe].uv_pages);
        assert_eq!(p.tiles[&output].textures.len(), 2);
        assert_eq!(p.tiles[&output].alias, "Continuous iso grass");
        let before = p.isometric_bakes[&output].signature;
        p.tiles.get_mut(&recipe).unwrap().textures[1] = Texture::from_color([80, 95, 45, 255]);
        p.procedural_material_cache_keys.insert(recipe, 2);
        assert!(synchronize(&mut p).is_empty());
        assert_ne!(before, p.isometric_bakes[&output].signature);
        assert_eq!(p.tiles[&output].textures[1].data, vec![80, 95, 45, 255]);
        let source = p.tiles.get_mut(&recipe).unwrap();
        source.uv_pages.as_mut().unwrap().size = [3, 1];
        source.textures.push(Texture::from_color([30, 60, 30, 255]));
        p.procedural_material_cache_keys.insert(recipe, 3);
        assert!(synchronize(&mut p).is_empty());
        assert_eq!(p.tiles[&output].uv_pages.as_ref().unwrap().size, [3, 1]);
        assert_eq!(p.tiles[&output].textures.len(), 3);
    }
    #[test]
    fn isometric_tile_recipes_resolve_shared_sources_after_reopening() {
        let mut project = Project::default();
        let material = Uuid::new_v4();
        let geometry = Uuid::new_v4();
        project
            .procedural_materials
            .insert(material, "source".into());
        project
            .procedural_appearances
            .insert(geometry, "source".into());
        for (recipe, ground) in [(material, true), (geometry, false)] {
            let output = Uuid::new_v4();
            project.isometric_bakes.insert(
                output,
                Bake {
                    recipe,
                    ground,
                    yaw: 90.,
                    directions: 1,
                    motion: None,
                    frames: 1,
                    signature: 0,
                },
            );
            let reopened: Project =
                serde_json::from_str(&serde_json::to_string(&project).unwrap()).unwrap();
            assert_eq!(
                tile_recipe(&reopened, output),
                Some(if ground {
                    TileRecipe::Material(recipe)
                } else {
                    TileRecipe::Geometry(recipe)
                })
            );
            assert!(!reopened.tiles.contains_key(&recipe));
        }
        assert_eq!(
            tile_recipe(&project, material),
            Some(TileRecipe::Material(material))
        );
        assert_eq!(tile_recipe(&project, Uuid::new_v4()), None);
    }
    #[test]
    fn isometric_mode_automatically_bakes_models_and_preserves_flat_materials() {
        let mut project = Project::default();
        project.regions[0].map.camera = rusterix::MapCamera::TwoDIso;
        let material = Uuid::new_v4();
        let mut flat =
            rusterix::Tile::from_texture(Texture::new([80, 120, 30, 255].repeat(16), 4, 4));
        flat.id = material;
        let original = flat.textures.clone();
        project.tiles.insert(material, flat);
        project
            .procedural_materials
            .insert(material, "cached material".into());
        let model = Uuid::new_v4();
        project.procedural_appearances.insert(model,r#"{"version":1,"id":"wall","label":"MZ wall","rows":[{"id":"wall","label":"Wall","tiles":[{"id":"box","op":{"type":"box","size":[1,2.4,0.22]}}]}],"output":"wall"}"#.into());
        assert!(synchronize(&mut project).is_empty());
        let tile = &project.tiles[&material];
        assert_eq!(tile.textures, original);
        let sprite = tile.isometric.as_ref().unwrap();
        assert_eq!((sprite.frames[0].width, sprite.frames[0].height), (64, 32));
        let model_tile = &project.tiles[&model];
        let sprite = model_tile.isometric.as_ref().unwrap();
        assert_eq!(sprite.directions, 4);
        assert_eq!(model_tile.textures.len(), 4);
        let bounds = (model_tile.textures[0].width, model_tile.textures[0].height);
        assert!(
            model_tile
                .textures
                .iter()
                .all(|t| (t.width, t.height) == bounds)
        );
        assert!(
            model_tile
                .textures
                .iter()
                .all(|t| t.data.chunks_exact(4).all(|c| c[3] == 0 || c[3] == 255))
        );
        assert_ne!(model_tile.textures[0].data, model_tile.textures[1].data);
    }
    #[test]
    fn source_edits_rebuild_tiles_and_invalid_edits_keep_last_good_bake() {
        let mut project = Project::default();
        let recipe = Uuid::new_v4();
        let tile = Uuid::new_v4();
        let source = r#"{"version":1,"id":"test","label":"Test","rows":[
          {"id":"paint","label":"Paint","tiles":[{"id":"p","op":{"type":"material","palette":7,"roughness":0.9,"metallic":0}}]},
          {"id":"box","label":"Box","tiles":[{"id":"b","op":{"type":"box","size":[1,1,0.2]}},{"id":"m","op":{"type":"assign_material","row":"paint"}}]}],"output":"box"}"#;
        project.procedural_appearances.insert(recipe, source.into());
        project.isometric_bakes.insert(
            tile,
            Bake {
                recipe,
                ground: false,
                yaw: 0.,
                directions: 1,
                motion: None,
                frames: 1,

                signature: 0,
            },
        );
        assert!(synchronize(&mut project).is_empty());
        let first = project.tiles[&tile].textures[0].data.clone();
        let signature = project.isometric_bakes[&tile].signature;
        assert!(synchronize(&mut project).is_empty());
        assert_eq!(signature, project.isometric_bakes[&tile].signature);
        // UI palette selection and unrelated library installation cannot rebake art.
        project.palette.current_index = 12;
        project
            .procedural_materials
            .insert(Uuid::new_v4(), "unrelated material".into());
        assert!(synchronize(&mut project).is_empty());
        assert_eq!(signature, project.isometric_bakes[&tile].signature);
        project
            .procedural_appearances
            .insert(recipe, source.replace("\"palette\":7", "\"palette\":36"));
        assert!(synchronize(&mut project).is_empty());
        assert_ne!(signature, project.isometric_bakes[&tile].signature);
        assert_ne!(first, project.tiles[&tile].textures[0].data);
        let good = project.tiles[&tile].clone();
        project
            .procedural_appearances
            .insert(recipe, "invalid".into());
        assert_eq!(synchronize(&mut project).len(), 1);
        assert_eq!(project.tiles[&tile], good);
    }
}
