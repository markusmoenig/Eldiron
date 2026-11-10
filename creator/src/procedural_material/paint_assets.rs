//! Native Paint artwork exposed as linked, editable MZ image-source recipes.
use super::*;
use rusterix::material_library::MaterialDefinition;
use shared::paint_source::Spec;
pub(crate) fn install(project: &mut Project) -> Result<(), String> {
    let mut entries = vec![];
    for family in 0..15 {
        for finish in 0..4 {
            let mut spec = Spec::new("material", false);
            spec.material_id = family * 4 + finish;
            spec.colors = vec![[132, 132, 128, 255]];
            entries.push((
                format!(
                    "Paint / {} / {}",
                    rusterix::material_library::MATERIAL_PRESET_NAMES[family as usize],
                    rusterix::material_library::MATERIAL_FINISH_NAMES[finish as usize]
                ),
                "Paint materials",
                spec,
            ));
        }
    }
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
        entries.push((format!("Paint / {brush}"), "Paint brushes", spec));
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
        entries.push((format!("Stamp / {brush}"), "Organic stamps", spec));
    }
    for variant in ["daisies", "poppies", "bluebells"] {
        let mut spec = Spec::new("flowers", true);
        spec.variant = variant.into();
        spec.material_id = 44;
        entries.push((format!("Stamp / {variant}"), "Organic stamps", spec));
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
        let asset = EditableMaterialAsset::parse(&id.to_string(), &recipe, "surface")
            .map_err(|e| e.to_string())?;
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
            Some((*id, spec.layout(), replacement))
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
        assert_eq!(p.tiles.values().filter(|t| t.paint.is_some()).count(), 85);
        let tree = p
            .tiles
            .values()
            .find(|tile| tile.alias == "Stamp / tree")
            .unwrap();
        let tree_recipe = EditableMaterialAsset::reopen(&p.procedural_materials[&tree.id]).unwrap();
        let foliage = Spec::from_asset(&tree_recipe).unwrap().colors[1];
        assert!(
            foliage[1] > foliage[0] && foliage[1] > foliage[2],
            "tree palette must retain green foliage"
        );
        let flower = p
            .tiles
            .iter()
            .find(|(_, t)| t.alias == "Stamp / flowers")
            .map(|(id, _)| *id)
            .unwrap();
        let paint = p
            .tiles
            .iter()
            .find(|(_, t)| t.alias == "Paint / stone / natural")
            .map(|(id, _)| *id)
            .unwrap();
        let stable = p.procedural_materials.clone();
        assert!(composition::prepare_project(&mut p).is_empty());
        assert_eq!(
            p.procedural_materials, stable,
            "opening the new library must preserve valid native caches"
        );
        let before = p.procedural_materials[&flower].clone();
        let asset = EditableMaterialAsset::reopen(&before).unwrap();
        let spec = Spec::from_asset(&asset).unwrap();
        let mut native = IsoPaintStamp::new(
            spec.brush.clone(),
            IsoPaintPoint::new([0; 2], None, None),
            spec.colors[0],
            vec![],
            spec.colors.clone(),
            spec.material_id,
            spec.size,
            1.,
            spec.variant.clone(),
            0.,
            0.,
        );
        native.variation = spec.seed;
        let (original, anchor) =
            shared::iso_paint_render::IsoPaintRenderer::iso_paint_native_stamp_raster(&native);
        assert_eq!(p.tiles[&flower].textures[0].data, original.pixels());
        assert_eq!(
            p.tiles[&flower].paint.as_ref().unwrap().anchor,
            anchor.map(|v| v as f32)
        );
        let modified = super::super::edited_for_project(
            &asset,
            MaterialEditorCommand::SetNumber {
                operation: "paint-source".into(),
                parameter: "@paint-seed:0".into(),
                value: 731.,
            },
            &p,
        )
        .unwrap();
        assert_ne!(
            modified.root.project.materials[0].rows[0].tiles[0].image,
            asset.root.project.materials[0].rows[0].tiles[0].image
        );
        p.procedural_materials
            .insert(flower, modified.save().unwrap());
        let edited_before_install = p.procedural_materials[&flower].clone();
        install(&mut p).unwrap();
        assert_eq!(
            p.procedural_materials[&flower], edited_before_install,
            "opening Paint must preserve recipe edits"
        );
        let mut region = shared::region::Region::new();
        region.map.camera = rusterix::MapCamera::TwoDIso;
        let mut server = ServerContext::default();
        server.pc = ProjectContext::Region(region.id);
        server.curr_region = region.id;
        server.curr_map_tool_type = MapToolType::IsoPaint;
        p.regions = vec![region];
        let (mut ui, mut ctx) = super::super::tests::workspace_ui(1500, 950);
        let mut toolbar = TheHLayout::new(TheId::named("Game Tool Params"));
        toolbar.limiter_mut().set_max_height(30);
        let mut top = TheCanvas::new();
        top.set_layout(toolbar);
        ui.canvas.set_top(top);
        assert!(paint_picker::pick(paint, &mut p, &server));
        paint_picker::toolbar(&mut ui, &mut ctx, &mut p, &server);
        assert!(ui.get_widget("Paint Picker Edit").is_some());
        assert!(ui.get_widget("Paint Picker Density").is_none());
        let layer = &mut p.regions[0].iso_paint;
        let point = IsoPaintPoint::new(
            [10, 10],
            Some(Vec3::zero()),
            Some(IsoPaintOwner::Terrain { x: 0, z: 0 }),
        )
        .with_surface_uv(Some(Vec2::new(0.5, 0.5)))
        .with_paint_geo(Some(rusterix::isometric::paint::TERRAIN_PAINT_GEO));
        layer.begin_stroke(point.clone());
        let mut replace = IsoPaintLayer::default();
        replace.active_source = Some(paint);
        replace.active_material_mode = "replace".into();
        replace.active_brush_shape = "solid".into();
        replace.source_images.insert(
            paint,
            std::sync::Arc::new(rusterix::Tile::from_texture(rusterix::Texture::alloc(1, 1))),
        );
        replace.begin_stroke(point.clone());
        assert!(
            replace.baked_chunks.values().any(|c| c
                .material_rgba
                .chunks_exact(4)
                .zip(c.color_rgba.chunks_exact(4))
                .any(|(m, c)| (128..=187).contains(&m[0]) && m[3] == 255 && c[3] == 0)),
            "a transparent MZ Replace must retain its full brush coverage"
        );
        assert_eq!(
            layer.chunks.values().next().unwrap().strokes[0].source,
            Some(paint)
        );
        assert!(layer.baked_chunks.values().any(|c| {
            c.material_rgba
                .chunks_exact(4)
                .any(|v| (64..=123).contains(&v[0]) && v[3] > 0)
        }));
        assert!(paint_picker::pick(flower, &mut p, &server));
        paint_picker::toolbar(&mut ui, &mut ctx, &mut p, &server);
        assert!(ui.get_widget("Paint Picker Density").is_some());
        let layer = &mut p.regions[0].iso_paint;
        layer.add_stamp(point);
        let stamp = layer
            .chunks
            .values()
            .flat_map(|c| &c.stamps)
            .next()
            .unwrap();
        assert_eq!(stamp.source, Some(flower));
        let sprite = shared::iso_paint_render::IsoPaintRenderer::terrain_stamp_sprite(stamp);
        assert!(sprite.texture.data.chunks_exact(4).any(|p| p[3] > 0));
        assert!(sprite.surface.is_some());
        let bytes = shared::project_io::encode_project(&p).unwrap();
        let mut p = shared::project_io::decode_project(&bytes).unwrap();
        let mut assets = rusterix::Assets::new();
        assets.set_tiles(p.tiles.clone());
        let layer = &mut p.regions[0].iso_paint;
        layer.refresh_sources(&assets);
        assert!(
            layer
                .chunks
                .values()
                .flat_map(|c| &c.stamps)
                .all(|s| s.source_image.is_some())
        );
        let old = layer.baked_chunks.clone();
        let mut tiles = assets.tiles.clone();
        let tile = tiles.get_mut(&paint).unwrap();
        for texture in &mut tile.textures {
            texture.fill([240, 20, 30, 255]);
            texture.set_materials_all(0.2, 0.8, 1., 0.4);
        }
        tile.baked_material_data = tile.textures.iter().map(|t| t.material_bytes()).collect();
        assets.set_tiles(tiles);
        layer.refresh_sources(&assets);
        assert_ne!(
            layer.baked_chunks, old,
            "recipe edits must replay existing strokes"
        );
        assert!(layer.baked_chunks.values().any(|c| {
            c.color_rgba
                .chunks_exact(4)
                .any(|p| p[0] == 240 && p[1] == 20 && p[3] > 0)
        }));
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
        ui.init(&mut ctx);
        ui.get_stack_layout("Game Output Stack")
            .unwrap()
            .set_index(0);
        let mut toolrow = TheHLayout::new(TheId::named("Game Tool Params"));
        toolrow.limiter_mut().set_max_height(30);
        let mut top = TheCanvas::new();
        top.set_layout(toolrow);
        ui.canvas.set_top(top);
        let mut dock = TilesDock::new();
        let bottom = dock.setup(&mut ctx);
        ui.canvas.set_bottom(bottom);
        ui.get_layout("Tiles Dock Tabs")
            .unwrap()
            .limiter_mut()
            .set_max_height(230);
        let collection = p
            .tile_collections
            .values()
            .find(|c| c.name == "Organic stamps")
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
        paint_picker::pick(flower, &mut p, &server);
        server.curr_tile_id = Some(flower);
        paint_picker::toolbar(&mut ui, &mut ctx, &mut p, &server);
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
        ui.draw(&mut pixels, &mut ctx);
        std::fs::write(
            out.join("paint-picker-toolbar.png"),
            TheRGBABuffer::from(pixels, 1660, 960).to_png().unwrap(),
        )
        .unwrap();
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
