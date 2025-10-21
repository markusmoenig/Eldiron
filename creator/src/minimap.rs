use crate::prelude::*;
use rusterix::prelude::*;
use vek::Vec2;

use crate::editor::{PALETTE, RUSTERIX, SIDEBARMODE};

pub static MINIMAPBUFFER: LazyLock<RwLock<TheRGBABuffer>> =
    LazyLock::new(|| RwLock::new(TheRGBABuffer::default()));

pub static MINIMAPBOX: LazyLock<RwLock<Vec4<f32>>> = LazyLock::new(|| RwLock::new(Vec4::one()));

pub fn draw_camera_marker(region: &Region, buffer: &mut TheRGBABuffer, server_ctx: &ServerContext) {
    let camera_pos = Vec2::new(region.editing_position_3d.x, region.editing_position_3d.z);

    let dim = *buffer.dim();
    let bbox = *MINIMAPBOX.read().unwrap();

    let pos = world_to_minimap_pixel(
        camera_pos,
        Vec2::new(dim.width as f32, dim.height as f32),
        bbox,
    );

    let w = 4;
    buffer.draw_rect_outline(
        &TheDim::rect(pos.x as i32 - w, pos.y as i32 - w, w * 2, w * 2),
        &vek::Rgba::red().into_array(),
    );

    if server_ctx.editor_view_mode == EditorViewMode::FirstP {
        let look_at_pos = Vec2::new(region.editing_look_at_3d.x, region.editing_look_at_3d.z);

        let pos = world_to_minimap_pixel(
            look_at_pos,
            Vec2::new(dim.width as f32, dim.height as f32),
            bbox,
        );

        buffer.draw_rect_outline(
            &TheDim::rect(pos.x as i32 - w, pos.y as i32 - w, w * 2, w * 2),
            &vek::Rgba::yellow().into_array(),
        );
    }
}

pub fn draw_material_minimap(
    buffer: &mut TheRGBABuffer,
    project: &Project,
    server_ctx: &ServerContext,
) {
    if let Some(shader_id) = server_ctx.curr_material_id {
        if let Some(shader) = project.shaders.get(&shader_id) {
            crate::utils::draw_shader_into(shader, buffer);
        }
    }
}

pub fn draw_minimap(
    orig_region: &Region,
    buffer: &mut TheRGBABuffer,
    server_ctx: &ServerContext,
    hard: bool,
) {
    if *SIDEBARMODE.read().unwrap() == SidebarMode::Palette {
        buffer.render_hsl_hue_waveform();

        if let Some(color) = PALETTE.read().unwrap().get_current_color() {
            if let Some(pos) = buffer.find_closest_color_position(color.to_u8_array_3()) {
                let w = 4;
                buffer.draw_rect_outline(
                    &TheDim::rect(pos.x - w, pos.y - w, w * 2, w * 2),
                    &vek::Rgba::white().into_array(),
                );
            }
        }

        return;
    }

    if !hard {
        buffer.copy_into(0, 0, &MINIMAPBUFFER.read().unwrap());
        draw_camera_marker(orig_region, buffer, server_ctx);
        return;
    }

    let dim = buffer.dim();
    // println!("hard update");

    let width = dim.width as f32;
    let height = dim.height as f32;
    let background = [42, 42, 42, 255];

    let mut region = orig_region.clone();
    if let Some(mut bbox) = region.map.bounding_box() {
        if let Some(tbbox) = region.map.terrain.compute_bounds() {
            let bbox_min = Vec2::new(bbox.x, bbox.y);
            let bbox_max = bbox_min + Vec2::new(bbox.z, bbox.w);

            let new_min = bbox_min.map2(tbbox.min, f32::min);
            let new_max = bbox_max.map2(tbbox.max, f32::max);

            bbox.x = new_min.x;
            bbox.y = new_min.y;
            bbox.z = new_max.x - new_min.x;
            bbox.w = new_max.y - new_min.y;
        }

        bbox.x -= 0.5;
        bbox.y -= 0.5;
        bbox.z += 1.0;
        bbox.w += 1.0;

        *MINIMAPBOX.write().unwrap() = bbox;

        let scale_x = width / bbox.z;
        let scale_y = height / bbox.w;

        region.map.selected_linedefs.clear();
        region.map.selected_sectors.clear();
        region.map.grid_size = scale_x.min(scale_y);
        region.map.camera = MapCamera::TwoD;

        // Compute the center of the bounding box
        let bbox_center_x = bbox.x + bbox.z / 2.0;
        let bbox_center_y = bbox.y + bbox.w / 2.0;

        // Compute the offset to center the map
        region.map.offset.x = -bbox_center_x * region.map.grid_size;
        region.map.offset.y = bbox_center_y * region.map.grid_size;

        let mut rusterix = RUSTERIX.write().unwrap();
        let translation_matrix = Mat3::<f32>::translation_2d(Vec2::new(
            region.map.offset.x + width / 2.0,
            -region.map.offset.y + height / 2.0,
        ));
        let scale_matrix = Mat3::new(
            region.map.grid_size,
            0.0,
            0.0,
            0.0,
            region.map.grid_size,
            0.0,
            0.0,
            0.0,
            1.0,
        );

        let transform = translation_matrix * scale_matrix;
        let assets = rusterix.assets.clone();

        let old_grid_pos = region.map.curr_grid_pos;
        region.map.curr_grid_pos = None;

        let anim_frame = rusterix.client.scene.animation_frame;
        rusterix.client.scene.animation_frame = 0;
        Rasterizer::setup(Some(transform), Mat4::identity(), Mat4::identity())
            .background(background)
            .ambient(Vec4::one())
            .render_mode(RenderMode::render_2d().ignore_background_shader(true))
            .rasterize(
                &mut rusterix.client.scene,
                buffer.pixels_mut(),
                width as usize,
                height as usize,
                40,
                &assets,
            );
        rusterix.client.scene.animation_frame = anim_frame;
        MINIMAPBUFFER
            .write()
            .unwrap()
            .resize(buffer.dim().width, buffer.dim().height);

        MINIMAPBUFFER.write().unwrap().copy_into(0, 0, buffer);
        draw_camera_marker(orig_region, buffer, server_ctx);

        region.map.curr_grid_pos = old_grid_pos;
    } else {
        buffer.fill(background);
    }
}

fn world_to_minimap_pixel(
    world_pos: Vec2<f32>,
    render_dim: Vec2<f32>,
    bbox: Vec4<f32>, // x, y, w, h
) -> Vec2<f32> {
    let width = render_dim.x;
    let height = render_dim.y;

    let scale_x = width / bbox.z;
    let scale_y = height / bbox.w;
    let grid_size = scale_x.min(scale_y);

    let bbox_center_x = bbox.x + bbox.z / 2.0;
    let bbox_center_y = bbox.y + bbox.w / 2.0;

    let offset_x = -bbox_center_x * grid_size;
    let offset_y = bbox_center_y * grid_size;

    let pixel_x = (world_pos.x * grid_size) + offset_x + (width / 2.0);
    let pixel_y = (-world_pos.y * grid_size) + offset_y + (height / 2.0);

    Vec2::new(pixel_x, render_dim.y - pixel_y)
}
