# Networking release hardening

This pass adds a repeatable real-socket gate and fixes connection lifetime, service shutdown and input-send cadence. It preserves WSS protocol 4, replication DTO 3, existing save formats, embedded single-player/Creator execution and browser/WASM transport.

## Runtime changes

The dedicated serving future owns its socket tasks in a bounded, reaped `JoinSet`. Graceful shutdown checkpoints the world, aborts and joins socket tasks, then publishes final metrics. Cancelling or failing the owner also drops its task set, preventing old-world sockets from surviving inside an embedding process. TCP permits and active counters belong to those tasks, including tasks cancelled before their first poll. Half-open HTTP/TLS connections share this lifetime.

The CLI handles Unix SIGTERM through the same durable shutdown path as Ctrl-C. This lets a service/container supervisor request a checkpoint rather than relying on process destruction. Other platforms retain Ctrl-C support. An ambiguous save failure still fails closed; shutdown does not publish or save uncommitted effects.

`InputCadence` bounds native graphical Player and WASM demo movement sends independently of render/poll rate. It retains fractional frame time at 60/144 Hz, suppresses empty batches and sends pending input at the fixed movement cadence. A long suspension cannot bank a catch-up packet burst. Prediction and pending-frame ownership remain with `MotionClient`; actions, acknowledgment handling and resync requests keep their existing paths. This prevents legitimate high-refresh clients from exceeding the server's message-rate limit with repeated/empty inputs.

## Real-socket release gate

```sh
python3 tools/verify_network_release.py --repeats 2 \
  --output /tmp/eldiron-release-gate.json
```

Requires Rust dependencies already available for `--offline`, OpenSSL and permission to bind temporary loopback ports. The runner uses optimized builds and serial test cases. Fixtures own and stop their child server processes and relays and remove private temporary accounts, saves, certificates and keys. There is no public debug/admin endpoint or test mutation command in the gameplay protocol.

The persistent scenario runs the actual `eldiron-server` executable through opaque TCP relays. TLS stays end-to-end with certificate and hostname verification. Relays add 5–25 ms delay per read and uneven 31/127/509/1,024-byte writes. They can hold one downstream and cut its connections. This models latency, fragmentation and disconnection; it does not claim to emulate WAN packet loss or reorder TCP/WebSocket messages.

The fixture verifies:

- Three saved characters and their starter items survive a process kill. An offline verified fixture copy adds one contested item and passive actors for a large, valid baseline.
- Two players request the same item over separate WSS connections; exactly one gets it and ownership remains unique.
- A downstream is held until the private committed head contains an operation's receipt, then cut before the client consumes its acknowledgment. Reconnect, token rotation, exclusive socket takeover and exact retries do not repeat the effect.
- A second kill occurs with committed journal records present. Fresh-process restoration retains characters/items and receipts; old-context retries return the saved result. Reusing a sequence for different parameters conflicts. Restart invalidates memory-only resume tokens.
- Foreign entities' inventory/wallet fields stay absent from received public snapshots. A full host rejects an extra character, and an unowned character cannot be selected.
- Malformed/oversized packets, a wrong WebSocket subprotocol, a denied Origin and a certificate hostname mismatch fail without stopping healthy peers.
- A stalled peer requesting large baselines hits bounded output handling and disconnects; healthy peers continue receiving frames and answering pings.
- A separate world running the same release/certificate accepts neither the first world's live resume token nor its username/password. World/account directories remain owner-local.
- SIGTERM advances the durable checkpoint, exits successfully and leaves the final active transport count at zero.

Separate tests drive native prediction, private replicas and HTTPS physics loading through the delayed relay. Another test stops or cancels an embedded serving future while established and half-open sockets exist; both must close promptly. These complement the existing storage fault-injection tests.

## Graphical Player and WASM checks

Native graphical Player socket tests exercise the account lobby and accountless demo without creating a local authority. The demo test now polls input/network handling at a high frequency for two seconds; it must remain connected and preserve its remote-only runtime. Run only those socket tests, without selecting GPU-only ignored tests:

```sh
cargo test -p eldiron-player --lib multiplayer_project_ --offline \
  -- --ignored --test-threads=1
```

A further native test runs sixty wall-clock seconds with three-second HTTP content delays and three demo joins. Shared content delivery and UI backpressure preserve queued events during loading; see [the content delivery contract](client-content-delivery.md).

Build the WASM network module and matching wasm-bindgen output, then use `tools/verify_online_browser.mjs`. It runs two isolated Chrome tabs over WSS/HTTPS, checks shared private replicas, preloads, movement and sequenced actions, then exercises more than 120 Rust pump calls per second while counting actual socket sends. Background timer throttling is disabled for this explicit rate test. Reloading Alice must allocate one replacement guest while Bob retains its existing body; no old guest remains in the public world. The browser runner now also delays HTTPS content by three seconds, cancels a pending fetch, checks bounded catch-up beyond the old socket inbox size and runs a sixty-second post-reload soak. Native Player/Creator and WASM Player/Creator compilation and the renderer/CPU boundaries remain part of verification.

## Hosting and remaining release work

Use an isolated process and protected save/parts/lock/backup/metrics paths per world. Keep credentials and operational controls outside public content and the gameplay protocol. Configure a supervisor's SIGTERM grace above the measured worst checkpoint duration for the actual project/filesystem. A forced kill can lose ordinary simulation since its last durable capture; committed action receipts remain authoritative on reopening. Do not deploy two copies of one logical world as simultaneous owners.

This gate is local TCP/TLS evidence, not production capacity or a long deployment soak. It does not cover arbitrary scripted AI, sandbox/resource enforcement, multi-hour WAN/mobile/background-tab behavior, every browser/OS, public visual-asset publishing, account recovery or fleet administration. Large active-world budgets and slow content delivery still need representative deployment tests. Those remain prerequisites for broader release and managed hosting, as described in the [hosting concept](../concepts/eldiron-managed-hosting-concept.md).

See [recorded evidence](../concepts/networking-evidence/release-hardening.md), [operations](server-operations.md), [storage recovery](network-performance.md) and [scheduling semantics](simulation-scheduling.md).

## Longer local WAN-emulation and deployment follow-up

The [deployment testing gate](network-deployment-testing.md) extends this with sustained saved-world traffic, pipelined propagation delay/bandwidth controls, longer blackouts, process restarts, periodic backup retention and restoration to a fresh slot. The browser gate now checks advancing authority ticks, motion diagnostics, memory and acknowledged requests; its WAN mode exercises walking/reconciliation and disconnection expiry. These remain local emulation and bounded-duration tests, with target-platform and real-WAN limits stated in the recorded evidence.
