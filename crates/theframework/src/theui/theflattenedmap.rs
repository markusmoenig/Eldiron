use crate::prelude::*;

/// A 2D map with a flat storage structure.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TheFlattenedMap<T> {
    pub data: Vec<Option<T>>,
    pub width: i32,
    pub height: i32,
}

impl<T> TheFlattenedMap<T>
where
    T: Clone,
{
    /// Creates a new `TheFlattenedMap` with specified width and height.
    pub fn new(width: i32, height: i32) -> Self {
        TheFlattenedMap {
            data: vec![None; (width * height) as usize],
            width,
            height,
        }
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        if self.width == width && self.height == height {
            return;
        }
        self.data = vec![None; (width * height) as usize];
        self.width = width;
        self.height = height;
    }

    /// Converts a 2D key into a 1D index.
    fn key_to_index(&self, key: (i32, i32)) -> Option<usize> {
        let (x, y) = key;
        if x >= 0 && x < self.width && y >= 0 && y < self.height {
            Some((y * self.width + x) as usize)
        } else {
            None
        }
    }

    /// Sets the value for a key.
    pub fn set(&mut self, key: (i32, i32), value: T) {
        if let Some(index) = self.key_to_index(key) {
            self.data[index] = Some(value);
        }
    }

    /// Gets a reference to the value for a key, if it exists.
    pub fn get(&self, key: (i32, i32)) -> Option<&T> {
        self.key_to_index(key)
            .and_then(|index| self.data[index].as_ref())
    }

    /// Gets a mutable reference to the value for a key, if it exists.
    pub fn get_mut(&mut self, key: (i32, i32)) -> Option<&mut T> {
        if let Some(index) = self.key_to_index(key) {
            self.data[index].as_mut()
        } else {
            None
        }
    }

    /// Returns the underlying data for direct indexing.
    /// Use with caution, primarily for internal operations.
    pub fn data(&self) -> &Vec<Option<T>> {
        &self.data
    }

    /// Clears the map, setting all values to None.
    pub fn clear(&mut self) {
        for element in &mut self.data {
            *element = None;
        }
    }

    /// Copies the content of another `TheFlattenedMap` into this map at the specified (x, y) offset.
    pub fn copy_into(&mut self, x: i32, y: i32, other: &TheFlattenedMap<T>) {
        for other_y in 0..other.height {
            for other_x in 0..other.width {
                let target_x = x + other_x;
                let target_y = y + other_y;

                // Check if the target position is within the bounds of the current map.
                if target_x >= 0 && target_x < self.width && target_y >= 0 && target_y < self.height
                {
                    if let Some(src_index) = other.key_to_index((other_x, other_y)) {
                        if let Some(dest_index) = self.key_to_index((target_x, target_y)) {
                            self.data[dest_index].clone_from(&other.data[src_index]);
                        }
                    }
                }
            }
        }
    }
}

/// A 3D map with a flat storage structure.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TheFlattenedMap3D<T> {
    data: Vec<Option<T>>,
    min_bounds: (i32, i32, i32),
    max_bounds: (i32, i32, i32),
}

impl<T> TheFlattenedMap3D<T>
where
    T: Clone,
{
    /// Creates a new `TheFlattenedMap3D` with specified bounds.
    pub fn new(min_bounds: (i32, i32, i32), max_bounds: (i32, i32, i32)) -> Self {
        let dimensions = (
            (max_bounds.0 - min_bounds.0 + 1) as usize,
            (max_bounds.1 - min_bounds.1 + 1) as usize,
            (max_bounds.2 - min_bounds.2 + 1) as usize,
        );

        TheFlattenedMap3D {
            data: vec![None; dimensions.0 * dimensions.1 * dimensions.2],
            min_bounds,
            max_bounds,
        }
    }

    /// Converts a 3D key into a 1D index.
    fn key_to_index(&self, key: (i32, i32, i32)) -> Option<usize> {
        let (x, y, z) = key;
        if x >= self.min_bounds.0
            && x <= self.max_bounds.0
            && y >= self.min_bounds.1
            && y <= self.max_bounds.1
            && z >= self.min_bounds.2
            && z <= self.max_bounds.2
        {
            let index = ((z - self.min_bounds.2)
                * (self.max_bounds.0 - self.min_bounds.0 + 1)
                * (self.max_bounds.1 - self.min_bounds.1 + 1)
                + (y - self.min_bounds.1) * (self.max_bounds.0 - self.min_bounds.0 + 1)
                + (x - self.min_bounds.0)) as usize;
            Some(index)
        } else {
            None
        }
    }

    /// Sets the value for a key.
    pub fn set(&mut self, key: (i32, i32, i32), value: T) {
        if let Some(index) = self.key_to_index(key) {
            self.data[index] = Some(value);
        }
    }

    /// Gets a reference to the value for a key, if it exists.
    pub fn get(&self, key: (i32, i32, i32)) -> Option<&T> {
        self.key_to_index(key)
            .and_then(|index| self.data[index].as_ref())
    }

    /// Gets a mutable reference to the value for a key, if it exists.
    pub fn get_mut(&mut self, key: (i32, i32, i32)) -> Option<&mut T> {
        if let Some(index) = self.key_to_index(key) {
            self.data[index].as_mut()
        } else {
            None
        }
    }

    /// Returns the underlying data for direct indexing.
    /// Use with caution, primarily for internal operations.
    pub fn data(&self) -> &Vec<Option<T>> {
        &self.data
    }

    /// Clears the map, setting all values to None.
    pub fn clear(&mut self) {
        for element in &mut self.data {
            *element = None;
        }
    }
}
