use theframework::prelude::*;

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

pub fn ray_sphere(ray: crate::Ray, center: Vec3<f32>, radius: f32) -> Option<f32> {
    let l = center - ray.o;
    let tca = l.dot(ray.d);
    let d2 = l.dot(l) - tca * tca;
    let radius2 = radius * radius;
    if d2 > radius2 {
        return None;
    }
    let thc = (radius2 - d2).sqrt();
    let mut t0 = tca - thc;
    let mut t1 = tca + thc;

    if t0 > t1 {
        std::mem::swap(&mut t0, &mut t1);
    }

    if t0 < 0.0 {
        t0 = t1;
        if t0 < 0.0 {
            return None;
        }
    }

    Some(t0)
}
