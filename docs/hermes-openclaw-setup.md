# Hermes Agent and OpenClaw Integration Guide (Unified `/v1`)

This guide provides a practical setup path so **Hermes Agent** and **OpenClaw** can share the same proxy configuration.

- Unified endpoint: `http://127.0.0.1:18080/v1`
- Unified auth: `Authorization: Bearer <proxy_token>`
- Recommended model usage: start with an alias (for example `claude-sonnet`)

## 0. Verify Proxy Availability First

Start the runtime (headless):

```bash
./start-headless.sh
```

Run quick verification:

```bash
./scripts/quick-verify-api.sh
```

Interpretation:
- `/v1/models` should be `200`.
- `/v1/chat/completions` can be `200`, `429`, or `503` (quota/high-demand cases).
- If `/v1beta` is `200`, native Gemini passthrough is working.

If you deploy with Docker/Compose, use one-command container verification:

```bash
./scripts/docker-smoke-api.sh
```

## 1. Hermes Agent Setup

### 1.1 Base config (recommended)

Add to `~/.hermes/.env`:

```dotenv
GEMINI_API_KEY=sk-user-123456
GEMINI_BASE_URL=http://127.0.0.1:18080/v1
```

Set default provider and model:

```bash
hermes config set model.provider gemini
hermes config set model.base_url http://127.0.0.1:18080/v1
hermes config set model.default claude-sonnet
```

### 1.2 Clean old credentials (recommended)

If Hermes previously pointed to the official Gemini endpoint:

```bash
hermes auth list
```

Remove old entries and restart Hermes:

```bash
hermes auth remove gemini <index|id|label>
```

### 1.3 Validate Hermes routing

```bash
GEMINI_API_KEY=sk-user-123456 \
GEMINI_BASE_URL=http://127.0.0.1:18080/v1 \
hermes chat -q "Reply with token HERMES_DOC_OK only" --provider gemini -m claude-sonnet -Q
```

If you see `HERMES_DOC_OK`, Hermes is correctly routed through local proxy.

## 2. OpenClaw Setup

Using common `~/.picoclaw/config.json` format (OpenClaw/PicoClaw compatible):

### 2.1 Recommended config (`openai` provider + `/v1`)

```json
{
  "providers": {
    "openai": {
      "api_key": "sk-user-123456",
      "api_base": "http://127.0.0.1:18080/v1"
    }
  },
  "agents": {
    "defaults": {
      "model": "claude-sonnet"
    }
  }
}
```

### 2.2 Quick patch with `jq` (optional)

```bash
cp ~/.picoclaw/config.json ~/.picoclaw/config.json.bak

jq '
  .providers.openai.api_key = "sk-user-123456" |
  .providers.openai.api_base = "http://127.0.0.1:18080/v1" |
  .agents.defaults.model = "claude-sonnet"
' ~/.picoclaw/config.json > /tmp/picoclaw.config.json && mv /tmp/picoclaw.config.json ~/.picoclaw/config.json
```

Restart OpenClaw after editing.

## 3. Minimal API Validation

### 3.1 `/v1/models`

```bash
curl -sS http://127.0.0.1:18080/v1/models \
  -H "Authorization: Bearer sk-user-123456"
```

### 3.2 `/v1/chat/completions`

```bash
curl -sS http://127.0.0.1:18080/v1/chat/completions \
  -H "Authorization: Bearer sk-user-123456" \
  -H "Content-Type: application/json" \
  -d '{
    "model":"claude-sonnet",
    "messages":[{"role":"user","content":"Reply OK"}]
  }'
```

### 3.3 `/v1beta` (native Gemini path)

```bash
curl -sS http://127.0.0.1:18080/v1beta/models/gemini-2.5-flash:generateContent \
  -H "X-goog-api-key: sk-user-123456" \
  -H "Content-Type: application/json" \
  -d '{
    "contents":[{"parts":[{"text":"ping"}]}]
  }'
```

## 4. Troubleshooting

1. `/v1/models` is `200`, but chat intermittently returns `400 API key expired`
- Your key pool likely contains expired keys.
- Remove expired keys from `.env` `API_KEYS`, restart, then test again.

2. `Missing or invalid Authorization header`
- Verify base URL is `http://127.0.0.1:18080/v1` (must include `/v1`).
- Verify token exists in `.env` `ALLOWED_TOKENS`.

3. `429` / `503`
- Usually upstream quota/high-demand, not local route failure.
- Retry later or switch alias/model.
