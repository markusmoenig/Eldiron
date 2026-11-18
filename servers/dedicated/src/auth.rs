//! Local credentials are one login method attached to a stable account ID.
//! Future providers resolve to the same IDs without changing character owners.
use argon2::{
    Algorithm, Argon2, Params, PasswordHasher, PasswordVerifier, Version,
    password_hash::{PasswordHash, SaltString, rand_core::OsRng},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};
use theframework_core::Uuid;
#[derive(Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    #[serde(default)]
    pub security_revision: u64,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Directory {
    pub accounts: BTreeMap<String, Account>,
}
pub type SharedDirectory = Arc<RwLock<Directory>>;
fn algorithm() -> Argon2<'static> {
    Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(19_456, 2, 1, None).expect("fixed Argon2 parameters"),
    )
}
pub fn username(value: &str) -> Option<String> {
    ((3..=32).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'))
    .then(|| value.to_ascii_lowercase())
}
pub fn hash(password: &str) -> Result<String, ()> {
    if !(12..=256).contains(&password.len()) {
        return Err(());
    }
    algorithm()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|h| h.to_string())
        .map_err(|_| ())
}
pub fn verify(password: &str, encoded: &str) -> bool {
    if !(12..=256).contains(&password.len()) {
        return false;
    }
    PasswordHash::new(encoded).ok().is_some_and(|hash| {
        algorithm()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credentials_are_salted_and_canonicalized() {
        let a = hash("correct horse battery").unwrap();
        let b = hash("correct horse battery").unwrap();
        assert_ne!(a, b);
        assert!(a.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
        assert!(verify("correct horse battery", &a));
        assert!(!verify("incorrect password", &a));
        assert!(hash("short").is_err());
        assert_eq!(username("Test_User"), Some("test_user".into()));
        assert!(username("../root").is_none());
        let id = Uuid::new_v4();
        let legacy: Account = serde_json::from_value(
            serde_json::json!({"id":id,"username":"tester","password_hash":a}),
        )
        .unwrap();
        assert_eq!(legacy.security_revision, 0);
        let mut directory = Directory::default();
        directory.accounts.insert("tester".into(), legacy);
        directory.validate().unwrap();
        directory.revise(id, Some(b)).unwrap();
        assert_eq!(directory.account(id).unwrap().security_revision, 1);
        assert_eq!(directory.account(id).unwrap().id, id);
    }
}

impl Directory {
    pub fn account(&self, id: Uuid) -> Option<&Account> {
        self.accounts.values().find(|a| a.id == id)
    }
    pub fn validate(&self) -> Result<(), String> {
        let mut ids = std::collections::BTreeSet::new();
        if self.accounts.len() > 4096 {
            return Err("Account directory exceeds capacity".into());
        }
        for (key, account) in &self.accounts {
            if username(key).as_ref() != Some(key)
                || account.username != *key
                || account.id.is_nil()
                || !ids.insert(account.id)
                || PasswordHash::new(&account.password_hash).is_err()
            {
                return Err("Invalid private account directory".into());
            }
        }
        Ok(())
    }
    pub fn revise(&mut self, id: Uuid, password_hash: Option<String>) -> Result<(), String> {
        let a = self
            .accounts
            .values_mut()
            .find(|a| a.id == id)
            .ok_or("Account not found")?;
        a.security_revision = a
            .security_revision
            .checked_add(1)
            .ok_or("Account security revision exhausted")?;
        if let Some(hash) = password_hash {
            a.password_hash = hash;
        }
        self.validate()
    }
}
