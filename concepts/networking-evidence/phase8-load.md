# Phase 8: measured host workloads

Measured 2026-11-17T04:37:30.421023+00:00 on macOS-27.0.1-arm64-arm-64bit, 10 available logical CPUs. The chip model was unavailable in the sandbox and RAM was not recorded; hardware fallback is `arm64`. This limits hardware-to-hardware comparisons.

These short, fixture-specific optimized samples establish a workload boundary for this machine. Eight players and 300 synthetic actors asked to walk met the p95 criterion in both fixtures; sixteen players and 500 actors missed it. This does not establish a general supported player count. Real authored AI, longer runs, networking and deployment resource limits need separate measurements.

## Method

- Build: `cargo build --release -p eldiron-server --bins --offline`. Run: `python3 tools/measure_online_host.py --ticks 200`.
- One sequential sample per configuration, after other compilation/heavy tests finished; 20 warmup steps and 200 measured steps. Four seconds of supplied simulation time per measured sample, executed as fast as possible. No statistical confidence interval or sustained-load claim.
- Authored maps/rules remain active, including regions without players. Each player sends real typed movement commands; the server performs real interest projection and MessagePack output encoding. Client output decoding occurs after the timed server slice.
- Extra synthetic actors are placed around a controlled player in StartScene (3DStarter) or Harbor (Hideout2D). They block/collide and optionally use the engine Forward action, without full scripted AI. The moved count includes warmup and is observed: many 3D actors are blocked by the room/crowd.
- Persistent samples use four accounts/characters, the real file store, and two sequenced `look` requests per player (eight immediately durable request commits). They exercise snapshot/receipt durability, not a complex economic workload.
- Server slice includes input/request handling and one host tick. TLS, real packet scheduling, HTTPS downloads, input wire decoding, authentication/reconnect storms, rendering and OS memory accounting are excluded.
- Sample criterion: p95 at most 20 ms, maximum at most 250 ms, and no client loss. This permits individual steps above 20 ms and is not a guarantee of uninterrupted 50 Hz play. Every final sample retained all requested players in the crowd region; no slow-client/replica disconnects or save failures occurred. The harness now fails if a requested client disconnects.

The run exposed a join-burst backlog: control commands could publish repeatedly before the next timer tick. The fix flushes committed control-command frames between joins. A separate twenty-player test with no intervening timer ticks verifies retention; an independent release-load check also found randomly generated IDs on legacy faces. Missing IDs now normalize deterministically per object/face ordinal while authored IDs and paint links are preserved. The final samples below were regenerated after both fixes, with one stable content identity per fixture.

## 3DStarter.eldiron

Content fingerprint: `00ba8514c5c63b6a89a3e071a017cc13ae0b6e12fd4073c0f0ffdea9c382dc7e`. Loaded regions: 1. Live authored entities before synthetic additions: 0 (excluding the one player).

| Players | Extra actors | Workload | Observed moved | Slice p95 ms | p99 ms | Max ms | Criterion |
| ---: | ---: | --- | ---: | ---: | ---: | ---: | --- |
| 1 | 0 | stationary crowd | 0 | 0.20 | 0.21 | 0.28 | Met |
| 4 | 100 | stationary crowd | 0 | 2.06 | 2.15 | 2.25 | Met |
| 4 | 100 | walk | 56 | 3.23 | 3.39 | 3.48 | Met |
| 8 | 300 | walk | 89 | 14.70 | 15.62 | 16.06 | Met |
| 16 | 500 | walk | 89 | 43.66 | 45.31 | 46.12 | Missed |
| 32 | 1000 | walk | 93 | 167.61 | 170.30 | 170.50 | Missed |
| 4 | 100 | walk + file commits | 56 | 2.92 | 3.02 | 96.18 | Met |

## Hideout2D.eldiron

Content fingerprint: `e964a28a3e65d3660bb083bda26e5392aa79301735b1d383700bca0afde4eab0`. Loaded regions: 2. Live authored entities before synthetic additions: 8 (excluding the one player).

