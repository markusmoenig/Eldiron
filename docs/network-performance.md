# Storage and replication optimization

This pass targets the stalls measured in [phase 8](../concepts/networking-evidence/phase8-load.md). It keeps one authoritative owner per world, atomic effects/receipts and the existing embedded single-player/Creator session paths. It does not introduce region workers, a database service or delayed durability.

## Replication

The current gameplay transport is **WSS protocol 4**, WebSocket subprotocol `eldiron.v4`, with replication DTO version 3. Rebuild the server and native/browser clients together. Earlier wire versions are rejected; project mode, account/demo selection and HTTPS content delivery keep their existing roles.

A region delta can now contain compact entity poses: identity, position, orientation, tilt and motion revision. An enter, attribute deletion/change, equipment/inventory/wallet change or permission change still sends a complete recipient-filtered entity replacement. A pose cannot introduce an unknown actor, replace private data, conflict with a removal/replacement, repeat an identity or contain non-finite coordinates. The receiver validates the complete frame before changing visible state. Baselines still replace the selected world state and generation.

The authority captures public projections once per publication and shares immutable records among recipients. It tracks public appearance revisions separately from motion, so moving an actor does not repeatedly clone and compare every public attribute for every client. Owned/party entities and explicit container/trade access still use recipient-specific projections; private projections never enter the shared public cache. Policy changes invalidate caches and baselines; removal/reset prunes cached identities. The cache remains proportional to the live world rather than retaining historical poses.

Client replicas maintain entity/item identity indices. Pose application is proportional to the changed poses; complete replacements and enters use indexed lookup. Removals rebuild the affected index while preserving vector order, and baselines replace indices with their maps. Both the dedicated authority's recipient mirrors and actual native/WASM receivers use this path. Presentation continues to use independent predicted/interpolated maps; no new simulation, inventory callbacks or transient events run during pose application.

## Durable encoding and stores

The world schema remains version 1. The save **container** now writes version 2:

```json
{"format":2,"checksum":"SHA-256 of exact payload bytes","payload":{"version":1}}
```

The illustration omits the rest of the world. The payload is a raw JSON object rather than an escaped second JSON document inside a string. Loading borrows that payload and decodes directly into typed records, avoiding a second complete JSON value tree. Checksums and the 128 MiB encoded limit remain enforced. Non-finite/invalid data still fails checked roundtrip validation.

Existing container-1 saves/backups remain readable, including the original envelope without a `format` field. They upgrade on the next successful write; older binaries cannot read container 2. Keep backups and matching releases for rollback. The checksum covers exact payload bytes, so reformatting a snapshot changes its validity; use the recovery tools to inspect or migrate it.

`PreparedSave` owns a checked world image and its exact encoding. Its fields are private and accessors are read-only. A durable operation prepares it once, validates any staged engine reconstruction before commit, passes the same encoding to storage, and installs the validated head only after storage succeeds. Sequenced live actions also avoid constructing two full JSON trees solely to recheck a metadata-edit callback which is internally a no-op. Live operations with a host edit callback retain that guard. Capture copies metadata and captures the live world directly instead of cloning a discarded previous world image.

`WorldStore::commit_prepared` has a compatible default that calls the existing `commit` method, preserving custom stores and fault-injection behavior. Native, memory and browser adapters reuse checked bytes. Native writes still use a private same-directory temporary file, file sync, atomic rename and Unix parent-directory sync. IndexedDB still stores identity/revision/bytes in one strict read-write transaction; its database schema and slot isolation do not change. No acknowledgment or committed frame is released early.

Under its exclusive owner lock, the Unix file adapter caches only the validated head identity and file signature (device, inode, length, modification/change times with nanoseconds). If that signature changes, it reads and verifies the head before comparing revisions; ordinary `load` always verifies the file. A failed write invalidates the cache. Platforms without that signature use the full verified-read fallback. This is an optimization for the existing cooperative single-owner contract, not permission for another writer to edit an active world file.

## Metrics and hosting

Save diagnostics now separate `capture`, `encoding` (serialization plus checked decode) and `commit` (adapter/CAS/durable I/O). Earlier commit measurements included repeated serialization and decoding, so compare complete server slices and combined save costs, not just the renamed cost boundary. Region/simulation/projection/encoding and queue/byte counters remain available through the private operator metrics file.

See the [measured comparison](../concepts/networking-evidence/storage-replication-optimization.md) and its raw samples. The fixtures are short in-process synthetic-crowd workloads, with real authored maps and file commits; they do not establish full-AI, TLS, sustained-load or paid-hosting capacity. CPU simulation, live-world capture, synchronous checkpoint/commit I/O, resource/script limits and off-site operations remain separate considerations. Database adapters must preserve compare-and-swap, atomic effects/receipts and commit-before-acknowledgment semantics. The existing store boundary supports that work without changing project modes or exposing storage details to players.


## Incremental native commits and action groups

Dedicated `FileWorldStore::open_journaled` uses a checked full checkpoint plus changed records. Default `open` retains standalone snapshots and detects existing journal heads on load. The portable export format remains container 2; storage format 3 is a head plus private sidecars. Browser, memory and custom stores keep snapshot commits, and `WorldStore::commit_update` provides a compatible snapshot fallback.

`PreparedUpdate` holds a checked resulting image, changed records and its previous revision. The owner maintains an immutable record index. Top-level metadata is discovered through serde so new persisted fields are included; actors include complete inventory/equipment/wallet trees, and region records include VM state, timers, notifications and behaviors. Record order is explicit. Unchanged geometry reuses its encoded record after a binary SHA-256 comparison that preserves float bits; this does not depend on renderer dirty flags. Changed records still roundtrip through typed decoding. A full world capture remains necessary for correctness; this is not a best-effort dirty-flag journal.

Groups stage at most eight sequenced live actions, validate against staged receipts, capture once, and commit state and all receipts together. `saved()` continues to expose only the committed head; `action_journal()` supplies staged sequence checks. Other durable APIs reject work while a group is open. The host gates all encoded output and drains replication internally after each action so private offer revocation stays current. Commit failure poisons the owner and discards the output gate. The transport collects for at most two milliseconds and preserves control-command order. Single action calls remain immediately durable.

The [operations contract](server-operations.md) defines fsync/publication ordering, bounded replay, compaction, tail recovery and portable backup commands. Storage work remains synchronous on the native world owner. Grouping reduces repeated capture and I/O; it does not move scripts or saves to another process or make a capacity promise.

See [incremental storage evidence](../concepts/networking-evidence/incremental-storage.md) for current controlled results, including the small-world individual-commit regression and remaining simulation limits.

The subsequent [simulation profiling/scheduling phase](simulation-scheduling.md) adds detailed CPU samples, cheaper actor collision/merging and explicit inactive-region suspension. Its [controlled measurements](../concepts/networking-evidence/simulation-performance.md) preserve the earlier evidence.


## Simulation follow-up

The subsequent [simulation phase](simulation-scheduling.md) profiles inner collision, actor clone/merge, script and behavior costs; removes distant-actor attribute/allocation work and quadratic identity merging; and adds opt-in unoccupied-region suspension for dedicated projects. It preserves embedded and active-region clocks. When at most one region is awake under that policy, native region dispatch is sequential; multiple awake regions retain parallel execution. See [controlled measurements and recovery checks](../concepts/networking-evidence/simulation-performance.md).
