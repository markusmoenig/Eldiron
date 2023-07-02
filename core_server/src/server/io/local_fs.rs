use crate::prelude::*;

// This implementation is for local games with a single user. No login required.

struct LocalUserFS {

    path                : PathBuf,
}

impl ServerIO for LocalUserFS {

    fn new() -> Self where Self: Sized {
        Self {
            path        : PathBuf::new(),
        }
    }

    fn set_local_path(&mut self, path: PathBuf) {
        self.path = path;
    }

}