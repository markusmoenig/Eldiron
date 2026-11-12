use crate::prelude::*;
use rusterix::{D3Camera, D3OrbitCamera};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewCubeFace {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

impl ViewCubeFace {
    fn normal(self) -> Vec3<f32> {
        match self {
            Self::PositiveX => Vec3::unit_x(),
            Self::NegativeX => -Vec3::unit_x(),
            Self::PositiveY => Vec3::unit_y(),
            Self::NegativeY => -Vec3::unit_y(),
            Self::PositiveZ => Vec3::unit_z(),
            Self::NegativeZ => -Vec3::unit_z(),
        }
    }

    fn corners(self) -> [Vec3<f32>; 4] {
        match self {
            Self::PositiveX => [
                Vec3::new(1.0, -1.0, -1.0),
                Vec3::new(1.0, -1.0, 1.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(1.0, 1.0, -1.0),
            ],
            Self::NegativeX => [
                Vec3::new(-1.0, -1.0, 1.0),
                Vec3::new(-1.0, -1.0, -1.0),
                Vec3::new(-1.0, 1.0, -1.0),
                Vec3::new(-1.0, 1.0, 1.0),
            ],
            Self::PositiveY => [
                Vec3::new(-1.0, 1.0, -1.0),
                Vec3::new(1.0, 1.0, -1.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(-1.0, 1.0, 1.0),
            ],
            Self::NegativeY => [
                Vec3::new(-1.0, -1.0, 1.0),
                Vec3::new(1.0, -1.0, 1.0),
                Vec3::new(1.0, -1.0, -1.0),
                Vec3::new(-1.0, -1.0, -1.0),
            ],
            Self::PositiveZ => [
                Vec3::new(1.0, -1.0, 1.0),
                Vec3::new(-1.0, -1.0, 1.0),
                Vec3::new(-1.0, 1.0, 1.0),
                Vec3::new(1.0, 1.0, 1.0),
            ],
            Self::NegativeZ => [
                Vec3::new(-1.0, -1.0, -1.0),
                Vec3::new(1.0, -1.0, -1.0),
                Vec3::new(1.0, 1.0, -1.0),
                Vec3::new(-1.0, 1.0, -1.0),
            ],
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::PositiveX => "+X",
            Self::NegativeX => "-X",
            Self::PositiveY => "+Y",
            Self::NegativeY => "-Y",
            Self::PositiveZ => "+Z",
            Self::NegativeZ => "-Z",
        }
    }

    fn color(self, facing: f32) -> [u8; 4] {
        let base = match self {
            Self::PositiveX | Self::NegativeX => [188.0, 72.0, 76.0],
            Self::PositiveY | Self::NegativeY => [76.0, 164.0, 104.0],
            Self::PositiveZ | Self::NegativeZ => [68.0, 120.0, 194.0],
        };
        let light = (0.68 + facing.clamp(0.0, 1.0) * 0.30) as f32;
        [
            (base[0] * light) as u8,
            (base[1] * light) as u8,
            (base[2] * light) as u8,
            245,
        ]
    }
}

#[derive(Clone)]
struct ProjectedFace {
    face: ViewCubeFace,
    points: [Vec2<f32>; 4],
    depth: f32,
    facing: f32,
}

pub struct ViewCube;

impl ViewCube {
    const CENTER_Y: f32 = 102.0;
    const HALF_EXTENT: f32 = 23.0;

    fn center(dim: TheDim) -> Option<Vec2<f32>> {
        (dim.width >= 100 && dim.height >= 150)
            .then(|| Vec2::new(dim.width as f32 - 52.0, Self::CENTER_Y))
    }

    fn projected_faces(dim: TheDim, camera: &D3OrbitCamera) -> Vec<ProjectedFace> {
        let Some(center) = Self::center(dim) else {
            return Vec::new();
        };
        let (forward, right, up) = camera.basis_vectors();
        let toward_camera = -forward;
        let mut faces = Vec::new();
        for face in [
            ViewCubeFace::PositiveX,
            ViewCubeFace::NegativeX,
            ViewCubeFace::PositiveY,
            ViewCubeFace::NegativeY,
            ViewCubeFace::PositiveZ,
            ViewCubeFace::NegativeZ,
        ] {
            let normal = face.normal();
            let facing = normal.dot(toward_camera);
            if facing <= 0.001 {
                continue;
            }
            let corners = face.corners();
            let points = corners.map(|corner| {
                Vec2::new(
                    center.x + corner.dot(right) * Self::HALF_EXTENT,
                    center.y - corner.dot(up) * Self::HALF_EXTENT,
                )
            });
            faces.push(ProjectedFace {
                face,
                points,
                depth: normal.dot(forward),
                facing,
            });
        }
        // Painter's order: faces closest to the eye are drawn last.
        faces.sort_by(|a, b| b.depth.total_cmp(&a.depth));
        faces
    }

    fn edge(a: Vec2<f32>, b: Vec2<f32>, p: Vec2<f32>) -> f32 {
        (p.x - a.x) * (b.y - a.y) - (p.y - a.y) * (b.x - a.x)
    }

    fn point_in_triangle(point: Vec2<f32>, a: Vec2<f32>, b: Vec2<f32>, c: Vec2<f32>) -> bool {
        let e0 = Self::edge(a, b, point);
        let e1 = Self::edge(b, c, point);
        let e2 = Self::edge(c, a, point);
        (e0 >= 0.0 && e1 >= 0.0 && e2 >= 0.0) || (e0 <= 0.0 && e1 <= 0.0 && e2 <= 0.0)
    }

    fn point_in_face(point: Vec2<f32>, face: &ProjectedFace) -> bool {
        Self::point_in_triangle(point, face.points[0], face.points[1], face.points[2])
            || Self::point_in_triangle(point, face.points[0], face.points[2], face.points[3])
    }

    pub(crate) fn prepare_overlay(
        size: (usize, usize),
        camera: &D3OrbitCamera,
    ) -> crate::editor_overlay::EditorOverlay {
        let dim = TheDim::sized(
            size.0.min(i32::MAX as usize) as i32,
            size.1.min(i32::MAX as usize) as i32,
        );
        Self::commands_for_faces(size, &Self::projected_faces(dim, camera))
    }

    fn commands_for_faces(
        size: (usize, usize),
        faces: &[ProjectedFace],
    ) -> crate::editor_overlay::EditorOverlay {
        let mut paint = crate::editor_overlay::EditorOverlay::new(size);
        for projected in faces {
            let color = projected.face.color(projected.facing);
            paint.triangle(
                [
                    [projected.points[0].x, projected.points[0].y],
                    [projected.points[1].x, projected.points[1].y],
                    [projected.points[2].x, projected.points[2].y],
                ],
                color,
            );
            paint.triangle(
                [
                    [projected.points[0].x, projected.points[0].y],
                    [projected.points[2].x, projected.points[2].y],
                    [projected.points[3].x, projected.points[3].y],
                ],
                color,
            );
            for edge in 0..4 {
                let a = projected.points[edge];
                let b = projected.points[(edge + 1) % 4];
                paint.line(
                    a.x.round() as i32,
                    a.y.round() as i32,
                    b.x.round() as i32,
                    b.y.round() as i32,
                    [220, 226, 236, 255],
                );
            }
            let label_center = projected
                .points
                .iter()
                .copied()
                .fold(Vec2::zero(), |sum, point| sum + point)
                / 4.0;
            let label_rect = (
                (label_center.x - 11.0).max(0.0) as usize,
                (label_center.y - 7.0).max(0.0) as usize,
                22,
                14,
            );
            paint.text_rect_blend(
                &label_rect,
                projected.face.label(),
                TheFontSettings {
                    size: 9.0,
                    ..Default::default()
                },
                &[246, 248, 252, 255],
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );
        }
        paint
    }

    pub fn draw(buffer: &mut TheRGBABuffer, ctx: &mut TheContext, camera: &D3OrbitCamera) {
        let size = (
            buffer.dim().width.max(0) as usize,
            buffer.dim().height.max(0) as usize,
        );
        Self::prepare_overlay(size, camera).draw_reference(buffer, ctx);
    }

    pub fn hit_test(coord: Vec2<i32>, dim: TheDim, camera: &D3OrbitCamera) -> Option<ViewCubeFace> {
        let point = coord.map(|value| value as f32);
        Self::projected_faces(dim, camera)
            .into_iter()
            .rev()
            .find(|face| Self::point_in_face(point, face))
            .map(|face| face.face)
    }

    pub fn snap_camera(camera: &mut D3OrbitCamera, face: ViewCubeFace) {
        const POLE_EPSILON: f32 = 0.01;
        match face {
            ViewCubeFace::PositiveX => {
                camera.azimuth = 0.0;
                camera.elevation = 0.0;
            }
            ViewCubeFace::NegativeX => {
                camera.azimuth = std::f32::consts::PI;
                camera.elevation = 0.0;
            }
            ViewCubeFace::PositiveY => {
                camera.elevation = std::f32::consts::FRAC_PI_2 - POLE_EPSILON;
            }
            ViewCubeFace::NegativeY => {
                camera.elevation = -std::f32::consts::FRAC_PI_2 + POLE_EPSILON;
            }
            ViewCubeFace::PositiveZ => {
                camera.azimuth = std::f32::consts::FRAC_PI_2;
                camera.elevation = 0.0;
            }
            ViewCubeFace::NegativeZ => {
                camera.azimuth = -std::f32::consts::FRAC_PI_2;
                camera.elevation = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapping_places_camera_on_requested_axis() {
        let mut camera = D3OrbitCamera::new();
        for (face, expected) in [
            (ViewCubeFace::PositiveX, Vec3::unit_x()),
            (ViewCubeFace::NegativeX, -Vec3::unit_x()),
            (ViewCubeFace::PositiveY, Vec3::unit_y()),
            (ViewCubeFace::NegativeY, -Vec3::unit_y()),
            (ViewCubeFace::PositiveZ, Vec3::unit_z()),
            (ViewCubeFace::NegativeZ, -Vec3::unit_z()),
        ] {
            ViewCube::snap_camera(&mut camera, face);
            let from_center = (camera.position() - camera.center).normalized();
            assert!(from_center.dot(expected) > 0.999);
        }
    }
}

#[cfg(test)]
#[path = "viewcube/tests.rs"]
mod native_tests;

mod native;
pub(crate) use native::{CubeFrame, CubePick, CubeTarget, ViewCubeFrames};
