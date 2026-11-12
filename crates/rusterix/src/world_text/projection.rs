//! Projection against one graphical view; CPU world policy does not depend on a widget.
use crate::client::widget::game::GameWidget;
use vek::{Vec2, Vec3, Vec4};
pub fn project(game: &GameWidget, world: Vec3<f32>) -> Option<Vec2<f32>> {
    if ![
        world.x,
        world.y,
        world.z,
        game.rect.x,
        game.rect.y,
        game.rect.width,
        game.rect.height,
    ]
    .iter()
    .all(|v| v.is_finite())
        || game.rect.width <= 0.
        || game.rect.height <= 0.
    {
        return None;
    }
    let p = if game.camera.is_isometric_2d() {
        game.isometric_projection?
            .screen(Vec2::new(world.x, world.z), world.y)
    } else if crate::client::screen_model::is_2d_camera(&game.camera) {
        game.ground_to_screen(Vec2::new(world.x, world.z)) - Vec2::new(0., world.y * game.grid_size)
    } else {
        let vp = game
            .camera_d3
            .projection_matrix(game.rect.width, game.rect.height)
            * game
                .camera_d3
                .view_matrix_for_surface(game.rect.width, game.rect.height);
        let c = vp * Vec4::new(world.x, world.y, world.z, 1.);
        if c.w <= 0. || (c.z / c.w).abs() > 1. {
            return None;
        }
        Vec2::new(
            (c.x / c.w * 0.5 + 0.5) * game.rect.width,
            (0.5 - c.y / c.w * 0.5) * game.rect.height,
        )
    };
    if !p.x.is_finite()
        || !p.y.is_finite()
        || p.x < 0.
        || p.y < 0.
        || p.x > game.rect.width
        || p.y > game.rect.height
    {
        return None;
    }
    Some(p + Vec2::new(game.rect.x, game.rect.y))
}
