//! A service deadline does not turn an audio/job poll into a visual frame.
use std::time::Duration;
use theframework::platform::FrameRequest;
#[derive(Default)]
pub(super) struct FrameSchedule {
    next: Duration,
    interval: Option<Duration>,
    source_deadline: Option<Duration>,
}
impl FrameSchedule {
    pub fn pending_sources(&mut self, request: &mut FrameRequest, now: Duration, pending: bool) {
        if !pending {
            self.source_deadline = None;
            return;
        }
        let next = self.source_deadline.get_or_insert(now);
        if now >= *next {
            request.redraw = true;
            *next = now.saturating_add(Duration::from_millis(16));
        }
        let wake = next.saturating_sub(now).max(Duration::from_millis(1));
        request.wake_after = Some(request.wake_after.map_or(wake, |old| old.min(wake)));
    }
    pub fn request(&mut self, now: Duration, fps: f64, service: Option<Duration>) -> FrameRequest {
        let interval = FrameRequest::continuous(fps).wake_after.unwrap();
        if self.interval != Some(interval) {
            self.interval = Some(interval);
            self.next = now;
        }
        let redraw = now >= self.next;
        if redraw {
            self.next = now.saturating_add(interval);
        }
        FrameRequest {
            redraw,
            wake_after: Some(
                service
                    .into_iter()
                    .chain(Some(self.next.saturating_sub(now)))
                    .min()
                    .unwrap()
                    .max(Duration::from_millis(1)),
            ),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cold_audio_can_poll_at_low_visual_fps_without_creating_extra_frames_or_timer_debt() {
        let mut frames = FrameSchedule::default();
        let poll = Some(Duration::from_millis(5));
        let request = frames.request(Duration::ZERO, 1., poll);
        assert!(request.redraw);
        assert_eq!(request.wake_after, poll);
        for ms in (5..=250).step_by(5) {
            let request = frames.request(Duration::from_millis(ms), 1., poll);
            assert!(
                !request.redraw,
                "an audio completion poll must not request GPU work"
            );
            assert_eq!(request.wake_after, poll);
        }
        let quiet = frames.request(Duration::from_millis(250), 1., None);
        assert!(!quiet.redraw);
        assert_eq!(quiet.wake_after, Some(Duration::from_millis(750)));
        assert!(frames.request(Duration::from_secs(1), 1., None).redraw);
        let missed = frames.request(Duration::from_secs(10), 1., None);
        assert!(missed.redraw);
        assert_eq!(missed.wake_after, Some(Duration::from_secs(1)));
        assert!(!frames.request(Duration::from_secs(10), 1., None).redraw);
        assert!(
            frames.request(Duration::from_secs(10), 60., None).redraw,
            "changed visual FPS takes effect immediately"
        );
    }
}
