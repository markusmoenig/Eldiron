# Dedicated world operations

The phase 8 tools operate one world per dedicated process. Project-selected single player and temporary Creator tests keep their existing embedded runtime. WebSocket protocol 4 and browser/WASM transport remain unchanged. The new recovery binary and filesystem operations are native operator tools.

## Starting a saved world

```sh
cargo build --release -p eldiron-server --bins

target/release/eldiron-server --project game.eldiron --state private/world.save \
  --bind 0.0.0.0:8443 --tls-cert cert.pem --tls-key key.pem \
  --origin https://game.example --max-players 8 \
  --metrics-file private/metrics.json \
  --backup-dir private/backups --backup-seconds 300 --backup-retain 8
```

`--max-players` admits 1–64 simultaneously controlled characters. It is an admission limit, not a capacity promise; lower it for demanding maps or constrained hosts. Accountless `--demo` supports the same admission/metrics controls, remains disposable and refuses durable backup configuration. Account registration/login and guest play remain selected by the host mode.

Metrics and backup paths belong outside any public web root. Backups include private accounts, password hashes, characters, inventories and retry journals. Files are created with mode 0600 on Unix. Backups have checksums; they are not encrypted or signed. Keep an additional protected copy on separate storage with the matching project release and server build.

## Metrics

`--metrics-file` enables CPU instrumentation and atomically replaces a private JSON snapshot every five seconds, at startup and on normal/fail-closed exit. There is no public metrics or administrator HTTP endpoint. A collector may read this file locally. Output failure increments a counter and logs an operator error; it does not stop gameplay.

The schema reports loaded/occupied region counts, entity/item counts, cache bytes, player admission, queue depth/peaks, bytes enqueued/sent, slow-client/replication disconnects, busy/rate rejections and discarded wall time. Optional runtime samples separate simulation, event collection and replication; each region separates movement, redraw and game ticks. Host samples include commands, complete action groups (including commit), frame encoding, complete host ticks, maintenance and backups. Durable sessions report capture, checked encoding and file-commit time, failed commits, grouped action counts and reload-required status. `saves.storage` reports backend kind; the journal backend also reports current replay records/bytes, process-lifetime append/checkpoint counts, bytes written and discarded unpublished tails. Snapshot adapters leave those additional storage counters at zero (unavailable); their commits remain counted in the save timing summary. Byte counts include checkpoint/log/head payloads, excluding filesystem metadata and lock files. Transport samples include socket-write latency, command-queue peak and authentication worker saturation.

Timings use microseconds with lifetime counts/totals/maxima and exact percentiles over the latest 256 samples per category. These are elapsed wall-time measurements of each phase, not operating-system CPU accounting. Samples overlap: parallel region work can overlap, region work is inside simulation, encoding is inside host tick/commands, and save work can occur inside commands/maintenance. Do not add overlapping totals. Transport byte counters count application payloads, excluding TLS/WebSocket framing. Counts contain no account names, bearer tokens, IP addresses, chat, inventory contents or request payloads.

Client `MotionClient::diagnostics()` separately exposes prediction/replay/correction counts, cumulative/last/maximum positional correction distance (world units), pose snaps and pending input/edge counts. The existing `corrections` count measures smoothed corrections; `pose_snaps` counts reconciliation snaps. Distance samples include both. Compare count changes over an interval to measure correction frequency. This stays client-local; the host does not accept untrusted diagnostic reports. Embedded sessions leave timing instrumentation disabled unless explicitly enabled with `GameSession::enable_diagnostics(true)` or the corresponding durable-session API.

## Journaled working saves

Dedicated saved worlds use a format-3 head at `world.save` and a private `world.save.parts/` directory. **Keep these together.** Copying just the head does not copy the world. Use `eldiron-world backup` or configured live backups to obtain a portable, complete `.snapshot`. Existing standalone container-1/2 working saves load normally and upgrade on the next successful dedicated write. Older binaries cannot read format 3; export/restore a portable snapshot with the current tools before changing deployment versions.

Each transaction appends changed records and retry receipts, syncs the log, then atomically publishes and syncs the committed-prefix head. Recovery verifies that prefix against the checkpoint, checksums, chain, world and revision. A damaged/truncated committed prefix fails closed. Unpublished trailing bytes are ignored on load and truncated before the next append; they cannot turn into an acknowledged grant. A publish error can be ambiguous: stop and reopen, then use the recovered receipt.

