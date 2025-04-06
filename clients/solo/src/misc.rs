use instant::{Duration, Instant};

pub struct UpdateTracker {
    last_redraw_update: Instant,
    last_tick_update: Instant,
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
            last_redraw_update: now,
            last_tick_update: now,
        }
    }

    pub fn update(&mut self, redraw_ms: u64, tick_ms: u64) -> (bool, bool) {
        let mut redraw_update = false;
        let mut tick_update = false;

        if self.last_redraw_update.elapsed() >= Duration::from_millis(redraw_ms) {
            self.last_redraw_update = Instant::now();
            redraw_update = true;
        }

        if self.last_tick_update.elapsed() >= Duration::from_millis(tick_ms) {
            self.last_tick_update = Instant::now();
            tick_update = true;
        }

        (redraw_update, tick_update)
    }
}
