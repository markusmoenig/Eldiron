use crate::prelude::*;
use ToolEvent::*;

use crate::editor::{MODELFXEDITOR, PRERENDERTHREAD, UNDOMANAGER};

pub struct DrawTool {
    id: TheId,

    processed_coords: FxHashSet<Vec2i>,

    material_index: i32,

    align_index: i32,
    brush_size: f32,
    falloff: f32,
}

impl Tool for DrawTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Draw Tool"),
            processed_coords: FxHashSet::default(),

            material_index: 0,
            align_index: 0,
            brush_size: 1.0,
            falloff: 0.0,
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

                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();

                    // Material Group
                    let mut gb = TheGroupButton::new(TheId::named("Material Group"));
                    gb.add_text_status(
                        str!("Material #1"),
                        str!("Draw aligned to the tiles of the regions."),
                    );
                    gb.add_text_status(str!("Material #2"), str!("Draw without any restrictions."));
                    gb.set_item_width(85);

                    gb.set_index(self.align_index);

                    layout.add_widget(Box::new(gb));

                    //
                    let mut spacer = TheIconView::new(TheId::empty());
                    spacer.limiter_mut().set_max_width(5);
                    layout.add_widget(Box::new(spacer));

                    // Brush Size

                    let mut text = TheText::new(TheId::empty());
                    text.set_text("Brush Size".to_string());
                    layout.add_widget(Box::new(text));

                    let mut brush_size = TheSlider::new(TheId::named("Brush Size"));
                    brush_size.set_value(TheValue::Float(self.brush_size));
                    brush_size.set_default_value(TheValue::Float(1.0));
                    brush_size.set_range(TheValue::RangeF32(0.01..=5.0));
                    brush_size.set_continuous(true);
                    brush_size.limiter_mut().set_max_width(120);
                    brush_size.set_status_text("The brush size.");
                    layout.add_widget(Box::new(brush_size));

                    // Falloff

                    let mut text = TheText::new(TheId::empty());
                    text.set_text("Falloff".to_string());
                    layout.add_widget(Box::new(text));

                    let mut falloff = TheSlider::new(TheId::named("Falloff"));
                    falloff.set_value(TheValue::Float(self.falloff));
                    falloff.set_default_value(TheValue::Float(0.0));
                    falloff.set_range(TheValue::RangeF32(0.0..=1.0));
                    falloff.set_continuous(true);
                    falloff.limiter_mut().set_max_width(120);
                    falloff.set_status_text("The falloff off the brush.");
                    layout.add_widget(Box::new(falloff));

                    // Align Group
                    let mut gb = TheGroupButton::new(TheId::named("Draw Align Group"));
                    gb.add_text_status(
                        str!("Tile Align"),
                        str!("Draw aligned to the tiles of the regions."),
                    );
                    gb.add_text_status(str!("Freeform"), str!("Draw without any restrictions."));
                    gb.set_item_width(75);

                    gb.set_index(self.align_index);

                    layout.add_widget(Box::new(gb));

                    layout.set_reverse_index(Some(1));
                }

                return true;
            }
            DeActivate => {
                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();
                    layout.set_reverse_index(None);
                }
                return true;
            }
            _ => {
                return false;
            }
        };

        if let Some(editor) = ui.get_rgba_layout("Region Editor") {
            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                let f = rgba_view.float_pos();
                println!("f {}", f);
            }
        }

        let mut index = 0;
        if let Some(material_id) = server_ctx.curr_material_object {
            if let Some(full) = project.materials.get_full(&material_id) {
                index = full.0;
            }
        }

        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
            let mut region_to_render: Option<Region> = None;
            let mut tiles_to_render: Vec<Vec2i> = vec![];

            if let Some(material_id) = server_ctx.curr_material_object {
                if server_ctx.curr_layer_role == Layer2DRole::Ground {
                    // Paint on the heightmap

                    if index <= 254 {
                        let prev = region.heightmap.clone();

                        let mut mask = if let Some(m) =
                            region.heightmap.get_material_mask_mut(coord.x, coord.y)
                        {
                            m.clone()
                        } else {
                            TheRGBBuffer::new(TheDim::sized(region.grid_size, region.grid_size))
                        };
                        mask.fill([(index + 1) as u8, 0, 0]);
                        region.heightmap.set_material_mask(coord.x, coord.y, mask);
                        server.update_region(region);
                        region_to_render = Some(region.shallow_clone());
                        tiles_to_render = vec![coord];

                        let undo = RegionUndoAtom::HeightmapEdit(
                            prev,
                            region.heightmap.clone(),
                            tiles_to_render.clone(),
                        );
                        UNDOMANAGER
                            .lock()
                            .unwrap()
                            .add_region_undo(&region.id, undo, ctx);
                    }
                } else if server_ctx.curr_layer_role == Layer2DRole::Wall {
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
                                        server_ctx.curr_geo_node =
                                            Some(geo_obj.nodes[node_index].id);

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

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
        _server: &mut Server,
        _client: &mut Client,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        match &event {
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Material Group" {
                    self.material_index = *index as i32;
                } else if id.name == "Draw Align Group" {
                    self.align_index = *index as i32;
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Brush Size" {
                    if let Some(size) = value.to_f32() {
                        self.brush_size = size;
                    }
                }
                if id.name == "Falloff" {
                    if let Some(size) = value.to_f32() {
                        self.falloff = size;
                    }
                }
            }
            _ => {}
        }
        false
    }
}
