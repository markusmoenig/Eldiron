
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq)]
pub struct GameUpdate {

    pub position                : Option<(usize, isize, isize)>,
    pub tile                    : Option<(usize, usize, usize)>,

}