#![cfg(feature = "ui")]

use theframework::prelude::*;

fn native_buffer(width: i32, height: i32, scale: f32) -> TheRGBABuffer {
    let mut buffer = TheRGBABuffer::new(TheDim::sized(width, height));
    buffer.set_render_scale(scale);
    buffer
}

#[test]
fn density_changes_storage_without_changing_layout_or_assets() {
    let mut buffer = native_buffer(19, 11, 1.25);
    assert_eq!(*buffer.dim(), TheDim::sized(19, 11));
    assert_eq!((buffer.pixel_width(), buffer.pixel_height()), (24, 14));
    assert_eq!(buffer.len(), 24 * 14 * 4);
    buffer.fill([42, 12, 8, 255]);
    assert!(!buffer.set_render_scale(1.25));
    assert_eq!(buffer.get_pixel(18, 10), Some([42, 12, 8, 255]));
    assert!(buffer.set_render_scale(2.0));
    assert_eq!(buffer.len(), 38 * 22 * 4);
    assert!(buffer.pixels().iter().all(|v| *v == 0));
    assert_eq!(TheRGBABuffer::new(TheDim::sized(19, 11)).len(), 19 * 11 * 4);
}

#[test]
fn text_is_rasterized_at_display_resolution() {
    let draw = TheDraw2D::new();
    let mut native = native_buffer(120, 30, 2.0);
    let mut expected = vec![0; 240 * 60 * 4];
    let mut logical = TheRGBABuffer::new(TheDim::sized(120, 30));
    draw.text_rect_blend(
        native.draw_target(),
        &(3, 2, 110, 24),
        120,
        "Display scaling",
        TheFontSettings {
            size: 13.0,
            ..Default::default()
        },
        &[255; 4],
        TheHorizontalAlign::Left,
        TheVerticalAlign::Top,
    );
    draw.text_rect_blend(
        &mut expected,
        &(6, 4, 220, 48),
        240,
        "Display scaling",
        TheFontSettings {
            size: 26.0,
            ..Default::default()
        },
        &[255; 4],
        TheHorizontalAlign::Left,
        TheVerticalAlign::Top,
    );
    draw.text_rect_blend(
        logical.draw_target(),
        &(3, 2, 110, 24),
        120,
        "Display scaling",
        TheFontSettings {
            size: 13.0,
            ..Default::default()
        },
        &[255; 4],
        TheHorizontalAlign::Left,
        TheVerticalAlign::Top,
    );
    assert!(native.pixels().iter().any(|v| *v != 0));
    assert_eq!(native.pixels(), expected);
    let mut enlarged = native_buffer(120, 30, 2.0);
    enlarged.copy_into(0, 0, &logical);
    assert_ne!(
        native.pixels(),
        enlarged.pixels(),
        "must not enlarge 1x glyph bitmaps"
    );
}

#[test]
fn fractional_shapes_and_clips_use_the_same_transform() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut buffer = native_buffer(17, 13, scale);
        let w = buffer.pixel_width();
        let h = buffer.pixel_height();
        let mut painter = ThePainter::new();
        let clip;
        {
            let mut surface = TheSurfaceMut::new(buffer.draw_target(), 17, 13).unwrap();
            clip = surface.set_clip(ThePixelRect::new(3, 2, 9, 8));
            painter.fill_round_rect(
                &mut surface,
                ThePixelRect::new(-2, -3, 25, 22),
                3.5,
                &ThePaint::solid([240, 40, 20, 255]),
            );
        }
        assert!(buffer.pixels().chunks_exact(4).any(|p| p[3] != 0));
        for y in 0..h {
            for x in 0..w {
                if !clip.contains(x as i32, y as i32) {
                    assert_eq!(buffer.pixels()[(y * w + x) * 4 + 3], 0);
                }
            }
        }
    }
}

#[test]
fn scrolling_and_negative_composition_preserve_native_rows() {
    let mut source = native_buffer(5, 6, 2.0);
    for (i, pixel) in source.pixels_mut().chunks_exact_mut(4).enumerate() {
        pixel.copy_from_slice(&[i as u8, (i / 10) as u8, 0, 255]);
    }
    let mut dest = native_buffer(5, 3, 2.0);
    dest.copy_vertical_range_into(0, 0, &source, 2..5);
    assert_eq!(dest.pixels(), &source.pixels()[4 * 10 * 4..10 * 10 * 4]);
    let mut clipped = native_buffer(3, 3, 2.0);
    clipped.copy_into(-1, -2, &source);
    for y in 0..6 {
        for x in 0..6 {
            assert_eq!(
                &clipped.pixels()[(y * 6 + x) * 4..(y * 6 + x + 1) * 4],
                &source.pixels()[((y + 4) * 10 + x + 2) * 4..((y + 4) * 10 + x + 3) * 4]
            );
        }
    }
}

#[test]
fn adjacent_fractional_rectangles_leave_no_unpainted_seam() {
    let draw = TheDraw2D::new();
    for scale in [1.25, 1.5, 1.75, 2.0] {
        let mut buffer = native_buffer(15, 7, scale);
        for x in 0..3 {
            draw.rect(
                buffer.draw_target(),
                &(x * 5, 0, 5, 7),
                15,
                &[30, 60, 90, 255],
            );
        }
        assert!(
            buffer
                .pixels()
                .chunks_exact(4)
                .all(|p| p == [30, 60, 90, 255])
        );
    }
}

#[test]
fn physical_presentation_crops_or_pads_edges_without_resampling() {
    for (logical_width, physical_width, scale) in [(9, 17, 2.0), (3, 6, 1.75)] {
        let mut ctx = TheContext::new(logical_width, 5, scale);
        ctx.ui_render_scale = scale;
        ctx.framebuffer_width = physical_width;
        ctx.framebuffer_height = (5.0 * scale).round() as usize;
        let mut ui = TheUI::new();
        ui.init(&mut ctx);
        let mut frame = vec![0; physical_width * ctx.framebuffer_height * 4];
        ui.draw(&mut frame, &mut ctx);
        let sw = ui.canvas.buffer.pixel_width();
        for (i, pixel) in ui
            .canvas
            .buffer
            .pixels_mut()
            .chunks_exact_mut(4)
            .enumerate()
        {
            pixel.copy_from_slice(&[(i % sw) as u8, (i / sw) as u8, 0, 255]);
        }
        ui.draw(&mut frame, &mut ctx);
        for y in 0..ctx.framebuffer_height {
            for x in 0..physical_width {
                assert_eq!(
                    &frame[(y * physical_width + x) * 4..(y * physical_width + x + 1) * 4],
                    &[x.min(sw - 1) as u8, y as u8, 0, 255]
                );
            }
        }
    }
}

#[test]
fn serialized_density_keeps_dimensions_and_pixel_storage_consistent() {
    let buffer = native_buffer(13, 7, 1.5);
    let json = serde_json::to_string(&buffer).unwrap();
    let restored: TheRGBABuffer = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, buffer);
    let legacy = TheRGBABuffer::new(TheDim::sized(13, 7));
    let json = serde_json::to_string(&legacy).unwrap();
    assert!(!json.contains("render_scale"));
    assert_eq!(
        serde_json::from_str::<TheRGBABuffer>(&json).unwrap(),
        legacy
    );
}
