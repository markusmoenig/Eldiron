
pub struct Draw2D {
}

impl Draw2D {

    /// Draws the given rectangle
    pub fn draw_rect(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, color: &[u8; 4]) {
        for y in rect.1..rect.1+rect.3 {
            for x in rect.0..rect.0+rect.2 {
                let i = x * 4 + y * stride * 4;
                frame[i..i + 4].copy_from_slice(color);
            }
        }
    }

    /// Draws a circle
    pub fn draw_circle(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, color: &[u8; 4], radius: f64) {
        let center = (rect.0 as f64 + rect.2 as f64 / 2.0, rect.1 as f64 + rect.3 as f64 / 2.0);
        for y in rect.1..rect.1+rect.3 {
            for x in rect.0..rect.0+rect.2 {
                let i = x * 4 + y * stride * 4;

                let mut d = (x as f64 - center.0).powf(2.0) + (y as f64 - center.1).powf(2.0);
                d = d.sqrt() - radius;

                if d < 0.0 {
                    let t = self.fill_mask(d);
                    //let t = self.smoothstep(0.0, -2.0, r);

                    let background = &[frame[i], frame[i+1], frame[i+2], 255];
                    let mixed_color = self.mix_color(&background, &color, t);

                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    /// Draws a circle with a border of a given size
    pub fn draw_circle_with_border(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, color: &[u8; 4], radius: f64, border_color: &[u8; 4], border_size: f64) {
        let center = (rect.0 as f64 + rect.2 as f64 / 2.0, rect.1 as f64 + rect.3 as f64 / 2.0);
        for y in rect.1..rect.1+rect.3 {
            for x in rect.0..rect.0+rect.2 {
                let i = x * 4 + y * stride * 4;

                let mut d = (x as f64 - center.0).powf(2.0) + (y as f64 - center.1).powf(2.0);
                d = d.sqrt() - radius;

                if d < 1.0 {
                    let t = self.fill_mask(d);

                    let background = &[frame[i], frame[i+1], frame[i+2], 255];
                    let mut mixed_color = self.mix_color(&background, &color, t);

                    let b = self.border_mask(d, border_size);
                    mixed_color = self.mix_color(&mixed_color, &border_color, b);

                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    /// Draws a rounded rect
    pub fn draw_rounded_rect(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, size: &(f64, f64), color: &[u8; 4], rounding: &(f64, f64, f64, f64)) {
        let center = (rect.0 as f64 + rect.2 as f64 / 2.0, rect.1 as f64 + rect.3 as f64 / 2.0);
        for y in rect.1..rect.1+rect.3 {
            for x in rect.0..rect.0+rect.2 {
                let i = x * 4 + y * stride * 4;

                let p = (x as f64 - center.0, y as f64 - center.1);               
                let mut r : (f64, f64);

                if p.0 > 0.0 {
                    r = (rounding.0, rounding.1);
                } else {
                    r = (rounding.2, rounding.3);
                }

                if p.1 <= 0.0 {
                    r.0 = r.1;
                }

                let q : (f64, f64) = (p.0.abs() - size.0 + r.0, p.1.abs() - size.1 + r.0);
                let d = f64::min(f64::max(q.0, q.1), 0.0) + self.length((f64::max(q.0, 0.0), f64::max(q.1, 0.0))) - r.0;
                
                if d < 0.0 {
                    let t = self.fill_mask(d);

                    let background = &[frame[i], frame[i+1], frame[i+2], 255];
                    let mixed_color = self.mix_color(&background, &color, t);

                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    /// Draws a rounded rect with a border
    pub fn draw_rounded_rect_with_border(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, size: &(f64, f64), color: &[u8; 4], rounding: &(f64, f64, f64, f64), border_color: &[u8; 4], border_size: f64) {
        let center = (rect.0 as f64 + rect.2 as f64 / 2.0, rect.1 as f64 + rect.3 as f64 / 2.0);
        for y in rect.1..rect.1+rect.3 {
            for x in rect.0..rect.0+rect.2 {
                let i = x * 4 + y * stride * 4;

                let p = (x as f64 - center.0, y as f64 - center.1);               
                let mut r : (f64, f64);

                if p.0 > 0.0 {
                    r = (rounding.0, rounding.1);
                } else {
                    r = (rounding.2, rounding.3);
                }

                if p.1 <= 0.0 {
                    r.0 = r.1;
                }

                let q : (f64, f64) = (p.0.abs() - size.0 + r.0, p.1.abs() - size.1 + r.0);
                let d = f64::min(f64::max(q.0, q.1), 0.0) + self.length((f64::max(q.0, 0.0), f64::max(q.1, 0.0))) - r.0;
                
                if d < 1.0 {
                    let t = self.fill_mask(d);

                    let background = &[frame[i], frame[i+1], frame[i+2], 255];
                    let mut mixed_color = self.mix_color(&background, &color, t);

                    let b = self.border_mask(d, border_size);
                    mixed_color = self.mix_color(&mixed_color, &border_color, b);

                    frame[i..i + 4].copy_from_slice(&mixed_color);
                }
            }
        }
    }

    /// Draws the given rectangle
    pub fn draw_square_pattern(&self, frame: &mut [u8], rect: &(usize, usize, usize, usize), stride: usize, color: &[u8; 4], line_color: &[u8; 4], pattern_size: usize) {
        for y in rect.1..rect.1+rect.3 {
            for x in rect.0..rect.0+rect.2 {
                let i = x * 4 + y * stride * 4;

                if x % pattern_size == 0 || y % pattern_size == 0 {
                    frame[i..i + 4].copy_from_slice(line_color);
                } else {
                    frame[i..i + 4].copy_from_slice(color);
                }
            }
        }
    }

    fn fill_mask(&self, dist : f64) -> f64 {
        (-dist).clamp(0.0, 1.0)
    }

    fn border_mask(&self, dist : f64, width: f64) -> f64 {
       (dist + width).clamp(0.0, 1.0) - dist.clamp(0.0, 1.0)
    }

    /// Smoothstep for f32
    pub fn smoothstep(&self, e0: f64, e1: f64, x: f64) -> f64 {
        let t = ((x - e0) / (e1 - e0)). clamp(0.0, 1.0);
        return t * t * (3.0 - 2.0 * t); 
    }

    /// Mixes two colors based on v
    pub fn mix_color(&self, a: &[u8;4], b: &[u8;4], v: f64) -> [u8; 4] {
        [   (((1.0 - v) * (a[0] as f64 / 255.0) + b[0] as f64 / 255.0 * v) * 255.0) as u8, 
            (((1.0 - v) * (a[1] as f64 / 255.0) + b[1] as f64 / 255.0 * v) * 255.0) as u8,
            (((1.0 - v) * (a[2] as f64 / 255.0) + b[2] as f64 / 255.0 * v) * 255.0) as u8,
        255]
    }

    // Length of a 2d vector
    pub fn length(&self, v: (f64, f64)) -> f64 {
        ((v.0).powf(2.0) + (v.1).powf(2.0)).sqrt()
    }
}