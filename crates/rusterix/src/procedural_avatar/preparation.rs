//! Equipment preparation runs before presentation, never under a render-cache
//! lock. Only the update thread publishes completed variants, atomically between
//! frames, so mesh, rig and sprite sampling cannot see different variants.
use super::*;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc,
};

struct Job {
    generation: u64,
    asset: Arc<Appearance>,
    assets: Arc<Assets>,
    plan: Arc<VariantPlan>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Assets, Arc<Appearance>, Entity) {
        let (mut assets, body, mut entity) = crate::procedural_avatar::tests::fixture();
        entity.id = 1;
        let body = Arc::new(body);
        assets
            .procedural_avatars
            .insert("humanoid".into(), body.clone());
        body.compiled_for(&entity, &assets).unwrap();
        (assets, body, entity)
    }
    fn bow() -> crate::Item {
        let templates = eldiron_ruleset::ruleset_item_templates_from_source(
            eldiron_ruleset::latest_official_ruleset(),
        )
        .unwrap();
        let mut item = crate::Item::default();
        crate::server::data::apply_item_data(
            &mut item,
            &templates
                .iter()
                .find(|t| t.id == "hunting_bow")
                .unwrap()
                .data,
        );
        item.set_attribute("slot", crate::Value::Str("main_hand".into()));
        item
    }
    #[test]
    fn inventory_variant_is_prepared_without_blocking_or_midframe_publication() {
        let (assets, body, mut entity) = fixture();
        let old = body.compiled_for(&entity, &assets).unwrap().1;
        let remembered = body.last_ready.lock().unwrap().len();
        // Temporary item/editor previews must not add one fallback per draw.
        for _ in 0..10 {
            body.compiled_for(&Entity::default(), &assets).unwrap();
        }
        assert_eq!(body.last_ready.lock().unwrap().len(), remembered);
        entity.inventory = vec![None; 4];
        entity.add_item(bow()).unwrap();
        let mut next = entity.clone();
        next.equipped.insert("main_hand".into(), bow());
        let expected_key = body.variant_plan(&next, &assets).key;
        let mut preparation = Preparation::default();
        preparation.prepare(std::iter::once(&entity), &assets);
        let completed = preparation
            .results
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert_eq!(completed.key, expected_key);
        assert!(
            !body.compiled.lock().unwrap().contains_key(&expected_key),
            "worker must not publish during a draw"
        );
        assert_eq!(
            body.compiled_for(&next, &assets).unwrap().1,
            old,
            "keep the previous ready outfit"
        );
        preparation.publish(completed);
        let (prepared, key) = body.compiled_for(&next, &assets).unwrap();
        assert_eq!(key, expected_key);
        let reference = body
            .evaluate_plan(&body.variant_plan(&next, &assets), &assets)
            .unwrap();
        assert_eq!(prepared.evaluation.mesh, reference.evaluation.mesh);
        assert_eq!(
            prepared
                .materials
                .iter()
                .map(|m| &m.texture.data)
                .collect::<Vec<_>>(),
            reference
                .materials
                .iter()
                .map(|m| &m.texture.data)
                .collect::<Vec<_>>()
        );
        let actor = (entity.creator_id, entity.id);
        let planned = preparation.plans[&actor].plans[0].clone();
        entity.position.x += 1.;
        entity.set_attribute("avatar_attack_left", crate::Value::Float(0.2));
        preparation.prepare(std::iter::once(&entity), &assets);
        assert!(
            Arc::ptr_eq(&planned, &preparation.plans[&actor].plans[0]),
            "Movement and animation must not re-plan the inventory"
        );
        entity.inventory[0]
            .as_mut()
            .unwrap()
            .set_attribute("active", crate::Value::Bool(true));
        preparation.prepare(std::iter::once(&entity), &assets);
        assert!(
            !Arc::ptr_eq(&planned, &preparation.plans[&actor].plans[0]),
            "Item appearance state must invalidate cached plans"
        );
    }
    #[test]
    fn obsolete_equipment_job_cannot_publish_into_a_replaced_registry() {
        let (mut assets, body, mut entity) = fixture();
        entity.equipped.insert("main_hand".into(), bow());
        let mut preparation = Preparation::default();
        preparation.prepare(std::iter::once(&entity), &assets);
        let completed = preparation
            .results
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        let key = completed.key;
        let equipment = body
            .equipment
            .iter()
            .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
            .collect::<Vec<_>>();
        let replacement =
            Arc::new(Appearance::new(&body.source, &equipment, body.signature).unwrap());
        assets
            .procedural_avatars
            .insert("humanoid".into(), replacement.clone());
        preparation.prepare(std::iter::once(&entity), &assets);
        preparation.publish(completed);
        assert!(!replacement.compiled.lock().unwrap().contains_key(&key));
        assert!(!body.compiled.lock().unwrap().contains_key(&key));
    }
}
struct Completed {
    generation: u64,
    asset: Arc<Appearance>,
    key: u64,
    result: Result<Arc<Compiled>, String>,
}

struct PlannedEntity {
    signature: u64,
    plans: Vec<Arc<VariantPlan>>,
}

#[derive(Default)]
pub(crate) struct Preparation {
    generation: u64,
    current: Arc<AtomicU64>,
    snapshot: Option<Arc<Assets>>,
    jobs: Option<mpsc::SyncSender<Job>>,
    results: Option<Mutex<mpsc::Receiver<Completed>>>,
    pending: FxHashSet<(usize, u64)>,
    failed: FxHashSet<(usize, u64)>,
    plans: FxHashMap<(Uuid, u32), PlannedEntity>,
}

