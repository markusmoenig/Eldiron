use crate::editor::{PALETTE, RUSTERIX, UNDOMANAGER};
use crate::hud::{Hud, HudMode};
use crate::prelude::*;
use rayon::prelude::*;
use shared::prelude::*;

use rusterix::{D3Camera, D3OrbitCamera, PixelSource, Terrain, TerrainChunk, ValueContainer};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BrushType {
    Elevation,
    Fill,
    Smooth,
    Fractal,
}

impl BrushType {
    pub fn set_from_index(&mut self, index: usize) {
        match index {
            1 => *self = BrushType::Fill,
            2 => *self = BrushType::Smooth,
            3 => *self = BrushType::Fractal,
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

    hud: Hud,

    pub first_draw: bool,
    apply_brush: bool,

    undo_chunks: FxHashMap<(i32, i32), TerrainChunk>,
    edited: bool,
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

            hud: Hud::new(HudMode::Terrain),

            first_draw: true,
            apply_brush: false,

            undo_chunks: FxHashMap::default(),
            edited: false,
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
        brush_switch.set_item_width(80);
        text_layout.add_pair("".to_string(), Box::new(brush_switch));

        let mut radius = TheTextLineEdit::new(TheId::named("Brush Radius"));
        radius.set_value(TheValue::Float(10.0));
        radius.set_range(TheValue::RangeF32(0.5..=10.0));
        // radius.set_continuous(true);
        radius.set_status_text("The falloff of the brush.");
        radius.limiter_mut().set_max_width(200);
        text_layout.add_pair("Radius".to_string(), Box::new(radius));

        let mut falloff = TheTextLineEdit::new(TheId::named("Brush Falloff"));
        falloff.set_value(TheValue::Float(2.0));
        falloff.set_range(TheValue::RangeF32(0.5..=4.0));
        // falloff.set_continuous(true);
        falloff.set_status_text("The falloff of the brush.");
        falloff.limiter_mut().set_max_width(200);
        text_layout.add_pair("Falloff".to_string(), Box::new(falloff));

        let mut strength = TheTextLineEdit::new(TheId::named("Brush Strength"));
        strength.set_value(TheValue::Float(0.2));
        strength.set_range(TheValue::RangeF32(0.01..=1.0));
        // strength.set_continuous(true);
        strength.set_status_text("The falloff of the brush.");
        strength.limiter_mut().set_max_width(200);
        text_layout.add_pair("Strength".to_string(), Box::new(strength));

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
        build_values: &mut ValueContainer,
    ) {
        if self.apply_brush {
            if let Some(hit) = self.terrain_hit {
                if let Some(map) = project.get_map_mut(server_ctx) {
                    self.apply_brush(&mut map.terrain, Vec2::new(hit.x, hit.z), ui);
                    let mut rusterix = RUSTERIX.write().unwrap();
                    rusterix.build_terrain_d3(map, &ValueContainer::default());
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

                region.map.properties.remove("fog_enabled");
                if self.first_draw {
                    rusterix.build_scene_d3(&region.map, build_values);
                    rusterix.build_terrain_d3(&mut region.map, &ValueContainer::default());
                    self.first_draw = false;
                }

                // let assets = rusterix.assets.clone();
                // rusterix
                //     .client
                //     .apply_entities_items_d3(&region.map, &assets);
                rusterix.client.draw_d3(
                    &region.map,
                    buffer.pixels_mut(),
                    dim.width as usize,
                    dim.height as usize,
                );

                self.hud.draw(
                    buffer,
                    &mut region.map,
                    ctx,
                    server_ctx,
                    None,
                    &PALETTE.read().unwrap(),
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
                if let Some(hit) = map.terrain.ray_terrain_hit(&ray, 100.0) {
                    let p = self.world_to_editor(map.terrain.scale, hit.world_pos);
                    server_ctx.hover_cursor = Some(p);
                    self.terrain_hit = Some(hit.world_pos);
                }
            }
        };

        match &map_event {
            MapEvent::MapClicked(coord) => {
                self.drag_coord = *coord;
                self.undo_chunks = map.terrain.clone_chunks_clean();
                self.edited = false;

                if server_ctx.curr_world_tool_helper == WorldToolHelper::Brushes {
                    if !ui.logo {
                        self.apply_brush = true;
                    }
                } else {
                    self.apply_action(map, ui, server_ctx);
                }
            }
            MapEvent::MapUp(_coord) => {
                self.apply_brush = false;
                if self.edited {
                    let undo_atom = RegionUndoAtom::TerrainEdit(
                        Box::new(self.undo_chunks.clone()),
                        Box::new(map.terrain.clone_chunks_clean()),
                    );
                    UNDOMANAGER.write().unwrap().add_region_undo(
                        &server_ctx.curr_region,
                        undo_atom,
                        ctx,
                    );

                    self.undo_chunks = FxHashMap::default();
                    self.edited = false;
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
                } else if ui.logo {
                    self.orbit_camera
                        .rotate((*coord - self.drag_coord).map(|v| v as f32 * 5.0));
                } else {
                    self.apply_action(map, ui, server_ctx);
                }

                self.drag_coord = *coord;
            }
            MapEvent::MapHover(coord) => hover(*coord),
            _ => {}
        }

        None
    }

    /// Applies the given action
    pub fn apply_action(&mut self, map: &mut Map, ui: &TheUI, server_ctx: &mut ServerContext) {
        if let Some(hit) = self.terrain_hit {
            let mut rusterix = RUSTERIX.write().unwrap();

            if server_ctx.curr_world_tool_helper == WorldToolHelper::Brushes {
                self.apply_brush(&mut map.terrain, Vec2::new(hit.x, hit.z), ui);
                rusterix.build_terrain_d3(map, &ValueContainer::default());
            } else if server_ctx.curr_world_tool_helper == WorldToolHelper::MaterialPicker {
                if let Some(id) = server_ctx.curr_material_id {
                    let source = PixelSource::MaterialId(id);
                    map.terrain.set_source(hit.x as i32, hit.z as i32, source);
                    // rusterix.set_dirty();
                    rusterix.build_terrain_d3(map, &ValueContainer::default());
                }
            }
            map.terrain.mark_clean();
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
}
