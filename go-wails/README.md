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
