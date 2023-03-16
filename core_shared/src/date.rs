use crate::prelude::*;

/// Holds the current date and time
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Date {
    pub total_minutes           : usize,

    pub hours                   : i32,
    pub minutes                 : i32,
}

impl Date {

    pub fn new() -> Self {
        Self {
            total_minutes       : 0,

            hours               : 0,
            minutes             : 0,
        }
    }

    pub fn from_ticks(&mut self, ticks: usize) {
        let minutes = ticks / 4;
        self.hours = (minutes / 60) as i32;
        self.minutes = (minutes % 60) as i32;

        self.total_minutes = minutes;
    }

    pub fn get_hours(&mut self) -> i32 {
        self.hours
    }

    pub fn get_minutes(&mut self) -> i32 {
        self.minutes
    }

    pub fn time24(&mut self) -> String {
        format!("{:0>2}:{:0>2}", self.hours, self.minutes)
    }
}

use std::cmp::*;

impl PartialOrd for Date {
    fn partial_cmp(&self, other: &Date) -> Option<Ordering> {
        if self.total_minutes == other.total_minutes {
            return  Some(Ordering::Equal);
        }
        if self.total_minutes < other.total_minutes {
            return  Some(Ordering::Less);
        }
        if self.total_minutes > other.total_minutes {
            return  Some(Ordering::Greater);
        }
        None
    }
}

pub fn script_register_date_api(engine: &mut rhai::Engine) {
    engine.register_type_with_name::<Date>("Date")
        .register_get("hours", Date::get_hours)
        .register_get("minutes", Date::get_minutes)

        .register_fn("time24", Date::time24);
}