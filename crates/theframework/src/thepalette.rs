pub use crate::prelude::*;
use std::ops::{Index, IndexMut};

/// Holds an array of colors.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ThePalette {
    #[serde(default)]
    pub current_index: u16,
    pub colors: Vec<Option<TheColor>>,
}

impl Default for ThePalette {
    fn default() -> Self {
        Self::empty_256()
    }
}

impl ThePalette {
    pub fn new(colors: Vec<Option<TheColor>>) -> Self {
        Self {
            current_index: 0,
            colors,
        }
    }

    pub fn empty_256() -> Self {
        let mut colors = Vec::new();
        for _ in 0..256 {
            colors.push(None);
        }
        Self {
            current_index: 0,
            colors,
        }
    }

    /// Test if the palette is empty
    pub fn is_empty(&self) -> bool {
        for v in self.colors.iter() {
            if v.is_some() {
                return false;
            }
        }
        true
    }

    /// Get the color at the current index
    pub fn get_current_color(&self) -> Option<TheColor> {
        self.colors[self.current_index as usize].clone()
    }

    /// Clears all palette colors.
    pub fn clear(&mut self) {
        for v in self.colors.iter_mut() {
            *v = None;
        }
    }

    /// Load the palette from a Paint.net TXT file
    pub fn load_from_txt(&mut self, txt: String) {
        let mut index = self.current_index as usize;
        for line in txt.lines() {
            // Ignore comments
            if line.starts_with(';') {
                continue;
            }

            let mut chars = line.chars();

            // Skip Alpha
            if chars.next().is_none() {
                return;
            }
            if chars.next().is_none() {
                return;
            }

            // R
            let mut r_string = "".to_string();
            if let Some(c) = chars.next() {
                r_string.push(c);
            }
            if let Some(c) = chars.next() {
                r_string.push(c);
            }

            let r = u8::from_str_radix(&r_string, 16);

            // G
            let mut g_string = "".to_string();
            if let Some(c) = chars.next() {
                g_string.push(c);
            }
            if let Some(c) = chars.next() {
                g_string.push(c);
            }

            let g = u8::from_str_radix(&g_string, 16);

            // B
            let mut b_string = "".to_string();
            if let Some(c) = chars.next() {
                b_string.push(c);
            }
            if let Some(c) = chars.next() {
                b_string.push(c);
            }

            let b = u8::from_str_radix(&b_string, 16);

            if r.is_ok() && g.is_ok() && b.is_ok() {
                let r = r.ok().unwrap();
                let g = g.ok().unwrap();
                let b = b.ok().unwrap();

                if index < self.colors.len() {
                    self.colors[index] = Some(TheColor::from_u8(r, g, b, 0xFF));
                }

                index += 1;
            }
        }
    }

    /// Adds a color to the palette if it doesn't already exist.
    /// Returns the index where the color exists or was inserted.
    pub fn add_unique_color(&mut self, color: TheColor) -> Option<usize> {
        // Check if color already exists
        for (i, existing) in self.colors.iter().enumerate() {
            if let Some(existing_color) = existing {
                if *existing_color == color {
                    return Some(i);
                }
            }
        }

        // Try to insert into the first empty slot
        for (i, slot) in self.colors.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(color);
                return Some(i);
            }
        }

        // Palette is full
        None
    }

    /*
    /// Returns the index of the closest matching color in the palette.
    /// Returns `None` if the palette is empty.
    pub fn find_closest_color_index(&self, color: &TheColor) -> Option<usize> {
        let mut best_index = None;
        let mut best_distance = f32::MAX;

        for (i, entry) in self.colors.iter().enumerate() {
            if let Some(existing) = entry {
                // Compute squared Euclidean distance in RGBA space
                let dr = existing.r - color.r;
                let dg = existing.g - color.g;
                let db = existing.b - color.b;
                let da = existing.a - color.a;

                let dist_sq = dr * dr + dg * dg + db * db + da * da;

                if dist_sq < best_distance {
                    best_distance = dist_sq;
                    best_index = Some(i);
                }
            }
        }

        best_index
    }*/

    /// Returns the index of the palette color that best matches the given color.
    /// Used for palette remapping (closest-color quantization).
    /// Returns `None` if the palette is empty.
    pub fn find_closest_color_index(&self, color: &TheColor) -> Option<usize> {
        let mut best_index = None;
        let mut best_distance = f32::MAX;

        for (i, entry) in self.colors.iter().enumerate() {
            if let Some(existing) = entry {
                // Perceptual weighted distance in linear RGBA space
                let dr = existing.r - color.r;
                let dg = existing.g - color.g;
                let db = existing.b - color.b;
                let da = existing.a - color.a;

                // Human-vision–weighted RGB, alpha has lower influence
                let dist = dr * dr * 0.30 + dg * dg * 0.59 + db * db * 0.11 + da * da * 0.05;

                if dist < best_distance {
                    best_distance = dist;
                    best_index = Some(i);
                }
            }
        }

        best_index
    }
}

impl Index<usize> for ThePalette {
    type Output = Option<TheColor>;

    fn index(&self, index: usize) -> &Self::Output {
        if index < self.colors.len() {
            &self.colors[index]
        } else {
            panic!("Color Index out of bounds!");
        }
    }
}

impl IndexMut<usize> for ThePalette {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index < self.colors.len() {
            &mut self.colors[index]
        } else {
            panic!("Color Index out of bounds!");
        }
    }
}
