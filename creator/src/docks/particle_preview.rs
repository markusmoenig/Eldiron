use crate::prelude::*;

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

pub fn render_particle_output_preview(
    source_pixels: &[u8],
    particle: &shared::tilegraph::TileParticleOutput,
    width: i32,
    height: i32,
    time: f32,
) -> TheRGBABuffer {
    render_particle_preview_common(
        average_rgba_color(source_pixels),
        Some(&particle.ramp_colors),
        particle.rate,
        particle.spread,
        particle.lifetime_min,
        particle.lifetime_max,
        particle.radius_min,
        particle.radius_max,
        particle.speed_min,
        particle.speed_max,
        particle.color_variation,
        width,
        height,
        time,
    )
}

pub fn render_particle_emitter_preview(
    emitter: &rusterix::ParticleEmitter,
    width: i32,
    height: i32,
    time: f32,
) -> TheRGBABuffer {
    render_particle_preview_common(
        emitter.color,
        emitter.color_ramp.as_ref(),
        emitter.rate,
        emitter.spread,
        emitter.lifetime_range.0,
        emitter.lifetime_range.1,
        emitter.radius_range.0,
        emitter.radius_range.1,
        emitter.speed_range.0,
        emitter.speed_range.1,
        emitter.color_variation,
        width,
        height,
        time,
    )
}

fn render_particle_preview_common(
    base: [u8; 4],
    ramp: Option<&[[u8; 4]; 4]>,
    rate: f32,
    spread: f32,
    lifetime_min: f32,
    lifetime_max: f32,
    radius_min: f32,
    radius_max: f32,
    speed_min: f32,
    speed_max: f32,
    color_variation: u8,
    width: i32,
    height: i32,
    time: f32,
) -> TheRGBABuffer {
    let width = width.max(1);
    let height = height.max(1);
    let mut preview = TheRGBABuffer::new(TheDim::sized(width, height));
    preview.fill([10, 12, 16, 255]);

    let ramp = ramp.copied().unwrap_or_else(|| derive_particle_ramp(base));
    let glow = [
        ((ramp[1][0] as f32) * 0.35 + 24.0).clamp(0.0, 255.0) as u8,
        ((ramp[1][1] as f32) * 0.35 + 18.0).clamp(0.0, 255.0) as u8,
        ((ramp[1][2] as f32) * 0.35 + 20.0).clamp(0.0, 255.0) as u8,
        255,
    ];
    for y in 0..height {
        let t = y as f32 / height.max(1) as f32;
        let row = [
            ((glow[0] as f32) * (1.0 - t) + 6.0 * t) as u8,
            ((glow[1] as f32) * (1.0 - t) + 8.0 * t) as u8,
            ((glow[2] as f32) * (1.0 - t) + 14.0 * t) as u8,
            255,
        ];
        preview.draw_horizontal_line(0, width - 1, y, row);
    }

    let emitter_x = width as f32 * 0.5;
    let emitter_y = height as f32 * 0.88;
    let spread_scale = (spread / std::f32::consts::PI).clamp(0.0, 1.0);
    let speed_scale = ((speed_min + speed_max) * 0.5).clamp(0.05, 8.0);
    let rate_scale = rate.clamp(1.0, 128.0);
    let radius_scale = ((radius_min + radius_max) * 0.5).clamp(0.01, 2.0);
    let count = ((rate_scale / 4.0).round() as usize).clamp(12, 72);

    for i in 0..count {
        let seed = i as f32 * 12.9898;
        let life = lifetime_min + (lifetime_max - lifetime_min) * (0.5 + 0.5 * (seed * 0.73).sin());
        let local_t = ((time * (0.65 + (seed * 0.17).sin().abs()) + i as f32 * 0.071)
            / life.max(0.05))
        .fract();
        let age = local_t;
        let rise = age.powf(0.82);
        let side = ((seed * 1.37).sin() * 0.5 + (time * 0.9 + seed).sin() * 0.5)
            * spread_scale
            * width as f32
            * 0.32;
        let swirl = (age * std::f32::consts::TAU + seed).sin() * width as f32 * 0.03;
        let x = emitter_x + side + swirl;
        let y = emitter_y - rise * height as f32 * (0.44 + speed_scale * 0.22);
        let size = ((radius_scale * 34.0) * (1.0 - age * 0.45)).clamp(5.0, 42.0);
        let alpha = ((1.0 - age).powf(1.15) * 0.9 + 0.1).clamp(0.0, 1.0);

        let jitter = color_variation as f32 * ((seed * 0.31).cos() * 0.5 + 0.5);
        let ramp_t = age.clamp(0.0, 0.999);
        let scaled = ramp_t * 3.0;
        let idx = scaled.floor() as usize;
        let frac = scaled.fract();
        let c0 = ramp[idx.min(3)];
        let c1 = ramp[(idx + 1).min(3)];
        let mut color = [0u8; 4];
        for channel in 0..3 {
            color[channel] = (c0[channel] as f32 * (1.0 - frac) + c1[channel] as f32 * frac)
                .clamp(0.0, 255.0) as u8;
        }
        color[0] = (color[0] as f32 + jitter * 0.35).clamp(0.0, 255.0) as u8;
        color[1] = (color[1] as f32 + jitter * 0.2).clamp(0.0, 255.0) as u8;
        color[2] = (color[2] as f32 + jitter * 0.1).clamp(0.0, 255.0) as u8;
        color[3] = (255.0 * alpha) as u8;
        draw_soft_particle(&mut preview, x, y, size, color);
    }

    let emitter_dim = TheDim::new((emitter_x as i32) - 6, (emitter_y as i32) - 6, 12, 12);
    preview.draw_disc(&emitter_dim, &[255, 255, 255, 140], 1.0, &[0, 0, 0, 0]);
    preview
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
