use crate::prelude::*;
use rusterix::prelude::*;
use vek::Vec2;

use crate::editor::RUSTERIX;

pub fn draw_minimap(orig_region: &Region, buffer: &mut TheRGBABuffer) {
    let dim = buffer.dim();

    let width = dim.width as f32;
    let height = dim.height as f32;

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
        if let Some(camera_pos) = region.map.camera_xz {
            builder.set_camera_info(
                Some(Vec3::new(camera_pos.x, 0.0, camera_pos.y)),
                Some(Vec3::new(
                    region.editing_look_at_3d.x,
                    0.0,
                    region.editing_look_at_3d.z,
                )),
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
            "preview",
            &ValueContainer::default(),
        );

        Rasterizer::setup(None, Mat4::identity(), Mat4::identity()).rasterize(
            &mut scene,
            buffer.pixels_mut(),
            width as usize,
            height as usize,
            64,
        );
    }
}
