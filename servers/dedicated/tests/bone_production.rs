//! Supplied world-to-armor chain through the bounded multiplayer codec.
use eldiron_network::*;
use eldiron_runtime::{ClientFrame, Project};
use eldiron_server::host::{Command, Host};
use shared::rulesets::equipment_family::BONE_PIECES;
use std::time::Duration;
use theframework_core::Uuid;
use tokio::sync::mpsc;

async fn message(host: &mut Host, peer: Uuid, message: ClientMessage) {
    // Exercise the actual inbound codec, not just an in-memory engine action.
    let bytes = codec::encode_client(&message).unwrap();
    host.command(Command::Message {
        peer,
        message: codec::decode_client(&bytes).unwrap(),
    })
    .await
    .unwrap();
}
fn drain(rx: &mut mpsc::Receiver<Vec<u8>>) -> Vec<ServerMessage> {
    let mut messages = Vec::new();
    while let Ok(bytes) = rx.try_recv() {
        messages.push(codec::decode_server(&bytes).unwrap());
    }
    messages
}
async fn join(host: &mut Host, name: &str) -> (Uuid, mpsc::Receiver<Vec<u8>>, Uuid, ClientFrame) {
    let peer = Uuid::new_v4();
    let (output, mut rx) = mpsc::channel(64);
    host.command(Command::Open { peer, output }).await.unwrap();
    message(
        host,
        peer,
        ClientMessage::Hello {
            version: PROTOCOL_VERSION,
        },
    )
    .await;
    message(host, peer, ClientMessage::Guest { name: name.into() }).await;
    host.tick(Duration::from_millis(20)).await.unwrap();
    let messages = drain(&mut rx);
    let stream = messages
        .iter()
        .find_map(|m| {
            if let ServerMessage::ActionStream { stream, .. } = m {
                Some(*stream)
            } else {
                None
            }
        })
        .unwrap();
    let frame = messages
        .into_iter()
        .filter_map(|m| {
            if let ServerMessage::Frame { frame, .. } = m {
                Some(frame)
            } else {
                None
            }
        })
        .last()
        .unwrap();
    (peer, rx, stream, frame)
}
fn actor(host: &mut Host, id: u32) -> rusterix::Entity {
    let ctx = host.world.game().server.instances[0]
        .lock()
        .unwrap()
        .context()
        .unwrap();
    let actor = ctx
        .lock()
        .unwrap()
        .map
        .entities
        .iter()
        .find(|e| e.id == id)
        .unwrap()
        .clone();
    actor
}
fn assert_processed(messages: &[ServerMessage], sequence: u64) {
    assert!(messages.iter().any(|m| matches!(m, ServerMessage::ActionReply(reply) if reply.sequence == sequence && reply.result == ActionResult::Outcome(RequestOutcome::Processed))), "{messages:?}");
}

use rusterix::{
    Entity, GeometryObject, Item, Value,
    server::region::resources::{self, ResourceRequest},
};
use shared::rulesets::world_resources::{ResourceBinding, bind_source};

