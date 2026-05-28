//! Eldiron Scepter automation command model.
//!
//! This crate intentionally starts as a small, platform-neutral core. Creator,
//! Eldrin automation, JSON-RPC, CLI tools, and AI integrations should all be
//! adapters over these command and Lorebook definitions.

mod command;
mod lorebook;
mod plan;

pub use command::*;
pub use lorebook::*;
pub use plan::*;
