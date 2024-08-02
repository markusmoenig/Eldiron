use crate::prelude::*;
use core::f32;
use std::f32::consts::PI;
use theframework::prelude::*;

pub struct PrerenderedCamera {
    pub ratio: f32,
    pub pixel_size: Vec2f,
    pub half_width: f32,
    pub half_height: f32,
    pub w: Vec3f,
    pub u: Vec3f,
    pub v: Vec3f,
}

/// Camera
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Camera {
    pub origin: Vec3f,
    pub center: Vec3f,
    pub fov: f32,

    // For orbit
    pub distance: f32,

    pub forward: Vec3f,
    pub up: Vec3f,
    pub right: Vec3f,

    pub orbit_x: f32,
    pub orbit_y: f32,
}

impl Camera {
    pub fn new(origin: Vec3f, center: Vec3f, fov: f32) -> Self {
        Self {
            origin,
            center,
            fov,

            distance: 2.0,

            forward: Vec3f::new(0.0, 0.0, -1.0),
            up: Vec3f::new(0.0, 1.0, 0.0),
            right: Vec3f::new(1.0, 0.0, 0.0),

            orbit_x: 0.0,
            orbit_y: -90.0,
        }
    }

    /// Set the camera's origin and center based on the top-down angle (in degrees)
    pub fn set_top_down_angle(&mut self, angle_deg: f32, distance: f32, look_at: Vec3f) {
        let angle_rad = angle_deg.to_radians();
        let height = distance * angle_rad.sin();
        let horizontal_distance = distance * angle_rad.cos();

        self.center = look_at;

        // Assuming the camera looks along the negative z-axis by default
        self.origin = Vec3f {
            x: look_at.x,
            y: look_at.y + height,
            z: look_at.z - horizontal_distance,
        };
    }

    /// Zoom the camera by a given factor
    pub fn zoom(&mut self, delta: f32) {
        let direction = normalize(self.center - self.origin);

        self.origin += direction * delta;
        self.center += direction * delta;
    }

    // Move the camera by a given displacement
    pub fn move_by(&mut self, x_offset: f32, y_offset: f32) {
        // self.origin += Vec3f::new(x_offset, y_offset, 0.0);
        // self.center += Vec3f::new(x_offset, y_offset, 0.0);

        let direction = normalize(self.center - self.origin);
        let up_vector = vec3f(0.0, 1.0, 0.0);
        let right_vector = cross(direction, up_vector);

        let displacement = right_vector * x_offset + up_vector * y_offset;

        self.origin += displacement;
        self.center += displacement;

        /*
        let direction = normalize(self.center - self.origin);
        let up_vector = vec3f(0.0, 1.0, 0.0);
        let right_vector = cross(direction, up_vector);

        self.origin += direction * y_offset + right_vector * x_offset;
        self.center += direction * y_offset + right_vector * x_offset;*/
    }

    /// Pan the camera horizontally and vertically
    pub fn pan(&mut self, horizontal: f32, vertical: f32) {
        let w = normalize(self.origin - self.center);
        let up_vector = vec3f(0.0, 1.0, 0.0);
        let u = cross(up_vector, w);
        let v = cross(w, u);

        self.center += u * horizontal + v * vertical;
    }

    /// Rotate the camera around its center
    pub fn rotate(&mut self, yaw: f32, pitch: f32) {
        fn magnitude(vec: Vec3f) -> f32 {
            (vec.x.powi(2) + vec.y.powi(2) + vec.z.powi(2)).sqrt()
        }

        let radius = magnitude(self.origin - self.center);

        let mut theta = ((self.origin.z - self.center.z) / radius).acos();
        let mut phi = ((self.origin.x - self.center.x) / (radius * theta.sin())).acos();

        theta += pitch.to_radians();
        phi += yaw.to_radians();

        theta = theta.clamp(0.1, PI - 0.1); //theta.max(0.1).min(PI - 0.1);

        self.origin.x = self.center.x + radius * theta.sin() * phi.cos();
        self.origin.y = self.center.y + radius * theta.cos();
        self.origin.z = self.center.z + radius * theta.sin() * phi.sin();
    }

