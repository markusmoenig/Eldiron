use crate::{D3Camera, D3IsoCamera, map::particle::ParticleEmitter};
use vek::{Mat4, Vec2, Vec3, Vec4};

pub struct ParticleHarnessImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub struct ParticleHarnessPair {
    pub preview: ParticleHarnessImage,
    pub bright_wall: ParticleHarnessImage,
    pub builder_iso: ParticleHarnessImage,
}

pub fn reference_emitter() -> ParticleEmitter {
    let mut emitter = ParticleEmitter::new(Vec3::zero(), Vec3::new(0.0, 1.0, 0.0));
    emitter.rate = 24.0;
    emitter.spread = std::f32::consts::FRAC_PI_4 * 0.55;
    emitter.lifetime_range = (0.55, 1.0);
    emitter.radius_range = (0.06, 0.13);
    emitter.speed_range = (0.45, 0.95);
    emitter.color = [255, 164, 72, 255];
    emitter.color_variation = 8;
    emitter.color_ramp = Some([
        [255, 224, 128, 255],
        [255, 150, 64, 255],
        [176, 64, 32, 255],
        [48, 34, 34, 255],
    ]);
    emitter
}

pub fn render_reference_pair(width: u32, height: u32, time: f32) -> ParticleHarnessPair {
    ParticleHarnessPair {
        preview: render_emitter(width, height, time, [10, 12, 16, 255]),
        bright_wall: render_emitter(width, height, time, [232, 228, 218, 255]),
        builder_iso: render_builder_iso(width, height, time),
    }
}

pub fn render_builder_iso(width: u32, height: u32, time: f32) -> ParticleHarnessImage {
    let width = width.max(1);
    let height = height.max(1);
    let mut emitter = reference_emitter();
    let steps = ((time.max(0.0) / (1.0 / 30.0)).ceil() as usize).max(1);
    let dt = time.max(0.0) / steps as f32;
    for _ in 0..steps {
        emitter.update(dt);
    }

    let mut pixels = vec![0u8; (width * height * 4) as usize];
    for px in pixels.chunks_exact_mut(4) {
        px.copy_from_slice(&[232, 228, 218, 255]);
    }

    let wall_x0 = (width as i32 / 2) - 24;
    let wall_x1 = (width as i32 / 2) + 24;
    for y in 0..height as i32 {
        for x in wall_x0..=wall_x1 {
            if x < 0 || x >= width as i32 {
                continue;
            }
            let idx = ((y * width as i32 + x) * 4) as usize;
            pixels[idx] = 245;
            pixels[idx + 1] = 242;
            pixels[idx + 2] = 236;
            pixels[idx + 3] = 255;
        }
    }

    let mut camera = D3IsoCamera::new();
    camera.center = Vec3::new(0.0, 1.3, 0.0);
    camera.scale = 6.0;
    camera.distance = 20.0;
    let vp = camera.projection_matrix(width as f32, height as f32) * camera.view_matrix();
    let pixels_per_world = height as f32 / (camera.scale * 2.0);

    let torch_tip = Vec3::new(0.0, 1.42, 0.0);
    for particle in &emitter.particles {
        let world = torch_tip + particle.pos;
        if let Some((sx, sy)) = project_world(vp, world, width, height) {
            let radius_world = (particle.radius * 4.6).max(0.28) * 0.5;
            let radius_px = (radius_world * pixels_per_world).clamp(2.0, 22.0);
            let alpha = (particle.lifetime / particle.initial_lifetime.max(0.001)).clamp(0.0, 1.0);
            draw_soft_particle(
                &mut pixels,
                width as i32,
                height as i32,
                sx,
                sy,
                radius_px,
                [
                    particle.color[0],
                    particle.color[1],
                    particle.color[2],
                    (alpha * 255.0) as u8,
                ],
            );
        }
    }

    ParticleHarnessImage {
        width,
        height,
        pixels,
    }
}

pub fn render_emitter(
    width: u32,
    height: u32,
    time: f32,
    background: [u8; 4],
) -> ParticleHarnessImage {
    let width = width.max(1);
    let height = height.max(1);
    let mut emitter = reference_emitter();
    let steps = ((time.max(0.0) / (1.0 / 30.0)).ceil() as usize).max(1);
    let dt = time.max(0.0) / steps as f32;
    for _ in 0..steps {
        emitter.update(dt);
    }

    let mut pixels = vec![0u8; (width * height * 4) as usize];
    for px in pixels.chunks_exact_mut(4) {
        px.copy_from_slice(&background);
    }

    let emitter_x = width as f32 * 0.5;
    let emitter_y = height as f32 * 0.82;

    for particle in &emitter.particles {
        let x = emitter_x + particle.pos.x * width as f32 * 0.22;
        let y = emitter_y - particle.pos.y * height as f32 * 0.45;
        let radius = (particle.radius * 38.0).clamp(3.0, 18.0);
        let alpha = (particle.lifetime / particle.initial_lifetime.max(0.001)).clamp(0.0, 1.0);
        draw_soft_particle(
            &mut pixels,
            width as i32,
            height as i32,
            x,
            y,
            radius,
            [
                particle.color[0],
                particle.color[1],
                particle.color[2],
                (alpha * 255.0) as u8,
            ],
        );
    }

    ParticleHarnessImage {
        width,
        height,
        pixels,
    }
}

fn draw_soft_particle(
    pixels: &mut [u8],
    width: i32,
    height: i32,
    center_x: f32,
    center_y: f32,
    radius: f32,
    color: [u8; 4],
) {
    let min_x = (center_x - radius - 1.0).floor() as i32;
    let max_x = (center_x + radius + 1.0).ceil() as i32;
    let min_y = (center_y - radius - 1.0).floor() as i32;
    let max_y = (center_y + radius + 1.0).ceil() as i32;
    for y in min_y..=max_y {
        if y < 0 || y >= height {
            continue;
        }
        for x in min_x..=max_x {
            if x < 0 || x >= width {
                continue;
            }
            let dx = (x as f32 + 0.5 - center_x) / radius.max(0.001);
            let dy = (y as f32 + 0.5 - center_y) / radius.max(0.001);
            let dist2 = dx * dx + dy * dy;
            if dist2 > 1.0 {
                continue;
            }
            let falloff = (1.0 - dist2).powf(1.8);
            let alpha = (color[3] as f32 / 255.0) * falloff;
            let idx = ((y * width + x) * 4) as usize;
            let inv = 1.0 - alpha;
            pixels[idx] = (pixels[idx] as f32 * inv + color[0] as f32 * alpha) as u8;
            pixels[idx + 1] = (pixels[idx + 1] as f32 * inv + color[1] as f32 * alpha) as u8;
            pixels[idx + 2] = (pixels[idx + 2] as f32 * inv + color[2] as f32 * alpha) as u8;
            pixels[idx + 3] = 255;
        }
    }
}

fn project_world(vp: Mat4<f32>, pos: Vec3<f32>, width: u32, height: u32) -> Option<(f32, f32)> {
    let clip = vp * Vec4::new(pos.x, pos.y, pos.z, 1.0);
    if clip.w.abs() <= 1e-6 {
        return None;
    }
    let ndc = clip.xyz() / clip.w;
    if ndc.z < -1.0 || ndc.z > 1.0 {
        return None;
    }
    let screen = Vec2::new(
        (ndc.x * 0.5 + 0.5) * width as f32,
        (1.0 - (ndc.y * 0.5 + 0.5)) * height as f32,
    );
    Some((screen.x, screen.y))
}
