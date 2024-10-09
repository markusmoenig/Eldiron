use crate::{prelude::*, DEFAULT_VLAYOUT_RATIO};
use rayon::prelude::*;
use ToolEvent::*;

use crate::editor::{BRUSHLIST, MODELFXEDITOR, PANELS, PRERENDERTHREAD, TILEDRAWER, UNDOMANAGER};

pub struct DrawTool {
    id: TheId,

    processed_coords: FxHashSet<Vec2i>,

    material_offset: i32,
    align_index: i32,
}

impl Tool for DrawTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Draw Tool"),
            processed_coords: FxHashSet::default(),

            material_offset: 0,
            align_index: 0,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Draw Tool (D). Draw with materials on the heightmap and objects.")
    }
    fn icon_name(&self) -> String {
        str!("brush")
    }
    fn accel(&self) -> Option<char> {
        Some('d')
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
        let (coord, _coord_f) = match tool_event {
            TileDown(c, c_f) => {
                self.processed_coords.clear();
                (c, c_f)
            }
            TileDrag(c, c_f) => (c, c_f),
            Activate => {
                PANELS.lock().unwrap().set_brush_panel(ui, ctx);

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

                    // Align Group
                    /*
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
                    */
                }

                if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                    layout.set_shared_ratio(0.75);
                }

                return true;
            }
            DeActivate => {
                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();
                    layout.set_reverse_index(None);
                }
                if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                    layout.set_shared_ratio(DEFAULT_VLAYOUT_RATIO);
                }
                return true;
            }
            _ => {
                return false;
            }
        };

        if let Some(brush) = BRUSHLIST
            .lock()
            .unwrap()
            .brushes
            .get(&server_ctx.curr_brush)
        {
            if server_ctx.curr_material_object.is_none() {
                return false;
            }

            let material_obj = project
                .materials
                .get(&server_ctx.curr_material_object.unwrap())
                .cloned();

            let palette = project.palette.clone();
            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                let mut region_to_render: Option<Region> = None;
                let mut tiles_to_render: Vec<Vec2i> = vec![];
                let modelfx = MODELFXEDITOR.lock().unwrap();

                if let Some(material_id) = server_ctx.curr_material_object {
                    if server_ctx.curr_layer_role == Layer2DRole::Ground {
                        // Paint on the heightmap

                        if let Some(material_obj) = material_obj {
                            let prev = region.heightmap.clone();

                            let mut mask = if let Some(m) =
                                region.heightmap.get_material_mask_mut(coord.x, coord.y)
                            {
                                m.clone()
                            } else {
                                TheRGBBuffer::new(TheDim::sized(region.grid_size, region.grid_size))
                            };

                            let mut mask2 = if let Some(m) =
                                region.heightmap.get_material_mask_mut2(coord.x, coord.y)
                            {
                                m.clone()
                            } else {
                                TheRGBBuffer::new(TheDim::sized(region.grid_size, region.grid_size))
                            };

                            // -- Paint the material into the tile

                            let mat_obj_params = material_obj.load_parameters(&TheTime::default());

                            let width = mask.dim().width as usize;
                            let height = mask.dim().height;

                            let p = Vec2f::zero();
                            let brush_coord = vec2f(0.5, 0.5);

                            let settings = BrushSettings {
                                size: modelfx.brush_size + 0.01,
                                falloff: modelfx.falloff,
                            };
                            let opacity = modelfx.opacity;

                            let tiles = TILEDRAWER.lock().unwrap();

                            pub fn mix_color(a: &[u8], b: &[u8; 4], v: f32) -> [u8; 3] {
                                [
                                    (((1.0 - v) * (a[0] as f32 / 255.0) + b[0] as f32 / 255.0 * v)
                                        * 255.0) as u8,
                                    (((1.0 - v) * (a[1] as f32 / 255.0) + b[1] as f32 / 255.0 * v)
                                        * 255.0) as u8,
                                    (((1.0 - v) * (a[2] as f32 / 255.0) + b[2] as f32 / 255.0 * v)
                                        * 255.0) as u8,
                                ]
                            }

                            mask.pixels_mut()
                                .par_rchunks_exact_mut(width * 3)
                                .zip(mask2.pixels_mut().par_rchunks_exact_mut(width * 3))
                                .enumerate()
                                .for_each(|(j, (line1, line2))| {
                                    line1
                                        .chunks_exact_mut(3)
                                        .zip(line2.chunks_exact_mut(3))
                                        .enumerate()
                                        .for_each(|(i, (pixel1, pixel2))| {
                                            let i = j * width + i;

                                            let x = (i % width) as f32;
                                            let y = (i / width) as f32;

                                            let uv =
                                                vec2f(x / width as f32, 1.0 - y / height as f32);
                                            let p = p + uv;
                                            let d = brush.distance(p, brush_coord, &settings);

                                            let tile_x_f = coord.x as f32 + uv.x;
                                            let tile_y_f = coord.y as f32 + uv.y;

                                            if d < 0.0 {
                                                let mut hit = Hit {
                                                    two_d: true,
                                                    ..Default::default()
                                                };

                                                hit.normal = vec3f(0.0, 1.0, 0.0);
                                                hit.hit_point = vec3f(uv.x, 0.0, uv.y);

                                                hit.uv = uv;
                                                hit.global_uv = vec2f(tile_x_f, tile_y_f);
                                                hit.pattern_pos = hit.global_uv;

                                                material_obj.compute(
                                                    &mut hit,
                                                    &palette,
                                                    &tiles.tiles,
                                                    &mat_obj_params,
                                                );

                                                let col = TheColor::from_vec3f(hit.mat.base_color)
                                                    .to_u8_array();

                                                let c = mix_color(pixel1, &col, opacity);

                                                pixel1[0] = c[0];
                                                pixel1[1] = c[1];
                                                pixel1[2] = c[2];

                                                let roughness = lerp(
                                                    pixel2[0] as f32 / 255.0,
                                                    hit.mat.roughness,
                                                    opacity,
                                                );

                                                let metallic = lerp(
                                                    pixel2[1] as f32 / 255.0,
                                                    hit.mat.metallic,
                                                    opacity,
                                                );

                                                pixel2[0] = (roughness * 255.0) as u8;
                                                pixel2[1] = (metallic * 255.0) as u8;

                                                hit.mode = HitMode::Bump;
                                                material_obj.follow_trail(
                                                    0,
                                                    0,
                                                    &mut hit,
                                                    &palette,
                                                    &tiles.tiles,
                                                    &mat_obj_params,
                                                );

                                                let bump = lerp(
                                                    pixel2[2] as f32 / 255.0,
                                                    hit.bump,
                                                    opacity,
                                                );

                                                pixel2[2] = (bump * 255.0) as u8;
                                            }
                                        });
                                });

                            // --

                            region.heightmap.set_material_mask(coord.x, coord.y, mask);
                            region.heightmap.set_material_mask2(coord.x, coord.y, mask2);
                            server.update_region(region);
                            region_to_render = Some(region.clone());
                            tiles_to_render = vec![coord];

                            let undo = RegionUndoAtom::HeightmapEdit(
                                Box::new(prev),
                                Box::new(region.heightmap.clone()),
                                tiles_to_render.clone(),
                            );
                            UNDOMANAGER
                                .lock()
                                .unwrap()
                                .add_region_undo(&region.id, undo, ctx);
                        }

                        /*
                        #[allow(clippy::collapsible_if)]
                        if self.align_index == 0 {
                            // Fill a single tile with the brush
                            if material_index <= 254 {
                                let prev = region.heightmap.clone();

                                let mut mask = if let Some(m) =
                                    region.heightmap.get_material_mask_mut(coord.x, coord.y)
                                {
                                    m.clone()
                                } else {
                                    TheRGBBuffer::new(TheDim::sized(
                                        region.grid_size,
                                        region.grid_size,
                                    ))
                                };

                                self.fill_mask(
                                    self.material_offset as usize,
                                    &mut mask,
                                    vec2f(0.0, 0.0),
                                    vec2f(0.5, 0.5),
                                    (material_index + 1) as u8,
                                    brush.as_ref(),
                                    &BrushSettings {
                                        size: modelfx.brush_size + 0.01,
                                        falloff: modelfx.falloff,
                                    },
                                );

                                region.heightmap.set_material_mask(coord.x, coord.y, mask);
                                server.update_region(region);
                                region_to_render = Some(region.clone());
                                tiles_to_render = vec![coord];

                                let undo = RegionUndoAtom::HeightmapEdit(
                                    Box::new(prev),
                                    Box::new(region.heightmap.clone()),
                                    tiles_to_render.clone(),
                                );
                                UNDOMANAGER
                                    .lock()
                                    .unwrap()
                                    .add_region_undo(&region.id, undo, ctx);
                            }
                        } else {
                            let size = modelfx.brush_size.ceil() as i32 * 2;
                            let prev = region.heightmap.clone();

                            for y in coord.y - size..coord.y + size {
                                for x in coord.x - size..coord.x + size {
                                    let mut mask = if let Some(m) =
                                        region.heightmap.get_material_mask_mut(x, y)
                                    {
                                        m.clone()
                                    } else {
                                        TheRGBBuffer::new(TheDim::sized(
                                            region.grid_size,
                                            region.grid_size,
                                        ))
                                    };

                                    self.fill_mask(
                                        self.material_offset as usize,
                                        &mut mask,
                                        vec2f(x as f32, y as f32),
                                        coord_f,
                                        (material_index + 1) as u8,
                                        brush.as_ref(),
                                        &BrushSettings {
                                            size: modelfx.brush_size,
                                            falloff: modelfx.falloff,
                                        },
                                    );

                                    region.heightmap.set_material_mask(x, y, mask);
                                    tiles_to_render.push(vec2i(x, y));
                                }
                            }

                            server.update_region(region);
                            region_to_render = Some(region.clone());

                            let undo = RegionUndoAtom::HeightmapEdit(
                                Box::new(prev),
                                Box::new(region.heightmap.clone()),
                                tiles_to_render.clone(),
                            );
                            UNDOMANAGER
                                .lock()
                                .unwrap()
                                .add_region_undo(&region.id, undo, ctx);
                        }*/
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
                        } else if let Some((obj, node_index)) = region
                            .get_closest_geometry(Vec2f::from(coord), server_ctx.curr_layer_role)
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
        #[allow(clippy::single_match)]
        match &event {
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Material Group" {
                    self.material_offset = *index as i32;
                } else if id.name == "Draw Align Group" {
                    self.align_index = *index as i32;
                }
            }
            _ => {}
        }
        false
    }
}
