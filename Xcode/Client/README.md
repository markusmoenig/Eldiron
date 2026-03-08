# Client Xcode Wrapper

This folder contains a SceneVM Swift wrapper project template wired for
`unified_app_runner_*` FFI symbols.

The Rust crate providing those symbols is:

`clients/client-wgpu` (`package = eldiron-client-wgpu`)

## Build Rust Library

```bash
cargo build --release --package eldiron-client-wgpu --target aarch64-apple-darwin
```

Output library:

`target/aarch64-apple-darwin/release/libscenevm_unified_app.a`

For iOS/tvOS, build additional target slices as needed and add them to Xcode.
