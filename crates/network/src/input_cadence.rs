//! Send input at the fixed movement cadence, independently of render rate.
//! Pending frames remain owned by MotionClient until the authority acks them.
use crate::{ClientMessage, MovementCommand};
use std::time::Duration;
#[derive(Default)]
pub struct InputCadence {
    elapsed: Duration,
}
impl InputCadence {
    pub fn advance(
        &mut self,
        elapsed: Duration,
        frames: Vec<MovementCommand>,
    ) -> Option<ClientMessage> {
        let interval = Duration::from_millis(20);
        // Retain fractional frame time at 60/144Hz, but never bank a long
        // suspension into a burst of sends when the client resumes.
        if elapsed >= interval {
            self.elapsed = interval;
        } else {
            self.elapsed += elapsed;
        }
        if frames.is_empty() {
            self.elapsed = self.elapsed.min(interval);
            return None;
        }
        if self.elapsed < interval {
            return None;
        }
        self.elapsed -= interval;
        Some(ClientMessage::Inputs {
            frames: frames.into_iter().take(32).collect(),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use eldiron_runtime::MovementIntent;
    #[test]
    fn high_render_rates_do_not_flood_transport_or_reserve_clock_catchup() {
        for fps in [30, 60, 144, 240, 1000] {
            let mut cadence = InputCadence::default();
            let dt = Duration::from_secs_f64(1. / fps as f64);
            let mut packets = 0;
            for n in 0..fps * 3 {
                let frame = MovementCommand {
                    generation: 1,
                    sequence: n as u64 + 1,
                    intent: MovementIntent::Forward,
                    yaw: 0.,
                };
                if cadence.advance(dt, vec![frame]).is_some() {
                    packets += 1;
                }
            }
            assert!(packets <= 151, "{fps} FPS emitted {packets} packets");
            assert!(
                packets >= (fps * 3).min(149),
                "{fps} FPS emitted only {packets} packets"
            );
        }
        let mut cadence = InputCadence::default();
        assert!(cadence.advance(Duration::MAX, vec![]).is_none());
        let frame = MovementCommand {
            generation: 2,
            sequence: 1,
            intent: MovementIntent::Stop,
            yaw: 0.,
        };
        assert!(cadence.advance(Duration::ZERO, vec![frame]).is_some());
        assert!(cadence.advance(Duration::ZERO, vec![frame]).is_none());
    }
}
