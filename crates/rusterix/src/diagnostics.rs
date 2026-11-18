//! Opt-in CPU timings. Snapshots contain counters, not game/account payloads.
use serde::Serialize;
use std::{
    collections::{BTreeMap, VecDeque},
    time::Duration,
};

#[derive(Clone, Debug, Default)]
pub struct Timing {
    calls: u64,
    total_us: u64,
    last_us: u64,
    max_us: u64,
    recent: VecDeque<u64>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct TimingSummary {
    pub calls: u64,
    pub total_us: u64,
    pub last_us: u64,
    pub max_us: u64,
    pub rolling_samples: usize,
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
}
impl Timing {
    pub fn record(&mut self, duration: Duration) {
        let us = duration.as_micros().min(u64::MAX as u128) as u64;
        self.calls = self.calls.saturating_add(1);
        self.total_us = self.total_us.saturating_add(us);
        self.last_us = us;
        self.max_us = self.max_us.max(us);
        if self.recent.len() == 256 {
            self.recent.pop_front();
        }
        self.recent.push_back(us);
    }
    pub fn summary(&self) -> TimingSummary {
        let mut values: Vec<_> = self.recent.iter().copied().collect();
        values.sort_unstable();
        let percentile = |n: usize| {
            if values.is_empty() {
                0
            } else {
                values[(values.len() * n).div_ceil(100).saturating_sub(1)]
            }
        };
        TimingSummary {
            calls: self.calls,
            total_us: self.total_us,
            last_us: self.last_us,
            max_us: self.max_us,
            rolling_samples: values.len(),
            p50_us: percentile(50),
            p95_us: percentile(95),
            p99_us: percentile(99),
        }
    }
}
/// Inner phases overlap their enclosing region/pump timers. No payload data.
#[derive(Clone, Debug, Default)]
pub struct RegionDetail {
    pub dynamic_collision: Timing,
    pub geometry_collision: Timing,
    pub entity_clone: Timing,
    pub entity_merge: Timing,
    pub scripts: Timing,
    pub behaviors: Timing,
    pub entity_candidates: u64,
    pub item_candidates: u64,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct RegionDetailSummary {
    pub dynamic_collision: TimingSummary,
    pub geometry_collision: TimingSummary,
    pub entity_clone: TimingSummary,
    pub entity_merge: TimingSummary,
    pub scripts: TimingSummary,
    pub behaviors: TimingSummary,
    pub entity_candidates: u64,
    pub item_candidates: u64,
}
impl RegionDetail {
    pub fn summary(&self) -> RegionDetailSummary {
        RegionDetailSummary {
            dynamic_collision: self.dynamic_collision.summary(),
            geometry_collision: self.geometry_collision.summary(),
            entity_clone: self.entity_clone.summary(),
            entity_merge: self.entity_merge.summary(),
            scripts: self.scripts.summary(),
            behaviors: self.behaviors.summary(),
            entity_candidates: self.entity_candidates,
            item_candidates: self.item_candidates,
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct RegionTiming {
    pub redraw: Timing,
    pub movement: Timing,
    pub game: Timing,
}
#[derive(Clone, Debug, Serialize)]
pub struct RegionTimingSummary {
    pub region: u32,
    pub details: RegionDetailSummary,
    pub redraw: TimingSummary,
    pub movement: TimingSummary,
    pub game: TimingSummary,
}
pub type RegionTimings = BTreeMap<u32, RegionTiming>;
pub fn region_summaries(timings: &RegionTimings) -> Vec<RegionTimingSummary> {
    timings
        .iter()
        .map(|(region, t)| RegionTimingSummary {
            region: *region,
            details: Default::default(),
            redraw: t.redraw.summary(),
            movement: t.movement.summary(),
            game: t.game.summary(),
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timings_bound_memory_and_keep_rolling_percentiles_and_lifetime_totals() {
        let mut timing = Timing::default();
        for i in 1..=1000 {
            timing.record(Duration::from_micros(i));
        }
        let s = timing.summary();
        assert_eq!(s.calls, 1000);
        assert_eq!(s.total_us, 500500);
        assert_eq!(s.rolling_samples, 256);
        assert_eq!(s.p95_us, 988);
        assert_eq!(s.max_us, 1000);
    }
}
