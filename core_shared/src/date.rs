use crate::prelude::*;

/// Holds the current date and time
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Date {
    pub total_minutes           : usize,

    pub hours                   : i32,
    pub minutes                 : i32,
    pub seconds                 : i32,
    pub minutes_in_day          : i32,
}

impl Date {

    pub fn new() -> Self {
        Self {
            total_minutes       : 0,

            hours               : 0,
            minutes             : 0,
            seconds             : 0,
            minutes_in_day      : 0,
        }
    }

    /// New time from hours and minutes
    pub fn new_time(hours: i32, minutes: i32) -> Self {
        Self {
            total_minutes       : hours as usize * 60 + minutes as usize,

            hours,
            minutes,
            seconds             : 0,
            minutes_in_day      : hours * 60 + minutes,
        }
    }

    /// Calculate the date and time from server ticks
    pub fn from_ticks(&mut self, ticks: usize, ticks_per_minute: usize) {
        let minutes = ticks / ticks_per_minute;
        self.seconds = (ticks as i32 % 4) * (60 / ticks_per_minute) as i32;
        self.hours = (minutes / 60) as i32;
        self.minutes = (minutes % 60) as i32;

        self.total_minutes = minutes;
        self.minutes_in_day = (minutes % 1440) as i32;
    }

    /// From Time24
    pub fn from_time24(string: String) -> Self {

        let mut hours = 0;
        let mut minutes = 0;

        let a : Vec<&str> = string.split(":").collect();
        if a.len() == 2 {
            if let Some(h) = a[0].parse::<i32>().ok() {
                if h >= 0 && h <= 23 {
                    hours = h;
                }
            }
            if let Some(m) = a[1].parse::<i32>().ok() {
                if m >= 0 && m <= 59 {
                    minutes = m;
                }
            }
        }

        Self {
            total_minutes       : hours as usize * 60 + minutes as usize,

            hours,
            minutes,
            seconds             : 0,
            minutes_in_day      : hours * 60 + minutes,
        }
    }

    pub fn get_hours(&mut self) -> i32 {
        self.hours
    }

    pub fn get_minutes(&mut self) -> i32 {
        self.minutes
    }

    pub fn get_seconds(&mut self) -> i32 {
        self.seconds
    }

    /// For Rhai, need a mut
    pub fn time24(&mut self) -> String {
        format!("{:0>2}:{:0>2}", self.hours, self.minutes)
    }

    pub fn time12(&mut self) -> String {
        format!("{}{}", self.hours % 12, if self.hours > 12 { "pm" } else { "am" })
    }

    pub fn to_time24(&self) -> String {
        format!("{:0>2}:{:0>2}", self.hours, self.minutes)
    }

    /// Verify if the given string is a valid time24
    pub fn verify_time24(string: String) -> bool {
        let a : Vec<&str> = string.split(":").collect();
        if a.len() == 2 {
            if let Some(h) = a[0].parse::<i32>().ok() {
                if h >= 0 && h <= 23 {
                    if let Some(m) = a[1].parse::<i32>().ok() {
                        if m >= 0 && m <= 59 {
                            return true;
                        }
                    }
                }
            }
        }

        false
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
        .register_get("seconds", Date::get_seconds)

        .register_fn("time12", Date::time12)
        .register_fn("time24", Date::time24);
}