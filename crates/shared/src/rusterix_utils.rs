//! Compatibility adapters for the legacy Rusterix presentation owner.
use crate::prelude::*;
use crate::runtime_assets::{insert_bundled_ruleset_avatars, insert_bundled_ruleset_textures};
pub use crate::runtime_assets::{
    insert_content_into_maps, insert_content_into_maps_mode, insert_procedural_ruleset_icons,
    prepare_live_rules, sync_display_templates, sync_procedural_avatar_assets,
};
use rusterix::{Command, Rusterix};

pub fn start_server(
    rusterix: &mut Rusterix,
    project: &mut Project,
    debug: bool,
) -> Result<(), String> {
    let old = project
        .isometric_bakes
        .values()
        .map(|b| b.signature)
        .collect::<Vec<_>>();
    crate::runtime_assets::start_server(
        &mut rusterix.server,
        &mut rusterix.assets,
        project,
        debug,
    )?;
    if old
        != project
            .isometric_bakes
            .values()
            .map(|b| b.signature)
            .collect::<Vec<_>>()
    {
        rusterix.refresh_tile_textures(project.tiles.clone(), false);
    }
    rusterix.set_block_props(project.block_props.clone());
    rusterix.scene_handler.mark_dynamics_dirty();
    Ok(())
}

pub fn sync_item_icon_assets(rusterix: &mut Rusterix, project: &Project) {
    crate::runtime_assets::sync_item_icons(&mut rusterix.assets, project);
    rusterix.set_overlay_dirty();
}

pub fn sync_editor_visual_assets(rusterix: &mut Rusterix, project: &Project) {
    rusterix.assets.config = project.config.clone();
    rusterix.assets.rules = project.rules_source().unwrap_or_else(|_| String::new());
    rusterix.assets.palette = project.palette.clone();
    rusterix.assets.read_rules_metadata();

    rusterix.assets.avatars.clear();
    insert_bundled_ruleset_avatars(&mut rusterix.assets, project);
    for avatar in project.avatars.values() {
        rusterix
            .assets
            .avatars
            .insert(avatar.name.clone(), avatar.clone());
    }
    sync_item_icon_assets(rusterix, project);
    rusterix.set_tiles(project.tiles.clone(), true);
    sync_procedural_avatar_assets(&mut rusterix.assets, project);
}

pub fn sync_live_rules(rusterix: &mut Rusterix, project: &Project) -> Result<bool, String> {
    let Some(table) = prepare_live_rules(&mut rusterix.assets, project)? else {
        return Ok(false);
    };
    rusterix::server::publish_rules(table);
    // Creator stops regular region redraws while paused. Drain the edit now;
    // the paused region path handles messages without advancing gameplay.
    if rusterix.server.state == rusterix::ServerState::Paused {
        rusterix.server.redraw_tick();
    }
    rusterix.set_overlay_dirty();
    Ok(true)
}

pub fn warmup_runtime(rusterix: &mut Rusterix, project: &mut Project, ticks: usize) {
    for _ in 0..ticks {
        rusterix.server.system_tick();
        rusterix.server.redraw_tick();

        if let Some(new_region_name) = rusterix.update_server() {
            rusterix.client.current_map = new_region_name;
        }

        for region in &mut project.regions {
            rusterix.server.apply_entities_items(&mut region.map);
            if let Some(time) = rusterix.server.get_time(&region.map.id) {
                rusterix.client.set_server_time(time);
                project.time = time;
            }
        }
    }
}

pub fn setup_client(rusterix: &mut Rusterix, project: &mut Project) -> Vec<Command> {
    rusterix.assets.terrain_paint.clear();
    for region in &mut project.regions {
        crate::iso_paint_terrain::sync(&mut rusterix.assets, region.map.id, &mut region.iso_paint);
    }
    rusterix.assets.config = project.config.clone();
    rusterix.assets.world_source = project.world_source.clone();
    rusterix.assets.rules = project.rules_source().unwrap_or_else(|err| {
        eprintln!("Ruleset resolution error: {}", err);
        String::new()
    });
    rusterix.assets.read_rules_metadata();
    rusterix.assets.locales_src =
        crate::rulesets::resolve_project_locales(&project.config, &project.locales).unwrap_or_else(
            |err| {
                eprintln!("Ruleset locale resolution error: {}", err);
                project.locales.clone()
            },
        );
    rusterix.assets.audio_fx_src = project.audio_fx.clone();
    rusterix.assets.authoring_src = project.authoring.clone();
    rusterix.assets.region_sources.clear();
    rusterix.set_block_props(project.block_props.clone());
    rusterix.assets.read_locales();
    rusterix.assets.palette = project.palette.clone();
    rusterix.assets.palette_materials = project
        .palette_materials
        .iter()
        .map(|m| m.rmoe_values())
        .collect();
    rusterix.assets.palette_material_ids = project
        .palette_materials
        .iter()
        .map(|m| m.material_id())
        .collect();

    rusterix.assets.maps.clear();
    for region in &mut project.regions {
        region.map.name.clone_from(&region.name);
        rusterix
            .assets
            .maps
            .insert(region.map.name.clone(), region.map.clone());
        rusterix
            .assets
            .region_sources
            .insert(region.map.id, region.source.clone());
    }
    rusterix.assets.screens.clear();
    rusterix.assets.screen_settings.clear();
    for (_, screen) in &project.screens {
        let scr = screen.map.clone();
        let name = screen.map.name.clone();
        rusterix.assets.screens.insert(name.clone(), scr);
        rusterix
            .assets
            .screen_settings
            .insert(name, screen.settings.clone());
    }
    rusterix.assets.avatars.clear();
    insert_bundled_ruleset_avatars(&mut rusterix.assets, project);
    for avatar in project.avatars.values() {
        rusterix
            .assets
            .avatars
            .insert(avatar.name.clone(), avatar.clone());
    }
    insert_bundled_ruleset_textures(&mut rusterix.assets, project);
    sync_item_icon_assets(rusterix, project);
    sync_procedural_avatar_assets(&mut rusterix.assets, project);
    rusterix.assets.fonts.clear();
    rusterix.assets.audio.clear();
    for (_, asset) in project.assets.iter() {
        if let AssetBuffer::Font(bytes) = &asset.buffer {
            if let Ok(font) =
                fontdue::Font::from_bytes(bytes.clone(), fontdue::FontSettings::default())
            {
                rusterix.assets.fonts.insert(asset.name.clone(), font);
            }
        } else if let AssetBuffer::Audio(bytes) = &asset.buffer {
            rusterix
                .assets
                .audio
                .insert(asset.name.clone(), bytes.clone());
        }
    }
    // Client setup can swap maps/widgets while keeping SceneVM alive.
    // Invalidate dynamic caches so first game frame always reuploads lights/dynamics.
    rusterix.scene_handler.mark_dynamics_dirty();
    rusterix.setup_client()
}
