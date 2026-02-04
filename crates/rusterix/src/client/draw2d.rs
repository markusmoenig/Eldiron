use fontdue::Font;
use fontdue::layout::{
    CoordinateSystem, HorizontalAlign, Layout, LayoutSettings, TextStyle, VerticalAlign,
};
use vek::*;

#[derive(PartialEq, Clone, Eq)]
pub enum TheHorizontalAlign {
    Left,
    Center,
    Right,
}

#[derive(PartialEq)]
pub enum TheVerticalAlign {
    Top,
    Center,
    Bottom,
}

#[derive(PartialEq, Debug)]
pub struct Draw2D {
    pub mask: Option<Vec<f32>>,
    pub mask_size: (usize, usize),
}

impl Default for Draw2D {
    fn default() -> Self {
        Self::new()
    }
}

impl Draw2D {
    pub fn new() -> Self {
        Self {
            mask: None,
            mask_size: (0, 0),
        }
    }

    /// Draws the mask
    pub fn blend_mask(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        mask_frame: &[u8],
        mask_size: &(usize, usize),
        color: &[u8; 4],
    ) {
        for y in 0..mask_size.1 {
            for x in 0..mask_size.0 {
                let i = (x + rect.0) * 4 + (y + rect.1) * stride * 4;
                let m = mask_frame[x + y * mask_size.0];
                let c: [u8; 4] = [color[0], color[1], color[2], m];

                let background = &[frame[i], frame[i + 1], frame[i + 2], frame[i + 3]];
                frame[i..i + 4].copy_from_slice(&self.mix_color(background, &c, m as f32 / 255.0));
            }
        }
    }

