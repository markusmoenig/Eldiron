use crate::prelude::*;
use rusterix::{PixelSource, Value};
use std::str::FromStr;

pub const EDIT_VERTEX_ACTION_ID: &str = "260fcd81-c456-4ad4-894c-85e7552c856f";

pub struct EditVertex {
    id: TheId,
    nodeui: TheNodeUI,
}

impl EditVertex {
    fn build_nodeui() -> TheNodeUI {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        nodeui.add_item(TheNodeUIItem::Text(
            "actionVertexName".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));

        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionVertexX".into(),
            "".into(),
            "".into(),
            0.0,
            0.0..=0.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionVertexY".into(),
            "".into(),
            "".into(),
            0.0,
            0.0..=0.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionVertexZ".into(),
            "".into(),
            "".into(),
            0.0,
            0.0..=0.0,
            false,
        ));

        nodeui.add_item(TheNodeUIItem::OpenTree("billboard".into()));
        nodeui.add_item(TheNodeUIItem::Icons(
            "actionTile".into(),
            "".into(),
            "".into(),
            vec![(
                TheRGBABuffer::new(TheDim::sized(36, 36)),
                "".to_string(),
                Uuid::nil(),
            )],
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionSize".into(),
            "".into(),
            "".into(),
            1.0,
            0.0..=0.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("Terrain".into()));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionTerrainControl".into(),
            "Terrain control".into(),
            "Shape the shared smooth terrain with this vertex. Y is the hill height.".into(),
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionTerrainRadius".into(),
            "Influence radius".into(),
            "Outer radius of the smooth hill, in world units".into(),
            4.,
            0.25..=128.,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui
    }

    fn geometry_object_local_point(
        object: &rusterix::GeometryObject,
        world: Vec3<f32>,
    ) -> Vec3<f32> {
        let m = object.transform;
        let translation = Vec3::new(m[3][0], m[3][1], m[3][2]);
        let rel = world - translation;
        let axis_x = Vec3::new(m[0][0], m[0][1], m[0][2]);
        let axis_y = Vec3::new(m[1][0], m[1][1], m[1][2]);
        let axis_z = Vec3::new(m[2][0], m[2][1], m[2][2]);
        let project = |axis: Vec3<f32>| {
            let len_sq = axis.dot(axis);
            if len_sq > 1e-6 {
                rel.dot(axis) / len_sq
            } else {
                0.0
            }
        };
        Vec3::new(project(axis_x), project(axis_y), project(axis_z))
    }

    fn selected_geometry_vertex_world(map: &Map) -> Option<(usize, usize, Vec3<f32>)> {
        let (object_id, vertex_index) = *map.selected_geometry_vertices.first()?;
        let object_index = map
            .geometry_objects
            .iter()
            .position(|object| object.id == object_id)?;
        let object = map.geometry_objects.get(object_index)?;
        let vertex = *object.vertices.get(vertex_index)?;
        Some((object_index, vertex_index, object.transform_point(vertex)))
    }

    fn set_geometry_vertex_world(map: &mut Map, world: Vec3<f32>) -> bool {
        let Some((object_index, vertex_index, _)) = Self::selected_geometry_vertex_world(map)
        else {
            return false;
        };
        let Some(object) = map.geometry_objects.get_mut(object_index) else {
            return false;
        };
        if vertex_index >= object.vertices.len() {
            return false;
        }

        let local = Self::geometry_object_local_point(object, world);
        if (object.vertices[vertex_index] - local).magnitude_squared() <= 0.000001 {
            return false;
        }
        object.vertices[vertex_index] = local;
        true
    }
}

