//! Authoring controls shared by the compact mesh inspector and iso tile brush.
use crate::editor::UNDOMANAGER;
use crate::prelude::*;
use rusterix::{MapCamera, PixelSource, Value};

pub fn placement_canvas() -> TheCanvas {
    let mut form = TheTextLayout::new(TheId::named("Iso Placement Controls"));
    form.limiter_mut().set_max_height(270);
    form.set_margin(Vec4::new(8, 8, 8, 8));
    for (id, label, default, range) in [
        ("Iso Placement Height", "Top height", 0., 0.0..=32.0),
        ("Iso Placement Step", "Block height", 0.25, 0.05..=8.0),
    ] {
        let mut input = TheTextLineEdit::new(TheId::named(id));
        input.set_range(TheValue::RangeF32(range));
        input.set_value(TheValue::Float(default));
        input.set_status_text(if id.ends_with("Height") {
            "Paint and select on this elevation"
        } else {
            "Thickness of each tile when filling a column"
        });
        form.add_pair(label.into(), Box::new(input));
    }
    for (id, label, default) in [
        ("Iso Placement Walkable", "Walkable top", false),
        ("Iso Placement Stack", "Fill below", false),
        ("Iso Placement Bounds", "Follow recipe shape", false),
        ("Iso Placement Snap", "Snap connectors", true),
    ] {
        let mut check = TheCheckButton::new(TheId::named(id));
        check.set_value(TheValue::Bool(default));
        if id.ends_with("Bounds") {
            check.set_status_text("Use the MZ Footprint contour for collision and support, or mesh bounds when none is defined. Recipe edits keep the anchor fixed.");
        }
        form.add_pair(label.into(), Box::new(check));
    }
    let mut pick = TheTraybarButton::new(TheId::named("Iso Pick Surface"));
    pick.set_text("Pick surface height".into());
    form.add_pair(String::new(), Box::new(pick));
    let mut detach = TheTraybarButton::new(TheId::named("Iso Detach Connection"));
    detach.set_text("Detach connection".into());
    form.add_pair(String::new(), Box::new(detach));
    let mut canvas = TheCanvas::new();
    canvas.set_layout(form);
    canvas
}

/// Selecting a placement adopts its mode; an explicit brush choice then remains
/// active until the selection changes, including when the selected shape is custom.
pub fn sync_bounds_selection(map: &mut Map, previous: &mut Option<(Uuid, Vec<u32>)>) {
    let selection = (map.id, map.selected_sectors.clone());
    if previous.as_ref() != Some(&selection) {
        if let Some(s) = map
            .selected_sectors
            .first()
            .and_then(|id| map.find_sector(*id))
        {
            let enabled = s
                .properties
                .get_bool_default("iso_follow_mesh_bounds", false);
            map.properties
                .set("iso_brush_follow_mesh_bounds", Value::Bool(enabled));
        }
        *previous = Some(selection);
    }
}

pub fn sync(ui: &mut TheUI, project: &Project, server: &ServerContext, ctx: &mut TheContext) {
    let map = project
        .get_map(server)
        .filter(|m| m.camera == MapCamera::TwoDIso);
    if let Some(layout) = ui.get_layout("Iso Placement Controls") {
        let height = if map.is_some() { 270 } else { 0 };
        if layout.limiter().get_max_height() != height {
            layout.limiter_mut().set_max_height(height);
            ctx.ui.relayout = true;
        }
    }
    let Some(map) = map else {
        return;
    };
    for (name, value) in [
        (
            "Iso Placement Snap",
            TheValue::Bool(map.properties.get_bool_default("iso_brush_snap", true)),
        ),
        (
            "Iso Placement Height",
            TheValue::Float(map.properties.get_float_default("iso_edit_height", 0.)),
        ),
        (
            "Iso Placement Step",
            TheValue::Float(map.properties.get_float_default("iso_step_height", 0.25)),
        ),
        (
            "Iso Placement Walkable",
            TheValue::Bool(map.properties.get_bool_default("iso_brush_walkable", false)),
        ),
        (
            "Iso Placement Bounds",
            TheValue::Bool(
                map.properties
                    .get_bool_default("iso_brush_follow_mesh_bounds", false),
            ),
        ),
        (
            "Iso Placement Stack",
            TheValue::Bool(map.properties.get_bool_default("iso_brush_stack", false)),
        ),
    ] {
        if ctx.ui.focus.as_ref().is_some_and(|id| id.name == name) {
            continue;
        }
        if let Some(w) = ui.get_widget(name) {
            if w.value() != value {
                w.set_value(value);
            }
        }
    }
}

