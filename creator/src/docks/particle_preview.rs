use crate::prelude::*;
use rusterix::ParticleEmitter;

const PREVIEW_PARTICLE_SIM_FPS: f32 = 15.0;
const PREVIEW_PARTICLE_TIME_SCALE: f32 = 1.0;

pub fn average_rgba_color(pixels: &[u8]) -> [u8; 4] {
    if pixels.is_empty() {
        return [255, 255, 255, 255];
    }
    let mut sum = [0_u64; 4];
    let mut count = 0_u64;
    for rgba in pixels.chunks_exact(4) {
        if rgba[3] == 0 {
            continue;
        }
        sum[0] += rgba[0] as u64;
        sum[1] += rgba[1] as u64;
        sum[2] += rgba[2] as u64;
        sum[3] += rgba[3] as u64;
        count += 1;
    }
    if count == 0 {
        return [255, 176, 72, 255];
    }
    let avg = [
        (sum[0] / count) as u8,
        (sum[1] / count) as u8,
        (sum[2] / count) as u8,
        (sum[3] / count) as u8,
    ];
    if avg[0] < 12 && avg[1] < 12 && avg[2] < 12 {
        [255, 176, 72, 255]
    } else {
        avg
    }
}

pub fn dominant_light_rgba_color(pixels: &[u8]) -> [u8; 4] {
    if pixels.is_empty() {
        return [255, 176, 72, 255];
    }

    let mut max_luma = 0.0f32;
    for rgba in pixels.chunks_exact(4) {
        if rgba[3] == 0 {
            continue;
        }
        let luma = 0.2126 * rgba[0] as f32 + 0.7152 * rgba[1] as f32 + 0.0722 * rgba[2] as f32;
        max_luma = max_luma.max(luma);
    }

    if max_luma <= 1.0 {
        return average_rgba_color(pixels);
    }

    let threshold = (max_luma * 0.45).max(24.0);
    let mut sum = [0.0f32; 4];
    let mut weight_sum = 0.0f32;
    for rgba in pixels.chunks_exact(4) {
        if rgba[3] == 0 {
            continue;
        }
        let luma = 0.2126 * rgba[0] as f32 + 0.7152 * rgba[1] as f32 + 0.0722 * rgba[2] as f32;
        if luma < threshold {
            continue;
        }
        let weight = (luma / 255.0).powf(1.5) * (rgba[3] as f32 / 255.0);
        sum[0] += rgba[0] as f32 * weight;
        sum[1] += rgba[1] as f32 * weight;
        sum[2] += rgba[2] as f32 * weight;
        sum[3] += rgba[3] as f32 * weight;
        weight_sum += weight;
    }

    if weight_sum <= 0.0 {
        return average_rgba_color(pixels);
    }

    [
        (sum[0] / weight_sum).clamp(0.0, 255.0) as u8,
        (sum[1] / weight_sum).clamp(0.0, 255.0) as u8,
        (sum[2] / weight_sum).clamp(0.0, 255.0) as u8,
        (sum[3] / weight_sum).clamp(0.0, 255.0) as u8,
    ]
}

#[derive(Clone, Debug)]
pub(crate) struct PreviewCircle {
    pub center: [f32; 2],
    pub radius: f32,
    pub color: [u8; 4],
}
#[derive(Clone, Debug)]
pub(crate) struct ParticlePreviewFrame {
    pub size: Vec2<i32>,
    pub top: [u8; 4],
    pub circles: Vec<PreviewCircle>,
    pub marker: TheDim,
}

/// Simulation/projection only. No destination pixels or renderer are needed.
pub(crate) fn prepare_particle_emitter_preview(
    source: &ParticleEmitter,
    width: i32,
    height: i32,
    time: f32,
) -> ParticlePreviewFrame {
    let (width, height) = (width.max(1), height.max(1));
    let ramp = source
        .color_ramp
        .unwrap_or_else(|| derive_particle_ramp(source.color));
    let top = [
        ((ramp[1][0] as f32) * 0.35 + 24.0).clamp(0.0, 255.0) as u8,
        ((ramp[1][1] as f32) * 0.35 + 18.0).clamp(0.0, 255.0) as u8,
        ((ramp[1][2] as f32) * 0.35 + 20.0).clamp(0.0, 255.0) as u8,
        255,
    ];
    let mut emitter = source.clone();
    emitter.particles.clear();
    emitter.time_accum = 0.;
    let steps = ((time.max(0.0) / (1.0 / PREVIEW_PARTICLE_SIM_FPS)).ceil() as usize).max(1);
    let dt = (time.max(0.0) / steps as f32) * PREVIEW_PARTICLE_TIME_SCALE;
    for _ in 0..steps {
        emitter.update(dt);
    }
    let emitter_x = width as f32 * 0.5;
    let emitter_y = height as f32 * 0.88;
    let mut circles = Vec::with_capacity(emitter.particles.len() + 2);
    if source.flame_base {
        circles.push(PreviewCircle {
            center: [emitter_x, emitter_y - 5.0],
            radius: (((source.radius_range.0 + source.radius_range.1) * 0.5) * 36.0)
                .clamp(12.0, 34.0),
            color: [ramp[1][0], ramp[1][1], ramp[1][2], 245],
        });
        circles.push(PreviewCircle {
            center: [emitter_x, emitter_y - 8.0],
            radius: (((source.radius_range.0 + source.radius_range.1) * 0.5) * 24.0)
                .clamp(8.0, 22.0),
            color: [ramp[0][0], ramp[0][1], ramp[0][2], 245],
        });
    }
    for particle in &emitter.particles {
        let alpha = particle.color[3] as f32 / 255.;
        circles.push(PreviewCircle {
            center: [
                emitter_x + particle.pos.x * width as f32 * 0.28,
                emitter_y - particle.pos.y * height as f32 * 0.36,
            ],
            radius: (particle.radius * 34.0).clamp(4.0, 42.0),
            color: [
                particle.color[0],
                particle.color[1],
                particle.color[2],
                (255. * alpha) as u8,
            ],
        });
    }
    ParticlePreviewFrame {
        size: Vec2::new(width, height),
        top,
        circles,
        marker: TheDim::new((emitter_x as i32) - 6, (emitter_y as i32) - 6, 12, 12),
    }
}

