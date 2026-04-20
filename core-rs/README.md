# Core Rust Workspace

## Crates
- `gateway-core`: key pool, runtime domain models, dashboard/config schema.
- `gateway-server`: Axum API runtime (`/api/v1/*`, `/v1/*`, v2 routes return `410 Gone`).
- `config-secure`: macOS Keychain + encrypted config storage utilities.

## Run
```bash
cp .env.example .env
./start-gateway-server.sh
```

## Test
```bash
RUSTC=$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc \
RUSTDOC=$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustdoc \
$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo test --workspace
```
