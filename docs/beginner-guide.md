# Gemini Balance Beginner's Guide (Desktop + Rust Runtime)

> Applicable Version: `v1 only` (`/api/v1/*`, `/v1/*`, and `/v1beta`)  
> `v2` routes are entirely disabled; legacy calls will directly return `410 Gone`.

## 1. What You Get
- **Local Desktop Dashboard (Tauri)**: Easily log in, manage API keys, configure Pool strategies, and monitor proxy state in real-time.
- **High-Performance API Gateway (Rust)**: Canonical integration path is `/v1/chat/completions` (OpenAI-compatible). `/v1beta` remains available as a compatibility path for native Gemini clients.
- **Built-in Key Pool Rotation**: Supports `round_robin`, `random`, and `least_fail` logic natively.

## 2. Requirements
Both development and runtime environments run locally on your machine. You will need:
- **OS**: macOS (Apple Silicon M1/M2/M3 prioritized)
- **Compiler**: `rustup` (with standard stable toolchain ~1.77+)
- **Frontend Dependencies**: Node.js 20+ and `npm`
- **Utility Tools**: `lsof` (to automatically check for port conflicts at startup)

Verify your environment:
```bash
rustup --version
node -v
npm -v
lsof -v
```

## 3. First Launch (Setup Under 5 Minutes)
1. Environment Setup (Auto or Manual):

**Automatic Setup (Recommended)**: Simply execute:
```bash
./setup.sh
```
Follow the interactive prompts to paste your API Keys, and the script will package and generate the configuration files automatically!

**Manual Setup**:
```bash
cp .env.example .env
```
Open and manually define the required variables in `.env` (Check `.env.example` for details on defining `API_KEYS`).

2. Choose Startup Mode:
You can boot into Desktop or Server-only mode:

- **Desktop Version with GUI**
```bash
./start-desktop.sh
```

- **Headless Mode (Server-only)**
Ideal for server deployments relying solely on the fast backend proxy:
```bash
./start-headless.sh
```

3. Log into the Management Dashboard (Desktop only):
- After the Tauri window opens, check the login page and input the `AUTH_TOKEN` defined in your `.env`.
- Upon successful login, you can navigate routes such as `/keys` and `/config`.

## 4. Top 3 Things To Verify After Login
1. **Keys Page**: Confirm that your API keys loaded and the Pool displays a healthy state.
2. **Config Page**: Ensure your `POOL_STRATEGY` is properly detected (It is recommended to maintain `round_robin` initially).
3. **Terminal Test**: Run a cURL request on `/v1/chat/completions` to verify full upstream routing and proxy responsiveness.

## 5. Unified URL Endpoints Example

Once running, your proxy point will listen primarily on port `18080` (default).

Recommended defaults for new integrations:
- Base URL: `http://127.0.0.1:18080/v1`
- Auth: `Authorization: Bearer <proxy_token>`
- Model value: use alias (for example `sonnet`, `fast`)

### 5.0 Quick Verify Script
Use this script for a full smoke check (`/v1/models`, `/v1/chat/completions`, alias routing, and `/v1beta`):
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
If chat/native checks return `429` or `503`, the script reports a warning (upstream quota/high-demand), not a local proxy breakage.

### 5.1 Retrieve Available Models
```bash
curl -sS http://127.0.0.1:18080/v1/models \
  -H "Authorization: Bearer sk-user-123456"
```

### 5.2 Chat Completion (OpenAI Format)
```bash
curl -sS http://127.0.0.1:18080/v1/chat/completions \
  -H "Authorization: Bearer sk-user-123456" \
  -H "Content-Type: application/json" \
  -d '{
    "model":"sonnet",
    "messages":[{"role":"user","content":"Respond back: hello"}]
  }'
```

### 5.3 Requesting Model Aliases (Model Pool)
If mapped in `.env` (e.g., `MODEL_POOLS={"claude-sonnet":["gemini-2.5-pro"]}`), you can dictate the proxy to resolve your alias target dynamically.
```bash
curl -sS http://127.0.0.1:18080/v1/chat/completions \
  -H "Authorization: Bearer sk-user-123456" \
  -H "Content-Type: application/json" \
  -d '{
    "model":"claude-sonnet",
    "messages":[{"role":"user","content":"Hello!"}]
  }'
```
The Proxy intelligently replaces the alias with the raw `gemini-2.5-pro` model on the Google upstream side.

### 5.4 Native Gemini (`/v1beta`) Compatibility
```bash
curl -sS http://127.0.0.1:18080/v1beta/models/gemini-2.5-flash:generateContent \
  -H "X-goog-api-key: sk-user-123456" \
  -H "Content-Type: application/json" \
  -d '{
    "contents": [{"parts": [{"text": "Explain AI"}]}]
  }'
```
Compatibility note: `x-api-key`, `x-goog-api-key`, and query `?key=` are still accepted for legacy clients.

## 6. FAQ (Frequently Asked Questions)

### Q1: Continuous 401 Unauthorized errors during Dashboard login?
- The password you entered mismatches the `AUTH_TOKEN` value set in your `.env`.
- You modified `.env` but forgot to reboot the system via `./start-desktop.sh`.

### Q2: Notification claiming port occupied?
- `start-desktop.sh` aggressively checks and dumps conflicting sub-processes. Should forced release fail, manually trace and kill whatever relies on ports `18080-18099` or `1420`.

### Q3: Getting a 503 Service Unavailable during Proxy requests?
- In general, this denotes an empty "Healthy Key" state (all your keys are either Invalid or hitting the CoolDown constraint).
- Head to the `/keys` route on the dashboard, trace errors in your logs, and either Reset or replace the malfunctioning API keys manually.

## 7. Recommended Routine Ops
1. Regularly glance at the `/keys` health pool metric.
2. If limited rate scenarios present, adjust rules and fallback options in the `/config` UI.
3. In client configurations, prefer `http://127.0.0.1:18080/v1` with `Authorization: Bearer`.
4. Use `/v1beta` only when a specific client requires native Gemini endpoints.

## 8. Hermes / OpenClaw Quick Guide
If you are connecting from local agent tools, see:
- [Hermes/OpenClaw Integration Guide (Chinese)](hermes-openclaw-setup-zh.md)
