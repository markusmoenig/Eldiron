# WebGPU browser packages

Build a self-contained HTTP directory from the repository root:

```sh
python3 tools/package_web.py creator /tmp/eldiron-web/creator --offline
python3 tools/package_web.py player /tmp/eldiron-web/player --offline
python3 -m http.server 8000 --directory /tmp/eldiron-web
```

Copy the desired `.eldiron` archive into the corresponding directory, then open
`http://localhost:8000/creator/?project=Project.eldiron` or the player URL. Use
localhost or an HTTPS origin with WebGPU support. The packager builds the
workspace-pinned bindings tool; a global wasm-bindgen CLI is unnecessary.
Omit `--offline` when locked dependencies still need downloading. The generic
`cargo run-wasm` example launcher does not assemble worker sidecars; use this
packager for Creator and player projects.

Keep the generated application JS/Wasm, codec-worker JS/Wasm and
`eldiron-bake-codec-worker-bootstrap.js` together. Serve JS as JavaScript and
Wasm as `application/wasm`. Deployments with a Content Security Policy must
allow same-origin module workers. No shared-memory or cross-origin-isolation
headers are required for the transferable-buffer protocol.

The codec module is built with `rusterix --no-default-features`; it initializes
no window, GPU, UI or game session. Its only task is decoding the five original
orthographic bake channels. The application retains source identity, budget
charges, cancellation and publication. Up to two requests occupy worker slots;
a shared 64 MiB transport reservation bounds extra transfer/comparison copies.
Each decoded-source cache also preserves its existing 32 MiB owned-input/output
budget. Codec scratch is independently limited by the decoder. Retained Wasm
linear-memory and browser allocator overhead are separate from owned-byte
counters.

Closing a document cancels publication. Already-running requests retain their
slot and charges until a result/error arrives; results cannot enter a different
controller. A request cancelled before worker readiness can retire without ever
being sent. Missing worker assets surface an error rather than silently moving
canonical decoding back onto the UI thread. Unconfigured explicit CPU/reference
consumers retain their cooperative implementation.

These commands build packages. They do not establish the complete platform,
input, project, worker-cancellation or performance acceptance matrix.

For live resource inspection, the generated application module exports
`eldironCodecWorkerStats()`, returning JSON with active requests, owned job and
transport bytes, completed/cancelled requests and failures. DOM diagnostic
attributes describe the most recently prepared frame and can remain unchanged
while an idle application retires a background result.
