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
    #[serde(default = "default_local_password")]
    pub local_password: bool,
}
fn default_local_password() -> bool {
    true
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Directory {
    pub accounts: BTreeMap<String, Account>,
    #[serde(default)]
    pub identities: BTreeMap<String, ExternalIdentity>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ExternalIdentity {
    pub issuer: String,
    pub subject: String,
    pub account: Uuid,
}
pub fn identity_key(issuer: &str, subject: &str) -> String {
    use sha2::{Digest, Sha256};
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(issuer, subject)).expect("identity strings"))
    )
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
/// An external-only record must remain unguessable even if an older binary
/// ignores its local-password flag. Discard the random password after hashing.
pub fn external_placeholder_hash() -> Result<String, ()> {
    use rand::RngCore;
    let mut random = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut random);
    hash(
        &random
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
    )
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
    #[test]
    fn external_only_fallback_is_random_and_unknown_to_older_password_readers() {
        let a = external_placeholder_hash().unwrap();
        let b = external_placeholder_hash().unwrap();
        assert_ne!(a, b);
        assert!(!verify("dummy missing account password", &a));
        assert!(!verify("dummy missing account password", &b));
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
        if self.identities.len() > 8192
            || self.identities.iter().any(|(key, identity)| {
                *key != identity_key(&identity.issuer, &identity.subject)
                    || identity.issuer.is_empty()
                    || identity.issuer.len() > 2048
                    || identity.subject.is_empty()
                    || identity.subject.len() > 256
                    || self.account(identity.account).is_none()
            })
        {
            return Err("Invalid private external identity directory".into());
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
            a.local_password = true;
        }
        self.validate()
    }
}
