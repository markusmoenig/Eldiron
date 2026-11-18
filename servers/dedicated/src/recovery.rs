//! Offline/operator recovery, never a gameplay or public HTTP command.
use eldiron_runtime::{Project, persistence::*};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct SnapshotInfo {
    pub world: theframework_core::Uuid,
    pub revision: u64,
    pub content: String,
    pub purpose: SavePurpose,
    pub characters: usize,
    pub regions: usize,
    pub entities: usize,
    pub action_streams: usize,
    pub creation_receipts: usize,
    pub tick: u64,
    pub movement_tick: u64,
}
pub fn describe(saved: &WorldSave) -> SnapshotInfo {
    SnapshotInfo {
        world: saved.world,
        revision: saved.revision,
        content: saved.content.clone(),
        purpose: saved.purpose.clone(),
        characters: saved.characters.len(),
        regions: saved.world_state.regions.len(),
        entities: saved
            .world_state
            .regions
            .iter()
            .map(|r| r.entities.len() + r.stashed.len())
            .sum(),
        action_streams: saved.action_streams.len(),
        creation_receipts: saved.requests.len(),
        tick: saved.tick,
        movement_tick: saved.movement_tick,
    }
}
pub fn load_project(path: impl AsRef<Path>) -> Result<Project, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut project = shared::project_io::decode_project(&bytes)?;
    project.migrate_default_ruleset();
    Ok(project)
}
pub fn validate_project(saved: &WorldSave, project: &Project) -> Result<(), SaveError> {
    saved.validate()?;
    if saved.content != content_revision(project)? {
        return Err(SaveError::Incompatible(
            "Project/engine content differs; use the matching release or an explicit migration"
                .into(),
        ));
    }
    Ok(())
}
pub fn copy_snapshot(
    source: impl AsRef<Path>,
    output: impl AsRef<Path>,
) -> Result<SnapshotInfo, SaveError> {
    let saved = read_snapshot(source)?;
    write_snapshot_new(output, &saved)?;
    Ok(describe(&saved))
}
/// Conservative migration: deployment/mode/TOML formatting only. Gameplay,
/// schema, templates and authored geometry changes require domain-specific work.
pub fn migrate_deployment(
    saved: &WorldSave,
    from: &Project,
    to: &Project,
) -> Result<WorldSave, SaveError> {
    validate_project(saved, from)?;
    if eldiron_network::project_content_revision(from).map_err(SaveError::Invalid)?
        != eldiron_network::project_content_revision(to).map_err(SaveError::Invalid)?
    {
        return Err(SaveError::Incompatible(
            "Gameplay content changed; a domain-specific migration is required".into(),
        ));
    }
    let mut next = saved.clone();
    next.content = content_revision(to)?;
    next.revision = next.revision.checked_add(1).ok_or(SaveError::Conflict)?;
    next.validate()?;
    Ok(next)
}

/// Operator-assisted recovery publishes a fresh private snapshot. The original
/// world is never edited; stop its owner before deploying the recovered slot.
pub fn reset_password(
    saved: &WorldSave,
    project: &Project,
    username: &str,
    password: &str,
) -> Result<WorldSave, SaveError> {
    validate_project(saved, project)?;
    if saved.purpose != SavePurpose::Dedicated {
        return Err(SaveError::Incompatible(
            "Account recovery requires a dedicated save".into(),
        ));
    }
    let username = crate::auth::username(username)
        .ok_or_else(|| SaveError::Invalid("Invalid account name".into()))?;
    let mut directory: crate::auth::Directory = serde_json::from_value(
        saved
            .host_data
            .get("accounts.v1")
            .cloned()
            .ok_or_else(|| SaveError::Invalid("No account directory".into()))?,
    )
    .map_err(|_| SaveError::Invalid("Invalid account directory".into()))?;
    directory.validate().map_err(SaveError::Invalid)?;
    let id = directory
        .accounts
        .get(&username)
        .ok_or_else(|| SaveError::Invalid("Account not found".into()))?
        .id;
    let hash = crate::auth::hash(password)
        .map_err(|_| SaveError::Invalid("New password must contain 12–256 UTF-8 bytes".into()))?;
    directory
        .revise(id, Some(hash))
        .map_err(SaveError::Invalid)?;
    let mut next = saved.clone();
    next.revision = next.revision.checked_add(1).ok_or(SaveError::Conflict)?;
    next.host_data.insert(
        "accounts.v1".into(),
        serde_json::to_value(directory)
            .map_err(|_| SaveError::Invalid("Cannot encode account directory".into()))?,
    );
    next.validate()?;
    Ok(next)
}