struct Crafter {
    host: Host,
    peer: Uuid,
    rx: mpsc::Receiver<Vec<u8>>,
    stream: Uuid,
    frame: ClientFrame,
    actor: u32,
    sequence: u64,
}
impl Crafter {
    async fn request(&mut self, request: GameRequest) {
        // Stay below the production eight-action budget; retries do not count.
        if self.sequence > 1 && (self.sequence - 1) % 6 == 0 {
            tokio::time::sleep(Duration::from_millis(1100)).await;
        }
        let action = GameAction {
            stream: self.stream,
            sequence: self.sequence,
            world: self.frame.world,
            generation: self.frame.generation,
            region: self.frame.current_region.unwrap(),
            request,
        };
        message(
            &mut self.host,
            self.peer,
            ClientMessage::Action(action.clone()),
        )
        .await;
        assert_processed(&drain(&mut self.rx), self.sequence);
        message(&mut self.host, self.peer, ClientMessage::Action(action)).await;
        assert_processed(&drain(&mut self.rx), self.sequence);
        self.sequence += 1;
    }
    async fn craft(&mut self, action: &str) {
        self.request(GameRequest::Entity {
            target: self.actor,
            intent: Some(format!("action:{action}")),
        })
        .await;
    }
    async fn collect(
        &mut self,
        access: Uuid,
        interaction: &str,
        quantity: u32,
        tool: Option<u32>,
        vessel: Option<u32>,
    ) {
        self.request(GameRequest::Resource(ResourceRequest {
            access,
            interaction: interaction.into(),
            quantity,
            tool,
            vessel,
        }))
        .await;
    }
    fn item(&mut self, name: &str) -> Item {
        let actor = actor(&mut self.host, self.actor);
        actor
            .inventory
            .into_iter()
            .flatten()
            .chain(actor.equipped.into_values())
            .find(|i| i.attributes.get_str("ruleset_id") == Some(name))
            .unwrap_or_else(|| panic!("Missing {name}"))
    }
}
#[tokio::test]
async fn supplied_production_and_repair_preserve_ownership_and_retries_never_duplicate_changes() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_projects/3DStarter.eldiron");
    let mut project: Project =
        shared::project_io::decode_project(&std::fs::read(path).unwrap()).unwrap();
    // The released starter owns its older rules; this fixture opts into current supplied content.
    project.rules_document = Default::default();
    let mut host = Host::open(project, None).await.unwrap();
    let (peer, rx, stream, frame) = join(&mut host, "World crafter").await;
    let body = frame.controlled_entity.unwrap();
    let context = host.world.game().server.instances[0]
        .lock()
        .unwrap()
        .context()
        .unwrap();
    let mut sources = Vec::new();
    let mut hides = Vec::new();
    let mut bones = Vec::new();
    {
        let mut ctx = context.lock().unwrap();
        let player = ctx.map.entities.iter_mut().find(|e| e.id == body).unwrap();
        player.inventory = vec![None; 24];
        player.equipped.clear();
        player.set_attribute("class", Value::Str("Warrior".into()));
        let position = player.position;
        for profile in [
            "loose_flint",
            "fallen_timber",
            "wild_flax",
            "mature_tree",
            "mature_tree",
            "river",
        ] {
            let mut object = GeometryObject::box_(
                profile,
                position,
                theframework_core::prelude::Vec3::new(0.1, 0.1, 0.1),
            );
            object.solid = false;
            object.properties.set(
                "data",
                Value::Str(
                    bind_source(
                        &ctx.rules,
                        "",
                        ResourceBinding {
                            profile: profile.into(),
                            ..Default::default()
                        },
                    )
                    .unwrap(),
                ),
            );
            sources.push(object.id);
            ctx.map.geometry_objects.push(object);
        }
        resources::initialize(&mut ctx, false).unwrap();
        for (profile, accesses) in [
            ("skeleton_remains", &mut bones),
            ("deer_remains", &mut hides),
        ] {
            for _ in 0..4 {
                let mut dead = Entity::default();
                dead.id = ctx.world.next_id();
                dead.position = position;
                let id = dead.id;
                dead.set_attribute("mode", Value::Str("dead".into()));
                dead.set_attribute("remains_profile", Value::Str(profile.into()));
                ctx.map.entities.push(dead);
                accesses.push(resources::create_remains(&mut ctx, id).unwrap().unwrap());
            }
        }
    }
    let mut c = Crafter {
        host,
        peer,
        rx,
        stream,
        frame,
        actor: body,
        sequence: 1,
    };
    c.collect(sources[0], "collect_flint", 1, None, None).await;
    for _ in 0..13 {
        c.collect(sources[1], "collect_branches", 1, None, None)
            .await;
    }
    for _ in 0..4 {
        c.collect(sources[2], "collect_fiber", 2, None, None).await;
        c.craft("craft_twist_cord").await;
    }
    c.craft("craft_make_stone_knife").await;
    let knife = c.item("stone_knife");
    c.craft("craft_carve_bowl").await;
    let bowl = c.item("wooden_bowl");
    c.craft("craft_build_tanning_frame").await;
    c.craft("craft_build_bone_workbench").await;
    let position = actor(&mut c.host, body).get_pos_xz();
    for name in ["tanning_frame", "bone_workbench"] {
        let station = c.item(name);
        c.request(GameRequest::Transfer {
            source: ItemSource::Direct(ItemAddress {
                item: station.id,
                owner: ItemOwner::Owned,
            }),
            destination: ItemDestination::Ground([position.x, position.y]),
        })
        .await;
    }
    for source in hides {
        c.collect(source, "collect_hide", 1, Some(knife.id), None)
            .await;
    }
    for tree in [sources[3], sources[4]] {
        for _ in 0..4 {
            c.collect(tree, "collect_bark", 1, Some(knife.id), None)
                .await;
        }
    }
    for source in bones {
        c.collect(source, "collect_bones", 1, None, None).await;
    }
    for _ in 0..4 {
        c.collect(sources[5], "fill_water", 4, None, Some(bowl.id))
            .await;
        c.craft("craft_tan_hide").await;
    }
    for _ in 0..7 {
        c.craft("craft_cut_straps").await;
    }
    for n in 0..22 {
        if n % 4 == 0 {
            c.collect(
                sources[5],
                "fill_water",
                (22 - n).min(4),
                None,
                Some(bowl.id),
            )
            .await;
        }
        c.craft("craft_shape_bone").await;
    }
    assert_eq!(c.item("bone_plate").stack_quantity(), 22);
    for (suffix, _, slot, _, _, _) in BONE_PIECES {
        c.craft(&format!("craft_bone_{suffix}")).await;
        let armor = c.item(&format!("bone_{suffix}"));
        c.request(GameRequest::Transfer {
            source: ItemSource::Direct(ItemAddress {
                item: armor.id,
                owner: ItemOwner::Owned,
            }),
            destination: ItemDestination::Equipment(slot.into()),
        })
        .await;
    }
    let player = actor(&mut c.host, body);
    assert_eq!(player.equipped.len(), 5);
    assert_eq!(
        player
            .equipped
            .values()
            .map(|i| i.attributes.get_float_default("ARMOR", 0.0))
            .sum::<f32>(),
        10.0
    );
    for (name, expected) in [
        ("raw_bone", 2),
        ("leather", 1),
        ("leather_strap", 1),
        ("twisted_cord", 1),
    ] {
        assert_eq!(c.item(name).stack_quantity(), expected);
    }
    assert_eq!(
        c.item("stone_knife").attributes.get_int("condition"),
        Some(52)
    );
    let saved_bowl = c.item("wooden_bowl");
    assert_eq!(
        (saved_bowl.id, saved_bowl.creator_id),
        (bowl.id, bowl.creator_id)
    );
    assert_eq!(saved_bowl.attributes.get_int("liquid_quantity"), Some(0));

    // Prepare spare inputs from the remaining world-sourced materials. There
    // are enough for a duplicate repair, so its condition would reveal a replay.
    c.craft("craft_cut_straps").await;
    c.collect(sources[5], "fill_water", 2, None, Some(bowl.id))
        .await;
    for _ in 0..2 {
        c.craft("craft_shape_bone").await;
    }
    let cuirass = c.item("bone_cuirass");
    let attachment;
    {
        let mut ctx = context.lock().unwrap();
        let values = rusterix::effective_values::ValueRules::resolve(&ctx.rules).unwrap();
        let player = ctx.map.entities.iter_mut().find(|e| e.id == body).unwrap();
        let item = player.equipped.get_mut("torso").unwrap();
        attachment = rusterix::effective_values::attach(item, &values, "strength").unwrap();
        item.set_attribute("condition", Value::Int(0));
        item.set_attribute("quality", Value::Int(73));
        item.set_attribute("charges", Value::Int(3));
        item.set_attribute(
            "opaque_attachment",
            Value::Str("unchanged extension data".into()),
        );
        player.dirty_flags |= 0b10000;
    }
    let (other, mut output, other_stream, other_frame) = join(&mut c.host, "Other crafter").await;
    for (index, owner) in [ItemOwner::Owned, ItemOwner::Corpse(body)]
        .into_iter()
        .enumerate()
    {
        message(
            &mut c.host,
            other,
            ClientMessage::Action(GameAction {
                stream: other_stream,
                sequence: (index + 1) as u64,
                world: other_frame.world,
                generation: other_frame.generation,
                region: other_frame.current_region.unwrap(),
                request: GameRequest::Item {
                    target: ItemAddress {
                        item: cuirass.id,
                        owner,
                    },
                    intent: Some("action:repair_bone_armor".into()),
                },
            }),
        )
        .await;
        assert!(drain(&mut output).iter().any(|m| matches!(m,ServerMessage::ActionReply(reply)
            if reply.sequence == (index+1) as u64 && matches!(reply.result,ActionResult::Outcome(RequestOutcome::Denied(_))))));
    }
    assert_eq!(
        c.item("bone_cuirass").attributes.get_int("condition"),
        Some(0)
    );
    c.request(GameRequest::Item {
        target: ItemAddress {
            item: cuirass.id,
            owner: ItemOwner::Owned,
        },
        intent: Some("action:repair_bone_armor".into()),
    })
    .await;
    let repaired = c.item("bone_cuirass");
    assert_eq!(
        (repaired.id, repaired.creator_id),
        (cuirass.id, cuirass.creator_id)
    );
    assert_eq!(repaired.attributes.get_int("condition"), Some(20));
    assert_eq!(repaired.attributes.get_int("quality"), Some(73));
    assert_eq!(repaired.attributes.get_int("charges"), Some(3));
    assert_eq!(
        rusterix::effective_values::enchantments(&repaired).unwrap()[0].id,
        attachment
    );
    let owner = actor(&mut c.host, body);
    let values = {
        let ctx = context.lock().unwrap();
        rusterix::effective_values::ValueRules::resolve(&ctx.rules).unwrap()
    };
    assert_eq!(
        values.explain(&owner, "STR", 0.).unwrap().value,
        owner.attributes.get_float_default("STR", 0.) + 2.
    );
    message(&mut c.host, c.peer, ClientMessage::Resync).await;
    assert!(
        drain(&mut c.rx).iter().any(|message| match message {
            ServerMessage::Frame { frame, .. } =>
                serde_json::to_string(frame).unwrap().contains(&attachment),
            _ => false,
        }),
        "the encoded resync must preserve the actual attachment identity"
    );
    assert_eq!(
        repaired.attributes.get_str("opaque_attachment"),
        Some("unchanged extension data")
    );
    assert_eq!(c.item("bone_plate").stack_quantity(), 1);
    assert_eq!(c.item("leather_strap").stack_quantity(), 2);
    assert_eq!(
        c.item("stone_knife").attributes.get_int("condition"),
        Some(48)
    );
}