/// Explicit pixel/reference consumers. Native preview publication uses the
/// prepared geometry instead of calling this display bitmap adapter.
#[cfg(any(not(feature = "native-ui"), test))]
pub fn render_particle_emitter_preview(
    emitter: &ParticleEmitter,
    width: i32,
    height: i32,
    time: f32,
) -> TheRGBABuffer {
    let frame = prepare_particle_emitter_preview(emitter, width, height, time);
    frame.reference_pixels()
}

impl ParticlePreviewFrame {
    #[cfg(any(not(feature = "native-ui"), test))]
    pub(crate) fn reference_pixels(&self) -> TheRGBABuffer {
        let mut preview = TheRGBABuffer::new(TheDim::sized(self.size.x, self.size.y));
        for y in 0..self.size.y {
            let t = y as f32 / self.size.y as f32;
            let row = [
                (self.top[0] as f32 * (1. - t) + 6. * t) as u8,
                (self.top[1] as f32 * (1. - t) + 8. * t) as u8,
                (self.top[2] as f32 * (1. - t) + 14. * t) as u8,
                255,
            ];
            preview.draw_horizontal_line(0, self.size.x - 1, y, row);
        }
        for circle in &self.circles {
            draw_soft_particle(
                &mut preview,
                circle.center[0],
                circle.center[1],
                circle.radius,
                circle.color,
            );
        }
        preview.draw_disc(&self.marker, &[255, 255, 255, 140], 1., &[0; 4]);
        preview
    }
}

#[cfg(feature = "native-ui")]
impl ParticlePreviewFrame {
    pub(crate) fn native_layer(
        &self,
        opaque: bool,
    ) -> std::sync::Arc<theframework::paint::PixelRadialLayer> {
        use theframework::paint::{PixelRadial, PixelRadialLayer, Rect};
        let color = |color: [u8; 4]| color.map(|v| v as f32 / 255.);
        let mut radials = self
            .circles
            .iter()
            .map(|circle| PixelRadial::SoftCircle {
                center: circle.center,
                radius: circle.radius,
                color: color(circle.color),
            })
            .collect::<Vec<_>>();
        radials.push(PixelRadial::Disc {
            rect: Rect::new(
                self.marker.x as f32,
                self.marker.y as f32,
                self.marker.width as f32,
                self.marker.height as f32,
            ),
            color: color([255, 255, 255, 140]),
            border_width: 1.,
            border: [0.; 4],
        });
        std::sync::Arc::new(PixelRadialLayer {
            opaque,
            size: [self.size.x as f32, self.size.y as f32],
            top: color(self.top),
            bottom: color([6, 8, 14, 255]),
            radials: radials.into(),
        })
    }
}

fn derive_particle_ramp(base: [u8; 4]) -> [[u8; 4]; 4] {
    [
        [
            (base[0] as f32 * 1.15).clamp(0.0, 255.0) as u8,
            (base[1] as f32 * 1.1).clamp(0.0, 255.0) as u8,
            (base[2] as f32 * 0.9 + 24.0).clamp(0.0, 255.0) as u8,
            255,
        ],
        [
            (base[0] as f32).clamp(0.0, 255.0) as u8,
            (base[1] as f32).clamp(0.0, 255.0) as u8,
            (base[2] as f32).clamp(0.0, 255.0) as u8,
            255,
        ],
        [
            (base[0] as f32 * 0.75).clamp(0.0, 255.0) as u8,
            (base[1] as f32 * 0.45).clamp(0.0, 255.0) as u8,
            (base[2] as f32 * 0.3).clamp(0.0, 255.0) as u8,
            255,
        ],
        [36, 32, 32, 255],
    ]
}

#[cfg(any(not(feature = "native-ui"), test))]
fn draw_soft_particle(
    buffer: &mut TheRGBABuffer,
    center_x: f32,
    center_y: f32,
    radius: f32,
    color: [u8; 4],
) {
    let min_x = (center_x - radius - 1.0).floor() as i32;
    let max_x = (center_x + radius + 1.0).ceil() as i32;
    let min_y = (center_y - radius - 1.0).floor() as i32;
    let max_y = (center_y + radius + 1.0).ceil() as i32;
    let stride = buffer.dim().width;
    let height = buffer.dim().height;

    for y in min_y..=max_y {
        if y < 0 || y >= height {
            continue;
        }
        for x in min_x..=max_x {
            if x < 0 || x >= stride {
                continue;
            }
            let dx = (x as f32 + 0.5 - center_x) / radius.max(0.001);
            let dy = (y as f32 + 0.5 - center_y) / radius.max(0.001);
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > 1.0 {
                continue;
            }
            let falloff = (1.0 - dist).powf(2.0);
            let alpha = (color[3] as f32 / 255.0) * falloff;
            let index = ((y * stride + x) * 4) as usize;
            let dst = &mut buffer.pixels_mut()[index..index + 4];
            let inv = 1.0 - alpha;
            dst[0] = (dst[0] as f32 * inv + color[0] as f32 * alpha) as u8;
            dst[1] = (dst[1] as f32 * inv + color[1] as f32 * alpha) as u8;
            dst[2] = (dst[2] as f32 * inv + color[2] as f32 * alpha) as u8;
            dst[3] = 255;
        }
    }
}
