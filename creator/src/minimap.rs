use crate::prelude::*;
use rusterix::{Surface, ValueContainer};
use std::collections::hash_map::DefaultHasher;
use vek::Vec2;

use crate::editor::{PALETTE, RUSTERIX, SIDEBARMODE};
use std::hash::{Hash, Hasher};

pub static MINIMAPBUFFER: LazyLock<RwLock<TheRGBABuffer>> =
    LazyLock::new(|| RwLock::new(TheRGBABuffer::default()));

pub static MINIMAPBOX: LazyLock<RwLock<Vec4<f32>>> = LazyLock::new(|| RwLock::new(Vec4::one()));
pub static MINIMAPCACHEKEY: LazyLock<RwLock<u64>> = LazyLock::new(|| RwLock::new(0));

fn minimap_context_key(server_ctx: &ServerContext) -> u64 {
    let mut hasher = DefaultHasher::default();
    match server_ctx.get_map_context() {
        MapContext::Region => 0_u8.hash(&mut hasher),
        MapContext::Screen => 1_u8.hash(&mut hasher),
        MapContext::Character => 2_u8.hash(&mut hasher),
        MapContext::Item => 3_u8.hash(&mut hasher),
    }
    server_ctx.curr_region.hash(&mut hasher);
    server_ctx.curr_screen.hash(&mut hasher);
    if let Some(surface) = &server_ctx.editing_surface {
        surface.id.hash(&mut hasher);
    }
    hasher.finish()
}

