//! Screen-consumer coverage for MZ's baked runtime tiles.
//! Native screen initialization tests need a GPU adapter; sampling/compositing tests run on the CPU.
#![cfg(feature = "graphics")]

use crate::{
    Assets, Client, Item, Map, PixelSource, Rect, SceneHandler, Texture, Value,
    client::{
        draw2d::Draw2D,
        widget::{ButtonVisualState, Widget, screen::ScreenWidget, stat::StatWidget},
    },
    map::tile::TileUvPages,
    prelude::Tile,
};
use denrim_materials::integration::{
    EditableMaterialAsset, InstanceRecord, PaletteBinding, ParameterKey,
};
use theframework::prelude::*;

fn mz_frame(color: &str) -> Texture {
    mz_frame_in_mode(color, "pbr")
}

fn mz_frame_in_mode(color: &str, mode: &str) -> Texture {
    let mut palette = ThePalette::default();
    palette.colors = vec![Some(TheColor::from_hex(color))];
    let mut asset = EditableMaterialAsset::parse(
        "screen-audit",
        r#"project 1
material "panel" called "Panel"
  mode pbr
  row "paint"
    use "Color" id "paint"
  output base color from row "paint" xyz
  output opacity from row "paint" w
"#,
        "panel",
    )
    .unwrap();
    asset.root.project.materials[0].mode = mode.into();
    let mut record = InstanceRecord::new("eldiron-palette");
    record.palette_bindings.push(PaletteBinding {
        key: ParameterKey::new("screen-audit", "paint", "color"),
        index: 0,
    });
    asset.instances.push(record);
    crate::ruleset_material::bake(&asset, &palette, 4).unwrap()
}

fn assets_with(tile: &Tile) -> Assets {
    let mut assets = Assets::default();
    assets.set_tiles([(tile.id, tile.clone())].into_iter().collect());
    assets
}

fn screen_map(id: Uuid, role: Option<&str>) -> Map {
    let mut map = Map::default();
    let vertices = [(0., 0.), (2., 0.), (2., 2.), (0., 2.)].map(|(x, y)| map.add_vertex_at(x, y));
    for i in 0..4 {
        map.create_linedef(vertices[i], vertices[(i + 1) % 4]);
    }
    assert_eq!(map.sectors.len(), 1);
    map.sectors[0]
        .properties
        .set("source", Value::Source(PixelSource::TileId(id)));
    if let Some(role) = role {
        map.sectors[0]
            .properties
            .set("data", Value::Str(format!("[ui]\nrole = \"{role}\"\n")));
    }
    map
}

fn screen() -> ScreenWidget {
    let mut screen = ScreenWidget::new();
    screen.buffer.resize(16, 16);
    screen.grid_size = 8.;
    screen
}

fn pixel(buffer: &TheRGBABuffer, x: usize, y: usize) -> [u8; 4] {
    let i = (y * buffer.dim().width as usize + x) * 4;
    buffer.pixels()[i..i + 4].try_into().unwrap()
}

#[test]
fn screen_renders_mz_palette_color_opacity_and_rebaked_palette() {
    let tile = Tile::from_texture(mz_frame("#4080c080"));
    let mut assets = assets_with(&tile);
    let map = screen_map(tile.id, None);
    let mut screen = screen();
    screen.background_color = [20, 40, 60, 255];
    screen.build(&map, &assets);
    screen.draw(&map, &TheTime::new(), &assets);
    // Stay off the shared triangle edge; its double blending is a separate gap.
    let actual = pixel(&screen.buffer, 4, 5);
    let foreground = &tile.textures[0].data[..4];
    let expected: [u8; 4] = std::array::from_fn(|c| {
        if c == 3 {
            255
        } else {
            (foreground[c] as f32 * foreground[3] as f32 / 255.
                + screen.background_color[c] as f32 * (1. - foreground[3] as f32 / 255.))
                as u8
        }
    });
    assert_eq!(actual, expected);
    let mut rebaked = tile.clone();
    rebaked.textures = vec![mz_frame("#d09030")];
    assets.set_tiles([(tile.id, rebaked.clone())].into_iter().collect());
    screen.build(&map, &assets);
    screen.draw(&map, &TheTime::new(), &assets);
    assert_eq!(pixel(&screen.buffer, 4, 4), rebaked.textures[0].data[..4]);
}

