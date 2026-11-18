# Longer network and deployment testing

These gates exercise real WSS/TLS sockets and real dedicated processes with **local WAN emulation**. They do not contact a public deployment or claim to reproduce kernel packet loss, radio behavior or geographic WAN routing. Single player and Creator retain their embedded execution; WASM uses browser WebSocket/fetch APIs. Wire, replica and save formats stay unchanged.

## Saved-world deployment drill

```sh
python3 tools/verify_network_deployment.py --seconds 600 \
  --output concepts/networking-evidence/deployment-soak.json
```

Requires Unix signals, `ps`, OpenSSL, cached Rust dependencies and permission to bind loopback ports. The runner builds optimized test/host binaries, records build/project/source fingerprints, writes a mode-0600 companion log and emits progress. Duration is configurable from 180 seconds to 24 hours. Build/startup/shutdown have additional bounded time. Interrupting the runner terminates its owned process group so a temporary world owner cannot remain running accidentally.

The main fixture has two saved characters and a separate saved world has a third, continuously active sentinel. Each world has isolated credentials, state, backups, metrics and process lifetime. The drill checks:

- Repeated authoritative requests, owned inventory/equipment transfers and exact retries. Durable watermarks and unique starter-item identities/ownership are checked throughout.
- Increasing propagation delay/jitter and bandwidth limits, a two-second two-way blackout and a 35-second outage. Affected clients explicitly reconnect; healthy peers keep receiving frames and acknowledged requests.
- Exclusive working-save ownership: a second process must refuse an already-owned state slot.
- SIGTERM checkpoint/restart, SIGKILL/reopen and replay of the exact previously committed action DTO. Original creation requests must resolve to the same character/body without allocating starters again.
- Periodic 30-second backups with retention three, verified snapshots and restore through `eldiron-world` to a fresh working path after stopping its previous owner. State/receipts from the selected backup are authoritative; operations newer than that snapshot are intentionally rolled back.
- The independent world's progress across main-world restarts/restoration. A restart retargets only that world's test relay endpoint.
- Healthy private metrics, bounded connection/command/output queues, content-cache bounds, journal sidecar count and backup retention. Clean shutdown must leave zero active transports and relay connections. The stopped durable owner changes its runtime incarnation, so final metrics can report `healthy=false` after a successful shutdown; the test uses process exit/checkpoint success and zero active connections at that point, rather than applying the running-host health assertion.

RSS is sampled per server generation using `ps`. A generous 256 MiB growth guard catches large regressions within a generation; samples and peaks are retained. This is not a leak-freedom proof, operating-system memory quota or hosted plan size. The fixture is a small starter project with low request volume and does not measure scripted-MMO capacity. Native test peers validate recipient privacy and frame progress; they do not render a graphical window or run client movement prediction. The browser gate below runs the actual WASM prediction/client path.

## WASM browser WAN-emulation gate

Build the WASM network module and generate matching wasm-bindgen web output in `/tmp/eldiron-online-web/pkg`, as for [content delivery](client-content-delivery.md), then run:

```sh
ELDIRON_SERVER=target/release/eldiron-server \
  ELDIRON_WAN=1 ELDIRON_SOAK_SECONDS=300 \
  node tools/verify_online_browser.mjs
```

SIGINT/SIGTERM rejects pending browser work and stops/reaps its owned host and Chrome processes, closes relays and removes its temporary profile/certificate directory. The opt-in cancellation regression is `ELDIRON_SERVER=target/release/eldiron-server node tools/verify_online_browser_cancel.mjs`.

The default non-WAN browser check remains available without `ELDIRON_WAN`. The soak duration accepts 0–86,400 seconds. The WASM state diagnostics include the controlled actor's authoritative position for client-local reconciliation checks; no administrator endpoint or client-trusted host telemetry is introduced. The WAN gate runs two Chrome tabs through separate opaque relays to one disposable demo world. Required HTTPS downloads still receive the three-second delay and cancellation checks. It verifies shared actors, private ownership, sequenced request outcomes, high-refresh input pacing, and explicit reload/reconnect without duplicate guest bodies.

During the longer interval it changes latency/bandwidth, holds Alice's connection for two seconds and later for 35 seconds, checks Bob's continuing authoritative progress and body identity, then explicitly reconnects Alice. Every health check requires an advancing server tick, bounded content queues and prediction history. The request probe selects Look targeting, so walking first clears that intent through the authoritative request path. The WAN gate also walks forward/backward periodically, checks that the presented player converges to its authoritative position after stopping, and starts walking immediately before the long outage so Bob can verify that the disconnected body stops. `ELDIRON_WALK=0` retains the standing/input baseline for comparison. Request acknowledgments, replica/motion diagnostics and WASM linear-memory sizes are recorded; a static last-rendered "Playing" status cannot satisfy the test. Browser linear-memory growth also has a generous 256 MiB guard relative to the first sample; it is not browser process RSS or a quota.

## Impairment model

Both test relays keep TLS end to end with hostname/certificate verification. Native tests trust only the fixture certificate; Chrome pins the temporary fixture certificate. Nothing disables verification globally. The relays do not decode, alter or reorder application packets.

The ordinary profile adds 45–90 ms one-way propagation delay with a 192 KiB/s limit per direction. The distant profile uses 120–200 ms and 96 KiB/s per direction. Read chunks are at most 8 KiB and an eight-chunk queue pipelines propagation delay independently from serialization bandwidth. Uneven writes exercise stream framing; current read/write buffers and kernel socket buffers remain additional finite queues. This is important: adding a sleep serially to every read would reduce throughput far below the requested bandwidth and manufacture excessive head-of-line delays. Recorded ping/action round trips are the observed result, not a guarantee that RTT equals twice the configured minimum.

Blackouts hold writes in both directions while retaining TCP order and bounded queue/backpressure behavior. Cuts close existing connections. They model transport stalls and connection loss, not random IP packet loss or WebSocket message reordering. A prediction reset or disconnect under prolonged loss is expected; the authority must not leave a body walking indefinitely or reapply an uncertain economic request.

Run the JavaScript relay integrity/lifetime tests separately with:

```sh
node --test tools/network_relay.test.mjs
```

## Release and hosting interpretation

Run these with the existing [release gate](network-release-hardening.md), embedded mode/scheduling tests, native/WASM Player/Creator compilation and renderer boundary checks. [Recorded results](../concepts/networking-evidence/deployment-testing.md) separate the tested durations and profiles from remaining deployment work.

A managed pilot still needs representative content, operating-system/container quotas, script limits, filesystem capacity/failure checks, off-site backups and restore drills, supervisor restart limits, real DNS/certificate renewal, public asset publishing and observation across actual client networks and target operating systems. Those controls remain outside the gameplay protocol. Keep one owner and private paths per logical world; do not use restore to create a second simultaneous owner. Demo visitors remain accountless and create no persistent saves.

For future geographic WAN tests, use a designated staging deployment with a matching client release and normal certificate trust, disposable test identities and explicit cleanup. Record both client and server versions, elapsed time, latency/correction distributions and operational metrics. Run across actual target networks before making latency, concurrency or paid-hosting promises. This pass uses local emulation by the user's choice and provisions no external infrastructure.
