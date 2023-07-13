use crate::prelude::*;

pub mod fs;
pub mod local_fs;

pub enum IOError {
    NotImplemented,
    UserNotFound,
    UserAlreadyExists,
    WrongPassword
}

use IOError::*;

/// This trait defines all IO for the server, like creating or listing users and their characters. Implementations can do this locally in the filesytem or access databases etc.
#[allow(unused)]
pub trait ServerIO : Sync + Send {

    /// Create a new IO for the server.
    fn new() -> Self where Self: Sized;

    /// For local file servers, set the path to the directory where users and their characters are stored.
    fn set_local_path(&mut self, path: PathBuf) {}

    /// Login for database based implementations and similar.
    fn system_login(&mut self, url: String, password: String) -> Result<bool, IOError> { Err(NotImplemented) }

    /// Login the given user
    fn login_user(&self, user_name: String, password: String) -> Result<(), IOError> { Err(NotImplemented) }

    /// Does the user exist ?
    fn does_user_exist(&mut self, user_name: String) -> Result<bool, IOError> { Err(NotImplemented) }

    /// Create a new user
    fn create_user(&self, user_name: String, password: String) -> Result<(), IOError> { Err(NotImplemented) }

    /// Save a user character
    fn save_user_character(&self, user_name: String, sheet: Sheet) -> Result<(), IOError> { Err(NotImplemented) }

    /// Get a user character
    fn get_user_character(&self, user_name: String, character_name: String) -> Result<Sheet, IOError> { Err(NotImplemented) }

    /// List all user characters
    fn list_user_characters(&self, user_name: String) -> Result<Vec<CharacterData>, IOError> { Err(NotImplemented) }

    /// Returns a human readable error message
    fn error_message(&self, err: IOError) -> Option<String> {
        match err {
            UserNotFound => {
                Some("User not found.".into())
            },
            UserAlreadyExists => {
                Some("User already exsits.".into())
            },
            WrongPassword => {
                Some("Wrong password.".into())
            },
            _ => {
                None
            }
        }
    }
}