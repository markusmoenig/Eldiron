/// Ray AABB / Cube hit test wth normal.
pub fn ray_aabb(ray: &Ray, aabb_min: Vec3f, aabb_max: Vec3f) -> Option<Hit> {
    let t0s = (aabb_min - ray.o) * ray.inv_direction;
    let t1s = (aabb_max - ray.o) * ray.inv_direction;

    let mut tmin = f32::NEG_INFINITY;
    let mut tmax = f32::INFINITY;
    let mut normal = Vec3::new(0.0, 0.0, 0.0);

    for i in 0..3 {
        let axis_normal = match i {
            0 => Vec3f::new(1.0, 0.0, 0.0),
            1 => Vec3f::new(0.0, 1.0, 0.0),
            _ => Vec3f::new(0.0, 0.0, 1.0),
        };
        if ray.inv_direction[i] >= 0.0 {
            if t0s[i] > tmin {
                tmin = t0s[i];
                normal = axis_normal * -1.0; // Invert the normal if we're hitting the min side
            }
            tmax = tmax.min(t1s[i]);
        } else {
            if t1s[i] > tmin {
                tmin = t1s[i];
                normal = axis_normal; // Normal points in the positive axis direction
            }
            tmax = tmax.min(t0s[i]);
        }
    }

    if tmax >= tmin && tmin >= 0.0 {
        // Calculate intersection point
        let hit_point = ray.o + ray.d * tmin;
        let mut face = HitFace::XFace;

        // Determine which face of the box was hit and calculate UV coordinates
        let mut u = 0.0;
        let mut v = 0.0;
        if normal == Vec3::new(1.0, 0.0, 0.0) || normal == Vec3::new(-1.0, 0.0, 0.0) {
            // Hit the X face
            v = 1.0 - (hit_point.y - aabb_min.y) / (aabb_max.y - aabb_min.y);
            u = (hit_point.z - aabb_min.z) / (aabb_max.z - aabb_min.z);
            face = HitFace::XFace;
        } else if normal == Vec3::new(0.0, 1.0, 0.0) || normal == Vec3::new(0.0, -1.0, 0.0) {
            // Hit the Y face
            u = (hit_point.x - aabb_min.x) / (aabb_max.x - aabb_min.x);
            v = (hit_point.z - aabb_min.z) / (aabb_max.z - aabb_min.z);
            face = HitFace::YFace;
        } else if normal == Vec3::new(0.0, 0.0, 1.0) || normal == Vec3::new(0.0, 0.0, -1.0) {
            // Hit the Z face
            u = (hit_point.x - aabb_min.x) / (aabb_max.x - aabb_min.x);
            v = 1.0 - (hit_point.y - aabb_min.y) / (aabb_max.y - aabb_min.y);
            face = HitFace::ZFace;
        }

        Some(Hit {
            distance: tmin,
            hit_point,
            normal,
            uv: vec2f(u, v),
            face,
        })
    } else {
        None
    }
}
