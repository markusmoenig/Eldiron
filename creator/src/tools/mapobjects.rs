use crate::prelude::*;
use ToolEvent::*;

use crate::editor::{MODELFXEDITOR, PRERENDERTHREAD, TILEDRAWER, UNDOMANAGER};

pub struct MapObjectsTool {
    id: TheId,

    processed_coords: FxHashSet<Vec2i>,
}

impl Tool for MapObjectsTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Model Tool"),
            processed_coords: FxHashSet::default(),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Model Tool (M). Place 3D objects on the map.")
    }
    fn icon_name(&self) -> String {
        str!("mapobjects")
    }
    fn accel(&self) -> Option<char> {
        Some('m')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let coord = match tool_event {
            TileDown(c, _) => {
                self.processed_coords.clear();
                c
            }
            TileDrag(c, _) => c,
            Activate => {
                MODELFXEDITOR.lock().unwrap().set_geometry_mode(true);
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Set Region Modeler"),
                    TheValue::Empty,
                ));
                return true;
            }
            _ => {
                return false;
            }
        };

        let mut region_to_render: Option<Region> = None;
        let mut tiles_to_render: Vec<Vec2i> = vec![];
        let palette = project.palette.clone();

        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
            if !self.processed_coords.contains(&coord) {
                // Add Geometry
                let geo = MODELFXEDITOR.lock().unwrap().get_geo_node(ui);
                if let Some(mut geo) = geo {
                    let new_id = Uuid::new_v4();
                    geo.id = new_id;
                    geo.set_default_position(coord);
                    // let obj_id = region.add_geo_node(geo);
                    // if let Some((geo_obj, _)) = region.find_geo_node(new_id) {
                    //     tiles_to_render.clone_from(&geo_obj.area);
                    // }

                    let mut geo_obj = GeoFXObject::default();
                    geo_obj.nodes.push(geo);
                    geo_obj.init();
                    geo_obj.update_area();
                    tiles_to_render.clone_from(&geo_obj.area);
                    let geo_obj_id = geo_obj.id;
                    region.geometry.insert(geo_obj_id, geo_obj);

                    region.update_geometry_areas();

                    region.compile_geo(geo_obj_id, &palette, &TILEDRAWER.lock().unwrap().tiles);
                    server_ctx.curr_geo_object = Some(geo_obj_id);
                    server_ctx.curr_geo_node = Some(new_id);
                    region_to_render = Some(region.clone());

                    server.update_region(region);

                    if let Some(obj) = region.geometry.get(&geo_obj_id) {
                        let undo = RegionUndoAtom::GeoFXObjectEdit(
                            geo_obj_id,
                            None,
                            Some(obj.clone()),
                            tiles_to_render.clone(),
                        );
                        UNDOMANAGER
                            .lock()
                            .unwrap()
                            .add_region_undo(&region.id, undo, ctx);
                    }

                    MODELFXEDITOR
                        .lock()
                        .unwrap()
                        .set_geo_node_ui(server_ctx, project, ui, ctx);

                    self.processed_coords.insert(coord);
                }
            }

            if let Some(region) = region_to_render {
                PRERENDERTHREAD
                    .lock()
                    .unwrap()
                    .render_region(region, Some(tiles_to_render));
            }
        }

        false
    }
}
