//! Native atomic recovery artifacts. Import always creates a fresh path;
//! restoring never overwrites a running host or changes retry watermarks.
use super::*;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
};
fn io(error: std::io::Error) -> SaveError {
    SaveError::Store(error.to_string())
}
pub fn read_snapshot(path: impl AsRef<Path>) -> Result<WorldSave, SaveError> {
    super::journal_file::read_image(path.as_ref())
}
/// Reads of the store's atomically renamed head see a complete old/new image.
/// A live owner should checkpoint first when its latest live state is required.
pub fn write_snapshot_new(path: impl AsRef<Path>, snapshot: &WorldSave) -> Result<(), SaveError> {
    let path = path.as_ref();
    if path.extension().is_some_and(|v| v == "eldiron") {
        return Err(SaveError::Invalid(
            "Use a separate snapshot path, not authored .eldiron content".into(),
        ));
    }
    let bytes = snapshot.encode()?;
    WorldSave::decode(&bytes)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(io)?;
    let parent = parent.canonicalize().map_err(io)?;
    let target = parent.join(
        path.file_name()
            .ok_or_else(|| SaveError::Invalid("Snapshot filename required".into()))?,
    );
    let _owner = if target.extension().is_some_and(|v| v == "snapshot") {
        None
    } else {
        Some(FileWorldStore::open(&target)?)
    };
    if target.exists() {
        return Err(SaveError::Conflict);
    }
    let temporary = parent.join(format!(
        ".eldiron-{}.tmp",
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
        let mut file = options.open(&temporary).map_err(io)?;
        file.write_all(&bytes).map_err(io)?;
        file.sync_all().map_err(io)?;
        drop(file);
        // Link is atomic and refuses an existing destination, including a
        // symlink. Unlike rename it cannot replace another save during a race.
        std::fs::hard_link(&temporary, &target).map_err(io)?;
        #[cfg(unix)]
        File::open(&parent).and_then(|f| f.sync_all()).map_err(io)?;
        Ok(())
    })();
    let _ = std::fs::remove_file(&temporary);
    result
}
