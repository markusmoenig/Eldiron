use crate::prelude::*;

pub struct TheSizeLimiter {
    min_size: Vec2<i32>,
    max_size: Vec2<i32>,
}

impl Default for TheSizeLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl TheSizeLimiter {
    pub fn new() -> Self {
        Self {
            min_size: Vec2::new(0, 0),
            max_size: Vec2::new(i32::MAX, i32::MAX),
        }
    }

    /// Sets the minimum dimensions of the limiter.
    pub fn set_min_size(&mut self, size: Vec2<i32>) {
        self.min_size = size;
    }

    /// Sets the maximum dimensions of the limiter.
    pub fn set_max_size(&mut self, size: Vec2<i32>) {
        self.max_size = size;
    }

    /// Sets the minimum width of the limiter.
    pub fn set_min_width(&mut self, width: i32) {
        self.min_size.x = width;
    }

    /// Sets the maximum height of the limiter.
    pub fn set_min_height(&mut self, height: i32) {
        self.min_size.y = height;
    }

    /// Sets the maximum width of the limiter.
    pub fn set_max_width(&mut self, width: i32) {
        self.max_size.x = width;
    }

    /// Returns the max width.
    pub fn get_max_width(&self) -> i32 {
        self.max_size.x
    }

    /// Returns the max height.
    pub fn get_max_height(&self) -> i32 {
        self.max_size.y
    }

    /// Sets the maximum height of the limiter.
    pub fn set_max_height(&mut self, height: i32) {
        self.max_size.y = height;
    }

    /// Return the width
    pub fn get_width(&self, max_width: i32) -> i32 {
        if self.max_size.x >= max_width {
            max_width
        } else {
            self.max_size.x
        }
    }

    /// Return the width
    pub fn get_height(&self, max_height: i32) -> i32 {
        if self.max_size.y >= max_height {
            max_height
        } else {
            self.max_size.y
        }
    }
}
