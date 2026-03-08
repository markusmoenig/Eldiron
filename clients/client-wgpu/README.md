# Eldiron Client WGPU

GPU-direct Eldiron player using `scenevm::SceneVMApp`.

## Native

```bash
cargo run -p eldiron-client-wgpu -- path/to/game.eldiron
```

If no path is provided it tries `./game.eldiron`.

## Xcode / Apple Platforms

This crate exports a static library named `libscenevm_unified_app.a` and
compatibility FFI symbols (`unified_app_runner_*`) so it can be used with the
SceneVM Swift wrapper template in:

`Xcode/Client`.
