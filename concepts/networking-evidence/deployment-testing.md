# Longer local network and deployment evidence

Recorded 2026-11-17 on macOS / arm64. The user selected local WAN emulation; no external infrastructure was provisioned or contacted. [Procedure and reproducible commands](../../docs/network-deployment-testing.md).

The final gates passed against dirty workspace builds identified by source/project/binary fingerprints in the JSON reports. These are targeted local socket, process and compilation checks, not a full-workspace test-suite or production-capacity claim.

## Final saved-world drill

[Ten-minute report](deployment-soak.json), [compressed recovery preflight](deployment-preflight.json), [original release-gate regression](deployment-release-regression.json).

| Observation | Result |
| --- | --- |
| Sustained interval | 600.370 seconds, after fixture startup |
| Worlds / characters | Two isolated saved worlds; two main characters and one independent sentinel |
| Process generations | Initial main world, graceful restart, crash/reopen, backup restore; sentinel remains running |
| New acknowledged requests | 329 across the worlds, plus exact cached retries |
| Main-world starter identities | Eight item IDs, each owned exactly once throughout, including restore |
| Ping round trips | p50 182.1 ms, p95 383.3 ms, maximum 390.2 ms |
| Operational samples | 56 snapshots, including per-generation RSS and backup/queue state |
| Memory | Warm RSS approximately 374–426 MiB across generations; no growth in the sampled values within each generation |
| Shutdown | Zero active transports and relay connections |

The run changed from the ordinary propagation/bandwidth profile to the distant profile after 60 seconds, held one connection for two seconds after 120 seconds and for 35 seconds after 210 seconds, then reauthenticated it. The long-outage recovery completed at 247.4 seconds. Graceful restart completed at 343.5 seconds, crash/reopen at 463.2 seconds and fresh-path backup restoration at 554.7 seconds. The second world continued acknowledging requests and advancing replicas throughout.

Creation retries resolved to the original character/body. Inventory/equipment transfers and exact action retries preserved ownership and durable watermarks. Another process was refused access to the actively owned save. Thirty-second periodic backups retained at most three verified snapshots. Restore stopped the old owner and used `eldiron-world` to create a fresh working slot; the selected backup's older receipts/state were authoritative. Queue, content-cache, sidecar and artifact-retention assertions remained within their bounds. Running-host metrics stayed healthy, and normal child process exits/checkpoints and zero transports verified clean shutdown.

RSS is process-resident memory from `ps`, not a proposed hosting allocation. The per-generation guard permits up to 256 MiB additional growth; passing it does not establish leak freedom or account for every transient peak. The workload is a small starter world with low economic-action volume. Native peers check recipient privacy and frame progress; they do not run a graphical window or client prediction.

## Final browser movement and outage gate

[Walking/reconciliation report](deployment-browser-walking.json). [Non-walking comparison](deployment-browser.json) records the earlier five-minute input/request baseline; its equivalent current configuration is `ELDIRON_WALK=0`.

| Observation | Result |
| --- | --- |
| Sustained interval | 303.104 seconds |
| Client implementation | Two real WASM/Chrome tabs over WSS and HTTPS |
| Health checks | 90 checks requiring advancing authoritative ticks |
| Walking probes | 12 forward/backward probes; observed movement and zero measured settled presentation/authority error |
| Action acknowledgment round trips | p50 425 ms, p95 513 ms, maximum 602 ms, including look/clear-intent requests |
| Prediction history | Maximum 26 pending frames at health samples; bound 128 |
| WASM linear memory | 23,199,744 bytes (about 22.1 MiB) per tab throughout health samples |
| Long outage | 35 seconds; Bob retained its body/progress, Alice stopped moving while visible, explicit reconnect restored one guest body |
| Teardown | Both relays had zero active connections |

Profiles changed at 32.2 seconds; the two-second blackout began at 63.1 seconds and the long-outage/reconnect phase at 157.6 seconds. The run also retained the three-second required-HTTPS delay/cancellation, bounded ordered content delivery, private ownership, initial reload and high-refresh input-send checks.

The movement probe explicitly clears selected targeting through the authoritative request path before walking. An initial test left `look` selected; D2 directional movement intentionally remains targeting in that state, so advancing input sequences alone had not proved movement. The corrected probe requires actual displacement and convergence after stopping. The starter reports D2 controls with mesh collision; this gate does not establish every graphical camera or complicated dynamic-collision case. It adds no client-selected actor/tenant or administrator/debug RPC.

An interruption regression sent SIGTERM to the final browser harness after it began running. Both owned direct child processes (Chrome and the temporary host) exited, and its temporary profile/certificate directory was removed. The native deployment runner separately owns its complete disposable process group for interruption/timeout cleanup.

## Harness validation and compatibility

Both bounded relays pipeline propagation delay independently from bandwidth, preserving order and end-to-end TLS verification. Their ordinary profile is 45–90 ms one-way / 192 KiB/s per direction; the distant profile is 120–200 ms / 96 KiB/s. Queues contain at most eight 8-KiB reads, with current read/write and kernel socket buffers additional. These are TCP pacing, holds and connection cuts, not geographic WAN, random IP loss or message-reordering simulation.

The first relay implementation serially slept on each read, unintentionally reducing effective bandwidth; it was corrected before the final runs. A separate JavaScript relay fault discarded queued HTTP response bytes on upstream close; graceful response draining now has a regression. Three relay tests passed for byte integrity/cancellation, complete response draining and pipelined propagation/throughput. A duplicate-owner assertion was corrected to recognize the existing `Busy` refusal, and stopped-host checks distinguish clean shutdown from running-host health.

Additional final validation passed:

- Existing optimized release gate: committed-but-unobserved replies, competing ownership, crash/journal replay, world credential isolation, slow peers, TLS/origin/protocol rejection and transport lifetime.
- Sixteen networking library tests.
- Three native graphical account/demo/socket tests, including the sixty-second delayed-content reconnect soak.
- Two embedded project-mode and two scheduling/restoration tests, including Creator's local override and single-player isolation.
- Native Player/Creator/online-client compilation and WASM Player/Creator compilation.
- Renderer dependency boundaries, changed Rust formatting, JavaScript/Python syntax and tracked diff whitespace checks.

Single player and Creator keep their embedded paths. The production change in this pass adds client-local replica/motion/authoritative-pose diagnostics; wire, replication and save formats remain unchanged. The remaining implementation is opt-in test tooling and operational documentation.

## Limits before deployment claims

These runs do not cover multi-hour service lifetime, actual geographic WAN or IP loss, mobile/background-tab behavior, every browser/OS, all graphical cameras, large scripted combat/AI workloads, storage exhaustion/failover, public visual-asset publishing, certificate renewal or fleet administration. Fixed workload memory samples are not a substitute for tenant CPU/memory/script quotas. Demo transient VM state is reclaimed when its world becomes empty; sustained visitor churn with an always-present player still needs lifetime/quota validation.

See the [hosting concept](../eldiron-managed-hosting-concept.md) for the remaining pilot and control-plane requirements. This evidence supports a controlled next deployment pilot; it does not establish paid-hosting capacity or declare networking finished for arbitrary projects.
