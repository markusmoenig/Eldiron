use crate::prelude::*;

/// Holds the current date and time
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Date {
    pub hours                   : isize,
    pub minutes                 : isize,
}

impl Date {

    pub fn new() -> Self {
        Self {
            hours               : 0,
            minutes             : 0,
        }
    }

    pub fn time_as_24(&self) -> String {
        format!("{}:{}", self.hours, self.minutes)
    }
}

pub fn script_register_date_api(engine: &mut rhai::Engine) {
    engine.register_type_with_name::<Date>("Date")
        .register_fn("time_as_24", Date::time_as_24);
}