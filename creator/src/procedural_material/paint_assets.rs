//! Native Paint artwork exposed as linked, editable MZ image-source recipes.
use super::*;
use rusterix::material_library::MaterialDefinition;
use shared::paint_source::Spec;
fn normalize_collections(project: &mut Project) {
    project.tile_collections.retain(|_, collection| {
        collection.name != "Paint materials" && collection.name != "Paint Materials"
    });
    for collection in project.tile_collections.values_mut() {
        match collection.name.as_str() {
            "Paint brushes" => collection.name = "Paint Brushes".into(),
            "Organic stamps" => collection.name = "Organic Stamps".into(),
            _ => {}
        }
    }
}

fn native_template(id: Uuid, name: &str, spec: &Spec) -> Result<EditableMaterialAsset, String> {
    let [rough, metal, opacity, emission] =
        MaterialDefinition::from_id(spec.material_id).rmoe_values();
    let recipe = format!(
        "project 1\nmaterial \"surface\" called \"{name}\"\n  mode pbr\n  tiling repeat\n  default roughness to {rough}\n  default metallic to {metal}\n  default emission to {emission} {emission} {emission}\n  row \"artwork\" called \"Paint Source\"\n    use \"Image / Tile Source\" id \"paint-source\"\n  output base color from row \"artwork\" xyz\n  output opacity from row \"artwork\" w\n"
    );
    let header = denrim_materials::tilelang::source(denrim_materials::metadata::Kernel::Color)
        .split("image =")
        .next()
        .unwrap()
        .lines()
        .map(|line| format!("        {line}\n"))
        .collect::<String>();
    let recipe = recipe.replace(
        "output opacity from row \"artwork\" w",
        "output opacity from row \"alpha\" x",
    ) + &format!(
        "  row \"alpha\" called \"Opacity\"\n    use \"Color\" id \"paint-alpha\"\n      set \"color\" from row \"artwork\" wwww\n      source\n{header}        image = color * {opacity}\n      end source\n"
    );
    EditableMaterialAsset::parse(&id.to_string(), &recipe, "surface").map_err(|e| e.to_string())
}

fn native_layout(
    asset: &EditableMaterialAsset,
    spec: &Spec,
) -> rusterix::map::tile::PaintAssetLayout {
    let mut layout = spec.layout();
    let equivalent = (|| {
        let id = Uuid::parse_str(&asset.root.id).ok()?;
        let image = asset
            .root
            .project
            .materials
            .iter()
            .flat_map(|m| &m.rows)
            .flat_map(|r| &r.tiles)
            .find(|t| t.id == "paint-source")?
            .image
            .clone()?;
        let expected = image_source::set(
            &native_template(id, "", spec).ok()?,
            "paint-source",
            Some(image),
        )
        .ok()?;
        let normalize = |mut m: denrim_materials::format::Material| {
            m.label.clear();
            m.position = None;
            for row in &mut m.rows {
                row.label.clear();
                for tile in &mut row.tiles {
                    tile.image = None;
                    if let Some(code) = tile.code.take() {
                        tile.code = Some(
                            code.lines()
                                .map(str::trim)
                                .collect::<Vec<_>>()
                                .join("\n")
                                .trim()
                                .to_owned(),
                        );
                    }
                }
            }
            denrim_materials::format::write(&denrim_materials::format::Project {
                result: None,
                materials: vec![m],
            })
        };
        let actual = asset
            .root
            .project
            .materials
            .iter()
            .find(|m| m.id == asset.selected_material)?
            .clone();
        Some(normalize(actual) == normalize(expected.root.project.materials[0].clone()))
    })()
    .unwrap_or(false);
    if !equivalent {
        layout.generator = None;
    }
    layout
}

pub(crate) fn selected_spec(project: &Project, id: Uuid) -> Option<Spec> {
    Spec::from_asset(&EditableMaterialAsset::reopen(project.procedural_materials.get(&id)?).ok()?)
}

