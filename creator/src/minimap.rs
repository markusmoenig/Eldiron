use crate::prelude::*;
use rusterix::prelude::*;
use vek::Vec2;

use crate::editor::RUSTERIX;

// static MINIMAP: LazyLock<RwLock<Map>> = LazyLock::new(|| RwLock::new(Map::default()));
// static BUILDER: LazyLock<RwLock<D2PreviewBuilder>> =
//     LazyLock::new(|| RwLock::new(D2PreviewBuilder::default()));

// pub fn update_minimap(
//     orig_region: &Region,
//     buffer: &mut TheRGBABuffer,
//     server_ctx: &ServerContext,
// ) {
//     println!("update_minimap");
// }

pub fn draw_minimap(orig_region: &Region, buffer: &mut TheRGBABuffer, server_ctx: &ServerContext) {
    let dim = buffer.dim();

    let width = dim.width as f32;
    let height = dim.height as f32;
    let background = [42, 42, 42, 255];

    let mut region = orig_region.clone();
    if let Some(mut bbox) = region.map.bounding_box() {
        bbox.x -= 0.5;
        bbox.y -= 0.5;
        bbox.z += 1.0;
        bbox.w += 1.0;

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
        region.map.camera_xz = Some(Vec2::new(
            region.editing_position_3d.x,
            region.editing_position_3d.z,
        ));

        region.map.look_at_xz = Some(Vec2::new(
            region.editing_look_at_3d.x,
            region.editing_look_at_3d.z,
        ));

        let mut builder = D2PreviewBuilder::new();
        builder.set_map_tool_type(MapToolType::MiniMap);
        builder.draw_grid = false;
        if let Some(camera_pos) = region.map.camera_xz {
            builder.set_camera_info(
                Some(Vec3::new(camera_pos.x, 0.0, camera_pos.y)),
                if server_ctx.curr_map_tool_helper == MapToolHelper::Preview {
                    Some(Vec3::new(
                        region.editing_look_at_3d.x,
                        0.0,
                        region.editing_look_at_3d.z,
                    ))
                } else {
                    None
                },
            );
        }

        let rusterix = RUSTERIX.write().unwrap();

        let mut map = region.map.clone();
        map.clear_temp();
        map.entities.clear();
        map.items.clear();

        let mut scene = builder.build(
            &map,
            &rusterix.assets,
            Vec2::new(width, height),
            &ValueContainer::default(),
        );

        builder.build_entities_items(&map, &rusterix.assets, &mut scene, Vec2::new(width, height));

        let mut light = Light::new(LightType::Ambient);
        light.set_color([1.0, 1.0, 1.0]);
        light.set_intensity(1.0);

        scene.dynamic_lights.push(light);

        let translation_matrix = Mat3::<f32>::translation_2d(Vec2::new(
            map.offset.x + width / 2.0,
            -map.offset.y + height / 2.0,
        ));
        let scale_matrix = Mat3::new(
            map.grid_size,
            0.0,
            0.0,
            0.0,
            map.grid_size,
            0.0,
            0.0,
            0.0,
            1.0,
        );
        let transform = translation_matrix * scale_matrix;

        Rasterizer::setup(Some(transform), Mat4::identity(), Mat4::identity())
            .background(background)
            .rasterize(
                &mut scene,
                buffer.pixels_mut(),
                width as usize,
                height as usize,
                64,
            );

        // *MINIMAP.write().unwrap() = map;
        // *BUILDER.write().unwrap() = builder;
    } else {
        buffer.fill(background);
    }
}
