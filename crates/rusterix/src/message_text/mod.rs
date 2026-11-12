//! CPU message parsing and contextual/localized resolution, shared by runtime and presentation.
pub mod parser;
pub mod resolver;
pub use parser::{MsgParser, Tok};
pub use resolver::{MessageContext, MsgResolver};
