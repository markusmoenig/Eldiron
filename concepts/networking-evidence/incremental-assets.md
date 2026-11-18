# Incremental public asset delivery — 2026-11-18

Format-two public releases split renderer assets from their small bootstrap. The shared codec partitions stable global records into hashed packs, and publishes region geometry/paint/bakes as separate packs. All private-source exclusions from the single-file exporter apply. Endpoint/deployment overrides do not change the canonical game fingerprint or global artwork identity. The CLI syncs packs and atomically publishes the bootstrap last; existing release directories are refused.

The canonical native Player loads globals from the release directory; the browser loads up to four global packs concurrently from the bootstrap's origin with bounded streaming reads. A region pack follows its first authoritative content hint. The ordered delivery gate emits verified region artwork, physics/geometry and then the referencing frame. Socket receipt, heartbeats and input sends retain their existing independent paths. Installed regional artwork is reused for subsequent hints within the connection. Teardown drops/aborts regional jobs.

## Verification

- Eight CPU public-client tests passed, retaining all six single-file/privacy/integrity/material-pixel checks and adding streamed bootstrap/global assembly, deferred region hydration, missing/repeated/corrupt/oversized/truncated pack rejection and unchanged-pack reuse.
- Nineteen networking tests passed. New checks cover art → physics → frame ordering, reuse after the pack files are removed, source-origin/digest path restrictions, and actual HTTP socket closure when a stalled regional job is dropped. The cancellation assertion accepts EOF or a TCP reset; both close the socket.
- A real native graphical-player socket test passed with streamed globals and absent regional geometry in the lobby. Entering the authoritative demo installed the region, received the remote player and continued acknowledged movement prediction without local authoritative instances.
- The existing 200-event UI backpressure test passed through the new gate without dropping/reordering events.
- Two runtime project-mode tests and the Creator export transfer test passed. Single player and local Creator testing remain on their embedded path; ordinary single-file export still preserves document ownership and dirty state.
- Native Player/Creator/CLI, browser Player/Creator, Creator static-library and renderer dependency-boundary checks passed. No gameplay protocol or durable save format changed.

The full Chromium WebGPU run used a task-owned public directory and two-region private source. The bootstrap was **61,738 bytes compressed / 298,415 bytes MessagePack**, with **14 global packs and two region packs**. Global packs contained 51,874,239 decoded bytes in total; the two regional packs contained 12,248 decoded bytes in total. These are starter-fixture numbers, not representative large-world capacity measurements.

The browser fetched global assets before its lobby. No regional pack was requested until Play. The entered region's response was delayed 1.5 seconds; authoritative frame ticks remained hidden until artwork arrived. Exactly one regional pack was requested and the unvisited region remained unfetched. The Player continued presenting frames, reported zero runtime exceptions and zero implicit engine color readbacks. [Raw browser evidence](incremental-assets-browser.json).

## Running the browser check

Export a multiplayer source targeting explicit loopback `ws://127.0.0.1:18843/play` with `allow_insecure_loopback=true`, build/package the current WebGPU Player and codec worker into its public release directory, then run:

```sh
node tools/verify_public_client_browser.mjs PUBLIC_DIRECTORY PRIVATE_PROJECT
```

The harness supports both single-file and streamed releases, owns its server/browser profile and refuses authored `.eldiron` requests. For the unvisited-region assertion use a release with at least two regions. `ELDIRON_ASSET_DELAY_MS` controls the required-region delay. It uses local HTTP/WS; prior WSS and WAN-emulation results remain separate evidence.

The bootstrap and each pack are bounded, verified public content. Aggregate decoded source budgets are 128 MiB global plus 128 MiB regional; these are not total process/GPU quotas. All global artwork is still required at startup, and visited region art is retained within the aggregate release budget. Per-texture/audio streaming, region subdivision beyond the 16 MiB individual limit, foreign/Apple streamed document integration, multi-browser/device runtime coverage and managed hosting orchestration remain future work. Creator and opaque-byte foreign/Apple exports keep using the compatible single-file format.
