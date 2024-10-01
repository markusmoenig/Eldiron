use crate::prelude::*;
use ToolEvent::*;

use crate::editor::{PRERENDERTHREAD, UNDOMANAGER};

pub struct ResizeTool {
    id: TheId,
}

impl Tool for ResizeTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Resize Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Resize Tool (I). Resize the region.")
    }
    fn icon_name(&self) -> String {
        str!("transform")
    }
    fn accel(&self) -> Option<char> {
        Some('i')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &mut Project,
        _server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if let Activate = tool_event {
            if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                layout.clear();

                let mut text = TheText::new(TheId::empty());
                text.set_text("Expand".to_string());
                layout.add_widget(Box::new(text));

                let mut drop_down = TheDropdownMenu::new(TheId::named("Region Expansion Mode"));
                drop_down.add_option("Top / Left".to_string());
                drop_down.add_option("Top / Right".to_string());
                drop_down.add_option("Bottom / Left".to_string());
                drop_down.add_option("Bottom / Right".to_string());
                drop_down.set_status_text(
                    "Size changes will grow or shrink the region from the given corner.",
                );

                layout.add_widget(Box::new(drop_down));

                let mut hdivider = TheHDivider::new(TheId::empty());
                hdivider.limiter_mut().set_max_width(15);
                layout.add_widget(Box::new(hdivider));

                //layout.add_pair("Grow / Shrink".to_string(), Box::new(drop_down));
                let mut width_edit = TheTextLineEdit::new(TheId::named("Region Width Edit"));
                if let Some(region) = project.get_region(&server_ctx.curr_region) {
                    width_edit.set_value(TheValue::Int(region.width));
                }
                width_edit.set_range(TheValue::RangeI32(1..=100000));
                width_edit.set_status_text("The width of the region in grid units.");
                width_edit.limiter_mut().set_max_width(80);
                layout.add_widget(Box::new(width_edit));

                let mut text = TheText::new(TheId::empty());
                text.set_text("x".to_string());
                layout.add_widget(Box::new(text));

                let mut height_edit = TheTextLineEdit::new(TheId::named("Region Height Edit"));
                if let Some(region) = project.get_region(&server_ctx.curr_region) {
                    height_edit.set_value(TheValue::Int(region.height));
                }
                height_edit.set_range(TheValue::RangeI32(1..=100000));
                height_edit.set_status_text("The height of the region in grid units.");
                height_edit.limiter_mut().set_max_width(80);
                layout.add_widget(Box::new(height_edit));

                let mut hdivider = TheHDivider::new(TheId::empty());
                hdivider.limiter_mut().set_max_width(15);
                layout.add_widget(Box::new(hdivider));

                let mut resize_button = TheTraybarButton::new(TheId::named("Region Resize"));
                resize_button.set_text(str!("Resize!"));
                resize_button.set_status_text(
                    "Resizes the region (growing or shrinking it) based on the expansion mode. Adjusts all meta data like areas and code.",
                );
                //resize_button.set_disabled(true);

                layout.add_widget(Box::new(resize_button));

                // if let Some(region) = project.get_region(&server_ctx.curr_region) {
                //     //zoom.set_value(TheValue::Float(region.zoom));
                //     let mut text = TheText::new(TheId::empty());
                //     text.set_text(format!("{}x{}"));
                //     layout.add_widget(Box::new(text));
                // }

                //layout.set_reverse_index(Some(1));
            }

            return true;
        } else if let DeActivate = tool_event {
            if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                layout.clear();
                layout.set_reverse_index(None);
            }
            return true;
        }

        false
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        #[allow(clippy::single_match)]
        match event {
            // TheEvent::ValueChanged(id, value) => {
            //     if id.name == "Region Width Edit" {
            //         if let Some(width) = value.to_f32() {
            //             if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
            //             }
            //         }
            //     }
            // }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) => {
                if id.name == "Region Resize" {
                    let new_width = ui
                        .get_widget_value("Region Width Edit")
                        .unwrap()
                        .to_i32()
                        .unwrap();

                    let new_height = ui
                        .get_widget_value("Region Height Edit")
                        .unwrap()
                        .to_i32()
                        .unwrap();

                    let expansion_mode = ui
                        .get_widget_value("Region Expansion Mode")
                        .unwrap()
                        .to_i32()
                        .unwrap();

                    //println!("{} {} {}", expansion_mode, new_width, new_height);

                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        let width_changed = new_width != region.width;
                        let height_changed = new_height != region.height;

                        if !width_changed && !height_changed {
                            return false;
                        }

                        let prev = region.clone();

                        // Compute changes

                        let mut width_prefix = 0;
                        let mut height_prefix = 0;

                        if width_changed && (expansion_mode == 0 || expansion_mode == 2) {
                            width_prefix = new_width - region.width;
                        }

                        if height_changed && (expansion_mode == 0 || expansion_mode == 1) {
                            height_prefix = new_height - region.height;
                        }

                        if width_prefix != 0 || height_prefix != 0 {
                            println!("{} {}", width_prefix, height_prefix);

                            // Move Geos
                            let mut new_geometry_map: FxHashMap<Uuid, GeoFXObject> =
                                FxHashMap::default();
                            for geo_obj in region.geometry.values_mut() {
                                let mut pos = geo_obj.get_position();
                                if width_prefix > 0 {
                                    pos.x += width_prefix as f32;
                                }
                                if height_prefix > 0 {
                                    pos.y += height_prefix as f32;
                                }
                                geo_obj.set_position(pos);
                                geo_obj.update_area();
                                new_geometry_map.insert(geo_obj.id, geo_obj.clone());
                            }
                            region.geometry = new_geometry_map;

                            // Move Heightmap
                            let mut new_mask_map: FxHashMap<(i32, i32), TheRGBBuffer> =
                                FxHashMap::default();
                            for (pos, mask) in region.heightmap.material_mask.iter() {
                                let mut p = *pos;
                                if width_prefix > 0 {
                                    p.0 += width_prefix;
                                }
                                if height_prefix > 0 {
                                    p.1 += height_prefix;
                                }
                                new_mask_map.insert(p, mask.clone());
                            }
                            region.heightmap.material_mask = new_mask_map;

                            // Move Tiles
                            let mut new_tile_map: FxHashMap<(i32, i32), RegionTile> =
                                FxHashMap::default();
                            for (pos, tile) in region.tiles.iter() {
                                let mut p = *pos;
                                if width_prefix > 0 {
                                    p.0 += width_prefix;
                                }
                                if height_prefix > 0 {
                                    p.1 += height_prefix;
                                }
                                new_tile_map.insert(p, tile.clone());
                            }
                            region.tiles = new_tile_map;

                            // Move Effects
                            let mut new_tilefx_map: FxHashMap<Vec3i, TileFXObject> =
                                FxHashMap::default();
                            for (pos, tilefx) in region.effects.iter() {
                                let mut p = *pos;
                                if width_prefix > 0 {
                                    p.x += width_prefix;
                                }
                                if height_prefix > 0 {
                                    p.z += height_prefix;
                                }
                                new_tilefx_map.insert(p, tilefx.clone());
                            }
                            region.effects = new_tilefx_map;

                            // Move Area
                            let mut new_area_map: FxHashMap<Uuid, Area> = FxHashMap::default();
                            for (id, area) in region.areas.iter() {
                                let mut area = area.clone();
                                let mut new_area = FxHashSet::default();
                                for t in area.area.iter() {
                                    let mut p = *t;
                                    if width_prefix > 0 {
                                        p.0 += width_prefix;
                                    }
                                    if height_prefix > 0 {
                                        p.1 += height_prefix;
                                    }
                                    new_area.insert(p);
                                }
                                area.area = new_area;
                                new_area_map.insert(*id, area);
                            }
                            region.areas = new_area_map;

                            // Move Positions in Character Instances
                            let mut new_character_map: FxHashMap<Uuid, Character> =
                                FxHashMap::default();
                            for (id, character) in region.characters.iter() {
                                let mut character = character.clone();
                                // Move Positions of the instance
                                character
                                    .instance
                                    .move_positions_by(vec2i(width_prefix, height_prefix));
                                // Update the instance on the server
                                server.update_character_instance_bundle(
                                    region.id,
                                    *id,
                                    character.instance.clone(),
                                );
                                new_character_map.insert(*id, character);
                            }
                            region.characters = new_character_map;

                            // Move Positions in Item Instances
                            let mut new_item_map: FxHashMap<Uuid, Item> = FxHashMap::default();
                            for (id, item) in region.items.iter() {
                                let mut item = item.clone();
                                // Move Positions of the instance
                                item.instance
                                    .move_positions_by(vec2i(width_prefix, height_prefix));
                                // Update the instance on the server
                                server.update_item_instance_bundle(
                                    region.id,
                                    *id,
                                    item.instance.clone(),
                                );
                                new_item_map.insert(*id, item);
                            }
                            region.items = new_item_map;
                        }

                        // Update the region
                        region.width = new_width;
                        region.height = new_height;
                        if let Some(rgba_layout) = ui.get_rgba_layout("Region Editor") {
                            if let Some(rgba) = rgba_layout.rgba_view_mut().as_rgba_view() {
                                let width = region.width * region.grid_size;
                                let height = region.height * region.grid_size;
                                let buffer = TheRGBABuffer::new(TheDim::new(0, 0, width, height));
                                rgba.set_buffer(buffer);
                                ctx.ui.relayout = true;
                            }
                        }
                        region.update_geometry_areas();
                        server.update_region(region);

                        let undo =
                            RegionUndoAtom::RegionResize(Box::new(prev), Box::new(region.clone()));
                        UNDOMANAGER
                            .lock()
                            .unwrap()
                            .add_region_undo(&region.id, undo, ctx);

                        PRERENDERTHREAD
                            .lock()
                            .unwrap()
                            .render_region(region.clone(), None);
                    }
                }
            }
            _ => {}
        }

        false
    }
}
