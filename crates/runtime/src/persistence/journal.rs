//! Durable high-water marks keep retry safety bounded without expiring old
//! action IDs back into executable requests. One stream belongs to a character.
use super::SaveError;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
pub const MAX_ACTION_RECEIPTS: usize = 128;
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActionJournal {
    pub last: u64,
    pub recent: VecDeque<ActionReceipt>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionReceipt {
    pub sequence: u64,
    pub fingerprint: String,
    pub outcome: serde_json::Value,
}
impl ActionJournal {
    pub fn next(&self) -> Result<u64, SaveError> {
        self.last
            .checked_add(1)
            .ok_or_else(|| SaveError::Invalid("Action sequence exhausted".into()))
    }
    pub fn check(
        &self,
        sequence: u64,
        fingerprint: &str,
    ) -> Result<Option<serde_json::Value>, SaveError> {
        if sequence == 0 || fingerprint.len() != 64 {
            return Err(SaveError::Invalid("Invalid action identity".into()));
        }
        if sequence <= self.last {
            return match self.recent.iter().find(|r| r.sequence == sequence) {
                Some(receipt) if receipt.fingerprint == fingerprint => {
                    Ok(Some(receipt.outcome.clone()))
                }
                Some(_) => Err(SaveError::Conflict),
                None => Err(SaveError::AlreadyCommitted),
            };
        }
        if sequence != self.next()? {
            return Err(SaveError::ActionSequence { next: self.next()? });
        }
        Ok(None)
    }
    pub fn append(
        &mut self,
        sequence: u64,
        fingerprint: String,
        outcome: serde_json::Value,
    ) -> Result<(), SaveError> {
        if self.check(sequence, &fingerprint)?.is_some() {
            return Err(SaveError::Conflict);
        }
        self.last = sequence;
        self.recent.push_back(ActionReceipt {
            sequence,
            fingerprint,
            outcome,
        });
        while self.recent.len() > MAX_ACTION_RECEIPTS {
            self.recent.pop_front();
        }
        Ok(())
    }
    pub fn valid(&self) -> bool {
        self.recent.len() <= MAX_ACTION_RECEIPTS
            && self.recent.len() as u64 <= self.last
            && (self.last == 0 && self.recent.is_empty()
                || self.last > 0 && self.recent.back().is_some_and(|r| r.sequence == self.last))
            && self.recent.iter().enumerate().all(|(i, r)| {
                r.sequence == self.last - self.recent.len() as u64 + 1 + i as u64
                    && r.fingerprint.len() == 64
            })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_receipts_never_make_old_actions_executable_again() {
        let mut journal = ActionJournal::default();
        let fingerprint = "a".repeat(64);
        for sequence in 1..=300 {
            journal
                .append(sequence, fingerprint.clone(), serde_json::json!(sequence))
                .unwrap();
        }
        assert_eq!(journal.recent.len(), MAX_ACTION_RECEIPTS);
        assert!(journal.valid());
        assert_eq!(
            journal.check(1, &fingerprint),
            Err(SaveError::AlreadyCommitted)
        );
        assert_eq!(
            journal.check(300, &fingerprint).unwrap(),
            Some(serde_json::json!(300))
        );
        assert_eq!(
            journal.check(300, &"b".repeat(64)),
            Err(SaveError::Conflict)
        );
        assert_eq!(
            journal.check(302, &fingerprint),
            Err(SaveError::ActionSequence { next: 301 })
        );
    }
}
