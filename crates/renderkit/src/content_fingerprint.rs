//! Fast bucket selection for immutable content. Cache hits still compare every
//! input byte; this fingerprint is never a resource identity or equality proof.
use std::hash::{Hash, Hasher};

pub(crate) fn content_fingerprint(bytes: &[u8]) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    bytes.hash(&mut hasher);
    hasher.finish()
}
