<p align="center">
  <a href="https://github.com/alan/gemini-pool-proxy" target="_blank">
    <img src="https://img.shields.io/badge/Refactored%20by-Alan-orange" alt="Refactored by Alan"/>
  </a>
</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.77%2B-brown.svg" alt="Rust"></a>
  <a href="https://tauri.app/"><img src="https://img.shields.io/badge/Tauri-2.0-blue.svg" alt="Tauri"></a>
</p>

# Gemini Pool Proxy

Gemini Pool Proxy is a high-performance API proxy and Key Pool management tool built with Rust and Tauri, focusing on **Desktop Management + Local Proxy** to provide seamless integration for developer tools like Claude Code.

> ⚠️ **IMPORTANT**: This project is licensed under [CC BY-NC 4.0](LICENSE). **Commercial use or resale is strictly prohibited.**

## ✨ Features

- **Unified API Standard** (`/v1/chat/completions`): Use one OpenAI-compatible contract for all clients.
- **Gemini Native Compatibility** (`/v1beta`): Kept for legacy/native clients, not recommended as the primary integration path.
- **Model Pool Aliasing**: Supports custom model aliases with automatic rotation across a pool of backend models.
- **Automated Key Rotation**: Built-in strategies (Round Robin / Random / Least Fail) to manage and rotate API Keys.
- **High Performance Rust Core**: Transparent, extremely low latency stream reverse proxy.
- **Desktop GUI Management (Tauri)**: Intuitive UI dashboard where configuration changes are applied immediately.

## 📍 Project Structure

- `core-rs/`: Core Rust Workspace (includes `gateway-server` & `gateway-core`).
- `desktop/tauri-app/`: Desktop app frontend built with React + Vite + TypeScript.
- `gemini-balance-next/`: (WIP) Experimental Go rewrite.

## 🚀 Quick Start

### 1. Requirements
Ensure **Rust 1.77 or higher** is installed. If you encounter the `lock file version 4 was found` error, simply run:
```bash
rustup update
```

### 2. Environment Configuration

**Option 1: Interactive Setup (Recommended for Beginners & Headless env)**
```bash
./setup.sh
```
This script will interactively prompt you for your keys and generate a valid `.env` file automatically.

**Option 2: Manual Configuration**
```bash
cp .env.example .env
```
Edit `.env` manually and define `AUTH_TOKEN`, `ALLOWED_TOKENS`, and `API_KEYS` arrays.

### 3. Choose Startup Mode

**Run Desktop Version (GUI):**
```bash
./start-desktop.sh
```
Once started, log in to the management dashboard using the `AUTH_TOKEN` string.

**Run Headless Mode (Server-only):**
Ideal for lightweight server environments. Bypasses the Tauri interface to boot strictly the performant Rust core proxy.
```bash
./start-headless.sh
```

## ⚙️ Example API Usage

Recommended defaults for new integrations:
- Base URL: `http://127.0.0.1:18080/v1`
- Auth header: `Authorization: Bearer <proxy_token>`
- Model name: use alias (for example `sonnet`, `fast`)

### Quick Verify (Smoke Test)
Run a one-shot verification script to check `/v1`, model alias routing, and `/v1beta` native compatibility:
```bash
./scripts/quick-verify-api.sh
```

Optional overrides:
```bash
BASE_URL=http://127.0.0.1:18080 \
PROXY_TOKEN=sk-user-123456 \
OPENAI_MODEL=gemini-2.5-flash \
ALIAS_MODEL=claude-sonnet \
NATIVE_MODEL=gemini-2.5-flash \
./scripts/quick-verify-api.sh
```
If a chat/native check returns `429` or `503`, the script marks it as a warning (upstream quota/high-demand) instead of a local proxy failure.

### Verify `per_key_cycle` Rotation
When `MODEL_POOL_STRATEGY=per_key_cycle`, use the dedicated verifier to assert:
- API key selection is strictly next-key round-robin
- Model selection changes only after one full key round

```bash
./scripts/verify-per-key-cycle.sh
```

Optional overrides:
```bash
BASE_URL=http://127.0.0.1:18080 \
MODEL_ALIAS=claude-sonnet \
REQUEST_COUNT=8 \
./scripts/verify-per-key-cycle.sh
```

If you run commands through `rtk` and need loopback access in that environment:
```bash
USE_RTK_PROXY_CURL=1 ./scripts/verify-per-key-cycle.sh
```

One-command headless startup + verification:
```bash
./scripts/headless-verify-per-key-cycle.sh
```

### List Models
```bash
curl -sS http://127.0.0.1:18080/v1/models \
  -H "Authorization: Bearer sk-123456"
```

### Chat Completions (OpenAI Compatible)
```bash
curl -sS http://127.0.0.1:18080/v1/chat/completions \
  -H "Authorization: Bearer sk-123456" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "sonnet",
    "messages": [{"role": "user", "content": "hello"}]
  }'
```

### Native Gemini (Compatibility Path)
```bash
curl -sS http://127.0.0.1:18080/v1beta/models/gemini-2.5-flash:generateContent \
  -H "X-goog-api-key: sk-123456" \
  -H "Content-Type: application/json" \
  -d '{
    "contents": [{"parts": [{"text": "Explain AI"}]}]
  }'
```
Compatibility note: `x-api-key`, `x-goog-api-key`, and query `?key=` are supported for legacy clients.

### Model Pool Configuration
Add the following to your `.env`:
```env
MODEL_POOLS={"claude-sonnet":["gemini-2.5-pro"],"fast":["gemini-2.5-flash","gemma-3-4b-it"]}
```
You can now pass `claude-sonnet` or `fast` as the requested model, and the system will automatically map and rotate among the underlying models.

## 🤝 Credits

This project is refactored and maintained by **Alan**.
Special thanks to the original upstream project: [snailyp/gemini-balance](https://github.com/snailyp/gemini-balance).

## 📚 Documentation & Guides (Map of Content)

To keep this repo lean, detailed instructions and specifications are housed in the `docs/` directory:

- ⚙️ **[Configuration Guide](docs/configuration.md)**: Comprehensive explanation of all `.env` parameters (Model pools, timeouts, capabilities).
- 🧭 **[Unified API Standard](docs/unified-api-standard-zh.md)**: Canonical API/auth/model alias contract for multi-provider routing.
- 📖 **[Beginner's Guide](docs/beginner-guide.md)**: A step-by-step guide to run your proxy instance within 5 minutes.
- ⚡ **[Hermes/OpenClaw Quickstart](docs/hermes-openclaw-setup-quickstart.md)**: Copy-paste onboarding for team members.
- 🤖 **[Hermes/OpenClaw Integration Guide](docs/hermes-openclaw-setup.md)**: Full English setup and troubleshooting guide.
- 🤖 **[Hermes/OpenClaw Integration Guide (ZH)](docs/hermes-openclaw-setup-zh.md)**: Practical setup flow using a unified `/v1` entrypoint.
- 🏗️ **[Architecture Overview](docs/architecture.md)**: Deep dive into the Rust (Gateway) + Tauri (Frontend) component interaction.

## 📜 License

Licensed under [CC BY-NC 4.0](LICENSE). Commercial use or resale is strictly prohibited.
