//! Project execution policy, independent of transports, renderers and hosting.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectMode {
    #[default]
    SinglePlayer,
    Multiplayer,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiplayerSettings {
    pub endpoint: String,
    /// Public provider-neutral deployment identity. Resolution belongs to the
    /// publisher/control plane; gameplay never receives cloud credentials.
    pub deployment_id: String,
    pub allow_insecure_loopback: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectPlaySettings {
    pub mode: ProjectMode,
    pub multiplayer: MultiplayerSettings,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchPurpose {
    Player,
    CreatorTest,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LaunchPlan {
    Local,
    Remote(MultiplayerSettings),
}
impl ProjectPlaySettings {
    pub fn parse(source: &str) -> Result<Self, String> {
        let table = source
            .parse::<toml::Table>()
            .map_err(|e| format!("Game configuration: {e}"))?;
        let mode = match table.get("game").and_then(|g| g.get("mode")) {
            None => ProjectMode::SinglePlayer,
            Some(toml::Value::String(mode)) if mode == "single_player" => ProjectMode::SinglePlayer,
            Some(toml::Value::String(mode)) if mode == "multiplayer" => ProjectMode::Multiplayer,
            _ => return Err("[game].mode must be 'single_player' or 'multiplayer'".into()),
        };
        let mut multiplayer = MultiplayerSettings::default();
        if let Some(value) = table.get("multiplayer") {
            let settings = value.as_table().ok_or("[multiplayer] must be a table")?;
            for (key, target) in [
                ("endpoint", &mut multiplayer.endpoint),
                ("deployment_id", &mut multiplayer.deployment_id),
            ] {
                if let Some(value) = settings.get(key) {
                    *target = value
                        .as_str()
                        .ok_or_else(|| format!("[multiplayer].{key} must be a string"))?
                        .trim()
                        .into();
                }
            }
            if let Some(value) = settings.get("allow_insecure_loopback") {
                multiplayer.allow_insecure_loopback = value
                    .as_bool()
                    .ok_or("allow_insecure_loopback must be true or false")?;
            }
        }
        if let Ok(endpoint) = url::Url::parse(&multiplayer.endpoint) {
            multiplayer.endpoint = endpoint.to_string();
        }
        Ok(Self { mode, multiplayer })
    }
    /// Creator can author/test an online project before assigning a deployment.
    pub fn launch(&self, purpose: LaunchPurpose) -> Result<LaunchPlan, String> {
        if purpose == LaunchPurpose::CreatorTest || self.mode == ProjectMode::SinglePlayer {
            return Ok(LaunchPlan::Local);
        }
        self.multiplayer.validate()?;
        Ok(LaunchPlan::Remote(self.multiplayer.clone()))
    }
}
impl MultiplayerSettings {
    pub fn validate(&self) -> Result<(), String> {
        let url = url::Url::parse(&self.endpoint)
            .map_err(|_| "Multiplayer projects need a valid [multiplayer].endpoint")?;
        let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.host_str().is_none()
            || url.path() != "/play"
            || !(url.scheme() == "wss"
                || url.scheme() == "ws" && self.allow_insecure_loopback && loopback)
        {
            return Err("Multiplayer endpoint must be wss://host/play without credentials/query/fragment; explicit insecure tests must use loopback".into());
        }
        if self.deployment_id.len() > 128 || self.deployment_id.chars().any(char::is_control) {
            return Err("Invalid multiplayer deployment identity".into());
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_and_creator_override_do_not_require_a_network() {
        assert_eq!(
            ProjectPlaySettings::parse("")
                .unwrap()
                .launch(LaunchPurpose::Player)
                .unwrap(),
            LaunchPlan::Local
        );
        let p = ProjectPlaySettings::parse("[game]\nmode='multiplayer'").unwrap();
        assert!(p.launch(LaunchPurpose::Player).is_err());
        assert_eq!(
            p.launch(LaunchPurpose::CreatorTest).unwrap(),
            LaunchPlan::Local
        );
        assert!(ProjectPlaySettings::parse("[game]\nmode=true").is_err());
        assert!(ProjectPlaySettings::parse("[game]\nmode='multi'").is_err());
    }
    #[test]
    fn remote_policy_is_portable_explicit_and_contains_no_credentials() {
        for endpoint in ["wss://game.example/play", "wss://game.example:8443/play"] {
            let p=ProjectPlaySettings::parse(&format!("[game]\nmode='multiplayer'\n[multiplayer]\nendpoint='{endpoint}'\ndeployment_id='world-a'")).unwrap();
            assert!(matches!(
                p.launch(LaunchPurpose::Player).unwrap(),
                LaunchPlan::Remote(_)
            ));
        }
        for endpoint in [
            "ws://example.com/play",
            "wss://name:password@example.com/play",
            "wss://example.com/play?token=secret",
            "wss://example.com/other",
        ] {
            assert!(
                MultiplayerSettings {
                    endpoint: endpoint.into(),
                    allow_insecure_loopback: true,
                    ..Default::default()
                }
                .validate()
                .is_err()
            );
        }
        assert!(
            MultiplayerSettings {
                endpoint: "ws://127.0.0.1:8080/play".into(),
                allow_insecure_loopback: true,
                ..Default::default()
            }
            .validate()
            .is_ok()
        );
    }
}
