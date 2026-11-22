use super::*;
use rusterix::{
    MapCamera, Value,
    map::light::{Light, LightType},
};

fn sector(region: &mut Region, x: f32, z: f32) -> u32 {
    let map = &mut region.map;
    let corners = [[x, z], [x + 1., z], [x + 1., z + 1.], [x, z + 1.]];
    let mut created = None;
    let ids = corners.map(|[x, z]| map.add_vertex_at(x, z));
    for (from, to) in ids.into_iter().zip(ids.into_iter().cycle().skip(1)) {
        if let (_, Some(id)) = map.create_linedef(from, to) {
            created = Some(id);
        }
    }
    created.unwrap()
}

#[test]
#[ignore = "Creator context requires a native GPU adapter"]
fn iso_attached_sector_drag_offsets_building_and_undoes_as_one_edit() {
    let mut region = Region::default();
    region.map = Map::default();
    region.map.camera = MapCamera::TwoDIso;
    region.map.grid_size = 40.;
    let footprint = sector(&mut region, 0., 0.);
    let roof = sector(&mut region, 0., 1.);
    let road = sector(&mut region, 1., 0.);
    let owner = region.map.find_sector(footprint).unwrap().creator_id;
    let part = region.map.find_sector_mut(roof).unwrap();
    part.properties.set(ISO_SECTOR_OWNER, Value::Id(owner));
    part.properties.set("iso_anchor", Value::Vec2([0.5, 1.5]));
    let mut door = shared::item::Item::new();
    let door_id = door.id;
    let door_start = Vec3::new(0.5, 0.25, 1.);
    door.position = door_start;
    door.data = format!("[placement]\n{ISO_SECTOR_OWNER} = \"{owner}\"\n");
    region.items.insert(door_id, door);
    let mut npc = shared::character::Character::new();
    let npc_id = npc.id;
    let npc_start = Vec3::new(0.4, 0.25, 0.4);
    npc.position = npc_start;
    npc.data = format!("[placement]\n{ISO_SECTOR_OWNER} = \"{owner}\"\n");
    region.characters.insert(npc_id, npc);
    let light_start = Vec3::new(0.4, 2., 0.6);
    let mut light = Light::new(LightType::Point).with_position(light_start);
    light.properties.set(ISO_SECTOR_OWNER, Value::Id(owner));
    region.map.lights.push(light);

    let mut server = CreatorContext::default();
    server.shared.editor_view_mode = EditorViewMode::D2;
    server.shared.curr_region = region.id;
    server.shared.pc = ProjectContext::Region(region.id);
    let mut tool = SectorTool::new();
    let mut ui = TheUI::new();
    let mut ctx = TheContext::new(1000, 700, 1.);
    let mut view = TheRenderView::new(TheId::named("PolyView"));
    view.set_dim(TheDim::sized(1000, 700), &mut ctx);
    ui.canvas.set_widget(view);
    let road_geometry = region
        .map
        .find_sector(road)
        .unwrap()
        .vertices_world(&region.map);
    let old_edges = region.map.linedefs.len();

    let down = Vec2::new(580, 420);
    let offset = Vec2::new(2., -1.);
    let travel = Vec2::new(40, -60);
    server.shared.hover.2 = Some(roof);
    ui.ctrl = true;
    assert!(
        tool.region_map_event(
            MapClicked(down),
            &mut ui,
            &mut ctx,
            &mut region,
            &mut server
        )
        .is_none()
    );
    ui.ctrl = false;
    assert_eq!(region.map.selected_sectors, vec![footprint]);
    for point in [down + travel, down, down + travel] {
        assert!(
            tool.region_map_event(
                MapDragged(point),
                &mut ui,
                &mut ctx,
                &mut region,
                &mut server
            )
            .is_none()
        );
        if point == down {
            assert_eq!(region.map.linedefs.len(), old_edges);
            assert_eq!(region.items[&door_id].position, door_start);
        }
    }
    let atom = tool
        .region_map_event(
            MapUp(down + travel),
            &mut ui,
            &mut ctx,
            &mut region,
            &mut server,
        )
        .unwrap();
    let ProjectUndoAtom::RegionEdit(_, before, after) = &atom else {
        panic!("expected one undo entry for the region and its placements");
    };
    assert_eq!(before.items[&door_id].position, door_start);
    let delta = Vec3::new(offset.x, 0., offset.y);
    assert_eq!(after.items[&door_id].position, door_start + delta);
    assert_eq!(after.characters[&npc_id].position, npc_start + delta);
    assert_eq!(after.map.lights[0].position(), light_start + delta);
    assert_eq!(
        after
            .map
            .find_sector(roof)
            .unwrap()
            .properties
            .get_vec2("iso_anchor"),
        Some([2.5, 0.5])
    );
    assert_eq!(
        after
            .map
            .find_sector(road)
            .unwrap()
            .vertices_world(&after.map),
        road_geometry
    );
    let mut project = Project::default();
    project.regions = vec![region];
    atom.undo(&mut project, &mut ui, &mut ctx, &mut server);
    assert_eq!(project.regions[0].items[&door_id].position, door_start);
    assert_eq!(project.regions[0].map.linedefs.len(), old_edges);
    atom.redo(&mut project, &mut ui, &mut ctx, &mut server);
    assert_eq!(
        project.regions[0].items[&door_id].position,
        door_start + delta
    );
}
