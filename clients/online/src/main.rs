#[cfg(target_arch = "wasm32")]
fn main() {}
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Online client: {error}");
        std::process::exit(1);
    }
}
#[cfg(not(target_arch = "wasm32"))]
async fn run() -> Result<(), String> {
    use eldiron_network::{native, remote::RemoteClient, *};
    use eldiron_runtime::MovementIntent;
    use std::time::{Duration, Instant};
    use tokio::io::{AsyncBufReadExt, BufReader};
    let mut endpoint = "wss://localhost:8443/play".to_owned();
    let mut guest = None;
    let mut username = None;
    let mut register = false;
    let mut insecure = false;
    let mut chosen = "Adventurer".to_string();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--server" => endpoint = args.next().ok_or("Missing server")?,
            "--guest" => guest = Some(args.next().ok_or("Missing guest name")?),
            "--username" => username = Some(args.next().ok_or("Missing username")?),
            "--name" => chosen = args.next().ok_or("Missing name")?,
            "--register" => register = true,
            "--insecure-loopback" => insecure = true,
            "--help" => {
                println!(
                    "eldiron-online --server wss://host/play (--guest Name | --username account [--register]) [--name Character] [--insecure-loopback]\nAccount password: first line of stdin. Commands: w/a/s/d, x stop, q logout (press Enter)."
                );
                return Ok(());
            }
            _ => return Err(format!("Unknown option {arg}")),
        }
    }
    if guest.is_some() == username.is_some() {
        return Err("Choose --guest or --username".into());
    }
    let mut stdin = BufReader::new(tokio::io::stdin()).lines();
    let login = if let Some(username) = username {
        eprintln!("Reading account password from first stdin line.");
        let password = Secret(
            stdin
                .next_line()
                .await
                .map_err(|e| e.to_string())?
                .ok_or("Password required")?,
        );
        if register {
            ClientMessage::Register { username, password }
        } else {
            ClientMessage::Login { username, password }
        }
    } else {
        ClientMessage::Guest {
            name: guest.unwrap(),
        }
    };
    let mut socket = native::connect(&endpoint, insecure).await?;
    native::send(
        &mut socket,
        &ClientMessage::Hello {
            version: PROTOCOL_VERSION,
        },
    )
    .await?;
    match native::receive(&mut socket).await? {
        ServerMessage::Welcome { mode, .. } => println!("Connected: {mode:?}"),
        _ => return Err("Missing welcome".into()),
    }
    native::send(&mut socket, &login).await?;
    let mut remote: Option<RemoteClient> = None;
    let mut content_stream = ContentStream::default();
    let mut staged = Vec::new();
    let mut clock = tokio::time::interval(Duration::from_millis(20));
    clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last = Instant::now();
    let mut heartbeat = Instant::now();
    let mut shown = Instant::now();
    println!("Controls: w/a/s/d, x stop, q logout (then Enter).");
    loop {
        for _ in 0..8 {
            let Some(delivery) = content_stream.poll(&endpoint)? else {
                break;
            };
            match delivery {
                ContentDelivery::Content(hint, package) => staged.push((hint, package)),
                ContentDelivery::Packet(msg) => match msg {
                    ServerMessage::Authorized { characters, .. } => {
                        let msg = if let Some(character) = characters.first() {
                            ClientMessage::JoinCharacter {
                                character: character.id,
                            }
                        } else {
                            ClientMessage::CreateCharacter {
                                request: theframework_core::Uuid::new_v4(),
                                name: chosen.clone(),
                            }
                        };
                        native::send(&mut socket, &msg).await?;
                    }
                    ServerMessage::Frame { frame, .. } => {
                        let client = remote.get_or_insert_with(|| RemoteClient::new(&frame));
                        for (hint, package) in staged.drain(..) {
                            client.install(&hint, package)?;
                        }
                        if client.receive(&frame).is_err() {
                            native::send(&mut socket, &ClientMessage::Resync).await?;
                        }
                        if shown.elapsed() > Duration::from_secs(1) {
                            if let Some(map) = client.motion.authority().current_map() {
                                let actor = client.motion.authority().controlled_entity();
                                if let Some(entity) =
                                    map.entities.iter().find(|e| Some(e.id) == actor)
                                {
                                    println!(
                                        "{}: ({:.2}, {:.2}) · {} actors",
                                        map.name,
                                        entity.position.x,
                                        entity.position.z,
                                        map.entities.len()
                                    );
                                }
                            }
                            shown = Instant::now();
                        }
                    }
                    ServerMessage::Rejected {
                        code: RejectCode::InputResync,
                    } => {
                        if let Some(client) = &mut remote {
                            client.motion.resume();
                        }
                    }
                    ServerMessage::Rejected { code } => {
                        return Err(format!("Server rejected request: {code:?}"));
                    }
                    ServerMessage::CharacterCreated { character, .. } => {
                        println!("Created character {character}")
                    }
                    _ => {}
                },
            }
        }
        tokio::select! {
            msg=native::receive(&mut socket)=>content_stream.push(&endpoint,msg?)?,
            line=stdin.next_line()=>{
                let Some(line)=line.map_err(|e| e.to_string())? else {native::send(&mut socket,&ClientMessage::Logout).await?;return Ok(());};
                if line.trim()=="q" {native::send(&mut socket,&ClientMessage::Logout).await?;return Ok(());}
                if let Some(client)=&mut remote {let intent=match line.trim() {"w"=>MovementIntent::Forward,"s"=>MovementIntent::Backward,"a"=>MovementIntent::StrafeLeft,"d"=>MovementIntent::StrafeRight,_=>MovementIntent::Stop};client.motion.set_intent(intent);}
            },
            _=clock.tick()=>{
                let now=Instant::now();let elapsed=now.duration_since(last);last=now;
                if let Some(client)=&mut remote {let message=client.advance(elapsed.min(Duration::from_millis(100)));native::send(&mut socket,&message).await?;}
                if heartbeat.elapsed()>Duration::from_secs(5) {native::send(&mut socket,&ClientMessage::Ping{nonce:0}).await?;heartbeat=Instant::now();}
            }
        }
    }
}
