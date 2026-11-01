//! Tile composition authoring and lowering to ordinary whole-material mixers.
use super::*;
mod raster_source;
mod shape_picker;
pub(super) mod surface_pages;
use denrim_materials::integration::InstanceRecord;
use shape_picker::{PRESETS, ShapePicker};
use shared::material_composition::{
    CompositionLayer, CompositionMask, CompositionShape, MaterialComposition,
};
use std::collections::{HashMap, HashSet};

pub(super) const STACK: &str = "Procedural Editor Kind";
pub(super) const INSPECTOR: &str = "Composition Inspector";
const SAVE_COPY: &str = "Composition Save Copy";
const BASE: &str = "Composition Base";
const LAYER: &str = "Composition Layer";
const MATERIAL: &str = "Composition Material";
const SHAPE: &str = "Composition Shape";
const ADD: &str = "Composition Add";
const REMOVE: &str = "Composition Remove";
const UP: &str = "Composition Up";
const DOWN: &str = "Composition Down";
const POSITION: &str = "Composition Position";
const ANGLE: &str = "Composition Angle";
const FEATHER: &str = "Composition Feather";
const WOBBLE: &str = "Composition Wobble";
const WOBBLE_SCALE: &str = "Composition Wobble Scale";
const AMOUNT: &str = "Composition Amount";
const MASK_MATERIAL: &str = "Composition Mask Material";
const MASK_ROW: &str = "Composition Mask Row";
const MASK_CHANNEL: &str = "Composition Mask Channel";
const MASK_REMAP: &str = "Composition Mask Remap";
const MASK_MIN: &str = "Composition Mask Minimum";
const MASK_MAX: &str = "Composition Mask Maximum";
const BLEND: &str = "Composition Blend";
const OVERLAY: &str = "Composition Overlay Level";
const DIRECTION: &str = "Composition UV Direction";
const LEVELS: [&str; 3] = [
    "Composition UV Level 1",
    "Composition UV Level 2",
    "Composition UV Level 3",
];
const EXTRAS: [&str; 2] = ["Composition Material 3", "Composition Material 4"];
const SHAPES: [(CompositionShape, &str); 10] = [
    (CompositionShape::Split, "Split"),
    (CompositionShape::Blend, "Blend"),
    (CompositionShape::Mask, "Mask"),
    (CompositionShape::Height, "Height"),
    (CompositionShape::Overlay, "Overlay"),
    (CompositionShape::UvPosition, "UV Position"),
    (CompositionShape::Corner, "Curved corner"),
    (CompositionShape::Strips, "Edge strips"),
    (CompositionShape::Road, "Road"),
    (CompositionShape::Crossing, "Crossing"),
];
const CHANNELS: [&str; 6] = ["xxx", "yyy", "zzz", "www", "xyz", "distance"];

/// Source UUIDs are resolved once, including mask-only and UV-only dependencies.
fn sources(composition: &MaterialComposition) -> Vec<Uuid> {
    let mut sources = vec![composition.base];
    for layer in &composition.layers {
        sources.push(layer.material);
        if layer.shape == CompositionShape::UvPosition {
            sources.extend(layer.extra_materials.iter().flatten().copied());
        }
        if layer.shape == CompositionShape::Mask {
            sources.extend(layer.mask.iter().map(|mask| mask.material));
        }
    }
    sources
}

fn append_mask_row(
    recipe: &mut String,
    row: &str,
    tile: &str,
    expression: &str,
) -> Result<(), String> {
    let source = denrim_materials::tilelang::source(denrim_materials::metadata::Kernel::Color);
    let header = source
        .split_once("\nimage =")
        .ok_or("Color source has no image output")?
        .0;
    recipe.push_str(&format!("  mixing mask from row \"{row}\" xyz\n  row \"{row}\"\n    use \"Color\" id \"{tile}\"\n      source\n"));
    for line in format!("{header}\n{expression}").lines() {
        recipe.push_str(&format!("        {line}\n"));
    }
    recipe.push_str("      end source\n");
    Ok(())
}

fn shape_code(layer: &CompositionLayer) -> String {
    let a = layer.angle.to_radians();
    let d = match layer.shape {
        CompositionShape::Split => format!(
            "p.x * {} + p.y * {} + {}",
            a.cos(),
            a.sin(),
            0.5 - layer.position
        ),
        CompositionShape::Corner => {
            let c = shape_picker::corner(layer.angle);
            format!(
                "{} - length(fract(uv) - vec2({}, {}))",
                layer.position, c[0], c[1]
            )
        }
        CompositionShape::Strips => {
            let axis = if layer.angle.abs() < 45. { "y" } else { "x" };
            format!(
                "max({0} - fract(uv).{axis}, fract(uv).{axis} - (1 - {0}))",
                layer.position
            )
        }
        CompositionShape::Road => format!(
            "{} - abs(-p.x * {} + p.y * {})",
            layer.position / 2.,
            a.sin(),
            a.cos()
        ),
        CompositionShape::Crossing => format!(
            "{} - min(abs(p.x * {} + p.y * {}), abs(-p.x * {} + p.y * {}))",
            layer.position / 2.,
            a.cos(),
            a.sin(),
            a.sin(),
            a.cos()
        ),
        _ => unreachable!(),
    };
    // Four hashed lattice values with smooth interpolation: stable in global
    // surface UV, including negative coordinates and shared tile boundaries.
    let noise = if layer.wobble > 0. {
        format!(
            "let q = uv * {}\nlet c = floor(q)\nlet f = fract(q)\nlet t = f * f * (3 - 2 * f)\nlet a = hash33(vec3(c, 0)).x\nlet b = hash33(vec3(c + vec2(1, 0), 0)).x\nlet c1 = hash33(vec3(c + vec2(0, 1), 0)).x\nlet e = hash33(vec3(c + vec2(1, 1), 0)).x\nlet wobble = (mix(mix(a, b, t.x), mix(c1, e, t.x), t.y) * 2 - 1) * {}\n",
            layer.wobble_scale, layer.wobble
        )
    } else {
        "let wobble = 0\n".into()
    };
    let mask = if layer.feather == 0. {
        "step(0, d)".into()
    } else {
        format!(
            "smoothstep({}, {}, d)",
            -layer.feather / 2.,
            layer.feather / 2.
        )
    };
    format!(
        "let p = fract(uv) - vec2(0.5)\n{noise}let d = {d} + wobble\nlet m = {mask}\nimage = vec4(vec3(m), 1)"
    )
}

pub(super) fn canvas() -> TheCanvas {
    let mut canvas = TheCanvas::default();
    let mut inspector = TheTextLayout::new(TheId::named(INSPECTOR));
    inspector.set_margin(Vec4::new(10, 8, 10, 8));
    inspector.set_padding(7);
    inspector.set_fixed_text_width(80);
    canvas.set_layout(inspector);
    canvas
}

fn material_label(project: &Project, id: Uuid) -> String {
    let Some(tile) = project.tiles.get(&id) else {
        return "Missing material".into();
    };
    if tile.alias.is_empty() {
        return id.to_string();
    }
    let duplicate_name = project
        .procedural_materials
        .keys()
        .filter_map(|key| project.tiles.get(key))
        .filter(|other| other.alias == tile.alias)
        .count()
        > 1;
    if duplicate_name {
        if let Some(cell) = project.tile_board_tiles.get(&id) {
            return format!("{} · ({}, {})", tile.alias, cell.x, cell.y);
        }
        return format!("{} · {}", tile.alias, &id.to_string()[..8]);
    }
    tile.alias.clone()
}

/// Lower references afresh. Shared materials never become private editable copies.
/// The resulting bundle is self contained, so the existing worker and runtime bake
/// paths need no composition-specific rendering or blend math.
pub(super) fn bundle(
    id: Uuid,
    project: &Project,
    candidate: Option<&MaterialComposition>,
) -> Result<String, String> {
    let mut builder = Builder {
        project,
        candidate: candidate.map(|c| (id, c)),
        visiting: HashSet::new(),
        assets: HashMap::new(),
    };
    let mut asset = builder.build(id)?;
    if let Some(binding) = project.map_material_bindings.get(&id) {
        set_surface_domain(&mut asset, project, binding);
    }
    let pages = project
        .map_material_bindings
        .get(&id)
        .and_then(|binding| surface_pages::for_surface(project, binding))
        .or_else(|| {
            if project.map_material_bindings.contains_key(&id) {
                None
            } else {
                surface_pages::for_template(project, id)
            }
        });
    surface_pages::apply(&mut asset, pages.as_ref());
    asset.save().map_err(|e| e.to_string())
}
pub(super) const SURFACE_DOMAIN: &str = "eldiron-surface-domain";
fn surface_origin(
    project: &Project,
    binding: &shared::material_composition::MapMaterialBinding,
) -> [f32; 2] {
    use shared::material_composition::CompositionSurface;
    match binding.surface {
        CompositionSurface::SectorTile { cell, .. }
        | CompositionSurface::GeometryTile { cell, .. }
        | CompositionSurface::TerrainTile { cell } => [cell[0] as f32, cell[1] as f32],
        CompositionSurface::RectTile { sector } => project
            .regions
            .iter()
            .find(|r| r.map.id == binding.map)
            .and_then(|r| {
                r.map.find_sector(sector).map(|s| {
                    let b = s.bounding_box(&r.map);
                    if b.min.x.is_finite() && b.min.y.is_finite() {
                        [b.min.x, b.min.y]
                    } else {
                        [0.; 2]
                    }
                })
            })
            .unwrap_or([0.; 2]),
        _ => [0.; 2],
    }
}
fn set_surface_domain(
    asset: &mut EditableMaterialAsset,
    project: &Project,
    binding: &shared::material_composition::MapMaterialBinding,
) {
    let origin = surface_origin(project, binding);
    let mut record = InstanceRecord::new(SURFACE_DOMAIN);
    record.uv_transform[4] = origin[0];
    record.uv_transform[5] = origin[1];
    asset.instances.retain(|r| r.id != SURFACE_DOMAIN);
    asset.instances.push(record);
}
pub(super) fn bundle_for_surface(
    id: Uuid,
    project: &Project,
    composition: &MaterialComposition,
    binding: &shared::material_composition::MapMaterialBinding,
) -> Result<String, String> {
    let mut asset = EditableMaterialAsset::reopen(&bundle(id, project, Some(composition))?)
        .map_err(|e| e.to_string())?;
    set_surface_domain(&mut asset, project, binding);
    surface_pages::apply(
        &mut asset,
        surface_pages::for_surface(project, binding).as_ref(),
    );
    asset.save().map_err(|e| e.to_string())
}

