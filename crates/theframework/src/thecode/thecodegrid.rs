use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TheCodeGridMessageType {
    Error,
    Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TheCodeGridMessage {
    pub message_type: TheCodeGridMessageType,
    pub message: String,
}

impl TheCodeGridMessage {
    pub fn new(message_type: TheCodeGridMessageType, message: String) -> Self {
        Self {
            message_type,
            message,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TheCodeGrid {
    pub id: Uuid,
    pub name: String,

    #[serde(with = "vectorize")]
    pub code: FxHashMap<(u16, u16), TheCodeAtom>,

    #[serde(skip)]
    pub messages: FxHashMap<(u16, u16), TheCodeGridMessage>,
    pub current_pos: Option<(u16, u16)>,
}

impl Default for TheCodeGrid {
    fn default() -> Self {
        TheCodeGrid::new()
    }
}

impl TheCodeGrid {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Unnamed".to_string(),

            code: FxHashMap::default(),

            messages: FxHashMap::default(),
            current_pos: None,
        }
    }

    /// Insert a code atom into the grid.
    pub fn insert_atom(&mut self, pos: (u16, u16), atom: TheCodeAtom) {
        self.code.insert(pos, atom);
    }

    /// Returns the max xy values in the grid
    pub fn max_xy(&self) -> Option<(u16, u16)> {
        let mut max_x = None;
        let mut max_y = None;

        for (x, y) in self.code.keys() {
            max_x = Some(max_x.map_or(*x, |mx| std::cmp::max(mx, *x)));
            max_y = Some(max_y.map_or(*y, |my| std::cmp::max(my, *y)));
        }

        match (max_x, max_y) {
            (Some(max_x), Some(max_y)) => Some((max_x, max_y)),
            _ => None, // Return None if the grid is empty
        }
    }

    /// Returns the next TheCodeAtom in the grid.
    pub fn get_next(&mut self, peek: bool) -> TheCodeAtom {
        if let Some(max_pos) = self.max_xy() {
            //println!("get_next current: {:?}, max_pos: {:?}",  self.current_pos, max_pos);

            if let Some((mut x, mut y)) = self.current_pos {
                // Check if we're at or beyond the maximum position
                if x == max_pos.0 && y == max_pos.1 {
                    return TheCodeAtom::EndOfCode; // Reached the end of the grid
                }

                // Attempt to find the next non-empty position
                //loop {
                if x == max_pos.0 {
                    x = 0;
                    y += 1;
                } else {
                    x += 1;
                }

                if let Some(atom) = self.code.get(&(x, y)) {
                    if !peek {
                        self.current_pos = Some((x, y));
                    }
                    return atom.clone();
                }

                if x == max_pos.0 && y >= max_pos.1 {
                    return TheCodeAtom::EndOfCode; // Reached the end of the grid
                }

                if !peek {
                    self.current_pos = Some((x, y));
                }
                return TheCodeAtom::EndOfExpression;
                //}
            } else {
                // Start from the first position if current_pos is None
                let mut start_y = 0;
                while start_y <= max_pos.1 {
                    if let Some(atom) = self.code.get(&(0, start_y)) {
                        if !peek {
                            self.current_pos = Some((0, start_y));
                        }
                        return atom.clone();
                    }
                    start_y += 1;
                }
            }
        }

        TheCodeAtom::EndOfCode
    }

    /// Checks if the next non-empty TheCodeAtom is on a following line compared to the current position.
    pub fn is_next_on_new_line(&self) -> bool {
        if let Some(current_pos) = self.current_pos {
            let mut next_pos = current_pos;

            // Advance to the next position
            loop {
                if next_pos.0 == self.max_xy().unwrap().0 {
                    next_pos.0 = 0;
                    next_pos.1 += 1;
                } else {
                    next_pos.0 += 1;
                }

                // Break if we find a non-empty atom or reach the end of the grid
                if self.code.contains_key(&next_pos) || next_pos == self.max_xy().unwrap() {
                    break;
                }
            }

            // Compare the y coordinate of the current position with the next non-empty position
            return next_pos.1 > current_pos.1;
        }

        false
    }

    /// Reset the grid iterator.
    pub fn reset_iterator(&mut self) {
        self.current_pos = None;
    }

    /// Clears the messages for the grid.
    pub fn clear_messages(&mut self) {
        self.messages = FxHashMap::default();
    }

    /// Adds a message to the grid.
    pub fn add_message(&mut self, location: (u16, u16), message: TheCodeGridMessage) {
        self.messages.insert(location, message);
    }

    /// Returns the message for the given location (if any).
    pub fn message(&self, location: (u16, u16)) -> Option<TheCodeGridMessage> {
        let mut message: Option<TheCodeGridMessage> = None;
        if let Some(m) = self.messages.get(&location) {
            message = Some(m.clone());
        }
        message
    }

    /// Simulate a 'return' press from a given position, moving all subsequent content down by one line.
    pub fn move_one_line_down(&mut self, start_pos: (u16, u16)) {
        let mut new_code = FxHashMap::default();

        for ((x, y), atom) in self.code.drain() {
            if y > start_pos.1 || (y == start_pos.1 && x >= start_pos.0) {
                // Shift all elements below and including the start position one line down
                new_code.insert((x, y + 2), atom);
            } else {
                // Keep elements above the start position unchanged
                new_code.insert((x, y), atom);
            }
        }

        self.code = new_code;
    }

    /*
    /// Deletes the atom at the current position and moves all subsequent content one position to the left.
    pub fn delete(&mut self, pos: (u16, u16)) -> (u16, u16) {
        let (x, y) = pos;

        let mut new_code = FxHashMap::default();
        let is_start_of_line = x == 0;
        let previous_line_empty =
            is_start_of_line && self.is_line_empty(y - 2) && self.is_line_empty(y - 1);

        if is_start_of_line && previous_line_empty && y > 1 {
            // If at the start of the line and the line above is empty, shift the entire line up
            for ((cx, cy), atom) in self.code.drain() {
                if cy == y {
                    new_code.insert((cx, cy - 2), atom);
                } else if cy != y {
                    new_code.insert((cx, cy), atom);
                }
            }
            // Set the new cursor position to the beginning of the moved up line
            self.code = new_code;
            return (0, y - 2);
        }

        self.code.remove(&pos);
        pos

        /*
        for ((cx, cy), atom) in self.code.drain() {
            if cy < y || (cy == y && cx < x) {
                new_code.insert((cx, cy), atom);
            } else if cy == y && cx == x {
                continue; // Skip the element directly to the left of the current position
            } else if cy == y && cx >= x {
                new_code.insert((cx - 1, cy), atom);
            } else {
                new_code.insert((cx, cy), atom);
            }
        }*/

        // Determine the new position for other cases
        // if is_start_of_line {
        //     if previous_line_empty {
        //         (0, y - 2)
        //     } else {
        //         pos
        //     }
        // } else {
        //     (x, y)
        // }
    }*/

    /// Function to delete an element at a given position
    pub fn delete(&mut self, position: (u16, u16)) -> (u16, u16) {
        // Check if the position is occupied
        if self.code.contains_key(&position) {
            // Check if there are no elements in front of it in the line
            // and if the above line is empty
            if position.1 >= 2 && self.is_line_clear(position) && self.is_line_empty(position.1 - 2)
            {
                // Move all following elements one line up
                self.move_line_up(position);
                return (0, position.1 - 2);
            } else {
                // Remove the element at the position
                self.code.remove(&position);
            }
        }

        position
    }

    /// Moves the items on the line of the given position one position to the right.
    pub fn insert_space(&mut self, pos: (u16, u16)) {
        let (x, y) = pos;
        let mut new_code = FxHashMap::default();

        // Iterate over the code elements and shift each element on the specified line
        for ((cx, cy), atom) in self.code.drain() {
            if cy == y && cx >= x {
                // Shift elements on the specified line and to the right of the position one position to the right
                new_code.insert((cx + 2, cy), atom);
            } else {
                // Keep other elements unchanged
                new_code.insert((cx, cy), atom);
            }
        }

        self.code = new_code;
    }

    // Function to check if there are no elements in front of the position in the same line
    fn is_line_clear(&self, position: (u16, u16)) -> bool {
        let (x, y) = position;
        for col in 0..x {
            if self.code.contains_key(&(col, y)) {
                return false;
            }
        }
        true
    }

    // Function to move all following elements one line up
    fn move_line_up(&mut self, position: (u16, u16)) {
        let (_, y) = position;
        let mut new_code = FxHashMap::default();

        for (&(x, old_y), value) in self.code.iter() {
            if old_y >= y && old_y >= 2 {
                new_code.insert((x, old_y - 2), value.clone());
            } else {
                new_code.insert((x, old_y), value.clone());
            }
        }

        self.code = new_code;
    }

    /// Find the maximum x position in a given line
    fn _max_x_in_line(&self, line: u16) -> u16 {
        self.code
            .iter()
            .filter(|&((_, cy), _)| *cy == line)
            .map(|((cx, _), _)| *cx)
            .max()
            .unwrap_or(0)
    }

    /// Check if a line is empty
    fn is_line_empty(&self, line: u16) -> bool {
        !self.code.iter().any(|(&(_cx, cy), _)| cy == line)
    }

    /// Move all Position values by the given amount.
    pub fn move_positions_by(&mut self, move_by: Vec2<i32>) {
        for atom in self.code.values_mut() {
            if let TheCodeAtom::Value(TheValue::Position(p)) = atom {
                p.x += move_by.x as f32;
                p.z += move_by.y as f32;
            }
        }
    }

    /// Create a grid from json.
    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or(TheCodeGrid::new())
    }

    /// Convert the grid to json.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}