Replay is bounded to 128 transactions and 16 MiB. Crossing either limit, or a transaction too large for the journal, writes a new full checkpoint. Periodic simulation checkpoints and clean shutdown also compact. Normal compaction retires only the previous verified generation after publishing the new one. Base/log files use mode 0600 and the parts directory 0700 on Unix. Crash orphans are left intact; a directory quota of 16 entries stops repeated accumulation. If it fills, inspect/export the verified head and restore to a fresh slot, preserving the original for investigation. Do not manually delete referenced parts.

Consecutive saved gameplay actions share a group of at most eight, collected for at most two milliseconds. A tick, authentication, join, content request or other control is a barrier. Each action retains an independent sequence and receipt; the entire group uses one save revision. Replies, frames and private offers are buffered until commit succeeds, including cached replies mixed with new actions. Offers update internally after each action so stale choices cannot be reused within a group. Buffered output shares each peer's 64-packet/4-MiB budget; an overflowing peer disconnects after commit. A failed group drops all buffered output and stops the owner.

Default single-player and Creator file stores retain standalone snapshots; IndexedDB keeps its existing atomic snapshot transaction and database schema. Custom stores retain the compatible snapshot fallback. No network protocol change is required.

## Live backups and retention

A periodic backup first checkpoints the complete world through the same atomic durable store, then writes a verified immutable `.snapshot` artifact. Effects, account metadata, characters, clock, creation receipts and action watermarks travel together. The minimum interval is 30 seconds; retention is 1–64 verified images (default eight). Temporary files are synced before publication, output creation refuses an existing destination, and Unix parent directories are synced.

Retention only removes verified snapshots for this world with the generated filename prefix. Other worlds, unrelated files, symlinks and corrupt files are left alone. The newest verified save revisions survive; corrupt files do not consume retention slots. A backup directory has a 4096-entry preflight quota, preventing repeated writes when retention cannot run. Use a directory per world. An artifact/retention error is observable and retried at the next interval. Failure of the underlying durable checkpoint stops the host and requires reopening the committed head.

`.snapshot` is reserved for immutable recovery artifacts. A working `--state` must use a separate name such as `world.save`; the file store refuses to open a `.snapshot` for writing. Working paths retain their existing exclusive owner lock.

## Inspect, copy and restore

```sh
# Validates schema and checksum; optionally also the matching project release.
target/release/eldiron-world inspect --source private/world.save --project game.eldiron

# Copies the last complete committed head, even while the owner is running.
# Use periodic live backups when a fresh checkpoint of live simulation is needed.
target/release/eldiron-world backup --source private/world.save --output private/manual.snapshot

# Restore validates project compatibility and creates a fresh working path.
target/release/eldiron-world restore --source private/manual.snapshot \
  --project game.eldiron --output private/restored.save
```

Inspection prints a summary of world/revision/content identity, clock and object/receipt counts. It does not print account metadata or inventory contents. Every output must be a fresh path; no command overwrites an existing file or an actively owned working save. Corrupt, unsupported or oversized images fail validation.

For recovery, stop the old owner first, preserve its files for investigation, restore to a fresh path, then start the matching release with `--state private/restored.save`. Never run the original and restored copies as two owners of the same logical world. A restore retains accounts, body IDs, starter items and action/creation receipts. Sessions reconnect with fresh connection handles and new baselines. Changes newer than the chosen backup are lost; receipts from the selected snapshot are authoritative.

For an existing account's forgotten password, `eldiron-world reset-password` writes a fresh private save using piped password input, a matching project and the existing username. It retains world/account/character identities and increases the account security revision. Stop/switch the intended owner as for restore; changing a copy does not revoke a still-running original. See [account management and the recovery command](account-management.md).

A storage failure can be ambiguous (the write may have committed before an error). The host fails closed rather than publishing more state or saving potentially uncommitted effects on shutdown. Reopen the committed head; durable retry receipts determine whether an operation already happened. `healthy=false` while serving indicates an unusable durable owner; `saves.poisoned=true` identifies a poisoned persistence session rather than an ordinary client disconnect. A clean shutdown also ends the runtime incarnation and can leave final health false: assess shutdown through its successful process exit, durable checkpoint and zero active transports.

Legacy geometry faces that omitted serialized IDs now receive deterministic object-scoped identities on load. Existing authored IDs and paint links are preserved. This prevents fresh loads of the same release from generating different save/client fingerprints. A save created with the former random missing IDs still requires its matching materialized release or a specific repair; the recovery tool does not bypass compatibility checks or guess old random IDs.

## Deployment migration

```sh
target/release/eldiron-world migrate-deployment --source private/world.save \
  --from-project old.eldiron --to-project relocated.eldiron --output private/relocated.save
```

