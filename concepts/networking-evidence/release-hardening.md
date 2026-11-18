# Real-network release hardening — 2026-11-17

The [release contract](../../docs/network-release-hardening.md) describes the production fixes and test boundaries. This evidence records optimized real-socket verification, not a player-capacity claim.

## Actual TCP/TLS and process tests

The [repeated gate](release-hardening.json) contains two passing runs of three integration cases: persistent process/retry/peer isolation, delayed WSS movement/private snapshots/HTTPS physics, and socket owner lifetime. The [final gate](release-hardening-final.json) adds separate-world account/token isolation and passes all three cases. JSON retains the command, server binary SHA-256, raw project SHA-256, elapsed time, scenario result and test summaries.

| Gate | Passed | Elapsed including build/test setup |
|---|---|---:|
| Repeated run 1 | yes | 62.421 s |
| Repeated run 2 | yes | 60.101 s |
| Final world-isolation gate | yes | 131.808 s |

The persistent scenario runs the actual server executable, four accounts and three characters, two forced process restarts and one contested item. Each run verifies:

- One winner for simultaneous pickup requests; exactly one item copy across world/inventory ownership.
- A committed drop/pickup whose downstream reply is held and connection cut. Exact retries, rotated resume tokens and socket takeover do not repeat effects; conflicting reuse is rejected.
- The second kill occurs while journal records are present, exercising committed-prefix replay in a fresh process. Character/body IDs and starter item IDs remain unique and intact.
- Restart invalidates memory-only resume tokens. Foreign character joins and creation beyond the configured active-player limit are rejected.
- Every full actor snapshot/replacement seen by the fixture omits foreign inventory/wallet data. No party/container grants are established.
- A blocked peer requesting large valid baselines is evicted by output backpressure while healthy peers still receive frames and pongs. Malformed/oversized frames, wrong subprotocol/origin and a certificate-hostname mismatch do not stop the world.
- Unix SIGTERM completes a new checkpoint, exits successfully and leaves zero active socket tasks in final metrics. Cancelling an embedded serving future also closes established and half-open connections.
- The final gate launches a separate world using the same release/certificate: a live token and matching credentials from the first world grant no access; its world identity differs and character roster stays empty.

Relays preserve the encrypted TLS stream, introduce 5–25 ms delay per read and use uneven 31/127/509/1,024-byte fragments. A held downstream plus connection cut creates an unobserved acknowledgment. They do not delete/reorder TCP bytes or claim to emulate WAN loss. The movement/content integration also runs through this relay and checks real fixed-input acknowledgments, prediction, private replicas and verified HTTPS content.

## Native graphical and WASM clients

The native graphical demo and account-lobby socket tests pass. The demo includes a two-second ~240 Hz networking poll loop; it remains remote and cannot create a local authoritative world. Its HUD inventory actions still update the private remote replica. A final rerun of the demo with the fractional-time input cadence passes.

The shared cadence/codec/action/content unit suite passes 11 tests, including 30/60/144/240/1,000 Hz input-call rates, bounded input batches, long-suspension behavior and duplicate action replies. The sender preserves fractional time, limits steady input traffic and suppresses empty batches; it does not advance prediction, authority or receipts itself.

[Raw browser result](release-hardening-browser.json) uses two actual Rust WASM clients in isolated Chrome tabs over WSS/HTTPS. It checks shared actors, content preloads, fixed input, sequenced request success/denial and reloading Alice while Bob keeps the same body.

During the final three-second fast-poll window, the WASM client completed **1368 pump calls** and sent **165 WebSocket messages**, remaining connected. This exceeds the server message limit in caller frequency while keeping actual sends below that limit. Background timer throttling is disabled and the fast client is foregrounded; the count measures polling, not rendered frames.

Native Player/Creator checks and browser Player/Creator WASM checks pass. Four embedded project-mode/scheduling/restore tests pass, including multiplayer Creator override and ignored dedicated sleep settings in embedded play. CPU/renderer dependency boundaries pass.

## Production changes covered

1. Socket workers are owned/reaped/bounded and cancelled with the world owner. Graceful exit joins them before final metrics.
2. Unix SIGTERM uses the same durable shutdown path as Ctrl-C.
3. Native graphical/WASM demo input sends follow a capped movement cadence rather than redraw frequency; empty/repeated render calls cannot flood transport. Prediction remains immediate.

Wire protocol 4, replication DTO 3, save schema/container/journal formats and IndexedDB layout remain unchanged. The gate introduces no public debug endpoint, arbitrary world mutation request, tenant selector, billing credential or administration API. Private fixtures and temporary TLS keys are removed.

## Reproduce and limits

```sh
python3 tools/verify_network_release.py --repeats 2 --output /tmp/release-gate.json
cargo test -p eldiron-player --lib multiplayer_project_ --offline -- --ignored --test-threads=1
cargo build -p eldiron-network --target wasm32-unknown-unknown --offline
# Generate web bindings with the wasm-bindgen CLI matching Cargo.lock.
ELDIRON_SERVER=target/release/eldiron-server node tools/verify_online_browser.mjs
```

OpenSSL and temporary loopback socket/process access are required. The recorded runs are macOS/Chrome; they do not establish execution results for every browser, device or OS. These are bounded fixture tests, not multi-hour deployment/WAN/mobile/background-tab soaks, full scripted-AI load, adversarial script isolation or paid-hosting capacity. Visual-asset publishing, account recovery, external quotas/fleet operations and representative long-running pilots remain separate work. See [the hosting concept](../eldiron-managed-hosting-concept.md).
