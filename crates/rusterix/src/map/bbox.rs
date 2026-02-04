use theframework::prelude::*;
use vek::Vec2;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct BBox {
    pub min: Vec2<f32>, // Bottom-left corner
    pub max: Vec2<f32>, // Top-right corner
}

impl BBox {
    /// Constructs a BBox from min and max coordinates
    pub fn new(min: Vec2<f32>, max: Vec2<f32>) -> Self {
        Self { min, max }
    }

    /// Constructs a BBox from position and size
    pub fn from_pos_size(pos: Vec2<f32>, size: Vec2<f32>) -> Self {
        Self {
            min: pos,
            max: pos + size,
        }
    }

    /// Returns the width and height of the bounding box
    pub fn size(&self) -> Vec2<f32> {
        self.max - self.min
    }

    /// Returns the center of the bounding box
    pub fn center(&self) -> Vec2<f32> {
        (self.min + self.max) * 0.5
    }

    /// Checks if a point is inside the bounding box
    pub fn contains(&self, point: Vec2<f32>) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// Returns true if this bounding box intersects another bounding box
    pub fn intersects(&self, other: &BBox) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    /// Expands (or shrinks) the bounding box by a given amount
    pub fn expand(&mut self, amount: Vec2<f32>) {
        self.min -= amount * 0.5;
        self.max += amount * 0.5;
    }

    /// Returns a new bounding box expanded (or shrunk) by a given amount
    pub fn expanded(&self, amount: Vec2<f32>) -> BBox {
        BBox {
            min: self.min - amount * 0.5,
            max: self.max + amount * 0.5,
        }
    }

    /// Expands the bounding box to include another bounding box
    pub fn expand_bbox(&mut self, other: BBox) {
        self.min.x = self.min.x.min(other.min.x);
        self.min.y = self.min.y.min(other.min.y);
        self.max.x = self.max.x.max(other.max.x);
        self.max.y = self.max.y.max(other.max.y);
    }

    /// Returns true if the line from `a` to `b` intersects this bounding box
    pub fn line_intersects(&self, a: Vec2<f32>, b: Vec2<f32>) -> bool {
        let min = self.min;
        let max = self.max;

        // Liang–Barsky algorithm
        let dx = b.x - a.x;
        let dy = b.y - a.y;

        let mut tmin = 0.0;
        let mut tmax = 1.0;

        let check = |p: f32, q: f32, tmin: &mut f32, tmax: &mut f32| -> bool {
            if p == 0.0 {
                return q >= 0.0; // Parallel and outside
            }
            let t = q / p;
            if p < 0.0 {
                if t > *tmax {
                    return false;
                }
                if t > *tmin {
                    *tmin = t;
                }
            } else {
                if t < *tmin {
                    return false;
                }
                if t < *tmax {
                    *tmax = t;
                }
            }
            true
        };

        check(-dx, a.x - min.x, &mut tmin, &mut tmax)
            && check(dx, max.x - a.x, &mut tmin, &mut tmax)
            && check(-dy, a.y - min.y, &mut tmin, &mut tmax)
            && check(dy, max.y - a.y, &mut tmin, &mut tmax)
    }
}
