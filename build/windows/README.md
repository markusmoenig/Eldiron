# Windows Build

This is the simplest way to build the Windows Creator binary from macOS or Linux.

## Requirements

- Docker Desktop or a compatible Docker daemon
- Rust + `rustup`
- `cross`

Install `cross` if needed:

```bash
cargo install cross --git https://github.com/cross-rs/cross
```

## One-Time Toolchain Setup

`cross` uses a Linux host toolchain internally. Make sure that toolchain and the Windows GNU target exist:

```bash
rustup toolchain install stable-x86_64-unknown-linux-gnu
rustup target add x86_64-pc-windows-gnu --toolchain stable-x86_64-unknown-linux-gnu
```

## Build The Creator

From the repository root:

```bash
cross build --release --target x86_64-pc-windows-gnu -p eldiron-creator
```

Output:

```text
target/x86_64-pc-windows-gnu/release/eldiron-creator.exe
```

## Notes

- Build only one Windows target at a time. Running multiple `cross` builds in parallel can race on the target/toolchain setup.
- If `cross` says it cannot connect to Docker, start Docker first and rerun the command.
- This workflow builds the GNU Windows target: `x86_64-pc-windows-gnu`.
