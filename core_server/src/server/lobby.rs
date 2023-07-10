use crate::prelude::*;
use crossbeam_channel::{ Sender, Receiver, tick, select };

pub struct Lobby {
    sender                          : Sender<Message>,
    receiver                        : Receiver<Message>,

    threaded                        : bool,

    pub game_behavior               : GameBehaviorData,
    pub scripts                     : FxHashMap<String, String>,
    pub users                       : FxHashMap<Uuid, User>,

    startup_tree_name               : String,
    startup_script_name             : String,
}

impl Lobby {

    pub fn new(threaded: bool, sender: Sender<Message>, receiver: Receiver<Message>) -> Self {

        Self {
            sender,
            receiver,

            threaded,

            game_behavior           : GameBehaviorData::new(),
            scripts                 : FxHashMap::default(),
            users                   : FxHashMap::default(),

            startup_tree_name       : "".to_string(),
            startup_script_name     : "".to_string(),
        }
    }

    pub fn setup(&mut self, game: String, scripts: FxHashMap<String, String>) {
        self.scripts = scripts;
        if let Some(game_behavior) = serde_json::from_str::<GameBehaviorData>(&game).ok() {

            let mut startup_name : Option<String> = None;

            // Get the name of the startup game tree and its script name

            for (_id, node) in &game_behavior.nodes {
                if node.behavior_type == BehaviorNodeType::BehaviorType {
                    if let Some(value )= node.values.get(&"startup".to_string()) {
                        startup_name = Some(value.to_string_value());
                        self.startup_tree_name = value.to_string_value();
                    }
                }
            }

            self.game_behavior = game_behavior;

            if let Some(startup_name) = startup_name {
                if let Some(script_name) = self.get_script_name_for_screen(startup_name) {
                    self.startup_script_name = script_name;
                }
            }
        }
    }

    /// The game loop for these regions. Only called when mt is available. Otherwise server calls tick() directly.
    pub fn run(&mut self) {

        let ticker = tick(std::time::Duration::from_millis(250));

        loop {

            select! {
                recv(ticker) -> _ => {
                    _ = self.tick();
                },
                recv(self.receiver) -> mess => {
                    if let Some(message) = mess.ok() {
                        match message {
                            Message::Quit() => {
                                break;
                            },
                            Message::AddUserToLobby(user) => {
                                self.add_user(user);
                            },
                            Message::RemoveUserFromLobby(id) => {
                                self.remove_user(id);
                            },
                            Message::SetUserName(id, name) => {
                                self.set_user_name(id, name);
                            },
                            Message::SetUserScreenName(id, name) => {
                                self.set_user_screen_name(id, name);
                            },
                            Message::SetUserCharacters(id, list) => {
                                self.set_user_characters(id, list);
                            },
                            _ => { log::error!("Unhandled message for region pool: {:?}", message); }
                        }
                    }
                }
            }
        }
    }

    /// Adds a user struct to the lobby
    pub fn add_user(&mut self, user: User) {
        self.users.insert(user.id, user);
    }

    /// Adds a user struct to the lobby
    pub fn remove_user(&mut self, user_id: Uuid) {
        if let Some(_user) = self.users.remove(&user_id) {
        }
    }

    /// Sets the name of the user
    pub fn set_user_name(&mut self, user_id: Uuid, name: String) {
        if let Some(user) = self.users.get_mut(&user_id) {
            user.name = name;
        }
    }

    /// Sets the screen name of the user
    pub fn set_user_screen_name(&mut self, user_id: Uuid, name: String) {
        if let Some(script_name) = self.get_script_name_for_screen(name) {
            if let Some(user) = self.users.get_mut(&user_id) {
                user.new_screen_script = Some(script_name);
            }
        }
    }

    /// Sets the user characters
    pub fn set_user_characters(&mut self, user_id: Uuid, list: Vec<CharacterData>) {
        if let Some(user) = self.users.get_mut(&user_id) {
            user.characters = list;
        }
    }

    pub fn tick(&mut self) -> Vec<Message> {
        let mut ret_messages : Vec<Message> = vec![];

        for (id, user) in &mut self.users {

            let mut update = GameUpdate::new();
            update.id = *id;

            if user.screen_script.is_none() {
                update.screen_scripts = Some(self.scripts.clone());
                update.screen_script_name = Some(self.startup_script_name.clone());
                user.screen_script = Some(self.startup_script_name.clone());
            }

            if let Some(new_screen_name) = &user.new_screen_script {
                update.screen_script_name = Some(new_screen_name.clone());
                user.screen_script = user.new_screen_script.clone();
                user.new_screen_script = None;
            }

            update.characters = user.characters.clone();

            let m = Message::PlayerUpdate(*id, update);

            match m {
                // Message::TransferCharacter(region_id, instance, sheet) => {
                //    characters_to_transfer.push((region_id, instance, sheet));
                //},
                _ => {
                    if self.threaded {
                        self.sender.send(m).unwrap()
                    } else {
                        ret_messages.push(m);
                    }
                }
            }
        }

        ret_messages
    }

    /// Returns the script name for a given game behavior tree
    fn get_script_name_for_screen(&self, screen_name: String) -> Option<String> {
        let mut screen_node_id : Option<Uuid> = None;

        for (id, node) in &self.game_behavior.nodes {
            if node.behavior_type == BehaviorNodeType::BehaviorTree {
                if node.name == screen_name {
                    for c in &self.game_behavior.connections {
                        if c.0 == *id {
                            screen_node_id = Some(c.2);
                        }
                    }
                }
            }
        }

        if let Some(screen_node_id) = screen_node_id {
            if let Some(screen_node) = self.game_behavior.nodes.get(&screen_node_id) {
                if let Some(value) = screen_node.values.get("script_name") {
                    if let Some(script_name) = value.to_string() {
                        return Some(script_name);
                    }
                }
            }
        }
        None
    }

}