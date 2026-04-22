# Model Access Verification Report (2026-04-22)

## Scope
- Verify model-access health for current runtime.
- Confirm proxy-level reachability for `/v1/models`, `/v1/chat/completions`, and `/v1beta`.
- Provide a Docker build + simple API smoke flow for repeatable checks.

## Environment
- Workspace: `gemini-pool-proxy`
- Date: `2026-04-22`
- Runtime mode used for this verification: `./start-headless.sh`
- Due to local conflict on `18080` (OrbStack helper occupied that port), this run used:
  - `RUNTIME_PORT_START=18180`
  - `RUNTIME_PORT_END=18199`
  - effective test endpoint: `http://127.0.0.1:18180`

## Commands Executed

1. Start headless runtime on alternate port range:

```bash
RUNTIME_PORT_START=18180 RUNTIME_PORT_END=18199 ./start-headless.sh
```

2. Quick API verification:

```bash
BASE_URL=http://127.0.0.1:18180 ./scripts/quick-verify-api.sh
```

3. Additional repeated chat checks (5 sequential calls):

```bash
for i in 1 2 3 4 5; do
  curl -sS -o /tmp/chat_$i.json -w "%{http_code}" \
    http://127.0.0.1:18180/v1/chat/completions \
    -H "Authorization: Bearer sk-user-123456" \
    -H "Content-Type: application/json" \
    -d '{"model":"gemini-2.5-flash","messages":[{"role":"user","content":"Reply OK"}]}'
done
```

4. Docker flow verification (`go-wails` compose service):

```bash
docker compose build go-wails-headless
docker compose up -d go-wails-headless
START_COMPOSE=0 BASE_URL=http://127.0.0.1:18080 PROXY_TOKEN=sk-user-123456 MODEL=gemini-2.5-flash ./scripts/docker-smoke-api.sh
docker compose stop go-wails-headless
```

## Observed Results

### A) `quick-verify-api.sh`
- `/v1/models`: `200` (PASS)
- `/v1/chat/completions` (real model): `400` with `API key expired` (FAIL)
- `/v1/chat/completions` (alias model): `400` with `API key expired` (FAIL)
- `/v1beta/models/...:generateContent`: `200` (PASS)
- Script summary: `2 passed, 0 warning, 2 failed`

### B) Repeated chat calls (5 attempts)
- status sequence: `503`, `400`, `400`, `200`, `200`
- meaning:
  - `503`: temporary upstream high-demand/unavailable
  - `400 API key expired`: at least one key in pool is expired
  - `200`: model can be reached successfully when rotated to a healthy key

### C) Docker verification (`go-wails` on port `18080`)
- `docker compose build go-wails-headless`: success
- `docker compose up -d go-wails-headless`: success
- `scripts/docker-smoke-api.sh` result: `3 passed, 0 failed`
  - `/api/v1/health`: `200`
  - `/v1/models`: `200`
  - `/v1/chat/completions`: `200`

## Conclusion
- **Model access is functionally available** (confirmed by successful `200` chat responses and successful `/v1beta` call).
- **Current key pool quality is mixed**:
  - Some keys are valid (chat can return `200`)
  - Some keys are expired (`400 API key expired` appears during rotation)
- Operational recommendation:
  1. Rotate out expired keys from `.env` `API_KEYS`.
  2. Restart runtime.
  3. Re-run smoke verification until chat path stabilizes to mostly `200` (with occasional `429/503` acceptable under upstream load).

## Docker Build + Simple API Smoke (Standard Flow)

This repository includes:
- script: `scripts/docker-smoke-api.sh`

Default flow (build + up + 3 endpoint checks):

```bash
./scripts/docker-smoke-api.sh
```

Checks:
1. `GET /api/v1/health` must be `200`
2. `GET /v1/models` must be `200`
3. `POST /v1/chat/completions` accepts `200,400,429,503`

Useful overrides:

```bash
PROXY_TOKEN=sk-user-demo MODEL=gemini-2.5-flash ./scripts/docker-smoke-api.sh
START_COMPOSE=0 BASE_URL=http://127.0.0.1:18180 PROXY_TOKEN=sk-user-123456 ./scripts/docker-smoke-api.sh
CLEANUP=1 ./scripts/docker-smoke-api.sh
```
