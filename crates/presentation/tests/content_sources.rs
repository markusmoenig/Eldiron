use eldiron_presentation::screen::{ContentRevision, ContentSources};
use rusterix::{
    Assets, Avatar, AvatarAnimation, AvatarAnimationFrame, AvatarDirection, AvatarPerspective,
    AvatarPerspectiveCount, Entity, Item, Texture, Tile, Value,
    client::{
        content::ContentRecipe,
        widget::{avatar::AvatarWidget, profile::ProfileWidget},
    },
};
use std::sync::Arc;

#[test]
fn profile_art_is_source_sized_cached_and_invalidated_by_assets_and_frames() {
    let mut assets = Assets::default();
    let tile = Tile::from_textures(vec![
        Texture::new(vec![12, 34, 56, 255, 78, 90, 12, 255], 2, 1),
        Texture::from_color([50, 60, 70, 255]),
    ]);
    let id = tile.id;
    assets.set_tiles([(id, tile)].into_iter().collect());
    let mut entity = Entity::default();
    entity.attributes.set("profile_tile_id", Value::Id(id));
    let mut widget = ProfileWidget::new();
    let mut sources = ContentSources::default();
    let first = sources
        .prepare(widget.content_recipe(&entity), &assets, ContentRevision(1))
        .unwrap()
        .unwrap();
    assert_eq!((first.image().width(), first.image().height()), (2, 1));
    assert!(!first.avatar);
    assert!(widget.buffer.pixels().is_empty());
    let again = sources
        .prepare(widget.content_recipe(&entity), &assets, ContentRevision(1))
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&first, &again));
    assert_eq!(sources.builds(), 1);
    widget.animation_frame = 1;
    let animated = sources
        .prepare(widget.content_recipe(&entity), &assets, ContentRevision(1))
        .unwrap()
        .unwrap();
    assert_eq!(animated.image().pixels(), &[50, 60, 70, 255]);
    let mut tile = assets.tiles[&id].clone();
    tile.textures[1] = Texture::from_color([90, 80, 70, 255]);
    assets.set_tiles([(id, tile)].into_iter().collect());
    let edited = sources
        .prepare(widget.content_recipe(&entity), &assets, ContentRevision(2))
        .unwrap()
        .unwrap();
    assert_eq!(edited.image().pixels(), &[90, 80, 70, 255]);
    assert_eq!(animated.image().pixels(), &[50, 60, 70, 255]); // outstanding lease
    assert_ne!(edited.image_key(), animated.image_key());
}

#[test]
fn avatar_art_does_not_rebuild_for_motion_damage_or_display_identity() {
    let mut assets = Assets::default();
    let avatar = Avatar {
        id: Default::default(),
        name: "Human".into(),
        resolution: 1,
        perspective_count: AvatarPerspectiveCount::Four,
        animations: vec![AvatarAnimation {
            id: Default::default(),
            name: "Idle".into(),
            speed: 1.,
            perspectives: vec![AvatarPerspective {
                direction: AvatarDirection::Front,
                frames: vec![AvatarAnimationFrame::new(Texture::from_color([
                    24, 128, 240, 255,
                ]))],
                weapon_main_anchor: None,
                weapon_off_anchor: None,
            }],
        }],
    };
    assets.avatars.insert("Human".into(), avatar);
    assets.default_avatar = Some("Human".into());
    let mut entity = Entity::default();
    let widget = AvatarWidget::new();
    let mut sources = ContentSources::default();
    let first = sources
        .prepare(widget.content_recipe(&entity), &assets, ContentRevision(0))
        .unwrap()
        .unwrap();
    entity.position.x = 5.;
    entity.attributes.set("health", Value::Float(10.));
    entity
        .attributes
        .set("display_name", Value::Str("Changed".into()));
    let moved = sources
        .prepare(widget.content_recipe(&entity), &assets, ContentRevision(0))
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&first, &moved));
    assert!(first.avatar);
    assert!(widget.buffer.pixels().is_empty());
    entity
        .attributes
        .set("hair_color", Value::Str("#123456".into()));
    let changed = sources
        .prepare(widget.content_recipe(&entity), &assets, ContentRevision(0))
        .unwrap()
        .unwrap();
    assert!(!Arc::ptr_eq(&first, &changed));
    let mut equipment = Item::default();
    equipment
        .attributes
        .set("torso_color", Value::Str("#334455".into()));
    entity.equipped.insert("body".into(), equipment);
    let equipped = sources
        .prepare(widget.content_recipe(&entity), &assets, ContentRevision(0))
        .unwrap()
        .unwrap();
    assert!(!Arc::ptr_eq(&changed, &equipped));
    assert_eq!(sources.builds(), 3);
}

