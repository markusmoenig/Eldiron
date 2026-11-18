use super::*;
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct StoreDiagnostics {
    pub kind: String,
    pub records: u64,
    pub journal_bytes: u64,
    pub transactions: u64,
    pub checkpoints: u64,
    pub bytes_written: u64,
    pub tails_discarded: u64,
}
/// Commit is compare-and-swap on the resulting complete image. Incremental
/// adapters may store checked changes; snapshot adapters keep the default path. Implementations must not
/// report success until state and retry receipts are durably committed together.
#[allow(async_fn_in_trait)]
pub trait WorldStore {
    fn supports_incremental(&self) -> bool {
        false
    }
    fn diagnostics(&self) -> StoreDiagnostics {
        StoreDiagnostics {
            kind: "snapshot".into(),
            ..Default::default()
        }
    }
    async fn commit_update(
        &mut self,
        expected: u64,
        saved: &PreparedUpdate,
    ) -> Result<(), SaveError> {
        self.commit_prepared(Some(expected), &saved.snapshot()?)
            .await
    }
    async fn load(&mut self) -> Result<Option<WorldSave>, SaveError>;
    async fn commit(&mut self, expected: Option<u64>, saved: &WorldSave) -> Result<(), SaveError>;
    /// Compatible default for custom stores; optimized adapters reuse the
    /// already checked encoding. Success still means a durable atomic commit.
    async fn commit_prepared(
        &mut self,
        expected: Option<u64>,
        saved: &PreparedSave,
    ) -> Result<(), SaveError> {
        self.commit(expected, saved.image()).await
    }
}
fn compare(
    current: Option<&WorldSave>,
    expected: Option<u64>,
    saved: &WorldSave,
) -> Result<(), SaveError> {
    saved.validate()?;
    if current.map(|s| s.revision) != expected
        || saved.revision
            != expected
                .unwrap_or(0)
                .checked_add(1)
                .ok_or(SaveError::Conflict)?
        || current.is_some_and(|s| {
            s.world != saved.world || s.content != saved.content || s.purpose != saved.purpose
        })
    {
        return Err(SaveError::Conflict);
    }
    Ok(())
}
#[derive(Clone, Default)]
pub struct MemoryWorldStore {
    bytes: Option<Vec<u8>>,
}
impl MemoryWorldStore {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, SaveError> {
        WorldSave::decode(&bytes)?;
        Ok(Self { bytes: Some(bytes) })
    }
    pub fn bytes(&self) -> Option<&[u8]> {
        self.bytes.as_deref()
    }
}
impl WorldStore for MemoryWorldStore {
    async fn load(&mut self) -> Result<Option<WorldSave>, SaveError> {
        self.bytes.as_deref().map(WorldSave::decode).transpose()
    }
    async fn commit(&mut self, expected: Option<u64>, saved: &WorldSave) -> Result<(), SaveError> {
        let prepared = PreparedSave::new(saved)?;
        self.commit_prepared(expected, &prepared).await
    }
    async fn commit_prepared(
        &mut self,
        expected: Option<u64>,
        saved: &PreparedSave,
    ) -> Result<(), SaveError> {
        let current = self.load().await?;
        compare(current.as_ref(), expected, saved.image())?;
        self.bytes = Some(saved.bytes().to_vec());
        Ok(())
    }
}
#[cfg(not(target_arch = "wasm32"))]
mod file {
    use super::*;
    use std::{
        fs::{File, OpenOptions},
        io::{Read, Write},
        path::{Path, PathBuf},
    };
    fn io(error: std::io::Error) -> SaveError {
        SaveError::Store(error.to_string())
    }
    #[derive(Clone)]
    struct Head {
        world: theframework_core::Uuid,
        revision: u64,
        content: String,
        purpose: SavePurpose,
    }
    impl Head {
        fn of(saved: &WorldSave) -> Self {
            Self {
                world: saved.world,
                revision: saved.revision,
                content: saved.content.clone(),
                purpose: saved.purpose.clone(),
            }
        }
    }
    #[derive(PartialEq, Eq)]
    struct Stamp {
        device: u64,
        inode: u64,
        len: u64,
        modified: i64,
        modified_ns: i64,
        changed: i64,
        changed_ns: i64,
    }
    fn stamp(metadata: &std::fs::Metadata) -> Option<Stamp> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Some(Stamp {
                device: metadata.dev(),
                inode: metadata.ino(),
                len: metadata.len(),
                modified: metadata.mtime(),
                modified_ns: metadata.mtime_nsec(),
                changed: metadata.ctime(),
                changed_ns: metadata.ctime_nsec(),
            })
        }
        #[cfg(not(unix))]
        {
            let _ = metadata;
            None
        }
    }
    /// One authority per save slot. The OS lock is released even on process
    /// death; a leftover lock filename is not a stale logical lease.
    pub struct FileWorldStore {
        path: PathBuf,
        _lock: File,
        checked_head: Option<(Stamp, Head)>,
        journal: Option<super::super::journal_file::JournalFile>,
    }
    impl FileWorldStore {
        pub fn open(path: impl AsRef<Path>) -> Result<Self, SaveError> {
            let path = path.as_ref();
            if path
                .extension()
                .is_some_and(|e| e == "eldiron" || e == "snapshot")
            {
                return Err(SaveError::Store(
                    "Working saves must be separate from authored .eldiron content and immutable .snapshot backups".into(),
                ));
            }
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            std::fs::create_dir_all(parent).map_err(io)?;
            let parent = parent.canonicalize().map_err(io)?;
            let path = parent.join(
                path.file_name()
                    .ok_or_else(|| SaveError::Store("Save path requires a filename".into()))?,
            );
            if path.is_symlink() {
                return Err(SaveError::Store("Save path cannot be a symlink".into()));
            }
            let mut lock_name = path.as_os_str().to_os_string();
            lock_name.push(".lock");
            let lock = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(lock_name)
                .map_err(io)?;
            lock.try_lock().map_err(|_| SaveError::Busy)?;
            Ok(Self {
                path,
                _lock: lock,
                checked_head: None,
                journal: None,
            })
        }
        pub fn open_journaled(path: impl AsRef<Path>) -> Result<Self, SaveError> {
            let mut store = Self::open(path)?;
            store.journal = Some(super::super::journal_file::JournalFile::new(
                store.path.clone(),
            ));
            Ok(store)
        }
        fn read(&mut self) -> Result<Option<WorldSave>, SaveError> {
            if let Some(journal) = &mut self.journal {
                return journal.load();
            }
            let file = match File::open(&self.path) {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    self.checked_head = None;
                    return Ok(None);
                }
                Err(e) => return Err(io(e)),
            };
            let signature = stamp(&file.metadata().map_err(io)?);
            let mut bytes = vec![];
            file.take(MAX_SAVE_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(io)?;
            if super::super::journal_file::is_manifest(&bytes) {
                self.journal = Some(super::super::journal_file::JournalFile::new(
                    self.path.clone(),
                ));
                return self.journal.as_mut().unwrap().load();
            }
            let saved = WorldSave::decode(&bytes)?;
            self.checked_head = signature.map(|signature| (signature, Head::of(&saved)));
            Ok(Some(saved))
        }
        fn decode_file(file: File) -> Result<WorldSave, SaveError> {
            let mut bytes = vec![];
            file.take(MAX_SAVE_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(io)?;
            WorldSave::decode(&bytes)
        }
        fn current_head(&mut self) -> Result<Option<Head>, SaveError> {
            let file = match File::open(&self.path) {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    self.checked_head = None;
                    return Ok(None);
                }
                Err(e) => return Err(io(e)),
            };
            let signature = stamp(&file.metadata().map_err(io)?);
            if let (Some(signature), Some((old, head))) = (&signature, &self.checked_head)
                && signature == old
            {
                return Ok(Some(head.clone()));
            }
            let saved = Self::decode_file(file)?;
            let head = Head::of(&saved);
            self.checked_head = signature.map(|signature| (signature, head.clone()));
            Ok(Some(head))
        }
    }

    impl WorldStore for FileWorldStore {
        fn supports_incremental(&self) -> bool {
            self.journal.is_some()
        }
        fn diagnostics(&self) -> StoreDiagnostics {
            self.journal
                .as_ref()
                .map(|j| j.diagnostics())
                .unwrap_or_else(|| StoreDiagnostics {
                    kind: "snapshot".into(),
                    ..Default::default()
                })
        }
        async fn commit_update(
            &mut self,
            expected: u64,
            saved: &PreparedUpdate,
        ) -> Result<(), SaveError> {
            if let Some(journal) = &mut self.journal {
                journal.append(expected, saved)
            } else {
                self.commit_prepared(Some(expected), &saved.snapshot()?)
                    .await
            }
        }
        async fn load(&mut self) -> Result<Option<WorldSave>, SaveError> {
            self.read()
        }
        async fn commit(
            &mut self,
            expected: Option<u64>,
            saved: &WorldSave,
        ) -> Result<(), SaveError> {
            let prepared = PreparedSave::new(saved)?;
            self.commit_prepared(expected, &prepared).await
        }
        async fn commit_prepared(
            &mut self,
            expected: Option<u64>,
            prepared: &PreparedSave,
        ) -> Result<(), SaveError> {
            if let Some(journal) = &mut self.journal {
                return journal.checkpoint(expected, prepared);
            }
            let current = self.current_head()?;
            let saved = prepared.image();
            if current.as_ref().map(|head| head.revision) != expected
                || saved.revision
                    != expected
                        .unwrap_or(0)
                        .checked_add(1)
                        .ok_or(SaveError::Conflict)?
                || current.as_ref().is_some_and(|head| {
                    head.world != saved.world
                        || head.content != saved.content
                        || head.purpose != saved.purpose
                })
            {
                return Err(SaveError::Conflict);
            }
            let bytes = prepared.bytes();
            self.checked_head = None;
            // Same directory/filesystem; orphaned temporary files never become
            // save heads and are never considered during recovery.
            let temporary = self
                .path
                .with_file_name(format!(".{}.tmp", theframework_core::Uuid::new_v4()));
            let result = (|| {
                let mut options = OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(0o600);
                }
                let mut file = options.open(&temporary).map_err(io)?;
                file.write_all(&bytes).map_err(io)?;
                file.sync_all().map_err(io)?;
                drop(file);
                std::fs::rename(&temporary, &self.path).map_err(io)?;
                #[cfg(unix)]
                File::open(self.path.parent().unwrap())
                    .and_then(|f| f.sync_all())
                    .map_err(io)?;
                Ok(())
            })();
            if temporary.exists() {
                let _ = std::fs::remove_file(temporary);
            }
            if result.is_ok() {
                // Only immutable identity is cached, never a second world image.
                // Replacements/in-place changes invalidate this checked head.
                if let Ok(metadata) = std::fs::metadata(&self.path) {
                    self.checked_head = stamp(&metadata).map(|stamp| (stamp, Head::of(saved)));
                }
            }
            result
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub use file::FileWorldStore;
#[cfg(target_arch = "wasm32")]
#[path = "browser.rs"]
mod browser;
#[cfg(target_arch = "wasm32")]
pub use browser::BrowserWorldStore;
