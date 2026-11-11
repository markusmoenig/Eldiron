//! Paint uses the shared Tile Picker; this toolbar owns application settings only.
use crate::prelude::*;
pub const MODE: &str = "Paint Picker Mode";
const EDIT: &str = "Paint Picker Edit";
pub(crate) fn layer<'a>(
    project: &'a mut Project,
    server: &ServerContext,
) -> Option<&'a mut IsoPaintLayer> {
    if let ProjectContext::Prefab(id) = server.pc {
        Some(project.block_prop_paint.entry(id).or_default())
    } else {
        project
            .get_region_mut(&server.curr_region)
            .map(|r| &mut r.iso_paint)
    }
}
pub fn pick(id: Uuid, project: &mut Project, server: &ServerContext) -> bool {
    let Some(tile) = project
        .tiles
        .get(&id)
        .filter(|t| !t.textures.is_empty() && !project.procedural_appearances.contains_key(&id))
        .cloned()
    else {
        return false;
    };
    let Some(paint) = layer(project, server) else {
        return false;
    };
    paint.active_source = Some(id);
    paint
        .source_images
        .insert(id, std::sync::Arc::new(tile.clone()));
    paint.source_revision = None;
    if let Some(layout) = &tile.paint {
        paint.active_brush = if layout.stamp || layout.generator.is_some() {
            layout.brush.clone()
        } else {
            "material".into()
        };
        paint.active_brush_shape = if layout.stamp || layout.generator.is_some() {
            layout.shape.clone()
        } else {
            "solid".into()
        };
        paint.active_material_id = layout.material_id;
        paint.active_size = layout.size;
        paint.active_opacity = layout.opacity;
        paint.active_material_mode = if layout.stamp { "stamp" } else { "coat" }.into();
    } else {
        paint.active_brush = "material".into();
        paint.active_brush_shape = "solid".into();
        paint.active_material_mode = "coat".into();
    }
    paint.apply_source_settings();
    paint.active_operation = "draw".into();
    true
}
// Settings are drawn by PaintOverlay in the viewport, not mounted in the app chrome.
pub fn toolbar(
    _ui: &mut TheUI,
    ctx: &mut TheContext,
    project: &mut Project,
    server: &ServerContext,
) {
    if let Some(paint) = layer(project, server) {
        paint.apply_source_settings();
    }
    ctx.ui.redraw_all = true;
}

fn publish(
    ui: &mut TheUI,
    ctx: &mut TheContext,
    project: &mut Project,
    server: &ServerContext,
    spec: shared::paint_source::Spec,
) -> Result<(), String> {
    let id = layer(project, server)
        .and_then(|p| p.active_source)
        .ok_or("Select a Paint recipe")?;
    let (before, after) = crate::procedural_material::paint_assets::update_spec(project, id, spec)?;
    if before != after {
        crate::editor::UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::ProceduralMaterialEdit(id, before, after),
            ctx,
        );
        crate::utils::editor_scene_refresh_material_textures(project, server);
        let assets = crate::editor::RUSTERIX.read().unwrap();
        if let Some(paint) = layer(project, server) {
            paint.refresh_sources(&assets.assets);
            paint.apply_source_settings();
        }
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tilepicker Textures"),
            TheValue::Empty,
        ));
    }
    toolbar(ui, ctx, project, server);
    Ok(())
}

