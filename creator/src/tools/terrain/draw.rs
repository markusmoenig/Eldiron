use crate::prelude::*;
use ToolEvent::*;

use crate::editor::{
    BRUSHLIST, MODELFXEDITOR, PANELS, PRERENDERTHREAD, SIDEBARMODE, TERRAINEDITOR, TILEDRAWER,
    UNDOMANAGER,
};

pub struct TerrainDrawTool {
    id: TheId,

    processed_coords: FxHashSet<Vec2i>,
    opacity: f32,

    material_params: FxHashMap<Uuid, Vec<Vec<f32>>>,

    undo_prev: Heightmap,
    affected_tiles: Vec<Vec2i>,

    palette: ThePalette,
}

impl Tool for TerrainDrawTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Draw Tool"),
            processed_coords: FxHashSet::default(),
            opacity: 1.0,

            material_params: FxHashMap::default(),

            undo_prev: Heightmap::default(),
            affected_tiles: vec![],

            palette: ThePalette::default(),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Draw Tool (D). Draw with materials on the heightmap.")
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
        _tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        _server: &mut Server,
        _client: &mut Client,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        let (_coord, _coord_f) = match tool_event {
            TileDown(c, c_f) => {
                self.processed_coords.clear();
                (c, c_f)
            }
            TileDrag(c, c_f) => (c, c_f),
            Activate => {
                PANELS.lock().unwrap().set_brush_panel(ui, ctx);

                if let Some(layout) = ui.get_hlayout("Terrain Tool Params") {
                    layout.clear();

                    // Opacity
                    let mut text = TheText::new(TheId::empty());
                    text.set_text("Opacity".to_string());
                    layout.add_widget(Box::new(text));

                    let mut opacity = TheSlider::new(TheId::named("Opacity"));
                    opacity.set_value(TheValue::Float(self.opacity));
                    opacity.set_default_value(TheValue::Float(1.0));
                    opacity.set_range(TheValue::RangeF32(0.0..=1.0));
                    opacity.set_continuous(true);
                    opacity.limiter_mut().set_max_width(170);
                    opacity.set_status_text("The opacity off the brush.");
                    layout.add_widget(Box::new(opacity));

                    // let mut gb = TheGroupButton::new(TheId::named("Terrain View Group"));
                    // gb.add_text_status(str!("Top Down"), str!("Top Down View."));
                    // gb.add_text_status(str!("Iso Top Down"), str!("Isometric Perspective."));
                    // gb.set_item_width(100);

                    // gb.set_index(TERRAINEDITOR.lock().unwrap().view_mode as i32);
                    // layout.add_widget(Box::new(gb));
                    layout.set_reverse_index(Some(2));
                }

                if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                    layout.set_shared_ratio(0.75);
                }

                return true;
            }
            DeActivate => {
                if let Some(layout) = ui.get_hlayout("Terrain Tool Params") {
                    layout.clear();
                    layout.set_reverse_index(None);
                }
                if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                    layout.set_shared_ratio(crate::DEFAULT_VLAYOUT_RATIO);
                }

                // Clear the brush by repainting the buffer
                let terrain_editor = TERRAINEDITOR.lock().unwrap();
                if let Some(editor) = ui.get_rgba_layout("TerrainMap") {
                    if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                        let b = rgba_view.buffer_mut();
                        if terrain_editor.buffer.len() == b.len() {
                            b.pixels_mut()
                                .copy_from_slice(terrain_editor.buffer.pixels());
                        }
                    }
                }

                return true;
            }
            _ => {
                return false;
            }
        };

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
        fn border_mask(dist: f32, width: f32) -> f32 {
            (dist + width).clamp(0.0, 1.0) - dist.clamp(0.0, 1.0)
        }
        pub fn mix_color(a: &[u8; 4], b: &[u8; 4], v: f32) -> [u8; 4] {
            [
                (((1.0 - v) * (a[0] as f32 / 255.0) + b[0] as f32 / 255.0 * v) * 255.0) as u8,
                (((1.0 - v) * (a[1] as f32 / 255.0) + b[1] as f32 / 255.0 * v) * 255.0) as u8,
                (((1.0 - v) * (a[2] as f32 / 255.0) + b[2] as f32 / 255.0 * v) * 255.0) as u8,
                255,
            ]
        }
        let modelfx = MODELFXEDITOR.lock().unwrap();
        let brush_scale = 50.0;
        match &event {
            TheEvent::TileEditorClicked(id, coord) | TheEvent::TileEditorDragged(id, coord) => {
                if server_ctx.curr_material_object.is_none() {
                    return false;
                }

                let mode = SIDEBARMODE.lock().unwrap();
                let palette_color = project.palette.get_current_color();

                let material_id = server_ctx.curr_material_object.unwrap();

                let mut terrain_editor = TERRAINEDITOR.lock().unwrap();
                let half_brush = (modelfx.brush_size * brush_scale / 2.0) as i32;

                // On Click, Init the paint specific stuff and undo
                if matches!(*event, TheEvent::TileEditorClicked(_, _)) {
                    if let Some(region) = project.get_region(&server_ctx.curr_region) {
                        self.undo_prev = region.heightmap.clone();
                    }
                    self.affected_tiles = vec![];
                    self.material_params.clear();
                    let time = TheTime::default();
                    for (id, material) in &project.materials {
                        let params = material.load_parameters(&time);
                        self.material_params.insert(*id, params);
                    }
                    self.palette.clone_from(&project.palette);
                }

                let settings = BrushSettings {
                    size: modelfx.brush_size * brush_scale + 0.01,
                    falloff: modelfx.falloff,
                };
                let opacity = self.opacity;

                let mut selection_area = FxHashSet::default();
                if let Some(tilearea) = &server_ctx.tile_selection {
                    if !tilearea.is_empty() {
                        selection_area = tilearea.merged();
                    }
                }

                if id.name == "TerrainMap View" {
                    if let Some(brush) = BRUSHLIST
                        .lock()
                        .unwrap()
                        .brushes
                        .get(&server_ctx.curr_brush)
                    {
                        if let Some(editor) = ui.get_rgba_layout("TerrainMap") {
                            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                                let b = rgba_view.buffer_mut();

                                if terrain_editor.buffer.len() == b.len() {
                                    b.pixels_mut()
                                        .copy_from_slice(terrain_editor.buffer.pixels());
                                }

                                let material = project.materials.get(&material_id).cloned();

                                for y in coord.y - half_brush..=coord.y + half_brush {
                                    for x in coord.x - half_brush..=coord.x + half_brush {
                                        let d = brush.distance(
                                            vec2f(x as f32, y as f32),
                                            Vec2f::from(*coord),
                                            &settings,
                                        );

                                        if d <= 0.0 {
                                            if let Some(region) =
                                                project.get_region_mut(&server_ctx.curr_region)
                                            {
                                                let tile_id_f = vec2f(
                                                    x as f32 / region.grid_size as f32,
                                                    y as f32 / region.grid_size as f32,
                                                );

                                                let tile_id = Vec2i::from(tile_id_f);

                                                let mut draw_it = true;
                                                if !selection_area.is_empty()
                                                    && !selection_area
                                                        .contains(&(tile_id.x, tile_id.y))
                                                {
                                                    draw_it = false;
                                                }

                                                if draw_it {
                                                    if !self.affected_tiles.contains(&tile_id) {
                                                        self.affected_tiles.push(tile_id);
                                                    }

                                                    let falloff = clamp(
                                                        -d / (modelfx.falloff * brush_scale),
                                                        //* (1.0 - modelfx.falloff),
                                                        0.0,
                                                        1.0,
                                                    );

                                                    let px = x % region.grid_size;
                                                    let py = y % region.grid_size;

                                                    let mut hit = Hit {
                                                        pattern_pos: tile_id_f,
                                                        global_uv: tile_id_f,
                                                        ..Default::default()
                                                    };

                                                    let mut color = BLACK;
                                                    let mut bump = 0.0;
                                                    let mut metallic = 0.0;
                                                    let mut roughness = 0.0;

                                                    if *mode == SidebarMode::Material {
                                                        if let Some(material_params) =
                                                            self.material_params.get(&material_id)
                                                        {
                                                            if let Some(material) = &material {
                                                                material.compute(
                                                                    &mut hit,
                                                                    &self.palette,
                                                                    &TILEDRAWER
                                                                        .lock()
                                                                        .unwrap()
                                                                        .tiles,
                                                                    material_params,
                                                                );

                                                                color = TheColor::from(
                                                                    hit.mat.base_color,
                                                                )
                                                                .to_u8_array();

                                                                roughness = hit.mat.roughness;
                                                                metallic = hit.mat.metallic;

                                                                hit.mode = HitMode::Bump;
                                                                material.follow_trail(
                                                                    0,
                                                                    0,
                                                                    &mut hit,
                                                                    &self.palette,
                                                                    &TILEDRAWER
                                                                        .lock()
                                                                        .unwrap()
                                                                        .tiles,
                                                                    material_params,
                                                                );
                                                                bump = hit.bump;
                                                            }
                                                        }
                                                    } else if let Some(col) = &palette_color {
                                                        color = col.to_u8_array();
                                                    }

                                                    let mut mask = if let Some(m) = region
                                                        .heightmap
                                                        .get_material_mask_mut(tile_id.x, tile_id.y)
                                                    {
                                                        m.clone()
                                                    } else {
                                                        TheRGBBuffer::new(TheDim::sized(
                                                            region.grid_size,
                                                            region.grid_size,
                                                        ))
                                                    };

                                                    let mut mask2 = if let Some(m) =
                                                        region.heightmap.get_material_mask_mut2(
                                                            tile_id.x, tile_id.y,
                                                        ) {
                                                        m.clone()
                                                    } else {
                                                        TheRGBBuffer::new(TheDim::sized(
                                                            region.grid_size,
                                                            region.grid_size,
                                                        ))
                                                    };

                                                    if let Some(mut pixel) = mask.get_pixel(px, py)
                                                    {
                                                        let mut old_color = BLACK;
                                                        old_color[0] = pixel[0];
                                                        old_color[1] = pixel[1];
                                                        old_color[2] = pixel[2];
                                                        old_color[3] = 255;

                                                        let mix_value = opacity * falloff;

                                                        let new_color = mix_color(
                                                            &old_color, &color, mix_value,
                                                        );

                                                        pixel[0] = new_color[0];
                                                        pixel[1] = new_color[1];
                                                        pixel[2] = new_color[2];

                                                        /*
                                                        pixel[self.material_offset as usize] =
                                                            material_index as u8 + 1;

                                                        if self.material_offset == 1 {
                                                            let a = pixel[2] as i32;
                                                            let b = (falloff * 255.0) as i32;
                                                            // pixel[2] = clamp(a + b, 0, 255) as u8;
                                                            pixel[2] =
                                                                clamp(max(a, b), 0, 255) as u8;
                                                        }*/

                                                        b.set_pixel(x, y, &new_color);
                                                        terrain_editor
                                                            .buffer
                                                            .set_pixel(x, y, &new_color);
                                                        mask.set_pixel(px, py, &pixel);

                                                        region.heightmap.set_material_mask(
                                                            tile_id.x, tile_id.y, mask,
                                                        );
                                                    }

                                                    if let Some(mut pixel) = mask2.get_pixel(px, py)
                                                    {
                                                        let roughness = lerp(
                                                            pixel[0] as f32 / 255.0,
                                                            roughness,
                                                            opacity,
                                                        );

                                                        let metallic = lerp(
                                                            pixel[1] as f32 / 255.0,
                                                            metallic,
                                                            opacity,
                                                        );

                                                        pixel[0] = (roughness * 255.0) as u8;
                                                        pixel[1] = (metallic * 255.0) as u8;

                                                        let bump = lerp(
                                                            pixel[2] as f32 / 255.0,
                                                            bump,
                                                            opacity,
                                                        );

                                                        pixel[2] = (bump * 255.0) as u8;

                                                        mask2.set_pixel(px, py, &pixel);
                                                        region.heightmap.set_material_mask2(
                                                            tile_id.x, tile_id.y, mask2,
                                                        );
                                                    }
                                                }
                                            }
                                        }

                                        // Brush border mask
                                        let bd = border_mask(d, 1.5);
                                        if bd > 0.0 {
                                            if let Some(mut pixel) = b.get_pixel(x, y) {
                                                pixel = mix_color(&pixel, &WHITE, bd);
                                                b.set_pixel(x, y, &pixel);
                                            }
                                        }
                                    }
                                }

                                if let Some(tilearea) = &server_ctx.tile_selection {
                                    TILEDRAWER.lock().unwrap().draw_tile_selection(
                                        &tilearea.merged(),
                                        b,
                                        terrain_editor.grid_size,
                                        WHITE,
                                        ctx,
                                    );
                                }
                            }
                        }
                    }
                    //println!("coord {}", coord);
                }
            }
            TheEvent::TileEditorUp(id) => {
                if id.name == "TerrainMap View" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        server.update_region(region);

                        let undo = RegionUndoAtom::HeightmapEdit(
                            Box::new(self.undo_prev.clone()),
                            Box::new(region.heightmap.clone()),
                            self.affected_tiles.clone(),
                        );
                        UNDOMANAGER
                            .lock()
                            .unwrap()
                            .add_region_undo(&region.id, undo, ctx);

                        PRERENDERTHREAD
                            .lock()
                            .unwrap()
                            .render_region(region.clone(), Some(self.affected_tiles.clone()));

                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Minimap"),
                            TheValue::Empty,
                        ));
                    }
                }
            }
            TheEvent::TileEditorHoverChanged(id, coord) => {
                let terrain_editor = TERRAINEDITOR.lock().unwrap();
                let half_brush = (modelfx.brush_size * brush_scale / 2.0) as i32;

                let settings = BrushSettings {
                    size: modelfx.brush_size * brush_scale + 0.01,
                    falloff: modelfx.falloff,
                };

                if id.name == "TerrainMap View" {
                    if let Some(brush) = BRUSHLIST
                        .lock()
                        .unwrap()
                        .brushes
                        .get(&server_ctx.curr_brush)
                    {
                        if let Some(editor) = ui.get_rgba_layout("TerrainMap") {
                            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                                let b = rgba_view.buffer_mut();
                                //b.copy_into(0, 0, &terrain_editor.buffer);
                                // unsafe {
                                //     std::ptr::copy_nonoverlapping(
                                //         terrain_editor.buffer.pixels().as_ptr(),
                                //         b.pixels_mut().as_mut_ptr(),
                                //         b.len(),
                                //     );
                                // }
                                if terrain_editor.buffer.len() == b.len() {
                                    b.pixels_mut()
                                        .copy_from_slice(terrain_editor.buffer.pixels());
                                }

                                for y in coord.y - half_brush..=coord.y + half_brush {
                                    for x in coord.x - half_brush..=coord.x + half_brush {
                                        let d = brush.distance(
                                            vec2f(x as f32, y as f32),
                                            Vec2f::from(*coord),
                                            &settings,
                                        );

                                        let bd = border_mask(d, 1.5);
                                        if bd > 0.0 {
                                            if let Some(mut pixel) = b.get_pixel(x, y) {
                                                pixel = mix_color(&pixel, &WHITE, bd);
                                                b.set_pixel(x, y, &pixel);
                                            }
                                        }
                                    }
                                }

                                if let Some(tilearea) = &server_ctx.tile_selection {
                                    TILEDRAWER.lock().unwrap().draw_tile_selection(
                                        &tilearea.merged(),
                                        b,
                                        terrain_editor.grid_size,
                                        WHITE,
                                        ctx,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Opacity" {
                    if let Some(size) = value.to_f32() {
                        self.opacity = size;
                    }
                }
            }
            _ => {}
        }
        false
    }
}
