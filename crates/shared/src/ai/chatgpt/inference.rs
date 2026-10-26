use super::*;
use std::{
    io::{BufRead, BufReader, Read},
    time::Instant,
};
// Catalog entries include substantial service metadata beyond the picker fields.
// This bound is separate from credentials, error responses and turn output.
const MAX_MODEL_CATALOG: usize = 16 * 1024 * 1024;

pub(super) struct ChatGptProvider {
    pub(super) manager: Manager,
    pub(super) account: Uuid,
    pub(super) model: String,
}
impl Provider for ChatGptProvider {
    fn complete(&self, instructions: &str, context: &Json) -> Result<Json, String> {
        let started = Instant::now();
        let token = self
            .manager
            .access_token(self.account, Duration::from_secs(10))?;
        let remaining = Duration::from_secs(28)
            .checked_sub(started.elapsed())
            .filter(|d| *d > Duration::from_secs(1))
            .ok_or("ChatGPT credential refresh timed out.")?;
        request(
            &format!("{RESOURCE}/responses"),
            &token,
            &self.model,
            instructions,
            context,
            remaining,
        )
    }
}
fn request(
    endpoint: &str,
    token: &str,
    model: &str,
    instructions: &str,
    context: &Json,
    timeout: Duration,
) -> Result<Json, String> {
    // JSON mode validates input messages separately from the instructions field.
    // Keep this host-owned instruction explicit even when player/context text
    // contains no mention of JSON.
    let body=json!({"model":model,"instructions":instructions,"input":[
        {"role":"developer","content":"Return only JSON following the response contract in the instructions."},
        {"role":"user","content":context.to_string()}
    ],"store":false,"stream":true,"text":{"format":{"type":"json_object"}}}).to_string();
    if body.len() > 64 * 1024 {
        return Err("ChatGPT context exceeds the request limit.".into());
    }
    let response = agent(timeout)
        .post(endpoint)
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .set("Accept", "text/event-stream")
        .send_string(&body)
        .map_err(|error| inference_http_error(error, token))?;
    parse_stream(response.into_reader())
}
fn inference_http_error(error: ureq::Error, token: &str) -> String {
    match error {
        ureq::Error::Status(status, response) => {
            let request_id = response.header("x-request-id").map(str::to_string);
            let data = read_json(response).ok();
            diagnostic(status, data.as_ref(), request_id.as_deref(), token)
        }
        other => http_error(other),
    }
}
fn diagnostic(status: u16, data: Option<&Json>, request_id: Option<&str>, token: &str) -> String {
    let safe = |text: &str| {
        let text = if token.is_empty() {
            text.to_string()
        } else {
            text.replace(token, "[redacted]")
        };
        text.chars()
            .filter(|c| !c.is_control())
            .take(512)
            .collect::<String>()
    };
    let mut text = format!("ChatGPT returned HTTP {status}");
    if let Some(data) = data {
        if let Some(message) = data["error"]["message"]
            .as_str()
            .or(data["detail"].as_str())
            .or(data["message"].as_str())
        {
            text.push_str(": ");
            text.push_str(&safe(message));
        }
        if let Some(param) = data["error"]["param"].as_str() {
            text.push_str(&format!(" (parameter: {})", safe(param)));
        }
        if let Some(code) = data["error"]["code"].as_str() {
            text.push_str(&format!(" (code: {})", safe(code)));
        }
    }
    if let Some(id) = request_id {
        text.push_str(&format!(" [request: {}]", safe(id)));
    }
    if status == 401 {
        text.push_str(" Sign in again if your session has expired.");
    }
    text
}
impl Manager {
    pub fn models(&self) -> Result<Vec<ModelInfo>, String> {
        let account = self.selected()?.ok_or("Sign in to ChatGPT first.")?;
        let token = self.access_token(account.id, Duration::from_secs(10))?;
        let response = agent(Duration::from_secs(15))
            .get(&format!("{RESOURCE}/models"))
            .set("Authorization", &format!("Bearer {token}"))
            .call()
            .map_err(http_error)?;
        let models = read_catalog(response)?;
        self.locked(|store| {
            let selected = store
                .accounts
                .iter_mut()
                .find(|a| a.id == account.id)
                .ok_or("ChatGPT account was removed.")?;
            if selected.model.is_none() {
                selected.model = Some(models[0].slug.clone());
                self.save(store)?;
            }
            Ok(())
        })?;
        Ok(models)
    }
}
fn read_catalog(response: ureq::Response) -> Result<Vec<ModelInfo>, String> {
    parse_models(&read_json_with_limit(
        response,
        MAX_MODEL_CATALOG,
        "ChatGPT model catalog",
    )?)
}
fn parse_models(data: &Json) -> Result<Vec<ModelInfo>, String> {
    let models = data["models"]
        .as_array()
        .ok_or("ChatGPT returned no model catalog.")?
        .iter()
        .filter(|m| m["visibility"] == "list")
        .filter_map(|m| {
            Some(ModelInfo {
                slug: m["slug"].as_str()?.into(),
                display_name: clean(m["display_name"].as_str()?),
            })
        })
        .collect::<Vec<_>>();
    if models.is_empty() {
        Err("No models are available for this ChatGPT account.".into())
    } else {
        Ok(models)
    }
}
fn parse_stream(reader: impl Read) -> Result<Json, String> {
    const LIMIT: usize = 512 * 1024;
    let mut reader = BufReader::new(reader.take((LIMIT + 1) as u64));
    let mut bytes = 0;
    let mut data = String::new();
    let mut finished_items = std::collections::BTreeMap::<u64, String>::new();
    loop {
        let mut line = String::new();
        let n = reader
            .read_line(&mut line)
            .map_err(|_| "ChatGPT response stream was interrupted.")?;
        bytes += n;
        if bytes > LIMIT {
            return Err("ChatGPT response stream exceeded the size limit.".into());
        }
        if n == 0 {
            return Err("ChatGPT stream ended without completed inference.".into());
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if let Some(value) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(value.trim_start());
        }
        if !line.is_empty() {
            continue;
        }
        if data.is_empty() {
            continue;
        }
        let event: Json =
            serde_json::from_str(&data).map_err(|_| "Invalid ChatGPT stream event.")?;
        data.clear();
        match event["type"].as_str() {
            Some("response.completed") => {
                let response = &event["response"];
                if response["status"] != "completed" {
                    return Err("ChatGPT inference did not complete.".into());
                }
                let mut text = output_text(
                    response["output"]
                        .as_array()
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                )?;
                // store:false can leave the final response output empty. Its
                // completed message items were already delivered in the stream.
                // They stay buffered until this successful terminal event.
                if text.is_empty() {
                    text = finished_items.values().cloned().collect();
                }
                if text.len() > 8000 {
                    return Err("ChatGPT reply exceeded the size limit.".into());
                }
                return serde_json::from_str(&text)
                    .map_err(|_| "ChatGPT did not return the required JSON object.".into());
            }
            Some("response.output_item.done") => {
                let text = output_text(std::slice::from_ref(&event["item"]))?;
                if !text.is_empty() {
                    let index = event["output_index"]
                        .as_u64()
                        .ok_or("ChatGPT output item has no index.")?;
                    finished_items.insert(index, text);
                    if finished_items.values().map(String::len).sum::<usize>() > 8000 {
                        return Err("ChatGPT reply exceeded the size limit.".into());
                    }
                }
            }
            Some("response.failed" | "response.incomplete" | "error") => {
                let code = event["response"]["error"]["code"]
                    .as_str()
                    .or(event["code"].as_str())
                    .unwrap_or("incomplete_response");
                let code = if code.len() < 100
                    && code.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    code
                } else {
                    "inference_failed"
                };
                return Err(format!(
                    "ChatGPT request failed ({code}). Check plan usage or sign in again."
                ));
            }
            _ => {}
        }
    }
}
fn output_text(items: &[Json]) -> Result<String, String> {
    let mut text = String::new();
    for item in items {
        if item["type"] != "message"
            || item["phase"]
                .as_str()
                .is_some_and(|phase| phase != "final_answer")
        {
            continue;
        }
        if let Some(content) = item["content"].as_array() {
            for part in content {
                match part["type"].as_str() {
                    Some("output_text") => text.push_str(
                        part["text"]
                            .as_str()
                            .ok_or("Invalid ChatGPT output text.")?,
                    ),
                    Some("refusal") => return Err("ChatGPT declined this request.".into()),
                    _ => {}
                }
            }
        }
    }
    Ok(text)
}

#[cfg(test)]
pub(super) mod test_support {
    use super::*;
    pub fn http(endpoint: &str) -> Result<Json, String> {
        request(
            endpoint,
            "test-access",
            "test-model",
            "Return JSON.",
            &json!({"player":"hello"}),
            Duration::from_secs(5),
        )
    }
    pub fn stream(data: &[u8]) -> Result<Json, String> {
        parse_stream(data)
    }
    pub fn diagnostic_message(status: u16, data: &Json, token: &str) -> String {
        diagnostic(status, Some(data), Some("req_test"), token)
    }
    pub fn catalog_response(response: ureq::Response) -> Result<Vec<ModelInfo>, String> {
        read_catalog(response)
    }
    pub fn catalog_limit() -> usize {
        MAX_MODEL_CATALOG
    }
    pub fn models(data: &Json) -> Result<Vec<ModelInfo>, String> {
        parse_models(data)
    }
}
