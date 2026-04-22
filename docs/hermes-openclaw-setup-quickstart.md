# Hermes/OpenClaw Quickstart (Team Copy-Paste)

Use this for a minimal local setup where both Hermes and OpenClaw go through the same proxy.

## 1) Verify proxy is alive

```bash
./start-headless.sh
./scripts/quick-verify-api.sh
```

Expected:
- `/v1/models` returns `200`
- chat may return `200`, `429`, or `503`

## 2) Shared connection settings

- Base URL: `http://127.0.0.1:18080/v1`
- Auth header: `Authorization: Bearer <proxy_token>`
- Recommended model: `claude-sonnet`

## 3) Hermes quick setup

```bash
cat >> ~/.hermes/.env <<'EOF'
GEMINI_API_KEY=sk-user-123456
GEMINI_BASE_URL=http://127.0.0.1:18080/v1
EOF

hermes config set model.provider gemini
hermes config set model.base_url http://127.0.0.1:18080/v1
hermes config set model.default claude-sonnet
```

Quick test:

```bash
GEMINI_API_KEY=sk-user-123456 \
GEMINI_BASE_URL=http://127.0.0.1:18080/v1 \
hermes chat -q "Reply HERMES_OK only" --provider gemini -m claude-sonnet -Q
```

## 4) OpenClaw quick setup

```bash
cp ~/.picoclaw/config.json ~/.picoclaw/config.json.bak
jq '
  .providers.openai.api_key = "sk-user-123456" |
  .providers.openai.api_base = "http://127.0.0.1:18080/v1" |
  .agents.defaults.model = "claude-sonnet"
' ~/.picoclaw/config.json > /tmp/picoclaw.config.json && mv /tmp/picoclaw.config.json ~/.picoclaw/config.json
```

Restart OpenClaw after editing.

## 5) One-command Docker smoke (optional)

```bash
./scripts/docker-smoke-api.sh
```

## 6) If chat intermittently returns `400 API key expired`

Your `API_KEYS` pool likely contains expired keys. Remove expired keys from `.env`, restart, and verify again.

## Full Guides

- EN: `docs/hermes-openclaw-setup.md`
- ZH: `docs/hermes-openclaw-setup-zh.md`
