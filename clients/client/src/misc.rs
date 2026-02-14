use instant::{Duration, Instant};

pub struct UpdateTracker {
    next_redraw_update: Instant,
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
            next_redraw_update: now,
            next_tick_update: now,
        }
    }

    pub fn update(&mut self, redraw_ms: u64, tick_ms: u64) -> (bool, bool) {
        let mut redraw_update = false;
        let mut tick_update = false;
        let now = Instant::now();
        let redraw_period = Duration::from_millis(redraw_ms.max(1));
        let tick_period = Duration::from_millis(tick_ms.max(1));

        if now >= self.next_redraw_update {
            redraw_update = true;
            while self.next_redraw_update <= now {
                self.next_redraw_update += redraw_period;
            }
        }

        if now >= self.next_tick_update {
            tick_update = true;
            while self.next_tick_update <= now {
                self.next_tick_update += tick_period;
            }
        }

        (redraw_update, tick_update)
    }
}
