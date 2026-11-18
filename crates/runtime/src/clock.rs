pub use rusterix::simulation::SimulationClock as SessionClock;
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
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