pub fn handle_event(
    event: &TheEvent,
    project: &mut Project,
    server: &ServerContext,
    ctx: &mut TheContext,
) -> bool {
    if matches!(event,TheEvent::StateChanged(id,TheWidgetState::Clicked) if id.name=="Iso Detach Connection")
    {
        if let Some(map) = project.get_map_mut(server) {
            let before = map.clone();
            let linked = map.selected_sectors.iter().any(|id| {
                map.find_sector(*id)
                    .is_some_and(|s| s.properties.get_int("iso_snap_target").is_some())
            });
            for id in map.selected_sectors.clone() {
                if let Some(s) = map.find_sector_mut(id) {
                    shared::isometric::snapping::detach(s);
                }
            }
            if linked {
                map.changed = map.changed.wrapping_add(1);
                UNDOMANAGER.write().unwrap().add_undo(
                    crate::undo::project_atoms::ProjectUndoAtom::MapEdit(
                        server.pc,
                        Box::new(before),
                        Box::new(map.clone()),
                    ),
                    ctx,
                );
                ctx.ui.redraw_all = true;
            }
        }
        return true;
    }
    if matches!(event,TheEvent::StateChanged(id,TheWidgetState::Clicked) if id.name=="Iso Pick Surface")
    {
        if let Some(map) = project.get_map_mut(server) {
            map.properties.set("iso_pick_height", Value::Bool(true));
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                "Click a visible tile top to choose its editing height".into(),
            ));
        }
        return true;
    }
    if let TheEvent::ValueChanged(id, value) = event {
        if id.name == "Iso Placement Bounds" {
            let enabled = matches!(value, TheValue::Bool(true));
            let before = project
                .get_map(server)
                .filter(|m| m.camera == MapCamera::TwoDIso)
                .cloned();
            if let Some(before) = before {
                if let Some(map) = project.get_map_mut(server) {
                    map.properties
                        .set("iso_brush_follow_mesh_bounds", Value::Bool(enabled));
                    for id in map.selected_sectors.clone() {
                        if let Some(s) = map.find_sector_mut(id) {
                            if (3..=denrim_materials::geometry::footprint::MAX_POINTS)
                                .contains(&s.linedefs.len())
                            {
                                s.properties
                                    .set("iso_follow_mesh_bounds", Value::Bool(enabled));
                            }
                        }
                    }
                    map.changed = map.changed.wrapping_add(1);
                }
                shared::isometric::placement::synchronize(project);
                let after = project.get_map(server).unwrap().clone();
                if !before.selected_sectors.is_empty() {
                    UNDOMANAGER.write().unwrap().add_undo(
                        crate::undo::project_atoms::ProjectUndoAtom::MapEdit(
                            server.pc,
                            Box::new(before.clone()),
                            Box::new(after.clone()),
                        ),
                        ctx,
                    );
                }
                crate::utils::editor_scene_apply_map_edit(project, server, &before, &after);
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Soft Update Minimap"),
                    TheValue::Empty,
                ));
                ctx.ui.redraw_all = true;
            }
            return true;
        }
    }
    let TheEvent::ValueChanged(id, value) = event else {
        return false;
    };
    let (key, value) = match id.name.as_str() {
        "Iso Placement Height" | "Iso Placement Step" => {
            let Some(number) = value.to_f32().filter(|n| n.is_finite()) else {
                return true;
            };
            if id.name.ends_with("Height") {
                ("iso_edit_height", Value::Float(number.clamp(0., 32.)))
            } else {
                ("iso_step_height", Value::Float(number.clamp(0.05, 8.)))
            }
        }
        "Iso Placement Snap" => (
            "iso_brush_snap",
            Value::Bool(matches!(value, TheValue::Bool(true))),
        ),
        "Iso Placement Walkable" => (
            "iso_brush_walkable",
            Value::Bool(matches!(value, TheValue::Bool(true))),
        ),
        "Iso Placement Stack" => (
            "iso_brush_stack",
            Value::Bool(matches!(value, TheValue::Bool(true))),
        ),
        _ => return false,
    };
    if let Some(map) = project
        .get_map_mut(server)
        .filter(|m| m.camera == MapCamera::TwoDIso)
    {
        map.properties.set(key, value);
        map.curr_rectangle = None;
        map.changed = map.changed.wrapping_add(1);
        crate::editor::RUSTERIX.write().unwrap().set_dirty();
        ctx.ui.redraw_all = true;
    }
    true
}