This explicitly permits project mode, multiplayer deployment configuration and TOML formatting changes only, under the current engine/schema. The source must match the old strict save identity; both projects must retain the same normalized gameplay content identity. The new image advances its revision and changes only its strict project identity. World state, account ownership and journals remain intact. Changes to gameplay config, geometry, templates, rules or engine/schema require a domain-specific migration and are refused by this tool. Keep the original release and snapshot for rollback.

## Time, queues and overload

By default, all loaded regions continue simulation, including those with no controlled players. Dedicated projects can now explicitly suspend named unoccupied regions using [simulation scheduling](simulation-scheduling.md); occupied/spawning regions stay active and wake never catches up sleeping time. No region workers are introduced. Saved simulation clock/counters resume from the checkpoint; offline wall time is not simulated. The dedicated host caps elapsed wall time to 250 ms per host tick and counts discarded time. Overload slows world time rather than building an unbounded catch-up backlog. Local single-player and Creator clock behavior is unchanged.

The transport allows up to 128 accepted TCP/TLS connections and 64 gameplay WebSockets, leaving HTTP content headroom when gameplay slots are full. Existing IP/connection/authentication rate limits still apply. There are 128 pending world commands and four password-hash jobs. Each gameplay output queue has 64 packets and a 4 MiB queued-payload budget. Slow/overflowed clients lose their connection and can reconnect for a baseline; they do not terminate other players' world. A replica stream overflow likewise disconnects that subscriber. Successful join/control commands flush their committed frames between commands, so a burst of joins does not accumulate replica frames until the next timer tick. Existing protocol, input, action, content cache and download concurrency limits remain in force.

These bounds do not sandbox project scripts or establish a paid hosting platform. For managed hosting, isolate each world process, save/backup directory and credentials; enforce CPU, memory, disk and script execution limits externally. Add fleet supervision, secure account recovery, off-site backups and operational monitoring before offering a production service. Multi-process region simulation is deferred until measurements justify its complexity.

## Reproducing measurements

```sh
cargo build --release -p eldiron-server --bin eldiron-server-bench --offline
python3 tools/measure_online_host.py --ticks 200
```

The benchmark runs sequential samples on 3DStarter and Hideout2D: real authored map/rules, 1–32 admitted players, stationary or moving synthetic actors, movement inputs, projection and MessagePack output encoding. Persistent rows include real file commits of gameplay requests. The loop supplies 20 ms of simulation time per iteration and runs as fast as possible; it does not pace real sockets. Warmup/startup is excluded and reported separately. The crowd is placed in a controlled player’s starting region, with the region and final player presence recorded. Moving actor counts (including warmup) are observed, not assumed: collision can prevent motion. The harness fails if a client disconnects or the requested player population is not admitted. JSON records content fingerprint, available hardware/build details, queues, bytes, elapsed phase timings and save costs. The 20 ms p95/250 ms maximum sample criteria flag this workload's budget only. Synthetic actors are not full scripted NPC AI, and this in-process measurement excludes TLS, network latency, content downloads, graphics, client input decoding and authentication/reconnect storms. Do not sell the admission maximum as measured MMO capacity.

The original baseline is preserved in [phase 8 evidence](../concepts/networking-evidence/phase8-load.md); the prior optimization results are in the [storage/replication comparison](../concepts/networking-evidence/storage-replication-optimization.md).

The [storage/replication optimization contract](network-performance.md) documents compact pose updates, save container 2 with legacy reads, reusable prepared encodings and native head caching.

Current journal/group results and controlled snapshot comparisons are recorded in [incremental storage evidence](../concepts/networking-evidence/incremental-storage.md).

Detailed collision/script/clone/merge timings and opt-in inactive-region rules are documented in [simulation scheduling](simulation-scheduling.md).

Simulation diagnostics now include per-region collision, clone/merge, script and behavior phases plus scheduling sleep/wake/skip counters. [Measured simulation results](../concepts/networking-evidence/simulation-performance.md) include matched active-world comparisons and saved suspension/restore runs.


## Service shutdown and release verification

Unix SIGTERM follows the same durable checkpoint path as Ctrl-C. Allow the measured worst checkpoint duration before forcing termination. The serving owner closes and joins its bounded socket tasks before final graceful-exit metrics, including half-open connections; cancelled owners also cancel their sockets. See the [real-network release gate](network-release-hardening.md) for restart/lost-reply, slow-peer and world-namespace checks.

## Longer deployment drills

The [deployment testing gate](network-deployment-testing.md) runs isolated saved worlds through paced TCP/TLS, outages, supervisor-style restarts, exclusive-owner rejection, retained backups and fresh-path restoration. Results include bounded queue checks and per-generation RSS samples. These local drills complement the release gate; they do not establish production capacity, kernel packet-loss behavior or real geographic WAN coverage.
