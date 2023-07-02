use crate::prelude::*;

pub mod local_fs;

pub enum IOError {
    NotImplemented,
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
    fn login_user(&mut self, user_name: String, password: String) -> Result<bool, IOError> { Err(NotImplemented) }

    /// Does the user exist ?
    fn does_user_exist(&mut self, user_name: String) -> Result<bool, IOError> { Err(NotImplemented) }

    /// Create a new user
    fn create_user(&mut self, user_name: String, password: String) -> Result<bool, IOError> { Err(NotImplemented) }

    /// Create a character
    fn save_user_character(&mut self, user_name: String, character: Sheet) -> Result<bool, IOError> { Err(NotImplemented) }

    /// Create a character
    fn list_user_characters(&mut self, user_name: String) -> Result<Vec<Sheet>, IOError> { Err(NotImplemented) }
}