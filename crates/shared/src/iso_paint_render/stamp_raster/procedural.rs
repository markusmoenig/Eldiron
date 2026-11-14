//! Original procedural stamp kernels shared by reference and virtual targets.
use super::*;

impl IsoPaintRenderer {
    pub(in crate::iso_paint_render) fn iso_paint_blend_line(
        buffer: &mut impl Target,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        color: [u8; 4],
    ) {
        let dim = *buffer.dim();
        if dim.width <= 0 || dim.height <= 0 {
            return;
        }
        let mut x = x0;
        let mut y = y0;
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            if Self::iso_paint_stamp_pixel_visible(surface_buffer, stamp_depth, owner_geo_id, x, y)
            {
                buffer.blend(x, y, color);
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    pub(in crate::iso_paint_render) fn iso_paint_blend_stamp_pixel(
        buffer: &mut impl Target,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        x: i32,
        y: i32,
        color: [u8; 4],
    ) {
        if !Self::iso_paint_stamp_pixel_visible(surface_buffer, stamp_depth, owner_geo_id, x, y) {
            return;
        }
        buffer.blend(x, y, color);
    }

    pub(in crate::iso_paint_render) fn draw_iso_paint_rotated_ellipse(
        buffer: &mut impl Target,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        center: [i32; 2],
        radius_major: f32,
        radius_minor: f32,
        angle: f32,
        color: [u8; 4],
        variation: u32,
    ) {
        let radius_major = radius_major.max(1.0);
        let radius_minor = radius_minor.max(1.0);
        let cos = angle.cos();
        let sin = angle.sin();
        let bound = (radius_major.max(radius_minor) + 1.0).ceil() as i32;
        for y in -bound..=bound {
            for x in -bound..=bound {
                let lx = x as f32 * cos + y as f32 * sin;
                let ly = -x as f32 * sin + y as f32 * cos;
                let edge = lx * lx / (radius_major * radius_major)
                    + ly * ly / (radius_minor * radius_minor);
                if edge > 1.0 {
                    continue;
                }
                let hash = iso_paint_brush::hash_u32(center[0] + x, center[1] + y, variation);
                let noise = (hash & 0xff) as f32 / 255.0;
                let shade = if ly < -radius_minor * 0.35 {
                    1.08 + noise * 0.14
                } else if edge > 0.78 || ly > radius_minor * 0.45 {
                    0.62 + noise * 0.18
                } else {
                    0.82 + noise * 0.20
                };
                let mut pixel = Self::iso_paint_adjust_rgb(color, shade);
                if edge > 0.9 {
                    pixel[3] = ((pixel[3] as f32) * 0.65).round() as u8;
                }
                Self::iso_paint_blend_stamp_pixel(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    center[0] + x,
                    center[1] + y,
                    pixel,
                );
            }
        }
    }

    pub(in crate::iso_paint_render) fn draw_iso_paint_leaves_stamp(
        buffer: &mut impl Target,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        opacity: f32,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let leaf_count = 5 + (variation % 6) as i32;
        let spread = (size * 10.0).round().clamp(5.0, 38.0) as i32;
        let shadow = [12, 10, 7, (opacity * 42.0).round() as u8];
        for i in 0..leaf_count {
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x27d4_eb2d))
                .rotate_left(((i * 5) as u32) & 15);
            let ox = ((seed & 0xff) as i32 - 128) * spread / 190;
            let oy = (((seed >> 8) & 0xff) as i32 - 128) * spread / 300;
            let center = [screen[0] + ox, screen[1] + oy];
            let angle = rotation + (((seed >> 16) & 0xff) as f32 / 255.0 - 0.5) * 0.85;
            let major = size * (2.2 + ((seed >> 24) & 0x7f) as f32 / 75.0);
            let minor = major * (0.34 + ((seed >> 11) & 0x3f) as f32 / 260.0);
            let shade = 0.68 + ((seed >> 5) & 0xff) as f32 / 255.0 * 0.78;
            let mut leaf = Self::iso_paint_adjust_rgb(color, shade);
            leaf[3] = (opacity * 215.0).round() as u8;
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [center[0] + 1, center[1] + 1],
                major,
                minor,
                angle,
                shadow,
                seed ^ 0x51ad_0001,
            );
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                center,
                major,
                minor,
                angle,
                leaf,
                seed,
            );
            let vein_alpha = (opacity * 92.0).round() as u8;
            let vein = Self::iso_paint_adjust_rgb(leaf, 0.42);
            let vein = [vein[0], vein[1], vein[2], vein_alpha];
            let vx = (angle.cos() * major * 0.65).round() as i32;
            let vy = (angle.sin() * major * 0.65).round() as i32;
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                center[0] - vx,
                center[1] - vy,
                center[0] + vx,
                center[1] + vy,
                vein,
            );
        }
    }

    pub(in crate::iso_paint_render) fn draw_iso_paint_flowers_stamp(
        buffer: &mut impl Target,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        palette: &[[u8; 4]],
        opacity: f32,
        variant: &str,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let flower_count = match variant {
            "bluebells" => 3 + (variation % 3) as i32,
            "poppies" => 3 + (variation % 4) as i32,
            _ => 4 + (variation % 5) as i32,
        };
        let spread = (size * 8.0).round().clamp(5.0, 32.0) as i32;
        let stem_source = palette.first().copied().unwrap_or(color);
        let mut stem = Self::iso_paint_adjust_rgb(stem_source, 0.82);
        stem[3] = (opacity * 220.0).round() as u8;
        let mut leaf = Self::iso_paint_adjust_rgb(stem_source, 1.12);
        leaf[3] = (opacity * 165.0).round() as u8;
        let mut shadow = Self::iso_paint_adjust_rgb(stem_source, 0.18);
        shadow[3] = (opacity * 48.0).round() as u8;

        for i in 0..flower_count {
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x7feb_352d))
                .rotate_left(((i * 6) as u32) & 15);
            let ox = ((seed & 0xff) as i32 - 128) * spread / 190;
            let oy = (((seed >> 8) & 0xff) as i32 - 128) * spread / 420;
            let base = [screen[0] + ox, screen[1] + oy];
            let height = (size * (5.6 + ((seed >> 16) & 0x7f) as f32 / 32.0))
                .round()
                .clamp(5.0, 24.0) as i32;
            let lean = ((seed >> 24) as i32 & 0xff) - 128;
            let lean = lean * spread / 520 + (rotation.sin() * spread as f32 * 0.2).round() as i32;
            let tip = [base[0] + lean, base[1] - height];

            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base[0] + 1,
                base[1] + 1,
                tip[0] + 1,
                tip[1] + 1,
                shadow,
            );
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base[0],
                base[1],
                tip[0],
                tip[1],
                stem,
            );

            if i % 2 == 0 {
                let leaf_center = [
                    base[0] + lean / 2 + if seed & 1 == 0 { -1 } else { 1 },
                    base[1] - height / 2,
                ];
                Self::draw_iso_paint_rotated_ellipse(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    leaf_center,
                    (size * 1.5).clamp(1.2, 5.0),
                    (size * 0.55).clamp(0.9, 2.4),
                    rotation + if seed & 1 == 0 { -0.45 } else { 0.45 },
                    leaf,
                    seed ^ 0x3311_aa01,
                );
            }

            let petal_slot = if variant == "wildflowers" {
                1 + (((seed >> 13) as usize) % 3)
            } else {
                1
            };
            let petal_fallback = Self::iso_paint_adjust_rgb(stem_source, 1.25);
            let petal = Self::iso_paint_stamp_palette_color(
                palette,
                petal_slot,
                petal_fallback,
                opacity,
                225.0,
            );
            let radius = (size * (0.9 + ((seed >> 5) & 0x3f) as f32 / 120.0)).clamp(1.1, 4.2);
            let petal_count = if variant == "poppies" { 5 } else { 4 };
            for petal_index in 0..petal_count {
                let angle = rotation
                    + petal_index as f32 * std::f32::consts::TAU / petal_count as f32
                    + ((seed >> 9) & 0x1f) as f32 / 255.0;
                let center = if variant == "bluebells" {
                    [
                        tip[0] + ((petal_index as f32 - 1.5) * radius * 0.45).round() as i32,
                        tip[1] + (radius * (petal_index as f32 * 0.9 + 0.6)).round() as i32,
                    ]
                } else {
                    [
                        tip[0] + (angle.cos() * radius * 0.75).round() as i32,
                        tip[1] + (angle.sin() * radius * 0.55).round() as i32,
                    ]
                };
                Self::draw_iso_paint_rotated_ellipse(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    center,
                    radius,
                    if variant == "bluebells" {
                        radius * 0.8
                    } else {
                        radius * 0.62
                    },
                    angle,
                    petal,
                    seed ^ petal_index as u32,
                );
            }
            let center_slot = if variant == "wildflowers" { 2 } else { 3 };
            let center_fallback = Self::iso_paint_adjust_rgb(stem_source, 0.55);
            let center = Self::iso_paint_stamp_palette_color(
                palette,
                center_slot,
                center_fallback,
                opacity,
                if variant == "bluebells" { 150.0 } else { 230.0 },
            );
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                tip,
                (radius * 0.55).max(1.0),
                (radius * 0.45).max(1.0),
                rotation,
                center,
                seed ^ 0x7777_0013,
            );
        }
    }

    pub(in crate::iso_paint_render) fn draw_iso_paint_vines_stamp(
        buffer: &mut impl Target,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        palette: &[[u8; 4]],
        opacity: f32,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let mut stem = Self::iso_paint_stamp_palette_color(
            palette,
            0,
            Self::iso_paint_adjust_rgb(color, 0.74),
            opacity,
            225.0,
        );
        stem = Self::iso_paint_adjust_rgb(stem, 0.86);
        let leaf_a = Self::iso_paint_stamp_palette_color(
            palette,
            1,
            Self::iso_paint_adjust_rgb(color, 1.12),
            opacity,
            205.0,
        );
        let leaf_b = Self::iso_paint_stamp_palette_color(
            palette,
            2,
            Self::iso_paint_adjust_rgb(color, 0.92),
            opacity,
            190.0,
        );
        let mut shadow = Self::iso_paint_adjust_rgb(stem, 0.22);
        shadow[3] = (opacity * 54.0).round() as u8;

        let vine_count = 2 + (variation % 3) as i32;
        let spread = (size * 7.0).round().clamp(3.0, 28.0) as i32;
        for i in 0..vine_count {
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x632b_e5ab))
                .rotate_left(((i * 5) as u32) & 15);
            let base = [
                screen[0] + ((seed & 0xff) as i32 - 128) * spread / 220,
                screen[1] + (((seed >> 8) & 0xff) as i32 - 128) * spread / 300,
            ];
            let length = (size * (14.0 + ((seed >> 16) & 0x7f) as f32 / 4.6))
                .round()
                .clamp(10.0, 58.0);
            let angle = rotation - std::f32::consts::FRAC_PI_2
                + ((seed >> 24) & 0xff) as f32 / 255.0 * 1.35
                - 0.68;
            let dir = [angle.cos(), angle.sin()];
            let normal = [-dir[1], dir[0]];
            let sway = (size * (3.0 + ((seed >> 10) & 0x3f) as f32 / 22.0)).clamp(2.0, 13.0);
            let phase = ((seed >> 4) & 0xff) as f32 / 255.0 * std::f32::consts::TAU;
            let segments = 5 + ((seed >> 7) & 3) as i32;
            let mut prev = base;
            for step in 1..=segments {
                let t = step as f32 / segments as f32;
                let wave = (phase + t * std::f32::consts::TAU * 0.72).sin() * sway;
                let point = [
                    base[0] + (dir[0] * length * t + normal[0] * wave).round() as i32,
                    base[1] + (dir[1] * length * t + normal[1] * wave).round() as i32,
                ];
                Self::iso_paint_blend_line(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    prev[0] + 1,
                    prev[1] + 1,
                    point[0] + 1,
                    point[1] + 1,
                    shadow,
                );
                Self::iso_paint_blend_line(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    prev[0],
                    prev[1],
                    point[0],
                    point[1],
                    stem,
                );

                if step % 2 == 0 || step == segments {
                    let side = if (seed >> (step as u32)) & 1 == 0 {
                        -1.0
                    } else {
                        1.0
                    };
                    let leaf_center = [
                        point[0] + (normal[0] * side * size * 2.2).round() as i32,
                        point[1] + (normal[1] * side * size * 2.2).round() as i32,
                    ];
                    let leaf_seed = seed ^ (step as u32).wrapping_mul(0x45d9_f3b);
                    Self::draw_iso_paint_rotated_ellipse(
                        buffer,
                        surface_buffer,
                        stamp_depth,
                        owner_geo_id,
                        leaf_center,
                        (size * 2.4).clamp(1.5, 8.0),
                        (size * 0.85).clamp(0.8, 3.2),
                        angle + side * 0.78,
                        if step % 3 == 0 { leaf_b } else { leaf_a },
                        leaf_seed,
                    );
                }
                prev = point;
            }
        }
    }

    pub(in crate::iso_paint_render) fn draw_iso_paint_roots_stamp(
        buffer: &mut impl Target,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        palette: &[[u8; 4]],
        opacity: f32,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let mut root = Self::iso_paint_stamp_palette_color(
            palette,
            0,
            Self::iso_paint_adjust_rgb(color, 0.82),
            opacity,
            230.0,
        );
        root = Self::iso_paint_adjust_rgb(root, 0.92);
        let dark = Self::iso_paint_stamp_palette_color(
            palette,
            1,
            Self::iso_paint_adjust_rgb(color, 0.48),
            opacity,
            180.0,
        );
        let highlight = Self::iso_paint_stamp_palette_color(
            palette,
            2,
            Self::iso_paint_adjust_rgb(color, 1.18),
            opacity,
            145.0,
        );
        let branch_count = 3 + (variation % 4) as i32;
        let spread = (size * 9.0).round().clamp(5.0, 34.0) as i32;
        let base_angle = rotation + std::f32::consts::FRAC_PI_2;

        for i in 0..branch_count {
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x94d0_49bb))
                .rotate_left(((i * 4) as u32) & 15);
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            let angle = base_angle + side * (0.35 + ((seed >> 8) & 0xff) as f32 / 255.0 * 0.7);
            let dir = [angle.cos(), angle.sin()];
            let normal = [-dir[1], dir[0]];
            let start = [
                screen[0] + (((seed >> 16) & 0xff) as i32 - 128) * spread / 300,
                screen[1] + (((seed >> 24) & 0xff) as i32 - 128) * spread / 420,
            ];
            let length = (size * (11.0 + ((seed >> 4) & 0x7f) as f32 / 5.0))
                .round()
                .clamp(9.0, 42.0);
            let bend = (size * (2.0 + ((seed >> 11) & 0x3f) as f32 / 24.0)).clamp(1.5, 10.0);
            let segments = 4 + ((seed >> 19) & 3) as i32;
            let mut prev = start;
            for step in 1..=segments {
                let t = step as f32 / segments as f32;
                let wave = (t * std::f32::consts::PI).sin() * bend * side;
                let point = [
                    start[0] + (dir[0] * length * t + normal[0] * wave).round() as i32,
                    start[1] + (dir[1] * length * t + normal[1] * wave).round() as i32,
                ];
                Self::iso_paint_blend_line(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    prev[0] + 1,
                    prev[1] + 1,
                    point[0] + 1,
                    point[1] + 1,
                    dark,
                );
                Self::iso_paint_blend_line(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    prev[0],
                    prev[1],
                    point[0],
                    point[1],
                    root,
                );
                if size > 1.35 && step < segments {
                    Self::iso_paint_blend_line(
                        buffer,
                        surface_buffer,
                        stamp_depth,
                        owner_geo_id,
                        prev[0],
                        prev[1] - 1,
                        point[0],
                        point[1] - 1,
                        root,
                    );
                }

                if step == 2 || (step == segments - 1 && seed & 1 == 0) {
                    let twig_angle = angle - side * 0.7;
                    let twig_len = (size * (4.5 + ((seed >> 13) & 0x3f) as f32 / 16.0))
                        .round()
                        .clamp(3.0, 16.0);
                    let twig_end = [
                        point[0] + (twig_angle.cos() * twig_len).round() as i32,
                        point[1] + (twig_angle.sin() * twig_len).round() as i32,
                    ];
                    Self::iso_paint_blend_line(
                        buffer,
                        surface_buffer,
                        stamp_depth,
                        owner_geo_id,
                        point[0],
                        point[1],
                        twig_end[0],
                        twig_end[1],
                        dark,
                    );
                }
                prev = point;
            }

            if i % 2 == 0 {
                Self::draw_iso_paint_rotated_ellipse(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    start,
                    (size * 1.35).clamp(1.0, 5.0),
                    (size * 0.8).clamp(0.8, 3.0),
                    angle,
                    highlight,
                    seed ^ 0x7015_0001,
                );
            }
        }
    }

    pub(in crate::iso_paint_render) fn draw_iso_paint_leaf_mass(
        buffer: &mut impl Target,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        center: [i32; 2],
        radius_x: f32,
        radius_y: f32,
        seed: u32,
        dark: [u8; 4],
        mid: [u8; 4],
        light: [u8; 4],
    ) {
        let radius_x = radius_x.max(2.0);
        let radius_y = radius_y.max(2.0);
        let bound_x = (radius_x + 2.0).ceil() as i32;
        let bound_y = (radius_y + 2.0).ceil() as i32;
        for y in -bound_y..=bound_y {
            for x in -bound_x..=bound_x {
                let nx = x as f32 / radius_x;
                let ny = y as f32 / radius_y;
                let edge = nx * nx + ny * ny;
                let hash = iso_paint_brush::hash_u32(center[0] + x, center[1] + y, seed);
                let noise = (hash & 0xff) as f32 / 255.0;
                let wobble = (((hash >> 8) & 0xff) as f32 / 255.0 - 0.5) * 0.34;
                if edge > 0.94 + wobble {
                    continue;
                }
                if edge > 0.62 && ((hash >> 17) & 7) == 0 {
                    continue;
                }
                if edge < 0.36 && ((hash >> 21) & 31) == 0 {
                    continue;
                }

                let mut pixel = if ny < -0.34 && noise > 0.34 {
                    light
                } else if ny > 0.28 || edge > 0.72 {
                    dark
                } else {
                    mid
                };
                let shade = 0.76 + noise * 0.42 + (-ny).max(0.0) * 0.16;
                pixel = Self::iso_paint_adjust_rgb(pixel, shade);
                if edge > 0.76 {
                    pixel[3] = ((pixel[3] as f32) * (0.58 + noise * 0.32)).round() as u8;
                }
                Self::iso_paint_blend_stamp_pixel(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    center[0] + x,
                    center[1] + y,
                    pixel,
                );

                if radius_x > 10.0 && noise > 0.91 && edge < 0.68 {
                    let mut fleck = light;
                    fleck[3] = ((fleck[3] as f32) * 0.72).round() as u8;
                    Self::iso_paint_blend_stamp_pixel(
                        buffer,
                        surface_buffer,
                        stamp_depth,
                        owner_geo_id,
                        center[0] + x + 1,
                        center[1] + y,
                        fleck,
                    );
                }
            }
        }
    }

    pub(in crate::iso_paint_render) fn draw_iso_paint_bushes_stamp(
        buffer: &mut impl Target,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        palette: &[[u8; 4]],
        opacity: f32,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let dark = Self::iso_paint_stamp_palette_color(
            palette,
            0,
            Self::iso_paint_adjust_rgb(color, 0.68),
            opacity,
            220.0,
        );
        let mid = Self::iso_paint_stamp_palette_color(
            palette,
            1,
            Self::iso_paint_adjust_rgb(color, 0.98),
            opacity,
            230.0,
        );
        let light = Self::iso_paint_stamp_palette_color(
            palette,
            2,
            Self::iso_paint_adjust_rgb(color, 1.22),
            opacity,
            175.0,
        );
        let branch =
            Self::iso_paint_stamp_wood_color(palette, 3, [74, 49, 28, 255], opacity, 210.0);
        let bark_dark = Self::iso_paint_adjust_rgb(branch, 0.48);
        let root_y = screen[1];
        let art_size = (size * 0.58).clamp(1.0, 5.0);
        let stem_count = 2 + (variation % 2) as i32;
        let spread_x = (art_size * 3.0).round().clamp(3.0, 12.0) as i32;
        let stem_height = (art_size * 6.6).round().clamp(7.0, 30.0) as i32;

        for i in 0..stem_count {
            let seed = variation ^ (i as u32 + 1).wrapping_mul(0x45d9_f3b);
            let lane = i - stem_count / 2;
            let base_x = screen[0] + lane * spread_x / stem_count.max(1);
            let lean = (((seed >> 8) & 0xff) as i32 - 128) * spread_x / 360
                + (rotation.sin() * art_size * 1.2).round() as i32;
            let top = [
                base_x + lean,
                root_y - stem_height + (((seed >> 16) & 0x1f) as i32 * stem_height / 170),
            ];
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base_x + 1,
                root_y,
                top[0] + 1,
                top[1],
                bark_dark,
            );
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base_x,
                root_y,
                top[0],
                top[1],
                branch,
            );

            for node in 0..2 {
                let t = 0.42 + node as f32 * 0.25;
                let center = [
                    (base_x as f32 + (top[0] - base_x) as f32 * t).round() as i32,
                    (root_y as f32 + (top[1] - root_y) as f32 * t).round() as i32,
                ];
                let side = if (seed >> (node as u32)) & 1 == 0 {
                    -1.0
                } else {
                    1.0
                };
                let leaf_center = [
                    center[0] + (side * art_size * (0.95 + node as f32 * 0.42)).round() as i32,
                    center[1] - (art_size * 0.35).round() as i32,
                ];
                let leaf_seed = seed ^ (node as u32).wrapping_mul(0x9e37_79b9);
                Self::draw_iso_paint_leaf_mass(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    leaf_center,
                    (art_size * (0.78 + node as f32 * 0.12)).clamp(1.6, 4.0),
                    (art_size * (0.98 + node as f32 * 0.12)).clamp(2.0, 5.4),
                    leaf_seed,
                    dark,
                    mid,
                    light,
                );
            }

            let tip_leaf = if i % 2 == 0 { light } else { mid };
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                top,
                (art_size * 0.72).clamp(1.3, 3.7),
                (art_size * 1.05).clamp(1.8, 5.2),
                rotation + lean as f32 * 0.03,
                tip_leaf,
                seed ^ 0xb055_0001,
            );

            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base_x + 1,
                root_y,
                top[0] + 1,
                top[1],
                bark_dark,
            );
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base_x,
                root_y,
                top[0],
                top[1],
                branch,
            );
        }

        let base = Self::iso_paint_adjust_rgb(branch, 0.62);
        for dx in -spread_x / 2..=spread_x / 2 {
            if dx.abs() <= spread_x / 3 {
                Self::iso_paint_blend_stamp_pixel(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    screen[0] + dx,
                    root_y,
                    base,
                );
            }
        }
    }

    pub(in crate::iso_paint_render) fn draw_iso_paint_tree_stamp(
        buffer: &mut impl Target,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        palette: &[[u8; 4]],
        opacity: f32,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let leaf_dark = Self::iso_paint_stamp_palette_color(
            palette,
            0,
            Self::iso_paint_adjust_rgb(color, 0.7),
            opacity,
            230.0,
        );
        let leaf_mid = Self::iso_paint_stamp_palette_color(
            palette,
            1,
            Self::iso_paint_adjust_rgb(color, 1.02),
            opacity,
            235.0,
        );
        let trunk =
            Self::iso_paint_stamp_palette_color(palette, 2, [92, 58, 36, 255], opacity, 225.0);
        let leaf_light = Self::iso_paint_stamp_palette_color(
            palette,
            3,
            Self::iso_paint_adjust_rgb(color, 1.25),
            opacity,
            185.0,
        );
        let base = [screen[0], screen[1] + (size * 2.4).round() as i32];
        let canopy_center = [screen[0], screen[1] - (size * 12.5).round() as i32];
        let shadow = [6, 7, 5, (opacity * 68.0).round() as u8];
        Self::draw_iso_paint_rotated_ellipse(
            buffer,
            surface_buffer,
            stamp_depth,
            owner_geo_id,
            base,
            (size * 4.5).clamp(3.0, 18.0),
            (size * 1.8).clamp(1.4, 8.0),
            rotation * 0.2,
            shadow,
            variation ^ 0x7aee_0001,
        );

        let trunk_height = (size * 14.0).round().clamp(10.0, 46.0) as i32;
        let trunk_width = (size * 1.5).round().clamp(2.0, 7.0) as i32;
        for dx in -trunk_width..=trunk_width {
            let shade = if dx < 0 { 1.12 } else { 0.64 };
            let trunk_pixel = Self::iso_paint_adjust_rgb(trunk, shade);
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base[0] + dx,
                base[1],
                base[0] + dx / 2,
                base[1] - trunk_height,
                trunk_pixel,
            );
        }

        let bark_dark = Self::iso_paint_adjust_rgb(trunk, 0.42);
        for stripe in [-1, 1] {
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base[0] + stripe,
                base[1] - 1,
                base[0] + stripe / 2,
                base[1] - trunk_height + 2,
                bark_dark,
            );
        }

        for side in [-1.0_f32, 1.0] {
            let start = [base[0], base[1] - trunk_height * 2 / 3];
            let end = [
                start[0] + (side * size * 7.2).round() as i32,
                start[1] - (size * 5.3).round() as i32,
            ];
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                start[0],
                start[1],
                end[0],
                end[1],
                bark_dark,
            );
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                start[0],
                start[1] - 1,
                end[0],
                end[1] - 1,
                trunk,
            );
        }

        let crown = [
            (-8.5_f32, -7.0_f32, 7.4_f32, 7.2_f32),
            (6.5, -9.5, 8.2, 7.8),
            (-15.0, -1.8, 7.0, 6.8),
            (14.5, -1.2, 7.0, 6.6),
            (-5.0, 3.5, 9.2, 7.2),
            (6.5, 4.5, 8.4, 6.5),
            (0.0, -16.0, 7.4, 7.4),
            (-1.5, -3.6, 10.8, 8.2),
        ];
        for (i, (ox, oy, rx, ry)) in crown.iter().enumerate() {
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x85eb_ca6b))
                .rotate_left(((i * 6) as u32) & 15);
            let jitter_x = ((seed & 0xff) as f32 / 255.0 - 0.5) * size * 2.4;
            let jitter_y = (((seed >> 8) & 0xff) as f32 / 255.0 - 0.5) * size * 2.0;
            let center = [
                canopy_center[0] + (ox * size + jitter_x).round() as i32,
                canopy_center[1] + (oy * size + jitter_y).round() as i32,
            ];
            Self::draw_iso_paint_leaf_mass(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                center,
                (rx * size).clamp(4.0, 28.0),
                (ry * size).clamp(3.0, 24.0),
                seed,
                leaf_dark,
                leaf_mid,
                leaf_light,
            );
        }

        for i in 0..18 {
            let seed = variation ^ (i as u32).wrapping_mul(0x27d4_eb2d);
            if (seed & 3) == 0 {
                continue;
            }
            let x = canopy_center[0] + (((seed >> 8) & 0xff) as i32 - 128) * (size as i32 + 10) / 9;
            let y =
                canopy_center[1] + (((seed >> 16) & 0xff) as i32 - 128) * (size as i32 + 8) / 11;
            let fleck = if (seed & 8) == 0 {
                leaf_light
            } else {
                leaf_dark
            };
            Self::iso_paint_blend_stamp_pixel(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                x,
                y,
                fleck,
            );
        }
    }

    pub(in crate::iso_paint_render) fn draw_iso_paint_candles_stamp(
        buffer: &mut impl Target,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        palette: &[[u8; 4]],
        opacity: f32,
        variation: u32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let wax = Self::iso_paint_stamp_palette_color(palette, 0, color, opacity, 235.0);
        let side = Self::iso_paint_stamp_palette_color(
            palette,
            1,
            Self::iso_paint_adjust_rgb(wax, 0.66),
            opacity,
            210.0,
        );
        let flame =
            Self::iso_paint_stamp_palette_color(palette, 2, [255, 151, 45, 230], opacity, 230.0);
        let core =
            Self::iso_paint_stamp_palette_color(palette, 3, [255, 239, 142, 245], opacity, 245.0);
        let candle_count = 1 + (variation % 3) as i32;
        let shadow = [8, 6, 4, (opacity * 62.0).round() as u8];
        let glow = [255, 151, 45, (opacity * 32.0).round() as u8];
        for i in 0..candle_count {
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x85eb_ca6b))
                .rotate_left(((i * 6) as u32) & 15);
            let offset = (i - (candle_count - 1) / 2) as f32;
            let jitter = ((seed & 0xff) as f32 / 255.0 - 0.5) * size * 4.0;
            let base = [
                screen[0] + (offset * size * 6.0 + jitter).round() as i32,
                screen[1] + (((seed >> 8) & 0x3f) as f32 / 63.0 * size * 3.0).round() as i32,
            ];
            let height = (size * (8.0 + ((seed >> 14) & 0x7f) as f32 / 13.0))
                .round()
                .clamp(7.0, 28.0) as i32;
            let half_width = (size * (1.45 + ((seed >> 22) & 0x3f) as f32 / 72.0))
                .round()
                .clamp(1.0, 6.0) as i32;
            let top_y = base[1] - height;
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [base[0], base[1] + 1],
                half_width as f32 + 1.2,
                (size * 1.0).clamp(0.8, 3.2),
                0.0,
                shadow,
                seed ^ 0x1188_0001,
            );
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [base[0], top_y - (size * 5.3).round() as i32],
                (size * 4.4).clamp(2.0, 14.0),
                (size * 5.6).clamp(2.6, 18.0),
                0.0,
                glow,
                seed ^ 0x7f7f_0009,
            );
            for dx in -half_width..=half_width {
                let mut body = if dx > half_width / 2 { side } else { wax };
                let shade = if dx < -half_width / 2 { 1.09 } else { 1.0 };
                body = Self::iso_paint_adjust_rgb(body, shade);
                Self::iso_paint_blend_line(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    base[0] + dx,
                    base[1],
                    base[0] + dx,
                    top_y,
                    body,
                );
            }
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [base[0], base[1]],
                half_width as f32 + 0.6,
                (size * 0.85).clamp(0.7, 2.8),
                0.0,
                side,
                seed ^ 0x5511_0001,
            );
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [base[0], top_y],
                half_width as f32 + 0.4,
                (size * 0.75).clamp(0.7, 2.4),
                0.0,
                wax,
                seed ^ 0x5511_0002,
            );
            let wick = [24, 17, 12, (opacity * 185.0).round() as u8];
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base[0],
                top_y,
                base[0],
                top_y - (size * 2.3).round().max(2.0) as i32,
                wick,
            );
            let flame_y = top_y - (size * 4.2).round().max(4.0) as i32;
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [base[0], flame_y],
                (size * 1.7).clamp(1.2, 5.5),
                (size * 3.2).clamp(2.0, 8.8),
                0.0,
                flame,
                seed ^ 0xf17e_0001,
            );
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [base[0], flame_y + (size * 0.55).round() as i32],
                (size * 0.85).clamp(0.8, 3.0),
                (size * 1.55).clamp(1.0, 4.8),
                0.0,
                core,
                seed ^ 0xf17e_0002,
            );
        }
    }

    pub(in crate::iso_paint_render) fn draw_iso_paint_stamp_shape(
        buffer: &mut impl Target,
        stamp: &IsoPaintStamp,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
    ) {
        if let Some(spec) = stamp
            .source_image
            .as_deref()
            .and_then(crate::paint_source::Spec::from_tile)
        {
            // Evaluate the original artwork at the actual placement size. The picker image
            // is a thumbnail, never a replacement for the native stamp generator.
            let mut native = stamp.clone();
            native.source_image = None;
            native.source_material_pass = false;
            native.kind = spec.brush;
            native.color = spec.colors[0];
            native.palette_colors = spec.colors;
            native.material_id = spec.material_id;
            native.variant = spec.variant;
            native.variation ^= spec.seed.wrapping_sub(19);
            if IsoPaintStamp::keeps_upright(&native.kind) {
                native.rotation = 0.;
            }
            let [r, m, o, e] =
                rusterix::material_library::MaterialDefinition::from_id(spec.material_id)
                    .rmoe_values();
            native.opacity *= o;
            if stamp.source_material_pass {
                let mut pixels = buffer.blank();
                Self::draw_iso_paint_stamp_shape(
                    &mut pixels,
                    &native,
                    surface_buffer,
                    screen,
                    stamp_depth,
                    owner_geo_id,
                    size,
                );
                let [rm, oe] = rusterix::Texture::encode_material_bytes(r, m, o, e);
                buffer.material_mask(pixels, [64 + spec.material_id.min(59), rm, oe]);
            } else {
                Self::draw_iso_paint_stamp_shape(
                    buffer,
                    &native,
                    surface_buffer,
                    screen,
                    stamp_depth,
                    owner_geo_id,
                    size,
                );
            }
            return;
        }
        if let Some(tile) = &stamp.source_image
            && let Some(layout) = &tile.paint
        {
            let scale = size / layout.size.max(0.01);
            // Correct previously placed stamps as well as new ones. Only flat
            // details such as leaves and footprints rotate their whole bitmap.
            let rotation = if IsoPaintStamp::keeps_upright(&layout.brush) {
                0.
            } else {
                stamp.rotation
            };
            let (sin, cos) = rotation.sin_cos();
            let width = layout.native_size[0] as f32 * scale;
            let height = layout.native_size[1] as f32 * scale;
            let anchor = layout.anchor.map(|v| v * scale);
            let corners = [[0., 0.], [width, 0.], [0., height], [width, height]].map(|p| {
                let dx = p[0] - anchor[0];
                let dy = p[1] - anchor[1];
                [dx * cos - dy * sin, dx * sin + dy * cos]
            });
            let dim = *buffer.dim();
            let min_x = corners
                .iter()
                .map(|p| p[0])
                .fold(f32::INFINITY, f32::min)
                .floor() as i32;
            let max_x = corners
                .iter()
                .map(|p| p[0])
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil() as i32;
            let min_y = corners
                .iter()
                .map(|p| p[1])
                .fold(f32::INFINITY, f32::min)
                .floor() as i32;
            let max_y = corners
                .iter()
                .map(|p| p[1])
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil() as i32;
            // Inverse sampling keeps rotated/scaled sprites solid without holes between pixels.
            for y in min_y.max(-screen[1])..max_y.min(dim.height - screen[1]) {
                for x in min_x.max(-screen[0])..max_x.min(dim.width - screen[0]) {
                    let sx = x as f32 * cos + y as f32 * sin + anchor[0];
                    let sy = -x as f32 * sin + y as f32 * cos + anchor[1];
                    if sx < 0. || sy < 0. || sx >= width || sy >= height {
                        continue;
                    }
                    let (mut color, material) = tile
                        .sample_paint_surface([sx / width, sy / height], stamp.variation as usize);
                    if color[3] == 0 {
                        continue;
                    }
                    color[3] = (color[3] as f32 * stamp.opacity).round() as u8;
                    let at = [screen[0] + x, screen[1] + y];
                    if stamp.source_material_pass {
                        if Self::iso_paint_stamp_pixel_visible(
                            surface_buffer,
                            stamp_depth,
                            owner_geo_id,
                            at[0],
                            at[1],
                        ) {
                            let [rm, oe] = rusterix::Texture::encode_material_bytes(
                                material[0],
                                material[1],
                                material[2],
                                material[3],
                            );
                            buffer.replace(
                                at[0],
                                at[1],
                                [64 + stamp.material_id.min(59), rm, oe, color[3]],
                            );
                        }
                    } else {
                        Self::iso_paint_blend_stamp_pixel(
                            buffer,
                            surface_buffer,
                            stamp_depth,
                            owner_geo_id,
                            at[0],
                            at[1],
                            color,
                        );
                    }
                }
            }
            return;
        }
        match stamp.kind.as_str() {
            "grass" | "grass_stamp" => Self::draw_iso_paint_grass_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                stamp.opacity,
                stamp.variation,
                stamp.rotation,
            ),
            "rubble" => Self::draw_iso_paint_rubble_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                stamp.opacity,
                stamp.variation,
                stamp.rotation,
            ),
            "leaves" => Self::draw_iso_paint_leaves_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                stamp.opacity,
                stamp.variation,
                stamp.rotation,
            ),
            "flowers" => Self::draw_iso_paint_flowers_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                &stamp.palette_colors,
                stamp.opacity,
                stamp.variant.as_str(),
                stamp.variation,
                stamp.rotation,
            ),
            "vines" => Self::draw_iso_paint_vines_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                &stamp.palette_colors,
                stamp.opacity,
                stamp.variation,
                stamp.rotation,
            ),
            "roots" => Self::draw_iso_paint_roots_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                &stamp.palette_colors,
                stamp.opacity,
                stamp.variation,
                stamp.rotation,
            ),
            "bushes" => Self::draw_iso_paint_bushes_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                &stamp.palette_colors,
                stamp.opacity,
                stamp.variation,
                stamp.rotation,
            ),
            "tree" => Self::draw_iso_paint_tree_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                &stamp.palette_colors,
                stamp.opacity,
                stamp.variation,
                stamp.rotation,
            ),
            "candles" => Self::draw_iso_paint_candles_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                &stamp.palette_colors,
                stamp.opacity,
                stamp.variation,
            ),
            "footprints" => Self::draw_iso_paint_footprints_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                stamp.opacity,
                stamp.variation,
                stamp.rotation,
            ),
            "mud" => Self::draw_iso_paint_mud_stamp(
                buffer,
                surface_buffer,
                screen,
                stamp_depth,
                owner_geo_id,
                size,
                stamp.color,
                stamp.opacity,
                stamp.variation,
                stamp.rotation,
            ),
            _ => {}
        }
    }

    pub(in crate::iso_paint_render) fn draw_iso_paint_footprints_stamp(
        buffer: &mut impl Target,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        opacity: f32,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let angle = rotation;
        let forward = [angle.cos(), angle.sin()];
        let side = [-forward[1], forward[0]];
        let step = (size * 4.6).round().clamp(3.0, 16.0);
        let stride = (size * 4.2).round().clamp(3.0, 16.0);
        let foot_len = (size * 3.5).clamp(3.0, 13.0);
        let foot_w = (size * 1.35).clamp(1.2, 5.0);
        let shadow = [8, 6, 5, (opacity * 45.0).round() as u8];
        for i in 0..2 {
            let phase = if i == 0 { -1.0 } else { 1.0 };
            let seed = variation ^ (i as u32 + 1).wrapping_mul(0x9e37_79b9);
            let center = [
                screen[0]
                    + (side[0] * step * phase + forward[0] * stride * phase * 0.55).round() as i32,
                screen[1]
                    + (side[1] * step * phase + forward[1] * stride * phase * 0.55).round() as i32,
            ];
            let foot_angle = angle + phase * 0.16;
            let shade = 0.64 + ((seed >> 12) & 0xff) as f32 / 255.0 * 0.26;
            let mut print = Self::iso_paint_adjust_rgb(color, shade);
            print[3] = (opacity * 190.0).round() as u8;
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [center[0] + 1, center[1] + 1],
                foot_len,
                foot_w,
                foot_angle,
                shadow,
                seed ^ 0x5a5a_0011,
            );
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                center,
                foot_len,
                foot_w,
                foot_angle,
                print,
                seed,
            );
            let toe = [
                center[0] + (forward[0] * foot_len * 0.68).round() as i32,
                center[1] + (forward[1] * foot_len * 0.68).round() as i32,
            ];
            for toe_side in [-0.9_f32, 0.0, 0.9] {
                let toe_center = [
                    toe[0] + (side[0] * foot_w * toe_side).round() as i32,
                    toe[1] + (side[1] * foot_w * toe_side).round() as i32,
                ];
                Self::draw_iso_paint_rotated_ellipse(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    toe_center,
                    foot_w * 0.48,
                    foot_w * 0.42,
                    foot_angle,
                    print,
                    seed ^ ((toe_side.to_bits()).wrapping_mul(0x45d9_f3b)),
                );
            }
        }
    }

    pub(in crate::iso_paint_render) fn draw_iso_paint_mud_stamp(
        buffer: &mut impl Target,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        opacity: f32,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let spread = (size * 9.0).round().clamp(5.0, 34.0) as i32;
        let shadow = [8, 6, 5, (opacity * 48.0).round() as u8];
        let mut base = Self::iso_paint_adjust_rgb(color, 0.78);
        base[3] = (opacity * 165.0).round() as u8;
        for i in 0..3 {
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x4cf5_ad43))
                .rotate_left(((i * 4) as u32) & 15);
            let ox = if i == 0 {
                0
            } else {
                ((seed & 0xff) as i32 - 128) * spread / 260
            };
            let oy = if i == 0 {
                0
            } else {
                (((seed >> 8) & 0xff) as i32 - 128) * spread / 360
            };
            let center = [screen[0] + ox, screen[1] + oy];
            let angle = rotation * 0.18 + (((seed >> 18) & 0xff) as f32 / 255.0 - 0.5) * 0.5;
            let major = size * (4.3 + ((seed >> 10) & 0x7f) as f32 / 52.0);
            let minor = size * (2.0 + ((seed >> 25) & 0x3f) as f32 / 55.0);
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [center[0] + 1, center[1] + 1],
                major,
                minor,
                angle,
                shadow,
                seed ^ 0x011d_1111,
            );
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                center,
                major,
                minor,
                angle,
                base,
                seed,
            );
        }

        let bubble_count = 3 + (variation % 4) as i32;
        for i in 0..bubble_count {
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x9e37_79b9))
                .rotate_left(((i * 7) as u32) & 15);
            let ox = ((seed & 0xff) as i32 - 128) * spread / 180;
            let oy = (((seed >> 8) & 0xff) as i32 - 128) * spread / 280;
            let center = [screen[0] + ox, screen[1] + oy];
            let radius = (size * (1.05 + ((seed >> 16) & 0x7f) as f32 / 92.0)).clamp(1.2, 6.0);
            let mut dome = Self::iso_paint_adjust_rgb(color, 1.18);
            dome[3] = (opacity * 122.0).round() as u8;
            let mut rim = Self::iso_paint_adjust_rgb(color, 0.54);
            rim[3] = (opacity * 120.0).round() as u8;
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                center,
                radius,
                radius * 0.72,
                rotation,
                rim,
                seed ^ 0x8b8b_0001,
            );
            Self::draw_iso_paint_rotated_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                [center[0], center[1] - 1],
                radius * 0.68,
                radius * 0.44,
                rotation,
                dome,
                seed,
            );
            let highlight = [210, 224, 208, (opacity * 112.0).round() as u8];
            Self::iso_paint_blend_stamp_pixel(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                center[0] - radius.round() as i32 / 2,
                center[1] - radius.round() as i32 / 2,
                highlight,
            );
        }
    }

    pub(in crate::iso_paint_render) fn draw_iso_paint_rubble_ellipse(
        buffer: &mut impl Target,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        center: [i32; 2],
        radius_x: i32,
        radius_y: i32,
        color: [u8; 4],
        variation: u32,
    ) {
        let radius_x = radius_x.max(1);
        let radius_y = radius_y.max(1);
        let rx2 = (radius_x * radius_x).max(1) as f32;
        let ry2 = (radius_y * radius_y).max(1) as f32;
        for y in -radius_y..=radius_y {
            for x in -radius_x..=radius_x {
                let edge = x as f32 * x as f32 / rx2 + y as f32 * y as f32 / ry2;
                if edge > 1.0 {
                    continue;
                }
                let hash = iso_paint_brush::hash_u32(center[0] + x, center[1] + y, variation);
                let noise = (hash & 0xff) as f32 / 255.0;
                let shade = if y <= -radius_y / 3 && x <= radius_x / 3 {
                    1.18 + noise * 0.16
                } else if y >= radius_y / 3 || edge > 0.78 {
                    0.56 + noise * 0.16
                } else {
                    0.80 + noise * 0.24
                };
                let mut pixel = Self::iso_paint_adjust_rgb(color, shade);
                if edge > 0.88 {
                    pixel[3] = ((pixel[3] as f32) * 0.72).round() as u8;
                }
                Self::iso_paint_blend_stamp_pixel(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    center[0] + x,
                    center[1] + y,
                    pixel,
                );
            }
        }
    }

    pub(in crate::iso_paint_render) fn draw_iso_paint_rubble_stamp(
        buffer: &mut impl Target,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        opacity: f32,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let stone_count = 4 + (variation % 5) as i32;
        let spread = (size * 9.0).round().clamp(5.0, 36.0) as i32;
        let shadow = [9, 8, 7, (opacity * 72.0).round() as u8];
        for i in 0..stone_count {
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x85eb_ca6b))
                .rotate_left(((i * 3) as u32) & 15);
            let ox = ((seed & 0xff) as i32 - 128) * spread / 210;
            let oy = (((seed >> 8) & 0xff) as i32 - 128) * spread / 360;
            let lean = (rotation.sin() * spread as f32 * 0.18).round() as i32;
            let center = [screen[0] + ox + lean, screen[1] + oy];
            let radius_x = (size * (1.7 + ((seed >> 16) & 0x7f) as f32 / 90.0))
                .round()
                .clamp(2.0, 10.0) as i32;
            let radius_y = (radius_x as f32 * (0.42 + ((seed >> 23) & 0x3f) as f32 / 180.0))
                .round()
                .max(1.0) as i32;
            for sx in -radius_x..=radius_x {
                Self::iso_paint_blend_stamp_pixel(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    center[0] + sx,
                    center[1] + radius_y,
                    shadow,
                );
            }
            let shade = 0.68 + ((seed >> 11) & 0xff) as f32 / 255.0 * 0.64;
            let mut stone = Self::iso_paint_adjust_rgb(color, shade);
            stone[3] = (opacity * 235.0).round() as u8;
            Self::draw_iso_paint_rubble_ellipse(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                center,
                radius_x,
                radius_y,
                stone,
                seed,
            );
        }
    }

    pub(in crate::iso_paint_render) fn draw_iso_paint_grass_stamp(
        buffer: &mut impl Target,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
        color: [u8; 4],
        opacity: f32,
        variation: u32,
        rotation: f32,
    ) {
        let opacity = opacity.clamp(0.0, 1.0);
        let blade_count = 5 + (variation % 5) as i32;
        let base_alpha = (opacity * 235.0).round() as u8;
        let base_color = [color[0], color[1], color[2], base_alpha];
        let shadow = [7, 11, 8, (opacity * 72.0).round() as u8];
        let height = (size * 12.0).round().clamp(10.0, 56.0) as i32;
        let spread = (size * 5.0).round().clamp(4.0, 28.0) as i32;
        for sx in -spread / 2..=spread / 2 {
            let x = screen[0] + sx;
            if Self::iso_paint_stamp_pixel_visible(
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                x,
                screen[1],
            ) {
                buffer.blend(x, screen[1], shadow);
            }
        }
        for i in 0..blade_count {
            let lane = i - blade_count / 2;
            let seed = variation
                .wrapping_add((i as u32).wrapping_mul(0x9e37_79b9))
                .rotate_left((i as u32) & 15);
            let bend = ((seed & 0xff) as i32 - 128) * spread / 190;
            let lean = (rotation.sin() * spread as f32 * 0.45).round() as i32;
            let base_x = screen[0] + lane * spread / blade_count.max(1);
            let top_x = base_x + bend + lean;
            let top_y = screen[1] - height + ((seed >> 8) & 9) as i32;
            let shade = 0.68 + ((seed >> 16) & 0xff) as f32 / 255.0 * 0.68;
            let blade = Self::iso_paint_adjust_rgb(base_color, shade);
            Self::iso_paint_blend_line(
                buffer,
                surface_buffer,
                stamp_depth,
                owner_geo_id,
                base_x,
                screen[1],
                top_x,
                top_y,
                blade,
            );
            if size > 1.7 {
                Self::iso_paint_blend_line(
                    buffer,
                    surface_buffer,
                    stamp_depth,
                    owner_geo_id,
                    base_x + 1,
                    screen[1],
                    top_x + 1,
                    top_y,
                    blade,
                );
            }
        }
    }
}
