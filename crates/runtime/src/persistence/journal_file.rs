//! Private native sidecars and an atomically published committed-prefix head.
//! A complete but unpublished log tail is never mistaken for a committed grant.
use super::incremental::{ImageIndex, MAX_UPDATE_BYTES};
use super::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
use theframework_core::Uuid;
const FORMAT: u32 = 3;
const MAGIC: &[u8; 8] = b"ELDTXN01";
const FRAME: usize = 8 + 4 + 32;
const MAX_RECORDS: u32 = 128;
const MAX_FILES: usize = 16;
const MAX_HEAD: usize = 8192;
#[derive(Clone, Serialize, Deserialize)]
struct Manifest {
    kind: String,
    generation: Uuid,
    world: Uuid,
    content: String,
    purpose: SavePurpose,
    revision: u64,
    base_revision: u64,
    base_bytes: u64,
    base_hash: String,
    journal_bytes: u64,
    chain: String,
    records: u32,
}
#[derive(Serialize, Deserialize)]
struct Root {
    format: u32,
    checksum: String,
    head: Manifest,
}
#[derive(Clone)]
struct Identity {
    world: Uuid,
    content: String,
    purpose: SavePurpose,
    revision: u64,
}
impl Identity {
    fn of(s: &WorldSave) -> Self {
        Self {
            world: s.world,
            content: s.content.clone(),
            purpose: s.purpose.clone(),
            revision: s.revision,
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
struct Mark {
    dev: u64,
    ino: u64,
    len: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}
fn mark(path: &Path) -> Option<Mark> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let m = std::fs::metadata(path).ok()?;
        Some(Mark {
            dev: m.dev(),
            ino: m.ino(),
            len: m.len(),
            mtime: m.mtime(),
            mtime_ns: m.mtime_nsec(),
            ctime: m.ctime(),
            ctime_ns: m.ctime_nsec(),
        })
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
}
#[derive(Clone)]
struct Cached {
    root_hash: String,
    manifest: Option<Manifest>,
    identity: Identity,
    base: Option<Mark>,
    log: Option<Mark>,
}
pub(crate) struct JournalFile {
    path: PathBuf,
    cached: Option<Cached>,
    failed: bool,
    stats: StoreDiagnostics,
    #[cfg(test)]
    fault: u8,
}
fn error(e: impl std::fmt::Display) -> SaveError {
    SaveError::Store(e.to_string())
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn chain(previous: &str, bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(previous.as_bytes());
    h.update((bytes.len() as u64).to_be_bytes());
    h.update(bytes);
    format!("{:x}", h.finalize())
}
fn read(path: &Path, limit: usize) -> Result<Vec<u8>, SaveError> {
    let mut bytes = vec![];
    File::open(path)
        .map_err(error)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(error)?;
    if bytes.len() > limit {
        return Err(error("Storage file exceeds size limit"));
    }
    Ok(bytes)
}
pub(crate) fn is_manifest(bytes: &[u8]) -> bool {
    #[derive(Deserialize)]
    struct Probe {
        #[serde(default)]
        format: u32,
    }
    serde_json::from_slice::<Probe>(bytes).is_ok_and(|h| h.format == FORMAT)
}
fn parse(bytes: &[u8]) -> Result<Manifest, SaveError> {
    if bytes.len() > MAX_HEAD {
        return Err(error("Journal head exceeds limit"));
    }
    let root: Root = serde_json::from_slice(bytes).map_err(error)?;
    let h = root.head;
    if root.format != FORMAT
        || h.kind != "eldiron.world.journal"
        || h.generation.is_nil()
        || h.world.is_nil()
        || h.content.len() != 64
        || h.base_hash.len() != 64
        || h.chain.len() != 64
        || h.base_revision == 0
        || h.records > MAX_RECORDS
        || h.journal_bytes > MAX_UPDATE_BYTES as u64
        || h.base_bytes > MAX_SAVE_BYTES as u64
        || h.base_revision.checked_add(h.records as u64) != Some(h.revision)
        || hash(&serde_json::to_vec(&h).map_err(error)?) != root.checksum
    {
        return Err(error("Invalid journal head/checksum"));
    }
    Ok(h)
}
fn encode(h: &Manifest) -> Result<Vec<u8>, SaveError> {
    serde_json::to_vec(&Root {
        format: FORMAT,
        checksum: hash(&serde_json::to_vec(h).map_err(error)?),
        head: h.clone(),
    })
    .map_err(error)
}
fn directory(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".parts");
    PathBuf::from(name)
}
fn paths(path: &Path, h: &Manifest) -> (PathBuf, PathBuf) {
    let dir = directory(path);
    (
        dir.join(format!("{}.base", h.generation)),
        dir.join(format!("{}.log", h.generation)),
    )
}
fn sync_dir(path: &Path) -> Result<(), SaveError> {
    #[cfg(unix)]
    File::open(path).and_then(|f| f.sync_all()).map_err(error)?;
    Ok(())
}
fn options() -> OpenOptions {
    let mut o = OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o
}
fn private_dir(path: &Path) -> Result<(), SaveError> {
    if path.is_symlink() {
        return Err(error("Journal directory cannot be a symlink"));
    }
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match builder.create(path) {
        Ok(()) => sync_dir(path.parent().unwrap())?,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(error(e)),
    }
    Ok(())
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), SaveError> {
    let mut f = options().open(path).map_err(error)?;
    f.write_all(bytes).map_err(error)?;
    f.sync_all().map_err(error)
}
fn publish(path: &Path, bytes: &[u8]) -> Result<(), SaveError> {
    let tmp = path.with_file_name(format!(".eldiron-head-{}.tmp", Uuid::new_v4()));
    let result = (|| {
        write_new(&tmp, bytes)?;
        std::fs::rename(&tmp, path).map_err(error)?;
        sync_dir(path.parent().unwrap())
    })();
    let _ = std::fs::remove_file(tmp);
    result
}
fn restore(path: &Path, bytes: &[u8]) -> Result<(WorldSave, Option<Manifest>), SaveError> {
    if !is_manifest(bytes) {
        return Ok((WorldSave::decode(bytes)?, None));
    }
    let h = parse(bytes)?;
    let (base, log) = paths(path, &h);
    if directory(path).is_symlink() || base.is_symlink() || log.is_symlink() {
        return Err(error("Journal parts cannot be symlinks"));
    }
    let checkpoint = read(&base, MAX_SAVE_BYTES)?;
    if checkpoint.len() as u64 != h.base_bytes || hash(&checkpoint) != h.base_hash {
        return Err(error("Checkpoint checksum/length differs"));
    }
    let image = WorldSave::decode(&checkpoint)?;
    if image.world != h.world
        || image.content != h.content
        || image.purpose != h.purpose
        || image.revision != h.base_revision
    {
        return Err(error("Checkpoint identity differs from journal head"));
    }
    let mut file = File::open(&log).map_err(error)?;
    let mut bytes = vec![];
    (&mut file)
        .take(h.journal_bytes)
        .read_to_end(&mut bytes)
        .map_err(error)?;
    if bytes.len() as u64 != h.journal_bytes {
        return Err(error("Committed journal prefix was truncated"));
    }
    let mut previous = h.base_hash.clone();
    let mut offset = 0;
    let mut revision = image.revision;
    let mut records = 0;
    let mut index = if h.records > 0 {
        Some(ImageIndex::from_image(&image)?)
    } else {
        None
    };
    while offset < bytes.len() {
        if bytes.len() - offset < FRAME || &bytes[offset..offset + 8] != MAGIC {
            return Err(error("Invalid committed journal frame"));
        }
        let len = u32::from_be_bytes(bytes[offset + 8..offset + 12].try_into().unwrap()) as usize;
        if len > MAX_UPDATE_BYTES || len > bytes.len() - offset - FRAME {
            return Err(error("Invalid committed transaction length"));
        }
        let payload = &bytes[offset + FRAME..offset + FRAME + len];
        let digest = chain(&previous, payload);
        let expected = Sha256::digest(digest.as_bytes());
        if bytes[offset + 12..offset + FRAME] != expected[..] {
            return Err(error("Committed transaction checksum differs"));
        }
        revision = index
            .as_mut()
            .ok_or_else(|| error("Unexpected journal transaction"))?
            .apply(payload, h.world, revision)?;
        previous = digest;
        offset += FRAME + len;
        records += 1;
        if records > MAX_RECORDS {
            return Err(error("Replay record quota exceeded"));
        }
    }
    if previous != h.chain || records != h.records || revision != h.revision {
        return Err(error("Journal chain/revision differs"));
    }
    let image = if let Some(index) = index {
        index.materialize()?
    } else {
        image
    };
    if image.world != h.world
        || image.content != h.content
        || image.purpose != h.purpose
        || image.revision != h.revision
    {
        return Err(error("Replayed identity differs"));
    }
    Ok((image, Some(h)))
}
/// Readers retry if compaction retires a generation between opening the head
/// and its parts. A corrupt/missing part of an unchanged head is never skipped.
pub(crate) fn read_image(path: &Path) -> Result<WorldSave, SaveError> {
    for _ in 0..3 {
        let bytes = read(path, MAX_SAVE_BYTES)?;
        let result = restore(path, &bytes);
        if result.is_ok() {
            return result.map(|v| v.0);
        }
        if read(path, MAX_SAVE_BYTES).is_ok_and(|current| current != bytes) {
            continue;
        }
        return result.map(|v| v.0);
    }
    Err(SaveError::Busy)
}
impl JournalFile {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self {
            path,
            #[cfg(test)]
            fault: 0,
            cached: None,
            failed: false,
            stats: StoreDiagnostics {
                kind: "journal".into(),
                ..Default::default()
            },
        }
    }
    fn trip(&self, point: u8) -> Result<(), SaveError> {
        #[cfg(test)]
        if self.fault == point {
            return Err(error("Injected publication failure"));
        }
        let _ = point;
        Ok(())
    }
    pub(crate) fn diagnostics(&self) -> StoreDiagnostics {
        self.stats.clone()
    }
    fn cache(&mut self, root_hash: String, image: &WorldSave, manifest: Option<Manifest>) {
        let (base, log) = manifest
            .as_ref()
            .map(|h| {
                let (b, l) = paths(&self.path, h);
                (mark(&b), mark(&l))
            })
            .unwrap_or((None, None));
        self.stats.records = manifest.as_ref().map_or(0, |h| h.records as u64);
        self.stats.journal_bytes = manifest.as_ref().map_or(0, |h| h.journal_bytes);
        self.cached = Some(Cached {
            root_hash,
            identity: Identity::of(image),
            manifest,
            base,
            log,
        });
    }
    pub(crate) fn load(&mut self) -> Result<Option<WorldSave>, SaveError> {
        let bytes = match read(&self.path, MAX_SAVE_BYTES) {
            Ok(v) => v,
            Err(_)
                if std::fs::symlink_metadata(&self.path)
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
            {
                self.cached = None;
                return Ok(None);
            }
            Err(e) => return Err(e),
        };
        let (image, manifest) = restore(&self.path, &bytes)?;
        self.cache(hash(&bytes), &image, manifest);
        Ok(Some(image))
    }
    fn current(&mut self) -> Result<Option<Cached>, SaveError> {
        let bytes = match read(&self.path, MAX_SAVE_BYTES) {
            Ok(v) => v,
            Err(_)
                if std::fs::symlink_metadata(&self.path)
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
            {
                self.cached = None;
                return Ok(None);
            }
            Err(e) => return Err(e),
        };
        let digest = hash(&bytes);
        if let Some(cache) = &self.cached
            && cache.root_hash == digest
            && let Some(h) = &cache.manifest
        {
            let (base, log) = paths(&self.path, h);
            if !directory(&self.path).is_symlink()
                && !base.is_symlink()
                && !log.is_symlink()
                && cache.base.is_some()
                && cache.base == mark(&base)
                && cache.log.is_some()
                && cache.log == mark(&log)
            {
                return Ok(Some(cache.clone()));
            }
        }
        let (image, manifest) = restore(&self.path, &bytes)?;
        self.cache(digest, &image, manifest);
        Ok(self.cached.clone())
    }
    fn compare(
        &mut self,
        expected: Option<u64>,
        image: &WorldSave,
    ) -> Result<Option<Cached>, SaveError> {
        if self.failed {
            return Err(SaveError::NeedsReload);
        }
        let current = self.current()?;
        if current.as_ref().map(|c| c.identity.revision) != expected
            || image.revision
                != expected
                    .unwrap_or(0)
                    .checked_add(1)
                    .ok_or(SaveError::Conflict)?
            || current.as_ref().is_some_and(|c| {
                c.identity.world != image.world
                    || c.identity.content != image.content
                    || c.identity.purpose != image.purpose
            })
        {
            return Err(SaveError::Conflict);
        }
        Ok(current)
    }
    pub(crate) fn checkpoint(
        &mut self,
        expected: Option<u64>,
        saved: &PreparedSave,
    ) -> Result<(), SaveError> {
        let previous = self.compare(expected, saved.image())?;
        let dir = directory(&self.path);
        private_dir(&dir)?;
        if std::fs::read_dir(&dir)
            .map_err(error)?
            .take(MAX_FILES)
            .count()
            >= MAX_FILES - 2
        {
            return Err(error(
                "Journal parts quota exceeded; export/restore to a fresh slot",
            ));
        }
        let generation = Uuid::new_v4();
        let image = saved.image();
        let digest = hash(saved.bytes());
        let h = Manifest {
            kind: "eldiron.world.journal".into(),
            generation,
            world: image.world,
            content: image.content.clone(),
            purpose: image.purpose.clone(),
            revision: image.revision,
            base_revision: image.revision,
            base_bytes: saved.bytes().len() as u64,
            base_hash: digest.clone(),
            journal_bytes: 0,
            chain: digest,
            records: 0,
        };
        let (base, log) = paths(&self.path, &h);
        let encoded = encode(&h)?;
        let result = (|| {
            write_new(&base, saved.bytes())?;
            write_new(&log, &[])?;
            sync_dir(&dir)?;
            self.trip(1)?;
            publish(&self.path, &encoded)?;
            self.trip(2)
        })();
        if let Err(e) = result {
            self.failed = true;
            return Err(e);
        }
        self.stats.checkpoints += 1;
        self.stats.bytes_written = self
            .stats
            .bytes_written
            .saturating_add(saved.bytes().len() as u64 + encoded.len() as u64);
        self.cache(hash(&encoded), image, Some(h));
        // Only retire the exact generation we verified/owned. Unknown files are
        // left alone, and the directory quota bounds orphan accumulation.
        if let Some(old) = previous.and_then(|c| c.manifest) {
            let (b, l) = paths(&self.path, &old);
            let _ = std::fs::remove_file(b);
            let _ = std::fs::remove_file(l);
            let _ = sync_dir(&dir);
        }
        Ok(())
    }
    pub(crate) fn append(
        &mut self,
        expected: u64,
        update: &PreparedUpdate,
    ) -> Result<(), SaveError> {
        let current = self.compare(Some(expected), update.image())?;
        let Some(mut h) = current.and_then(|c| c.manifest) else {
            return self.checkpoint(Some(expected), &update.snapshot()?);
        };
        if update.previous_revision() != expected {
            return Err(SaveError::Conflict);
        }
        let frame_size = FRAME + update.bytes().len();
        if update.bytes().len() > MAX_UPDATE_BYTES
            || h.records >= MAX_RECORDS
            || h.journal_bytes.saturating_add(frame_size as u64) > MAX_UPDATE_BYTES as u64
        {
            return self.checkpoint(Some(expected), &update.snapshot()?);
        }
        let (_, log) = paths(&self.path, &h);
        let digest = chain(&h.chain, update.bytes());
        let mut frame = Vec::with_capacity(frame_size);
        frame.extend(MAGIC);
        frame.extend((update.bytes().len() as u32).to_be_bytes());
        frame.extend(Sha256::digest(digest.as_bytes()));
        frame.extend(update.bytes());
        let committed = h.journal_bytes;
        h.journal_bytes += frame.len() as u64;
        h.revision = update.image().revision;
        h.records += 1;
        h.chain = digest;
        let encoded = encode(&h)?;
        let result = (|| {
            let mut file = OpenOptions::new().write(true).open(&log).map_err(error)?;
            if file.metadata().map_err(error)?.len() > committed {
                file.set_len(committed).map_err(error)?;
                self.stats.tails_discarded += 1;
            }
            file.seek(SeekFrom::Start(committed)).map_err(error)?;
            file.write_all(&frame).map_err(error)?;
            file.sync_all().map_err(error)?;
            drop(file);
            self.trip(1)?;
            publish(&self.path, &encoded)?;
            self.trip(2)
        })();
        if let Err(e) = result {
            self.failed = true;
            self.cached = None;
            return Err(e);
        }
        self.stats.transactions += 1;
        self.stats.bytes_written = self
            .stats
            .bytes_written
            .saturating_add(frame.len() as u64 + encoded.len() as u64);
        self.cache(hash(&encoded), update.image(), Some(h));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::incremental::tests::fixture;
    use super::*;
    fn slot() -> (PathBuf, JournalFile) {
        let dir = std::env::temp_dir().join(format!("eldiron-journal-{}", Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("world.save");
        (dir, JournalFile::new(path))
    }
    fn update(base: &WorldSave) -> PreparedUpdate {
        let mut next = base.clone();
        next.revision += 1;
        next.tick += 1;
        next.world_state.regions[0].entities[0]
            .wallet
            .balances
            .insert("c".into(), 7);
        next.action_streams
            .entry(base.characters[0].id)
            .or_default()
            .append(next.tick, "a".repeat(64), serde_json::json!("paid"))
            .unwrap();
        PreparedUpdate::new(base, &ImageIndex::from_image(base).unwrap(), next).unwrap()
    }
    #[test]
    fn committed_prefix_and_ambiguous_publication_recover_together() {
        for fault in [1, 2] {
            let (dir, mut store) = slot();
            let base = fixture();
            store
                .checkpoint(None, &PreparedSave::new(&base).unwrap())
                .unwrap();
            let transaction = update(&base);
            store.fault = fault;
            assert!(store.append(1, &transaction).is_err());
            assert!(matches!(
                store.append(1, &transaction),
                Err(SaveError::NeedsReload)
            ));
            let mut reopened = JournalFile::new(store.path.clone());
            let recovered = reopened.load().unwrap().unwrap();
            assert_eq!(recovered.revision, fault as u64);
            assert_eq!(
                recovered
                    .action_streams
                    .contains_key(&base.characters[0].id),
                fault == 2
            );
            if fault == 1 {
                reopened.append(1, &transaction).unwrap();
                assert_eq!(reopened.diagnostics().tails_discarded, 1);
            }
            let recovered = read_image(&store.path).unwrap();
            assert_eq!(recovered.revision, 2);
            assert_eq!(recovered.action_streams[&base.characters[0].id].last, 1);
            assert_eq!(
                recovered.world_state.regions[0].entities[0].wallet.balances["c"],
                7
            );
            std::fs::remove_dir_all(dir).unwrap();
        }
    }
    #[test]
    fn corruption_truncation_and_missing_parts_fail_closed() {
        let (dir, mut store) = slot();
        let base = fixture();
        store
            .checkpoint(None, &PreparedSave::new(&base).unwrap())
            .unwrap();
        store.append(1, &update(&base)).unwrap();
        let h = parse(&read(&store.path, MAX_SAVE_BYTES).unwrap()).unwrap();
        let (b, l) = paths(&store.path, &h);
        let bytes = read(&l, MAX_UPDATE_BYTES).unwrap();
        std::fs::write(&l, &bytes[..bytes.len() - 1]).unwrap();
        assert!(read_image(&store.path).is_err());
        let mut corrupt = bytes.clone();
        corrupt[FRAME + 10] ^= 1;
        std::fs::write(&l, &corrupt).unwrap();
        assert!(read_image(&store.path).is_err());
        std::fs::write(&l, &bytes).unwrap();
        std::fs::rename(&b, b.with_extension("off")).unwrap();
        assert!(read_image(&store.path).is_err());
        std::fs::rename(b.with_extension("off"), &b).unwrap();
        assert_eq!(read_image(&store.path).unwrap().revision, 2);
        let mut root = read(&store.path, MAX_SAVE_BYTES).unwrap();
        root[20] ^= 1;
        std::fs::write(&store.path, root).unwrap();
        assert!(read_image(&store.path).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn snapshots_upgrade_compact_and_export_without_replaying_grants() {
        let (dir, mut store) = slot();
        let base = fixture();
        std::fs::write(&store.path, base.encode().unwrap()).unwrap(); // legacy standalone working slot
        store.load().unwrap();
        store.append(1, &update(&base)).unwrap(); // upgrades via checkpoint
        assert_eq!(store.diagnostics().records, 0);
        let current = store.load().unwrap().unwrap();
        store.append(2, &update(&current)).unwrap();
        let current = store.load().unwrap().unwrap();
        let export = dir.join("portable.snapshot");
        write_snapshot_new(&export, &current).unwrap();
        assert_eq!(
            WorldSave::decode(&read(&export, MAX_SAVE_BYTES).unwrap())
                .unwrap()
                .revision,
            3
        );
        let mut next = current.clone();
        next.revision += 1;
        store
            .checkpoint(Some(3), &PreparedSave::new(&next).unwrap())
            .unwrap();
        assert_eq!(
            std::fs::read_dir(directory(&store.path)).unwrap().count(),
            2
        );
        assert_eq!(
            read_image(&store.path).unwrap().action_streams[&base.characters[0].id].last,
            2
        );
        // Bound normal replay and fall back to a full checkpoint at the cap.
        for _ in 0..=MAX_RECORDS {
            let base = store.load().unwrap().unwrap();
            store.append(base.revision, &update(&base)).unwrap();
        }
        assert_eq!(store.diagnostics().records, 0);
        assert_eq!(
            std::fs::read_dir(directory(&store.path)).unwrap().count(),
            2
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn checkpoint_publication_recovers_old_or_new_generation() {
        for fault in [1, 2] {
            let (dir, mut store) = slot();
            let base = fixture();
            store
                .checkpoint(None, &PreparedSave::new(&base).unwrap())
                .unwrap();
            let mut next = base.clone();
            next.revision += 1;
            next.host_data
                .insert("checkpoint".into(), serde_json::json!(true));
            store.fault = fault;
            assert!(
                store
                    .checkpoint(Some(1), &PreparedSave::new(&next).unwrap())
                    .is_err()
            );
            let recovered = read_image(&store.path).unwrap();
            assert_eq!(recovered.revision, fault as u64);
            assert_eq!(recovered.host_data.contains_key("checkpoint"), fault == 2);
            assert_eq!(
                std::fs::read_dir(directory(&store.path)).unwrap().count(),
                4
            );
            std::fs::remove_dir_all(dir).unwrap();
        }
    }
    #[test]
    fn orphan_quota_prevents_unbounded_crash_artifacts() {
        let (dir, mut store) = slot();
        let base = fixture();
        store
            .checkpoint(None, &PreparedSave::new(&base).unwrap())
            .unwrap();
        for n in 0..12 {
            write_new(&directory(&store.path).join(format!("unknown-{n}")), &[]).unwrap();
        }
        let before = read(&store.path, MAX_SAVE_BYTES).unwrap();
        let mut next = base.clone();
        next.revision += 1;
        assert!(
            store
                .checkpoint(Some(1), &PreparedSave::new(&next).unwrap())
                .is_err()
        );
        assert_eq!(read(&store.path, MAX_SAVE_BYTES).unwrap(), before);
        assert_eq!(
            std::fs::read_dir(directory(&store.path)).unwrap().count(),
            14
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
