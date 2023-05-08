use crate::prelude::*;

pub struct RegionUtility {
    pub rng                 : ThreadRng,

    pub roll_regex          : regex::Regex
}

impl RegionUtility {
    pub fn new() -> Self {

        Self {
            rng             : rand::thread_rng(),

            roll_regex      : regex::Regex::new(r"^(\d+)?d(\d+)([+-]\d+)?$").unwrap()
        }
    }

    pub fn roll(&mut self, dice_expression: &str) -> Result<i32, Box<dyn std::error::Error>> {
        let caps = self.roll_regex.captures(dice_expression).ok_or("Invalid dice expression")?;

        let num_dice = caps.get(1).map_or(1, |m| m.as_str().parse::<u32>().unwrap());
        let num_sides = caps.get(2).unwrap().as_str().parse::<u32>()?;
        let modifier = caps.get(3).map_or(0, |m| m.as_str().parse::<i32>().unwrap());

        if num_sides <= 0 {
            return Err("Number of sides must be at least 1".into());
        }

        let total: u32 = (0..num_dice).map(|_| self.rng.gen_range(1..=num_sides)).sum();
        Ok(total as i32 + modifier)
    }
}