pub fn pick_surface(map: &mut Map, size: Vec2<f32>, pixel: Vec2<f32>) {
    let projection = rusterix::isometric::Projection::editor(map, size);
    let base = projection.ground(pixel);
    let picked = map
        .sectors
        .iter()
        .filter_map(|s| {
            let h = s.properties.get_float_default("floor_height", 0.);
            let p = base + Vec2::new(-1.2247449, 1.2247449) * h;
            s.is_inside(map, p).then_some((
                p.y - p.x + h * 0.8164966,
                h,
                s.properties.get_bool_default("iso_walkable", false),
            ))
        })
        .max_by(|a, b| a.0.total_cmp(&b.0));
    let terrain =
        rusterix::chunkbuilder::terrain_surface::TerrainSurface::cached(map).and_then(|t| {
            t.raycast(
                Vec3::new(base.x, 0., base.y),
                Vec3::new(-1.2247449, 1., 1.2247449),
                t.max_height,
            )
        });
    let picked = terrain
        .map(|p| (p.z - p.x + p.y * 0.8164966, p.y, true))
        .into_iter()
        .chain(picked)
        .max_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((_, h, walkable)) = picked {
        map.properties.set("iso_edit_height", Value::Float(h));
        map.properties
            .set("iso_brush_walkable", Value::Bool(walkable));
    }
    map.properties.set("iso_pick_height", Value::Bool(false));
    map.curr_rectangle = None;
    map.changed = map.changed.wrapping_add(1);
}

/// A cell at an explicit level, with independent rings for stacked placements.
/// Used by the normal paint stroke, so preview, live updates and undo stay shared.
/// The normal Rect placement path, also used by the reproducible tutorial project.
pub fn paint_cell_with_tiles(
    map: &mut Map,
    vertices: [Vec2<f32>; 4],
    source: Option<PixelSource>,
    erase: bool,
    mode: i32,
    tiles: &indexmap::IndexMap<Uuid, rusterix::Tile>,
) -> bool {
    let bitmap_ground = source.as_ref().and_then(|source| match source {
        PixelSource::TileId(id) => tiles.get(id).and_then(|t|t.isometric.as_ref()),
        _ => None,
    }).is_some_and(|s|s.bitmap && s.ground);
    let snap = map.properties.get_bool_default("iso_brush_follow_mesh_bounds",false)
        && map.properties.get_bool_default("iso_brush_snap",true)
        && !map.properties.get_bool_default("iso_brush_stack",false);
    if !snap {return paint_cell_kind(map,vertices,source,erase,mode,bitmap_ground);}
    let before: std::collections::HashSet<_> = map.sectors.iter().map(|s|s.id).collect();
    let changed=paint_cell_kind(map,vertices,source,erase,mode,bitmap_ground);
    if changed && !erase {
        let created=map.sectors.iter().filter(|s|!before.contains(&s.id)).map(|s|s.id).collect::<Vec<_>>();
        if !created.is_empty() {
            for id in created {shared::isometric::snapping::snap(map,tiles,id);}
            shared::isometric::placement::update_map(map,tiles);
        }
    }
    changed
}

