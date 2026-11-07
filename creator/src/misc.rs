// use crate::prelude::*;
use std::time::{Duration, Instant};

/// Opt-in frame phase timings for native benchmark runs. Disabled by default.
pub(crate) struct FrameProfile {
    state: Option<(Instant, Vec<(&'static str, f64)>)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_redraws_preserve_the_game_tick_deadline() {
        let mut tracker = UpdateTracker::new();
        tracker.next_tick_update = Instant::now() + Duration::from_secs(60);
        for _ in 0..5 {
            tracker.force_redraw();
            assert_eq!(tracker.update(33, 250), (true, false));
        }
    }
}
impl FrameProfile {
    pub fn new() -> Self {
        static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        Self {
            state: (*ENABLED.get_or_init(|| std::env::var_os("ELDIRON_PROFILE_EDITOR").is_some()))
                .then(|| (Instant::now(), Vec::new())),
        }
    }
    pub fn lap(&mut self, name: &'static str) {
        if let Some((last, phases)) = &mut self.state {
            phases.push((name, last.elapsed().as_secs_f64() * 1000.));
            *last = Instant::now();
        }
    }
}
impl Drop for FrameProfile {
    fn drop(&mut self) {
        if let Some((_, phases)) = &self.state {
            eprintln!(
                "Editor phases {}",
                phases
                    .iter()
                    .map(|(name, ms)| format!("{name}={ms:.2}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
    }
}
// use theframework::prelude::*;

pub struct UpdateTracker {
    //update_counter: u32,
    //last_fps_check: Instant,
    next_redraw_update: Instant,
    next_tick_update: Instant,
}

impl Default for UpdateTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateTracker {
    /// Benchmark a redraw without accelerating the simulation tick clock.
    #[cfg(test)]
    pub fn force_redraw(&mut self) {
        self.next_redraw_update = Instant::now();
    }

    pub fn new() -> Self {
        let now = Instant::now();
        UpdateTracker {
            //update_counter: 0,
            //last_fps_check: Instant::now(),
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

        // self.update_counter += 1;

        // if self.last_fps_check.elapsed() >= Duration::from_secs(1) {
        //     self.calculate_and_reset_fps();
        // }

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

    // fn calculate_and_reset_fps(&mut self) {
    //     //let fps = self.update_counter;
    //     self.update_counter = 0;
    //     self.last_fps_check = Instant::now();
    //     //println!("FPS: {}", fps);
    // }
}
