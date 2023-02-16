use crate::prelude::*;

pub mod region_instance;
pub mod region_pool;
pub mod message;
pub mod nodes;
pub mod script_utilities;

use crossbeam_channel::{ Sender, Receiver, unbounded };

pub struct RegionPoolMeta {
    sender                  : Sender<Message>,
    _receiver               : Receiver<Message>,

    region_ids              : Vec<Uuid>,
}

pub struct Server<'a> {

    to_server_receiver      : Receiver<Message>,
    to_server_sender        : Sender<Message>,

    pub regions             : HashMap<Uuid, String>,
    pub region_behavior     : HashMap<Uuid, Vec<String>>,

    pub behavior            : Vec<String>,
    pub systems             : Vec<String>,
    pub items               : Vec<String>,
    pub game                : String,

    /// If we don't use threads (for example for the web), all regions are in here.
    pub pool                : Option<RegionPool<'a>>,

    /// The meta data for all pools
    metas                   : Vec<RegionPoolMeta>,

    /// The default starting position for players
    player_default_position : Option<Position>,

    /// The region ids for each player uuid so that we know where to send messages.
    players_region_ids      : HashMap<Uuid, Uuid>,

    /// We are multi-threaded
    threaded                : bool,
}

impl Server<'_> {

    pub fn new() -> Self {

        let (sender, receiver) = unbounded();

        Self {

            to_server_receiver          : receiver,
            to_server_sender            : sender,

            regions                     : HashMap::new(),
            region_behavior             : HashMap::new(),

            behavior                    : vec![],
            systems                     : vec![],
            items                       : vec![],
            game                        : "".to_string(),
            pool                        : None,
            metas                       : vec![],

            player_default_position     : None,
            players_region_ids          : HashMap::new(),

            threaded                    : false,
        }
    }

    /// Collects all data (assets, regions, behaviors etc.) and store them as JSON so that we can distribute them to threads as needed.
    pub fn collect_data(&mut self, data: &GameData) {

        for (id, region) in &data.regions {
            if let Some(json) = serde_json::to_string(&region.data).ok() {
                self.regions.insert(*id, json);

                let mut behavior = vec![];
                for b in &region.behaviors {
                    if let Some(json) = serde_json::to_string(&b.data).ok() {
                        behavior.push(json);
                    }
                }
                self.region_behavior.insert(*id, behavior);
            }
        }

        for (id, behavior) in &data.behaviors {
            if behavior.data.name == "Player" {
                self.player_default_position = data.get_behavior_default_position(*id);
            }
            if let Some(json) = serde_json::to_string(&behavior.data).ok() {
                self.behavior.push(json);
            }
        }

        for (_id, system) in &data.systems {
            if let Some(json) = serde_json::to_string(&system.data).ok() {
                self.systems.push(json);
            }
        }

        for (_id, item) in &data.items {
            if let Some(json) = serde_json::to_string(&item.data).ok() {
                self.items.push(json);
            }
        }

        if let Some(json) = serde_json::to_string(&data.game.behavior.data).ok() {
            self.game = json;
        }
    }

    /// Starts the server and distributes regions over threads. max_num_threads limits the max number of threads or does not use threads at all if None.
    pub fn start(&mut self, max_num_threads: Option<i32>) -> Result<(), String> {

        if  let Some(_max_num_threads) = max_num_threads {

            self.threaded = true;

            let max_regions_per_pool = 100;

            let mut regions = vec![];
            let mut region_ids : Vec<Uuid> = vec![];

            let mut start_thread = |region_ids: Vec<Uuid>, regions: Vec<String>| {

                let (sender, receiver) = unbounded();

                let r = receiver.clone();

                let mut region_behavior: HashMap<Uuid, Vec<String>> = HashMap::new();
                for rid in &region_ids {
                    let behavior = self.region_behavior.get(rid).unwrap().clone();
                    region_behavior.insert(*rid, behavior);
                }

                let meta = RegionPoolMeta {
                    sender,
                    _receiver : receiver,
                    region_ids,
                };

                let behaviors = self.behavior.clone();
                let systems = self.systems.clone();
                let items = self.items.clone();
                let game = self.game.clone();

                let to_server_sender = self.to_server_sender.clone();

                let _handle = std::thread::spawn( move || {
                    let mut pool = RegionPool::new(true, to_server_sender, r);
                    pool.add_regions(regions, region_behavior, behaviors, systems, items, game);
                });

                //meta.sender.send(Message::Status("Startup".to_string())).unwrap();
                self.metas.push(meta);
            };

            for (id, json) in &self.regions {

                if regions.len() < max_regions_per_pool {
                    regions.push(json.clone());
                    region_ids.push(*id);
                } else {
                    start_thread(region_ids, regions.clone());
                    regions = vec![];
                    region_ids = vec![];
                }
            }

            if regions.is_empty() == false {
                start_thread(region_ids, regions.clone());
            }
        } else {
            let (sender, receiver) = unbounded::<Message>();

            let to_server_sender = self.to_server_sender.clone();
            sender.send(Message::Status("Startup".to_string())).unwrap();

            let mut pool = RegionPool::new(false, to_server_sender, receiver);
            pool.add_regions(self.regions.values().cloned().collect(), self.region_behavior.clone(), self.behavior.clone(), self.systems.clone(), self.items.clone(), self.game.clone());
            self.pool = Some(pool);
        }

        log::info!("Server started successfully!");

        Ok(())
    }

    /// Shutdown the system
    pub fn shutdown(&mut self) -> Result<(), String> {

        for meta in &self.metas {
            meta.sender.send(Message::Quit()).unwrap();
        }

        Ok(())
    }

    /// Only called when running on a non threaded system (web)
    pub fn tick(&mut self) -> Vec<Message> {
        let mut messages : Vec<Message> = vec![];

        if let Some(pool) = &mut self.pool {
            if let Some(msg) = pool.tick() {
                for m in msg {
                    match m {
                        Message::CharacterHasBeenTransferredInsidePool(uuid, region_id) => {
                            self.players_region_ids.insert(uuid, region_id);
                        },
                        _ => messages.push(m.clone())
                    }
                }
            }
        }

        messages
    }

    /// Gets the current messages to the server from the queue
    pub fn check_for_messages(&mut self) -> Vec<Message> {
        let mut messages : Vec<Message> = vec![];
        loop {
            if let Some(message) = self.to_server_receiver.try_recv().ok() {
                //println!("message {:?}", message);
                match message {
                    Message::CharacterHasBeenTransferredInsidePool(uuid, region_id) => {
                        self.players_region_ids.insert(uuid, region_id);
                    }
                    _ => messages.push(message),
                }
            } else {
                break;
            }
        }
        messages
    }

    /// Create a new player instance
    pub fn create_player_instance(&mut self) -> Uuid {
        let uuid = uuid::Uuid::new_v4();
        if let Some(position) = &self.player_default_position {
            if self.threaded {
                self.send_message_to_region(position.region, Message::CreatePlayerInstance(uuid, position.clone()));
            } else {
                if let Some(pool) = &mut self.pool {
                    pool.create_player_instance(uuid, position.clone());
                }
            }
            self.players_region_ids.insert(uuid, position.region);
        }
        uuid
    }

    /// Destroy a player instance
    pub fn destroy_player_instance(&mut self, uuid: Uuid) {
        if self.threaded {
            for m in &self.metas {
                _ = m.sender.send(Message::DestroyPlayerInstance(uuid));
            }
        } else {
            if let Some(pool) = &mut self.pool {
                pool.destroy_player_instance(uuid);
            }
        }
    }

    /// Send the behavior id to debug to all pools.
    pub fn set_debug_behavior_id(&self, behavior_id: Uuid) {
        for m in &self.metas {
            m.sender.send(Message::SetDebugBehaviorId(behavior_id)).unwrap();
        }
    }

    /// Assign an action to an instance
    pub fn execute_packed_player_action(&mut self, player_uuid: Uuid, action: String) {
        if let Some(region_id) = self.players_region_ids.get(&player_uuid) {
            if let Some(action) = serde_json::from_str::<PlayerAction>(&action).ok() {
                if self.threaded {
                    let message = Message::ExecutePlayerAction(player_uuid, *region_id, action);
                    self.send_message_to_region(*region_id, message);
                } else {
                    if let Some(pool) = &mut self.pool {
                        pool.execute_player_action(player_uuid, *region_id, action);
                    }
                }
            }
        }
    }

    /// Send a message to the given region
    pub fn send_message_to_region(&self, region_id: Uuid, message: Message) {
        for m in &self.metas {
            if m.region_ids.contains(&region_id) {
                _ = m.sender.send(message);
                break;
            }
        }
    }

}