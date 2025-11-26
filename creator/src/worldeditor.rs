use crate::editor::{RUSTERIX, SCENEMANAGER, UNDOMANAGER};
use crate::hud::{Hud, HudMode};
use crate::prelude::*;
use rayon::prelude::*;
use shared::prelude::*;

use rusterix::{
    BrushPreview, D3Camera, D3OrbitCamera, PixelSource, Terrain, TerrainBlendMode, TerrainChunk,
    ValueContainer,
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BrushType {
    Elevation,
    Fill,
    Smooth,
    Fractal,
    Fixed,
}

impl BrushType {
    pub fn set_from_index(&mut self, index: usize) {
        match index {
            1 => *self = BrushType::Fill,
            2 => *self = BrushType::Smooth,
            3 => *self = BrushType::Fractal,
            4 => *self = BrushType::Fixed,
            _ => *self = BrushType::Elevation,
        }
    }
}

pub struct WorldEditor {
    orbit_camera: D3OrbitCamera,

    terrain_hit: Option<Vec3<f32>>,
    drag_coord: Vec2<i32>,

    pub brush_type: BrushType,

    pub radius: f32,
    pub falloff: f32,
    pub strength: f32,
    pub fixed: f32,

    hud: Hud,

    apply_brush: bool,

    undo_chunks: FxHashMap<(i32, i32), TerrainChunk>,
    edited: bool,

    // Tile Paint Rulez
    pub blend_radius: i32,
    pub tile_rules: bool,
    pub tile_rules_distance: f32,
    pub tile_rules_height: f32,
    pub tile_rules_steepness: f32,
}

#[allow(clippy::new_without_default)]
impl WorldEditor {
    pub fn new() -> Self {
        Self {
            orbit_camera: D3OrbitCamera::new(),

            terrain_hit: None,
            drag_coord: Vec2::zero(),

            brush_type: BrushType::Elevation,

            radius: 10.0,
            falloff: 2.0,
            strength: 0.2,
            fixed: 2.0,

            hud: Hud::new(HudMode::Terrain),

            apply_brush: false,

            undo_chunks: FxHashMap::default(),
            edited: false,

            blend_radius: 0,
            tile_rules: true,

            tile_rules_distance: 5.0,
            tile_rules_height: 1.0,
            tile_rules_steepness: 1.0,
        }
    }

    pub fn build_brush_canvas(&mut self) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut text_layout = TheTextLayout::new(TheId::named("Brush Settings"));
        text_layout.limiter_mut().set_max_width(200);

        let mut brush_switch = TheGroupButton::new(TheId::named("Brush Type"));
        brush_switch.add_text_status(
            "Elevation".to_string(),
            "Raise (click-drag) or lower the terrain (shift click-drag).".to_string(),
        );
        brush_switch.add_text_status(
            "Fill".to_string(),
            "Fill missing areas with flat terrain (click-drag) or removes it (shift click-drag)"
                .to_string(),
        );
        brush_switch.add_text_status(
            "Smooth".to_string(),
            "Smooth the terrain (click-drag) or roughen it (shift click-drag).".to_string(),
        );
        brush_switch.add_text_status(
            "Fractal".to_string(),
            "Add fractal noise to create natural terrain (click-drag).".to_string(),
        );
        brush_switch.add_text_status(
            "Fixed".to_string(),
            "Set terrain to a fixed height(click-drag).".to_string(),
        );
        brush_switch.set_item_width(80);
        text_layout.add_pair("".to_string(), Box::new(brush_switch));

        let mut spacer = TheSpacer::new(TheId::empty());
        spacer.limiter_mut().set_max_height(20);
        text_layout.add_pair("".to_string(), Box::new(spacer));

        let mut radius = TheTextLineEdit::new(TheId::named("Brush Radius"));
        radius.set_value(TheValue::Float(10.0));
        radius.set_range(TheValue::RangeF32(0.5..=10.0));
        radius.set_info_text(Some("Radius".into()));
        radius.set_status_text(&fl!("status_world_editor_brush_radius"));
        radius.limiter_mut().set_max_width(300);
        text_layout.add_pair("".to_string(), Box::new(radius));

        let mut falloff = TheTextLineEdit::new(TheId::named("Brush Falloff"));
        falloff.set_value(TheValue::Float(2.0));
        falloff.set_range(TheValue::RangeF32(0.5..=4.0));
        falloff.set_info_text(Some("Falloff".into()));
        falloff.set_status_text(&fl!("status_world_editor_brush_falloff"));
        falloff.limiter_mut().set_max_width(300);
        text_layout.add_pair("".to_string(), Box::new(falloff));

        let mut strength = TheTextLineEdit::new(TheId::named("Brush Strength"));
        strength.set_value(TheValue::Float(0.2));
        strength.set_range(TheValue::RangeF32(0.01..=1.0));
        strength.set_info_text(Some("Strength".into()));
        strength.set_status_text(&fl!("status_world_editor_brush_strength"));
        strength.limiter_mut().set_max_width(300);
        text_layout.add_pair("".to_string(), Box::new(strength));

        let mut fixed = TheTextLineEdit::new(TheId::named("Brush Fixed"));
        fixed.set_value(TheValue::Float(2.0));
        fixed.set_range(TheValue::RangeF32(-10.0..=50.0));
        fixed.set_info_text(Some("Fixed".into()));
        fixed.set_status_text(&fl!("status_world_editor_brush_fixed"));
        fixed.limiter_mut().set_max_width(300);
        text_layout.add_pair("".to_string(), Box::new(fixed));

        center.set_layout(text_layout);

        let mut preview_canvas: TheCanvas = TheCanvas::new();
        let mut render_view = TheRenderView::new(TheId::named("Brush Preview"));
        render_view.limiter_mut().set_max_size(Vec2::new(300, 300));
        preview_canvas.set_widget(render_view);

        center.set_right(preview_canvas);

        center
    }

    pub fn draw(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
        _build_values: &mut ValueContainer,
    ) {
        if self.apply_brush {
            if let Some(hit) = self.terrain_hit {
                if let Some(map) = project.get_map_mut(server_ctx) {
                    map.terrain.mark_clean();
                    self.apply_brush(&mut map.terrain, Vec2::new(hit.x, hit.z), ui);
                    if let Some(hit) = self.terrain_hit {
                        server_ctx.hover_height = Some(map.terrain.sample_height(hit.x, hit.z));
                    }
                    let cloned = map.terrain.clone_dirty_chunks();
                    if !cloned.is_empty() {
                        SCENEMANAGER
                            .write()
                            .unwrap()
                            .set_dirty_terrain_chunks(cloned);
                    }
                    map.terrain.mark_clean();
                }
            }
        }

        if let Some(render_view) = ui.get_render_view("PolyView") {
            let dim = *render_view.dim();

            let buffer = render_view.render_buffer_mut();
            buffer.resize(dim.width, dim.height);

            let mut rusterix = RUSTERIX.write().unwrap();

            rusterix.client.camera_d3 = Box::new(self.orbit_camera.clone());

            if let Some(region) = project.get_region_ctx_mut(server_ctx) {
                rusterix
                    .client
                    .camera_d3
                    .set_parameter_vec3("center", region.editing_position_3d);

                // let assets = rusterix.assets.clone();
                // rusterix
                //     .client
                //     .apply_entities_items_d3(&region.map, &assets);

                if let Some(hit) = self.terrain_hit {
                    rusterix.client.brush_preview = Some(BrushPreview {
                        position: hit,
                        radius: if server_ctx.curr_world_tool_helper == WorldToolHelper::Brushes {
                            self.radius
                        } else {
                            0.5
                        },
                        falloff: if server_ctx.curr_world_tool_helper == WorldToolHelper::Brushes {
                            self.falloff
                        } else {
                            0.5
                        },
                    });
                }
                rusterix.client.scene.dynamic_lights = vec![];
                rusterix.draw_d3(
                    &region.map,
                    buffer.pixels_mut(),
                    dim.width as usize,
                    dim.height as usize,
                );
                rusterix.client.brush_preview = None;

                self.hud.draw(
                    buffer,
                    &mut region.map,
                    ctx,
                    server_ctx,
                    None,
                    &rusterix.assets,
                );
            }
        }
    }

    pub fn map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        let mut hover = |coord: Vec2<i32>| {
            if let Some(render_view) = ui.get_render_view("PolyView") {
                let dim = *render_view.dim();

                // self.orbit_camera
                //     .set_parameter_vec3("center", self.camera_center);

                let rusterix = RUSTERIX.read().unwrap();
                let ray = rusterix.client.camera_d3.create_ray(
                    Vec2::new(
                        coord.x as f32 / dim.width as f32,
                        coord.y as f32 / dim.height as f32,
                    ),
                    Vec2::new(dim.width as f32, dim.height as f32),
                    Vec2::zero(),
                );

                self.terrain_hit = None;
                if let Some(hit) = map.terrain.ray_terrain_hit(&ray, 2000.0) {
                    let p = self.world_to_editor(map.terrain.scale, hit.world_pos);
                    server_ctx.hover_cursor = Some(p);
                    self.terrain_hit = Some(hit.world_pos);
                    server_ctx.hover_height =
                        Some(map.terrain.sample_height(hit.world_pos.x, hit.world_pos.z));
                }
            }
        };

        match &map_event {
            MapEvent::MapClicked(coord) => {
                self.drag_coord = *coord;
                self.undo_chunks = map.terrain.clone_chunks();
                self.edited = false;
                map.terrain.mark_clean();

                if !ui.logo && !ui.ctrl {
                    SCENEMANAGER
                        .write()
                        .unwrap()
                        .set_terrain_modifier_state(false);

                    if server_ctx.curr_world_tool_helper == WorldToolHelper::Brushes {
                        self.apply_brush = true;
                    }
                    // else {
                    //     self.apply_action(map, ui, server_ctx);
                    // }
                }
            }
            MapEvent::MapUp(_coord) => {
                self.apply_brush = false;
                if self.edited {
                    let cloned = map.terrain.clone_dirty_chunks();
                    if !cloned.is_empty() {
                        SCENEMANAGER
                            .write()
                            .unwrap()
                            .set_dirty_terrain_chunks(cloned);
                    }
                    map.terrain.mark_clean();
                    let undo_atom = RegionUndoAtom::TerrainEdit(
                        Box::new(self.undo_chunks.clone()),
                        Box::new(map.terrain.clone_chunks()),
                    );
                    UNDOMANAGER.write().unwrap().add_region_undo(
                        &server_ctx.curr_region,
                        undo_atom,
                        ctx,
                    );

                    self.undo_chunks = FxHashMap::default();
                    self.edited = false;

                    SCENEMANAGER
                        .write()
                        .unwrap()
                        .set_terrain_modifier_state(true);
                }
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Minimap"),
                    TheValue::Empty,
                ));
            }
            MapEvent::MapDragged(coord) => {
                hover(*coord);
                if ui.alt {
                    self.orbit_camera.zoom((*coord - self.drag_coord).y as f32);
                } else if ui.logo || ui.ctrl {
                    self.orbit_camera
                        .rotate((*coord - self.drag_coord).map(|v| v as f32 * 5.0));
                }

                self.drag_coord = *coord;
            }
            MapEvent::MapHover(coord) => hover(*coord),
            _ => {}
        }

        None
    }

    /*
    /// Applies the given action
    pub fn apply_action(&mut self, map: &mut Map, _ui: &TheUI, server_ctx: &mut ServerContext) {
        if let Some(hit) = self.terrain_hit {
            map.terrain.mark_clean();
            if server_ctx.curr_world_tool_helper == WorldToolHelper::MaterialPicker {
                if let Some(id) = server_ctx.curr_material_id {
                    let source = PixelSource::MaterialId(id);
                    let x = hit.x.floor() as i32;
                    let z = hit.z.floor() as i32;

                    if !self.tile_rules {
                        map.terrain.set_source(x, z, source);
                        let blend_mode = if self.blend_radius == 0 {
                            TerrainBlendMode::None
                        } else {
                            TerrainBlendMode::Blend(self.blend_radius as u8)
                        };
                        map.terrain.set_blend_mode(x, z, blend_mode);
                    } else {
                        self.apply_source_rules(x, z, map, source);
                    }
                    // rusterix.build_terrain_d3(map, true);
                    self.edited = true;
                }
            }
            let cloned = map.terrain.clone_dirty_chunks();
            if !cloned.is_empty() {
                SCENEMANAGER
                    .write()
                    .unwrap()
                    .set_dirty_terrain_chunks(cloned);
            }
            map.terrain.mark_clean();
        }
    }*/

    pub fn apply_source_rules(&mut self, x: i32, z: i32, map: &mut Map, source: PixelSource) {
        let world = Vec2::new(x as f32, z as f32);
        let height = map.terrain.get_height(x, z);
        let normal = map.terrain.sample_normal(world);
        let steepness = 1.0 - normal.y;

        for tile_ref in map.terrain.iter_tiles_mut() {
            // println!("{:?}", tile_ref.world_coords);
            unsafe {
                let chunk = &mut *tile_ref.chunk;
                let (wx, wz) = tile_ref.world_coords;
                let (lx, lz) = tile_ref.local_coords;
                let world_x = (chunk).origin.x + lx as i32;
                let world_z = (chunk).origin.y + lz as i32;

                let mut is_valid = false;
                if world.distance(Vec2::new(wx, wz)) < self.tile_rules_distance {
                    let local_height = chunk.get_height(world_x, world_z);
                    let local_normal = chunk.sample_normal(Vec2::new(world_x, world_z));

                    let diff = (local_height - height).abs(); // both in world units
                    let height_range = self.tile_rules_height;
                    // normalize diff into range [0.0..1.0] by mapping some max difference
                    // (e.g., 10.0 world units == range 1.0)
                    let normalized = (diff / 10.0).clamp(0.0, 1.0);
                    let height_match = normalized <= height_range;

                    let local_steepness = 1.0 - local_normal.y;
                    let steepness_diff = (local_steepness - steepness).abs();
                    let steepness_match = steepness_diff <= self.tile_rules_steepness;

                    is_valid = height_match && steepness_match;
                }

                if is_valid {
                    chunk.set_source(world_x, world_z, source.clone());
                    let blend_mode = if self.blend_radius == 0 {
                        TerrainBlendMode::None
                    } else {
                        TerrainBlendMode::Blend(self.blend_radius as u8)
                    };
                    chunk.set_blend_mode(world_x, world_z, blend_mode);
                    chunk.mark_dirty();
                    self.edited = true;
                }
            }
        }
    }

    /// Applies the current brush
    pub fn apply_brush(&mut self, terrain: &mut Terrain, center: Vec2<f32>, ui: &TheUI) {
        if self.brush_type == BrushType::Elevation {
            self.elevation_brush(
                terrain,
                center,
                self.radius,
                self.falloff,
                self.strength,
                !ui.shift,
            );
        } else if self.brush_type == BrushType::Fill {
            self.fill_brush(terrain, center, self.radius, ui.shift);
        } else if self.brush_type == BrushType::Smooth {
            self.smooth_brush(
                terrain,
                center,
                self.radius,
                self.falloff,
                self.strength,
                !ui.shift,
            );
        } else if self.brush_type == BrushType::Fractal {
            self.fractal_brush(terrain, center, self.radius, self.falloff, self.strength);
        } else if self.brush_type == BrushType::Fixed {
            self.fixed_brush(terrain, center, self.radius, self.falloff, self.fixed);
        }
    }

    /// Apply a circular brush to the terrain at a ray hit
    pub fn elevation_brush(
        &mut self,
        terrain: &mut Terrain,
        center: Vec2<f32>,
        radius: f32,
        falloff: f32,
        strength: f32,
        add: bool,
    ) {
        let radius2 = radius * radius;

        let min_x = ((center.x - radius) / terrain.scale.x).floor() as i32;
        let max_x = ((center.x + radius) / terrain.scale.x).ceil() as i32;
        let min_y = ((center.y - radius) / terrain.scale.y).floor() as i32;
        let max_y = ((center.y + radius) / terrain.scale.y).ceil() as i32;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let world_pos = Vec2::new(x as f32 * terrain.scale.x, y as f32 * terrain.scale.y);
                let dist2 = (world_pos - center).magnitude_squared();

                if dist2 <= radius2 {
                    let dist = dist2.sqrt();
                    let mut factor = 1.0 - (dist / radius);

                    // Apply falloff curve
                    factor = factor.powf(falloff.max(0.01));

                    let delta = strength * factor;
                    let current_height = terrain.get_height(x, y);

                    if add {
                        terrain.set_height(x, y, current_height + delta);
                    } else {
                        terrain.set_height(x, y, current_height - delta);
                    }
                    self.edited = true;
                }
            }
        }
    }

    /// Create terrain (fill missing cells with height 0.0) inside a circular brush.
    pub fn fill_brush(
        &mut self,
        terrain: &mut Terrain,
        center: Vec2<f32>,
        radius: f32,
        clear: bool,
    ) {
        let radius_squared = radius * radius;

        let min_x = ((center.x - radius) / terrain.scale.x).floor() as i32;
        let max_x = ((center.x + radius) / terrain.scale.x).ceil() as i32;
        let min_y = ((center.y - radius) / terrain.scale.y).floor() as i32;
        let max_y = ((center.y + radius) / terrain.scale.y).ceil() as i32;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let world_pos = Vec2::new(x as f32 * terrain.scale.x, y as f32 * terrain.scale.y);
                let dist_squared = (world_pos - center).magnitude_squared();

                if dist_squared <= radius_squared {
                    if !clear {
                        if !terrain.exists(x, y) {
                            terrain.set_height(x, y, 0.0);
                        }
                    } else {
                        terrain.remove_height(x, y);
                    }
                    self.edited = true;
                }
            }
        }
    }

    /// Smoothen brush
    pub fn smooth_brush(
        &mut self,
        terrain: &mut Terrain,
        center: Vec2<f32>,
        radius: f32,
        falloff: f32,
        strength: f32,
        smooth: bool,
    ) {
        let radius2 = radius * radius;

        let min_x = ((center.x - radius) / terrain.scale.x).floor() as i32;
        let max_x = ((center.x + radius) / terrain.scale.x).ceil() as i32;
        let min_y = ((center.y - radius) / terrain.scale.y).floor() as i32;
        let max_y = ((center.y + radius) / terrain.scale.y).ceil() as i32;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let world_pos = Vec2::new(x as f32 * terrain.scale.x, y as f32 * terrain.scale.y);
                let dist2 = (world_pos - center).magnitude_squared();

                if dist2 <= radius2 {
                    let dist = dist2.sqrt();
                    let mut factor = 1.0 - (dist / radius);

                    // Apply falloff curve
                    factor = factor.powf(falloff.max(0.01));

                    let center_height = terrain.get_height(x, y);
                    let mut neighbor_sum = 0.0;
                    let mut neighbor_count = 0.0;

                    // Average 8 neighbors (or 4 if you want simpler)
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            neighbor_sum += terrain.get_height(x + dx, y + dy);
                            neighbor_count += 1.0;
                        }
                    }

                    if neighbor_count > 0.0 {
                        let neighbor_avg = neighbor_sum / neighbor_count;
                        let delta = (neighbor_avg - center_height) * strength * factor;

                        let new_height = if smooth {
                            center_height + delta
                        } else {
                            center_height - delta
                        };

                        terrain.set_height(x, y, new_height);
                        self.edited = true;
                    }
                }
            }
        }
    }

    /// Fractal brush
    pub fn fractal_brush(
        &mut self,
        terrain: &mut Terrain,
        center: Vec2<f32>,
        radius: f32,
        falloff: f32,
        strength: f32,
    ) {
        use rand::Rng;
        let mut rng = rand::rng();

        let radius2 = radius * radius;

        let min_x = ((center.x - radius) / terrain.scale.x).floor() as i32;
        let max_x = ((center.x + radius) / terrain.scale.x).ceil() as i32;
        let min_y = ((center.y - radius) / terrain.scale.y).floor() as i32;
        let max_y = ((center.y + radius) / terrain.scale.y).ceil() as i32;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let world_pos = Vec2::new(x as f32 * terrain.scale.x, y as f32 * terrain.scale.y);
                let dist2 = (world_pos - center).magnitude_squared();

                if dist2 <= radius2 {
                    let dist = dist2.sqrt();
                    let mut factor = 1.0 - (dist / radius);

                    // Apply falloff
                    factor = factor.powf(falloff.max(0.01));

                    // Generate a small random offset
                    let noise = (rng.random::<f32>() * 2.0 - 1.0) * strength * factor;

                    let current_height = terrain.get_height(x, y);
                    terrain.set_height(x, y, current_height + noise);
                    self.edited = true;
                }
            }
        }
    }

    /// Apply a circular fixed-height brush to the terrain at a ray hit
    pub fn fixed_brush(
        &mut self,
        terrain: &mut Terrain,
        center: Vec2<f32>,
        radius: f32,
        falloff: f32,
        target_height: f32,
    ) {
        let radius2 = radius * radius;

        let min_x = ((center.x - radius) / terrain.scale.x).floor() as i32;
        let max_x = ((center.x + radius) / terrain.scale.x).ceil() as i32;
        let min_y = ((center.y - radius) / terrain.scale.y).floor() as i32;
        let max_y = ((center.y + radius) / terrain.scale.y).ceil() as i32;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let world_pos = Vec2::new(x as f32 * terrain.scale.x, y as f32 * terrain.scale.y);
                let dist2 = (world_pos - center).magnitude_squared();

                if dist2 <= radius2 {
                    let dist = dist2.sqrt();
                    let mut factor = 1.0 - (dist / radius);

                    // Apply falloff curve
                    factor = factor.powf(falloff.max(0.01)).clamp(0.0, 1.0);

                    let current_height = terrain.get_height(x, y);
                    let new_height = current_height * (1.0 - factor) + target_height * factor;

                    terrain.set_height(x, y, new_height);
                    self.edited = true;
                }
            }
        }
    }

    fn world_to_editor(&self, grid_scale: Vec2<f32>, world_pos: Vec3<f32>) -> Vec2<f32> {
        Vec2::new(world_pos.x / grid_scale.x, -world_pos.z / grid_scale.y)
    }

    pub fn scroll_by(
        &mut self,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
        coord: Vec2<i32>,
    ) {
        self.orbit_camera.zoom(coord.y as f32);
    }

    /// Create a preview of the brush
    pub fn update_brush_preview(&self, ui: &mut TheUI) {
        if let Some(render_view) = ui.get_render_view("Brush Preview") {
            let dim = *render_view.dim();

            let buffer = render_view.render_buffer_mut();
            buffer.resize(dim.width, dim.height);

            let width = buffer.dim().width as usize;
            let height = buffer.dim().height as usize;

            let falloff = self.falloff.max(0.01);

            buffer
                .pixels_mut()
                .par_chunks_exact_mut(width * 4)
                .enumerate()
                .for_each(|(j, line)| {
                    for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                        let x = i as f32 / width as f32;
                        let y = j as f32 / height as f32;

                        let dx = x - 0.5;
                        let dy = y - 0.5;
                        let dist = (dx * dx + dy * dy).sqrt() * 2.0;

                        let mut strength = (1.0 - dist).clamp(0.0, 1.0);
                        strength = strength.powf(falloff);

                        let color = Vec4::new(strength, strength, strength, 1.0);
                        pixel.copy_from_slice(&TheColor::from_vec4f(color).to_u8_array());
                    }
                });
        }
    }

    pub fn set_tile_rules_ui(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        switch_to_nodes: bool,
    ) {
        let mut nodeui = TheNodeUI::default();

        let item = TheNodeUIItem::IntEditSlider(
            "tileRulesBlendRadius".into(),
            "Blend Radius".into(),
            "Controls how far neighboring tiles influence blending. A value of 0 disables blending.".into(),
            self.blend_radius,
            0..=3,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Selector(
            "tileRules".into(),
            "Use Rules".into(),
            "Enable or disable rule-based tile painting.".into(),
            vec!["Yes".to_string(), "No".to_string()],
            if self.tile_rules { 0 } else { 1 },
        );
        nodeui.add_item(item);

        nodeui.add_item(TheNodeUIItem::Separator("Tile Rules".into()));

        let item = TheNodeUIItem::FloatEditSlider(
            "tileRulesDistance".into(),
            "Distance".into(),
            "Affects tiles within this radius (in world units) around the painted point.".into(),
            self.tile_rules_distance,
            1.0..=100.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "tileRulesHeight".into(),
            "Height".into(),
            "Controls how much height difference is allowed from the painted tile (0 = exact match, 1 = any height).".into(),
            self.tile_rules_height,
            0.0..=1.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "tileRulesSteepness".into(),
            "Steepness".into(),
            "Controls how much steepness variation is allowed from the painted tile (0 = same steepness, 1 = any slope).".into(),
            self.tile_rules_steepness,
            0.0..=1.0,
            false,
        );
        nodeui.add_item(item);

        if let Some(layout) = ui.get_text_layout("Node Settings") {
            nodeui.apply_to_text_layout(layout);
            // layout.relayout(ctx);
            ctx.ui.relayout = true;

            if switch_to_nodes {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Show Node Settings"),
                    TheValue::Text("Tile Rules Settings".into()),
                ));
            }
        }
    }
}
