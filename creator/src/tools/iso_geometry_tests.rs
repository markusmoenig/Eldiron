use super::*;
use super::{linedef::LinedefTool, sector::SectorTool, vertex::VertexTool};
use crate::actions::{Action, edit_sector::EditSector};
use rusterix::{MapCamera, Value};

/// Exercise the real tool event handlers, including the named-area inspector and
/// the destination lookup used by goto(), on flat ground and shared terrain.
#[test]
#[ignore = "editor tool publication uses SceneVM's native Metal context"]
fn iso_geometry_tools_create_edit_and_name_goto_areas() {
    for terrain in [false, true] {
        let mut map = Map::default();
        map.camera = MapCamera::TwoDIso;
        map.grid_size = 40.;
        map.offset = Vec2::new(24., -16.);
        map.subdivisions = 2.;
        if terrain {
            map.properties.set("terrain_enabled", Value::Bool(true));
            map.properties.set("terrain_edge_falloff", Value::Float(0.));
            map.add_vertex_at(-8., -8.);
            map.add_vertex_at(8., 8.);
            let id = map.add_vertex_at(0., 0.);
            let v = map.find_vertex_mut(id).unwrap();
            v.z = 0.8;
            v.properties.set("terrain_control", Value::Bool(true));
            v.properties.set("smoothness", Value::Float(1.));
        }
        let size = Vec2::new(1000., 700.);
        let screen = |map: &Map, point| {
            ServerContext::map_grid_to_local(size, point, map).map(|v| v.round() as i32)
        };
        let mut ctx = TheContext::new(1000, 700, 1.);
        let mut ui = TheUI::new();
        let mut view = TheRenderView::new(TheId::named("PolyView"));
        view.set_dim(TheDim::sized(1000, 700), &mut ctx);
        ui.canvas.set_widget(view);
        let mut server = ServerContext::default();
        server.editor_view_mode = EditorViewMode::D2;
        server.pc = ProjectContext::Region(Uuid::new_v4());
        let mut line = LinedefTool::new();
        let corners = [
            Vec2::new(-2., -1.),
            Vec2::new(1., -1.),
            Vec2::new(1., 2.),
            Vec2::new(-2., 2.),
        ];
        for (i, point) in corners.into_iter().chain([corners[0]]).enumerate() {
            let q = screen(&map, point);
            line.map_event(
                MapEvent::MapHover(q),
                &mut ui,
                &mut ctx,
                &mut map,
                &mut server,
            );
            assert_eq!(server.hover_cursor, Some(point));
            let undo = line.map_event(
                MapEvent::MapClicked(q),
                &mut ui,
                &mut ctx,
                &mut map,
                &mut server,
            );
            assert_eq!(undo.is_some(), i != 0);
            line.map_event(MapEvent::MapUp(q), &mut ui, &mut ctx, &mut map, &mut server);
        }
        assert_eq!(map.sectors.len(), 1);
        assert!(map.curr_grid_pos.is_none());
        assert!(map.possible_polygon.is_empty());
        let id = map.sectors[0].id;
        assert_eq!(map.sectors[0].linedefs.len(), 4);
        let mut sector = SectorTool::new();
        let center = screen(&map, Vec2::new(-0.5, 0.5));
        sector.map_event(
            MapEvent::MapHover(center),
            &mut ui,
            &mut ctx,
            &mut map,
            &mut server,
        );
        assert_eq!(server.hover.2, Some(id));
        sector.map_event(
            MapEvent::MapClicked(center),
            &mut ui,
            &mut ctx,
            &mut map,
            &mut server,
        );
        sector.map_event(
            MapEvent::MapUp(center),
            &mut ui,
            &mut ctx,
            &mut map,
            &mut server,
        );
        assert_eq!(map.selected_sectors, vec![id]);
        let mut action = EditSector::new();
        assert!(action.is_applicable(&map, &mut ctx, &server));
        action.load_params(&map);
        action.handle_event(
            &TheEvent::ValueChanged(
                TheId::named("actionSectorName"),
                TheValue::Text("Meeting place".into()),
            ),
            &mut Project::default(),
            &mut ui,
            &mut ctx,
            &mut server,
        );
        assert!(
            action
                .apply(&mut map, &mut ui, &mut ctx, &mut server)
                .is_some()
        );
        let destination = map.named_area_center_3d("Meeting place").unwrap();
        assert!(
            (Vec2::new(destination.x, destination.z) - Vec2::new(-0.5, 0.5)).magnitude() < 0.001
        );
        let saved: Map = serde_json::from_str(&serde_json::to_string(&map).unwrap()).unwrap();
        assert_eq!(
            saved.named_area_center_3d("Meeting place"),
            Some(destination)
        );

        // A selected line drags along the iso grid and records normal map undo.
        map.selected_sectors.clear();
        let midpoint = screen(&map, Vec2::new(-0.5, -1.));
        line.map_event(
            MapEvent::MapHover(midpoint),
            &mut ui,
            &mut ctx,
            &mut map,
            &mut server,
        );
        assert!(server.hover.1.is_some(), "edge picking, terrain={terrain}");
        line.map_event(
            MapEvent::MapClicked(midpoint),
            &mut ui,
            &mut ctx,
            &mut map,
            &mut server,
        );
        assert_eq!(map.selected_linedefs.len(), 1);
        let end = screen(&map, Vec2::new(0., -1.));
        line.map_event(
            MapEvent::MapDragged(end),
            &mut ui,
            &mut ctx,
            &mut map,
            &mut server,
        );
        let undo = line
            .map_event(
                MapEvent::MapUp(end),
                &mut ui,
                &mut ctx,
                &mut map,
                &mut server,
            )
            .unwrap();
        let ProjectUndoAtom::MapEdit(_, before, after) = undo else {
            panic!("Expected map undo")
        };
        assert_ne!(before.vertices, after.vertices);
        map = *before;
        map.selected_linedefs.clear();

        let mut vertex = VertexTool::new();
        let q = screen(&map, corners[0]);
        vertex.map_event(
            MapEvent::MapHover(q),
            &mut ui,
            &mut ctx,
            &mut map,
            &mut server,
        );
        assert!(server.hover.0.is_some());
        vertex.map_event(
            MapEvent::MapClicked(q),
            &mut ui,
            &mut ctx,
            &mut map,
            &mut server,
        );
        let end = screen(&map, corners[0] + Vec2::new(-0.5, 0.));
        vertex.map_event(
            MapEvent::MapDragged(end),
            &mut ui,
            &mut ctx,
            &mut map,
            &mut server,
        );
        assert!(
            vertex
                .map_event(
                    MapEvent::MapUp(end),
                    &mut ui,
                    &mut ctx,
                    &mut map,
                    &mut server
                )
                .is_some()
        );
        let selected = map.selected_vertices[0];
        assert_eq!(
            map.get_vertex(selected),
            Some(corners[0] + Vec2::new(-0.5, 0.))
        );

        // Selection rectangle is screen-aligned, independent of iso grid axes.
        map.selected_vertices.clear();
        let a = Vec2::new(350, 230);
        let b = Vec2::new(690, 460);
        vertex.map_event(
            MapEvent::MapHover(a),
            &mut ui,
            &mut ctx,
            &mut map,
            &mut server,
        );
        assert!(server.hover.0.is_none());
        vertex.map_event(
            MapEvent::MapClicked(a),
            &mut ui,
            &mut ctx,
            &mut map,
            &mut server,
        );
        vertex.map_event(
            MapEvent::MapDragged(b),
            &mut ui,
            &mut ctx,
            &mut map,
            &mut server,
        );
        assert_eq!(
            map.curr_rectangle,
            Some((a.map(|v| v as f32), b.map(|v| v as f32)))
        );
        assert!(map.selected_vertices.contains(&selected));
        vertex.map_event(MapEvent::MapUp(b), &mut ui, &mut ctx, &mut map, &mut server);
        assert!(map.curr_rectangle.is_none());
    }
}
