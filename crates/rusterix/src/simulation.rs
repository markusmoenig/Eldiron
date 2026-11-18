//! CPU scheduling. Only the host supplies time; input and redraw never do.
use std::time::Duration;

pub const MOVEMENT_STEP: Duration = Duration::from_millis(20);
pub const MAX_MOVEMENT_STEPS: u32 = 128;
pub const MAX_GAME_TICKS: u32 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScheduledStep {
    Game,
    Movement,
}

#[derive(Debug)]
pub struct SimulationClock {
    step: Duration,
    pending: Duration,
    movement_pending: Duration,
    debt: Duration,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SavedClock {
    pub step: Duration,
    pub pending: Duration,
    pub movement_pending: Duration,
    pub debt: Duration,
}
impl SimulationClock {
    pub fn save(&self) -> SavedClock {
        SavedClock {
            step: self.step,
            pending: self.pending,
            movement_pending: self.movement_pending,
            debt: self.debt,
        }
    }
    pub fn restore(saved: &SavedClock) -> Result<Self, String> {
        if saved.step < Duration::from_millis(1)
            || saved.pending >= saved.step
            || saved.movement_pending >= MOVEMENT_STEP
        {
            return Err("Invalid saved simulation clock".into());
        }
        Ok(Self {
            step: saved.step,
            pending: saved.pending,
            movement_pending: saved.movement_pending,
            debt: saved.debt,
        })
    }
    pub fn new(step: Duration) -> Self {
        Self {
            step: step.max(Duration::from_millis(1)),
            pending: Duration::ZERO,
            movement_pending: Duration::ZERO,
            debt: Duration::ZERO,
        }
    }
    /// Compatibility game-only scheduler. Hosts driving a complete world use
    /// schedule() instead, so game ticks and movement keep chronological order.
    pub fn advance(&mut self, elapsed: Duration) -> u32 {
        self.pending = self.pending.saturating_add(elapsed);
        let steps =
            (self.pending.as_nanos() / self.step.as_nanos()).min(MAX_GAME_TICKS as u128) as u32;
        self.pending = self.pending.saturating_sub(self.step * steps);
        steps
    }
    pub fn step(&self) -> Duration {
        self.step
    }
    pub fn reset_elapsed(&mut self) {
        self.pending = Duration::ZERO;
        self.movement_pending = Duration::ZERO;
        self.debt = Duration::ZERO;
    }
    pub fn until_tick(&self) -> Duration {
        self.step
            .saturating_sub(self.pending.saturating_add(self.debt))
    }

    /// Bounded catch-up retains time debt, including partial steps. At a shared
    /// deadline movement runs first, followed by the game tick.
    pub fn schedule(&mut self, elapsed: Duration) -> Vec<ScheduledStep> {
        self.debt = self.debt.saturating_add(elapsed);
        let mut events = Vec::new();
        let (mut game, mut movement) = (0, 0);
        loop {
            let until_game = self.step.saturating_sub(self.pending);
            let until_movement = MOVEMENT_STEP.saturating_sub(self.movement_pending);
            let next = until_game.min(until_movement);
            if self.debt < next {
                self.pending = self.pending.saturating_add(self.debt);
                self.movement_pending += self.debt;
                self.debt = Duration::ZERO;
                break;
            }
            if (next == until_game && game == MAX_GAME_TICKS)
                || (next == until_movement && movement == MAX_MOVEMENT_STEPS)
            {
                break;
            }
            self.debt -= next;
            self.pending = self.pending.saturating_add(next);
            self.movement_pending += next;
            if next == until_movement {
                self.movement_pending -= MOVEMENT_STEP;
                events.push(ScheduledStep::Movement);
                movement += 1;
            }
            if next == until_game {
                self.pending -= self.step;
                events.push(ScheduledStep::Game);
                game += 1;
            }
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn drain(clock: &mut SimulationClock, elapsed: Duration) -> Vec<ScheduledStep> {
        let mut events = clock.schedule(elapsed);
        loop {
            let more = clock.schedule(Duration::ZERO);
            if more.is_empty() {
                return events;
            }
            events.extend(more);
        }
    }
    #[test]
    fn frame_partition_and_catchup_preserve_the_same_timeline() {
        let expected = drain(
            &mut SimulationClock::new(Duration::from_millis(250)),
            Duration::from_secs(3),
        );
        assert_eq!(
            expected
                .iter()
                .filter(|s| **s == ScheduledStep::Game)
                .count(),
            12
        );
        assert_eq!(
            expected
                .iter()
                .filter(|s| **s == ScheduledStep::Movement)
                .count(),
            150
        );
        for fps in [10, 30, 60, 144] {
            let mut clock = SimulationClock::new(Duration::from_millis(250));
            let mut actual = Vec::new();
            for frame in 0..fps * 3 {
                let start = Duration::from_nanos(frame * 1_000_000_000 / fps);
                let end = Duration::from_nanos((frame + 1) * 1_000_000_000 / fps);
                actual.extend(clock.schedule(end - start));
            }
            assert_eq!(actual, expected, "{fps} FPS");
            assert!(clock.schedule(Duration::ZERO).is_empty());
        }
    }
    #[test]
    fn overload_is_bounded_without_losing_time_and_pause_discards_debt() {
        let mut clock = SimulationClock::new(Duration::from_millis(250));
        let events = clock.schedule(Duration::from_secs(10));
        assert!(
            events.iter().filter(|s| **s == ScheduledStep::Game).count() <= MAX_GAME_TICKS as usize
        );
        assert!(
            events
                .iter()
                .filter(|s| **s == ScheduledStep::Movement)
                .count()
                <= MAX_MOVEMENT_STEPS as usize
        );
        clock.reset_elapsed();
        assert!(clock.schedule(Duration::ZERO).is_empty());
        assert!(
            clock
                .schedule(MOVEMENT_STEP - Duration::from_nanos(1))
                .is_empty()
        );
        assert_eq!(
            clock.schedule(Duration::from_nanos(1)),
            vec![ScheduledStep::Movement]
        );
        let events = drain(
            &mut SimulationClock::new(Duration::from_millis(1)),
            Duration::from_secs(1),
        );
        assert_eq!(
            events.iter().filter(|s| **s == ScheduledStep::Game).count(),
            1000
        );
        assert_eq!(
            events
                .iter()
                .filter(|s| **s == ScheduledStep::Movement)
                .count(),
            50
        );
    }
}
