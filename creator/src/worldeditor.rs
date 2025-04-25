use crate::editor::{PALETTE, RUSTERIX, UNDOMANAGER};
use crate::prelude::*;
use shared::prelude::*;

use rusterix::{D3Camera, D3OrbitCamera, ValueContainer};

pub struct WorldEditor {
    orbit_camera: D3OrbitCamera,

    drag_coord: Vec2<i32>,
}

#[allow(clippy::new_without_default)]
impl WorldEditor {
    pub fn new() -> Self {
        Self {
            orbit_camera: D3OrbitCamera::new(),
            drag_coord: Vec2::zero(),
        }
    }

    pub fn build(&mut self) -> TheCanvas {
        let mut center = TheCanvas::new();

        // Toolbar
        let mut top_toolbar = TheCanvas::new();
        top_toolbar.set_widget(TheTraybar::new(TheId::empty()));

        let mut toolbar_hlayout = TheHLayout::new(TheId::named("Material Tool Params"));
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(10, 4, 5, 4));

        /*
        for i in 0..20 {
            let mut icon = TheIconView::new(TheId::named(&format!("Material Icon #{}", i)));
            // ground_icon.set_text(Some("FLOOR".to_string()));
            // ground_icon.set_text_size(10.0);
            // ground_icon.set_text_color([200, 200, 200, 255]);
            icon.limiter_mut().set_max_size(Vec2::new(20, 20));
            icon.set_border_color(Some(BLACK));

            toolbar_hlayout.add_widget(Box::new(icon));
        }*/

        let mut create_button = TheTraybarButton::new(TheId::named("Create Graph Button"));
        create_button.set_status_text("Apply the source to the selected geometry.");
        create_button.set_text("Create Graph".to_string());
        toolbar_hlayout.add_widget(Box::new(create_button));

        let mut nodes_button = TheTraybarButton::new(TheId::named("ShapeFX Nodes"));
        //add_button.set_icon_name("icon_role_add".to_string());
        nodes_button.set_text(str!("Nodes"));
        nodes_button.set_status_text("Available region effect nodes.");
        nodes_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Color".to_string(), TheId::named("Color")),
                TheContextMenuItem::new("Gradient".to_string(), TheId::named("Gradient")),
                TheContextMenuItem::new("Outline".to_string(), TheId::named("Outline")),
                TheContextMenuItem::new("Glow".to_string(), TheId::named("Glow")),
                TheContextMenuItem::new("Noise Overlay".to_string(), TheId::named("Noise Overlay")),
            ],
            ..Default::default()
        }));

        // let mut nodes_drop_down = TheDropdownMenu::new(TheId::named("Nodes Selector"));
        // for role in ShapeFXRole::iterator() {
        //     if role != ShapeFXRole::Geometry {
        //         nodes_drop_down.add_option(role.to_string());
        //     }
        // }
        toolbar_hlayout.add_widget(Box::new(nodes_button));

        toolbar_hlayout.set_reverse_index(Some(2));
        top_toolbar.set_layout(toolbar_hlayout);
        center.set_top(top_toolbar);

        let mut material_node_canvas = TheCanvas::new();
        let node_view = TheNodeCanvasView::new(TheId::named("ShapeFX NodeCanvas"));
        material_node_canvas.set_widget(node_view);

        center.set_center(material_node_canvas);

        center
    }

    pub fn draw(
        &mut self,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
        build_values: &mut ValueContainer,
    ) {
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
                rusterix.build_scene_d3(&region.map, build_values);
                let assets = rusterix.assets.clone();
                rusterix
                    .client
                    .apply_entities_items_d3(&region.map, &assets);
                rusterix.client.draw_d3(
                    &region.map,
                    buffer.pixels_mut(),
                    dim.width as usize,
                    dim.height as usize,
                );
            }
        }
    }

    pub fn map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        _map: &mut Map,
        _server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        match &map_event {
            MapEvent::MapClicked(coord) => {
                self.drag_coord = *coord;
            }
            MapEvent::MapDragged(coord) => {
                if ui.alt {
                    self.orbit_camera.zoom((*coord - self.drag_coord).y as f32);
                } else {
                    self.orbit_camera
                        .rotate((*coord - self.drag_coord).map(|v| v as f32 * 5.0));
                }

                self.drag_coord = *coord;
            }
            _ => {}
        }

        None
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
}
