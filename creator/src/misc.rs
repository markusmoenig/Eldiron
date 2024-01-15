use std::time::{Duration, Instant};

pub struct UpdateTracker {
    //update_counter: u32,
    //last_fps_check: Instant,
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
        UpdateTracker {
            //update_counter: 0,
            //last_fps_check: Instant::now(),
            last_redraw_update: Instant::now(),
            last_tick_update: Instant::now(),
        }
    }

    pub fn update(&mut self, redraw_ms: u64, tick_ms: u64) -> (bool, bool) {
        let mut redraw_update = false;
        let mut tick_update = false;

        // self.update_counter += 1;

        // if self.last_fps_check.elapsed() >= Duration::from_secs(1) {
        //     self.calculate_and_reset_fps();
        // }

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

    // fn calculate_and_reset_fps(&mut self) {
    //     //let fps = self.update_counter;
    //     self.update_counter = 0;
    //     self.last_fps_check = Instant::now();
    //     //println!("FPS: {}", fps);
    // }
}
