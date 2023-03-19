use message_io::network::{NetEvent, Transport, Endpoint};
use message_io::node::{self, NodeEvent};

use core_server::prelude::*;

use std::time::{Duration};//, SystemTime, UNIX_EPOCH};

use std::path::PathBuf;
/*

/// Gets the current time in milliseconds
fn get_time() -> u128 {
    let stop = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
        stop.as_millis()
}*/

fn main() {
    env_logger::init();

    // Init server
    let game_data = GameData::load_from_path(PathBuf::from(".."));

    let mut server = core_server::server::Server::new();
    server.collect_data(&game_data);

    // Start the server with a maximum of 10 thread pools
    _ = server.start( Some(10) );

    // let mut timer : u128 = 0;
    // let mut game_tick_timer : u128 = 0;

    let mut endpoint_uuid : FxHashMap<Endpoint, Uuid> = FxHashMap::default();
    let mut uuid_endpoint : FxHashMap<Uuid, Endpoint> = FxHashMap::default();

    // Init network

    let (handler, listener) = node::split::<()>();

    // Listen for TCP, UDP and WebSocket messages at the same time.
    handler.network().listen(Transport::FramedTcp, "0.0.0.0:3042").unwrap();
    handler.network().listen(Transport::Udp, "0.0.0.0:3043").unwrap();
    handler.network().listen(Transport::Ws, "0.0.0.0:3044").unwrap();

    handler.signals().send_with_timer((), Duration::from_millis(10));

    // Read incoming network events.
    listener.for_each(move |event| match event {
        NodeEvent::Network(net_event) => match net_event {
            NetEvent::Connected(_, _) => {
                unreachable!(); // Used for explicit connections.
            },
            NetEvent::Accepted(_endpoint, _listener) => {
                println!("Client connected."); // Tcp or Ws
            },
            NetEvent::Message(endpoint, data) => {

                let cmd : ServerCmd = ServerCmd::from_bin(&data)
                    .unwrap_or(ServerCmd::NoOp);

                match cmd {
                    ServerCmd::LoginAnonymous => {
                        let player_uuid = server.create_player_instance();
                        println!("logged in anonymous {:?}", player_uuid);
                        endpoint_uuid.insert(endpoint, player_uuid);
                        uuid_endpoint.insert(player_uuid, endpoint);
                    },
                    ServerCmd::GameCmd(action) => {
                        //println!("game cmd {:?}", game_cmd);
                        if let Some(player_id) = endpoint_uuid.get(&endpoint) {
                            server.execute_packed_player_action(*player_id, action)
                        }
                    },
                    _ => {
                    }
                }

                //println!("Received: {:?}", cmd);
                handler.network().send(endpoint, data);
            },
            NetEvent::Disconnected(endpoint) => {
                if let Some(player_id) = endpoint_uuid.get(&endpoint) {
                    server.destroy_player_instance(*player_id);
                    uuid_endpoint.remove(player_id);
                }
                endpoint_uuid.remove(&endpoint);
                println!("Client disconnected"); //Tcp or Ws
            }
        },

        NodeEvent::Signal(signal) => match signal {
            _ => {
                let messages = server.check_for_messages();

                for message in messages {
                    match message {
                        Message::PlayerUpdate(_uuid, update) => {
                            if let Some(client) = uuid_endpoint.get(&update.id) {
                                let cmd = ServerCmd::GameUpdate(update);
                                if let Some(bin) = cmd.to_bin() {
                                    //println!("{:?}", bin.len());
                                    handler.network().send(*client, &bin);
                                }
                            }
                        },
                        _ => {}
                    }
                }

                handler.signals().send_with_timer((), Duration::from_millis(10));
            },
        }
    });
}
