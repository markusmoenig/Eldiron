use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct User {
    pub id                      : Uuid,
    pub name                    : String,
    pub screen_script           : Option<String>,
    pub new_screen_script       : Option<String>,
}

impl User {

    pub fn new() -> Self {

        Self {
            id                  : Uuid::new_v4(),
            name                : String::new(),
            screen_script       : None,
            new_screen_script   : None,
        }
    }
}