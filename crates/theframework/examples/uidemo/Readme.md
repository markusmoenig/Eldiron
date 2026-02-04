# UIdemo

Provides a demo of the user interface option of TheFramework.

## Running on the Desktop

```bash
cargo run --release --package uidemo
```

Will run the example on the Desktop utilizing pixels and winit.

## Running on the Web

Install the WASM32 target:

```bash
rustup target add wasm32-unknown-unknown
```

Build the project and start a local server to host it:

```bash
cargo run-wasm --release --package uidemo
```

Open http://localhost:8000/ in your browser to run the example.

To build the project without serving it:

```bash
cargo run-wasm --release --build-only --package uidemo
```

## Building for Xcode

To build for Xcode you need to uncomment the last three lines in the Cargo.toml file of the Circle example:

```toml
[lib]
name = "rustapi"
crate-type = ["staticlib"]
```

and than build to a static lib via

```bash
cargo build --release --package uidemo
```

Copy the resulting librust.a lib to the Xcode/TheFramework folder, open the project in Xcode and run or deploy it.
