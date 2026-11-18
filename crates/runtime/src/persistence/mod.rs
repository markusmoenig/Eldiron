//! Complete world checkpoints and staged, idempotent durable operations.
mod journal;
mod records;
pub use journal::{ActionJournal, ActionReceipt, MAX_ACTION_RECEIPTS};
mod session;
mod store;
pub use records::*;
pub use rusterix::server::persistence::{RegionRecord, SavedEntity, SavedItem, WorldRecord};
pub use session::DurableSession;
#[cfg(target_arch = "wasm32")]
pub use store::BrowserWorldStore;
#[cfg(not(target_arch = "wasm32"))]
pub use store::FileWorldStore;
pub use store::{MemoryWorldStore, StoreDiagnostics, WorldStore};

#[cfg(not(target_arch = "wasm32"))]
mod archive;
#[cfg(not(target_arch = "wasm32"))]
pub use archive::{read_snapshot, write_snapshot_new};

mod incremental;
pub use incremental::{MAX_GROUP_ACTIONS, PreparedUpdate};

#[cfg(not(target_arch = "wasm32"))]
mod journal_file;
