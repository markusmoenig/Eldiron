use eldiron_network::*;
use eldiron_runtime::{ClientFrame, Project};
use eldiron_server::host::{Command, Host};
use rusterix::{GeometryObject, Item, Value, server::region::resources::ResourceRequest};
use shared::rulesets::{
    document::RulesState,
    world_resources::{ResourceBinding, bind_source},
};
use std::time::Duration;
use theframework_core::{Uuid, prelude::Vec3};
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

#[tokio::test]
async fn encoded_resource_requests_share_stock_and_retries_do_not_refill_after_recovery() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_projects/3DStarter.eldiron");
    let mut project: Project =
        shared::project_io::decode_project(&std::fs::read(path).unwrap()).unwrap();
    project.rules_document = RulesState::default();
    let mut accesses = Vec::new();
    let bounds = project.regions[0].map.bbox();
    for _ in 0..2 {
        // A non-solid test access area spans the existing map. Exact reach and
        // vertical access are tested separately in runtime/world_resources.
        let mut object = GeometryObject::box_from_bounds(
            "Reservoir access",
            Vec3::new(bounds.min.x, -10., bounds.min.y),
            Vec3::new(bounds.max.x, 10., bounds.max.y),
        );
        object.solid = false;
        object.properties.set(
            "data",
            Value::Str(
                bind_source(
                    project.rules_document.document().unwrap().table(),
                    "",
                    ResourceBinding {
                        profile: "finite_well".into(),
                        pool: Some("village_well".into()),
                        ..Default::default()
                    },
                )
                .unwrap(),
            ),
        );
        accesses.push(object.id);
        project.regions[0].map.geometry_objects.push(object);
    }
    let mut host = Host::open(project, None).await.unwrap();
    let (alice, mut a, stream_a, frame_a) = join(&mut host, "Collector A").await;
    let (bob, mut b, stream_b, frame_b) = join(&mut host, "Collector B").await;
    drain(&mut a);
    let mut vessels = Vec::new();
    {
        let context = host.world.game().server.instances[0]
            .lock()
            .unwrap()
            .context()
            .unwrap();
        let mut ctx = context.lock().unwrap();
        for actor in [
            frame_a.controlled_entity.unwrap(),
            frame_b.controlled_entity.unwrap(),
        ] {
            let mut vessel = Item::default();
            vessel.id = ctx.world.next_id();
            vessel.set_attribute("liquid_capacity", Value::Int(4));
            vessel.set_attribute("liquid_quantity", Value::Int(0));
            vessels.push(vessel.id);
            let actor = ctx.map.entities.iter_mut().find(|e| e.id == actor).unwrap();
            actor.inventory.resize(32, None);
            actor.add_item(vessel).unwrap();
        }
    }
    let command = |frame: &ClientFrame, stream, access, vessel| GameAction {
        stream,
        sequence: 1,
        world: frame.world,
        generation: frame.generation,
        region: frame.current_region.unwrap(),
        request: GameRequest::Resource(ResourceRequest {
            access,
            interaction: "fill_water".into(),
            quantity: 4,
            tool: None,
            vessel: Some(vessel),
        }),
    };
    let first = command(&frame_a, stream_a, accesses[0], vessels[0]);
    let second = command(&frame_b, stream_b, accesses[1], vessels[1]);
    message(&mut host, alice, ClientMessage::Action(first.clone())).await;
    let output = drain(&mut a);
    assert_processed(&output, 1);
    assert!(output.iter().any(|m|matches!(m,ServerMessage::Frame {frame,..} if matches!(&frame.update,eldiron_runtime::replication::WorldUpdate::Delta(regions) if regions.iter().any(|r|r.resource_views.as_ref().is_some_and(|views|views.iter().all(|v|v.supplies.get("water")==Some(&Some(0)))))))));
    message(&mut host, bob, ClientMessage::Action(second.clone())).await;
    drain(&mut b);
    let a_id = frame_a.controlled_entity.unwrap();
    let b_id = frame_b.controlled_entity.unwrap();
    assert_eq!(
        actor(&mut host, a_id)
            .get_item(vessels[0])
            .unwrap()
            .attributes
            .get_int_default("liquid_quantity", 0),
        3
    );
    assert_eq!(
        actor(&mut host, b_id)
            .get_item(vessels[1])
            .unwrap()
            .attributes
            .get_int_default("liquid_quantity", 0),
        0
    );
    {
        let context = host.world.game().server.instances[0]
            .lock()
            .unwrap()
            .context()
            .unwrap();
        let mut ctx = context.lock().unwrap();
        ctx.map
            .entities
            .iter_mut()
            .find(|e| e.id == a_id)
            .unwrap()
            .get_item_mut(vessels[0])
            .unwrap()
            .set_attribute("liquid_quantity", Value::Int(0));
        ctx.world
            .advance_resource_time(Duration::from_millis(43_200_000));
    }
    message(&mut host, alice, ClientMessage::Action(first.clone())).await;
    assert_processed(&drain(&mut a), 1);
    message(&mut host, bob, ClientMessage::Action(second)).await;
    drain(&mut b);
    assert_eq!(
        actor(&mut host, a_id)
            .get_item(vessels[0])
            .unwrap()
            .attributes
            .get_int_default("liquid_quantity", 0),
        0
    );
    assert_eq!(
        actor(&mut host, b_id)
            .get_item(vessels[1])
            .unwrap()
            .attributes
            .get_int_default("liquid_quantity", 0),
        0
    );
    let mut fresh = first;
    fresh.sequence = 2;
    message(&mut host, alice, ClientMessage::Action(fresh)).await;
    assert_processed(&drain(&mut a), 2);
    assert_eq!(
        actor(&mut host, a_id)
            .get_item(vessels[0])
            .unwrap()
            .attributes
            .get_int_default("liquid_quantity", 0),
        3
    );
}
