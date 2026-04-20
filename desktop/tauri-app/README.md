# Gemini Balance Desktop (Tauri)

## Features in this baseline
- Tray menu (`Show` / `Quit`)
- Start/stop Rust sidecar (`gateway-server`)
- Runtime URL discovery (`runtime_base_url`)
- Legacy `.env` import command (`import_legacy_env`) into Keychain-backed encrypted store
- Runtime API policy: `v1` only (`/api/v1/*`, `/v1/*`), `v2` routes return `410 Gone`

## Development
```bash
npm install
npm run tauri:dev
```

`tauri:dev` / `tauri:build` 會自動透過 `scripts/tauri-with-rustup-stable.sh` 使用 rustup stable toolchain，
避免系統上其他 Rust 安裝（例如損壞的 Homebrew `rustc`）導致 Tauri CLI 啟動失敗。

To force sidecar path:
```bash
GATEWAY_SERVER_BIN=/absolute/path/to/gateway-server npm run tauri:dev
```
