pub use crate::prelude::*;
use std::cmp::{Ord, Ordering, PartialOrd};

/// Represents a time.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Eq, Copy, Hash)]
pub struct TheTime {
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
}

impl Default for TheTime {
    fn default() -> Self {
        Self::new()
    }
}

impl TheTime {
    /// Creates a new instance set to midnight.
    pub fn new() -> Self {
        Self {
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }

    /// Creates a new TheTime with specified hours and minutes, and seconds set to 0.
    /// Validates the input to ensure it falls within the correct range.
    pub fn new_time(hours: u8, minutes: u8) -> Result<Self, String> {
        if hours > 23 || minutes > 59 {
            return Err(
                "Hours must be between 0 and 23, and minutes must be between 0 and 59.".to_string(),
            );
        }
        Ok(Self {
            hours,
            minutes,
            seconds: 0,
        })
    }

    /// Creates a TheTime from an offset in a widget.
    pub fn from_widget_offset(offset_px: u32, total_width_px: u32) -> Self {
        let fraction_of_day = offset_px as f64 / total_width_px as f64;
        let minutes_since_midnight = (1440f64 * fraction_of_day).round() as u32;
        let hours = (minutes_since_midnight / 60) as u8;
        let minutes = (minutes_since_midnight % 60) as u8;

        Self {
            hours,
            minutes,
            seconds: 0,
        }
    }

    /// Calculates the pixel offset in a widget of a given width from the current TheTime.
    pub fn to_widget_offset(&self, total_width_px: u32) -> u32 {
        let total_minutes_in_day: u32 = 1440; // Total minutes in a 24-hour period
        let time_in_minutes = self.hours as u32 * 60 + self.minutes as u32;

        // Calculate the fraction of the day that has passed.
        let fraction_of_day_passed = time_in_minutes as f64 / total_minutes_in_day as f64;

        // Calculate the pixel offset based on the fraction of the day passed.
        (fraction_of_day_passed * total_width_px as f64).round() as u32
    }

    /// Calculate TheTime from server (game) ticks.
    pub fn from_ticks(ticks: i64, ticks_per_minute: u32) -> Self {
        let total_minutes = (ticks / ticks_per_minute as i64) as u32;
        Self {
            hours: ((total_minutes / 60) % 24) as u8,
            minutes: (total_minutes % 60) as u8,
            seconds: (((ticks % ticks_per_minute as i64) * 60 / ticks_per_minute as i64) % 60)
                as u8,
        }
    }

    /// Calculate the ticks from the time.
    pub fn to_ticks(&self, ticks_per_minute: u32) -> i64 {
        let minutes_since_midnight = self.hours as u32 * 60 + self.minutes as u32;
        let seconds_total = self.seconds as u32;

        // Calculate the total ticks from the minutes and seconds part.
        // Ticks for the hours and minutes.
        let ticks_from_minutes = minutes_since_midnight as i64 * ticks_per_minute as i64;
        // Additional ticks from the seconds part, assuming 60 seconds per minute.
        let ticks_from_seconds = (seconds_total as i64 * ticks_per_minute as i64) / 60;

        ticks_from_minutes + ticks_from_seconds
    }

    /// Updates the current TheTime instance based on a time string in "HH:MM" format.
    pub fn from_time_string(&mut self, time_str: &str) -> Result<Self, String> {
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() != 2 {
            return Err("Time must be in HH:MM format".to_string());
        }

        let hours = parts[0]
            .parse::<u8>()
            .map_err(|_| "Invalid hour format".to_string())?;
        let minutes = parts[1]
            .parse::<u8>()
            .map_err(|_| "Invalid minute format".to_string())?;

        if hours >= 24 || minutes >= 60 {
            return Err(
                "Hour must be between 0 and 23 and minutes must be between 0 and 59".to_string(),
            );
        }

        Ok(Self {
            hours,
            minutes,
            seconds: 0,
        })
    }

    /// Returns the total minutes from midnight.
    pub fn total_minutes(&self) -> i32 {
        self.hours as i32 * 60 + self.minutes as i32
    }

    /// Converts the time to total seconds since midnight.
    pub fn to_total_seconds(&self) -> u32 {
        self.hours as u32 * 3600 + self.minutes as u32 * 60 + self.seconds as u32
    }

    /// Calculates the duration between two TheTime instances.
    pub fn duration_between(start: &TheTime, end: &TheTime) -> TheTime {
        let start_in_seconds =
            start.hours as u32 * 3600 + start.minutes as u32 * 60 + start.seconds as u32;
        let end_in_seconds = end.hours as u32 * 3600 + end.minutes as u32 * 60 + end.seconds as u32;

        let mut duration_in_seconds = if end_in_seconds >= start_in_seconds {
            end_in_seconds - start_in_seconds
        } else {
            // If the end time is before the start time, calculate as if it's the next day.
            24 * 3600 + end_in_seconds - start_in_seconds
        };

        let hours = (duration_in_seconds / 3600) as u8 % 24;
        duration_in_seconds %= 3600;

        let minutes = (duration_in_seconds / 60) as u8;
        duration_in_seconds %= 60;

        let seconds = duration_in_seconds as u8;

        TheTime {
            hours,
            minutes,
            seconds,
        }
    }

    /// Returns a string representation in 24-hour format ("HH:MM").
    pub fn to_time24(&self) -> String {
        format!("{:02}:{:02}", self.hours, self.minutes)
    }

    /// Returns a string representation in 12-hour format with AM/PM suffix.
    pub fn to_time12(&self) -> String {
        let period = if self.hours >= 12 { "PM" } else { "AM" };
        let adjusted_hour = if self.hours % 12 == 0 {
            12
        } else {
            self.hours % 12
        };
        format!("{:02}:{:02} {}", adjusted_hour, self.minutes, period)
    }

    /// Returns time as a an f32
    pub fn to_f32(&self) -> f32 {
        self.to_total_seconds() as f32 / 3600.0
    }

    /// Checks if the current TheTime instance is between two other TheTime instances.
    pub fn is_time_between(&self, start: &TheTime, end: &TheTime) -> bool {
        let current_time_in_minutes = self.hours as u16 * 60 + self.minutes as u16;
        let start_time_in_minutes = start.hours as u16 * 60 + start.minutes as u16;
        let end_time_in_minutes = end.hours as u16 * 60 + end.minutes as u16;

        if start_time_in_minutes <= end_time_in_minutes {
            // Normal case: start time is earlier in the day than end time.
            current_time_in_minutes >= start_time_in_minutes
                && current_time_in_minutes <= end_time_in_minutes
        } else {
            // Overnight case: end time is on the next day.
            current_time_in_minutes >= start_time_in_minutes
                || current_time_in_minutes <= end_time_in_minutes
        }
    }
}

impl PartialOrd for TheTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TheTime {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.hours, self.minutes, self.seconds).cmp(&(other.hours, other.minutes, other.seconds))
    }
}