    /// Create a pinhole ray
    pub fn create_ray(&self, uv: Vec2f, screen: Vec2f, offset: Vec2f) -> Ray {
        let ratio = screen.x / screen.y;
        let pixel_size = vec2f(1.0 / screen.x, 1.0 / screen.y);

        let half_width = (self.fov.to_radians() * 0.5).tan();
        let half_height = half_width / ratio;

        let up_vector = vec3f(0.0, 1.0, 0.0);

        let w = normalize(self.origin - self.center);
        let u = cross(up_vector, w);
        let v = cross(w, u);

        let lower_left = self.origin - u * half_width - v * half_height - w;
        let horizontal = u * half_width * 2.0;
        let vertical = v * half_height * 2.0;
        let mut dir = lower_left - self.origin;

        dir += horizontal * (pixel_size.x * offset.x + uv.x);
        dir += vertical * (pixel_size.y * offset.y + uv.y);

        Ray::new(self.origin, normalize(dir))
    }

    pub fn create_ortho_ray(&self, uv: Vec2f, screen: Vec2f, offset: Vec2f) -> Ray {
        let ratio = screen.x / screen.y;
        let pixel_size = Vec2f::new(1.0 / screen.x, 1.0 / screen.y);

        let cam_origin = self.origin;
        let cam_look_at = self.center;

        let half_width = ((self.fov + 100.0).to_radians() * 0.5).tan();
        let half_height = half_width / ratio;

        let up_vector = Vec3f::new(0.0, 1.0, 0.0);

        let w = normalize(cam_origin - cam_look_at);
        let u = cross(up_vector, w);
        let v = cross(w, u);

        let horizontal = u * half_width * 2.0;
        let vertical = v * half_height * 2.0;

        let mut out_origin = cam_origin;
        out_origin += horizontal * (pixel_size.x * offset.x + uv.x - 0.5);
        out_origin += vertical * (pixel_size.y * offset.y + uv.y - 0.5);

        Ray::new(out_origin, normalize(-w))
    }

    pub fn create_ortho_ray2(
        &self,
        uv: Vec2f,
        screen: Vec2f,
        tiles: Vec2f,
        offset: Vec2f,
        scale_factor: f32,
    ) -> Ray {
        let pixel_size = Vec2f::new(1.0 / screen.x, 1.0 / screen.y);

        let cam_origin = self.origin;
        let cam_look_at = self.center;

        let half_width = tiles.x;
        let half_height = tiles.y;

        let up_vector = Vec3f::new(0.0, 1.0, 0.0);

        let w = normalize(cam_origin - cam_look_at);
        let u = cross(up_vector, w);
        let v = cross(w, u);

        let horizontal = u * half_width * scale_factor;
        let vertical = v * half_height * scale_factor;

        let mut out_origin = cam_origin;
        out_origin += horizontal * (pixel_size.x * offset.x + uv.x - 0.5);
        out_origin += vertical * (pixel_size.y * offset.y + uv.y - 0.5);

        Ray::new(out_origin, normalize(-w))
    }

    pub fn create_ortho_ray_prerendered(&self, uv: Vec2f, prerender: &PrerenderedCamera) -> Ray {
        let cam_origin = self.origin;

        let horizontal = prerender.u * prerender.half_width;
        let vertical = prerender.v * prerender.half_height;

        let mut out_origin = cam_origin;
        out_origin += horizontal * (uv.x - 0.5);
        out_origin += vertical * (uv.y - 0.5);

        Ray::new(out_origin, -prerender.w)
    }

    pub fn create_tilted_isometric_ray(
        &self,
        uv: Vec2f,
        screen: Vec2f,
        offset: Vec2f,
        alignment: i32,
    ) -> Ray {
        let ratio = screen.x / screen.y;
        let pixel_size = Vec2f::new(1.0 / screen.x, 1.0 / screen.y);

        let cam_origin = self.origin;
        let cam_look_at = self.center;

        let half_width = ((self.fov + 100.0).to_radians() * 0.5).tan();
        let half_height = half_width / ratio;

        let up_vector = Vec3f::new(0.0, 1.0, 0.0);

        let w = normalize(cam_origin - cam_look_at);
        let u = cross(up_vector, w);
        let v = cross(w, u);

        let horizontal = u * half_width * 2.0;
        let vertical = v * half_height * 2.0;

        let mut out_origin = cam_origin;
        out_origin += horizontal * (pixel_size.x * offset.x + uv.x - 0.5);
        out_origin += vertical * (pixel_size.y * offset.y + uv.y - 0.5);
        out_origin.y = cam_origin.y;

        Ray::new(
            out_origin,
            normalize(vec3f(
                if alignment == 0 { -0.35 } else { 0.35 },
                -1.0,
                -0.35,
            )),
        )
    }

