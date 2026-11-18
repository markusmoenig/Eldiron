use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupplyState {
    pub remaining: Option<u32>,
    pub next_recovery_ms: Option<u64>,
    #[serde(default)]
    pub emptied_at: Vec<u64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceState {
    pub version: u32,
    /// Resolved policy is retained to detect edits requiring explicit migration.
    pub policy: ResourceProfile,
    pub phase: String,
    pub created_ms: u64,
    pub settled_ms: u64,
    pub suspended_until_ms: Option<u64>,
    pub supplies: BTreeMap<String, SupplyState>,
    pub revision: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Extraction {
    pub supply: String,
    pub stock_units: u32,
    pub before: Option<u32>,
    pub after: Option<u32>,
    pub outputs: Vec<Output>,
    pub phase: String,
    pub appearance: String,
    pub next_recovery_ms: Option<u64>,
}
impl ResourceState {
    pub fn new(policy: ResourceProfile, now: u64) -> Result<Self, String> {
        let mut supplies = BTreeMap::new();
        for (id, supply) in &policy.supplies {
            let remaining = supply.capacity.map(|c| supply.initial.unwrap_or(c));
            let next = if remaining.zip(supply.capacity).is_some_and(|(n, c)| n < c) {
                supply
                    .recovery
                    .as_ref()
                    .map(|r| {
                        now.checked_add(r.interval_ms)
                            .ok_or("Recovery deadline overflow")
                    })
                    .transpose()?
            } else {
                None
            };
            supplies.insert(
                id.clone(),
                SupplyState {
                    remaining,
                    next_recovery_ms: next,
                    emptied_at: Vec::new(),
                },
            );
        }
        let mut result = Self {
            version: 1,
            phase: policy.initial_phase.clone(),
            policy,
            created_ms: now,
            settled_ms: now,
            suspended_until_ms: None,
            supplies,
            revision: 0,
        };
        if result.policy.spent_when_empty
            && result.supplies.values().all(|s| s.remaining == Some(0))
        {
            result.phase = "spent".into();
        }
        Ok(result)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.created_ms > self.settled_ms
            || self.supplies.len() != self.policy.supplies.len()
        {
            return Err("Invalid saved resource schema or clock".into());
        }
        let valid_phase = self.phase == self.policy.initial_phase
            || matches!(self.phase.as_str(), "expired" | "spent" | "damaged")
            || self
                .policy
                .interactions
                .values()
                .any(|i| i.transition.as_deref() == Some(&self.phase));
        if !valid_phase {
            return Err("Invalid saved resource lifecycle".into());
        }
        if self.suspended_until_ms.is_some_and(|until| {
            until <= self.settled_ms
                || self.phase != "damaged"
                || self.policy.exploitation.is_none()
        }) {
            return Err("Invalid saved resource suspension".into());
        }
        for (name, policy) in &self.policy.supplies {
            let state = self
                .supplies
                .get(name)
                .ok_or("Missing saved resource supply")?;
            match (policy.capacity, state.remaining) {
                (Some(capacity), Some(stock)) if stock <= capacity => {}
                (None, None) => {}
                _ => return Err("Saved resource stock requires explicit reconciliation".into()),
            }
            let recovering = self.phase == self.policy.initial_phase
                && self.suspended_until_ms.is_none()
                && policy.recovery.is_some()
                && state
                    .remaining
                    .zip(policy.capacity)
                    .is_some_and(|(n, c)| n < c);
            if recovering != state.next_recovery_ms.is_some()
                || state
                    .next_recovery_ms
                    .is_some_and(|time| time <= self.settled_ms)
                || state.emptied_at.iter().any(|t| *t > self.settled_ms)
            {
                return Err("Invalid saved resource recovery history".into());
            }
        }
        Ok(())
    }
    pub fn reconcile_policy(&self, policy: &ResourceProfile) -> Result<(), String> {
        let mut previous = self.policy.clone();
        previous.name = policy.name.clone();
        // Initial stock is a creation default, not a live refill instruction.
        for (id, supply) in &mut previous.supplies {
            if let Some(current) = policy.supplies.get(id) {
                supply.initial = current.initial;
            }
        }
        if &previous != policy {
            Err("Resource policy changed; explicit live-state migration is required".into())
        } else {
            Ok(())
        }
    }
    pub fn settle(&mut self, now: u64) -> Result<(), String> {
        if now < self.settled_ms {
            return Err("Resource world clock cannot go backwards".into());
        }
        let mut revisions = 0u64;
        if self
            .policy
            .expires_after_ms
            .is_some_and(|n| now.saturating_sub(self.created_ms) >= n)
        {
            if self.phase != "expired" {
                revisions = revisions
                    .checked_add(1)
                    .ok_or("Resource revision exhausted")?;
            }
            self.phase = "expired".into();
            self.suspended_until_ms = None;
            for state in self.supplies.values_mut() {
                if state.remaining.is_some() {
                    state.remaining = Some(0);
                }
                state.next_recovery_ms = None;
            }
        } else {
            if let Some(until) = self.suspended_until_ms {
                if until <= now {
                    revisions = revisions
                        .checked_add(1)
                        .ok_or("Resource revision exhausted")?;
                    self.suspended_until_ms = None;
                    self.phase = self.policy.initial_phase.clone();
                    for (name, state) in &mut self.supplies {
                        let supply = &self.policy.supplies[name];
                        if state
                            .remaining
                            .zip(supply.capacity)
                            .is_some_and(|(n, c)| n < c)
                        {
                            state.next_recovery_ms = supply
                                .recovery
                                .as_ref()
                                .map(|r| {
                                    until
                                        .checked_add(r.interval_ms)
                                        .ok_or("Recovery deadline overflow")
                                })
                                .transpose()?;
                        }
                    }
                }
            }
            if self.phase == self.policy.initial_phase && self.suspended_until_ms.is_none() {
                for (name, state) in &mut self.supplies {
                    let supply = &self.policy.supplies[name];
                    if let (Some(deadline), Some(stock), Some(capacity), Some(recovery)) = (
                        state.next_recovery_ms,
                        state.remaining,
                        supply.capacity,
                        supply.recovery.as_ref(),
                    ) {
                        if deadline <= now {
                            let intervals = (now - deadline) / recovery.interval_ms + 1;
                            let effective = intervals.min(
                                (u64::from(capacity - stock) + u64::from(recovery.amount) - 1)
                                    / u64::from(recovery.amount),
                            );
                            revisions = revisions
                                .checked_add(effective)
                                .ok_or("Resource revision exhausted")?;
                            let recovered = (u128::from(stock)
                                + u128::from(intervals) * u128::from(recovery.amount))
                            .min(u128::from(capacity))
                                as u32;
                            state.remaining = Some(recovered);
                            state.next_recovery_ms = if recovered == capacity {
                                None
                            } else {
                                Some(
                                    deadline
                                        .checked_add(
                                            intervals
                                                .checked_mul(recovery.interval_ms)
                                                .ok_or("Recovery deadline overflow")?,
                                        )
                                        .ok_or("Recovery deadline overflow")?,
                                )
                            };
                        }
                    }
                }
            }
        }
        self.settled_ms = now;
        self.revision = self
            .revision
            .checked_add(revisions)
            .ok_or("Resource revision exhausted")?;
        Ok(())
    }
    pub fn appearance(&self, supply: &str) -> String {
        if self.phase != self.policy.initial_phase {
            return self.phase.clone();
        }
        match self.supplies.get(supply).and_then(|s| s.remaining) {
            None => "healthy".into(),
            Some(0) => "bare".into(),
            Some(n)
                if self
                    .policy
                    .supplies
                    .get(supply)
                    .and_then(|s| s.healthy_at)
                    .is_some_and(|h| n < h) =>
            {
                "sparse".into()
            }
            _ => "healthy".into(),
        }
    }
    /// Source plan only; the host must validate destination, wear and skill on
    /// prospective actor state before committing either participant.
    pub fn extract(
        &mut self,
        interaction_id: &str,
        requested: u32,
        destination_limit: u32,
        now: u64,
    ) -> Result<Extraction, String> {
        let mut candidate = self.clone();
        candidate.settle(now)?;
        let result = candidate.extract_inner(interaction_id, requested, destination_limit, now)?;
        *self = candidate;
        Ok(result)
    }
    fn extract_inner(
        &mut self,
        interaction_id: &str,
        requested: u32,
        destination_limit: u32,
        now: u64,
    ) -> Result<Extraction, String> {
        if self.phase != self.policy.initial_phase || self.suspended_until_ms.is_some() {
            return Err(format!(
                "Resource lifecycle '{}' does not allow collection",
                self.phase
            ));
        }
        let interaction = self
            .policy
            .interactions
            .get(interaction_id)
            .ok_or("Unknown resource interaction")?
            .clone();
        if requested == 0 || requested > interaction.quantity {
            return Err("Requested quantity is outside the interaction limit".into());
        }
        let supply = self
            .policy
            .supplies
            .get(&interaction.supply)
            .ok_or("Unknown supply")?;
        let state = self
            .supplies
            .get_mut(&interaction.supply)
            .ok_or("Missing persistent supply")?;
        let before = state.remaining;
        let units = match interaction.policy {
            QuantityPolicy::Whole => {
                if requested != interaction.quantity
                    || state.remaining.is_some_and(|s| s < requested)
                    || destination_limit < requested
                {
                    return Err("The complete resource batch is unavailable or does not fit".into());
                }
                requested
            }
            QuantityPolicy::Clamped => requested
                .min(state.remaining.unwrap_or(u32::MAX))
                .min(destination_limit),
        };
        if units == 0 {
            return Err("No positive resource transfer is possible".into());
        }
        if let Some(stock) = state.remaining {
            let capacity = supply.capacity.ok_or("Finite supply has no capacity")?;
            state.remaining = Some(stock - units);
            if stock == capacity {
                state.next_recovery_ms = supply
                    .recovery
                    .as_ref()
                    .map(|r| {
                        now.checked_add(r.interval_ms)
                            .ok_or("Recovery deadline overflow")
                    })
                    .transpose()?;
            }
            if stock > 0 && stock == units {
                if let Some(e) = &self.policy.exploitation {
                    if e.supply == interaction.supply {
                        state
                            .emptied_at
                            .retain(|time| now.saturating_sub(*time) < e.window_ms);
                        state.emptied_at.push(now);
                        if state.emptied_at.len() >= e.empty_count as usize {
                            self.phase = "damaged".into();
                            self.suspended_until_ms = Some(
                                now.checked_add(e.suspension_ms)
                                    .ok_or("Suspension deadline overflow")?,
                            );
                            for state in self.supplies.values_mut() {
                                state.next_recovery_ms = None;
                            }
                        }
                    }
                }
            }
        }
        let outputs = supply
            .outputs
            .iter()
            .map(|o| {
                Ok(Output {
                    item: o.item.clone(),
                    quantity: o
                        .quantity
                        .checked_mul(units)
                        .ok_or("Resource output quantity overflow")?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        if let Some(phase) = interaction.transition {
            self.phase = phase;
        }
        if interaction.discard_supplies {
            for state in self.supplies.values_mut() {
                if state.remaining.is_some() {
                    state.remaining = Some(0);
                }
                state.next_recovery_ms = None;
            }
        }
        if self.policy.spent_when_empty && self.supplies.values().all(|s| s.remaining == Some(0)) {
            self.phase = "spent".into();
        }
        if self.phase != self.policy.initial_phase {
            for state in self.supplies.values_mut() {
                state.next_recovery_ms = None;
            }
        }
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or("Resource revision exhausted")?;
        let state = &self.supplies[&interaction.supply];
        Ok(Extraction {
            supply: interaction.supply.clone(),
            stock_units: units,
            before,
            after: state.remaining,
            outputs,
            phase: self.phase.clone(),
            appearance: self.appearance(&interaction.supply),
            next_recovery_ms: state.next_recovery_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const DAY: u64 = 86_400_000;
    fn profile(id: &str) -> ResourceProfile {
        profiles(
            crate::document::RulesState::default()
                .document()
                .unwrap()
                .table(),
        )
        .unwrap()
        .remove(id)
        .unwrap()
    }
    #[test]
    fn tree_recovery_is_phase_based_and_felling_cancels_it() {
        let mut tree = ResourceState::new(profile("mature_tree"), 0).unwrap();
        for _ in 0..6 {
            tree.extract("collect_branches", 2, 100, 0).unwrap();
        }
        assert_eq!(tree.appearance("branches"), "bare");
        let unloaded = tree.clone();
        tree.settle(DAY).unwrap();
        assert_eq!(tree.supplies["branches"].remaining, Some(4));
        tree.settle(2 * DAY).unwrap();
        assert_eq!(tree.appearance("branches"), "healthy");
        tree.settle(3 * DAY).unwrap();
        let mut catch_up = unloaded;
        catch_up.settle(3 * DAY).unwrap();
        assert_eq!(tree, catch_up);
        tree.settle(10 * DAY).unwrap();
        tree.extract("collect_branches", 2, 100, 10 * DAY).unwrap();
        assert_eq!(tree.supplies["branches"].next_recovery_ms, Some(11 * DAY));
        let outcome = tree.extract("fell_tree", 1, 100, 10 * DAY).unwrap();
        assert_eq!(outcome.outputs[0].quantity, 6);
        tree.settle(20 * DAY).unwrap();
        assert_eq!(tree.phase, "stump");
        assert_eq!(tree.supplies["branches"].remaining, Some(0));
        assert!(tree.extract("collect_branches", 2, 100, 20 * DAY).is_err());
    }
    #[test]
    fn finite_and_unlimited_water_clamp_and_never_bank_recovery() {
        let mut well = ResourceState::new(profile("finite_well"), 0).unwrap();
        assert_eq!(well.extract("fill_water", 4, 4, 0).unwrap().stock_units, 3);
        assert!(well.extract("fill_water", 4, 4, 0).is_err());
        well.settle(DAY / 2).unwrap();
        assert_eq!(
            well.extract("fill_water", 4, 1, DAY / 2)
                .unwrap()
                .stock_units,
            1
        );
        assert_eq!(well.supplies["water"].next_recovery_ms, Some(DAY));
        well.settle(3 * DAY).unwrap();
        assert_eq!(well.supplies["water"].remaining, Some(12));
        assert_eq!(well.supplies["water"].next_recovery_ms, None);
        let mut river = ResourceState::new(profile("river"), 0).unwrap();
        assert_eq!(river.extract("fill_water", 4, 1, 0).unwrap().stock_units, 1);
        assert_eq!(river.supplies["water"].remaining, None);
        let before = river.clone();
        assert!(river.extract("fill_water", 4, 0, 0).is_err());
        assert_eq!(river, before);
    }
    #[test]
    fn remains_are_single_use_and_expiry_wins_at_the_boundary() {
        let mut remains = ResourceState::new(profile("skeleton_remains"), 0).unwrap();
        let output = remains.extract("collect_bones", 1, 1, 0).unwrap();
        assert_eq!(output.outputs[0].quantity, 6);
        assert_eq!(remains.phase, "spent");
        let saved = serde_json::to_string(&remains).unwrap();
        let mut restored: ResourceState = serde_json::from_str(&saved).unwrap();
        assert!(restored.extract("collect_bones", 1, 1, DAY).is_err());
        let mut unspent = ResourceState::new(profile("skeleton_remains"), 0).unwrap();
        unspent.settle(2 * DAY).unwrap();
        assert_eq!(unspent.phase, "expired");
        assert!(unspent.extract("collect_bones", 1, 1, 2 * DAY).is_err());
    }
    #[test]
    fn exploitation_has_no_suspended_recovery_debt() {
        let mut policy = profile("mature_tree");
        policy.exploitation = Some(Exploitation {
            supply: "branches".into(),
            empty_count: 3,
            window_ms: 3 * DAY,
            suspension_ms: 3 * DAY,
        });
        let mut tree = ResourceState::new(policy, 0).unwrap();
        for _ in 0..6 {
            tree.extract("collect_branches", 2, 100, 0).unwrap();
        }
        for day in [1, 2] {
            tree.settle(day * DAY).unwrap();
            for _ in 0..2 {
                tree.extract("collect_branches", 2, 100, day * DAY).unwrap();
            }
        }
        assert_eq!(tree.phase, "damaged");
        tree.settle(5 * DAY).unwrap();
        assert_eq!(tree.supplies["branches"].remaining, Some(0));
        tree.settle(6 * DAY).unwrap();
        assert_eq!(tree.supplies["branches"].remaining, Some(4));
    }
    #[test]
    fn binding_rejects_inline_outputs_pool_overrides_and_implicit_live_migration() {
        let document = crate::document::RulesState::default().document().unwrap();
        let root = document.table();
        assert!(
            binding(
                "[resource]\nprofile='mature_tree'\noutputs=[{item='training_sword',quantity=99}]"
            )
            .is_err()
        );
        let mut b = ResourceBinding {
            profile: "finite_well".into(),
            pool: Some("village_well".into()),
            ..Default::default()
        };
        b.supplies.insert(
            "water".into(),
            SupplyOverride {
                initial: Some(12),
                ..Default::default()
            },
        );
        assert!(resolve(root, &b).is_err());
        let p = profile("mature_tree");
        let mut bark_access = ResourceBinding {
            profile: "mature_tree".into(),
            interaction: Some("collect_bark".into()),
            ..Default::default()
        };
        assert_eq!(resolve(root, &bark_access).unwrap(), p);
        bark_access.interaction = Some("invented_reward".into());
        assert!(resolve(root, &bark_access).is_err());
        let state = ResourceState::new(p.clone(), 0).unwrap();
        let mut changed = p;
        changed.supplies.get_mut("branches").unwrap().capacity = Some(20);
        assert!(state.reconcile_policy(&changed).is_err());
        let bound = bind_source(
            root,
            "[look]\nname='Old Tree'\n",
            ResourceBinding {
                profile: "mature_tree".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(bound.contains("Old Tree"));
        assert!(binding(&bound).unwrap().is_some());
    }
}
