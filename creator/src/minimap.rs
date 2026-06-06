use crate::prelude::*;
use rusterix::ValueContainer;
use std::collections::BTreeSet;
use std::collections::hash_map::DefaultHasher;
use vek::Vec2;

use crate::editor::{ACTIONLIST, DOCKMANAGER, PALETTE, RUSTERIX};
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

fn hash_minimap_map_state(map: &Map, hasher: &mut DefaultHasher) {
    map.changed.hash(hasher);
    map.geometry_objects.len().hash(hasher);
    map.selected_linedefs.hash(hasher);
    map.selected_sectors.hash(hasher);
    map.selected_vertices.hash(hasher);
    map.selected_geometry_objects.hash(hasher);
    map.selected_geometry_faces.hash(hasher);
    map.selected_geometry_vertices.hash(hasher);
    map.selected_geometry_surface_points.hash(hasher);
    map.selected_geometry_surface_segments.hash(hasher);
}

pub fn minimap_bbox_for_map(map: &Map) -> Option<Vec4<f32>> {
    let bbox = map.bbox();
    let min = bbox.min - Vec2::broadcast(0.5);
    let mut size = bbox.size() + Vec2::broadcast(1.0);
    size.x = size.x.max(1.0);
    size.y = size.y.max(1.0);
    Some(Vec4::new(min.x, min.y, size.x, size.y))
}

fn draw_geometry_minimap(map: &Map, buffer: &mut TheRGBABuffer, bbox: Vec4<f32>) {
    let dim = *buffer.dim();
    let render_dim = Vec2::new(dim.width as f32, dim.height as f32);
    let edge_color = [150, 154, 158, 255];
    let selected_edge_color = [235, 226, 180, 255];
    let selected_vertex_color = [255, 245, 178, 255];
    let selected_surface_color = [202, 150, 236, 255];
    let bbox_color = [86, 88, 92, 255];

    for object in &map.geometry_objects {
        let selected = map.selected_geometry_objects.contains(&object.id);
        let selected_faces = map
            .selected_geometry_faces
            .iter()
            .filter_map(|(object_id, face_index)| (*object_id == object.id).then_some(*face_index))
            .collect::<BTreeSet<_>>();
        let selected_vertices = map
            .selected_geometry_vertices
            .iter()
            .filter_map(|(object_id, vertex_index)| {
                (*object_id == object.id).then_some(*vertex_index)
            })
            .collect::<BTreeSet<_>>();
        let color = if selected {
            selected_edge_color
        } else {
            edge_color
        };

        if let Some(object_bbox) = object.bbox() {
            let min = world_to_minimap_pixel(object_bbox.min, render_dim, bbox);
            let max = world_to_minimap_pixel(object_bbox.max, render_dim, bbox);
            let x = min.x.min(max.x) as i32;
            let y = min.y.min(max.y) as i32;
            let w = (min.x - max.x).abs().max(1.0) as i32;
            let h = (min.y - max.y).abs().max(1.0) as i32;
            buffer.draw_rect_outline(&TheDim::rect(x, y, w, h), &bbox_color);
        }

        let mut drawn_edges = BTreeSet::new();
        for (face_index, face) in object.faces.iter().enumerate() {
            if face.indices.len() < 2 {
                continue;
            }
            let face_selected = selected_faces.contains(&face_index);
            for index in 0..face.indices.len() {
                let a_index = face.indices[index];
                let b_index = face.indices[(index + 1) % face.indices.len()];
                let edge = (a_index.min(b_index), a_index.max(b_index));
                let edge_selected = face_selected
                    || selected_vertices.contains(&a_index) && selected_vertices.contains(&b_index);
                if !edge_selected && !drawn_edges.insert(edge) {
                    continue;
                }
                let Some(a) = object.vertices.get(a_index) else {
                    continue;
                };
                let Some(b) = object.vertices.get(b_index) else {
                    continue;
                };
                let a = object.transform_point(*a);
                let b = object.transform_point(*b);
                if !a.x.is_finite() || !a.z.is_finite() || !b.x.is_finite() || !b.z.is_finite() {
                    continue;
                }
                let a = world_to_minimap_pixel(Vec2::new(a.x, a.z), render_dim, bbox);
                let b = world_to_minimap_pixel(Vec2::new(b.x, b.z), render_dim, bbox);
                let line_color = if edge_selected {
                    selected_edge_color
                } else {
                    color
                };
                buffer.draw_line(a.x as i32, a.y as i32, b.x as i32, b.y as i32, line_color);
            }
        }

        for vertex_index in selected_vertices {
            let Some(vertex) = object.vertices.get(vertex_index) else {
                continue;
            };
            let world = object.transform_point(*vertex);
            if !world.x.is_finite() || !world.z.is_finite() {
                continue;
            }
            let pos = world_to_minimap_pixel(Vec2::new(world.x, world.z), render_dim, bbox);
            buffer.draw_rect_outline(
                &TheDim::rect(pos.x as i32 - 2, pos.y as i32 - 2, 4, 4),
                &selected_vertex_color,
            );
        }

        for (object_id, face_index, segment_index) in &map.selected_geometry_surface_segments {
            if *object_id != object.id {
                continue;
            }
            let Some(face) = object.faces.get(*face_index) else {
                continue;
            };
            let Some(segment) = face.surface_segments.get(*segment_index) else {
                continue;
            };
            let Some(start) = face.surface_points.get(segment.start) else {
                continue;
            };
            let Some(end) = face.surface_points.get(segment.end) else {
                continue;
            };
            let start = object.transform_point(start.position);
            let end = object.transform_point(end.position);
            if !start.x.is_finite()
                || !start.z.is_finite()
                || !end.x.is_finite()
                || !end.z.is_finite()
            {
                continue;
            }
            let start = world_to_minimap_pixel(Vec2::new(start.x, start.z), render_dim, bbox);
            let end = world_to_minimap_pixel(Vec2::new(end.x, end.z), render_dim, bbox);
            buffer.draw_line(
                start.x as i32,
                start.y as i32,
                end.x as i32,
                end.y as i32,
                selected_surface_color,
            );
        }

        for (object_id, face_index, point_index) in &map.selected_geometry_surface_points {
            if *object_id != object.id {
                continue;
            }
            let Some(face) = object.faces.get(*face_index) else {
                continue;
            };
            let Some(point) = face.surface_points.get(*point_index) else {
                continue;
            };
            let world = object.transform_point(point.position);
            if !world.x.is_finite() || !world.z.is_finite() {
                continue;
            }
            let pos = world_to_minimap_pixel(Vec2::new(world.x, world.z), render_dim, bbox);
            buffer.draw_rect_outline(
                &TheDim::rect(pos.x as i32 - 2, pos.y as i32 - 2, 4, 4),
                &selected_surface_color,
            );
        }
    }
}

