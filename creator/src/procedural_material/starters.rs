//! Standalone embedded recipes copied into newly-created projects.
use super::*;
use denrim_materials::ir::Channel;

const DIRECTORY: &str = "materials/";

pub(crate) fn install(project: &mut Project) -> Result<(), String> {
    let mut files: Vec<_> = crate::Embedded::iter()
        .filter(|name| name.starts_with(DIRECTORY) && name.ends_with(".material"))
        .map(|name| name.into_owned())
        .collect();
    files.sort();
    // Validate everything before mutating the project. Stable embedded identities
    // make installation idempotent without comparing editable names or recipes.
    let mut prepared = Vec::new();
    for path in files {
        // Explicit FNV-1a keeps identities stable across Rust/compiler versions.
        let hash = path.bytes().fold(0xcbf29ce484222325_u64, |h, byte| {
            (h ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        });
        let id = Uuid::from_u128((0x454c444d41544552_u128 << 64) | u128::from(hash));
        if project.tiles.contains_key(&id) || project.procedural_materials.contains_key(&id) {
            continue;
        }
        let prepare = || -> Result<_, String> {
            let bytes = crate::Embedded::get(&path).ok_or("Embedded recipe is missing")?;
            let source = std::str::from_utf8(bytes.data.as_ref()).map_err(|e| e.to_string())?;
            let document = format::parse(source).map_err(|e| e.to_string())?;
            let selected = document
                .result
                .as_deref()
                .or_else(|| document.materials.first().map(|m| m.id.as_str()))
                .ok_or("Recipe has no material")?;
            let mut asset = EditableMaterialAsset::parse(&id.to_string(), source, selected)
                .map_err(|e| e.to_string())?;
            super::palette::ensure_color_bindings(&mut asset, &project.palette)?;
            let palette_path = path.replace(".material", ".palette.json");
            if let Some(bytes) = crate::Embedded::get(&palette_path) {
                let references: std::collections::BTreeMap<
                    String,
                    std::collections::BTreeMap<String, u16>,
                > = serde_json::from_slice(bytes.data.as_ref()).map_err(|e| e.to_string())?;
                for (operation, parameters) in references {
                    for (parameter, index) in parameters {
                        // Custom palettes may have fewer slots; retain the nearest
                        // existing color chosen on import rather than inserting RGB.
                        if project
                            .palette
                            .colors
                            .get(index as usize)
                            .is_some_and(Option::is_some)
                        {
                            asset = super::palette::bind_parameter(
                                &asset,
                                &operation,
                                &parameter,
                                index,
                                &project.palette,
                            )?;
                        }
                    }
                }
            }
            let material = asset
                .root
                .project
                .materials
                .iter()
                .find(|m| m.id == selected)
                .ok_or("Result material is missing")?;
            if material.mode != "pbr" {
                return Err("Starter materials must use PBR mode".into());
            }
            let default = |channel: Channel| -> &[f32] {
                material
                    .defaults
                    .iter()
                    .find(|(name, _)| name == channel.name())
                    .map_or(channel.default(), |(_, value)| value.as_slice())
            };
            // A useful swatch is available before the asynchronous GPU bake.
            // This is a cache only; the complete standalone recipe stays editable.
            let color = default(Channel::BaseColor);
            let opacity = default(Channel::Opacity)[0];
            let rgb: [u8; 3] = std::array::from_fn(|i| {
                let v = color[i].max(0.);
                let v = if v <= 0.0031308 {
                    v * 12.92
                } else {
                    1.055 * v.powf(1. / 2.4) - 0.055
                };
                (v.clamp(0., 1.) * 255.).round() as u8
            });
            let size = super::pixel_style::bake_size(super::pixel_style::density(&project.config));
            let mut texture = rusterix::Texture::alloc(size, size);
            texture.fill([
                rgb[0],
                rgb[1],
                rgb[2],
                (opacity.clamp(0., 1.) * 255.).round() as u8,
            ]);
            let emission = default(Channel::Emission)
                .iter()
                .copied()
                .fold(0_f32, f32::max);
            for y in 0..size {
                for x in 0..size {
                    texture.set_materials(
                        x as u32,
                        y as u32,
                        default(Channel::Roughness)[0],
                        default(Channel::Metallic)[0],
                        1.,
                        emission,
                    );
                }
            }
            let mut tile = rusterix::Tile::from_texture(texture);
            tile.id = id;
            tile.alias = if material.label.is_empty() {
                material.id.clone()
            } else {
                material.label.clone()
            };
            tile.blocking = matches!(tile.alias.as_str(), "Rock" | "Water" | "Walls");
            tile.role = match tile.alias.as_str() {
                "Grass" => rusterix::TileRole::Nature,
                "Rock" | "Hills" => rusterix::TileRole::Mountain,
                "Street" => rusterix::TileRole::Road,
                "Water" => rusterix::TileRole::Water,
                "Walls" => rusterix::TileRole::ManMade,
                _ => rusterix::TileRole::ManMade,
            };
            tile.baked_material_data = vec![tile.textures[0].material_bytes()];
            let bundle = asset.save().map_err(|e| e.to_string())?;
            Ok((tile, bundle))
        };
        prepared.push(prepare().map_err(|e| format!("{path}: {e}"))?);
    }
    let mut ruleset_tiles = shared::rulesets::bundled_tile_ids_for_project(&project.config)?;
    ruleset_tiles.extend(
        shared::ruleset_materials::definitions(&project.config)
            .iter()
            .map(|d| shared::rulesets::material_asset_id(d.id)),
    );
    let visible = |id: &Uuid| {
        !ruleset_tiles.contains(id)
            && !project.map_material_bindings.contains_key(id)
            && !shared::ruleset_materials::is_library_tile(&project.config, id)
    };
    let cols = project.tile_board_cols.max(2);
    let mut next = project
        .tile_board_tiles
        .iter()
        .filter(|(id, _)| visible(id))
        .map(|(_, pos)| pos)
        .filter(|p| p.x >= 0 && p.y >= 0)
        .map(|p| p.y * cols + p.x + 1)
        .max()
        .unwrap_or(0)
        .max(project.tiles.keys().filter(|id| visible(id)).count() as i32);
    for (tile, bundle) in prepared {
        let mut pos = Vec2::new(next % cols, next / cols);
        while project.tile_board_tiles.iter().any(|(id, p)| {
            !ruleset_tiles.contains(id)
                && !shared::ruleset_materials::is_library_tile(&project.config, id)
                && !project.map_material_bindings.contains_key(id)
                && *p == pos
        }) || project.tile_board_empty_slots.contains(&pos)
        {
            next += 1;
            pos = Vec2::new(next % cols, next / cols);
        }
        let id = tile.id;
        project.tiles.insert(id, tile);
        project.procedural_materials.insert(id, bundle);
        project.place_tile_at_picker_selection(id, Some((None, pos)));
        // No cache key: the usual GPU worker supplies the first real bake.
        next += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn starter_metadata_reaches_the_2d_blocking_asset_set() {
        let mut project = Project::default();
        install(&mut project).unwrap();
        let mut assets = rusterix::Assets::default();
        assets.tiles = project.tiles.clone();
        let blocked = assets.blocking_tiles();
        for (name, role, blocking) in [
            ("Grass", rusterix::TileRole::Nature, false),
            ("Rock", rusterix::TileRole::Mountain, true),
            ("Hills", rusterix::TileRole::Mountain, false),
            ("Street", rusterix::TileRole::Road, false),
            ("Walls", rusterix::TileRole::ManMade, true),
            ("Water", rusterix::TileRole::Water, true),
        ] {
            let tile = project
                .tiles
                .values()
                .find(|tile| tile.alias == name)
                .unwrap();
            assert_eq!(tile.role, role, "{name}");
            assert_eq!(
                blocked.contains(&tile.id),
                blocking,
                "{name} must have the correct 2D collision behavior"
            );
        }
    }

    #[test]
    fn embedded_starters_are_standalone_editable_pbr_materials() {
        let mut project = Project::default();
        install(&mut project).unwrap();
        let names: Vec<_> = project.tiles.values().map(|t| t.alias.as_str()).collect();
        assert_eq!(
            names,
            ["Grass", "Rock", "Hills", "Street", "Walls", "Water"]
        );
        assert_eq!(project.tile_board_tiles.len(), 6);
        assert!(project.procedural_material_cache_keys.is_empty());
        for (id, bundle) in &project.procedural_materials {
            let asset = EditableMaterialAsset::reopen(bundle).unwrap();
            asset.validate().unwrap();
            assert_eq!(asset.root.id, id.to_string());
            assert!(asset.dependencies.is_empty());
            let source = format::write(&asset.root.project);
            let program = denrim_materials::compile(&source).unwrap();
            let evaluator = denrim_materials::cpu::Evaluator::new(&program);
            let mut colors = Vec::new();
            for uv in [[0.13, 0.27], [0.63, 0.81], [-0.3, 1.4], [2.11, -1.2]] {
                for channel in [
                    Channel::BaseColor,
                    Channel::Roughness,
                    Channel::Metallic,
                    Channel::Opacity,
                    Channel::Emission,
                ] {
                    let value = evaluator
                        .sample(&asset.selected_material, channel, uv)
                        .unwrap();
                    assert!(
                        value.iter().all(|v| v.is_finite()),
                        "{} {channel:?}",
                        project.tiles[id].alias
                    );
                    if channel == Channel::BaseColor {
                        colors.push(value);
                    }
                }
            }
            // Sparse asphalt grain can intentionally leave several isolated
            // samples in the same palette band. Check a whole surface patch.
            for y in 0..16 {
                for x in 0..16 {
                    colors.push(
                        evaluator
                            .sample(
                                &asset.selected_material,
                                Channel::BaseColor,
                                [(x as f32 + 0.37) / 8., (y as f32 + 0.61) / 8.],
                            )
                            .unwrap(),
                    );
                }
            }
            assert!(
                colors.windows(2).any(|pair| pair[0] != pair[1]),
                "{} should have texture detail",
                project.tiles[id].alias
            );
            assert!(
                project.tiles[id].textures[0]
                    .data
                    .chunks_exact(4)
                    .all(|p| p[3] == 255)
            );
        }
    }
    #[test]
    fn installing_twice_preserves_edits_positions_and_other_tiles() {
        let mut project = Project::default();
        let mut existing = rusterix::Tile::from_texture(rusterix::Texture::alloc(8, 8));
        existing.alias = "User art".into();
        let original = existing.id;
        project.tiles.insert(original, existing);
        project
            .set_tile_board_position(rusterix::TileSource::SingleTile(original), Vec2::new(3, 1));
        project.reserve_tile_board_empty_slot(Vec2::new(4, 1));
        install(&mut project).unwrap();
        let id = project.procedural_materials.keys().next().copied().unwrap();
        project.tiles.get_mut(&id).unwrap().alias = "My grass".into();
        project
            .procedural_materials
            .insert(id, "Edited by the user".into());
        let before = project.clone();
        install(&mut project).unwrap();
        assert_eq!(project.tiles, before.tiles);
        assert_eq!(project.procedural_materials, before.procedural_materials);
        assert_eq!(project.tile_board_tiles, before.tile_board_tiles);
        assert_eq!(
            project.tile_board_empty_slots,
            before.tile_board_empty_slots
        );
        assert_eq!(project.tiles.len(), 7);
        let unique: std::collections::HashSet<_> = project
            .tile_board_tiles
            .values()
            .map(|p| (p.x, p.y))
            .collect();
        assert_eq!(unique.len(), project.tile_board_tiles.len());
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod gpu_tests {
    use super::*;
    #[test]
    #[ignore = "requires a GPU device"]
    fn brick_grid_and_water_support_repeating_and_seamless_gpu_bakes() {
        let (device, queue) = pollster::block_on(async {
            let adapter = wgpu::Instance::default()
                .request_adapter(&Default::default())
                .await
                .unwrap();
            adapter.request_device(&Default::default()).await.unwrap()
        });
        let worker = super::super::gpu::Worker::new(device, queue);
        let mut project = Project::default();
        install(&mut project).unwrap();
        for (id, bundle) in &project.procedural_materials {
            if !matches!(project.tiles[id].alias.as_str(), "Walls" | "Water") {
                continue;
            }
            let mut asset = EditableMaterialAsset::reopen(bundle).unwrap();
            let pages = rusterix::TileUvPages {
                animation_frames: 1,
                repeat_period: [0.; 2],
                origin: [0, 0],
                size: [2, 1],
                axes: [[1., 0., 0.], [0., 0., 1.]],
                units_per_page: 1,
            };
            super::super::composition::surface_pages::apply(&mut asset, Some(&pages));
            for mode in ["repeat", "seamless"] {
                asset
                    .root
                    .project
                    .materials
                    .iter_mut()
                    .find(|m| m.id == asset.selected_material)
                    .unwrap()
                    .tiling
                    .mode = denrim_materials::periodic::Mode::parse(mode).unwrap();
                worker.send(super::super::gpu::Job {
                    pixels_per_tile: 16,
                    tile: *id,
                    revision: 0,
                    bundle: asset.save().unwrap(),
                    continuous: true,
                    selection: None,
                    preview_aspect: 1.,
                });
                let frame = worker
                    .results
                    .recv_timeout(std::time::Duration::from_secs(60))
                    .unwrap()
                    .frame
                    .unwrap_or_else(|e| panic!("{} {mode}: {e}", project.tiles[id].alias));
                assert!(frame.warnings.is_empty(), "{:?}", frame.warnings);
                let pages = frame.surface_pages.unwrap();
                assert_eq!(
                    pages.textures[0].data, pages.textures[1].data,
                    "{} {mode}",
                    project.tiles[id].alias
                );
            }
        }
    }
    #[test]
    #[ignore = "requires a GPU device; writes /tmp/eldiron-starter-materials.png"]
    fn embedded_starters_bake_with_the_existing_gpu_worker() {
        let (device, queue) = pollster::block_on(async {
            let adapter = wgpu::Instance::default()
                .request_adapter(&Default::default())
                .await
                .unwrap();
            adapter.request_device(&Default::default()).await.unwrap()
        });
        let worker = super::super::gpu::Worker::new(device, queue);
        let mut project = Project::default();
        install(&mut project).unwrap();
        let count = project.procedural_materials.len();
        let mut preview = vec![0; count * 192 * 192 * 4];
        let mut world_preview = preview.clone();
        for (column, (id, bundle)) in project.procedural_materials.iter().enumerate() {
            worker.send(super::super::gpu::Job {
                pixels_per_tile: super::super::pixel_style::density(&project.config),
                tile: *id,
                revision: 0,
                bundle: bundle.clone(),
                continuous: false,
                selection: None,
                preview_aspect: 1.,
            });
            let completed = worker
                .results
                .recv_timeout(std::time::Duration::from_secs(60))
                .unwrap();
            assert_eq!(completed.tile, *id);
            let frame = completed
                .frame
                .unwrap_or_else(|e| panic!("{}: {e}", project.tiles[id].alias));
            assert!(
                frame.warnings.is_empty(),
                "{}: {:?}",
                project.tiles[id].alias,
                frame.warnings
            );
            assert_eq!(frame.tile.width, 16);
            assert_eq!(frame.tile.height, 16);
            let first = &frame.tile.data[..3];
            assert!(
                frame.tile.data.chunks_exact(4).any(|p| &p[..3] != first),
                "{} should bake texture detail",
                project.tiles[id].alias
            );
            assert!(frame.tile.data.chunks_exact(4).all(|p| p[3] == 255));
            for y in 0..192 {
                for x in 0..192 {
                    let src = ((y * frame.size / 192) * frame.size + x * frame.size / 192) * 4;
                    let dst = (y * count * 192 + column * 192 + x) * 4;
                    preview[dst..dst + 4].copy_from_slice(&frame.images[0][src..src + 4]);
                }
            }
            worker.send(super::super::gpu::Job {
                pixels_per_tile: super::super::pixel_style::density(&project.config),
                tile: *id,
                revision: 0,
                bundle: bundle.clone(),
                continuous: true,
                selection: None,
                preview_aspect: 1.,
            });
            let world = worker
                .results
                .recv_timeout(std::time::Duration::from_secs(60))
                .unwrap()
                .frame
                .unwrap();
            for y in 0..192 {
                for x in 0..192 {
                    let src = ((y * world.size / 192) * world.size + x * world.size / 192) * 4;
                    let dst = (y * count * 192 + column * 192 + x) * 4;
                    world_preview[dst..dst + 4].copy_from_slice(&world.images[0][src..src + 4]);
                }
            }
        }
        for (path, pixels) in [
            ("/tmp/eldiron-starter-materials.png", preview),
            ("/tmp/eldiron-starter-materials-world.png", world_preview),
        ] {
            let file = std::fs::File::create(path).unwrap();
            let mut encoder = png::Encoder::new(file, (count * 192) as u32, 192);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&pixels)
                .unwrap();
        }
    }
}
