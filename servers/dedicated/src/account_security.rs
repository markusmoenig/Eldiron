//! Serialized account mutations. Hash workers return a captured security revision;
//! only this owner can publish a new revision and revoke live credentials.
use super::*;
impl Host {
    pub(super) fn password_context(&self, peer: PeerId) -> Result<Account, RejectCode> {
        if self.world.mode() != HostMode::Persistent {
            return Err(RejectCode::WrongMode);
        }
        let account = self
            .peers
            .get(&peer)
            .filter(|p| p.hello)
            .and_then(|p| p.account)
            .ok_or(RejectCode::AuthenticationRequired)?;
        self.directory
            .read()
            .unwrap()
            .account(account)
            .cloned()
            .ok_or(RejectCode::InvalidCredentials)
    }
    async fn publish_security(&mut self, directory: Directory) -> Result<(), SaveError> {
        let data = serde_json::to_value(&directory).map_err(failure)?;
        let World::Saved(session) = &mut self.world else {
            return Err(failure("Account changes require persistent storage"));
        };
        session.commit_host_metadata("accounts.v1", data).await?;
        *self.directory.write().unwrap() = directory;
        Ok(())
    }
    fn revoke_account(&mut self, account: Uuid) {
        self.tokens.retain(|_, t| t.account != account);
        let peers: Vec<_> = self
            .peers
            .iter()
            .filter(|(_, p)| p.account == Some(account))
            .map(|(id, _)| *id)
            .collect();
        for peer in peers {
            self.disconnect(peer);
        }
    }
    pub(super) async fn password_verified(
        &mut self,
        peer: PeerId,
        account: Uuid,
        revision: u64,
        hash: String,
    ) -> Result<(), SaveError> {
        let valid = self
            .password_context(peer)
            .is_ok_and(|a| a.id == account && a.security_revision == revision);
        if !valid {
            self.send(
                peer,
                ServerMessage::AccountUpdated(AccountUpdate::Rejected(
                    RejectCode::InvalidCredentials,
                )),
            );
            return Ok(());
        }
        let mut directory = self.directory.read().unwrap().clone();
        directory.revise(account, Some(hash)).map_err(failure)?;
        self.publish_security(directory).await?;
        self.send(
            peer,
            ServerMessage::AccountUpdated(AccountUpdate::PasswordChanged),
        );
        self.revoke_account(account);
        Ok(())
    }
    pub(super) async fn logout_all(&mut self, peer: PeerId) -> Result<(), SaveError> {
        let account = match self.password_context(peer) {
            Ok(a) => a,
            Err(code) => {
                self.send(
                    peer,
                    ServerMessage::AccountUpdated(AccountUpdate::Rejected(code)),
                );
                return Ok(());
            }
        };
        let mut directory = self.directory.read().unwrap().clone();
        directory.revise(account.id, None).map_err(failure)?;
        self.publish_security(directory).await?;
        self.send(
            peer,
            ServerMessage::AccountUpdated(AccountUpdate::LoggedOutEverywhere),
        );
        self.revoke_account(account.id);
        Ok(())
    }
}
