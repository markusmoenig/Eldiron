//! Native Sign in with ChatGPT. Shared by Creator and the terminal; no project
//! owns credentials. Protocol: developers.openai.com/siwc/token-sharing-open-source/.
mod auth;
mod inference;
mod storage;
#[cfg(test)]
mod tests;

use super::provider::Provider;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use theframework::prelude::Uuid;

const ISSUER: &str = "https://auth.openai.com";
const RESOURCE: &str = "https://api.openai.com/v1";
const DIRECT_SCOPE: &str = "chatgpt.tokens.use.direct";
const MAX_JSON: usize = 256 * 1024;
pub const USAGE_URL: &str = "https://chatgpt.com/settings/usage";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountInfo {
    pub id: Uuid,
    pub label: String,
    pub signed_in: bool,
    pub plan_enabled: bool,
    pub model: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub slug: String,
    pub display_name: String,
}

// Tokens and authorization attempts intentionally do not implement Debug.
#[derive(Clone, Serialize, Deserialize)]
struct Tokens {
    access_token: String,
    refresh_token: Option<String>,
    id_token: String,
    expires_at: u64,
    scopes: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Account {
    id: Uuid,
    subject: String,
    email: Option<String>,
    client_id: String,
    model: Option<String>,
    tokens: Option<Tokens>,
}
impl Account {
    fn info(&self) -> AccountInfo {
        let name = self.email.as_deref().unwrap_or("ChatGPT account");
        AccountInfo {
            id: self.id,
            label: format!("{} · {}", clean(name), &self.id.to_string()[..8]),
            signed_in: self.tokens.is_some(),
            plan_enabled: self
                .tokens
                .as_ref()
                .is_some_and(|t| t.scopes.iter().any(|s| s == DIRECT_SCOPE)),
            model: self.model.clone(),
        }
    }
}
#[derive(Serialize, Deserialize)]
struct Store {
    version: u32,
    host_id: String,
    selected: Option<Uuid>,
    accounts: Vec<Account>,
}
impl Default for Store {
    fn default() -> Self {
        Self {
            version: 1,
            host_id: format!("urn:uuid:{}", Uuid::new_v4()),
            selected: None,
            accounts: vec![],
        }
    }
}

#[derive(Clone)]
pub struct Manager {
    directory: PathBuf,
    #[cfg(test)]
    refresh_endpoint: Option<String>,
}
impl Manager {
    pub fn local() -> Result<Self, String> {
        Ok(Self {
            #[cfg(test)]
            refresh_endpoint: None,
            directory: dirs::config_dir()
                .ok_or("Cannot locate local ChatGPT settings.")?
                .join("Eldiron")
                .join("ChatGPT"),
        })
    }
    fn refresh_endpoint(&self) -> String {
        #[cfg(test)]
        if let Some(endpoint) = &self.refresh_endpoint {
            return endpoint.clone();
        }
        format!("{ISSUER}/api/accounts/oauth/token")
    }
    pub fn accounts(&self) -> Result<Vec<AccountInfo>, String> {
        self.locked(|s| Ok(s.accounts.iter().map(Account::info).collect()))
    }
    pub fn selected(&self) -> Result<Option<AccountInfo>, String> {
        self.locked(|s| {
            Ok(s.accounts
                .iter()
                .find(|a| Some(a.id) == s.selected)
                .map(Account::info))
        })
    }
    pub fn select(&self, id: Uuid) -> Result<(), String> {
        self.locked(|s| {
            if !s.accounts.iter().any(|a| a.id == id) {
                return Err("Unknown ChatGPT account.".into());
            }
            s.selected = Some(id);
            self.save(s)
        })
    }
    pub fn set_model(&self, slug: &str) -> Result<(), String> {
        if slug.is_empty() || slug.len() > 200 || slug.chars().any(char::is_control) {
            return Err("Choose a valid ChatGPT model slug.".into());
        }
        self.locked(|s| {
            let a = s
                .accounts
                .iter_mut()
                .find(|a| Some(a.id) == s.selected)
                .ok_or("Sign in to ChatGPT first.")?;
            a.model = Some(slug.into());
            self.save(s)
        })
    }
    pub fn provider(&self) -> Result<Arc<dyn Provider>, String> {
        let account = self
            .selected()?
            .ok_or("Continue with ChatGPT to sign in first.")?;
        if !account.signed_in {
            return Err(
                "This ChatGPT account is signed out. Continue with ChatGPT to reconnect.".into(),
            );
        }
        if !account.plan_enabled {
            return Err(
                "ChatGPT plan usage was not authorized. Sign in again and enable plan usage."
                    .into(),
            );
        }
        let model = account
            .model
            .ok_or("Fetch ChatGPT models and select one first.")?;
        Ok(Arc::new(inference::ChatGptProvider {
            manager: self.clone(),
            account: account.id,
            model,
        }))
    }
    /// Opens the system browser. A new registration is made only on explicit Add account.
    pub fn sign_in(&self, new_account: bool, cancel: &AtomicBool) -> Result<AccountInfo, String> {
        let attempt = self.begin_sign_in(new_account)?;
        open::that(attempt.url.as_str())
            .map_err(|_| "Could not open the browser for ChatGPT sign-in.")?;
        self.finish_sign_in(attempt, cancel)
    }
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn clean(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).take(160).collect()
}
fn agent(timeout: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .redirects(0)
        .timeout(timeout)
        .build()
}
fn read_json(response: ureq::Response) -> Result<Json, String> {
    read_json_with_limit(response, MAX_JSON, "ChatGPT response")
}
fn read_json_with_limit(
    response: ureq::Response,
    limit: usize,
    label: &str,
) -> Result<Json, String> {
    use std::io::Read;
    let mut bytes = vec![];
    response
        .into_reader()
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read ChatGPT response.")?;
    if bytes.len() > limit {
        return Err(format!("{label} exceeded the size limit."));
    }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid ChatGPT response.".into())
}
fn http_error(error: ureq::Error) -> String {
    match error {
        ureq::Error::Status(status, response) => {
            let code = read_json(response).ok().and_then(|v| {
                v["error"]["code"]
                    .as_str()
                    .or(v["error"].as_str())
                    .map(str::to_owned)
            });
            let detail = code
                .filter(|s| {
                    s.len() < 100 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                })
                .map(|s| format!(" ({s})"))
                .unwrap_or_default();
            format!(
                "ChatGPT returned HTTP {status}{detail}. Check account access, plan limits, or sign in again."
            )
        }
        _ => "ChatGPT could not be reached or timed out.".into(),
    }
}
fn approved_auth_url(value: &str) -> Result<String, String> {
    let url = url::Url::parse(value).map_err(|_| "Invalid ChatGPT discovery URL.")?;
    if url.scheme() != "https"
        || url.host_str() != Some("auth.openai.com")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("ChatGPT discovery returned an unexpected endpoint.".into());
    }
    Ok(url.to_string())
}
fn discovery() -> Result<Json, String> {
    let data = read_json(
        agent(Duration::from_secs(10))
            .get(&format!("{ISSUER}/.well-known/openid-configuration"))
            .call()
            .map_err(http_error)?,
    )?;
    if data["issuer"] != ISSUER {
        return Err("Unexpected ChatGPT identity issuer.".into());
    }
    Ok(data)
}