#[test]
fn screen_background_follows_mz_animation_frames() {
    let tile = Tile::from_textures(vec![mz_frame("#4080c0"), mz_frame("#d09030")]);
    let assets = assets_with(&tile);
    let map = screen_map(tile.id, None);
    let mut screen = screen();
    for frame in 0..4 {
        screen.scene.animation_frame = frame;
        screen.build(&map, &assets);
        screen.draw(&map, &TheTime::new(), &assets);
        assert_eq!(
            pixel(&screen.buffer, 4, 4),
            tile.textures[frame % 2].data[..4]
        );
    }
}

#[test]
fn screen_renders_mz_art_mode_color() {
    let tile = Tile::from_texture(mz_frame_in_mode("#4080c0", "art"));
    let assets = assets_with(&tile);
    let map = screen_map(tile.id, None);
    let mut screen = screen();
    screen.build(&map, &assets);
    screen.draw(&map, &TheTime::new(), &assets);
    assert_eq!(pixel(&screen.buffer, 4, 5), [64, 128, 192, 255]);
}

fn client_screen(tile: &Tile, role: &str) -> (Client, Assets) {
    let mut assets = assets_with(tile);
    assets
        .screens
        .insert("audit".into(), screen_map(tile.id, Some(role)));
    let mut client = Client::default();
    client.init_screen("audit".into(), &mut assets, &mut SceneHandler::default());
    (client, assets)
}

fn button_pixel(client: &mut Client, assets: &Assets, frame: usize) -> [u8; 4] {
    let mut target = TheRGBABuffer::new(TheDim::sized(16, 16));
    let button = client.button_widgets.values_mut().next().unwrap();
    button.rect = Rect::new(0., 0., 16., 16.);
    button.update_draw(
        &mut target,
        &Map::default(),
        assets,
        None,
        &Draw2D::default(),
        &frame,
        ButtonVisualState::Normal,
        None,
    );
    pixel(&target, 4, 4)
}

#[test]
#[ignore = "Requires a native GPU adapter for Client/SceneHandler initialization"]
fn screen_button_renders_static_mz_tile() {
    let tile = Tile::from_texture(mz_frame("#4080c0"));
    let (mut client, assets) = client_screen(&tile, "button");
    assert_eq!(
        button_pixel(&mut client, &assets, 0),
        tile.textures[0].data[..4]
    );
}

#[test]
#[ignore = "Requires a native GPU adapter for Client/SceneHandler initialization"]
fn screen_button_follows_mz_animation_frames() {
    let tile = Tile::from_textures(vec![mz_frame("#4080c0"), mz_frame("#d09030")]);
    let (mut client, assets) = client_screen(&tile, "button");
    assert_eq!(
        button_pixel(&mut client, &assets, 1),
        tile.textures[1].data[..4]
    );
}

#[test]
#[ignore = "Requires a native GPU adapter for Client/SceneHandler initialization"]
fn screen_button_follows_rebaked_palette() {
    let tile = Tile::from_texture(mz_frame("#4080c0"));
    let (mut client, mut assets) = client_screen(&tile, "button");
    let mut rebaked = tile.clone();
    rebaked.textures = vec![mz_frame("#d09030")];
    assets.set_tiles([(tile.id, rebaked.clone())].into_iter().collect());
    assert_eq!(
        button_pixel(&mut client, &assets, 0),
        rebaked.textures[0].data[..4]
    );
}

#[test]
#[ignore = "Requires a native GPU adapter for Client/SceneHandler initialization"]
fn screen_deco_renders_assigned_mz_tile() {
    let tile = Tile::from_texture(mz_frame("#4080c0"));
    let (mut client, assets) = client_screen(&tile, "deco");
    let mut target = TheRGBABuffer::new(TheDim::sized(16, 16));
    let deco = client.deco_widgets.values_mut().next().unwrap();
    deco.rect = Rect::new(0., 0., 16., 16.);
    deco.update_draw(&mut target, &Map::default(), &Default::default(), &assets);
    assert_eq!(pixel(&target, 4, 4), tile.textures[0].data[..4]);
}

#[test]
fn screen_background_selects_continuous_mz_pages() {
    let mut tile = Tile::from_textures(vec![mz_frame("#4080c0"), mz_frame("#d09030")]);
    tile.uv_pages = Some(TileUvPages {
        animation_frames: 1,
        repeat_period: [0.; 2],
        origin: [0; 2],
        size: [2, 1],
        axes: [[0.; 3]; 2],
        units_per_page: 1,
    });
    let assets = assets_with(&tile);
    let map = screen_map(tile.id, None);
    let mut screen = screen();
    screen.build(&map, &assets);
    screen.draw(&map, &TheTime::new(), &assets);
    assert_eq!(pixel(&screen.buffer, 4, 4), tile.textures[0].data[..4]);
    assert_eq!(pixel(&screen.buffer, 12, 4), tile.textures[1].data[..4]);
}

