use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct TileArea {
    pub start: (i32, i32),
    pub end: (i32, i32),

    pub ongoing: bool,
}

impl Default for TileArea {
    fn default() -> Self {
        Self::new()
    }
}

impl TileArea {
    pub fn new() -> Self {
        Self {
            start: (0, 0),
            end: (0, 0),
            ongoing: true,
        }
    }

    pub fn grow_by(&mut self, (x, y): (i32, i32)) {
        if (x > self.start.0 && y == self.start.1) || y > self.start.1 {
            self.end.0 = x;
            self.end.1 = y;
        }
        else if (x < self.start.0 && y == self.start.1) || y < self.start.1 {
            self.start.0 = x;
            self.start.1 = y;
        }
        else {
            self.end.0 = self.start.0;
            self.end.1 = self.start.1;
        }
    }

    /// Adds all grid values spanned by the start and end position to an FxHashSet.
    pub fn tiles(&self) -> FxHashSet<(i32, i32)> {
        let mut set = FxHashSet::default();

        for x in self.start.0..=self.end.0 {
            for y in self.start.1..=self.end.1 {
                set.insert((x, y));
            }
        }

        set
    }

    /// Create a region from json.
    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or(TileArea::new())
    }

    /// Convert the region to json.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}