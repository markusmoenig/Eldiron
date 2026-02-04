use std::fs;
use std::io;
use std::path::Path;

/// A trait to unify input handling for loading any type of data (e.g., from files or memory).
pub trait IntoDataInput {
    /// Generic loader method, returns the data as a string or a byte buffer.
    fn load_data(self) -> Result<Vec<u8>, io::Error>;
}

impl IntoDataInput for &Path {
    fn load_data(self) -> Result<Vec<u8>, io::Error> {
        fs::read(self)
    }
}

impl IntoDataInput for &str {
    fn load_data(self) -> Result<Vec<u8>, io::Error> {
        Ok(self.as_bytes().to_vec())
    }
}

impl IntoDataInput for &[u8] {
    fn load_data(self) -> Result<Vec<u8>, io::Error> {
        Ok(self.to_vec())
    }
}

impl IntoDataInput for String {
    fn load_data(self) -> Result<Vec<u8>, io::Error> {
        Ok(self.into_bytes())
    }
}
