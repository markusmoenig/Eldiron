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
    trace: bool,
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
            trace: false,
        }
    }

    pub fn build_trace_canvas(&mut self) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut text_layout = TheTextLayout::new(TheId::named("Brush Settings"));

        let mut trace_button = TheTraybarButton::new(TheId::named("Trace Button"));
        trace_button.set_text("Start Trace".into());
        trace_button.limiter_mut().set_min_width(120);
        text_layout.add_pair("Trace".into(), Box::new(trace_button));

        center.set_layout(text_layout);

        /*
        let mut preview_canvas: TheCanvas = TheCanvas::new();
        let mut render_view = TheRenderView::new(TheId::named("Brush Preview"));
        render_view.limiter_mut().set_max_size(Vec2::new(300, 300));
        preview_canvas.set_widget(render_view);

        center.set_right(preview_canvas);*/

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
                rusterix.client.scene_d3.dynamic_lights = vec![];
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
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _map: &mut Map,
        _server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        match &map_event {
            MapEvent::MapClicked(coord) => {
                self.drag_coord = *coord;
                self.edited = false;
            }
            MapEvent::MapUp(_coord) => {}
            _ => {}
        }

        None
    }

    pub fn switch_trace(&mut self) {
        println!("trace");
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