#[test]
fn item_priority_animation_and_missing_source_cache_are_independent_of_gpu() {
    let mut assets = Assets::default();
    let tile = Tile::from_textures(vec![
        Texture::from_color([255, 0, 0, 255]),
        Texture::new(vec![0, 255, 0, 255, 0, 0, 255, 255], 2, 1),
    ]);
    let mut item = Item::default();
    item.attributes.set("icon", Value::Str(tile.id.to_string()));
    let key = item.creator_id.to_string().to_ascii_lowercase();
    assets.set_tiles([(tile.id, tile)].into_iter().collect());
    let mut sources = ContentSources::default();
    let first = sources
        .prepare(
            ContentRecipe::Item {
                item: &item,
                frame: 1,
            },
            &assets,
            ContentRevision(0),
        )
        .unwrap()
        .unwrap();
    assert_eq!((first.image().width(), first.image().height()), (2, 1));
    assert_eq!(first.image().pixels()[1], 255);
    assets
        .item_icons
        .insert(key.clone(), vec![Texture::from_color([80, 40, 120, 255])]);
    assets.project_item_icon_keys.insert(key);
    let override_art = sources
        .prepare(
            ContentRecipe::Item {
                item: &item,
                frame: 1,
            },
            &assets,
            ContentRevision(1),
        )
        .unwrap()
        .unwrap();
    assert_eq!(override_art.image().pixels(), &[80, 40, 120, 255]);
    let entity = Entity::default();
    let avatar = AvatarWidget::new();
    for _ in 0..3 {
        assert!(
            sources
                .prepare(avatar.content_recipe(&entity), &assets, ContentRevision(1))
                .unwrap()
                .is_none()
        );
    }
    assert_eq!(sources.builds(), 3);
}

#[test]
fn budgets_bound_cache_ownership_and_preserve_outstanding_cpu_leases() {
    let mut assets = Assets::default();
    let tile = Tile::from_texture(Texture::from_color([10, 20, 30, 255]));
    let mut entity = Entity::default();
    entity.attributes.set("profile_tile_id", Value::Id(tile.id));
    assets.set_tiles([(tile.id, tile)].into_iter().collect());
    let widget = ProfileWidget::new();
    let mut sources = ContentSources::new(4, 1);
    let retained = sources
        .prepare(widget.content_recipe(&entity), &assets, ContentRevision(0))
        .unwrap()
        .unwrap();
    for revision in 1..10 {
        sources
            .prepare(
                widget.content_recipe(&entity),
                &assets,
                ContentRevision(revision),
            )
            .unwrap();
        assert_eq!(sources.resident_bytes(), 4);
        assert_eq!(sources.len(), 1);
    }
    sources.clear();
    assert_eq!(retained.image().pixels(), &[10, 20, 30, 255]);
    let mut no_storage = ContentSources::new(0, 0);
    assert!(
        no_storage
            .prepare(widget.content_recipe(&entity), &assets, ContentRevision(0))
            .unwrap()
            .is_some()
    );
    assert!(no_storage.is_empty());
    assert_eq!(no_storage.resident_bytes(), 0);
}

#[test]
fn profile_crop_and_layout_changes_reuse_the_original_avatar_source() {
    use eldiron_presentation::screen::ProfileLayout;
    use theframework::text::TextSystem;
    let mut pixels = vec![0; 8 * 8 * 4];
    for y in 1..7 {
        for x in 2..6 {
            pixels[(y * 8 + x) * 4..(y * 8 + x + 1) * 4].copy_from_slice(&[10, 80, 200, 255]);
        }
    }
    let avatar = Avatar {
        id: Default::default(),
        name: "Human".into(),
        resolution: 8,
        perspective_count: AvatarPerspectiveCount::Four,
        animations: vec![AvatarAnimation {
            id: Default::default(),
            name: "Idle".into(),
            speed: 1.,
            perspectives: vec![AvatarPerspective {
                direction: AvatarDirection::Front,
                frames: vec![AvatarAnimationFrame::new(Texture::new(pixels, 8, 8))],
                weapon_main_anchor: None,
                weapon_off_anchor: None,
            }],
        }],
    };
    let mut assets = Assets::default();
    assets.avatars.insert("Human".into(), avatar);
    assets.default_avatar = Some("Human".into());
    let mut widget = ProfileWidget::new();
    widget.rect = rusterix::Rect::new(0., 0., 200., 160.);
    widget.toml_str = "[ui]\ncrop='full'\nshow_weapons=false".into();
    widget.init();
    let entity = Entity::default();
    let mut sources = ContentSources::default();
    let content = sources
        .prepare(widget.content_recipe(&entity), &assets, ContentRevision(0))
        .unwrap()
        .unwrap();
    assert_eq!(content.alpha_bounds, Some([2, 1, 6, 7]));
    let mut text = TextSystem::default();
    text.register_font_named(
        "Body",
        include_bytes!("../../theframework/embedded/fonts/Roboto-Bold.ttf").to_vec(),
    );
    let mut layout = ProfileLayout::default();
    layout.prepare(&widget, &entity, &mut text, "Body", Some(&content));
    let full_uv = layout.source_uv();
    let count = layout.layouts_built();
    widget.rect.width = 240.;
    widget.toml_str = "[ui]\ncrop='face'\nshow_weapons=false\nstats_layout='vertical'\n[[ui.stats]]\nstat='HP'\nwidth=9.0".into();
    widget.init();
    let cropped = sources
        .prepare(widget.content_recipe(&entity), &assets, ContentRevision(0))
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&content, &cropped));
    layout.prepare(&widget, &entity, &mut text, "Body", Some(&content));
    assert_ne!(layout.source_uv(), full_uv);
    assert_eq!(layout.layouts_built(), count);
    assert_eq!(sources.builds(), 1);
    assert!(layout.view().unwrap().bars[0].vertical);
    assert!(widget.buffer.pixels().is_empty());
}