    pub fn create_tilted_isometric_ray2(
        &self,
        uv: Vec2f,
        screen: Vec2f,
        tiles: Vec2f,
        offset: Vec2f,
        alignment: i32,
        scale_factor: f32,
    ) -> Ray {
        let pixel_size = Vec2f::new(1.0 / screen.x, 1.0 / screen.y);

        let cam_origin = self.origin;
        let cam_look_at = self.center;

        let half_width = tiles.x;
        let half_height = tiles.y;

        let up_vector = Vec3f::new(0.0, 1.0, 0.0);

        let w = normalize(cam_origin - cam_look_at);
        let u = cross(up_vector, w);
        let v = cross(w, u);

        let horizontal = u * half_width * scale_factor;
        let vertical = v * half_height * scale_factor;

        let mut out_origin = cam_origin;
        out_origin += horizontal * (pixel_size.x * offset.x + uv.x - 0.5);
        out_origin += vertical * (pixel_size.y * offset.y + uv.y - 0.5);
        out_origin.y = cam_origin.y;

        Ray::new(
            out_origin,
            normalize(vec3f(
                if alignment == 0 { -0.35 } else { 0.35 },
                -1.0,
                -0.35,
            )),
        )
    }

    pub fn create_tilted_isometric_ray_prerendered(
        &self,
        uv: Vec2f,
        alignment: i32,
        prerender: &PrerenderedCamera,
    ) -> Ray {
        let cam_origin = self.origin;

        let horizontal = prerender.u * prerender.half_width;
        let vertical = prerender.v * prerender.half_height;

        let mut out_origin = cam_origin;
        out_origin += horizontal * (uv.x - 0.5);
        out_origin += vertical * (uv.y - 0.5);
        out_origin.y = cam_origin.y;

        Ray::new(
            out_origin,
            //normalize(
            vec3f(
                if alignment == 0 { -0.35 } else { 0.35 },
                -1.0,
                -0.35,
                //    )
            ),
        )
    }

    pub fn prerender(origin: Vec3f, center: Vec3f, screen: Vec2f, fov: f32) -> PrerenderedCamera {
        let ratio = screen.x / screen.y;
        let pixel_size = Vec2f::new(1.0 / screen.x, 1.0 / screen.y);

        let half_width = ((fov + 100.0).to_radians() * 0.5).tan();
        let half_height = half_width / ratio;

        let up_vector = Vec3f::new(0.0, 1.0, 0.0);

        let w = normalize(origin - center);
        let u = cross(up_vector, w);
        let v = cross(w, u);

        PrerenderedCamera {
            ratio,
            half_width: half_width * 2.0,
            half_height: half_height * 2.0,
            pixel_size,
            w,
            u,
            v,
        }
    }

    /// Computes the orbi camera vectors. Based on https://www.shadertoy.com/view/ttfyzN
    pub fn compute_orbit(&mut self, mouse_delta: Vec2f) {
        #[inline(always)]
        pub fn mix(a: &f32, b: &f32, v: f32) -> f32 {
            (1.0 - v) * a + b * v
        }

        let min_camera_angle = 0.01;
        let max_camera_angle = std::f32::consts::PI - 0.01;

        self.orbit_x += mouse_delta.x;
        self.orbit_y += mouse_delta.y;

        let angle_x = -self.orbit_x;
        let angle_y = mix(&min_camera_angle, &max_camera_angle, self.orbit_y);

        let mut camera_pos = Vec3f::zero();

        camera_pos.x = sin(angle_x) * sin(angle_y) * self.distance;
        camera_pos.y = -cos(angle_y) * self.distance;
        camera_pos.z = cos(angle_x) * sin(angle_y) * self.distance;

        camera_pos += self.center;

        self.origin = camera_pos;
        self.forward = normalize(self.center - camera_pos);
        self.right = normalize(cross(vec3f(0.0, 1.0, 0.0), -self.forward));
        self.up = normalize(cross(-self.forward, self.right));
    }

    /// Create an orbit camera ray
    pub fn create_orbit_ray(&self, uv: Vec2f, screen_dim: Vec2f, offset: Vec2f) -> Ray {
        let camera_pos = self.origin;
        let camera_fwd = self.forward;
        let camera_up = self.up;
        let camera_right = self.right;

        let uv_jittered = (uv * screen_dim + (offset - 0.5)) / screen_dim;
        let mut screen = uv_jittered * 2.0 - 1.0;

        let aspect_ratio = screen_dim.x / screen_dim.y;
        screen.y /= aspect_ratio;

        let camera_distance = tan(self.fov * 0.5 * std::f32::consts::PI / 180.0);
        let mut ray_dir = vec3f(screen.x, screen.y, camera_distance);
        ray_dir = normalize(Mat3f::from((camera_right, camera_up, camera_fwd)) * ray_dir);

        Ray::new(camera_pos, ray_dir)
    }
}
