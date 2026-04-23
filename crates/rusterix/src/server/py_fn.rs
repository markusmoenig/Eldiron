use rand::*;
use vek::Vec2;

/// Find a random poition max_distance away from pos.
pub fn find_random_position(pos: Vec2<f32>, max_distance: f32) -> Vec2<f32> {
    let mut rng = rand::rng();
    let angle = rng.random_range(0.0..std::f32::consts::TAU);
    let dx = max_distance * angle.cos();
    let dy = max_distance * angle.sin();
    Vec2::new(pos.x + dx, pos.y + dy)
}