impl Action for EditVertex {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named_with_id(
                &fl!("action_edit_vertex"),
                Uuid::from_str(EDIT_VERTEX_ACTION_ID).unwrap(),
            ),
            nodeui: Self::build_nodeui(),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_edit_vertex_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            map.selected_vertices.len() == 1
        } else {
            map.selected_geometry_vertices.len() == 1
        }
    }

    fn load_params(&mut self, map: &Map) {
        if let Some((_, _, world)) = Self::selected_geometry_vertex_world(map) {
            self.nodeui
                .set_text_value("actionVertexName", String::new());
            self.nodeui.set_text_value("actionTileId", String::new());
            self.nodeui.set_f32_value("actionSize", 1.0);
            if let Some(item) = self.nodeui.get_item_mut("actionTile")
                && let TheNodeUIItem::Icons(_, _, _, items) = item
                && items.len() == 1
            {
                items[0].2 = Uuid::nil();
            }

            self.nodeui.set_f32_value("actionVertexX", world.x);
            self.nodeui.set_f32_value("actionVertexY", world.y);
            self.nodeui.set_f32_value("actionVertexZ", world.z);
            return;
        }

        if let Some(vertex_id) = map.selected_vertices.first()
            && let Some(vertex) = map.find_vertex(*vertex_id)
        {
            self.nodeui.set_bool_value(
                "actionTerrainControl",
                vertex.properties.get_bool_default("terrain_control", false),
            );
            self.nodeui.set_f32_value(
                "actionTerrainRadius",
                vertex.properties.get_float_default("smoothness", 1.) * 4.,
            );
            self.nodeui
                .set_text_value("actionVertexName", vertex.name.clone());
            self.nodeui.set_f32_value(
                "actionSize",
                vertex.properties.get_float_default("source_size", 1.0),
            );

            let billboard_tile_id = if let Some(Value::Source(PixelSource::TileId(id))) =
                vertex.properties.get("source")
            {
                *id
            } else {
                Uuid::nil()
            };
            self.nodeui.set_text_value(
                "actionTileId",
                if billboard_tile_id == Uuid::nil() {
                    String::new()
                } else {
                    billboard_tile_id.to_string()
                },
            );
            if let Some(item) = self.nodeui.get_item_mut("actionTile")
                && let TheNodeUIItem::Icons(_, _, _, items) = item
                && items.len() == 1
            {
                items[0].2 = billboard_tile_id;
            }

            let world = map
                .get_vertex_3d(*vertex_id)
                .unwrap_or_else(|| vertex.as_vec3_world());
            self.nodeui.set_f32_value("actionVertexX", world.x);
            self.nodeui.set_f32_value("actionVertexY", world.y);
            self.nodeui.set_f32_value("actionVertexZ", world.z);
        }
    }

    fn load_params_project(&mut self, project: &Project, server_ctx: &mut ServerContext) {
        let mut tile_icon = TheRGBABuffer::new(TheDim::sized(36, 36));
        let mut tile_id = Uuid::nil();

        if let Some(map) = project.get_map(server_ctx)
            && let Some(vertex_id) = map.selected_vertices.first()
            && let Some(vertex) = map.find_vertex(*vertex_id)
            && let Some(Value::Source(PixelSource::TileId(id))) = vertex.properties.get("source")
            && let Some(tile) = project.tiles.get(id)
            && !tile.is_empty()
        {
            tile_icon = tile.textures[0].to_rgba();
            tile_id = *id;
        }

        if let Some(item) = self.nodeui.get_item_mut("actionTile")
            && let TheNodeUIItem::Icons(_, _, _, items) = item
            && items.len() == 1
        {
            items[0].0 = tile_icon;
            items[0].2 = tile_id;
        }
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let mut changed = false;
        let prev = map.clone();

        let name = self
            .nodeui
            .get_text_value("actionVertexName")
            .unwrap_or_default();
        let tile_size = self.nodeui.get_f32_value("actionSize").unwrap_or(1.0);
        let tile_id = self
            .nodeui
            .get_tile_id("actionTile", 0)
            .unwrap_or(Uuid::nil());
        let tile_id_text = self
            .nodeui
            .get_text_value("actionTileId")
            .unwrap_or_default();
        let source = parse_tile_id_pixelsource(&tile_id_text)
            .or_else(|| (tile_id != Uuid::nil()).then_some(PixelSource::TileId(tile_id)));

        let x = self.nodeui.get_f32_value("actionVertexX").unwrap_or(0.0);
        let y = self.nodeui.get_f32_value("actionVertexY").unwrap_or(0.0);
        let z = self.nodeui.get_f32_value("actionVertexZ").unwrap_or(0.0);
        let terrain = self
            .nodeui
            .get_bool_value("actionTerrainControl")
            .unwrap_or(false);
        let radius = self
            .nodeui
            .get_f32_value("actionTerrainRadius")
            .unwrap_or(4.)
            .clamp(0.25, 128.);

        if !map.selected_geometry_vertices.is_empty() {
            let world = Vec3::new(x, y, z);
            changed |= Self::set_geometry_vertex_world(map, world);
        } else if let Some(vertex_id) = map.selected_vertices.first()
            && let Some(vertex) = map.find_vertex_mut(*vertex_id)
        {
            if vertex.properties.get_bool_default("terrain_control", false) != terrain {
                vertex
                    .properties
                    .set("terrain_control", Value::Bool(terrain));
                changed = true;
            }
            if terrain && vertex.properties.get_float_default("smoothness", 1.) != radius / 4. {
                vertex
                    .properties
                    .set("smoothness", Value::Float(radius / 4.));
                changed = true;
            }
            let existing_tile_size = vertex.properties.get_float_default("source_size", 1.0);
            if existing_tile_size != tile_size {
                vertex
                    .properties
                    .set("source_size", Value::Float(tile_size));
                changed = true;
            }

            if let Some(source) = source {
                let has_changed = match vertex.properties.get("source") {
                    Some(Value::Source(existing)) => *existing != source,
                    _ => true,
                };
                if has_changed {
                    vertex.properties.set("source", Value::Source(source));
                    changed = true;
                }
            } else if vertex.properties.contains("source") {
                vertex.properties.remove("source");
                changed = true;
            }

            if name != vertex.name {
                vertex.name = name;
                changed = true;
            }
            if x != vertex.x {
                vertex.x = x;
                changed = true;
            }
            // World space to vertex space mapping.
            if y != vertex.z {
                vertex.z = y;
                changed = true;
            }
            if z != vertex.y {
                vertex.y = z;
                changed = true;
            }
        }

        if changed {
            if terrain && map.selected_geometry_vertices.is_empty() {
                if !map.properties.get_bool_default("terrain_enabled", false) {
                    map.properties.set("terrain_enabled", Value::Bool(true));
                    if !map.properties.contains("terrain_edge_falloff") {
                        map.properties.set("terrain_edge_falloff", Value::Float(0.));
                    }
                    if !map.properties.contains("terrain_subdivisions") {
                        map.properties.set("terrain_subdivisions", Value::Int(4));
                    }
                }
                if !map.properties.contains("default_terrain_tile") {
                    if let Some(id) = server_ctx.curr_tile_id {
                        map.properties.set(
                            "default_terrain_tile",
                            Value::Source(PixelSource::TileId(id)),
                        );
                    }
                }
            }
            map.changed = map.changed.wrapping_add(1);
            Some(ProjectUndoAtom::MapEdit(
                server_ctx.pc,
                Box::new(prev),
                Box::new(map.clone()),
            ))
        } else {
            None
        }
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        if let TheEvent::TileDropped(id, tile_id, index) = event
            && id.name == "actionTile"
            && let Some(item) = self.nodeui.get_item_mut(&id.name)
            && let TheNodeUIItem::Icons(_, _, _, items) = item
            && *index < items.len()
            && let Some(tile) = project.tiles.get(tile_id)
            && !tile.is_empty()
        {
            items[*index].0 = tile.textures[0].to_rgba();
            items[*index].2 = *tile_id;
            self.nodeui
                .set_text_value("actionTileId", tile_id.to_string());
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Update Action List"),
                TheValue::Empty,
            ));
            return true;
        }
        self.nodeui.handle_event(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vertex_action_authors_shared_smooth_terrain_with_height_radius_and_undo_snapshot() {
        let mut map = Map::default();
        map.camera = rusterix::MapCamera::TwoDIso;
        map.vertices = vec![
            rusterix::Vertex::new(0, -8., -8.),
            rusterix::Vertex::new(1, 8., 8.),
            rusterix::Vertex::new(2, 0.5, 0.5),
        ];
        map.selected_vertices = vec![2];
        let mut action = EditVertex::new();
        action.load_params(&map);
        action.nodeui.set_bool_value("actionTerrainControl", true);
        action.nodeui.set_f32_value("actionVertexY", 1.5);
        action.nodeui.set_f32_value("actionTerrainRadius", 6.);
        let mut server = ServerContext::default();
        let material = Uuid::new_v4();
        server.curr_tile_id = Some(material);
        let undo = action
            .apply(
                &mut map,
                &mut TheUI::new(),
                &mut TheContext::new(64, 64, 1.),
                &mut server,
            )
            .unwrap();
        let ProjectUndoAtom::MapEdit(_, before, after) = undo else {
            panic!("Expected a normal map edit");
        };
        assert!(!before.properties.get_bool_default("terrain_enabled", false));
        assert!(after.properties.get_bool_default("terrain_enabled", false));
        assert_eq!(
            map.find_vertex(2)
                .unwrap()
                .properties
                .get_float_default("smoothness", 0.),
            1.5
        );
        assert_eq!(
            map.properties.get_source("default_terrain_tile"),
            Some(&PixelSource::TileId(material))
        );
        let terrain =
            rusterix::chunkbuilder::terrain_surface::TerrainSurface::from_map(&map).unwrap();
        assert!((terrain.height_at(Vec2::new(0.5, 0.5)).unwrap() - 1.5).abs() < 0.0001);
    }

    #[test]
    fn edit_vertex_is_applicable_to_single_geometry_vertex_in_3d() {
        let mut map = Map::default();
        let object = rusterix::GeometryObject::box_("Box", Vec3::zero(), Vec3::new(1.0, 1.0, 1.0));
        let object_id = object.id;
        map.geometry_objects.push(object);
        map.selected_geometry_vertices.push((object_id, 0));

        let mut ctx = TheContext::new(64, 64, 1.0);
        let mut server_ctx = ServerContext::default();
        server_ctx.editor_view_mode = EditorViewMode::Iso;

        assert!(EditVertex::new().is_applicable(&map, &mut ctx, &server_ctx));
    }

    #[test]
    fn edit_vertex_moves_selected_geometry_vertex_in_world_coordinates() {
        let mut map = Map::default();
        let mut object =
            rusterix::GeometryObject::box_("Box", Vec3::zero(), Vec3::new(1.0, 1.0, 1.0));
        object.transform[3][0] = 10.0;
        object.transform[3][1] = 2.0;
        object.transform[3][2] = -4.0;
        let object_id = object.id;
        map.geometry_objects.push(object);
        map.selected_geometry_vertices.push((object_id, 0));

        let mut action = EditVertex::new();
        action.load_params(&map);
        assert_eq!(action.nodeui.get_f32_value("actionVertexX"), Some(9.5));
        assert_eq!(action.nodeui.get_f32_value("actionVertexY"), Some(1.5));
        assert_eq!(action.nodeui.get_f32_value("actionVertexZ"), Some(-4.5));
        action.nodeui.set_f32_value("actionVertexX", 12.0);
        action.nodeui.set_f32_value("actionVertexY", 3.0);
        action.nodeui.set_f32_value("actionVertexZ", -1.0);

        let mut ui = TheUI::default();
        let mut ctx = TheContext::new(64, 64, 1.0);
        let mut server_ctx = ServerContext::default();
        server_ctx.pc = ProjectContext::Region(Uuid::new_v4());

        let Some(ProjectUndoAtom::MapEdit(_, old_map, new_map)) =
            action.apply(&mut map, &mut ui, &mut ctx, &mut server_ctx)
        else {
            panic!("editing a geometry vertex should return a MapEdit undo atom");
        };

        let moved = map.geometry_objects[0].transform_point(map.geometry_objects[0].vertices[0]);
        assert_eq!(moved, Vec3::new(12.0, 3.0, -1.0));
        assert_ne!(
            old_map.geometry_objects[0].vertices[0],
            new_map.geometry_objects[0].vertices[0]
        );
    }

    #[test]
    fn edit_vertex_loads_2d_vertex_with_world_coordinate_order() {
        let mut map = Map::default();
        let id = map.add_vertex_at_3d(0.0, 1.0, 6.0, false);
        map.selected_vertices.push(id);

        let mut action = EditVertex::new();
        action.load_params(&map);

        assert_eq!(action.nodeui.get_f32_value("actionVertexX"), Some(0.0));
        assert_eq!(action.nodeui.get_f32_value("actionVertexY"), Some(6.0));
        assert_eq!(action.nodeui.get_f32_value("actionVertexZ"), Some(1.0));
    }
}
