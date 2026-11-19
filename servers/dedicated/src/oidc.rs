//! Private OpenID Connect device broker. No provider token/device secret enters
//! gameplay packets. The owner, not this worker, publishes account mutations.
use eldiron_network::{IdentityProvider, Secret};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use serde::{Deserialize, de::DeserializeOwned};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use theframework_core::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub providers: Vec<ProviderConfig>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderConfig {
    pub id: String,
    pub label: String,
    pub issuer: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    #[serde(default)]
    pub allowed_verification_hosts: Vec<String>,
    #[serde(default)]
    pub allow_insecure_loopback: bool,
}
#[derive(Deserialize)]
struct Discovery {
    issuer: String,
    device_authorization_endpoint: String,
    token_endpoint: String,
    jwks_uri: String,
}
pub struct Broker {
    http: reqwest::Client,
    providers: BTreeMap<String, Provider>,
    pub slots: Arc<tokio::sync::Semaphore>,
}
struct Provider {
    config: ProviderConfig,
    discovery: Discovery,
}
pub struct Pending {
    provider: String,
    device_code: String,
    nonce: String,
    deadline: Instant,
    interval: Duration,
}
pub struct Challenge {
    pub provider: String,
    pub verification_uri: String,
    pub user_code: Secret,
    pub expires_in: u32,
}
pub struct VerifiedIdentity {
    pub issuer: String,
    pub subject: String,
}
#[derive(Deserialize)]
struct DeviceReply {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u32,
    interval: Option<u64>,
}
#[derive(Deserialize)]
struct TokenReply {
    id_token: Option<String>,
    error: Option<String>,
}
#[derive(Deserialize)]
struct Claims {
    sub: String,
    iat: u64,
    exp: u64,
    aud: serde_json::Value,
    azp: Option<String>,
    nonce: Option<String>,
}
fn valid_url(value: &str, loopback: bool) -> Result<reqwest::Url, ()> {
    let url = reqwest::Url::parse(value).map_err(|_| ())?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || value.len() > 2048
        || !(url.scheme() == "https"
            || loopback
                && url.scheme() == "http"
                && matches!(
                    url.host_str(),
                    Some("127.0.0.1" | "localhost" | "[::1]" | "::1")
                ))
    {
        return Err(());
    }
    Ok(url)
}
async fn body<T: DeserializeOwned>(mut response: reqwest::Response) -> Result<T, ()> {
    if response.content_length().is_some_and(|n| n > 256 * 1024) {
        return Err(());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| ())? {
        if bytes.len() + chunk.len() > 256 * 1024 {
            return Err(());
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| ())
}
impl Broker {
    #[cfg(test)]
    pub(crate) fn empty_for_owner_tests() -> Arc<Self> {
        Arc::new(Self {
            http: reqwest::Client::new(),
            providers: BTreeMap::new(),
            slots: Arc::new(tokio::sync::Semaphore::new(4)),
        })
    }
    pub async fn load(path: impl AsRef<std::path::Path>) -> Result<Arc<Self>, String> {
        let bytes =
            std::fs::read(path).map_err(|_| "Cannot read private identity configuration")?;
        if bytes.len() > 64 * 1024 {
            return Err("Identity configuration exceeds bounds".into());
        }
        let config: Config = toml::from_str(
            std::str::from_utf8(&bytes).map_err(|_| "Invalid identity configuration")?,
        )
        .map_err(|_| "Invalid identity configuration")?;
        Self::new(config)
            .await
            .map(Arc::new)
            .map_err(|_| "Invalid or unavailable OpenID Connect provider configuration".into())
    }
    pub async fn new(config: Config) -> Result<Self, ()> {
        if config.providers.is_empty() || config.providers.len() > 8 {
            return Err(());
        }
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|_| ())?;
        let mut providers = BTreeMap::new();
        for config in config.providers {
            if !(1..=32).contains(&config.id.len())
                || !config
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                || config.label.is_empty()
                || config.label.len() > 64
                || config.label.chars().any(char::is_control)
                || config.client_id.is_empty()
                || config.client_id.len() > 512
                || config
                    .client_secret
                    .as_ref()
                    .is_some_and(|s| s.len() > 4096)
                || config.allowed_verification_hosts.len() > 8
            {
                return Err(());
            }
            valid_url(&config.issuer, config.allow_insecure_loopback)?;
            let discovery = format!(
                "{}/.well-known/openid-configuration",
                config.issuer.trim_end_matches('/')
            );
            let response = http.get(&discovery).send().await.map_err(|_| ())?;
            if !response.status().is_success() {
                return Err(());
            }
            let discovery: Discovery = body(response).await?;
            if discovery.issuer != config.issuer {
                return Err(());
            }
            for url in [
                &discovery.device_authorization_endpoint,
                &discovery.token_endpoint,
                &discovery.jwks_uri,
            ] {
                valid_url(url, config.allow_insecure_loopback)?;
            }
            if providers
                .insert(config.id.clone(), Provider { config, discovery })
                .is_some()
            {
                return Err(());
            }
        }
        Ok(Self {
            http,
            providers,
            slots: Arc::new(tokio::sync::Semaphore::new(4)),
        })
    }
    pub fn descriptions(&self) -> Vec<IdentityProvider> {
        self.providers
            .values()
            .map(|p| IdentityProvider {
                id: p.config.id.clone(),
                label: p.config.label.clone(),
            })
            .collect()
    }
    pub fn issuer(&self, provider: &str) -> Option<String> {
        self.providers
            .get(provider)
            .map(|p| p.config.issuer.clone())
    }
    pub async fn begin(&self, id: &str) -> Result<(Pending, Challenge), ()> {
        let provider = self.providers.get(id).ok_or(())?;
        let nonce = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let mut form = vec![
            ("client_id", provider.config.client_id.as_str()),
            ("scope", "openid"),
            ("nonce", nonce.as_str()),
        ];
        if let Some(secret) = &provider.config.client_secret {
            form.push(("client_secret", secret));
        }
        let response = self
            .http
            .post(&provider.discovery.device_authorization_endpoint)
            .form(&form)
            .send()
            .await
            .map_err(|_| ())?;
        if !response.status().is_success() {
            return Err(());
        }
        let reply: DeviceReply = body(response).await?;
        let url = valid_url(
            &reply.verification_uri,
            provider.config.allow_insecure_loopback,
        )?;
        let issuer = valid_url(
            &provider.config.issuer,
            provider.config.allow_insecure_loopback,
        )?;
        if url.host_str() != issuer.host_str()
            && !provider
                .config
                .allowed_verification_hosts
                .iter()
                .any(|host| Some(host.as_str()) == url.host_str())
        {
            return Err(());
        }
        if !(1..=600).contains(&reply.expires_in)
            || reply.device_code.is_empty()
            || reply.device_code.len() > 4096
            || reply.user_code.is_empty()
            || reply.user_code.len() > 128
            || reply.user_code.chars().any(char::is_control)
            || reply.interval.is_some_and(|v| !(1..=30).contains(&v))
        {
            return Err(());
        }
        let pending = Pending {
            provider: id.into(),
            device_code: reply.device_code,
            nonce,
            deadline: Instant::now() + Duration::from_secs(reply.expires_in.into()),
            interval: Duration::from_secs(reply.interval.unwrap_or(5)),
        };
        Ok((
            pending,
            Challenge {
                provider: id.into(),
                verification_uri: reply.verification_uri,
                user_code: Secret(reply.user_code),
                expires_in: reply.expires_in,
            },
        ))
    }
    pub async fn finish(&self, mut pending: Pending) -> Result<VerifiedIdentity, ()> {
        let provider = self.providers.get(&pending.provider).ok_or(())?;
        loop {
            tokio::time::sleep(pending.interval).await;
            if Instant::now() >= pending.deadline {
                return Err(());
            }
            let mut form = vec![
                ("client_id", provider.config.client_id.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", pending.device_code.as_str()),
            ];
            if let Some(secret) = &provider.config.client_secret {
                form.push(("client_secret", secret));
            }
            let response = tokio::time::timeout_at(
                tokio::time::Instant::from_std(pending.deadline),
                self.http
                    .post(&provider.discovery.token_endpoint)
                    .form(&form)
                    .send(),
            )
            .await
            .map_err(|_| ())?;
            let response = match response {
                Ok(response) => response,
                Err(error) if error.is_timeout() => {
                    pending.interval = (pending.interval * 2).min(Duration::from_secs(60));
                    continue;
                }
                Err(_) => return Err(()),
            };
            let success = response.status().is_success();
            let reply: TokenReply = body(response).await?;
            match reply.error.as_deref() {
                Some("authorization_pending") => continue,
                Some("slow_down") => {
                    pending.interval += Duration::from_secs(5);
                    continue;
                }
                Some(_) => return Err(()),
                None if success => {}
                None => return Err(()),
            }
            let token = reply.id_token.ok_or(())?;
            let response = self
                .http
                .get(&provider.discovery.jwks_uri)
                .send()
                .await
                .map_err(|_| ())?;
            if !response.status().is_success() {
                return Err(());
            }
            let keys: JwkSet = body(response).await?;
            let subject = verify(
                &token,
                &keys,
                &provider.config.issuer,
                &provider.config.client_id,
                &pending.nonce,
            )?;
            return Ok(VerifiedIdentity {
                issuer: provider.config.issuer.clone(),
                subject,
            });
        }
    }
}
fn verify(
    token: &str,
    keys: &JwkSet,
    issuer: &str,
    client: &str,
    nonce: &str,
) -> Result<String, ()> {
    if token.len() > 32 * 1024 || keys.keys.is_empty() || keys.keys.len() > 64 {
        return Err(());
    }
    let header = decode_header(token).map_err(|_| ())?;
    if header.alg != Algorithm::RS256 {
        return Err(());
    }
    let kid = header.kid.ok_or(())?;
    if kid.len() > 256 {
        return Err(());
    }
    let matching: Vec<_> = keys
        .keys
        .iter()
        .filter(|k| k.common.key_id.as_deref() == Some(kid.as_str()))
        .collect();
    if matching.len() != 1 {
        return Err(());
    }
    let key = matching[0];
    if key
        .common
        .public_key_use
        .as_ref()
        .is_some_and(|u| *u != jsonwebtoken::jwk::PublicKeyUse::Signature)
        || key
            .common
            .key_algorithm
            .is_some_and(|alg| alg != jsonwebtoken::jwk::KeyAlgorithm::RS256)
        || key
            .common
            .key_operations
            .as_ref()
            .is_some_and(|ops| !ops.contains(&jsonwebtoken::jwk::KeyOperations::Verify))
    {
        return Err(());
    }
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[issuer]);
    validation.set_audience(&[client]);
    validation.set_required_spec_claims(&["iss", "sub", "aud", "exp", "iat"]);
    validation.validate_nbf = true;
    validation.leeway = 30;
    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_jwk(key).map_err(|_| ())?,
        &validation,
    )
    .map_err(|_| ())?
    .claims;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ())?
        .as_secs();
    if claims.sub.is_empty()
        || claims.sub.len() > 256
        || claims.sub.chars().any(char::is_control)
        || claims.nonce.as_deref() != Some(nonce)
        || claims.iat > now + 30
        || claims.iat + 630 < now
        || claims.exp <= claims.iat
        || claims.azp.as_ref().is_some_and(|azp| azp != client)
        || claims.aud.as_array().is_some_and(|aud| aud.len() > 1)
            && claims.azp.as_deref() != Some(client)
    {
        return Err(());
    }
    Ok(claims.sub)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use rsa::{pkcs1::EncodeRsaPrivateKey, traits::PublicKeyParts};
    #[test]
    fn signed_identity_requires_trusted_key_issuer_audience_nonce_and_freshness() {
        let private = rsa::RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).unwrap();
        let encode = |bytes: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
        let keys:JwkSet = serde_json::from_value(serde_json::json!({"keys":[{"kty":"RSA","kid":"test","alg":"RS256","use":"sig","n":encode(&private.n().to_bytes_be()),"e":encode(&private.e().to_bytes_be())}]})).unwrap();
        let key =
            jsonwebtoken::EncodingKey::from_rsa_der(private.to_pkcs1_der().unwrap().as_bytes());
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let claims = serde_json::json!({"iss":"https://issuer.example","sub":"stable-subject","aud":"game-client","iat":now,"exp":now+300,"nonce":"request-nonce"});
        let token = |claims: &serde_json::Value, kid: &str| {
            let mut header = jsonwebtoken::Header::new(Algorithm::RS256);
            header.kid = Some(kid.into());
            jsonwebtoken::encode(&header, claims, &key).unwrap()
        };
        let signed = token(&claims, "test");
        assert_eq!(
            verify(
                &signed,
                &keys,
                "https://issuer.example",
                "game-client",
                "request-nonce"
            )
            .unwrap(),
            "stable-subject"
        );
        for (field, value) in [
            ("iss", serde_json::json!("https://attacker.example")),
            ("aud", serde_json::json!("another-client")),
            ("nonce", serde_json::json!("another-request")),
            ("exp", serde_json::json!(now - 60)),
            ("iat", serde_json::json!(now + 60)),
            ("iat", serde_json::json!(now - 700)),
            ("sub", serde_json::json!("")),
            ("aud", serde_json::json!(["game-client", "another-client"])),
        ] {
            let mut bad = claims.clone();
            bad[field] = value;
            assert!(
                verify(
                    &token(&bad, "test"),
                    &keys,
                    "https://issuer.example",
                    "game-client",
                    "request-nonce"
                )
                .is_err(),
                "accepted altered {field}"
            );
        }
        assert!(
            verify(
                &token(&claims, "wrong-key"),
                &keys,
                "https://issuer.example",
                "game-client",
                "request-nonce"
            )
            .is_err()
        );
        let mut altered = signed.into_bytes();
        let last = altered.len() - 10;
        altered[last] = if altered[last] == b'A' { b'B' } else { b'A' };
        assert!(
            verify(
                std::str::from_utf8(&altered).unwrap(),
                &keys,
                "https://issuer.example",
                "game-client",
                "request-nonce"
            )
            .is_err()
        );
        for url in [
            "http://issuer.example",
            "https://secret@issuer.example",
            "https://issuer.example/#fragment",
        ] {
            assert!(valid_url(url, false).is_err());
        }
        assert!(valid_url("http://127.0.0.1:1234/verify", true).is_ok());
        assert!(valid_url("http://127.0.0.1:1234/verify", false).is_err());
    }
}
