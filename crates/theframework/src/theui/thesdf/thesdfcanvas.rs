use crate::prelude::*;
use rayon::prelude::*;

pub struct TheSDFCanvas {
    pub background: crate::thecolor::TheColor,
    pub highlight: crate::thecolor::TheColor,
    pub hover_highlight: crate::thecolor::TheColor,
    pub error_highlight: crate::thecolor::TheColor,

    pub selected: Option<usize>,
    pub hover: Option<usize>,
    pub error: Option<usize>,

    pub sdfs: Vec<TheSDF>,
    pub patterns: Vec<ThePattern>,
}

impl Default for TheSDFCanvas {
    fn default() -> Self {
        Self::new()
    }
}

impl TheSDFCanvas {
    pub fn new() -> Self {
        Self {
            sdfs: vec![],
            patterns: vec![],

            selected: None,
            hover: None,
            error: None,

            background: TheColor::black(),
            highlight: TheColor::white(),
            hover_highlight: TheColor::white(),
            error_highlight: crate::thecolor::TheColor::from_u8_array([209, 42, 42, 255]),
        }
    }

    /// Adds an SDF to the canvas.
    pub fn add(&mut self, sdf: TheSDF, pattern: ThePattern) {
        self.sdfs.push(sdf);
        self.patterns.push(pattern);
    }

    /// Renders the sdfs into the given buffer.
    pub fn render(&self, buffer: &mut TheRGBABuffer) {
        let width = buffer.dim().width as usize;
        let height = buffer.dim().height;

        let pixels = buffer.pixels_mut();

        pixels
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let x = (i % width) as i32;
                    let y = height - (i / width) as i32 - 1;

                    let mut color = self.background.clone();
                    let mut distance = f32::MAX;

                    let p = Vec2::new(x as f32, y as f32);
                    let mut i: Option<usize> = None;

                    for (index, sdf) in self.sdfs.iter().enumerate() {
                        let d = sdf.distance(p);

                        if d < distance {
                            // let c1 = self.patterns[index].get_color(
                            //     p,
                            //     &d,
                            //     &color,
                            //     self.highlight(index),
                            // );

                            // if i.is_some() {
                            //     //let cx = (x as f32 /*- width as f32 / 2.0*/) / width as f32;
                            //     //color = color.mix(&c1, cx);
                            //     let k = 50.0;
                            //     let h = clamp( 0.5 + 0.5*(distance-d)/k, 0.0, 1.0 );
                            // } else {
                            //     color = c1;
                            // }

                            i = Some(index);
                            distance = d;
                        }
                    }

                    if let Some(index) = i {
                        color = self.patterns[index].get_color(
                            p,
                            &distance,
                            &color,
                            self.highlight(index),
                        );
                    }

                    // if let Some(index) = self.selected {
                    //     let d = self.sdfs[index].distance(p);
                    //     color = self.patterns[index].get_color(
                    //         p,
                    //         &d,
                    //         &color,
                    //         self.highlight(index),
                    //     );
                    // }

                    pixel.copy_from_slice(&color.to_u8_array());
                }
            });
    }

    /// Returns the index of the sdf at the given position.
    pub fn index_at(&self, p: Vec2<f32>) -> Option<usize> {
        for (index, sdf) in self.sdfs.iter().enumerate() {
            let d = sdf.distance(p);
            if d < 0.0 {
                return Some(index);
            }
        }
        None
    }

    /// Returns the selected color if the given sdf index is highlighted.
    #[inline(always)]
    fn highlight(&self, index: usize) -> Option<&TheColor> {
        if self.error == Some(index) {
            Some(&self.error_highlight)
        } else if self.selected == Some(index) {
            Some(&self.highlight)
        } else if self.hover == Some(index) {
            Some(&self.hover_highlight)
        } else {
            None
        }
    }

    /// Clear the canvas.
    pub fn clear(&mut self) {
        self.sdfs.clear();
        self.patterns.clear();
        self.selected = None;
        self.hover = None;
    }

    /// Returns true if the canvas is empty.
    pub fn is_empty(&self) -> bool {
        self.sdfs.is_empty()
    }
}
