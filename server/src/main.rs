use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
#[cfg(feature = "tls")]
use tokio_native_tls::{native_tls, TlsAcceptor, TlsStream};
use tokio_tungstenite::{tungstenite, WebSocketStream};

use core_server::prelude::*;

#[cfg(feature = "tls")]
use std::{fs::File, io::Read};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration};//, SystemTime, UNIX_EPOCH};
/*

/// Gets the current time in milliseconds
fn get_time() -> u128 {
    let stop = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
        stop.as_millis()
}*/

#[cfg(feature = "tls")]
type UuidPeerMap = FxHashMap<
    Uuid, 
    SplitSink<WebSocketStream<TlsStream<TcpStream>>, tungstenite::Message>
>;

#[cfg(not(feature = "tls"))]
type UuidPeerMap = FxHashMap<
    Uuid, 
    SplitSink<WebSocketStream<TcpStream>, tungstenite::Message>
>;

enum Stream {
    #[cfg(not(feature = "tls"))]
    Plain(WebSocketStream<TcpStream>),
    #[cfg(feature = "tls")]
    Tls(WebSocketStream<TlsStream<TcpStream>>),
}

#[cfg(not(feature = "tls"))]
async fn handle_client_connection(
    stream: TcpStream,
    server: Arc<Mutex<Server<'_>>>,
    uuid_endpoint: Arc<Mutex<UuidPeerMap>>,
) {
    let ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();

    handle_client_messages(
        Stream::Plain(ws_stream),
        server,
        uuid_endpoint
    ).await;
}

#[cfg(feature = "tls")]
async fn handle_client_connection_with_tls(
    stream: TcpStream,
    tls_acceptor: Arc<TlsAcceptor>,
    server: Arc<Mutex<Server<'_>>>,
    uuid_endpoint: Arc<Mutex<UuidPeerMap>>,
) {
    let tls_stream = tls_acceptor.accept(stream).await.unwrap();

    let ws_stream = tokio_tungstenite::accept_async(tls_stream).await.unwrap();

    handle_client_messages(
        Stream::Tls(ws_stream),
        server,
        uuid_endpoint
    ).await;
}

async fn handle_client_messages(
    ws_stream: Stream,
    server: Arc<Mutex<Server<'_>>>,
    uuid_endpoint: Arc<Mutex<UuidPeerMap>>,
) {
    let (sink, mut stream) = match ws_stream {
        #[cfg(not(feature = "tls"))]
        Stream::Plain(ws_stream) => ws_stream.split(),
        #[cfg(feature = "tls")]
        Stream::Tls(ws_stream) => ws_stream.split(),
    };
    
    if !wait_for_login(&mut stream).await {
        return;
    }

    let uuid = server.lock().await.create_player_instance();
    println!("logged in anonymous {:?}", uuid);
    uuid_endpoint.lock().await.insert(uuid, sink);

    loop {
        let msg = stream.try_next().await;
        if msg.is_err() {
            server.lock().await.destroy_player_instance(uuid);
            uuid_endpoint.lock().await.remove(&uuid);
            println!("Client disconnected");
            break;
        }

        if let Some(msg) = msg.unwrap() {
            match msg {
                tungstenite::Message::Binary(bin) => {
                    let cmd : ServerCmd = ServerCmd::from_bin(&bin)
                        .unwrap_or(ServerCmd::NoOp);
    
                    match cmd {
                        ServerCmd::GameCmd(action) => {
                            server
                                .lock()
                                .await
                                .execute_packed_player_action(uuid, action)
                        },
                        _ => {}
                    }
                },
                _ => {}
            }
        }
    }
}

async fn handle_server_messages(
    server: Arc<Mutex<core_server::server::Server<'_>>>,
    uuid_endpoint: Arc<Mutex<UuidPeerMap>>,
) {
    loop {
        tokio::time::sleep(Duration::from_millis(10)).await;

        let messages = server.lock().await.check_for_messages();
        
        for message in messages {
            match message {
                Message::PlayerUpdate(_uuid, update) => {
                    if let Some(sink) = uuid_endpoint.lock().await.get_mut(&update.id) {
                        let cmd = ServerCmd::GameUpdate(update);
                        if let Some(bin) = cmd.to_bin() {
                            sink
                                .send(tungstenite::Message::binary(bin))
                                .await
                                .unwrap();
                        }
                    }
                },
                _ => {}
            }
        }
    }
}

#[cfg(feature = "tls")]
fn read_tls_acceptor(file_path: &PathBuf, password: &str) -> TlsAcceptor {
    let mut file = File::open(file_path).unwrap();

    let mut identity = vec![];
    file.read_to_end(&mut identity).unwrap();

    let identity = native_tls::Identity::from_pkcs12(&identity, password).unwrap();

    TlsAcceptor::from(native_tls::TlsAcceptor::new(identity).unwrap())
}

async fn wait_for_login<S>(stream: &mut SplitStream<WebSocketStream<S>>) -> bool
where
    S: AsyncRead + AsyncWrite + Unpin
{
    let msg = stream.try_next().await;

    if msg.is_err() {
        println!("Client disconnected");
        return false;
    }

    if let Some(msg) = msg.unwrap() {
        match msg {
            tungstenite::Message::Binary(bin) => {
                let cmd : ServerCmd = ServerCmd::from_bin(&bin)
                    .unwrap_or(ServerCmd::NoOp);

                match cmd {
                    ServerCmd::LoginAnonymous => {
                        return true;
                    },
                    _ => {}
                }
            },
            _ => {}
        }
    }

    false
}

#[tokio::main]
async fn main() {
    env_logger::init();

    // Init server
    let game_data = GameData::load_from_path(PathBuf::from(".."));

    let mut server = core_server::server::Server::new();
    server.collect_data(&game_data);

    // Start the server with a maximum of 10 thread pools
    _ = server.start( Some(10) );

    let server = Arc::new(Mutex::new(server));

    // let mut timer : u128 = 0;
    // let mut game_tick_timer : u128 = 0;

    let uuid_endpoint : Arc<Mutex<UuidPeerMap>> = Arc::new(Mutex::new(FxHashMap::default()));

    tokio::spawn(handle_server_messages(server.clone(), uuid_endpoint.clone()));
    
    // Init network

    let tcp_listener = TcpListener::bind("127.0.0.1:3042").await.unwrap();

    while let Ok((stream, _)) = tcp_listener.accept().await {
        #[cfg(feature = "tls")]
        {
            let tls_acceptor = Arc::new(read_tls_acceptor(&PathBuf::from("keyStore.p12"), ""));
            
            tokio::spawn(handle_client_connection_with_tls(
                stream,
                tls_acceptor.clone(),
                server.clone(),
                uuid_endpoint.clone()
            ));
        }

        #[cfg(not(feature = "tls"))]
        {
            tokio::spawn(handle_client_connection(
                stream,
                server.clone(),
                uuid_endpoint.clone()
            ));
        }
    }
}
