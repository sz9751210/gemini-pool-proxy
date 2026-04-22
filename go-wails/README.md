# go-wails (Phase 1)

## Run Headless

```bash
cd go-wails
rtk go run ./cmd/server
```

## Run GUI

```bash
cd go-wails
rtk wails dev -p cmd/wails
```

## Verify

```bash
cd go-wails
rtk go test ./...
rtk bash scripts/smoke-phase1.sh
```

## Phase 2 Verify

```bash
cd go-wails
rtk go test ./...
rtk bash scripts/smoke-phase2.sh
```

## Docker

Build image:

```bash
cd go-wails
rtk docker build -t gemini-pool-proxy-go-wails:local .
```

Run container (headless):

```bash
rtk docker run --rm -p 18080:18080 \
  -e AUTH_TOKEN=sk-admin-demo \
  -e ALLOWED_TOKENS='["sk-user-demo"]' \
  -e API_KEYS='["AIza-demo"]' \
  gemini-pool-proxy-go-wails:local
```

## Docker Compose

The root `docker-compose.yml` starts the Go headless service and loads runtime keys/tokens from root `.env`.

```bash
cd ..
rtk docker compose up --build -d go-wails-headless
rtk docker compose logs -f go-wails-headless
```
