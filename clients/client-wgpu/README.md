# Eldiron Client WGPU

The native `eldiron-client-wgpu` and `eldiron-client` executables use the same
application-owned WGPU host, runtime session and game-screen presenter. Scene
views, UI and Swash glyph atlases compose on the shared GPU without a viewport
framebuffer readback.

## Native

```bash
cargo run -p eldiron-client-wgpu -- path/to/game.eldiron
cargo run -p eldiron-client -- path/to/game.eldiron
```

Both default to `./game.eldiron`. Use `--help` for options, `--debug` for authored
debug behaviors, and `--render-debug` for transfer/presentation diagnostics.
`--smoke` requires three successful surface presentations within ten seconds;
preparing an offscreen frame alone does not count as a presentation.

The former launchers remain explicit comparison adapters during migration:

```bash
cargo run -p eldiron-client-wgpu --features legacy-reference --bin eldiron-client-wgpu-reference -- path/to/game.eldiron
cargo run -p eldiron-client --features legacy-reference --bin eldiron-client-reference -- path/to/game.eldiron
```

The classic browser entry still uses its compatibility adapter. Browser WebGPU
host/bootstrap and complete platform/workflow verification remain tracked in
`concepts/native-wgpu-rewrite-progress.md`.

## Xcode / Apple Platforms

The static library keeps the name `libscenevm_unified_app.a` and compatibility
exports `unified_app_runner_*` used by `Xcode/Client`:

```bash
MACOSX_DEPLOYMENT_TARGET=26.1 cargo build --release -p eldiron-client-wgpu --lib --no-default-features --features native-player --target aarch64-apple-darwin
```

Disabling defaults excludes Winit. This configuration now connects the retained
Apple ABI to the common player and Rust-created surface over the host's retained
CAMetalLayer. The former adapter remains only in builds without `native-player`.
See `Xcode/Client/README.md` for build, document/layer lifetime and verification
instructions. Physical platform, native IME and complete release acceptance remain
pending in `concepts/native-wgpu-rewrite-progress.md`.
