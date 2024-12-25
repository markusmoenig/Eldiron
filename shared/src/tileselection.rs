use theframework::prelude::*;

#[derive(PartialEq, Clone, Debug)]
pub enum TileSelectionMode {
    Additive,
    Subtractive,
}

#[derive(PartialEq, Clone, Debug)]
pub struct TileSelection {
    pub mode: TileSelectionMode,

    pub rect_start: (i32, i32),
    pub rect_end: (i32, i32),

    pub tiles: FxHashSet<(i32, i32)>,
}

impl Default for TileSelection {
    fn default() -> Self {
        Self::new()
    }
}

impl TileSelection {
    pub fn new() -> Self {
        Self {
            mode: TileSelectionMode::Additive,

            rect_start: (0, 0),
            rect_end: (0, 0),

            tiles: FxHashSet::default(),
        }
    }

    /// Grow the rect by the given amount
    pub fn grow_rect_by(&mut self, (x, y): (i32, i32)) {
        self.rect_start.0 = self.rect_start.0.min(x);
        self.rect_start.1 = self.rect_start.1.min(y);
        self.rect_end.0 = self.rect_end.0.max(x);
        self.rect_end.1 = self.rect_end.1.max(y);
    }

    /// Adds all grid values spanned by the start and end position to an FxHashSet.
    pub fn rect_tiles(&self) -> FxHashSet<(i32, i32)> {
        let mut set = FxHashSet::default();

        for x in self.rect_start.0..=self.rect_end.0 {
            for y in self.rect_start.1..=self.rect_end.1 {
                set.insert((x, y));
            }
        }

        set
    }

    /// Returns the merge of the ongoing selection and the original tiles.
    pub fn merged(&self) -> FxHashSet<(i32, i32)> {
        let mut tiles = self.tiles.clone();

        let new_tiles = self.rect_tiles();

        if self.mode == TileSelectionMode::Additive {
            for nt in new_tiles {
                tiles.insert(nt);
            }
        } else if self.mode == TileSelectionMode::Subtractive {
            for nt in new_tiles {
                tiles.remove(&nt);
            }
        }

        tiles
    }

    /// Returns the bounding box dimensions of the tiles in the selection
    pub fn tile_dimensions(&self) -> Option<(Vec2<i32>, Vec2<i32>, i32, i32)> {
        if self.tiles.is_empty() {
            None
        } else {
            let mut min_x = i32::MAX;
            let mut max_x = i32::MIN;
            let mut min_y = i32::MAX;
            let mut max_y = i32::MIN;

            for &(x, y) in &self.tiles {
                if x < min_x {
                    min_x = x;
                }
                if x > max_x {
                    max_x = x;
                }
                if y < min_y {
                    min_y = y;
                }
                if y > max_y {
                    max_y = y;
                }
            }

            let width = max_x - min_x + 1; // inclusive range
            let height = max_y - min_y + 1; // inclusive range
            Some((
                Vec2::new(min_x, min_y),
                Vec2::new(max_x, max_y),
                width,
                height,
            ))
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty() && self.rect_start == (0, 0) && self.rect_end == (0, 0)
    }
}
