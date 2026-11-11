//! Backend-neutral completed authoring edits. Consumers validate binding/source
//! ownership and serial order before applying packets to an authoritative graph.
use super::*;

/// One completed gesture produces one reversible edit. The host can persist/undo it.
#[derive(Clone, Debug, PartialEq)]
pub enum GraphEdit {
    AddNode {
        node: GraphNode,
    },
    Move {
        node: GraphId,
        before: Point,
        after: Point,
    },
    SetValue {
        node: GraphId,
        row: GraphId,
        before: GraphControlValue,
        after: GraphControlValue,
    },
    Connection {
        connection: GraphConnection,
        added: bool,
    },
    ReplaceNode {
        before: GraphNode,
        after: GraphNode,
    },
}
/// A completed reversible transaction addressed to one authoring document.
/// The application acknowledges serials after applying them to its authoritative
/// document. Save barriers can collect pending commits synchronously; later UI
/// wake-up events for acknowledged serials then become harmless notifications.
#[derive(Clone, Debug)]
pub struct GraphCommit {
    pub binding: u64,
    pub serial: u64,
    /// Process-wide publication order for merging retained sources into one
    /// application history. Serial acknowledgement remains per binding.
    pub sequence: u64,
    pub edits: Vec<GraphEdit>,
}

impl GraphCommit {
    pub fn new(binding: u64, serial: u64, edits: Vec<GraphEdit>) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_SEQUENCE: AtomicU64 = AtomicU64::new(1);
        let mut sequence = NEXT_SEQUENCE.load(Ordering::Relaxed);
        loop {
            let next = sequence
                .checked_add(1)
                .expect("graph commit sequence exhausted");
            match NEXT_SEQUENCE.compare_exchange_weak(
                sequence,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => sequence = actual,
            }
        }
        Self {
            binding,
            serial,
            sequence,
            edits,
        }
    }
}

impl GraphEdit {
    pub fn apply(&self, doc: &mut GraphDocument, forward: bool) {
        match self {
            Self::AddNode { node } => {
                doc.nodes.retain(|n| n.id != node.id);
                if forward {
                    doc.nodes.push(node.clone());
                }
            }
            Self::ReplaceNode { before, after } => {
                if let Some(n) = doc.nodes.iter_mut().find(|n| n.id == before.id) {
                    *n = if forward { after } else { before }.clone();
                }
            }
            Self::Move {
                node,
                before,
                after,
            } => {
                if let Some(n) = doc.nodes.iter_mut().find(|n| n.id == *node) {
                    n.position = if forward { *after } else { *before };
                }
            }
            Self::SetValue {
                node,
                row,
                before,
                after,
            } => {
                if let Some(r) = doc
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == *node)
                    .and_then(|n| n.rows.iter_mut().find(|r| r.id == *row))
                {
                    r.value = if forward { after } else { before }.clone();
                }
            }
            Self::Connection { connection, added } => {
                doc.connections.retain(|c| c.id != connection.id);
                if *added == forward {
                    doc.connections.push(connection.clone());
                }
            }
        }
    }
}