fn active_action_minimap_preview(
    map: &Map,
    server_ctx: &ServerContext,
) -> Option<(String, Vec<crate::actions::ActionMinimapSegment>)> {
    let Some(action_id) = server_ctx.curr_action_id else {
        return None;
    };
    let actionlist = ACTIONLIST.read().unwrap();
    let Some(action) = actionlist.get_action_by_id(action_id) else {
        return None;
    };
    if !action.uses_minimap_preview() {
        return None;
    };
    let segments = action.minimap_preview_segments(map, server_ctx);
    if segments.is_empty() {
        return None;
    }
    Some((action.id().name.clone(), segments))
}

fn minimap_bbox_for_segments(
    segments: &[crate::actions::ActionMinimapSegment],
) -> Option<Vec4<f32>> {
    let mut min = Vec2::broadcast(f32::INFINITY);
    let mut max = Vec2::broadcast(f32::NEG_INFINITY);
    for segment in segments {
        for point in [segment.start, segment.end] {
            if !point.x.is_finite() || !point.y.is_finite() {
                continue;
            }
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.y);
            max.x = max.x.max(point.x);
            max.y = max.y.max(point.y);
        }
    }
    if !min.x.is_finite() || !min.y.is_finite() || !max.x.is_finite() || !max.y.is_finite() {
        return None;
    }

    let mut size = max - min;
    size.x = size.x.max(1.0);
    size.y = size.y.max(1.0);
    let pad = Vec2::new(size.x * 0.12, size.y * 0.12);
    Some(Vec4::new(
        min.x - pad.x,
        min.y - pad.y,
        size.x + pad.x * 2.0,
        size.y + pad.y * 2.0,
    ))
}

