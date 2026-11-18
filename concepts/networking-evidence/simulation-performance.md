# Simulation performance evidence — 2026-11-17

This phase measures movement/collision and adds opt-in suspension of named unoccupied regions. All samples use optimized native hosts with real authored maps, rules, players, projection and output encoding. Crowd actors are synthetic movers, not full scripted NPC AI.

## Movement comparison

The [instrumented baseline](simulation-baseline.json) and [optimized sample](simulation-optimized.json) each contain four sequential runs. Matched rows have identical project content fingerprints, populations, movement inputs, warmup and 200 measured 20-ms iterations. Both include the same detailed profiling. No player disconnected. Each report identifies its binary SHA-256 and exact commands.

| Project | Players / moving actors | Baseline p95 | Optimized p95 | Reduction |
|---|---:|---:|---:|---:|
| 3DStarter.eldiron | 8 / 300 | 13.78 ms | 7.08 ms | 48.6% |
| 3DStarter.eldiron | 32 / 1000 | 97.97 ms | 51.79 ms | 47.1% |
| Hideout2D.eldiron | 8 / 300 | 8.87 ms | 8.02 ms | 9.5% |
| Hideout2D.eldiron | 32 / 1000 | 66.36 ms | 56.83 ms | 14.4% |

The busiest 3D sample spent 10.49 seconds in dynamic collision checks across the measured run; that fell to 1.86 seconds. Collision now rejects distant actors before reading mode/height attributes and avoids allocating mode strings. Entity side-effect merging uses one ID index, preserving original order and first-match behavior. Contact order, vertical overlap, collision iterations, item footprints, scripted callbacks and active-region time budgets remain unchanged.

These are one-pass elapsed-time samples with normal machine/filesystem variance. The largest active worlds still exceed 20 ms p95; the optimized 32-player maxima are approximately 153 ms (3DStarter) and 322 ms (Hideout2D). Full spatial collision indexing, further projection/capture work, scripted-AI budgets and checkpoint offloading remain separate work. Do not infer production hosting capacity from these fixtures.

## Unoccupied-region scheduling

[Scheduling comparison](simulation-scheduling.json): four players and 100 movers in the occupied region, plus a deterministic extra unoccupied region with 1,000 movers. Each pair uses one binary and identical content; only a trusted host policy switch differs. The configured grace is 100 ms and warmup is 400 ms, so the eligible region is asleep before measurement. The harness asserts its positions and local time stay frozen.

| Project | All regions ticking p95 | Opt-in suspension p95 |
|---|---:|---:|
| 3DStarter.eldiron | 18.72 ms | 3.72 ms |
| Hideout2D.eldiron | 19.98 ms | 4.89 ms |

This comparison predates the final dispatch refinement below. It retains its original binary identity. Suspension reduces simulation work but keeps state loaded, saved and available for replication. It is a project gameplay choice: local NPC behavior, cooldowns and timers pause. Shared world resource time continues. See the [configuration and clock contract](../../docs/simulation-scheduling.md).

## Saved soak, dispatch refinement and restoration

The [earlier soak](simulation-soak-before-dispatch.json) exposed expensive parallel dispatch with only one awake region. The final implementation visits regions sequentially when an opt-in scheduled world has at most one awake region, including sleepers for commands and counters. Unconfigured native worlds and worlds with multiple awake regions retain parallel execution. WASM remains sequential.

[Final saved runs](simulation-soak.json) repeat the same commands/content as the earlier soak: four players, 100 occupied-region movers, 1,000 unoccupied-region movers, 2,000 measured iterations (40 simulated seconds), 16 sequenced actions, periodic checkpoints and graceful shutdown. No competing build/test jobs were running during either measurement. They are accelerated in-process runs, not 40 seconds of wall-clock network traffic.

| Project | Before dispatch refinement p95 | Final p95 | Final maximum | Checkpoints / action groups |
|---|---:|---:|---:|---:|
| 3DStarter.eldiron | 38.09 ms | 3.08 ms | 50.64 ms | 8 / 4 |
| Hideout2D.eldiron | 43.44 ms | 4.91 ms | 131.50 ms | 8 / 4 |

Both final runs retained all four players and had zero save failures. A fresh host reopens the committed save after shutdown. The harness compares the complete saved metadata/image, then separately validates the live restored world and matches actors, inventories, items, wallets, local region clocks, world values and the ID allocator. Transient dialogue dismissal and hash-map iteration order are intentionally excluded from the live-engine comparison. The sleeping region remains unchanged throughout measurement. This complements injected-failure tests; it does not replace process-kill, filesystem-fault or long real-socket stress testing.

## Verification

- Engine: scheduling sleep/wake/transfer/effect rules, dense ordered contacts/dead/vertical actors, inventory side-effect merge, three fixed-movement/clock tests and two tile/mesh kernel tests pass.
- Runtime: movement, nine prediction/reconciliation tests and two new scheduling/persistence tests pass. Single-player and Creator tests explicitly retain active regions even when the project contains dedicated scheduling settings. Journaled restore retains frozen state without duplicating bodies.
- Dedicated: strict policy validation, operations/account tests, spatial/private replication and guest wake/demo reset pass. Real WSS/HTTPS and two actual Chrome WASM clients pass; native Player/Creator and browser Player/Creator checks and CPU/renderer boundaries pass.

## Reproduce

```sh
cargo build --release -p eldiron-server --bins --offline
python3 tools/measure_simulation.py --label optimized --output /tmp/simulation.json
python3 tools/measure_simulation.py --label scheduling --scheduling-pairs \
  --output /tmp/simulation-scheduling.json
target/release/eldiron-server-bench --project test_projects/3DStarter.eldiron \
  --players 4 --entities 100 --moving-entities --idle-entities 1000 --pause-idle \
  --persistent --saved-actions 4 --ticks 2000 --verify-restart
```

Use the recorded baseline binary/source to reproduce historical baseline timing. The current binary implements the optimized paths; rerunning it is a fresh sample, not a recreation of the older implementation. Run without competing compilers/tests and inspect content/build/population fields before comparing results.