#[test]
fn screen_transparent_pass_preserves_straight_mz_color() {
    let tile = Tile::from_texture(mz_frame("#4080c080"));
    let assets = assets_with(&tile);
    let map = screen_map(tile.id, None);
    let mut screen = screen();
    screen.build(&map, &assets);
    screen.draw_transparent(&map, &TheTime::new(), &assets);
    assert_eq!(pixel(&screen.buffer, 4, 5), tile.textures[0].data[..4]);
}

#[test]
fn screen_translucent_geometry_has_no_internal_triangle_seam() {
    let tile = Tile::from_texture(mz_frame("#4080c080"));
    let assets = assets_with(&tile);
    let map = screen_map(tile.id, None);
    let mut screen = screen();
    screen.build(&map, &assets);
    screen.draw(&map, &TheTime::new(), &assets);
    assert_eq!(pixel(&screen.buffer, 4, 4), pixel(&screen.buffer, 4, 5));
}

#[test]
fn screen_stat_resolves_current_mz_frame_and_refreshed_pixels() {
    let tile = Tile::from_textures(vec![mz_frame("#4080c0"), mz_frame("#d09030")]);
    let mut assets = assets_with(&tile);
    let mut stat = StatWidget::new();
    stat.rect = Rect::new(0., 0., 16., 16.);
    stat.buffer.resize(16, 16);
    stat.toml_str = format!(
        "[ui]\nmode = \"tiles\"\nclip = false\ntile_id = \"{}\"\n",
        tile.id
    );
    stat.init();
    let mut target = TheRGBABuffer::new(TheDim::sized(16, 16));
    stat.update_draw(&mut target, &assets, None, &Draw2D::default(), 1);
    assert_eq!(pixel(&target, 4, 5), tile.textures[1].data[..4]);
    let mut rebaked = tile.clone();
    rebaked.textures[1] = mz_frame("#b070d0");
    assets.set_tiles([(tile.id, rebaked.clone())].into_iter().collect());
    stat.update_draw(&mut target, &assets, None, &Draw2D::default(), 1);
    assert_eq!(pixel(&target, 4, 5), rebaked.textures[1].data[..4]);
}

#[test]
fn inventory_item_icon_resolves_current_mz_frame_and_refreshed_pixels() {
    let tile = Tile::from_textures(vec![mz_frame("#4080c0"), mz_frame("#d09030")]);
    let mut assets = assets_with(&tile);
    let mut item = Item::default();
    item.attributes.set("icon", Value::Str(tile.id.to_string()));
    let mut target = TheRGBABuffer::new(TheDim::sized(16, 16));
    let rect = Rect::new(0., 0., 16., 16.);
    assert!(Widget::draw_item_icon(
        &mut target,
        rect,
        &assets,
        &item,
        &Draw2D::default(),
        1
    ));
    assert_eq!(pixel(&target, 6, 7), tile.textures[1].data[..4]);
    let mut rebaked = tile.clone();
    rebaked.textures[1] = mz_frame("#b070d0");
    assets.set_tiles([(tile.id, rebaked.clone())].into_iter().collect());
    assert!(Widget::draw_item_icon(
        &mut target,
        rect,
        &assets,
        &item,
        &Draw2D::default(),
        1
    ));
    assert_eq!(pixel(&target, 6, 7), rebaked.textures[1].data[..4]);
}

#[test]
fn screen_overlapping_alpha_layers_composite_once() {
    let tile = Tile::from_texture(Texture::from_color([200, 0, 0, 128]));
    let assets = assets_with(&tile);
    let map = screen_map(tile.id, None);
    let mut screen = screen();
    screen.build(&map, &assets);
    let mut overlay = screen.scene.d2_static[0].clone();
    overlay.source = PixelSource::Pixel([0, 0, 200, 128]);
    screen.scene.d2_static.push(overlay);
    screen.draw_transparent(&map, &TheTime::new(), &assets);
    assert_eq!(pixel(&screen.buffer, 4, 5), [66, 0, 133, 192]);
    let mut target = TheRGBABuffer::new(TheDim::sized(16, 16));
    target.fill([0, 0, 0, 255]);
    target.blend_into(0, 0, &screen.buffer);
    assert_eq!(pixel(&target, 4, 5), [49, 0, 100, 255]);
}

