# Simulation profiling and region scheduling

The dedicated host now supports opt-in suspension of named, unoccupied regions. Embedded single player and Creator keep all regions active. Walking, collision, script callbacks and timers retain their existing cadence in active regions. Wire protocol 4, save schema 1, journal format and IndexedDB layout are unchanged.

## Project configuration

```toml
[multiplayer.simulation]
pause_unoccupied_regions = ["Dungeon", "Side cave"]
idle_after_ms = 5000
```

The default list is empty. Names must each identify exactly one authored region; duplicates, unknown names, unknown settings and invalid types fail startup. At most 1,024 names are allowed. The idle interval defaults to 5,000 ms and accepts 20–600,000 ms. A listed region becomes eligible to suspend at a host boundary after that much accepted idle host time. This acts on whole simulation regions, independently of drawing chunks and replication cells.

Only the dedicated host applies this project table. A single-player launch and Creator Play ignore it, including Creator tests of a multiplayer project. Trusted custom hosts can select `Server::set_region_scheduling`; default CPU runtimes opt into nothing. Changing deployment configuration of an existing saved game still requires the existing explicit deployment migration/compatibility workflow.

## Sleep and wake rules

A region stays active while any connection controls a body there or is spawning there. Incoming commands, pending entity transfers/removals, queued gameplay effects and debug mode keep it awake and reset its idle grace period. Outbound snapshots/time messages do not count as activity. Addressed commands wake a region even if they arrive between scheduling passes. Joining a saved character wakes it before the next simulation/input grant. Guest creation wakes its target, and disposable demo reset reapplies the policy.

While suspended, a region consumes addressed commands but receives no autonomous movement, game ticks, behavior scheduling, animation timers, cooldowns, condition timers, local notifications or region time. Existing state stays loaded and can still be saved, exported and replicated. Wake resumes from the saved local time with the normal next step; elapsed sleeping time is discarded, with no catch-up burst. A command-only poll still cannot grant movement or timer time.

The shared world resource clock and the host/session clock continue. A world-scoped effect addressed into a region wakes it; world/region scripts that execute on that region's local ticks pause with those ticks. Suspension does not freeze all global state. Prefetching geometry or watching party state in another region does not count as controlling a body there.

Disconnected saved bodies freeze with a suspended region. Keep combat areas, economies and other regions requiring continuous background progress off the list. This is an explicit gameplay policy, not a transparent rendering optimization. It does not unload a map or reduce the memory needed for its assets/records. Sleeping regions still incur some collection/replication/checkpoint work.

Scheduling leases/counters are ephemeral host state. Save records retain the actual frozen world, region time and pending timers. A reopened host starts a fresh grace period and does not simulate offline time. Durable effects and receipts still commit together before replies or frames escape; suspension does not bypass that boundary.

## Profiling and optimizations

Private diagnostics now separate dynamic actor/item collision, geometry collision, entity cloning, entity merging, VM/event callbacks and behavior scheduling. Candidate counters show how many entity/item entries collision examined. These samples overlap their enclosing region/pump timers; callback and behavior samples can also overlap one another. Do not add nested timings as independent CPU time. Instrumentation remains opt-in and contains counts/timings, not coordinates, accounts, scripts or item payloads. Prediction/replay uses the same collision kernel with diagnostics disabled.

The measured movement improvement rejects distant actors before reading their mode/height attributes and avoids allocating mode strings in collision checks. Actor iteration, collision iterations, contact ordering, vertical overlap, item footprints and tile/mesh floor resolution are preserved. Entity side effects merge through an identity index while preserving original order and first-match behavior, replacing the quadratic ID search. Inventory/equipment/wallet and combat side effects still merge before stashed bodies are removed.

`HostSnapshot.scheduling` reports each runtime region's eligibility, suspended status, idle duration, sleep/wake counts and skipped game/movement grants. `inactive_region_policy` identifies the selected policy. Detailed timings are under `runtime.regions[].details`. Native and WASM use the same scheduling/movement code; no new renderer or native networking dependency is introduced.

See [measurements and validation](../concepts/networking-evidence/simulation-performance.md) for controlled before/after profiles, the separate suspension comparison and remaining limits. Busy occupied regions still exceed the 20 ms budget at large populations. Scripts are neither preempted nor automatically throttled by this phase.

When a scheduled native world has at most one awake region, dispatch visits regions sequentially to avoid worker wake/coordination overhead for sleepers. With multiple awake regions, normal parallel dispatch remains. No region is omitted from command polling or scheduling counters. WASM continues to use its existing sequential execution.

[Measurement and validation evidence](../concepts/networking-evidence/simulation-performance.md) records active-world, unoccupied-region and saved restore runs, with their remaining limits.
