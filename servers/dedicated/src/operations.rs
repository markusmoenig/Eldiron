use eldiron_runtime::persistence::{SaveError, WorldSave, read_snapshot};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
pub struct Operations {
    pub metrics_file: Option<PathBuf>,
    pub backup_dir: Option<PathBuf>,
    pub backup_interval: Duration,
    pub backup_retain: usize,
    pub(crate) last_metrics: Instant,
    pub(crate) last_backup: Instant,
}
impl Default for Operations {
    fn default() -> Self {
        Self {
            metrics_file: None,
            backup_dir: None,
            backup_interval: Duration::from_secs(300),
            backup_retain: 8,
            last_metrics: Instant::now(),
            last_backup: Instant::now(),
        }
    }
}
impl Operations {
    pub fn validate(&self) -> Result<(), String> {
        if self
            .metrics_file
            .as_ref()
            .is_some_and(|p| p.extension().is_none_or(|v| v != "json"))
        {
            return Err(
                "Metrics output must use a .json file separate from game/save content".into(),
            );
        }
        if self.backup_interval < Duration::from_secs(30) || !(1..=64).contains(&self.backup_retain)
        {
            return Err("Backup interval must be at least 30 seconds; retention 1..64".into());
        }
        Ok(())
    }
    pub(crate) fn backup_path(&self, saved: &WorldSave) -> Result<Option<PathBuf>, SaveError> {
        let Some(directory) = &self.backup_dir else {
            return Ok(None);
        };
        // Refuse before creating another artifact if retention cannot safely run.
        if directory.exists() && backup_entries(directory)?.len() >= 4096 {
            return Err(SaveError::Store(
                "Backup directory entry quota exceeded".into(),
            ));
        }
        Ok(Some(directory.join(format!(
            "eldiron-{}-r{:020}-{}.snapshot",
            saved.world,
            saved.revision.saturating_add(1),
            theframework_core::Uuid::new_v4()
        ))))
    }
    pub(crate) fn prune(&self, saved: &WorldSave) -> Result<(), SaveError> {
        let Some(directory) = &self.backup_dir else {
            return Ok(());
        };
        let prefix = format!("eldiron-{}-r", saved.world);
        let mut files = vec![];
        for entry in backup_entries(directory)? {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(&prefix)
                && name.ends_with(".snapshot")
                && entry.file_type().map_err(store)?.is_file()
                && let Ok(backup) = read_snapshot(entry.path())
                && backup.world == saved.world
            {
                files.push((backup.revision, name.to_string(), entry.path()));
            }
        }
        files.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
        for (_, _, path) in files.into_iter().skip(self.backup_retain) {
            std::fs::remove_file(path).map_err(store)?;
        }
        Ok(())
    }
}
fn backup_entries(directory: &Path) -> Result<Vec<std::fs::DirEntry>, SaveError> {
    let mut entries = vec![];
    for entry in std::fs::read_dir(directory).map_err(store)? {
        if entries.len() >= 4096 {
            return Err(SaveError::Store(
                "Backup directory entry quota exceeded".into(),
            ));
        }
        entries.push(entry.map_err(store)?);
    }
    Ok(entries)
}
fn store(error: std::io::Error) -> SaveError {
    SaveError::Store(error.to_string())
}
/// Resolve aliases without creating paths, including missing output parents.
pub fn path_identity(path: &Path) -> Result<PathBuf, String> {
    let mut base = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let mut suffix = vec![];
    while !base.exists() {
        suffix.push(base.file_name().ok_or("Invalid output path")?.to_owned());
        if !base.pop() {
            return Err("Invalid output path".into());
        }
    }
    let mut resolved = base.canonicalize().map_err(|e| e.to_string())?;
    for part in suffix.into_iter().rev() {
        resolved.push(part);
    }
    Ok(resolved)
}
pub fn write_metrics(path: &Path, snapshot: &crate::metrics::HostSnapshot) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    if path.is_symlink() {
        return Err("Metrics path cannot be a symlink".into());
    }
    let temporary = parent.join(format!(
        ".eldiron-metrics-{}.tmp",
        theframework_core::Uuid::new_v4()
    ));
    let result = (|| {
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary).map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec_pretty(snapshot).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(&temporary, path).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(())
    })();
    let _ = std::fs::remove_file(temporary);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use eldiron_runtime::persistence::*;
    use theframework_core::Uuid;
    fn saved() -> WorldSave {
        WorldSave {
            version: SAVE_VERSION,
            world: Uuid::new_v4(),
            purpose: SavePurpose::Dedicated,
            content: "a".repeat(64),
            revision: 1,
            characters: vec![],
            local_character: None,
            requests: Default::default(),
            host_data: Default::default(),
            action_streams: Default::default(),
            world_state: WorldRecord {
                next_id: 1,
                values: Default::default(),
                regions: vec![],
                render: Default::default(),
                region_render: vec![],
            },
            clock: rusterix::simulation::SimulationClock::new(Duration::from_millis(250)).save(),
            tick: 0,
            movement_tick: 0,
        }
    }
    #[test]
    fn retention_keeps_newest_valid_world_images_and_refuses_full_directory_before_writing() {
        let dir = std::env::temp_dir().join(format!("eldiron-retention-{}", Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let mut ops = Operations::default();
        ops.backup_dir = Some(dir.clone());
        ops.backup_retain = 2;
        let mut image = saved();
        let mut paths = vec![];
        for revision in 1..=4 {
            image.revision = revision;
            let path = ops.backup_path(&image).unwrap().unwrap();
            write_snapshot_new(&path, &image).unwrap();
            paths.push(path);
        }
        let corrupt = dir.join(format!(
            "eldiron-{}-r99999999999999999999-corrupt.snapshot",
            image.world
        ));
        std::fs::write(&corrupt, b"corrupt").unwrap();
        let unrelated = dir.join("leave-me-alone.txt");
        std::fs::write(&unrelated, b"operator file").unwrap();
        ops.prune(&image).unwrap();
        assert!(!paths[0].exists());
        assert!(!paths[1].exists());
        assert!(paths[2].exists());
        assert!(paths[3].exists());
        assert!(corrupt.exists());
        assert!(unrelated.exists());
        for n in 0..4092 {
            std::fs::write(dir.join(format!("unrelated-{n}")), []).unwrap();
        }
        assert!(ops.backup_path(&image).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn deployment_migration_preserves_private_records_but_refuses_gameplay_changes() {
        let mut from = eldiron_runtime::Project::default();
        from.config = "[game]\nmode='single_player'\ngame_tick_ms=250".into();
        let mut to = from.clone();
        to.config="[game]\nmode='multiplayer'\ngame_tick_ms=250\n[multiplayer]\nendpoint='wss://new.example/play'".into();
        let mut image = saved();
        image.content = content_revision(&from).unwrap();
        image.tick = 234;
        image
            .host_data
            .insert("private".into(), serde_json::json!({"account":"retained"}));
        let migrated = crate::recovery::migrate_deployment(&image, &from, &to).unwrap();
        assert_eq!(migrated.world, image.world);
        assert_eq!(migrated.revision, image.revision + 1);
        assert_eq!(migrated.tick, image.tick);
        assert_eq!(migrated.host_data, image.host_data);
        crate::recovery::validate_project(&migrated, &to).unwrap();
        assert!(crate::recovery::validate_project(&image, &to).is_err());
        to.config = to.config.replace("250", "100");
        assert!(crate::recovery::migrate_deployment(&image, &from, &to).is_err());
        assert_eq!(image.content, content_revision(&from).unwrap());
    }
}
