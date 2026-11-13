# Creator Xcode GPU host

Creator's Xcode wrapper uses `NativeCreatorApplication` and the same paint,
Swash glyph-atlas and GPU view composition as the Rust window host. Swift owns
the view and retains its CAMetalLayer; Rust creates the WGPU surface and is the
only drawable consumer/presenter. There is no `rust_draw` bitmap ABI or Swift
texture-copy renderer.

Build a matching Rust library slice before building the wrapper:

```sh
MACOSX_DEPLOYMENT_TARGET=26.1 cargo build -p eldiron-creator --lib --release \
  --no-default-features --features staticlib --target aarch64-apple-darwin
xcodebuild -project Xcode/Eldiron/Eldiron.xcodeproj -scheme 'Eldiron macOS' \
  MACOSX_DEPLOYMENT_TARGET=26.1 CODE_SIGNING_ALLOWED=NO build
```

For iOS, use `aarch64-apple-ios` or `aarch64-apple-ios-sim`, setting
`IPHONEOS_DEPLOYMENT_TARGET=26.1`. `ELDIRON_RUST_LIB_DIR` selects the library
directory, defaulting to the matching target's release directory. Override it
when using a shared Cargo target directory or a debug library. These commands
describe the verified SDK configuration; older OS versions remain unverified.

`Bridge.h` documents the main-thread ABI. Resize dimensions are physical pixels;
input is in logical points. Keep the layer alive until `rust_shutdown` or a
successful `rust_replace_layer` returns. Swift never reads `currentDrawable`.
Display callbacks honor the application's update cadence and only present dirty
frames. Suspend/resume and device/layer replacement retain the CPU application.
Clipboard completions carry the original request ID; Edit menu commands use the
same controller route as keyboard shortcuts.

`Tests/NativeCreatorHostCheck.swift` links to the actual static library and checks
presentations, zero root bitmap/readback, resize/density, suspend/resume and layer
replacement on retained detached layers. Obtain native link arguments with
`cargo rustc -p eldiron-creator --lib --no-default-features --features staticlib --
--print native-static-libs`; pass those arguments, `-leldiron_creator`, the library
search directory and `-import-objc-header 'Xcode/Eldiron/Eldiron Shared/Bridge.h'`
to `swiftc` when linking that executable. Run it on the main thread with Metal
available. It is a detached-layer check, separate from physical window evidence.

Native macOS/iOS composition/accessibility, complete physical input/document
workflows, mobile runtime execution, multiwindow ownership and distribution
remain in the rewrite acceptance matrix. The legacy Swift text-key adapter is
still an input adapter, pending adoption of the client's native text responders.
