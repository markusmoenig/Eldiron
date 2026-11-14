# Eldiron player Xcode host

`Xcode/Client` drives the same `PlayerApplication`, runtime session, scene/UI
compositor and Swash glyph atlases as the native desktop clients. Swift supplies
a retained CAMetalLayer, input, file bytes and display callbacks. Rust creates the
WGPU surface, configures its device/format and exclusively presents its drawables.

The static library remains `libscenevm_unified_app.a`; the established
`unified_app_runner_*` and `eldiron_player_runner_*` exports are preserved. The
foreign-host build explicitly excludes Winit:

```bash
MACOSX_DEPLOYMENT_TARGET=26.1 cargo build --release -p eldiron-client-wgpu --lib --no-default-features --features native-apple --target aarch64-apple-darwin
```

For iOS device/simulator, use `aarch64-apple-ios` / `aarch64-apple-ios-sim`
and set `IPHONEOS_DEPLOYMENT_TARGET=26.1` for the Rust/C dependency build.
Build a matching slice for each requested architecture. Xcode resolves its
library through `ELDIRON_RUST_LIB_DIR`, defaulting to the corresponding target's
release directory. Override it for a different Cargo target directory/profile:

```bash
xcodebuild -project Xcode/Client/Client.xcodeproj -scheme SceneVM -sdk macosx \
  ELDIRON_RUST_LIB_DIR=/absolute/path/to/target/debug CODE_SIGNING_ALLOWED=NO build
```

Use `ARCHS=arm64` when validating a single ARM64 simulator slice. Generic
simulator builds otherwise also request Intel. Universal/XCFramework packaging
and signed/device distribution remain separate work.

Open an `.eldiron` project (legacy `.scenevm` document associations also remain).
Both JSON and ZIP archives pass to the existing Rust decoder as binary Data.
Saving preserves the original authored bytes, including after playing. An optional
bundled `game.eldiron` supplies the new-document default. Each document retains its
CPU session across Swift view/layer recreation; only GPU presentation is rebuilt.

All FFI calls and destruction run on the main/creation thread. The Swift handle
strongly retains the backing layer until Rust destroys/replaces its surface.
See [eldiron_player.h](eldiron_player.h) for buffer ownership, scale, input-token
and return-code contracts. Swift does not create a separate Metal renderer or
acquire its own drawables.

Run `CARGO_TARGET_DIR=/absolute/path/to/target python3 tools/check_native_player_apple.py`
from the repository root to exercise the real Swift/static-library bridge on
retained detached layers. This checks actual presentations, zero implicit
readback, authored source preservation and session survival across replacement.
It requires macOS/Xcode/Metal and does not open a Winit window.

Native macOS input uses one retained NSTextInputContext/client per logical editor.
UIKit uses one zero-size, non-drawing UITextInput responder per editor. Both query
surrounding text, selection, marked text and geometry from the shared Parley
controller; revisioned UTF-16 ranges reject stale owners and surrogate splits.
The Rust editor owns composition, typed insertion, backspace and undo; Swift does
not draw or maintain a separate editable document. The bridge tool executes the
macOS client ownership/Unicode checks and typechecks the UIKit protocol harness
against the simulator SDK. `Tests/NativeUIKitTextClientCheck.swift` is ready for a
runtime harness; SDK compilation does not execute it.

Full physical macOS/iOS and locale/IME interaction, UIKit runtime behavior,
release measurements and Creator's separate Xcode wrapper remain tracked in
`concepts/native-wgpu-rewrite-progress.md`. An SDK build or detached-layer test
does not establish those acceptance criteria.
