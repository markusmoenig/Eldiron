use super::Location;
use std::{fmt, path::PathBuf};

/// Represents a parser error.
#[derive(Debug)]
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
            if let Some(_file) = self.path.to_str() {
                // write!(f, "{} in {} at line {}.", self.message, file, self.line)
                write!(f, "{} at line {}.", self.message, self.line)
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
            if let Some(_file) = self.path.to_str() {
                // write!(f, "{} in {} at line {}.", self.message, file, self.line)
                write!(f, "{} at line {}.", self.message, self.line)
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

/// Unified VM error for parse/compile steps.
pub enum VMError {
    Parse(ParseError),
    Compile(RuntimeError),
}

impl fmt::Debug for VMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl fmt::Display for VMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VMError::Parse(e) => write!(f, "{e}"),
            VMError::Compile(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for VMError {}

impl From<ParseError> for VMError {
    fn from(value: ParseError) -> Self {
        VMError::Parse(value)
    }
}

impl From<RuntimeError> for VMError {
    fn from(value: RuntimeError) -> Self {
        VMError::Compile(value)
    }
}

impl VMError {
    /// Return the 1-based line number if available.
    pub fn line(&self) -> Option<usize> {
        match self {
            VMError::Parse(err) => Some(err.line).filter(|l| *l > 0),
            VMError::Compile(err) => Some(err.line).filter(|l| *l > 0),
        }
    }

    /// Return the underlying error message text.
    pub fn text(&self) -> &str {
        match self {
            VMError::Parse(err) => &err.message,
            VMError::Compile(err) => &err.message,
        }
    }
}
