# Content loading without blocking network progress

Native graphical Player, browser graphical Player, the WASM demo viewer and the native online CLI use one ordered `ContentStream`. Required public geometry/physics downloads run separately from socket receipt, outbound inputs and heartbeats. Optional preloads keep their separate cache path. No protocol, replication DTO or save format changes are required.

Gameplay delivery waits at the frame needing a missing package. Verified packages arrive before that frame; later frames, private events, offers and action receipts keep their original order. Packets are neither dropped nor coalesced to catch up. Pong packets need no gameplay delivery, and optional preloads can begin immediately without subscribing to live entities. A frame receives its exact immutable content stamp even when a newer optional package has already replaced the older cache entry.

This removes network waits from the receive loop. Download verification, decoding and installation still require CPU time; WASM installation runs on the browser thread. During an in-game content change, the client can continue predicting against its last installed collision state until the ordered authoritative frames arrive. Existing bounded prediction history and resync behavior still apply.

## Bounds and lifetime

- Ordered gameplay staging: at most 256 packets and 4 MiB of encoded packet data. Overflow fails explicitly and requires reconnect/resync. Any terminal loading/queue error remains latched and cancels fetch jobs; repeated polling cannot accidentally resume with an incomplete frame.
- The graphical worker delivers at most eight events per ten-millisecond poll into its existing 64-event UI channel. A full UI channel retains one unsent event and pauses delivery while socket reads continue; it does not discard frames or receipts. Prolonged stalls still hit the ordered queue limit. Terminal errors remain available even when that channel is full.
- One required fetch at a time. A front frame waiting for downloads has a ten-second total deadline, including multiple missing packages.
- Each HTTP package retains the existing 8 MiB transport/hash/structure validation limit. Packages delivered ahead of one frame have a combined 16 MiB encoded staging budget.
- Optional prefetch: at most four concurrent owned jobs. The reusable cache retains at most four whole-region packages / 16 MiB of encoded data. Exact geometry, physics and content-key stamps determine reuse and pending-job deduplication.
- Native jobs are aborted when their owner closes. Browser fetches use an AbortController and ten-second timeout; dropping/reloading the client aborts outstanding requests and clears timers.

These are encoded-payload limits, not process RSS guarantees. Decoded geometry, installed world maps and bounded socket/UI channels have their own memory costs. A sufficiently slow consumer still disconnects explicitly; the queues do not grow without limit. Private inventories, credentials and authored scripts never enter public HTTP packages.

`ContentStream::diagnostics`, `DemoClient.state().content_delivery` and `PlayerApplication::online_content_diagnostics()` expose current and peak packet/byte counts, required downloads and whether content is pending. They expose loading state without granting server authority.

## Repeatable verification

```sh
cargo test -p eldiron-network --lib --offline
cargo test -p eldiron-player --lib multiplayer_project_ --offline \
  -- --ignored --test-threads=1 --nocapture
```

Socket tests require permission to bind loopback ports. The network tests exercise ordered delivery across 200 frames, an older exact physics revision after a newer cache entry, bounded packet/byte queues, HTTP failure, deadline handling and cancellation of a real HTTP socket. The ignored native graphical soak runs for sixty wall-clock seconds with three-second HTTP delays and three demo joins. It checks unique guest bodies and absence of local authoritative simulation. Existing native saved-account and starter-inventory interaction tests also run under that filter.

Build the WASM network module and use the matching wasm-bindgen version to populate `/tmp/eldiron-online-web/pkg`, then run:

```sh
ELDIRON_SERVER=target/release/eldiron-server \
  node tools/verify_online_browser.mjs
```

The browser runner defaults to a three-second delay on HTTPS `/content/` requests and a sixty-second soak after interaction/reload checks. `ELDIRON_CONTENT_DELAY_MS` (0–4,000) and `ELDIRON_SOAK_SECONDS` (0–86,400) allow shorter or longer bounded runs. Each tab pumps real elapsed time; CDP delays content requests while leaving WSS traffic alone. The test cancels a pending client's fetch, replaces its guest, checks ordered catch-up above the previous 64-packet inbox size, exercises sequenced actions and high-refresh movement, then reloads one tab while the other retains its body.

See [recorded results](../concepts/networking-evidence/client-content-delivery.md). This is short local socket/TLS evidence. Multi-hour deployments, background/mobile suspension, representative heavy scripted worlds, WAN conditions and fleet quotas remain release/hosting work.

## Hosting implications

Each connection owns its disposable queues, HTTP jobs and cache; closing a session cancels that work. World selection, authentication, save ownership and public content publication remain server/deployment concerns. No client-chosen tenant, asset URL, public administration RPC or local-authority fallback is introduced. Single player and Creator testing keep their embedded execution and local content path.

For longer outages, bandwidth/latency profiles and saved-host restarts, see [deployment testing](network-deployment-testing.md).
