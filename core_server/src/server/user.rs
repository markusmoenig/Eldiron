use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct User {
    pub id              : Uuid,
}

impl User {

    pub fn new() -> Self {

        Self {
            id          : Uuid::new_v4(),
        }
    }

}