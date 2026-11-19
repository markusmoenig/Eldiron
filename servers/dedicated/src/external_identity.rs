//! Provider results resolve to world-scoped account UUIDs. No email-based merge.
use super::*;
pub struct Context {
    pub attempt: Uuid,
    pub link: Option<Account>,
    pub revisions: std::collections::BTreeMap<Uuid, u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn peer(host: &mut Host) -> (Uuid, mpsc::Receiver<Vec<u8>>) {
        let id = Uuid::new_v4();
        let (output, rx) = mpsc::channel(64);
        host.command(Command::Open { peer: id, output })
            .await
            .unwrap();
        host.command(Command::Message {
            peer: id,
            message: ClientMessage::Hello {
                version: PROTOCOL_VERSION,
            },
        })
        .await
        .unwrap();
        (id, rx)
    }
    fn identity() -> crate::oidc::VerifiedIdentity {
        crate::oidc::VerifiedIdentity {
            issuer: "https://issuer.example".into(),
            subject: "stable-provider-subject".into(),
        }
    }
    #[tokio::test]
    async fn linking_retains_owner_and_stale_or_cancelled_provider_results_cannot_authorize() {
        let root = std::env::temp_dir().join(format!("eldiron-identities-{}", Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("world.save");
        let mut p = Project::new();
        p.regions[0].name = "Identity".into();
        p.regions[0].map.name = "Identity".into();
        p.config="[game]\nstart_region='Identity'\nauto_create_player=false\n[ruleset]\nsource='project'".into();
        p.rules_document.current.clear();
        p.rules_document.original.clear();
        p.preview_workbench_rules();
        let mut host = Host::open(p.clone(), Some(path.clone())).await.unwrap();
        host.configure_identity(crate::oidc::Broker::empty_for_owner_tests())
            .unwrap();
        let (a, _rx) = peer(&mut host).await;
        let hash = crate::auth::hash("local original password").unwrap();
        host.command(Command::Register {
            peer: a,
            username: "alice".into(),
            hash: hash.clone(),
        })
        .await
        .unwrap();
        let account = host.peers[&a].account.unwrap();
        let context = host.external_context(a).unwrap();
        host.external_verified(a, context, identity(), hash.clone())
            .await
            .unwrap();
        assert!(!host.peers.contains_key(&a));
        let directory = host.directory.read().unwrap().clone();
        assert_eq!(directory.accounts.len(), 1);
        assert_eq!(directory.identities.len(), 1);
        assert_eq!(directory.accounts["alice"].security_revision, 1);
        assert_eq!(
            directory.identities.values().next().unwrap().account,
            account
        );
        let (pending, _rx) = peer(&mut host).await;
        let stale = host.external_context(pending).unwrap();
        let (b, _rx) = peer(&mut host).await;
        host.command(Command::Verified {
            peer: b,
            account,
            security_revision: 1,
        })
        .await
        .unwrap();
        host.command(Command::Message {
            peer: b,
            message: ClientMessage::LogoutAll,
        })
        .await
        .unwrap();
        host.external_verified(pending, stale, identity(), hash.clone())
            .await
            .unwrap();
        assert!(host.peers[&pending].account.is_none());
        let cancelled = host.external_context(pending).unwrap();
        host.command(Command::Message {
            peer: pending,
            message: ClientMessage::CancelExternalLogin,
        })
        .await
        .unwrap();
        host.external_verified(pending, cancelled, identity(), hash.clone())
            .await
            .unwrap();
        assert!(host.peers[&pending].account.is_none());
        host.shutdown().await.unwrap();
        drop(host);
        let mut host = Host::open(p, Some(path)).await.unwrap();
        host.configure_identity(crate::oidc::Broker::empty_for_owner_tests())
            .unwrap();
        let (c, _rx) = peer(&mut host).await;
        let context = host.external_context(c).unwrap();
        host.external_verified(c, context, identity(), hash.clone())
            .await
            .unwrap();
        assert_eq!(host.peers[&c].account, Some(account));
        let (external, _rx) = peer(&mut host).await;
        let context = host.external_context(external).unwrap();
        host.external_verified(
            external,
            context,
            crate::oidc::VerifiedIdentity {
                issuer: "https://issuer.example".into(),
                subject: "new-subject".into(),
            },
            hash,
        )
        .await
        .unwrap();
        let directory = host.directory.read().unwrap().clone();
        let new = host.peers[&external].account.unwrap();
        assert_ne!(new, account);
        assert!(!directory.account(new).unwrap().local_password);
        host.command(Command::PasswordVerified {
            peer: external,
            account: new,
            security_revision: 0,
            hash: crate::auth::hash("must not enable local password").unwrap(),
        })
        .await
        .unwrap();
        assert!(
            !host
                .directory
                .read()
                .unwrap()
                .account(new)
                .unwrap()
                .local_password
        );
        assert_eq!(
            host.directory
                .read()
                .unwrap()
                .account(new)
                .unwrap()
                .security_revision,
            0
        );
        host.unlink_external(c, account, 1, "https://issuer.example")
            .await
            .unwrap();
        assert_eq!(
            host.directory
                .read()
                .unwrap()
                .account(account)
                .unwrap()
                .security_revision,
            2
        );
        host.unlink_external(c, account, 2, "https://issuer.example")
            .await
            .unwrap();
        let directory = host.directory.read().unwrap().clone();
        assert_eq!(directory.account(account).unwrap().security_revision, 3);
        assert!(
            !directory
                .identities
                .values()
                .any(|identity| identity.account == account)
        );
        assert!(
            directory
                .identities
                .values()
                .any(|identity| identity.account == new)
        );
        assert!(host.peers[&external].account == Some(new));
        host.shutdown().await.unwrap();
        drop(host);
        std::fs::remove_dir_all(root).unwrap();
    }
}
impl Host {
    pub(super) async fn unlink_external(
        &mut self,
        peer: PeerId,
        account: Uuid,
        revision: u64,
        issuer: &str,
    ) -> Result<(), SaveError> {
        if !self
            .password_context(peer)
            .is_ok_and(|a| a.id == account && a.security_revision == revision && a.local_password)
        {
            self.send(
                peer,
                ServerMessage::AccountUpdated(AccountUpdate::Rejected(
                    RejectCode::InvalidCredentials,
                )),
            );
            return Ok(());
        }
        let mut directory = self.directory.read().unwrap().clone();
        let count = directory.identities.len();
        directory
            .identities
            .retain(|_, identity| identity.account != account || identity.issuer != issuer);
        if directory.identities.len() == count {
            self.send(
                peer,
                ServerMessage::AccountUpdated(AccountUpdate::Rejected(RejectCode::InvalidRequest)),
            );
            return Ok(());
        }
        directory.revise(account, None).map_err(failure)?;
        self.publish_security(directory).await?;
        self.send(
            peer,
            ServerMessage::AccountUpdated(AccountUpdate::IdentityUnlinked),
        );
        self.revoke_account(account);
        Ok(())
    }
    pub fn configure_identity(&mut self, broker: Arc<crate::oidc::Broker>) -> Result<(), String> {
        if !self.peers.is_empty() || self.world.mode() != HostMode::Persistent {
            return Err("Identity providers require a persistent host before admission".into());
        }
        self.identity = Some(broker);
        Ok(())
    }
    pub(super) fn external_context(&mut self, peer: PeerId) -> Result<Context, RejectCode> {
        if self.identity.is_none() || self.world.mode() != HostMode::Persistent {
            return Err(RejectCode::WrongMode);
        }
        let p = self
            .peers
            .get_mut(&peer)
            .filter(|p| p.hello)
            .ok_or(RejectCode::AuthenticationRequired)?;
        if p.external_attempt.is_some() {
            return Err(RejectCode::Unavailable);
        }
        let directory = self.directory.read().unwrap();
        let link = p
            .account
            .map(|id| {
                directory
                    .account(id)
                    .cloned()
                    .ok_or(RejectCode::InvalidCredentials)
            })
            .transpose()?;
        if link.as_ref().is_some_and(|account| !account.local_password) {
            return Err(RejectCode::InvalidCredentials);
        }
        let attempt = Uuid::new_v4();
        p.external_attempt = Some(attempt);
        Ok(Context {
            attempt,
            link,
            revisions: directory
                .accounts
                .values()
                .map(|a| (a.id, a.security_revision))
                .collect(),
        })
    }
    pub(super) async fn external_verified(
        &mut self,
        peer: PeerId,
        context: Context,
        identity: crate::oidc::VerifiedIdentity,
        placeholder_hash: String,
    ) -> Result<(), SaveError> {
        let Some(p) = self
            .peers
            .get_mut(&peer)
            .filter(|p| p.hello && p.external_attempt == Some(context.attempt))
        else {
            return Ok(());
        };
        p.external_attempt = None;
        let selected = p.account;
        let mut directory = self.directory.read().unwrap().clone();
        let key = crate::auth::identity_key(&identity.issuer, &identity.subject);
        let existing = directory.identities.get(&key).map(|i| i.account);
        let account = if let Some(link) = context.link {
            if selected != Some(link.id)
                || directory
                    .account(link.id)
                    .is_none_or(|a| a.security_revision != link.security_revision)
                || existing.is_some_and(|id| id != link.id)
            {
                self.reject(peer, RejectCode::InvalidCredentials);
                return Ok(());
            }
            directory.identities.insert(
                key,
                crate::auth::ExternalIdentity {
                    issuer: identity.issuer,
                    subject: identity.subject,
                    account: link.id,
                },
            );
            directory.revise(link.id, None).map_err(failure)?;
            self.publish_security(directory).await?;
            self.send(
                peer,
                ServerMessage::AccountUpdated(AccountUpdate::IdentityLinked),
            );
            self.revoke_account(link.id);
            return Ok(());
        } else if selected.is_some() {
            self.reject(peer, RejectCode::InvalidCredentials);
            return Ok(());
        } else if let Some(id) = existing {
            if directory
                .account(id)
                .is_none_or(|a| context.revisions.get(&id) != Some(&a.security_revision))
            {
                self.reject(peer, RejectCode::InvalidCredentials);
                return Ok(());
            }
            id
        } else {
            if directory.accounts.len() >= 4096 || directory.identities.len() >= 8192 {
                self.reject(peer, RejectCode::Unavailable);
                return Ok(());
            }
            let id = Uuid::new_v4();
            let username = format!("ext_{}", &id.simple().to_string()[..24]);
            directory.accounts.insert(
                username.clone(),
                Account {
                    id,
                    username,
                    password_hash: placeholder_hash,
                    security_revision: 0,
                    local_password: false,
                },
            );
            directory.identities.insert(
                key,
                crate::auth::ExternalIdentity {
                    issuer: identity.issuer,
                    subject: identity.subject,
                    account: id,
                },
            );
            directory.validate().map_err(failure)?;
            self.publish_security(directory).await?;
            id
        };
        self.authorize(peer, account);
        Ok(())
    }
}
