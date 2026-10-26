use super::*;
use ring::{
    digest,
    rand::{SecureRandom, SystemRandom},
    signature,
};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::TcpListener,
    sync::atomic::Ordering,
    time::Instant,
};

pub(super) struct Attempt {
    listener: TcpListener,
    pub(super) url: url::Url,
    state: String,
    nonce: String,
    verifier: String,
    redirect: String,
    target: Option<Account>,
}
fn random() -> Result<String, String> {
    let mut bytes = [0; 32];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| "Cannot generate secure sign-in values.")?;
    Ok(B64.encode(bytes))
}
impl Manager {
    pub(super) fn begin_sign_in(&self, new_account: bool) -> Result<Attempt, String> {
        let (host, target) = self.locked(|store| {
            self.save(store)?;
            Ok((
                store.host_id.clone(),
                if new_account {
                    None
                } else {
                    store
                        .accounts
                        .iter()
                        .find(|a| Some(a.id) == store.selected)
                        .cloned()
                },
            ))
        })?;
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|_| "Cannot start the local ChatGPT sign-in callback.")?;
        listener
            .set_nonblocking(true)
            .map_err(|_| "Cannot configure sign-in listener.")?;
        let redirect = format!(
            "http://127.0.0.1:{}/auth/callback",
            listener
                .local_addr()
                .map_err(|_| "Cannot locate callback port.")?
                .port()
        );
        let state = random()?;
        let nonce = random()?;
        let verifier = random()?;
        let challenge = B64.encode(digest::digest(&digest::SHA256, verifier.as_bytes()));
        let mut url = url::Url::parse(&format!("{ISSUER}/api/accounts/authorize")).unwrap();
        {
            let mut query = url.query_pairs_mut();
            query.extend_pairs([
                (
                    "client_id",
                    target
                        .as_ref()
                        .map(|a| a.client_id.as_str())
                        .unwrap_or("dynamic_agent_client"),
                ),
                ("ext_agent_host_id", host.as_str()),
                ("response_type", "code"),
                ("redirect_uri", redirect.as_str()),
                (
                    "scope",
                    "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct",
                ),
                ("resource", RESOURCE),
                ("state", state.as_str()),
                ("nonce", nonce.as_str()),
                ("code_challenge_method", "S256"),
                ("code_challenge", challenge.as_str()),
            ]);
            if target.is_none() {
                query.append_pair("agent_name_hint", "Eldiron");
            }
            // No id_token_hint in URLs: interactive returning-account selection is intentional.
        }
        Ok(Attempt {
            listener,
            url,
            state,
            nonce,
            verifier,
            redirect,
            target,
        })
    }
    pub(super) fn finish_sign_in(
        &self,
        attempt: Attempt,
        cancel: &AtomicBool,
    ) -> Result<AccountInfo, String> {
        let start = Instant::now();
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("ChatGPT sign-in cancelled.".into());
            }
            if start.elapsed() > Duration::from_secs(300) {
                return Err("ChatGPT sign-in timed out. Try again.".into());
            }
            let (mut stream, _) = match attempt.listener.accept() {
                Ok(value) => value,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                Err(_) => return Err("ChatGPT callback listener stopped.".into()),
            };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
            let mut line = vec![];
            while line.len() < 8192 && !line.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                if stream.read_exact(&mut byte).is_err() {
                    break;
                }
                line.push(byte[0]);
            }
            let parsed = std::str::from_utf8(&line)
                .ok()
                .and_then(|line| line.strip_prefix("GET "))
                .and_then(|line| line.split_once(' '))
                .map(|(path, _)| path);
            let callback = parsed.and_then(|path| parse_callback(&attempt, path).ok());
            let Some(callback) = callback else {
                let _ = write!(
                    stream,
                    "HTTP/1.1 400 Bad Request\r\nConnection: close\r\nContent-Length: 16\r\n\r\nInvalid callback"
                );
                continue;
            };
            let response = "ChatGPT authorization received. You can return to Eldiron.";
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                response.len(),
                response
            );
            drop(stream);
            if callback.contains_key("error") {
                return Err("ChatGPT authorization was denied or cancelled. Your existing account was kept.".into());
            }
            let client_id = callback_client(&attempt, &callback)?;
            let code = callback
                .get("code")
                .filter(|s| !s.is_empty())
                .ok_or("ChatGPT callback has no authorization code.")?;
            let response = agent(Duration::from_secs(15))
                .post(&format!("{ISSUER}/api/accounts/oauth/token"))
                .send_form(&[
                    ("grant_type", "authorization_code"),
                    ("client_id", client_id.as_str()),
                    ("code", code.as_str()),
                    ("code_verifier", &attempt.verifier),
                    ("redirect_uri", &attempt.redirect),
                    ("resource", RESOURCE),
                ])
                .map_err(http_error)?;
            let data = read_json(response)?;
            let tokens = tokens_from_response(&data, None)?;
            let metadata = discovery()?;
            let jwks_url = approved_auth_url(
                metadata["jwks_uri"]
                    .as_str()
                    .ok_or("ChatGPT discovery has no signing keys.")?,
            )?;
            let jwks = read_json(
                agent(Duration::from_secs(10))
                    .get(&jwks_url)
                    .call()
                    .map_err(http_error)?,
            )?;
            let identity =
                validate_identity(&tokens.id_token, &jwks, &client_id, &attempt.nonce, now())?;
            if let Some(target) = &attempt.target {
                if identity.0 != target.subject {
                    return Err("The signed-in identity does not match the selected ChatGPT account. Use Add account for another identity.".into());
                }
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("ChatGPT sign-in cancelled.".into());
            }
            return self.locked(|store| {
                if cancel.load(Ordering::Relaxed) {
                    return Err("ChatGPT sign-in cancelled.".into());
                }
                let id = attempt
                    .target
                    .as_ref()
                    .map(|a| a.id)
                    .unwrap_or_else(Uuid::new_v4);
                let account = Account {
                    id,
                    subject: identity.0,
                    email: identity.1,
                    client_id,
                    model: attempt.target.as_ref().and_then(|a| a.model.clone()),
                    tokens: Some(tokens),
                };
                let info = account.info();
                if let Some(existing) = store.accounts.iter_mut().find(|a| a.id == id) {
                    if existing.client_id != account.client_id
                        || existing.subject != account.subject
                    {
                        return Err("ChatGPT account registration changed while signing in.".into());
                    }
                    *existing = account;
                } else {
                    store.accounts.push(account);
                }
                store.selected = Some(id);
                self.save(store)?;
                Ok(info)
            });
        }
    }
}
fn parse_callback(attempt: &Attempt, path: &str) -> Result<BTreeMap<String, String>, String> {
    if !path.starts_with("/auth/callback?") {
        return Err("Unexpected callback path.".into());
    }
    let url =
        url::Url::parse(&format!("http://127.0.0.1{path}")).map_err(|_| "Invalid callback URL.")?;
    let mut params = BTreeMap::new();
    for (key, value) in url.query_pairs() {
        if params
            .insert(key.into_owned(), value.into_owned())
            .is_some()
        {
            return Err("Duplicate callback parameter.".into());
        }
    }
    if params.get("state") != Some(&attempt.state) {
        return Err("Invalid sign-in state.".into());
    }
    Ok(params)
}
fn callback_client(
    attempt: &Attempt,
    callback: &BTreeMap<String, String>,
) -> Result<String, String> {
    let client = match &attempt.target {
        Some(target) => {
            if callback
                .get("client_id")
                .is_some_and(|s| s != &target.client_id)
            {
                return Err("ChatGPT callback changed the registered client ID.".into());
            }
            target.client_id.clone()
        }
        None => callback
            .get("client_id")
            .filter(|s| !s.is_empty() && s.as_str() != "dynamic_agent_client")
            .cloned()
            .ok_or("ChatGPT registration did not return an issued client ID.")?,
    };
    Ok(client)
}
pub(super) fn tokens_from_response(
    data: &Json,
    previous: Option<&Tokens>,
) -> Result<Tokens, String> {
    if data["token_type"]
        .as_str()
        .is_none_or(|s| !s.eq_ignore_ascii_case("Bearer"))
    {
        return Err("ChatGPT returned an unsupported token type.".into());
    }
    let access = data["access_token"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or("ChatGPT did not return an access token.")?;
    let scopes = data["scope"]
        .as_str()
        .map(|s| s.split_whitespace().map(str::to_string).collect())
        .or_else(|| previous.map(|t| t.scopes.clone()))
        .ok_or("ChatGPT did not return granted permissions.")?;
    let id_token = data["id_token"]
        .as_str()
        .map(str::to_string)
        .or_else(|| previous.map(|t| t.id_token.clone()))
        .ok_or("ChatGPT did not return an identity token.")?;
    let expires = data["expires_in"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= 86400)
        .ok_or("ChatGPT returned an invalid token expiry.")?;
    let refresh = data["refresh_token"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    if previous.is_some() && refresh.is_none() {
        return Err(
            "ChatGPT refresh did not return replacement credentials. Sign in again.".into(),
        );
    }
    Ok(Tokens {
        access_token: access.into(),
        refresh_token: refresh,
        id_token,
        expires_at: now() + expires,
        scopes,
    })
}
fn validate_identity(
    token: &str,
    jwks: &Json,
    client: &str,
    nonce: &str,
    time: u64,
) -> Result<(String, Option<String>), String> {
    let parts = token.split('.').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err("Invalid ChatGPT identity token.".into());
    }
    let header: Json = serde_json::from_slice(
        &B64.decode(parts[0])
            .map_err(|_| "Invalid identity token header.")?,
    )
    .map_err(|_| "Invalid identity token header.")?;
    let key = jwks["keys"]
        .as_array()
        .and_then(|keys| {
            keys.iter()
                .find(|key| key["kid"].as_str().is_some() && key["kid"] == header["kid"])
        })
        .ok_or("Unknown ChatGPT identity signing key.")?;
    if key.get("alg").is_some_and(|alg| alg != &header["alg"])
        || key.get("use").is_some_and(|usage| usage != "sig")
    {
        return Err("ChatGPT signing key does not permit this signature.".into());
    }
    let signature = B64
        .decode(parts[2])
        .map_err(|_| "Invalid identity signature.")?;
    let message = format!("{}.{}", parts[0], parts[1]);
    let component = |name: &str| -> Result<Vec<u8>, String> {
        B64.decode(key[name].as_str().ok_or("Invalid identity signing key.")?)
            .map_err(|_| "Invalid identity signing key.".into())
    };
    let valid = match header["alg"].as_str() {
        Some("RS256") if key["kty"] == "RSA" => signature::RsaPublicKeyComponents {
            n: component("n")?,
            e: component("e")?,
        }
        .verify(
            &signature::RSA_PKCS1_2048_8192_SHA256,
            message.as_bytes(),
            &signature,
        ),
        Some("ES256") if key["kty"] == "EC" && key["crv"] == "P-256" => {
            let mut public = vec![4];
            public.extend(component("x")?);
            public.extend(component("y")?);
            signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_FIXED, public)
                .verify(message.as_bytes(), &signature)
        }
        _ => return Err("Unsupported ChatGPT identity signing algorithm.".into()),
    };
    valid.map_err(|_| "ChatGPT identity signature verification failed.")?;
    let claims: Json = serde_json::from_slice(
        &B64.decode(parts[1])
            .map_err(|_| "Invalid identity claims.")?,
    )
    .map_err(|_| "Invalid identity claims.")?;
    let audiences = claims["aud"].as_array();
    let audience = claims["aud"] == client
        || audiences.is_some_and(|values| values.iter().any(|v| v == client));
    if claims["iss"] != ISSUER
        || !audience
        || claims["nonce"] != nonce
        || claims["exp"].as_u64().is_none_or(|n| n <= time)
        || claims["nbf"].as_u64().is_some_and(|n| n > time + 30)
        || (audiences.is_some_and(|a| a.len() > 1) && claims["azp"] != client)
    {
        return Err("ChatGPT identity issuer, audience, nonce or expiry is invalid.".into());
    }
    let subject = claims["sub"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or("ChatGPT identity has no subject.")?
        .to_string();
    Ok((subject, claims["email"].as_str().map(clean)))
}

#[cfg(test)]
pub(super) mod test_support {
    use super::*;
    pub fn validate(
        token: &str,
        jwks: &Json,
        client: &str,
        nonce: &str,
        time: u64,
    ) -> Result<(String, Option<String>), String> {
        validate_identity(token, jwks, client, nonce, time)
    }
    pub fn returning(manager: &Manager) -> Result<(), String> {
        let attempt = manager.begin_sign_in(false)?;
        let target = attempt.target.as_ref().ok_or("Missing test account.")?;
        let params: BTreeMap<_, _> = attempt
            .url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        assert_eq!(params["client_id"], target.client_id);
        assert!(!params.contains_key("agent_name_hint"));
        let path = format!("/auth/callback?state={}&code=code", attempt.state);
        assert_eq!(
            callback_client(&attempt, &parse_callback(&attempt, &path)?)?,
            target.client_id
        );
        let changed = format!("{path}&client_id=wrong");
        assert!(callback_client(&attempt, &parse_callback(&attempt, &changed)?).is_err());
        Ok(())
    }
    pub fn denial(manager: &Manager) -> Result<(), String> {
        let attempt = manager.begin_sign_in(true)?;
        let url = format!(
            "{}?state={}&error=access_denied",
            attempt.redirect, attempt.state
        );
        let sender = std::thread::spawn(move || {
            agent(Duration::from_secs(5))
                .get(&url)
                .call()
                .map(|r| r.status())
        });
        assert!(
            manager
                .finish_sign_in(attempt, &AtomicBool::new(false))
                .unwrap_err()
                .contains("denied")
        );
        assert_eq!(sender.join().unwrap().unwrap(), 200);
        let attempt = manager.begin_sign_in(true)?;
        assert!(
            manager
                .finish_sign_in(attempt, &AtomicBool::new(true))
                .unwrap_err()
                .contains("cancelled")
        );
        Ok(())
    }
    pub fn callback(manager: &Manager) -> Result<(), String> {
        let attempt = manager.begin_sign_in(true)?;
        assert!(parse_callback(&attempt, "/auth/callback?state=wrong&code=secret").is_err());
        let path = format!(
            "/auth/callback?state={}&code=code&client_id=oaiapp_test",
            attempt.state
        );
        assert_eq!(
            callback_client(&attempt, &parse_callback(&attempt, &path)?)?,
            "oaiapp_test"
        );
        let missing = format!("/auth/callback?state={}&code=code", attempt.state);
        assert!(callback_client(&attempt, &parse_callback(&attempt, &missing)?).is_err());
        let params: BTreeMap<_, _> = attempt
            .url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        assert_eq!(
            params["code_challenge"],
            B64.encode(digest::digest(&digest::SHA256, attempt.verifier.as_bytes()))
        );
        assert_eq!(params["agent_name_hint"], "Eldiron");
        assert_eq!(params["client_id"], "dynamic_agent_client");
        assert_eq!(params["redirect_uri"], attempt.redirect);
        Ok(())
    }
}
