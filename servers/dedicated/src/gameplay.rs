use super::*;
use eldiron_runtime::{
    persistence::{ActionJournal, SaveError},
    requests::RequestOutcome,
};
fn apply(
    game: &mut GameSession,
    conn: PlayerConnectionId,
    request: &GameAction,
    offered: Option<&rusterix::Choice>,
) -> Result<serde_json::Value, SaveError> {
    let outcome = match game.prepare_game_request(
        conn,
        request.world,
        request.generation,
        request.region,
        &request.request,
        offered,
    ) {
        Ok(actions) => {
            for action in actions {
                game.connected_player_action(conn, action)
                    .map_err(failure)?;
            }
            for _ in 0..32 {
                let update = game.poll();
                if let Some(error) = update.replication_error {
                    return Err(failure(error));
                }
                if game.server.persistence_ready() {
                    break;
                }
            }
            if !game.server.persistence_ready() {
                return Err(failure("Gameplay did not settle"));
            }
            RequestOutcome::Processed
        }
        Err(error) => RequestOutcome::Denied(error),
    };
    serde_json::to_value(outcome).map_err(failure)
}
impl Host {
    pub(super) fn open_action_stream(
        &mut self,
        connection: PlayerConnectionId,
    ) -> Result<(), SaveError> {
        let Some(peer) = self
            .peers
            .iter()
            .find_map(|(id, p)| (p.connection == Some(connection)).then_some(*id))
        else {
            return Ok(());
        };
        let stream = match &self.world {
            World::Demo(_) => connection.uuid(),
            World::Saved(session) => {
                let body = session
                    .game
                    .server
                    .player_control(connection)
                    .ok_or_else(|| failure("No selected body"))?
                    .entity_id;
                session
                    .saved()
                    .characters
                    .iter()
                    .find(|c| c.entity == body && c.account == self.peers[&peer].account)
                    .ok_or_else(|| failure("Character ownership missing"))?
                    .id
            }
        };
        let next = match &self.world {
            World::Demo(_) => 1,
            World::Saved(session) => session
                .saved()
                .action_streams
                .get(&stream)
                .map_or(Ok(1), ActionJournal::next)?,
        };
        let state = self.peers.get_mut(&peer).unwrap();
        state.stream = Some(stream);
        state.journal = ActionJournal::default();
        state.offers.clear();
        state.offer_context = None;
        self.send(
            peer,
            ServerMessage::ActionStream {
                stream,
                next_sequence: next,
            },
        );
        Ok(())
    }
    pub(super) async fn game_action(
        &mut self,
        peer: PeerId,
        action: GameAction,
    ) -> Result<(), SaveError> {
        if !action.bounded() {
            self.reject(peer, RejectCode::InvalidRequest);
            return Ok(());
        }
        let Some(state) = self.peers.get_mut(&peer) else {
            return Ok(());
        };
        let Some(connection) = state.connection else {
            self.reject(peer, RejectCode::AuthenticationRequired);
            return Ok(());
        };
        if state.stream != Some(action.stream) {
            self.reject(peer, RejectCode::CharacterUnavailable);
            return Ok(());
        }
        let journal = match &self.world {
            World::Demo(_) => &state.journal,
            World::Saved(session) => session
                .action_journal(action.stream)
                .unwrap_or(&state.journal),
        };
        let next = journal.next()?;
        let fingerprint = action.fingerprint();
        let cached = journal.check(action.sequence, &fingerprint);
        if state.action_window.0.elapsed() > Duration::from_secs(1) {
            state.action_window = (Instant::now(), 0);
        }
        // Replays can return receipts without reserving a new simulation action.
        if matches!(cached, Ok(None)) && state.action_window.1 >= 8 {
            self.metrics.rate_rejections = self.metrics.rate_rejections.saturating_add(1);
            self.send(
                peer,
                ServerMessage::ActionReply(ActionReply {
                    stream: action.stream,
                    sequence: action.sequence,
                    next_sequence: next,
                    result: ActionResult::RateLimited,
                }),
            );
            return Ok(());
        }
        let result = match cached {
            Ok(Some(outcome)) => Ok(outcome),
            Err(error) => Err(error),
            Ok(None) => {
                state.action_window.1 += 1;
                let offered = match &action.request {
                    GameRequest::Choose { offer } => {
                        state.offers.get(offer).map(|entry| entry.choice.clone())
                    }
                    _ => None,
                };
                match &mut self.world {
                    World::Demo(game) => {
                        let value = apply(game, connection, &action, offered.as_ref())?;
                        self.peers.get_mut(&peer).unwrap().journal.append(
                            action.sequence,
                            fingerprint,
                            value.clone(),
                        )?;
                        Ok(value)
                    }
                    World::Saved(session) => {
                        // The owner has exclusive world access until both effects
                        // and receipt commit. No action output escapes on failure.
                        session
                            .commit_sequenced_live(
                                action.stream,
                                action.sequence,
                                &fingerprint,
                                |game| apply(game, connection, &action, offered.as_ref()),
                            )
                            .await
                    }
                }
            }
        };
        let outcome = match result {
            Ok(value) => ActionResult::Outcome(serde_json::from_value(value).map_err(failure)?),
            Err(SaveError::AlreadyCommitted) => ActionResult::AlreadyCommitted,
            Err(SaveError::Conflict) => ActionResult::Conflict,
            Err(SaveError::ActionSequence { .. }) => ActionResult::OutOfOrder,
            Err(error) => return Err(error),
        };
        let next = match &self.world {
            World::Demo(_) => self.peers[&peer].journal.next()?,
            World::Saved(session) => session
                .action_journal(action.stream)
                .map_or(Ok(1), ActionJournal::next)?,
        };
        self.send(
            peer,
            ServerMessage::ActionReply(ActionReply {
                stream: action.stream,
                sequence: action.sequence,
                next_sequence: next,
                result: outcome,
            }),
        );
        self.flush_frames()
    }
    pub(super) fn publish_offers(&mut self, peer: PeerId, frame: &eldiron_runtime::ClientFrame) {
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        let Some(conn) = state.connection else {
            return;
        };
        let context = (frame.world, frame.generation, frame.current_region);
        let reset = state.offer_context != Some(context);
        let mut offers = std::mem::take(&mut state.offers);
        if reset {
            offers.clear();
        }
        let before = offers.len();
        offers.retain(|_, offered| {
            self.world
                .game()
                .server
                .player_choice_is_active(conn, &offered.choice)
        });
        let mut changed = reset || before != offers.len();
        for delivery in &frame.events {
            if Some(delivery.region) != frame.current_region {
                continue;
            }
            for group in &delivery.events.choices {
                if Some(group.to) != frame.controlled_entity {
                    continue;
                }
                offers.retain(|_, entry| entry.choice.session_meta().0 != group.from);
                let cancel = rusterix::Choice::Cancel(
                    group.from,
                    group.to,
                    group.expires_at_tick,
                    group.max_distance,
                );
                for choice in group.choices.iter().chain(std::iter::once(&cancel)) {
                    if offers.len() >= 256 {
                        offers.clear();
                        break;
                    }
                    if self
                        .world
                        .game()
                        .server
                        .player_choice_is_active(conn, choice)
                    {
                        let id = Uuid::new_v4();
                        offers.insert(
                            id,
                            OfferedChoice {
                                id,
                                choice: choice.clone(),
                            },
                        );
                    }
                }
                changed = true;
            }
        }
        if let Some(state) = self.peers.get_mut(&peer) {
            state.offers = offers;
            state.offer_context = Some(context);
        }
        if changed {
            let offers = self
                .peers
                .get(&peer)
                .map(|p| p.offers.values().cloned().collect())
                .unwrap_or_default();
            self.send(
                peer,
                ServerMessage::Offers {
                    world: frame.world,
                    generation: frame.generation,
                    region: frame.current_region,
                    offers,
                },
            );
        }
    }
}
