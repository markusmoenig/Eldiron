# SceneVM Unified App Template

Single Rust app that runs on:
- Native desktop via winit (`cargo run -p unified-app`)
- WebAssembly (`cargo run-wasm --package unified-app` or your own wasm-bindgen pipeline)
- macOS/iOS/iPadOS via SwiftUI + CAMetalLayer using the bundled Xcode project

## Rust entry points
- `src/lib.rs` defines `TemplateApp` implementing `scenevm::SceneVMApp`.
- `src/main.rs` calls `scenevm::run_scenevm_app(TemplateApp::new())` for native and wasm.

## Xcode template
- Location: `SceneVMAppTemplate.xcodeproj` with sources in `SceneVM/`.
- Swift FFI wrapper: `SceneVM/SceneVMFFI.swift` calls these symbols from the static library:
  - `unified_app_runner_create/destroy/resize/render`
  - `unified_app_runner_mouse_down/up/move/scroll`
- Build the Rust static lib (Apple Silicon example):
  ```bash
  cargo build --release --package unified-app --target aarch64-apple-darwin
  ```
  Artifact: `target/aarch64-apple-darwin/release/libscenevm_unified_app.a`

  Additional Apple targets (optional):
  ```bash
  # iOS / iPadOS device
  cargo build --release --package unified-app --target aarch64-apple-ios
  # Apple Silicon simulator
  cargo build --release --package unified-app --target aarch64-apple-ios-sim
  # Intel simulator (only if you need it)
  cargo build --release --package unified-app --target x86_64-apple-ios
  ```
  Artifacts:
  - `target/aarch64-apple-ios/release/libscenevm_unified_app.a`
  - `target/aarch64-apple-ios-sim/release/libscenevm_unified_app.a`
  - `target/x86_64-apple-ios/release/libscenevm_unified_app.a`
  Add the slices you need to Xcode’s “Link Binary With Libraries” and point Library Search Paths at the corresponding `target/<triple>/release` dirs.
- Xcode links `libscenevm_unified_app.a` and presents directly into a `CAMetalLayer` (GPU-only).
- Input handling in Swift is wired to the `mouse_*`/`scroll` hooks exposed by `TemplateApp`.

## Customizing
- Edit `TemplateApp` in `src/lib.rs` to change the scene or handle inputs.
- If you rename the runner functions, update `SceneVMFFI.swift` to match.
- Library search paths in the Xcode project assume `$(PROJECT_DIR)/../target/<triple>/release`.
