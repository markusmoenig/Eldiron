use std::time::Duration;
/// Hosts supply elapsed time. Rendering frequency never advances simulation on
/// its own; bounded catch-up retains the remaining debt for the next update.
#[derive(Debug)]
pub struct SessionClock {
    step: Duration,
    pending: Duration,
    max_steps: u32,
}
impl SessionClock {
    pub fn new(step: Duration) -> Self {
        Self {
            step: step.max(Duration::from_millis(1)),
            pending: Duration::ZERO,
            max_steps: 8,
        }
    }
    pub fn advance(&mut self, elapsed: Duration) -> u32 {
        self.pending = self.pending.saturating_add(elapsed);
        let steps =
            (self.pending.as_nanos() / self.step.as_nanos()).min(self.max_steps as u128) as u32;
        self.pending = self.pending.saturating_sub(self.step * steps);
        steps
    }
    pub fn step(&self) -> Duration {
        self.step
    }
    pub fn reset_elapsed(&mut self) {
        self.pending = Duration::ZERO;
    }
    pub fn until_tick(&self) -> Duration {
        self.step.saturating_sub(self.pending)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frame_rate_does_not_change_tick_count_or_drop_catchup_debt() {
        let mut clock = SessionClock::new(Duration::from_millis(250));
        assert_eq!(
            (0..100)
                .map(|_| clock.advance(Duration::from_millis(10)) as u64)
                .sum::<u64>(),
            4
        );
        assert_eq!(clock.advance(Duration::from_secs(3)), 8);
        assert_eq!(clock.advance(Duration::ZERO), 4);
        assert_eq!(clock.until_tick(), Duration::from_millis(250));
        clock.advance(Duration::from_millis(100));
        clock.reset_elapsed();
        assert_eq!(clock.until_tick(), Duration::from_millis(250));
    }
}
