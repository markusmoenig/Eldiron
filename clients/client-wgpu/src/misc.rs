use instant::{Duration, Instant};

pub struct UpdateTracker {
    next_tick_update: Instant,
}

impl Default for UpdateTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateTracker {
    pub fn new() -> Self {
        let now = Instant::now();
        UpdateTracker {
            next_tick_update: now,
        }
    }

    pub fn update(&mut self, tick_period: Duration) -> bool {
        let mut tick_update = false;
        let now = Instant::now();

        if now >= self.next_tick_update {
            tick_update = true;
            while self.next_tick_update <= now {
                self.next_tick_update += tick_period;
            }
        }

        tick_update
    }
}
