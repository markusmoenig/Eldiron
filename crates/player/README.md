# Shared Eldiron player

`eldiron-player` is the common player library used by the normal desktop client,
WebGPU browser client, and Xcode/Apple hosts. The user-facing graphical executable
is `eldiron-client`, in `clients/client`. The terminal client remains separate.

Scene views, game screens and Swash text use the shared GPU presentation path.
The library owns the common player controller and host adapters; it does not
provide a second packaged graphical client.

## Desktop

From the workspace root:

```bash
cargo run --release --locked -p eldiron-client -- path/to/game.eldiron
```

The client defaults to `./game.eldiron`. Use `--help` for options, `--debug` for
authored debug behaviours, and `--render-debug` for presentation diagnostics.
`--smoke` requires three successful surface presentations, rather than only
preparing an offscreen frame.

## Browser WebGPU

Package the normal client and its CPU codec worker together:

```bash
python3 tools/package_web.py player target/web-player
python3 -m http.server 8000 --bind 127.0.0.1 --directory target/web-player
```

Place `game.eldiron` in the generated directory, or select another file using
`?project=Example.eldiron`. Add `--offline` to the packaging command when the
locked dependencies are cached. The output includes matching JS/WASM bindings
and the worker bootstrap; a bare WASM executable build is not a complete package.
The browser must support WebGPU.

## Apple hosts

The library deliberately retains the name `scenevm_unified_app` and archive
`libscenevm_unified_app.a`, together with its established exported ABI, so the
Xcode hosts keep their existing linkage. Only the Cargo package/location changed.

```bash
MACOSX_DEPLOYMENT_TARGET=26.1 cargo build --release --locked -p eldiron-player --lib --no-default-features --features native-apple --target aarch64-apple-darwin
```

See [Xcode host instructions](../../Xcode/Client/README.md) for platform and
lifetime details. `native-window`, `native-web` and `native-apple` select the
existing host adapters; `native-player` selects the controller without Winit.

## Reference adapters

The explicit CPU comparison client remains available:

```bash
cargo run --release --locked -p eldiron-client --features legacy-reference --bin eldiron-client-reference -- path/to/game.eldiron
```

The historical SceneVM-runner adapter is retained as the optional
`legacy_reference` library example for comparison, rather than a packaged client.
Remaining workflow, hardware and performance acceptance is tracked in
[the rewrite progress record](../../concepts/native-wgpu-rewrite-progress.md).
