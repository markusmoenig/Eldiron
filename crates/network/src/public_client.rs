//! Release identity belongs to networking; the portable display format stays
//! renderer/transport independent in shared. No server state store is opened.
use crate::*;
pub fn export(project: &eldiron_runtime::Project) -> Result<Vec<u8>, String> {
    shared::public_client::capture(
        project,
        project_content_revision(project)?,
        PROTOCOL_VERSION,
    )
}
pub fn export_source(
    bytes: &[u8],
    endpoint: Option<&str>,
    deployment: Option<&str>,
) -> Result<Vec<u8>, String> {
    let mut project = shared::project_io::decode_project(bytes)?;
    let mut config = project
        .config
        .parse::<toml::Table>()
        .map_err(|e| e.to_string())?;
    if endpoint.is_some() || deployment.is_some() {
        let settings = config
            .entry("multiplayer".to_string())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .ok_or("Invalid multiplayer configuration")?;
        if let Some(endpoint) = endpoint {
            settings.insert("endpoint".into(), endpoint.into());
        }
        if let Some(deployment) = deployment {
            settings.insert("deployment_id".into(), deployment.into());
        }
        project.config = toml::to_string(&config).map_err(|e| e.to_string())?;
    }
    export(&project)
}
pub fn load(bytes: &[u8]) -> Result<eldiron_runtime::Project, String> {
    shared::public_client::load_player(bytes, PROTOCOL_VERSION)
}

pub fn export_streamed(
    project: &eldiron_runtime::Project,
) -> Result<shared::public_client::streaming::Bundle, String> {
    shared::public_client::streaming::partition(
        shared::public_client::capture_payload(project)?,
        project_content_revision(project)?,
        PROTOCOL_VERSION,
    )
}
pub fn export_streamed_source(
    bytes: &[u8],
    endpoint: Option<&str>,
    deployment: Option<&str>,
) -> Result<shared::public_client::streaming::Bundle, String> {
    let mut project = shared::project_io::decode_project(bytes)?;
    let mut config = project
        .config
        .parse::<toml::Table>()
        .map_err(|e| e.to_string())?;
    if endpoint.is_some() || deployment.is_some() {
        let settings = config
            .entry("multiplayer".to_owned())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .ok_or("Invalid multiplayer configuration")?;
        if let Some(endpoint) = endpoint {
            settings.insert("endpoint".into(), endpoint.into());
        }
        if let Some(deployment) = deployment {
            settings.insert("deployment_id".into(), deployment.into());
        }
        project.config = toml::to_string(&config).map_err(|e| e.to_string())?;
    }
    export_streamed(&project)
}
