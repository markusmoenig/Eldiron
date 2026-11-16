# Eldiron Client WGPU

The native `eldiron-client-wgpu` and `eldiron-client` executables use the same
application-owned WGPU host, runtime session and game-screen presenter. Scene
views, UI and Swash glyph atlases compose on the shared GPU without a viewport
framebuffer readback.

## Native

```bash
cargo run --release --locked -p eldiron-client-wgpu -- path/to/game.eldiron
cargo run --release --locked -p eldiron-client -- path/to/game.eldiron
```

Both default to `./game.eldiron`. Use `--help` for options, `--debug` for authored
debug behaviors, and `--render-debug` for transfer/presentation diagnostics.
`--smoke` requires three successful surface presentations within ten seconds;
preparing an offscreen frame alone does not count as a presentation.

The former launchers remain explicit comparison adapters during migration:

```bash
cargo run --release --locked -p eldiron-client-wgpu --features legacy-reference --bin eldiron-client-wgpu-reference -- path/to/game.eldiron
cargo run --release --locked -p eldiron-client --features legacy-reference --bin eldiron-client-reference -- path/to/game.eldiron
```

## Browser WebGPU

Both graphical executable names use the same browser launcher and common player.
Serve the generated files over HTTPS or localhost with `game.eldiron` alongside
`index.html`. Loading uses asynchronous fetch and the same project/archive decoder
as desktop. `?project=SomeGame.eldiron` selects another URL; `?debug=1` enables
its authored debug behaviors.

From the workspace root, package the release host, matching pinned WASM
bindings, and the CPU codec worker used by project asset jobs:

```bash
python3 tools/package_web.py player target/web-player
python3 -m http.server 8000 --bind 127.0.0.1 --directory target/web-player
```

Copy the project into `target/web-player/` before serving it. The package includes
the worker bootstrap and its generated JS/WASM; a bare `cargo run-wasm` build of
the player alone does not package those resources. Add `--offline` to the
packaging command when all locked dependencies are cached.
The browser must provide WebGPU; startup errors appear on the page. Its canvas
uses the shared GPU compositor/scene outputs with no Canvas2D image presentation.
Browser text input uses a host-owned invisible textarea for composition and
committed text; the common controller retains selection, history and GPU text
painting. Native key commands and DOM text insertion remain separate.
Successful surface acknowledgements update `data-eldiron-presentations` on the
root element; `data-eldiron-state` reports loading, starting, playing or error.

Appearance variant preparation uses a bounded CPU queue, processing at most one
variant per browser application update. Native builds retain the cancellable
worker. Full browser input, audio, lifecycle/device-loss and loaded-project
performance acceptance remain tracked in
`concepts/native-wgpu-rewrite-progress.md`.

## Xcode / Apple Platforms

The static library keeps the name `libscenevm_unified_app.a` and compatibility
exports `unified_app_runner_*` used by `Xcode/Client`:

```bash
MACOSX_DEPLOYMENT_TARGET=26.1 cargo build --release --locked -p eldiron-client-wgpu --lib --no-default-features --features native-apple --target aarch64-apple-darwin
```

Disabling defaults excludes Winit. This configuration now connects the retained
Apple ABI to the common player and Rust-created surface over the host's retained
CAMetalLayer. The former adapter remains only in builds without `native-player`.
See `Xcode/Client/README.md` for build, document/layer lifetime and verification
instructions. Physical platform, native IME and complete release acceptance remain
pending in `concepts/native-wgpu-rewrite-progress.md`.
