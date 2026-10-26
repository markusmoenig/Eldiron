use super::*;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    time::Instant,
};
impl Manager {
    pub(super) fn locked<T>(
        &self,
        f: impl FnOnce(&mut Store) -> Result<T, String>,
    ) -> Result<T, String> {
        self.locked_for(Duration::from_millis(100), f)
    }
    fn locked_for<T>(
        &self,
        wait: Duration,
        f: impl FnOnce(&mut Store) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut dir = std::fs::DirBuilder::new();
        dir.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            dir.mode(0o700);
        }
        dir.create(&self.directory)
            .map_err(|_| "Cannot create local ChatGPT settings directory.")?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options
            .open(self.directory.join("session.lock"))
            .map_err(|_| "Cannot lock ChatGPT credentials.")?;
        let start = Instant::now();
        loop {
            match lock.try_lock() {
                Ok(()) => break,
                Err(std::fs::TryLockError::WouldBlock) if start.elapsed() < wait => {
                    std::thread::sleep(Duration::from_millis(20))
                }
                _ => {
                    return Err(
                        "ChatGPT credentials are busy in another process. Try again.".into(),
                    );
                }
            }
        }
        let result = (|| {
            let mut store = match std::fs::read(self.directory.join("accounts.json")) {
                Ok(bytes) if bytes.len() <= MAX_JSON => serde_json::from_slice::<Store>(&bytes)
                    .map_err(|_| "Cannot read local ChatGPT credentials.")?,
                Ok(_) => return Err("ChatGPT credential file is too large.".into()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Store::default(),
                Err(_) => return Err("Cannot open local ChatGPT credentials.".into()),
            };
            if store.version != 1 {
                return Err("Unsupported ChatGPT credential version.".into());
            }
            f(&mut store)
        })();
        let _ = File::unlock(&lock);
        result
    }
    pub(super) fn save(&self, store: &Store) -> Result<(), String> {
        let mut temporary = tempfile::NamedTempFile::new_in(&self.directory)
            .map_err(|_| "Cannot write ChatGPT credentials.")?;
        serde_json::to_writer(temporary.as_file_mut(), store)
            .map_err(|_| "Cannot encode ChatGPT credentials.")?;
        temporary
            .flush()
            .and_then(|_| temporary.as_file().sync_all())
            .map_err(|_| "Cannot save ChatGPT credentials.")?;
        temporary
            .persist(self.directory.join("accounts.json"))
            .map_err(|_| "Cannot replace ChatGPT credentials.")?;
        Ok(())
    }
    pub(super) fn access_token(&self, id: Uuid, timeout: Duration) -> Result<String, String> {
        self.locked_for(Duration::from_secs(12), |store| {
            let account = store
                .accounts
                .iter_mut()
                .find(|a| a.id == id)
                .ok_or("Unknown ChatGPT account.")?;
            let tokens = account
                .tokens
                .as_ref()
                .ok_or("ChatGPT is signed out. Sign in again.")?;
            if !tokens.scopes.iter().any(|s| s == DIRECT_SCOPE) {
                return Err("ChatGPT plan usage is not authorized.".into());
            }
            if tokens.expires_at > now() + 45 {
                return Ok(tokens.access_token.clone());
            }
            let refresh = tokens
                .refresh_token
                .as_deref()
                .ok_or("ChatGPT sign-in expired. Sign in again.")?;
            let response = agent(timeout).post(&self.refresh_endpoint()).send_form(&[
                ("grant_type", "refresh_token"),
                ("client_id", &account.client_id),
                ("refresh_token", refresh),
                ("resource", RESOURCE),
            ]);
            let response = match response {
                Ok(response) => read_json(response)?,
                Err(error) => {
                    let message = http_error(error);
                    if message.contains("(invalid_grant)") {
                        account.tokens = None;
                        self.save(store)?;
                    }
                    return Err(message);
                }
            };
            let updated = auth::tokens_from_response(&response, Some(tokens))?;
            if !updated.scopes.iter().any(|s| s == DIRECT_SCOPE) {
                return Err(
                    "ChatGPT plan permission is no longer available. Sign in again.".into(),
                );
            }
            let access = updated.access_token.clone();
            account.tokens = Some(updated);
            self.save(store)?;
            Ok(access)
        })
    }
    /// Clears locally even if remote revocation cannot be confirmed. Retains mapping/host ID.
    pub fn sign_out(&self) -> Result<String, String> {
        self.locked(|store| {
            let account=store.accounts.iter_mut().find(|a|Some(a.id)==store.selected).ok_or("No ChatGPT account selected.")?;
            let revoked=if let Some(refresh)=account.tokens.as_ref().and_then(|t|t.refresh_token.as_deref()) {
                (|| {
                    let metadata=discovery()?;
                    let endpoint=approved_auth_url(metadata["revocation_endpoint"].as_str().ok_or("Missing ChatGPT revocation endpoint.")?)?;
                    for retry in 0..2 {
                        match agent(Duration::from_secs(8)).post(&endpoint).send_form(&[("token",refresh),("token_type_hint","refresh_token"),("client_id",&account.client_id)]) {
                            Ok(response) if response.status()==200=>return Ok(()),
                            Err(ureq::Error::Status(code,_)) if code<500=>break,
                            _ if retry==0=>std::thread::sleep(Duration::from_millis(250)),
                            _=>break,
                        }
                    }
                    Err("Revocation not confirmed.".to_string())
                })().is_ok()
            } else {true};
            account.tokens=None;self.save(store)?;
            Ok(if revoked {"Signed out of ChatGPT."} else {"Signed out locally. Remote revocation was not confirmed; disconnect Eldiron in ChatGPT Settings."}.into())
        })
    }
}
