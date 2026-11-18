//! Scheduler records, separate from compiled plans and diagnostic history.
use super::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct SavedActor {
    handle: ActorHandle,
    map: Uuid,
    render_id: u32,
    visible: bool,
    activity: Activity,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct SavedRuntime {
    actors: Vec<SavedActor>,
    conditions: Vec<(ActorHandle, BTreeMap<Uuid, bool>)>,
    watchers: Vec<(ActorHandle, BTreeSet<Uuid>)>,
    live: Vec<(ActorHandle, (Uuid, EventObservation))>,
    pending: VecDeque<(ActorHandle, EventObservation, Option<Uuid>)>,
    failed: Vec<(ActorHandle, (Uuid, EventObservation))>,
}
impl Runtime {
    pub fn save(&self) -> SavedRuntime {
        SavedRuntime {
            actors: self
                .actors
                .values()
                .map(|a| SavedActor {
                    handle: a.handle,
                    map: a.map,
                    render_id: a.render_id,
                    visible: a.visible,
                    activity: a.activity,
                })
                .collect(),
            conditions: self
                .conditions
                .iter()
                .map(|(a, b)| (*a, b.clone()))
                .collect(),
            watchers: self.watchers.iter().map(|(a, b)| (*a, b.clone())).collect(),
            live: self
                .live_activity
                .iter()
                .map(|(a, b)| (*a, b.clone()))
                .collect(),
            pending: self.pending.clone(),
            failed: self.failed.iter().map(|(a, b)| (*a, b.clone())).collect(),
        }
    }
    pub fn restore(saved: &SavedRuntime, plan: Option<Arc<Plan>>) -> Result<Self, String> {
        let mut runtime = Self::default();
        for a in &saved.actors {
            if runtime
                .actors
                .insert(
                    a.handle.identity,
                    Actor {
                        handle: a.handle,
                        map: a.map,
                        render_id: a.render_id,
                        visible: a.visible,
                        activity: a.activity,
                        graph: plan.clone(),
                    },
                )
                .is_some()
            {
                return Err("Duplicate saved behavior actor".into());
            }
        }
        let valid = |handle: &ActorHandle| runtime.is_current(*handle);
        if saved.pending.len() > 4096
            || saved.pending.iter().any(|(h, _, _)| !valid(h))
            || saved.live.iter().any(|(h, _)| !valid(h))
            || saved.failed.iter().any(|(h, _)| !valid(h))
        {
            return Err("Invalid saved behavior scheduler".into());
        }
        runtime.conditions = saved.conditions.iter().cloned().collect();
        runtime.watchers = saved.watchers.iter().cloned().collect();
        runtime.live_activity = saved.live.iter().cloned().collect();
        runtime.pending = saved.pending.clone();
        runtime.failed = saved.failed.iter().cloned().collect();
        Ok(runtime)
    }
}
