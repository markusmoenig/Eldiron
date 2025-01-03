use crate::prelude::*;
// use rayon::prelude::*;

pub struct DiscBrush {
    id: TheId,
}

impl Brush for DiscBrush {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Disc Brush"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        str!("Disc")
    }

    fn distance(&self, p: Vec2<f32>, pos: Vec2<f32>, settings: &BrushSettings) -> f32 {
        (p - pos).magnitude() - settings.size / 2.0
    }

    fn preview(&self, _buffer: &mut TheRGBABuffer) {
        // fn mix_color(a: &[u8; 4], b: &[u8; 4], v: f32) -> [u8; 4] {
        //     [
        //         (((1.0 - v) * (a[0] as f32 / 255.0) + b[0] as f32 / 255.0 * v) * 255.0) as u8,
        //         (((1.0 - v) * (a[1] as f32 / 255.0) + b[1] as f32 / 255.0 * v) * 255.0) as u8,
        //         (((1.0 - v) * (a[2] as f32 / 255.0) + b[2] as f32 / 255.0 * v) * 255.0) as u8,
        //         (((1.0 - v) * (a[3] as f32 / 255.0) + b[3] as f32 / 255.0 * v) * 255.0) as u8,
        //     ]
        // }

        // let width = buffer.dim().width as usize;
        // let height = buffer.dim().height;

        // buffer
        //     .pixels_mut()
        //     .par_rchunks_exact_mut(width * 4)
        //     .enumerate()
        //     .for_each(|(j, line)| {
        //         for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
        //             let i = j * width + i;

        //             let x = (i % width) as f32;
        //             let y = (i / width) as f32;

        //             // let p = Vec2::new(x / width as f32, y / height as f32);
        //             // let d = length(p - Vec2::new(0.5, 0.5)) - 0.4;
        //             let t = 0.5; //smoothstep(-0.03, 0.0, d);

        //             let color = [209, 209, 209, 255];
        //             pixel.copy_from_slice(&mix_color(&color, &[81, 81, 81, 255], t));
        //         }
        //     });
    }
}