/// Publish simple Paint controls back into the same MZ recipe and refresh its native cache.
pub(crate) fn update_spec(
    project: &mut Project,
    id: Uuid,
    mut spec: Spec,
) -> Result<(String, String), String> {
    spec.resolve_palette(&project.palette);
    spec.validate()?;
    let before = project
        .procedural_materials
        .get(&id)
        .ok_or("Paint recipe is missing")?
        .clone();
    let mut asset = EditableMaterialAsset::reopen(&before).map_err(|e| e.to_string())?;
    let old = Spec::from_asset(&asset).ok_or("Select a Paint recipe")?;
    let operation = asset
        .root
        .project
        .materials
        .iter()
        .flat_map(|m| &m.rows)
        .flat_map(|r| &r.tiles)
        .find(|t| t.image.as_ref().and_then(Spec::from_image).is_some())
        .map(|t| t.id.clone())
        .ok_or("Paint source is missing")?;
    let (image, _, mut textures) = spec.render()?;
    asset = image_source::set(&asset, &operation, Some(image))?;
    if old.material_id != spec.material_id {
        let expected = native_template(id, "", &spec)?;
        let mat = asset
            .root
            .project
            .materials
            .iter_mut()
            .find(|m| m.id == asset.selected_material)
            .ok_or("Material is missing")?;
        mat.defaults = expected.root.project.materials[0].defaults.clone();
        if let Some(alpha) = mat
            .rows
            .iter_mut()
            .flat_map(|r| &mut r.tiles)
            .find(|t| t.id == "paint-alpha")
        {
            alpha.code = expected.root.project.materials[0].rows[1].tiles[0]
                .code
                .clone();
        }
    }
    asset.validate().map_err(|e| e.to_string())?;
    let layout = native_layout(&asset, &spec);
    let bundle = asset.save().map_err(|e| e.to_string())?;
    if let Some(tile) = project.tiles.get_mut(&id) {
        if layout.generator.is_some() {
            let [r, m, o, e] = MaterialDefinition::from_id(spec.material_id).rmoe_values();
            for texture in &mut textures {
                for pixel in texture.data.chunks_exact_mut(4) {
                    pixel[3] = (pixel[3] as f32 * o).round() as u8;
                }
                texture.set_materials_all(r, m, o, e);
            }
            tile.textures = textures;
            tile.baked_material_data = tile.textures.iter().map(|t| t.material_bytes()).collect();
            project.procedural_material_cache_keys.insert(
                id,
                pixel_style::bake_key(&bundle, pixel_style::density(&project.config)),
            );
        } else {
            project.procedural_material_cache_keys.shift_remove(&id);
        }
        tile.paint = Some(layout);
    }
    project.procedural_materials.insert(id, bundle.clone());
    Ok((before, bundle))
}

