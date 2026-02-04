use vek::Vec2;

/// Rectangle
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Default for Rect {
    fn default() -> Self {
        Self::empty()
    }
}

impl Rect {
    pub fn empty() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }

    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, point: Vec2<f32>) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    pub fn size(&self) -> Vec2<f32> {
        Vec2::new(self.width, self.height)
    }

    pub fn with_border(&self, border: f32) -> Self {
        let double = border * 2.0;
        if double <= self.width && double <= self.height {
            Self {
                x: self.x + border,
                y: self.y + border,
                width: self.width - double,
                height: self.height - double,
            }
        } else {
            *self
        }
    }
}