fn bbox_with_render_aspect(mut bbox: Vec4<f32>, render_dim: Vec2<f32>) -> Vec4<f32> {
    if bbox.z <= 0.0 || bbox.w <= 0.0 || render_dim.x <= 0.0 || render_dim.y <= 0.0 {
        return bbox;
    }

    let render_aspect = render_dim.x / render_dim.y;
    let bbox_aspect = bbox.z / bbox.w;
    if bbox_aspect < render_aspect {
        let new_width = bbox.w * render_aspect;
        bbox.x -= (new_width - bbox.z) * 0.5;
        bbox.z = new_width;
    } else if bbox_aspect > render_aspect {
        let new_height = bbox.z / render_aspect;
        bbox.y -= (new_height - bbox.w) * 0.5;
        bbox.w = new_height;
    }
    bbox
}

fn draw_action_minimap_preview(
    buffer: &mut TheRGBABuffer,
    bbox: Vec4<f32>,
    segments: &[crate::actions::ActionMinimapSegment],
) {
    let dim = *buffer.dim();
    let render_dim = Vec2::new(dim.width as f32, dim.height as f32);
    let preview_color = [88, 210, 255, 235];
    for segment in segments {
        let start = preview_to_minimap_pixel(segment.start, render_dim, bbox);
        let end = preview_to_minimap_pixel(segment.end, render_dim, bbox);
        buffer.draw_line(
            start.x as i32,
            start.y as i32,
            end.x as i32,
            end.y as i32,
            preview_color,
        );
    }
}

fn preview_to_minimap_pixel(
    preview_pos: Vec2<f32>,
    render_dim: Vec2<f32>,
    bbox: Vec4<f32>,
) -> Vec2<f32> {
    let width = render_dim.x;
    let height = render_dim.y;

    let scale_x = width / bbox.z;
    let scale_y = height / bbox.w;

    let bbox_center_x = bbox.x + bbox.z / 2.0;
    let bbox_center_y = bbox.y + bbox.w / 2.0;

    let offset_x = -bbox_center_x * scale_x;
    let offset_y = bbox_center_y * scale_y;

    let pixel_x = (preview_pos.x * scale_x) + offset_x + (width / 2.0);
    let pixel_y = (-preview_pos.y * scale_y) + offset_y + (height / 2.0);

    Vec2::new(pixel_x, pixel_y)
}

