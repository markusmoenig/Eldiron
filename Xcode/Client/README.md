# Client Xcode Wrapper

This folder contains a SceneVM Swift wrapper project template wired for
`unified_app_runner_*` FFI symbols.

The Rust crate providing those symbols is:

`clients/client-wgpu` (`package = eldiron-client-wgpu`)

## Build Rust Library

```bash
cargo build --release --package eldiron-client-wgpu --lib --no-default-features --features native-player --target aarch64-apple-darwin
```

Output library:

`target/aarch64-apple-darwin/release/libscenevm_unified_app.a`

For iOS/tvOS, build additional target slices as needed and add them to Xcode.

The foreign-host library build excludes the default Winit desktop launcher. The
`unified_app_runner_*` compatibility ABI and library name remain stable. The FFI
application adapter still needs convergence on the common native player; building
this slice is not verification of complete Xcode application parity. Rust owns
the WGPU surface/acquisition/presentation over the host's retained CAMetalLayer.
