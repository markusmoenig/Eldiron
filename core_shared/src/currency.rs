use crate::prelude::*;

const GOLD_IN_SILVER: i32 = 10;

/// Holds the current date and time
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
pub struct Currency {
    pub gold: i32,
    pub silver: i32,
}

impl Currency {
    pub fn empty() -> Self {
        Self { gold: 0, silver: 0 }
    }

    /// New currency from gold and silver
    pub fn new(gold: i32, silver: i32) -> Self {
        Self { gold, silver }
    }

    /// Absolute value in silver, used to compare amounts of money
    pub fn absolute(&self) -> i32 {
        self.gold * GOLD_IN_SILVER + self.silver
    }

    pub fn get_gold(&mut self) -> i32 {
        self.gold
    }

    pub fn get_silver(&mut self) -> i32 {
        self.silver
    }

    /// Add the given amount of money.
    pub fn add(&mut self, other: Currency) {
        self.gold += other.gold;
        self.silver += other.silver;

        self.gold += self.silver / GOLD_IN_SILVER;
        self.silver += self.silver % GOLD_IN_SILVER;
    }

    /// Remove the given amount of money.
    pub fn remove(&mut self, other: Currency) {
        self.gold -= other.gold;
        self.silver -= other.silver;

        if self.gold < 0 {
            self.silver -= self.gold.abs() * GOLD_IN_SILVER;
        }

        if self.silver < 0 {
            self.gold -= self.silver.abs() / GOLD_IN_SILVER;
        }

        self.gold = self.gold.clamp(0, i32::MAX);
        self.silver = self.silver.clamp(0, i32::MAX);

        self.gold += self.silver / GOLD_IN_SILVER;
        self.silver += self.silver % GOLD_IN_SILVER;
    }

    /// For Rhai, need a mut
    pub fn to_string(&mut self) -> String {
        if self.gold != 0 && self.silver != 0 {
            format!("{}G {}S", self.gold, self.silver)
        } else if self.gold != 0 {
            format!("{}G", self.gold)
        } else {
            format!("{}S", self.silver)
        }
    }

    pub fn register(engine: &mut rhai::Engine) {
        engine
            .register_type_with_name::<Currency>("Currency")
            .register_get("gold", Currency::get_gold)
            .register_get("silver", Currency::get_silver)
            .register_fn("to_string", Currency::to_string);
    }
}

use std::cmp::*;

impl PartialOrd for Currency {
    fn partial_cmp(&self, other: &Currency) -> Option<Ordering> {
        if self.absolute() == other.absolute() {
            return Some(Ordering::Equal);
        }
        if self.absolute() < other.absolute() {
            return Some(Ordering::Less);
        }
        if self.absolute() > other.absolute() {
            return Some(Ordering::Greater);
        }
        None
    }
}
