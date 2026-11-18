//! Fixed region budgets. Avoid worker dispatch when an opt-in scheduled world
//! has at most one awake region; still visit sleepers for commands/counters.
use super::*;
#[derive(Clone, Copy)]
enum Tick {
    Game,
    Redraw,
    Movement,
}
impl Server {
    fn tick_regions(&self, phase: Tick) {
        let visit = |instance: &Arc<Mutex<region::RegionInstance>>| {
            let mut region = instance.lock().unwrap();
            let started = self.diagnostics.as_ref().map(|_| Instant::now());
            match phase {
                Tick::Game => region.system_tick(),
                Tick::Redraw => region.redraw_tick(),
                Tick::Movement => region.movement_tick(),
            }
            if let (Some(started), Some(metrics)) = (started, &self.diagnostics) {
                let mut metrics = metrics.lock().unwrap();
                let t = metrics.entry(region.id).or_default();
                match phase {
                    Tick::Game => &mut t.game,
                    Tick::Redraw => &mut t.redraw,
                    Tick::Movement => &mut t.movement,
                }
                .record(started.elapsed());
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            let parallel = !self.has_region_scheduling()
                || self
                    .instances
                    .iter()
                    .filter(|r| !r.lock().unwrap().idle.suspended)
                    .take(2)
                    .count()
                    > 1;
            if parallel {
                self.instances.par_iter().for_each(visit);
                return;
            }
        }
        self.instances.iter().for_each(visit);
    }
    pub fn system_tick(&self) {
        self.tick_regions(Tick::Game);
    }
    pub fn redraw_tick(&self) {
        self.tick_regions(Tick::Redraw);
    }
    pub fn movement_tick(&self) {
        if self.state != ServerState::Running {
            return;
        }
        self.world
            .advance_resource_time(crate::simulation::MOVEMENT_STEP);
        self.tick_regions(Tick::Movement);
    }
}
