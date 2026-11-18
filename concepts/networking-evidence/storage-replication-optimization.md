# Storage and replication optimization measurements

Measured 2026-11-17T05:40:29.417037+00:00 on macOS-27.0.1-arm64-arm-64bit, 10 available logical CPUs. The chip model is unavailable in this sandbox; RAM was not measured.

Compared with the preserved [phase-8 baseline](phase8-load.md), the same authored content fingerprints and workload parameters were used. All requested players stayed in the crowd region; no save failures, replica overflows or slow-client disconnects occurred. These are single short samples, not sustained capacity or production service guarantees.

The largest Hideout2D demo sample reduced mean recipient projection from about 107 ms to 18 ms per step and reduced encoded payload bytes by about 71%. The Hideout2D file-commit burst maximum decreased from about 685 ms to 198 ms. Storage still stalls the single owner during synchronous full-image commits; the optimization reduces that cost without acknowledging effects before durability.

## Method and compatibility

- Optimized build: `cargo build --release -p eldiron-server --bins --offline`.
- Run: `python3 tools/measure_online_host.py --ticks 200`. Its default output now preserves the phase-8 baseline and writes `storage-replication-optimization.json`.
- Fourteen configurations: 1–32 real player sessions, 0–1000 extra synthetic stationary/walking actors, and four-player persistent samples with two sequenced `look` requests per player (eight real durable commits). Authored rules and all loaded regions remain active; synthetic crowd actors are not full scripted AI.
- Samples run sequentially after compilation/heavy tests, with 20 warmup steps and 200 measured steps, each supplying 20 ms of simulation time. The loop runs as fast as possible. Client output decoding follows the timed server slice; startup/warmup is excluded and retained separately in JSON.
- The complete server slice includes commands, serialization, store work and a host tick. TLS, real network scheduling, HTTPS downloads, input wire decoding, graphics, reconnect/authentication storms and memory accounting are excluded. No confidence interval or cross-hardware capacity claim is made.
- The saved container writes version 2 while world schema stays 1. Old container-1 snapshots remain readable. Gameplay uses WSS protocol 4 (`eldiron.v4`), replication DTO version 3; rebuild clients/server together.
- Each final fixture retains one stable content fingerprint matching every baseline row. Full row data includes build mode, exact benchmark binary SHA-256, source content identity, observed moved-actor counts, setup time, counters and commands. Blocked actors, especially in the 3D room, are counted as observed rather than assumed to move.

Benchmark binary SHA-256: `8c70c23da8b06f69e43d562da2e55fcb49b943a6699aff37059af1218024deb9`.

## Complete server slices

Milliseconds. The sample criterion is p95 ≤20 ms and maximum ≤250 ms, with no client loss. A passing criterion still permits occasional steps over 20 ms; it is not uninterrupted 50 Hz service.

| Fixture | Players / extra actors | Workload | Old p95 | New p95 | Old max | New max | New criterion |
| --- | ---: | --- | ---: | ---: | ---: | ---: | --- |
| 3DStarter.eldiron | 1 / 0 | stationary crowd | 0.20 | 0.24 | 0.28 | 0.34 | Met |
| 3DStarter.eldiron | 4 / 100 | stationary crowd | 2.06 | 1.43 | 2.25 | 1.82 | Met |
| 3DStarter.eldiron | 4 / 100 | walk | 3.23 | 2.91 | 3.48 | 9.85 | Met |
| 3DStarter.eldiron | 8 / 300 | walk | 14.70 | 10.55 | 16.06 | 11.35 | Met |
| 3DStarter.eldiron | 16 / 500 | walk | 43.66 | 26.69 | 46.12 | 48.66 | Missed |
| 3DStarter.eldiron | 32 / 1000 | walk | 167.61 | 91.74 | 170.50 | 132.56 | Missed |
| 3DStarter.eldiron | 4 / 100 | walk + file commits | 2.92 | 2.97 | 96.18 | 61.71 | Met |
| Hideout2D.eldiron | 1 / 0 | stationary crowd | 1.60 | 1.48 | 2.52 | 2.05 | Met |
| Hideout2D.eldiron | 4 / 100 | stationary crowd | 4.46 | 3.31 | 6.26 | 5.38 | Met |
| Hideout2D.eldiron | 4 / 100 | walk | 4.90 | 3.52 | 18.64 | 18.48 | Met |
| Hideout2D.eldiron | 8 / 300 | walk | 16.18 | 9.10 | 64.31 | 61.32 | Met |
| Hideout2D.eldiron | 16 / 500 | walk | 43.71 | 19.44 | 138.78 | 131.44 | Met |
| Hideout2D.eldiron | 32 / 1000 | walk | 162.20 | 62.26 | 399.77 | 338.54 | Missed |
| Hideout2D.eldiron | 4 / 100 | walk + file commits | 4.41 | 3.40 | 685.21 | 197.72 | Met |

