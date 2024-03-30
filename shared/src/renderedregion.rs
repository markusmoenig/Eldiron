use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct RenderedRegionData {
    line_width: usize,
    height: usize,
    pixel_lines: Vec<Vec<Vec3f>>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct RenderedRegion {
    data: Option<RenderedRegionData>,
}

impl Default for RenderedRegion {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderedRegion {
    pub fn new() -> Self {
        Self { data: None }
    }

    pub fn setup(&mut self, region: &Region) {
        let grid_size = region.grid_size as usize;
        let width = region.width as usize * grid_size;
        let height = region.height as usize * grid_size;

        let mut pixel_lines: Vec<Vec<Vec3f>> = vec![];

        for _ in 0..height {
            let pixel_line = vec![Vec3f::zero(); width];

            // for x in 0..width {
            //     pixel_line[x] = Vec3f::zero();
            // }

            pixel_lines.push(pixel_line);
        }

        self.data = Some(RenderedRegionData {
            line_width: width,
            height,
            pixel_lines,
        });
    }
}
