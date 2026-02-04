use crate::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, PartialOrd, Clone, Copy, Debug, Default)]
pub struct TheDim {
    /// The x offset in screen coordinates
    pub x: i32,
    /// The y offset in screen coordinates
    pub y: i32,

    pub width: i32,
    pub height: i32,

    /// The x offset relative to the canvas buffer
    pub buffer_x: i32,
    /// The y offset relative to the canvas buffer
    pub buffer_y: i32,
}

impl TheDim {
    pub fn zero() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            buffer_x: 0,
            buffer_y: 0,
        }
    }

    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            buffer_x: 0,
            buffer_y: 0,
        }
    }

    pub fn rect(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            buffer_x: x,
            buffer_y: y,
        }
    }

    pub fn sized(width: i32, height: i32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
            buffer_x: 0,
            buffer_y: 0,
        }
    }

    /// Sets the offset relative to the canvas buffer.
    pub fn set_buffer_offset(&mut self, buffer_x: i32, buffer_y: i32) {
        self.buffer_x = buffer_x;
        self.buffer_y = buffer_y;
    }

    pub fn screen_coord(&self) -> Vec2<i32> {
        Vec2::new(self.x, self.y)
    }

    /// Check for size validity
    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }

    /// Checks if the given coordinate is inside the dimension.
    pub fn contains(&self, coord: Vec2<i32>) -> bool {
        self.x <= coord.x
            && self.x + self.width > coord.x
            && self.y <= coord.y
            && self.y + self.height > coord.y
    }

    /// Returns the given screen coordinate as a local coordinate.
    pub fn to_local(&self, coord: Vec2<i32>) -> Vec2<i32> {
        coord - self.screen_coord()
    }

    /// Returns the dimension as an usize tuple relative to the buffer origin (used by the drawing routines)
    pub fn to_buffer_utuple(&self) -> (usize, usize, usize, usize) {
        (
            self.buffer_x as usize,
            self.buffer_y as usize,
            self.width as usize,
            self.height as usize,
        )
    }

    /// Returns the dimension as an usize tuple relative to the buffer origin (used by the drawing routines)
    pub fn to_buffer_shrunk_utuple(
        &self,
        shrinker: &TheDimShrinker,
    ) -> (usize, usize, usize, usize) {
        (
            (self.buffer_x + shrinker.left) as usize,
            (self.buffer_y + shrinker.top) as usize,
            (self.width - shrinker.left - shrinker.right) as usize,
            (self.height - shrinker.top - shrinker.bottom) as usize,
        )
    }

    /// Returns the center of the buffer.
    pub fn center(&self) -> Vec2<f32> {
        Vec2::new(
            self.x as f32 + self.width as f32 / 2.0,
            self.y as f32 + self.height as f32 / 2.0,
        )
    }

    /// Return the radius of the dimension.
    pub fn radius(&self) -> f32 {
        self.width.min(self.height) as f32 / 2.0
    }
}

/// Shrink content of TheDim, used in styles to provide a way to implement custom sized borders for widgets.
#[derive(PartialEq, PartialOrd, Clone, Copy)]
pub struct TheDimShrinker {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl TheDimShrinker {
    pub fn zero() -> Self {
        Self {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        }
    }

    /// Shrink by the given value
    pub fn shrink(&mut self, value: i32) {
        self.left += value;
        self.top += value;
        self.right += value;
        self.bottom += value;
    }

    /// Shrink by the given amounts.
    pub fn shrink_by(&mut self, left: i32, top: i32, right: i32, bottom: i32) {
        self.left += left;
        self.top += top;
        self.right += right;
        self.bottom += bottom;
    }

    /// Resets the shrinker, i.e. sets all fields to 0.
    pub fn reset(&mut self) {
        self.left = 0;
        self.top = 0;
        self.right = 0;
        self.bottom = 0;
    }
}
