use std::time::{Duration, Instant};

pub struct UpdateTracker {
    update_counter: u32,
    internal_update_counter: u32,
    last_fps_check: Instant,
    last_internal_update: Instant,
}

impl Default for UpdateTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateTracker {
    pub fn new() -> Self {
        UpdateTracker {
            update_counter: 0,
            internal_update_counter: 0,
            last_fps_check: Instant::now(),
            last_internal_update: Instant::now(),
        }
    }

    pub fn update(&mut self, trigger_ms: u64) -> bool {
        self.update_counter += 1;

        if self.last_fps_check.elapsed() >= Duration::from_secs(1) {
            self.calculate_and_reset_fps();
        }

        if self.last_internal_update.elapsed() >= Duration::from_millis(trigger_ms) {
            self.internal_update();
            self.last_internal_update = Instant::now();
            return true;
        }
        false
    }

    fn calculate_and_reset_fps(&mut self) {
        //let fps = self.update_counter;
        self.update_counter = 0;
        self.last_fps_check = Instant::now();
        //println!("FPS: {}", fps);
    }

    fn internal_update(&mut self) {
        self.internal_update_counter += 1;
        //println!("Internal update triggered: {}", self.internal_update_counter);
    }
}
