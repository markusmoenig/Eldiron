//! Shared native/WASM action vocabulary and UI conversion. This is not a serde
//! wrapper around EntityAction: identities, callbacks and distances are stripped.
use crate::*;
pub use eldiron_runtime::requests::{
    GameRequest, ItemAddress, ItemDestination, ItemOwner, ItemSource, RequestError, RequestOutcome,
};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameAction {
    pub stream: Uuid,
    pub sequence: u64,
    pub world: Uuid,
    pub generation: u64,
    pub region: Uuid,
    pub request: GameRequest,
}
impl GameAction {
    pub fn bounded(&self) -> bool {
        !self.stream.is_nil()
            && !self.world.is_nil()
            && !self.region.is_nil()
            && self.sequence > 0
            && self.generation > 0
            && self.request.bounded()
    }
    pub fn fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(self).expect("bounded gameplay DTO"))
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionResult {
    Outcome(RequestOutcome),
    AlreadyCommitted,
    Conflict,
    OutOfOrder,
    RateLimited,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionReply {
    pub stream: Uuid,
    pub sequence: u64,
    pub next_sequence: u64,
    pub result: ActionResult,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OfferedChoice {
    pub id: Uuid,
    pub choice: rusterix::Choice,
}

/// Convert trusted UI intent into the narrow wire vocabulary. Incoming network
/// packets never use this adapter and cannot provide arbitrary EntityActions.
pub fn request_from_action(
    action: &rusterix::EntityAction,
    map: &rusterix::Map,
    actor: u32,
    offers: &[OfferedChoice],
    selected_intent: Option<&str>,
) -> Result<GameRequest, RequestError> {
    use rusterix::EntityAction as A;
    let owner = |owner: Option<u32>| match owner {
        None => ItemOwner::World,
        Some(id) if id == actor => ItemOwner::Owned,
        Some(id) => ItemOwner::Corpse(id),
    };
    let direct = |item: u32, provided: Option<u32>| {
        let scope = if provided.is_some() {
            owner(provided)
        } else if map
            .entities
            .iter()
            .find(|e| e.id == actor)
            .is_some_and(|e| {
                e.get_item(item).is_some() || e.equipped.values().any(|i| i.id == item)
            })
        {
            ItemOwner::Owned
        } else {
            ItemOwner::World
        };
        ItemAddress { item, owner: scope }
    };
    let destination = |target: Option<u32>,
                       index: Option<usize>,
                       slot: &Option<String>|
     -> Result<ItemDestination, RequestError> {
        if target.is_some_and(|id| id != actor) || index.is_some() && slot.is_some() {
            return Err(RequestError::Forbidden);
        }
        Ok(if let Some(slot) = slot {
            ItemDestination::Equipment(slot.clone())
        } else {
            ItemDestination::Inventory(index.map(|i| u32::try_from(i).unwrap_or(u32::MAX)))
        })
    };
    let request = match action {
        A::Intent(intent) => GameRequest::Intent(intent.clone()),
        A::ResourceInteract(request) => GameRequest::Resource(request.clone()),
        A::EntityClicked(target, _, intent) => GameRequest::Entity {
            target: *target,
            intent: intent.clone(),
        },
        A::ItemClicked(item, _, intent, provided) => GameRequest::Item {
            target: ItemAddress {
                item: *item,
                owner: owner(*provided),
            },
            intent: intent.clone(),
        },
        A::BlockPropInteract {
            instance_id,
            part_id,
            target_id,
            verb,
            explicit,
        } => GameRequest::Prop {
            instance: *instance_id,
            part: *part_id,
            target: *target_id,
            intent: verb.clone(),
            explicit: *explicit,
        },
        A::TerrainClicked(position) => GameRequest::Terrain {
            position: [position.x, position.y],
            intent: selected_intent.unwrap_or("").to_owned(),
        },
        A::OpenContainer {
            item_id,
            owner_entity_id,
        } => GameRequest::OpenContainer(ItemAddress {
            item: *item_id,
            owner: owner(*owner_entity_id),
        }),
        A::MoveItem {
            item_id,
            owner_entity_id,
            target_entity_id,
            to_inventory_index,
            to_equipped_slot,
        } => GameRequest::Transfer {
            source: ItemSource::Direct(direct(*item_id, *owner_entity_id)),
            destination: destination(*target_entity_id, *to_inventory_index, to_equipped_slot)?,
        },
        A::MoveContainerItem {
            item_id,
            container_item_id,
            container_owner_entity_id,
            target_entity_id,
            to_inventory_index,
            to_equipped_slot,
        } => GameRequest::Transfer {
            source: ItemSource::Container {
                container: ItemAddress {
                    item: *container_item_id,
                    owner: owner(*container_owner_entity_id),
                },
                item: *item_id,
            },
            destination: destination(*target_entity_id, *to_inventory_index, to_equipped_slot)?,
        },
        A::MoveItemToContainer {
            item_id,
            owner_entity_id,
            source_container_item_id,
            source_container_owner_entity_id,
            container_item_id,
            container_owner_entity_id,
        } => GameRequest::Transfer {
            source: if let Some(container) = source_container_item_id {
                ItemSource::Container {
                    container: ItemAddress {
                        item: *container,
                        owner: owner(*source_container_owner_entity_id),
                    },
                    item: *item_id,
                }
            } else {
                ItemSource::Direct(direct(*item_id, *owner_entity_id))
            },
            destination: ItemDestination::Container(ItemAddress {
                item: *container_item_id,
                owner: owner(*container_owner_entity_id),
            }),
        },
        A::DropItemAt {
            item_id,
            owner_entity_id,
            position,
        } => GameRequest::Transfer {
            source: ItemSource::Direct(direct(*item_id, *owner_entity_id)),
            destination: ItemDestination::Ground([position.x, position.y]),
        },
        A::DropContainerItemAt {
            item_id,
            container_item_id,
            container_owner_entity_id,
            position,
        } => GameRequest::Transfer {
            source: ItemSource::Container {
                container: ItemAddress {
                    item: *container_item_id,
                    owner: owner(*container_owner_entity_id),
                },
                item: *item_id,
            },
            destination: ItemDestination::Ground([position.x, position.y]),
        },
        A::Choice(choice) => GameRequest::Choose {
            offer: offers
                .iter()
                .find(|o| o.choice == *choice)
                .ok_or(RequestError::ExpiredOffer)?
                .id,
        },
        A::SetCommandSlot { slot, command } => GameRequest::CommandSlot {
            slot: slot.clone(),
            command: command.clone(),
        },
        _ => return Err(RequestError::Unsupported),
    };
    if !request.bounded() {
        return Err(RequestError::Invalid);
    }
    Ok(request)
}

/// One in-flight action plus a bounded unsent queue. Simulation is never
/// predicted for these requests. Only movement has a prediction history.
#[derive(Default)]
pub struct ActionClient {
    stream: Option<Uuid>,
    next: u64,
    ready: bool,
    frame_ready: bool,
    context: Option<(Uuid, u64, Uuid)>,
    queue: std::collections::VecDeque<GameRequest>,
    inflight: Option<GameAction>,
    offers: Vec<OfferedChoice>,
    since_send: std::time::Duration,
    retry_after: std::time::Duration,
}
impl ActionClient {
    pub fn disconnect(&mut self) {
        self.ready = false;
        self.frame_ready = false;
        self.context = None;
        self.queue.clear();
        self.offers.clear();
    }
    pub fn stream_ready(&mut self, stream: Uuid, next: u64) {
        if stream.is_nil() || next == 0 {
            return;
        }
        if self.stream != Some(stream) {
            *self = Self::default();
        }
        self.stream = Some(stream);
        self.next = next;
        self.ready = true;
        if self.inflight.as_ref().is_some_and(|a| a.sequence < next) {
            self.inflight = None;
        }
        self.since_send = std::time::Duration::MAX;
        self.retry_after = std::time::Duration::ZERO;
    }
    fn context(&mut self, world: Uuid, generation: u64, region: Option<Uuid>) {
        let context = region.map(|r| (world, generation, r));
        if self.context != context {
            self.queue.clear();
            self.offers.clear();
            self.frame_ready = false;
            self.context = context;
        }
    }
    pub fn observe_frame(&mut self, frame: &ClientFrame) {
        self.context(frame.world, frame.generation, frame.current_region);
        self.frame_ready = frame.controlled_entity.is_some() && frame.current_region.is_some();
    }
    pub fn offers(
        &mut self,
        world: Uuid,
        generation: u64,
        region: Option<Uuid>,
        offers: Vec<OfferedChoice>,
    ) {
        self.context(world, generation, region);
        self.offers = offers.into_iter().take(256).collect();
    }
    pub fn enqueue(&mut self, request: GameRequest) -> Result<(), RequestError> {
        if !self.ready || !self.frame_ready || self.context.is_none() {
            return Err(RequestError::NotControlled);
        }
        if self.queue.len() >= 64 || !request.bounded() {
            return Err(RequestError::Invalid);
        }
        self.queue.push_back(request);
        Ok(())
    }
    pub fn enqueue_action(
        &mut self,
        action: &rusterix::EntityAction,
        map: &rusterix::Map,
        actor: u32,
        intent: Option<&str>,
    ) -> Result<(), RequestError> {
        let request = request_from_action(action, map, actor, &self.offers, intent)?;
        self.enqueue(request)
    }
    pub fn advance(&mut self, elapsed: std::time::Duration) -> Option<ClientMessage> {
        if !self.ready {
            return None;
        }
        self.since_send = self.since_send.saturating_add(elapsed);
        if let Some(action) = &self.inflight {
            if self.since_send < self.retry_after {
                return None;
            }
            self.since_send = std::time::Duration::ZERO;
            self.retry_after = std::time::Duration::from_millis(750);
            return Some(ClientMessage::Action(action.clone()));
        }
        if !self.frame_ready {
            return None;
        }
        let (world, generation, region) = self.context?;
        let request = self.queue.pop_front()?;
        let action = GameAction {
            stream: self.stream?,
            sequence: self.next,
            world,
            generation,
            region,
            request,
        };
        self.inflight = Some(action.clone());
        self.since_send = std::time::Duration::ZERO;
        self.retry_after = std::time::Duration::from_millis(750);
        Some(ClientMessage::Action(action))
    }
    /// Duplicated replies cannot produce another UI notification or effect.
    pub fn reply(&mut self, reply: &ActionReply) -> Option<ActionResult> {
        if self.stream != Some(reply.stream)
            || !self
                .inflight
                .as_ref()
                .is_some_and(|a| a.sequence == reply.sequence)
        {
            return None;
        }
        if reply.result == ActionResult::RateLimited {
            self.since_send = std::time::Duration::ZERO;
            self.retry_after = std::time::Duration::from_secs(1);
            return None;
        }
        self.inflight = None;
        self.next = reply.next_sequence;
        if matches!(
            reply.result,
            ActionResult::Conflict | ActionResult::OutOfOrder
        ) {
            self.queue.clear();
        }
        Some(reply.result.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    fn client() -> ActionClient {
        let mut client = ActionClient::default();
        client.stream_ready(Uuid::new_v4(), 1);
        client.context(Uuid::new_v4(), 1, Some(Uuid::new_v4()));
        client.frame_ready = true;
        client
    }
    fn action(message: Option<ClientMessage>) -> GameAction {
        let Some(ClientMessage::Action(action)) = message else {
            panic!("action expected")
        };
        action
    }
    #[test]
    fn retries_reuse_identity_rate_limit_backs_off_and_duplicate_ack_is_ignored() {
        let mut client = client();
        client
            .enqueue(GameRequest::Intent("attack".into()))
            .unwrap();
        client.enqueue(GameRequest::Intent("talk".into())).unwrap();
        let first = action(client.advance(Duration::ZERO));
        assert!(client.advance(Duration::from_millis(749)).is_none());
        assert_eq!(action(client.advance(Duration::from_millis(1))), first);
        let mut reply = ActionReply {
            stream: first.stream,
            sequence: 1,
            next_sequence: 1,
            result: ActionResult::RateLimited,
        };
        assert!(client.reply(&reply).is_none());
        assert!(client.advance(Duration::from_millis(999)).is_none());
        assert_eq!(action(client.advance(Duration::from_millis(1))), first);
        reply.result = ActionResult::Outcome(RequestOutcome::Processed);
        reply.next_sequence = 2;
        assert_eq!(client.reply(&reply), Some(reply.result.clone()));
        assert!(client.reply(&reply).is_none());
        assert_eq!(action(client.advance(Duration::ZERO)).sequence, 2);
    }
    #[test]
    fn reconnect_and_baselines_preserve_unacknowledged_identity_but_discard_unsent_context() {
        let mut client = client();
        client
            .enqueue(GameRequest::Intent("attack".into()))
            .unwrap();
        client.enqueue(GameRequest::Intent("talk".into())).unwrap();
        let first = action(client.advance(Duration::ZERO));
        client.context(Uuid::new_v4(), 2, Some(Uuid::new_v4()));
        assert!(client.queue.is_empty());
        client.disconnect();
        assert!(client.advance(Duration::MAX).is_none());
        client.stream_ready(first.stream, 1);
        assert_eq!(
            action(client.advance(Duration::ZERO)),
            first,
            "never reuse a pending sequence for a different command/context"
        );
        client.disconnect();
        client.stream_ready(first.stream, 2);
        assert!(
            client.inflight.is_none(),
            "durable server watermark resolves a lost reply"
        );
        client.context(first.world, 3, Some(first.region));
        client.frame_ready = true;
        client.enqueue(GameRequest::Intent("talk".into())).unwrap();
        assert_eq!(action(client.advance(Duration::ZERO)).sequence, 2);
    }
    #[test]
    fn ui_conversion_strips_distance_rejects_foreign_actor_and_requires_exact_offer() {
        use rusterix::EntityAction as A;
        let map = rusterix::Map::default();
        assert_eq!(
            request_from_action(
                &A::EntityClicked(2, 10000., Some("attack".into())),
                &map,
                1,
                &[],
                None
            )
            .unwrap(),
            GameRequest::Entity {
                target: 2,
                intent: Some("attack".into())
            }
        );
        let steal = A::MoveItem {
            item_id: 10,
            owner_entity_id: Some(1),
            target_entity_id: Some(2),
            to_inventory_index: None,
            to_equipped_slot: None,
        };
        assert_eq!(
            request_from_action(&steal, &map, 1, &[], None),
            Err(RequestError::Forbidden)
        );
        let choice =
            rusterix::Choice::ScriptChoice("Gift".into(), "grant".into(), 2, 1, 0, 100, 3.);
        assert_eq!(
            request_from_action(&A::Choice(choice.clone()), &map, 1, &[], None),
            Err(RequestError::ExpiredOffer)
        );
        let offer = OfferedChoice {
            id: Uuid::new_v4(),
            choice: choice.clone(),
        };
        assert_eq!(
            request_from_action(&A::Choice(choice), &map, 1, &[offer.clone()], None).unwrap(),
            GameRequest::Choose { offer: offer.id }
        );
        let forged =
            rusterix::Choice::ScriptChoice("Gift".into(), "grant_admin".into(), 2, 1, 0, 100, 3.);
        assert_eq!(
            request_from_action(&A::Choice(forged), &map, 1, &[offer], None),
            Err(RequestError::ExpiredOffer)
        );
    }
    #[test]
    fn codec_roundtrip_and_bounds_reject_invalid_payloads_and_unbounded_queues() {
        let mut client = client();
        for _ in 0..64 {
            client.enqueue(GameRequest::Intent("look".into())).unwrap();
        }
        assert_eq!(
            client.enqueue(GameRequest::Intent("look".into())),
            Err(RequestError::Invalid)
        );
        let action = action(client.advance(Duration::ZERO));
        let bytes = codec::encode_client(&ClientMessage::Action(action.clone())).unwrap();
        assert!(
            matches!(codec::decode_client(&bytes).unwrap(),ClientMessage::Action(a) if a==action)
        );
        let mut invalid = action;
        invalid.request = GameRequest::Intent("x".repeat(257));
        assert!(
            codec::decode_client(
                &codec::encode_client(&ClientMessage::Action(invalid.clone())).unwrap()
            )
            .is_err()
        );
        invalid.request = GameRequest::Terrain {
            position: [f32::NAN, 0.],
            intent: "attack".into(),
        };
        assert!(
            codec::decode_client(&codec::encode_client(&ClientMessage::Action(invalid)).unwrap())
                .is_err()
        );
        assert!(
            serde_json::from_value::<GameRequest>(
                serde_json::json!({"Choose":{"offer":Uuid::new_v4(),"event":"grant","price":-100}})
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<GameRequest>(
                serde_json::json!({"Entity":{"target":1,"actor":2,"distance":0}})
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod context_tests {
    use super::*;
    #[test]
    fn offers_cannot_make_a_future_context_ready_before_its_authoritative_frame() {
        let mut client = ActionClient::default();
        let world = Uuid::new_v4();
        let region = Uuid::new_v4();
        client.stream_ready(Uuid::new_v4(), 1);
        client.offers(world, 2, Some(region), vec![]);
        assert_eq!(
            client.enqueue(GameRequest::Intent("attack".into())),
            Err(RequestError::NotControlled)
        );
        let frame = ClientFrame {
            version: 1,
            world,
            connection: Uuid::new_v4(),
            generation: 2,
            sequence: 1,
            tick: 0,
            movement_tick: 0,
            motion: None,
            controlled_entity: Some(1),
            current_region: Some(region),
            state: rusterix::ServerState::Running,
            update: eldiron_runtime::replication::WorldUpdate::Baseline(vec![]),
            events: vec![],
        };
        client.observe_frame(&frame);
        client
            .enqueue(GameRequest::Intent("attack".into()))
            .unwrap();
        client.offers(world, 3, Some(region), vec![]);
        assert!(client.advance(std::time::Duration::MAX).is_none());
        assert_eq!(
            client.enqueue(GameRequest::Intent("talk".into())),
            Err(RequestError::NotControlled)
        );
    }
}
