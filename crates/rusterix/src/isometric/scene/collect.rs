use super::*;

pub(super) fn collect<'a>(
    map: &'a Map,
    assets: &'a Assets,
    seconds: f32,
    avatars: &'a HashMap<usize, AvatarSprite>,
    terrain_model: Option<Arc<TerrainSurface>>,
) -> ScenePlan<'a> {
    let mut casters = Vec::new();
    let mut mesh_instances = Vec::new();
    let mut ground = Vec::new();
    let mut upright = Vec::new();
    let mut floors = Vec::new();
    let columns: Vec<_> = map
        .sectors
        .iter()
        .filter(|s| s.properties.get_bool_default("iso_walkable", false))
        .map(|s| {
            (
                s.bounding_box(map),
                s.properties.get_float_default("floor_height", 0.),
            )
        })
        .collect();
    let player = map
        .entities
        .iter()
        .find(|e| e.is_player())
        .map(|e| e.get_pos_xz())
        .or_else(|| {
            map.entities.iter().find_map(|entity| {
                let shown = crate::server::data::character_for_display(entity, assets);
                shown.is_player().then_some(shown.get_pos_xz())
            })
        });
    let inside = player.is_some_and(|p| {
        map.sectors
            .iter()
            .any(|s| s.properties.get_bool_default("iso_interior", false) && s.is_inside(map, p))
    });
    for sector in &map.sectors {
        // Classic isometric interiors remove the roof and near walls entirely.
        // Keeping translucent roof sprites here obscures furniture and actors.
        let hidden = inside && sector.properties.get_bool_default("iso_cutaway", false);
        let Some(source) = sector.properties.get_default_source() else {
            continue;
        };
        let Some(tile) = source
            .render_tile_id(assets)
            .and_then(|id| assets.tile_index(&id))
            .and_then(|i| assets.tile_list.get(i as usize))
        else {
            continue;
        };
        let Some(layout) = &tile.isometric else {
            floors.push((sector, tile));
            continue;
        };
        if layout.ground && !layout.bitmap && (!layout.frames.is_empty() || tile.uv_pages.is_some())
        {
            let height = sector.properties.get_float_default("floor_height", 0.);
            if height.abs() < 0.001 {
                floors.push((sector, tile));
            } else if let Some(pos) = sector.center(map) {
                upright.push(Sprite {
                    owner: Some(SpriteOwner::Sector(sector.id)),
                    tile: Some(tile),
                    avatar: None,
                    stamp: None,
                    scale: 1.,
                    pos,
                    height,
                    frame: 0,
                    opacity: 1.,
                    tie: sector.id as u64,
                    support: Some(sector),
                    radius: 0.,
                    floor: Some(sector),
                });
            }
            continue;
        }
        let Some(center) = sector.center(map) else {
            continue;
        };
        let pos = sector
            .properties
            .get_vec2("iso_anchor_offset")
            .map(|offset| center + Vec2::from(offset))
            .or_else(|| sector.properties.get_vec2("iso_anchor").map(Vec2::from))
            .unwrap_or(center);
        let entry = Sprite {
            owner: Some(SpriteOwner::Sector(sector.id)),
            tile: Some(tile),
            avatar: None,
            stamp: None,
            scale: 1.,
            pos,
            height: {
                let h = sector.properties.get_float_default("floor_height", 0.);
                if h.abs() < 0.001
                    && !sector.properties.get_bool_default("iso_walkable", false)
                    && sector.properties.get_int("iso_snap_target").is_none()
                {
                    terrain_model
                        .as_ref()
                        .and_then(|t| t.height_at(pos))
                        .unwrap_or(h)
                } else {
                    h
                }
            },
            frame: {
                let yaw = sector
                    .properties
                    .get_float_default("iso_yaw", 0.)
                    .to_radians();
                layout.frame(Vec2::new(yaw.sin(), yaw.cos()), true, seconds)
            },
            opacity: 1.,
            tie: sector.id as u64,
            support: if layout.bitmap && layout.ground {
                Some(sector)
            } else if !layout.ground && !tile.blocking && !columns.is_empty() {
                let bounds = sector.bounding_box(map);
                let height = sector.properties.get_float_default("floor_height", 0.);
                // Fill blocks share a walkable column, but ordinary upright
                // props at the same cell must remain occluders.
                let footprint_top =
                    (bounds.min.y - pos.y - (bounds.max.x - pos.x)) * 0.5 * layout.pixels_per_unit;
                let is_top = sector.properties.get_bool_default("iso_walkable", false);
                let below_anchor = layout.anchor[1] as f32 <= -footprint_top + 4.;
                ((is_top || below_anchor)
                    && columns.iter().any(|(b, h)| {
                        *h + 0.001 >= height
                            && (b.min - bounds.min).magnitude_squared() < 0.00001
                            && (b.max - bounds.max).magnitude_squared() < 0.00001
                    }))
                .then_some(sector)
            } else {
                None
            },
            radius: 0.,
            floor: None,
        };
        if let Some(mesh) = layout
            .surfaces
            .get(entry.frame)
            .and_then(|f| f.shadow_mesh.as_ref())
        {
            mesh_instances.push((
                mesh,
                Vec3::new(entry.pos.x, entry.height, entry.pos.y),
                entry.scale,
            ));
        }
        if let Some(grid) = layout
            .surfaces
            .get(entry.frame)
            .and_then(|f| f.shadow.as_ref())
        {
            casters.push(lighting::Caster {
                exact: layout
                    .surfaces
                    .get(entry.frame)
                    .is_some_and(|s| s.shadow_mesh.is_some()),
                grid,
                position: Vec3::new(entry.pos.x, entry.height, entry.pos.y),
                scale: entry.scale,
            });
        }
        if hidden {
            continue;
        }
        if layout.ground && !layout.bitmap && entry.height.abs() < 0.001 {
            ground.push(entry)
        } else {
            upright.push(entry)
        }
        if sector.properties.get_bool_default("iso_walkable", false) {
            if let Some(top) = layout
                .top_material
                .and_then(|id| assets.tile_index(&id))
                .and_then(|i| assets.tile_list.get(i as usize))
                .filter(|t| t.uv_pages.is_some())
            {
                upright.push(Sprite {
                    owner: Some(SpriteOwner::Sector(sector.id)),
                    tile: Some(top),
                    avatar: None,
                    stamp: None,
                    scale: 1.,
                    pos,
                    height: sector.properties.get_float_default("floor_height", 0.),
                    frame: 0,
                    opacity: 1.,
                    tie: sector.id as u64,
                    support: Some(sector),
                    radius: 0.,
                    floor: Some(sector),
                });
            }
        }
    }
    for (index, item) in map.items.iter().enumerate() {
        if !item.attributes.get_bool_default("visible", true) {
            continue;
        }
        if item_cutaway(map, item) {
            // Cutaway objects still cast their physical mesh shadows, like hidden walls.
            if let Some(surface) = avatars
                .get(&item_sprite_key(index))
                .and_then(|a| a.surface.as_ref())
                && let Some(mesh) = surface.shadow_mesh.as_ref()
            {
                mesh_instances.push((
                    mesh,
                    item.position,
                    item.attributes.get_float_default("appearance_scale", 1.),
                ));
            }
            continue;
        }
        if let Some(sprite) = avatars.get(&item_sprite_key(index)) {
            upright.push(Sprite {
                owner: Some(SpriteOwner::Item {
                    index,
                    id: item.id,
                    creator_id: item.creator_id,
                }),
                tile: None,
                avatar: Some(sprite),
                stamp: None,
                scale: item.attributes.get_float_default("appearance_scale", 1.),
                pos: item.get_pos_xz(),
                height: item.position.y,
                frame: 0,
                opacity: 1.,
                tie: (1 << 32) + item.id as u64,
                support: None,
                radius: item.attributes.get_float_default("radius", 0.15).max(0.),
                floor: None,
            });
            continue;
        }
        let Some(Value::Source(source)) = item.attributes.get("source") else {
            continue;
        };
        let Some(tile) = source
            .render_tile_id(assets)
            .and_then(|id| assets.tile_index(&id))
            .and_then(|i| assets.tile_list.get(i as usize))
        else {
            continue;
        };
        let Some(layout) = &tile.isometric else {
            continue;
        };
        upright.push(Sprite {
            owner: Some(SpriteOwner::Item {
                index,
                id: item.id,
                creator_id: item.creator_id,
            }),
            tile: Some(tile),
            avatar: None,
            stamp: None,
            scale: item.attributes.get_float_default("size_2d", 1.),
            pos: item.get_pos_xz(),
            height: item.position.y,
            frame: layout.frame(Vec2::new(0., 1.), true, seconds),
            opacity: 1.,
            tie: (1 << 32) + item.id as u64,
            support: None,
            radius: item.attributes.get_float_default("radius", 0.15).max(0.),
            floor: None,
        });
    }
    for (index, original) in map.entities.iter().enumerate() {
        let display;
        let entity = if original.attributes.get("source").is_none() {
            display = crate::server::data::character_for_display(original, assets);
            &display
        } else {
            original
        };
        if let Some(avatar) = avatars.get(&index) {
            upright.push(Sprite {
                owner: Some(SpriteOwner::Entity {
                    index,
                    id: original.id,
                    creator_id: original.creator_id,
                }),
                tile: None,
                avatar: Some(avatar),
                stamp: None,
                scale: entity.attributes.get_float_default("size_2d", 1.),
                pos: entity.get_pos_xz(),
                height: entity.position.y,
                frame: 0,
                opacity: 1.,
                tie: (2 << 32) + index as u64,
                support: None,
                radius: entity.attributes.get_float_default("radius", 0.22).max(0.),
                floor: None,
            });
            continue;
        }
        if !entity.attributes.get_bool_default("visible", true) {
            continue;
        }
        let Some(Value::Source(source)) = entity.attributes.get("source") else {
            continue;
        };
        let Some(tile) = source
            .render_tile_id(assets)
            .and_then(|id| assets.tile_index(&id))
            .and_then(|i| assets.tile_list.get(i as usize))
        else {
            continue;
        };
        let walking = entity
            .attributes
            .get_str("avatar_animation")
            .is_some_and(|a| a.eq_ignore_ascii_case("walk"));
        upright.push(Sprite {
            owner: Some(SpriteOwner::Entity {
                index,
                id: original.id,
                creator_id: original.creator_id,
            }),
            tile: Some(tile),
            avatar: None,
            stamp: None,
            scale: entity.attributes.get_float_default("size_2d", 1.),
            pos: entity.get_pos_xz(),
            height: entity.position.y,
            frame: tile.isometric.as_ref().map_or_else(
                || tile.frame_index((seconds / 0.25) as usize),
                |layout| layout.frame(entity.orientation, walking, seconds),
            ),
            opacity: 1.,
            tie: (2 << 32) + entity.id as u64,
            support: None,
            radius: entity.attributes.get_float_default("radius", 0.22).max(0.),
            floor: None,
        });
    }
    if let Some(paint) = assets.terrain_paint.get(&map.id) {
        for stamp in &paint.stamps {
            let pos = Vec2::from(stamp.position);
            let height = if let Some(terrain) = &terrain_model {
                let Some(height) = terrain.height_at(pos) else {
                    continue;
                };
                height
            } else {
                if !flat_terrain_contains(map, pos) {
                    continue;
                }
                0.
            };
            upright.push(Sprite {
                owner: Some(SpriteOwner::TerrainStamp(stamp.id)),
                tile: None,
                avatar: None,
                stamp: Some(stamp),
                scale: 1.,
                pos,
                height,
                frame: 0,
                opacity: 1.,
                tie: (3 << 32) + stamp.order,
                support: None,
                radius: 0.,
                floor: None,
            });
        }
    }
    for entry in &upright {
        if entry.tie < (1u64 << 32) {
            continue;
        }
        if let Some(mesh) = entry
            .tile
            .and_then(|t| t.isometric.as_ref())
            .and_then(|s| s.surfaces.get(entry.frame))
            .and_then(|s| s.shadow_mesh.as_ref())
            .or_else(|| {
                entry
                    .avatar
                    .and_then(|a| a.surface.as_ref())
                    .and_then(|s| s.shadow_mesh.as_ref())
            })
        {
            mesh_instances.push((
                mesh,
                Vec3::new(entry.pos.x, entry.height, entry.pos.y),
                entry.scale,
            ));
        }
        if let Some(grid) = entry
            .tile
            .and_then(|t| t.isometric.as_ref())
            .and_then(|s| s.surfaces.get(entry.frame))
            .and_then(|s| s.shadow.as_ref())
        {
            casters.push(lighting::Caster {
                exact: entry
                    .tile
                    .and_then(|t| t.isometric.as_ref())
                    .and_then(|s| s.surfaces.get(entry.frame))
                    .is_some_and(|s| s.shadow_mesh.is_some()),
                grid,
                position: Vec3::new(entry.pos.x, entry.height, entry.pos.y),
                scale: entry.scale,
            });
        }
    }
    ground.sort_by(Sprite::painter_cmp);
    upright.sort_by(Sprite::painter_cmp);

    ScenePlan {
        material_frame: super::super::materials::animation_frame(seconds),
        terrain: terrain_model,
        ground,
        upright,
        floors,
        casters,
        mesh_instances,
    }
}
