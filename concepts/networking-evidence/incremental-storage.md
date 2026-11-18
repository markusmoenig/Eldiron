# Incremental durable storage evidence — 2026-11-17

Dedicated saves now use an incremental journal and bounded action groups. [Raw scale samples](incremental-storage.json) and the [controlled storage comparison](incremental-storage-comparison.json) retain commands, binary SHA-256, content fingerprints, host/runtime/save timings and queue/population checks.

## Controlled comparison

All six rows use the same optimized binary, the same content fingerprint within each map, four admitted players, 100 moving synthetic actors, 200 iterations supplying 20 ms each, and two saved actions per player. Startup and 20 warmup ticks are excluded. Actions land on two measured iterations. The reference uses the unchanged standalone snapshot adapter and immediate per-action calls. The second uses journaled individual calls. The third supplies the same eight actions as two groups of four. No test row disconnected a player.

| Map | Store/action path | File commits | Longest complete slice | Checked encoding, total |
|---|---|---:|---:|---:|
| 3DStarter | Snapshot per action | 8 | 61.27 ms | 16.44 ms |
| 3DStarter | Journal per action | 8 | 78.12 ms | 8.49 ms |
| 3DStarter | Journal groups | 2 | 26.98 ms | 2.44 ms |
| Hideout2D | Snapshot per action | 8 | 197.58 ms | 223.59 ms |
| Hideout2D | Journal per action | 8 | 132.10 ms | 82.95 ms |
| Hideout2D | Journal groups | 2 | 45.62 ms | 20.60 ms |

Groups reduce the longest measured slices by approximately 56% and 77% respectively. The 3D map's individual journal commits are slower than snapshot commits despite lower encoding cost: syncing the log and separately publishing its committed head adds I/O. This backend is not a universal latency win for a small world with isolated requests. Hideout's larger geometry benefits even without grouping.

Grouping here is a controlled burst, not an assumption that all live requests coalesce. The transport waits at most two milliseconds and respects control/input barriers; sparse or interleaved traffic may produce groups of one. The benchmark excludes that collection wait, real TLS/sockets, HTTP content, client decoding/rendering, reconnect storms and full scripted NPC AI. The counts are a single sequential pass on this machine/filesystem, not a statistical capacity guarantee.

`diagnostics.saves.commit.calls` is the file-commit count for every adapter. Journal diagnostics additionally report measured append/checkpoint bytes and records. Snapshot adapter storage counters are unavailable/default zero, so its `storage_measured.bytes_written=0` is not a zero-byte write and must not be compared to journal byte totals. Groups/actions count only the explicitly grouped API, not individual calls.

## Scale samples and limits

The 14-row scale pass retains 1–32 players across both maps with stationary/moving crowds and saved-action rows. Busy 16/32-player samples still exceed the 20 ms p95 criterion. At 32 players/1,000 moving actors, p95 is approximately 102 ms in 3DStarter and 63 ms in Hideout2D; the latter also has a 347 ms maximum. These are simulation/replication workloads, not storage-only stalls. Region sleeping/workers, script budgets and checkpoint offloading are not implemented by this change.

Project/rules content changed alongside this phase, so these rows have different content fingerprints from the earlier phase-8/optimization evidence. Compare their coverage and limits, not their timing deltas as a controlled storage result. The six-row comparison above isolates storage paths within current content and one binary.

## Reproduce

```sh
cargo build --release -p eldiron-server --bins --offline
python3 tools/measure_online_host.py --ticks 200 --output /tmp/eldiron-scale.json
python3 tools/measure_online_host.py --ticks 200 --storage-pairs-only \
  --output /tmp/eldiron-storage-comparison.json
```

Run without competing builds/tests. The latter command runs snapshot-per-action, journal-per-action and grouped-journal rows for each map. The raw JSON carries the exact commands and build identity. The first scale pass and subsequent controlled comparison use distinct binaries; they are separate experiments.

## Correctness and compatibility verification

- Eight runtime storage unit tests cover complete record replay, inventory/stash moves and metadata, geometry edits without revision changes, duplicate keys/non-finite data, ambiguous append/checkpoint publication, committed-prefix corruption/truncation, ignored unpublished tails, compaction/replay caps, orphan quotas, legacy upgrade, portable export and default-adapter journal detection.
- Ten persistence integration tests pass, including grouped grants/receipts before and after injected storage ambiguity, no duplicate startup items, local restoration and existing snapshot adapters.
- Dedicated tests: six library, five host and one release-identity test pass. The group host test verifies one revision for several actions and no frames or even cached replies released on commit failure. Collector tests verify group limits and control order. The real WSS/private-replica/HTTPS integration test passes with loopback access.
- Native Player/Creator and browser Player/Creator WASM checks pass, as do renderer/CPU boundary checks. Two actual Chrome WASM clients share the demo world, load HTTPS physics/preloads and execute sequenced actions. A real IndexedDB test verifies one CAS winner, rollback, isolated slots and reopen.

The [operations contract](../../docs/server-operations.md) covers the format-3 head plus private parts directory, portable snapshots, fail-closed recovery and bounded output. Single-player/Creator defaults and IndexedDB schema remain unchanged.