| Players | Extra actors | Workload | Observed moved | Slice p95 ms | p99 ms | Max ms | Criterion |
| ---: | ---: | --- | ---: | ---: | ---: | ---: | --- |
| 1 | 0 | stationary crowd | 0 | 1.60 | 2.25 | 2.52 | Met |
| 4 | 100 | stationary crowd | 0 | 4.46 | 4.74 | 6.26 | Met |
| 4 | 100 | walk | 100 | 4.90 | 17.22 | 18.64 | Met |
| 8 | 300 | walk | 300 | 16.18 | 61.26 | 64.31 | Met |
| 16 | 500 | walk | 500 | 43.71 | 135.04 | 138.78 | Missed |
| 32 | 1000 | walk | 1000 | 162.20 | 374.88 | 399.77 | Missed |
| 4 | 100 | walk + file commits | 100 | 4.41 | 18.77 | 685.21 | Missed |

## Where the time and bandwidth went

Mean elapsed milliseconds per measured step for demo samples. These columns overlap with higher-level host measurements; they are not OS CPU accounting. Payload rate is total encoded server application bytes divided by supplied simulation seconds, at a hypothetical 50 Hz delivery cadence; it is not measured TLS throughput.

| Fixture | Players / extra actors | Simulation ms | Collection ms | Projection ms | Encoding ms | Aggregate payload MiB/s |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 3DStarter.eldiron | 4 / 100 | 1.23 | 0.16 | 1.45 | 0.09 | 3.90 |
| 3DStarter.eldiron | 8 / 300 | 6.29 | 0.42 | 6.57 | 0.36 | 14.65 |
| 3DStarter.eldiron | 16 / 500 | 15.11 | 0.86 | 23.45 | 1.09 | 37.65 |
| 3DStarter.eldiron | 32 / 1000 | 53.85 | 1.82 | 97.35 | 5.06 | 147.33 |
| Hideout2D.eldiron | 4 / 100 | 1.98 | 0.21 | 2.30 | 0.06 | 2.03 |
| Hideout2D.eldiron | 8 / 300 | 5.43 | 0.39 | 8.21 | 0.24 | 8.79 |
| Hideout2D.eldiron | 16 / 500 | 12.15 | 0.57 | 28.47 | 1.16 | 36.63 |
| Hideout2D.eldiron | 32 / 1000 | 37.80 | 1.05 | 106.83 | 5.38 | 148.48 |

Simulation and recipient projection both matter in the larger workloads. Adding region processes alone would not remove recipient projection/egress costs. The payload rates also require network testing and budgeting before selecting a hosting tier.

## File-save latency

| Fixture | Capture mean / max ms | Commit mean / max ms | Commit calls | Max complete server slice ms |
| --- | ---: | ---: | ---: | ---: |
| 3DStarter.eldiron | 1.22 / 1.24 | 14.71 / 15.22 | 8 | 96.18 |
| Hideout2D.eldiron | 3.03 / 3.95 | 86.91 / 88.17 | 8 | 685.21 |

Each action synchronously commits a complete coherent image before acknowledgment. A step containing several file commits can stall even when normal-step p95 looks healthy; Hideout2D exceeded the 250 ms criterion. The file backend is a recovery foundation, not demonstrated production transaction throughput. A future storage design must preserve atomic effects/receipts while addressing these stalls. Simply returning an acknowledgment before a durable commit would weaken the contract.

## Reproduction and validation

All raw phase/region timings, queue/traffic counters, save timings, content identities, setup time and commands are preserved in [phase8-load.json](phase8-load.json). Repeat on the intended hardware with realistic scripts, player distribution, actions and real network traffic before setting an operational admission limit. The configurable 64-player ceiling remains a structural admission bound, not a benchmarked capacity claim.

Phase validation: 56 runtime/network tests; five dedicated unit tests, four host integrations, independent release-load identity integration, spatial integration and real native WSS/HTTPS integration; eleven geometry serialization/paint/identity tests; bounded timing test; native/WASM Player and Creator checks; renderer boundary checks; two actual isolated browser/WASM clients over WSS/HTTPS. A focused pose-snap test also verifies that correction-distance diagnostics include large snaps. Backups/restores retain account metadata, live body/inventory identity and durable creation/action receipts.

See [the operations runbook](../../docs/server-operations.md) for recovery, admission, backup retention and overload policies.
