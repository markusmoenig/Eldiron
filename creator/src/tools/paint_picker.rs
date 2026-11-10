//! Paint uses the shared Tile Picker; this toolbar owns application settings only.
use crate::prelude::*;
pub const MODE: &str = "Paint Picker Mode";
const EDIT: &str = "Paint Picker Edit";
fn layer<'a>(project: &'a mut Project, server: &ServerContext) -> Option<&'a mut IsoPaintLayer> {
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
        paint.active_brush = if layout.stamp {
            layout.brush.clone()
        } else {
            "material".into()
        };
        paint.active_brush_shape = if layout.stamp {
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
    paint.active_operation = "draw".into();
    true
}
pub fn toolbar(
    ui: &mut TheUI,
    ctx: &mut TheContext,
    project: &mut Project,
    server: &ServerContext,
) {
    let Some(paint) = layer(project, server) else {
        return;
    };
    let source = paint.active_source;
    let picked_color = paint.active_color;
    let stamp = paint.active_material_mode == "stamp";
    let selected = match paint.active_operation.as_str() {
        "erase" => 2,
        "pick" => 3,
        "select" => 4,
        _ if stamp => 1,
        _ => 0,
    };
    let size = paint.active_size;
    let opacity = paint.active_opacity;
    let density = paint.active_stamp_density;
    let jitter = paint.active_stamp_size_jitter;
    let rotation = paint.active_stamp_rotation_jitter;
    let name = source.and_then(|id| project.tiles.get(&id)).map_or_else(
        || {
            format!(
                "{} #{:02X}{:02X}{:02X}",
                fl!("iso_paint_color"),
                picked_color[0],
                picked_color[1],
                picked_color[2]
            )
        },
        |t| t.alias.clone(),
    );
    let editable = source.is_some_and(|id| project.procedural_materials.contains_key(&id));
    let Some(bar) = ui.get_hlayout("Game Tool Params") else {
        return;
    };
    bar.clear();
    bar.set_margin(Vec4::new(5, 1, 5, 1));
    bar.set_padding(4);
    let mut modes = TheGroupButton::new(TheId::named(MODE));
    for text in [
        fl!("paint_picker_paint"),
        fl!("paint_picker_scatter"),
        fl!("iso_paint_operation_erase"),
        fl!("iso_paint_operation_pick"),
        fl!("iso_paint_operation_select"),
    ] {
        modes.add_text(text);
    }
    modes.set_item_width(50);
    modes.set_index(selected);
    bar.add_widget(Box::new(modes));
    if !stamp {
        let mut blend = TheGroupButton::new(TheId::named("Paint Picker Blend"));
        blend.add_text(fl!("iso_paint_material_mode_coat"));
        blend.add_text(fl!("iso_paint_material_mode_replace"));
        blend.set_item_width(48);
        blend.set_index(
            if layer(project, server).is_some_and(|p| p.active_material_mode == "replace") {
                1
            } else {
                0
            },
        );
        bar.add_widget(Box::new(blend));
    }
    let mut name_label = TheText::new(TheId::named("Paint Picker Source"));
    name_label.set_text(name);
    name_label.limiter_mut().set_max_width(100);
    bar.add_widget(Box::new(name_label));
    for (id, label, value, min, max) in [
        (
            "Size",
            fl!("paint_picker_size"),
            size,
            0.05,
            if stamp { 8. } else { 16. },
        ),
        ("Opacity", fl!("iso_paint_opacity"), opacity, 0., 1.),
    ]
    .into_iter()
    .chain(stamp.then_some(("Density", fl!("iso_paint_stamp_density"), density, 0., 1.)))
    .chain(stamp.then_some(("Jitter", fl!("paint_picker_jitter"), jitter, 0., 1.)))
    .chain(stamp.then_some(("Rotate", fl!("paint_picker_rotate"), rotation, 0., 1.)))
    {
        let mut text = TheText::new(TheId::empty());
        text.set_text(label);
        text.limiter_mut().set_max_width(55);
        bar.add_widget(Box::new(text));
        let mut field = TheTextLineEdit::new(TheId::named(&format!("Paint Picker {id}")));
        field.set_value(TheValue::Float(value));
        field.set_range(TheValue::RangeF32(min..=max));
        field.limiter_mut().set_max_width(44);
        bar.add_widget(Box::new(field));
    }
    if selected == 4 {
        let mut fill = TheTraybarButton::new(TheId::named("Paint Picker Fill"));
        fill.set_text(fl!("iso_paint_fill_selection"));
        bar.add_widget(Box::new(fill));
        let mut clear = TheTraybarButton::new(TheId::named("Paint Picker Clear"));
        clear.set_text(fl!("iso_paint_clear_selection"));
        bar.add_widget(Box::new(clear));
    }
    let mut edit = TheTraybarButton::new(TheId::named(EDIT));
    edit.set_text(fl!("paint_picker_edit"));
    edit.set_disabled(!editable);
    bar.add_widget(Box::new(edit));
    ctx.ui.relayout = true;
    ctx.ui.redraw_all = true;
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
                    .is_some_and(|p| !p.stamp && p.brush == "material" && p.material_id == 0)
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
        .map_or("Paint materials", |source| {
            if source.stamp {
                "Organic stamps"
            } else if source.brush == "material" {
                "Paint materials"
            } else {
                "Paint brushes"
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
    match event {
        TheEvent::Custom(id, _) if id.name == "Paint Picker Refresh" => {
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
