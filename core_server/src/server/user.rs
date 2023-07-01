use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct User {
    pub id              : Uuid,
    pub initialized     : bool,
}

impl User {

    pub fn new() -> Self {

        Self {
            id          : Uuid::new_v4(),
            initialized : false,
        }
    }

}