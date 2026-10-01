pub mod d3firstp;
pub mod d3iso;
pub mod d3orbit;
pub mod mouse_look;

use crate::Ray;
use vek::{Mat4, Vec2, Vec3, Vec4};

#[allow(unused)]
pub trait D3Camera: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn id(&self) -> String {
        "".to_string()
    }

    fn view_matrix(&self) -> Mat4<f32> {
        Mat4::identity()
    }

    fn view_matrix_for_surface(&self, _width: f32, _height: f32) -> Mat4<f32> {
        self.view_matrix()
    }

    fn position(&self) -> Vec3<f32> {
        Vec3::zero()
    }

    fn scale(&self) -> f32 {
        1.0
    }

    fn fov(&self) -> f32 {
        1.0
    }

    fn distance(&self) -> f32 {
        1.0
    }

    fn basis_vectors(&self) -> (Vec3<f32>, Vec3<f32>, Vec3<f32>);

    fn projection_matrix(&self, width: f32, height: f32) -> Mat4<f32>;

    /// Get an f32 parameter.
    fn get_parameter_f32(&mut self, key: &str) -> f32 {
        0.0
    }

    /// Set an f32 parameter.
    fn set_parameter_f32(&mut self, key: &str, value: f32) {}

    /// Set a Vec2 parameter.
    fn set_parameter_vec2(&mut self, key: &str, value: Vec2<f32>) {}

    /// Set a Vec3 parameter.
    fn set_parameter_vec3(&mut self, key: &str, value: Vec3<f32>) {}

    /// Set a Vec4 parameter.
    fn set_parameter_vec4(&mut self, key: &str, value: Vec4<f32>) {}

    /// Creates a ray
    fn create_ray(&self, uv: Vec2<f32>, screen: Vec2<f32>, offset: Vec2<f32>) -> Ray {
        Ray::default()
    }

    /// Rotate (only used by orbit camera)
    fn rotate(&mut self, delta: Vec2<f32>) {}

    /// Zoom (only used by orbit camera)
    fn zoom(&mut self, delta: f32) {}

    /// Generate a SceneVM Camera
    fn as_scenevm_camera(&self) -> scenevm::Camera3D;

    /// Generate a SceneVM camera aligned to a concrete render surface.
    fn as_scenevm_camera_for_surface(&self, _width: f32, _height: f32) -> scenevm::Camera3D {
        self.as_scenevm_camera()
    }
}
