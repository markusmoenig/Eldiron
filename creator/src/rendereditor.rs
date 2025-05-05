use crate::editor::{CUSTOMCAMERA, PALETTE, RUSTERIX};
use crate::hud::{Hud, HudMode};
use crate::prelude::*;
use shared::prelude::*;

use rusterix::ValueContainer;

pub enum RenderMoveAction {
    Forward,
    Backward,
    Left,
    Right,
}

pub struct RenderEditor {
    terrain_hit: Option<Vec3<f32>>,
    drag_coord: Vec2<i32>,

    hud: Hud,

    pub first_draw: bool,

    edited: bool,
}

#[allow(clippy::new_without_default)]
impl RenderEditor {
    pub fn new() -> Self {
        Self {
            terrain_hit: None,
            drag_coord: Vec2::zero(),

            hud: Hud::new(HudMode::Terrain),

            first_draw: true,

            edited: false,
        }
    }

    pub fn draw(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
        build_values: &mut ValueContainer,
    ) {
        if let Some(render_view) = ui.get_render_view("PolyView") {
            let dim = *render_view.dim();

            let buffer = render_view.render_buffer_mut();
            buffer.resize(dim.width, dim.height);

            if let Some(region) = project.get_region_ctx_mut(server_ctx) {
                CUSTOMCAMERA
                    .write()
                    .unwrap()
                    .update_action(region, server_ctx);
                CUSTOMCAMERA
                    .write()
                    .unwrap()
                    .update_camera(region, server_ctx);

                let mut rusterix = RUSTERIX.write().unwrap();
                if self.first_draw {
                    rusterix.build_scene_d3(&region.map, build_values);
                    rusterix.build_terrain_d3(&mut region.map, &ValueContainer::default());
                    self.first_draw = false;
                }

                // let assets = rusterix.assets.clone();
                // rusterix
                //     .client
                //     .apply_entities_items_d3(&region.map, &assets);

                if let Some(hit) = self.terrain_hit {
                    rusterix.client.terrain_hover = Some(hit);
                }
                rusterix.client.draw_d3(
                    &region.map,
                    buffer.pixels_mut(),
                    dim.width as usize,
                    dim.height as usize,
                );
                rusterix.client.terrain_hover = None;

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
        _ctx: &mut TheContext,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        let mut hover = |coord: Vec2<i32>| {
            if let Some(render_view) = ui.get_render_view("PolyView") {
                let dim = *render_view.dim();

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
                    server_ctx.hover_height =
                        Some(map.terrain.sample_height(hit.world_pos.x, hit.world_pos.z));
                }
            }
        };

        match &map_event {
            MapEvent::MapClicked(coord) => {
                self.drag_coord = *coord;
                self.edited = false;
            }
            MapEvent::MapUp(_coord) => {}
            MapEvent::MapDragged(coord) => {
                hover(*coord);
                self.drag_coord = *coord;
            }
            MapEvent::MapHover(coord) => hover(*coord),
            _ => {}
        }

        None
    }

    fn world_to_editor(&self, grid_scale: Vec2<f32>, world_pos: Vec3<f32>) -> Vec2<f32> {
        Vec2::new(world_pos.x / grid_scale.x, -world_pos.z / grid_scale.y)
    }

    pub fn scroll_by(
        &mut self,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
        _coord: Vec2<i32>,
    ) {
        // self.orbit_camera.zoom(coord.y as f32);
    }
}
