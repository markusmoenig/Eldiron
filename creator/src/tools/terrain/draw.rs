use crate::prelude::*;
use rayon::prelude::*;
use ToolEvent::*;

use crate::editor::{
    BRUSHLIST, MODELFXEDITOR, PANELS, PRERENDERTHREAD, TERRAINEDITOR, TILEDRAWER, UNDOMANAGER,
};

pub struct TerrainDrawTool {
    id: TheId,

    processed_coords: FxHashSet<Vec2i>,

    material_params: FxHashMap<Uuid, Vec<Vec<f32>>>,
    material_offset: i32,

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

            material_params: FxHashMap::default(),
            material_offset: 0,

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

                    // Material Group
                    let mut gb = TheGroupButton::new(TheId::named("Material Group"));
                    gb.add_text_status(
                        str!("Material #1"),
                        str!("Draw aligned to the tiles of the regions."),
                    );
                    gb.add_text_status(str!("Material #2"), str!("Draw without any restrictions."));
                    gb.set_item_width(85);

                    gb.set_index(self.material_offset);

                    layout.add_widget(Box::new(gb));

                    //
                    // let mut spacer = TheIconView::new(TheId::empty());
                    // spacer.limiter_mut().set_max_width(5);
                    // layout.add_widget(Box::new(spacer));
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
                let mut terrain_editor = TERRAINEDITOR.lock().unwrap();
                let half_brush = (modelfx.brush_size * brush_scale / 2.0) as i32;

                let mut material_index = 0;
                if let Some(material_id) = server_ctx.curr_material_object {
                    if let Some(full) = project.materials.get_full(&material_id) {
                        material_index = full.0;
                    }
                }

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

                                                if let Some(mask) = region
                                                    .heightmap
                                                    .get_material_mask_mut(tile_id.x, tile_id.y)
                                                {
                                                    if let Some(mut pixel) = mask.get_pixel(px, py)
                                                    {
                                                        pixel[self.material_offset as usize] =
                                                            material_index as u8 + 1;

                                                        if self.material_offset == 1 {
                                                            let a = pixel[2] as i32;
                                                            let b = (falloff * 255.0) as i32;
                                                            // pixel[2] = clamp(a + b, 0, 255) as u8;
                                                            pixel[2] =
                                                                clamp(max(a, b), 0, 255) as u8;
                                                        }

                                                        mask.set_pixel(px, py, &pixel);
                                                    }
                                                }

                                                if let Some(material_id) =
                                                    server_ctx.curr_material_object
                                                {
                                                    if let Some(material) =
                                                        project.materials.get(&material_id)
                                                    {
                                                        let mut hit = Hit {
                                                            pattern_pos: tile_id_f,
                                                            global_uv: tile_id_f,
                                                            ..Default::default()
                                                        };

                                                        if let Some(material_params) =
                                                            self.material_params.get(&material_id)
                                                        {
                                                            material.compute(
                                                                &mut hit,
                                                                &self.palette,
                                                                &TILEDRAWER.lock().unwrap().tiles,
                                                                material_params,
                                                            );

                                                            let pixel =
                                                                TheColor::from(hit.mat.base_color)
                                                                    .to_u8_array();
                                                            b.set_pixel(x, y, &pixel);
                                                            terrain_editor
                                                                .buffer
                                                                .set_pixel(x, y, &pixel);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        let bd = border_mask(d, 1.5);
                                        if bd > 0.0 {
                                            if let Some(mut pixel) = b.get_pixel(x, y) {
                                                pixel = mix_color(&pixel, &WHITE, bd);
                                                b.set_pixel(x, y, &pixel);
                                            }
                                        }
                                    }
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
                            TheId::named("Update Minimaps"),
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
                            }
                        }
                    }
                    //println!("coord {}", coord);
                }
            }
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Material Group" {
                    self.material_offset = *index as i32;
                }
            }
            _ => {}
        }
        false
    }

    fn fill_mask(
        &self,
        material_offset: usize,
        buffer: &mut TheRGBBuffer,
        p: Vec2f,
        coord: Vec2f,
        material_index: u8,
        brush: &dyn Brush,
        settings: &BrushSettings,
    ) {
        let width = buffer.dim().width as usize;
        let height = buffer.dim().height;

        buffer
            .pixels_mut()
            .par_rchunks_exact_mut(width * 3)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(3).enumerate() {
                    let i = j * width + i;

                    let x = (i % width) as f32;
                    let y = (i / width) as f32;

                    let p = p + vec2f(x / width as f32, 1.0 - y / height as f32);
                    let d = brush.distance(p, coord, settings);

                    if d < 0.0 {
                        pixel[material_offset] = material_index;
                    }
                }
            });
    }
}