## Projection and encoded payload

Demo walking samples only. Projection is mean elapsed milliseconds per measured step, including recipient mirrors. Payload is all encoded server application bytes divided by supplied simulation seconds, at a hypothetical 50 Hz cadence. It excludes TLS/framing and is not measured network throughput.

| Fixture | Players / actors | Old projection ms | New projection ms | Old MiB/s | New MiB/s | Payload reduction |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 3DStarter.eldiron | 4 / 100 | 1.45 | 0.66 | 3.90 | 1.34 | 65.7% |
| 3DStarter.eldiron | 8 / 300 | 6.57 | 2.14 | 14.65 | 6.13 | 58.1% |
| 3DStarter.eldiron | 16 / 500 | 23.45 | 5.99 | 37.65 | 17.26 | 54.1% |
| 3DStarter.eldiron | 32 / 1000 | 97.35 | 18.22 | 147.33 | 43.42 | 70.5% |
| Hideout2D.eldiron | 4 / 100 | 2.30 | 0.95 | 2.03 | 0.57 | 72.1% |
| Hideout2D.eldiron | 8 / 300 | 8.21 | 2.48 | 8.79 | 2.40 | 72.7% |
| Hideout2D.eldiron | 16 / 500 | 28.47 | 5.90 | 36.63 | 10.50 | 71.3% |
| Hideout2D.eldiron | 32 / 1000 | 106.83 | 18.38 | 148.48 | 43.03 | 71.0% |

The shared public projection cache and compact poses reduce repeated attribute copying/materialization and byte volume. This does not remove busy-region simulation cost. In particular the 32-player/1000-actor configurations still miss the sample budget, and even passing Hideout samples have longer game-tick outliers. Admission limits remain workload-specific; 64 is a structural ceiling, not an advertised tier.

## Durable save costs

Current mean/maximum milliseconds per committed request. `encoding` includes serialization plus checked typed decode. Earlier commit counters contained additional serialization and full-head decoding, while some old live-operation work was outside those counters. Compare the complete server slices above for overall savings; do not interpret the new commit column alone as a disk-speed improvement. Capture, encoding and commit do not include every gameplay callback or replication operation.

| Fixture | Capture mean / max | Encoding mean / max | Commit mean / max | Committed requests |
| --- | ---: | ---: | ---: | ---: |
| 3DStarter.eldiron | 0.75 / 0.95 | 1.94 / 1.99 | 9.81 / 10.66 | 8 |
| Hideout2D.eldiron | 1.98 / 2.28 | 27.60 / 28.48 | 12.52 / 13.16 | 8 |

A burst of four economic/request commits is still serialized on one world owner. Full-image encoding, filesystem sync latency and large geometry/state remain costs. Group commit, incremental durable records or a transactional managed backend are future options; each must commit all effects/receipts before acknowledging them and retain bounded per-world queues. No background acknowledgment, speculative economic mutation, lost-receipt window or external database service was introduced.

## Correctness and supported paths

- Runtime/network baseline suite: 56 tests passed. After the final container/index changes, all nine persistence and nine replication tests passed, including legacy upgrade with retained creation/action receipts, non-finite data rejection, cached-head external replacement/corruption, pose validation atomicity, index compaction, and private party/container/trade grant/revocation.
- Dedicated server unit/host/spatial/recovery tests passed, covering durable restoration, account ownership, no starter/item duplication, scheduled backups/retention, overload isolation and join bursts. Real native WSS/HTTPS passed.
- Native graphical Player account/demo socket tests passed (two tests); native/WASM Player and Creator checks passed. Two actual browser/WASM clients passed WSS/HTTPS preloads, movement and sequenced requests; actual IndexedDB passed atomic compare-and-swap and isolated save slots.
- Rustdoc and renderer dependency/source boundary checks passed. Apple device execution and sustained real-network load were not tested.

Raw data: [optimized samples](storage-replication-optimization.json), [baseline samples](phase8-load.json). Implementation and compatibility: [network performance contract](../../docs/network-performance.md). Operations: [server runbook](../../docs/server-operations.md).