struct Builder<'a> {
    project: &'a Project,
    candidate: Option<(Uuid, &'a MaterialComposition)>,
    visiting: HashSet<Uuid>,
    assets: HashMap<Uuid, EditableMaterialAsset>,
}
impl Builder<'_> {
    fn build(&mut self, id: Uuid) -> Result<EditableMaterialAsset, String> {
        if let Some(asset) = self.assets.get(&id) {
            return Ok(asset.clone());
        }
        if !self.visiting.insert(id) {
            return Err("Composition materials contain a cycle".into());
        }
        let composition = self
            .candidate
            .filter(|(tile, _)| *tile == id)
            .map(|(_, c)| c)
            .or_else(|| self.project.material_compositions.get(&id))
            .cloned();
        let asset = if let Some(composition) = composition {
            let mut documents = Vec::new();
            let mut selections = HashMap::new();
            let mut source_names = HashMap::new();
            for source in sources(&composition) {
                if selections.contains_key(&source) {
                    continue;
                }
                if !self.project.tiles.contains_key(&source) {
                    return Err(format!("Composition source {source} is missing"));
                }
                let asset = self.build(source)?;
                let mut document = asset
                    .resolved(&InstanceRecord::new("composition-source"))
                    .map_err(|e| e.to_string())?;
                // Standalone picker materials commonly all have the local ID "surface".
                // Namespace the entire DAG, including all mixer references.
                let names: HashMap<_, _> = document
                    .materials
                    .iter()
                    .map(|m| (m.id.clone(), format!("{source}:{}", m.id)))
                    .collect();
                let selected = names
                    .get(&asset.selected_material)
                    .ok_or("Composition source has no output")?
                    .clone();
                for material in &mut document.materials {
                    material.id = names[&material.id].clone();
                    // Editor stack slots are local presentation state. The input
                    // DAG is already explicit, so slots must not collide when
                    // independent source documents are combined.
                    material.stack_slot = None;
                    for tile in material.rows.iter_mut().flat_map(|r| &mut r.tiles) {
                        tile.id = format!("{source}:{}", tile.id);
                    }
                    if let Some(mix) = &mut material.mixing {
                        for input in mix.inputs.iter_mut().flatten() {
                            *input = names
                                .get(input)
                                .ok_or("Unresolved composition material")?
                                .clone();
                        }
                        if let Some(owner) = &mut mix.mask_material {
                            *owner = names.get(owner).ok_or("Unresolved mask material")?.clone();
                        }
                    }
                }
                // The same graph may be reached through several composed sources.
                // Its newly namespaced identities are distinct in each source DAG.
                documents.extend(document.materials);
                selections.insert(source, selected);
                source_names.insert(source, names);
            }
            let root = format!("composition-{id}");
            let mut previous = selections[&composition.base].clone();
            let mut recipe = String::from("project 1\n");
            // A pass-through root also supports removing the last layer.
            if composition.layers.is_empty() {
                recipe.push_str(&format!("material \"{root}\" called \"Composition\"\n  mode pbr\n  mix \"Mask\"\n  mixing input 1 from material \"{previous}\"\n  mixing mask to 0 0 0\n"));
            }
            let mut layer_ids = HashSet::new();
            for (index, layer) in composition.layers.iter().enumerate() {
                if !layer_ids.insert(layer.id) {
                    return Err("Duplicate composition layer identity".into());
                }
                validate_layer(layer)?;
                let output = if index + 1 == composition.layers.len() {
                    root.clone()
                } else {
                    format!("layer-{}", layer.id)
                };
                if layer.shape == CompositionShape::UvPosition {
                    // Native UV Position uses centred coordinates (uv + .5) and
                    // restricts thresholds to 0..1. Eldiron paints 0..1 tiles:
                    // lower its four-way selector into native complete-material
                    // masks, keeping all source sampling in the original domain.
                    let inputs = [
                        Some(layer.material),
                        layer.extra_materials[0],
                        layer.extra_materials[1],
                    ];
                    for (slot, input) in inputs.iter().enumerate() {
                        let stage = if slot == 2 {
                            output.clone()
                        } else {
                            format!("uv-{}-{slot}", layer.id)
                        };
                        recipe.push_str(&format!("material \"{stage}\" called \"UV Position\"\n  mode pbr\n  tiling raw\n  mix \"Mask\"\n  mixing input 1 from material \"{previous}\"\n  mixing blend true\n"));
                        if let Some(input) = input {
                            recipe.push_str(&format!(
                                "  mixing input 2 from material \"{}\"\n",
                                selections[input]
                            ));
                        }
                        let axis = if layer.uv_direction == 0 { "x" } else { "y" };
                        append_mask_row(
                            &mut recipe,
                            "uv-position",
                            &format!("uv-{}-{slot}", layer.id),
                            &format!(
                                "let m = select(greater(fract(uv).{axis}, {}), 1, 0)\nimage = vec4(vec3(m), 1)",
                                layer.uv_levels[slot]
                            ),
                        )?;
                        previous = stage;
                    }
                    continue;
                }
                let kind = match layer.shape {
                    CompositionShape::Split
                    | CompositionShape::Blend
                    | CompositionShape::Mask
                    | CompositionShape::Corner
                    | CompositionShape::Strips
                    | CompositionShape::Road
                    | CompositionShape::Crossing => "Mask",
                    CompositionShape::Height => "Bump Height",
                    CompositionShape::Overlay => "Overlay",
                    CompositionShape::UvPosition => unreachable!(),
                };
                recipe.push_str(&format!("material \"{output}\" called \"Composition\"\n  mode pbr\n  tiling raw\n  mix \"{kind}\"\n  mixing input 1 from material \"{previous}\"\n  mixing input 2 from material \"{}\"\n  mixing blend {}\n", selections[&layer.material], layer.blend));
                match layer.shape {
                    CompositionShape::Blend | CompositionShape::Mask => {
                        recipe.push_str(&format!("  mixing mask to {0} {0} {0}\n", layer.amount))
                    }
                    CompositionShape::Height => {}
                    CompositionShape::Overlay => recipe.push_str(&format!(
                        "  mixing levels {} 0.5 0.75\n",
                        layer.overlay_level
                    )),
                    CompositionShape::UvPosition => unreachable!(),
                    CompositionShape::Split
                    | CompositionShape::Corner
                    | CompositionShape::Strips
                    | CompositionShape::Road
                    | CompositionShape::Crossing => {
                        append_mask_row(
                            &mut recipe,
                            "shape",
                            &format!("shape-{}", layer.id),
                            &shape_code(layer),
                        )?;
                    }
                }
                previous = output;
            }
            let mut asset = EditableMaterialAsset::parse(&id.to_string(), &recipe, &root)
                .map_err(|e| e.to_string())?;
            // Assign row masks through the AST, preserving arbitrary quoted IDs.
            for (index, layer) in composition.layers.iter().enumerate() {
                if layer.shape != CompositionShape::Mask {
                    continue;
                }
                let Some(mask) = &layer.mask else {
                    continue;
                };
                let owner = source_names
                    .get(&mask.material)
                    .and_then(|names| names.get(&mask.owner))
                    .ok_or("Composition mask material is missing")?;
                if !documents
                    .iter()
                    .any(|m| &m.id == owner && m.rows.iter().any(|r| r.id == mask.row))
                {
                    return Err("Composition mask row is missing".into());
                }
                let output = if index + 1 == composition.layers.len() {
                    root.clone()
                } else {
                    format!("layer-{}", layer.id)
                };
                let mix = asset
                    .root
                    .project
                    .materials
                    .iter_mut()
                    .find(|m| m.id == output)
                    .and_then(|m| m.mixing.as_mut())
                    .ok_or("Composition mixer is missing")?;
                mix.mask_material = Some(owner.clone());
                mix.mask = format::Binding::From(format::Reference {
                    row: mask.row.clone(),
                    components: mask.components.clone(),
                    mapping: mask.mapping.map(|v| (v[0], v[1])),
                });
            }
            asset.root.project.materials.extend(documents);
            asset
                .root
                .project
                .materials
                .iter_mut()
                .find(|m| m.id == root)
                .ok_or("Composition output is missing")?
                .tiling = denrim_materials::periodic::Tiling {
                mode: denrim_materials::periodic::Mode::parse(&composition.tiling.mode)
                    .map_err(|e| e.to_string())?,
                period: composition.tiling.period,
                blend: composition.tiling.blend,
            }
            .validate()
            .map_err(|e| e.to_string())?;
            asset.validate().map_err(|e| e.to_string())?;
            asset
        } else if let Some(stored) = self.project.procedural_materials.get(&id) {
            EditableMaterialAsset::reopen(stored).map_err(|e| e.to_string())?
        } else {
            raster_source::asset(
                id,
                self.project
                    .tiles
                    .get(&id)
                    .ok_or("Composition source tile is missing")?,
            )?
        };
        self.visiting.remove(&id);
        self.assets.insert(id, asset.clone());
        Ok(asset)
    }
}
fn validate_layer(layer: &CompositionLayer) -> Result<(), String> {
    for (value, min, max) in [
        (layer.position, 0., 1.),
        (layer.angle, -180., 180.),
        (layer.feather, 0., 1.),
        (layer.amount, 0., 1.),
        (layer.wobble, 0., 0.25),
        (layer.wobble_scale, 0.1, 16.),
        (layer.overlay_level, 0., 1.),
    ] {
        if !value.is_finite() || value < min || value > max {
            return Err("Composition values must be finite and within their ranges".into());
        }
    }
    if layer.uv_direction > 1
        || layer
            .uv_levels
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        || layer.uv_levels.windows(2).any(|v| v[0] > v[1])
    {
        return Err("UV levels must be ordered between 0 and 1, with an X or Y direction".into());
    }
    if let Some(mask) = &layer.mask {
        if mask.owner.is_empty()
            || mask.row.is_empty()
            || !(mask.components == "distance"
                || mask.components.len() == 3
                    && mask.components.chars().all(|c| "xyzw".contains(c)))
        {
            return Err("Mask needs a material, row, and three channels or distance".into());
        }
        if mask
            .mapping
            .is_some_and(|mapping| mapping.iter().any(|v| !v.is_finite()))
        {
            return Err("Mask remapping endpoints must be finite".into());
        }
    }
    Ok(())
}

/// All authored rows remain addressable, including rows in a composed source DAG.
fn mask_rows(project: &Project, source: Uuid) -> Vec<(String, String, String)> {
    let Ok(bundle) = bundle(source, project, None) else {
        return vec![];
    };
    let Ok(asset) = EditableMaterialAsset::reopen(&bundle) else {
        return vec![];
    };
    let Ok(document) = asset.resolved(&InstanceRecord::new("composition-mask")) else {
        return vec![];
    };
    document
        .materials
        .into_iter()
        .flat_map(|material| {
            material
                .rows
                .into_iter()
                .filter(|row| !row.tiles.is_empty())
                .map(move |row| {
                    let label = format!("{} · {}", material.label, row.label);
                    (material.id.clone(), row.id, label)
                })
        })
        .collect()
}

pub(super) fn input_key(project: &Project) -> u64 {
    let mut text = serde_json::to_string(&project.material_compositions).unwrap_or_default();
    for (id, bundle) in &project.procedural_materials {
        if !project.material_compositions.contains_key(id) {
            text.push_str(&id.to_string());
            text.push_str(bundle);
        }
    }
    // Removing an asset invalidates dependent compositions as well.
    for (id, tile) in &project.tiles {
        text.push_str(&id.to_string());
        text.push_str(&tile.alias);
    }
    let images: HashSet<_> = project
        .material_compositions
        .values()
        .flat_map(sources)
        .filter(|id| !project.procedural_materials.contains_key(id))
        .collect();
    let mut images: Vec<_> = images.into_iter().collect();
    images.sort();
    for id in images {
        if let Some(tile) = project.tiles.get(&id) {
            text.push_str(&raster_source::key(tile).to_string());
        }
    }
    text.push_str(&serde_json::to_string(&project.map_material_bindings).unwrap_or_default());
    // Hash authored geometry once. Calculating page layouts here would rebuild
    // wall assembly meshes for every material on every editor update.
    if !project.procedural_materials.is_empty() {
        for region in &project.regions {
            let map = &region.map;
            text.push_str(
                &serde_json::to_string(&(
                    map.id,
                    &map.vertices,
                    &map.linedefs,
                    &map.sectors,
                    &map.surfaces,
                    &map.geometry_objects,
                    &map.wall_assemblies,
                    &map.properties,
                ))
                .unwrap_or_default(),
            );
        }
    }
    cache_key(&text)
}

/// Only changes to the inspector's structure replace its native controls.
/// Numeric edits already update their field and must retain focus and drag state.
fn view_key(project: &Project, composition: &MaterialComposition) -> u64 {
    let mut schema = composition.clone();
    for layer in &mut schema.layers {
        layer.wobble = 0.;
        layer.wobble_scale = 2.;
        layer.position = 0.;
        layer.feather = 0.;
        layer.amount = 0.;
        layer.overlay_level = 0.;
        layer.uv_levels = [0.; 3];
        if let Some(mask) = &mut layer.mask {
            if mask.mapping.is_some() {
                mask.mapping = Some([0., 1.]);
            }
        }
    }
    let mut text = serde_json::to_string(&schema).unwrap_or_default();
    let mut choices: Vec<_> = project
        .tiles
        .iter()
        .filter(|(id, _)| !project.map_material_bindings.contains_key(*id))
        .collect();
    choices.sort_by_key(|(id, _)| **id);
    for (id, tile) in choices {
        text.push_str(&id.to_string());
        text.push_str(&tile.alias);
    }
    cache_key(&text)
}

