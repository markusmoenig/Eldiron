//! CPU update and addressed output ownership, independent of storage/renderer.
use crate::{EventInterests, Project, RegionEvents, SessionClock, SessionUpdate};
use rustc_hash::FxHashMap;
use rusterix::{Assets, Server, ServerState};
use std::time::Duration;
use theframework_core::Uuid;

/// One pump per local runtime incarnation. GameSession owns all source storage;
/// migration hosts may lend explicit CPU parts while retiring their facade.
/// Presentation never calls this API or drains Server output channels.
pub struct SessionPump {
    session_id: Uuid,
    tick_sequence: u64,
    interests: EventInterests,
    events: FxHashMap<Uuid, RegionEvents>,
}
impl SessionPump {
    pub fn new(interests: EventInterests) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            tick_sequence: 0,
            interests,
            events: FxHashMap::default(),
        }
    }
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }
    pub fn tick_sequence(&self) -> u64 {
        self.tick_sequence
    }
    pub(crate) fn renew_identity(&mut self) {
        self.session_id = Uuid::new_v4();
    }
    pub(crate) fn restart_collection(&mut self, interests: EventInterests) {
        self.interests = interests;
        self.tick_sequence = 0;
        self.events.clear();
    }
    pub fn invalidate(&mut self) {
        self.session_id = Uuid::new_v4();
        self.events.clear();
    }
    pub fn clear_events(&mut self) {
        self.events.clear();
    }
    pub fn take_events(&mut self, id: &Uuid) -> RegionEvents {
        self.events.remove(id).unwrap_or_default()
    }
    pub fn pending_events(&mut self, id: &Uuid) -> Option<&mut RegionEvents> {
        self.events.get_mut(id)
    }
    pub fn advance(
        &mut self,
        clock: &mut SessionClock,
        elapsed: Duration,
        project: &mut Project,
        server: &mut Server,
        assets: &mut Assets,
        current_map: &mut String,
    ) -> SessionUpdate {
        let ticks = if server.state == ServerState::Running {
            clock.advance(elapsed)
        } else {
            clock.reset_elapsed();
            0
        };
        self.update(ticks, true, project, server, assets, current_map)
    }
    /// Explicit stepping/polling and legacy redraw cadence share the same sole
    /// collector. Time is returned to the owning document/runtime policy.
    pub fn update(
        &mut self,
        ticks: u32,
        redraw: bool,
        project: &mut Project,
        server: &mut Server,
        assets: &mut Assets,
        current_map: &mut String,
    ) -> SessionUpdate {
        self.tick_sequence = self.tick_sequence.saturating_add(u64::from(ticks));
        for _ in 0..ticks {
            server.system_tick();
        }
        if redraw {
            server.redraw_tick();
        }
        let changed = server.update(assets);
        if let Some(name) = &changed {
            *current_map = name.clone();
        }
        let mut time = None;
        for region in &mut project.regions {
            server.apply_entities_items(&mut region.map);
            if region.map.name == *current_map {
                time = server.get_time(&region.map.id);
            }
            self.events.entry(region.map.id).or_default().collect(
                server,
                &region.map.id,
                self.interests,
            );
        }
        SessionUpdate {
            system_ticks: ticks,
            region_changed: changed,
            time,
        }
    }
}
