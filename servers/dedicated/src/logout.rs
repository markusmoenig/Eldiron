//! Project-owned logout policy. Disconnected bodies remain vulnerable until
//! both the departure grace and the last combat grace have elapsed.
use super::*;
use rusterix::{Entity, EntityAction, Value};

#[derive(Clone, Copy, Default)]
pub(super) enum LogoutPolicy {
    #[default]
    RetainBody,
    SafeDespawn {
        grace_ms: i64,
    },
}
pub(super) fn configuration(project: &Project) -> Result<LogoutPolicy, SaveError> {
    let config = project.config.parse::<toml::Table>().map_err(failure)?;
    let Some(table) = config.get("multiplayer").and_then(|v| v.get("logout")) else {
        return Ok(LogoutPolicy::default());
    };
    let table = table
        .as_table()
        .ok_or_else(|| failure("[multiplayer.logout] must be a table"))?;
    if table
        .keys()
        .any(|k| !matches!(k.as_str(), "policy" | "grace_seconds"))
    {
        return Err(failure("Unknown multiplayer.logout setting"));
    }
    let grace = table
        .get("grace_seconds")
        .map(|v| {
            v.as_integer()
                .filter(|v| (10..=3600).contains(v))
                .ok_or_else(|| failure("Logout grace_seconds must be 10..3600"))
        })
        .transpose()?
        .unwrap_or(30);
    match table
        .get("policy")
        .map(toml::Value::as_str)
        .unwrap_or(Some("retain_body"))
    {
        Some("retain_body") => Ok(LogoutPolicy::RetainBody),
        Some("safe_despawn") => Ok(LogoutPolicy::SafeDespawn {
            grace_ms: grace * 1000,
        }),
        _ => Err(failure("Logout policy must be retain_body or safe_despawn")),
    }
}
fn timestamp(entity: &Entity, key: &str) -> Option<i64> {
    match entity.attributes.get(key) {
        Some(Value::Int64(v)) => Some(*v),
        _ => None,
    }
}
fn targets(entity: &Entity, id: u32) -> bool {
    matches!(entity.action, EntityAction::FollowAttack(target, ..) if target == id)
        || ["attack_target", "target"]
            .iter()
            .any(|key| match entity.attributes.get(key) {
                Some(Value::UInt(target)) => *target == id,
                Some(Value::Int(target)) => *target >= 0 && *target as u32 == id,
                Some(Value::Int64(target)) => *target >= 0 && *target as u64 == id as u64,
                Some(Value::Str(target)) => target.parse::<u32>().ok() == Some(id),
                _ => false,
            })
}
impl Host {
    pub(super) fn initialize_logout(&self) {
        if !matches!(self.logout, LogoutPolicy::SafeDespawn { .. }) {
            return;
        }
        let World::Saved(session) = &self.world else {
            return;
        };
        for instance in &session.game.server.instances {
            if let Some(ctx) = instance.lock().unwrap().context() {
                let mut ctx = ctx.lock().unwrap();
                let now = ctx.world.elapsed_ms();
                for entity in &mut ctx.map.entities {
                    if session
                        .saved()
                        .characters
                        .iter()
                        .any(|c| c.entity == entity.id)
                    {
                        // A restart grants no offline time or instant escape.
                        entity.set_attribute("__disconnected_at_ms", Value::Int64(now));
                    }
                }
            }
        }
    }
    pub(super) fn mark_disconnected(&mut self, connection: PlayerConnectionId) {
        if !matches!(self.logout, LogoutPolicy::SafeDespawn { .. })
            || !matches!(self.world, World::Saved(_))
        {
            return;
        }
        let Some(control) = self.world.game().server.player_control(connection) else {
            return;
        };
        for instance in &self.world.game().server.instances {
            if let Some(ctx) = instance.lock().unwrap().context() {
                let mut ctx = ctx.lock().unwrap();
                let now = ctx.world.elapsed_ms();
                if let Some(entity) = ctx
                    .map
                    .entities
                    .iter_mut()
                    .find(|e| e.id == control.entity_id)
                {
                    entity.set_attribute("__disconnected_at_ms", Value::Int64(now));
                }
            }
        }
    }
    pub(super) async fn settle_logout(&mut self) -> Result<(), SaveError> {
        let LogoutPolicy::SafeDespawn { grace_ms } = self.logout else {
            return Ok(());
        };
        let World::Saved(session) = &mut self.world else {
            return Ok(());
        };
        let active: std::collections::HashSet<_> = self
            .peers
            .values()
            .filter_map(|p| p.connection)
            .filter_map(|id| session.game.server.player_control(id))
            .map(|c| c.entity_id)
            .collect();
        let mut changed = false;
        for instance in &session.game.server.instances {
            if let Some(ctx) = instance.lock().unwrap().context() {
                let mut ctx = ctx.lock().unwrap();
                let now = ctx.world.elapsed_ms();
                let eligible: Vec<_> = ctx
                    .map
                    .entities
                    .iter()
                    .filter(|entity| {
                        !active.contains(&entity.id)
                            && entity.get_mode() != "dead"
                            && session
                                .saved()
                                .characters
                                .iter()
                                .any(|c| c.entity == entity.id)
                            && timestamp(entity, "__disconnected_at_ms").is_some_and(|left| {
                                now.saturating_sub(
                                    left.max(timestamp(entity, "__last_combat_ms").unwrap_or(0)),
                                ) >= grace_ms
                            })
                            && !ctx.map.entities.iter().any(|other| {
                                other.id != entity.id
                                    && other.get_mode() != "dead"
                                    && targets(other, entity.id)
                            })
                    })
                    .map(|e| e.id)
                    .collect();
                for id in eligible {
                    if let Some(entity) = ctx.map.entities.iter_mut().find(|e| e.id == id) {
                        entity.set_attribute("__logout_hidden", Value::Bool(true));
                    }
                    changed |= ctx.stash_entity(id);
                }
            }
        }
        if changed {
            session.checkpoint().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn project() -> Project {
        let mut p = Project::new();
        p.regions[0].name = "Logout".into();
        p.regions[0].map.name = "Logout".into();
        p.config="[game]\nstart_region='Logout'\nauto_create_player=false\n[ruleset]\nsource='project'\n[multiplayer.logout]\npolicy='safe_despawn'\ngrace_seconds=10\n[multiplayer.simulation]\npause_unoccupied_regions=['Logout']\nidle_after_ms=20".into();
        p.rules_document.current.clear();
        p.rules_document.original.clear();
        p.preview_workbench_rules();
        let mut template = shared::character::Character::new();
        template.name = "Hero".into();
        template.data = "[attributes]\nplayer=true\ninventory_slots=2".into();
        let mut instance = shared::character::Character::new();
        instance.character_id = template.id;
        p.regions[0].characters.insert(instance.id, instance);
        p.characters.insert(template.id, template);
        p
    }
    async fn peer(host: &mut Host) -> (Uuid, mpsc::Receiver<Vec<u8>>) {
        let id = Uuid::new_v4();
        let (output, rx) = mpsc::channel(64);
        host.command(Command::Open { peer: id, output })
            .await
            .unwrap();
        host.command(Command::Message {
            peer: id,
            message: ClientMessage::Hello {
                version: PROTOCOL_VERSION,
            },
        })
        .await
        .unwrap();
        (id, rx)
    }
    fn context(host: &mut Host) -> Arc<std::sync::Mutex<rusterix::server::regionctx::RegionCtx>> {
        host.world.game().server.instances[0]
            .lock()
            .unwrap()
            .context()
            .unwrap()
    }
    async fn advance(host: &mut Host, count: usize) {
        for _ in 0..count {
            host.tick(Duration::from_millis(250)).await.unwrap();
        }
    }
    #[tokio::test]
    async fn combat_grace_and_targeting_delay_safe_despawn_and_rejoin_retains_body() {
        let root = std::env::temp_dir().join(format!("eldiron-logout-{}", Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("world.save");
        let p = project();
        let mut host = Host::open(p.clone(), Some(path.clone())).await.unwrap();
        let (a, mut rx) = peer(&mut host).await;
        host.command(Command::Register {
            peer: a,
            username: "tester".into(),
            hash: crate::auth::hash("logout testing password").unwrap(),
        })
        .await
        .unwrap();
        let account = host.peers[&a].account.unwrap();
        while rx.try_recv().is_ok() {}
        host.command(Command::Message {
            peer: a,
            message: ClientMessage::CreateCharacter {
                request: Uuid::new_v4(),
                name: "Hero".into(),
            },
        })
        .await
        .unwrap();
        let World::Saved(saved) = &host.world else {
            panic!()
        };
        let record = saved.saved().characters[0].clone();
        let receipts = saved.saved().requests.len();
        host.command(Command::Close(a)).await.unwrap();
        advance(&mut host, 39).await;
        assert!(host.world.game().server.region_schedule().iter().all(|r| !r.suspended),"logout grace keeps combat simulation awake");
        let ctx = context(&mut host);
        {
            let mut ctx = ctx.lock().unwrap();
            let now = ctx.world.elapsed_ms();
            ctx.map
                .entities
                .iter_mut()
                .find(|e| e.id == record.entity)
                .unwrap()
                .set_attribute("__last_combat_ms", Value::Int64(now));
        }
        advance(&mut host, 8).await;
        assert!(
            ctx.lock().unwrap().is_entity_present(record.entity),
            "combat extends the initial departure grace"
        );
        let enemy = {
            let mut ctx = ctx.lock().unwrap();
            let mut enemy = Entity::default();
            enemy.id = ctx.world.next_id();
            enemy.set_attribute("target", Value::UInt(record.entity));
            let id = enemy.id;
            ctx.map.entities.push(enemy);
            id
        };
        advance(&mut host, 40).await;
        assert!(
            ctx.lock().unwrap().is_entity_present(record.entity),
            "active targeting prevents hiding"
        );
        ctx.lock()
            .unwrap()
            .map
            .entities
            .iter_mut()
            .find(|e| e.id == enemy)
            .unwrap()
            .attributes
            .remove("target");
        advance(&mut host, 1).await;
        assert!(ctx.lock().unwrap().is_entity_stashed(record.entity));
        assert!(!ctx.lock().unwrap().is_entity_dead_ctx(record.entity));
        host.shutdown().await.unwrap();
        drop(host);
        let mut host = Host::open(p, Some(path)).await.unwrap();
        let (b, _rx) = peer(&mut host).await;
        host.command(Command::Verified {
            peer: b,
            account,
            security_revision: 0,
        })
        .await
        .unwrap();
        host.command(Command::Message {
            peer: b,
            message: ClientMessage::JoinCharacter {
                character: record.id,
            },
        })
        .await
        .unwrap();
        let ctx = context(&mut host);
        let ctx = ctx.lock().unwrap();
        assert!(ctx.is_entity_present(record.entity));
        assert!(!ctx.is_entity_stashed(record.entity));
        assert_eq!(
            ctx.map
                .entities
                .iter()
                .filter(|e| e.id == record.entity)
                .count(),
            1
        );
        drop(ctx);
        let World::Saved(saved) = &host.world else {
            panic!()
        };
        assert_eq!(saved.saved().characters.len(), 1);
        assert_eq!(saved.saved().requests.len(), receipts);
        host.shutdown().await.unwrap();
        drop(host);
        std::fs::remove_dir_all(root).unwrap();
    }
}