impl ProceduralMaterialEditor {
    pub(super) fn sync_compositions(
        &mut self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        let key = input_key(project);
        if self.composition_key == Some(key) {
            return;
        }
        self.composition_key = Some(key);
        let sources: Vec<_> = project
            .procedural_materials
            .keys()
            .filter(|id| !project.material_compositions.contains_key(*id))
            .copied()
            .collect();
        for id in sources {
            if let Ok(mut asset) = EditableMaterialAsset::reopen(&project.procedural_materials[&id])
            {
                surface_pages::apply(
                    &mut asset,
                    surface_pages::for_template(project, id).as_ref(),
                );
                if let Ok(bundle) = asset.save() {
                    project.procedural_materials.insert(id, bundle);
                }
            }
        }
        let updates: Vec<_> = project
            .material_compositions
            .keys()
            .map(|id| (*id, bundle(*id, project, None)))
            .collect();
        for (id, result) in updates {
            match result {
                Ok(bundle) => {
                    project.procedural_materials.insert(id, bundle);
                }
                Err(error) => Self::status(
                    ui,
                    ctx,
                    format!("Composition: {error}. Keeping the last working preview."),
                ),
            }
        }
        self.composition_key = Some(input_key(project));
    }
    pub(super) fn composition_view(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
    ) {
        let composition = self
            .tile
            .and_then(|id| project.material_compositions.get(&id));
        if let Some(stack) = ui.get_stack_layout(STACK) {
            stack.set_index(usize::from(composition.is_some()));
        }
        let Some(composition) = composition else {
            self.composition_preview = None;
            return;
        };
        let key = (
            self.tile.unwrap(),
            view_key(project, composition),
            self.composition_layer,
        );
        if self.composition_ui_key == Some(key) {
            return;
        }
        self.composition_layer = self
            .composition_layer
            .min(composition.layers.len().saturating_sub(1));
        let target = if self.composition_layer + 1 < composition.layers.len() {
            format!("layer-{}", composition.layers[self.composition_layer].id)
        } else {
            format!("composition-{}", self.tile.unwrap())
        };
        if self.composition_preview.as_ref() != Some(&target) {
            self.composition_preview = Some(target);
            self.preview_dirty = true;
            self.preview_request = self.preview_tile;
        }
        self.composition_choices = project
            .tiles
            .iter()
            .filter(|(id, tile)| {
                Some(**id) != self.tile
                    && !project.map_material_bindings.contains_key(*id)
                    && !tile.textures.is_empty()
            })
            .map(|(id, _)| *id)
            .collect();
        self.composition_choices
            .sort_by_key(|id| (material_label(project, *id), *id));
        let Some(layout) = ui.get_text_layout(INSPECTOR) else {
            return;
        };
        layout.clear();
        if let Some(binding) = &self.map_binding {
            use shared::material_composition::CompositionSurface;
            let label = match binding.surface {
                CompositionSurface::RectTile { sector } => format!("Painted tile #{sector}"),
                CompositionSurface::SectorTile { cell, .. }
                | CompositionSurface::GeometryTile { cell, .. }
                | CompositionSurface::TerrainTile { cell } => {
                    format!("Tile ({}, {})", cell[0], cell[1])
                }
                CompositionSurface::Sector { id, ceiling } => {
                    format!("{} #{id}", if ceiling { "Ceiling" } else { "Floor" })
                }
                CompositionSurface::Linedef { id, row } => format!("Wall #{id} · row {row}"),
                CompositionSurface::GeometryFace { face, .. } => format!("Face {}", face + 1),
                CompositionSurface::WallArea { .. } => "Wall surface".into(),
                CompositionSurface::WallSlot { slot, .. } => {
                    format!("Wall material slot {}", slot + 1)
                }
            };
            let mut text = TheText::new(TheId::named("Composition Surface"));
            text.set_text(label);
            layout.add_pair("On map".into(), Box::new(text));
        }
        let add_material =
            |layout: &mut dyn TheTextLayoutTrait, name: &str, label: &str, selected: Uuid| {
                let mut menu = TheDropdownMenu::new(TheId::named(name));
                menu.limiter_mut().set_max_width(i32::MAX);
                for id in &self.composition_choices {
                    menu.add_option(material_label(project, *id));
                }
                if let Some(index) = self
                    .composition_choices
                    .iter()
                    .position(|id| *id == selected)
                {
                    menu.set_selected_index(index as i32);
                } else {
                    menu.add_option("Missing material".into());
                    menu.set_selected_index(self.composition_choices.len() as i32);
                }
                layout.add_pair(label.into(), Box::new(menu));
            };
        add_material(layout, BASE, "Background", composition.base);
        let mut layers = TheDropdownMenu::new(TheId::named(LAYER));
        layers.limiter_mut().set_max_width(i32::MAX);
        for (i, layer) in composition.layers.iter().enumerate() {
            let label = material_label(project, layer.material);
            let kind = SHAPES
                .iter()
                .find(|(shape, _)| *shape == layer.shape)
                .unwrap()
                .1;
            layers.add_option(format!("{} · {kind} · {label}", i + 1));
        }
        if composition.layers.is_empty() {
            layers.add_option("Base only".into());
        }
        layers.set_selected_index(self.composition_layer as i32);
        layout.add_pair("Layer".into(), Box::new(layers));
        let mut controls = TheGroupButton::new(TheId::named("Composition Actions"));
        for (label, help) in [
            ("Add layer", "Add another layer over the result"),
            ("Remove", "Remove this layer"),
            ("Earlier", "Move this layer earlier"),
            ("Later", "Move this layer later"),
        ] {
            controls.add_text_status(label.into(), help.into());
        }
        controls.set_item_width(74);
        layout.add_pair(String::new(), Box::new(controls));
        let mut save = TheTraybarButton::new(TheId::named(SAVE_COPY));
        save.set_text("Save copy to Tilepicker".into());
        save.set_status_text("Create a reusable copy of these layers. Paint it anywhere, then edit individual placements independently.");
        layout.add_pair(String::new(), Box::new(save));
        if let Some(layer) = composition.layers.get(self.composition_layer) {
            add_material(layout, MATERIAL, "Add material", layer.material);
            if self.map_binding.is_some() {
                let mut button = TheTraybarButton::new(TheId::named("Composition Use Picked"));
                button.set_text("Use picked material".into());
                button.set_status_text("Use the tile selected in the Tilepicker for this layer.");
                layout.add_pair(String::new(), Box::new(button));
            }
            let mut hint = TheText::new(TheId::named("Composition Shape Help"));
            hint.set_text("Highlight = added material. Rest = background.".into());
            hint.limiter_mut().set_max_height(34);
            layout.add_pair(String::new(), Box::new(hint));
            layout.add_pair(String::new(), Box::new(ShapePicker::for_layer(layer)));
            let mut advanced = TheTraybarButton::new(TheId::named("Composition Advanced"));
            advanced.set_text(
                if self.composition_advanced {
                    "Hide advanced"
                } else {
                    "Advanced mixing…"
                }
                .into(),
            );
            layout.add_pair(String::new(), Box::new(advanced));
            if self.composition_advanced {
                let mut shape = TheDropdownMenu::new(TheId::named(SHAPE));
                for (_, label) in SHAPES {
                    shape.add_option(label.into());
                }
                shape.set_selected_index(
                    SHAPES
                        .iter()
                        .position(|(shape, _)| *shape == layer.shape)
                        .unwrap() as i32,
                );
                layout.add_pair("Shape".into(), Box::new(shape));
            }
            let mut fields = match layer.shape {
                CompositionShape::Split => vec![
                    (POSITION, "Edge position", layer.position, 0., 1.),
                    (FEATHER, "Soft edge", layer.feather, 0., 1.),
                ],
                CompositionShape::Blend => vec![(AMOUNT, "Amount", layer.amount, 0., 1.)],
                CompositionShape::Road | CompositionShape::Crossing => vec![
                    (POSITION, "Road width", layer.position, 0., 1.),
                    (FEATHER, "Soft edge", layer.feather, 0., 1.),
                ],
                CompositionShape::Corner => vec![
                    (POSITION, "Corner size", layer.position, 0., 1.),
                    (FEATHER, "Soft edge", layer.feather, 0., 1.),
                ],
                CompositionShape::Strips => vec![
                    (POSITION, "Strip width", layer.position, 0., 0.5),
                    (FEATHER, "Soft edge", layer.feather, 0., 1.),
                ],
                CompositionShape::Overlay if self.composition_advanced => {
                    vec![(OVERLAY, "Level", layer.overlay_level, 0., 1.)]
                }
                _ => vec![],
            };
            if matches!(
                layer.shape,
                CompositionShape::Split
                    | CompositionShape::Corner
                    | CompositionShape::Strips
                    | CompositionShape::Road
                    | CompositionShape::Crossing
            ) {
                fields.push((WOBBLE, "Uneven edge", layer.wobble, 0., 0.25));
                if self.composition_advanced {
                    fields.push((WOBBLE_SCALE, "Edge detail", layer.wobble_scale, 0.1, 16.));
                }
            }
            if self.composition_advanced && layer.shape == CompositionShape::Split {
                fields.push((ANGLE, "Angle", layer.angle, -180., 180.));
            }
            let optional_material = |layout: &mut dyn TheTextLayoutTrait,
                                     name: &str,
                                     label: &str,
                                     selected: Option<Uuid>,
                                     none: &str| {
                let mut menu = TheDropdownMenu::new(TheId::named(name));
                menu.limiter_mut().set_max_width(i32::MAX);
                menu.add_option(none.into());
                for id in &self.composition_choices {
                    menu.add_option(material_label(project, *id));
                }
                menu.set_selected_index(
                    selected
                        .and_then(|selected| {
                            self.composition_choices
                                .iter()
                                .position(|id| *id == selected)
                        })
                        .map_or(0, |index| index + 1) as i32,
                );
                layout.add_pair(label.into(), Box::new(menu));
            };
            if self.composition_advanced && layer.shape == CompositionShape::Mask {
                optional_material(
                    layout,
                    MASK_MATERIAL,
                    "Mask source",
                    layer.mask.as_ref().map(|mask| mask.material),
                    "Constant",
                );
                if let Some(mask) = &layer.mask {
                    let rows = mask_rows(project, mask.material);
                    let mut menu = TheDropdownMenu::new(TheId::named(MASK_ROW));
                    menu.limiter_mut().set_max_width(i32::MAX);
                    for (_, _, label) in &rows {
                        menu.add_option(label.clone());
                    }
                    if let Some(index) = rows
                        .iter()
                        .position(|(owner, row, _)| *owner == mask.owner && *row == mask.row)
                    {
                        menu.set_selected_index(index as i32);
                    } else {
                        menu.add_option("Missing row".into());
                        menu.set_selected_index(rows.len() as i32);
                    }
                    layout.add_pair("Mask row".into(), Box::new(menu));
                    let mut menu = TheDropdownMenu::new(TheId::named(MASK_CHANNEL));
                    for label in ["R", "G", "B", "Opacity", "RGB", "Distance"] {
                        menu.add_option(label.into());
                    }
                    menu.set_selected_index(
                        CHANNELS
                            .iter()
                            .position(|channels| *channels == mask.components)
                            .unwrap_or(4) as i32,
                    );
                    layout.add_pair("Channels".into(), Box::new(menu));
                    let mut menu = TheDropdownMenu::new(TheId::named(MASK_REMAP));
                    menu.add_option("Off".into());
                    menu.add_option("On".into());
                    menu.set_selected_index(i32::from(mask.mapping.is_some()));
                    layout.add_pair("Remap".into(), Box::new(menu));
                    if let Some([low, high]) = mask.mapping {
                        fields.extend([
                            (MASK_MIN, "Remap low", low, -1000., 1000.),
                            (MASK_MAX, "Remap high", high, -1000., 1000.),
                        ]);
                    }
                } else {
                    fields.push((AMOUNT, "Amount", layer.amount, 0., 1.));
                }
                let mut menu = TheDropdownMenu::new(TheId::named(BLEND));
                menu.add_option("Off · transparent edge".into());
                menu.add_option("On".into());
                menu.set_selected_index(i32::from(layer.blend));
                layout.add_pair("Blend".into(), Box::new(menu));
            }
            if self.composition_advanced && layer.shape == CompositionShape::UvPosition {
                for (slot, name) in EXTRAS.iter().enumerate() {
                    optional_material(
                        layout,
                        name,
                        if slot == 0 {
                            "Material 3"
                        } else {
                            "Material 4"
                        },
                        layer.extra_materials[slot],
                        "Unconnected",
                    );
                }
                let mut menu = TheDropdownMenu::new(TheId::named(DIRECTION));
                menu.add_option("X".into());
                menu.add_option("Y".into());
                menu.set_selected_index(layer.uv_direction as i32);
                layout.add_pair("Direction".into(), Box::new(menu));
                for (index, id) in LEVELS.iter().enumerate() {
                    fields.push((
                        *id,
                        ["Level 1", "Level 2", "Level 3"][index],
                        layer.uv_levels[index],
                        0.,
                        1.,
                    ));
                }
            }
            if self.composition_advanced && layer.shape == CompositionShape::Height {
                let mut text = TheText::new(TheId::named("Composition Height Help"));
                text.set_text(
                    "The material with the greater Height wins; ties keep the base.".into(),
                );
                layout.add_pair(String::new(), Box::new(text));
            }
            for (id, label, value, min, max) in fields {
                let mut edit = TheTextLineEdit::new(TheId::named(id));
                edit.set_range(TheValue::RangeF32(min..=max));
                edit.set_value(TheValue::Float(value));
                edit.limiter_mut().set_max_width(i32::MAX);
                edit.set_status_text("Drag to adjust, or type a value and press Return.");
                layout.add_pair(label.into(), Box::new(edit));
            }
        }
        self.composition_ui_key = Some((key.0, key.1, self.composition_layer));
        ctx.ui.relayout = true;
        ctx.ui.redraw_all = true;
    }
    fn save_composition_copy(
        &self,
        project: &mut Project,
        ctx: &mut TheContext,
        server: &ServerContext,
    ) -> Result<Uuid, String> {
        let source = self.tile.ok_or("Select a composition first")?;
        let composition = project
            .material_compositions
            .get(&source)
            .cloned()
            .ok_or("Select a composition first")?;
        let mut tile = project
            .tiles
            .get(&source)
            .cloned()
            .ok_or("Composition tile is missing")?;
        tile.id = Uuid::new_v4();
        let id = tile.id;
        tile.textures = vec![
            self.frame
                .as_ref()
                .map_or_else(|| tile.textures[0].clone(), |f| f.tile.clone()),
        ];
        tile.uv_pages = None;
        tile.baked_material_data = vec![tile.textures[0].material_bytes()];
        tile.alias = format!("{} composition", material_label(project, composition.base));
        let bundle = bundle(id, project, Some(&composition))?;
        let before = project.clone();
        project.tiles.insert(id, tile);
        project.material_compositions.insert(id, composition);
        project.procedural_materials.insert(id, bundle);
        project.place_tile_at_picker_selection(id, server.curr_tile_board_cell);
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::TilePickerEdit(Box::new(before), Box::new(project.clone())),
            ctx,
        );
        Ok(id)
    }
    pub(super) fn composition_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &ServerContext,
    ) -> bool {
        let Some(tile) = self.tile else {
            return false;
        };
        let Some(before) = project.material_compositions.get(&tile).cloned() else {
            return false;
        };
        let mut after = before.clone();
        let name;
        match event {
            TheEvent::IndexChanged(id, index) if id.name == "Material Tiling" => {
                ctx.ui.set_focus(id);
                let Some(mode) = ["raw", "repeat", "seamless"].get(*index) else {
                    return true;
                };
                after.tiling.mode = (*mode).into();
                name = "";
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) if id.name == SAVE_COPY => {
                match self.save_composition_copy(project, ctx, server) {
                    Ok(id) => {
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Tilepicker"),
                            TheValue::Empty,
                        ));
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Pick Tile Source"),
                            TheValue::List(vec![TheValue::Text("single".into()), TheValue::Id(id)]),
                        ));
                        Self::status(
                            ui,
                            ctx,
                            "Composition copied to Tilepicker. Pick it and paint; each edited placement is independent.",
                        );
                    }
                    Err(error) => Self::status(ui, ctx, error),
                }
                return true;
            }

            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "Composition Advanced" =>
            {
                self.composition_advanced = !self.composition_advanced;
                self.composition_ui_key = None;
                return true;
            }
            TheEvent::IndexChanged(id, index) if id.name == shape_picker::PICKER => {
                if let (Some(layer), Some(preset)) = (
                    after.layers.get_mut(self.composition_layer),
                    PRESETS.get(*index),
                ) {
                    preset.apply(layer);
                }
                name = "";
            }

            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "Composition Use Picked" =>
            {
                let Some(material) = server.curr_tile_id.filter(|id| {
                    *id != tile
                        && project
                            .tiles
                            .get(id)
                            .is_some_and(|tile| !tile.textures.is_empty())
                        && !project.map_material_bindings.contains_key(id)
                }) else {
                    Self::status(ui, ctx, "Pick a tile in the Tilepicker first.");
                    return true;
                };
                if let Some(layer) = after.layers.get_mut(self.composition_layer) {
                    layer.material = material;
                }
                name = "";
            }
            TheEvent::IndexChanged(id, index) if id.name == LAYER => {
                if self.composition_layer != *index {
                    self.composition_layer = *index;
                    self.composition_ui_key = None;
                }
                return true;
            }
            TheEvent::IndexChanged(id, index) if id.name == "Composition Actions" => {
                name = match index {
                    0 => ADD,
                    1 => REMOVE,
                    2 => UP,
                    _ => DOWN,
                };
            }
            TheEvent::IndexChanged(id, index)
                if matches!(id.name.as_str(), BASE | MATERIAL | SHAPE) =>
            {
                if id.name == SHAPE {
                    if let Some(layer) = after.layers.get_mut(self.composition_layer) {
                        if let Some((shape, _)) = SHAPES.get(*index) {
                            layer.shape = *shape;
                            // Inactive settings are retained between modes, but
                            // an asset/row removed since then must not trap the
                            // user outside the controls needed to repair it.
                            if *shape == CompositionShape::Mask
                                && layer.mask.as_ref().is_some_and(|mask| {
                                    !mask_rows(project, mask.material).iter().any(
                                        |(owner, row, _)| *owner == mask.owner && *row == mask.row,
                                    )
                                })
                            {
                                layer.mask = None;
                            }
                            if *shape == CompositionShape::UvPosition {
                                for source in &mut layer.extra_materials {
                                    if source.is_some_and(|source| {
                                        !project.tiles.contains_key(&source)
                                            || project.tiles[&source].textures.is_empty()
                                    }) {
                                        *source = None;
                                    }
                                }
                            }
                        }
                    }
                } else if let Some(source) = self.composition_choices.get(*index) {
                    if id.name == BASE {
                        after.base = *source;
                    } else if let Some(layer) = after.layers.get_mut(self.composition_layer) {
                        layer.material = *source;
                    }
                }
                name = "";
            }
            TheEvent::IndexChanged(id, index)
                if matches!(
                    id.name.as_str(),
                    MASK_MATERIAL | MASK_ROW | MASK_CHANNEL | MASK_REMAP | BLEND | DIRECTION
                ) || EXTRAS.contains(&id.name.as_str()) =>
            {
                if let Some(layer) = after.layers.get_mut(self.composition_layer) {
                    match id.name.as_str() {
                        MASK_MATERIAL => {
                            if *index == 0 {
                                layer.mask = None;
                            } else if let Some(source) = self.composition_choices.get(index - 1) {
                                let rows = mask_rows(project, *source);
                                if let Some((owner, row, _)) = rows.first() {
                                    layer.mask = Some(CompositionMask {
                                        material: *source,
                                        owner: owner.clone(),
                                        row: row.clone(),
                                        components: "xxx".into(),
                                        mapping: None,
                                    });
                                } else {
                                    Self::status(
                                        ui,
                                        ctx,
                                        "This source has no populated rows to use as a mask.",
                                    );
                                }
                            }
                        }
                        MASK_ROW => {
                            if let Some(mask) = &mut layer.mask {
                                if let Some((owner, row, _)) =
                                    mask_rows(project, mask.material).get(*index)
                                {
                                    mask.owner = owner.clone();
                                    mask.row = row.clone();
                                }
                            }
                        }
                        MASK_CHANNEL => {
                            if let (Some(mask), Some(channels)) =
                                (&mut layer.mask, CHANNELS.get(*index))
                            {
                                mask.components = (*channels).into();
                            }
                        }
                        MASK_REMAP => {
                            if let Some(mask) = &mut layer.mask {
                                mask.mapping = if *index == 0 { None } else { Some([0., 1.]) };
                            }
                        }
                        BLEND => layer.blend = *index != 0,
                        DIRECTION => layer.uv_direction = *index,
                        name => {
                            if let Some(slot) = EXTRAS.iter().position(|id| *id == name) {
                                layer.extra_materials[slot] = index
                                    .checked_sub(1)
                                    .and_then(|index| self.composition_choices.get(index))
                                    .copied();
                            }
                        }
                    }
                }
                name = "";
            }
            TheEvent::ValueChanged(id, value)
                if matches!(
                    id.name.as_str(),
                    POSITION
                        | ANGLE
                        | FEATHER
                        | AMOUNT
                        | OVERLAY
                        | MASK_MIN
                        | MASK_MAX
                        | WOBBLE
                        | WOBBLE_SCALE
                ) || LEVELS.contains(&id.name.as_str()) =>
            {
                if let Some(layer) = after.layers.get_mut(self.composition_layer) {
                    let value = value.to_f32().unwrap_or(f32::NAN);
                    match id.name.as_str() {
                        POSITION => layer.position = value,
                        WOBBLE => layer.wobble = value,
                        WOBBLE_SCALE => layer.wobble_scale = value,
                        ANGLE => layer.angle = value,
                        FEATHER => layer.feather = value,
                        AMOUNT => layer.amount = value,
                        OVERLAY => layer.overlay_level = value,
                        MASK_MIN | MASK_MAX => {
                            if let Some(mapping) =
                                layer.mask.as_mut().and_then(|mask| mask.mapping.as_mut())
                            {
                                mapping[usize::from(id.name == MASK_MAX)] = value;
                            }
                        }
                        name => {
                            if let Some(index) = LEVELS.iter().position(|id| *id == name) {
                                layer.uv_levels[index] = value;
                            }
                        }
                    }
                }
                name = "";
            }
            _ => return false,
        }
        match name {
            ADD => {
                let source = server
                    .curr_tile_id
                    .filter(|id| {
                        *id != tile
                            && project
                                .tiles
                                .get(id)
                                .is_some_and(|tile| !tile.textures.is_empty())
                            && !project.map_material_bindings.contains_key(id)
                    })
                    .unwrap_or_else(|| after.layers.last().map_or(after.base, |l| l.material));
                after.layers.push(CompositionLayer::split(source));
                self.composition_layer = after.layers.len() - 1;
            }
            REMOVE if self.composition_layer < after.layers.len() => {
                after.layers.remove(self.composition_layer);
            }
            UP if self.composition_layer > 0 && self.composition_layer < after.layers.len() => {
                after
                    .layers
                    .swap(self.composition_layer, self.composition_layer - 1);
                self.composition_layer -= 1;
            }
            DOWN if self.composition_layer + 1 < after.layers.len() => {
                after
                    .layers
                    .swap(self.composition_layer, self.composition_layer + 1);
                self.composition_layer += 1;
            }
            _ => {}
        }
        let document_changed = before != after;
        if document_changed {
            match bundle(tile, project, Some(&after)) {
                Ok(bundle) => {
                    project.material_compositions.insert(tile, after.clone());
                    project.procedural_materials.insert(tile, bundle);
                    UNDOMANAGER.write().unwrap().add_undo(
                        ProjectUndoAtom::MaterialCompositionEdit(tile, before, after),
                        ctx,
                    );
                }
                Err(error) => {
                    self.composition_ui_key = None;
                    Self::status(ui, ctx, format!("Composition: {error}"));
                }
            }
        }
        if document_changed && !matches!(event, TheEvent::ValueChanged(..)) {
            self.composition_ui_key = None;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use denrim_materials::{cpu::Evaluator, ir::Channel};
    fn source(project: &mut Project, color: [f32; 3], roughness: f32) -> Uuid {
        let tile = rusterix::Tile::from_textures(vec![rusterix::Texture::alloc(4, 4)]);
        let id = tile.id;
        let text = format!(
            "project 1\nmaterial \"surface\" called \"Source\"\n  mode pbr\n  default base color to {} {} {}\n  default opacity to 0.8\n  default roughness to {roughness}\n  default metallic to 0.2\n  default emission to 0.1 0.2 0.3\n",
            color[0], color[1], color[2]
        );
        let asset = EditableMaterialAsset::parse(&id.to_string(), &text, "surface").unwrap();
        project.tiles.insert(id, tile);
        project
            .procedural_materials
            .insert(id, asset.save().unwrap());
        id
    }
    fn composition_tile(project: &mut Project, c: MaterialComposition) -> Uuid {
        let tile = rusterix::Tile::from_textures(vec![rusterix::Texture::alloc(4, 4)]);
        let id = tile.id;
        let text = bundle(id, project, Some(&c)).unwrap();
        project.tiles.insert(id, tile);
        project.procedural_materials.insert(id, text);
        project.material_compositions.insert(id, c);
        id
    }
    fn sample(project: &Project, id: Uuid, uv: [f32; 2], channel: Channel) -> Vec<f32> {
        let asset = EditableMaterialAsset::reopen(&bundle(id, project, None).unwrap()).unwrap();
        let p = asset.resolved(&InstanceRecord::new("test")).unwrap();
        let p = denrim_materials::compile(&format::write(&p)).unwrap();
        Evaluator::new(&p)
            .sample(&asset.selected_material, channel, uv)
            .unwrap()
            .to_vec()
    }
    fn default_value(project: &mut Project, id: Uuid, name: &str, value: Vec<f32>) {
        let mut asset = EditableMaterialAsset::reopen(&project.procedural_materials[&id]).unwrap();
        let defaults = &mut asset.root.project.materials[0].defaults;
        defaults.retain(|(channel, _)| channel != name);
        defaults.push((name.into(), value));
        project
            .procedural_materials
            .insert(id, asset.save().unwrap());
    }
    fn mask_source(project: &mut Project, color: [f32; 4]) -> Uuid {
        let id = source(project, [0.; 3], 0.);
        let recipe = format!(
            "project 1\nmaterial \"surface\" called \"Mask\"\n  mode pbr\n  row \"mask\" called \"Mask row\"\n    use \"Color\"\n      set \"color\" to {} {} {} {}\n",
            color[0], color[1], color[2], color[3]
        );
        let asset = EditableMaterialAsset::parse(&id.to_string(), &recipe, "surface").unwrap();
        project
            .procedural_materials
            .insert(id, asset.save().unwrap());
        id
    }
    #[test]
    fn row_mask_supports_channels_remap_transparency_and_roundtrip() {
        let mut p = Project::default();
        let a = source(&mut p, [1., 0., 0.], 0.2);
        let b = source(&mut p, [0., 1., 0.], 0.8);
        let mask = mask_source(&mut p, [0.25, 0.75, 1., 0.5]);
        let mut layer = CompositionLayer::split(b);
        layer.shape = CompositionShape::Mask;
        layer.mask = Some(CompositionMask {
            material: mask,
            owner: "surface".into(),
            row: "mask".into(),
            components: "yyy".into(),
            mapping: Some([1., 0.]),
        });
        let id = composition_tile(
            &mut p,
            MaterialComposition {
                tiling: Default::default(),
                base: a,
                layers: vec![layer],
            },
        );
        let v = sample(&p, id, [0., 0.], Channel::BaseColor);
        assert_eq!(v, vec![0.75, 0.25, 0.]);
        assert!((sample(&p, id, [0., 0.], Channel::Roughness)[0] - 0.35).abs() < 1e-5);
        p = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(sample(&p, id, [0., 0.], Channel::BaseColor), v);
        p.material_compositions.get_mut(&id).unwrap().layers[0].blend = false;
        assert_eq!(sample(&p, id, [0., 0.], Channel::Opacity), vec![0.]);
        p.material_compositions.get_mut(&id).unwrap().layers[0]
            .mask
            .as_mut()
            .unwrap()
            .components = "zzz".into();
        assert_eq!(
            sample(&p, id, [0., 0.], Channel::BaseColor),
            vec![1., 0., 0.]
        );
        p.material_compositions.get_mut(&id).unwrap().layers[0]
            .mask
            .as_mut()
            .unwrap()
            .row = "missing".into();
        assert!(bundle(id, &p, None).unwrap_err().contains("mask row"));
    }
    #[test]
    fn height_and_overlay_select_complete_surfaces_and_preserve_height_in_chains() {
        let mut p = Project::default();
        let a = source(&mut p, [1., 0., 0.], 0.2);
        let b = source(&mut p, [0., 1., 0.], 0.8);
        default_value(&mut p, a, "height", vec![0.4]);
        default_value(&mut p, b, "height", vec![0.6]);
        default_value(&mut p, b, "opacity", vec![0.3]);
        let mut layer = CompositionLayer::split(b);
        layer.shape = CompositionShape::Height;
        let id = composition_tile(
            &mut p,
            MaterialComposition {
                tiling: Default::default(),
                base: a,
                layers: vec![layer],
            },
        );
        assert_eq!(
            sample(&p, id, [0., 0.], Channel::BaseColor),
            vec![0., 1., 0.]
        );
        assert_eq!(sample(&p, id, [0., 0.], Channel::Opacity), vec![0.3]);
        default_value(&mut p, b, "height", vec![0.4]);
        assert_eq!(
            sample(&p, id, [0., 0.], Channel::BaseColor),
            vec![1., 0., 0.],
            "Ties keep the base"
        );
        let layer = &mut p.material_compositions.get_mut(&id).unwrap().layers[0];
        layer.shape = CompositionShape::Overlay;
        layer.overlay_level = 0.2;
        assert_eq!(
            sample(&p, id, [0., 0.], Channel::BaseColor),
            vec![0., 1., 0.]
        );
        assert_eq!(sample(&p, id, [0., 0.], Channel::Height), vec![0.8]);
        p.material_compositions.get_mut(&id).unwrap().layers[0].overlay_level = 0.5;
        assert_eq!(
            sample(&p, id, [0., 0.], Channel::BaseColor),
            vec![1., 0., 0.]
        );
    }
    #[test]
    fn uv_position_uses_four_inputs_ordered_levels_and_direction() {
        let mut p = Project::default();
        let a = source(&mut p, [1., 0., 0.], 0.1);
        let b = source(&mut p, [0., 1., 0.], 0.2);
        let c = source(&mut p, [0., 0., 1.], 0.3);
        let d = source(&mut p, [1., 1., 1.], 0.4);
        let mut layer = CompositionLayer::split(b);
        layer.shape = CompositionShape::UvPosition;
        layer.extra_materials = [Some(c), Some(d)];
        let id = composition_tile(
            &mut p,
            MaterialComposition {
                tiling: Default::default(),
                base: a,
                layers: vec![layer],
            },
        );
        for (x, color) in [
            (0.1, vec![1., 0., 0.]),
            (0.4, vec![0., 1., 0.]),
            (0.6, vec![0., 0., 1.]),
            (0.9, vec![1., 1., 1.]),
        ] {
            assert_eq!(sample(&p, id, [x, 0.], Channel::BaseColor), color);
            assert_eq!(
                sample(&p, id, [x + 1., 0.], Channel::BaseColor),
                color,
                "The selector repeats per tile"
            );
        }
        p.material_compositions.get_mut(&id).unwrap().layers[0].uv_direction = 1;
        assert_eq!(
            sample(&p, id, [0.1, 0.9], Channel::BaseColor),
            vec![1., 1., 1.]
        );
        p.material_compositions.get_mut(&id).unwrap().layers[0].uv_levels = [0.8, 0.5, 0.7];
        assert!(bundle(id, &p, None).unwrap_err().contains("ordered"));
        p.material_compositions.get_mut(&id).unwrap().layers[0].uv_levels = [0.25, 0.5, 0.75];
        p.material_compositions.get_mut(&id).unwrap().layers[0].extra_materials[0] = Some(id);
        assert!(bundle(id, &p, None).unwrap_err().contains("cycle"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[ignore = "requires a GPU device"]
    fn gpu_image_tiles_and_wobbly_roads_match_cpu_in_world_uv() {
        let (device, queue) = pollster::block_on(async {
            let adapter = wgpu::Instance::default()
                .request_adapter(&Default::default())
                .await
                .unwrap();
            adapter.request_device(&Default::default()).await.unwrap()
        });
        let worker = gpu::Worker::new(device, queue);
        let mut p = Project::default();
        let a = image_tile(&mut p);
        let b = source(&mut p, [0.1, 0.2, 0.7], 0.8);
        let mut layer = CompositionLayer::split(b);
        layer.shape = CompositionShape::Crossing;
        layer.angle = 45.;
        layer.position = 0.3;
        layer.wobble = 0.13;
        layer.wobble_scale = 1.7;
        layer.feather = 0.15;
        let id = composition_tile(
            &mut p,
            MaterialComposition {
                tiling: Default::default(),
                base: a,
                layers: vec![layer],
            },
        );
        p.map_material_bindings.insert(
            id,
            shared::material_composition::MapMaterialBinding {
                map: Uuid::new_v4(),
                surface: shared::material_composition::CompositionSurface::TerrainTile {
                    cell: [-2, 3],
                },
            },
        );
        let text = bundle(id, &p, None).unwrap();
        let asset = EditableMaterialAsset::reopen(&text).unwrap();
        assert_eq!(
            asset
                .instances
                .iter()
                .find(|r| r.id == SURFACE_DOMAIN)
                .unwrap()
                .uv_transform[4..],
            [-2., 3.]
        );
        let program = denrim_materials::compile(&format::write(
            &asset.resolved(&InstanceRecord::new("test")).unwrap(),
        ))
        .unwrap();
        let eval = Evaluator::new(&program);
        worker.send(gpu::Job {
            pixels_per_tile: 0,
            tile: id,
            revision: 1,
            bundle: text,
            continuous: false,
            selection: None,
            preview_aspect: 1.,
        });
        let frame = worker
            .results
            .recv_timeout(std::time::Duration::from_secs(60))
            .unwrap()
            .frame
            .unwrap();
        for (size, images) in [
            (
                frame.size,
                frame.images.iter().map(Vec::as_slice).collect::<Vec<_>>(),
            ),
            (frame.tile.width, vec![frame.tile.data.as_slice()]),
        ] {
            for y in [size / 8, size / 3, size * 3 / 4] {
                for x in [size / 8, size / 3, size * 3 / 4] {
                    let uv = [
                        -2. + (x as f32 + 0.5) / size as f32,
                        3. + (y as f32 + 0.5) / size as f32,
                    ];
                    for (channel, index, srgb) in [
                        (Channel::BaseColor, 0, true),
                        (Channel::Opacity, 1, false),
                        (Channel::Roughness, 2, false),
                        (Channel::Metallic, 3, false),
                        (Channel::Emission, 4, true),
                    ] {
                        if index >= images.len() {
                            continue;
                        }
                        let expected = eval.sample(&asset.selected_material, channel, uv).unwrap();
                        let at = (y * size + x) * 4;
                        for (c, v) in expected.iter().enumerate() {
                            let v = if srgb {
                                if *v <= 0.0031308 {
                                    *v * 12.92
                                } else {
                                    1.055 * v.powf(1. / 2.4) - 0.055
                                }
                            } else {
                                *v
                            };
                            assert!(
                                (images[index][at + c] as f32 - v * 255.).abs() <= 3.,
                                "{channel:?} {uv:?} component {c}"
                            );
                        }
                    }
                }
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[ignore = "requires a GPU device"]
    fn gpu_all_mix_modes_match_cpu_and_bake() {
        let (device, queue) = pollster::block_on(async {
            let adapter = wgpu::Instance::default()
                .request_adapter(&Default::default())
                .await
                .unwrap();
            adapter.request_device(&Default::default()).await.unwrap()
        });
        let worker = gpu::Worker::new(device, queue);
        let mut p = Project::default();
        let a = source(&mut p, [1., 0., 0.], 0.2);
        let b = source(&mut p, [0., 1., 0.], 0.8);
        let c = source(&mut p, [0., 0., 1.], 0.6);
        let d = source(&mut p, [1., 1., 1.], 0.4);
        let mask = mask_source(&mut p, [0.25, 0.75, 1., 0.5]);
        default_value(&mut p, a, "height", vec![0.1]);
        default_value(&mut p, b, "height", vec![0.4]);
        let mut layer = CompositionLayer::split(b);
        layer.extra_materials = [Some(c), Some(d)];
        layer.mask = Some(CompositionMask {
            material: mask,
            owner: "surface".into(),
            row: "mask".into(),
            components: "yyy".into(),
            mapping: Some([1., 0.]),
        });
        let id = composition_tile(
            &mut p,
            MaterialComposition {
                tiling: Default::default(),
                base: a,
                layers: vec![layer],
            },
        );
        for (revision, (shape, _)) in SHAPES.iter().enumerate() {
            p.material_compositions.get_mut(&id).unwrap().layers[0].shape = *shape;
            worker.send(gpu::Job {
                pixels_per_tile: 0,
                tile: id,
                revision: revision as u64,
                bundle: bundle(id, &p, None).unwrap(),
                continuous: false,
                selection: None,
                preview_aspect: 1.,
            });
            let frame = worker
                .results
                .recv_timeout(std::time::Duration::from_secs(60))
                .unwrap()
                .frame
                .unwrap();
            let x = frame.size / 8;
            let at = (x * frame.size + x) * 4;
            let uv = [(x as f32 + 0.5) / frame.size as f32; 2];
            for (channel, index, srgb) in [
                (Channel::BaseColor, 0, true),
                (Channel::Roughness, 2, false),
                (Channel::Metallic, 3, false),
                (Channel::Emission, 4, true),
            ] {
                let values = sample(&p, id, uv, channel);
                for (component, value) in values.into_iter().enumerate() {
                    let encoded = if srgb {
                        if value <= 0.0031308 {
                            12.92 * value
                        } else {
                            1.055 * value.powf(1. / 2.4) - 0.055
                        }
                    } else {
                        value
                    };
                    assert!(
                        (frame.images[index][at + component] as f32 - encoded * 255.).abs() < 2.,
                        "{shape:?} {channel:?} component {component}"
                    );
                }
            }
            if *shape == CompositionShape::UvPosition {
                for (quarter, expected) in [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 255]]
                    .iter()
                    .enumerate()
                {
                    let x = frame.size * (quarter * 2 + 1) / 8;
                    let at = (frame.size / 8 * frame.size + x) * 4;
                    assert_eq!(
                        &frame.images[0][at..at + 3],
                        expected,
                        "All four sources cover their authored quarters of one baked tile"
                    );
                    let x = frame.tile.width * (quarter * 2 + 1) / 8;
                    let at = (frame.tile.height / 8 * frame.tile.width + x) * 4;
                    assert_eq!(
                        &frame.tile.data[at..at + 3],
                        expected,
                        "The runtime bake has the same four-way coverage"
                    );
                }
            }
            assert!(!frame.tile.material_bytes().is_empty());
        }
    }
    #[test]
    fn split_blend_chain_is_ordered_and_mixes_every_pbr_channel() {
        let mut p = Project::default();
        let a = source(&mut p, [1., 0., 0.], 0.2);
        let b = source(&mut p, [0., 1., 0.], 0.8);
        let c = source(&mut p, [0., 0., 1.], 0.4);
        let split = CompositionLayer::split(b);
        let mut blend = CompositionLayer::split(c);
        blend.shape = CompositionShape::Blend;
        blend.amount = 0.25;
        let id = composition_tile(
            &mut p,
            MaterialComposition {
                tiling: Default::default(),
                base: a,
                layers: vec![split, blend],
            },
        );
        assert_eq!(
            sample(&p, id, [0.25, 0.5], Channel::BaseColor),
            vec![0.75, 0., 0.25]
        );
        assert_eq!(
            sample(&p, id, [0.75, 0.5], Channel::BaseColor),
            vec![0., 0.75, 0.25]
        );
        assert!((sample(&p, id, [0.75, 0.5], Channel::Roughness)[0] - 0.7).abs() < 1e-5);
        assert!((sample(&p, id, [0.75, 0.5], Channel::Opacity)[0] - 0.8).abs() < 1e-5);
        assert!((sample(&p, id, [0.75, 0.5], Channel::Metallic)[0] - 0.2).abs() < 1e-5);
        assert!((sample(&p, id, [0.75, 0.5], Channel::Emission)[2] - 0.3).abs() < 1e-5);
        let layers = &mut p.material_compositions.get_mut(&id).unwrap().layers;
        layers.swap(0, 1);
        assert_eq!(
            sample(&p, id, [0.75, 0.5], Channel::BaseColor),
            vec![0., 1., 0.]
        );
    }
    #[test]
    fn split_position_rotation_feather_and_tile_local_mask_are_independent() {
        let mut p = Project::default();
        let a = source(&mut p, [1., 0., 0.], 0.2);
        let b = source(&mut p, [0., 1., 0.], 0.8);
        let mut layer = CompositionLayer::split(b);
        layer.position = 0.25;
        layer.feather = 0.2;
        let id = composition_tile(
            &mut p,
            MaterialComposition {
                tiling: Default::default(),
                base: a,
                layers: vec![layer],
            },
        );
        let v = sample(&p, id, [0.25, 0.5], Channel::BaseColor);
        assert!((v[0] - 0.5).abs() < 1e-5);
        assert_eq!(v, sample(&p, id, [-1.75, 3.5], Channel::BaseColor));
        let layer = &mut p.material_compositions.get_mut(&id).unwrap().layers[0];
        layer.angle = 90.;
        layer.feather = 0.;
        assert_eq!(
            sample(&p, id, [0.1, 0.8], Channel::BaseColor),
            vec![0., 1., 0.]
        );
        assert_eq!(
            sample(&p, id, [0.9, 0.1], Channel::BaseColor),
            vec![1., 0., 0.]
        );
    }
    #[test]
    fn persistence_references_nested_compositions_and_invalid_inputs() {
        let mut p = Project::default();
        let a = source(&mut p, [1., 0., 0.], 0.2);
        let b = source(&mut p, [0., 1., 0.], 0.8);
        let inner = composition_tile(
            &mut p,
            MaterialComposition {
                tiling: Default::default(),
                base: a,
                layers: vec![CompositionLayer::split(b)],
            },
        );
        let outer = composition_tile(
            &mut p,
            MaterialComposition {
                tiling: Default::default(),
                base: inner,
                layers: vec![CompositionLayer::split(a)],
            },
        );
        let saved = serde_json::to_string(&p).unwrap();
        let mut p: Project = serde_json::from_str(&saved).unwrap();
        assert_eq!(
            sample(&p, outer, [0.2, 0.4], Channel::BaseColor),
            vec![1., 0., 0.]
        );
        let old = bundle(outer, &p, None).unwrap();
        let mut changed = EditableMaterialAsset::reopen(&p.procedural_materials[&a]).unwrap();
        changed.root.project.materials[0].defaults[0].1 = vec![0., 0., 1.];
        p.procedural_materials.insert(a, changed.save().unwrap());
        assert_ne!(old, bundle(outer, &p, None).unwrap());
        assert_eq!(
            sample(&p, outer, [0.2, 0.4], Channel::BaseColor),
            vec![0., 0., 1.]
        );
        p.material_compositions.get_mut(&inner).unwrap().base = outer;
        assert!(bundle(outer, &p, None).unwrap_err().contains("cycle"));
        p.material_compositions.get_mut(&inner).unwrap().base = a;
        p.material_compositions.get_mut(&outer).unwrap().layers[0].position = f32::NAN;
        assert!(bundle(outer, &p, None).unwrap_err().contains("finite"));
        p.material_compositions
            .get_mut(&outer)
            .unwrap()
            .layers
            .clear();
        assert!(bundle(outer, &p, None).is_ok());
        p.tiles.shift_remove(&a);
        assert!(bundle(outer, &p, None).unwrap_err().contains("missing"));
    }
    fn image_tile(project: &mut Project) -> Uuid {
        let tile = rusterix::Tile::from_textures(vec![rusterix::Texture::new(
            vec![
                128, 0, 0, 128, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
            2,
            2,
        )]);
        let id = tile.id;
        project.tiles.insert(id, tile);
        id
    }
    #[test]
    fn ordinary_tiles_remain_images_and_mix_in_linear_color_with_alpha() {
        let mut p = Project::default();
        let image = image_tile(&mut p);
        let saved = p.tiles[&image].clone();
        let color = sample(&p, image, [0.25, 0.25], Channel::BaseColor);
        let linear = ((128. / 255. + 0.055) / 1.055_f32).powf(2.4);
        assert!((color[0] - linear).abs() < 0.0001);
        assert!((sample(&p, image, [0.25, 0.25], Channel::Opacity)[0] - 128. / 255.).abs() < 0.005);
        assert_eq!(
            sample(&p, image, [0.75, 0.25], Channel::BaseColor),
            vec![0., 1., 0.]
        );
        let blue = source(&mut p, [0., 0., 1.], 0.2);
        let mut layer = CompositionLayer::split(blue);
        layer.shape = CompositionShape::Blend;
        layer.amount = 0.25;
        let id = composition_tile(
            &mut p,
            MaterialComposition {
                tiling: Default::default(),
                base: image,
                layers: vec![layer],
            },
        );
        let mixed = sample(&p, id, [0.25, 0.25], Channel::BaseColor);
        assert!((mixed[0] - linear * 0.75).abs() < 0.0001 && (mixed[2] - 0.25).abs() < 0.0001);
        assert_eq!(p.tiles[&image], saved);
        assert!(!p.procedural_materials.contains_key(&image));
        let key = input_key(&p);
        p.tiles.get_mut(&image).unwrap().textures[0].data[0] = 255;
        assert_ne!(input_key(&p), key);
        assert!(sample(&p, id, [0.25, 0.25], Channel::BaseColor)[0] > 0.74);
    }
    #[test]
    fn numeric_composition_edits_keep_native_controls_and_do_not_rebuild_on_noop_clicks() {
        let (mut ui, mut ctx) = super::super::tests::workspace_ui(1200, 950);
        let mut p = Project::default();
        let a = source(&mut p, [1., 0., 0.], 0.2);
        let b = image_tile(&mut p);
        let id = composition_tile(
            &mut p,
            MaterialComposition {
                tiling: Default::default(),
                base: a,
                layers: vec![CompositionLayer::split(b)],
            },
        );
        let mut host = ProceduralMaterialEditor::default();
        let mut server = ServerContext::default();
        host.open(id, &mut ui, &mut ctx, &p);
        host.sync_compositions(&mut p, &mut ui, &mut ctx);
        host.composition_view(&mut ui, &mut ctx, &p);
        let control = ui.get_widget(POSITION).unwrap().id().uuid;
        let (sender, receiver) = std::sync::mpsc::channel();
        ctx.ui.state_events_sender = Some(sender);
        ctx.ui.relayout = false;
        ctx.ui.redraw_all = false;
        host.handle_event(
            &TheEvent::ValueChanged(TheId::named(POSITION), TheValue::Float(0.4)),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        host.sync_compositions(&mut p, &mut ui, &mut ctx);
        host.composition_view(&mut ui, &mut ctx, &p);
        assert_eq!(ui.get_widget(POSITION).unwrap().id().uuid, control);
        assert!(!ctx.ui.relayout && !ctx.ui.redraw_all);
        assert!(
            !receiver
                .try_iter()
                .any(|event| matches!(event,TheEvent::Custom(id,_) if id.name == "Update Tiles"))
        );
        let control = ui.get_widget(POSITION).unwrap().id().uuid;
        host.handle_event(
            &TheEvent::ValueChanged(TheId::named(POSITION), TheValue::Float(0.4)),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        host.composition_view(&mut ui, &mut ctx, &p);
        assert_eq!(ui.get_widget(POSITION).unwrap().id().uuid, control);
        assert!(host.composition_choices.contains(&b));
    }
    #[test]
    fn wobbly_roads_use_coherent_global_uv_and_soft_edges_at_shared_boundaries() {
        let mut p = Project::default();
        let a = source(&mut p, [0., 0., 0.], 0.2);
        let b = source(&mut p, [1., 1., 1.], 0.8);
        let mut layer = CompositionLayer::split(b);
        layer.shape = CompositionShape::Road;
        layer.position = 0.4;
        layer.wobble = 0.12;
        layer.wobble_scale = 1.7;
        layer.feather = 0.2;
        let id = composition_tile(
            &mut p,
            MaterialComposition {
                tiling: Default::default(),
                base: a,
                layers: vec![layer],
            },
        );
        let asset = EditableMaterialAsset::reopen(&bundle(id, &p, None).unwrap()).unwrap();
        let program = denrim_materials::compile(&format::write(
            &asset.resolved(&InstanceRecord::new("road")).unwrap(),
        ))
        .unwrap();
        let eval = Evaluator::new(&program);
        let at = |uv| {
            eval.sample(&asset.selected_material, Channel::BaseColor, uv)
                .unwrap()[0]
        };
        for boundary in [-2., 0., 1., 3.] {
            for y in [0.27, 0.3, 0.7, 0.73] {
                assert!((at([boundary - 0.00001, y]) - at([boundary + 0.00001, y])).abs() < 0.002);
            }
        }
        let values: Vec<_> = (0..20).map(|i| at([i as f32 / 5., 0.3])).collect();
        assert!(values.iter().any(|v| *v > 0.05 && *v < 0.95));
        assert!(
            values
                .windows(2)
                .any(|pair| (pair[0] - pair[1]).abs() > 0.01)
        );
        let mut layer = p.material_compositions[&id].layers[0].clone();
        layer.wobble = 0.;
        p.material_compositions.get_mut(&id).unwrap().layers[0] = layer;
        assert!(
            (sample(&p, id, [0.1, 0.3], Channel::BaseColor)[0]
                - sample(&p, id, [1.1, 0.3], Channel::BaseColor)[0])
                .abs()
                < 0.0001
        );
    }
    pub(super) fn add_floor(map: &mut Map, id: u32, source: Uuid, min: [f32; 2], max: [f32; 2]) {
        let vertices = [
            [min[0], min[1]],
            [min[0], max[1]],
            [max[0], max[1]],
            [max[0], min[1]],
        ]
        .map(|p| map.add_vertex_at(p[0], p[1]));
        let lines = (0..4)
            .map(|i| map.create_linedef_manual(vertices[i], vertices[(i + 1) % 4]))
            .collect();
        let mut sector = rusterix::Sector::new(id, lines);
        sector.properties.set(
            "source",
            Value::Source(rusterix::PixelSource::TileId(source)),
        );
        map.sectors.push(sector);
    }
    #[test]
    fn saved_composition_is_reusable_and_editing_placements_copies_the_existing_layers() {
        let (mut ui, mut ctx) = super::super::tests::workspace_ui(1200, 950);
        let mut p = Project::default();
        let a = source(&mut p, [0.2, 0.1, 0.], 0.7);
        let b = source(&mut p, [0., 0.5, 0.1], 0.3);
        let mut layer = CompositionLayer::split(b);
        layer.shape = CompositionShape::Road;
        layer.wobble = 0.08;
        let original = composition_tile(
            &mut p,
            MaterialComposition {
                tiling: Default::default(),
                base: a,
                layers: vec![layer],
            },
        );
        let mut host = ProceduralMaterialEditor::default();
        host.open(original, &mut ui, &mut ctx, &p);
        let mut server = ServerContext::default();
        server.curr_tile_board_cell = Some((None, Vec2::new(6, 6)));
        let before = p.clone();
        let saved = host
            .save_composition_copy(&mut p, &mut ctx, &server)
            .unwrap();
        let after = p.clone();
        assert_ne!(saved, original);
        assert!(!p.map_material_bindings.contains_key(&saved));
        assert_eq!(p.tile_board_tiles[&saved], Vec2::new(6, 6));
        assert_eq!(
            p.material_compositions[&saved],
            p.material_compositions[&original]
        );
        let atom = ProjectUndoAtom::TilePickerEdit(Box::new(before), Box::new(after));
        atom.undo(&mut p, &mut ui, &mut ctx, &mut server);
        assert!(!p.tiles.contains_key(&saved));
        atom.redo(&mut p, &mut ui, &mut ctx, &mut server);
        assert!(p.tiles.contains_key(&saved));
        let mut region = Region::new();
        region.map = Map::default();
        add_floor(&mut region.map, 1, saved, [-2., 0.], [0., 1.]);
        add_floor(&mut region.map, 2, saved, [0., 0.], [2., 1.]);
        server.pc = ProjectContext::Region(region.id);
        server.curr_region = region.id;
        let map_id = region.map.id;
        p.regions.push(region);
        let text = bundle(saved, &p, None).unwrap();
        let asset = EditableMaterialAsset::reopen(&text).unwrap();
        let pages = surface_pages::decode(&asset).unwrap().unwrap();
        assert_eq!(pages.origin, [-2, 0]);
        assert_eq!(pages.size, [4, 1]);
        let original_model = p.material_compositions[&saved].clone();
        let mut copies = Vec::new();
        for sector in [1, 2] {
            let binding = shared::material_composition::MapMaterialBinding {
                map: map_id,
                surface: shared::material_composition::CompositionSurface::RectTile { sector },
            };
            host.open_map_composition(binding, &mut ui, &mut ctx, &mut p, &mut server);
            let id = host.tile.unwrap();
            copies.push(id);
            assert_eq!(
                p.material_compositions[&id], original_model,
                "Copy the stack, rather than nesting it under a fresh split"
            );
        }
        assert_ne!(copies[0], copies[1]);
        p.material_compositions.get_mut(&copies[0]).unwrap().layers[0].position = 0.8;
        assert_eq!(p.material_compositions[&saved], original_model);
        assert_eq!(p.material_compositions[&copies[1]], original_model);
        let restored: Project = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(restored.material_compositions[&saved], original_model);
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[ignore = "requires a GPU device"]
    fn gpu_whole_surface_pages_sample_distinct_world_positions() {
        let (device, queue) = pollster::block_on(async {
            let adapter = wgpu::Instance::default()
                .request_adapter(&Default::default())
                .await
                .unwrap();
            adapter.request_device(&Default::default()).await.unwrap()
        });
        let worker = gpu::Worker::new(device, queue);
        let mut p = Project::default();
        let input = source(&mut p, [0., 0., 0.], 0.7);
        let code = denrim_materials::tilelang::source(denrim_materials::metadata::Kernel::Color);
        let header = code.split_once("\nimage =").unwrap().0;
        let code = format!(
            "{header}\nimage = vec4(fract(uv.x * 0.137 + 0.37), fract(uv.y * 0.173 + 0.27), 0.1, 1)"
        );
        let mut text="project 1\nmaterial \"surface\" called \"World field\"\n  mode pbr\n  row \"image\"\n    use \"Color\" id \"world\"\n      source\n".to_owned();
        for line in code.lines() {
            text.push_str("        ");
            text.push_str(line);
            text.push('\n');
        }
        text.push_str("      end source\n  output base color from row \"image\" xyz\n");
        let asset = EditableMaterialAsset::parse(&input.to_string(), &text, "surface").unwrap();
        p.procedural_materials.insert(input, asset.save().unwrap());
        let id = composition_tile(
            &mut p,
            MaterialComposition {
                tiling: Default::default(),
                base: input,
                layers: vec![],
            },
        );
        let mut region = Region::new();
        region.map = Map::default();
        add_floor(&mut region.map, 1, id, [-2., 0.], [2., 2.]);
        p.regions.push(region);
        let text = bundle(id, &p, None).unwrap();
        let asset = EditableMaterialAsset::reopen(&text).unwrap();
        let program = denrim_materials::compile(&format::write(
            &asset.resolved(&InstanceRecord::new("test")).unwrap(),
        ))
        .unwrap();
        let eval = Evaluator::new(&program);
        worker.send(gpu::Job {
            pixels_per_tile: 0,
            tile: id,
            revision: 1,
            bundle: text,
            continuous: false,
            selection: None,
            preview_aspect: 1.,
        });
        let frame = worker
            .results
            .recv_timeout(std::time::Duration::from_secs(60))
            .unwrap()
            .frame
            .unwrap();
        let pages = frame.surface_pages.unwrap();
        assert_eq!(pages.layout.origin, [-2, 0]);
        assert_eq!(pages.layout.size, [4, 2]);
        assert_eq!(pages.textures.len(), 8);
        assert_ne!(pages.textures[0].data, pages.textures[1].data);
        for y in 0..2 {
            for x in 0..4 {
                let uv = [-2. + x as f32 + 0.5 / 64., y as f32 + 0.5 / 64.];
                let values = eval
                    .sample(&asset.selected_material, Channel::BaseColor, uv)
                    .unwrap();
                for (c, value) in values.iter().enumerate() {
                    let v = if *value <= 0.0031308 {
                        *value * 12.92
                    } else {
                        1.055 * value.powf(1. / 2.4) - 0.055
                    };
                    assert!((pages.textures[y * 4 + x].data[c] as f32 - v * 255.).abs() <= 2.);
                }
            }
        }
    }
    #[test]
    fn shape_thumbnail_masks_match_material_output_and_repeat_per_tile() {
        let mut project = Project::default();
        let background = source(&mut project, [1., 0., 0.], 0.2);
        let added = source(&mut project, [0., 1., 0.], 0.8);
        for preset in &PRESETS {
            let mut layer = CompositionLayer::split(added);
            preset.apply(&mut layer);
            let composition = MaterialComposition {
                tiling: Default::default(),
                base: background,
                layers: vec![layer],
            };
            let id = Uuid::new_v4();
            let asset =
                EditableMaterialAsset::reopen(&bundle(id, &project, Some(&composition)).unwrap())
                    .unwrap();
            let document = asset.resolved(&InstanceRecord::new("preset-test")).unwrap();
            let program = denrim_materials::compile(&format::write(&document)).unwrap();
            let eval = Evaluator::new(&program);
            for y in 0..5 {
                for x in 0..5 {
                    let uv = [(x as f32 + 0.31) / 5., (y as f32 + 0.27) / 5.];
                    let expected = preset.coverage(uv);
                    for uv in [uv, [uv[0] + 2., uv[1] - 1.]] {
                        let actual = eval
                            .sample(&asset.selected_material, Channel::BaseColor, uv)
                            .unwrap();
                        assert!(
                            (actual[0] - (1. - expected)).abs() < 0.0001
                                && (actual[1] - expected).abs() < 0.0001,
                            "{} at {uv:?}: {actual:?}",
                            preset.label
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn composition_tiling_is_authored_and_survives_regeneration_copy_and_undo() {
        let (mut ui, mut ctx) = super::super::tests::workspace_ui(1024, 768);
        let mut project = Project::default();
        let a = source(&mut project, [1., 0., 0.], 0.2);
        let b = source(&mut project, [0., 1., 0.], 0.8);
        let id = composition_tile(
            &mut project,
            MaterialComposition {
                tiling: Default::default(),
                base: a,
                layers: vec![CompositionLayer::split(b)],
            },
        );
        let before = project.material_compositions[&id].clone();
        let mut host = ProceduralMaterialEditor::default();
        let mut server = ServerContext::default();
        host.open(id, &mut ui, &mut ctx, &project);
        assert!(host.handle_event(
            &TheEvent::IndexChanged(TheId::named("Material Tiling"), 2),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server
        ));
        let after = project.material_compositions[&id].clone();
        assert_eq!(after.tiling.mode, "seamless");
        project.material_compositions.get_mut(&id).unwrap().layers[0].position = 0.3;
        host.sync_compositions(&mut project, &mut ui, &mut ctx);
        let asset = EditableMaterialAsset::reopen(&project.procedural_materials[&id]).unwrap();
        assert_eq!(material(&asset).unwrap().tiling.mode.name(), "seamless");
        let copy = host
            .save_composition_copy(&mut project, &mut ctx, &server)
            .unwrap();
        assert_eq!(project.material_compositions[&copy].tiling.mode, "seamless");
        let atom = ProjectUndoAtom::MaterialCompositionEdit(id, before.clone(), after.clone());
        atom.undo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(project.material_compositions[&id].tiling.mode, "raw");
        atom.redo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(project.material_compositions[&id].tiling.mode, "seamless");
        let reopened: Project =
            serde_json::from_str(&serde_json::to_string(&project).unwrap()).unwrap();
        assert_eq!(reopened.material_compositions[&id].tiling, after.tiling);
    }
    #[test]
    fn shape_grid_is_default_and_selection_saves_undoes_and_retains_layer_identity() {
        let (mut ui, mut ctx) = super::super::tests::workspace_ui(1200, 950);
        ui.init(&mut ctx);
        ui.set_theme(Box::new(TheBrownTheme::new()), &mut ctx);
        let mut project = Project::default();
        let a = source(&mut project, [1., 0., 0.], 0.2);
        let b = source(&mut project, [0., 1., 0.], 0.8);
        project.tiles.get_mut(&a).unwrap().alias = "Road".into();
        project.tiles.get_mut(&b).unwrap().alias = "Grass".into();
        let id = composition_tile(
            &mut project,
            MaterialComposition {
                tiling: Default::default(),
                base: a,
                layers: vec![CompositionLayer::split(b)],
            },
        );
        let mut host = ProceduralMaterialEditor::default();
        let mut server = ServerContext::default();
        host.open(id, &mut ui, &mut ctx, &project);
        host.composition_view(&mut ui, &mut ctx, &project);
        assert!(ui.get_widget(shape_picker::PICKER).is_some());
        assert!(
            ui.get_widget(SHAPE).is_none(),
            "Technical mode chooser is collapsed"
        );
        assert!(ui.get_widget(ANGLE).is_none());
        let before = project.material_compositions[&id].clone();
        assert!(host.handle_event(
            &TheEvent::IndexChanged(TheId::named(shape_picker::PICKER), 7),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server
        ));
        let after = project.material_compositions[&id].clone();
        assert_eq!(after.layers[0].shape, CompositionShape::Corner);
        assert_eq!(after.layers[0].angle, 90.);
        assert_eq!(after.layers[0].id, before.layers[0].id);
        let reopened: MaterialComposition =
            serde_json::from_str(&serde_json::to_string(&after).unwrap()).unwrap();
        assert_eq!(reopened, after);
        let atom = ProjectUndoAtom::MaterialCompositionEdit(id, before.clone(), after.clone());
        atom.undo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(project.material_compositions[&id], before);
        atom.redo(&mut project, &mut ui, &mut ctx, &mut server);
        assert_eq!(project.material_compositions[&id], after);
        host.composition_view(&mut ui, &mut ctx, &project);
        let mut pixels = vec![0; 1200 * 950 * 4];
        ui.draw(&mut pixels, &mut ctx);
        let output = std::fs::File::create("/tmp/eldiron-shape-picker-workspace.png").unwrap();
        let mut encoder = png::Encoder::new(output, 1200, 950);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
        host.handle_event(
            &TheEvent::StateChanged(
                TheId::named("Composition Advanced"),
                TheWidgetState::Clicked,
            ),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        host.composition_view(&mut ui, &mut ctx, &project);
        assert!(ui.get_widget(SHAPE).is_some());
    }
    #[test]
    fn layer_controls_and_undo_preserve_shared_sources() {
        let (mut ui, mut ctx) = super::super::tests::workspace_ui(1200, 850);
        let mut p = Project::default();
        let a = source(&mut p, [1., 0., 0.], 0.2);
        let b = source(&mut p, [0., 1., 0.], 0.8);
        let sources = p.procedural_materials.clone();
        let id = composition_tile(
            &mut p,
            MaterialComposition {
                tiling: Default::default(),
                base: a,
                layers: vec![CompositionLayer::split(b)],
            },
        );
        let mut server = ServerContext::default();
        let mut host = ProceduralMaterialEditor::default();
        host.open(id, &mut ui, &mut ctx, &p);
        host.composition_view(&mut ui, &mut ctx, &p);
        assert_eq!(ui.get_stack_layout(STACK).unwrap().index(), 1);
        assert!(ui.get_widget(POSITION).unwrap().supports_text_input());
        let before = p.material_compositions[&id].clone();
        for event in [
            TheEvent::ValueChanged(TheId::named(POSITION), TheValue::Text("0.25".into())),
            TheEvent::IndexChanged(TheId::named("Composition Actions"), 0),
        ] {
            assert!(host.handle_event(&event, &mut ui, &mut ctx, &mut p, &mut server));
        }
        assert_eq!(p.material_compositions[&id].layers.len(), 2);
        assert_eq!(p.material_compositions[&id].layers[0].position, 0.25);
        host.composition_view(&mut ui, &mut ctx, &p);
        for event in [
            TheEvent::IndexChanged(TheId::named(SHAPE), 1),
            TheEvent::ValueChanged(TheId::named(AMOUNT), TheValue::Text("0.3".into())),
        ] {
            host.handle_event(&event, &mut ui, &mut ctx, &mut p, &mut server);
        }
        let after = p.material_compositions[&id].clone();
        assert_eq!(after.layers[1].shape, CompositionShape::Blend);
        host.handle_event(
            &TheEvent::ValueChanged(TheId::named(AMOUNT), TheValue::Text("3".into())),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        assert_eq!(
            p.material_compositions[&id], after,
            "Out-of-range manual entry is transactional"
        );
        p.name = "Unrelated rename".into();
        let atom = ProjectUndoAtom::MaterialCompositionEdit(id, before.clone(), after.clone());
        atom.undo(&mut p, &mut ui, &mut ctx, &mut server);
        host.sync_compositions(&mut p, &mut ui, &mut ctx);
        assert_eq!(p.material_compositions[&id], before);
        assert_eq!(p.name, "Unrelated rename");
        atom.redo(&mut p, &mut ui, &mut ctx, &mut server);
        host.composition_key = None;
        host.sync_compositions(&mut p, &mut ui, &mut ctx);
        assert_eq!(p.material_compositions[&id], after);
        assert_eq!(p.procedural_materials[&a], sources[&a]);
        assert_eq!(p.procedural_materials[&b], sources[&b]);
        assert_eq!(p.procedural_materials[&id], bundle(id, &p, None).unwrap());
        let mut legacy = serde_json::to_value(&p).unwrap();
        legacy
            .as_object_mut()
            .unwrap()
            .remove("material_compositions");
        assert!(
            serde_json::from_value::<Project>(legacy)
                .unwrap()
                .material_compositions
                .is_empty()
        );
    }

    #[test]
    fn mixer_inspector_edits_mask_and_four_input_uv_controls() {
        let (mut ui, mut ctx) = super::super::tests::workspace_ui(1200, 850);
        let mut p = Project::default();
        let a = source(&mut p, [1., 0., 0.], 0.2);
        let b = source(&mut p, [0., 1., 0.], 0.8);
        let mask = mask_source(&mut p, [0.25, 0.75, 1., 0.5]);
        let id = composition_tile(
            &mut p,
            MaterialComposition {
                tiling: Default::default(),
                base: a,
                layers: vec![CompositionLayer::split(b)],
            },
        );
        let mut server = ServerContext::default();
        let mut host = ProceduralMaterialEditor::default();
        host.open(id, &mut ui, &mut ctx, &p);
        host.composition_advanced = true;
        host.composition_view(&mut ui, &mut ctx, &p);
        let mask_index = host
            .composition_choices
            .iter()
            .position(|source| *source == mask)
            .unwrap()
            + 1;
        for event in [
            TheEvent::IndexChanged(TheId::named(SHAPE), 2),
            TheEvent::IndexChanged(TheId::named(MASK_MATERIAL), mask_index),
            TheEvent::IndexChanged(TheId::named(MASK_CHANNEL), 1),
            TheEvent::IndexChanged(TheId::named(MASK_REMAP), 1),
            TheEvent::ValueChanged(TheId::named(MASK_MIN), TheValue::Text("1".into())),
            TheEvent::ValueChanged(TheId::named(MASK_MAX), TheValue::Text("0".into())),
            TheEvent::IndexChanged(TheId::named(BLEND), 0),
        ] {
            assert!(host.handle_event(&event, &mut ui, &mut ctx, &mut p, &mut server));
        }
        host.composition_view(&mut ui, &mut ctx, &p);
        assert!(ui.get_widget(MASK_MIN).unwrap().supports_text_input());
        let layer = &p.material_compositions[&id].layers[0];
        assert_eq!(layer.mask.as_ref().unwrap().components, "yyy");
        assert_eq!(layer.mask.as_ref().unwrap().mapping, Some([1., 0.]));
        assert!(!layer.blend);
        for event in [
            TheEvent::IndexChanged(TheId::named(SHAPE), 5),
            TheEvent::IndexChanged(TheId::named(EXTRAS[0]), mask_index),
            TheEvent::IndexChanged(TheId::named(EXTRAS[1]), mask_index),
            TheEvent::IndexChanged(TheId::named(DIRECTION), 1),
            TheEvent::ValueChanged(TheId::named(LEVELS[0]), TheValue::Text("0.3".into())),
        ] {
            assert!(host.handle_event(&event, &mut ui, &mut ctx, &mut p, &mut server));
        }
        host.composition_view(&mut ui, &mut ctx, &p);
        assert!(ui.get_widget(LEVELS[0]).unwrap().supports_text_input());
        let after = p.material_compositions[&id].clone();
        assert_eq!(after.layers[0].extra_materials, [Some(mask), Some(mask)]);
        assert_eq!(after.layers[0].uv_direction, 1);
        assert_eq!(after.layers[0].uv_levels, [0.3, 0.5, 0.75]);
        host.handle_event(
            &TheEvent::ValueChanged(TheId::named(LEVELS[0]), TheValue::Text("0.9".into())),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        assert_eq!(
            p.material_compositions[&id], after,
            "Unordered manual levels are transactional"
        );
        host.handle_event(
            &TheEvent::IndexChanged(TheId::named(SHAPE), 1),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        p.tiles.shift_remove(&mask);
        p.procedural_materials.shift_remove(&mask);
        for shape in [5, 2] {
            host.handle_event(
                &TheEvent::IndexChanged(TheId::named(SHAPE), shape),
                &mut ui,
                &mut ctx,
                &mut p,
                &mut server,
            );
        }
        let layer = &p.material_compositions[&id].layers[0];
        assert_eq!(layer.shape, CompositionShape::Mask);
        assert_eq!(layer.extra_materials, [None, None]);
        assert_eq!(
            layer.mask, None,
            "Missing inactive sources do not trap mode selection"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[ignore = "requires a GPU device; writes /tmp/eldiron-composition-workspace.png"]
    fn gpu_split_chain_renders_pbr_maps_bakes_and_minimap() {
        let (device, queue) = pollster::block_on(async {
            let adapter = wgpu::Instance::default()
                .request_adapter(&Default::default())
                .await
                .unwrap();
            adapter.request_device(&Default::default()).await.unwrap()
        });
        let worker = gpu::Worker::new(device, queue);
        let mut p = Project::default();
        let a = source(&mut p, [1., 0., 0.], 0.2);
        let b = source(&mut p, [0., 1., 0.], 0.8);
        let c = source(&mut p, [0., 0., 1.], 0.4);
        p.tiles.get_mut(&a).unwrap().alias = "Red".into();
        p.tiles.get_mut(&b).unwrap().alias = "Green".into();
        p.tiles.get_mut(&c).unwrap().alias = "Blue".into();
        let mut blend = CompositionLayer::split(c);
        blend.shape = CompositionShape::Blend;
        blend.amount = 0.25;
        let id = composition_tile(
            &mut p,
            MaterialComposition {
                tiling: Default::default(),
                base: a,
                layers: vec![CompositionLayer::split(b), blend],
            },
        );
        let (mut ui, mut ctx) = super::super::tests::workspace_ui(1200, 850);
        ui.init(&mut ctx);
        let mut host = ProceduralMaterialEditor::default();
        host.open(id, &mut ui, &mut ctx, &p);
        host.composition_view(&mut ui, &mut ctx, &p);
        let mut server = ServerContext::default();
        for (revision, position) in [(1, 0.5), (2, 0.2)] {
            p.material_compositions.get_mut(&id).unwrap().layers[0].position = position;
            host.composition_view(&mut ui, &mut ctx, &p);
            let bundle = bundle(id, &p, None).unwrap();
            worker.send(gpu::Job {
                pixels_per_tile: 0,
                tile: id,
                revision,
                bundle,
                continuous: true,
                selection: Some(PreviewSelection::Material(format!(
                    "layer-{}",
                    p.material_compositions[&id].layers[0].id
                ))),
                preview_aspect: 2.5,
            });
            let frame = worker
                .results
                .recv_timeout(std::time::Duration::from_secs(60))
                .unwrap()
                .frame
                .unwrap();
            assert_eq!(frame.tile.width, 64);
            let pixel = |x: usize| {
                &frame.images[0][(frame.size / 4 * frame.size + x) * 4
                    ..(frame.size / 4 * frame.size + x) * 4 + 4]
            };
            let left = pixel((0.1 / 5. * frame.size as f32) as usize);
            let right = pixel((0.8 / 5. * frame.size as f32) as usize);
            assert!(left[0] > 200 && left[1] < 2 && left[2] > 130);
            assert!(right[0] < 2 && right[1] > 200 && right[2] > 130);
            let selection = &frame.selection.as_ref().unwrap().1;
            let at = (frame.size / 4 * frame.size + (0.8 / 5. * frame.size as f32) as usize) * 4;
            assert!(
                selection[at + 1] > 250 && selection[at + 2] < 2,
                "Selected layer excludes the subsequent blue blend"
            );
            assert!(
                (frame.images[2][at] as i32 - 179).abs() <= 1,
                "GPU mixes roughness with the same coverage"
            );
            if revision == 2 {
                let moved = pixel((0.35 / 5. * frame.size as f32) as usize);
                assert!(moved[1] > 200 && moved[0] < 2);
            }
            assert!(frame.tile.material_bytes().len() > 0);
            host.frame = Some(frame);
            host.preview_dirty = true;
            let mut pixels = vec![0; 1200 * 850 * 4];
            ui.draw(&mut pixels, &mut ctx);
            assert!(host.draw_preview(&mut ui, 0));
            ui.draw(&mut pixels, &mut ctx);
            let preview_dim = *ui.get_render_view(PREVIEW).unwrap().dim();
            let at = ((preview_dim.y + preview_dim.height / 2) as usize * 1200
                + (preview_dim.x + preview_dim.width / 2) as usize)
                * 4;
            assert!(
                pixels[at..at + 3].iter().any(|value| *value > 100),
                "The GPU image must reach the visible minimap"
            );
            assert_eq!(ui.get_stack_layout("Game Output Stack").unwrap().index(), 1);
            assert_eq!(ui.get_stack_layout(PREVIEW_STACK).unwrap().index(), 1);
            if revision == 2 {
                let output =
                    std::fs::File::create("/tmp/eldiron-composition-workspace.png").unwrap();
                let mut encoder = png::Encoder::new(output, 1200, 850);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder
                    .write_header()
                    .unwrap()
                    .write_image_data(&pixels)
                    .unwrap();
            }
        }
        // The controls still edit the selected layer after a real render.
        assert!(host.handle_event(
            &TheEvent::ValueChanged(TheId::named(POSITION), TheValue::Float(0.4)),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server
        ));
    }
}
