# Delayed content delivery and client soak evidence

Recorded 2026-11-17 on macOS / arm64. [Machine-readable results](client-content-delivery.json) identify the dirty source set, project, optimized browser host binary and generated WASM module by SHA-256. The test commands and loading contract are in [client content delivery](../../docs/client-content-delivery.md).

## Findings and changes

Required HTTP content had been awaited inside socket processing. In the WASM demo this also blocked subsequent pump calls, allowing the 64-packet socket inbox to fill during a region download. Native graphical loading used the same synchronous receive-loop pattern.

A shared ordered stream now downloads separately while receiving gameplay packets and sending inputs/heartbeats. It preserves every frame and control/event packet in order and delivers exact verified geometry/physics before its frame. Packet and byte budgets, a front-frame deadline, aggregate package staging bounds and latched terminal failures prevent indefinite buffering or partial recovery. Owned native tasks and browser AbortControllers cancel on close/reload.

The first delayed native run exposed another failure after the download completed: replay filled the UI channel during the first scene's installation. Delivery now retains one unsent event and pauses on UI backpressure while socket receipt continues. A capacity-one UI channel regression delivers all 200 events in order, including repeated delivery attempts while full. The worker also retains its terminal error when a full output channel cannot carry a failure notification.

## Final observed runs

| Check | Observed result |
| --- | --- |
| Native graphical application soak | 60.005 seconds, 3 successful joins with distinct view/input incarnations, three-second content delays |
| Native ordered staging peak | 153 packets / 114,027 encoded bytes; limits 256 packets / 4 MiB |
| Native authority | 0 local simulation ticks; no local server instances; exactly one guest body in each live view |
| WASM browser soak | 60.041 seconds after initial interactions/reloads, 2 Chrome tabs, 24 health/state checks |
| WASM required HTTP content | Three-second HTTPS request delays; queue peaks 156 and 161 packets / 117,677 and 122,156 encoded bytes |
| Browser cancellation | 2 canceled content requests observed; 8 content requests delayed |
| Browser fast polling | 1,364 Rust pump calls and 165 actual socket sends over three seconds; remained connected |
| Browser reload | Alice replaced without a duplicate body; Bob retained its existing body |
| Networking regressions | 16 passed, including FIFO across 200 frames, exact older physics after a newer cache entry, queue limits, HTTP failure/deadline and actual HTTP cancellation |
| UI backpressure regression | 1 passed |
| Native socket tests | 3 passed: saved account/rejoin, demo gameplay/private inventory and delayed-content soak |
| Embedded project modes / scheduling | 2 + 2 passed, including Creator's local override and single-player policy isolation |
| Compilation | Native Player, Creator, online CLI; WASM Player/Creator passed |
| Boundaries / syntax | Renderer boundary gate, changed Rust formatting, JavaScript syntax and tracked diff whitespace checks passed |

An empty accountless demo resets its transient world, so the native actor number was `1` on all three joins. This is expected: identity and uniqueness assertions use a fresh session/view incarnation and the live body's count. Saved-account identity/restoration remains covered separately by the account test and earlier release gate.

The native test drives the actual graphical application's lobby, runtime and presentation code without opening a GPU window. Browser tests use actual compiled WASM, WebSocket and HTTPS APIs in isolated temporary Chrome profiles. TLS pins a temporary local certificate. Native fixtures use explicit insecure loopback WS/HTTP. The browser host is the optimized binary from the previous release gate, with its binary hash recorded; no server-side protocol or debug endpoint was added in this pass. Temporary hosts, relays, profiles, keys and account/save fixtures are cleaned up.

## Practical limits

These are short local socket/TLS tests and targeted compilation/runtime regressions. They do not establish WAN loss behavior, visual rendering quality, all browser/OS combinations, multi-hour/background/mobile behavior, heavy scripted-world capacity or managed-hosting readiness. The delayed-content scenarios cover initial loading and reconnect; representative in-game region/door updates under load remain deployment work. Hash verification, decoding and installation still cost CPU time, including browser main-thread work.

WSS protocol 4, replication DTO 3 and save formats remain unchanged. Single player and Creator retain their embedded paths. Deployment-specific content publishing, tenant quotas and fleet administration remain separate, as described in the [hosting concept](../eldiron-managed-hosting-concept.md).