pub(crate) fn install(project: &mut Project) -> Result<(), String> {
    normalize_collections(project);
    let mut color = Spec::new("material", false);
    color.colors = vec![[132, 132, 128, 255]];
    let mut entries = vec![("Paint / Color".into(), "Paint Brushes", color)];
    for brush in shared::paint_source::BRUSHES {
        if matches!(
            brush,
            "material" | "vines" | "roots" | "bushes" | "tree" | "candles"
        ) {
            continue;
        }
        let mut spec = Spec::new(brush, false);
        spec.material_id = match brush {
            "moss" | "grass" | "leaves" | "flowers" | "vines" | "bushes" | "tree" => 44,
            "puddle" => 27,
            "mud" => 11,
            "dirt" => 9,
            "rubble" | "brick" | "crack" => 4,
            "candles" => 32,
            "roots" => 12,
            _ => 8,
        };
        entries.push((format!("Paint / {brush}"), "Paint Brushes", spec));
    }
    for brush in shared::paint_source::STAMPS {
        let mut spec = Spec::new(brush, true);
        spec.material_id = match brush {
            "grass" | "leaves" | "flowers" | "vines" | "bushes" | "tree" => 44,
            "rubble" => 4,
            "mud" => 11,
            "candles" => 32,
            "roots" => 12,
            _ => 8,
        };
        entries.push((format!("Stamp / {brush}"), "Organic Stamps", spec));
    }
    for variant in ["daisies", "poppies", "bluebells"] {
        let mut spec = Spec::new("flowers", true);
        spec.variant = variant.into();
        spec.material_id = 44;
        entries.push((format!("Stamp / {variant}"), "Organic Stamps", spec));
    }
    let mut artwork: std::collections::HashMap<
        String,
        (
            denrim_materials::image_source::ImageSource,
            rusterix::map::tile::PaintAssetLayout,
            Vec<rusterix::Texture>,
        ),
    > = Default::default();
    for (name, collection, mut spec) in entries {
        let hash = name.bytes().fold(0xcbf29ce484222325u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
        });
        let id = Uuid::from_u128((0x454c445041494e54u128 << 64) | hash as u128);
        if project.procedural_materials.contains_key(&id) {
            continue;
        }
        spec.palette = spec
            .colors
            .iter()
            .map(|rgb| {
                shared::palette::closest(
                    &TheColor::from_u8(rgb[0], rgb[1], rgb[2], rgb[3]),
                    &project
                        .palette
                        .colors
                        .iter()
                        .map(|c| c.clone().unwrap_or_default())
                        .collect::<Vec<_>>(),
                )
                .map(|i| i as u16)
            })
            .collect();
        spec.resolve_palette(&project.palette);
        let mut visual = spec.clone();
        visual.material_id = 0;
        let key = serde_json::to_string(&visual).map_err(|e| e.to_string())?;
        let (mut image, mut layout, mut textures) = if let Some(image) = artwork.get(&key) {
            image.clone()
        } else {
            let image = spec.render()?;
            artwork.insert(key, image.clone());
            image
        };
        layout.material_id = spec.material_id;
        image.reference = Some(format!(
            "{}{}",
            shared::paint_source::PREFIX,
            serde_json::to_string(&spec).map_err(|e| e.to_string())?
        ));
        let mat = MaterialDefinition::from_id(spec.material_id);
        let [rough, metal, opacity, emission] = mat.rmoe_values();
        let asset = native_template(id, &name, &spec)?;
        let mut asset = image_source::set(&asset, "paint-source", Some(image))?;
        // Four images are stable variants, not an animation in the painter.
        composition::surface_pages::apply(
            &mut asset,
            Some(&rusterix::TileUvPages {
                animation_frames: 4,
                repeat_period: [1.; 2],
                origin: [0; 2],
                size: [1; 2],
                axes: [[0.; 3]; 2],
                units_per_page: 1,
            }),
        );
        for t in &mut textures {
            for pixel in t.data.chunks_exact_mut(4) {
                pixel[3] = (pixel[3] as f32 * opacity).round() as u8;
            }
            t.set_materials_all(rough, metal, opacity, emission);
        }
        let mut tile = rusterix::Tile::from_textures(textures);
        tile.id = id;
        tile.alias = name;
        tile.paint = Some(layout);
        tile.baked_material_data = tile.textures.iter().map(|t| t.material_bytes()).collect();
        tile.uv_pages = Some(rusterix::TileUvPages {
            animation_frames: 4,
            repeat_period: [1.; 2],
            origin: [0; 2],
            size: [1; 2],
            axes: [[0.; 3]; 2],
            units_per_page: 1,
        });
        let saved = asset.save().map_err(|e| e.to_string())?;
        project.procedural_material_cache_keys.insert(
            id,
            pixel_style::bake_key(&saved, pixel_style::density(&project.config)),
        );
        project.procedural_materials.insert(id, saved);
        project.tiles.insert(id, tile);
        let collection_id = project
            .tile_collections
            .values()
            .find(|c| c.name == collection)
            .map(|c| c.id)
            .unwrap_or_else(|| {
                let c = shared::project::TileCollectionAsset::new(collection.into());
                let id = c.id;
                project.add_tile_collection(c);
                id
            });
        project.add_source_to_collection(&collection_id, rusterix::TileSource::SingleTile(id));
    }
    synchronize(project);
    Ok(())
}
pub(super) fn decorate(view: &mut MaterialEditorView, asset: &EditableMaterialAsset) {
    for tile in view.rows.iter_mut().flat_map(|r| &mut r.operations) {
        let Some(spec) = asset
            .root
            .project
            .materials
            .iter()
            .flat_map(|m| &m.rows)
            .flat_map(|r| &r.tiles)
            .find(|t| t.id == tile.id)
            .and_then(|t| t.image.as_ref())
            .and_then(Spec::from_image)
        else {
            continue;
        };
        tile.label = if spec.stamp {
            fl!("paint_stamp_source_title")
        } else {
            fl!("paint_source_title")
        };
        tile.parameters.clear();
        tile.properties.clear();
        let mut controls = vec![];
        if spec.brush != "brick" {
            controls.push((
                "seed",
                fl!("paint_source_seed"),
                spec.seed as f32,
                0.,
                65535.,
            ));
        }
        if spec.stamp {
            controls.push(("size", fl!("paint_source_artwork_size"), spec.size, 0.1, 8.));
        } else {
            controls.push((
                "scale",
                fl!("iso_paint_pattern_scale"),
                spec.scale,
                0.25,
                4.,
            ));
            if spec.brush == "brick" {
                controls.push((
                    "detail",
                    fl!("iso_paint_pattern_detail"),
                    spec.detail,
                    0.,
                    1.,
                ));
                controls.push((
                    "variation",
                    fl!("iso_paint_pattern_variation"),
                    spec.variation,
                    0.,
                    1.,
                ));
            }
        }
        for (id, label, value, min, max) in controls {
            let parameter = format!("@paint-{id}:0");
            tile.parameters.push(MaterialNumberView {
                id: parameter.clone(),
                label: label.clone(),
                value,
                min,
                max,
            });
            tile.properties.push(MaterialPropertyView {
                id: format!("@paint-{id}"),
                label,
                group: tile.label.clone(),
                kind: MaterialPropertyKind::Number,
                components: vec![parameter],
                choices: vec![],
                tooltip: String::new(),
            });
        }
        for (i, color) in spec.colors.iter().enumerate() {
            if let Some(index) = spec.palette.get(i).copied().flatten() {
                view.palette_bindings.push(MaterialPaletteBindingView {
                    operation: tile.id.clone(),
                    parameter: format!("@paint-color-{i}"),
                    index,
                });
            }
            let components = (0..3)
                .map(|c| {
                    let id = format!("@paint-color-{i}:{c}");
                    tile.parameters.push(MaterialNumberView {
                        id: id.clone(),
                        label: format!("{} {}", fl!("iso_paint_color"), i + 1),
                        value: color[c] as f32 / 255.,
                        min: 0.,
                        max: 1.,
                    });
                    id
                })
                .collect();
            tile.properties.push(MaterialPropertyView {
                id: format!("@paint-color-{i}"),
                label: format!("{} {}", fl!("iso_paint_color"), i + 1),
                group: fl!("palette"),
                kind: MaterialPropertyKind::Color,
                components,
                choices: vec![],
                tooltip: fl!("paint_source_palette_hint"),
            });
        }
    }
}
pub(super) fn edit(
    asset: &EditableMaterialAsset,
    operation: &str,
    parameter: &str,
    value: f32,
    project: &Project,
) -> Result<EditableMaterialAsset, String> {
    if !value.is_finite() {
        return Err("Paint source value must be finite".into());
    }
    let image = asset
        .root
        .project
        .materials
        .iter()
        .flat_map(|m| &m.rows)
        .flat_map(|r| &r.tiles)
        .find(|t| t.id == operation)
        .and_then(|t| t.image.as_ref())
        .ok_or("Select a paint source")?;
    let mut spec = Spec::from_image(image).ok_or("Select a paint source")?;
    let key = parameter
        .strip_prefix("@paint-")
        .unwrap_or(parameter)
        .split(':')
        .next()
        .unwrap();
    match key {
        "seed" => spec.seed = value.clamp(0., 65535.).round() as u32,
        "size" => spec.size = value.clamp(0.1, 8.),
        "scale" => spec.scale = value.clamp(0.25, 4.),
        "detail" => spec.detail = value.clamp(0., 1.),
        "variation" => spec.variation = value.clamp(0., 1.),
        _ if key.starts_with("color-") => {
            let i = key[6..]
                .parse::<usize>()
                .map_err(|_| "Invalid paint color")?;
            let c = parameter
                .rsplit(':')
                .next()
                .and_then(|c| c.parse::<usize>().ok())
                .unwrap_or(0);
            if i >= spec.colors.len() || c >= 3 {
                return Err("Invalid paint color".into());
            }
            spec.colors[i][c] = (value.clamp(0., 1.) * 255.).round() as u8;
            if let Some(slot) = spec.palette.get_mut(i) {
                *slot = None;
            }
        }
        _ => return Err("Unknown paint source parameter".into()),
    }
    spec.resolve_palette(&project.palette);
    image_source::set(asset, operation, Some(spec.render()?.0))
}
pub(super) fn edit_palette(
    asset: &EditableMaterialAsset,
    operation: &str,
    parameter: &str,
    index: u16,
    project: &Project,
) -> Result<EditableMaterialAsset, String> {
    let mut spec = asset
        .root
        .project
        .materials
        .iter()
        .flat_map(|m| &m.rows)
        .flat_map(|r| &r.tiles)
        .find(|t| t.id == operation)
        .and_then(|t| t.image.as_ref())
        .and_then(Spec::from_image)
        .ok_or("Select a paint source")?;
    let i = parameter
        .strip_prefix("@paint-color-")
        .and_then(|s| s.parse::<usize>().ok())
        .ok_or("Invalid paint color")?;
    if i >= spec.colors.len() {
        return Err("Invalid paint color".into());
    }
    spec.palette.resize(spec.colors.len(), None);
    spec.palette[i] = Some(index);
    spec.resolve_palette(&project.palette);
    image_source::set(asset, operation, Some(spec.render()?.0))
}
pub(crate) fn synchronize(project: &mut Project) {
    normalize_collections(project);
    let updates: Vec<_> = project
        .procedural_materials
        .iter()
        .filter(|(_, b)| b.contains(shared::paint_source::PREFIX))
        .filter_map(|(id, b)| {
            let a = EditableMaterialAsset::reopen(b).ok()?;
            let mut spec = Spec::from_asset(&a)?;
            let before = spec.clone();
            spec.resolve_palette(&project.palette);
            let replacement = if before != spec {
                let operation = a
                    .root
                    .project
                    .materials
                    .iter()
                    .flat_map(|m| &m.rows)
                    .flat_map(|r| &r.tiles)
                    .find(|t| t.image.as_ref().and_then(Spec::from_image).is_some())?;
                image_source::set(&a, &operation.id, Some(spec.render().ok()?.0))
                    .ok()?
                    .save()
                    .ok()
            } else {
                None
            };
            Some((*id, native_layout(&a, &spec), replacement))
        })
        .collect();
    for (id, layout, replacement) in updates {
        if let Some(tile) = project.tiles.get_mut(&id) {
            tile.paint = Some(layout);
        }
        if let Some(bundle) = replacement {
            project.procedural_materials.insert(id, bundle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::paint_picker;
    #[test]
    fn picker_recipes_preserve_native_art_and_edits_drive_persistent_paint_and_stamps() {
        let mut p = Project::default();
        install(&mut p).unwrap();
        assert_eq!(p.tile_collections.len(), 2);
        assert!(composition::prepare_project(&mut p).is_empty());
        assert!(
            p.tiles.values().filter(|t| t.paint.is_some()).all(|t| t
                .paint
                .as_ref()
                .unwrap()
                .generator
                .is_some()),
            "stock recipes must retain their native evaluation path"
        );
        let at = IsoPaintPoint::new(
            [42, 48],
            Some(Vec3::new(0.5, 0., 0.5)),
            Some(IsoPaintOwner::Terrain { x: 0, z: 0 }),
        )
        .with_surface_uv(Some(Vec2::new(0.5, 0.5)))
        .with_paint_geo(Some(rusterix::isometric::paint::TERRAIN_PAINT_GEO));
        // Compare every stock brush and stamp against the original implementation at multiple
        // real brush sizes. Thumbnail equality alone cannot detect a rendering regression.
        for (id, tile) in &p.tiles {
            let Some(spec) = Spec::from_tile(tile) else {
                continue;
            };
            for size in [0.7, 1.9, 3.1] {
                let mut linked = IsoPaintLayer::default();
                linked.active_source = Some(*id);
                linked
                    .source_images
                    .insert(*id, std::sync::Arc::new(tile.clone()));
                linked.apply_source_settings();
                linked.active_size = size;
                linked.active_opacity = spec.opacity;
                linked.active_stamp_size_jitter = 0.;
                linked.active_stamp_rotation_jitter = 0.;
                let mut original = linked.clone();
                original.active_source = None;
                if spec.stamp {
                    let a = linked.add_stamp(at.clone());
                    let b = original.add_stamp(at.clone());
                    let find = |layer: &IsoPaintLayer, id| {
                        layer
                            .chunks
                            .values()
                            .flat_map(|c| &c.stamps)
                            .find(|s| s.id == id)
                            .unwrap()
                            .clone()
                    };
                    let mut first = find(&linked, a);
                    let mut second = find(&original, b);
                    second.variant = spec.variant.clone();
                    first.variant = spec.variant.clone();
                    let a =
                        shared::iso_paint_render::IsoPaintRenderer::iso_paint_native_stamp_raster(
                            &first,
                        );
                    let b =
                        shared::iso_paint_render::IsoPaintRenderer::iso_paint_native_stamp_raster(
                            &second,
                        );
                    assert_eq!(
                        a.0.pixels(),
                        b.0.pixels(),
                        "native stamp parity: {} at {size}",
                        tile.alias
                    );
                    assert_eq!(a.1, b.1);
                    let mut tilted = first.clone();
                    tilted.rotation = 2.3;
                    if IsoPaintStamp::keeps_upright(&spec.brush) {
                        assert_eq!(shared::iso_paint_render::IsoPaintRenderer::iso_paint_native_stamp_raster(&tilted).0.pixels(),a.0.pixels());
                    }
                    assert!(
                        shared::iso_paint_render::IsoPaintRenderer::terrain_stamp_sprite(&first)
                            .surface
                            .is_some()
                    );
                } else {
                    linked.begin_stroke(at.clone());
                    original.begin_stroke(at.clone());
                    assert_eq!(
                        linked.baked_chunks, original.baked_chunks,
                        "native brush color/material parity: {} at {size}",
                        tile.alias
                    );
                }
            }
        }
        let flower = *p
            .tiles
            .iter()
            .find(|(_, t)| t.alias == "Stamp / flowers")
            .unwrap()
            .0;
        let paint = *p
            .tiles
            .iter()
            .find(|(_, t)| t.alias == "Paint / moss")
            .unwrap()
            .0;
        let mut region = shared::region::Region::new();
        region.map.camera = rusterix::MapCamera::TwoDIso;
        let mut server = ServerContext::default();
        server.curr_region = region.id;
        server.pc = ProjectContext::Region(region.id);
        server.curr_map_tool_type = MapToolType::IsoPaint;
        p.regions = vec![region];
        let (mut ui, mut ctx) = super::super::tests::workspace_ui(1500, 950);
        let map_canvas = crate::mapeditor::MapEditor::new().init_ui(&mut ui, &mut ctx, &mut p);
        ui.canvas.set_center(map_canvas);
        let mut tool = crate::tools::iso_paint::IsoPaintTool::new();
        tool.handle_event(
            &TheEvent::Custom(TheId::named("Tile Picked"), TheValue::Id(paint)),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        ui.draw(&mut vec![0; 1500 * 950 * 4], &mut ctx);
        assert!(
            ui.get_hlayout("Game Tool Params").is_none(),
            "Paint must not take a strip out of the map viewport"
        );
        let mut buffer = TheRGBABuffer::new(TheDim::sized(1000, 700));
        tool.draw_hud(
            &mut buffer,
            &mut p.regions[0].map,
            &mut ctx,
            &mut server,
            &rusterix::Assets::default(),
        );
        let before = p.regions[0].iso_paint.clone();
        tool.region_map_event(
            MapEvent::MapClicked(Vec2::new(10, 30)),
            &mut ui,
            &mut ctx,
            &mut p.regions[0],
            &mut server,
        );
        tool.region_map_event(
            MapEvent::MapDragged(Vec2::new(800, 500)),
            &mut ui,
            &mut ctx,
            &mut p.regions[0],
            &mut server,
        );
        tool.region_map_event(
            MapEvent::MapUp(Vec2::new(800, 500)),
            &mut ui,
            &mut ctx,
            &mut p.regions[0],
            &mut server,
        );
        assert_eq!(
            p.regions[0].iso_paint, before,
            "overlay clicks and drags must never paint behind the panel"
        );
        p.regions[0].iso_paint.begin_stroke(at.clone());
        assert!(paint_picker::pick(flower, &mut p, &server));
        p.regions[0].iso_paint.add_stamp(at);
        let mut spec = selected_spec(&p, paint).unwrap();
        spec.colors = vec![[240, 20, 30, 255]; spec.colors.len()];
        spec.palette = vec![None; spec.colors.len()];
        spec.material_id = 18;
        let (old, edited) = update_spec(&mut p, paint, spec).unwrap();
        assert_ne!(old, edited);
        let mut assets = rusterix::Assets::default();
        assets.set_tiles(p.tiles.clone());
        p.regions[0].iso_paint.refresh_sources(&assets);
        assert!(
            p.regions[0].iso_paint.baked_chunks.values().any(|c| c
                .material_rgba
                .chunks_exact(4)
                .any(|m| m[0] == 254 && m[1] == 18 && m[3] > 0)),
            "native material semantics must survive direct recipe edits"
        );
        assert!(
            p.regions[0].iso_paint.baked_chunks.values().any(|c| c
                .color_rgba
                .chunks_exact(4)
                .any(|c| c[0] > c[1] * 2 && c[3] > 0)),
            "color edits must update existing native strokes"
        );
        install(&mut p).unwrap();
        assert_eq!(p.procedural_materials[&paint], edited);
        let bytes = shared::project_io::encode_project(&p).unwrap();
        let mut reopened = shared::project_io::decode_project(&bytes).unwrap();
        reopened.regions[0].iso_paint.refresh_sources(&assets);
        assert_eq!(
            reopened.regions[0].iso_paint.baked_chunks,
            p.regions[0].iso_paint.baked_chunks
        );
        // Extending the MZ graph must keep evaluating that graph, rather than silently
        // replacing its outputs with the built-in generator.
        let original = EditableMaterialAsset::reopen(&p.procedural_materials[&paint]).unwrap();
        let custom = super::super::edited_for_project(
            &original,
            MaterialEditorCommand::SetNumber {
                operation: "@surface".into(),
                parameter: "roughness:0".into(),
                value: 0.123,
            },
            &p,
        )
        .unwrap();
        p.procedural_materials.insert(paint, custom.save().unwrap());
        synchronize(&mut p);
        assert!(
            p.tiles[&paint].paint.as_ref().unwrap().generator.is_none(),
            "custom MZ outputs must stay authoritative"
        );
    }

    #[test]
    #[ignore = "native renderer and editable source review; outputs remain in generated/"]
    fn capture_paint_picker_and_mz_source_edit() {
        use crate::docks::{Dock, tiles::TilesDock};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let out = root.join("generated/paint-picker");
        std::fs::create_dir_all(&out).unwrap();
        let mut p = shared::project_io::decode_project(
            &std::fs::read(root.join("test_projects/UnendingNight.eldiron")).unwrap(),
        )
        .unwrap();
        let start = std::time::Instant::now();
        install(&mut p).unwrap();
        eprintln!("Paint library install: {:?}", start.elapsed());
        let mut server = ServerContext::default();
        server.curr_region = p.regions[0].id;
        server.pc = ProjectContext::Region(server.curr_region);
        server.curr_map_tool_type = MapToolType::IsoPaint;
        let asset = |p: &Project, name: &str| {
            p.tiles
                .iter()
                .find(|(_, t)| t.alias == name)
                .map(|(id, _)| *id)
                .unwrap()
        };
        let flower = asset(&p, "Stamp / flowers");
        for (name, points) in [
            (
                "Stamp / flowers",
                vec![
                    [17., 17.],
                    [18., 17.5],
                    [20., 18.],
                    [20.3, 18.5],
                    [23., 17.8],
                ],
            ),
            (
                "Stamp / grass",
                vec![[18.5, 18.], [21., 18.], [22., 17.8], [23.5, 18.5]],
            ),
        ] {
            let id = asset(&p, name);
            assert!(paint_picker::pick(id, &mut p, &server));
            for at in points {
                let mut point = IsoPaintPoint::new(
                    [0; 2],
                    Some(Vec3::new(at[0], 0., at[1])),
                    Some(IsoPaintOwner::Terrain { x: 0, z: 0 }),
                );
                point.surface_uv = Some(at);
                point.paint_geo = Some(rusterix::isometric::paint::TERRAIN_PAINT_GEO);
                point.viewport_size = Some([1280, 900]);
                point.camera_scale = Some(900. / 64.);
                p.regions[0].iso_paint.add_stamp(point);
            }
        }
        let player = p
            .tiles
            .iter()
            .find(|(_, t)| t.alias.to_lowercase().contains("character"))
            .map(|(id, _)| *id)
            .unwrap();
        super::super::woodland_demo::uo_art::render_at_hour(
            &p,
            player,
            &out,
            "paint-picker-scene",
            Vec3::new(16., 0., 16.),
            48.,
            Vec2::new(21., 19.),
            11.,
        );
        let (mut ui, mut ctx) = super::super::tests::workspace_ui(1660, 960);
        ui.set_theme(Box::new(TheBrownTheme::new()), &mut ctx);
        crate::hud::set_theme(&TheBrownTheme::new());
        ui.init(&mut ctx);
        ui.get_stack_layout("Game Output Stack")
            .unwrap()
            .set_index(0);
        let map_canvas = crate::mapeditor::MapEditor::new().init_ui(&mut ui, &mut ctx, &mut p);
        ui.canvas.set_center(map_canvas);
        let mut dock = TilesDock::new();
        let bottom = dock.setup(&mut ctx);
        ui.canvas.set_bottom(bottom);
        ui.get_layout("Tiles Dock Tabs")
            .unwrap()
            .limiter_mut()
            .set_max_height(230);
        // Exercise the real tool-switch path and mounted map toolbar, not a
        // manually assembled row that could hide missing editor integration.
        let mut tools = crate::toollist::ToolList::new();
        let paint_tool = tools
            .get_game_tool_uuid_by_command_id("tool.iso_paint")
            .unwrap();
        tools.set_tool(paint_tool, &mut ui, &mut ctx, &mut p, &mut server);
        assert!(ui.get_hlayout("Game Tool Params").is_none());
        let collection = p
            .tile_collections
            .values()
            .find(|c| c.name == "Organic Stamps")
            .unwrap()
            .id;
        dock.activate(&mut ui, &mut ctx, &p, &mut server);
        dock.handle_event(
            &TheEvent::Custom(
                TheId::named("Reveal Tile Collection"),
                TheValue::Id(collection),
            ),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        tools.handle_event(
            &TheEvent::Custom(TheId::named("Tile Picked"), TheValue::Id(flower)),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        server.curr_tile_id = Some(flower);
        let mut pixels = vec![0; 1660 * 960 * 4];
        ui.draw(&mut pixels, &mut ctx);
        dock.handle_event(
            &TheEvent::Custom(TheId::named("Update Tilepicker"), TheValue::Empty),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        let data = std::fs::read(out.join("paint-picker-scene.png")).unwrap();
        let mut decoder = png::Decoder::new(std::io::Cursor::new(data))
            .read_info()
            .unwrap();
        let mut rgba = vec![0; decoder.output_buffer_size().unwrap()];
        let info = decoder.next_frame(&mut rgba).unwrap();
        let image =
            TheRGBABuffer::from(rgba[..info.buffer_size()].to_vec(), info.width, info.height);
        let view = ui.get_render_view("PolyView").unwrap();
        let dim = *view.dim();
        *view.render_buffer_mut() = image.scaled(dim.width, dim.height);
        let mut assets = rusterix::Assets::default();
        assets.set_tiles(p.tiles.clone());
        tools.draw_hud(
            view.render_buffer_mut(),
            &mut p.regions[0].map,
            &mut ctx,
            &mut server,
            &assets,
        );
        ui.draw(&mut pixels, &mut ctx);
        std::fs::write(
            out.join("paint-overlay.png"),
            TheRGBABuffer::from(pixels.clone(), 1660, 960)
                .to_png()
                .unwrap(),
        )
        .unwrap();
        // Click the real overlay swatch, then the normal palette widget. Route the
        // resulting native UI event through ToolList, as the running editor does.
        let events = ui.add_state_listener("Paint Overlay Review".into());
        ui.update(&mut ctx);
        for _ in events.try_iter() {}
        let swatch = Vec2::new(16 + 82 + 36 + 15, 34 + 105 + 14);
        tools.get_current_tool().region_map_event(
            MapEvent::MapClicked(swatch),
            &mut ui,
            &mut ctx,
            &mut p.regions[0],
            &mut server,
        );
        tools.get_current_tool().region_map_event(
            MapEvent::MapUp(swatch),
            &mut ui,
            &mut ctx,
            &mut p.regions[0],
            &mut server,
        );
        ui.draw(&mut pixels, &mut ctx);
        std::fs::write(
            out.join("paint-overlay-colors.png"),
            TheRGBABuffer::from(pixels.clone(), 1660, 960)
                .to_png()
                .unwrap(),
        )
        .unwrap();
        let dim = *ui.get_widget("Paint Overlay Palette 1").unwrap().dim();
        let target = shared::palette::closest(
            &TheColor::from_u8(225, 85, 140, 255),
            &p.palette
                .colors
                .iter()
                .map(|c| c.clone().unwrap_or_default())
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let cell = ((dim.width - 20 - 15) / 16).min((dim.height - 16 - 15) / 16);
        let click = Vec2::new(
            dim.x + 10 + (target as i32 % 16) * (cell + 1) + cell / 2,
            dim.y + 8 + (target as i32 / 16) * (cell + 1) + cell / 2,
        );
        ui.touch_down(click.x as f32, click.y as f32, &mut ctx);
        ui.touch_up(click.x as f32, click.y as f32, &mut ctx);
        ui.update(&mut ctx);
        let palette_events:Vec<_>=events.try_iter().filter(|e|matches!(e,TheEvent::PaletteIndexChanged(id,_) if id.name=="Paint Overlay Palette 1")).collect();
        assert_eq!(
            palette_events.len(),
            1,
            "palette must expose a real, usable control"
        );
        for event in palette_events {
            tools.handle_event(&event, &mut ui, &mut ctx, &mut p, &mut server);
        }
        assert_eq!(
            selected_spec(&p, flower).unwrap().palette[1],
            Some(target as u16)
        );
        tools.handle_event(
            &TheEvent::ContextMenuSelected(
                TheId::named("Paint Overlay Choices"),
                TheId::named("Paint Overlay Choice Finish 1"),
            ),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        assert_eq!(selected_spec(&p, flower).unwrap().material_id, 45);
        assert!(
            Spec::from_tile(&p.tiles[&flower]).is_some(),
            "routine controls must preserve native generation"
        );
        let candle = asset(&p, "Stamp / candles");
        paint_picker::pick(candle, &mut p, &server);
        let mut point = IsoPaintPoint::new(
            [0; 2],
            Some(Vec3::new(19., 0., 18.)),
            Some(IsoPaintOwner::Terrain { x: 0, z: 0 }),
        );
        point.surface_uv = Some([19., 18.]);
        point.paint_geo = Some(rusterix::isometric::paint::TERRAIN_PAINT_GEO);
        point.viewport_size = Some([1280, 900]);
        point.camera_scale = Some(900. / 64.);
        p.regions[0].iso_paint.add_stamp(point);
        let stamp = p.regions[0]
            .iso_paint
            .chunks
            .values()
            .flat_map(|c| &c.stamps)
            .find(|s| s.source == Some(candle))
            .unwrap();
        let surface = shared::iso_paint_render::IsoPaintRenderer::terrain_stamp_sprite(stamp)
            .surface
            .unwrap();
        assert!(
            (0..surface.data.len() / 8)
                .any(|i| surface.sample(i).is_some_and(|s| s.emission > 0.9)),
            "emissive materials must reach the live iso lighting pass"
        );
        super::super::woodland_demo::uo_art::render_at_hour(
            &p,
            player,
            &out,
            "paint-materials-night",
            Vec3::new(16., 0., 16.),
            48.,
            Vec2::new(21., 19.),
            0.,
        );
        let source = EditableMaterialAsset::reopen(&p.procedural_materials[&flower]).unwrap();
        let changed = edit(&source, "paint-source", "@paint-seed:0", 1129., &p).unwrap();
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let worker = super::super::gpu::Worker::new(device, queue);
        worker.send(super::super::gpu::Job {
            tile: flower,
            revision: 0,
            bundle: changed.save().unwrap(),
            pixels_per_tile: 64,
            continuous: false,
            selection: None,
            preview_aspect: 1.,
        });
        let result = worker
            .results
            .recv_timeout(std::time::Duration::from_secs(120))
            .unwrap()
            .frame
            .unwrap();
        let pages = result.surface_pages.unwrap();
        assert_eq!(pages.layout.animation_frames, 4);
        assert_eq!(pages.textures.len(), 4);
        assert!(
            pages
                .textures
                .iter()
                .any(|t| t.data.chunks_exact(4).any(|p| p[3] > 0))
        );
        assert_ne!(pages.textures[0].data, p.tiles[&flower].textures[0].data);
    }
}