pub fn minimap_bbox_for_map(map: &Map) -> Option<Vec4<f32>> {
    let mut bbox = map.bounding_box()?;
    if let Some(tbbox) = map.terrain.compute_bounds() {
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
    Some(bbox)
}

fn surface_uv_outline(surface: &Surface) -> Option<Vec<Vec2<f32>>> {
    if surface.world_vertices.len() < 2 {
        return None;
    }
    let mut points = Vec::with_capacity(surface.world_vertices.len());
    for p in &surface.world_vertices {
        let mut uv = surface.world_to_uv(*p);
        uv.y = -uv.y;
        points.push(uv);
    }
    Some(points)
}

fn minimap_bbox_for_surface(surface: &Surface) -> Option<Vec4<f32>> {
    let points = surface_uv_outline(surface)?;
    let mut min = points[0];
    let mut max = points[0];
    for p in points.iter().skip(1) {
        min = min.map2(*p, f32::min);
        max = max.map2(*p, f32::max);
    }
    let mut bbox = Vec4::new(min.x, min.y, max.x - min.x, max.y - min.y);
    bbox.x -= 0.5;
    bbox.y -= 0.5;
    bbox.z += 1.0;
    bbox.w += 1.0;
    Some(bbox)
}

fn draw_surface_outline_on_minimap(buffer: &mut TheRGBABuffer, surface: &Surface, bbox: Vec4<f32>) {
    let Some(points) = surface_uv_outline(surface) else {
        return;
    };
    if points.len() < 2 {
        return;
    }
    let dim = *buffer.dim();
    let render_dim = Vec2::new(dim.width as f32, dim.height as f32);
    let line_color = [235, 235, 235, 255];
    for i in 0..points.len() {
        let p0 = world_to_minimap_pixel(points[i], render_dim, bbox);
        let p1 = world_to_minimap_pixel(points[(i + 1) % points.len()], render_dim, bbox);
        buffer.draw_line(
            p0.x.round() as i32,
            p0.y.round() as i32,
            p1.x.round() as i32,
            p1.y.round() as i32,
            line_color,
        );
    }
}

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

pub fn draw_minimap_context_label(
    buffer: &mut TheRGBABuffer,
    ctx: &mut TheContext,
    server_ctx: &ServerContext,
) {
    let label = if server_ctx.get_map_context() == MapContext::Region {
        if server_ctx.editing_surface.is_some() {
            "Profile"
        } else {
            "Region"
        }
    } else if server_ctx.get_map_context() == MapContext::Screen {
        "Screen"
    } else if server_ctx.get_map_context() == MapContext::Character {
        "Character"
    } else if server_ctx.get_map_context() == MapContext::Item {
        "Item"
    } else {
        "Map"
    };

    let stride = buffer.stride();
    let bg = [36, 36, 36, 180];
    let fg = [215, 215, 215, 255];
    let text_pad_left = 4;
    let approx_char_w = 7;
    let label_w = (label.len() as i32 * approx_char_w + text_pad_left * 2).max(24);
    ctx.draw.rect(
        buffer.pixels_mut(),
        &(0, 0, label_w as usize, 16),
        stride,
        &bg,
    );
    ctx.draw.text_rect(
        buffer.pixels_mut(),
        &(
            text_pad_left as usize,
            0,
            (label_w - text_pad_left) as usize,
            16,
        ),
        stride,
        label,
        TheFontSettings {
            size: 11.0,
            ..Default::default()
        },
        &fg,
        &bg,
        TheHorizontalAlign::Left,
        TheVerticalAlign::Center,
    );
}

pub fn draw_minimap(
    project: &Project,
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

    let cache_key = minimap_context_key(server_ctx);
    let mut hard = hard;
    if !hard {
        let cached_key = *MINIMAPCACHEKEY.read().unwrap();
        let cached = MINIMAPBUFFER.read().unwrap();
        let cached_dim = *cached.dim();
        let dim = *buffer.dim();
        if cached_key != cache_key
            || cached_dim.width != dim.width
            || cached_dim.height != dim.height
        {
            hard = true;
        }
    }

    let region_marker = if server_ctx.get_map_context() == MapContext::Region
        && server_ctx.editing_surface.is_none()
    {
        project.get_region(&server_ctx.curr_region)
    } else {
        None
    };

    if !hard {
        buffer.copy_into(0, 0, &MINIMAPBUFFER.read().unwrap());
        if let Some(region) = region_marker {
            draw_camera_marker(region, buffer, server_ctx);
        }
        return;
    }

    let dim = buffer.dim();

    let width = dim.width as f32;
    let height = dim.height as f32;
    let background = [42, 42, 42, 255];

    if let Some(map) = project.get_map(server_ctx) {
        let bbox_from_surface = server_ctx
            .editing_surface
            .as_ref()
            .and_then(minimap_bbox_for_surface);
        let Some(bbox) = minimap_bbox_for_map(map).or(bbox_from_surface) else {
            buffer.fill(background);
            return;
        };

        *MINIMAPBOX.write().unwrap() = bbox;

        let scale_x = width / bbox.z;
        let scale_y = height / bbox.w;

        let rusterix = RUSTERIX.write().unwrap();
        let mut map_copy = map.clone();
        map_copy.selected_linedefs.clear();
        map_copy.selected_sectors.clear();
        map_copy.grid_size = scale_x.min(scale_y);
        map_copy.camera = MapCamera::TwoD;

        let bbox_center_x = bbox.x + bbox.z / 2.0;
        let bbox_center_y = bbox.y + bbox.w / 2.0;
        map_copy.offset.x = -bbox_center_x * scale_x;
        map_copy.offset.y = bbox_center_y * scale_y;

        let translation_matrix = Mat3::<f32>::translation_2d(Vec2::new(
            map_copy.offset.x + width / 2.0,
            -map_copy.offset.y + height / 2.0,
        ));
        let scale_matrix = Mat3::new(scale_x, 0.0, 0.0, 0.0, scale_y, 0.0, 0.0, 0.0, 1.0);
        let transform = translation_matrix * scale_matrix;

        let mut builder = rusterix::D2PreviewBuilder::new();
        let mut scene = builder.build(
            &map_copy,
            &rusterix.assets,
            Vec2::new(width, height),
            &ValueContainer::default(),
        );
        rusterix::Rasterizer::setup(Some(transform), Mat4::identity(), Mat4::identity())
            .background(background)
            .ambient(Vec4::one())
            .render_mode(rusterix::RenderMode::render_2d().ignore_background_shader(true))
            .rasterize(
                &mut scene,
                buffer.pixels_mut(),
                width as usize,
                height as usize,
                40,
                &rusterix.assets,
            );

        // Only overlay linedefs while editing a profile surface in D2 mode.
        let show_profile_lines = server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode == EditorViewMode::D2
            && server_ctx.editing_surface.is_some();
        if show_profile_lines && !map_copy.linedefs.is_empty() {
            let dim = Vec2::new(width, height);
            let line_color = [235, 235, 235, 255];
            for linedef in &map_copy.linedefs {
                if let Some(start) = map_copy.get_vertex(linedef.start_vertex)
                    && let Some(end) = map_copy.get_vertex(linedef.end_vertex)
                {
                    let p0 = world_to_minimap_pixel(start, dim, bbox);
                    let p1 = world_to_minimap_pixel(end, dim, bbox);
                    buffer.draw_line(
                        p0.x.round() as i32,
                        p0.y.round() as i32,
                        p1.x.round() as i32,
                        p1.y.round() as i32,
                        line_color,
                    );
                }
            }
        }
        if map_copy.sectors.is_empty()
            && map_copy.linedefs.is_empty()
            && let Some(surface) = server_ctx.editing_surface.as_ref()
        {
            draw_surface_outline_on_minimap(buffer, surface, bbox);
        }

        MINIMAPBUFFER
            .write()
            .unwrap()
            .resize(buffer.dim().width, buffer.dim().height);

        MINIMAPBUFFER.write().unwrap().copy_into(0, 0, buffer);
        *MINIMAPCACHEKEY.write().unwrap() = cache_key;
        if let Some(region) = region_marker {
            draw_camera_marker(region, buffer, server_ctx);
        }
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

    let bbox_center_x = bbox.x + bbox.z / 2.0;
    let bbox_center_y = bbox.y + bbox.w / 2.0;

    let offset_x = -bbox_center_x * scale_x;
    let offset_y = bbox_center_y * scale_y;

    let pixel_x = (world_pos.x * scale_x) + offset_x + (width / 2.0);
    let pixel_y = (-world_pos.y * scale_y) + offset_y + (height / 2.0);

    Vec2::new(pixel_x, render_dim.y - pixel_y)
}