pub fn activate(
    ui: &mut TheUI,
    ctx: &mut TheContext,
    project: &mut Project,
    server: &mut ServerContext,
) {
    server.curr_action_id = None;
    if let Err(error) = crate::procedural_material::paint_assets::install(project) {
        ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
    }
    let current = layer(project, server).and_then(|p| p.active_source);
    let selected = current.or_else(|| {
        project
            .tiles
            .iter()
            .find(|(_, t)| {
                t.paint
                    .as_ref()
                    .is_some_and(|p| !p.stamp && p.brush == "moss")
            })
            .map(|(id, _)| *id)
    });
    if let Some(id) = selected {
        if current.is_none() {
            pick(id, project, server);
        }
        server.curr_tile_id = Some(id);
        server.curr_tile_source = Some(rusterix::TileSource::SingleTile(id));
    }
    crate::editor::DOCKMANAGER
        .write()
        .unwrap()
        .set_dock("Tiles".into(), ui, ctx, project, server);
    crate::utils::editor_scene_refresh_material_textures(project, server);
    ctx.ui.send(TheEvent::Custom(
        TheId::named("Update Tilepicker"),
        TheValue::Empty,
    ));
    let collection_name = selected
        .and_then(|id| project.tiles.get(&id))
        .and_then(|tile| tile.paint.as_ref())
        .map_or("Paint Brushes", |source| {
            if source.stamp {
                "Organic Stamps"
            } else {
                "Paint Brushes"
            }
        });
    let collection = project
        .tile_collections
        .values()
        .find(|c| c.name == collection_name)
        .map(|c| c.id);
    if let Some(id) = collection {
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Reveal Tile Collection"),
            TheValue::Id(id),
        ));
    }
    toolbar(ui, ctx, project, server);
}
pub fn event(
    event: &TheEvent,
    ui: &mut TheUI,
    ctx: &mut TheContext,
    project: &mut Project,
    server: &mut ServerContext,
) -> bool {
    if server.curr_map_tool_type != MapToolType::IsoPaint {
        return false;
    }
    let source = layer(project, server).and_then(|p| p.active_source);
    let mut spec =
        source.and_then(|id| crate::procedural_material::paint_assets::selected_spec(project, id));
    let mut source_edit = false;
    match event {
        TheEvent::ValueChanged(id, value) if id.name.starts_with("Paint Overlay ") => {
            let Some(number) = value.to_f32().filter(|v| v.is_finite()) else {
                return true;
            };
            let key = id.name.trim_start_matches("Paint Overlay ");
            if let Some(spec) = spec.as_mut() {
                match key {
                    "Scale" => {
                        spec.scale = number.clamp(0.25, 4.);
                        source_edit = true;
                    }
                    "Mortar" => {
                        spec.mortar = number.clamp(0., 0.4);
                        source_edit = true;
                    }
                    "Detail" => {
                        spec.detail = number.clamp(0., 1.);
                        source_edit = true;
                    }
                    "Variation" => {
                        spec.variation = number.clamp(0., 1.);
                        source_edit = true;
                    }
                    _ => {}
                }
            }
            if !source_edit {
                if let Some(paint) = layer(project, server) {
                    match key {
                        "Size" => paint.active_size = number.clamp(0.05, 16.),
                        "Opacity" => paint.active_opacity = number.clamp(0., 1.),
                        "Density" => paint.active_stamp_density = number.clamp(0., 1.),
                        "Jitter" => paint.active_stamp_size_jitter = number.clamp(0., 1.),
                        "Rotate" => paint.active_stamp_rotation_jitter = number.clamp(0., 1.),
                        _ => {}
                    }
                }
            }
        }
        TheEvent::IndexChanged(id, index)
            if id.name == "Paint Overlay Pattern" || id.name == "Paint Overlay Clip" =>
        {
            if id.name == "Paint Overlay Pattern" {
                if let Some(spec) = spec.as_mut() {
                    spec.pattern = ["tiles", "bricks", "arch"][(*index).min(2)].into();
                    source_edit = true;
                }
            } else if let Some(paint) = layer(project, server) {
                paint.active_clip = if *index == 1 { "surface" } else { "none" }.into();
            }
        }
        TheEvent::PaletteIndexChanged(id, index)
            if id.name.starts_with("Paint Overlay Palette ") =>
        {
            if source.unwrap_or_default() != id.uuid {
                return true;
            }
            if let Some(slot) = id
                .name
                .trim_start_matches("Paint Overlay Palette ")
                .parse::<usize>()
                .ok()
            {
                if let Some(spec) = spec.as_mut() {
                    if slot < spec.colors.len() {
                        spec.palette.resize(spec.colors.len(), None);
                        spec.palette[slot] = Some(*index);
                        source_edit = true;
                    }
                } else if source.is_none() {
                    if let Some(color) = project
                        .palette
                        .colors
                        .get(*index as usize)
                        .and_then(Option::as_ref)
                        .map(|c| c.to_u8_array())
                    {
                        if let Some(paint) = layer(project, server) {
                            paint.active_color = color;
                            paint.active_palette_colors = vec![color];
                            paint.active_palette_indices = vec![*index];
                        }
                    }
                }
            }
            ui.clear_popover(ctx);
        }
        TheEvent::ContextMenuSelected(id, item) if id.name == "Paint Overlay Choices" => {
            let parts: Vec<_> = item.name.split_whitespace().collect();
            if let Some(index) = parts.last().and_then(|s| s.parse::<usize>().ok()) {
                let key = parts.get(3).copied().unwrap_or("");
                if let Some(spec) = spec.as_mut() {
                    match key {
                        "Material" => {
                            spec.material_id = (index.min(14) * 4) as u8 + spec.material_id % 4;
                            source_edit = true;
                        }
                        "Finish" => {
                            spec.material_id = (spec.material_id / 4) * 4 + index.min(3) as u8;
                            source_edit = true;
                        }
                        "Shape" => {
                            spec.shape = [
                                "solid", "soft", "dirt", "speckle", "jagged", "scratch", "wash",
                            ][index.min(6)]
                            .into();
                            source_edit = true;
                        }
                        _ => {}
                    }
                } else if source.is_none() {
                    if let Some(paint) = layer(project, server) {
                        match key {
                            "Material" => {
                                paint.active_material_id =
                                    (index.min(14) * 4) as u8 + paint.active_material_id % 4
                            }
                            "Finish" => {
                                paint.active_material_id =
                                    (paint.active_material_id / 4) * 4 + index.min(3) as u8
                            }
                            "Shape" => {
                                paint.active_brush_shape = [
                                    "solid", "soft", "dirt", "speckle", "jagged", "scratch", "wash",
                                ][index.min(6)]
                                .into()
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        TheEvent::Custom(id, TheValue::Text(command)) if id.name == "Paint Overlay Command" => {
            match command.as_str() {
                "Edit" => {
                    if let Some(id) = source {
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Open Procedural Tile"),
                            TheValue::Id(id),
                        ));
                    }
                }
                "Fill" | "Clear" => {
                    return self::event(
                        &TheEvent::StateChanged(
                            TheId::named(if command == "Fill" {
                                "Paint Picker Fill"
                            } else {
                                "Paint Picker Clear"
                            }),
                            TheWidgetState::Clicked,
                        ),
                        ui,
                        ctx,
                        project,
                        server,
                    );
                }
                "Visible" => {
                    if let Some(p) = layer(project, server) {
                        p.visible = !p.visible;
                    }
                }
                "Coat" | "Replace" => {
                    if let Some(p) = layer(project, server) {
                        p.active_material_mode = command.to_lowercase();
                    }
                }
                _ => {}
            }
        }
        _ => return legacy_event(event, ui, ctx, project, server),
    }
    if source_edit {
        if let Some(spec) = spec {
            if let Err(error) = publish(ui, ctx, project, server, spec) {
                ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
            }
        }
    }
    toolbar(ui, ctx, project, server);
    true
}

fn legacy_event(
    event: &TheEvent,
    ui: &mut TheUI,
    ctx: &mut TheContext,
    project: &mut Project,
    server: &mut ServerContext,
) -> bool {
    match event {
        TheEvent::Custom(id, _)
            if id.name == "Paint Picker Refresh"
                || id.name == "Update Tilepicker Textures"
                || id.name == "Procedural Tiles Restored" =>
        {
            toolbar(ui, ctx, project, server);
            return true;
        }
        TheEvent::Custom(id, TheValue::Id(tile)) if id.name == "Tile Picked" => {
            if pick(*tile, project, server) {
                toolbar(ui, ctx, project, server);
                return true;
            }
        }
        TheEvent::StateChanged(id, TheWidgetState::Clicked)
            if id.name == "Paint Picker Fill" || id.name == "Paint Picker Clear" =>
        {
            let context = server.pc;
            let Some(paint) = layer(project, server) else {
                return false;
            };
            let before = paint.clone();
            let changed = if id.name == "Paint Picker Fill" {
                paint.fill_active_selection()
            } else {
                paint.clear_active_selection()
            };
            if changed {
                let atom = if let ProjectContext::Prefab(asset) = context {
                    ProjectUndoAtom::PrefabPaintEdit(
                        context,
                        asset,
                        Box::new(before),
                        Box::new(paint.clone()),
                    )
                } else {
                    ProjectUndoAtom::RegionPaintEdit(
                        context,
                        server.curr_region,
                        Box::new(before),
                        Box::new(paint.clone()),
                    )
                };
                crate::editor::UNDOMANAGER
                    .write()
                    .unwrap()
                    .add_undo(atom, ctx);
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Soft Update Minimap"),
                    TheValue::Empty,
                ));
                ctx.ui.redraw_all = true;
            }
            return true;
        }
        TheEvent::StateChanged(id, TheWidgetState::Clicked) if id.name == EDIT => {
            if let Some(id) = layer(project, server).and_then(|p| p.active_source) {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Open Procedural Tile"),
                    TheValue::Id(id),
                ));
            }
            return true;
        }
        TheEvent::IndexChanged(id, index) if id.name == "Paint Picker Blend" => {
            if let Some(paint) = layer(project, server) {
                paint.active_material_mode = if *index == 1 { "replace" } else { "coat" }.into();
            }
            return true;
        }
        TheEvent::IndexChanged(id, index) if id.name == MODE => {
            if *index == 1 {
                let source = layer(project, server).and_then(|p| p.active_source);
                if !source
                    .and_then(|id| project.tiles.get(&id))
                    .and_then(|t| t.paint.as_ref())
                    .is_some_and(|p| p.stamp)
                {
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        fl!("paint_picker_choose_stamp"),
                    ));
                    toolbar(ui, ctx, project, server);
                    return true;
                }
            }
            let Some(paint) = layer(project, server) else {
                return false;
            };
            paint.active_operation = match index {
                2 => "erase",
                3 => "pick",
                4 => "select",
                _ => "draw",
            }
            .into();
            if *index <= 1 {
                paint.active_material_mode = if *index == 1 { "stamp" } else { "coat" }.into();
            }
            toolbar(ui, ctx, project, server);
            return true;
        }
        TheEvent::ValueChanged(id, value) if id.name.starts_with("Paint Picker ") => {
            let number = match value {
                TheValue::Text(s) => s.parse::<f32>().ok(),
                _ => value.to_f32(),
            };
            let Some(number) = number.filter(|v| v.is_finite()) else {
                return true;
            };
            if let Some(paint) = layer(project, server) {
                match id.name.as_str() {
                    "Paint Picker Size" => paint.active_size = number.clamp(0.05, 16.),
                    "Paint Picker Opacity" => paint.active_opacity = number.clamp(0., 1.),
                    "Paint Picker Density" => paint.active_stamp_density = number.clamp(0., 1.),
                    "Paint Picker Jitter" => paint.active_stamp_size_jitter = number.clamp(0., 1.),
                    "Paint Picker Rotate" => {
                        paint.active_stamp_rotation_jitter = number.clamp(0., 1.)
                    }
                    _ => {}
                }
            }
            return true;
        }
        _ => {}
    }
    false
}

pub fn deactivate(_ui: &mut TheUI, ctx: &mut TheContext) {
    ctx.ui.redraw_all = true;
}
