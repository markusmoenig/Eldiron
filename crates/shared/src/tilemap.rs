use rusterix::TileRole;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Tilemap {
    pub id: Uuid,

    pub name: String,
    pub buffer: TheRGBABuffer,

    pub grid_size: i32,
    /// Source image frame size; legacy sheets use square grid_size cells.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_size: Option<[i32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isometric: Option<crate::iso_bitmap::Settings>,
    /// Original separate-image canvases inside the browsing atlas.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_frames: Vec<SourceFrame>,
    pub scroll_offset: Vec2<i32>,
    pub zoom: f32,

    pub tiles: Vec<Tile>,
}

impl Default for Tilemap {
    fn default() -> Self {
        Self::new()
    }
}

impl Tilemap {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),

            name: String::new(),
            buffer: TheRGBABuffer::empty(),

            grid_size: 24,
            frame_size: None,
            isometric: None,
            source_frames: Vec::new(),
            scroll_offset: Vec2::zero(),
            zoom: 1.0,

            tiles: vec![],
        }
    }

    pub fn grid_dimensions(&self) -> Vec2<i32> {
        let [w, h] = self.frame_size.unwrap_or([self.grid_size; 2]);
        Vec2::new(w.max(1), h.max(1))
    }
    /// Restore original PNG padding and names when extracting separate images.
    pub fn selected_images(
        &self,
        sequence: &TheRGBARegionSequence,
        include_empty: bool,
    ) -> Vec<(String, rusterix::Texture)> {
        sequence
            .regions
            .iter()
            .filter_map(|selected| {
                let original = self
                    .source_frames
                    .iter()
                    .find(|frame| frame.region.x == selected.x && frame.region.y == selected.y);
                let region = original.map(|f| &f.region).unwrap_or(selected);
                if region.x + region.width > self.buffer.dim().width as usize
                    || region.y + region.height > self.buffer.dim().height as usize
                {
                    return None;
                }
                let image = self.buffer.extract_region(region);
                if !include_empty && !image.pixels().chunks_exact(4).any(|p| p[3] > 0) {
                    return None;
                }
                Some((
                    original.map(|f| f.name.clone()).unwrap_or_else(|| {
                        format!("{} ({}, {})", self.name, selected.x, selected.y)
                    }),
                    rusterix::Texture::new(image.pixels().to_vec(), region.width, region.height),
                ))
            })
            .collect()
    }
    /// Set the buffer
    pub fn set_buffer(&mut self, buffer: TheRGBABuffer) {
        self.buffer = buffer;
    }
}

fn default_tile_scale() -> f32 {
    1.0
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Tile {
    pub id: Uuid,
    pub name: String,
    pub role: TileRole,
    #[serde(default = "default_tile_scale")]
    pub scale: f32,

    pub sequence: TheRGBARegionSequence,
    pub blocking: bool,
    #[serde(default)]
    pub render_mode: u8,
}

impl Default for Tile {
    fn default() -> Self {
        Self::new()
    }
}

impl Tile {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            role: TileRole::Nature,
            scale: 1.0,

            sequence: TheRGBARegionSequence::new(),
            blocking: false,
            render_mode: 0,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct SourceFrame {
    pub name: String,
    pub region: TheRGBARegion,
}
