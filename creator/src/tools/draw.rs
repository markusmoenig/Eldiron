use crate::prelude::*;
use ToolEvent::*;

use crate::editor::{MODELFXEDITOR, PRERENDERTHREAD, UNDOMANAGER};

pub struct DrawTool {
    id: TheId,

    processed_coords: FxHashSet<Vec2i>,
}

impl Tool for DrawTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Draw Tool"),
            processed_coords: FxHashSet::default(),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Draw Tool. Draw with materials.")
    }
    fn icon_name(&self) -> String {
        str!("brush")
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let coord = match tool_event {
            TileDown(c) => {
                self.processed_coords.clear();
                c
            }
            TileDrag(c) => c,
            Activate => {
                MODELFXEDITOR.lock().unwrap().set_geometry_mode(false);
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Set Region Material"),
                    TheValue::Empty,
                ));
                return true;
            }
            _ => {
                return false;
            }
        };

        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
            let mut region_to_render: Option<Region> = None;
            let mut tiles_to_render: Vec<Vec2i> = vec![];

            if let Some(material_id) = server_ctx.curr_material_object {
                // Set the material to the current geometry node.
                if tool_context == ToolContext::TwoD {
                    if let Some(editor) = ui.get_rgba_layout("Region Editor") {
                        if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                            let p = rgba_view.float_pos();
                            if let Some((obj, node_index)) =
                                region.get_closest_geometry(p, server_ctx.curr_layer_role)
                            {
                                if let Some(geo_obj) = region.geometry.get_mut(&obj) {
                                    server_ctx.curr_geo_object = Some(geo_obj.id);
                                    server_ctx.curr_geo_node = Some(geo_obj.nodes[node_index].id);

                                    let prev = geo_obj.clone();

                                    geo_obj.material_id = material_id;
                                    geo_obj.update_area();

                                    tiles_to_render.clone_from(&geo_obj.area);

                                    let undo = RegionUndoAtom::GeoFXObjectEdit(
                                        geo_obj.id,
                                        Some(prev),
                                        Some(geo_obj.clone()),
                                        tiles_to_render.clone(),
                                    );
                                    UNDOMANAGER
                                        .lock()
                                        .unwrap()
                                        .add_region_undo(&region.id, undo, ctx);

                                    server.update_region(region);
                                    region_to_render = Some(region.clone());
                                }
                            }
                        }
                    }
                } else if let Some((obj, node_index)) =
                    region.get_closest_geometry(Vec2f::from(coord), server_ctx.curr_layer_role)
                {
                    if let Some(geo_obj) = region.geometry.get_mut(&obj) {
                        server_ctx.curr_geo_object = Some(geo_obj.id);
                        server_ctx.curr_geo_node = Some(geo_obj.nodes[node_index].id);

                        let prev = geo_obj.clone();

                        geo_obj.material_id = material_id;
                        geo_obj.update_area();

                        tiles_to_render.clone_from(&geo_obj.area);

                        let undo = RegionUndoAtom::GeoFXObjectEdit(
                            geo_obj.id,
                            Some(prev),
                            Some(geo_obj.clone()),
                            tiles_to_render.clone(),
                        );
                        UNDOMANAGER
                            .lock()
                            .unwrap()
                            .add_region_undo(&region.id, undo, ctx);

                        server.update_region(region);
                        region_to_render = Some(region.clone());
                    }
                }

                // Render the region area covered by the object with the new material.
                if let Some(region) = region_to_render {
                    PRERENDERTHREAD
                        .lock()
                        .unwrap()
                        .render_region(region, Some(tiles_to_render));
                }
            }
        }

        false
    }
}
