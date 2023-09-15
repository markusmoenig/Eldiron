use crate::prelude::*;

#[derive(PartialEq, Clone, Debug)]
pub enum ScriptShapes {
    Rect(ScriptRect),
    RoundedRect(ScriptRect, ScriptRect),
}

// --- Shape

#[derive(PartialEq, Clone, Debug)]
pub struct ScriptShape {
    pub shapes: Vec<ScriptShapes>,

    pub background: [f32; 4],
    pub fill_color: [f32; 4],
    pub border_color: [f32; 4],
    pub border_size: f32,
}

impl ScriptShape {
    pub fn shape() -> Self {
        Self {
            shapes: vec![],

            background: [0.0, 0.0, 0.0, 1.0],
            fill_color: [0.5, 0.5, 0.5, 1.0],
            border_color: [1.0, 1.0, 1.0, 1.0],
            border_size: 1.0,
        }
    }

    pub fn add_rect(&mut self, rect: ScriptRect) {
        self.shapes.push(ScriptShapes::Rect(rect));
    }

    pub fn add_rounded_rect(&mut self, rect: ScriptRect, round: ScriptRect) {
        self.shapes.push(ScriptShapes::RoundedRect(rect, round));
    }

    pub fn draw(&self, frame: &mut [u8], size: (usize, usize)) {
        let (width, height) = size;

        let background = self.background;
        let border_color = self.border_color;
        let fill_color = self.fill_color;
        let border_size = self.border_size;

        for y in 0..height {
            for x in 0..width {
                let i = x * 4 + y * width * 4;

                // Length of a 2d vector
                fn length(v: (f32, f32)) -> f32 {
                    ((v.0).powf(2.0) + (v.1).powf(2.0)).sqrt()
                }

                fn fill_mask(dist: f32) -> f32 {
                    (-dist).clamp(0.0, 1.0)
                }

                fn border_mask(dist: f32, width: f32) -> f32 {
                    (dist + width).clamp(0.0, 1.0) - dist.clamp(0.0, 1.0)
                }

                fn mix_color(a: &[f32; 4], b: &[f32; 4], v: f32) -> [f32; 4] {
                    [
                        (1.0 - v) * a[0] + b[0] * v,
                        (1.0 - v) * a[1] + b[1] * v,
                        (1.0 - v) * a[2] + b[2] * v,
                        1.0,
                    ]
                }

                let mut dist = 10000.0;
                for shape in &self.shapes {
                    let d = match shape {
                        ScriptShapes::Rect(rect) => {
                            let rounding = (0.0, 0.0, 0.0, 0.0);
                            let mut r: (f32, f32);
                            let rect = (
                                rect.rect.0 as f32,
                                rect.rect.1 as f32,
                                rect.rect.2 as f32,
                                rect.rect.3 as f32,
                            );

                            let p = (
                                x as f32 - (rect.0 + rect.2 / 2.0),
                                y as f32 - (rect.1 + rect.3 / 2.0),
                            );

                            if p.0 > 0.0 {
                                r = (rounding.0, rounding.1);
                            } else {
                                r = (rounding.2, rounding.3);
                            }

                            if p.1 <= 0.0 {
                                r.0 = r.1;
                            }

                            let q: (f32, f32) = (
                                p.0.abs() - rect.2 / 2.0 + border_size + r.0,
                                p.1.abs() - rect.3 / 2.0 + border_size + r.0,
                            );
                            f32::min(f32::max(q.0, q.1), 0.0)
                                + length((f32::max(q.0, 0.0), f32::max(q.1, 0.0)))
                                - r.0
                        }
                        ScriptShapes::RoundedRect(rect, round) => {
                            let rounding = (
                                round.rect.3 as f32,
                                round.rect.1 as f32,
                                round.rect.2 as f32,
                                round.rect.0 as f32,
                            );
                            let mut r: (f32, f32);
                            let rect = (
                                rect.rect.0 as f32,
                                rect.rect.1 as f32,
                                rect.rect.2 as f32,
                                rect.rect.3 as f32,
                            );

                            let p = (
                                x as f32 - (rect.0 + rect.2 / 2.0),
                                y as f32 - (rect.1 + rect.3 / 2.0),
                            );

                            if p.0 > 0.0 {
                                r = (rounding.0, rounding.1);
                            } else {
                                r = (rounding.2, rounding.3);
                            }

                            if p.1 <= 0.0 {
                                r.0 = r.1;
                            }

                            let q: (f32, f32) = (
                                p.0.abs() - rect.2 / 2.0 + border_size + r.0,
                                p.1.abs() - rect.3 / 2.0 + border_size + r.0,
                            );
                            f32::min(f32::max(q.0, q.1), 0.0)
                                + length((f32::max(q.0, 0.0), f32::max(q.1, 0.0)))
                                - r.0
                        }
                    };
                    dist = f32::min(dist, d);
                }

                let t = fill_mask(dist);
                let mut mixed_color = mix_color(&background, &fill_color, t * fill_color[3]);

                let b = border_mask(dist, border_size);
                mixed_color = mix_color(&mixed_color, &border_color, b);

                let color = [
                    (mixed_color[0] * 255.0) as u8,
                    (mixed_color[1] * 255.0) as u8,
                    (mixed_color[2] * 255.0) as u8,
                    (mixed_color[3] * 255.0) as u8,
                ];

                frame[i..i + 4].copy_from_slice(&color);
            }
        }
    }

    pub fn set_color(&mut self, color: ScriptRGB) {
        self.fill_color = color.to_normalized();
    }

    pub fn set_border_color(&mut self, color: ScriptRGB) {
        self.border_color = color.to_normalized();
    }

    pub fn set_border_size(&mut self, size: f32) {
        self.border_size = size;
    }
}