#[test]
fn command_icons_follow_temporal_frames_and_rebakes() {
    let tile = Tile::from_textures(vec![mz_frame("#4080c0"), mz_frame("#d09030")]);
    let mut assets = assets_with(&tile);
    assets.rules = format!("[ui.commands.inspect]\nicon = \"{}\"\n", tile.id);
    assert_eq!(
        Widget::command_icon_texture(&assets, Some("ui.inspect"), ButtonVisualState::Normal, 1)
            .unwrap()
            .data,
        tile.textures[1].data
    );
    let mut updated = tile.clone();
    updated.textures[1] = mz_frame("#b070d0");
    assets.set_tiles([(tile.id, updated.clone())].into_iter().collect());
    assert_eq!(
        Widget::command_icon_texture(&assets, Some("ui.inspect"), ButtonVisualState::Normal, 1)
            .unwrap()
            .data,
        updated.textures[1].data
    );
}

#[test]
fn spatial_pages_respect_negative_origins_page_scale_and_animation_phase() {
    let mut tile = Tile::from_textures(vec![
        Texture::from_color([10, 0, 0, 255]),
        Texture::from_color([20, 0, 0, 255]),
        Texture::from_color([30, 0, 0, 255]),
        Texture::from_color([40, 0, 0, 255]),
    ]);
    tile.uv_pages = Some(TileUvPages {
        animation_frames: 2,
        repeat_period: [0.; 2],
        origin: [-2, 0],
        size: [2, 1],
        axes: [[0.; 3]; 2],
        units_per_page: 2,
    });
    for (uv, frame, red) in [
        ([-3.5, 0.5], 0, 10),
        ([-1.5, 0.5], 0, 20),
        ([-3.5, 0.5], 1, 30),
        ([-1.5, 0.5], 1, 40),
    ] {
        assert_eq!(
            tile.sample_2d(
                uv,
                frame,
                crate::SampleMode::Nearest,
                crate::RepeatMode::RepeatXY
            ),
            [red, 0, 0, 255]
        );
    }
    let mut item = Item::default();
    item.attributes.set("icon", Value::Str(tile.id.to_string()));
    let assets = assets_with(&tile);
    let mut target = TheRGBABuffer::new(TheDim::sized(16, 16));
    Widget::draw_item_icon(
        &mut target,
        Rect::new(0., 0., 16., 16.),
        &assets,
        &item,
        &Draw2D::default(),
        1,
    );
    assert_eq!(pixel(&target, 6, 7), [30, 0, 0, 255]);
}

#[test]
#[ignore = "Requires a native GPU adapter for Client/SceneHandler initialization"]
fn screen_button_state_and_animation_are_independent_and_deco_tracks_ticks() {
    let tile = Tile::from_textures(vec![mz_frame("#4080c0"), mz_frame("#d09030")]);
    let active = Tile::from_textures(vec![mz_frame("#b070d0"), mz_frame("#70b0d0")]);
    let mut assets = assets_with(&tile);
    assets.set_tiles(
        [(tile.id, tile.clone()), (active.id, active.clone())]
            .into_iter()
            .collect(),
    );
    let mut map = screen_map(tile.id, Some("button"));
    map.sectors[0].properties.set(
        "ceiling_source",
        Value::Source(PixelSource::TileId(active.id)),
    );
    assets.screens.insert("audit".into(), map);
    let mut client = Client::default();
    client.init_screen("audit".into(), &mut assets, &mut SceneHandler::default());
    let button = client.button_widgets.values_mut().next().unwrap();
    button.rect = Rect::new(0., 0., 16., 16.);
    let mut target = TheRGBABuffer::new(TheDim::sized(16, 16));
    button.update_draw(
        &mut target,
        &Map::default(),
        &assets,
        None,
        &Draw2D::default(),
        &1,
        ButtonVisualState::Selected,
        None,
    );
    assert_eq!(pixel(&target, 4, 5), active.textures[1].data[..4]);
    let (mut client, mut assets) = client_screen(&tile, "deco");
    client.inc_animation_frame();
    let deco = client.deco_widgets.values_mut().next().unwrap();
    deco.rect = Rect::new(0., 0., 16., 16.);
    deco.update_draw(&mut target, &Map::default(), &Default::default(), &assets);
    assert_eq!(pixel(&target, 4, 5), tile.textures[1].data[..4]);
    let mut rebaked = tile.clone();
    rebaked.textures[1] = mz_frame("#b070d0");
    assets.set_tiles([(tile.id, rebaked.clone())].into_iter().collect());
    deco.update_draw(&mut target, &Map::default(), &Default::default(), &assets);
    assert_eq!(pixel(&target, 4, 5), rebaked.textures[1].data[..4]);
}
