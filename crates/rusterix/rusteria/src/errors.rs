use crate::Location;
use std::{fmt, path::PathBuf};

/// Represents a parser error.
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub path: PathBuf,
}

impl ParseError {
    pub fn new<M>(message: M, line: usize, path: &PathBuf) -> Self
    where
        M: Into<String>,
    {
        Self {
            message: message.into(),
            line,
            path: path.clone(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            if let Some(file) = self.path.to_str() {
                write!(f, "{} in {} at line {}.", self.message, file, self.line)
            } else {
                write!(f, "{} in <unknown file>.", self.message)
            }
        } else {
            if let Some(file) = self.path.to_str() {
                write!(f, "{}: \"{}\".", self.message, file)
            } else {
                write!(f, "{} in <unknown file>.", self.message)
            }
        }
    }
}

#[derive(Debug)]
pub struct RuntimeError {
    pub message: String,
    pub line: usize,
    pub path: PathBuf,
}

impl RuntimeError {
    pub fn new<M>(message: M, loc: &Location) -> Self
    where
        M: Into<String>,
    {
        Self {
            message: message.into(),
            line: loc.line,
            path: loc.path.clone(),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            if let Some(file) = self.path.to_str() {
                write!(f, "{} in {} at line {}.", self.message, file, self.line)
            } else {
                write!(f, "{} in <unknown file>.", self.message)
            }
        } else {
            if let Some(file) = self.path.to_str() {
                write!(f, "{}: \"{}\".", self.message, file)
            } else {
                write!(f, "{} in <unknown file>.", self.message)
            }
        }
    }
}
