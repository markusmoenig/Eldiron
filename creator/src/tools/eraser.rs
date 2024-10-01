use crate::prelude::*;
use ToolEvent::*;

use crate::editor::{PRERENDERTHREAD, RENDERER, UNDOMANAGER};

pub struct EraserTool {
    id: TheId,
}

impl Tool for EraserTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Eraser Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Eraser Tool (E). Erase content in the region editors.")
    }
    fn icon_name(&self) -> String {
        str!("eraser")
    }
    fn accel(&self) -> Option<char> {
        Some('e')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let coord = match tool_event {
            TileDown(c, _) => c,
            TileDrag(c, _) => c,
            Activate => {
                return true;
            }
            _ => {
                return false;
            }
        };

        // If there is a character instance at the position we delete the instance.
        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
            // We only check to delete models and tiles.
            // Characters, items and areas need to be erased by the Sidebar Region Content List.

            /*
            if let Some(c) =
                server.get_character_at(server_ctx.curr_region, vec2i(coord.x, coord.y))
            {
                // Delete the character at the given position.

                if let Some((value, _)) =
                    server.get_character_property(region.id, c.0, "name".to_string())
                {
                    open_delete_confirmation_dialog(
                        "Delete Character Instance ?",
                        format!("Permanently delete '{}' ?", value.describe()).as_str(),
                        c.0,
                        ui,
                        ctx,
                    );
                }
            } else if let Some(c) =
                server.get_item_at(server_ctx.curr_region, vec2i(coord.x, coord.y))
            {
                // Delete the item at the given position.

                if let Some((value, _)) =
                    server.get_character_property(region.id, c.0, "name".to_string())
                {
                    open_delete_confirmation_dialog(
                        "Delete Item Instance ?",
                        format!("Permanently delete '{}' ?", value.describe()).as_str(),
                        c.0,
                        ui,
                        ctx,
                    );
                }
            } else {
            */
            //let area_id: Option<Uuid> = None;

            /*
            // Check for area at the given position.
            for area in region.areas.values() {
                if area.area.contains(&(coord.x, coord.y)) {
                    // Ask to delete it.
                    open_delete_confirmation_dialog(
                        "Delete Area ?",
                        format!("Permanently delete area '{}' ?", area.name).as_str(),
                        area.id,
                        ui,
                        ctx,
                    );
                    area_id = Some(area.id);
                    break;
                }
                }*/

            let mut region_to_render: Option<Region> = None;
            let mut tiles_to_render: Vec<Vec2i> = vec![];

            // Delete the tile at the given position.

            let mut changed = false;

            // Check for geometry to delete
            if let Some(geo_obj_ids) = region.geometry_areas.get_mut(&vec3i(coord.x, 0, coord.y)) {
                let mut objects = vec![];
                for obj_id in geo_obj_ids {
                    let mut remove_it = false;

                    if let Some(geo_obj) = region.geometry.get(obj_id) {
                        remove_it = Some(server_ctx.curr_layer_role) == geo_obj.get_layer_role();
                    }

                    if remove_it {
                        if let Some(geo_obj) = region.geometry.remove(obj_id) {
                            for a in &geo_obj.area {
                                tiles_to_render.push(*a);
                            }
                            objects.push(geo_obj.clone());
                        }
                    }
                }

                if !objects.is_empty() {
                    changed = true;
                    region_to_render = Some(region.clone());

                    region.update_geometry_areas();
                    let undo =
                        RegionUndoAtom::GeoFXObjectsDeletion(objects, tiles_to_render.clone());
                    UNDOMANAGER
                        .lock()
                        .unwrap()
                        .add_region_undo(&region.id, undo, ctx);
                }
            }

            // Check for heightmap

            if !changed
                && region
                    .heightmap
                    .material_mask
                    .contains_key(&(coord.x, coord.y))
            {
                let prev = region.heightmap.clone();
                if let Some(mask) = region.heightmap.material_mask.get_mut(&(coord.x, coord.y)) {
                    mask.fill([0, 0, 0]);
                }
                region_to_render = Some(region.clone());
                tiles_to_render.push(vec2i(coord.x, coord.y));

                let undo = RegionUndoAtom::HeightmapEdit(
                    Box::new(prev),
                    Box::new(region.heightmap.clone()),
                    tiles_to_render.clone(),
                );
                UNDOMANAGER
                    .lock()
                    .unwrap()
                    .add_region_undo(&region.id, undo, ctx);

                changed = true;
            }

            // Check for tiles to delete
            if !changed {
                if let Some(tile) = region.tiles.get_mut(&(coord.x, coord.y)) {
                    let prev = Some(tile.clone());
                    if server_ctx.curr_layer_role == Layer2DRole::Ground && tile.layers[0].is_some()
                    {
                        tile.layers[0] = None;
                        changed = true;
                    } else if server_ctx.curr_layer_role == Layer2DRole::Wall
                        && tile.layers[1].is_some()
                    {
                        tile.layers[1] = None;
                        changed = true;
                    } else if server_ctx.curr_layer_role == Layer2DRole::Ceiling
                        && tile.layers[2].is_some()
                    {
                        tile.layers[2] = None;
                        changed = true;
                    }
                    if changed {
                        tiles_to_render.push(coord);
                        let undo = RegionUndoAtom::RegionTileEdit(
                            vec2i(coord.x, coord.y),
                            prev,
                            Some(tile.clone()),
                        );
                        UNDOMANAGER
                            .lock()
                            .unwrap()
                            .add_region_undo(&region.id, undo, ctx);
                    }
                }

                if changed {
                    region_to_render = Some(region.clone());
                }
            }

            if changed {
                server.update_region(region);
                RENDERER.lock().unwrap().set_region(region);
                //self.set_icon_previews(region, &palette, coord, ui);
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
