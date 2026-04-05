use crate::prelude::*;
use rusterix::ValueContainer;
use std::collections::hash_map::DefaultHasher;
use vek::Vec2;

use crate::editor::{DOCKMANAGER, PALETTE, RUSTERIX};
use std::hash::{Hash, Hasher};

pub static MINIMAPBUFFER: LazyLock<RwLock<TheRGBABuffer>> =
    LazyLock::new(|| RwLock::new(TheRGBABuffer::default()));

pub static MINIMAPBOX: LazyLock<RwLock<Vec4<f32>>> = LazyLock::new(|| RwLock::new(Vec4::one()));
pub static MINIMAPCACHEKEY: LazyLock<RwLock<u64>> = LazyLock::new(|| RwLock::new(0));

fn palette_minimap_active(server_ctx: &ServerContext) -> bool {
    server_ctx.palette_tool_active && DOCKMANAGER.read().unwrap().dock == "Palette"
}

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

pub fn draw_camera_marker(
    map: &Map,
    region: Option<&Region>,
    buffer: &mut TheRGBABuffer,
    server_ctx: &ServerContext,
) {
    let camera_pos = if server_ctx.editor_view_mode == EditorViewMode::D2 || region.is_none() {
        // In 2D (region/screen/profile), marker follows the current map view center.
        Vec2::new(-map.offset.x / map.grid_size, map.offset.y / map.grid_size)
    } else if let Some(region) = region {
        // In 3D region mode, marker follows the active 3D editing anchor.
        Vec2::new(region.editing_position_3d.x, region.editing_position_3d.z)
    } else {
        Vec2::zero()
    };

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

    if server_ctx.editor_view_mode == EditorViewMode::FirstP
        && let Some(region) = region
    {
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
    if palette_minimap_active(server_ctx) {
        return;
    }

    let label = if server_ctx.get_map_context() == MapContext::Region {
        "Region"
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
    if palette_minimap_active(server_ctx) {
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

    if !hard {
        buffer.copy_into(0, 0, &MINIMAPBUFFER.read().unwrap());
        let map = if server_ctx.get_map_context() == MapContext::Region {
            project
                .get_region(&server_ctx.curr_region)
                .map(|region| &region.map)
        } else {
            project.get_map(server_ctx)
        };

        if let Some(map) = map {
            let region_marker = if server_ctx.get_map_context() == MapContext::Region {
                project.get_region(&server_ctx.curr_region)
            } else {
                None
            };
            draw_camera_marker(map, region_marker, buffer, server_ctx);
        }
        return;
    }

    let dim = buffer.dim();

    let width = dim.width as f32;
    let height = dim.height as f32;
    let background = [42, 42, 42, 255];

    let map = if server_ctx.get_map_context() == MapContext::Region {
        project
            .get_region(&server_ctx.curr_region)
            .map(|region| &region.map)
    } else {
        project.get_map(server_ctx)
    };

    if let Some(map) = map {
        let Some(bbox) = minimap_bbox_for_map(map) else {
            buffer.fill(background);
            return;
        };

        *MINIMAPBOX.write().unwrap() = bbox;

        let scale_x = width / bbox.z;
        let scale_y = height / bbox.w;

        let mut rusterix = RUSTERIX.write().unwrap();
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

        let use_scenevm_region = server_ctx.get_map_context() == MapContext::Region;

        if use_scenevm_region {
            // Minimap readability: render with stable daytime lighting.
            let hour = 12.0;
            let anim_counter = rusterix.client.animation_frame;
            let scenevm_mode_2d = rusterix.scene_handler.settings.scenevm_mode_2d();
            let scene_handler = &mut rusterix.scene_handler;
            let layer_count = scene_handler.vm.vm_layer_count();
            let mut layer_enabled_before = Vec::with_capacity(layer_count);
            for i in 0..layer_count {
                layer_enabled_before.push(scene_handler.vm.is_layer_enabled(i).unwrap_or(true));
            }
            // Minimap should show base scene only, never editor/game overlays.
            for i in 1..layer_count {
                scene_handler.vm.set_layer_enabled(i, false);
            }
            if matches!(scenevm_mode_2d, scenevm::RenderMode::Compute2D) {
                scene_handler.vm.execute(scenevm::Atom::SetGP0(Vec4::new(
                    map_copy.grid_size,
                    map_copy.subdivisions,
                    map_copy.offset.x,
                    -map_copy.offset.y,
                )));
            }
            scene_handler
                .vm
                .execute(scenevm::Atom::SetRenderMode(scenevm_mode_2d));
            scene_handler.settings.apply_hour(hour);
            scene_handler.settings.apply_2d(&mut scene_handler.vm);
            // Minimap 2D background should stay black regardless of project sky settings.
            scene_handler
                .vm
                .execute(scenevm::Atom::SetGP0(Vec4::zero()));
            scene_handler
                .vm
                .execute(scenevm::Atom::SetTransform2D(transform));
            scene_handler
                .vm
                .execute(scenevm::Atom::SetAnimationCounter(anim_counter));
            scene_handler
                .vm
                .execute(scenevm::Atom::SetBackground(Vec4::zero()));
            scene_handler
                .vm
                .render_frame(buffer.pixels_mut(), width as u32, height as u32);
            for (i, enabled) in layer_enabled_before.into_iter().enumerate() {
                scene_handler.vm.set_layer_enabled(i, enabled);
            }
        } else {
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
        }

        MINIMAPBUFFER
            .write()
            .unwrap()
            .resize(buffer.dim().width, buffer.dim().height);

        MINIMAPBUFFER.write().unwrap().copy_into(0, 0, buffer);
        *MINIMAPCACHEKEY.write().unwrap() = cache_key;
        let region_marker = if server_ctx.get_map_context() == MapContext::Region {
            project.get_region(&server_ctx.curr_region)
        } else {
            None
        };
        draw_camera_marker(map, region_marker, buffer, server_ctx);
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