    /// Draws the given rectangle
    pub fn rect(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
    ) {
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;
                frame[i..i + 4].copy_from_slice(color);
            }
        }
    }

    /// Draws the given rectangle and checks the frame boundaries.
    pub fn rect_safe(
        &self,
        frame: &mut [u8],
        rect: &(isize, isize, usize, usize),
        stride: usize,
        color: &[u8; 4],
        safe_rect: &(usize, usize, usize, usize),
    ) {
        let dest_stride_isize: isize = stride as isize;
        for y in rect.1..rect.1 + rect.3 as isize {
            if y >= safe_rect.1 as isize && y < (safe_rect.1 + safe_rect.3) as isize {
                for x in rect.0..rect.0 + rect.2 as isize {
                    if x >= safe_rect.0 as isize && x < (safe_rect.0 + safe_rect.2) as isize {
                        let i = (x * 4 + y * dest_stride_isize * 4) as usize;
                        frame[i..i + 4].copy_from_slice(color);
                    }
                }
            }
        }
    }

    /// Blend the given rectangle
    pub fn blend_rect(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
    ) {
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;

                let background = &[frame[i], frame[i + 1], frame[i + 2], frame[i + 3]];
                frame[i..i + 4].copy_from_slice(&self.mix_color(
                    background,
                    color,
                    color[3] as f32 / 255.0,
                ));
            }
        }
    }

    /// Blend the given rectangle
    pub fn blend_rect_safe(
        &self,
        frame: &mut [u8],
        rect: &(isize, isize, isize, isize),
        stride: usize,
        color: &[u8; 4],
        safe_rect: &(isize, isize, isize, isize),
    ) {
        for y in rect.1..rect.1 + rect.3 {
            if y >= safe_rect.1 && y < (safe_rect.1 + safe_rect.3) {
                for x in rect.0..rect.0 + rect.2 {
                    if x >= safe_rect.0 && x < (safe_rect.0 + safe_rect.2) {
                        let i = x as usize * 4 + y as usize * stride * 4;

                        let background = &[frame[i], frame[i + 1], frame[i + 2], frame[i + 3]];
                        frame[i..i + 4].copy_from_slice(&self.mix_color(
                            background,
                            color,
                            color[3] as f32 / 255.0,
                        ));
                    }
                }
            }
        }
    }

    /// Draws the outline of a given rectangle
    pub fn rect_outline(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
    ) {
        let y = rect.1;
        for x in rect.0..rect.0 + rect.2 {
            let mut i = x * 4 + y * stride * 4;
            frame[i..i + 4].copy_from_slice(color);

            i = x * 4 + (y + rect.3 - 1) * stride * 4;
            frame[i..i + 4].copy_from_slice(color);
        }

        let x = rect.0;
        for y in rect.1..rect.1 + rect.3 {
            let mut i = x * 4 + y * stride * 4;
            frame[i..i + 4].copy_from_slice(color);

            i = (x + rect.2 - 1) * 4 + y * stride * 4;
            frame[i..i + 4].copy_from_slice(color);
        }
    }

    /// Draws the outline of a given rectangle with a given thickness
    pub fn rect_outline_thickness(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
        thickness: usize,
    ) {
        let (x0, y0, w, h) = *rect;
        let x1 = x0 + w;
        let y1 = y0 + h;

        // Clamp thickness to not exceed half width/height
        let thickness = thickness.min(w / 2).min(h / 2).max(1);

        // Top and Bottom
        for y in y0..(y0 + thickness) {
            for x in x0..x1 {
                let i = x * 4 + y * stride * 4;
                if i + 4 <= frame.len() {
                    frame[i..i + 4].copy_from_slice(color);
                }
            }
        }
        for y in (y1 - thickness)..y1 {
            for x in x0..x1 {
                let i = x * 4 + y * stride * 4;
                if i + 4 <= frame.len() {
                    frame[i..i + 4].copy_from_slice(color);
                }
            }
        }

        // Left and Right
        for y in y0..y1 {
            for x in x0..(x0 + thickness) {
                let i = x * 4 + y * stride * 4;
                if i + 4 <= frame.len() {
                    frame[i..i + 4].copy_from_slice(color);
                }
            }
            for x in (x1 - thickness)..x1 {
                let i = x * 4 + y * stride * 4;
                if i + 4 <= frame.len() {
                    frame[i..i + 4].copy_from_slice(color);
                }
            }
        }
    }

    /// Draws the outline of a given rectangle
    pub fn rect_outline_border(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
        border: usize,
    ) {
        let y = rect.1;
        for x in rect.0 + border..rect.0 + rect.2 - border {
            let mut i = x * 4 + y * stride * 4;
            frame[i..i + 4].copy_from_slice(color);

            i = x * 4 + (y + rect.3 - 1) * stride * 4;
            frame[i..i + 4].copy_from_slice(color);
        }

        let x = rect.0;
        for y in rect.1 + border..rect.1 + rect.3 - border {
            let mut i = x * 4 + y * stride * 4;
            frame[i..i + 4].copy_from_slice(color);

            i = (x + rect.2 - 1) * 4 + y * stride * 4;
            frame[i..i + 4].copy_from_slice(color);
        }
    }

    /// Draws the outline of a given rectangle
    pub fn rect_outline_border_safe(
        &self,
        frame: &mut [u8],
        rect: &(isize, isize, usize, usize),
        stride: usize,
        color: &[u8; 4],
        border: isize,
        safe_rect: &(usize, usize, usize, usize),
    ) {
        let dest_stride_isize: isize = stride as isize;
        let y = rect.1;
        if y >= safe_rect.1 as isize && y < (safe_rect.1 + safe_rect.3) as isize {
            for x in rect.0 + border..rect.0 + rect.2 as isize - border {
                if x >= safe_rect.0 as isize && x < (safe_rect.0 + safe_rect.2) as isize {
                    let mut i = (x * 4 + y * dest_stride_isize * 4) as usize;
                    frame[i..i + 4].copy_from_slice(color);

                    if (y + rect.3 as isize - 1) >= safe_rect.1 as isize
                        && (y + rect.3 as isize - 1) < (safe_rect.1 + safe_rect.3) as isize
                    {
                        i = (x * 4 + (y + rect.3 as isize - 1) * dest_stride_isize * 4) as usize;
                        frame[i..i + 4].copy_from_slice(color);
                    }
                }
            }
        }

        let x = rect.0;
        if x >= safe_rect.0 as isize && x < (safe_rect.0 + safe_rect.2) as isize {
            for y in rect.1 + border..rect.1 + rect.3 as isize - border {
                if y >= safe_rect.1 as isize && y < (safe_rect.1 + safe_rect.3) as isize {
                    let mut i = (x * 4 + y * dest_stride_isize * 4) as usize;
                    frame[i..i + 4].copy_from_slice(color);

                    if (x + rect.2 as isize - 1) >= safe_rect.0 as isize
                        && (x + rect.2 as isize - 1) < (safe_rect.0 + safe_rect.2) as isize
                    {
                        i = ((x + rect.2 as isize - 1) * 4 + y * dest_stride_isize * 4) as usize;
                        frame[i..i + 4].copy_from_slice(color);
                    }
                }
            }
        }
    }

    /// Draws a circle
    pub fn circle(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
        radius: f32,
    ) {
        let center = (
            rect.0 as f32 + rect.2 as f32 / 2.0,
            rect.1 as f32 + rect.3 as f32 / 2.0,
        );
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;

                let mut d = (x as f32 - center.0).powf(2.0) + (y as f32 - center.1).powf(2.0);
                d = d.sqrt() - radius;

                if d <= 0.0 {
                    // let t = self.fill_mask(d);
                    let t = self._smoothstep(0.0, -2.0, d);

                    let background = &[frame[i], frame[i + 1], frame[i + 2], 255];
                    let mixed_color = self.mix_color(background, color, t);

                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Draws a circle with a border of a given size
    pub fn circle_with_border(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
        radius: f32,
        border_color: &[u8; 4],
        border_size: f32,
    ) {
        let center = (
            rect.0 as f32 + rect.2 as f32 / 2.0,
            rect.1 as f32 + rect.3 as f32 / 2.0,
        );
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;

                let mut d = (x as f32 - center.0).powf(2.0) + (y as f32 - center.1).powf(2.0);
                d = d.sqrt() - radius;

                if d < 1.0 {
                    let t = self.fill_mask(d);

                    let background = &[frame[i], frame[i + 1], frame[i + 2], frame[i + 3]];
                    let mut mixed_color = self.mix_color(background, color, t);

                    let b = self.border_mask(d, border_size);
                    mixed_color = self.mix_color(&mixed_color, border_color, b);

                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    /// Draws a rounded rect
    pub fn rounded_rect(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
        rounding: &(f32, f32, f32, f32),
    ) {
        let center = (
            (rect.0 as f32 + rect.2 as f32 / 2.0).round(),
            (rect.1 as f32 + rect.3 as f32 / 2.0).round(),
        );
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;

                let p = (x as f32 - center.0, y as f32 - center.1);
                let mut r: (f32, f32);

                if p.0 > 0.0 {
                    r = (rounding.0, rounding.1);
                } else {
                    r = (rounding.2, rounding.3);
                }

                if p.1 <= 0.0 {
                    r.0 = r.1;
                }

                let q: (f32, f32) = (
                    p.0.abs() - rect.2 as f32 / 2.0 + r.0,
                    p.1.abs() - rect.3 as f32 / 2.0 + r.0,
                );
                let d = f32::min(f32::max(q.0, q.1), 0.0)
                    + self.length((f32::max(q.0, 0.0), f32::max(q.1, 0.0)))
                    - r.0;

                if d < 0.0 {
                    let t = self.fill_mask(d);

                    let background = &[frame[i], frame[i + 1], frame[i + 2], frame[i + 3]];
                    let mut mixed_color =
                        self.mix_color(background, color, t * (color[3] as f32 / 255.0));
                    mixed_color[3] = (mixed_color[3] as f32 * (color[3] as f32 / 255.0)) as u8;
                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Draws a rounded rect with a border
    pub fn rounded_rect_with_border(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
        rounding: &(f32, f32, f32, f32),
        border_color: &[u8; 4],
        border_size: f32,
    ) {
        let hb = border_size / 2.0;
        let center = (
            (rect.0 as f32 + rect.2 as f32 / 2.0 - hb).round(),
            (rect.1 as f32 + rect.3 as f32 / 2.0 - hb).round(),
        );
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;

                let p = (x as f32 - center.0, y as f32 - center.1);
                let mut r: (f32, f32);

                if p.0 > 0.0 {
                    r = (rounding.0, rounding.1);
                } else {
                    r = (rounding.2, rounding.3);
                }

                if p.1 <= 0.0 {
                    r.0 = r.1;
                }

                let q: (f32, f32) = (
                    p.0.abs() - rect.2 as f32 / 2.0 + hb + r.0,
                    p.1.abs() - rect.3 as f32 / 2.0 + hb + r.0,
                );
                let d = f32::min(f32::max(q.0, q.1), 0.0)
                    + self.length((f32::max(q.0, 0.0), f32::max(q.1, 0.0)))
                    - r.0;

                if d < 1.0 {
                    let t = self.fill_mask(d);

                    let background = &[frame[i], frame[i + 1], frame[i + 2], frame[i + 3]];
                    let mut mixed_color =
                        self.mix_color(background, color, t * (color[3] as f32 / 255.0));

                    let b = self.border_mask(d, border_size);
                    mixed_color = self.mix_color(&mixed_color, border_color, b);

                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Draws a hexagon with a border
    pub fn hexagon_with_border(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
        border_color: &[u8; 4],
        border_size: f32,
    ) {
        let hb = border_size / 2.0;
        let center = (
            (rect.0 as f32 + rect.2 as f32 / 2.0 - hb).round(),
            (rect.1 as f32 + rect.3 as f32 / 2.0 - hb).round(),
        );
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;

                let mut p: Vec2<f32> =
                    Vec2::new((x as f32 - center.0).abs(), (y as f32 - center.1).abs());
                let r = rect.2 as f32 / 2.33;
                let k: Vec3<f32> = Vec3::new(-0.866_025_4, 0.5, 0.577_350_26);
                p -= 2.0 * k.xy() * k.xy().dot(p).min(0.0);
                p = p.clamped(Vec2::broadcast(-k.z * r), Vec2::broadcast(k.z * r));
                let d = p.magnitude() * p.y.signum();

                if d < 1.0 {
                    let t = self.fill_mask(d);
                    // let t = self._smoothstep(0.0, -2.0, d);

                    let background: &[u8; 4] =
                        &[frame[i], frame[i + 1], frame[i + 2], frame[i + 3]];
                    let mut mixed_color =
                        self.mix_color(background, color, t * (color[3] as f32 / 255.0));

                    let b = self.border_mask(d, border_size);
                    mixed_color = self.mix_color(&mixed_color, border_color, b);

                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Draws a rhombus rect with a border
    pub fn rhombus_with_border(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
        border_color: &[u8; 4],
        border_size: f32,
    ) {
        let hb = border_size / 2.0;
        let center = (
            (rect.0 as f32 + rect.2 as f32 / 2.0 - hb).round(),
            (rect.1 as f32 + rect.3 as f32 / 2.0 - hb).round(),
        );

        // fn ndot(a: Vec2<f32>, b: Vec2<f32>) -> f32 {
        //     a.x * b.x - a.y * b.y
        // }

        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;

                /*
                float ndot(vec2 a, vec2 b ) { return a.x*b.x - a.y*b.y; }
                float sdRhombus( in vec2 p, in vec2 b )
                {
                    p = abs(p);
                    float h = clamp( ndot(b-2.0*p,b)/dot(b,b), -1.0, 1.0 );
                    float d = length( p-0.5*b*vec2(1.0-h,1.0+h) );
                    return d * sign( p.x*b.y + p.y*b.x - b.x*b.y );
                }*/

                let p = Vec2::new((x as f32 - center.0).abs(), (y as f32 - center.1).abs());
                let b = Vec2::new(rect.2 as f32 / 2.0, rect.3 as f32 / 2.0);

                let h = (Vec2::dot(b - 2.0 * p, b) / Vec2::dot(b, b)).clamp(-1.0, 1.0);
                let mut d = (p - 0.5 * b * Vec2::new(1.0 - h, 1.0 + h)).magnitude();
                d *= (p.x * b.y + p.y * b.x - b.x * b.y).signum();

                if d < 1.0 {
                    let t = self.fill_mask(d);

                    let background: &[u8; 4] =
                        &[frame[i], frame[i + 1], frame[i + 2], frame[i + 3]];
                    let mut mixed_color =
                        self.mix_color(background, color, t * (color[3] as f32 / 255.0));

                    let b = self.border_mask(d, border_size);
                    mixed_color = self.mix_color(&mixed_color, border_color, b);

                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    /// Draws a square pattern
    pub fn square_pattern(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        color: &[u8; 4],
        line_color: &[u8; 4],
        pattern_size: usize,
    ) {
        for y in rect.1..rect.1 + rect.3 {
            for x in rect.0..rect.0 + rect.2 {
                let i = x * 4 + y * stride * 4;

                if x % pattern_size == 0 || y % pattern_size == 0 {
                    frame[i..i + 4].copy_from_slice(line_color);
                } else {
                    frame[i..i + 4].copy_from_slice(color);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Draws a text aligned inside a rect
    pub fn text_rect(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        font: &Font,
        size: f32,
        text: &str,
        color: &[u8; 4],
        background: &[u8; 4],
        halign: TheHorizontalAlign,
        valign: TheVerticalAlign,
    ) {
        let mut text_to_use = text.trim_end().to_string().clone();
        text_to_use = text_to_use.replace('\n', "");
        if text_to_use.trim_end().is_empty() {
            return;
        }

        let mut text_size = self.get_text_size(font, size, text_to_use.as_str());

        let mut add_trail = false;
        // Text is too long ??
        while text_size.0 >= rect.2 {
            text_to_use.pop();
            text_size = self.get_text_size(font, size, (text_to_use.clone() + "...").as_str());
            add_trail = true;
        }

        if add_trail {
            text_to_use += "...";
        }

        let layout = self.get_text_layout(
            font,
            size,
            &text_to_use,
            LayoutSettings {
                max_width: Some(rect.2 as f32),
                max_height: Some(rect.3 as f32),
                horizontal_align: if halign == TheHorizontalAlign::Left {
                    HorizontalAlign::Left
                } else if halign == TheHorizontalAlign::Right {
                    HorizontalAlign::Right
                } else {
                    HorizontalAlign::Center
                },
                vertical_align: if valign == TheVerticalAlign::Top {
                    VerticalAlign::Top
                } else if valign == TheVerticalAlign::Bottom {
                    VerticalAlign::Bottom
                } else {
                    VerticalAlign::Middle
                },
                ..LayoutSettings::default()
            },
        );
        for glyph in layout.glyphs() {
            let (metrics, alphamap) = font.rasterize(glyph.parent, glyph.key.px);
            //println!("Metrics: {:?}", glyph);

            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let i = (x + rect.0 + glyph.x as usize) * 4
                        + (y + rect.1 + glyph.y as usize) * stride * 4;
                    let m = alphamap[x + y * metrics.width];

                    frame[i..i + 4].copy_from_slice(&self.mix_color(
                        background,
                        color,
                        m as f32 / 255.0,
                    ));
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Draws a text aligned inside a rect
    pub fn text_rect_clip(
        &self,
        frame: &mut [u8],
        top_left: &Vec2<i32>,
        clip_rect: &(usize, usize, usize, usize),
        stride: usize,
        font: &Font,
        size: f32,
        text: &str,
        color: &[u8; 4],
        background: &[u8; 4],
        halign: TheHorizontalAlign,
        valign: TheVerticalAlign,
    ) {
        let mut text_to_use = text.trim_end().to_string().clone();
        text_to_use = text_to_use.replace('\n', "");
        if text_to_use.trim_end().is_empty() {
            return;
        }

        let layout = self.get_text_layout(
            font,
            size,
            &text_to_use,
            LayoutSettings {
                horizontal_align: if halign == TheHorizontalAlign::Left {
                    HorizontalAlign::Left
                } else if halign == TheHorizontalAlign::Right {
                    HorizontalAlign::Right
                } else {
                    HorizontalAlign::Center
                },
                vertical_align: if valign == TheVerticalAlign::Top {
                    VerticalAlign::Top
                } else if valign == TheVerticalAlign::Bottom {
                    VerticalAlign::Bottom
                } else {
                    VerticalAlign::Middle
                },
                wrap_hard_breaks: false,
                ..LayoutSettings::default()
            },
        );
        for glyph in layout.glyphs() {
            let (metrics, alphamap) = font.rasterize(glyph.parent, glyph.key.px);

            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let coord_x = top_left.x + (x + glyph.x.ceil() as usize) as i32;
                    let coord_y = top_left.y + (y + glyph.y.ceil() as usize) as i32;
                    if coord_x < 0 || coord_y < 0 {
                        continue;
                    }

                    let coord_x = coord_x as usize;
                    let coord_y = coord_y as usize;
                    if coord_x < clip_rect.0
                        || coord_x > clip_rect.0 + clip_rect.2
                        || coord_y < clip_rect.1
                        || coord_y > clip_rect.1 + clip_rect.3
                    {
                        continue;
                    }

                    let i = coord_x * 4 + coord_y * stride * 4;
                    let m = alphamap[x + y * metrics.width];

                    frame[i..i + 4].copy_from_slice(&self.mix_color(
                        background,
                        color,
                        m as f32 / 255.0,
                    ));
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Blends a text aligned inside a rect and blends it with the existing background
    pub fn text_rect_blend_safe(
        &self,
        frame: &mut [u8],
        rect: &(isize, isize, isize, isize),
        stride: usize,
        font: &Font,
        size: f32,
        text: &str,
        color: &[u8; 4],
        halign: TheHorizontalAlign,
        valign: TheVerticalAlign,
        safe_rect: &(isize, isize, isize, isize),
    ) {
        let mut text_to_use = text.trim_end().to_string().clone();
        if text_to_use.trim_end().is_empty() {
            return;
        }

        let mut text_size = self.get_text_size(font, size, text_to_use.as_str());

        let mut add_trail = false;
        // Text is too long ??
        while text_size.0 >= rect.2 as usize {
            text_to_use.pop();
            text_size = self.get_text_size(font, size, (text_to_use.clone() + "...").as_str());
            add_trail = true;
        }

        if add_trail {
            text_to_use += "...";
        }

        let layout = self.get_text_layout(
            font,
            size,
            &text_to_use,
            LayoutSettings {
                max_width: Some(rect.2 as f32),
                max_height: Some(rect.3 as f32),
                horizontal_align: if halign == TheHorizontalAlign::Left {
                    HorizontalAlign::Left
                } else if halign == TheHorizontalAlign::Right {
                    HorizontalAlign::Right
                } else {
                    HorizontalAlign::Center
                },
                vertical_align: if valign == TheVerticalAlign::Top {
                    VerticalAlign::Top
                } else if valign == TheVerticalAlign::Bottom {
                    VerticalAlign::Bottom
                } else {
                    VerticalAlign::Middle
                },
                ..LayoutSettings::default()
            },
        );
        for glyph in layout.glyphs() {
            let (metrics, alphamap) = font.rasterize(glyph.parent, glyph.key.px);
            //println!("Metrics: {:?}", glyph);

            for y in 0..metrics.height {
                if rect.1 + y as isize + glyph.y as isize >= safe_rect.1
                    && rect.1 + y as isize + (glyph.y as isize) < (safe_rect.1 + safe_rect.3)
                {
                    for x in 0..metrics.width {
                        if rect.0 + x as isize + glyph.x as isize >= safe_rect.0
                            && rect.0 + x as isize + (glyph.x as isize)
                                < (safe_rect.0 + safe_rect.2)
                        {
                            // if (y + rect.1) >= rect.1
                            //     && (y + rect.1) < (rect.1 + rect.3)
                            //     && (x + rect.0) >= rect.0
                            //     && (x + rect.0) < (rect.0 + rect.2)
                            // {

                            let i = (x + rect.0 as usize + glyph.x as usize) * 4
                                + (y + rect.1 as usize + glyph.y as usize) * stride * 4;
                            let m = alphamap[x + y * metrics.width];

                            let background = &[frame[i], frame[i + 1], frame[i + 2], frame[i + 3]];
                            frame[i..i + 4].copy_from_slice(&self.mix_color(
                                background,
                                color,
                                m as f32 / 255.0,
                            ));
                        }
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Blends a text aligned inside a rect and blends it with the existing background
    pub fn text_rect_blend_clip(
        &self,
        frame: &mut [u8],
        top_left: &Vec2<i32>,
        clip_rect: &(usize, usize, usize, usize),
        stride: usize,
        font: &Font,
        size: f32,
        text: &str,
        color: &[u8; 4],
        halign: TheHorizontalAlign,
        valign: TheVerticalAlign,
    ) {
        let text_to_use = text.trim_end().to_string().clone();
        if text_to_use.trim_end().is_empty() {
            return;
        }

        let layout = self.get_text_layout(
            font,
            size,
            &text_to_use,
            LayoutSettings {
                horizontal_align: if halign == TheHorizontalAlign::Left {
                    HorizontalAlign::Left
                } else if halign == TheHorizontalAlign::Right {
                    HorizontalAlign::Right
                } else {
                    HorizontalAlign::Center
                },
                vertical_align: if valign == TheVerticalAlign::Top {
                    VerticalAlign::Top
                } else if valign == TheVerticalAlign::Bottom {
                    VerticalAlign::Bottom
                } else {
                    VerticalAlign::Middle
                },
                ..LayoutSettings::default()
            },
        );
        for glyph in layout.glyphs() {
            let (metrics, alphamap) = font.rasterize(glyph.parent, glyph.key.px);

            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let coord_x = top_left.x + (x + glyph.x.ceil() as usize) as i32;
                    let coord_y = top_left.y + (y + glyph.y.ceil() as usize) as i32;
                    if coord_x < 0 || coord_y < 0 {
                        continue;
                    }

                    let coord_x = coord_x as usize;
                    let coord_y = coord_y as usize;
                    if coord_x < clip_rect.0
                        || coord_x > clip_rect.0 + clip_rect.2
                        || coord_y < clip_rect.1
                        || coord_y > clip_rect.1 + clip_rect.3
                    {
                        continue;
                    }

                    let i = coord_x * 4 + coord_y * stride * 4;
                    let m = alphamap[x + y * metrics.width];

                    let background = &[frame[i], frame[i + 1], frame[i + 2], frame[i + 3]];
                    frame[i..i + 4].copy_from_slice(&self.mix_color(
                        background,
                        color,
                        m as f32 / 255.0,
                    ));
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Draws the given text
    pub fn text(
        &self,
        frame: &mut [u8],
        pos: &(usize, usize),
        stride: usize,
        font: &Font,
        size: f32,
        text: &str,
        color: &[u8; 4],
        background: &[u8; 4],
    ) {
        if text.is_empty() {
            return;
        }

        let layout = self.get_text_layout(font, size, text, LayoutSettings::default());
        for glyph in layout.glyphs() {
            let (metrics, alphamap) = font.rasterize(glyph.parent, glyph.key.px);
            //println!("Metrics: {:?}", glyph);

            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let i = (x + pos.0 + glyph.x as usize) * 4
                        + (y + pos.1 + glyph.y as usize) * stride * 4;
                    let m = alphamap[x + y * metrics.width];

                    frame[i..i + 4].copy_from_slice(&self.mix_color(
                        background,
                        color,
                        m as f32 / 255.0,
                    ));
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Draws the given text
    pub fn text_blend(
        &self,
        frame: &mut [u8],
        pos: &(usize, usize),
        stride: usize,
        font: &Font,
        size: f32,
        text: &str,
        color: &[u8; 4],
    ) {
        if text.is_empty() {
            return;
        }

        let layout = self.get_text_layout(font, size, text, LayoutSettings::default());
        for glyph in layout.glyphs() {
            let (metrics, alphamap) = font.rasterize(glyph.parent, glyph.key.px);
            //println!("Metrics: {:?}", glyph);

            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let i = (x + pos.0 + glyph.x as usize) * 4
                        + (y + pos.1 + glyph.y as usize) * stride * 4;
                    let m = alphamap[x + y * metrics.width];

                    let background = &[frame[i], frame[i + 1], frame[i + 2], frame[i + 3]];
                    frame[i..i + 4].copy_from_slice(&self.mix_color(
                        background,
                        color,
                        m as f32 / 255.0,
                    ));
                }
            }
        }
    }

    /// Returns the layout of the given text
    pub fn get_text_layout(
        &self,
        font: &Font,
        size: f32,
        text: &str,
        settings: LayoutSettings,
    ) -> Layout {
        let fonts = &[font];

        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&settings);
        layout.append(fonts, &TextStyle::new(text, size, 0));

        layout
    }

    /// Returns the size of the given text
    pub fn get_text_size(&self, font: &Font, size: f32, text: &str) -> (usize, usize) {
        if text.is_empty() {
            return (0, 0);
        }

        let layout = self.get_text_layout(font, size, text, LayoutSettings::default());
        let glyphs = layout.glyphs();

        let x = glyphs[glyphs.len() - 1].x.ceil() as usize + glyphs[glyphs.len() - 1].width + 1;
        (x, layout.height() as usize)
    }

    /// Copies rect from the source frame into the dest frame
    pub fn copy_slice(
        &self,
        dest: &mut [u8],
        source: &[u8],
        rect: &(usize, usize, usize, usize),
        dest_stride: usize,
    ) {
        for y in 0..rect.3 {
            let d = rect.0 * 4 + (y + rect.1) * dest_stride * 4;
            let s = y * rect.2 * 4;
            dest[d..d + rect.2 * 4].copy_from_slice(&source[s..s + rect.2 * 4]);
        }
    }

    /// Blends rect from the source frame into the dest frame
    pub fn blend_slice(
        &self,
        dest: &mut [u8],
        source: &[u8],
        rect: &(usize, usize, usize, usize),
        dest_stride: usize,
    ) {
        for y in 0..rect.3 {
            let d = rect.0 * 4 + (y + rect.1) * dest_stride * 4;
            let s = y * rect.2 * 4;

            for x in 0..rect.2 {
                let dd = d + x * 4;
                let ss = s + x * 4;

                let background = &[dest[dd], dest[dd + 1], dest[dd + 2], dest[dd + 3]];
                let color = &[source[ss], source[ss + 1], source[ss + 2], source[ss + 3]];
                dest[dd..dd + 4].copy_from_slice(&self.mix_color(
                    background,
                    color,
                    (color[3] as f32) / 255.0,
                ));
            }
        }
    }

    /// Blends rect from the source frame into the dest frame
    pub fn blend_slice_alpha(
        &self,
        dest: &mut [u8],
        source: &[u8],
        rect: &(usize, usize, usize, usize),
        dest_stride: usize,
        alpha: f32,
    ) {
        for y in 0..rect.3 {
            let d = rect.0 * 4 + (y + rect.1) * dest_stride * 4;
            let s = y * rect.2 * 4;

            for x in 0..rect.2 {
                let dd = d + x * 4;
                let ss = s + x * 4;

                let background = &[dest[dd], dest[dd + 1], dest[dd + 2], dest[dd + 3]];
                let color = &[source[ss], source[ss + 1], source[ss + 2], source[ss + 3]];
                dest[dd..dd + 4].copy_from_slice(&self.mix_color(
                    background,
                    color,
                    (color[3] as f32 * alpha) / 255.0,
                ));
            }
        }
    }

    /// Blends rect from the source frame into the dest frame
    pub fn blend_slice_f32(
        &self,
        dest: &mut [u8],
        source: &[f32],
        rect: &(usize, usize, usize, usize),
        dest_stride: usize,
    ) {
        for y in 0..rect.3 {
            let d = rect.0 * 4 + (y + rect.1) * dest_stride * 4;
            let s = y * rect.2 * 4;

            for x in 0..rect.2 {
                let dd = d + x * 4;
                let ss = s + x * 4;

                let background = &[dest[dd], dest[dd + 1], dest[dd + 2], dest[dd + 3]];
                let color = &[
                    (source[ss] * 255.0) as u8,
                    (source[ss + 1] * 255.0) as u8,
                    (source[ss + 2] * 255.0) as u8,
                    (source[ss + 3] * 255.0) as u8,
                ];
                dest[dd..dd + 4].copy_from_slice(&self.mix_color(
                    background,
                    color,
                    (color[3] as f32) / 255.0,
                ));
            }
        }
    }

    /// Blends rect from the source frame into the dest frame with a vertical source offset (used by scrolling containers)
    pub fn blend_slice_offset(
        &self,
        dest: &mut [u8],
        source: &[u8],
        rect: &(usize, usize, usize, usize),
        offset: usize,
        dest_stride: usize,
    ) {
        for y in 0..rect.3 {
            let d = rect.0 * 4 + (y + rect.1) * dest_stride * 4;
            let s = (y + offset) * rect.2 * 4;

            for x in 0..rect.2 {
                let dd = d + x * 4;
                let ss = s + x * 4;

                let background = &[dest[dd], dest[dd + 1], dest[dd + 2], dest[dd + 3]];
                let color = &[source[ss], source[ss + 1], source[ss + 2], source[ss + 3]];
                dest[dd..dd + 4].copy_from_slice(&self.mix_color(
                    background,
                    color,
                    (color[3] as f32) / 255.0,
                ));
            }
        }
    }

    /// Blends rect from the source frame into the dest frame and honors the safe rect
    pub fn blend_slice_safe(
        &self,
        dest: &mut [u8],
        source: &[u8],
        rect: &(isize, isize, usize, usize),
        dest_stride: usize,
        safe_rect: &(usize, usize, usize, usize),
    ) {
        let dest_stride_isize = dest_stride as isize;
        for y in 0..rect.3 as isize {
            let d = rect.0 * 4 + (y + rect.1) * dest_stride_isize * 4;
            let s = y * (rect.2 as isize) * 4;

            // TODO: Make this faster

            if (y + rect.1) >= safe_rect.1 as isize
                && (y + rect.1) < (safe_rect.1 + safe_rect.3) as isize
            {
                for x in 0..rect.2 as isize {
                    if (x + rect.0) >= safe_rect.0 as isize
                        && (x + rect.0) < (safe_rect.0 + safe_rect.2) as isize
                    {
                        let dd = (d + x * 4) as usize;
                        let ss = (s + x * 4) as usize;

                        let background = &[dest[dd], dest[dd + 1], dest[dd + 2], dest[dd + 3]];
                        let color = &[source[ss], source[ss + 1], source[ss + 2], source[ss + 3]];
                        dest[dd..dd + 4].copy_from_slice(&self.mix_color(
                            background,
                            color,
                            (color[3] as f32) / 255.0,
                        ));
                    }
                }
            }
        }
    }

    /// Scale a chunk to the destination size
    pub fn scale_chunk(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        source_frame: &[u8],
        source_size: &(usize, usize),
        blend_factor: f32,
    ) {
        let x_ratio = source_size.0 as f32 / rect.2 as f32;
        let y_ratio = source_size.1 as f32 / rect.3 as f32;

        for sy in 0..rect.3 {
            let y = (sy as f32 * y_ratio) as usize;

            for sx in 0..rect.2 {
                let x = (sx as f32 * x_ratio) as usize;

                let d = (rect.0 + sx) * 4 + (sy + rect.1) * stride * 4;
                let s = x * 4 + y * source_size.0 * 4;

                frame[d..d + 4].copy_from_slice(&[
                    source_frame[s],
                    source_frame[s + 1],
                    source_frame[s + 2],
                    ((source_frame[s + 3] as f32) * blend_factor) as u8,
                ]);
            }
        }
    }

    /// Scale a chunk to the destination size
    pub fn blend_scale_chunk(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        source_frame: &[u8],
        source_size: &(usize, usize),
    ) {
        let x_ratio = source_size.0 as f32 / rect.2 as f32;
        let y_ratio = source_size.1 as f32 / rect.3 as f32;

        for sy in 0..rect.3 {
            let y = (sy as f32 * y_ratio) as usize;

            for sx in 0..rect.2 {
                let x = (sx as f32 * x_ratio) as usize;

                let d = (rect.0 + sx) * 4 + (sy + rect.1) * stride * 4;
                let s = x * 4 + y * source_size.0 * 4;

                let color = &[
                    source_frame[s],
                    source_frame[s + 1],
                    source_frame[s + 2],
                    source_frame[s + 3],
                ];
                let background = &[frame[d], frame[d + 1], frame[d + 2], frame[d + 3]];
                frame[d..d + 4].copy_from_slice(&self.mix_color(
                    background,
                    color,
                    (color[3] as f32) / 255.0,
                ));
            }
        }
    }

    /// Scale a chunk to the destination size with a global alpha
    pub fn blend_scale_chunk_alpha(
        &self,
        frame: &mut [u8],
        rect: &(usize, usize, usize, usize),
        stride: usize,
        source_frame: &[u8],
        source_size: &(usize, usize),
        alpha: f32,
    ) {
        let x_ratio = source_size.0 as f32 / rect.2 as f32;
        let y_ratio = source_size.1 as f32 / rect.3 as f32;

        for sy in 0..rect.3 {
            let y = (sy as f32 * y_ratio) as usize;

            for sx in 0..rect.2 {
                let x = (sx as f32 * x_ratio) as usize;

                let d = (rect.0 + sx) * 4 + (sy + rect.1) * stride * 4;
                let s = x * 4 + y * source_size.0 * 4;

                let color = &[
                    source_frame[s],
                    source_frame[s + 1],
                    source_frame[s + 2],
                    source_frame[s + 3],
                ];
                let background = &[frame[d], frame[d + 1], frame[d + 2], frame[d + 3]];
                frame[d..d + 4].copy_from_slice(&self.mix_color(
                    background,
                    color,
                    (color[3] as f32 * alpha) / 255.0,
                ));
            }
        }
    }

    /// Scale a chunk to the destination size with linear interpolation and blend onto destination
    pub fn blend_scale_chunk_linear(
        &self,
        dest: &mut [u8],
        dest_rect: &(usize, usize, usize, usize),
        dest_stride: usize,
        source: &[u8],
        source_size: &(usize, usize),
    ) {
        let x_ratio = (source_size.0 - 1) as f32 / dest_rect.2 as f32;
        let y_ratio = (source_size.1 - 1) as f32 / dest_rect.3 as f32;

        for dy in 0..dest_rect.3 {
            let sy = (dy as f32 * y_ratio).round() as usize;
            let sy_frac = dy as f32 * y_ratio - sy as f32;

            for dx in 0..dest_rect.2 {
                let sx = (dx as f32 * x_ratio).round() as usize;
                let sx_frac = dx as f32 * x_ratio - sx as f32;

                let d = (dest_rect.0 + dx) * 4 + (dest_rect.1 + dy) * dest_stride * 4;

                // Interpolate between four neighboring pixels
                let mut interpolated_color = [0; 4];
                for c in 0..4 {
                    let tl = source[(sy * source_size.0 + sx) * 4 + c] as f32;
                    let tr = source[(sy * source_size.0 + sx + 1) * 4 + c] as f32;
                    let bl = source[((sy + 1) * source_size.0 + sx) * 4 + c] as f32;
                    let br = source[((sy + 1) * source_size.0 + sx + 1) * 4 + c] as f32;

                    let top = tl * (1.0 - sx_frac) + tr * sx_frac;
                    let bottom = bl * (1.0 - sx_frac) + br * sx_frac;

                    interpolated_color[c] = (top * (1.0 - sy_frac) + bottom * sy_frac) as u8;
                }

                // Blend the interpolated color onto the destination
                let background = &[dest[d], dest[d + 1], dest[d + 2], dest[d + 3]];
                dest[d..d + 4].copy_from_slice(&self.mix_color(
                    background,
                    &interpolated_color,
                    interpolated_color[3] as f32 / 255.0,
                ));
            }
        }
    }

    /// The fill mask for an SDF distance
    fn fill_mask(&self, dist: f32) -> f32 {
        (-dist).clamp(0.0, 1.0)
    }

    /// The border mask for an SDF distance
    fn border_mask(&self, dist: f32, width: f32) -> f32 {
        (dist + width).clamp(0.0, 1.0) - dist.clamp(0.0, 1.0)
    }

    /// Smoothstep for f32
    pub fn _smoothstep(&self, e0: f32, e1: f32, x: f32) -> f32 {
        let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// Mixes two colors based on v
    pub fn mix_color(&self, a: &[u8; 4], b: &[u8; 4], v: f32) -> [u8; 4] {
        [
            (((1.0 - v) * (a[0] as f32 / 255.0) + b[0] as f32 / 255.0 * v) * 255.0) as u8,
            (((1.0 - v) * (a[1] as f32 / 255.0) + b[1] as f32 / 255.0 * v) * 255.0) as u8,
            (((1.0 - v) * (a[2] as f32 / 255.0) + b[2] as f32 / 255.0 * v) * 255.0) as u8,
            (((1.0 - v) * (a[3] as f32 / 255.0) + b[3] as f32 / 255.0 * v) * 255.0) as u8,
        ]
    }

    // Length of a 2d vector
    pub fn length(&self, v: (f32, f32)) -> f32 {
        ((v.0).powf(2.0) + (v.1).powf(2.0)).sqrt()
    }
}
