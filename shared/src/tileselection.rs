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
}