impl Drop for Preparation {
    fn drop(&mut self) {
        self.current.store(0, Ordering::Release);
        if let Some(assets) = &self.snapshot {
            for asset in assets.procedural_avatars.values() {
                asset.preparing.lock().unwrap().clear();
            }
        }
    }
}

impl Preparation {
    fn start(&mut self) {
        let (jobs, input) = mpsc::sync_channel::<Job>(16);
        let (output, results) = mpsc::sync_channel(16);
        let current = self.current.clone();
        std::thread::Builder::new()
            .name("equipment-variants".into())
            .spawn(move || {
                while let Ok(job) = input.recv() {
                    if current.load(Ordering::Acquire) != job.generation {
                        continue;
                    }
                    let result = job.asset.evaluate_plan(&job.plan, &job.assets);
                    if output
                        .send(Completed {
                            generation: job.generation,
                            asset: job.asset,
                            key: job.plan.key,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("equipment preparation worker");
        self.jobs = Some(jobs);
        self.results = Some(Mutex::new(results));
    }

    fn request(&mut self, asset: &Arc<Appearance>, actor: (Uuid, u32), plan: &Arc<VariantPlan>) {
        let assets = self.snapshot.as_ref().unwrap();
        if asset.compiled.lock().unwrap().contains_key(&plan.key) {
            return;
        }
        // An actor's first visible appearance is built exactly during loading.
        // Do not compete with it or build a duplicate on a background thread.
        if !asset.last_ready.lock().unwrap().contains_key(&actor) {
            return;
        }
        asset.preparing.lock().unwrap().insert((actor, plan.key));
        let token = (Arc::as_ptr(asset) as usize, plan.key);
        if self.pending.contains(&token) || self.failed.contains(&token) {
            return;
        }
        if self
            .jobs
            .as_ref()
            .unwrap()
            .try_send(Job {
                generation: self.generation,
                asset: asset.clone(),
                assets: assets.clone(),
                plan: plan.clone(),
            })
            .is_ok()
        {
            self.pending.insert(token);
        }
    }

    fn publish(&mut self, completed: Completed) {
        if completed.generation != self.generation {
            return;
        }
        let token = (Arc::as_ptr(&completed.asset) as usize, completed.key);
        self.pending.remove(&token);
        match completed.result {
            Ok(ready) => completed.asset.publish_variant(completed.key, ready),
            Err(error) => {
                self.failed.insert(token);
                eprintln!("Equipment appearance preparation: {error}");
            }
        }
    }

    pub fn prepare<'a>(
        &mut self,
        entities: impl Iterator<Item = &'a Entity> + Clone,
        assets: &Assets,
    ) {
        if assets.procedural_avatars.is_empty() {
            return;
        }
        let mut versions = assets
            .procedural_avatars
            .iter()
            .map(|(id, a)| (id.as_str(), a.signature, Arc::as_ptr(a) as usize))
            .collect::<Vec<_>>();
        versions.sort_unstable();
        let generation = fingerprint(&versions);
        if self.snapshot.is_none() || self.generation != generation {
            self.generation = generation;
            self.current.store(generation, Ordering::Release);
            self.pending.clear();
            self.failed.clear();
            self.plans.clear();
            self.snapshot = Some(Arc::new(assets.appearance_preparation_snapshot()));
        }
        if self.jobs.is_none() {
            self.start();
        }
        // Publishing happens only here, before scene/HUD drawing starts.
        let completed = self
            .results
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .try_iter()
            .collect::<Vec<_>>();
        for completed in completed {
            self.publish(completed);
        }
        let live = entities
            .clone()
            .map(|e| (e.creator_id, e.id))
            .collect::<FxHashSet<_>>();
        for asset in assets.procedural_avatars.values() {
            asset.preparing.lock().unwrap().clear();
            asset
                .last_ready
                .lock()
                .unwrap()
                .retain(|id, _| live.contains(id));
        }
        self.plans.retain(|id, _| live.contains(id));
        let mut batches = Vec::new();
        for entity in entities {
            let Some(asset) = Runtime::resolve(entity, assets) else {
                continue;
            };
            let actor = (entity.creator_id, entity.id);
            let mut hash = FxHasher::default();
            crate::SceneHandler::hash_character_items(&mut hash, entity);
            hash.write_usize(Arc::as_ptr(asset) as usize);
            let signature = hash.finish();
            if self
                .plans
                .get(&actor)
                .is_none_or(|p| p.signature != signature)
            {
                let mut plans = vec![Arc::new(asset.variant_plan(entity, assets))];
                for (_, item) in entity.iter_inventory() {
                    let Some(slot) = item.attributes.get_str("slot") else {
                        continue;
                    };
                    if Runtime::resolve_item(item, assets).is_none()
                        && Runtime::resolve_wearable(item, assets).is_none()
                    {
                        continue;
                    }
                    let mut candidate = entity.clone();
                    if item
                        .attributes
                        .get_bool_default("appearance_two_handed", false)
                    {
                        candidate.equipped.shift_remove("off_hand");
                        candidate.equipped.shift_remove("shield");
                    }
                    candidate.equipped.insert(slot.into(), item.clone());
                    plans.push(Arc::new(asset.variant_plan(&candidate, assets)));
                }
                self.plans.insert(actor, PlannedEntity { signature, plans });
            }
            batches.push((actor, asset.clone(), self.plans[&actor].plans.clone()));
        }
        // Current variants have priority over speculative inventory variants.
        for (actor, asset, plans) in &batches {
            self.request(asset, *actor, &plans[0]);
        }
        for (actor, asset, plans) in &batches {
            for plan in &plans[1..] {
                if self.pending.len() >= 2 {
                    return;
                }
                self.request(asset, *actor, plan);
            }
        }
    }
}
