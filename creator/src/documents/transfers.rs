//! One immutable, addressed document save at a time. This is CPU data, not a
//! renderer/canvas snapshot; the host copies bytes before doing its own I/O.
use super::*;
#[derive(Clone, Copy, serde::Serialize)]
pub(crate) struct SaveInfo {
    pub ticket: u64,
    pub document: Uuid,
    pub length: usize,
}
pub(crate) struct PreparedSave {
    pub info: SaveInfo,
    pub bytes: Vec<u8>,
    pub source: Vec<u8>,
    pub public_export: bool,
}
pub(crate) struct Transfers {
    next: u64,
    pending: Option<PreparedSave>,
}
impl Default for Transfers {
    fn default() -> Self {
        Self {
            next: 1,
            pending: None,
        }
    }
}
impl Transfers {
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn stage(
        &mut self,
        document: Uuid,
        bytes: Vec<u8>,
        source: Vec<u8>,
    ) -> Result<SaveInfo, String> {
        if self.busy() {
            return Err("A document save is already pending".into());
        }
        let ticket = self.next;
        self.next = ticket
            .checked_add(1)
            .ok_or("Document save identity exhausted")?;
        let info = SaveInfo {
            ticket,
            document,
            length: bytes.len(),
        };
        self.pending = Some(PreparedSave {
            info,
            bytes,
            source,
            public_export: false,
        });
        Ok(info)
    }
    pub fn stage_export(&mut self, document: Uuid, bytes: Vec<u8>) -> Result<SaveInfo, String> {
        let info = self.stage(document, bytes, Vec::new())?;
        self.pending.as_mut().unwrap().public_export = true;
        Ok(info)
    }
    pub fn bytes(&self, ticket: u64) -> Option<&[u8]> {
        self.pending
            .as_ref()
            .filter(|save| save.info.ticket == ticket)
            .map(|save| save.bytes.as_slice())
    }
    pub fn take(&mut self, ticket: u64) -> Option<PreparedSave> {
        if self.pending.as_ref()?.info.ticket != ticket {
            return None;
        }
        self.pending.take()
    }
    pub fn cancel_document(&mut self, document: Uuid) {
        if self
            .pending
            .as_ref()
            .is_some_and(|save| save.info.document == document)
        {
            self.pending = None;
        }
    }
}