fn draw_minimap_overlays(
    map: &Map,
    region_marker: Option<&Region>,
    buffer: &mut TheRGBABuffer,
    server_ctx: &ServerContext,
) {
    draw_camera_marker(map, region_marker, buffer, server_ctx);
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

    let label = if let Some(action_id) = server_ctx.curr_action_id
        && let Some(action) = ACTIONLIST.read().unwrap().get_action_by_id(action_id)
        && action.uses_minimap_preview()
    {
        action.id().name.clone()
    } else if server_ctx.get_map_context() == MapContext::Region {
        "Region".to_string()
    } else if server_ctx.get_map_context() == MapContext::Screen {
        "Screen".to_string()
    } else if server_ctx.get_map_context() == MapContext::Character {
        "Character".to_string()
    } else if server_ctx.get_map_context() == MapContext::Item {
        "Item".to_string()
    } else {
        "Map".to_string()
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
        &label,
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

    let map = if server_ctx.get_map_context() == MapContext::Region {
        project
            .get_region(&server_ctx.curr_region)
            .map(|region| &region.map)
    } else {
        project.get_map(server_ctx)
    };

    if let Some(map) = map
        && let Some((_label, segments)) = active_action_minimap_preview(map, server_ctx)
    {
        let background = [42, 42, 42, 255];
        buffer.fill(background);
        if let Some(bbox) = minimap_bbox_for_segments(&segments) {
            let dim = *buffer.dim();
            let bbox =
                bbox_with_render_aspect(bbox, Vec2::new(dim.width as f32, dim.height as f32));
            *MINIMAPBOX.write().unwrap() = bbox;
            draw_action_minimap_preview(buffer, bbox, &segments);
        }
        return;
    }

    let mut cache_key = minimap_context_key(server_ctx);
    if let Some(map) = map {
        let mut hasher = DefaultHasher::default();
        cache_key.hash(&mut hasher);
        hash_minimap_map_state(map, &mut hasher);
        cache_key = hasher.finish();
    }

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
        if let Some(map) = map {
            let region_marker = if server_ctx.get_map_context() == MapContext::Region {
                project.get_region(&server_ctx.curr_region)
            } else {
                None
            };
            draw_minimap_overlays(map, region_marker, buffer, server_ctx);
        }
        return;
    }

    let dim = buffer.dim();

    let width = dim.width as f32;
    let height = dim.height as f32;
    let background = [42, 42, 42, 255];

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

        let use_geometry_region = server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
            && !map.geometry_objects.is_empty();
        let use_scenevm_region =
            server_ctx.get_map_context() == MapContext::Region && !use_geometry_region;

        if use_geometry_region {
            buffer.fill(background);
            draw_geometry_minimap(map, buffer, bbox);
        } else if use_scenevm_region {
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
        draw_minimap_overlays(map, region_marker, buffer, server_ctx);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimap_bbox_includes_geometry_objects_without_2d_vertices() {
        let mut map = Map::new();
        map.geometry_objects
            .push(rusterix::GeometryObject::box_from_bounds(
                "box",
                Vec3::new(10.0, 0.0, 20.0),
                Vec3::new(12.0, 2.0, 24.0),
            ));

        let bbox = minimap_bbox_for_map(&map).unwrap();

        assert!(bbox.x <= 9.5);
        assert!(bbox.y <= 19.5);
        assert!(bbox.z >= 3.0);
        assert!(bbox.w >= 5.0);
    }

    #[test]
    fn action_preview_bbox_keeps_render_aspect() {
        let bbox = Vec4::new(0.0, 0.0, 2.0, 10.0);
        let adjusted = bbox_with_render_aspect(bbox, Vec2::new(200.0, 100.0));

        assert!((adjusted.z / adjusted.w - 2.0).abs() < 0.001);
        assert_eq!(adjusted.w, 10.0);
        assert!(adjusted.z > bbox.z);
    }

    #[test]
    fn geometry_minimap_draws_non_empty_projection() {
        let mut map = Map::new();
        map.geometry_objects
            .push(rusterix::GeometryObject::box_from_bounds(
                "box",
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(2.0, 2.0, 2.0),
            ));
        let bbox = minimap_bbox_for_map(&map).unwrap();
        let background = [42, 42, 42, 255];
        let mut buffer = TheRGBABuffer::new(TheDim::sized(96, 96));
        buffer.fill(background);

        draw_geometry_minimap(&map, &mut buffer, bbox);

        assert!(
            buffer
                .pixels()
                .chunks_exact(4)
                .any(|pixel| pixel != background)
        );
    }

    #[test]
    fn minimap_world_projection_places_bbox_center_in_buffer_center() {
        let bbox = Vec4::new(10.0, 20.0, 4.0, 8.0);
        let pos = world_to_minimap_pixel(Vec2::new(12.0, 24.0), Vec2::new(100.0, 200.0), bbox);

        assert!((pos.x - 50.0).abs() < 0.001);
        assert!((pos.y - 100.0).abs() < 0.001);
    }

    #[test]
    fn action_preview_projection_treats_positive_y_as_up() {
        let bbox = Vec4::new(0.0, 0.0, 10.0, 10.0);
        let lower = preview_to_minimap_pixel(Vec2::new(5.0, 2.0), Vec2::new(100.0, 100.0), bbox);
        let upper = preview_to_minimap_pixel(Vec2::new(5.0, 8.0), Vec2::new(100.0, 100.0), bbox);

        assert!(upper.y < lower.y);
        assert!((upper.x - lower.x).abs() < 0.001);
    }

    #[test]
    fn minimap_cache_state_changes_with_geometry_selection() {
        let mut map = Map::new();
        let object = rusterix::GeometryObject::box_from_bounds(
            "box",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 2.0, 2.0),
        );
        let object_id = object.id;
        map.geometry_objects.push(object);

        let mut before = DefaultHasher::default();
        hash_minimap_map_state(&map, &mut before);
        map.selected_geometry_objects.push(object_id);
        let mut after = DefaultHasher::default();
        hash_minimap_map_state(&map, &mut after);

        assert_ne!(before.finish(), after.finish());
    }

    #[test]
    fn geometry_minimap_draws_selected_vertex_highlight() {
        let mut map = Map::new();
        let object = rusterix::GeometryObject::box_from_bounds(
            "box",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 2.0, 2.0),
        );
        let object_id = object.id;
        map.geometry_objects.push(object);
        map.selected_geometry_vertices.push((object_id, 0));
        let bbox = minimap_bbox_for_map(&map).unwrap();
        let background = [42, 42, 42, 255];
        let selected_vertex_color = [255, 245, 178, 255];
        let mut buffer = TheRGBABuffer::new(TheDim::sized(96, 96));
        buffer.fill(background);

        draw_geometry_minimap(&map, &mut buffer, bbox);

        assert!(
            buffer
                .pixels()
                .chunks_exact(4)
                .any(|pixel| pixel == selected_vertex_color)
        );
    }
}
