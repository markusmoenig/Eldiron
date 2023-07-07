use crate::prelude::*;
use IOError::*;

// This implementation is for local games with a single user. No login required.

pub struct UserFS {

    path                : PathBuf,
}

impl ServerIO for UserFS {

    fn new() -> Self where Self: Sized {
        Self {
            path        : PathBuf::new(),
        }
    }

    fn set_local_path(&mut self, path: PathBuf) {
        self.path = path;
    }

    fn create_user(&mut self, user_name: String, password: String) -> Result<(), IOError> {

        println!("local create {} {}", user_name, password);

        Err(UserNotFound)
    }

}