#[cfg(test)]
pub fn paint_cell(
    map: &mut Map,
    vertices: [Vec2<f32>; 4],
    source: Option<PixelSource>,
    erase: bool,
    mode: i32,
) -> bool {
    paint_cell_kind(map,vertices,source,erase,mode,false)
}
fn paint_cell_kind(map:&mut Map,vertices:[Vec2<f32>;4],source:Option<PixelSource>,erase:bool,mode:i32,bitmap_ground:bool)->bool {
    let min = vertices
        .iter()
        .fold(Vec2::<f32>::broadcast(f32::INFINITY), |a, p| {
            a.map2(*p, f32::min)
        });
    let max = min + 1.;
    let height = map
        .properties
        .get_float_default("iso_edit_height", 0.)
        .clamp(0., 32.);
    let walkable = bitmap_ground || map.properties.get_bool_default("iso_brush_walkable", false);
    let fill = map.properties.get_bool_default("iso_brush_stack", false);
    let step = map
        .properties
        .get_float_default("iso_step_height", 0.25)
        .clamp(0.05, 8.);
    let follow_bounds = map
        .properties
        .get_bool_default("iso_brush_follow_mesh_bounds", false);
    let at_cell = |map: &Map| {
        map.sectors
            .iter()
            .filter(|s| {
                if s.properties
                    .get_bool_default("iso_follow_mesh_bounds", false)
                {
                    return shared::isometric::placement::anchor(map, s).is_some_and(|at| {
                        at.x.floor() == min.x.floor() && at.y.floor() == min.y.floor()
                    });
                }
                let b = s.bounding_box(map);
                (b.min - min).magnitude_squared() < 0.0001
                    && (b.max - max).magnitude_squared() < 0.0001
            })
            .map(|s| s.id)
            .collect::<Vec<_>>()
    };
    if erase {
        let target = at_cell(map)
            .into_iter()
            .filter(|id| {
                (map.find_sector(*id)
                    .unwrap()
                    .properties
                    .get_float_default("floor_height", 0.)
                    - height)
                    .abs()
                    < 0.001
            })
            .last();
        let Some(id) = target else {
            return false;
        };
        // Other levels own separate geometry; remove only this placement.
        let s = map.find_sector(id).unwrap().clone();
        let promote = s.properties.get_bool_default("iso_walkable", false);
        let edges = s
            .linedefs
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        let used_elsewhere = map
            .linedefs
            .iter()
            .filter(|e| !edges.contains(&e.id))
            .flat_map(|e| [e.start_vertex, e.end_vertex])
            .collect::<std::collections::HashSet<_>>();
        let vertices = map
            .linedefs
            .iter()
            .filter(|e| edges.contains(&e.id))
            .flat_map(|e| [e.start_vertex, e.end_vertex])
            .filter(|v| !used_elsewhere.contains(v))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        map.delete_elements(&vertices, &s.linedefs, &[id]);
        if promote {
            let top = at_cell(map).into_iter().max_by(|a, b| {
                map.find_sector(*a)
                    .unwrap()
                    .properties
                    .get_float_default("floor_height", 0.)
                    .total_cmp(
                        &map.find_sector(*b)
                            .unwrap()
                            .properties
                            .get_float_default("floor_height", 0.),
                    )
            });
            if let Some(id) = top {
                map.find_sector_mut(id)
                    .unwrap()
                    .properties
                    .set("iso_walkable", Value::Bool(true));
            }
        }
        map.changed = map.changed.wrapping_add(1);
        return true;
    }
    let Some(source) = source else {
        return false;
    };
    let yaw = map.properties.get_float_default("iso_brush_yaw", 0.);
    let levels = if fill && height > 0. {
        (height / step).ceil().max(1.) as usize
    } else {
        1
    };
    let mut changed = false;
    for level in 1..=levels {
        let h = if fill {
            (level as f32 * step).min(height)
        } else {
            height
        };
        let top_walkable = walkable && level == levels;
        let existing = at_cell(map)
            .into_iter()
            .filter(|id| {
                (map.find_sector(*id)
                    .unwrap()
                    .properties
                    .get_float_default("floor_height", 0.)
                    - h)
                    .abs()
                    < 0.001
            })
            .last();
        if let Some(id) = existing {
            let sector = map.find_sector_mut(id).unwrap();
            let same = sector.properties.get_default_source() == Some(&source);
            if !same {
                shared::isometric::snapping::detach(sector);
            }
            if same || mode == 0 {
                let prev = sector.properties.clone();
                if bitmap_ground {changed |= sector.layer != Some(0);sector.layer=Some(0);}
                sector
                    .properties
                    .set("source", Value::Source(source.clone()));
                sector
                    .properties
                    .set("iso_walkable", Value::Bool(top_walkable));
                sector.properties.set("iso_yaw", Value::Float(yaw));
                sector
                    .properties
                    .set("iso_follow_mesh_bounds", Value::Bool(follow_bounds));
                changed |= prev != sector.properties;
                continue;
            }
        }
        map.possible_polygon.clear();
        let v = vertices.map(|p| {
            let id = map.find_free_vertex_id().unwrap();
            map.vertices.push(rusterix::Vertex::new(id, p.x, p.y));
            id
        });
        for i in 0..4 {
            map.create_linedef_manual(v[i], v[(i + 1) % 4]);
        }
        if let Some(id) = map.close_polygon_manual() {
            let yaw = map.properties.get_float_default("iso_brush_yaw", 0.);
            let s = map.find_sector_mut(id).unwrap();
            s.name = "Iso tile".into();
            s.layer = Some(if bitmap_ground{0}else{1});
            s.properties.set("rect", Value::Bool(true));
            s.properties.set("source", Value::Source(source.clone()));
            s.properties.set("floor_height", Value::Float(h));
            s.properties.set("iso_walkable", Value::Bool(top_walkable));
            s.properties.set("iso_yaw", Value::Float(yaw));
            s.properties
                .set("iso_follow_mesh_bounds", Value::Bool(follow_bounds));
            map.selected_sectors = vec![id];
            changed = true;
        }
    }
    if changed {
        map.changed = map.changed.wrapping_add(1);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imported_ground_uses_native_walkable_placement_on_create_and_replace() {
        let mut map=Map::default();map.camera=MapCamera::TwoDIso;
        let mut tile=rusterix::Tile::from_texture(rusterix::Texture::alloc(2,4));
        tile.isometric=Some(rusterix::isometric::SpriteLayout{bitmap:true,ground:true,..Default::default()});
        let source=Some(PixelSource::TileId(tile.id));let tiles=[(tile.id,tile)].into_iter().collect();
        assert!(paint_cell_with_tiles(&mut map,cell(0.,0.),source.clone(),false,0,&tiles));
        assert_eq!(map.sectors[0].layer,Some(0));assert!(map.sectors[0].properties.get_bool_default("iso_walkable",false));
        map.sectors[0].layer=Some(1);map.sectors[0].properties.set("iso_walkable",Value::Bool(false));map.selected_sectors.clear();
        assert!(paint_cell_with_tiles(&mut map,cell(0.,0.),source.clone(),false,0,&tiles));
        assert_eq!(map.sectors.len(),1);assert_eq!(map.sectors[0].layer,Some(0));assert!(map.sectors[0].properties.get_bool_default("iso_walkable",false));
    }
    fn cell(x: f32, z: f32) -> [Vec2<f32>; 4] {
        [
            Vec2::new(x, z),
            Vec2::new(x + 1., z),
            Vec2::new(x + 1., z + 1.),
            Vec2::new(x, z + 1.),
        ]
    }
    fn map() -> Map {
        let mut map = Map::default();
        map.camera = MapCamera::TwoDIso;
        map.properties.set("iso_brush_walkable", Value::Bool(true));
        map
    }

    #[test]
    fn iso_stacking_reuses_source_at_distinct_heights_and_erases_only_active_level() {
        let mut map = map();
        let source = PixelSource::TileId(Uuid::new_v4());
        map.properties.set("iso_edit_height", Value::Float(2.));
        map.properties.set("iso_brush_stack", Value::Bool(true));
        assert!(paint_cell(
            &mut map,
            cell(0., 0.),
            Some(source.clone()),
            false,
            0
        ));
        assert_eq!(map.sectors.len(), 8);
        assert_eq!(
            map.sectors
                .iter()
                .filter(|s| s.properties.get_bool_default("iso_walkable", false))
                .count(),
            1
        );
        assert!(!paint_cell(
            &mut map,
            cell(0., 0.),
            Some(source.clone()),
            false,
            0
        ));
        let mini = map.as_mini(&Default::default());
        assert_eq!(mini.elevation.height_at(Vec2::new(0.5, 0.5)), 2.);
        assert!(paint_cell(&mut map, cell(0., 0.), None, true, 0));
        assert_eq!(map.sectors.len(), 7);
        assert_eq!(
            map.as_mini(&Default::default())
                .elevation
                .height_at(Vec2::new(0.5, 0.5)),
            1.75
        );
        assert!(!paint_cell(&mut map, cell(0., 0.), None, true, 0));
        map.properties.set("iso_edit_height", Value::Float(0.5));
        map.properties.set("iso_brush_stack", Value::Bool(false));
        let before = map.sectors.len();
        assert!(
            !paint_cell(&mut map, cell(0., 0.), Some(source), false, 0)
                || map.sectors.len() == before
        );
        assert_eq!(map.sectors.len(), before);
    }

    #[test]
    fn iso_height_controls_build_walkable_stairs_and_pick_visible_upper_top() {
        let mut map = map();
        map.properties.set("iso_brush_stack", Value::Bool(true));
        let source = PixelSource::TileId(Uuid::new_v4());
        for x in 1..=8 {
            map.properties
                .set("iso_edit_height", Value::Float(x as f32 * 0.25));
            assert!(paint_cell(
                &mut map,
                cell(x as f32, 0.),
                Some(source.clone()),
                false,
                0
            ));
        }
        let mini = map.as_mini(&Default::default());
        let (p, blocked) = mini.elevation.move_distance(
            Vec3::new(0.5, 0., 0.5),
            Vec2::new(8., 0.),
            0.22,
            0.35,
            0.5,
        );
        assert!(!blocked);
        assert_eq!(p.y, 2.);
        let (p, blocked) = mini
            .elevation
            .move_distance(p, Vec2::new(-8., 0.), 0.22, 0.35, 0.5);
        assert!(!blocked);
        assert_eq!(p.y, 0.);
        let size = Vec2::new(1000., 700.);
        let pixel =
            rusterix::isometric::Projection::editor(&map, size).screen(Vec2::new(8.5, 0.5), 2.);
        map.properties.set("iso_edit_height", Value::Float(0.));
        pick_surface(&mut map, size, pixel);
        assert_eq!(map.properties.get_float_default("iso_edit_height", 0.), 2.);
        let server = ServerContext::default();
        let picked = server.local_to_map_cell(size, pixel, &map, 1.);
        assert_eq!(picked, Vec2::new(8., 0.));
        let selected = server
            .geometry_at_for_tool(size, pixel, &map, shared::context::MapToolType::Sector)
            .2
            .unwrap();
        assert_eq!(
            map.find_sector(selected)
                .unwrap()
                .properties
                .get_float_default("floor_height", 0.),
            2.
        );
        let selection = server.geometry_in_screen_rectangle(
            size,
            pixel - Vec2::broadcast(20.),
            pixel + Vec2::broadcast(20.),
            &map,
        );
        assert!(!selection.2.is_empty());
        assert!(selection.2.iter().all(|id| {
            map.find_sector(*id)
                .unwrap()
                .properties
                .get_float_default("floor_height", 0.)
                == 2.
        }));
        let copy = shared::project_io::decode_project(
            &shared::project_io::encode_project(&{
                let mut p = Project::default();
                p.regions[0].map = map;
                p
            })
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            copy.regions[0]
                .map
                .as_mini(&Default::default())
                .elevation
                .height_at(Vec2::new(8.5, 0.5)),
            2.
        );
    }
    #[test]
    fn auto_footprint_paint_replace_rotate_move_and_erase_use_the_placement_anchor() {
        let mut map = map();
        map.properties
            .set("iso_brush_follow_mesh_bounds", Value::Bool(true));
        let mut tile = rusterix::Tile::from_texture(rusterix::Texture::new(vec![255; 4], 1, 1));
        let source = PixelSource::TileId(tile.id);
        tile.isometric = Some(rusterix::isometric::SpriteLayout {
            placement_contours: (0..4)
                .map(|direction| {
                    [
                        [0., -0.06],
                        [2.1, -0.06],
                        [2.1, 0.06],
                        [0.1, 0.06],
                        [0.1, 0.5],
                        [0., 0.5],
                    ]
                    .map(|p| {
                        let (s, c) = (direction as f32 * std::f32::consts::FRAC_PI_2).sin_cos();
                        [c * p[0] + s * p[1], -s * p[0] + c * p[1]]
                    })
                    .to_vec()
                })
                .collect(),
            directions: 4,
            frames_per_direction: 1,
            placement_bounds: vec![
                [[0., -0.06], [2.1, 0.06]],
                [[-0.06, -2.1], [0.06, 0.]],
                [[-2.1, -0.06], [0., 0.06]],
                [[-0.06, 0.], [0.06, 2.1]],
            ],
            ..Default::default()
        });
        let tiles = [(tile.id, tile)].into_iter().collect();
        assert!(paint_cell(
            &mut map,
            cell(0., 0.),
            Some(source.clone()),
            false,
            0
        ));
        assert!(shared::isometric::placement::update_map(&mut map, &tiles));
        let id = map.sectors[0].id;
        assert!((map.sectors[0].bounding_box(&map).max.x - 2.6).abs() < 1e-5);
        assert!(!paint_cell(
            &mut map,
            cell(0., 0.),
            Some(source.clone()),
            false,
            0
        ));
        assert_eq!(map.sectors.len(), 1);
        map.properties.set("iso_brush_yaw", Value::Float(90.));
        assert!(paint_cell(&mut map, cell(0., 0.), Some(source), false, 0));
        assert!(shared::isometric::placement::update_map(&mut map, &tiles));
        assert_eq!(map.sectors[0].id, id);
        assert!((map.sectors[0].bounding_box(&map).min.y + 1.6).abs() < 1e-5);
        for v in &mut map.vertices {
            v.x += 3.;
            v.y += 2.;
        }
        assert!(!paint_cell(&mut map, cell(0., 0.), None, true, 0));
        assert!(paint_cell(&mut map, cell(3., 2.), None, true, 0));
        assert!(map.sectors.is_empty());
    }
    #[test]
    fn explicit_footprint_brush_choice_survives_sync_for_an_existing_selection() {
        let mut map = map();
        let source = PixelSource::TileId(Uuid::new_v4());
        paint_cell(&mut map, cell(0., 0.), Some(source.clone()), false, 0);
        let mut previous = None;
        sync_bounds_selection(&mut map, &mut previous);
        assert!(
            !map.properties
                .get_bool_default("iso_brush_follow_mesh_bounds", false)
        );
        map.properties
            .set("iso_brush_follow_mesh_bounds", Value::Bool(true));
        sync_bounds_selection(&mut map, &mut previous);
        assert!(
            map.properties
                .get_bool_default("iso_brush_follow_mesh_bounds", false)
        );
        paint_cell(&mut map, cell(2., 0.), Some(source), false, 0);
        assert!(
            map.sectors
                .last()
                .unwrap()
                .properties
                .get_bool_default("iso_follow_mesh_bounds", false)
        );
        sync_bounds_selection(&mut map, &mut previous);
        assert!(
            map.properties
                .get_bool_default("iso_brush_follow_mesh_bounds", false)
        );
        map.selected_sectors = vec![map.sectors[0].id];
        sync_bounds_selection(&mut map, &mut previous);
        assert!(
            !map.properties
                .get_bool_default("iso_brush_follow_mesh_bounds", false)
        );
    }
}
