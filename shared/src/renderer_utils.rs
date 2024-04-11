use crate::prelude::*;
use theframework::prelude::*;

/// Create the camera setup.
pub fn create_camera_setup(
    mut position: Vec3f,
    region: &Region,
    settings: &mut RegionDrawSettings,
) -> (Vec3f, Vec3f, f32, CameraMode, CameraType) {
    let mut facing = vec3f(0.0, 0.0, -1.0);
    if settings.center_on_character.is_some() {
        position = settings.center_3d + position;
        facing = settings.facing_3d;
    }

    // Get the camera settings

    let mut camera_type = CameraType::TiltedIso;
    let mut first_person_height = 0.5;
    let mut top_down_height = 4.0;
    let mut top_down_x_offset = -5.0;
    let mut top_down_z_offset = 5.0;
    let mut first_person_fov = 70.0;
    let mut top_down_fov = 55.0;
    let tilted_iso_height = 3.0;
    let mut tilted_iso_fov = 74.0;

    if let Some(TheValue::TextList(value, _)) = region.regionfx.get(
        str!("Camera"),
        str!("Camera Type"),
        &settings.time,
        TheInterpolation::Linear,
    ) {
        if value == 0 {
            camera_type = CameraType::FirstPerson;
        } else if value == 1 {
            camera_type = CameraType::TopDown;
        }
    }

    if let Some(v) = region.regionfx.get(
        str!("Camera"),
        str!("First Person FoV"),
        &settings.time,
        TheInterpolation::Linear,
    ) {
        if let Some(value) = v.to_f32() {
            first_person_fov = value;
        }
    }

    if let Some(v) = region.regionfx.get(
        str!("Camera"),
        str!("Top Down FoV"),
        &settings.time,
        TheInterpolation::Linear,
    ) {
        if let Some(value) = v.to_f32() {
            top_down_fov = value;
        }
    }

    if let Some(v) = region.regionfx.get(
        str!("Camera"),
        str!("First Person Height"),
        &settings.time,
        TheInterpolation::Linear,
    ) {
        if let Some(value) = v.to_f32() {
            first_person_height = value;
        }
    }

    if let Some(v) = region.regionfx.get(
        str!("Camera"),
        str!("Top Down Height"),
        &settings.time,
        TheInterpolation::Linear,
    ) {
        if let Some(value) = v.to_f32() {
            top_down_height = value;
        }
    }

    if let Some(v) = region.regionfx.get(
        str!("Camera"),
        str!("Top Down X Offset"),
        &settings.time,
        TheInterpolation::Linear,
    ) {
        if let Some(value) = v.to_f32() {
            top_down_x_offset = value;
        }
    }

    if let Some(v) = region.regionfx.get(
        str!("Camera"),
        str!("Top Down Z Offset"),
        &settings.time,
        TheInterpolation::Linear,
    ) {
        if let Some(value) = v.to_f32() {
            top_down_z_offset = value;
        }
    }

    if let Some(v) = region.regionfx.get(
        str!("Camera"),
        str!("Tilted Iso FoV"),
        &settings.time,
        TheInterpolation::Linear,
    ) {
        if let Some(value) = v.to_f32() {
            tilted_iso_fov = value;
        }
    }

    // Camera

    let mut ro = vec3f(position.x + 0.5, 0.5, position.z + 0.5);
    let rd;
    let fov;
    let mut camera_mode = CameraMode::Pinhole;

    if camera_type == CameraType::TopDown {
        rd = ro;
        ro.y = top_down_height;
        ro.x += top_down_x_offset;
        ro.z += top_down_z_offset;
        fov = top_down_fov;
        camera_mode = CameraMode::Orthogonal;
    } else if camera_type == CameraType::FirstPerson {
        // First person
        ro.y = first_person_height;
        rd = ro + facing * 2.0;
        fov = first_person_fov;
    } else {
        // Tilted iso
        rd = ro;
        ro.y = tilted_iso_height;
        ro.z += 1.0;
        fov = tilted_iso_fov;
        camera_mode = CameraMode::Orthogonal;
    }

    (ro, rd, fov, camera_mode, camera_type)
}

/// Gets the current time in milliseconds
pub fn get_time() -> u128 {
    let time;
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        time = t.as_millis();
    }
    #[cfg(target_arch = "wasm32")]
    {
        time = web_sys::window().unwrap().performance().unwrap().now() as u128;
    }
    time
}

/*
pub fn ray_models(ray: &Ray, models: &(Vec<ModelFXFloor>, Vec<ModelFXWall>)) -> Option<Hit> {
    let mut hit: Option<Hit> = None;
    let (floors, walls) = models;
    for fx in floors {
        if let Some(h) = fx.hit(ray) {
            if let Some(hit) = &mut hit {
                if h.distance < hit.distance {
                    *hit = h;
                }
            } else {
                hit = Some(h);
            }
        }
    }
    for fx in walls {
        if let Some(h) = fx.hit(ray) {
            if let Some(hit) = &mut hit {
                if h.distance < hit.distance {
                    *hit = h;
                }
            } else {
                hit = Some(h);
            }
        }
    }
    hit
}
*/
