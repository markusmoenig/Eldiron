use serde_json::{Value as Json, json};
use std::sync::Arc;
use std::time::Duration;
const LIMIT: usize = 64 * 1024;
/// Implement this for other transports/services. It runs on a worker thread.
/// Return JSON matching the stage's contract. Never log credentials or prompts.
pub trait Provider: Send + Sync {
    fn complete(&self, instructions: &str, context: &Json) -> Result<Json, String>;
}

/// Host-only settings, deliberately neither serializable nor Debug.
#[cfg(not(target_arch = "wasm32"))]
pub struct CompatibleProvider {
    endpoint: String,
    model: String,
    api_key: Option<String>,
}
#[cfg(not(target_arch = "wasm32"))]
impl CompatibleProvider {
    /// Base URL includes /v1 (or the service's equivalent). Plain HTTP is only
    /// allowed for loopback local servers. Redirects are disabled.
    pub fn new(base_url: &str, model: &str, api_key: Option<String>) -> Result<Self, String> {
        let url = url::Url::parse(base_url).map_err(|_| "Invalid AI base URL.")?;
        let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if !(url.scheme() == "https" || (url.scheme() == "http" && local))
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || model.trim().is_empty()
        {
            return Err("AI needs an HTTPS endpoint (or HTTP loopback), a model, and no URL credentials/query.".into());
        }
        Ok(Self {
            endpoint: format!("{}/chat/completions", base_url.trim_end_matches('/')),
            model: model.into(),
            api_key,
        })
    }
    pub fn from_env() -> Result<Option<Arc<dyn Provider>>, String> {
        let Ok(model) = std::env::var("ELDIRON_AI_MODEL") else {
            return Ok(None);
        };
        let base = std::env::var("ELDIRON_AI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".into());
        let key = std::env::var("ELDIRON_AI_API_KEY")
            .ok()
            .filter(|s| !s.is_empty());
        Ok(Some(Arc::new(Self::new(&base, &model, key)?)))
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl Provider for CompatibleProvider {
    fn complete(&self, instructions: &str, context: &Json) -> Result<Json, String> {
        use std::io::Read;
        let body = json!({"model":self.model,"stream":false,"store":false,
            "max_completion_tokens":2048,"response_format":{"type":"json_object"},
            "messages":[{"role":"system","content":instructions},
                        {"role":"user","content":context.to_string()}]})
        .to_string();
        if body.len() > LIMIT {
            return Err("AI context exceeds the request limit.".into());
        }
        let agent = ureq::AgentBuilder::new()
            .redirects(0)
            .timeout(Duration::from_secs(30))
            .build();
        let mut request = agent
            .post(&self.endpoint)
            .set("Content-Type", "application/json");
        if let Some(key) = &self.api_key {
            request = request.set("Authorization", &format!("Bearer {key}"));
        }
        let response = request.send_string(&body).map_err(|error| match error {
            ureq::Error::Status(code, _) => {
                format!("AI service returned HTTP {code}. Check provider settings.")
            }
            _ => "AI service could not be reached or timed out.".into(),
        })?;
        let mut bytes = Vec::new();
        response
            .into_reader()
            .take((LIMIT + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| "Could not read AI response.")?;
        if bytes.len() > LIMIT {
            return Err("AI response exceeds the size limit.".into());
        }
        let value: Json =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid AI response JSON.")?;
        let choice = &value["choices"][0];
        if choice["finish_reason"] != "stop" {
            return Err("AI response was incomplete or refused.".into());
        }
        let content = choice["message"]["content"]
            .as_str()
            .ok_or("AI response has no text.")?;
        serde_json::from_str(content)
            .map_err(|_| "AI did not return the required JSON object.".into())
    }
}
