use crate::prelude::*;
use crossbeam_channel::{ Sender, Receiver, tick, select };

pub struct Lobby {
    sender                          : Sender<Message>,
    receiver                        : Receiver<Message>,

    threaded                        : bool,

    pub scripts                     : FxHashMap<String, String>,
    pub users                       : FxHashMap<Uuid, User>
}

impl Lobby {

    pub fn new(threaded: bool, sender: Sender<Message>, receiver: Receiver<Message>) -> Self {

        Self {
            sender,
            receiver,

            threaded,

            scripts                 : FxHashMap::default(),
            users                   : FxHashMap::default()
        }
    }

    pub fn setup(&mut self, scripts: FxHashMap<String, String>) {
        self.scripts = scripts;
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
                                println!("lobby quit");
                                break;
                            },
                            Message::AddUserToLobby(user) => {
                                println!("AddUserToLobby");
                                self.users.insert(user.id, user);
                            }
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

    pub fn tick(&mut self) -> Vec<Message> {
        let mut ret_messages : Vec<Message> = vec![];

        println!("lobby tick");

        ret_messages
    }
}